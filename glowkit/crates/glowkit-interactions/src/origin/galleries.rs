//! Image galleries in motion: a tunnel flight, a spiral vortex, a 3D coverflow and a hover list
//! with a floating image window.

use super::common::{h01, idle_secs, lerp_rgb, textured_quad, strip};
use egui::epaint::{Mesh, Vertex};
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, TextureId, Ui, Vec2, Widget};
use glowkit_core::anim::Spring;
use glowkit_core::color::{hex, with_alpha};
use glowkit_core::{paint, Theme};
use glowkit_effects::PaintedScene;
use std::f32::consts::{PI, TAU};

/// UV rect that crops a texture of aspect `tex` to cover a frame of aspect `frame`.
fn cover_uv(tex: f32, frame: f32) -> Rect {
    if frame > tex {
        let h = tex / frame;
        Rect::from_min_max(pos2(0.0, 0.5 - h * 0.5), pos2(1.0, 0.5 + h * 0.5))
    } else {
        let w = frame / tex;
        Rect::from_min_max(pos2(0.5 - w * 0.5, 0.0), pos2(0.5 + w * 0.5, 1.0))
    }
}

/// A rotated, rounded image card (texture cropped to cover).
#[allow(clippy::too_many_arguments)]
fn image_card(painter: &Painter, tex: TextureId, tex_aspect: f32, center: Pos2, size: Vec2, angle: f32, radius: f32, tint: Color32) {
    let uv = cover_uv(tex_aspect, size.x / size.y.max(1.0));
    let local = Rect::from_center_size(Pos2::ZERO, size);
    let pts = paint::rounded_rect_points(local, radius.min(size.min_elem() * 0.5), 5);
    let rot = egui::emath::Rot2::from_angle(angle);
    let mut m = Mesh::with_texture(tex);
    let uv_of = |p: Pos2| {
        let f = (p - local.min) / local.size();
        pos2(uv.min.x + f.x * uv.width(), uv.min.y + f.y * uv.height())
    };
    m.vertices.push(Vertex { pos: center, uv: uv_of(Pos2::ZERO), color: tint });
    for p in &pts {
        m.vertices.push(Vertex { pos: center + rot * p.to_vec2(), uv: uv_of(*p), color: tint });
    }
    let n = pts.len() as u32;
    for i in 0..n {
        m.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(Shape::mesh(m));
}

fn scenes(ui: &Ui) -> Vec<TextureId> {
    (0..PaintedScene::COUNT).map(|i| PaintedScene::texture(ui.ctx(), i)).collect()
}

// ---------------------------------------------------------------------------------------------
// Gallery Tunnel
// ---------------------------------------------------------------------------------------------

/// An endless flight through a square tunnel of wireframe segments lined with images and bold
/// color panels. Click (or hold) to accelerate; the pointer steers the vanishing point.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::GalleryTunnel::new().speed(1.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct GalleryTunnel {
    size: Option<Vec2>,
    images: Vec<TextureId>,
    aspect: f32,
    speed: f32,
    fill: f32,
    colors: Option<Vec<Color32>>,
}

impl Default for GalleryTunnel {
    fn default() -> Self {
        Self::new()
    }
}

impl GalleryTunnel {
    /// A tunnel lined with the built-in painted scenes.
    pub fn new() -> Self {
        Self { size: None, images: Vec::new(), aspect: PaintedScene::ASPECT, speed: 1.0, fill: 0.55, colors: None }
    }
    /// Use your own images (textures of aspect `aspect` = width / height).
    pub fn images(mut self, images: Vec<TextureId>, aspect: f32) -> Self {
        self.images = images;
        self.aspect = aspect.max(0.1);
        self
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Cruise speed multiplier.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Share of wall cells that carry a panel (0..1).
    pub fn fill(mut self, k: f32) -> Self {
        self.fill = k.clamp(0.0, 1.0);
        self
    }
    /// Colors of the solid panels.
    pub fn colors(mut self, c: Vec<Color32>) -> Self {
        if !c.is_empty() {
            self.colors = Some(c);
        }
        self
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Flight {
    z: f32,
    boost: Spring,
}

impl Widget for GalleryTunnel {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = glowkit_core::desired_size(ui, self.size, 380.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let response = response.on_hover_cursor(CursorIcon::PointingHand);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let t = Theme::of(ui);
        let id = response.id;
        let time = glowkit_core::time(ui);
        let dt = glowkit_core::dt(ui);
        let down = response.is_pointer_button_down_on();
        let auto = response.hover_pos().is_none() && (time % 6.0) > 4.6;
        let clicked = response.clicked();
        let speed = self.speed;
        let fl = glowkit_core::with_state(ui, id.with("flight"), || Flight { z: 0.0, boost: Spring::new(0.0).with(60.0, 12.0) }, |f| {
            f.boost.target = if down || auto { 1.0 } else { 0.0 };
            if clicked {
                f.boost.velocity += 6.0;
            }
            f.boost.step(dt);
            f.z += dt * speed * (1.2 + f.boost.value.max(0.0) * 5.0);
            *f
        });
        let target = match response.hover_pos() {
            Some(p) => (p - rect.center()) / (rect.size() * 0.5),
            None => vec2((time * 0.3).sin() * 0.4, (time * 0.23).cos() * 0.3),
        };
        let look = glowkit_core::anim::spring2(ui, id.with("look"), target, 20.0, 8.0);
        let painter = ui.painter_at(rect);
        let bg = if t.dark { hex("#040406") } else { t.background };
        painter.rect_filled(rect, 0.0, bg);
        let textures = if self.images.is_empty() { scenes(ui) } else { self.images.clone() };
        let _aspect = if self.images.is_empty() { PaintedScene::ASPECT } else { self.aspect };
        let colors = self.colors.clone().unwrap_or_else(|| {
            vec![hex("#2563EB"), hex("#EA580C"), hex("#16A34A"), hex("#9333EA"), hex("#EAB308"), hex("#DC2626")]
        });
        let focal = rect.height() * 0.55;
        let vp = rect.center() - look * vec2(rect.width() * 0.12, rect.height() * 0.12);
        let aspect_r = rect.width() / rect.height();
        let (hx, hy) = (aspect_r.max(1.0), 1.0);
        let proj = |x: f32, y: f32, z: f32| -> Pos2 { vp + vec2(x, y) / z.max(0.05) * focal };
        let segs = 16;
        let fog = |z: f32| -> f32 { (1.0 - (z / segs as f32)).clamp(0.0, 1.0).powf(1.4) };
        let base_seg = fl.z.floor() as i64;
        let frac = fl.z - fl.z.floor();
        let wire = if t.dark { with_alpha(Color32::WHITE, 0.5) } else { with_alpha(hex("#18181B"), 0.45) };
        // cells: each wall split in 2 lanes across
        for s in (0..segs).rev() {
            let z0 = s as f32 + 1.0 - frac;
            let z1 = z0 + 1.0;
            if z0 < 0.04 {
                continue;
            }
            let seg_id = (base_seg + s as i64) as u64;
            let f0 = fog(z0);
            let f1 = fog(z1);
            for wall in 0..4 {
                for lane in 0..2 {
                    let seed = seg_id * 97 + wall as u64 * 13 + lane as u64 * 5 + 1;
                    let r = h01(seed);
                    if r > self.fill {
                        continue;
                    }
                    let (a, b) = (lane as f32 - 1.0, lane as f32);
                    // corners in the wall plane, from (a..b) across and z0..z1 along
                    let corner = |u: f32, z: f32| -> Pos2 {
                        match wall {
                            0 => proj(u * hx, -hy, z),
                            1 => proj(hx, u * hy, z),
                            2 => proj(-u * hx, hy, z),
                            _ => proj(-hx, -u * hy, z),
                        }
                    };
                    let q = [corner(a, z1), corner(b, z1), corner(b, z0), corner(a, z0)];
                    let alpha = |v: f32| f1 + (f0 - f1) * v;
                    if r < self.fill * 0.55 {
                        let tex = textures[(seed as usize) % textures.len()];
                        textured_quad(&painter, tex, q, 4, |_, v| with_alpha(Color32::WHITE, alpha(v)));
                    } else {
                        let c = colors[(seed as usize / 3) % colors.len()];
                        paint::quad(&painter, q, [lerp_rgb(bg, c, f1), lerp_rgb(bg, c, f1), lerp_rgb(bg, c, f0), lerp_rgb(bg, c, f0)]);
                    }
                }
            }
            // wire frame of the ring at z0 and the longitudinal edges
            let ring = |z: f32| -> Vec<Pos2> {
                vec![proj(-hx, -hy, z), proj(0.0, -hy, z), proj(hx, -hy, z), proj(hx, 0.0, z), proj(hx, hy, z), proj(0.0, hy, z), proj(-hx, hy, z), proj(-hx, 0.0, z), proj(-hx, -hy, z)]
            };
            let a0 = with_alpha(wire, f0 * 0.9);
            painter.add(Shape::line(ring(z0), Stroke::new(1.0, a0)));
            let r0 = ring(z0);
            let r1 = ring(z1);
            let mut mesh = Mesh::default();
            for k in 0..8 {
                strip(&mut mesh, &[r1[k], r0[k]], |_| 0.8, |i| with_alpha(wire, if i == 0 { f1 } else { f0 } * 0.8), |_| Color32::TRANSPARENT);
            }
            painter.add(Shape::mesh(mesh));
        }
        // speed streaks when boosting
        let b = fl.boost.value.clamp(0.0, 1.5);
        if b > 0.05 {
            let mut mesh = Mesh::default();
            for i in 0..40 {
                let a = h01(i * 7 + 1) * TAU;
                let z = ((h01(i * 7 + 2) * 8.0 - fl.z * 2.0) % 8.0 + 8.0) % 8.0 + 0.3;
                let d = vec2(a.cos() * hx, a.sin() * hy) * 0.9;
                let p0 = proj(d.x, d.y, z);
                let p1 = proj(d.x, d.y, z + 0.8 * b);
                let c = with_alpha(if t.dark { Color32::WHITE } else { hex("#18181B") }, 0.35 * b.min(1.0));
                strip(&mut mesh, &[p1, p0], |_| 1.0, |k| if k == 0 { Color32::TRANSPARENT } else { c }, |_| Color32::TRANSPARENT);
            }
            painter.add(Shape::mesh(mesh));
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------
// Spiral Gallery
// ---------------------------------------------------------------------------------------------

/// Image cards flowing along an Archimedean spiral from the edge into the center, shrinking and
/// turning as they are drawn into the vortex. Hovering slows the flow.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::SpiralGallery::new().cards(36));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SpiralGallery {
    size: Option<Vec2>,
    images: Vec<TextureId>,
    aspect: f32,
    cards: usize,
    turns: f32,
    speed: f32,
}

impl Default for SpiralGallery {
    fn default() -> Self {
        Self::new()
    }
}

impl SpiralGallery {
    /// A spiral of the built-in painted scenes.
    pub fn new() -> Self {
        Self { size: None, images: Vec::new(), aspect: PaintedScene::ASPECT, cards: 56, turns: 2.6, speed: 1.0 }
    }
    /// Use your own images (textures of aspect `aspect` = width / height).
    pub fn images(mut self, images: Vec<TextureId>, aspect: f32) -> Self {
        self.images = images;
        self.aspect = aspect.max(0.1);
        self
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Number of cards on the spiral (8..80).
    pub fn cards(mut self, n: usize) -> Self {
        self.cards = n.clamp(8, 80);
        self
    }
    /// Number of spiral turns (1..5).
    pub fn turns(mut self, k: f32) -> Self {
        self.turns = k.clamp(1.0, 5.0);
        self
    }
    /// Flow speed multiplier.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
}

impl Widget for SpiralGallery {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = glowkit_core::desired_size(ui, self.size, 380.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let t = Theme::of(ui);
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let slow = glowkit_core::anim::follow(ui, id.with("slow"), if response.hovered() { 0.25 } else { 1.0 }, 3.0);
        let speed = self.speed;
        let phase = glowkit_core::with_state(ui, id.with("phase"), || 0.0f32, |p| {
            *p += dt * 0.035 * speed * slow;
            *p
        });
        let painter = ui.painter_at(rect);
        let bg = if t.dark { hex("#030305") } else { t.background };
        painter.rect_filled(rect, 0.0, bg);
        let textures = if self.images.is_empty() { scenes(ui) } else { self.images.clone() };
        let aspect = if self.images.is_empty() { PaintedScene::ASPECT } else { self.aspect };
        let c = rect.center();
        let rmax = rect.size().length() * 0.52;
        let n = self.cards;
        let mut items: Vec<(f32, usize)> = (0..n).map(|i| (((i as f32 / n as f32) - phase).rem_euclid(1.0), i)).collect();
        items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        for (s, i) in items {
            // s: 0 = center, 1 = outer edge
            let r = rmax * s.powf(1.15);
            let ang = s * self.turns * TAU + 0.3;
            let p = c + vec2(ang.cos(), ang.sin()) * r;
            let w = (10.0 + 150.0 * s.powf(1.25)).min(rect.height() * 0.5);
            let sz = vec2(w, w * 1.08);
            let fade = (s * 6.0).min(1.0);
            let tint = lerp_rgb(bg, Color32::WHITE, fade);
            if s > 0.2 {
                paint::soft_shadow(&painter, Rect::from_center_size(p + vec2(0.0, w * 0.06), sz * 0.9), w * 0.12, w * 0.12, Vec2::ZERO, with_alpha(Color32::BLACK, 0.25 * fade));
            }
            image_card(&painter, textures[i % textures.len()], aspect, p, sz, ang + PI * 0.5 + 0.15 * (i as f32).sin(), w * 0.12, tint);
        }
        // the eye of the vortex
        if t.dark {
            paint::glow(&painter, c, rmax * 0.12, with_alpha(Color32::BLACK, 0.9));
        } else {
            paint::glow(&painter, c, rmax * 0.1, with_alpha(t.background, 0.9));
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------
// Coverflow Gallery
// ---------------------------------------------------------------------------------------------

/// A 3D coverflow carousel: the active slide faces you while its neighbours swing back in
/// perspective, overlapping like records in a crate. Autoplays; drag, click a side slide or use
/// the arrow keys to move. The active slide's title fades in below it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut index = 0usize;
/// ui.add(glowkit_interactions::CoverflowGallery::new(&mut index).autoplay(3.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CoverflowGallery<'a> {
    index: &'a mut usize,
    size: Option<Vec2>,
    images: Vec<TextureId>,
    titles: Vec<String>,
    aspect: f32,
    angle: f32,
    autoplay: f32,
    reflection: bool,
}

impl<'a> CoverflowGallery<'a> {
    /// A coverflow of the built-in painted scenes, editing the active `index`.
    pub fn new(index: &'a mut usize) -> Self {
        Self {
            index,
            size: None,
            images: Vec::new(),
            titles: Vec::new(),
            aspect: PaintedScene::ASPECT,
            angle: 0.95,
            autoplay: 3.0,
            reflection: true,
        }
    }
    /// Use your own images (textures of aspect `aspect`) and slide titles.
    pub fn images(mut self, images: Vec<TextureId>, titles: Vec<String>, aspect: f32) -> Self {
        self.images = images;
        self.titles = titles;
        self.aspect = aspect.max(0.1);
        self
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Swing angle of the side slides in radians.
    pub fn angle(mut self, a: f32) -> Self {
        self.angle = a.clamp(0.0, 1.4);
        self
    }
    /// Seconds per slide when idle (0 = off).
    pub fn autoplay(mut self, secs: f32) -> Self {
        self.autoplay = secs.max(0.0);
        self
    }
    /// Paint a faint floor reflection (default on).
    pub fn reflection(mut self, on: bool) -> Self {
        self.reflection = on;
        self
    }
}

#[derive(Clone, Copy, Debug)]
struct Flow {
    pos: Spring,
    drag: f32,
    last_auto: f64,
}

impl Widget for CoverflowGallery<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = glowkit_core::desired_size(ui, self.size, 380.0);
        let (rect, mut response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let id = response.id;
        let t = Theme::of(ui);
        let textures = if self.images.is_empty() { scenes(ui) } else { self.images.clone() };
        let n = textures.len().max(1);
        let titles: Vec<String> = if self.titles.is_empty() { (0..n).map(|i| PaintedScene::name(i).to_string()).collect() } else { self.titles.clone() };
        let aspect = if self.images.is_empty() { PaintedScene::ASPECT } else { self.aspect };
        let now = ui.input(|i| i.time);
        let dt = glowkit_core::dt(ui);
        let card_h = rect.height() * 0.6;
        let card_w = (card_h * 0.78).min(rect.width() * 0.3);
        let spacing = card_w * 0.42;
        let idle = idle_secs(ui, id, response.hovered() || response.dragged());
        let (left, right) = if response.hovered() {
            ui.input(|i| (i.key_pressed(egui::Key::ArrowLeft), i.key_pressed(egui::Key::ArrowRight)))
        } else {
            (false, false)
        };
        let drag_dx = response.drag_delta().x;
        let released = response.drag_stopped();
        let click_x = if response.clicked() { response.interact_pointer_pos().map(|p| p.x - rect.center().x) } else { None };
        let mut index = (*self.index).min(n - 1);
        let auto_secs = self.autoplay as f64;
        let start = index as f32;
        let st = glowkit_core::with_state(ui, id.with("flow"), || Flow { pos: Spring::new(start).with(120.0, 20.0), drag: 0.0, last_auto: now }, |s| {
            if drag_dx != 0.0 {
                s.drag += drag_dx / spacing;
                s.last_auto = now;
            }
            if released {
                let target = (index as f32 - s.drag).round().clamp(0.0, (n - 1) as f32);
                s.pos.value -= s.drag;
                s.drag = 0.0;
                index = target as usize;
            }
            if let Some(x) = click_x {
                if x.abs() > card_w * 0.5 {
                    index = if x > 0.0 { (index + 1).min(n - 1) } else { index.saturating_sub(1) };
                }
                s.last_auto = now;
            }
            if left {
                index = index.saturating_sub(1);
            }
            if right {
                index = (index + 1).min(n - 1);
            }
            if auto_secs > 0.0 && idle > 1.0 && now - s.last_auto > auto_secs {
                index = (index + 1) % n;
                s.last_auto = now;
            }
            s.pos.target = index as f32;
            s.pos.step(dt);
            *s
        });
        if index != *self.index {
            *self.index = index;
            response.mark_changed();
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        if !st.pos.is_settled() || self.autoplay > 0.0 || st.drag != 0.0 {
            glowkit_core::animate(ui);
        }
        let pos = st.pos.value - st.drag;
        let painter = ui.painter_at(rect);
        let center = pos2(rect.center().x, rect.min.y + rect.height() * 0.4);
        let focal = 900.0;
        let place = |off: f32| -> ([Pos2; 4], f32) {
            // off: signed slot offset from the active slide (fractional while moving)
            let a = off.clamp(-1.0, 1.0);
            let rot = -a * self.angle;
            let x = off.signum() * (off.abs().min(1.0) * card_w * 0.62 + (off.abs() - 1.0).max(0.0) * spacing);
            let z = -off.abs().min(1.0) * card_w * 0.55;
            let (s, c) = rot.sin_cos();
            let corner = |u: f32, v: f32| -> Pos2 {
                let lx = u * card_w * 0.5;
                let px = x + lx * c;
                let pz = z + lx * s;
                let k = focal / (focal - pz);
                pos2(center.x + px * k, center.y + v * card_h * 0.5 * k)
            };
            ([corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0)], z)
        };
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|a, b| {
            let da = (*a as f32 - pos).abs();
            let db = (*b as f32 - pos).abs();
            db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
        });
        let uv = cover_uv(aspect, card_w / card_h);
        for i in order {
            let off = i as f32 - pos;
            if off.abs() > 4.5 {
                continue;
            }
            let (q, _) = place(off);
            let shade = 1.0 - (off.abs() * 0.22).min(0.6);
            let tint = lerp_rgb(if t.dark { Color32::BLACK } else { hex("#D4D4D8") }, Color32::WHITE, shade);
            let fade = (1.0 - (off.abs() - 3.5).max(0.0)).clamp(0.0, 1.0);
            let tint = with_alpha(tint, fade);
            // shadow
            let bb = Rect::from_points(&q);
            paint::soft_shadow(&painter, bb.shrink(6.0).translate(vec2(0.0, 10.0)), 14.0, 22.0, Vec2::ZERO, with_alpha(Color32::BLACK, if t.dark { 0.55 } else { 0.18 } * fade));
            if self.reflection {
                let refl = [q[3], q[2], q[2] + (q[2] - q[1]) * 0.2, q[3] + (q[3] - q[0]) * 0.2];
                let rc = |v: f32| with_alpha(tint, 0.16 * (1.0 - v) * fade);
                let mut m = Mesh::with_texture(textures[i % textures.len()]);
                let uvs = [pos2(uv.min.x, uv.max.y), pos2(uv.max.x, uv.max.y), pos2(uv.max.x, uv.max.y - uv.height() * 0.2), pos2(uv.min.x, uv.max.y - uv.height() * 0.2)];
                for k in 0..4 {
                    m.vertices.push(Vertex { pos: refl[k] + vec2(0.0, 4.0), uv: uvs[k], color: rc(if k < 2 { 0.0 } else { 1.0 }) });
                }
                m.add_triangle(0, 1, 2);
                m.add_triangle(0, 2, 3);
                painter.add(Shape::mesh(m));
            }
            let mut m = Mesh::with_texture(textures[i % textures.len()]);
            let sub = 6;
            for j in 0..=sub {
                let v = j as f32 / sub as f32;
                let l = q[0].lerp(q[3], v);
                let r = q[1].lerp(q[2], v);
                for k in 0..=sub {
                    let u = k as f32 / sub as f32;
                    m.vertices.push(Vertex { pos: l.lerp(r, u), uv: pos2(uv.min.x + u * uv.width(), uv.min.y + v * uv.height()), color: tint });
                }
            }
            let w = (sub + 1) as u32;
            for j in 0..sub as u32 {
                for k in 0..sub as u32 {
                    let a = j * w + k;
                    m.add_triangle(a, a + 1, a + w + 1);
                    m.add_triangle(a, a + w + 1, a + w);
                }
            }
            painter.add(Shape::mesh(m));
            painter.add(Shape::closed_line(q.to_vec(), Stroke::new(1.0, with_alpha(Color32::WHITE, 0.25 * fade))));
        }
        // title
        let active = pos.round().clamp(0.0, (n - 1) as f32) as usize;
        let near = 1.0 - ((pos - active as f32).abs() * 2.0).min(1.0);
        let ty = center.y + card_h * 0.5 + card_h * 0.2 + 12.0;
        painter.text(pos2(center.x, ty + (1.0 - near) * 6.0), Align2::CENTER_CENTER, &titles[active % titles.len()], FontId::proportional(18.0), with_alpha(t.foreground, near));
        painter.text(pos2(center.x, ty + 22.0), Align2::CENTER_CENTER, format!("{:02} / {:02}", active + 1, n), FontId::monospace(11.0), with_alpha(t.muted_foreground, 0.8));
        response.on_hover_cursor(CursorIcon::Grab)
    }
}

// ---------------------------------------------------------------------------------------------
// Hover Image Reveal
// ---------------------------------------------------------------------------------------------

/// A list of large names; hovering a row floats an image window beside the pointer that follows
/// it on a spring, tilts with its velocity and switches image to match the row. Other rows dim.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::HoverImageReveal::new(vec!["New season".into(), "Essentials".into()]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct HoverImageReveal {
    items: Vec<String>,
    images: Vec<TextureId>,
    aspect: f32,
    font_size: f32,
    window: Vec2,
    autoplay: bool,
}

impl HoverImageReveal {
    /// A reveal list of `items` (images default to the built-in painted scenes).
    pub fn new(items: Vec<String>) -> Self {
        Self { items, images: Vec::new(), aspect: PaintedScene::ASPECT, font_size: 34.0, window: vec2(200.0, 136.0), autoplay: true }
    }
    /// One image per row (textures of aspect `aspect`).
    pub fn images(mut self, images: Vec<TextureId>, aspect: f32) -> Self {
        self.images = images;
        self.aspect = aspect.max(0.1);
        self
    }
    /// Row font size.
    pub fn font_size(mut self, s: f32) -> Self {
        self.font_size = s.clamp(12.0, 80.0);
        self
    }
    /// Size of the floating image window.
    pub fn window(mut self, s: Vec2) -> Self {
        self.window = s;
        self
    }
    /// Sweep a ghost pointer through the rows while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

#[derive(Clone, Copy, Debug)]
struct Reveal {
    x: Spring,
    y: Spring,
    show: Spring,
    row: usize,
    prev: usize,
    switch_t: f32,
}

impl Widget for HoverImageReveal {
    fn ui(self, ui: &mut Ui) -> Response {
        let rows = self.items.len().max(1);
        let row_h = self.font_size * 1.18;
        let size = vec2(ui.available_width(), row_h * rows as f32 + 80.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let id = response.id;
        let t = Theme::of(ui);
        let time = glowkit_core::time(ui);
        let dt = glowkit_core::dt(ui);
        let painter = ui.painter_at(rect.expand(self.window.max_elem()));
        let font = FontId::proportional(self.font_size);
        let top = rect.min.y + 40.0;
        let idle = idle_secs(ui, id, response.hovered());
        let ghost = self.autoplay && !response.hovered() && idle > 1.0;
        let pointer = response.hover_pos().or(if ghost {
            let cyc = (time * 0.32) % 1.0;
            let y = top + row_h * (0.5 + (cyc * 1.25 * rows as f32).min(rows as f32 - 0.5));
            let on = cyc < 0.8;
            if on {
                Some(pos2(rect.center().x - rect.width() * 0.12 + (time * 0.9).sin() * rect.width() * 0.1, y))
            } else {
                None
            }
        } else {
            None
        });
        let row = pointer.map(|p| (((p.y - top) / row_h).floor().max(0.0) as usize).min(rows - 1));
        let hov = pointer.is_some() && row.is_some();
        let target = pointer.unwrap_or(rect.center());
        let st = glowkit_core::with_state(
            ui,
            id.with("reveal"),
            || Reveal { x: Spring::new(target.x).with(160.0, 18.0), y: Spring::new(target.y).with(160.0, 18.0), show: Spring::new(0.0).with(220.0, 20.0), row: 0, prev: 0, switch_t: 1.0 },
            |s| {
                if let Some(p) = pointer {
                    s.x.target = p.x;
                    s.y.target = p.y;
                }
                s.show.target = if hov { 1.0 } else { 0.0 };
                if let Some(r) = row {
                    if r != s.row {
                        s.prev = s.row;
                        s.row = r;
                        s.switch_t = 0.0;
                    }
                }
                s.switch_t = (s.switch_t + dt * 4.0).min(1.0);
                for sp in [&mut s.x, &mut s.y, &mut s.show] {
                    sp.step(dt);
                }
                *s
            },
        );
        glowkit_core::animate(ui);
        let textures = if self.images.is_empty() { scenes(ui) } else { self.images.clone() };
        let aspect = if self.images.is_empty() { PaintedScene::ASPECT } else { self.aspect };
        // rows
        for (i, item) in self.items.iter().enumerate() {
            let y = top + row_h * (i as f32 + 0.5);
            let active = hov && row == Some(i);
            let k = glowkit_core::anim::follow(ui, id.with(("row", i)), if active { 1.0 } else { 0.0 }, 10.0);
            let dim = if hov { 0.28 + 0.72 * k } else { 1.0 };
            let col = with_alpha(t.foreground, dim);
            let x = rect.center().x + k * 10.0;
            // a faux-bold display weight: the label twice, half a point apart
            painter.text(pos2(x, y), Align2::CENTER_CENTER, item.to_uppercase(), font.clone(), col);
            painter.text(pos2(x + 0.6, y), Align2::CENTER_CENTER, item.to_uppercase(), font.clone(), col);
        }
        // floating window
        let show = st.show.value.clamp(0.0, 1.2);
        if show > 0.01 {
            let vel = st.x.velocity;
            let tilt = (vel / 3000.0).clamp(-0.25, 0.25);
            let c = pos2(st.x.value, st.y.value) + vec2(self.window.x * 0.62, 0.0);
            let sz = self.window * (0.6 + 0.4 * show);
            paint::soft_shadow(&painter, Rect::from_center_size(c + vec2(0.0, 12.0), sz), 12.0, 24.0, Vec2::ZERO, with_alpha(Color32::BLACK, 0.3 * show.min(1.0)));
            let e = glowkit_core::ease(glowkit_core::Easing::OutCubic, st.switch_t);
            let a = show.min(1.0);
            image_card(&painter, textures[st.prev % textures.len()], aspect, c, sz, tilt, 12.0, with_alpha(Color32::WHITE, a * (1.0 - e)));
            // incoming image slides up into the window
            image_card(&painter, textures[st.row % textures.len()], aspect, c + vec2(0.0, (1.0 - e) * 14.0), sz, tilt, 12.0, with_alpha(Color32::WHITE, a * e));
        }
        let _ = h01;
        response
    }
}
