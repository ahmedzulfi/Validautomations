use crate::util;
use egui::{vec2, Color32, Response, Sense, Ui, Widget};
use glowkit_core::{color::gradient_cyclic, Theme};
use std::f32::consts::TAU;

/// Letters bob up and down on a travelling sine wave, optionally tinted by a gradient that rides
/// along with the crest.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::WaveText::new("Riding the wave").size(48.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct WaveText {
    text: String,
    size: f32,
    color: Option<Color32>,
    colors: Vec<Color32>,
    bold: bool,
    amplitude: Option<f32>,
    period: f32,
    wavelength: f32,
    lift_scale: f32,
}

impl WaveText {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 44.0,
            color: None,
            colors: Vec::new(),
            bold: true,
            amplitude: None,
            period: 1.6,
            wavelength: 12.0,
            lift_scale: 0.08,
        }
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Text color (defaults to the theme foreground). Ignored when [`Self::colors`] is set.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Gradient that flows along the wave (e.g. `&theme.gradient`).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    /// Fake-bold the text (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Peak height in points (default 0.2 x font size).
    pub fn amplitude(mut self, points: f32) -> Self {
        self.amplitude = Some(points);
        self
    }
    /// Seconds per wave cycle.
    pub fn period(mut self, seconds: f32) -> Self {
        self.period = seconds.max(0.05);
        self
    }
    /// Letters per wave length.
    pub fn wavelength(mut self, letters: f32) -> Self {
        self.wavelength = letters.max(1.0);
        self
    }
    /// Extra scale at the crest (0 = none, default 0.08).
    pub fn crest_scale(mut self, k: f32) -> Self {
        self.lift_scale = k.max(0.0);
        self
    }
}

impl Widget for WaveText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let glyphs = crate::util::glyphs(ui.painter(), &self.text, font.clone(), Some(ui.available_width()));
        let amp = self.amplitude.unwrap_or(self.size * 0.2);
        let (rect, resp) = ui.allocate_exact_size(glyphs.size + vec2(bold, amp * 2.0), Sense::hover());
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let time = glowkit_core::time(ui);
        let painter = ui.painter();
        let color = self.color.unwrap_or(t.foreground);
        for g in &glyphs.glyphs {
            if g.ch.is_whitespace() {
                continue;
            }
            let phase = time / self.period * TAU - g.index as f32 / self.wavelength * TAU;
            let s = phase.sin();
            let center = rect.min + vec2(0.0, amp) + g.offset + g.size / 2.0 + vec2(0.0, -s * amp);
            let scale = 1.0 + self.lift_scale * s.max(0.0);
            let c = if self.colors.is_empty() {
                color
            } else {
                gradient_cyclic(&self.colors, g.index as f32 / self.wavelength * 0.5 - time / self.period * 0.5)
            };
            util::paint_char_xf(painter, g.ch, &font, center, scale, 0.0, c, bold);
        }
        resp
    }
}
