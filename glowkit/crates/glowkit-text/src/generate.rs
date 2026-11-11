use crate::util;
use egui::{Color32, Response, Sense, Ui, Widget};
use glowkit_core::{anim::stagger, ease, text::Glyphs, Clock, Easing, Theme, Trigger};

/// A paragraph whose words appear one after another, each fading in out of a blur, like text
/// streaming from an AI model.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::TextGenerate::new("Oxygen gets you high. In a catastrophic emergency...").max_width(600.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TextGenerate {
    text: String,
    size: f32,
    color: Option<Color32>,
    bold: bool,
    max_width: Option<f32>,
    delay: f32,
    duration: f32,
    blur: bool,
    trigger: Trigger,
}

impl TextGenerate {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 24.0,
            color: None,
            bold: false,
            max_width: None,
            delay: 0.09,
            duration: 0.55,
            blur: true,
            trigger: Trigger::OnAppear,
        }
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
    /// Fake-bold the text.
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Wrap width in points (defaults to the available width).
    pub fn max_width(mut self, w: f32) -> Self {
        self.max_width = Some(w);
        self
    }
    /// Seconds between words.
    pub fn delay(mut self, seconds: f32) -> Self {
        self.delay = seconds.max(0.0);
        self
    }
    /// Seconds each word takes to appear.
    pub fn duration(mut self, seconds: f32) -> Self {
        self.duration = seconds.max(0.01);
        self
    }
    /// Fade in from a blur (default) or only fade.
    pub fn blur(mut self, blur: bool) -> Self {
        self.blur = blur;
        self
    }
    /// When the text generates.
    pub fn trigger(mut self, t: Trigger) -> Self {
        self.trigger = t;
        self
    }
}

impl Widget for TextGenerate {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let wrap = self.max_width.unwrap_or(ui.available_width()).min(ui.available_width());
        let glyphs = Glyphs::layout(ui.painter(), &self.text, font.clone(), Some(wrap));
        let (rect, resp) = ui.allocate_exact_size(glyphs.size + egui::vec2(bold, 0.0), Sense::click());
        let words = glyphs.word_count();
        let total = self.duration + self.delay * words.saturating_sub(1) as f32;
        let e = Clock::triggered(ui, resp.id, self.trigger, total, resp.hovered(), resp.clicked());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter();
        let color = self.color.unwrap_or(t.foreground);
        for g in &glyphs.glyphs {
            if g.ch.is_whitespace() {
                continue;
            }
            let p = ease(Easing::OutQuad, stagger(e, g.word, self.delay, self.duration));
            if p <= 0.0 {
                continue;
            }
            let gal = util::layout_char(painter, g.ch, &font);
            let b = if self.blur { (1.0 - p) * self.size * 0.3 } else { 0.0 };
            util::paint_blurred(painter, rect.min + g.offset, &gal, color.gamma_multiply(p), b, bold);
        }
        resp
    }
}
