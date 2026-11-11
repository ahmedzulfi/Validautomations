use crate::util;
use egui::{pos2, vec2, Color32, Rect, Response, Sense, Ui, Widget};
use glowkit_core::{Rng, Theme};

/// Text drawn as thin horizontal rows that each jitter sideways every frame, like a noisy
/// analog signal. The fuzz gets stronger while hovered.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::FuzzyText::new("404").size(140.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct FuzzyText {
    text: String,
    size: f32,
    color: Option<Color32>,
    bold: bool,
    base: f32,
    hover: f32,
    range: Option<f32>,
    row: f32,
    fps: f32,
}

impl FuzzyText {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), size: 96.0, color: None, bold: true, base: 0.18, hover: 0.5, range: None, row: 2.0, fps: 60.0 }
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
    /// Fuzz amount at rest, 0..1 (default 0.18).
    pub fn base_intensity(mut self, k: f32) -> Self {
        self.base = k.clamp(0.0, 1.0);
        self
    }
    /// Fuzz amount while hovered, 0..1 (default 0.5).
    pub fn hover_intensity(mut self, k: f32) -> Self {
        self.hover = k.clamp(0.0, 1.0);
        self
    }
    /// Maximum sideways displacement in points at intensity 1 (default 0.3 x font size).
    pub fn fuzz_range(mut self, points: f32) -> Self {
        self.range = Some(points);
        self
    }
    /// Height of each jittering row in points (default 2).
    pub fn row_height(mut self, points: f32) -> Self {
        self.row = points.max(1.0);
        self
    }
    /// How many times per second the noise changes (default 60).
    pub fn fps(mut self, fps: f32) -> Self {
        self.fps = fps.max(1.0);
        self
    }
}

impl Widget for FuzzyText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let painter = ui.painter().clone();
        let galley = util::layout(&painter, self.text.clone(), &font);
        let range = self.range.unwrap_or(self.size * 0.3);
        let (rect, resp) = ui.allocate_exact_size(galley.size() + vec2(bold + range, 0.0), Sense::hover());
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let hov = util::ramp(ui, resp.id.with("fuzz"), resp.hovered(), 0.25);
        let intensity = self.base + (self.hover - self.base) * hov;
        let color = self.color.unwrap_or(t.foreground);
        let tick = (glowkit_core::time(ui) * self.fps) as u64;
        let mut rng = Rng::new(tick ^ resp.id.value());
        let origin = pos2(rect.min.x + range / 2.0, rect.min.y);
        // only rows that actually contain ink
        let ink = galley.mesh_bounds.translate(origin.to_vec2());
        let mut y = ink.min.y.floor();
        while y < ink.max.y {
            let band = Rect::from_min_max(pos2(rect.min.x - range, y), pos2(rect.max.x + range, y + self.row));
            let dx = (rng.f32() - 0.5) * intensity * range;
            let p = painter.with_clip_rect(band.intersect(ui.clip_rect()));
            util::paint_galley(&p, origin + vec2(dx, 0.0), &galley, color, bold);
            y += self.row;
        }
        resp
    }
}
