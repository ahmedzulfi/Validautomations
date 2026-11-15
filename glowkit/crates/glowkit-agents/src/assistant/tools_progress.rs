use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Rect, Response, Sense, Stroke, Ui, Widget};
use glowkit_ai::Icon;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// Status of a [`PlanStep`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PlanStatus {
    /// Not started.
    #[default]
    Todo,
    /// The agent is working on it.
    Active,
    /// Finished.
    Done,
    /// Skipped or failed.
    Skipped,
}

/// One item of a [`PlanResult`].
#[derive(Clone, Debug, PartialEq)]
pub struct PlanStep {
    /// What the agent will do.
    pub title: String,
    /// Optional muted detail (shown while active or done).
    pub detail: String,
    /// Current status.
    pub status: PlanStatus,
}

impl PlanStep {
    /// A step that has not started.
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), detail: String::new(), status: PlanStatus::Todo }
    }
    /// Detail line.
    pub fn detail(mut self, d: impl Into<String>) -> Self {
        self.detail = d.into();
        self
    }
    /// Set the status.
    pub fn status(mut self, s: PlanStatus) -> Self {
        self.status = s;
        self
    }
}

/// The agent's to-do plan as a live card: a title with "3 of 6" and a progress bar that springs
/// forward, then each step with a status mark (empty ring, a spinning arc while active, a check
/// that draws itself when done). The active row is highlighted and shows its detail line; finished
/// titles soften. Click the header to collapse the list to just the progress bar.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{PlanResult, PlanStatus, PlanStep};
/// let steps = [PlanStep::new("Read the failing test").status(PlanStatus::Done), PlanStep::new("Fix the parser").status(PlanStatus::Active)];
/// ui.add(PlanResult::new("Fix flaky checkout test", &steps));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PlanResult<'a> {
    title: &'a str,
    steps: &'a [PlanStep],
    width: Option<f32>,
    salt: Option<egui::IdSalt>,
}

impl<'a> PlanResult<'a> {
    /// A plan titled `title`.
    pub fn new(title: &'a str, steps: &'a [PlanStep]) -> Self {
        Self { title, steps, width: None, salt: None }
    }
    /// Card width (default: available, at most 440).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }
}

impl Widget for PlanResult<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_plan");
        let width = self.width.unwrap_or_else(|| ui.available_width().min(440.0));
        let time = glowkit_core::time(ui);
        let e = since_change(ui, id, hash_str(self.title));
        keep_alive(ui, e, 1.2);
        let n = self.steps.len();
        let done = self.steps.iter().filter(|s| s.status == PlanStatus::Done).count();
        let active = self.steps.iter().any(|s| s.status == PlanStatus::Active);
        if active {
            glowkit_core::animate(ui);
        }
        let collapsed: bool = glowkit_core::state(ui, id.with("col"), || false);
        let ck = toggle(ui, id.with("ck"), collapsed);
        let prog = glowkit_core::anim::spring(ui, id.with("prog"), done as f32 / n.max(1) as f32, 140.0, 18.0);
        let status = format!("{done} of {n}");
        let col = if done == n { t.success } else { t.info };
        let head = ToolHead { icon: Icon::Lightbulb, title: self.title, subtitle: None, status: Some((&status, col)) };
        let steps = self.steps;
        let mut toggle_col = false;
        let rect = tool_card(ui, appear_curve(e), Some(head), width, |ui| {
            let w = ui.available_width();
            let (br, bresp) = ui.allocate_exact_size(vec2(w, 14.0), Sense::click());
            if bresp.clicked() {
                toggle_col = true;
            }
            let p = ui.painter();
            let bar = Rect::from_min_size(pos2(br.min.x, br.center().y - 2.5), vec2(w - 22.0, 5.0));
            p.rect_filled(bar, cr(3.0), well(&t));
            let fill = Rect::from_min_size(bar.min, vec2(bar.width() * prog.clamp(0.0, 1.0), bar.height()));
            if fill.width() > 1.0 {
                paint::gradient_rect(p, fill, 2.5, 0.0, &[mix(t.primary, t.accent, 0.2), t.accent]);
            }
            icon(
                p,
                if collapsed { Icon::ChevronDown } else { Icon::ChevronUp },
                pos2(br.max.x - 7.0, br.center().y),
                12.0,
                t.muted_foreground,
            );
            let vis = 1.0 - ck;
            if vis < 0.01 {
                return;
            }
            for (i, s) in steps.iter().enumerate() {
                let sid = id.with(("s", i));
                let act = s.status == PlanStatus::Active;
                let show_detail = (act || s.status == PlanStatus::Done) && !s.detail.is_empty();
                let dk = toggle(ui, sid.with("d"), act && !s.detail.is_empty());
                let h = (30.0 + 16.0 * dk) * vis;
                let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
                let k_in = ((e - 0.2 - i as f32 * 0.06) * 5.0).clamp(0.0, 1.0) * vis;
                let p = ui.painter().with_clip_rect(r.intersect(ui.clip_rect()));
                let ak = toggle(ui, sid.with("a"), act);
                if ak > 0.01 {
                    p.rect_filled(r.expand2(vec2(6.0, 0.0)), cr(9.0), with_alpha(well(&t), ak));
                }
                let c = pos2(r.min.x + 9.0, r.min.y + 15.0);
                let ce = since_change(ui, sid.with("st"), s.status as u64);
                match s.status {
                    PlanStatus::Todo => {
                        p.circle_stroke(c, 7.0, Stroke::new(1.3, with_alpha(t.muted_foreground, 0.55 * k_in)));
                    }
                    PlanStatus::Active => {
                        p.circle_stroke(c, 7.0, Stroke::new(1.3, with_alpha(t.accent, 0.2 * k_in)));
                        paint::arc(&p, c, 7.0, time * 5.0, time * 5.0 + 2.0, 1.6, with_alpha(t.accent, k_in));
                    }
                    PlanStatus::Done => {
                        keep_alive(ui, ce, 0.8);
                        check_disc(&p, c, 8.0, with_alpha(t.success, k_in), Color32::WHITE, (ce * 2.5).clamp(0.0, 1.0));
                    }
                    PlanStatus::Skipped => {
                        p.circle_stroke(c, 7.0, Stroke::new(1.3, with_alpha(t.muted_foreground, 0.4 * k_in)));
                        p.line_segment(
                            [c + vec2(-3.5, 0.0), c + vec2(3.5, 0.0)],
                            Stroke::new(1.4, with_alpha(t.muted_foreground, 0.6 * k_in)),
                        );
                    }
                }
                let tc = match s.status {
                    PlanStatus::Done | PlanStatus::Skipped => t.muted_foreground,
                    _ => t.foreground,
                };
                let tr = if act {
                    text_bold(&p, pos2(c.x + 16.0, c.y), Align2::LEFT_CENTER, &s.title, 13.0, with_alpha(tc, k_in))
                } else {
                    text(&p, pos2(c.x + 16.0, c.y), Align2::LEFT_CENTER, &s.title, 13.0, with_alpha(tc, k_in))
                };
                if s.status == PlanStatus::Skipped {
                    p.line_segment(
                        [pos2(tr.min.x, tr.center().y), pos2(tr.max.x, tr.center().y)],
                        Stroke::new(1.0, with_alpha(t.muted_foreground, 0.7)),
                    );
                }
                if show_detail && dk > 0.01 {
                    text(
                        &p,
                        pos2(c.x + 16.0, c.y + 17.0),
                        Align2::LEFT_CENTER,
                        &s.detail,
                        11.5,
                        with_alpha(t.muted_foreground, dk * k_in),
                    );
                }
            }
        });
        if toggle_col {
            glowkit_core::set_state(ui, id.with("col"), !collapsed);
        }
        ui.interact(rect, id.with("card"), Sense::hover())
    }
}

/// A horizontal tool-progress tracker: numbered nodes joined by a rail that fills as the agent
/// moves through its phases. `progress` is a float step index (e.g. `1.4` = second step 40% done),
/// so the rail glides continuously; the current node pulses with a soft ring and a sweeping arc,
/// finished nodes flip to checks, and the label under the current node gets an elapsed timer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let steps = ["Search", "Read 6 sources", "Draft", "Cite"];
/// ui.add(glowkit_agents::ProgressSteps::new(&steps, 1.4).elapsed(3.2));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ProgressSteps<'a> {
    steps: &'a [&'a str],
    progress: f32,
    elapsed: Option<f32>,
    width: Option<f32>,
    accent: Option<Color32>,
    salt: Option<egui::IdSalt>,
}

impl<'a> ProgressSteps<'a> {
    /// Steps with a float `progress` (0 = at the first step, `steps.len()` = all done).
    pub fn new(steps: &'a [&'a str], progress: f32) -> Self {
        Self { steps, progress, elapsed: None, width: None, accent: None, salt: None }
    }
    /// Seconds spent in the current step (shown under it).
    pub fn elapsed(mut self, secs: f32) -> Self {
        self.elapsed = Some(secs);
        self
    }
    /// Width (default: available, at most 620).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Rail and node color (default: theme accent).
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = Some(c);
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }
}

impl Widget for ProgressSteps<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_steps");
        let w = self.width.unwrap_or_else(|| ui.available_width().min(620.0));
        let (rect, resp) = ui.allocate_exact_size(vec2(w, 78.0), Sense::hover());
        let n = self.steps.len().max(1);
        let target = self.progress.clamp(0.0, n as f32);
        let prog = glowkit_core::anim::spring(ui, id.with("p"), target, 120.0, 20.0);
        let time = glowkit_core::time(ui);
        if target < n as f32 {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let acc = self.accent.unwrap_or(t.accent);
        let p = ui.painter();
        let pad = 40.0;
        let x = |i: f32| rect.min.x + pad + (w - pad * 2.0) * if n > 1 { i / (n - 1) as f32 } else { 0.5 };
        let y = rect.min.y + 22.0;
        p.line_segment([pos2(x(0.0), y), pos2(x((n - 1) as f32), y)], Stroke::new(3.0, well(&t)));
        let fill_to = prog.min((n - 1) as f32);
        if fill_to > 0.0 {
            let a = pos2(x(0.0), y);
            let b = pos2(x(fill_to), y);
            paint::gradient_polyline(p, &[a, b], false, 3.0, |u| mix(mix(acc, t.primary, 0.3), acc, u));
            // travelling glint at the head of the rail
            if target < n as f32 {
                paint::glow(p, b, 10.0, with_alpha(acc, 0.35));
            }
        }
        for (i, label) in self.steps.iter().enumerate() {
            let c = pos2(x(i as f32), y);
            let fi = i as f32;
            let done_k = (prog - fi - 0.85).clamp(0.0, 0.15) / 0.15;
            let current = prog >= fi - 0.05 && prog < fi + 1.0 && (prog.floor() as usize == i);
            let reached = prog >= fi - 0.02;
            if current && done_k < 1.0 {
                let pulse = (time * 2.2).sin() * 0.5 + 0.5;
                p.circle_filled(c, 15.0 + 3.0 * pulse, with_alpha(acc, 0.12 * (1.0 - done_k)));
            }
            let fill = if reached { mix(surface(&t), acc, done_k) } else { surface(&t) };
            p.circle_filled(c, 12.0, fill);
            p.circle_stroke(
                c,
                12.0,
                Stroke::new(1.5, if reached { acc } else { mix(t.border, t.muted_foreground, 0.3) }),
            );
            if done_k > 0.02 {
                check_disc(p, c, 12.0, with_alpha(acc, done_k), Color32::WHITE, done_k);
            } else if current {
                paint::arc(p, c, 12.0, time * 4.0, time * 4.0 + 1.6, 2.0, acc);
                text_bold(p, c, Align2::CENTER_CENTER, &(i + 1).to_string(), 11.0, acc);
            } else {
                text(p, c, Align2::CENTER_CENTER, &(i + 1).to_string(), 11.0, t.muted_foreground);
            }
            let lc = if reached { t.foreground } else { t.muted_foreground };
            if current && done_k < 0.5 {
                text_bold(p, pos2(c.x, y + 26.0), Align2::CENTER_CENTER, label, 12.0, lc);
            } else {
                text(p, pos2(c.x, y + 26.0), Align2::CENTER_CENTER, label, 12.0, lc);
            }
            if current && done_k < 0.5 {
                if let Some(el) = self.elapsed {
                    mono(p, pos2(c.x, y + 43.0), Align2::CENTER_CENTER, &format!("{el:.1}s"), 10.5, t.muted_foreground);
                }
            }
        }
        resp
    }
}
