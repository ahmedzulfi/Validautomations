//! A container that keeps a fixed width : height ratio.

use egui::{vec2, Color32, InnerResponse, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder};
use glowkit_core::Theme;

/// A box that always keeps the same aspect ratio (16:9, 1:1, 4:3, ...) and lays its content out
/// inside, clipped to rounded corners: the shadcn "Aspect Ratio" pattern for images, videos,
/// maps and embeds.
///
/// The width defaults to the available width (optionally capped by [`AspectRatio::max_width`]);
/// the height follows from the ratio and can be capped with [`AspectRatio::max_height`], in which
/// case the width shrinks so the ratio is preserved.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::AspectRatio;
/// AspectRatio::new(16.0 / 9.0).max_width(480.0).show(ui, |ui| {
///     let r = ui.max_rect();
///     ui.painter().rect_filled(r, 0.0, egui::Color32::DARK_GRAY);
/// });
/// ```
#[must_use = "call `.show(ui, |ui| ...)`"]
pub struct AspectRatio {
    ratio: f32,
    width: Option<f32>,
    max_width: Option<f32>,
    max_height: Option<f32>,
    fill: Option<Color32>,
    border: bool,
    rounding: Option<f32>,
    padding: f32,
    surface: Option<Color32>,
}

impl AspectRatio {
    /// A container with the ratio `width / height` (e.g. `16.0 / 9.0`).
    pub fn new(ratio: f32) -> Self {
        Self {
            ratio: ratio.max(0.01),
            width: None,
            max_width: None,
            max_height: None,
            fill: None,
            border: true,
            rounding: None,
            padding: 0.0,
            surface: None,
        }
    }
    /// Square (1:1).
    pub fn square() -> Self {
        Self::new(1.0)
    }
    /// Widescreen video (16:9).
    pub fn video() -> Self {
        Self::new(16.0 / 9.0)
    }
    /// Exact width (default: the available width).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Never wider than this.
    pub fn max_width(mut self, w: f32) -> Self {
        self.max_width = Some(w);
        self
    }
    /// Never taller than this (the width shrinks to keep the ratio).
    pub fn max_height(mut self, h: f32) -> Self {
        self.max_height = Some(h);
        self
    }
    /// Background fill (default: theme `muted`). Use `Color32::TRANSPARENT` for none.
    pub fn fill(mut self, c: Color32) -> Self {
        self.fill = Some(c);
        self
    }
    /// Draw a hairline border (default on).
    pub fn border(mut self, b: bool) -> Self {
        self.border = b;
        self
    }
    /// Corner radius (default: theme radius).
    pub fn rounding(mut self, r: f32) -> Self {
        self.rounding = Some(r);
        self
    }
    /// Inner padding around the content (default 0).
    pub fn padding(mut self, p: f32) -> Self {
        self.padding = p;
        self
    }

    /// Color of the surface behind the box, painted over the content's square corners so it
    /// looks rounded (default: the fill of the nearest enclosing frame, or the panel fill).
    pub fn surface(mut self, c: Color32) -> Self {
        self.surface = Some(c);
        self
    }

    /// The size this container would take in `ui`.
    pub fn size_in(&self, ui: &Ui) -> egui::Vec2 {
        let mut w = self.width.unwrap_or_else(|| ui.available_width());
        if let Some(m) = self.max_width {
            w = w.min(m);
        }
        let mut h = w / self.ratio;
        if let Some(mh) = self.max_height {
            if h > mh {
                h = mh;
                w = h * self.ratio;
            }
        }
        vec2(w.max(1.0), h.max(1.0))
    }

    /// Allocate the box and lay `content` out inside it. The child ui's `max_rect()` is the inner
    /// rect, and painting is clipped to it.
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let t = Theme::of(ui);
        let size = self.size_in(ui);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        let radius = self.rounding.unwrap_or(t.radius);
        let cr = egui::CornerRadius::same(radius.round().clamp(0.0, 255.0) as u8);
        let fill = self.fill.unwrap_or(t.muted);
        if ui.is_rect_visible(rect) {
            ui.painter().rect_filled(rect, cr, fill);
        }
        let inner_rect = rect.shrink(self.padding);
        let mut child =
            ui.new_child(UiBuilder::new().max_rect(inner_rect).layout(egui::Layout::top_down(egui::Align::Min)));
        child.set_clip_rect(inner_rect.intersect(ui.clip_rect()));
        let inner = content(&mut child);
        if ui.is_rect_visible(rect) {
            // mask the square clip corners with the surrounding background so content looks rounded
            if radius > 0.5 {
                mask_corners(ui, rect, radius, self.surface.unwrap_or_else(|| ui.stack().bg_color()));
            }
            if self.border {
                ui.painter().rect_stroke(rect, cr, Stroke::new(1.0, t.border), StrokeKind::Inside);
            }
        }
        InnerResponse::new(inner, resp)
    }
}

/// Paint the outside of each rounded corner with `bg` so square-clipped content appears rounded.
pub(crate) fn mask_corners(ui: &Ui, rect: Rect, radius: f32, bg: Color32) {
    let p = ui.painter();
    let n = 8;
    for (corner, center, a0) in [
        (rect.left_top(), rect.left_top() + vec2(radius, radius), 180.0f32),
        (rect.right_top(), rect.right_top() + vec2(-radius, radius), 270.0),
        (rect.right_bottom(), rect.right_bottom() + vec2(-radius, -radius), 0.0),
        (rect.left_bottom(), rect.left_bottom() + vec2(radius, -radius), 90.0),
    ] {
        let mut pts = vec![corner];
        for i in 0..=n {
            let a = (a0 + 90.0 * i as f32 / n as f32).to_radians();
            pts.push(center + vec2(a.cos(), a.sin()) * (radius + 0.5));
        }
        // fan triangles from the corner point (the polygon is not convex in general, so use a mesh)
        let mut mesh = egui::Mesh::default();
        for q in &pts {
            mesh.colored_vertex(*q, bg);
        }
        for i in 1..pts.len() as u32 - 1 {
            mesh.add_triangle(0, i, i + 1);
        }
        p.add(egui::Shape::mesh(mesh));
    }
}
