//! Glowing interactive fields: fibre cables, a pointer ring field, a wireframe valley, light
//! ribbons, a beam junction with lightning, and a real (tiny) fluid simulation.

use super::common::{h01, lerp_rgb, lum, pointer_or_ghost, ramp, strip, Dots};
use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{pos2, vec2, Color32, Pos2, Rect, Response, Sense, Shape, Ui, Vec2, Widget};
use glowkit_core::color::{hex, with_alpha};
use glowkit_core::noise::noise3;
use glowkit_core::{paint, Theme};
use std::f32::consts::{PI, TAU};

fn alloc(ui: &mut Ui, size: Option<Vec2>, h: f32, sense: Sense) -> (Rect, Response) {
    let s = glowkit_core::desired_size(ui, size, h);
    ui.allocate_exact_size(s, sense)
}

/// Smoothly followed pointer (or ghost) position stored per widget.
fn follow_pos(ui: &Ui, id: egui::Id, target: Pos2, speed: f32) -> (Pos2, Vec2) {
    let dt = glowkit_core::dt(ui);
    glowkit_core::with_state(ui, id.with("origin_follow"), || (target, Vec2::ZERO), |(p, v)| {
        let np = *p + (target - *p) * (1.0 - (-dt * speed).exp());
        *v = (np - *p) / dt.max(1e-3);
        *p = np;
        (*p, *v)
    })
}

// ---------------------------------------------------------------------------------------------
// Light Cables
// ---------------------------------------------------------------------------------------------

/// A bundle of glowing fibre cables: they rise from a wide fan at the bottom, fold through one
/// bend and gather into a single bright point, while pulses of light run up them. The cables
/// lean toward the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::LightCables::new().cables(28));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct LightCables {
    size: Option<Vec2>,
    cables: usize,
    pulses: f32,
    bend: f32,
    colors: Option<[Color32; 2]>,
    background: Option<Color32>,
    ghost: bool,
}

impl Default for LightCables {
    fn default() -> Self {
        Self::new()
    }
}

impl LightCables {
    /// Cables with default settings.
    pub fn new() -> Self {
        Self { size: None, cables: 28, pulses: 1.0, bend: 0.5, colors: None, background: None, ghost: true }
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Number of cables (4..48).
    pub fn cables(mut self, n: usize) -> Self {
        self.cables = n.clamp(4, 48);
        self
    }
    /// Pulse speed multiplier (0 = no pulses).
    pub fn pulses(mut self, k: f32) -> Self {
        self.pulses = k.max(0.0);
        self
    }
    /// How sharply the bundle folds (0..1).
    pub fn bend(mut self, k: f32) -> Self {
        self.bend = k.clamp(0.0, 1.0);
        self
    }
    /// Gradient from the outer cables to the core.
    pub fn colors(mut self, outer: Color32, core: Color32) -> Self {
        self.colors = Some([outer, core]);
        self
    }
    /// Background fill (default: the theme background).
    pub fn background(mut self, c: Color32) -> Self {
        self.background = Some(c);
        self
    }
    /// Drift a ghost pointer while idle (default on).
    pub fn ghost(mut self, on: bool) -> Self {
        self.ghost = on;
        self
    }

    /// Paint into `rect`; the cables lean toward `pointer`.
    pub fn paint(&self, ui: &Ui, rect: Rect, pointer: Option<Pos2>) {
        let t = Theme::of(ui);
        let time = glowkit_core::time(ui);
        let painter = ui.painter_at(rect);
        let bg = self.background.unwrap_or(t.background);
        painter.rect_filled(rect, 0.0, bg);
        let add = t.dark;
        let [outer, core] = self.colors.unwrap_or(if add {
            [hex("#FF6A00"), hex("#FFE14D")]
        } else {
            [hex("#EA580C"), hex("#F59E0B")]
        });
        let n = self.cables;
        let apex = pos2(rect.center().x, rect.min.y + rect.height() * 0.1);
        let seg = 48;
        let mut glow = Mesh::default();
        let mut cores = Mesh::default();
        let mut pts = Vec::with_capacity(seg + 1);
        for i in 0..n {
            let f = if n > 1 { i as f32 / (n - 1) as f32 * 2.0 - 1.0 } else { 0.0 };
            let sway = (time * 0.6 + i as f32 * 0.7).sin() * 6.0;
            let base = pos2(rect.center().x + f * rect.width() * 0.36 + sway, rect.max.y + 30.0);
            let bend_y = rect.min.y + rect.height() * (0.42 + 0.1 * self.bend);
            let c1 = pos2(rect.center().x + f * rect.width() * 0.2 * (1.0 - self.bend * 0.6), bend_y + 40.0);
            let c2 = pos2(apex.x + f * 10.0, apex.y + rect.height() * 0.35);
            pts.clear();
            for k in 0..=seg {
                let s = k as f32 / seg as f32;
                let q = 1.0 - s;
                let mut p = (base.to_vec2() * q * q * q
                    + c1.to_vec2() * 3.0 * q * q * s
                    + c2.to_vec2() * 3.0 * q * s * s
                    + apex.to_vec2() * s * s * s)
                    .to_pos2();
                if let Some(m) = pointer {
                    let d = m - p;
                    let w = (-(d.length_sq()) / (rect.height() * rect.height() * 0.08)).exp();
                    p += d * 0.35 * w * (s * PI).sin();
                }
                pts.push(p);
            }
            let center = 1.0 - f.abs();
            let col = lerp_rgb(outer, core, center.powf(0.7));
            let fade = |k: usize| {
                let s = k as f32 / seg as f32;
                (s * 2.2).min(1.0) * (0.35 + 0.65 * s)
            };
            let a_glow = if add { 0.10 } else { 0.07 };
            strip(&mut glow, &pts, |_| 16.0, |k| lum(col, a_glow * fade(k), add), |_| Color32::TRANSPARENT);
            strip(&mut glow, &pts, |_| 5.0, |k| lum(col, a_glow * 2.0 * fade(k), add), |_| Color32::TRANSPARENT);
            // pulses: a bright travelling window along the cable
            let ph = h01(i as u64 * 31 + 5);
            let head = (time * 0.35 * self.pulses * (0.7 + 0.6 * ph) + ph) % 1.4;
            let core_col = |k: usize| {
                let s = k as f32 / seg as f32;
                let d = (s - head).abs();
                let pulse = if self.pulses > 0.0 { (1.0 - d / 0.12).max(0.0).powi(2) } else { 0.0 };
                lum(lerp_rgb(col, Color32::WHITE, pulse * 0.6), (0.55 * fade(k) + pulse).min(1.0), add)
            };
            strip(&mut cores, &pts, |k| 1.3 + 2.0 * (k as f32 / seg as f32), core_col, |_| Color32::TRANSPARENT);
        }
        painter.add(Shape::mesh(glow));
        painter.add(Shape::mesh(cores));
        // apex bloom
        if add {
            paint::glow(&painter, apex, 70.0, lum(core, 0.5, true));
            paint::glow(&painter, apex, 18.0, lum(Color32::WHITE, 0.8, true));
        } else {
            paint::glow(&painter, apex, 60.0, with_alpha(core, 0.35));
            paint::glow(&painter, apex, 14.0, with_alpha(outer, 0.6));
        }
    }
}

impl Widget for LightCables {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = alloc(ui, self.size, 380.0, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let time = glowkit_core::time(ui);
        let (p, _) = pointer_or_ghost(ui, rect, &response, time, self.ghost);
        let p = p.map(|p| follow_pos(ui, response.id, p, 6.0).0);
        self.paint(ui, rect, p);
        response
    }
}

// ---------------------------------------------------------------------------------------------
// Cursor Ring Field
// ---------------------------------------------------------------------------------------------

/// A field of small scattered capsules lit by a soft ring that trails the pointer: capsules on the
/// ring brighten, swell and get pushed outward, then drift back into place as the ring moves on.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::CursorRingField::new().ring_radius(110.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CursorRingField {
    size: Option<Vec2>,
    spacing: f32,
    ring: f32,
    push: f32,
    colors: Option<[Color32; 2]>,
    background: Option<Color32>,
    ghost: bool,
}

impl Default for CursorRingField {
    fn default() -> Self {
        Self::new()
    }
}

impl CursorRingField {
    /// A field with default settings.
    pub fn new() -> Self {
        Self { size: None, spacing: 22.0, ring: 110.0, push: 1.0, colors: None, background: None, ghost: true }
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Average distance between capsules in points (12..60).
    pub fn spacing(mut self, s: f32) -> Self {
        self.spacing = s.clamp(12.0, 60.0);
        self
    }
    /// Radius of the light ring in points.
    pub fn ring_radius(mut self, r: f32) -> Self {
        self.ring = r.max(20.0);
        self
    }
    /// How far the ring pushes capsules outward (0..3).
    pub fn push(mut self, k: f32) -> Self {
        self.push = k.clamp(0.0, 3.0);
        self
    }
    /// Resting and lit capsule colors.
    pub fn colors(mut self, rest: Color32, lit: Color32) -> Self {
        self.colors = Some([rest, lit]);
        self
    }
    /// Background fill (default: the theme background).
    pub fn background(mut self, c: Color32) -> Self {
        self.background = Some(c);
        self
    }
    /// Drift a ghost pointer while idle (default on).
    pub fn ghost(mut self, on: bool) -> Self {
        self.ghost = on;
        self
    }

    /// Paint into `rect` with the ring centered at `ring` (moving with velocity `vel`).
    pub fn paint(&self, ui: &Ui, rect: Rect, ring: Option<Pos2>, vel: Vec2) {
        let t = Theme::of(ui);
        let time = glowkit_core::time(ui);
        let painter = ui.painter_at(rect);
        let bg = self.background.unwrap_or(t.background);
        painter.rect_filled(rect, 0.0, bg);
        let add = t.dark;
        let [rest, lit] = self.colors.unwrap_or(if add {
            [hex("#2563EB"), hex("#5EEAD4")]
        } else {
            [hex("#93C5FD"), hex("#0D9488")]
        });
        let sp = self.spacing;
        let cols = (rect.width() / sp).ceil() as i32 + 1;
        let rows = (rect.height() / sp).ceil() as i32 + 1;
        let mut mesh = Mesh::default();
        let speed = vel.length();
        let ringr = self.ring * (1.0 + (speed / 2500.0).min(0.35));
        for j in 0..rows {
            for i in 0..cols {
                let seed = (j as u64) * 7_919 + i as u64 * 104_729 + 17;
                let base = pos2(
                    rect.min.x + (i as f32 + 0.5 + (h01(seed) - 0.5) * 0.8) * sp,
                    rect.min.y + (j as f32 + 0.5 + (h01(seed + 1) - 0.5) * 0.8) * sp,
                );
                // gentle idle drift
                let drift = vec2(
                    noise3(base.x * 0.01, base.y * 0.01, time * 0.15),
                    noise3(base.x * 0.01 + 9.0, base.y * 0.01, time * 0.15),
                ) * 4.0;
                let mut p = base + drift;
                let mut glow = 0.0;
                let ang = noise3(base.x * 0.004, base.y * 0.004, 3.0) * PI;
                let mut dir = vec2(ang.cos(), ang.sin());
                if let Some(c) = ring {
                    let d = p - c;
                    let l = d.length().max(1e-3);
                    let band = (-((l - ringr) / (ringr * 0.32)).powi(2)).exp();
                    let inside = (1.0 - l / ringr).clamp(0.0, 1.0);
                    glow = band;
                    p += d / l * (band * 26.0 + inside * 14.0) * self.push;
                    // capsules on the ring align tangentially
                    let tangent = d.rot90() / l;
                    dir = (dir * (1.0 - band) + tangent * band).normalized();
                }
                let len = 3.0 + 4.0 * glow + 1.5 * h01(seed + 2);
                let w = 1.8 + 1.2 * glow;
                let c = lerp_rgb(rest, lit, glow);
                let a = if add { 0.55 + 0.45 * glow } else { 0.5 + 0.5 * glow };
                let a = a * (0.7 + 0.3 * h01(seed + 3));
                let col = lum(c, a, add);
                let pts = [p - dir * len * 0.5, p + dir * len * 0.5];
                strip(&mut mesh, &pts, |_| w, |_| col, |_| Color32::TRANSPARENT);
            }
        }
        if let Some(c) = ring {
            if add {
                paint::glow(&painter, c, ringr * 1.4, lum(lit, 0.07, true));
            }
        }
        painter.add(Shape::mesh(mesh));
    }
}

impl Widget for CursorRingField {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = alloc(ui, self.size, 380.0, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let time = glowkit_core::time(ui);
        let (p, _) = pointer_or_ghost(ui, rect, &response, time, self.ghost);
        let fp = p.map(|p| follow_pos(ui, response.id, p, 7.0));
        self.paint(ui, rect, fp.map(|f| f.0), fp.map(|f| f.1).unwrap_or_default());
        response
    }
}

// ---------------------------------------------------------------------------------------------
// Wire Terrain
// ---------------------------------------------------------------------------------------------

/// An endless flight down a wireframe valley toward a banded horizon sun. Ridges rise on both
/// sides with proper hidden-line occlusion, and the pointer steers the camera.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::WireTerrain::new().speed(1.0).ridges(1.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct WireTerrain {
    size: Option<Vec2>,
    speed: f32,
    ridges: f32,
    sun: bool,
    line: Option<Color32>,
    background: Option<Color32>,
}

impl Default for WireTerrain {
    fn default() -> Self {
        Self::new()
    }
}

impl WireTerrain {
    /// A valley with default settings.
    pub fn new() -> Self {
        Self { size: None, speed: 1.0, ridges: 1.0, sun: true, line: None, background: None }
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Flight speed multiplier.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Ridge height multiplier (0..2).
    pub fn ridges(mut self, k: f32) -> Self {
        self.ridges = k.clamp(0.0, 2.0);
        self
    }
    /// Show the banded sun (default on).
    pub fn sun(mut self, on: bool) -> Self {
        self.sun = on;
        self
    }
    /// Wire color.
    pub fn line_color(mut self, c: Color32) -> Self {
        self.line = Some(c);
        self
    }
    /// Background fill (default: near-black on dark themes, the theme background on light).
    pub fn background(mut self, c: Color32) -> Self {
        self.background = Some(c);
        self
    }

    /// Paint into `rect`; `steer` in -1..1 pans the camera sideways.
    pub fn paint(&self, ui: &Ui, rect: Rect, steer: Vec2) {
        let t = Theme::of(ui);
        let time = glowkit_core::time(ui) * self.speed;
        let painter = ui.painter_at(rect);
        let add = t.dark;
        let bg = self.background.unwrap_or(if add { hex("#050208") } else { t.background });
        let line = self.line.unwrap_or(if add { hex("#FF6A1A") } else { hex("#DB2777") });
        let horizon = rect.min.y + rect.height() * (0.42 - steer.y * 0.04);
        // sky
        let sky_top = if add { hex("#0A0410") } else { lerp_rgb(bg, hex("#FDE68A"), 0.0) };
        let sky_low = if add { hex("#2A0A12") } else { lerp_rgb(bg, hex("#FBCFE8"), 0.7) };
        paint::shader_grid(&painter, Rect::from_min_max(rect.min, pos2(rect.max.x, horizon + 2.0)), 1, 8, |uv| {
            lerp_rgb(sky_top, sky_low, uv.y * uv.y)
        });
        painter.rect_filled(Rect::from_min_max(pos2(rect.min.x, horizon), rect.max), 0.0, bg);
        // sun with bands
        if self.sun {
            let r = rect.height() * 0.3;
            let c = pos2(rect.center().x - steer.x * 30.0, horizon - r * 0.3);
            let top = if add { hex("#FFB347") } else { hex("#FDBA74") };
            let bot = if add { hex("#FF3D2E") } else { hex("#F472B6") };
            paint::glow(&painter, c, r * 1.9, with_alpha(bot, if add { 0.25 } else { 0.2 }));
            let pts = paint::circle_points(c, r, 96);
            paint::fill_convex_fn(&painter, &pts, 4, |p| lerp_rgb(top, bot, ((p.y - (c.y - r)) / (2.0 * r)).clamp(0.0, 1.0)));
            // stripes cut through the lower half, thicker toward the bottom, scrolling down
            for k in 0..7 {
                let f = ((k as f32 + (time * 0.4) % 1.0) / 7.0).clamp(0.0, 1.0);
                let y = c.y + r * (0.05 + f * 0.95);
                let th = 1.0 + f * 7.0;
                painter.rect_filled(
                    Rect::from_min_max(pos2(c.x - r - 2.0, y - th * 0.5), pos2(c.x + r + 2.0, y + th * 0.5)),
                    0.0,
                    lerp_rgb(sky_low, bg, ((y - c.y + r * 0.3) / (r * 0.6)).clamp(0.0, 1.0)),
                );
            }
            painter.rect_filled(Rect::from_min_max(pos2(rect.min.x, horizon), rect.max), 0.0, bg);
        }
        // terrain
        let cols = 44;
        let rows = 30;
        let dz = 0.55;
        let travel = time * 1.6;
        let zoff = travel % dz;
        let cam_x = steer.x * 1.2;
        let cam_y = 1.0;
        let focal = rect.height() * 0.9;
        let height = |x: f32, z: f32| -> f32 {
            let ax = x.abs();
            let valley = ((ax - 1.4) / 3.0).clamp(0.0, 1.0);
            let n = noise3(x * 0.32, (z + travel) * 0.32, 0.5) * 0.5 + 0.5;
            let n2 = noise3(x * 0.9, (z + travel) * 0.9, 2.0) * 0.5 + 0.5;
            valley * valley * (n * 3.2 + n2 * 0.7) * self.ridges
        };
        let proj = |x: f32, y: f32, z: f32| -> Pos2 {
            let zz = z.max(0.05);
            pos2(rect.center().x + (x - cam_x) / zz * focal, horizon + (cam_y - y) / zz * focal)
        };
        let half = 9.0;
        let mut grid = vec![Pos2::ZERO; (cols + 1) * (rows + 1)];
        for r in 0..=rows {
            let z = 0.6 + r as f32 * dz - zoff;
            for c in 0..=cols {
                let x = -half + c as f32 / cols as f32 * half * 2.0;
                grid[r * (cols + 1) + c] = proj(x, height(x, z), z);
            }
        }
        let mut mesh = Mesh::default();
        let fade = |r: usize| -> f32 { 1.0 - (r as f32 / rows as f32).powf(1.6) };
        let w = cols + 1;
        for r in (0..rows).rev() {
            // occluding fill of this strip
            let a = fade(r);
            let fill = bg;
            let base = mesh.vertices.len() as u32;
            for c in 0..=cols {
                mesh.vertices.push(Vertex { pos: grid[r * w + c], uv: WHITE_UV, color: fill });
                mesh.vertices.push(Vertex { pos: grid[(r + 1) * w + c], uv: WHITE_UV, color: fill });
            }
            for c in 0..cols as u32 {
                let i = base + c * 2;
                mesh.add_triangle(i, i + 1, i + 3);
                mesh.add_triangle(i, i + 3, i + 2);
            }
            // far row line + column segments of this strip
            let col_far = lum(line, a * fade(r + 1).max(0.0) * 0.9 + 0.05, false);
            let col_near = lum(line, a * 0.95, false);
            let row_pts: Vec<Pos2> = (0..=cols).map(|c| grid[(r + 1) * w + c]).collect();
            strip(&mut mesh, &row_pts, |_| 1.0, |_| col_far, |_| Color32::TRANSPARENT);
            for c in 0..=cols {
                let seg = [grid[(r + 1) * w + c], grid[r * w + c]];
                strip(&mut mesh, &seg, |_| 1.0, |k| if k == 0 { col_far } else { col_near }, |_| Color32::TRANSPARENT);
            }
            if r == 0 {
                let near: Vec<Pos2> = (0..=cols).map(|c| grid[c]).collect();
                strip(&mut mesh, &near, |_| 1.2, |_| col_near, |_| Color32::TRANSPARENT);
            }
        }
        painter.add(Shape::mesh(mesh));
        // horizon haze
        let haze = if add { with_alpha(line, 0.25) } else { with_alpha(line, 0.15) };
        paint::glow_ellipse(&painter, pos2(rect.center().x, horizon), vec2(rect.width() * 0.6, 14.0), haze);
    }
}

impl Widget for WireTerrain {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = alloc(ui, self.size, 380.0, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let time = glowkit_core::time(ui);
        let target = match response.hover_pos() {
            Some(p) => ((p - rect.center()) / (rect.size() * 0.5)).clamp(Vec2::splat(-1.0), Vec2::splat(1.0)),
            None => vec2((time * 0.3).sin() * 0.6, 0.0),
        };
        let s = glowkit_core::anim::spring2(ui, response.id.with("steer"), target, 30.0, 10.0);
        self.paint(ui, rect, s);
        response
    }
}

// ---------------------------------------------------------------------------------------------
// Ribbon Glow
// ---------------------------------------------------------------------------------------------

/// Layered ribbons of light sweeping across the frame, twisting (they narrow and widen as they
/// turn) and bending into a bright swirl around the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::RibbonGlow::new().ribbons(6));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct RibbonGlow {
    size: Option<Vec2>,
    ribbons: usize,
    speed: f32,
    width: f32,
    angle: f32,
    colors: Option<Vec<Color32>>,
    background: Option<Color32>,
    ghost: bool,
}

impl Default for RibbonGlow {
    fn default() -> Self {
        Self::new()
    }
}

impl RibbonGlow {
    /// Ribbons with default settings.
    pub fn new() -> Self {
        Self { size: None, ribbons: 6, speed: 1.0, width: 1.0, angle: -0.45, colors: None, background: None, ghost: true }
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Number of ribbons (1..12).
    pub fn ribbons(mut self, n: usize) -> Self {
        self.ribbons = n.clamp(1, 12);
        self
    }
    /// Animation speed multiplier.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Ribbon width multiplier (0.3..3).
    pub fn width(mut self, k: f32) -> Self {
        self.width = k.clamp(0.3, 3.0);
        self
    }
    /// Overall rotation of the sweep in radians.
    pub fn angle(mut self, a: f32) -> Self {
        self.angle = a;
        self
    }
    /// Ribbon colors (cycled).
    pub fn colors(mut self, c: Vec<Color32>) -> Self {
        if !c.is_empty() {
            self.colors = Some(c);
        }
        self
    }
    /// Background fill (default: the theme background).
    pub fn background(mut self, c: Color32) -> Self {
        self.background = Some(c);
        self
    }
    /// Drift a ghost pointer while idle (default on).
    pub fn ghost(mut self, on: bool) -> Self {
        self.ghost = on;
        self
    }

    /// Paint into `rect`; ribbons swirl around `pointer`.
    pub fn paint(&self, ui: &Ui, rect: Rect, pointer: Option<Pos2>) {
        let t = Theme::of(ui);
        let time = glowkit_core::time(ui) * self.speed;
        let painter = ui.painter_at(rect);
        let add = t.dark;
        let bg = self.background.unwrap_or(if add { hex("#05070F") } else { t.background });
        // deep backdrop gradient
        let deep = if add { hex("#0B1230") } else { lerp_rgb(bg, hex("#C7D2FE"), 0.35) };
        paint::shader_grid(&painter, rect, 12, 8, |uv| {
            let d = ((uv.x - 0.6).powi(2) + (uv.y - 0.4).powi(2)).sqrt();
            lerp_rgb(deep, bg, (d * 1.6).min(1.0))
        });
        let cols = self.colors.clone().unwrap_or_else(|| {
            if add {
                vec![hex("#22D3EE"), hex("#8B5CF6"), hex("#60A5FA"), hex("#A5F3FC"), hex("#C084FC")]
            } else {
                vec![hex("#06B6D4"), hex("#7C3AED"), hex("#3B82F6"), hex("#0EA5E9"), hex("#A855F7")]
            }
        });
        let diag = rect.size().length();
        let dir = vec2(self.angle.cos(), self.angle.sin());
        let nrm = dir.rot90();
        let c = rect.center();
        let seg = 60;
        let mut mesh = Mesh::default();
        for i in 0..self.ribbons {
            let fi = i as f32;
            let col = cols[i % cols.len()];
            let off = (fi - (self.ribbons as f32 - 1.0) * 0.5) * 18.0;
            let mut centers = Vec::with_capacity(seg + 1);
            let mut widths = Vec::with_capacity(seg + 1);
            let mut normals = Vec::with_capacity(seg + 1);
            for k in 0..=seg {
                let s = k as f32 / seg as f32 * 2.0 - 1.0;
                let along = s * diag * 0.6;
                let wave = (s * 2.4 + time * 0.35 + fi * 0.9).sin() * 40.0 + (s * 5.1 - time * 0.22 + fi * 2.1).sin() * 18.0;
                let mut p = c + dir * along + nrm * (off + wave);
                if let Some(m) = pointer {
                    let d = m - p;
                    let l2 = d.length_sq();
                    let fall = (-l2 / (diag * diag * 0.02)).exp();
                    // pull toward the pointer and swirl around it
                    p += d * 0.45 * fall + d.rot90() * 0.35 * fall * (time * 0.8 + fi).sin();
                }
                centers.push(p);
                let twist = (s * 3.0 + time * 0.5 + fi * 1.3).cos();
                widths.push((6.0 + 34.0 * twist.abs()) * self.width);
            }
            for k in 0..=seg {
                let a = centers[k.saturating_sub(1)];
                let b = centers[(k + 1).min(seg)];
                normals.push((b - a).normalized().rot90());
            }
            // cross-section: transparent edge, soft, bright core, soft, transparent edge
            let prof = [-1.0f32, -0.45, 0.0, 0.45, 1.0];
            let base = mesh.vertices.len() as u32;
            for k in 0..=seg {
                let s = k as f32 / seg as f32;
                let fade = (s * PI).sin().powf(0.7);
                let thin = 1.0 - (widths[k] / (40.0 * self.width)).min(1.0);
                for (pi, q) in prof.iter().enumerate() {
                    let a = match pi {
                        0 | 4 => 0.0,
                        2 => if add { 0.38 + 0.4 * thin } else { 0.26 + 0.3 * thin },
                        _ => if add { 0.16 } else { 0.12 },
                    };
                    let cc = if pi == 2 { lerp_rgb(col, Color32::WHITE, thin * 0.7) } else { col };
                    mesh.vertices.push(Vertex {
                        pos: centers[k] + normals[k] * *q * widths[k],
                        uv: WHITE_UV,
                        color: lum(cc, a * fade, add),
                    });
                }
            }
            for k in 0..seg as u32 {
                let a = base + k * 5;
                let b = a + 5;
                for j in 0..4 {
                    mesh.add_triangle(a + j, b + j, b + j + 1);
                    mesh.add_triangle(a + j, b + j + 1, a + j + 1);
                }
            }
        }
        painter.add(Shape::mesh(mesh));
        if let Some(m) = pointer {
            if add {
                paint::glow(&painter, m, 60.0, lum(cols[0], 0.18, true));
            }
        }
    }
}

impl Widget for RibbonGlow {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = alloc(ui, self.size, 380.0, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let time = glowkit_core::time(ui);
        let (p, _) = pointer_or_ghost(ui, rect, &response, time, self.ghost);
        let p = p.map(|p| follow_pos(ui, response.id, p, 3.5).0);
        self.paint(ui, rect, p);
        response
    }
}

// ---------------------------------------------------------------------------------------------
// Matrix Junction
// ---------------------------------------------------------------------------------------------

/// Glowing beams converge from every edge on a bright node, which throws jittered lightning arcs
/// toward the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::MatrixJunction::new().beams(26));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct MatrixJunction {
    size: Option<Vec2>,
    beams: usize,
    bolts: usize,
    jag: f32,
    color: Option<Color32>,
    background: Option<Color32>,
    ghost: bool,
}

impl Default for MatrixJunction {
    fn default() -> Self {
        Self::new()
    }
}

impl MatrixJunction {
    /// A junction with default settings.
    pub fn new() -> Self {
        Self { size: None, beams: 26, bolts: 3, jag: 1.0, color: None, background: None, ghost: true }
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Number of converging beams (4..64).
    pub fn beams(mut self, n: usize) -> Self {
        self.beams = n.clamp(4, 64);
        self
    }
    /// Number of lightning bolts toward the pointer (0..8).
    pub fn bolts(mut self, n: usize) -> Self {
        self.bolts = n.min(8);
        self
    }
    /// Bolt jaggedness (0..2).
    pub fn jag(mut self, k: f32) -> Self {
        self.jag = k.clamp(0.0, 2.0);
        self
    }
    /// Beam color.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Background fill (default: the theme background).
    pub fn background(mut self, c: Color32) -> Self {
        self.background = Some(c);
        self
    }
    /// Drift a ghost pointer while idle (default on).
    pub fn ghost(mut self, on: bool) -> Self {
        self.ghost = on;
        self
    }

    /// Paint into `rect`; bolts reach toward `pointer`.
    pub fn paint(&self, ui: &Ui, rect: Rect, pointer: Option<Pos2>, node_shift: Vec2) {
        let t = Theme::of(ui);
        let time = glowkit_core::time(ui);
        let painter = ui.painter_at(rect);
        let add = t.dark;
        let bg = self.background.unwrap_or(t.background);
        painter.rect_filled(rect, 0.0, bg);
        let col = self.color.unwrap_or(if add { hex("#2F7BFF") } else { hex("#1D4ED8") });
        let node = rect.center() + node_shift;
        let diag = rect.size().length();
        let mut glow = Mesh::default();
        let mut cores = Mesh::default();
        for i in 0..self.beams {
            let seed = i as u64 * 977 + 3;
            let a = i as f32 / self.beams as f32 * TAU + (h01(seed) - 0.5) * 0.35 + (time * 0.05).sin() * 0.05;
            let far = node + vec2(a.cos(), a.sin()) * diag;
            let pulse = 0.6 + 0.4 * (time * (1.0 + h01(seed + 1) * 2.0) + h01(seed + 2) * TAU).sin();
            let n = 18;
            let pts: Vec<Pos2> = (0..=n).map(|k| far.lerp(node, k as f32 / n as f32)).collect();
            let fade = |k: usize| (k as f32 / n as f32).powf(1.5);
            let wmul = 0.6 + h01(seed + 3) * 0.8;
            strip(&mut glow, &pts, |k| (3.0 + 10.0 * fade(k)) * wmul, |k| lum(col, 0.12 * pulse * fade(k), add), |_| Color32::TRANSPARENT);
            let cc = lerp_rgb(col, Color32::WHITE, 0.45);
            strip(&mut cores, &pts, |k| 0.8 + 1.4 * fade(k) * wmul, |k| lum(cc, (0.25 + 0.6 * fade(k)) * pulse, add), |_| Color32::TRANSPARENT);
        }
        painter.add(Shape::mesh(glow));
        painter.add(Shape::mesh(cores));
        // bolts
        if let Some(m) = pointer {
            let tick = (time * 14.0) as u64;
            let mut bolt = Mesh::default();
            let d = m - node;
            let len = d.length();
            if len > 20.0 {
                let nrm = d.rot90() / len;
                for b in 0..self.bolts {
                    let n = 22;
                    let mut pts = Vec::with_capacity(n + 1);
                    for k in 0..=n {
                        let s = k as f32 / n as f32;
                        let env = (s * PI).sin();
                        let j = (h01(tick * 131 + b as u64 * 977 + k as u64 * 7) - 0.5) * 2.0;
                        let j2 = (h01(tick * 17 + b as u64 * 31 + (k / 4) as u64) - 0.5) * 2.0;
                        let off = (j * 10.0 + j2 * 26.0) * env * self.jag * (len / 300.0).clamp(0.4, 1.4);
                        pts.push(node + d * s + nrm * off);
                    }
                    let hot = lerp_rgb(col, Color32::WHITE, 0.7);
                    strip(&mut bolt, &pts, |_| 9.0, |_| lum(col, 0.16, add), |_| Color32::TRANSPARENT);
                    strip(&mut bolt, &pts, |_| 3.5, |_| lum(col, 0.35, add), |_| Color32::TRANSPARENT);
                    strip(&mut bolt, &pts, |k| 2.0 - k as f32 / n as f32 * 0.9, |_| lum(hot, 1.0, add), |_| Color32::TRANSPARENT);
                }
            }
            painter.add(Shape::mesh(bolt));
            if add {
                paint::glow(&painter, m, 24.0, lum(col, 0.4, true));
            } else {
                paint::glow(&painter, m, 20.0, with_alpha(col, 0.25));
            }
        }
        // node
        if add {
            paint::glow(&painter, node, 90.0, lum(col, 0.55, true));
            paint::glow(&painter, node, 26.0, lum(Color32::WHITE, 0.9, true));
        } else {
            paint::glow(&painter, node, 90.0, with_alpha(col, 0.35));
            paint::glow(&painter, node, 16.0, with_alpha(col, 0.9));
            painter.circle_filled(node, 3.5, Color32::WHITE);
        }
    }
}

impl Widget for MatrixJunction {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = alloc(ui, self.size, 380.0, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let time = glowkit_core::time(ui);
        let p = match response.hover_pos() {
            Some(p) => Some(p),
            None if self.ghost => {
                let a = time * 0.45;
                Some(rect.center() + vec2(a.cos() * rect.width() * 0.3, (a * 1.3).sin() * rect.height() * 0.32))
            }
            None => None,
        };
        let shift = p.map(|p| (p - rect.center()) * 0.06).unwrap_or_default();
        let shift = glowkit_core::anim::spring2(ui, response.id.with("node"), shift, 50.0, 12.0);
        self.paint(ui, rect, p, shift);
        response
    }
}

// ---------------------------------------------------------------------------------------------
// Fluid Trail
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct Fluid {
    w: usize,
    h: usize,
    u: Vec<f32>,
    v: Vec<f32>,
    d: Vec<[f32; 3]>,
    tmp: Vec<f32>,
    p: Vec<f32>,
    div: Vec<f32>,
    last: Option<Pos2>,
}

impl Fluid {
    fn new(w: usize, h: usize) -> Self {
        let n = w * h;
        Self {
            w,
            h,
            u: vec![0.0; n],
            v: vec![0.0; n],
            d: vec![[0.0; 3]; n],
            tmp: vec![0.0; n],
            p: vec![0.0; n],
            div: vec![0.0; n],
            last: None,
        }
    }
    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.w + x
    }
    fn sample(f: &[f32], w: usize, h: usize, x: f32, y: f32) -> f32 {
        let x = x.clamp(0.0, (w - 1) as f32 - 0.001);
        let y = y.clamp(0.0, (h - 1) as f32 - 0.001);
        let (x0, y0) = (x as usize, y as usize);
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let i = y0 * w + x0;
        let a = f[i] + (f[i + 1] - f[i]) * fx;
        let b = f[i + w] + (f[i + w + 1] - f[i + w]) * fx;
        a + (b - a) * fy
    }
    fn splat(&mut self, gx: f32, gy: f32, fx: f32, fy: f32, color: [f32; 3], radius: f32) {
        let r = radius.max(1.0);
        let (x0, x1) = ((gx - r * 2.0).max(1.0) as usize, (gx + r * 2.0).min((self.w - 2) as f32) as usize);
        let (y0, y1) = ((gy - r * 2.0).max(1.0) as usize, (gy + r * 2.0).min((self.h - 2) as f32) as usize);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d2 = (x as f32 - gx).powi(2) + (y as f32 - gy).powi(2);
                let k = (-d2 / (r * r)).exp();
                let i = self.idx(x, y);
                self.u[i] += fx * k;
                self.v[i] += fy * k;
                for c in 0..3 {
                    self.d[i][c] = (self.d[i][c] + color[c] * k).min(1.0);
                }
            }
        }
    }
    fn step(&mut self, dt: f32, dissipation: f32) {
        let (w, h) = (self.w, self.h);
        // advect velocity
        let u0 = self.u.clone();
        let v0 = self.v.clone();
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = y * w + x;
                let px = x as f32 - dt * u0[i];
                let py = y as f32 - dt * v0[i];
                self.u[i] = Self::sample(&u0, w, h, px, py) * 0.995;
                self.v[i] = Self::sample(&v0, w, h, px, py) * 0.995;
            }
        }
        // divergence + pressure (Jacobi)
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = y * w + x;
                self.div[i] = 0.5 * (self.u[i + 1] - self.u[i - 1] + self.v[i + w] - self.v[i - w]);
                self.p[i] *= 0.8;
            }
        }
        for _ in 0..16 {
            for y in 1..h - 1 {
                for x in 1..w - 1 {
                    let i = y * w + x;
                    self.tmp[i] = (self.p[i - 1] + self.p[i + 1] + self.p[i - w] + self.p[i + w] - self.div[i]) * 0.25;
                }
            }
            std::mem::swap(&mut self.p, &mut self.tmp);
        }
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = y * w + x;
                self.u[i] -= 0.5 * (self.p[i + 1] - self.p[i - 1]);
                self.v[i] -= 0.5 * (self.p[i + w] - self.p[i - w]);
            }
        }
        // advect dye
        let d0 = self.d.clone();
        let keep = (-dt * dissipation).exp();
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = y * w + x;
                let px = (x as f32 - dt * self.u[i]).clamp(0.0, (w - 1) as f32 - 0.001);
                let py = (y as f32 - dt * self.v[i]).clamp(0.0, (h - 1) as f32 - 0.001);
                let (x0, y0) = (px as usize, py as usize);
                let (fx, fy) = (px - x0 as f32, py - y0 as f32);
                let j = y0 * w + x0;
                let mut out = [0.0; 3];
                for (c, o) in out.iter_mut().enumerate() {
                    let a = d0[j][c] + (d0[j + 1][c] - d0[j][c]) * fx;
                    let b = d0[j + w][c] + (d0[j + w + 1][c] - d0[j + w][c]) * fx;
                    *o = (a + (b - a) * fy) * keep;
                }
                self.d[i] = out;
            }
        }
    }
}

/// A glowing fluid trail: the pointer stirs a small real-time fluid simulation (advection plus a
/// pressure solve), leaving swirling, color-cycling dye that curls and fades behind it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::FluidTrail::new().fade(0.8));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct FluidTrail {
    size: Option<Vec2>,
    resolution: usize,
    fade: f32,
    radius: f32,
    colors: Option<Vec<Color32>>,
    background: Option<Color32>,
    ghost: bool,
}

impl Default for FluidTrail {
    fn default() -> Self {
        Self::new()
    }
}

impl FluidTrail {
    /// A trail with default settings.
    pub fn new() -> Self {
        Self { size: None, resolution: 84, fade: 0.8, radius: 1.0, colors: None, background: None, ghost: true }
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Simulation columns (32..120); rows follow the aspect ratio.
    pub fn resolution(mut self, n: usize) -> Self {
        self.resolution = n.clamp(32, 120);
        self
    }
    /// How fast the dye fades (0.1..3, higher = shorter trail).
    pub fn fade(mut self, k: f32) -> Self {
        self.fade = k.clamp(0.1, 3.0);
        self
    }
    /// Splat radius multiplier (0.4..3).
    pub fn radius(mut self, k: f32) -> Self {
        self.radius = k.clamp(0.4, 3.0);
        self
    }
    /// Dye colors cycled over time.
    pub fn colors(mut self, c: Vec<Color32>) -> Self {
        if !c.is_empty() {
            self.colors = Some(c);
        }
        self
    }
    /// Background fill (default: the theme background).
    pub fn background(mut self, c: Color32) -> Self {
        self.background = Some(c);
        self
    }
    /// Stir with a ghost pointer while idle (default on).
    pub fn ghost(mut self, on: bool) -> Self {
        self.ghost = on;
        self
    }
}

impl Widget for FluidTrail {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = alloc(ui, self.size, 380.0, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let t = Theme::of(ui);
        let add = t.dark;
        let time = glowkit_core::time(ui);
        let dt = glowkit_core::dt(ui).max(1.0 / 120.0);
        let (p, _) = pointer_or_ghost(ui, rect, &response, time * 1.6, self.ghost);
        let gw = self.resolution;
        let gh = ((gw as f32 * rect.height() / rect.width().max(1.0)).round() as usize).clamp(16, 90);
        let cols = self.colors.clone().unwrap_or_else(|| {
            if add {
                vec![hex("#22D3EE"), hex("#8B5CF6"), hex("#F472B6"), hex("#F59E0B")]
            } else {
                vec![hex("#0891B2"), hex("#7C3AED"), hex("#DB2777"), hex("#EA580C")]
            }
        });
        let c = glowkit_core::color::gradient_cyclic(&cols, time * 0.12);
        let dye = [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0];
        let scale = vec2((gw - 1) as f32 / rect.width(), (gh - 1) as f32 / rect.height());
        let fade = self.fade;
        let rad = self.radius * gw as f32 / 36.0;
        let grid: Vec<[f32; 3]> = glowkit_core::with_state(ui, response.id.with("fluid"), || Fluid::new(gw, gh), |f| {
            if f.w != gw || f.h != gh {
                *f = Fluid::new(gw, gh);
            }
            if let Some(p) = p {
                let g = vec2((p.x - rect.min.x) * scale.x, (p.y - rect.min.y) * scale.y);
                if let Some(last) = f.last {
                    let lg = vec2((last.x - rect.min.x) * scale.x, (last.y - rect.min.y) * scale.y);
                    let vel = (g - lg) / dt;
                    let steps = ((g - lg).length() / 1.0).ceil().clamp(1.0, 12.0) as usize;
                    let sp = vel.length();
                    if sp > 0.5 {
                        for k in 0..steps {
                            let q = lg + (g - lg) * ((k + 1) as f32 / steps as f32);
                            let amt = (sp / 60.0).min(1.0) / steps as f32 * 1.1;
                            f.splat(q.x, q.y, vel.x * 0.12 / steps as f32, vel.y * 0.12 / steps as f32, [dye[0] * amt, dye[1] * amt, dye[2] * amt], rad);
                        }
                    }
                }
                f.last = Some(p);
            } else {
                f.last = None;
            }
            f.step(dt * 30.0, fade / 30.0);
            f.d.clone()
        });
        let painter = ui.painter_at(rect);
        let bg = self.background.unwrap_or(t.background);
        painter.rect_filled(rect, 0.0, bg);
        let mut m = Mesh::default();
        m.reserve_vertices(gw * gh);
        for y in 0..gh {
            for x in 0..gw {
                let d = grid[y * gw + x];
                let pos = pos2(rect.min.x + x as f32 / scale.x, rect.min.y + y as f32 / scale.y);
                let col = if add {
                    let edge = (x.min(gw - 1 - x).min(y).min(gh - 1 - y) as f32 / 3.0).min(1.0);
                    let k = |v: f32| ((1.0 - (-v * 1.5).exp()) * 255.0 * edge) as u8;
                    Color32::from_rgba_premultiplied(k(d[0]), k(d[1]), k(d[2]), 0)
                } else {
                    let dens = (d[0] + d[1] + d[2]).max(1e-4);
                    let cc = Color32::from_rgb(
                        (d[0] / dens * 3.0 * 160.0).min(255.0) as u8,
                        (d[1] / dens * 3.0 * 160.0).min(255.0) as u8,
                        (d[2] / dens * 3.0 * 160.0).min(255.0) as u8,
                    );
                    let edge = (x.min(gw - 1 - x).min(y).min(gh - 1 - y) as f32 / 3.0).min(1.0);
                    with_alpha(cc, (1.0 - (-dens * 1.1).exp()).min(0.92) * edge)
                };
                m.vertices.push(Vertex { pos, uv: WHITE_UV, color: col });
            }
        }
        for y in 0..(gh - 1) as u32 {
            for x in 0..(gw - 1) as u32 {
                let i = y * gw as u32 + x;
                m.add_triangle(i, i + 1, i + gw as u32 + 1);
                m.add_triangle(i, i + gw as u32 + 1, i + gw as u32);
            }
        }
        painter.add(Shape::mesh(m));
        if response.hover_pos().is_none() && self.ghost {
            if let Some(p) = p {
                let mut d = Dots::new(1);
                d.add(p, 3.0, with_alpha(t.foreground, 0.5));
                d.paint(&painter);
            }
        }
        let _ = ramp;
        response
    }
}
