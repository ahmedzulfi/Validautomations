use egui::{Color32, FontId, Rect, Response, Sense, Ui, Widget};
use glowkit_core::{paint, text::Glyphs, Theme};

/// Text with a band of light sweeping across it, like light catching metal.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::ShinyText::new("Shimmering text").size(32.0).speed(2.5));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ShinyText {
    text: String,
    size: f32,
    color: Option<Color32>,
    shine: Option<Color32>,
    speed: f32,
    band: f32,
    angle: f32,
    disabled: bool,
}

impl ShinyText {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), size: 28.0, color: None, shine: None, speed: 3.0, band: 0.18, angle: 20.0, disabled: false }
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Base (dim) text color. Defaults to the theme's muted foreground.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Color of the moving highlight. Defaults to the theme's foreground.
    pub fn shine_color(mut self, c: Color32) -> Self {
        self.shine = Some(c);
        self
    }
    /// Seconds per sweep.
    pub fn speed(mut self, seconds: f32) -> Self {
        self.speed = seconds.max(0.1);
        self
    }
    /// Width of the highlight band as a fraction of the text width.
    pub fn band(mut self, fraction: f32) -> Self {
        self.band = fraction.clamp(0.02, 1.0);
        self
    }
    /// Tilt of the highlight band in degrees.
    pub fn angle(mut self, degrees: f32) -> Self {
        self.angle = degrees;
        self
    }
    /// Stop the animation.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Widget for ShinyText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = FontId::proportional(self.size);
        let glyphs = Glyphs::layout(ui.painter(), &self.text, font, Some(ui.available_width()));
        let (rect, resp) = ui.allocate_exact_size(glyphs.size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let base = self.color.unwrap_or(t.muted_foreground);
        let shine = self.shine.unwrap_or(t.foreground);
        let painter = ui.painter();
        if self.disabled {
            painter.galley(rect.min, glyphs.galley.clone(), base);
            return resp;
        }
        let time = glowkit_core::time(ui);
        let phase = (time / self.speed).fract();
        // the band travels from fully left of the text to fully right of it
        let travel = 1.0 + self.band * 2.0;
        let center = -self.band + phase * travel;
        let slope = self.angle.to_radians().tan();
        let band = self.band;
        let r: Rect = rect;
        paint::galley_with(painter, rect.min, &glyphs.galley, |p| {
            let u = (p.x - r.min.x + (p.y - r.center().y) * slope) / r.width().max(1.0);
            let d = ((u - center) / band).abs();
            let k = (1.0 - d).clamp(0.0, 1.0);
            base.lerp_to_gamma(shine, k * k * (3.0 - 2.0 * k))
        });
        glowkit_core::animate(ui);
        resp
    }
}
