use crate::util;
use egui::epaint::{Mesh, Vertex};
use egui::{pos2, vec2, Color32, Galley, Painter, Pos2, Response, Sense, Ui, Vec2, Widget};
use glowkit_core::Theme;
use std::sync::Arc;

type V = (Pos2, Pos2); // (position, uv)

/// Keep the part of a convex polygon where `dot(p, dir) >= lo` (or `<= hi` when `keep_below`).
fn clip_poly(poly: &[V], dir: Vec2, limit: f32, keep_below: bool) -> Vec<V> {
    let inside = |v: &V| {
        let s = v.0.to_vec2().dot(dir);
        if keep_below {
            s <= limit
        } else {
            s >= limit
        }
    };
    let mut out = Vec::with_capacity(poly.len() + 2);
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let (ia, ib) = (inside(&a), inside(&b));
        if ia {
            out.push(a);
        }
        if ia != ib {
            let sa = a.0.to_vec2().dot(dir);
            let sb = b.0.to_vec2().dot(dir);
            let k = ((limit - sa) / (sb - sa)).clamp(0.0, 1.0);
            out.push((a.0.lerp(b.0, k), a.1.lerp(b.1, k)));
        }
    }
    out
}

/// Paint a galley filled with diagonal stripes (only the stripes, the gaps stay transparent).
/// `period` is the stripe spacing in points, `duty` the filled fraction, `phase` shifts them.
pub(crate) fn striped_galley(painter: &Painter, pos: Pos2, galley: &Arc<Galley>, color: Color32, period: f32, duty: f32, phase: f32) {
    let [tw, th] = painter.fonts(|f| f.font_image_size());
    let norm = vec2(1.0 / tw.max(1) as f32, 1.0 / th.max(1) as f32);
    let dir = vec2(1.0, 1.0).normalized();
    let period = period.max(1.0);
    let width = period * duty.clamp(0.05, 0.95);
    let mut mesh = Mesh::default();
    for row in &galley.rows {
        let src = &row.visuals.mesh;
        let off = pos.to_vec2() + row.pos.to_vec2();
        for tri in src.indices.chunks_exact(3) {
            let v: Vec<V> = tri
                .iter()
                .map(|&i| {
                    let s = src.vertices[i as usize];
                    (s.pos + off, pos2(s.uv.x * norm.x, s.uv.y * norm.y))
                })
                .collect();
            let ss: Vec<f32> = v.iter().map(|x| x.0.to_vec2().dot(dir)).collect();
            let (lo, hi) = (ss.iter().copied().fold(f32::MAX, f32::min), ss.iter().copied().fold(f32::MIN, f32::max));
            let k0 = ((lo - phase - width) / period).floor() as i64;
            let k1 = ((hi - phase) / period).ceil() as i64;
            for k in k0..=k1 {
                let a = k as f32 * period + phase;
                let b = a + width;
                if b < lo || a > hi {
                    continue;
                }
                let p = clip_poly(&v, dir, a, false);
                let p = clip_poly(&p, dir, b, true);
                if p.len() < 3 {
                    continue;
                }
                let base = mesh.vertices.len() as u32;
                for (q, uv) in &p {
                    mesh.vertices.push(Vertex { pos: *q, uv: *uv, color });
                }
                for i in 1..p.len() as u32 - 1 {
                    mesh.add_triangle(base, base + i, base + i + 1);
                }
            }
        }
    }
    if !mesh.is_empty() {
        painter.add(egui::Shape::mesh(mesh));
    }
}

/// Bold text with an offset "shadow" made of animated diagonal stripes sliding through it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::LineShadowText::new("Shadow").size(96.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct LineShadowText {
    text: String,
    size: f32,
    color: Option<Color32>,
    shadow: Option<Color32>,
    offset: Option<Vec2>,
    period: Option<f32>,
    speed: f32,
    bold: bool,
}

impl LineShadowText {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), size: 88.0, color: None, shadow: None, offset: None, period: None, speed: 1.0, bold: true }
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
    /// Stripe color (defaults to the text color).
    pub fn shadow_color(mut self, c: Color32) -> Self {
        self.shadow = Some(c);
        self
    }
    /// Shadow offset in points (default 0.06 x font size, down-right).
    pub fn offset(mut self, o: Vec2) -> Self {
        self.offset = Some(o);
        self
    }
    /// Stripe spacing in points (default 0.07 x font size).
    pub fn stripe_spacing(mut self, points: f32) -> Self {
        self.period = Some(points);
        self
    }
    /// Stripe speed in spacings per second (0 = still, negative reverses).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Fake-bold the text (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
}

impl Widget for LineShadowText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size) * 1.3;
        let painter = ui.painter().clone();
        let galley = util::layout(&painter, self.text.clone(), &font);
        let offset = self.offset.unwrap_or(vec2(1.0, 1.0) * self.size * 0.06);
        let (rect, resp) = ui.allocate_exact_size(galley.size() + offset.abs() + vec2(bold, 0.0), Sense::hover());
        if self.speed != 0.0 {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let color = self.color.unwrap_or(t.foreground);
        let shadow = self.shadow.unwrap_or(color);
        let period = self.period.unwrap_or(self.size * 0.07);
        let phase = (glowkit_core::time(ui) * self.speed).fract() * period;
        let pos = rect.min + vec2((-offset.x).max(0.0), (-offset.y).max(0.0));
        let steps = (bold / 0.7).ceil() as usize;
        for i in 0..=steps {
            let dx = bold * i as f32 / steps.max(1) as f32;
            striped_galley(&painter, pos + offset + vec2(dx, 0.0), &galley, shadow, period, 0.42, phase);
        }
        util::paint_galley(&painter, pos, &galley, color, bold);
        resp
    }
}
