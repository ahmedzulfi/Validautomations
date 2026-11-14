use super::common::idle_time;
use egui::epaint::{Mesh, Vertex};
use egui::{pos2, vec2, Align2, Color32, FontId, Response, Sense, Shape, TextureId, Ui, Vec2, Widget};
use glowkit_effects::PaintedScene;

/// A picture behind reeded (fluted) glass. Every vertical rib is its own little cylindrical
/// lens holding a squeezed, mirrored slice of the image, smeared into long vertical spikes, with
/// a bright lip and a dark seam per rib. The glass bends harder around the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::FlutedGlass;
/// ui.add(FlutedGlass::new().ribs(32).strength(1.0).caption("Every edge, gently bent."));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct FlutedGlass {
    image: Option<TextureId>,
    image_aspect: f32,
    scene: usize,
    ribs: usize,
    strength: f32,
    size: Option<Vec2>,
    caption: Option<String>,
    autoplay: bool,
}

impl Default for FlutedGlass {
    fn default() -> Self {
        Self::new()
    }
}

impl FlutedGlass {
    /// Fluted glass over a built-in painted scene.
    pub fn new() -> Self {
        Self { image: None, image_aspect: PaintedScene::ASPECT, scene: 5, ribs: 30, strength: 1.0, size: None, caption: None, autoplay: true }
    }
    /// Your own picture (`aspect` = width / height), shown with cover-fit.
    pub fn image(mut self, tex: TextureId, aspect: f32) -> Self {
        self.image = Some(tex);
        self.image_aspect = aspect.max(0.1);
        self
    }
    /// Which built-in painted scene to show (when no image is set).
    pub fn scene(mut self, i: usize) -> Self {
        self.scene = i;
        self
    }
    /// Number of ribs across the width (default 30).
    pub fn ribs(mut self, n: usize) -> Self {
        self.ribs = n.clamp(4, 120);
        self
    }
    /// Refraction strength (default 1).
    pub fn strength(mut self, k: f32) -> Self {
        self.strength = k.clamp(0.0, 3.0);
        self
    }
    /// Widget size (default: available width x 360).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// A caption in the bottom-left corner.
    pub fn caption(mut self, s: impl Into<String>) -> Self {
        self.caption = Some(s.into());
        self
    }
    /// Sweep a virtual pointer across while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for FlutedGlass {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = glowkit_core::desired_size(ui, self.size, 360.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let id = response.id;
        let time = glowkit_core::time(ui);
        let idle = idle_time(ui, id, rect);
        let tex = self.image.unwrap_or_else(|| PaintedScene::texture(ui.ctx(), self.scene));
        // cover-fit uv window
        let target = rect.width() / rect.height();
        let (u0, uw, v0, vh) = if target > self.image_aspect {
            let h = self.image_aspect / target;
            (0.0, 1.0, (1.0 - h) * 0.5, h)
        } else {
            let w = target / self.image_aspect;
            ((1.0 - w) * 0.5, w, 0.0, 1.0)
        };
        let pointer = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p));
        let auto = self.autoplay && idle > 1.5;
        let focus_target = match pointer {
            Some(p) => Some(p),
            None if auto => Some(pos2(
                rect.center().x + (time * 0.45).sin() * rect.width() * 0.38,
                rect.center().y + (time * 0.7).cos() * rect.height() * 0.2,
            )),
            None => None,
        };
        let fx = glowkit_core::anim::follow(ui, id.with("fx"), focus_target.map_or(rect.center().x, |p| p.x), 6.0);
        let fy = glowkit_core::anim::follow(ui, id.with("fy"), focus_target.map_or(rect.center().y, |p| p.y), 6.0);
        let fk = glowkit_core::anim::follow(ui, id.with("fk"), if focus_target.is_some() { 1.0 } else { 0.0 }, 4.0);
        if auto {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let ribs = self.ribs;
        let rw = rect.width() / ribs as f32;
        let sub = 8;
        let rows = 28;
        let strength = self.strength;
        let mut m = Mesh::with_texture(tex);
        for r in 0..ribs {
            let x0 = rect.min.x + r as f32 * rw;
            let cx = x0 + rw * 0.5;
            // pointer influence on this rib
            let near = (-((cx - fx) / (rect.width() * 0.18)).powi(2)).exp() * fk;
            let k = strength * (1.0 + 1.6 * near);
            let base = m.vertices.len() as u32;
            for j in 0..=rows {
                let v = j as f32 / rows as f32;
                let y = rect.min.y + v * rect.height();
                let vy = (-((y - fy) / (rect.height() * 0.35)).powi(2)).exp();
                for i in 0..=sub {
                    let s = i as f32 / sub as f32 * 2.0 - 1.0; // -1..1 across the rib
                    let x = x0 + (s * 0.5 + 0.5) * rw;
                    // each rib shows a squeezed, flipped slice centred on itself
                    let squeeze = 0.45 / (1.0 + 0.4 * k);
                    let su = (cx + (-s) * rw * squeeze * (1.0 + 0.6 * k) - rect.min.x) / rect.width();
                    // vertical spikes: the lens smears the picture more toward the rib edges
                    let smear = s * s * 0.05 * k * (1.0 + near * 1.5 * vy);
                    let sv = 0.5 + (v - 0.5) * (1.0 - smear) + (s * 3.0 + r as f32).sin() * 0.004 * k;
                    let uv = pos2(u0 + su.clamp(0.0, 1.0) * uw, v0 + sv.clamp(0.0, 1.0) * vh);
                    // lighting: a bright lip on the left of each rib, a dark seam on the right
                    let lip = (-(s + 0.55).powi(2) * 18.0).exp() * 0.35;
                    let seam = sstep_abs(s) * 0.45;
                    let l = (1.0 + lip - seam).clamp(0.0, 1.6);
                    let c = shade(l);
                    m.vertices.push(Vertex { pos: pos2(x, y), uv, color: c });
                }
            }
            let w = (sub + 1) as u32;
            for j in 0..rows as u32 {
                for i in 0..sub as u32 {
                    let a = base + j * w + i;
                    m.add_triangle(a, a + 1, a + w + 1);
                    m.add_triangle(a, a + w + 1, a + w);
                }
            }
        }
        painter.add(Shape::mesh(m));
        // specular highlights (additive-looking white overlay per rib lip)
        let mut hl = Mesh::default();
        for r in 0..ribs {
            let x = rect.min.x + r as f32 * rw + rw * 0.22;
            let near = (-((x - fx) / (rect.width() * 0.18)).powi(2)).exp() * fk;
            let a = 0.05 + 0.1 * near;
            let b = hl.vertices.len() as u32;
            for (px, al) in [(x - rw * 0.12, 0.0), (x, a), (x + rw * 0.12, 0.0)] {
                hl.colored_vertex(pos2(px, rect.min.y), with_a(al));
                hl.colored_vertex(pos2(px, rect.max.y), with_a(al));
            }
            for k in 0..2u32 {
                let i = b + k * 2;
                hl.add_triangle(i, i + 1, i + 3);
                hl.add_triangle(i, i + 3, i + 2);
            }
        }
        painter.add(Shape::mesh(hl));
        if let Some(c) = &self.caption {
            let pos = rect.left_bottom() + vec2(16.0, -16.0);
            painter.text(pos + vec2(0.0, 1.0), Align2::LEFT_BOTTOM, c, FontId::proportional(15.0), Color32::from_black_alpha(120));
            painter.text(pos, Align2::LEFT_BOTTOM, c, FontId::proportional(15.0), Color32::WHITE);
        }
        response
    }
}

fn sstep_abs(s: f32) -> f32 {
    glowkit_core::anim::smoothstep(0.55, 1.0, s)
}

fn shade(l: f32) -> Color32 {
    // tint multiplier for textured vertices (values above 1 can't brighten, so cap and let the
    // highlight overlay do the rest)
    let c = (l.min(1.0) * 255.0) as u8;
    Color32::from_rgb(c, c, c)
}

fn with_a(a: f32) -> Color32 {
    Color32::from_white_alpha((a.clamp(0.0, 1.0) * 255.0) as u8)
}
