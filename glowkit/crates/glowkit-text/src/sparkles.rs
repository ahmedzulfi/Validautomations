use crate::util;
use egui::epaint::Mesh;
use egui::{pos2, vec2, Color32, Painter, Pos2, Response, Sense, Ui, Widget};
use glowkit_core::{paint, rng::hash01, Rng, Theme};
use std::f32::consts::{PI, TAU};

/// A four-pointed star with concave (astroid) sides, the classic "magic" sparkle.
pub(crate) fn sparkle_star(painter: &Painter, center: Pos2, radius: f32, angle: f32, color: Color32) {
    if radius < 0.3 || color.a() == 0 {
        return;
    }
    let n = 40;
    let mut m = Mesh::default();
    m.colored_vertex(center, color);
    let (s, c) = angle.sin_cos();
    for i in 0..n {
        let t = i as f32 / n as f32 * TAU;
        let (x, y) = (t.cos(), t.sin());
        // astroid: x = cos^3, y = sin^3
        let p = vec2(x * x * x, y * y * y) * radius;
        let p = vec2(p.x * c - p.y * s, p.x * s + p.y * c);
        m.colored_vertex(center + p, color);
    }
    for i in 0..n {
        m.add_triangle(0, 1 + i as u32, 1 + ((i + 1) % n) as u32);
    }
    painter.add(egui::Shape::mesh(m));
}

/// Text surrounded by little star sparkles that pop in, twinkle and fade at random spots.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::SparklesText::new("Magic UI").size(64.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SparklesText {
    text: String,
    size: f32,
    color: Option<Color32>,
    colors: Vec<Color32>,
    count: usize,
    bold: bool,
}

impl SparklesText {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), size: 56.0, color: None, colors: Vec::new(), count: 10, bold: true }
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
    /// Sparkle colors (defaults to the first two theme gradient colors).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    /// Number of sparkles alive at once.
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.min(64);
        self
    }
    /// Fake-bold the text (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
}

impl Widget for SparklesText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let painter = ui.painter().clone();
        let galley = util::layout(&painter, self.text.clone(), &font);
        let margin = vec2(self.size * 0.3, self.size * 0.25);
        let (rect, resp) = ui.allocate_exact_size(galley.size() + vec2(bold, 0.0) + margin * 2.0, Sense::hover());
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let colors = if self.colors.is_empty() { vec![t.gradient[0], t.gradient[1]] } else { self.colors };
        let text_rect = rect.shrink2(margin);
        let ink = galley.mesh_bounds.translate(text_rect.min.to_vec2());
        util::paint_galley(&painter, text_rect.min, &galley, self.color.unwrap_or(t.foreground), bold);
        let time = glowkit_core::time(ui);
        let seed = resp.id.value();
        for i in 0..self.count {
            let life = 0.9 + 0.8 * hash01(seed ^ (i as u64 * 31 + 7));
            let off = hash01(seed ^ (i as u64 * 131 + 3)) * life;
            let tt = time + off;
            let gen = (tt / life).floor() as u64;
            let local = tt / life - gen as f32;
            let mut r = Rng::new(seed ^ (i as u64).wrapping_mul(0x9E37) ^ gen.wrapping_mul(0x85EB_CA6B));
            let pos = pos2(
                ink.min.x - self.size * 0.15 + r.f32() * (ink.width() + self.size * 0.3),
                ink.min.y - self.size * 0.2 + r.f32() * (ink.height() + self.size * 0.4),
            );
            let rad = self.size * r.range(0.13, 0.26);
            let col = colors[r.below(colors.len())];
            let k = (local * PI).sin().max(0.0);
            let k = k * k * (3.0 - 2.0 * k);
            let ang = r.range(0.0, 0.5) + local * 1.6;
            paint::glow(&painter, pos, rad * 2.2 * k, col.gamma_multiply(0.18 * k));
            sparkle_star(&painter, pos, rad * k, ang, col);
        }
        resp
    }
}
