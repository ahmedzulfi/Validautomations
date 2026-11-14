//! Draggable image strips and a lensed photo field: a looping marquee, a parallax slider and a
//! barrel-distorted infinite gallery.

use super::common::*;
use egui::epaint::{Mesh, RectShape, Vertex};
use egui::{pos2, vec2, Align2, Color32, Pos2, Rect, Response, Sense, Shape, TextureId, Ui, Vec2};
use glowkit_core::anim::{approach, lerp};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;
use glowkit_effects::PaintedScene;

fn scene(ui: &Ui, textures: &[TextureId], i: usize) -> TextureId {
    if textures.is_empty() {
        PaintedScene::texture(ui.ctx(), i % PaintedScene::COUNT)
    } else {
        textures[i % textures.len()]
    }
}

/// Paint `tex` into `rect` rotated by `angle` around its center, with rounded corners.
fn rotated_image(painter: &egui::Painter, rect: Rect, radius: f32, tex: TextureId, uv: Rect, angle: f32, tint: Color32) {
    painter.add(Shape::Rect(RectShape::filled(rect, cr(radius), tint).with_texture(tex, uv).with_angle(angle)));
}

// ---------------------------------------------------------------------------------------------
// DraggableMarquee

#[derive(Clone, Copy, Default)]
struct MarqueeState {
    dir: f32,
    off: f32,
    vel: f32,
    hover: [f32; 16],
    hover_v: [f32; 16],
    g: Gesture,
}

/// A continuous strip of portrait images that loops forever and can be grabbed and flung.
///
/// The track drifts at a steady pace. Grab it and it sticks to your finger; let go with a flick
/// and it coasts on the momentum before easing back into its cruising speed (in whichever
/// direction it was going). Cards lean into fast motion, a little like cards on a carousel,
/// and the one under the pointer lifts. Arrow keys nudge it while it is hovered.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::DraggableMarquee;
/// ui.add(DraggableMarquee::new().speed(40.0).gap(20.0).lean(50.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DraggableMarquee {
    textures: Vec<TextureId>,
    count: usize,
    speed: f32,
    gap: f32,
    lean: f32,
    card: Vec2,
    height: f32,
    autoplay: bool,
}

impl Default for DraggableMarquee {
    fn default() -> Self {
        Self::new()
    }
}

impl DraggableMarquee {
    /// A marquee over the eight painted scenes.
    pub fn new() -> Self {
        Self { textures: Vec::new(), count: 8, speed: 40.0, gap: 20.0, lean: 50.0, card: vec2(190.0, 250.0), height: 340.0, autoplay: false }
    }
    /// Your own images (portrait crops are taken from the middle).
    pub fn textures(mut self, t: Vec<TextureId>) -> Self {
        self.count = t.len().max(1);
        self.textures = t;
        self
    }
    /// Cruising speed in points per second (negative drifts right).
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Space between cards.
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g.max(0.0);
        self
    }
    /// How much cards lean into fast motion, 0..100.
    pub fn lean(mut self, l: f32) -> Self {
        self.lean = l.clamp(0.0, 100.0);
        self
    }
    /// Card size.
    pub fn card(mut self, s: Vec2) -> Self {
        self.card = s;
        self
    }
    /// Height of the strip.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Fling the strip around on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const MARQUEE_SCRIPT: [Stroke2; 2] = [drag(1.0, 1.35, (0.72, 0.5), (0.3, 0.52)), drag(4.0, 4.4, (0.3, 0.5), (0.8, 0.48))];

impl egui::Widget for DraggableMarquee {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let touch = touch(ui, &response, rect, self.autoplay, &MARQUEE_SCRIPT, 7.0);
        let mut st: MarqueeState = glowkit_core::state(ui, id.with("marquee"), MarqueeState::default);
        let g = st.g.update(&touch, now, dt);
        let n = self.count.max(1);
        let step = self.card.x + self.gap;
        let total = step * n as f32;
        if st.dir == 0.0 {
            st.dir = 1.0;
        }
        if g.released && st.vel.abs() > 60.0 {
            st.dir = -st.vel.signum();
        }
        let cruise = -self.speed * st.dir;
        if st.g.down {
            st.off += g.delta.x;
            st.vel = st.g.vel.x;
        } else {
            if response.hovered() {
                let (l, r) = ui.input(|i| (i.key_pressed(egui::Key::ArrowLeft), i.key_pressed(egui::Key::ArrowRight)));
                if l {
                    st.vel -= 600.0;
                }
                if r {
                    st.vel += 600.0;
                }
            }
            st.vel = approach(st.vel, cruise, 1.6, dt);
            st.off += st.vel * dt;
        }
        st.off = st.off.rem_euclid(total);
        let excess = (st.vel - cruise).clamp(-2500.0, 2500.0);
        let s = st;
        // hover springs (filled below while painting)
        let painter = ui.painter_at(rect);
        let stage_fill = if t.dark { hexa(0x10, 0x10, 0x12) } else { hexa(0x16, 0x16, 0x1A) };
        painter.rect_filled(rect, cr(18.0), stage_fill);
        let cy = rect.center().y;
        let hover_p = touch.pos.filter(|_| !touch.down);
        let mut st2 = s;
        let k0 = ((rect.min.x - s.off) / step).floor() as i32 - 1;
        let k1 = ((rect.max.x - s.off) / step).ceil() as i32 + 1;
        for k in k0..=k1 {
            let idx = k.rem_euclid(n as i32) as usize;
            let x = s.off + k as f32 * step + rect.min.x;
            let base = Rect::from_center_size(pos2(x + self.card.x / 2.0, cy), self.card);
            let slot = (k.rem_euclid(16)) as usize;
            let hov = hover_p.is_some_and(|p| base.contains(p));
            spring_step(&mut st2.hover[slot], &mut st2.hover_v[slot], if hov { 1.0 } else { 0.0 }, 300.0, 18.0, dt);
            let h = st2.hover[slot];
            // lean into motion, with a little curvature by distance from center
            let lean = -excess / 2500.0 * 0.22 * self.lean / 50.0;
            let r = Rect::from_center_size(base.center() - vec2(0.0, h * 6.0), self.card * (1.0 + h * 0.04));
            if !r.intersects(rect.expand(40.0)) {
                continue;
            }
            let uv = cover_uv(PaintedScene::ASPECT, self.card.x / self.card.y);
            rotated_image(&painter, r, 18.0, scene(ui, &self.textures, idx * 3), uv, lean, Color32::WHITE);
        }
        // soft fades at the ends
        let fade = 60.0;
        for (x0, x1, a0, a1) in [(rect.min.x, rect.min.x + fade, 1.0, 0.0), (rect.max.x - fade, rect.max.x, 0.0, 1.0)] {
            let mut mesh = Mesh::default();
            let c0 = with_alpha(stage_fill, a0);
            let c1 = with_alpha(stage_fill, a1);
            mesh.colored_vertex(pos2(x0, rect.min.y), c0);
            mesh.colored_vertex(pos2(x1, rect.min.y), c1);
            mesh.colored_vertex(pos2(x1, rect.max.y), c1);
            mesh.colored_vertex(pos2(x0, rect.max.y), c0);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            painter.add(Shape::mesh(mesh));
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        glowkit_core::set_state(ui, id.with("marquee"), st2);
        ui.ctx().request_repaint();
        response
    }
}

// ---------------------------------------------------------------------------------------------
// ParallaxSlider

#[derive(Clone, Copy, Default)]
struct ParallaxState {
    s: f32,
    v: f32,
    title: f32,
    title_v: f32,
    hovered: i32,
    g: Gesture,
}

/// An endless strip of tall image frames where each picture drifts against the motion inside its
/// frame, so the strip feels deep.
///
/// Drag it, flick it, scroll over it or use the arrow keys: the strip glides to a stop with
/// momentum, and while it moves every image slides the other way within its frame (the
/// parallax). Once the strip is still, hovering a frame raises its title underneath. Light and
/// dark themes are both respected.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::ParallaxSlider;
/// ui.add(ParallaxSlider::new().depth(60.0).frame(egui::vec2(200.0, 300.0)));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ParallaxSlider {
    textures: Vec<TextureId>,
    titles: Vec<String>,
    frame: Vec2,
    gap: f32,
    depth: f32,
    friction: f32,
    autoplay: bool,
}

impl Default for ParallaxSlider {
    fn default() -> Self {
        Self::new()
    }
}

impl ParallaxSlider {
    /// A slider over the painted scenes.
    pub fn new() -> Self {
        Self {
            textures: Vec::new(),
            titles: Vec::new(),
            frame: vec2(190.0, 290.0),
            gap: 18.0,
            depth: 60.0,
            friction: 50.0,
            autoplay: false,
        }
    }
    /// Your own images and their titles.
    pub fn slides(mut self, slides: Vec<(TextureId, String)>) -> Self {
        let (t, n): (Vec<_>, Vec<_>) = slides.into_iter().unzip();
        self.textures = t;
        self.titles = n;
        self
    }
    /// Frame size.
    pub fn frame(mut self, s: Vec2) -> Self {
        self.frame = s;
        self
    }
    /// Space between frames.
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g.max(0.0);
        self
    }
    /// Strength of the in-frame parallax, 0..100.
    pub fn depth(mut self, d: f32) -> Self {
        self.depth = d.clamp(0.0, 100.0);
        self
    }
    /// How quickly a flick slows down, 0..100.
    pub fn friction(mut self, f: f32) -> Self {
        self.friction = f.clamp(0.0, 100.0);
        self
    }
    /// Fling the strip on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const PARALLAX_SCRIPT: [Stroke2; 2] = [drag(1.0, 1.3, (0.75, 0.45), (0.35, 0.47)), drag(4.6, 5.0, (0.3, 0.45), (0.62, 0.44))];

impl egui::Widget for ParallaxSlider {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let h = self.frame.y + 70.0;
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let touch = touch(ui, &response, rect, self.autoplay, &PARALLAX_SCRIPT, 8.0);
        let mut st: ParallaxState = glowkit_core::state(ui, id.with("parallax"), ParallaxState::default);
        let g = st.g.update(&touch, now, dt);
        let n = if self.textures.is_empty() { PaintedScene::COUNT } else { self.textures.len() };
        let step = self.frame.x + self.gap;
        if st.g.down {
            st.s -= g.delta.x;
            st.v = -st.g.vel.x;
        } else {
            if g.released {
                st.v = -st.g.vel.x;
            }
            if response.hovered() {
                let (wheel, l, r) = ui.input(|i| (i.smooth_scroll_delta, i.key_pressed(egui::Key::ArrowLeft), i.key_pressed(egui::Key::ArrowRight)));
                st.v -= (wheel.x + wheel.y) * 18.0;
                if l {
                    st.v -= 900.0;
                }
                if r {
                    st.v += 900.0;
                }
            }
            st.s += st.v * dt;
            let decay = lerp(0.9985, 0.993, self.friction / 100.0);
            st.v *= decay.powf(dt * 1000.0);
            if st.v.abs() < 3.0 {
                st.v = 0.0;
            }
        }
        let still = st.v.abs() < 20.0 && !st.g.down;
        let hover_p = touch.pos.filter(|_| still);
        let base_x = rect.min.x + 12.0;
        let k0 = ((st.s - 40.0) / step).floor() as i32;
        let k1 = ((st.s + rect.width()) / step).ceil() as i32;
        let mut hovered: Option<i32> = None;
        for k in k0..=k1 {
            let x = base_x + k as f32 * step - st.s;
            let fr = Rect::from_min_size(pos2(x, rect.min.y + 10.0), self.frame);
            if hover_p.is_some_and(|p| fr.contains(p)) {
                hovered = Some(k);
            }
        }
        let tt = if hovered.is_some() { 1.0 } else { 0.0 };
        if let Some(k) = hovered {
            if k != st.hovered {
                st.title = 0.0;
                st.hovered = k;
            }
        }
        spring_step(&mut st.title, &mut st.title_v, tt, 260.0, 20.0, dt);
        let s = st;
        glowkit_core::set_state(ui, id.with("parallax"), st);
        if s.v != 0.0 || s.g.down || touch.ghost || !settled(s.title, s.title_v, tt) {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let names = ["Alpenglow", "Lagoon", "Aurora", "Dunes", "Evergreen", "Neon City", "Glacier", "Lavender"];
        let shift = (s.v / 2500.0).clamp(-1.0, 1.0) * self.depth / 100.0;
        for k in k0..=k1 {
            let idx = k.rem_euclid(n as i32) as usize;
            let x = base_x + k as f32 * step - s.s;
            let fr = Rect::from_min_size(pos2(x, rect.min.y + 10.0), self.frame);
            // the image is wider than its frame and slides against the motion: a constant
            // position-based parallax plus a velocity kick
            let rel = ((fr.center().x - rect.center().x) / rect.width()).clamp(-1.0, 1.0);
            let aspect = self.frame.x / self.frame.y;
            let tex_aspect = PaintedScene::ASPECT;
            let win = (aspect / tex_aspect).min(1.0) * 0.75;
            let room = 1.0 - win;
            let c = 0.5 - rel * room * 0.5 * self.depth / 60.0 + shift * room * 0.4;
            let u0 = (c - win / 2.0).clamp(0.0, 1.0 - win);
            let uv = Rect::from_min_max(pos2(u0, 0.0), pos2(u0 + win, 1.0));
            let tex = if self.textures.is_empty() { PaintedScene::texture(ui.ctx(), idx) } else { self.textures[idx] };
            image_rect(&painter, fr, 6.0, tex, uv, Color32::WHITE);
            if k == s.hovered && s.title > 0.01 {
                let title = self.titles.get(idx).cloned().unwrap_or_else(|| names[idx % names.len()].to_string());
                let a = s.title.clamp(0.0, 1.0);
                text(&painter, pos2(fr.min.x + 2.0, fr.max.y + 20.0 + (1.0 - s.title) * 10.0), Align2::LEFT_CENTER, title, 14.0, with_alpha(t.foreground, a));
                text(&painter, pos2(fr.max.x - 2.0, fr.max.y + 20.0 + (1.0 - s.title) * 10.0), Align2::RIGHT_CENTER, format!("{:02}", idx + 1), 12.0, with_alpha(t.muted_foreground, a));
            }
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}

// ---------------------------------------------------------------------------------------------
// LensGallery

#[derive(Clone, Copy, Default)]
struct LensState {
    pan: Vec2,
    vel: Vec2,
    g: Gesture,
}

/// An endless wall of photos seen through a fisheye lens.
///
/// The grid of images is tiled infinitely in both directions and drawn through a barrel lens:
/// tiles near the middle swell and bow outward, tiles at the rim shrink and fall into a
/// vignette. Drag to pan and fling to coast; the lens bends every tile (each is a subdivided,
/// textured mesh) as it slides past.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::LensGallery;
/// ui.add(LensGallery::new().lens(60.0).tile(egui::vec2(150.0, 110.0)));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct LensGallery {
    textures: Vec<TextureId>,
    tile: Vec2,
    gap: f32,
    lens: f32,
    height: f32,
    autoplay: bool,
}

impl Default for LensGallery {
    fn default() -> Self {
        Self::new()
    }
}

impl LensGallery {
    /// A gallery over the painted scenes.
    pub fn new() -> Self {
        Self { textures: Vec::new(), tile: vec2(150.0, 110.0), gap: 14.0, lens: 60.0, height: 380.0, autoplay: false }
    }
    /// Your own images.
    pub fn textures(mut self, t: Vec<TextureId>) -> Self {
        self.textures = t;
        self
    }
    /// Tile size before distortion.
    pub fn tile(mut self, s: Vec2) -> Self {
        self.tile = s;
        self
    }
    /// Space between tiles.
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g.max(0.0);
        self
    }
    /// Lens strength, 0..100.
    pub fn lens(mut self, l: f32) -> Self {
        self.lens = l.clamp(0.0, 100.0);
        self
    }
    /// Widget height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Pan around on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const LENS_SCRIPT: [Stroke2; 3] = [
    drag(1.0, 1.5, (0.62, 0.6), (0.36, 0.38)),
    drag(3.4, 3.9, (0.35, 0.4), (0.6, 0.66)),
    drag(5.6, 6.2, (0.7, 0.5), (0.3, 0.55)),
];

impl egui::Widget for LensGallery {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let touch = touch(ui, &response, rect, self.autoplay, &LENS_SCRIPT, 8.0);
        let mut st: LensState = glowkit_core::state(ui, id.with("lens"), LensState::default);
        let g = st.g.update(&touch, now, dt);
        if st.g.down {
            st.pan += g.delta;
            st.vel = st.g.vel;
        } else {
            st.pan += st.vel * dt;
            st.vel *= 0.996f32.powf(dt * 1000.0);
            if st.vel.length() < 3.0 {
                st.vel = Vec2::ZERO;
            }
        }
        let s = st;
        glowkit_core::set_state(ui, id.with("lens"), st);
        if s.vel != Vec2::ZERO || s.g.down || touch.ghost {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, cr(18.0), Color32::BLACK);
        let c = rect.center();
        let rad = rect.size().length() * 0.5;
        let k = self.lens / 100.0 * 1.4;
        let distort = |p: Pos2| -> Pos2 {
            let d = (p - c) / rad;
            let r = d.length();
            let f = (1.0 + k) / (1.0 + k * r * r);
            c + d * f * rad
        };
        let step = self.tile + Vec2::splat(self.gap);
        let ox = s.pan.x.rem_euclid(step.x);
        let oy = s.pan.y.rem_euclid(step.y);
        let cols = (rect.width() / step.x).ceil() as i32 + 2;
        let rows = (rect.height() / step.y).ceil() as i32 + 2;
        let ix0 = -((s.pan.x / step.x).floor() as i32);
        let iy0 = -((s.pan.y / step.y).floor() as i32);
        let sub = 6;
        for gy in -1..rows {
            for gx in -1..cols {
                let tile = Rect::from_min_size(pos2(rect.min.x + ox + gx as f32 * step.x - step.x * 0.5, rect.min.y + oy + gy as f32 * step.y - step.y * 0.5), self.tile);
                let (ix, iy) = (ix0 + gx, iy0 + gy);
                let idx = (ix * 3 + iy * 5).rem_euclid(8) as usize;
                let tex = if self.textures.is_empty() { PaintedScene::texture(ui.ctx(), idx) } else { self.textures[idx % self.textures.len()] };
                let uv = cover_uv(PaintedScene::ASPECT, self.tile.x / self.tile.y);
                let mut mesh = Mesh::with_texture(tex);
                let center_d = ((tile.center() - c) / rad).length();
                let shade = (1.0 - (center_d - 0.35).max(0.0) * 1.6).clamp(0.0, 1.0);
                let col = Color32::from_gray((255.0 * shade) as u8);
                for vy in 0..=sub {
                    for vx in 0..=sub {
                        let u = vx as f32 / sub as f32;
                        let v = vy as f32 / sub as f32;
                        let p = pos2(lerp(tile.min.x, tile.max.x, u), lerp(tile.min.y, tile.max.y, v));
                        mesh.vertices.push(Vertex {
                            pos: distort(p),
                            uv: pos2(lerp(uv.min.x, uv.max.x, u), lerp(uv.min.y, uv.max.y, v)),
                            color: col,
                        });
                    }
                }
                let w = (sub + 1) as u32;
                for vy in 0..sub as u32 {
                    for vx in 0..sub as u32 {
                        let a = vy * w + vx;
                        mesh.add_triangle(a, a + 1, a + w);
                        mesh.add_triangle(a + 1, a + w + 1, a + w);
                    }
                }
                painter.add(Shape::mesh(mesh));
            }
        }
        // glassy rim
        glowkit_core::paint::vignette(&painter, rect, Color32::BLACK, 0.9);
        let rim = mix(Color32::WHITE, Color32::BLACK, 0.85);
        painter.rect_stroke(rect, cr(18.0), egui::Stroke::new(1.0, rim), egui::StrokeKind::Inside);
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}
