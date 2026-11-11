//! "Shader" backgrounds: full-rect color fields evaluated per vertex with `shader_grid`.

use crate::{background, ghost, pointer, smoothstep, Base, Lut, Rgb};
use egui::{vec2, Color32, Rect, Ui, Vec2};
use glowkit_core::paint::shader_grid;
use std::f32::consts::TAU;

/// Pointer (or None) in the aspect-corrected, centered space `(-aspect/2..aspect/2, -0.5..0.5)`.
fn centered_pointer(ui: &Ui, rect: Rect) -> Option<Vec2> {
    pointer(ui, rect).map(|p| {
        let a = rect.width() / rect.height().max(1.0);
        let u = (p - rect.min) / rect.size();
        vec2((u.x - 0.5) * a, u.y - 0.5)
    })
}

fn centered(uv: Vec2, aspect: f32) -> Vec2 {
    vec2((uv.x - 0.5) * aspect, uv.y - 0.5)
}

fn rot(v: Vec2, a: f32) -> Vec2 {
    let (s, c) = a.sin_cos();
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

// ---------------------------------------------------------------------------------------------
// Silk
// ---------------------------------------------------------------------------------------------

/// Smooth, slowly undulating folds of shaded silk with soft specular highlights.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::Silk;
/// ui.add(Silk::new().color(glowkit_core::hex("#7B7481")).speed(1.2));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct Silk {
    base: Base,
    color: Option<Color32>,
    speed: f32,
    scale: f32,
    rotation: f32,
}

impl Default for Silk {
    fn default() -> Self {
        Self::new()
    }
}

impl Silk {
    /// Silk tinted from the theme accent.
    pub fn new() -> Self {
        Self { base: Base::default(), color: None, speed: 1.0, scale: 1.0, rotation: 0.0 }
    }
    /// Base color of the fabric (default: a muted tint of the theme accent).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Animation speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Fold scale: bigger is finer folds (default 1).
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale.clamp(0.2, 3.0);
        self
    }
    /// Rotation of the fold direction in degrees.
    pub fn rotation(mut self, degrees: f32) -> Self {
        self.rotation = degrees;
        self
    }

    /// Paint the silk into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_silk") else { return };
        let t = &f.theme;
        let base = Rgb::of(self.color.unwrap_or_else(|| glowkit_core::mix(t.accent, t.muted_foreground, 0.5)));
        let time = f.time * self.speed;
        let aspect = rect.width() / rect.height().max(1.0);
        let (scale, rotation) = (self.scale * 2.2, self.rotation.to_radians());
        let light_theme = !t.dark;
        shader_grid(&f.painter, rect, 96, 48, |uv| {
            let p = rot(centered(uv, aspect), rotation) * scale;
            let (x, y) = (p.x, p.y);
            // long diagonal folds, gently warped
            let warp = 0.55 * (1.3 * x - 0.35 * time).sin() + 0.4 * (1.7 * y + 0.9 * x + 0.3 * time).cos();
            let phase = (x * 0.8 + y) * 2.2 + warp * 1.4 - time * 0.4;
            let fold = phase.sin();
            let fine = (phase * 2.7 + 0.6 * (x * 2.0 - time * 0.2).sin()).sin() * 0.12;
            let s = (0.5 + 0.5 * fold + fine).clamp(0.0, 1.0);
            let shade = if light_theme { 0.6 + 0.55 * s.powf(1.3) } else { 0.22 + 0.85 * s.powf(1.3) };
            let spec = s.powi(8) * 0.28;
            base.scale(shade).lerp(Rgb(1.0, 1.0, 1.0), spec).to_color()
        });
    }
}

background!(Silk);

// ---------------------------------------------------------------------------------------------
// Iridescence
// ---------------------------------------------------------------------------------------------

/// An oily, iridescent film of shifting rainbow colors, like light on a soap bubble. The film
/// swirls away from the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::Iridescence;
/// ui.add(Iridescence::new().speed(0.8));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct Iridescence {
    base: Base,
    tint: Color32,
    speed: f32,
    amplitude: f32,
    interactive: bool,
}

impl Default for Iridescence {
    fn default() -> Self {
        Self::new()
    }
}

impl Iridescence {
    /// A pastel rainbow film.
    pub fn new() -> Self {
        Self { base: Base::default(), tint: Color32::WHITE, speed: 1.0, amplitude: 1.0, interactive: true }
    }
    /// Multiply the film by this color (default white = full rainbow).
    pub fn color(mut self, c: Color32) -> Self {
        self.tint = c;
        self
    }
    /// Animation speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Strength of the flowing distortion (default 1).
    pub fn amplitude(mut self, amplitude: f32) -> Self {
        self.amplitude = amplitude;
        self
    }
    /// Whether the pointer stirs the film (default true).
    pub fn interactive(mut self, on: bool) -> Self {
        self.interactive = on;
        self
    }

    /// Paint the film into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_iridescence") else { return };
        let tint = Rgb::of(self.tint);
        let time = f.time * self.speed * 0.5;
        let aspect = rect.width() / rect.height().max(1.0);
        let m = if self.interactive { centered_pointer(ui, rect) } else { None };
        let amp = self.amplitude;
        shader_grid(&f.painter, rect, 80, 44, |uv| {
            let mut p = centered(uv, aspect) * 3.0;
            if let Some(m) = m {
                let d = p - m * 3.0;
                p += d * 0.4 * (-d.length_sq() * 1.2).exp();
            }
            for i in 0..5 {
                let fi = i as f32;
                let k = 0.8 + fi * 0.25;
                p += vec2((p.y * k + time + fi * 1.1).sin(), (p.x * k * 0.9 - time * 0.8 + fi * 1.7).cos()) * (0.3 / k) * amp;
            }
            let v = p.x * 0.18 + p.y * 0.26 + time * 0.1;
            let film = Rgb(0.5 + 0.5 * (TAU * v).cos(), 0.5 + 0.5 * (TAU * (v + 0.3)).cos(), 0.5 + 0.5 * (TAU * (v + 0.62)).cos());
            // soft sheen: brighter where the film folds
            let sheen = 0.5 + 0.5 * (p.x * 1.3 - p.y * 0.9).sin();
            let c = film.lerp(Rgb(1.0, 1.0, 1.0), 0.38 + 0.2 * sheen).scale(0.86 + 0.14 * sheen);
            Rgb(c.0 * tint.0, c.1 * tint.1, c.2 * tint.2).to_color()
        });
    }
}

background!(Iridescence);

// ---------------------------------------------------------------------------------------------
// Swirl
// ---------------------------------------------------------------------------------------------

/// Psychedelic swirling paint: three colors stirred together into a slowly spinning vortex,
/// with crisp marbled bands.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::Swirl;
/// use glowkit_core::hex;
/// ui.add(Swirl::new().colors([hex("#DE443B"), hex("#006BB4"), hex("#162325")]));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct Swirl {
    base: Base,
    colors: Option<[Color32; 3]>,
    speed: f32,
    spin: f32,
    contrast: f32,
}

impl Default for Swirl {
    fn default() -> Self {
        Self::new()
    }
}

impl Swirl {
    /// Swirling paint in the theme's colors.
    pub fn new() -> Self {
        Self { base: Base::default(), colors: None, speed: 1.0, spin: 1.0, contrast: 1.0 }
    }
    /// The three paint colors: two vivid and one dark (default: from the theme).
    pub fn colors(mut self, colors: [Color32; 3]) -> Self {
        self.colors = Some(colors);
        self
    }
    /// Animation speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// How strongly the paint twists toward the center (default 1).
    pub fn spin(mut self, spin: f32) -> Self {
        self.spin = spin;
        self
    }
    /// Sharpness of the bands between colors (default 1).
    pub fn contrast(mut self, contrast: f32) -> Self {
        self.contrast = contrast.clamp(0.2, 3.0);
        self
    }

    /// Paint the swirl into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_swirl") else { return };
        let t = &f.theme;
        let [c1, c2, c3] = self
            .colors
            .unwrap_or_else(|| {
                if t.dark {
                    [t.gradient[1], t.accent, glowkit_core::color::shade(t.background, 0.08)]
                } else {
                    [t.gradient[1], t.accent, glowkit_core::color::shade(t.chart[1], 0.35)]
                }
            })
            .map(Rgb::of);
        let time = f.time * self.speed;
        let aspect = rect.width() / rect.height().max(1.0);
        let (spin, contrast) = (self.spin, self.contrast);
        shader_grid(&f.painter, rect, 72, 40, |uv| {
            let p = centered(uv, aspect);
            let r = p.length();
            let p = rot(p, spin * (1.6 / (r + 0.35)) + time * 0.15 * spin) * 3.2;
            let mut q = p;
            for i in 0..5 {
                let fi = i as f32;
                q += vec2((q.y * 0.9 + time * 0.25 + fi).sin(), (q.x * 0.8 - time * 0.2 + fi * 1.3).cos()) * 0.55;
            }
            let a = 0.5 + 0.5 * (q.x * 0.9 + q.y * 0.4).sin();
            let b = 0.5 + 0.5 * (q.y * 0.8 - q.x * 0.5 + 1.3).cos();
            let w = 0.18 / contrast;
            let mut c = c3;
            c = c.lerp(c1, smoothstep(0.55 - w, 0.55 + w, a));
            c = c.lerp(c2, smoothstep(0.6 - w, 0.6 + w, b) * 0.95);
            // subtle sheen on the bands
            let sheen = (1.0 - ((a - 0.55).abs() / 0.08).min(1.0)) * 0.12;
            c.lerp(Rgb(1.0, 1.0, 1.0), sheen).to_color()
        });
    }
}

background!(Swirl);

// ---------------------------------------------------------------------------------------------
// Liquid metal
// ---------------------------------------------------------------------------------------------

/// Rippling liquid chrome: a reflective metallic surface with flowing highlights. The pointer
/// sends ripples through it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::LiquidMetal;
/// ui.add(LiquidMetal::new().tint(glowkit_core::hex("#8FB8FF")));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct LiquidMetal {
    base: Base,
    tint: Option<Color32>,
    speed: f32,
    amplitude: f32,
    interactive: bool,
}

impl Default for LiquidMetal {
    fn default() -> Self {
        Self::new()
    }
}

impl LiquidMetal {
    /// Neutral chrome with a hint of the theme accent.
    pub fn new() -> Self {
        Self { base: Base::default(), tint: None, speed: 1.0, amplitude: 1.0, interactive: true }
    }
    /// Tint of the reflections (default: near-white with a hint of the accent).
    pub fn tint(mut self, c: Color32) -> Self {
        self.tint = Some(c);
        self
    }
    /// Animation speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Wave height (default 1).
    pub fn amplitude(mut self, amplitude: f32) -> Self {
        self.amplitude = amplitude;
        self
    }
    /// Whether the pointer makes ripples (default true).
    pub fn interactive(mut self, on: bool) -> Self {
        self.interactive = on;
        self
    }

    /// Paint the metal into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_liquid_metal") else { return };
        let t = &f.theme;
        let tint = Rgb::of(self.tint.unwrap_or_else(|| glowkit_core::mix(glowkit_core::hex("#D9DEE8"), t.accent, 0.1)));
        let dark = Rgb::of(glowkit_core::mix(Color32::from_rgb(12, 13, 18), t.accent, 0.06));
        let time = f.time * self.speed * 0.6;
        let aspect = rect.width() / rect.height().max(1.0);
        let m = if self.interactive { centered_pointer(ui, rect) } else { None };
        let amp = self.amplitude;
        let height = |p: Vec2| -> f32 {
            let mut h = (p.x * 3.0 + 1.3 * (p.y * 2.2 + time).sin() + time * 0.7).sin() * 0.5
                + (p.y * 3.4 - 1.1 * (p.x * 1.9 - time * 0.8).cos()).sin() * 0.35
                + ((p.x + p.y) * 2.1 + time * 0.5).sin() * 0.25;
            if let Some(m) = m {
                let d = (p - m).length();
                h += (d * 18.0 - time * 8.0).sin() * 0.18 * (-d * 3.5).exp();
            }
            h * amp
        };
        shader_grid(&f.painter, rect, 96, 52, |uv| {
            let p = centered(uv, aspect) * 1.4;
            let e = 0.015;
            let h = height(p);
            let nx = (height(p + vec2(e, 0.0)) - h) / e;
            let ny = (height(p + vec2(0.0, e)) - h) / e;
            // reflect a soft, banded "studio" environment
            let r = nx * 0.1 + ny * 0.16 + h * 0.25 + p.y * 0.7;
            let env = 0.5 + 0.5 * (r * 5.5 + 0.6).sin();
            let v = env.powf(1.6);
            let rim = smoothstep(0.88, 1.0, env) * 0.5;
            dark.lerp(tint, v).lerp(Rgb(1.0, 1.0, 1.0), rim).to_color()
        });
    }
}

background!(LiquidMetal);

// ---------------------------------------------------------------------------------------------
// Plasma
// ---------------------------------------------------------------------------------------------

/// Classic demoscene plasma: interfering sine fields mapped to a glowing, looping palette.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::Plasma;
/// ui.add(Plasma::new().colors(&glowkit_core::color::palettes::NEON).scale(1.2));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct Plasma {
    base: Base,
    colors: Option<Vec<Color32>>,
    speed: f32,
    scale: f32,
    intensity: f32,
    interactive: bool,
}

impl Default for Plasma {
    fn default() -> Self {
        Self::new()
    }
}

impl Plasma {
    /// Plasma in the theme's colors.
    pub fn new() -> Self {
        Self { base: Base::default(), colors: None, speed: 1.0, scale: 1.0, intensity: 1.0, interactive: true }
    }
    /// The looping palette (default: theme gradient plus a chart color).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }
    /// Animation speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Pattern scale: bigger is busier (default 1).
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale.clamp(0.2, 3.0);
        self
    }
    /// Brightness, `0..=1.5` (default 1).
    pub fn intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.clamp(0.0, 1.5);
        self
    }
    /// Whether the pattern bends toward the pointer (default true).
    pub fn interactive(mut self, on: bool) -> Self {
        self.interactive = on;
        self
    }

    /// Paint the plasma into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_plasma") else { return };
        let t = &f.theme;
        let colors = self.colors.clone().unwrap_or_else(|| vec![t.gradient[0], t.gradient[1], t.chart[1], t.chart[5]]);
        let lut = Lut::new(&colors, true);
        let bg = Rgb::of(f.bg);
        let dark = t.dark;
        let time = f.time * self.speed * 0.6;
        let aspect = rect.width() / rect.height().max(1.0);
        let m = if self.interactive { centered_pointer(ui, rect) } else { None }.unwrap_or_else(|| {
            let g = ghost(rect, time);
            let u = (g - rect.min) / rect.size();
            vec2((u.x - 0.5) * aspect, u.y - 0.5)
        });
        let (k, intensity) = (4.0 * self.scale, self.intensity);
        shader_grid(&f.painter, rect, 72, 40, |uv| {
            let p = centered(uv, aspect);
            let (x, y) = (p.x * k, p.y * k);
            let cx = x + 1.5 * (time * 0.4).sin();
            let cy = y + 1.5 * (time * 0.3).cos();
            let d = (p - m).length() * k;
            let v = (x + time).sin()
                + ((y + time) * 0.7).sin()
                + ((x * 0.6 + y * 0.8 + time) * 0.8).sin()
                + ((cx * cx + cy * cy + 1.0).sqrt() + time).sin()
                + (d * 0.9 - time * 1.5).sin() * 0.6;
            let c = lut.sample(v * 0.2 + time * 0.05);
            let glow = (0.5 + 0.5 * (v * 1.4).sin()).powf(0.8) * intensity;
            if dark {
                bg.add(c.scale((0.12 + glow * 0.9).min(1.0))).add(Rgb(1.0, 1.0, 1.0).scale((glow - 0.9).max(0.0) * 0.5)).to_color()
            } else {
                bg.lerp(c, (0.15 + glow * 0.8).min(1.0)).to_color()
            }
        });
    }
}

background!(Plasma);
