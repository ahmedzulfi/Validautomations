use crate::util;
use egui::{pos2, vec2, Color32, Response, Sense, Ui, Widget};
use glowkit_core::{ease, Clock, Easing, Theme};

/// Cycles through words by melting one into the next: the old word dissolves into a blur while
/// the new one condenses out of it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::MorphingText::new(["Hello", "Morphing", "Text"]).size(64.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct MorphingText {
    texts: Vec<String>,
    size: f32,
    color: Option<Color32>,
    bold: bool,
    morph: f32,
    hold: f32,
    blur: Option<f32>,
}

impl MorphingText {
    /// Cycle through `texts`.
    pub fn new<S: Into<String>>(texts: impl IntoIterator<Item = S>) -> Self {
        Self { texts: texts.into_iter().map(Into::into).collect(), size: 64.0, color: None, bold: true, morph: 1.1, hold: 1.5, blur: None }
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
    /// Fake-bold the text (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Seconds a morph takes.
    pub fn morph_time(mut self, seconds: f32) -> Self {
        self.morph = seconds.max(0.05);
        self
    }
    /// Seconds each word holds still.
    pub fn hold(mut self, seconds: f32) -> Self {
        self.hold = seconds.max(0.0);
        self
    }
    /// Peak blur radius in points (default 0.16 x font size).
    pub fn blur(mut self, points: f32) -> Self {
        self.blur = Some(points);
        self
    }
}

impl Widget for MorphingText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let painter = ui.painter().clone();
        let galleys: Vec<_> = self.texts.iter().map(|s| util::layout(&painter, s.clone(), &font)).collect();
        let w = galleys.iter().map(|g| g.size().x).fold(0.0, f32::max) + bold;
        let blur = self.blur.unwrap_or(self.size * 0.16);
        let (rect, resp) = ui.allocate_exact_size(vec2(w + blur * 2.0, self.size * 1.25), Sense::hover());
        glowkit_core::animate(ui);
        if galleys.is_empty() || !ui.is_rect_visible(rect) {
            return resp;
        }
        let n = galleys.len();
        let cycle = self.hold + self.morph;
        let e = Clock::elapsed(ui, resp.id);
        let k = (e / cycle) as usize;
        let local = e - k as f32 * cycle;
        let cur = k % n;
        let next = (k + 1) % n;
        let color = self.color.unwrap_or(t.foreground);
        let draw = |i: usize, alpha: f32, b: f32| {
            let g = &galleys[i];
            let pos = pos2(rect.center().x - (g.size().x + bold) / 2.0, rect.center().y - g.size().y / 2.0);
            util::paint_blurred(&painter, pos, g, color.gamma_multiply(alpha), b, bold);
        };
        if local < self.hold {
            draw(cur, 1.0, 0.0);
        } else {
            let p = ease(Easing::InOutSine, (local - self.hold) / self.morph);
            // the outgoing word blurs quickly and fades; the incoming one sharpens late
            let out_a = (1.0 - p).powf(0.6);
            let in_a = p.powf(0.6);
            draw(cur, out_a, blur * p.powf(0.7));
            draw(next, in_a, blur * (1.0 - p).powf(0.7));
        }
        resp
    }
}
