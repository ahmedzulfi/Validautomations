use crate::util;
use egui::{pos2, vec2, Color32, CornerRadius, Rect, Response, Sense, Ui, Widget};
use glowkit_core::{ease, Clock, Easing, Theme, Trigger};

/// A solid colored box sweeps across the text and slides away, uncovering text that rises into
/// place behind it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::BoxReveal::new("Magic UI.").size(64.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoxReveal {
    text: String,
    size: f32,
    color: Option<Color32>,
    box_color: Option<Color32>,
    bold: bool,
    duration: f32,
    delay: f32,
    trigger: Trigger,
}

impl BoxReveal {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), size: 48.0, color: None, box_color: None, bold: true, duration: 0.9, delay: 0.0, trigger: Trigger::OnAppear }
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Text color (defaults to the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Box color (defaults to the theme accent).
    pub fn box_color(mut self, c: Color32) -> Self {
        self.box_color = Some(c);
        self
    }
    /// Fake-bold the text (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Seconds for the whole reveal.
    pub fn duration(mut self, seconds: f32) -> Self {
        self.duration = seconds.max(0.05);
        self
    }
    /// Seconds to wait before starting (useful to cascade several lines).
    pub fn delay(mut self, seconds: f32) -> Self {
        self.delay = seconds.max(0.0);
        self
    }
    /// When the reveal plays.
    pub fn trigger(mut self, t: Trigger) -> Self {
        self.trigger = t;
        self
    }
}

impl Widget for BoxReveal {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let painter = ui.painter().clone();
        let galley = util::layout(&painter, self.text.clone(), &font);
        let (rect, resp) = ui.allocate_exact_size(galley.size() + vec2(bold, 0.0), Sense::click());
        // delays up to 1 s share one cycle length, so looping lines stay in step
        let total = self.duration + self.delay.max(1.0);
        let e = Clock::triggered(ui, resp.id, self.trigger, total, resp.hovered(), resp.clicked());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let p = ((e - self.delay) / self.duration).clamp(0.0, 1.0);
        // phase 1 (0..0.45): box grows from the left; phase 2 (0.45..1): box slides off to the right
        let grow = ease(Easing::OutCubic, (p / 0.45).min(1.0));
        let leave = ease(Easing::InOutCubic, ((p - 0.45) / 0.55).clamp(0.0, 1.0));
        let w = rect.width();
        let box_rect = Rect::from_min_max(pos2(rect.min.x + w * leave, rect.min.y + self.size * 0.08), pos2(rect.min.x + w * grow, rect.max.y - self.size * 0.02));
        // text rises in once the box covers it
        let tp = ease(Easing::OutCubic, ((p - 0.4) / 0.6).clamp(0.0, 1.0));
        if tp > 0.0 {
            let clip = painter.with_clip_rect(rect.intersect(ui.clip_rect()));
            let pos = rect.min + vec2(0.0, (1.0 - tp) * rect.height() * 0.6);
            util::paint_galley(&clip, pos, &galley, self.color.unwrap_or(t.foreground).gamma_multiply(tp), bold);
        }
        if p < 1.0 && box_rect.width() > 0.5 {
            painter.rect_filled(box_rect, CornerRadius::same(2), self.box_color.unwrap_or(t.accent));
        }
        resp
    }
}
