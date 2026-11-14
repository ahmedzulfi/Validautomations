//! Collapsible model reasoning and chain-of-thought steps.

use crate::common;
use crate::icons::{paint_icon, Icon};
use crate::loader::paint_shimmer;
use crate::markdown::Markdown;
use egui::{pos2, vec2, Align2, CornerRadius, Rect, Response, Sense, Stroke, Ui, Widget};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Copy)]
struct ReasoningState {
    started: f64,
    finished: Option<f64>,
    was_streaming: bool,
}

/// A clickable header to collapse a section: icon, label (shimmering while `busy`), chevron.
/// Returns the header response (clicked = toggle).
fn header(ui: &mut Ui, id: egui::Id, icon: Icon, label: &str, busy: bool, open: f32, extra: Option<&str>) -> Response {
    let t = Theme::of(ui);
    let size = 13.5;
    let g = common::galley(ui, label, size, t.muted_foreground);
    let extra_w = extra.map(|e| common::galley(ui, e, 12.0, t.muted_foreground).size().x + 10.0).unwrap_or(0.0);
    let w = 18.0 + 8.0 + g.size().x + extra_w + 22.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 26.0), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let h = common::anim_bool(ui, id.with("hh"), resp.hovered(), 0.12);
    let col = t.muted_foreground.lerp_to_gamma(t.foreground, h * 0.8);
    let p = ui.painter();
    paint_icon(p, icon, Rect::from_center_size(pos2(rect.min.x + 8.0, rect.center().y), vec2(16.0, 16.0)), col);
    let tx = rect.min.x + 25.0;
    if busy {
        paint_shimmer(
            ui,
            pos2(tx, rect.center().y - g.size().y / 2.0),
            label,
            size,
            t.muted_foreground,
            t.foreground,
            1.6,
        );
    } else {
        ui.painter().galley(pos2(tx, rect.center().y - g.size().y / 2.0), g.clone(), col);
    }
    let mut x = tx + g.size().x + 8.0;
    if let Some(e) = extra {
        let r = common::text(
            ui.painter(),
            pos2(x, rect.center().y),
            Align2::LEFT_CENTER,
            e,
            12.0,
            with_alpha(t.muted_foreground, 0.75),
        );
        x = r.max.x + 6.0;
    }
    common::chevron(ui.painter(), pos2(x + 5.0, rect.center().y), 9.0, col, open);
    resp
}

/// The model's reasoning in a collapsible block. While streaming the header shimmers
/// "Thinking…" and the block stays open; when done it reads "Thought for 4s" and folds away.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let reasoning_so_far = "";
/// ui.add(glowkit_ai::Reasoning::new(reasoning_so_far).streaming(true));
/// ui.add(glowkit_ai::Reasoning::new("The user wants...").duration(4.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Reasoning<'a> {
    text: &'a str,
    streaming: bool,
    duration: Option<f32>,
    default_open: bool,
    auto_close: bool,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> Reasoning<'a> {
    /// Reasoning text (Markdown).
    pub fn new(text: &'a str) -> Self {
        Self { text, streaming: false, duration: None, default_open: false, auto_close: true, id_salt: None }
    }
    /// The model is still thinking.
    pub fn streaming(mut self, on: bool) -> Self {
        self.streaming = on;
        self
    }
    /// How long the model thought, in seconds. Measured automatically if not given.
    pub fn duration(mut self, secs: f32) -> Self {
        self.duration = Some(secs);
        self
    }
    /// Open when first shown (when not streaming).
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }
    /// Fold away automatically when streaming ends (default on).
    pub fn auto_close(mut self, on: bool) -> Self {
        self.auto_close = on;
        self
    }
    /// Stable id for the open state.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }
}

impl Widget for Reasoning<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = common::widget_id(ui, self.id_salt);
        let now = ui.input(|i| i.time);
        let mut st: ReasoningState = glowkit_core::state(ui, id.with("rs"), || ReasoningState {
            started: now,
            finished: None,
            was_streaming: self.streaming,
        });
        let user_key = id.with("user");
        if self.streaming && !st.was_streaming {
            st.started = now;
            st.finished = None;
            glowkit_core::set_state(ui, user_key, false);
        }
        let user_toggled: bool = glowkit_core::state(ui, user_key, || false);
        if self.streaming && !user_toggled {
            glowkit_core::set_state(ui, id.with("open"), true);
        }
        if !self.streaming && st.was_streaming {
            st.finished = Some(now);
            if self.auto_close && !user_toggled {
                glowkit_core::set_state(ui, id.with("open"), false);
            }
        }
        st.was_streaming = self.streaming;
        glowkit_core::set_state(ui, id.with("rs"), st);
        let secs = self.duration.unwrap_or_else(|| (st.finished.unwrap_or(now) - st.started) as f32);
        let label = if self.streaming {
            "Thinking…".to_owned()
        } else {
            format!("Thought for {}", common::fmt_secs(secs.max(1.0)))
        };
        let (_, open_now) = common::toggle_state(ui, id, self.default_open || self.streaming, false);
        let resp = ui
            .vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                let h = header(ui, id, Icon::Lightbulb, &label, self.streaming, open_now, None);
                if h.clicked() {
                    common::toggle_state(ui, id, false, true);
                    glowkit_core::set_state(ui, id.with("user"), true);
                }
                let (_, open) = common::toggle_state(ui, id, false, false);
                common::reveal(ui, id.with("body"), open, |ui| {
                    let avail = ui.available_rect_before_wrap();
                    let inner = Rect::from_min_max(avail.min + vec2(18.0, 2.0), avail.max);
                    let r = ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
                        ui.add(
                            Markdown::new(self.text)
                                .size(13.5)
                                .color(mix(t.muted_foreground, t.foreground, 0.12))
                                .caret(self.streaming),
                        );
                    });
                    let rr = r.response.rect;
                    ui.painter().line_segment(
                        [pos2(avail.min.x + 7.5, rr.min.y), pos2(avail.min.x + 7.5, rr.max.y)],
                        Stroke::new(1.5, t.border),
                    );
                    ui.add_space(4.0);
                });
            })
            .response;
        if self.streaming {
            glowkit_core::animate(ui);
        }
        resp
    }
}

/// Status of a [`ThoughtStep`] or [`crate::TaskItem`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StepStatus {
    /// Not started yet.
    #[default]
    Pending,
    /// In progress.
    Active,
    /// Finished.
    Complete,
}

/// One step of a [`ChainOfThought`].
#[derive(Clone, Debug)]
pub struct ThoughtStep<'a> {
    /// Main line, e.g. "Searching for recent benchmarks".
    pub label: &'a str,
    /// Optional muted second line.
    pub description: Option<&'a str>,
    /// Icon in the timeline.
    pub icon: Icon,
    /// Progress state.
    pub status: StepStatus,
    /// Small chips under the step (search results, files read...).
    pub chips: Vec<&'a str>,
}

impl<'a> ThoughtStep<'a> {
    /// A step with a label and icon.
    pub fn new(icon: Icon, label: &'a str) -> Self {
        Self { label, description: None, icon, status: StepStatus::Complete, chips: Vec::new() }
    }
    /// Muted second line.
    pub fn description(mut self, d: &'a str) -> Self {
        self.description = Some(d);
        self
    }
    /// Progress state.
    pub fn status(mut self, s: StepStatus) -> Self {
        self.status = s;
        self
    }
    /// Result chips shown under the step.
    pub fn chips(mut self, chips: &[&'a str]) -> Self {
        self.chips = chips.to_vec();
        self
    }
}

/// A collapsible timeline of reasoning steps (search, read, analyze...) joined by a vertical line;
/// the active step shimmers, finished ones are solid, pending ones are dimmed.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::{ChainOfThought, Icon, StepStatus, ThoughtStep};
/// ui.add(ChainOfThought::new(vec![
///     ThoughtStep::new(Icon::Search, "Searching the web").chips(&["rust-lang.org", "github.com"]),
///     ThoughtStep::new(Icon::Brain, "Comparing results").status(StepStatus::Active),
/// ]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ChainOfThought<'a> {
    steps: Vec<ThoughtStep<'a>>,
    title: &'a str,
    default_open: bool,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> ChainOfThought<'a> {
    /// A timeline of steps.
    pub fn new(steps: Vec<ThoughtStep<'a>>) -> Self {
        Self { steps, title: "Chain of thought", default_open: true, id_salt: None }
    }
    /// Header label.
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }
    /// Open when first shown (default on).
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }
    /// Stable id for the open state.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }
}

impl Widget for ChainOfThought<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = common::widget_id(ui, self.id_salt);
        let busy = self.steps.iter().any(|s| s.status == StepStatus::Active);
        let (_, open) = common::toggle_state(ui, id, self.default_open, false);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            let h = header(ui, id, Icon::Sparkle, self.title, busy, open, None);
            if h.clicked() {
                common::toggle_state(ui, id, self.default_open, true);
            }
            let (_, open) = common::toggle_state(ui, id, self.default_open, false);
            common::reveal(ui, id.with("body"), open, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let time = glowkit_core::time(ui);
                let n = self.steps.len();
                let mut prev_center: Option<egui::Pos2> = None;
                for (i, s) in self.steps.iter().enumerate() {
                    let w = ui.available_width();
                    let text_x = 30.0;
                    let label_col = match s.status {
                        StepStatus::Pending => with_alpha(t.muted_foreground, 0.6),
                        StepStatus::Active => t.foreground,
                        StepStatus::Complete => mix(t.foreground, t.muted_foreground, 0.2),
                    };
                    let lg = common::galley_wrapped(ui, s.label, 13.5, label_col, w - text_x);
                    let dg = s.description.map(|d| common::galley_wrapped(ui, d, 12.5, t.muted_foreground, w - text_x));
                    let chips_h = if s.chips.is_empty() { 0.0 } else { 30.0 };
                    let h = lg.size().y
                        + dg.as_ref().map(|g| g.size().y + 3.0).unwrap_or(0.0)
                        + chips_h
                        + if i + 1 < n { 16.0 } else { 2.0 };
                    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
                    let c = pos2(
                        rect.min.x + 10.0,
                        rect.min.y + lg.rows.first().map(|r| r.row.size.y / 2.0).unwrap_or(9.0),
                    );
                    let p = ui.painter();
                    if let Some(pc) = prev_center {
                        p.line_segment([pos2(pc.x, pc.y + 12.0), pos2(c.x, c.y - 12.0)], Stroke::new(1.0, t.border));
                    }
                    prev_center = Some(c);
                    let ir = Rect::from_center_size(c, vec2(15.0, 15.0));
                    match s.status {
                        StepStatus::Active => {
                            common::spinner(p, c, 7.5, 1.6, t.accent, time);
                        }
                        StepStatus::Complete => paint_icon(p, s.icon, ir, t.muted_foreground),
                        StepStatus::Pending => paint_icon(p, s.icon, ir, with_alpha(t.muted_foreground, 0.45)),
                    }
                    let mut y = rect.min.y;
                    if s.status == StepStatus::Active {
                        paint_shimmer(
                            ui,
                            pos2(rect.min.x + text_x, y),
                            s.label,
                            13.5,
                            mix(t.muted_foreground, t.foreground, 0.3),
                            t.foreground,
                            1.6,
                        );
                    } else {
                        ui.painter().galley(pos2(rect.min.x + text_x, y), lg.clone(), label_col);
                    }
                    y += lg.size().y;
                    if let Some(dg) = dg {
                        y += 3.0;
                        ui.painter().galley(pos2(rect.min.x + text_x, y), dg.clone(), t.muted_foreground);
                        y += dg.size().y;
                    }
                    if !s.chips.is_empty() {
                        let mut x = rect.min.x + text_x;
                        for chip in &s.chips {
                            let g = common::galley(ui, *chip, 11.5, t.muted_foreground);
                            let r = Rect::from_min_size(pos2(x, y + 7.0), vec2(g.size().x + 26.0, 21.0));
                            if r.max.x > rect.max.x {
                                break;
                            }
                            let p = ui.painter();
                            p.rect_filled(r, CornerRadius::same(6), common::raised(&t));
                            p.rect_stroke(
                                r,
                                CornerRadius::same(6),
                                Stroke::new(1.0, t.border),
                                egui::StrokeKind::Inside,
                            );
                            paint_icon(
                                p,
                                Icon::Globe,
                                Rect::from_center_size(pos2(r.min.x + 11.0, r.center().y), vec2(11.0, 11.0)),
                                t.muted_foreground,
                            );
                            p.galley(pos2(r.min.x + 19.0, r.center().y - g.size().y / 2.0), g, t.muted_foreground);
                            x = r.max.x + 6.0;
                        }
                    }
                }
                ui.add_space(4.0);
            });
        })
        .response
    }
}
