use egui::{Color32, FontId, Response, Sense, Ui, Widget};
use glowkit_core::{color::gradient_cyclic, paint, text::Glyphs, Theme};

/// Text filled with a flowing multi-color gradient, optionally with a matching animated border.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_core::color::hex;
/// ui.add(glowkit_text::GradientText::new("Gradient text")
///     .colors(&[hex("#40ffaa"), hex("#4079ff"), hex("#ff40d9")])
///     .size(40.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct GradientText {
    text: String,
    size: f32,
    colors: Vec<Color32>,
    speed: f32,
    angle: f32,
    border: bool,
    bold: bool,
}

impl GradientText {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), size: 32.0, colors: Vec::new(), speed: 6.0, angle: 0.0, border: false, bold: false }
    }
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Gradient stops (wraps around seamlessly). Defaults to the theme gradient.
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    /// Seconds for one full cycle. `0` = static.
    pub fn speed(mut self, seconds: f32) -> Self {
        self.speed = seconds.max(0.0);
        self
    }
    /// Gradient direction in degrees (0 = horizontal).
    pub fn angle(mut self, degrees: f32) -> Self {
        self.angle = degrees;
        self
    }
    /// Draw a pill border with the same animated gradient around the text.
    pub fn border(mut self, border: bool) -> Self {
        self.border = border;
        self
    }
    /// Fake-bold by painting the text twice with a hairline offset.
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
}

impl Widget for GradientText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let colors = if self.colors.is_empty() { vec![t.gradient[0], t.gradient[1], t.gradient[2]] } else { self.colors };
        let glyphs = Glyphs::layout(ui.painter(), &self.text, FontId::proportional(self.size), Some(ui.available_width()));
        let pad = if self.border { egui::vec2(self.size * 0.6, self.size * 0.25) } else { egui::Vec2::ZERO };
        let (rect, resp) = ui.allocate_exact_size(glyphs.size + pad * 2.0, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let time = glowkit_core::time(ui);
        let shift = if self.speed > 0.0 { time / self.speed } else { 0.0 };
        let a = self.angle.to_radians();
        let dir = egui::vec2(a.cos(), a.sin());
        let w = rect.width().max(1.0);
        let painter = ui.painter();
        let f = |p: egui::Pos2| {
            let u = (p - rect.min).dot(dir) / w;
            gradient_cyclic(&colors, u * 0.5 - shift)
        };
        if self.border {
            let r = rect.height() / 2.0;
            paint::gradient_border(painter, rect, r, 1.5, |s| {
                // map perimeter param back to an x position for a consistent flow
                gradient_cyclic(&colors, s - shift)
            });
            painter.rect_filled(rect.shrink(1.5), egui::CornerRadius::same((r - 1.5).max(0.0) as u8), t.background.gamma_multiply(0.6));
        }
        let pos = rect.min + pad;
        if self.bold {
            paint::galley_with(painter, pos + egui::vec2(0.6, 0.0), &glyphs.galley, f);
        }
        paint::galley_with(painter, pos, &glyphs.galley, f);
        if self.speed > 0.0 {
            glowkit_core::animate(ui);
        }
        resp
    }
}
