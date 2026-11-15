//! glowkit-bench: renders every glowkit demo headlessly and measures what it costs.
//!
//! ```text
//! cargo run --release -p glowkit-bench -- [--frames N] [--filter id] [--json out.json] [--no-app]
//! ```
//!
//! For every demo a fresh `egui::Context` renders the demo in a fixed 1100x720 viewport for N
//! frames of simulated time (1/60 s per frame) while the pointer travels along a Lissajous path.
//! Per frame we time the UI pass (`Context::run_ui`) and tessellation (`Context::tessellate`),
//! count primitives / vertices / indices, and read a counting global allocator. After the
//! animated frames the pointer leaves and we run idle frames to see whether the demo keeps
//! asking for repaints. App-level numbers (gallery binary size, wasm size, peak RSS of a
//! gallery-equivalent workload) are collected unless `--filter` or `--no-app` is given.

use glowkit::core::demo::{Category, Demo};
use glowkit::egui;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::RefCell;
use std::fmt::Write as _;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------------------------
// Counting allocator
// ---------------------------------------------------------------------------------------------

struct Counting;

static CUR: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static COUNT: AtomicU64 = AtomicU64::new(0);

fn grow(n: usize) {
    let now = CUR.fetch_add(n, Relaxed) + n;
    PEAK.fetch_max(now, Relaxed);
}

// SAFETY: forwards every call to the system allocator unchanged; only bookkeeping is added.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = System.alloc(l);
        if !p.is_null() {
            grow(l.size());
            COUNT.fetch_add(1, Relaxed);
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let p = System.alloc_zeroed(l);
        if !p.is_null() {
            grow(l.size());
            COUNT.fetch_add(1, Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l);
        CUR.fetch_sub(l.size(), Relaxed);
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        let q = System.realloc(p, l, new);
        if !q.is_null() {
            if new > l.size() {
                grow(new - l.size());
            } else {
                CUR.fetch_sub(l.size() - new, Relaxed);
            }
            COUNT.fetch_add(1, Relaxed);
        }
        q
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

// ---------------------------------------------------------------------------------------------
// Setup shared with the gallery look (fonts + light theme)
// ---------------------------------------------------------------------------------------------

const W: f32 = 1100.0;
const H: f32 = 720.0;
const DT: f64 = 1.0 / 60.0;
const IDLE_FRAMES: usize = 30;
/// A demo that takes longer than this in total is cut short (frames actually run are recorded).
const DEMO_BUDGET: Duration = Duration::from_secs(20);
/// A demo whose frame has not finished after this long is reported as hung (e.g. a lock
/// re-entered and deadlocked) and its thread is abandoned. The slowest healthy frame is < 1 s.
const FRAME_STALL: Duration = Duration::from_secs(10);
/// Frames completed by the current worker; the watchdog watches it advance.
static PROGRESS: AtomicUsize = AtomicUsize::new(0);

fn install_fonts(ctx: &egui::Context) {
    use egui::{FontData, FontDefinitions, FontFamily};
    use std::sync::Arc;
    let mut fonts = FontDefinitions::default();
    let mut add = |name: &str, bytes: &'static [u8]| {
        fonts.font_data.insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    };
    add("inter", include_bytes!("../../gallery/fonts/Inter-Regular.ttf"));
    add("inter-semibold", include_bytes!("../../gallery/fonts/Inter-SemiBold.ttf"));
    add("jakarta-bold", include_bytes!("../../gallery/fonts/PlusJakartaSans-Bold.ttf"));
    add("jetbrains", include_bytes!("../../gallery/fonts/JetBrainsMono-Regular.ttf"));
    let prop = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let mono = fonts.families.get(&FontFamily::Monospace).cloned().unwrap_or_default();
    let with = |first: &str, rest: &[String]| {
        let mut v = vec![first.to_owned()];
        v.extend(rest.iter().cloned());
        v
    };
    fonts.families.insert(FontFamily::Proportional, with("inter", &prop));
    fonts.families.insert(FontFamily::Monospace, with("jetbrains", &mono));
    fonts
        .families
        .insert(FontFamily::Name("display".into()), with("jakarta-bold", &with("inter-semibold", &prop)));
    fonts.families.insert(FontFamily::Name("semibold".into()), with("inter-semibold", &prop));
    ctx.set_fonts(fonts);
}

fn new_context() -> egui::Context {
    let ctx = egui::Context::default();
    install_fonts(&ctx);
    let themes = glowkit::Theme::presets();
    let light = themes.iter().find(|t| !t.dark).unwrap_or(&themes[0]);
    light.install(&ctx);
    ctx
}

fn input(frame: usize, events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(W, H))),
        time: Some(frame as f64 * DT),
        predicted_dt: DT as f32,
        focused: true,
        events,
        ..Default::default()
    }
}

/// Pointer path: a slow Lissajous curve that sweeps most of the preview area.
fn pointer_at(frame: usize) -> egui::Pos2 {
    let t = frame as f32 * DT as f32;
    egui::pos2(W * 0.5 + W * 0.4 * (t * 1.3).sin(), H * 0.5 + H * 0.38 * (t * 1.9 + 0.7).sin())
}

/// Same framing as the gallery's preview pane: a padded, rounded card inside a scroll area.
fn preview(ui: &mut egui::Ui, d: &Demo) {
    let fill = glowkit::Theme::of(ui).muted;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::new()
            .fill(fill)
            .corner_radius(egui::CornerRadius::same(28))
            .inner_margin(egui::Margin::same(24))
            .outer_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_min_height(360.0);
                ui.push_id(d.id, |ui| (d.show)(ui));
            });
    });
}

// ---------------------------------------------------------------------------------------------
// Per-demo measurement
// ---------------------------------------------------------------------------------------------

#[derive(Default)]
struct Sample {
    ui: f64,   // ms
    tess: f64, // ms
    prims: usize,
    verts: usize,
    idx: usize,
    allocs: u64,
    peak_bytes: usize,
}

struct DemoResult {
    demo: Demo,
    error: Option<String>,
    cold_ms: f64,
    samples: Vec<Sample>, // excludes the cold first frame
    retained_bytes: i64,
    idle_repaint: &'static str,
}

thread_local! {
    static PANIC_MSG: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn run_frame(ctx: &egui::Context, d: &Demo, inp: egui::RawInput) -> (Sample, Duration) {
    PROGRESS.fetch_add(1, Relaxed);
    let base = CUR.load(Relaxed);
    PEAK.store(base, Relaxed);
    COUNT.store(0, Relaxed);
    let t0 = Instant::now();
    let out = ctx.run_ui(inp, |ui| preview(ui, d));
    let ui = t0.elapsed();
    let delay = out.viewport_output.get(&egui::ViewportId::ROOT).map_or(Duration::MAX, |v| v.repaint_delay);
    let ppp = out.pixels_per_point;
    let t1 = Instant::now();
    let prims = ctx.tessellate(out.shapes, ppp);
    let tess = t1.elapsed();
    let mut s = Sample {
        ui: ui.as_secs_f64() * 1e3,
        tess: tess.as_secs_f64() * 1e3,
        prims: prims.len(),
        ..Default::default()
    };
    for p in &prims {
        if let egui::epaint::Primitive::Mesh(m) = &p.primitive {
            s.verts += m.vertices.len();
            s.idx += m.indices.len();
        }
    }
    drop(prims);
    s.allocs = COUNT.load(Relaxed);
    s.peak_bytes = PEAK.load(Relaxed).saturating_sub(base);
    (s, delay)
}

fn bench_demo(d: Demo, frames: usize) -> DemoResult {
    let mut r = DemoResult {
        demo: d,
        error: None,
        cold_ms: 0.0,
        samples: Vec::with_capacity(frames),
        retained_bytes: 0,
        idle_repaint: "none",
    };
    let res = panic::catch_unwind(AssertUnwindSafe(|| {
        let ctx = new_context();
        // Warm the context itself (font atlas, style) so "cold" measures the demo, not egui.
        let _ = ctx.run_ui(input(0, vec![]), |ui| {
            ui.label("warm-up");
        });
        let before = CUR.load(Relaxed) as i64;
        let started = Instant::now();
        let mut delays = Vec::new();
        for f in 0..frames {
            let (s, _) = run_frame(&ctx, &d, input(f + 1, vec![egui::Event::PointerMoved(pointer_at(f))]));
            if f == 0 {
                r.cold_ms = s.ui + s.tess;
            } else {
                r.samples.push(s);
            }
            if started.elapsed() > DEMO_BUDGET {
                break;
            }
        }
        // Idle: pointer leaves, nothing happens. Does the demo still want frames?
        let start = frames + 1;
        for i in 0..IDLE_FRAMES {
            let ev = if i == 0 { vec![egui::Event::PointerGone] } else { vec![] };
            let (_, delay) = run_frame(&ctx, &d, input(start + i, ev));
            delays.push(delay);
        }
        let tail = &delays[delays.len() - 10..];
        let max_tail = tail.iter().max().copied().unwrap_or(Duration::MAX);
        let min_tail = tail.iter().min().copied().unwrap_or(Duration::MAX);
        r.idle_repaint = if max_tail <= Duration::from_millis(50) {
            "continuous"
        } else if min_tail < Duration::from_secs(60) {
            "periodic"
        } else {
            "none"
        };
        r.retained_bytes = CUR.load(Relaxed) as i64 - before;
        drop(ctx);
    }));
    if res.is_err() {
        let msg = PANIC_MSG.with(|m| m.borrow_mut().take()).unwrap_or_else(|| "panic".into());
        r.error = Some(msg);
    }
    r
}

/// Runs `bench_demo` on a worker thread so a deadlocking demo cannot hang the whole run.
fn bench_demo_guarded(d: Demo, frames: usize) -> DemoResult {
    let (tx, rx) = std::sync::mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name(format!("bench-{}", d.id))
        .stack_size(64 << 20)
        .spawn(move || {
            let _ = tx.send(bench_demo(d, frames));
        });
    let hung = |msg: String| DemoResult {
        demo: d,
        error: Some(msg),
        cold_ms: 0.0,
        samples: Vec::new(),
        retained_bytes: 0,
        idle_repaint: "none",
    };
    if spawned.is_err() {
        return hung("could not spawn worker thread".into());
    }
    let mut last = (PROGRESS.load(Relaxed), Instant::now());
    loop {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(r) => return r,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return hung("worker thread died".into()),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                let p = PROGRESS.load(Relaxed);
                if p != last.0 {
                    last = (p, Instant::now());
                } else if last.1.elapsed() > FRAME_STALL {
                    return hung(format!(
                        "hung: a frame made no progress for {} s (deadlock?)",
                        FRAME_STALL.as_secs()
                    ));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Stats helpers
// ---------------------------------------------------------------------------------------------

fn quantile(v: &[f64], q: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.total_cmp(b));
    let i = ((s.len() - 1) as f64 * q).round() as usize;
    s[i]
}

struct Stat {
    med: f64,
    p95: f64,
    max: f64,
}

fn stat(v: &[f64]) -> Stat {
    Stat { med: quantile(v, 0.5), p95: quantile(v, 0.95), max: v.iter().copied().fold(0.0, f64::max) }
}

fn f(x: f64) -> String {
    format!("{x:.4}")
}

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn stat_json(s: &Stat) -> String {
    format!("{{\"median\": {}, \"p95\": {}, \"max\": {}}}", f(s.med), f(s.p95), f(s.max))
}

// ---------------------------------------------------------------------------------------------
// App-level numbers
// ---------------------------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap_or_else(|_| PathBuf::from("."))
}

fn file_size(p: &Path) -> Option<u64> {
    std::fs::metadata(p).ok().map(|m| m.len())
}

fn gzip_size(p: &Path) -> Option<u64> {
    let out = Command::new("gzip").args(["-9", "-c"]).arg(p).output().ok()?;
    out.status.success().then_some(out.stdout.len() as u64)
}

fn cmd_out(cmd: &str, args: &[&str]) -> Option<String> {
    let o = Command::new(cmd).args(args).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).trim().to_owned())
}

fn proc_status_kb(key: &str) -> Option<u64> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = s.lines().find(|l| l.starts_with(key))?;
    line.split_whitespace().nth(1)?.parse().ok()
}

/// Child-process mode: render every category the way the gallery's category page does (a grid
/// of preview tiles), a few frames each, in one long-lived context; print peak RSS.
fn gallery_rss(skip: &[String]) {
    let ctx = new_context();
    let _ = ctx.run_ui(input(0, vec![]), |ui| {
        ui.label("warm-up");
    });
    let baseline = proc_status_kb("VmRSS:").unwrap_or(0);
    let demos: Vec<Demo> = glowkit::all_demos().into_iter().filter(|d| !skip.iter().any(|s| s == d.id)).collect();
    let mut frame = 1;
    for cat in Category::ALL {
        let list: Vec<&Demo> = demos.iter().filter(|d| d.category == cat).collect();
        for _ in 0..4 {
            let out = ctx.run_ui(input(frame, vec![egui::Event::PointerMoved(pointer_at(frame))]), |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for d in &list {
                            ui.allocate_ui(egui::vec2(340.0, 280.0), |ui| {
                                ui.set_clip_rect(ui.max_rect());
                                ui.push_id(d.id, |ui| (d.show)(ui));
                            });
                        }
                    });
                });
            });
            let _ = ctx.tessellate(out.shapes, out.pixels_per_point);
            frame += 1;
        }
    }
    let peak = proc_status_kb("VmHWM:").unwrap_or(0);
    println!("baseline_kb {baseline}");
    println!("peak_kb {peak}");
}

struct App {
    gallery_bin: Option<u64>,
    gallery_stripped: Option<u64>,
    wasm: Option<u64>,
    wasm_gz: Option<u64>,
    js: Option<u64>,
    rss_peak_kb: Option<u64>,
    rss_base_kb: Option<u64>,
}

fn app_numbers(skip: &[String]) -> App {
    let root = workspace_root();
    let bin_path = ["glowkit-gallery", "glowkit-gallery.exe"]
        .iter()
        .map(|n| root.join("target/release").join(n))
        .find(|p| p.exists());
    let bin = bin_path.as_deref().and_then(file_size);
    // Size after `strip` (what you would ship), measured on a temporary copy.
    let gallery_stripped = bin_path.as_deref().and_then(|p| {
        let tmp = std::env::temp_dir().join(format!("glowkit-gallery-stripped-{}", std::process::id()));
        let ok = Command::new("strip").arg("-o").arg(&tmp).arg(p).status().ok()?.success();
        let size = if ok { file_size(&tmp) } else { None };
        let _ = std::fs::remove_file(&tmp);
        size
    });
    let pkg = root.join("website/gallery/pkg");
    let wasm_p = pkg.join("glowkit-gallery_bg.wasm");
    let (mut rss_peak_kb, mut rss_base_kb) = (None, None);
    if let Ok(exe) = std::env::current_exe() {
        let mut c = Command::new(exe);
        c.arg("--gallery-rss");
        if !skip.is_empty() {
            c.arg("--skip").arg(skip.join(","));
        }
        c.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null());
        if let Some(stdout) = c.spawn().ok().and_then(|mut child| {
            let t0 = Instant::now();
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) if t0.elapsed() < Duration::from_secs(600) => std::thread::sleep(Duration::from_millis(200)),
                    _ => {
                        let _ = child.kill();
                        return None;
                    }
                }
            }
            let mut s = String::new();
            std::io::Read::read_to_string(&mut child.stdout.take()?, &mut s).ok()?;
            Some(s)
        }) {
            for l in stdout.lines() {
                let mut it = l.split_whitespace();
                match (it.next(), it.next().and_then(|v| v.parse().ok())) {
                    (Some("peak_kb"), v) => rss_peak_kb = v,
                    (Some("baseline_kb"), v) => rss_base_kb = v,
                    _ => {}
                }
            }
        }
    }
    App {
        gallery_bin: bin,
        gallery_stripped,
        wasm: file_size(&wasm_p),
        wasm_gz: gzip_size(&wasm_p),
        js: file_size(&pkg.join("glowkit-gallery.js")),
        rss_peak_kb,
        rss_base_kb,
    }
}

fn opt(v: Option<u64>) -> String {
    v.map_or("null".into(), |v| v.to_string())
}

// ---------------------------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let get = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    if args.iter().any(|a| a == "--gallery-rss") {
        let skip: Vec<String> =
            get("--skip").map(|s| s.split(',').map(str::to_owned).collect()).unwrap_or_default();
        gallery_rss(&skip);
        return;
    }
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("usage: glowkit-bench [--frames N] [--filter id-substring] [--json out.json] [--no-app]");
        return;
    }
    let frames: usize = get("--frames").and_then(|s| s.parse().ok()).unwrap_or(120).max(2);
    let filter = get("--filter");
    let json_path = get("--json");
    let with_app = filter.is_none() && !args.iter().any(|a| a == "--no-app");

    panic::set_hook(Box::new(|info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_owned()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "panic".to_owned()
        };
        let loc = info.location().map(|l| format!(" ({}:{})", l.file(), l.line())).unwrap_or_default();
        PANIC_MSG.with(|m| *m.borrow_mut() = Some(format!("{msg}{loc}")));
    }));

    let demos: Vec<Demo> = glowkit::all_demos()
        .into_iter()
        .filter(|d| filter.as_deref().is_none_or(|f| d.id.contains(f)))
        .collect();
    eprintln!("glowkit-bench: {} demos x {} frames @ {}x{}", demos.len(), frames, W, H);

    let started = Instant::now();
    let mut results = Vec::with_capacity(demos.len());
    for (i, d) in demos.iter().enumerate() {
        let r = bench_demo_guarded(*d, frames);
        let tot: Vec<f64> = r.samples.iter().map(|s| s.ui + s.tess).collect();
        let st = stat(&tot);
        match &r.error {
            Some(e) => eprintln!("[{:>3}/{}] {:<36} ERROR {e}", i + 1, demos.len(), d.id),
            None => eprintln!(
                "[{:>3}/{}] {:<36} med {:>7.3} ms  p95 {:>7.3} ms  cold {:>7.2} ms  {:>6} verts  {}",
                i + 1,
                demos.len(),
                d.id,
                st.med,
                st.p95,
                r.cold_ms,
                quantile(&r.samples.iter().map(|s| s.verts as f64).collect::<Vec<_>>(), 0.5) as usize,
                r.idle_repaint
            ),
        }
        results.push(r);
    }
    let elapsed = started.elapsed().as_secs_f64();

    // Table to stdout, heaviest first.
    let mut ok: Vec<(&DemoResult, Stat)> = results
        .iter()
        .filter(|r| r.error.is_none())
        .map(|r| (r, stat(&r.samples.iter().map(|s| s.ui + s.tess).collect::<Vec<_>>())))
        .collect();
    ok.sort_by(|a, b| b.1.p95.total_cmp(&a.1.p95));
    let meds: Vec<f64> = ok.iter().map(|(_, s)| s.med).collect();
    println!("\n{:<38} {:>10} {:>10} {:>10}", "component", "median ms", "p95 ms", "max ms");
    for (r, s) in ok.iter().take(25) {
        println!("{:<38} {:>10.3} {:>10.3} {:>10.3}", r.demo.id, s.med, s.p95, s.max);
    }
    let under = |ms: f64| meds.iter().filter(|m| **m < ms).count() as f64 / meds.len().max(1) as f64 * 100.0;
    println!(
        "\n{} measured, {} errors; median of medians {:.3} ms; {:.1}% < 1 ms; {:.1}% < 4 ms; {} heavy (p95 > 4 ms); {:.0}s",
        ok.len(),
        results.len() - ok.len(),
        quantile(&meds, 0.5),
        under(1.0),
        under(4.0),
        ok.iter().filter(|(_, s)| s.p95 > 4.0).count(),
        elapsed
    );

    let skip: Vec<String> = results.iter().filter(|r| r.error.is_some()).map(|r| r.demo.id.to_owned()).collect();
    let app = with_app.then(|| app_numbers(&skip));
    if let Some(a) = &app {
        println!(
            "gallery binary {:?} B, wasm {:?} B (gzip {:?} B), gallery-workload peak RSS {:?} kB",
            a.gallery_bin, a.wasm, a.wasm_gz, a.rss_peak_kb
        );
    }

    if let Some(path) = json_path {
        let json = to_json(&results, frames, elapsed, app.as_ref());
        std::fs::write(&path, json).expect("write json");
        eprintln!("wrote {path}");
    }
}

fn to_json(results: &[DemoResult], frames: usize, elapsed: f64, app: Option<&App>) -> String {
    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("model name")).map(|l| l.split(':').nth(1).unwrap_or("").trim().to_owned()))
        .unwrap_or_else(|| "unknown".into());
    let cores = std::thread::available_parallelism().map_or(0, |n| n.get());
    let mem_kb = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| s.lines().next().and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok()));
    let os = cmd_out("uname", &["-srm"]).unwrap_or_else(|| std::env::consts::OS.into());
    let rustc = cmd_out("rustc", &["-V"]).unwrap_or_default();
    let date = cmd_out("date", &["-u", "+%Y-%m-%d"]).unwrap_or_default();

    // Histogram of every measured frame (ui + tessellation) on log-spaced bins.
    let edges: Vec<f64> = vec![0.0, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 33.0, f64::INFINITY];
    let mut counts = vec![0u64; edges.len() - 1];
    for r in results {
        for s in &r.samples {
            let t = s.ui + s.tess;
            let i = edges.windows(2).position(|w| t >= w[0] && t < w[1]).unwrap_or(counts.len() - 1);
            counts[i] += 1;
        }
    }

    let mut o = String::new();
    o.push_str("{\n  \"meta\": {");
    let _ = write!(
        o,
        "\"date\": {}, \"cpu\": {}, \"cores\": {}, \"mem_kb\": {}, \"os\": {}, \"rustc\": {}, \"profile\": \"release (opt-level 3, thin LTO)\", \"egui\": \"0.37\", \"frames\": {}, \"idle_frames\": {}, \"viewport\": [{}, {}], \"dt\": {}, \"elapsed_s\": {:.1}",
        esc(&date),
        esc(&cpu),
        cores,
        opt(mem_kb),
        esc(&os),
        esc(&rustc),
        frames,
        IDLE_FRAMES,
        W,
        H,
        f(DT),
        elapsed
    );
    o.push_str("},\n  \"app\": ");
    match app {
        Some(a) => {
            let _ = write!(
                o,
                "{{\"gallery_binary_bytes\": {}, \"gallery_binary_stripped_bytes\": {}, \"wasm_bytes\": {}, \"wasm_gzip_bytes\": {}, \"js_bytes\": {}, \"gallery_rss_peak_kb\": {}, \"gallery_rss_baseline_kb\": {}}}",
                opt(a.gallery_bin),
                opt(a.gallery_stripped),
                opt(a.wasm),
                opt(a.wasm_gz),
                opt(a.js),
                opt(a.rss_peak_kb),
                opt(a.rss_base_kb)
            );
        }
        None => o.push_str("null"),
    }
    o.push_str(",\n  \"histogram\": {\"edges_ms\": [");
    o.push_str(
        &edges.iter().map(|e| if e.is_finite() { format!("{e}") } else { "null".into() }).collect::<Vec<_>>().join(", "),
    );
    o.push_str("], \"counts\": [");
    o.push_str(&counts.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", "));
    o.push_str("]},\n  \"components\": [\n");
    let rows: Vec<String> = results
        .iter()
        .map(|r| {
            let d = &r.demo;
            let mut row = format!(
                "    {{\"id\": {}, \"name\": {}, \"category\": {}",
                esc(d.id),
                esc(d.name),
                esc(d.category.label())
            );
            if let Some(e) = &r.error {
                let _ = write!(row, ", \"error\": {}}}", esc(e));
                return row;
            }
            let col = |g: &dyn Fn(&Sample) -> f64| r.samples.iter().map(g).collect::<Vec<f64>>();
            let total = stat(&col(&|s| s.ui + s.tess));
            let ui = stat(&col(&|s| s.ui));
            let tess = stat(&col(&|s| s.tess));
            let verts = col(&|s| s.verts as f64);
            let _ = write!(
                row,
                ", \"frames\": {}, \"cold_ms\": {}, \"total_ms\": {}, \"ui_ms\": {}, \"tess_ms\": {}, \"primitives\": {}, \"vertices\": {}, \"vertices_max\": {}, \"indices\": {}, \"allocs\": {}, \"alloc_peak_bytes\": {}, \"retained_bytes\": {}, \"idle_repaint\": {}}}",
                r.samples.len() + 1,
                f(r.cold_ms),
                stat_json(&total),
                stat_json(&ui),
                stat_json(&tess),
                quantile(&col(&|s| s.prims as f64), 0.5) as u64,
                quantile(&verts, 0.5) as u64,
                verts.iter().copied().fold(0.0, f64::max) as u64,
                quantile(&col(&|s| s.idx as f64), 0.5) as u64,
                quantile(&col(&|s| s.allocs as f64), 0.5) as u64,
                col(&|s| s.peak_bytes as f64).iter().copied().fold(0.0, f64::max) as u64,
                r.retained_bytes,
                esc(r.idle_repaint)
            );
            row
        })
        .collect();
    o.push_str(&rows.join(",\n"));
    o.push_str("\n  ]\n}\n");
    o
}
