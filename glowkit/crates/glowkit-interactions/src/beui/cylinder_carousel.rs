use super::common::{cr, hairline, idle_for};
use egui::{pos2, vec2, Color32, Mesh, Pos2, Sense, Shape, Stroke, TextureId, Ui, Vec2};
use glowkit_core::color::with_alpha;
use glowkit_core::{paint, Theme};
use glowkit_effects::PaintedScene;
use std::f32::consts::{PI, TAU};

/// How a [`CylinderCarousel`] bends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CylinderMode {
    /// Seen from inside the drum: items swell toward the edges.
    #[default]
    Concave,
    /// Seen from outside the drum: the middle item is the biggest.
    Convex,
}

#[derive(Clone, Copy, Debug)]
struct CylState {
    pos: f32,
    vel: f32,
}

/// A row of round artworks wrapped around a drum. Drag, scroll or use the arrow keys to roll it:
/// items swing around the cylinder, growing, shrinking and dimming with their angle, then coast
/// to a stop with the nearest item snapped to the middle. Concave and convex bends.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{CylinderCarousel, CylinderMode};
/// ui.add(CylinderCarousel::new().mode(CylinderMode::Convex).count(12));
/// ```
pub struct CylinderCarousel {
    mode: CylinderMode,
    count: usize,
    textures: Vec<TextureId>,
    item: f32,
    radius: Option<f32>,
    size: Option<Vec2>,
    autoplay: bool,
}

impl Default for CylinderCarousel {
    fn default() -> Self {
        Self::new()
    }
}

impl CylinderCarousel {
    /// A carousel of the built-in painted scenes.
    pub fn new() -> Self {
        Self { mode: CylinderMode::Concave, count: 12, textures: Vec::new(), item: 84.0, radius: None, size: None, autoplay: true }
    }
    /// Concave or convex.
    pub fn mode(mut self, m: CylinderMode) -> Self {
        self.mode = m;
        self
    }
    /// Number of items around the drum (default 12).
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.clamp(4, 40);
        self
    }
    /// Use your own textures (cycled to fill `count`).
    pub fn textures(mut self, t: impl IntoIterator<Item = TextureId>) -> Self {
        self.textures = t.into_iter().collect();
        self
    }
    /// Base diameter of an item.
    pub fn item_size(mut self, d: f32) -> Self {
        self.item = d.max(20.0);
        self
    }
    /// Radius of the drum (default: just wide enough for neighbours to almost touch).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = Some(r.max(60.0));
        self
    }
    /// Widget size (default: available width x 320).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Roll slowly on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for CylinderCarousel {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let size = glowkit_core::desired_size(ui, self.size, 320.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let id = resp.id;
        let n = self.count;
        let step = TAU / n as f32;
        let dt = glowkit_core::dt(ui);
        let dragging = resp.dragged();
        let drag = resp.drag_delta().x;
        let scroll = if resp.hovered() {
            let d = ui.input(|i| i.smooth_scroll_delta);
            d.x + d.y
        } else {
            0.0
        };
        let key = if resp.hovered() {
            ui.input(|i| i.key_pressed(egui::Key::ArrowRight) as i32 - i.key_pressed(egui::Key::ArrowLeft) as i32)
        } else {
            0
        };
        let idle = idle_for(ui, id, dragging || resp.hovered());
        let auto = self.autoplay && idle > 1.5;
        let r = self.radius.unwrap_or(self.item * 1.12 / step.sin()).min(rect.width() * 0.5);
        let unit = r * step; // points per item at the front
        let (pos, moving) = glowkit_core::with_state(
            ui,
            id,
            || CylState { pos: 0.0, vel: 0.0 },
            |s| {
                if dragging {
                    let d = -drag / unit;
                    s.pos += d;
                    s.vel = s.vel * 0.5 + 0.5 * if dt > 0.0 { d / dt } else { 0.0 };
                } else {
                    s.vel += (-scroll / unit) * 8.0 + key as f32 * 6.0;
                    if auto {
                        s.vel += (0.35 - s.vel) * (dt * 2.0).min(1.0);
                        s.pos += s.vel * dt;
                    } else {
                        // coast, then snap to the nearest item with a soft spring
                        s.pos += s.vel * dt;
                        s.vel *= (-3.2 * dt).exp();
                        if s.vel.abs() < 1.6 {
                            let target = s.pos.round();
                            s.vel += (target - s.pos) * 90.0 * dt;
                            s.vel *= (-9.0 * dt).exp();
                        }
                    }
                }
                (s.pos, dragging || s.vel.abs() > 1e-3 || (s.pos - s.pos.round()).abs() > 1e-3)
            },
        );
        if moving || auto {
            glowkit_core::animate(ui);
        }
        if resp.hovered() || dragging {
            ui.ctx().set_cursor_icon(if dragging { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter_at(rect);
        painter.rect(rect.shrink(1.0), cr(26.0), Color32::TRANSPARENT, hairline(&t), egui::StrokeKind::Inside);
        let mut items: Vec<(f32, usize, f32)> = (0..n)
            .map(|i| {
                let mut a = (i as f32 - pos) * step;
                a = (a + PI).rem_euclid(TAU) - PI;
                (a.cos(), i, a)
            })
            .filter(|x| x.0 > 0.0)
            .collect();
        // concave: the edges are nearer the eye, so paint the middle first
        match self.mode {
            CylinderMode::Convex => items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)),
            CylinderMode::Concave => items.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)),
        }
        let c = rect.center();
        let inner = painter.with_clip_rect(rect.shrink(2.0));
        for (cs, i, a) in items {
            // concave: seen from inside, the far side of the drum spreads out; convex: it bunches up
            let x = match self.mode {
                CylinderMode::Convex => c.x + a.sin() * r,
                CylinderMode::Concave => c.x + (a + a.powi(3) * 0.12) * r,
            };
            let depth = match self.mode {
                CylinderMode::Convex => 0.5 + 0.5 * cs,
                CylinderMode::Concave => 0.78 + (1.0 - cs) * 1.0,
            };
            let rad = self.item * 0.5 * depth;
            // discs turn away from the eye and narrow
            let squash = match self.mode {
                CylinderMode::Convex => cs.sqrt().max(0.12),
                CylinderMode::Concave => (0.82 + 0.18 * cs).max(0.12),
            };
            let p = pos2(x, c.y);
            let edge = ((rect.width() * 0.5 - (x - c.x).abs()) / 60.0).clamp(0.0, 1.0);
            let fade = (cs * 5.0).clamp(0.0, 1.0) * (0.35 + 0.65 * edge);
            let tint = with_alpha(Color32::WHITE, fade);
            let tex = if self.textures.is_empty() {
                PaintedScene::texture(ui.ctx(), i)
            } else {
                self.textures[i % self.textures.len()]
            };
            paint::glow_ellipse(&inner, p + vec2(0.0, rad * 1.05), vec2(rad * squash * 0.9, rad * 0.16), with_alpha(Color32::BLACK, 0.16 * fade));
            let mut mesh = Mesh::with_texture(tex);
            let nseg = 44;
            let uv_c = pos2(0.5 + a.sin() * 0.15, 0.5);
            mesh.vertices.push(egui::epaint::Vertex { pos: p, uv: uv_c, color: tint });
            for k in 0..=nseg {
                let ang = k as f32 / nseg as f32 * TAU;
                let dd = vec2(ang.cos(), ang.sin());
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: p + vec2(dd.x * rad * squash, dd.y * rad),
                    uv: uv_c + vec2(dd.x * 0.3 * squash, dd.y * 0.48),
                    color: tint,
                });
            }
            for k in 0..nseg as u32 {
                mesh.add_triangle(0, 1 + k, 2 + k);
            }
            inner.add(Shape::mesh(mesh));
            // the rim of the drum falls into shade
            let shade_a = (1.0 - cs).clamp(0.0, 1.0) * 0.4;
            if shade_a > 0.02 {
                let pts: Vec<Pos2> = (0..44)
                    .map(|k| {
                        let ang = k as f32 / 44.0 * TAU;
                        p + vec2(ang.cos() * rad * squash, ang.sin() * rad)
                    })
                    .collect();
                inner.add(Shape::convex_polygon(pts, with_alpha(Color32::BLACK, shade_a * fade), Stroke::NONE));
            }
            // a soft rim light
            inner.add(Shape::ellipse_stroke(p, vec2(rad * squash, rad), Stroke::new(1.0, with_alpha(Color32::WHITE, 0.18 * fade))));
        }
        resp
    }
}
