use super::util::{mesh_dot, mesh_rect};
use egui::epaint::Mesh;
use egui::{pos2, vec2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Ui, Vec2, Widget};
use glowkit_core::{
    anim::smoothstep,
    color::{gradient, gradient_cyclic, with_alpha},
    hex, mix,
    noise::{fbm3, noise3},
    paint,
    rng::hash01,
    Theme,
};
use std::f32::consts::TAU;

/// Smoothed pointer position in `rect` (or a wandering point when idle), plus "is real".
fn pointer(ui: &Ui, resp: &Response, rect: Rect) -> (Pos2, bool) {
    let time = glowkit_core::time(ui);
    let auto = pos2(rect.center().x + rect.width() * 0.3 * (time * 0.45).sin(), rect.center().y + rect.height() * 0.25 * (time * 0.7).cos());
    let target = resp.hover_pos().unwrap_or(auto);
    let x = glowkit_core::anim::follow(ui, resp.id.with("px"), target.x, 6.0);
    let y = glowkit_core::anim::follow(ui, resp.id.with("py"), target.y, 6.0);
    (pos2(x, y), resp.hovered())
}

macro_rules! bg_common {
    ($t:ty) => {
        impl $t {
            /// Size (defaults to available width x 360).
            pub fn size(mut self, w: f32, h: f32) -> Self {
                self.size = Some(vec2(w, h));
                self
            }
            /// Animation speed multiplier.
            pub fn speed(mut self, s: f32) -> Self {
                self.speed = s;
                self
            }
        }
        impl Widget for $t {
            fn ui(self, ui: &mut Ui) -> Response {
                let size = glowkit_core::desired_size(ui, self.size, 360.0);
                let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
                if ui.is_rect_visible(rect) {
                    glowkit_core::animate(ui);
                    let (ptr, _) = pointer(ui, &resp, rect);
                    self.paint_at(ui, rect, ptr);
                }
                resp
            }
        }
    };
}

/// A dark, slowly flowing veil of light: domain-warped noise lit in a single tint, like light
/// through smoke. Light themes get a pale version.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_blocks::DarkVeil::new());
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint_at(...)`"]
pub struct DarkVeil {
    size: Option<Vec2>,
    speed: f32,
    tint: Option<Color32>,
    hue_shift: bool,
}

impl Default for DarkVeil {
    fn default() -> Self {
        Self::new()
    }
}

impl DarkVeil {
    /// A veil tinted with the theme accent.
    pub fn new() -> Self {
        Self { size: None, speed: 1.0, tint: None, hue_shift: true }
    }
    /// Light color of the veil.
    pub fn tint(mut self, c: Color32) -> Self {
        self.tint = Some(c);
        self
    }
    /// Slowly drift the tint toward the second gradient color.
    pub fn hue_shift(mut self, b: bool) -> Self {
        self.hue_shift = b;
        self
    }
    /// Paint into `rect` (the pointer gently brightens the veil around it).
    pub fn paint_at(&self, ui: &Ui, rect: Rect, ptr: Pos2) {
        let t = Theme::of(ui);
        let time = glowkit_core::time(ui) * self.speed;
        let tint_a = self.tint.unwrap_or(t.gradient[0]);
        let tint = if self.hue_shift { tint_a.lerp_to_gamma(t.gradient[1], 0.5 + 0.5 * (time * 0.07).sin()) } else { tint_a };
        let (base, hi) = if t.dark { (hex("#040208"), mix(tint, Color32::WHITE, 0.25)) } else { (hex("#FBFAFF"), mix(tint, Color32::WHITE, 0.15)) };
        let aspect = rect.width() / rect.height().max(1.0);
        let p = ui.painter_at(rect);
        let pu = vec2((ptr.x - rect.min.x) / rect.width(), (ptr.y - rect.min.y) / rect.height());
        let cols = (rect.width() / 12.0).ceil() as usize;
        let rows = (rect.height() / 12.0).ceil() as usize;
        paint::shader_grid(&p, rect, cols, rows, |uv| {
            let x = uv.x * aspect * 1.4;
            let y = uv.y * 1.4;
            let qx = fbm3(x + time * 0.05, y, time * 0.06, 3);
            let qy = fbm3(x + 5.2, y - time * 0.04, 1.3 + time * 0.05, 3);
            let v = fbm3(x + 2.2 * qx, y + 2.2 * qy, time * 0.08, 4);
            // long diagonal folds
            let fold = (0.5 + 0.5 * ((x * 0.8 - y * 1.6) * 2.0 + v * 4.0 + time * 0.2).sin()).powf(3.0);
            let d = ((uv - pu) * vec2(aspect, 1.0)).length();
            let spot = (1.0 - d / 0.6).clamp(0.0, 1.0).powf(2.0) * 0.35;
            let lum = (smoothstep(-0.25, 0.65, v) * 0.75 + fold * 0.35 + spot).clamp(0.0, 1.2);
            let c = base.lerp_to_gamma(tint, (lum * 0.9).min(1.0));
            let c = c.lerp_to_gamma(hi, smoothstep(0.85, 1.2, lum));
            // edge vignette
            let vg = (uv.x * (1.0 - uv.x) * uv.y * (1.0 - uv.y) * 16.0).powf(0.25).min(1.0);
            base.lerp_to_gamma(c, vg)
        });
    }
}
bg_common!(DarkVeil);

/// Smooth bands of color that bend and flow, like light through a prism folded over itself.
/// The bends lean toward the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_blocks::ColorBends::new());
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint_at(...)`"]
pub struct ColorBends {
    size: Option<Vec2>,
    speed: f32,
    colors: Vec<Color32>,
    bands: f32,
}

impl Default for ColorBends {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorBends {
    /// Bends in a vivid default palette.
    pub fn new() -> Self {
        Self { size: None, speed: 1.0, colors: Vec::new(), bands: 2.5 }
    }
    /// Palette (wraps around).
    pub fn colors(mut self, c: &[Color32]) -> Self {
        self.colors = c.to_vec();
        self
    }
    /// How many color bands fit across.
    pub fn bands(mut self, n: f32) -> Self {
        self.bands = n.max(0.5);
        self
    }
    /// Paint into `rect`, bending toward `ptr`.
    pub fn paint_at(&self, ui: &Ui, rect: Rect, ptr: Pos2) {
        let t = Theme::of(ui);
        let colors = if self.colors.is_empty() { vec![hex("#FF3D7F"), hex("#FF9F1C"), hex("#FFE14D"), hex("#2EC4B6"), hex("#3A86FF"), hex("#8338EC")] } else { self.colors.clone() };
        let time = glowkit_core::time(ui) * self.speed;
        let aspect = rect.width() / rect.height().max(1.0);
        let pu = vec2((ptr.x - rect.min.x) / rect.width(), (ptr.y - rect.min.y) / rect.height());
        let bg = t.background;
        let p = ui.painter_at(rect);
        let cols = (rect.width() / 10.0).ceil() as usize;
        let rows = (rect.height() / 10.0).ceil() as usize;
        let bands = self.bands;
        paint::shader_grid(&p, rect, cols, rows, |uv| {
            let x = (uv.x - 0.5) * aspect;
            let y = uv.y - 0.5;
            let d = (uv - pu) * vec2(aspect, 1.0);
            let pull = (-d.length_sq() * 6.0).exp() * 0.35;
            let h = x * 0.75 + y * 0.55
                + 0.22 * (y * 3.1 + time * 0.6).sin()
                + 0.16 * (x * 2.3 - time * 0.45).sin() * (y * 1.7 + time * 0.3).cos()
                + pull * (d.y * 4.0).sin();
            let u = h * bands * 0.35 - time * 0.05;
            let c = gradient_cyclic(&colors, u);
            // soft ridges between the bands
            let ridge = 0.62 + 0.38 * (u * colors.len() as f32 * TAU).cos().abs().powf(0.5);
            let c = mix(Color32::BLACK, c, ridge);
            // fade into the page at the edges
            let vg = (uv.x * (1.0 - uv.x) * uv.y * (1.0 - uv.y) * 16.0).powf(0.35).min(1.0);
            bg.lerp_to_gamma(c, vg * if t.dark { 0.95 } else { 0.85 })
        });
    }
}
bg_common!(ColorBends);

/// Venetian blinds made of gradient slats: each slat is lit across its width, the colors flow
/// across the whole set, and a spotlight follows the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_blocks::GradientBlinds::new().slats(16));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint_at(...)`"]
pub struct GradientBlinds {
    size: Option<Vec2>,
    speed: f32,
    colors: Vec<Color32>,
    slats: usize,
    angle: f32,
}

impl Default for GradientBlinds {
    fn default() -> Self {
        Self::new()
    }
}

impl GradientBlinds {
    /// Blinds in the theme gradient.
    pub fn new() -> Self {
        Self { size: None, speed: 1.0, colors: Vec::new(), slats: 16, angle: 0.0 }
    }
    /// Gradient stops.
    pub fn colors(mut self, c: &[Color32]) -> Self {
        self.colors = c.to_vec();
        self
    }
    /// Number of slats.
    pub fn slats(mut self, n: usize) -> Self {
        self.slats = n.clamp(2, 80);
        self
    }
    /// Shear of the slats in degrees (0 = vertical).
    pub fn angle(mut self, deg: f32) -> Self {
        self.angle = deg;
        self
    }
    /// Paint into `rect` with the spotlight at `ptr`.
    pub fn paint_at(&self, ui: &Ui, rect: Rect, ptr: Pos2) {
        let t = Theme::of(ui);
        let colors = if self.colors.is_empty() { vec![t.gradient[0], t.gradient[1], t.gradient[2], t.gradient[0]] } else { self.colors.clone() };
        let time = glowkit_core::time(ui) * self.speed;
        let p = ui.painter_at(rect);
        let n = self.slats;
        let sw = rect.width() / n as f32;
        let shear = self.angle.to_radians().tan();
        let rows = 24;
        let sub = 6;
        let dark = if t.dark { hex("#050507") } else { hex("#F4F4F6") };
        let mut m = Mesh::default();
        for s in 0..n {
            let base_i = m.vertices.len() as u32;
            for r in 0..=rows {
                let v = r as f32 / rows as f32;
                let y = rect.min.y + v * rect.height();
                for k in 0..=sub {
                    let a = k as f32 / sub as f32;
                    let x = rect.min.x + (s as f32 + a) * sw + (v - 0.5) * rect.height() * shear;
                    let q = pos2(x, y);
                    let u = (q.x - rect.min.x) / rect.width();
                    let flow = 0.5 + 0.5 * (u * 2.0 + v * 0.8 - time * 0.25 + (time * 0.4 + s as f32 * 0.35).sin() * 0.12).sin();
                    let c = gradient(&colors, (u * 0.8 + 0.2 * flow).clamp(0.0, 1.0));
                    // per-slat lighting: bright leading edge, falloff, thin dark gap
                    let shade = (1.0 - a).powf(1.6) * 0.55 + 0.45 * (1.0 - (a - 0.25).abs() * 1.3).clamp(0.0, 1.0);
                    let gap = smoothstep(0.0, 0.06, a) * smoothstep(1.0, 0.94, a);
                    let d = (q - ptr).length() / rect.height();
                    let spot = (1.0 - d / 0.9).clamp(0.0, 1.0).powf(2.0);
                    let lum = (shade * (0.35 + 0.65 * flow) + spot * 0.7) * gap;
                    let col = dark.lerp_to_gamma(c, lum.clamp(0.0, 1.0));
                    let col = col.lerp_to_gamma(Color32::WHITE, (spot * shade * 0.35).clamp(0.0, 0.5));
                    m.colored_vertex(q, col);
                }
            }
            let w = (sub + 1) as u32;
            for r in 0..rows as u32 {
                for k in 0..sub as u32 {
                    let i = base_i + r * w + k;
                    m.add_triangle(i, i + 1, i + w + 1);
                    m.add_triangle(i, i + w + 1, i + w);
                }
            }
        }
        p.add(Shape::mesh(m));
        paint::vignette(&p, rect, with_alpha(dark, 0.9), 0.6);
    }
}
bg_common!(GradientBlinds);

/// Pixel shape for [`PixelBlast`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PixelShape {
    /// Square pixels.
    #[default]
    Square,
    /// Round dots.
    Circle,
    /// Rotated squares.
    Diamond,
}

/// Pattern driving a [`PixelBlast`] field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PixelPattern {
    /// Drifting fractal noise.
    #[default]
    Noise,
    /// Rolling interference waves (retro dither look).
    Waves,
}

const BAYER4: [f32; 16] = [0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0];

/// A pixelated, ordered-dithered (Bayer 4x4) animated field: noise or waves quantized into
/// on/off pixels, with ripples bursting from the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_blocks::{PixelBlast, PixelShape};
/// ui.add(PixelBlast::new().shape(PixelShape::Circle).pixel_size(7.0));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint_at(...)`"]
pub struct PixelBlast {
    size: Option<Vec2>,
    speed: f32,
    pixel: f32,
    shape: PixelShape,
    pattern: PixelPattern,
    colors: Option<[Color32; 2]>,
}

impl Default for PixelBlast {
    fn default() -> Self {
        Self::new()
    }
}

impl PixelBlast {
    /// Square pixels over drifting noise.
    pub fn new() -> Self {
        Self { size: None, speed: 1.0, pixel: 8.0, shape: PixelShape::Square, pattern: PixelPattern::Noise, colors: None }
    }
    /// Pixel pitch in points.
    pub fn pixel_size(mut self, s: f32) -> Self {
        self.pixel = s.clamp(3.0, 40.0);
        self
    }
    /// Pixel shape.
    pub fn shape(mut self, s: PixelShape) -> Self {
        self.shape = s;
        self
    }
    /// Driving pattern.
    pub fn pattern(mut self, p: PixelPattern) -> Self {
        self.pattern = p;
        self
    }
    /// Pixel color gradient (left → right). Defaults to the accent and second gradient color.
    pub fn colors(mut self, a: Color32, b: Color32) -> Self {
        self.colors = Some([a, b]);
        self
    }
    /// Paint into `rect`, rippling around `ptr`.
    pub fn paint_at(&self, ui: &Ui, rect: Rect, ptr: Pos2) {
        let t = Theme::of(ui);
        let [ca, cb] = self.colors.unwrap_or([t.gradient[0], t.gradient[1]]);
        let time = glowkit_core::time(ui) * self.speed;
        let pitch = self.pixel;
        let cols = (rect.width() / pitch) as usize;
        let rows = (rect.height() / pitch) as usize;
        let ox = rect.min.x + (rect.width() - cols as f32 * pitch) / 2.0;
        let oy = rect.min.y + (rect.height() - rows as f32 * pitch) / 2.0;
        let mut m = Mesh::default();
        let dot = pitch * 0.72;
        for y in 0..rows {
            for x in 0..cols {
                let c = pos2(ox + (x as f32 + 0.5) * pitch, oy + (y as f32 + 0.5) * pitch);
                let u = (c.x - rect.min.x) / rect.width();
                let v = (c.y - rect.min.y) / rect.height();
                let mut val = match self.pattern {
                    PixelPattern::Noise => 0.5 + 0.6 * fbm3(u * 3.2, v * 1.8 - time * 0.05, time * 0.15, 3),
                    PixelPattern::Waves => {
                        let a = (u * 9.0 + time * 0.9 + (v * 4.0 + time * 0.3).sin() * 1.4).sin();
                        let b = (v * 7.0 - time * 0.6 + (u * 3.0).cos() * 1.2).sin();
                        0.5 + 0.25 * (a + b) * (0.6 + 0.4 * noise3(u * 2.0, v * 2.0, time * 0.1))
                    }
                };
                // pointer ripple
                let d = c.distance(ptr);
                val += 0.35 * (d * 0.05 - time * 4.0).sin() * (-d / 120.0).exp();
                // edge fade
                let edge = smoothstep(0.0, 0.18, u.min(1.0 - u)) * smoothstep(0.0, 0.18, v.min(1.0 - v));
                val *= 0.25 + 0.75 * edge;
                let th = (BAYER4[(y % 4) * 4 + (x % 4)] + 0.5) / 16.0;
                if val <= th {
                    continue;
                }
                let col = ca.lerp_to_gamma(cb, (u * 0.8 + v * 0.2).clamp(0.0, 1.0));
                let a = (0.55 + 0.45 * ((val - th) * 3.0).min(1.0)) * (0.3 + 0.7 * edge);
                let col = with_alpha(col, a);
                match self.shape {
                    PixelShape::Square => mesh_rect(&mut m, Rect::from_center_size(c, Vec2::splat(dot)), col),
                    PixelShape::Circle => mesh_dot(&mut m, c, dot * 0.5, col),
                    PixelShape::Diamond => {
                        let i = m.vertices.len() as u32;
                        let r = dot * 0.62;
                        for q in [c - vec2(0.0, r), c + vec2(r, 0.0), c + vec2(0.0, r), c - vec2(r, 0.0)] {
                            m.colored_vertex(q, col);
                        }
                        m.add_triangle(i, i + 1, i + 2);
                        m.add_triangle(i, i + 2, i + 3);
                    }
                }
            }
        }
        ui.painter_at(rect).add(Shape::mesh(m));
    }
}
bg_common!(PixelBlast);

/// A glitchy green CRT: a grid of flickering glyphs under a slow brightness wave, with
/// scanlines, rolling interference, occasional row glitches and a pointer glow.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_blocks::FaultyTerminal::new());
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint_at(...)`"]
pub struct FaultyTerminal {
    size: Option<Vec2>,
    speed: f32,
    color: Color32,
    glyph: f32,
}

impl Default for FaultyTerminal {
    fn default() -> Self {
        Self::new()
    }
}

impl FaultyTerminal {
    /// Phosphor-green terminal.
    pub fn new() -> Self {
        Self { size: None, speed: 1.0, color: hex("#4ADE80"), glyph: 13.0 }
    }
    /// Phosphor color.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = c;
        self
    }
    /// Glyph size in points.
    pub fn glyph_size(mut self, s: f32) -> Self {
        self.glyph = s.clamp(6.0, 40.0);
        self
    }
    /// Paint into `rect`, glowing around `ptr`.
    pub fn paint_at(&self, ui: &Ui, rect: Rect, ptr: Pos2) {
        let time = glowkit_core::time(ui) * self.speed;
        let p = ui.painter_at(rect);
        let bg = hex("#020803");
        p.rect_filled(rect, 0.0, bg);
        let font = FontId::monospace(self.glyph);
        let cw = self.glyph * 0.75;
        let ch = self.glyph * 1.25;
        let cols = (rect.width() / cw) as usize;
        let rows = (rect.height() / ch) as usize;
        let ox = rect.min.x + (rect.width() - cols as f32 * cw) / 2.0;
        let oy = rect.min.y + (rect.height() - rows as f32 * ch) / 2.0;
        let charset: Vec<char> = "01アイウエオカキクケコｱｲｳｴｵ#$%&*+=<>/\\|{}[]".chars().collect();
        let tick = (time * 8.0) as u64;
        let flicker = 0.92 + 0.08 * hash01(tick);
        // a whole-screen glitch every few seconds
        let glitch_row = if hash01(tick / 3 + 77) < 0.12 { Some((hash01(tick + 5) * rows as f32) as usize) } else { None };
        let glyphs: Vec<std::sync::Arc<egui::Galley>> = charset.iter().map(|c| p.layout_no_wrap(c.to_string(), font.clone(), Color32::PLACEHOLDER)).collect();
        for y in 0..rows {
            let shift = if glitch_row.is_some_and(|g| g.abs_diff(y) <= 1) { (hash01(tick * 7 + y as u64) - 0.5) * cw * 6.0 } else { 0.0 };
            for x in 0..cols {
                let i = (y * cols + x) as u64;
                let c = pos2(ox + x as f32 * cw + shift, oy + y as f32 * ch);
                let u = x as f32 / cols as f32;
                let v = y as f32 / rows as f32;
                let wave = 0.5 + 0.5 * fbm3(u * 3.0, v * 2.0 - time * 0.12, time * 0.1, 2);
                let d = c.distance(ptr) / 140.0;
                let glow = (1.0 - d).clamp(0.0, 1.0).powf(2.0);
                let roll = (0.5 + 0.5 * ((v - time * 0.15) * TAU * 1.5).sin()).powf(6.0) * 0.4;
                let b = (wave.powf(2.2) * 0.85 + glow * 0.9 + roll) * flicker;
                if b < 0.06 {
                    continue;
                }
                let k = (hash01(i * 13 + (tick + i % 11) / (4 + i % 7)) * glyphs.len() as f32) as usize % glyphs.len();
                let col = mix(bg, self.color, b.min(1.0));
                let col = if b > 1.0 { col.lerp_to_gamma(Color32::WHITE, (b - 1.0).min(0.6)) } else { col };
                p.galley(c, glyphs[k].clone(), col);
            }
        }
        // scanlines
        let mut m = Mesh::default();
        let mut y = rect.min.y;
        while y < rect.max.y {
            mesh_rect(&mut m, Rect::from_min_size(pos2(rect.min.x, y), vec2(rect.width(), 1.0)), Color32::from_black_alpha(70));
            y += 3.0;
        }
        p.add(Shape::mesh(m));
        paint::glow(&p, ptr, 160.0, with_alpha(self.color, 0.06));
        paint::vignette(&p, rect, Color32::BLACK, 0.8);
    }
}
bg_common!(FaultyTerminal);
