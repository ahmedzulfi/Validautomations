//! Painted animated scenery: a cumulus sky, a voxel sunset meadow and glittering night hills.

use super::common::{h01, lerp_rgb, lum, ramp, Dots};
use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{pos2, vec2, Color32, Pos2, Rect, Response, Sense, Shape, Ui, Vec2, Widget};
use glowkit_core::color::{hex, with_alpha};
use glowkit_core::noise::{fbm3, noise3};
use glowkit_core::{paint, Theme};
use std::f32::consts::TAU;

fn alloc(ui: &mut Ui, size: Option<Vec2>, h: f32, sense: Sense) -> (Rect, Response) {
    let s = glowkit_core::desired_size(ui, size, h);
    ui.allocate_exact_size(s, sense)
}

fn wind_from(ui: &Ui, response: &Response, rect: Rect) -> f32 {
    let target = match response.hover_pos() {
        Some(p) => (p.x - rect.center().x) / (rect.width() * 0.5),
        None => 0.0,
    };
    glowkit_core::anim::follow(ui, response.id.with("wind"), target, 1.5)
}

/// A soft puff: opaque center fading to transparent.
fn puff(m: &mut Mesh, c: Pos2, r: Vec2, col: Color32, edge: Color32) {
    let i = m.vertices.len() as u32;
    m.vertices.push(Vertex { pos: c, uv: WHITE_UV, color: col });
    let n = 20u32;
    for k in 0..n {
        let a = k as f32 / n as f32 * TAU;
        m.vertices.push(Vertex { pos: c + vec2(a.cos() * r.x * 0.55, a.sin() * r.y * 0.55), uv: WHITE_UV, color: col });
    }
    for k in 0..n {
        let a = k as f32 / n as f32 * TAU;
        m.vertices.push(Vertex { pos: c + vec2(a.cos() * r.x, a.sin() * r.y), uv: WHITE_UV, color: edge });
    }
    for k in 0..n {
        let kn = (k + 1) % n;
        m.add_triangle(i, i + 1 + k, i + 1 + kn);
        m.add_triangle(i + 1 + k, i + 1 + n + k, i + 1 + n + kn);
        m.add_triangle(i + 1 + k, i + 1 + n + kn, i + 1 + kn);
    }
}

// ---------------------------------------------------------------------------------------------
// Cloud Sky
// ---------------------------------------------------------------------------------------------

/// An animated sky: discrete cumulus clouds drift on two parallax layers under a thin cirrus veil,
/// with a sun glow low in one corner. Moving the pointer left or right turns the wind.
/// Defaults to a clear day on light themes and a peach dusk on dark ones.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::CloudSky::new().clouds(9));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CloudSky {
    size: Option<Vec2>,
    clouds: usize,
    speed: f32,
    dusk: Option<bool>,
    cirrus: f32,
}

impl Default for CloudSky {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudSky {
    /// A sky with default settings.
    pub fn new() -> Self {
        Self { size: None, clouds: 7, speed: 1.0, dusk: None, cirrus: 0.6 }
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Clouds per layer (1..20).
    pub fn clouds(mut self, n: usize) -> Self {
        self.clouds = n.clamp(1, 20);
        self
    }
    /// Drift speed multiplier.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Force the dusk palette on or off (default: follows the theme).
    pub fn dusk(mut self, on: bool) -> Self {
        self.dusk = Some(on);
        self
    }
    /// Cirrus veil strength (0..1).
    pub fn cirrus(mut self, k: f32) -> Self {
        self.cirrus = k.clamp(0.0, 1.0);
        self
    }

    /// Paint into `rect`; `wind` in -1..1 shifts the drift.
    pub fn paint(&self, ui: &Ui, rect: Rect, wind: f32) {
        let t = Theme::of(ui);
        let time = glowkit_core::time(ui);
        let painter = ui.painter_at(rect);
        let dusk = self.dusk.unwrap_or(t.dark);
        let (top, mid, low, sun_c) = if dusk {
            (hex("#1E1B4B"), hex("#6D4C8F"), hex("#F6A58B"), hex("#FFD8A8"))
        } else {
            (hex("#1E6FD9"), hex("#4A9BF0"), hex("#BFE3FF"), hex("#FFFBEA"))
        };
        let sun = pos2(rect.max.x - rect.width() * 0.12, rect.max.y - rect.height() * 0.16);
        let cirrus = self.cirrus;
        paint::shader_grid(&painter, rect, 36, 20, |uv| {
            let mut c = ramp(&[top, mid, low], uv.y.powf(0.9));
            let veil = (fbm3(uv.x * 3.0 - time * 0.01, uv.y * 14.0, 1.0, 3) * 0.5 + 0.5).powf(2.5) * cirrus * (1.0 - uv.y) * 1.2;
            c = lerp_rgb(c, Color32::WHITE, veil.min(0.5));
            c
        });
        paint::glow(&painter, sun, rect.height() * 0.9, with_alpha(sun_c, 0.45));
        paint::glow(&painter, sun, rect.height() * 0.18, with_alpha(Color32::WHITE, 0.8));
        let (shade, body, light) = if dusk {
            (hex("#7B5C93"), hex("#F2C9C2"), hex("#FFF1E0"))
        } else {
            (hex("#A9BCD6"), hex("#F4F8FF"), Color32::WHITE)
        };
        for layer in 0..2 {
            let far = layer == 0;
            let scale = if far { 0.6 } else { 1.0 };
            let speed = (if far { 6.0 } else { 14.0 }) * self.speed * (1.0 + wind * 1.5);
            let span = rect.width() + 400.0 * scale;
            let mut m = Mesh::default();
            for i in 0..self.clouds {
                let seed = (layer * 100 + i) as u64 * 7919 + 3;
                let base_x = h01(seed) * span + time * speed;
                let x = rect.min.x - 200.0 * scale + base_x.rem_euclid(span);
                let y = rect.min.y + rect.height() * (if far { 0.2 + 0.45 * h01(seed + 1) } else { 0.12 + 0.62 * h01(seed + 1) });
                let w = (90.0 + 120.0 * h01(seed + 2)) * scale;
                let puffs = 7 + (h01(seed + 3) * 6.0) as usize;
                let alpha = if far { 0.75 } else { 1.0 };
                // shadow underside, body, lit tops
                for pass in 0..3 {
                    for k in 0..puffs {
                        let ps = seed * 31 + k as u64;
                        let fx = (k as f32 / (puffs - 1) as f32 - 0.5) * 2.0;
                        let hump = 1.0 - fx * fx;
                        let morph = noise3(ps as f32 * 0.1, time * 0.05, 0.0) * 0.12;
                        let r = w * (0.22 + 0.2 * hump + 0.08 * h01(ps)) * (1.0 + morph);
                        let c = pos2(x + fx * w * 0.6, y - hump * w * 0.12 + (h01(ps + 1) - 0.5) * w * 0.1);
                        let (col, off, rr) = match pass {
                            0 => (shade, vec2(0.0, r * 0.32), r * 1.1),
                            1 => (body, vec2(0.0, 0.0), r),
                            _ => (light, vec2(-r * 0.15, -r * 0.2), r * 0.6),
                        };
                        let a = alpha * match pass { 0 => 0.45, 1 => 0.95, _ => 0.7 };
                        puff(&mut m, c + off, vec2(rr * 1.15, rr * 0.9), with_alpha(col, a), with_alpha(col, 0.0));
                    }
                }
                // flat base
                puff(&mut m, pos2(x, y + w * 0.08), vec2(w * 0.8, w * 0.14), with_alpha(shade, 0.22 * alpha), with_alpha(shade, 0.0));
            }
            painter.add(Shape::mesh(m));
        }
    }
}

impl Widget for CloudSky {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = alloc(ui, self.size, 380.0, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let wind = wind_from(ui, &response, rect);
        self.paint(ui, rect, wind);
        response
    }
}

// ---------------------------------------------------------------------------------------------
// Voxel Meadow
// ---------------------------------------------------------------------------------------------

/// A ground-level voxel meadow at sunset: a pink sky with a square sun, drifting block clouds, a
/// blocky treeline and a field of cube grass and pixel flowers that sway in the breeze, with the
/// nearest blocks softened like a shallow depth of field.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::VoxelMeadow::new().flowers(0.4));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct VoxelMeadow {
    size: Option<Vec2>,
    flowers: f32,
    breeze: f32,
    blocks: usize,
}

impl Default for VoxelMeadow {
    fn default() -> Self {
        Self::new()
    }
}

impl VoxelMeadow {
    /// A meadow with default settings.
    pub fn new() -> Self {
        Self { size: None, flowers: 0.3, breeze: 1.0, blocks: 2200 }
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Share of blocks that are flowers (0..1).
    pub fn flowers(mut self, k: f32) -> Self {
        self.flowers = k.clamp(0.0, 1.0);
        self
    }
    /// Breeze strength (0..2).
    pub fn breeze(mut self, k: f32) -> Self {
        self.breeze = k.clamp(0.0, 2.0);
        self
    }
    /// Number of grass / flower blocks (200..3000).
    pub fn blocks(mut self, n: usize) -> Self {
        self.blocks = n.clamp(200, 3000);
        self
    }
}

fn cube(m: &mut Mesh, c: Pos2, s: f32, top_h: f32, front: Color32, top: Color32, side: Color32) {
    // front face square, a squashed top face, a thin right side
    let h = s * 0.5;
    let quad = |m: &mut Mesh, p: [Pos2; 4], col: Color32| {
        let i = m.vertices.len() as u32;
        for q in p {
            m.vertices.push(Vertex { pos: q, uv: WHITE_UV, color: col });
        }
        m.add_triangle(i, i + 1, i + 2);
        m.add_triangle(i, i + 2, i + 3);
    };
    let d = vec2(s * 0.18, -top_h);
    quad(m, [c + vec2(-h, -h), c + vec2(h, -h), c + vec2(h, -h) + d, c + vec2(-h, -h) + d], top);
    quad(m, [c + vec2(h, -h), c + vec2(h, h), c + vec2(h, h) + d, c + vec2(h, -h) + d], side);
    quad(m, [c + vec2(-h, -h), c + vec2(h, -h), c + vec2(h, h), c + vec2(-h, h)], front);
}

impl Widget for VoxelMeadow {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = alloc(ui, self.size, 380.0, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let time = glowkit_core::time(ui);
        let wind = wind_from(ui, &response, rect);
        let painter = ui.painter_at(rect);
        let horizon = rect.min.y + rect.height() * 0.46;
        // sky
        let sky = [hex("#E7A9C9"), hex("#F4B7B1"), hex("#FAD2A6"), hex("#FBE3B8")];
        paint::shader_grid(&painter, Rect::from_min_max(rect.min, pos2(rect.max.x, horizon + 4.0)), 1, 10, |uv| ramp(&sky, uv.y));
        // square sun with glow
        let sun = pos2(rect.min.x + rect.width() * 0.1, horizon - rect.height() * 0.14);
        paint::glow(&painter, sun, rect.height() * 0.55, with_alpha(hex("#FFF3D6"), 0.75));
        painter.rect_filled(Rect::from_center_size(sun, Vec2::splat(rect.height() * 0.09)), 0.0, hex("#FFFBF0"));
        // block clouds
        let mut m = Mesh::default();
        for i in 0..7 {
            let seed = i as u64 * 131 + 7;
            let span = rect.width() + 200.0;
            let x = rect.min.x - 100.0 + (h01(seed) * span + time * (6.0 + 6.0 * h01(seed + 1)) * (1.0 + wind)).rem_euclid(span);
            let y = rect.min.y + rect.height() * (0.08 + 0.25 * h01(seed + 2));
            let s = 16.0 + 10.0 * h01(seed + 3);
            let w = 3 + (h01(seed + 4) * 4.0) as i32;
            for bx in 0..w {
                for by in 0..2 {
                    if by == 0 && (bx == 0 || bx == w - 1) && h01(seed + bx as u64) > 0.4 {
                        continue;
                    }
                    let c = pos2(x + bx as f32 * s, y + by as f32 * s * 0.8);
                    cube(&mut m, c, s, s * 0.35, hex("#F8E0E4"), hex("#FFF6F2"), hex("#E9C3CC"));
                }
            }
        }
        painter.add(Shape::mesh(m));
        // treeline / distant blocks
        let mut far = Mesh::default();
        let bw = 18.0;
        let cols = (rect.width() / bw) as i32 + 2;
        for i in 0..cols {
            let n = (noise3(i as f32 * 0.21, 3.0, 0.0) * 1.6 + 0.5).clamp(0.0, 1.0);
            let hgt = ((n * 5.0) as i32).max(1);
            for k in 0..hgt {
                let c = pos2(rect.min.x + i as f32 * bw, horizon - k as f32 * bw * 0.6);
                let col = lerp_rgb(hex("#C9A06E"), hex("#E3B98B"), k as f32 / 4.0);
                cube(&mut far, c, bw, 3.0, col, lerp_rgb(col, Color32::WHITE, 0.25), lerp_rgb(col, Color32::BLACK, 0.1));
            }
        }
        painter.add(Shape::mesh(far));
        // ground
        paint::shader_grid(&painter, Rect::from_min_max(pos2(rect.min.x, horizon), rect.max), 1, 8, |uv| ramp(&[hex("#B8B45A"), hex("#7FA03A"), hex("#4E7A2A")], uv.y));
        // meadow blocks back to front, placed in a perspective ground plane
        let flowers = [hex("#E5484D"), hex("#F5C542"), hex("#FFFFFF"), hex("#4E7BEA"), hex("#F28AB2"), hex("#C25BD6")];
        let focal = rect.height() * 0.95;
        let cam_h = 0.42;
        let (zmin, zmax) = (0.6, 16.0);
        let aspect = rect.width() / rect.height();
        let mut items: Vec<(f32, u64)> = (0..self.blocks as u64)
            .map(|i| {
                let u = h01(i * 3 + 1);
                (zmin + (zmax - zmin) * u.powf(1.6), i)
            })
            .collect();
        items.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut mesh = Mesh::default();
        let mut blur = Mesh::default();
        for (z, i) in items {
            let xw = (h01(i * 3 + 2) - 0.5) * 2.2 * z * aspect * 0.55;
            let x = rect.center().x + xw / z * focal;
            let y = horizon + cam_h / z * focal;
            let s = (0.11 / z * focal).max(1.6) * (0.85 + 0.3 * h01(i * 3 + 3));
            let is_flower = h01(i * 7 + 5) < self.flowers;
            let sway = (time * 1.6 + xw * 2.0).sin() * (0.4 + wind) * self.breeze;
            let haze = ((z - 4.0) / 12.0).clamp(0.0, 0.55);
            let hz = |c: Color32| lerp_rgb(c, hex("#E9C99A"), haze);
            let target = if z < 1.25 { &mut blur } else { &mut mesh };
            if is_flower {
                let col = flowers[(i as usize * 7) % flowers.len()];
                let stem = 1 + (h01(i * 5) * 3.0) as i32;
                for k in 0..stem {
                    let c = pos2(x + sway * k as f32 * s * 0.1, y - k as f32 * s * 0.42);
                    cube(target, c, s * 0.3, s * 0.08, hz(hex("#4E8A2E")), hz(hex("#6FB043")), hz(hex("#3B6E22")));
                }
                let head = pos2(x + sway * stem as f32 * s * 0.1, y - stem as f32 * s * 0.42 - s * 0.15);
                cube(target, head, s * 0.7, s * 0.2, hz(col), hz(lerp_rgb(col, Color32::WHITE, 0.35)), hz(lerp_rgb(col, Color32::BLACK, 0.25)));
                if s > 9.0 {
                    cube(target, head + vec2(0.0, -s * 0.06), s * 0.24, s * 0.07, hex("#F2B33D"), hex("#FFD772"), hex("#C98C22"));
                }
            } else {
                let tall = 1 + (h01(i * 11) * 3.2) as i32;
                let g = hz(lerp_rgb(hex("#4F8A2A"), hex("#B5C84A"), h01(i * 13)));
                for k in 0..tall {
                    let c = pos2(x + sway * k as f32 * s * 0.14, y - k as f32 * s * 0.55);
                    cube(target, c, s * 0.48, s * 0.13, g, lerp_rgb(g, hex("#F7E7A0"), 0.35), lerp_rgb(g, Color32::BLACK, 0.2));
                }
            }
        }
        // warm haze near the horizon
        painter.add(Shape::mesh(mesh));
        paint::shader_grid(&painter, Rect::from_min_max(pos2(rect.min.x, horizon - 10.0), pos2(rect.max.x, horizon + rect.height() * 0.12)), 1, 4, |uv| {
            with_alpha(hex("#FBD9B0"), 0.45 * (1.0 - uv.y))
        });
        // foreground "out of focus": translucent and slightly doubled
        let mut b2 = blur.clone();
        for v in &mut blur.vertices {
            v.color = v.color.gamma_multiply(0.55);
        }
        for v in &mut b2.vertices {
            v.pos += vec2(2.5, 1.5);
            v.color = v.color.gamma_multiply(0.45);
        }
        painter.add(Shape::mesh(blur));
        painter.add(Shape::mesh(b2));
        paint::vignette(&painter, rect, hex("#6B4A3A"), 0.25);
        response
    }
}

// ---------------------------------------------------------------------------------------------
// Glimmer Vale
// ---------------------------------------------------------------------------------------------

/// A night meadow: rolling hills built from thousands of peach-to-violet glitter points under a
/// periwinkle sky, with a bloom of sparkles along each crest and soft out-of-focus daisies
/// drifting across the foreground.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::GlimmerVale::new().glitter(2400));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct GlimmerVale {
    size: Option<Vec2>,
    glitter: usize,
    daisies: usize,
    speed: f32,
}

impl Default for GlimmerVale {
    fn default() -> Self {
        Self::new()
    }
}

impl GlimmerVale {
    /// A vale with default settings.
    pub fn new() -> Self {
        Self { size: None, glitter: 3600, daisies: 22, speed: 1.0 }
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Number of glitter points on the hills (300..5000).
    pub fn glitter(mut self, n: usize) -> Self {
        self.glitter = n.clamp(300, 5000);
        self
    }
    /// Number of foreground daisies (0..60).
    pub fn daisies(mut self, n: usize) -> Self {
        self.daisies = n.min(60);
        self
    }
    /// Animation speed multiplier.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
}

impl Widget for GlimmerVale {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = alloc(ui, self.size, 380.0, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let time = glowkit_core::time(ui) * self.speed;
        let wind = wind_from(ui, &response, rect);
        let painter = ui.painter_at(rect);
        let sky = [hex("#6E74C8"), hex("#8E95DC"), hex("#B7B4E6"), hex("#3B2E58")];
        paint::shader_grid(&painter, rect, 1, 12, |uv| ramp(&sky, uv.y));
        // hills: crest(x) per layer, far to near
        let layers = [(0.46, 0.15, 0.8, 0.0), (0.6, 0.12, 1.25, 2.0), (0.76, 0.08, 1.7, 5.0)];
        let crest = |l: usize, x: f32| -> f32 {
            let (base, amp, freq, ph) = layers[l];
            let u = x / rect.width();
            rect.min.y + rect.height() * (base - amp * ((u * freq * TAU * 0.5 + ph + time * 0.02).sin() * 0.7 + (u * freq * 3.1 + ph * 1.7).sin() * 0.3))
        };
        let warm = hex("#FFC9A8");
        let violet = hex("#9A7BE0");
        let deep = hex("#1C1430");
        for l in 0..layers.len() {
            // dark body
            let cols = 48;
            let mut body = Mesh::default();
            for k in 0..=cols {
                let x = rect.min.x + k as f32 / cols as f32 * rect.width();
                let y = crest(l, x);
                let crest_c = lerp_rgb(lerp_rgb(violet, warm, 0.45), deep, 0.25 + l as f32 * 0.12);
                let mid_c = lerp_rgb(hex("#4A3878"), deep, 0.3 + l as f32 * 0.1);
                body.vertices.push(Vertex { pos: pos2(x, y), uv: WHITE_UV, color: crest_c });
                body.vertices.push(Vertex { pos: pos2(x, y + rect.height() * 0.08), uv: WHITE_UV, color: mid_c });
                body.vertices.push(Vertex { pos: pos2(x, rect.max.y), uv: WHITE_UV, color: deep });
            }
            for k in 0..cols as u32 {
                let i = k * 3;
                for r in 0..2 {
                    body.add_triangle(i + r, i + r + 1, i + r + 4);
                    body.add_triangle(i + r, i + r + 4, i + r + 3);
                }
            }
            painter.add(Shape::mesh(body));
            // glitter: dense along the crest, thinning downward
            let n = self.glitter / layers.len();
            let mut dots = Dots::new(n);
            for i in 0..n {
                let seed = (l * 100_000 + i) as u64 * 2_654_435_761 % 1_000_003;
                let x = rect.min.x + h01(seed) * rect.width();
                let depth = h01(seed + 1).powf(2.0) * rect.height() * 0.4;
                let y = crest(l, x) + depth + 1.0;
                if y > rect.max.y {
                    continue;
                }
                let near_crest = (1.0 - depth / 60.0).max(0.0);
                let tw = 0.5 + 0.5 * (time * (2.0 + 4.0 * h01(seed + 2)) + h01(seed + 3) * TAU).sin();
                let c = lerp_rgb(violet, warm, (h01(seed + 4) * 0.6 + near_crest * 0.6).min(1.0));
                let a = ((0.35 + 0.75 * tw) * (0.5 + 0.7 * near_crest) * (0.8 + 0.2 * l as f32 / 2.0)).min(1.0);
                let size = (0.7 + 1.4 * h01(seed + 5).powi(2)) * (0.8 + 0.3 * l as f32);
                dots.add(pos2(x, y), size, lum(c, a, true));
            }
            dots.paint(&painter);
            // crest bloom
            let mut bloom = Mesh::default();
            let pts: Vec<Pos2> = (0..=cols).map(|k| {
                let x = rect.min.x + k as f32 / cols as f32 * rect.width();
                pos2(x, crest(l, x))
            }).collect();
            super::common::strip(&mut bloom, &pts, |_| 14.0, |_| lum(warm, 0.10, true), |_| Color32::TRANSPARENT);
            super::common::strip(&mut bloom, &pts, |_| 4.0, |_| lum(warm, 0.16, true), |_| Color32::TRANSPARENT);
            painter.add(Shape::mesh(bloom));
        }
        // stars
        let mut stars = Dots::new(80);
        for i in 0..80u64 {
            let p = pos2(rect.min.x + h01(i * 5 + 1) * rect.width(), rect.min.y + h01(i * 5 + 2) * rect.height() * 0.4);
            let tw = 0.5 + 0.5 * (time * 2.0 + i as f32).sin();
            stars.add(p, 0.7, lum(Color32::WHITE, 0.2 + 0.5 * tw, true));
        }
        stars.paint(&painter);
        // out-of-focus daisies drifting in the foreground
        let mut m = Mesh::default();
        for i in 0..self.daisies {
            let seed = i as u64 * 977 + 13;
            let d = h01(seed);
            let span = rect.width() + 120.0;
            let x = rect.min.x - 60.0 + (h01(seed + 1) * span + time * (4.0 + 10.0 * d) * (1.0 + wind * 2.0)).rem_euclid(span);
            let y = rect.min.y + rect.height() * (0.62 + 0.36 * h01(seed + 2)) + (time * 0.8 + i as f32).sin() * 3.0;
            let r = 7.0 + 22.0 * d * d;
            let a = 0.45 + 0.35 * d;
            puff(&mut m, pos2(x, y), vec2(r * 1.25, r * 1.05), with_alpha(hex("#F5F0FF"), a * 0.9), with_alpha(hex("#F5F0FF"), 0.0));
            puff(&mut m, pos2(x, y), vec2(r * 0.42, r * 0.36), with_alpha(hex("#FF7A5C"), a), with_alpha(hex("#FF7A5C"), 0.0));
        }
        painter.add(Shape::mesh(m));
        let _ = fbm3;
        response
    }
}
