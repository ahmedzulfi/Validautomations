//! Developer inspector / profiler template: frame-time chart, span tree, zoomable flame graph,
//! property inspector and a filterable live log console.

use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Id, Key, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_components::{toast, LineIcon, Toast, UiIcon};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{Rng, Theme};

use super::common::{demo_notes, demo_user};
use super::prokit as pk;
use crate::shells::kit;
use crate::shells::{AppIcon, AppShell, NavItem, NavSection, ShellLayout, ShellNotification, Workspace};

/// (name, depth, weight, category, source)
const SHAPE: [(&str, usize, f32, usize, &str); 24] = [
    ("frame", 0, 1.0, 0, "src/main.rs:88"),
    ("update", 1, 0.34, 1, "src/app.rs:142"),
    ("input::poll", 2, 0.06, 1, "src/input/mod.rs:31"),
    ("physics::step", 2, 0.46, 2, "src/physics/world.rs:204"),
    ("broadphase", 3, 0.28, 2, "src/physics/bvh.rs:77"),
    ("narrowphase", 3, 0.38, 2, "src/physics/contact.rs:118"),
    ("solve_constraints", 3, 0.3, 2, "src/physics/solver.rs:56"),
    ("ai::tick", 2, 0.22, 3, "src/ai/mod.rs:40"),
    ("pathfind", 3, 0.7, 3, "src/ai/nav.rs:233"),
    ("scripts::run", 2, 0.2, 4, "src/scripting/vm.rs:410"),
    ("render", 1, 0.56, 5, "src/render/mod.rs:212"),
    ("cull", 2, 0.08, 5, "src/render/cull.rs:19"),
    ("shadows", 2, 0.22, 5, "src/render/shadows.rs:144"),
    ("gbuffer", 2, 0.3, 5, "src/render/gbuffer.rs:61"),
    ("draw_meshes", 3, 0.82, 5, "src/render/mesh.rs:302"),
    ("lighting", 2, 0.14, 5, "src/render/lighting.rs:90"),
    ("post", 2, 0.12, 5, "src/render/post.rs:25"),
    ("bloom", 3, 0.55, 5, "src/render/post/bloom.rs:48"),
    ("tonemap", 3, 0.3, 5, "src/render/post/tonemap.rs:12"),
    ("ui", 2, 0.1, 0, "src/ui/mod.rs:77"),
    ("layout", 3, 0.45, 0, "src/ui/layout.rs:130"),
    ("tessellate", 3, 0.4, 0, "src/ui/tess.rs:58"),
    ("present", 1, 0.07, 4, "src/render/swapchain.rs:66"),
    ("wait_vsync", 2, 0.85, 4, "src/render/swapchain.rs:81"),
];
const CATS: [&str; 6] = ["core", "update", "physics", "ai", "io", "gpu"];

#[derive(Clone, Debug)]
struct Span {
    shape: usize,
    depth: usize,
    start: f32,
    dur: f32,
    self_ms: f32,
    parent: Option<usize>,
}

fn build_spans(frame_ms: f32, seed: u64, spike: Option<usize>) -> Vec<Span> {
    let mut rng = Rng::new(seed);
    let mut spans: Vec<Span> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    // cursor per parent (start time for next child)
    let mut cursor: Vec<f32> = Vec::new();
    for (k, (_, depth, w, _, _)) in SHAPE.iter().enumerate() {
        while stack.len() > *depth {
            stack.pop();
            cursor.pop();
        }
        let parent = stack.last().copied();
        let (start, dur) = match parent {
            None => (0.0, frame_ms),
            Some(pi) => {
                let p = &spans[pi];
                let mut ww = *w * (0.8 + rng.f32() * 0.4);
                if spike == Some(k) {
                    ww *= 3.2;
                }
                let gap = p.dur * 0.012;
                let s = cursor.last().copied().unwrap_or(p.start) + gap;
                let d = (p.dur * ww).min(p.start + p.dur - s - gap).max(0.0);
                (s, d)
            }
        };
        if let Some(c) = cursor.last_mut() {
            *c = start + dur;
        }
        spans.push(Span { shape: k, depth: *depth, start, dur, self_ms: dur, parent });
        stack.push(k);
        cursor.push(start);
    }
    for i in 0..spans.len() {
        if let Some(p) = spans[i].parent {
            spans[p].self_ms -= spans[i].dur;
        }
    }
    for s in &mut spans {
        s.self_ms = s.self_ms.max(0.0);
    }
    spans
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Lvl {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug)]
struct LogLine {
    time: String,
    lvl: Lvl,
    target: &'static str,
    msg: String,
    frame: Option<u64>,
    born: f64,
}

/// A developer inspector / profiler for a running process (think a native companion to a game
/// engine or GUI app): a live frame-time chart with budget lines (click a frame to inspect it),
/// a collapsible span tree with total / self times, a zoomable flame graph (wheel zooms at the
/// cursor, drag pans, double-click fits a span, F resets), a property inspector with timings,
/// arguments and a per-frame history, and a log console with level filters, search and follow.
///
/// Keys: Space pauses recording, ← → step frames, F resets the flame-graph zoom.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::DevInspectorApp::default();
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct DevInspectorApp {
    /// Active route.
    pub route: String,
    /// Shell navigation.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// Capturing new frames.
    pub recording: bool,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    frames: Vec<f32>,
    first_frame: u64,
    selected_frame: Option<u64>,
    selected_span: usize,
    collapsed: Vec<bool>,
    view: (f32, f32),
    logs: Vec<LogLine>,
    levels: [bool; 5],
    log_query: String,
    follow: bool,
    last_frame_t: f64,
    seed: u64,
}

fn frame_ms(seed: u64, n: u64) -> (f32, Option<usize>) {
    let r = glowkit_core::rng::hash01(n.wrapping_mul(2654435761) ^ seed);
    let r2 = glowkit_core::rng::hash01(n.wrapping_add(77) ^ seed);
    if n % 37 == 21 {
        (31.0 + r * 14.0, Some(12)) // shadow cascade rebuild
    } else if n % 53 == 40 {
        (24.0 + r * 8.0, Some(9)) // script GC
    } else {
        (9.5 + r * 3.5 + r2 * r2 * 4.0, None)
    }
}

impl Default for DevInspectorApp {
    fn default() -> Self {
        let seed = 9;
        let first = 4_800;
        let frames: Vec<f32> = (first..first + 150).map(|n| frame_ms(seed, n).0).collect();
        let mut logs = Vec::new();
        let seedlogs: [(Lvl, &str, &str); 9] = [
            (Lvl::Info, "app", "Loaded scene \u{201c}harbor_night\u{201d} (2,418 entities) in 812 ms"),
            (Lvl::Debug, "render::shadows", "cascade splits = [4.0, 14.2, 41.7, 120.0]"),
            (Lvl::Info, "assets", "Streaming 38 textures (212 MB) from pack 3"),
            (Lvl::Trace, "input", "gamepad 0 connected: generic HID"),
            (Lvl::Warn, "physics", "Broadphase pairs exceeded soft limit (4,096): 4,412"),
            (Lvl::Debug, "ai::nav", "navmesh tile (12, 7) rebuilt in 3.1 ms"),
            (Lvl::Info, "net", "Connected to dev server 127.0.0.1:7777 (rtt 1 ms)"),
            (Lvl::Error, "scripting", "harbor/crane.lua:88: attempt to index nil value 'hook'"),
            (Lvl::Debug, "render", "pipeline cache hit rate 98.4%"),
        ];
        for (k, (l, tg, m)) in seedlogs.iter().enumerate() {
            logs.push(LogLine { time: format!("14:31:{:02}.{:03}", 2 + k * 3, (k * 137) % 1000), lvl: *l, target: tg, msg: m.to_string(), frame: None, born: -100.0 });
        }
        Self {
            route: "profiler".into(),
            nav: vec![
                NavSection::new("Inspect")
                    .item(NavItem::new("profiler", "Profiler").icon(UiIcon::Activity))
                    .item(NavItem::new("memory", "Memory").icon(AppIcon::Database))
                    .item(NavItem::new("entities", "Entities").icon(LineIcon::Layers))
                    .item(NavItem::new("logs", "Logs").icon(UiIcon::Terminal).badge("1")),
                NavSection::new("Session")
                    .item(NavItem::new("captures", "Captures").icon(LineIcon::Folder))
                    .item(NavItem::new("settings", "Settings").icon(LineIcon::Settings)),
            ],
            notifications: demo_notes(),
            recording: true,
            scoped_theme: false,
            frames,
            first_frame: first,
            selected_frame: Some(first + 142),
            selected_span: 12,
            collapsed: vec![false; SHAPE.len()],
            view: (0.0, 1.0),
            logs,
            levels: [false, true, true, true, true],
            log_query: String::new(),
            follow: true,
            last_frame_t: 0.0,
            seed,
        }
    }
}

const FRAME_INTERVAL: f64 = 1.0 / 14.0;
const MAX_FRAMES: usize = 240;

impl DevInspectorApp {
    fn last_frame(&self) -> u64 {
        self.first_frame + self.frames.len() as u64 - 1
    }

    fn cur_frame(&self) -> u64 {
        self.selected_frame.unwrap_or_else(|| self.last_frame()).clamp(self.first_frame, self.last_frame())
    }

    fn spans_for(&self, n: u64) -> Vec<Span> {
        let (ms, spike) = frame_ms(self.seed, n);
        build_spans(ms, n ^ self.seed, spike)
    }

    fn time_label(n: u64) -> String {
        let ms = n * 1000 / 60 + 31_000 * 60;
        format!("14:{:02}:{:02}.{:03}", ms / 60_000 % 60, ms / 1000 % 60, ms % 1000)
    }

    fn tick(&mut self, now: f64) {
        if self.last_frame_t == 0.0 {
            self.last_frame_t = now;
        }
        if !self.recording {
            self.last_frame_t = now;
            return;
        }
        let mut guard = 0;
        while now - self.last_frame_t >= FRAME_INTERVAL && guard < 6 {
            guard += 1;
            self.last_frame_t += FRAME_INTERVAL;
            let n = self.last_frame() + 1;
            let (ms, spike) = frame_ms(self.seed, n);
            self.frames.push(ms);
            if self.frames.len() > MAX_FRAMES {
                self.frames.remove(0);
                self.first_frame += 1;
            }
            let time = Self::time_label(n);
            if ms > 30.0 {
                let what = match spike {
                    Some(12) => "shadows: cascade 3 rebuilt (light moved)",
                    _ => "scripting: GC pause",
                };
                self.logs.push(LogLine { time: time.clone(), lvl: Lvl::Warn, target: "profiler", msg: format!("Frame {n} over budget: {ms:.1} ms ({what})"), frame: Some(n), born: now });
            } else if ms > 20.0 {
                self.logs.push(LogLine { time: time.clone(), lvl: Lvl::Info, target: "scripting", msg: format!("GC collected 18.2 MB in {:.1} ms", ms - 11.0), frame: Some(n), born: now });
            }
            if n % 5 == 0 {
                let opts: [(Lvl, &str, &str); 6] = [
                    (Lvl::Debug, "render", "draw calls 1,284 · triangles 2.1M · instanced 74%"),
                    (Lvl::Trace, "input", "mouse delta (3, -1)"),
                    (Lvl::Debug, "assets", "texture harbor_crane_albedo.ktx2 resident (4.0 MB)"),
                    (Lvl::Trace, "ai::nav", "agent 41 replanned path (17 nodes)"),
                    (Lvl::Info, "net", "snapshot 9,812 acked (1.2 KB)"),
                    (Lvl::Debug, "physics", "islands 37 · sleeping bodies 1,906"),
                ];
                let (l, tg, m) = opts[(n / 5 % 6) as usize];
                self.logs.push(LogLine { time, lvl: l, target: tg, msg: m.into(), frame: Some(n), born: now });
            }
            if self.logs.len() > 400 {
                self.logs.remove(0);
            }
        }
    }

    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::Sidebar)
            .start_collapsed(true)
            .workspaces(vec![Workspace::new("Lantern Inspector", "harbor · debug build")])
            .user(demo_user())
            .notifications(&mut notes)
            .padding(0.0)
            .scoped_theme(self.scoped_theme)
            .id_salt("dev_inspector_app")
            .show(ui, |ui, sc| {
                if sc.route.as_str() == "profiler" {
                    self.body(ui, sc.rect);
                } else {
                    super::analytics::placeholder(ui, sc.rect.shrink(24.0), sc.route);
                }
            });
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
    }

    fn body(&mut self, ui: &mut Ui, rect: Rect) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("inspector");
        let now = ui.input(|i| i.time);
        self.tick(now);
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(40));
        if !pk::typing(ui) {
            let (space, left, right, f) = ui.input(|i| (i.key_pressed(Key::Space), i.key_pressed(Key::ArrowLeft), i.key_pressed(Key::ArrowRight), i.key_pressed(Key::F)));
            if space {
                self.recording = !self.recording;
            }
            if left {
                self.selected_frame = Some(self.cur_frame().saturating_sub(1).max(self.first_frame));
            }
            if right {
                let n = self.cur_frame() + 1;
                self.selected_frame = if n >= self.last_frame() { None } else { Some(n) };
            }
            if f {
                self.view = (0.0, 1.0);
            }
        }
        let p = ui.painter().clone();
        let tool = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.min.y + 46.0));
        self.toolbar(ui, tool, id.with("tool"), now);
        kit::hline(&p, rect.min.x, rect.max.x, tool.max.y, &t);
        let chart = Rect::from_min_max(pos2(rect.min.x, tool.max.y + 1.0), pos2(rect.max.x, tool.max.y + 96.0));
        self.frame_chart(ui, chart, id.with("frames"));
        kit::hline(&p, rect.min.x, rect.max.x, chart.max.y, &t);
        let log_h = (rect.height() * 0.27).clamp(140.0, 210.0);
        let mid = Rect::from_min_max(pos2(rect.min.x, chart.max.y + 1.0), pos2(rect.max.x, rect.max.y - log_h));
        let logs = Rect::from_min_max(pos2(rect.min.x, mid.max.y), rect.max);
        kit::hline(&p, rect.min.x, rect.max.x, logs.min.y, &t);
        let cols = kit::split_x(mid, &[250.0, 0.0, 268.0], 0.0);
        kit::vline(&p, cols[0].max.x, mid.min.y, mid.max.y, &t);
        kit::vline(&p, cols[2].min.x, mid.min.y, mid.max.y, &t);
        p.rect_filled(cols[0], 0.0, kit::chrome_bg(&t));
        p.rect_filled(cols[2], 0.0, kit::chrome_bg(&t));
        let spans = self.spans_for(self.cur_frame());
        kit::child(ui, cols[0], |ui| self.tree(ui, cols[0], id.with("tree"), &spans));
        kit::child(ui, cols[1], |ui| self.flame(ui, cols[1], id.with("flame"), &spans));
        kit::child(ui, cols[2], |ui| self.props(ui, cols[2], id.with("props"), &spans));
        kit::child(ui, logs, |ui| self.console(ui, logs, id.with("logs"), now));
    }

    fn toolbar(&mut self, ui: &mut Ui, r: Rect, id: Id, now: f64) {
        let t = Theme::of(ui);
        let p = ui.painter().clone();
        let cy = r.center().y;
        let x0 = r.min.x + 14.0;
        // record
        let rb = Rect::from_min_size(pos2(x0, cy - 14.0), vec2(118.0, 28.0));
        let (resp, _) = kit::row(ui, rb, id.with("rec"), 7.0);
        p.rect_stroke(rb, 7.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
        if self.recording {
            pk::dot(&p, pos2(rb.min.x + 14.0, cy), 4.0, t.destructive, Some(now));
        } else {
            AppIcon::Pause.paint(&p, Rect::from_center_size(pos2(rb.min.x + 14.0, cy), vec2(12.0, 12.0)), t.muted_foreground);
        }
        kit::text_at(&p, pos2(rb.min.x + 26.0, cy), Align2::LEFT_CENTER, if self.recording { "Recording" } else { "Paused" }, 12.5, t.foreground, 70.0);
        pk::key_hint(&p, pos2(rb.max.x - 30.0, cy), "␣", &t);
        kit::tip(&resp, if self.recording { "Pause capture" } else { "Resume capture" }, Some("Space"));
        if resp.clicked() {
            self.recording = !self.recording;
        }
        // target
        let tr = Rect::from_min_size(pos2(rb.max.x + 10.0, cy - 14.0), vec2(214.0, 28.0));
        let (tresp, hv) = kit::row(ui, tr, id.with("target"), 7.0);
        p.rect_stroke(tr, 7.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
        UiIcon::Terminal.paint(&p, Rect::from_center_size(pos2(tr.min.x + 14.0, cy), vec2(13.0, 13.0)), t.muted_foreground);
        let nr = kit::text_at(&p, pos2(tr.min.x + 28.0, cy), Align2::LEFT_CENTER, "harbor-client", 12.5, t.foreground, 100.0);
        pk::mono(&p, pos2(nr.max.x + 6.0, cy), Align2::LEFT_CENTER, "pid 48213", 11.0, t.muted_foreground, 80.0);
        AppIcon::Ui(UiIcon::ChevronDown).paint(&p, Rect::from_center_size(pos2(tr.max.x - 12.0, cy), vec2(11.0, 11.0)), t.muted_foreground.lerp_to_gamma(t.foreground, hv));
        if tresp.clicked() {
            toast(ui.ctx(), Toast::new("Attach to process").description("3 instrumented processes found on this machine."));
        }
        // readouts
        let recent: Vec<f32> = self.frames.iter().rev().take(30).copied().collect();
        let avg = recent.iter().sum::<f32>() / recent.len().max(1) as f32;
        let mut sorted = recent.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let p99 = sorted.last().copied().unwrap_or(0.0);
        let mut x = tr.max.x + 22.0;
        for (val, cap, c) in [
            (format!("{:.0}", 1000.0 / avg.max(1.0)), "fps", t.foreground),
            (format!("{avg:.1} ms"), "avg frame", t.foreground),
            (format!("{p99:.1} ms"), "max (30)", if p99 > 33.3 { t.destructive } else if p99 > 16.7 { t.warning } else { t.foreground }),
            (kit::thousands(self.last_frame() as f64, 0), "frames", t.foreground),
        ] {
            let vr = pk::mono(&p, pos2(x, cy - 6.0), Align2::LEFT_CENTER, &val, 12.5, c, 90.0);
            let cr = kit::text_at(&p, pos2(x, cy + 10.0), Align2::LEFT_CENTER, cap, 10.0, t.muted_foreground, 80.0);
            x = vr.max.x.max(cr.max.x) + 20.0;
            if x > r.max.x - 250.0 {
                break;
            }
        }
        let ex = Rect::from_min_max(pos2(r.max.x - 122.0, cy - 14.0), pos2(r.max.x - 14.0, cy + 14.0));
        if pk::small_btn(ui, ex, id.with("export"), "Export trace", Some(UiIcon::Download.into()), None, None).clicked() {
            toast(ui.ctx(), Toast::success("Trace exported").description(format!("harbor-{}.trace.json · {} frames", self.last_frame(), self.frames.len())));
        }
        let cap = Rect::from_min_max(pos2(ex.min.x - 98.0, cy - 14.0), pos2(ex.min.x - 8.0, cy + 14.0));
        if pk::small_btn(ui, cap, id.with("snap"), "Snapshot", Some(AppIcon::Bookmark), None, None).clicked() {
            toast(ui.ctx(), Toast::success(format!("Frame {} pinned", self.cur_frame())));
            self.selected_frame = Some(self.cur_frame());
        }
    }

    fn frame_chart(&mut self, ui: &mut Ui, r: Rect, id: Id) {
        let t = Theme::of(ui);
        let p = ui.painter().clone();
        let area = Rect::from_min_max(pos2(r.min.x + 46.0, r.min.y + 10.0), pos2(r.max.x - 14.0, r.max.y - 8.0));
        let max_ms = 50.0f32;
        let y_of = |ms: f32| area.max.y - (ms / max_ms).min(1.0) * area.height();
        for (ms, label, c) in [(16.7, "16.7", t.warning), (33.3, "33.3", t.destructive)] {
            let y = y_of(ms);
            let mut xx = area.min.x;
            while xx < area.max.x {
                p.line_segment([pos2(xx, y), pos2((xx + 4.0).min(area.max.x), y)], Stroke::new(1.0, with_alpha(c, 0.55)));
                xx += 7.0;
            }
            pk::mono(&p, pos2(area.min.x - 8.0, y), Align2::RIGHT_CENTER, label, 10.0, with_alpha(c, 0.9), 40.0);
        }
        pk::mono(&p, pos2(area.min.x - 8.0, area.max.y - 4.0), Align2::RIGHT_CENTER, "0", 10.0, t.muted_foreground, 40.0);
        let bw = 5.0;
        let n_vis = ((area.width() / bw) as usize).min(self.frames.len());
        let start = self.frames.len() - n_vis;
        let resp = ui.interact(area, id.with("hit"), Sense::click());
        let left = area.max.x - n_vis as f32 * bw;
        let hover_k = resp.hover_pos().filter(|q| q.x >= left).map(|q| ((q.x - left) / bw).floor() as usize).filter(|k| *k < n_vis);
        let cur = self.cur_frame();
        for k in 0..n_vis {
            let ms = self.frames[start + k];
            let n = self.first_frame + (start + k) as u64;
            let c = if ms > 33.3 { t.destructive } else if ms > 16.7 { t.warning } else { t.success };
            let x = left + k as f32 * bw;
            let br = Rect::from_min_max(pos2(x + 0.5, y_of(ms)), pos2(x + bw - 0.5, area.max.y));
            let sel = n == cur;
            let hov = hover_k == Some(k);
            p.rect_filled(br, 1.0, with_alpha(c, if sel || hov { 1.0 } else { 0.55 }));
            if sel {
                p.rect_stroke(br.expand(1.5), 2.0, Stroke::new(1.5, t.foreground), StrokeKind::Outside);
            }
        }
        if let Some(k) = hover_k {
            let ms = self.frames[start + k];
            let n = self.first_frame + (start + k) as u64;
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            let x = left + k as f32 * bw + bw / 2.0;
            let txt = format!("Frame {n} · {ms:.1} ms");
            let w = kit::text_w(&p, &txt, 11.0) + 14.0;
            let tr = Rect::from_min_size(pos2((x - w / 2.0).clamp(area.min.x, area.max.x - w), area.min.y - 4.0), vec2(w, 20.0));
            p.rect_filled(tr, 5.0, t.foreground);
            p.text(tr.center(), Align2::CENTER_CENTER, txt, FontId::proportional(11.0), t.background);
            if resp.clicked() {
                self.selected_frame = if n == self.last_frame() { None } else { Some(n) };
            }
        }
        // live / pinned badge
        let live = self.selected_frame.is_none();
        let label = if live { "Following live" } else { "Pinned · → to step, click latest to follow" };
        kit::text_at(&p, pos2(area.max.x, r.min.y + 9.0), Align2::RIGHT_CENTER, label, 10.5, t.muted_foreground, 280.0);
    }

    fn tree(&mut self, ui: &mut Ui, r: Rect, id: Id, spans: &[Span]) {
        let t = Theme::of(ui);
        let p = ui.painter().clone();
        pk::label(&p, pos2(r.min.x + 14.0, r.min.y + 18.0), &format!("Frame {}", self.cur_frame()), &t, 120.0);
        kit::text_at(&p, pos2(r.max.x - 60.0, r.min.y + 18.0), Align2::RIGHT_CENTER, "total", 10.5, t.muted_foreground, 40.0);
        kit::text_at(&p, pos2(r.max.x - 12.0, r.min.y + 18.0), Align2::RIGHT_CENTER, "self", 10.5, t.muted_foreground, 40.0);
        let lr = Rect::from_min_max(pos2(r.min.x, r.min.y + 32.0), r.max);
        let mut select = None;
        let mut toggle = None;
        kit::child(ui, lr, |ui| {
            egui::ScrollArea::vertical().id_salt("insp_tree").auto_shrink([false, false]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let w = ui.available_width();
                let mut hidden_below: Option<usize> = None;
                for (i, s) in spans.iter().enumerate() {
                    if let Some(d) = hidden_below {
                        if s.depth > d {
                            continue;
                        }
                        hidden_below = None;
                    }
                    let (rr, resp) = ui.allocate_exact_size(vec2(w, 24.0), Sense::click());
                    let hv = kit::hover(ui, id.with(("r", i)), resp.hovered());
                    let p = ui.painter();
                    let sel = self.selected_span == i;
                    let rr2 = rr.shrink2(vec2(6.0, 1.0));
                    if sel {
                        p.rect_filled(rr2, 5.0, kit::active_fill(&t, kit::chrome_bg(&t)));
                    } else if hv > 0.01 {
                        p.rect_filled(rr2, 5.0, kit::hover_fill(&t, hv));
                    }
                    let has_kids = spans.get(i + 1).is_some_and(|n| n.depth > s.depth);
                    let x = rr.min.x + 12.0 + s.depth as f32 * 12.0;
                    let cy = rr.center().y;
                    if has_kids {
                        let cr = Rect::from_center_size(pos2(x + 5.0, cy), vec2(14.0, 14.0));
                        let icon = if self.collapsed[i] { UiIcon::ChevronRight } else { UiIcon::ChevronDown };
                        icon.paint(p, cr.shrink(2.0), t.muted_foreground);
                        if resp.clicked() && ui.input(|inp| inp.pointer.interact_pos().is_some_and(|q| q.x < x + 14.0)) {
                            toggle = Some(i);
                        }
                    }
                    let col = t.chart[SHAPE[s.shape].3 % 6];
                    p.rect_filled(Rect::from_center_size(pos2(x + 18.0, cy), vec2(3.0, 12.0)), 1.0, col);
                    let pct = s.dur / spans[0].dur.max(1e-3);
                    let tx = pk::mono(p, pos2(rr.max.x - 54.0, cy), Align2::RIGHT_CENTER, &format!("{:.2}", s.dur), 11.0, if pct > 0.25 && s.depth > 0 { pk::tone_fg(&t, t.warning) } else { t.foreground }, 50.0);
                    pk::mono(p, pos2(rr.max.x - 12.0, cy), Align2::RIGHT_CENTER, &format!("{:.2}", s.self_ms), 11.0, t.muted_foreground, 40.0);
                    kit::text_at(p, pos2(x + 26.0, cy), Align2::LEFT_CENTER, SHAPE[s.shape].0, 12.0, if sel { t.foreground } else { mix(t.foreground, t.muted_foreground, 0.25) }, tx.min.x - x - 32.0);
                    if resp.clicked() && toggle != Some(i) {
                        select = Some(i);
                    }
                    if resp.double_clicked() && has_kids {
                        toggle = Some(i);
                    }
                    if self.collapsed[i] {
                        hidden_below = Some(s.depth);
                    }
                }
            });
        });
        if let Some(i) = toggle {
            self.collapsed[i] = !self.collapsed[i];
        }
        if let Some(i) = select {
            self.selected_span = i;
        }
    }

    fn flame(&mut self, ui: &mut Ui, r: Rect, id: Id, spans: &[Span]) {
        let t = Theme::of(ui);
        let p = ui.painter().clone();
        let total = spans[0].dur.max(1e-3);
        let area = Rect::from_min_max(pos2(r.min.x + 12.0, r.min.y + 50.0), pos2(r.max.x - 12.0, r.max.y - 8.0));
        let resp = ui.interact(area, id.with("hit"), Sense::click_and_drag());
        // targets in fraction-of-frame space
        let (mut a, mut b) = self.view;
        if resp.hovered() {
            let (scroll, zd) = ui.input(|i| (i.smooth_scroll_delta.y + i.smooth_scroll_delta.x * 0.0, i.zoom_delta()));
            if scroll != 0.0 || zd != 1.0 {
                if let Some(q) = resp.hover_pos() {
                    let f = a + (q.x - area.min.x) / area.width() * (b - a);
                    let k = (1.0 - scroll * 0.003) / zd;
                    let span = ((b - a) * k).clamp(0.002, 1.0);
                    let rel = (f - a) / (b - a);
                    a = f - rel * span;
                    b = a + span;
                }
            }
        }
        if resp.dragged() {
            let d = -resp.drag_delta().x / area.width() * (b - a);
            a += d;
            b += d;
        }
        let span = (b - a).min(1.0);
        if a < 0.0 {
            a = 0.0;
            b = span;
        }
        if b > 1.0 {
            b = 1.0;
            a = 1.0 - span;
        }
        self.view = (a, b);
        let va = kit::follow(ui.ctx(), id.with("va"), a, 16.0);
        let vb = kit::follow(ui.ctx(), id.with("vb"), b, 16.0).max(va + 1e-4);
        let x_of = |ms: f32| area.min.x + ((ms / total) - va) / (vb - va) * area.width();
        // header + ruler
        kit::text_at(&p, pos2(r.min.x + 14.0, r.min.y + 16.0), Align2::LEFT_CENTER, "Flame graph · main thread", 12.5, t.foreground, 220.0);
        pk::hints(&p, pos2(r.min.x + 210.0, r.min.y + 16.0), &[("Wheel", "zoom"), ("Drag", "pan"), ("F", "reset")], &t, r.max.x - 70.0);
        kit::text_at(&p, pos2(r.max.x - 14.0, r.min.y + 16.0), Align2::RIGHT_CENTER, &format!("{:.0}%", 100.0 / (vb - va)), 11.0, t.muted_foreground, 50.0);
        let vis_ms = (vb - va) * total;
        let step = [0.01f32, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0].into_iter().find(|s| vis_ms / s <= 8.0).unwrap_or(10.0);
        let mut ms = (va * total / step).floor() * step;
        let p_clip = p.with_clip_rect(Rect::from_min_max(pos2(area.min.x, r.min.y + 30.0), area.max));
        while ms <= vb * total {
            let x = x_of(ms);
            p_clip.line_segment([pos2(x, area.min.y - 6.0), pos2(x, area.max.y)], Stroke::new(1.0, with_alpha(t.border, 0.55)));
            p_clip.text(pos2(x + 3.0, area.min.y - 4.0), Align2::LEFT_BOTTOM, format!("{ms:.2} ms"), FontId::monospace(9.5), t.muted_foreground);
            ms += step;
        }
        let rh = 22.0;
        let mut hovered = None;
        let hp = resp.hover_pos();
        for (i, s) in spans.iter().enumerate() {
            let x0 = x_of(s.start).max(area.min.x - 2.0);
            let x1 = x_of(s.start + s.dur).min(area.max.x + 2.0);
            if x1 < area.min.x || x0 > area.max.x || x1 - x0 < 0.5 {
                continue;
            }
            let y = area.min.y + 8.0 + s.depth as f32 * (rh + 2.0);
            let br = Rect::from_min_max(pos2(x0, y), pos2(x1.max(x0 + 1.0), y + rh));
            let base = t.chart[SHAPE[s.shape].3 % 6];
            let hov = hp.is_some_and(|q| br.contains(q));
            if hov {
                hovered = Some(i);
            }
            let sel = self.selected_span == i;
            let fill = mix(base, t.card, if t.dark { 0.45 } else { 0.55 } - if hov { 0.15 } else { 0.0 });
            p_clip.rect_filled(br.shrink2(vec2(0.5, 0.0)), 3.0, fill);
            if sel {
                p_clip.rect_stroke(br, 3.0, Stroke::new(1.5, t.foreground), StrokeKind::Inside);
            }
            if br.width() > 36.0 {
                let fg = if t.dark { Color32::WHITE } else { glowkit_core::color::shade(base, -0.45) };
                let lx = br.min.x.max(area.min.x) + 6.0;
                let label = format!("{} {:.2}ms", SHAPE[s.shape].0, s.dur);
                kit::text_at(&p_clip, pos2(lx, br.center().y), Align2::LEFT_CENTER, &label, 11.0, fg, br.max.x.min(area.max.x) - lx - 4.0);
            }
        }
        if let Some(i) = hovered {
            let s = &spans[i];
            ui.ctx().set_cursor_icon(if resp.dragged() { CursorIcon::Grabbing } else { CursorIcon::PointingHand });
            if let Some(q) = hp {
                let txt = format!("{}  {:.3} ms · {:.1}% of frame · self {:.3} ms", SHAPE[s.shape].0, s.dur, s.dur / total * 100.0, s.self_ms);
                let w = kit::text_w(&p, &txt, 11.5) + 16.0;
                let tr = Rect::from_min_size(pos2((q.x + 12.0).min(r.max.x - w - 4.0), q.y + 16.0), vec2(w, 24.0));
                glowkit_core::paint::soft_shadow(&p, tr, 6.0, 8.0, vec2(0.0, 2.0), kit::shadow(&t));
                p.rect_filled(tr, 6.0, t.popover);
                p.rect_stroke(tr, 6.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
                p.text(pos2(tr.min.x + 8.0, tr.center().y), Align2::LEFT_CENTER, txt, FontId::proportional(11.5), t.foreground);
            }
            if resp.clicked() {
                self.selected_span = i;
            }
            if resp.double_clicked() {
                let pad = s.dur * 0.04;
                self.view = (((s.start - pad) / total).max(0.0), ((s.start + s.dur + pad) / total).min(1.0));
            }
        } else if resp.double_clicked() {
            self.view = (0.0, 1.0);
        }
    }

    fn props(&mut self, ui: &mut Ui, r: Rect, id: Id, spans: &[Span]) {
        let t = Theme::of(ui);
        let p = ui.painter().clone();
        let i = self.selected_span.min(spans.len() - 1);
        let s = &spans[i];
        let (name, _, _, cat, src) = SHAPE[s.shape];
        let x0 = r.min.x + 16.0;
        let x1 = r.max.x - 16.0;
        pk::label(&p, pos2(x0, r.min.y + 18.0), "Inspector", &t, 120.0);
        let col = t.chart[cat % 6];
        pk::tag(&p, pos2(x1, r.min.y + 18.0), true, CATS[cat], col, &t, false);
        pk::mono(&p, pos2(x0, r.min.y + 44.0), Align2::LEFT_CENTER, name, 15.0, t.foreground, x1 - x0);
        let path: Vec<&str> = {
            let mut v = Vec::new();
            let mut c = s.parent;
            while let Some(pi) = c {
                v.push(SHAPE[spans[pi].shape].0);
                c = spans[pi].parent;
            }
            v.reverse();
            v
        };
        kit::text_at(&p, pos2(x0, r.min.y + 64.0), Align2::LEFT_CENTER, &if path.is_empty() { "root".to_string() } else { path.join(" › ") }, 11.0, t.muted_foreground, x1 - x0);
        let total = spans[0].dur.max(1e-3);
        let calls = match name {
            "draw_meshes" => "1,284",
            "pathfind" => "41",
            "narrowphase" => "4,412",
            _ => "1",
        };
        let rows: Vec<(&str, String)> = vec![
            ("Duration", format!("{:.3} ms", s.dur)),
            ("Self time", format!("{:.3} ms", s.self_ms)),
            ("Of frame", format!("{:.1}%", s.dur / total * 100.0)),
            ("Start", format!("+{:.3} ms", s.start)),
            ("Calls", calls.into()),
            ("Thread", "main (tid 1)".into()),
        ];
        let mut y = r.min.y + 88.0;
        for (k, v) in &rows {
            kit::text_at(&p, pos2(x0, y), Align2::LEFT_CENTER, k, 12.0, t.muted_foreground, 90.0);
            pk::mono(&p, pos2(x1, y), Align2::RIGHT_CENTER, v, 12.0, t.foreground, x1 - x0 - 90.0);
            y += 21.0;
        }
        // share bar
        pk::meter(&p, Rect::from_min_max(pos2(x0, y), pos2(x1, y + 5.0)), s.dur / total, col, &t);
        y += 20.0;
        // source
        let sr = Rect::from_min_max(pos2(x0, y), pos2(x1, y + 28.0));
        let (sresp, hv) = kit::row(ui, sr, id.with("src"), 6.0);
        p.rect_stroke(sr, 6.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
        UiIcon::Code.paint(&p, Rect::from_center_size(pos2(sr.min.x + 13.0, sr.center().y), vec2(12.0, 12.0)), t.muted_foreground);
        pk::mono(&p, pos2(sr.min.x + 26.0, sr.center().y), Align2::LEFT_CENTER, src, 11.0, t.foreground.lerp_to_gamma(t.primary, hv * 0.5), sr.width() - 50.0);
        AppIcon::External.paint(&p, Rect::from_center_size(pos2(sr.max.x - 13.0, sr.center().y), vec2(11.0, 11.0)), t.muted_foreground);
        kit::tip(&sresp, "Open in editor", None);
        if sresp.clicked() {
            toast(ui.ctx(), Toast::new("Opening in editor").description(src));
        }
        y += 44.0;
        // history of this span over recent frames
        pk::label(&p, pos2(x0, y), "Last 60 frames", &t, 140.0);
        let n_last = self.last_frame();
        let hist: Vec<f32> = (0..60u64).rev().map(|k| n_last.saturating_sub(k)).filter(|n| *n >= self.first_frame).map(|n| {
            let sp = self.spans_for(n);
            sp.get(i).map_or(0.0, |x| x.dur)
        }).collect();
        let hmax = hist.iter().copied().fold(0.01, f32::max);
        let avg = hist.iter().sum::<f32>() / hist.len().max(1) as f32;
        kit::text_at(&p, pos2(x1, y), Align2::RIGHT_CENTER, &format!("avg {avg:.2} · max {hmax:.2} ms"), 10.5, t.muted_foreground, 160.0);
        y += 12.0;
        let hr = Rect::from_min_max(pos2(x0, y), pos2(x1, (y + 52.0).min(r.max.y - 10.0)));
        if hr.height() > 10.0 {
            let bw = hr.width() / 60.0;
            let cur = self.cur_frame();
            for (k, v) in hist.iter().enumerate() {
                let n = n_last + 1 + k as u64 - hist.len() as u64;
                let h = (v / hmax) * hr.height();
                let br = Rect::from_min_max(pos2(hr.min.x + k as f32 * bw + 0.5, hr.max.y - h.max(1.0)), pos2(hr.min.x + (k + 1) as f32 * bw - 0.5, hr.max.y));
                p.rect_filled(br, 1.0, with_alpha(col, if n == cur { 1.0 } else { 0.5 }));
            }
            let ay = hr.max.y - avg / hmax * hr.height();
            p.line_segment([pos2(hr.min.x, ay), pos2(hr.max.x, ay)], Stroke::new(1.0, with_alpha(t.foreground, 0.35)));
        }
    }

    fn console(&mut self, ui: &mut Ui, r: Rect, id: Id, now: f64) {
        let t = Theme::of(ui);
        let p = ui.painter().clone();
        let cy = r.min.y + 22.0;
        let x0 = r.min.x + 14.0;
        kit::text_at(&p, pos2(x0, cy), Align2::LEFT_CENTER, "Console", 13.0, t.foreground, 80.0);
        let names = ["Trace", "Debug", "Info", "Warn", "Error"];
        let lvl_col = |k: usize| match k {
            0 => t.muted_foreground,
            1 => t.info,
            2 => t.success,
            3 => t.warning,
            _ => t.destructive,
        };
        let mut x = x0 + 74.0;
        for (k, n) in names.iter().enumerate() {
            let count = self.logs.iter().filter(|l| l.lvl as usize == k).count();
            let label = format!("{n} {count}");
            let w = pk::chip_w(&p, &label);
            let cr = Rect::from_min_size(pos2(x, cy - 11.0), vec2(w, 22.0));
            if pk::chip(ui, cr, id.with(("lv", k)), &label, self.levels[k], lvl_col(k)).clicked() {
                self.levels[k] = !self.levels[k];
            }
            x += w + 5.0;
        }
        let fr = Rect::from_min_max(pos2(r.max.x - 14.0 - 300.0, cy - 15.0), pos2(r.max.x - 14.0 - 96.0, cy + 15.0));
        if fr.min.x > x + 10.0 {
            pk::text_line(ui, fr, id.with("q"), &mut self.log_query, "Filter target or message", false);
        }
        let fb = Rect::from_min_max(pos2(r.max.x - 14.0 - 88.0, cy - 13.0), pos2(r.max.x - 14.0, cy + 13.0));
        if pk::chip(ui, fb, id.with("follow"), if self.follow { "Following" } else { "Follow" }, self.follow, t.primary).clicked() {
            self.follow = !self.follow;
        }
        let q = self.log_query.to_lowercase();
        let vis: Vec<&LogLine> = self
            .logs
            .iter()
            .filter(|l| self.levels[l.lvl as usize] && (q.is_empty() || l.msg.to_lowercase().contains(&q) || l.target.contains(&q)))
            .collect();
        let lr = Rect::from_min_max(pos2(r.min.x, cy + 20.0), r.max);
        let mut jump = None;
        kit::child(ui, lr, |ui| {
            let mut sa = egui::ScrollArea::vertical().id_salt("insp_logs").auto_shrink([false, false]);
            if self.follow {
                sa = sa.stick_to_bottom(true);
            }
            sa.show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let w = ui.available_width();
                for (k, l) in vis.iter().enumerate() {
                    let (rr, resp) = ui.allocate_exact_size(vec2(w, 20.0), Sense::click());
                    let p = ui.painter();
                    let lc = lvl_col(l.lvl as usize);
                    let fl = (1.0 - ((now - l.born) / 0.8) as f32).clamp(0.0, 1.0);
                    if l.lvl >= Lvl::Warn {
                        p.rect_filled(rr, 0.0, with_alpha(lc, if t.dark { 0.08 } else { 0.06 }));
                    }
                    if fl > 0.0 {
                        p.rect_filled(rr, 0.0, with_alpha(t.primary, 0.08 * fl));
                    }
                    if resp.hovered() {
                        p.rect_filled(rr, 0.0, kit::hover_fill(&t, 1.0));
                    }
                    let cy = rr.center().y;
                    let x = rr.min.x + 14.0;
                    pk::mono(p, pos2(x, cy), Align2::LEFT_CENTER, &l.time, 11.0, t.muted_foreground, 100.0);
                    pk::mono(p, pos2(x + 96.0, cy), Align2::LEFT_CENTER, &names[l.lvl as usize].to_uppercase(), 11.0, pk::tone_fg(&t, lc), 50.0);
                    pk::mono(p, pos2(x + 146.0, cy), Align2::LEFT_CENTER, l.target, 11.0, mix(t.foreground, t.muted_foreground, 0.4), 120.0);
                    let mx = x + 270.0;
                    let right = if l.frame.is_some() { rr.max.x - 80.0 } else { rr.max.x - 14.0 };
                    pk::mono(p, pos2(mx, cy), Align2::LEFT_CENTER, &l.msg, 11.5, t.foreground, right - mx);
                    if let Some(f) = l.frame {
                        if f >= self.first_frame {
                            pk::mono(p, pos2(rr.max.x - 14.0, cy), Align2::RIGHT_CENTER, &format!("#{f}"), 10.5, if resp.hovered() { t.primary } else { t.muted_foreground }, 70.0);
                            if resp.clicked() {
                                jump = Some(f);
                            }
                        }
                    }
                    let _ = k;
                }
                if vis.is_empty() {
                    let (er, _) = ui.allocate_exact_size(vec2(w, 60.0), Sense::hover());
                    ui.painter().text(er.center(), Align2::CENTER_CENTER, "No log lines match the filters", FontId::proportional(12.0), t.muted_foreground);
                }
            });
        });
        if let Some(f) = jump {
            self.selected_frame = Some(f);
            toast(ui.ctx(), Toast::new(format!("Jumped to frame {f}")));
        }
    }
}
