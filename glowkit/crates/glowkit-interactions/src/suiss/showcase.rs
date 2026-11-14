//! Showcase pieces: a hover image list, a split sponsor showcase and a glass prism.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};
use glowkit_core::anim::lerp;
use glowkit_core::color::{hsv, mix, with_alpha};
use glowkit_core::{paint, Theme};
use glowkit_effects::PaintedScene;
use std::f32::consts::PI;

// ---------------------------------------------------------------------------------------------
// HoverImageList

#[derive(Clone, Copy, Default)]
struct HoverState {
    p: Vec2,
    v: Vec2,
    scale: f32,
    scale_v: f32,
    idx: usize,
    prev: usize,
    swap: f32,
    shifts: [f32; 8],
    shift_v: [f32; 8],
    init: bool,
}

/// A list of big project titles; hovering one summons a floating image that trails the pointer.
///
/// Move over the rows and a photo appears beside the cursor, following it on a soft spring so
/// it lags and catches up, tilting in the direction it is being dragged. Moving to another row
/// swaps the picture with a quick slide inside the frame, the hovered title nudges right and the
/// others dim. Leaving the list shrinks the photo away.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::HoverImageList;
/// ui.add(HoverImageList::new(&[("Northern Lights", "Field study"), ("Salt Flats", "Long exposure")]).lag(50.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct HoverImageList {
    items: Vec<(String, String)>,
    textures: Vec<egui::TextureId>,
    lag: f32,
    tilt: f32,
    image: Vec2,
    autoplay: bool,
}

impl HoverImageList {
    /// Rows of `(title, subtitle)`.
    pub fn new(items: &[(&str, &str)]) -> Self {
        Self {
            items: items.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
            textures: Vec::new(),
            lag: 50.0,
            tilt: 50.0,
            image: vec2(170.0, 120.0),
            autoplay: false,
        }
    }
    /// One image per row (default: painted scenes).
    pub fn textures(mut self, t: Vec<egui::TextureId>) -> Self {
        self.textures = t;
        self
    }
    /// How far the image trails the pointer, 0..100.
    pub fn lag(mut self, l: f32) -> Self {
        self.lag = l.clamp(0.0, 100.0);
        self
    }
    /// How much the image tilts while moving, 0..100.
    pub fn tilt(mut self, t: f32) -> Self {
        self.tilt = t.clamp(0.0, 100.0);
        self
    }
    /// Size of the floating image.
    pub fn image(mut self, s: Vec2) -> Self {
        self.image = s;
        self
    }
    /// Move a ghost pointer over the rows while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for HoverImageList {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let row_h = 74.0;
        let n = self.items.len().clamp(1, 8);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), row_h * n as f32 + 2.0), Sense::hover());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let mut st: HoverState = glowkit_core::state(ui, id.with("hoverimg"), HoverState::default);
        let idle = idle_for(ui, id, response.hovered());
        let mut pointer = response.hover_pos();
        let mut ghost = None;
        if self.autoplay && !response.hovered() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
        }
        if self.autoplay && !response.hovered() && idle > GHOST_DELAY {
            let e = (idle - GHOST_DELAY) % 8.0;
            let ky = keys(e, &[(0.0, -0.3), (0.6, 0.5), (1.8, 0.5), (2.6, 1.5), (3.6, 1.5), (4.4, 2.5), (5.6, 2.4), (6.6, 0.6), (7.2, -0.4)]);
            let kx = keys(e, &[(0.0, 0.3), (1.8, 0.42), (2.6, 0.62), (3.6, 0.5), (4.4, 0.35), (5.6, 0.6), (7.2, 0.4)]);
            let p = pos2(rect.min.x + kx * rect.width(), rect.min.y + ky * row_h);
            let a = ramp(e, 0.0, 0.4) * (1.0 - ramp(e, 6.8, 7.3));
            if rect.contains(p) {
                pointer = Some(p);
            }
            ghost = Some((p, a));
            ui.ctx().request_repaint();
        }
        let hit = pointer.filter(|p| rect.contains(*p)).map(|p| (((p.y - rect.min.y) / row_h) as usize).min(n - 1));
        if let (Some(i), Some(p)) = (hit, pointer) {
            let target = p - rect.min + vec2(24.0, -self.image.y * 0.5);
            if !st.init || st.scale < 0.05 {
                st.p = target;
                st.init = true;
            }
            if i != st.idx {
                st.prev = st.idx;
                st.idx = i;
                st.swap = 0.0;
            }
            let k = lerp(320.0, 60.0, self.lag / 100.0);
            let d = 2.0 * k.sqrt() * 0.75;
            spring_step(&mut st.p.x, &mut st.v.x, target.x, k, d, dt);
            spring_step(&mut st.p.y, &mut st.v.y, target.y, k, d, dt);
        }
        let st_t = if hit.is_some() { 1.0 } else { 0.0 };
        spring_step(&mut st.scale, &mut st.scale_v, st_t, 260.0, 20.0, dt);
        st.swap = (st.swap + dt * 4.0).min(1.0);
        for i in 0..n {
            let tgt = if hit == Some(i) { 1.0 } else { 0.0 };
            spring_step(&mut st.shifts[i], &mut st.shift_v[i], tgt, 300.0, 24.0, dt);
        }
        let moving = st.v.length() > 1.0 || !settled(st.scale, st.scale_v, st_t) || st.swap < 1.0 || st.shift_v.iter().any(|v| v.abs() > 0.01);
        let s = st;
        glowkit_core::set_state(ui, id.with("hoverimg"), st);
        if moving {
            ui.ctx().request_repaint();
        }
        let painter = ui.painter_at(rect.expand(80.0));
        let panel = if t.dark { mix(t.background, t.foreground, 0.05) } else { hexa(0xF0, 0xF0, 0xF1) };
        painter.rect_filled(rect, cr(14.0), panel);
        for (i, (title, sub)) in self.items.iter().take(n).enumerate() {
            let y = rect.min.y + i as f32 * row_h;
            if i > 0 {
                painter.line_segment([pos2(rect.min.x, y), pos2(rect.max.x, y)], Stroke::new(1.0, with_alpha(t.foreground, 0.1)));
            }
            let dim = if hit.is_some() && hit != Some(i) { 0.45 } else { 1.0 };
            let x = rect.min.x + 28.0 + s.shifts[i] * 14.0;
            text(&painter, pos2(x, y + row_h / 2.0), Align2::LEFT_CENTER, title, 30.0, with_alpha(t.foreground, dim));
            text(&painter, pos2(rect.max.x - 28.0, y + row_h / 2.0), Align2::RIGHT_CENTER, sub, 13.5, with_alpha(t.muted_foreground, dim));
        }
        if s.scale > 0.01 {
            let tilt = (s.v.x / 1500.0).clamp(-1.0, 1.0) * 0.2 * self.tilt / 50.0;
            let center = rect.min + s.p + self.image * 0.5;
            let sz = self.image * s.scale.max(0.0);
            let r = Rect::from_center_size(center, sz);
            let tex_of = |i: usize| if self.textures.is_empty() { PaintedScene::texture(ui.ctx(), (i * 3 + 2) % PaintedScene::COUNT) } else { self.textures[i % self.textures.len()] };
            let uv = cover_uv(PaintedScene::ASPECT, self.image.x / self.image.y);
            shadow(&painter, &t, r, 10.0, s.scale.clamp(0.0, 1.0));
            // swap: the new picture slides up over the old one
            let k = glowkit_core::ease(glowkit_core::Easing::OutCubic, s.swap);
            painter.add(Shape::Rect(egui::epaint::RectShape::filled(r, cr(10.0), Color32::WHITE).with_texture(tex_of(s.prev), uv).with_angle(tilt)));
            let top = Rect::from_min_max(pos2(r.min.x, lerp(r.max.y, r.min.y, k)), r.max);
            let uv_top = Rect::from_min_max(pos2(uv.min.x, lerp(uv.max.y, uv.min.y, k)), uv.max);
            if tilt.abs() < 0.02 || k >= 0.999 {
                painter.add(Shape::Rect(egui::epaint::RectShape::filled(if k >= 0.999 { r } else { top }, cr(10.0), Color32::WHITE).with_texture(tex_of(s.idx), if k >= 0.999 { uv } else { uv_top }).with_angle(tilt)));
            } else {
                painter.add(Shape::Rect(egui::epaint::RectShape::filled(r, cr(10.0), with_alpha(Color32::WHITE, k)).with_texture(tex_of(s.idx), uv).with_angle(tilt)));
            }
        }
        if let Some((p, a)) = ghost {
            ghost_touch(&painter, p, 0.0, a);
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------
// SplitShowcase

/// One half of a [`SplitShowcase`]: a wordmark, its mark and a caption.
#[derive(Clone, Debug, PartialEq)]
pub struct ShowcaseSide {
    /// Wordmark text.
    pub name: String,
    /// Small caps caption under it.
    pub caption: String,
    /// Mark style: 0 = triangle, 1 = slanted bars, 2 = ring.
    pub mark: u8,
    /// Mark color (default: the foreground).
    pub color: Option<Color32>,
}

impl ShowcaseSide {
    /// A side with a wordmark and caption.
    pub fn new(name: impl Into<String>, caption: impl Into<String>) -> Self {
        Self { name: name.into(), caption: caption.into(), mark: 0, color: None }
    }
    /// Mark style: 0 = triangle, 1 = slanted bars, 2 = ring.
    pub fn mark(mut self, m: u8, color: Option<Color32>) -> Self {
        self.mark = m;
        self.color = color;
        self
    }
}

#[derive(Clone, Copy, Default)]
struct SplitState {
    h: [f32; 2],
    hv: [f32; 2],
    press: [f32; 2],
}

/// Two partner cards joined into one pill by a dotted seam, that come apart under the pointer.
///
/// At rest the halves read as a single rounded capsule split by a dotted line. Hover a half and
/// it springs outward, peeling away from its partner: the corners along the seam round out,
/// the card brightens and lifts, while the other half gives way slightly. Pressing squishes the
/// card. Motion is a soft, Apple-style spring.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{SplitShowcase, ShowcaseSide};
/// ui.add(SplitShowcase::new(ShowcaseSide::new("Northwind", "HOSTING PARTNER"), ShowcaseSide::new("Lumen", "ANALYTICS PARTNER").mark(1, None)));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SplitShowcase {
    sides: [ShowcaseSide; 2],
    shift: f32,
    bounce: f32,
    size: Vec2,
    autoplay: bool,
}

impl SplitShowcase {
    /// A showcase of two sides.
    pub fn new(left: ShowcaseSide, right: ShowcaseSide) -> Self {
        Self { sides: [left, right], shift: 50.0, bounce: 35.0, size: vec2(560.0, 160.0), autoplay: false }
    }
    /// How far a hovered half moves out, 0..100.
    pub fn shift(mut self, s: f32) -> Self {
        self.shift = s.clamp(0.0, 100.0);
        self
    }
    /// Overshoot of the spring, 0..100.
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 100.0);
        self
    }
    /// Overall size.
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = s;
        self
    }
    /// Hover the halves with a ghost pointer while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn mark(painter: &egui::Painter, kind: u8, c: Pos2, s: f32, col: Color32) {
    match kind {
        0 => {
            painter.add(Shape::convex_polygon(vec![pos2(c.x, c.y - s * 0.55), pos2(c.x + s * 0.6, c.y + s * 0.45), pos2(c.x - s * 0.6, c.y + s * 0.45)], col, Stroke::NONE));
        }
        1 => {
            for (i, a) in [0.45, 0.75, 1.0].iter().enumerate() {
                let x = c.x - s * 0.45 + i as f32 * s * 0.38;
                let pts = vec![pos2(x + s * 0.18, c.y - s * 0.5), pos2(x + s * 0.4, c.y - s * 0.5), pos2(x + s * 0.05, c.y + s * 0.5), pos2(x - s * 0.17, c.y + s * 0.5)];
                painter.add(Shape::convex_polygon(pts, with_alpha(col, *a), Stroke::NONE));
            }
        }
        _ => {
            paint::arc(painter, c, s * 0.42, 0.0, 2.0 * PI, s * 0.16, col);
            painter.circle_filled(c, s * 0.12, col);
        }
    }
}

impl egui::Widget for SplitShowcase {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = self.size.x.min(ui.available_width());
        let (rect, response) = ui.allocate_exact_size(vec2(w, self.size.y + 30.0), Sense::click());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let mut st: SplitState = glowkit_core::state(ui, id.with("split"), SplitState::default);
        let pill = Rect::from_center_size(rect.center(), vec2(w - 40.0, self.size.y));
        let idle = idle_for(ui, id, response.hovered());
        let mut pointer = response.hover_pos();
        let mut ghost = None;
        if self.autoplay && !response.hovered() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
        }
        if self.autoplay && !response.hovered() && idle > GHOST_DELAY {
            let e = (idle - GHOST_DELAY) % 6.0;
            let kx = keys(e, &[(0.0, 0.5), (0.5, 0.27), (2.2, 0.3), (2.9, 0.72), (4.6, 0.7), (5.4, 0.5)]);
            let ky = keys(e, &[(0.0, 1.25), (0.5, 0.55), (2.9, 0.45), (4.6, 0.5), (5.4, 1.3)]);
            let p = pos2(pill.min.x + kx * pill.width(), pill.min.y + ky * pill.height());
            if pill.contains(p) {
                pointer = Some(p);
            }
            ghost = Some((p, ramp(e, 0.0, 0.4) * (1.0 - ramp(e, 5.0, 5.5))));
            ui.ctx().request_repaint();
        }
        let hovered = pointer.filter(|p| pill.expand(8.0).contains(*p)).map(|p| if p.x < pill.center().x { 0 } else { 1 });
        let pressing = response.is_pointer_button_down_on();
        let mut moving = false;
        for k in 0..2 {
            let tgt = if hovered == Some(k) { 1.0 } else if hovered.is_some() { -0.35 } else { 0.0 };
            spring_step(&mut st.h[k], &mut st.hv[k], tgt, 220.0, damping_for(self.bounce, 220.0), dt);
            let pt = if pressing && hovered == Some(k) { 1.0 } else { 0.0 };
            st.press[k] += (pt - st.press[k]) * (dt * 14.0).min(1.0);
            moving |= !settled(st.h[k], st.hv[k], tgt) || (st.press[k] - pt).abs() > 0.01;
        }
        let s = st;
        glowkit_core::set_state(ui, id.with("split"), st);
        if moving {
            ui.ctx().request_repaint();
        }
        let painter = ui.painter_at(rect.expand(20.0));
        let base = if t.dark { mix(t.background, t.foreground, 0.08) } else { hexa(0xE4, 0xE4, 0xE7) };
        let lift = if t.dark { mix(t.background, t.foreground, 0.13) } else { hexa(0xEE, 0xEE, 0xF0) };
        let rad = self.size.y * 0.24;
        let shift = self.shift / 100.0 * 22.0;
        for k in 0..2 {
            let h = s.h[k];
            let dir = if k == 0 { -1.0 } else { 1.0 };
            let half = Rect::from_min_max(
                if k == 0 { pill.min } else { pos2(pill.center().x, pill.min.y) },
                if k == 0 { pos2(pill.center().x, pill.max.y) } else { pill.max },
            );
            let pos_h = h.max(0.0);
            let gap = pos_h * 8.0;
            let mut r = half.translate(vec2(dir * (h * shift + gap * 0.5), 0.0));
            // the inner edge pulls back so the seam opens
            if k == 0 {
                r.max.x -= gap * 0.5;
            } else {
                r.min.x += gap * 0.5;
            }
            let r = Rect::from_center_size(r.center(), r.size() * (1.0 - 0.03 * s.press[k]) * (1.0 + 0.02 * pos_h));
            let inner = (pos_h * rad).round().clamp(0.0, 255.0) as u8;
            let outer = rad.round() as u8;
            let corners = if k == 0 {
                egui::CornerRadius { nw: outer, sw: outer, ne: inner, se: inner }
            } else {
                egui::CornerRadius { nw: inner, sw: inner, ne: outer, se: outer }
            };
            if pos_h > 0.02 {
                let a = if t.dark { 0.5 } else { 0.12 };
                painter.add(Shape::Rect(egui::epaint::RectShape::filled(r.translate(vec2(0.0, 6.0 * pos_h)), corners, with_alpha(Color32::BLACK, a * pos_h.min(1.0))).with_blur_width(24.0 * pos_h)));
            }
            painter.rect_filled(r, corners, mix(base, lift, pos_h.min(1.0)));
            let side = &self.sides[k];
            let col = side.color.unwrap_or(t.foreground);
            let nw = text_width(&painter, &side.name, 30.0);
            let total = nw + 40.0;
            let cx = r.center().x;
            let cy = r.center().y - 12.0;
            mark(&painter, side.mark, pos2(cx - total / 2.0 + 16.0, cy), 30.0, if side.mark == 1 { side.color.unwrap_or(hexa(0x4F, 0x7C, 0xFF)) } else { col });
            text_bold(&painter, pos2(cx - total / 2.0 + 40.0, cy), Align2::LEFT_CENTER, &side.name, 30.0, t.foreground);
            let cap: String = side.caption.chars().flat_map(|c| [c, '\u{200A}']).collect();
            text(&painter, pos2(cx, cy + 40.0), Align2::CENTER_CENTER, cap, 11.5, t.muted_foreground);
        }
        // dotted seam, fading as the halves part
        let open = s.h[0].max(s.h[1]).max(0.0);
        let sx = pill.center().x + (s.h[1] - s.h[0]) * shift * 0.5;
        let dots = ((pill.height() - 60.0) / 6.0) as i32;
        for i in 0..dots {
            let y = pill.min.y + 30.0 + i as f32 * 6.0;
            painter.circle_filled(pos2(sx, y), 1.1, with_alpha(t.muted_foreground, 0.8 * (1.0 - open * 1.5).clamp(0.0, 1.0)));
        }
        if let Some((p, a)) = ghost {
            ghost_touch(&painter, p, 0.0, a);
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------
// PrismBeam

#[derive(Clone, Copy, Default)]
struct PrismState {
    src: Vec2,
    src_v: Vec2,
    target: Vec2,
    init: bool,
    g: Gesture,
}

/// A glass prism that splits a beam of white light into a spectrum. Drag to aim the beam.
///
/// A thin white beam enters from the left, refracts into the triangular prism (real Snell's law
/// at each face), and leaves the far face fanned out into a rainbow: each color bends by a
/// slightly different index of refraction, so the fan widens and turns as you move the source.
/// Press and drag anywhere to move the light source; it follows on a spring and the spectrum
/// sweeps, flares, and vanishes into total internal reflection at steep angles.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::PrismBeam;
/// ui.add(PrismBeam::new().dispersion(60.0).glow(50.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PrismBeam {
    height: f32,
    dispersion: f32,
    glow: f32,
    size: f32,
    autoplay: bool,
}

impl Default for PrismBeam {
    fn default() -> Self {
        Self::new()
    }
}

impl PrismBeam {
    /// A prism scene.
    pub fn new() -> Self {
        Self { height: 360.0, dispersion: 60.0, glow: 50.0, size: 190.0, autoplay: false }
    }
    /// Scene height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(160.0);
        self
    }
    /// How widely colors separate, 0..100.
    pub fn dispersion(mut self, d: f32) -> Self {
        self.dispersion = d.clamp(0.0, 100.0);
        self
    }
    /// Bloom of the spectrum, 0..100.
    pub fn glow(mut self, g: f32) -> Self {
        self.glow = g.clamp(0.0, 100.0);
        self
    }
    /// Side length of the prism.
    pub fn prism(mut self, s: f32) -> Self {
        self.size = s.max(60.0);
        self
    }
    /// Sweep the light source on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn ray_seg(o: Pos2, d: Vec2, a: Pos2, b: Pos2) -> Option<(f32, Vec2)> {
    let e = b - a;
    let den = d.x * e.y - d.y * e.x;
    if den.abs() < 1e-6 {
        return None;
    }
    let w = a - o;
    let t = (w.x * e.y - w.y * e.x) / den;
    let u = (w.x * d.y - w.y * d.x) / den;
    if t > 1e-3 && (0.0..=1.0).contains(&u) {
        let n = vec2(-e.y, e.x).normalized();
        Some((t, n))
    } else {
        None
    }
}

fn refract(i: Vec2, n: Vec2, eta: f32) -> Option<Vec2> {
    let n = if i.dot(n) > 0.0 { -n } else { n };
    let cos_i = -i.dot(n);
    let k = 1.0 - eta * eta * (1.0 - cos_i * cos_i);
    if k < 0.0 {
        None
    } else {
        Some((i * eta + n * (eta * cos_i - k.sqrt())).normalized())
    }
}

fn first_hit(o: Pos2, d: Vec2, tri: &[Pos2; 3], skip: Option<usize>) -> Option<(f32, Vec2, usize)> {
    let mut best: Option<(f32, Vec2, usize)> = None;
    for k in 0..3 {
        if Some(k) == skip {
            continue;
        }
        if let Some((t, n)) = ray_seg(o, d, tri[k], tri[(k + 1) % 3]) {
            if best.is_none_or(|b| t < b.0) {
                best = Some((t, n, k));
            }
        }
    }
    best
}

const PRISM_SCRIPT: [Stroke2; 2] = [drag(1.0, 3.2, (0.1, 0.62), (0.1, 0.3)), drag(4.4, 6.4, (0.12, 0.3), (0.16, 0.72))];

impl egui::Widget for PrismBeam {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let touch = touch(ui, &response, rect, self.autoplay, &PRISM_SCRIPT, 8.0);
        let mut st: PrismState = glowkit_core::state(ui, id.with("prism"), PrismState::default);
        let default_src = vec2(0.0, rect.height() * 0.55);
        if !st.init {
            st.init = true;
            st.src = default_src;
            st.target = default_src;
        }
        st.g.update(&touch, now, dt);
        if st.g.down {
            st.target = st.g.last - rect.min;
            st.target.x = st.target.x.clamp(0.0, rect.width() * 0.4);
        }
        spring_step(&mut st.src.x, &mut st.src_v.x, st.target.x, 120.0, 14.0, dt);
        spring_step(&mut st.src.y, &mut st.src_v.y, st.target.y, 120.0, 14.0, dt);
        let s = st;
        glowkit_core::set_state(ui, id.with("prism"), st);
        if s.src_v.length() > 0.5 || s.g.down || touch.ghost {
            ui.ctx().request_repaint();
        }
        let painter = ui.painter_at(rect);
        let c = rect.center() + vec2(-rect.width() * 0.04, 6.0);
        let side = self.size;
        let hgt = side * 3f32.sqrt() / 2.0;
        let tri = [pos2(c.x, c.y - hgt * 2.0 / 3.0), pos2(c.x + side / 2.0, c.y + hgt / 3.0), pos2(c.x - side / 2.0, c.y + hgt / 3.0)];
        let src = rect.min + s.src;
        // aim at the middle of the left face
        let aim = tri[2].lerp(tri[0], 0.45);
        let dir = (aim - src).normalized();
        let beam_col = if t.dark { Color32::from_gray(235) } else { Color32::from_gray(150) };
        let glow_k = self.glow / 100.0;
        let mut entry = None;
        if let Some((t0, n0, k0)) = first_hit(src, dir, &tri, None) {
            let e = src + dir * t0;
            painter.line_segment([src, e], Stroke::new(3.0, beam_col));
            painter.line_segment([src, e], Stroke::new(9.0, with_alpha(beam_col, 0.12)));
            entry = Some((e, n0, k0));
        } else {
            painter.line_segment([src, src + dir * rect.width()], Stroke::new(3.0, beam_col));
        }
        // glass body
        let pts = tri.to_vec();
        if !t.dark {
            painter.add(Shape::convex_polygon(pts.iter().map(|p| *p + vec2(0.0, 8.0)).collect(), with_alpha(Color32::BLACK, 0.05), Stroke::NONE));
        }
        let bounds = Rect::from_points(&pts);
        let shade_a = if t.dark { 0.10 } else { 0.95 };
        paint::fill_convex_fn(&painter, &pts, 3, |p| {
            let u = ((p.y - bounds.min.y) / bounds.height()).clamp(0.0, 1.0);
            let v = ((p.x - bounds.min.x) / bounds.width()).clamp(0.0, 1.0);
            let base = if t.dark { Color32::from_gray((40.0 + 30.0 * (1.0 - u)) as u8) } else { Color32::from_gray((250.0 - 30.0 * u - 12.0 * v) as u8) };
            with_alpha(base, shade_a)
        });
        let edge = if t.dark { with_alpha(Color32::WHITE, 0.35) } else { with_alpha(Color32::WHITE, 0.95) };
        painter.add(Shape::closed_line(pts.clone(), Stroke::new(1.5, edge)));
        painter.line_segment([tri[0], tri[1]], Stroke::new(1.0, with_alpha(Color32::BLACK, if t.dark { 0.0 } else { 0.06 })));
        // inside + dispersion
        if let Some((e, n0, k0)) = entry {
            let base_n = 1.5;
            let spread = 0.002 + self.dispersion / 100.0 * 0.05;
            let bands = 7;
            let mut exits: Vec<(Pos2, Vec2, f32)> = Vec::new();
            for b in 0..bands {
                let f = b as f32 / (bands - 1) as f32;
                let n_glass = base_n + (f - 0.5) * spread * 2.0;
                if let Some(d1) = refract(dir, n0, 1.0 / n_glass) {
                    if let Some((t1, n1, _)) = first_hit(e, d1, &tri, Some(k0)) {
                        let x = e + d1 * t1;
                        if b == bands / 2 {
                            painter.line_segment([e, x], Stroke::new(2.0, with_alpha(beam_col, 0.55)));
                        }
                        if let Some(d2) = refract(d1, n1, n_glass) {
                            exits.push((x, d2, f));
                        }
                    }
                }
            }
            let len = rect.width();
            // glow underlay then crisp bands, red (f=0) to violet (f=1)
            for pass in 0..2 {
                for w in exits.windows(2) {
                    let (x0, d0, f0) = w[0];
                    let (x1, d1, _) = w[1];
                    let col = hsv(f0 * 280.0, 0.85, 1.0);
                    let far0 = x0 + d0 * len;
                    let far1 = x1 + d1 * len;
                    let quad = vec![x0, far0, far1, x1];
                    let a = if pass == 0 { 0.18 * glow_k } else { 0.7 };
                    let quad = if pass == 0 {
                        let mid = (far0 + far1.to_vec2()) / 2.0;
                        let wide = (far1 - far0) * (1.0 + glow_k);
                        vec![x0, mid - wide / 2.0, mid + wide / 2.0, x1]
                    } else {
                        quad
                    };
                    // fade along the length
                    let mut mesh = egui::epaint::Mesh::default();
                    let ca = with_alpha(col, a);
                    let cb = with_alpha(col, a * 0.15);
                    mesh.colored_vertex(quad[0], ca);
                    mesh.colored_vertex(quad[1], cb);
                    mesh.colored_vertex(quad[2], cb);
                    mesh.colored_vertex(quad[3], ca);
                    mesh.add_triangle(0, 1, 2);
                    mesh.add_triangle(0, 2, 3);
                    painter.add(Shape::mesh(mesh));
                }
            }
            if let (Some(first), Some(last)) = (exits.first(), exits.last()) {
                paint::glow(&painter, first.0.lerp(last.0, 0.5), 22.0 + 20.0 * glow_k, with_alpha(Color32::WHITE, 0.5));
                // rainbow glints on the inner faces
                let g0 = tri[1].lerp(tri[2], 0.55);
                for i in 0..5 {
                    let f = i as f32 / 4.0;
                    let p = g0 + vec2(f * 26.0 - 13.0, -3.0);
                    painter.circle_filled(p, 2.4, with_alpha(hsv(f * 280.0, 0.8, 1.0), 0.6));
                }
            }
        }
        // source handle
        let pulse = 0.5 + 0.5 * (now * 3.0).sin();
        painter.circle_filled(src, 5.0 + pulse, with_alpha(beam_col, 0.25));
        let label = lerp(0.0, 1.0, 1.0 - ramp(touch.alpha, 0.0, 0.2));
        text(&painter, pos2(rect.min.x + 14.0, rect.max.y - 16.0), Align2::LEFT_CENTER, "Drag to aim the light", 12.0, with_alpha(t.muted_foreground, 0.7 * label.max(0.4)));
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}
