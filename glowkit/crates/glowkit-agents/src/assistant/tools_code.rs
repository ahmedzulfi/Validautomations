use super::common::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Ui};
use glowkit_ai::{CodeBlock, Icon};
use glowkit_core::color::{hex, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Copy, Default)]
struct RunState {
    started: Option<f64>,
    runs: u32,
}

/// A code tool result with a Run button: syntax-highlighted source (glowkit-ai's code block), and
/// a console that slides open under it when run. Output lines stream in one by one behind a
/// spinner, then the run settles with an exit chip (`✓ exit 0 · 0.84s`). Run again to replay.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let out = ["collecting 3 tests", "3 passed in 0.84s"];
/// glowkit_agents::CodeResult::new("python", "print('hi')", &out).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct CodeResult<'a> {
    lang: &'a str,
    code: &'a str,
    output: &'a [&'a str],
    filename: Option<&'a str>,
    line_time: f32,
    width: Option<f32>,
    run_now: bool,
    salt: Option<egui::IdSalt>,
}

impl<'a> CodeResult<'a> {
    /// `code` in `lang` that prints `output` when run.
    pub fn new(lang: &'a str, code: &'a str, output: &'a [&'a str]) -> Self {
        Self { lang, code, output, filename: None, line_time: 0.35, width: None, run_now: false, salt: None }
    }
    /// File name shown in the header.
    pub fn filename(mut self, f: &'a str) -> Self {
        self.filename = Some(f);
        self
    }
    /// Seconds between streamed output lines (default 0.35).
    pub fn line_time(mut self, s: f32) -> Self {
        self.line_time = s.max(0.02);
        self
    }
    /// Card width (default: available, at most 560).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Start a run this frame (as if Run was clicked).
    pub fn run_now(mut self, on: bool) -> Self {
        self.run_now = on;
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the card. Returns the card response and the Run button rect.
    pub fn show(self, ui: &mut Ui) -> (Response, Rect) {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_code");
        let width = self.width.unwrap_or_else(|| ui.available_width().min(560.0));
        let now = ui.input(|i| i.time);
        let mut st: RunState = glowkit_core::state(ui, id.with("run"), RunState::default);
        let e = since_change(ui, id, hash_str(self.code));
        keep_alive(ui, e, 1.0);
        let total = self.output.len() as f32 * self.line_time + 0.5;
        let run_e = st.started.map(|s| (now - s) as f32);
        let running = run_e.is_some_and(|r| r < total);
        if run_e.is_some_and(|r| r < total + 1.0) {
            ui.ctx().request_repaint();
        }
        let mut start = self.run_now && !running;
        let console_k = toggle(ui, id.with("con"), st.started.is_some());
        let mut run_rect = Rect::NOTHING;
        let status = match (st.started.is_some(), running) {
            (false, _) => ("ready", t.muted_foreground),
            (true, true) => ("running", t.info),
            _ => ("exit 0", t.success),
        };
        let head = ToolHead { icon: Icon::Terminal, title: self.filename.unwrap_or("run_code"), subtitle: Some(self.lang), status: Some(status) };
        let time = glowkit_core::time(ui);
        let code = self.code;
        let lang = self.lang;
        let output = self.output;
        let line_time = self.line_time;
        let rect = tool_card(ui, appear_curve(e), Some(head), width, |ui| {
            let w = ui.available_width();
            ui.add(CodeBlock::new(code).language(lang).size(12.5).header(false).line_numbers(true));
            // action row
            let (r, _) = ui.allocate_exact_size(vec2(w, 32.0), Sense::hover());
            let label = if running { "Running" } else if st.started.is_some() { "Run again" } else { "Run" };
            let bw = button_w(ui, label, 32.0, true);
            let br = Rect::from_min_size(pos2(r.max.x - bw, r.min.y), vec2(bw, 32.0));
            run_rect = br;
            if running {
                let p = ui.painter();
                p.rect_filled(br, cr(16.0), well(&t));
                spinner(p, pos2(br.min.x + 18.0, br.center().y), 6.0, time, t.foreground);
                text(p, pos2(br.min.x + 30.0, br.center().y), Align2::LEFT_CENTER, "Running", 12.5, t.foreground);
            } else if button(ui, id.with("run_b"), br, label, Btn::Primary, Some(Icon::Play)).clicked() {
                start = true;
            }
            text(ui.painter(), pos2(r.min.x, r.center().y), Align2::LEFT_CENTER, "Sandboxed · no network", 11.5, t.muted_foreground);
            // console
            if console_k > 0.01 {
                let shown = run_e.map(|r| ((r / line_time) as usize).min(output.len())).unwrap_or(0);
                let line_h = 18.0;
                let full_h = 20.0 + output.len().max(1) as f32 * line_h + 26.0;
                let (cr_, _) = ui.allocate_exact_size(vec2(w, full_h * console_k), Sense::hover());
                let p = ui.painter().with_clip_rect(cr_.intersect(ui.clip_rect()));
                let box_r = Rect::from_min_size(cr_.min, vec2(w, full_h));
                let bg = if t.dark { hex("#0B0B0E") } else { hex("#18181B") };
                p.rect_filled(box_r, cr(10.0), bg);
                let fg = hex("#E4E4E7");
                let mut y = box_r.min.y + 14.0;
                mono(&p, pos2(box_r.min.x + 12.0, y), Align2::LEFT_CENTER, "$ run", 11.5, with_alpha(fg, 0.5));
                y += line_h;
                for (i, l) in output.iter().enumerate().take(shown) {
                    let k = run_e.map(|r| ((r - i as f32 * line_time) * 6.0).clamp(0.0, 1.0)).unwrap_or(1.0);
                    let col = if l.contains("passed") || l.starts_with('✓') {
                        hex("#4ADE80")
                    } else if l.contains("error") || l.contains("FAIL") {
                        hex("#F87171")
                    } else {
                        fg
                    };
                    mono(&p, pos2(box_r.min.x + 12.0 + (1.0 - k) * 6.0, y), Align2::LEFT_CENTER, l, 11.5, with_alpha(col, k));
                    y += line_h;
                }
                if running {
                    let blink = ((time * 2.0).fract() < 0.5) as u8 as f32;
                    p.rect_filled(Rect::from_min_size(pos2(box_r.min.x + 12.0, y - 7.0), vec2(7.0, 14.0)), cr(1.0), with_alpha(fg, blink));
                } else if let Some(r) = run_e {
                    let k = ((r - total) * 4.0).clamp(0.0, 1.0);
                    let s = format!("exit 0 · {:.2}s", total - 0.5);
                    let g = p.layout_no_wrap(s.clone(), FontId::monospace(10.5), hex("#4ADE80"));
                    let chip = Rect::from_min_size(pos2(box_r.max.x - 12.0 - g.size().x - 26.0, box_r.max.y - 26.0), vec2(g.size().x + 26.0, 18.0));
                    p.rect_filled(chip, cr(9.0), with_alpha(hex("#22C55E"), 0.16 * k));
                    check_disc(&p, pos2(chip.min.x + 10.0, chip.center().y), 5.0, Color32::TRANSPARENT, with_alpha(hex("#4ADE80"), k), k);
                    p.galley(pos2(chip.min.x + 19.0, chip.center().y - g.size().y / 2.0), g, with_alpha(hex("#4ADE80"), k));
                }
            }
        });
        if start {
            st = RunState { started: Some(now), runs: st.runs + 1 };
            glowkit_core::set_state(ui, id.with("run"), st);
        }
        (ui.interact(rect, id.with("card"), Sense::hover()), run_rect)
    }
}
