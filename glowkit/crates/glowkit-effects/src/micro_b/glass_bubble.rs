use super::common::{idle_for, lissajous, shadow_color};
use crate::scroll::common::{clip_mesh, cover_uv, image_rect};
use crate::scroll::PaintedScene;
use egui::epaint::{Mesh, Vertex};
use egui::{pos2, vec2, Color32, Pos2, Rect, Response, Sense, Stroke, TextureId, Ui, Vec2, Widget};
use glowkit_core::anim::spring2;
use glowkit_core::color::with_alpha;
use glowkit_core::paint;
use std::f32::consts::TAU;

/// A ball of glass you drag over a picture. Under the ball the image is magnified like a
/// thick lens and bent hard toward the rim, with the red, green and blue channels refracted by
/// slightly different amounts so edges pick up rainbow fringes. A thin specular rim and soft
/// contact shadow sell the glass. While idle the bubble drifts around on its own.
///
/// Without a texture it shows one of the built-in [`PaintedScene`]s.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::GlassBubble::new().size(300.0).bend(0.6));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct GlassBubble {
    texture: Option<(TextureId, f32)>,
    scene: usize,
    size: Vec2,
    corner: f32,
    lens: f32,
    bend: f32,
    fringe: f32,
    autoplay: bool,
}

impl Default for GlassBubble {
    fn default() -> Self {
        Self::new()
    }
}

impl GlassBubble {
    /// A glass bubble over a painted landscape.
    pub fn new() -> Self {
        Self {
            texture: None,
            scene: 4,
            size: vec2(300.0, 300.0),
            corner: 20.0,
            lens: 58.0,
            bend: 0.6,
            fringe: 0.3,
            autoplay: true,
        }
    }
    /// Show your own image (`aspect` = width / height of the texture). It is cropped to fill.
    pub fn texture(mut self, tex: TextureId, aspect: f32) -> Self {
        self.texture = Some((tex, aspect.max(0.01)));
        self
    }
    /// Which built-in [`PaintedScene`] to show when no texture is set.
    pub fn scene(mut self, index: usize) -> Self {
        self.scene = index;
        self
    }
    /// Square picture of `side` points.
    pub fn size(mut self, side: f32) -> Self {
        self.size = Vec2::splat(side.max(60.0));
        self
    }
    /// Picture size (width, height).
    pub fn picture_size(mut self, size: Vec2) -> Self {
        self.size = size.max(vec2(60.0, 60.0));
        self
    }
    /// Corner radius of the picture.
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// Radius of the glass ball in points.
    pub fn lens_radius(mut self, r: f32) -> Self {
        self.lens = r.max(10.0);
        self
    }
    /// How strongly the glass bends the picture (0..1).
    pub fn bend(mut self, k: f32) -> Self {
        self.bend = k.clamp(0.0, 1.0);
        self
    }
    /// Strength of the colour fringes (0..1).
    pub fn fringe(mut self, k: f32) -> Self {
        self.fringe = k.clamp(0.0, 1.0);
        self
    }
    /// Drift around on its own while nobody is dragging it (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

/// Map a radius fraction `r` (0 center .. 1 rim) to the sampling radius fraction: a spherical
/// cap profile, so the middle is magnified and the rim compresses steeply.
fn lens_profile(r: f32, bend: f32) -> f32 {
    let cap = (1.0 - r * r).max(0.0).sqrt();
    r * (1.0 - bend * 0.55 * cap)
}

impl Widget for GlassBubble {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = glowkit_core::Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(self.size, Sense::click_and_drag());
        let id = response.id;
        let (tex, aspect) = self.texture.unwrap_or_else(|| (PaintedScene::texture(ui.ctx(), self.scene), PaintedScene::ASPECT));
        let uv = cover_uv(aspect, rect.aspect_ratio());
        let active = response.dragged() || response.hovered();
        let idle = idle_for(ui, id, response.dragged());
        let time = glowkit_core::time(ui);
        let home = vec2(0.62, 0.38);
        let mut target: Vec2 = glowkit_core::state(ui, id.with("target"), || home);
        if response.dragged() {
            if let Some(p) = response.interact_pointer_pos() {
                target = ((p - rect.min) / rect.size()).clamp(Vec2::ZERO, Vec2::splat(1.0));
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        if self.autoplay && idle > 2.0 && !active {
            glowkit_core::animate(ui);
            let inner = Rect::from_min_max(pos2(0.2, 0.2), pos2(0.8, 0.8));
            target = lissajous(inner, time * 0.8, 0.7).to_vec2();
        }
        glowkit_core::set_state(ui, id.with("target"), target);
        let stiff = if response.dragged() { 420.0 } else { 120.0 };
        let pos = spring2(ui, id.with("pos"), target, stiff, if response.dragged() { 30.0 } else { 16.0 });
        let vel = {
            let prev: Vec2 = glowkit_core::state(ui, id.with("prev"), || pos);
            glowkit_core::set_state(ui, id.with("prev"), pos);
            (pos - prev) * rect.size()
        };
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        paint::soft_shadow(painter, rect, self.corner, 22.0, vec2(0.0, 8.0), shadow_color(&t, 0.8));
        image_rect(painter, rect, self.corner, tex, uv, Color32::WHITE);

        // squash a little along the motion, like a drop of liquid glass
        let speed = vel.length().min(30.0);
        let stretch = 1.0 + speed / 30.0 * 0.12;
        let dir = if speed > 0.1 { vel.normalized() } else { vec2(1.0, 0.0) };
        let c = rect.min + pos * rect.size();
        let rad = self.lens;
        let outline = paint::rounded_rect_points(rect, self.corner, 8);
        let warp = |v: Vec2| -> Vec2 {
            // anisotropic scale along `dir`
            let along = v.dot(dir);
            let across = v.dot(dir.rot90());
            dir * along * stretch + dir.rot90() * across / stretch
        };
        let to_uv = |p: Pos2| -> Pos2 {
            let q = ((p - rect.min) / rect.size()).clamp(Vec2::ZERO, Vec2::splat(1.0));
            pos2(uv.min.x + q.x * uv.width(), uv.min.y + q.y * uv.height())
        };
        let rings = 22;
        let segs = 72;
        let build = |offset: f32, tint: Color32| -> Mesh {
            let mut m = Mesh::with_texture(tex);
            for k in 0..=rings {
                let r = k as f32 / rings as f32;
                let rs = (lens_profile(r, self.bend) * (1.0 + offset * r * r)).min(1.0);
                for s in 0..segs {
                    let a = s as f32 / segs as f32 * TAU;
                    let d = vec2(a.cos(), a.sin());
                    let screen = c + warp(d * r * rad);
                    let sample = c + d * rs * rad;
                    m.vertices.push(Vertex { pos: screen, uv: to_uv(sample), color: tint });
                }
            }
            for k in 0..rings {
                for s in 0..segs {
                    let a = (k * segs + s) as u32;
                    let b = (k * segs + (s + 1) % segs) as u32;
                    let a2 = a + segs as u32;
                    let b2 = b + segs as u32;
                    m.add_triangle(a, b, b2);
                    m.add_triangle(a, b2, a2);
                }
            }
            clip_mesh(&m, &outline, rect.shrink(self.corner))
        };
        // contact shadow ring just outside the glass
        let ring_pts: Vec<Pos2> = (0..=64).map(|i| c + warp(Vec2::angled(i as f32 / 64.0 * TAU) * (rad + 1.5))).collect();
        paint::gradient_polyline(painter, &ring_pts, true, 3.0, |_| with_alpha(Color32::BLACK, 0.07));
        // three additive channels with slightly different refraction = colour fringes
        let f = self.fringe * 0.12;
        let black = {
            let mut m = Mesh::default();
            m.colored_vertex(c, Color32::BLACK);
            for i in 0..segs {
                m.colored_vertex(c + warp(Vec2::angled(i as f32 / segs as f32 * TAU) * rad), Color32::BLACK);
            }
            for i in 0..segs as u32 {
                m.add_triangle(0, 1 + i, 1 + (i + 1) % segs as u32);
            }
            clip_mesh(&m, &outline, rect.shrink(self.corner))
        };
        painter.add(black);
        for (off, col) in [
            (f, Color32::from_rgba_premultiplied(255, 0, 0, 0)),
            (0.0, Color32::from_rgba_premultiplied(0, 255, 0, 0)),
            (-f, Color32::from_rgba_premultiplied(0, 0, 255, 0)),
        ] {
            painter.add(build(off, col));
        }
        // glass shading: faint inner glow at the bottom, specular rim and a highlight
        let clip = painter.with_clip_rect(rect);
        let lens_pts: Vec<Pos2> = (0..=72).map(|i| c + warp(Vec2::angled(i as f32 / 72.0 * TAU) * (rad - 0.8))).collect();
        clip.add(egui::Shape::line(lens_pts, Stroke::new(1.6, with_alpha(Color32::WHITE, 0.85))));
        let hl: Vec<Pos2> = (0..=20)
            .map(|i| c + warp(Vec2::angled(-2.75 + i as f32 / 20.0 * 1.0) * (rad * 0.8)))
            .collect();
        paint::gradient_polyline(&clip, &hl, false, 3.0, |u| with_alpha(Color32::WHITE, 0.5 * (u * std::f32::consts::PI).sin()));
        paint::glow(&clip, c + warp(vec2(0.0, rad * 0.55)), rad * 0.45, with_alpha(Color32::WHITE, 0.10));
        response
    }
}
