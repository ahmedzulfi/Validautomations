//! Local LLM desktop app: a model library with downloads and quantizations, RAM / VRAM meters,
//! and a streaming chat with system-prompt presets, sampling controls and a tokens/sec readout.

use egui::{pos2, vec2, Align2, Color32, Id, Key, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_ai::{ChatMessage, Conversation, MessageAction, PromptInput, Role};
use glowkit_components::{toast, LineIcon, Slider, Toast, UiIcon};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

use super::common::{demo_notes, demo_user};
use super::prokit as pk;
use crate::shells::kit;
use crate::shells::{AppIcon, AppShell, NavItem, NavSection, ShellLayout, ShellNotification, Workspace};

/// One quantization of a [`LocalModel`].
#[derive(Clone, Debug)]
pub struct ModelQuant {
    /// Name ("Q4_K_M").
    pub name: String,
    /// File size in GB.
    pub size_gb: f64,
    /// Downloaded to disk.
    pub downloaded: bool,
}

/// A model in the [`LocalLlmApp`] library.
#[derive(Clone, Debug)]
pub struct LocalModel {
    /// Display name.
    pub name: String,
    /// Family / architecture line.
    pub family: String,
    /// Parameter count in billions.
    pub params_b: f64,
    /// Context window in tokens.
    pub context: u32,
    /// Capability tags ("chat", "code", "vision").
    pub tags: Vec<String>,
    /// Available quantizations.
    pub quants: Vec<ModelQuant>,
    /// Selected quantization index.
    pub quant: usize,
    download: Option<(f64, f64)>,
}

fn q(name: &str, size: f64, dl: bool) -> ModelQuant {
    ModelQuant { name: name.into(), size_gb: size, downloaded: dl }
}

fn model(
    name: &str,
    family: &str,
    params: f64,
    ctx: u32,
    tags: &[&str],
    quants: Vec<ModelQuant>,
    quant: usize,
) -> LocalModel {
    LocalModel {
        name: name.into(),
        family: family.into(),
        params_b: params,
        context: ctx,
        tags: tags.iter().map(|s| s.to_string()).collect(),
        quants,
        quant,
        download: None,
    }
}

fn default_models() -> Vec<LocalModel> {
    vec![
        model(
            "Corvid 8B Instruct",
            "Corvid 3 · dense",
            8.0,
            32_768,
            &["chat", "tools"],
            vec![q("Q4_K_M", 4.9, true), q("Q5_K_M", 5.7, false), q("Q8_0", 8.5, false), q("F16", 16.1, false)],
            0,
        ),
        model(
            "Kestrel Coder 14B",
            "Kestrel · code",
            14.0,
            65_536,
            &["code", "fim"],
            vec![q("Q4_K_M", 8.9, true), q("Q6_K", 12.1, false), q("Q8_0", 15.7, false)],
            0,
        ),
        model(
            "Wren 3B",
            "Wren 2 · small",
            3.0,
            16_384,
            &["chat", "fast"],
            vec![q("Q4_K_M", 2.0, false), q("Q8_0", 3.4, true)],
            1,
        ),
        model(
            "Heron 32B Reasoning",
            "Heron · reasoning",
            32.0,
            131_072,
            &["reasoning"],
            vec![q("IQ3_XS", 13.7, false), q("Q4_K_M", 19.8, false), q("Q5_K_M", 23.3, false)],
            1,
        ),
        LocalModel {
            download: Some((-1.0, 7.0)),
            ..model(
                "Osprey Vision 11B",
                "Osprey · multimodal",
                11.0,
                32_768,
                &["vision", "chat"],
                vec![q("Q4_K_M", 7.0, false), q("Q8_0", 11.9, false)],
                0,
            )
        },
        model(
            "Finch 1.5B",
            "Finch · on-device",
            1.5,
            8_192,
            &["fast", "edge"],
            vec![q("Q4_0", 0.9, false), q("Q8_0", 1.6, false)],
            1,
        ),
        model(
            "Corvid 70B Instruct",
            "Corvid 3 · dense",
            70.0,
            32_768,
            &["chat", "tools"],
            vec![q("IQ2_M", 24.1, false), q("Q4_K_M", 42.5, false)],
            0,
        ),
    ]
}

const PRESETS: [(&str, &str); 4] = [
    ("Helpful assistant", "You are a helpful, accurate assistant. Answer clearly, use Markdown, and say when you are unsure."),
    ("Rust reviewer", "You are a senior Rust reviewer. Point out unsoundness, needless clones and unclear ownership first. Show fixed code."),
    ("Terse", "Answer in as few words as possible. No preamble. Code only when asked."),
    ("Storyteller", "You are a playful storyteller. Use vivid imagery, short paragraphs and end with a question."),
];

const RAM_TOTAL: f64 = 32.0;
const VRAM_TOTAL: f64 = 12.0;

#[derive(Clone, Debug)]
struct Gen {
    started: f64,
    answer: String,
    tps: f32,
}

/// A local-LLM desktop app (think a native front end for a llama.cpp-style runtime): a chat
/// that streams tokens with a live tokens/sec, time-to-first-token and context readout,
/// system-prompt presets and sampling sliders, a model switcher that loads weights into VRAM,
/// and a model library with quantization picks, "fits in VRAM" hints and downloads with
/// progress, speed and ETA. Model names are fictional.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::LocalLlmApp::default();
/// app.route = "models".into(); // or "chat"
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct LocalLlmApp {
    /// Active route ("chat" or "models").
    pub route: String,
    /// Shell navigation.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// The model library.
    pub models: Vec<LocalModel>,
    /// Index of the loaded model.
    pub loaded: usize,
    /// Messages: (from the user?, Markdown text, stats line).
    pub messages: Vec<(bool, String, String)>,
    /// Selected system-prompt preset.
    pub preset: usize,
    /// Sampling temperature.
    pub temperature: f32,
    /// Nucleus sampling.
    pub top_p: f32,
    /// Max new tokens.
    pub max_tokens: f32,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    input: String,
    generation: Option<Gen>,
    pending: Option<String>,
    loading: Option<(usize, f64)>,
    picker: bool,
    query: String,
    filter: usize,
    last_tps: f32,
    t0: Option<f64>,
}

impl Default for LocalLlmApp {
    fn default() -> Self {
        Self {
            route: "chat".into(),
            nav: vec![
                NavSection::new("Workspace")
                    .item(NavItem::new("chat", "Chat").icon(LineIcon::Mail))
                    .item(NavItem::new("models", "Models").icon(AppIcon::Database).badge("7"))
                    .item(NavItem::new("server", "Local server").icon(AppIcon::Server).badge("On"))
                    .item(NavItem::new("settings", "Settings").icon(LineIcon::Settings)),
                NavSection::new("Recent")
                    .item(NavItem::new("recent/1", "impl Trait vs dyn").icon(UiIcon::Sparkles))
                    .item(NavItem::new("recent/2", "SQL index advice").icon(UiIcon::Sparkles))
                    .item(NavItem::new("recent/3", "Bedtime story").icon(UiIcon::Sparkles)),
            ],
            notifications: demo_notes(),
            models: default_models(),
            loaded: 0,
            messages: vec![
                (true, "What's the difference between `Box<dyn Trait>` and `impl Trait` in return position?".into(), String::new()),
                (false, "**`impl Trait`** returns one concrete type chosen by the function, known at compile time: no allocation, static dispatch, inlinable.\n\n**`Box<dyn Trait>`** returns a heap-allocated trait object; the concrete type can differ per call and is dispatched through a vtable.\n\n```rust\nfn evens() -> impl Iterator<Item = u32> { (0..).step_by(2) }\nfn shapes(round: bool) -> Box<dyn Shape> {\n    if round { Box::new(Circle(1.0)) } else { Box::new(Square(1.0)) }\n}\n```\n\nUse `impl Trait` by default; reach for `dyn` when branches return different types.".into(), "41.8 tok/s · 164 tokens · 0.21 s to first token".into()),
            ],
            preset: 1,
            temperature: 0.7,
            top_p: 0.9,
            max_tokens: 1024.0,
            scoped_theme: false,
            input: String::new(),
            generation: None,
            pending: Some("Show a tiny example where `impl Trait` won't compile.".into()),
            loading: None,
            picker: false,
            query: String::new(),
            filter: 0,
            last_tps: 41.8,
            t0: None,
        }
    }
}

fn answer_for(prompt: &str, preset: usize) -> String {
    let p = prompt.to_lowercase();
    if preset == 2 {
        return "Different concrete types per branch. Use `Box<dyn Trait>` or an enum.".into();
    }
    if p.contains("impl trait") || p.contains("compile") {
        "Returning two different types from one `impl Trait` function fails, because the function must have **one** concrete return type:\n\n```rust\nfn shape(round: bool) -> impl Shape {\n    if round { Circle(1.0) } else { Square(1.0) } // error[E0308]\n}\n```\n\nThe compiler infers `Circle` from the first branch and rejects `Square`. Fix it with `Box<dyn Shape>`, or an `enum AnyShape { Circle(..), Square(..) }` that implements `Shape`.".into()
    } else {
        format!("Here's a quick take on *\u{201c}{}\u{201d}*:\n\n- Start from the smallest version that works\n- Measure before optimizing\n- Keep the interface boring and the internals swappable\n\nRunning fully offline on your GPU, nothing left this machine.", prompt.chars().take(60).collect::<String>())
    }
}

/// Simulated characters per token while streaming.
const CPT: f32 = 2.0;

fn stats_line(ui: &mut Ui, text: &str, t: &Theme) {
    if text.is_empty() {
        return;
    }
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 12.0), Sense::hover());
    pk::mono(
        ui.painter(),
        pos2(r.min.x + 44.0, r.min.y - 4.0),
        Align2::LEFT_CENTER,
        text,
        11.0,
        t.muted_foreground,
        w - 50.0,
    );
}

fn prefix_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

impl LocalLlmApp {
    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::Sidebar)
            .workspaces(vec![Workspace::new("Nestbox", "Local · offline")])
            .user(demo_user())
            .notifications(&mut notes)
            .padding(0.0)
            .scoped_theme(self.scoped_theme)
            .id_salt("local_llm_app")
            .show(ui, |ui, sc| {
                let now = ui.input(|i| i.time);
                self.tick(now, ui.ctx());
                match sc.route.as_str() {
                    "chat" | "recent/1" => self.chat(ui, sc.rect, now),
                    "models" => self.library(ui, sc.rect, now),
                    r => super::analytics::placeholder(ui, sc.rect.shrink(24.0), r),
                }
            });
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
    }

    fn tick(&mut self, now: f64, ctx: &egui::Context) {
        let t0 = *self.t0.get_or_insert(now);
        if let Some(p) = self.pending.take() {
            if now - t0 > 1.0 {
                self.submit(p, now);
            } else {
                self.pending = Some(p);
            }
        }
        if let Some((m, s)) = self.loading {
            if now - s > 1.6 {
                self.loaded = m;
                self.loading = None;
                toast(
                    ctx,
                    Toast::success(format!("{} loaded", self.models[m].name)).description(format!(
                        "{} · {:.1} GB in VRAM",
                        self.models[m].quants[self.models[m].quant].name,
                        self.vram_model(m)
                    )),
                );
            }
        }
        for m in &mut self.models {
            if let Some((start, size)) = m.download {
                if start < 0.0 {
                    m.download = Some((now - 22.0, size));
                    continue;
                }
                let speed = 0.085; // GB per second (simulated ~87 MB/s)
                if (now - start) * speed >= size {
                    m.download = None;
                    m.quants[m.quant].downloaded = true;
                    toast(
                        ctx,
                        Toast::success(format!("{} downloaded", m.name)).description(format!(
                            "{} · {}",
                            m.quants[m.quant].name,
                            pk::gb(size)
                        )),
                    );
                }
            }
        }
        if let Some(g) = &self.generation {
            let e = (now - g.started) as f32;
            let total = g.answer.chars().count();
            let cps = g.tps * CPT;
            if e > 0.22 && ((e - 0.22) * cps) as usize >= total {
                let tokens = (total as f32 / CPT) as usize;
                let stats = format!("{:.1} tok/s · {tokens} tokens · 0.22 s to first token", g.tps);
                self.last_tps = g.tps;
                self.messages.push((false, g.answer.clone(), stats));
                self.generation = None;
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(40));
    }

    fn submit(&mut self, prompt: String, now: f64) {
        self.messages.push((true, prompt.clone(), String::new()));
        let m = &self.models[self.loaded];
        let tps = (330.0 / m.params_b.max(1.0) as f32).clamp(6.0, 140.0) + 8.0;
        self.generation = Some(Gen { started: now, answer: answer_for(&prompt, self.preset), tps });
    }

    fn vram_model(&self, m: usize) -> f64 {
        let md = &self.models[m];
        md.quants[md.quant].size_gb + 0.6
    }

    fn mem(&self, now: f64) -> (f64, f64, f32) {
        let gen = self.generation.is_some();
        let mut vram = self.vram_model(self.loaded).min(VRAM_TOTAL - 0.3);
        if let Some((m, s)) = self.loading {
            let k = ((now - s) / 1.6).clamp(0.0, 1.0);
            vram = vram + (self.vram_model(m).min(VRAM_TOTAL - 0.3) - vram) * k;
        }
        let kv = 0.4 + self.messages.len() as f64 * 0.05;
        let ram = 11.2 + (now * 0.3).sin() * 0.15 + (self.vram_model(self.loaded) - VRAM_TOTAL).max(0.0);
        let util =
            if gen { 0.88 + 0.08 * ((now * 5.0).sin() as f32) } else { 0.04 + 0.02 * ((now * 1.3).sin() as f32).abs() };
        ((vram + kv).min(VRAM_TOTAL), ram, util)
    }

    // -----------------------------------------------------------------------------------------
    // chat
    // -----------------------------------------------------------------------------------------

    fn chat(&mut self, ui: &mut Ui, rect: Rect, now: f64) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("llm_chat");
        let parts = kit::split_x(rect, &[0.0, 296.0], 0.0);
        let (main, side) = (parts[0], parts[1]);
        let p = ui.painter().clone();
        p.rect_filled(side, 0.0, kit::chrome_bg(&t));
        kit::vline(&p, side.min.x, side.min.y, side.max.y, &t);
        // --- top strip ---
        let strip = Rect::from_min_max(main.min, pos2(main.max.x, main.min.y + 52.0));
        kit::hline(&p, strip.min.x, strip.max.x, strip.max.y, &t);
        let m = &self.models[self.loaded];
        let label = m.name.clone();
        let quant = m.quants[m.quant].name.clone();
        let pill_w = kit::text_w(&p, &label, 13.0) + kit::text_w(&p, &quant, 10.5) + 74.0;
        let pill = Rect::from_min_size(pos2(strip.min.x + 16.0, strip.center().y - 16.0), vec2(pill_w, 32.0));
        let (pr, hv) = kit::row(ui, pill, id.with("pill"), 8.0);
        p.rect_stroke(pill, 8.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
        let loading = self.loading;
        let dotc = if loading.is_some() { t.warning } else { t.success };
        pk::dot(&p, pos2(pill.min.x + 14.0, pill.center().y), 3.5, dotc, loading.map(|_| now));
        let nr = kit::text_at(
            &p,
            pos2(pill.min.x + 26.0, pill.center().y),
            Align2::LEFT_CENTER,
            &label,
            13.0,
            t.foreground,
            260.0,
        );
        pk::tag(&p, pos2(nr.max.x + 8.0, pill.center().y), false, &quant, t.primary, &t, true);
        AppIcon::Ui(UiIcon::ChevronDown).paint(
            &p,
            Rect::from_center_size(pos2(pill.max.x - 14.0, pill.center().y), vec2(12.0, 12.0)),
            t.muted_foreground.lerp_to_gamma(t.foreground, hv),
        );
        kit::tip(&pr, "Switch model", Some("Ctrl+L"));
        if pr.clicked() || ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, Key::L)) {
            self.picker = !self.picker;
        }
        // readouts (right)
        let gen = self.generation.clone();
        let tps = match &gen {
            Some(g) if now - g.started > 0.22 => g.tps + ((now * 7.0).sin() as f32) * 1.6,
            _ => self.last_tps,
        };
        let ctx_used: usize = self.messages.iter().map(|m| m.1.chars().count() / 4).sum::<usize>() + 180;
        let mut x = strip.max.x - 16.0;
        let ej = Rect::from_min_max(pos2(x - 70.0, strip.center().y - 14.0), pos2(x, strip.center().y + 14.0));
        if pk::small_btn(ui, ej, id.with("eject"), "Eject", None, None, None).clicked() {
            toast(
                ui.ctx(),
                Toast::new(format!("{label} unloaded")).description("VRAM freed. Pick a model to keep chatting."),
            );
        }
        x = ej.min.x - 16.0;
        for (k, (val, cap)) in [
            (format!("{} / {}k", kit::thousands(ctx_used as f64, 0), m.context / 1024), "context"),
            ("0.22 s".to_string(), "first token"),
            (format!("{tps:.1}"), "tok/s"),
        ]
        .iter()
        .enumerate()
        {
            let cr = kit::text_at(
                &p,
                pos2(x, strip.center().y + 8.0),
                Align2::RIGHT_CENTER,
                cap,
                10.5,
                t.muted_foreground,
                90.0,
            );
            let col = if k == 2 && gen.is_some() { pk::tone_fg(&t, t.success) } else { t.foreground };
            let vr = pk::mono(&p, pos2(x, strip.center().y - 7.0), Align2::RIGHT_CENTER, val, 13.0, col, 110.0);
            x = vr.min.x.min(cr.min.x) - 22.0;
        }
        // loading bar
        if let Some((mi, s)) = loading {
            let k = ((now - s) / 1.6).clamp(0.0, 1.0) as f32;
            p.rect_filled(
                Rect::from_min_size(pos2(strip.min.x, strip.max.y - 2.0), vec2(strip.width() * k, 2.0)),
                0.0,
                t.primary,
            );
            let _ = mi;
        }
        // --- conversation ---
        let comp_h = 132.0;
        let col_w = (main.width() - 48.0).min(740.0);
        let col = Rect::from_center_size(pos2(main.center().x, main.center().y), vec2(col_w, main.height()));
        let conv = Rect::from_min_max(pos2(col.min.x, strip.max.y + 1.0), pos2(col.max.x, main.max.y - comp_h));
        let mut copy = false;
        kit::child(ui, conv, |ui| {
            Conversation::new().height(conv.height()).fade_edges(t.background).id_salt("llm_conv").show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 18.0;
                ui.add_space(14.0);
                for (i, (user, text, stats)) in self.messages.iter().enumerate() {
                    if *user {
                        ui.add(ChatMessage::new(Role::User, text).id_salt(("llm_u", i)));
                    } else {
                        let out = ChatMessage::new(Role::Assistant, text)
                            .name(&label)
                            .actions(true)
                            .id_salt(("llm_a", i))
                            .show(ui, |_| {});
                        stats_line(ui, stats, &t);
                        if out.action == Some(MessageAction::Copy) {
                            copy = true;
                        }
                    }
                }
                if let Some(g) = &gen {
                    let e = (now - g.started) as f32;
                    let n = ((e - 0.22).max(0.0) * g.tps * CPT) as usize;
                    let part = prefix_chars(&g.answer, n);
                    ChatMessage::new(Role::Assistant, &part)
                        .name(&label)
                        .streaming(true)
                        .id_salt("llm_gen")
                        .show(ui, |_| {});
                    stats_line(ui, &format!("{tps:.1} tok/s · {} tokens · generating…", (n as f32 / CPT) as usize), &t);
                }
                ui.add_space(6.0);
            });
        });
        if copy {
            toast(ui.ctx(), Toast::success("Copied to clipboard"));
        }
        // --- composer ---
        let cr = Rect::from_min_max(pos2(col.min.x, main.max.y - comp_h + 8.0), pos2(col.max.x, main.max.y - 26.0));
        let mut submitted = None;
        let mut stop = false;
        let preset = self.preset;
        kit::child(ui, cr, |ui| {
            let out = PromptInput::new(&mut self.input)
                .placeholder("Message the local model…  (Enter to send)")
                .generating(gen.is_some())
                .attach_button(false)
                .id_salt("llm_prompt")
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(format!("Preset: {}", PRESETS[preset].0))
                            .size(11.5)
                            .color(t.muted_foreground),
                    );
                });
            submitted = out.submitted;
            stop = out.stop;
        });
        kit::text_at(
            &p,
            pos2(col.center().x, main.max.y - 13.0),
            Align2::CENTER_CENTER,
            "Runs entirely on this machine · nothing is sent to the network",
            11.0,
            t.muted_foreground,
            col.width(),
        );
        if stop {
            if let Some(g) = self.generation.take() {
                let n = (((now - g.started) as f32 - 0.22).max(0.0) * g.tps * CPT) as usize;
                self.messages.push((false, prefix_chars(&g.answer, n), "stopped by user".into()));
            }
        }
        if let Some(s) = submitted {
            if self.generation.is_none() {
                self.submit(s, now);
            }
        }
        // --- side panel ---
        self.side(ui, side, id.with("side"), now);
        // picker overlay last (on top)
        if self.picker {
            self.picker_overlay(ui, pill, rect, id.with("picker"), now);
        }
    }

    fn picker_overlay(&mut self, ui: &mut Ui, anchor: Rect, bounds: Rect, id: Id, now: f64) {
        let t = Theme::of(ui);
        let installed: Vec<usize> =
            (0..self.models.len()).filter(|i| self.models[*i].quants.iter().any(|q| q.downloaded)).collect();
        let h = 44.0 + installed.len() as f32 * 48.0;
        let r = Rect::from_min_size(pos2(anchor.min.x, anchor.max.y + 6.0), vec2(380.0, h));
        let p = ui.painter().clone();
        paint::soft_shadow(&p, r, 10.0, 18.0, vec2(0.0, 6.0), kit::shadow(&t));
        p.rect_filled(r, 10.0, t.popover);
        p.rect_stroke(r, 10.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
        pk::label(&p, pos2(r.min.x + 14.0, r.min.y + 20.0), "Downloaded models", &t, 200.0);
        kit::text_at(
            &p,
            pos2(r.max.x - 14.0, r.min.y + 20.0),
            Align2::RIGHT_CENTER,
            &format!("{VRAM_TOTAL:.0} GB VRAM"),
            11.0,
            t.muted_foreground,
            100.0,
        );
        let _ = ui.interact(r, id.with("eat"), Sense::click());
        let mut close = false;
        for (k, &mi) in installed.iter().enumerate() {
            let rr = Rect::from_min_size(
                pos2(r.min.x + 6.0, r.min.y + 38.0 + k as f32 * 48.0),
                vec2(r.width() - 12.0, 46.0),
            );
            let (resp, _) = kit::row(ui, rr, id.with(("m", mi)), 7.0);
            let md = &self.models[mi];
            let qi = md.quants.iter().position(|q| q.downloaded).unwrap_or(0);
            let size = md.quants[qi].size_gb;
            if mi == self.loaded {
                AppIcon::Ui(UiIcon::Check).paint(
                    &p,
                    Rect::from_center_size(pos2(rr.max.x - 16.0, rr.center().y), vec2(14.0, 14.0)),
                    t.primary,
                );
            }
            kit::text_at(
                &p,
                pos2(rr.min.x + 10.0, rr.min.y + 15.0),
                Align2::LEFT_CENTER,
                &md.name,
                13.0,
                t.foreground,
                rr.width() - 120.0,
            );
            let fits = size + 0.6 <= VRAM_TOTAL;
            let sub = format!(
                "{} · {} · {}",
                md.quants[qi].name,
                pk::gb(size),
                if fits { "fits in VRAM" } else { "partial offload" }
            );
            kit::text_at(
                &p,
                pos2(rr.min.x + 10.0, rr.min.y + 32.0),
                Align2::LEFT_CENTER,
                &sub,
                11.0,
                t.muted_foreground,
                rr.width() - 60.0,
            );
            if resp.clicked() {
                if mi != self.loaded {
                    self.models[mi].quant = qi;
                    self.loading = Some((mi, now));
                }
                close = true;
            }
        }
        let outside = ui.input(|i| {
            i.pointer.any_click() && i.pointer.interact_pos().is_some_and(|q| !r.contains(q) && !anchor.contains(q))
        });
        if close || outside || ui.input(|i| i.key_pressed(Key::Escape)) {
            self.picker = false;
        }
        let _ = bounds;
    }

    fn side(&mut self, ui: &mut Ui, r: Rect, id: Id, now: f64) {
        let t = Theme::of(ui);
        let p = ui.painter().clone();
        let x0 = r.min.x + 16.0;
        let x1 = r.max.x - 16.0;
        let mut y = r.min.y + 22.0;
        pk::label(&p, pos2(x0, y), "System prompt", &t, 200.0);
        y += 14.0;
        for (k, (name, _)) in PRESETS.iter().enumerate() {
            let rr = Rect::from_min_size(pos2(x0 - 6.0, y), vec2(x1 - x0 + 12.0, 30.0));
            let (resp, _) = kit::row(ui, rr, id.with(("preset", k)), 7.0);
            let on = self.preset == k;
            if on {
                p.rect_filled(rr, 7.0, kit::active_fill(&t, kit::chrome_bg(&t)));
            }
            let c = pos2(rr.min.x + 15.0, rr.center().y);
            p.circle_stroke(c, 6.0, Stroke::new(1.3, if on { t.primary } else { with_alpha(t.muted_foreground, 0.6) }));
            if on {
                p.circle_filled(c, 3.2, t.primary);
            }
            kit::text_at(
                &p,
                pos2(rr.min.x + 30.0, rr.center().y),
                Align2::LEFT_CENTER,
                name,
                12.5,
                if on { t.foreground } else { t.muted_foreground },
                rr.width() - 60.0,
            );
            if k < 9 {
                kit::text_at(
                    &p,
                    pos2(rr.max.x - 8.0, rr.center().y),
                    Align2::RIGHT_CENTER,
                    &format!("Alt {}", k + 1),
                    10.5,
                    with_alpha(t.muted_foreground, 0.7),
                    50.0,
                );
            }
            if resp.clicked() {
                self.preset = k;
            }
            y += 31.0;
        }
        for k in 0..PRESETS.len() {
            let key = [Key::Num1, Key::Num2, Key::Num3, Key::Num4][k];
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::ALT, key)) {
                self.preset = k;
            }
        }
        y += 4.0;
        let g = kit::wrapped(&p, PRESETS[self.preset].1, 11.5, t.muted_foreground, x1 - x0 - 20.0, 3);
        let br = Rect::from_min_size(pos2(x0, y), vec2(x1 - x0, g.size().y + 16.0));
        p.rect_filled(br, 7.0, if t.dark { mix(t.background, Color32::BLACK, 0.2) } else { t.background });
        p.rect_stroke(br, 7.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
        p.galley(br.min + vec2(10.0, 8.0), g, t.muted_foreground);
        y = br.max.y + 24.0;
        pk::label(&p, pos2(x0, y), "Sampling", &t, 200.0);
        y += 16.0;
        let sliders: [(&str, &mut f32, std::ops::RangeInclusive<f32>, usize); 3] = [
            ("Temperature", &mut self.temperature, 0.0..=2.0, 2),
            ("Top-p", &mut self.top_p, 0.05..=1.0, 2),
            ("Max tokens", &mut self.max_tokens, 64.0..=4096.0, 0),
        ];
        for (name, v, range, dec) in sliders {
            kit::text_at(&p, pos2(x0, y + 8.0), Align2::LEFT_CENTER, name, 12.0, t.foreground, 120.0);
            pk::mono(
                &p,
                pos2(x1, y + 8.0),
                Align2::RIGHT_CENTER,
                &format!("{:.*}", dec, *v),
                11.5,
                t.muted_foreground,
                60.0,
            );
            let sr = Rect::from_min_size(pos2(x0, y + 18.0), vec2(x1 - x0, 22.0));
            kit::child(ui, sr, |ui| ui.add(Slider::new(v, range).width(sr.width()).decimals(dec)));
            y += 44.0;
        }
        y += 8.0;
        kit::hline(&p, r.min.x, r.max.x, y, &t);
        y += 20.0;
        let (vram, ram, util) = self.mem(now);
        pk::label(&p, pos2(x0, y), "Hardware", &t, 200.0);
        kit::text_at(&p, pos2(x1, y), Align2::RIGHT_CENTER, "RTX-class GPU · 12 GB", 11.0, t.muted_foreground, 160.0);
        y += 20.0;
        let rows = [
            ("VRAM", vram, VRAM_TOTAL, if vram / VRAM_TOTAL > 0.9 { t.warning } else { t.chart[0] }),
            ("RAM", ram, RAM_TOTAL, t.chart[1]),
        ];
        for (k, (name, used, total, c)) in rows.iter().enumerate() {
            let f = kit::follow(ui.ctx(), id.with(("mem", k)), (*used / *total) as f32, 5.0);
            kit::text_at(&p, pos2(x0, y), Align2::LEFT_CENTER, name, 12.0, t.foreground, 60.0);
            pk::mono(
                &p,
                pos2(x1, y),
                Align2::RIGHT_CENTER,
                &format!("{:.1} / {:.0} GB", f as f64 * total, total),
                11.5,
                t.muted_foreground,
                120.0,
            );
            pk::meter(&p, Rect::from_min_max(pos2(x0, y + 10.0), pos2(x1, y + 16.0)), f, *c, &t);
            y += 32.0;
        }
        let u = kit::follow(ui.ctx(), id.with("util"), util, 6.0);
        kit::text_at(&p, pos2(x0, y), Align2::LEFT_CENTER, "GPU", 12.0, t.foreground, 60.0);
        pk::mono(&p, pos2(x1, y), Align2::RIGHT_CENTER, &format!("{:.0}%", u * 100.0), 11.5, t.muted_foreground, 60.0);
        // util history bars
        let hist: Vec<f32> = glowkit_core::state(ui, id.with("hist"), || vec![0.05f32; 48]);
        let step = (now * 6.0) as u64;
        let last_step: u64 = glowkit_core::state(ui, id.with("hstep"), || step);
        let mut hist = hist;
        if step != last_step {
            hist.remove(0);
            hist.push(u);
            glowkit_core::set_state(ui, id.with("hstep"), step);
        }
        let hr = Rect::from_min_max(pos2(x0, y + 10.0), pos2(x1, (y + 38.0).min(r.max.y - 8.0)));
        let bw = hr.width() / hist.len() as f32;
        for (i, v) in hist.iter().enumerate() {
            let bh = (hr.height() * v.clamp(0.08, 1.0)).max(2.0);
            let br = Rect::from_min_max(
                pos2(hr.min.x + i as f32 * bw + 0.5, hr.max.y - bh),
                pos2(hr.min.x + (i + 1) as f32 * bw - 0.5, hr.max.y),
            );
            p.rect_filled(br, 1.0, with_alpha(t.success, 0.35 + 0.6 * v));
        }
        glowkit_core::set_state(ui, id.with("hist"), hist);
    }

    // -----------------------------------------------------------------------------------------
    // model library
    // -----------------------------------------------------------------------------------------

    fn library(&mut self, ui: &mut Ui, rect: Rect, now: f64) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("llm_lib");
        let inner = rect.shrink2(vec2(24.0, 20.0));
        let p = ui.painter().clone();
        kit::text_at(&p, inner.min, Align2::LEFT_TOP, "Models", 22.0, t.foreground, 300.0);
        kit::text_at(
            &p,
            inner.min + vec2(0.0, 31.0),
            Align2::LEFT_TOP,
            "Download open-weight models and pick a quantization that fits your GPU.",
            13.5,
            t.muted_foreground,
            inner.width() * 0.6,
        );
        // hardware tiles
        let (vram, ram, util) = self.mem(now);
        let disk_used: f64 = 212.0
            + self.models.iter().flat_map(|m| m.quants.iter()).filter(|q| q.downloaded).map(|q| q.size_gb).sum::<f64>()
            - 16.3;
        let tiles =
            kit::cols(Rect::from_min_size(pos2(inner.min.x, inner.min.y + 64.0), vec2(inner.width(), 70.0)), 4, 12.0);
        let tile_data = [
            ("VRAM", format!("{vram:.1} / {VRAM_TOTAL:.0} GB"), (vram / VRAM_TOTAL) as f32, t.chart[0]),
            ("System RAM", format!("{ram:.1} / {RAM_TOTAL:.0} GB"), (ram / RAM_TOTAL) as f32, t.chart[1]),
            ("Disk (models)", format!("{disk_used:.0} / 512 GB"), (disk_used / 512.0) as f32, t.chart[2]),
            ("GPU load", format!("{:.0}%", util * 100.0), util, t.success),
        ];
        for (k, (r, (name, val, f, c))) in tiles.iter().zip(tile_data.iter()).enumerate() {
            kit::card(&p, *r, &t);
            pk::label(&p, pos2(r.min.x + 14.0, r.min.y + 18.0), name, &t, r.width() - 28.0);
            pk::mono(
                &p,
                pos2(r.min.x + 14.0, r.min.y + 40.0),
                Align2::LEFT_CENTER,
                val,
                14.0,
                t.foreground,
                r.width() - 28.0,
            );
            let f = kit::follow(ui.ctx(), id.with(("tile", k)), *f, 5.0);
            pk::meter(
                &p,
                Rect::from_min_max(pos2(r.min.x + 14.0, r.max.y - 14.0), pos2(r.max.x - 14.0, r.max.y - 9.0)),
                f,
                *c,
                &t,
            );
        }
        // toolbar
        let ty = inner.min.y + 150.0;
        pk::search(
            ui,
            Rect::from_min_size(pos2(inner.min.x, ty), vec2(260.0, 32.0)),
            &mut self.query,
            "Search models",
            "llm_search",
        );
        let filters = ["All", "Downloaded", "Fits in VRAM", "Code"];
        let mut x = inner.min.x + 276.0;
        for (k, f) in filters.iter().enumerate() {
            let w = pk::chip_w(&p, f) + 4.0;
            let cr = Rect::from_min_size(pos2(x, ty + 4.0), vec2(w, 24.0));
            if pk::chip(ui, cr, id.with(("f", k)), f, self.filter == k, t.primary).clicked() {
                self.filter = k;
            }
            x += w + 6.0;
        }
        let q = self.query.to_lowercase();
        let vis: Vec<usize> = (0..self.models.len())
            .filter(|&i| {
                let m = &self.models[i];
                let ok = match self.filter {
                    1 => m.quants.iter().any(|q| q.downloaded),
                    2 => m.quants[m.quant].size_gb + 0.6 <= VRAM_TOTAL,
                    3 => m.tags.iter().any(|t| t == "code"),
                    _ => true,
                };
                ok && (q.is_empty() || m.name.to_lowercase().contains(&q) || m.family.to_lowercase().contains(&q))
            })
            .collect();
        kit::text_at(
            &p,
            pos2(inner.max.x, ty + 16.0),
            Align2::RIGHT_CENTER,
            &format!("{} models", vis.len()),
            12.0,
            t.muted_foreground,
            100.0,
        );
        // list
        let lr = Rect::from_min_max(pos2(inner.min.x, ty + 44.0), pos2(inner.max.x, rect.max.y - 12.0));
        kit::card(&p, lr, &t);
        let mut actions: Vec<(usize, &'static str, usize)> = Vec::new();
        kit::child(ui, lr.shrink(1.0), |ui| {
            egui::ScrollArea::vertical().id_salt("llm_lib_list").auto_shrink([false, false]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let w = ui.available_width();
                for (row_i, &mi) in vis.iter().enumerate() {
                    let (rr, _) = ui.allocate_exact_size(vec2(w, 68.0), Sense::hover());
                    let hov = ui.rect_contains_pointer(rr);
                    let hv = kit::hover(ui, id.with(("rowh", mi)), hov);
                    let p = ui.painter().clone();
                    if hv > 0.01 {
                        p.rect_filled(rr, 0.0, kit::hover_fill(&t, hv * 0.8));
                    }
                    if row_i > 0 {
                        kit::hline(&p, rr.min.x, rr.max.x, rr.min.y, &t);
                    }
                    let m = self.models[mi].clone();
                    let x0 = rr.min.x + 16.0;
                    let cy = rr.center().y;
                    // tile
                    let tile = Rect::from_center_size(pos2(x0 + 18.0, cy), vec2(36.0, 36.0));
                    let tc = t.chart[mi % 6];
                    p.rect_filled(tile, 9.0, pk::soft(&t, tc, 1.6));
                    kit::text_at(
                        &p,
                        tile.center(),
                        Align2::CENTER_CENTER,
                        &format!("{}B", m.params_b),
                        11.0,
                        pk::tone_fg(&t, tc),
                        34.0,
                    );
                    // right block: action
                    let ax1 = rr.max.x - 16.0;
                    let aw = 132.0;
                    let ar = Rect::from_min_max(pos2(ax1 - aw, cy - 14.0), pos2(ax1, cy + 14.0));
                    let qd = m.quants[m.quant].downloaded;
                    if let Some((start, size)) = m.download {
                        let done = ((now - start) * 0.085).min(size);
                        let f = (done / size) as f32;
                        let eta = ((size - done) / 0.085).max(0.0);
                        pk::mono(
                            &p,
                            pos2(ar.min.x, cy - 10.0),
                            Align2::LEFT_CENTER,
                            &format!("{:.0}%", f * 100.0),
                            11.5,
                            t.foreground,
                            50.0,
                        );
                        kit::text_at(
                            &p,
                            pos2(ar.max.x - 22.0, cy - 10.0),
                            Align2::RIGHT_CENTER,
                            &format!("87 MB/s · {:.0}s", eta),
                            10.5,
                            t.muted_foreground,
                            100.0,
                        );
                        pk::meter(
                            &p,
                            Rect::from_min_max(pos2(ar.min.x, cy + 4.0), pos2(ar.max.x - 22.0, cy + 9.0)),
                            f,
                            t.primary,
                            &t,
                        );
                        let xr = Rect::from_center_size(pos2(ar.max.x - 8.0, cy - 2.0), vec2(20.0, 20.0));
                        if pk::icon_btn(ui, xr, id.with(("cancel", mi)), LineIcon::X.into(), "Cancel download", None)
                            .clicked()
                        {
                            actions.push((mi, "cancel", 0));
                        }
                    } else if mi == self.loaded && qd {
                        kit::status_pill(&p, pos2(ar.max.x, cy), true, "Loaded", t.success, &t);
                    } else if qd {
                        if pk::small_btn(
                            ui,
                            Rect::from_min_max(pos2(ar.max.x - 80.0, ar.min.y), ar.max),
                            id.with(("load", mi)),
                            "Load",
                            Some(AppIcon::Play),
                            None,
                            None,
                        )
                        .clicked()
                        {
                            actions.push((mi, "load", 0));
                        }
                    } else if pk::small_btn(
                        ui,
                        Rect::from_min_max(pos2(ar.max.x - 110.0, ar.min.y), ar.max),
                        id.with(("dl", mi)),
                        &pk::gb(m.quants[m.quant].size_gb),
                        Some(UiIcon::Download.into()),
                        Some(t.primary),
                        None,
                    )
                    .clicked()
                    {
                        actions.push((mi, "download", 0));
                    }
                    // fit indicator
                    let size = m.quants[m.quant].size_gb;
                    let (fit_txt, fit_c) = if size + 0.6 <= VRAM_TOTAL {
                        ("Fits in VRAM", t.success)
                    } else if size <= VRAM_TOTAL + RAM_TOTAL * 0.6 {
                        ("Partial offload", t.warning)
                    } else {
                        ("Too large", t.destructive)
                    };
                    let fx = ar.min.x - 24.0;
                    let fr = kit::text_at(
                        &p,
                        pos2(fx, cy),
                        Align2::RIGHT_CENTER,
                        fit_txt,
                        11.5,
                        pk::tone_fg(&t, fit_c),
                        110.0,
                    );
                    p.circle_filled(pos2(fr.min.x - 8.0, cy), 3.0, fit_c);
                    // quants
                    let mut qx1 = fr.min.x - 28.0;
                    let mut qrects = Vec::new();
                    for (qi, qq) in m.quants.iter().enumerate().rev() {
                        let w = kit::text_w(&p, &qq.name, 10.5) + 14.0;
                        let r = Rect::from_min_max(pos2(qx1 - w, cy - 10.0), pos2(qx1, cy + 10.0));
                        qrects.push((qi, r));
                        qx1 = r.min.x - 4.0;
                    }
                    let name_w = (qx1 - x0 - 50.0).max(60.0);
                    for (qi, r) in qrects {
                        if r.min.x < x0 + 200.0 {
                            continue;
                        }
                        let qq = &m.quants[qi];
                        let on = qi == m.quant;
                        let resp = ui
                            .interact(r, id.with(("q", mi, qi)), Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        let h = kit::hover(ui, id.with(("qh", mi, qi)), resp.hovered());
                        let c = if on { t.primary } else { t.muted_foreground };
                        p.rect_filled(
                            r,
                            5.0,
                            if on { pk::soft(&t, t.primary, 1.4) } else { kit::hover_fill(&t, h * 1.5) },
                        );
                        p.rect_stroke(
                            r,
                            5.0,
                            Stroke::new(1.0, if on { with_alpha(t.primary, 0.5) } else { t.border }),
                            StrokeKind::Inside,
                        );
                        pk::mono(
                            &p,
                            r.center(),
                            Align2::CENTER_CENTER,
                            &qq.name,
                            10.5,
                            if on { pk::tone_fg(&t, c) } else { t.muted_foreground.lerp_to_gamma(t.foreground, h) },
                            r.width(),
                        );
                        if qq.downloaded {
                            p.circle_filled(pos2(r.max.x - 2.0, r.min.y + 2.0), 2.5, t.success);
                        }
                        kit::tip(
                            &resp,
                            &format!(
                                "{} · {}{}",
                                qq.name,
                                pk::gb(qq.size_gb),
                                if qq.downloaded { " · downloaded" } else { "" }
                            ),
                            None,
                        );
                        if resp.clicked() && m.download.is_none() {
                            actions.push((mi, "quant", qi));
                        }
                    }
                    // name block
                    let nx = tile.max.x + 12.0;
                    let nr =
                        kit::text_at(&p, pos2(nx, cy - 9.0), Align2::LEFT_CENTER, &m.name, 13.5, t.foreground, name_w);
                    let mut tx = nr.max.x + 8.0;
                    for tg in m.tags.iter().take(2) {
                        if tx + 50.0 > nx + name_w {
                            break;
                        }
                        let r = pk::tag(&p, pos2(tx, cy - 9.0), false, tg, t.muted_foreground, &t, false);
                        tx = r.max.x + 4.0;
                    }
                    kit::text_at(
                        &p,
                        pos2(nx, cy + 11.0),
                        Align2::LEFT_CENTER,
                        &format!("{} · {}k context", m.family, m.context / 1024),
                        11.5,
                        t.muted_foreground,
                        name_w,
                    );
                }
                if vis.is_empty() {
                    let (er, _) = ui.allocate_exact_size(vec2(w, 120.0), Sense::hover());
                    ui.painter().text(
                        er.center(),
                        Align2::CENTER_CENTER,
                        "No models match",
                        egui::FontId::proportional(13.0),
                        t.muted_foreground,
                    );
                }
            });
        });
        for (mi, a, qi) in actions {
            match a {
                "download" => {
                    let s = self.models[mi].quants[self.models[mi].quant].size_gb;
                    self.models[mi].download = Some((now, s));
                }
                "cancel" => {
                    self.models[mi].download = None;
                    toast(ui.ctx(), Toast::new("Download cancelled"));
                }
                "load" => {
                    self.loading = Some((mi, now));
                }
                "quant" => self.models[mi].quant = qi,
                _ => {}
            }
        }
    }
}
