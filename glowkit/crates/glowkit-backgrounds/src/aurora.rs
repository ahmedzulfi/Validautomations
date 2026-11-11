use crate::{background, ghost, pointer, Base, Lut, Rgb};
use egui::{vec2, Color32, Pos2, Rect, Ui, Vec2};
use glowkit_core::noise::{fbm3, noise3};
use glowkit_core::paint::shader_grid;
use glowkit_core::Theme;

// ---------------------------------------------------------------------------------------------
// Aurora
// ---------------------------------------------------------------------------------------------

/// Flowing curtains of northern light: wavy, ray-streaked bands of color that drift and shimmer
/// across the top of the area and fade softly into the background.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::Aurora;
/// ui.add(Aurora::new());
/// ui.add(Aurora::new().colors(&[glowkit_core::hex("#3A29FF"), glowkit_core::hex("#FF94B4"), glowkit_core::hex("#FF3232")]).amplitude(1.4));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct Aurora {
    base: Base,
    colors: Option<Vec<Color32>>,
    speed: f32,
    amplitude: f32,
    blend: f32,
    height: f32,
}

impl Default for Aurora {
    fn default() -> Self {
        Self::new()
    }
}

impl Aurora {
    /// An aurora in the theme's colors.
    pub fn new() -> Self {
        Self { base: Base::default(), colors: None, speed: 1.0, amplitude: 1.0, blend: 0.9, height: 0.5 }
    }
    /// Gradient stops across the curtains, left to right (default: theme gradient).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }
    /// Animation speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// How far the curtains wave up and down (default 1).
    pub fn amplitude(mut self, amplitude: f32) -> Self {
        self.amplitude = amplitude;
        self
    }
    /// Light intensity, `0..=1.5` (default 0.9).
    pub fn blend(mut self, blend: f32) -> Self {
        self.blend = blend.clamp(0.0, 1.5);
        self
    }
    /// Where the lower edge of the curtains hangs, as a fraction of the height (default 0.5).
    pub fn height(mut self, fraction: f32) -> Self {
        self.height = fraction.clamp(0.1, 0.95);
        self
    }

    /// Paint the aurora into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_aurora") else { return };
        let t = &f.theme;
        let colors = self.colors.clone().unwrap_or_else(|| vec![t.gradient[0], t.chart[1], t.gradient[1]]);
        let lut = Lut::new(&colors, false);
        let bg = Rgb::of(f.bg);
        let time = f.time * self.speed;
        let dark = f.dark();
        let (amp, blend, hgt) = (self.amplitude, self.blend, self.height);
        // everything that depends only on x is computed once per column
        let (cols, rows) = (96usize, 56usize);
        let layers: Vec<[(f32, f32, f32); 3]> = (0..=cols)
            .map(|c| {
                let x = c as f32 / cols as f32;
                let mut out = [(0.0, 0.0, 0.0); 3];
                for (k, o) in out.iter_mut().enumerate() {
                    let kf = k as f32;
                    let edge = hgt + 0.07 * (kf - 1.0)
                        + amp * (0.16 * fbm3(x * 2.1 + kf * 3.7 + time * 0.03, kf * 5.3, time * 0.12, 3)
                            + 0.035 * (x * 9.0 + time * 0.7 + kf * 2.1).sin());
                    let reach = 0.18 + 0.32 * (0.5 + 0.5 * noise3(x * 1.6 + kf * 7.0, time * 0.09, kf + 3.0));
                    let rays = 0.55 + 0.45 * (0.5 + 0.5 * noise3(x * 13.0 + kf * 13.0 + time * 0.1, time * 0.25, kf)).powf(1.5);
                    *o = (edge, reach, rays);
                }
                out
            })
            .collect();
        shader_grid(&f.painter, rect, cols, rows, |uv| {
            let ci = ((uv.x * cols as f32).round() as usize).min(cols);
            let y = uv.y;
            let mut glow = 0.0;
            let mut hue = 0.0;
            for (k, &(edge, reach, rays)) in layers[ci].iter().enumerate() {
                let d = edge - y;
                // bright lower rim, rays reaching up, soft fall-off below
                let rim = (-(d / 0.07).powi(2)).exp();
                let up = (-d.max(0.0) / reach).exp() * rays * crate::smoothstep(-0.09, 0.03, d);
                let v = rim * 0.55 + up * 0.75;
                let w = [1.0, 0.75, 0.55][k];
                glow += v * w;
                hue += v * w * (k as f32 * 0.1 - 0.1);
            }
            let x = uv.x;
            let ct = (x * 0.9 + 0.05 + hue * 0.35 + 0.06 * (time * 0.15 + x * 3.0).sin()).clamp(0.0, 1.0);
            let c = lut.sample(ct);
            let i = (glow * blend * 0.85).min(1.6);
            if dark {
                // additive light, slightly whitened where it is brightest
                let lit = bg.add(c.scale(i.min(1.0)));
                lit.lerp(Rgb(1.0, 1.0, 1.0), ((i - 0.9) * 0.3).clamp(0.0, 0.18)).to_color()
            } else {
                bg.lerp(c, (i * 0.7).min(1.0)).to_color()
            }
        });
    }
}

background!(Aurora);

// ---------------------------------------------------------------------------------------------
// Gradient blobs
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct Follow {
    pos: Option<Pos2>,
}

/// Large soft blobs of color drifting over a tinted gradient, blending into each other like a
/// lava lamp. One extra blob follows the pointer (or wanders on its own when nobody hovers).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::GradientBlobs;
/// ui.add(GradientBlobs::new().interactive(true));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct GradientBlobs {
    base: Base,
    colors: Option<Vec<Color32>>,
    pointer_color: Option<Color32>,
    base_colors: Option<[Color32; 2]>,
    speed: f32,
    size: f32,
    interactive: bool,
}

impl Default for GradientBlobs {
    fn default() -> Self {
        Self::new()
    }
}

impl GradientBlobs {
    /// Blobs in the theme's chart colors.
    pub fn new() -> Self {
        Self { base: Base::default(), colors: None, pointer_color: None, base_colors: None, speed: 1.0, size: 1.0, interactive: true }
    }
    /// Colors of the drifting blobs (default: theme chart palette).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }
    /// Color of the pointer-following blob (default: theme accent).
    pub fn pointer_color(mut self, c: Color32) -> Self {
        self.pointer_color = Some(c);
        self
    }
    /// The two colors of the base gradient, top-left to bottom-right.
    pub fn base_gradient(mut self, from: Color32, to: Color32) -> Self {
        self.base_colors = Some([from, to]);
        self
    }
    /// Animation speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Blob size multiplier (default 1).
    pub fn blob_size(mut self, size: f32) -> Self {
        self.size = size.max(0.1);
        self
    }
    /// Whether a blob follows the pointer (default true).
    pub fn interactive(mut self, on: bool) -> Self {
        self.interactive = on;
        self
    }

    /// Paint the blobs into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_gradient_blobs") else { return };
        let t = &f.theme;
        let colors: Vec<Rgb> = self.colors.clone().unwrap_or_else(|| t.chart[..5].to_vec()).into_iter().map(Rgb::of).collect();
        let [b0, b1] = self.base_colors.unwrap_or_else(|| {
            if t.dark {
                [glowkit_core::mix(f.bg, t.gradient[0], 0.55), glowkit_core::mix(f.bg, t.chart[5], 0.35)]
            } else {
                [glowkit_core::mix(f.bg, t.gradient[0], 0.25), glowkit_core::mix(f.bg, t.chart[1], 0.2)]
            }
        });
        let (b0, b1) = (Rgb::of(b0), Rgb::of(b1));
        let time = f.time * self.speed;
        // pointer blob: eases toward the pointer (or a wandering point)
        let target = pointer(ui, rect).unwrap_or_else(|| ghost(rect, time * 0.8));
        let follow = glowkit_core::with_state(ui, f.id, Follow::default, |s| {
            let p = s.pos.unwrap_or(target);
            let k = 1.0 - (-f.dt * 3.0).exp();
            let np = p + (target - p) * k;
            s.pos = Some(np);
            np
        });
        let aspect = rect.width() / rect.height().max(1.0);
        let mut blobs: Vec<(Vec2, f32, Rgb)> = colors
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let fi = i as f32;
                let cx = (0.5 + 0.42 * (time * (0.13 + 0.03 * fi) + fi * 1.9).sin()) * 1.0;
                let cy = 0.5 + 0.32 * (time * (0.17 + 0.02 * fi) + fi * 2.7).cos();
                let r = (0.5 + 0.12 * (fi * 1.3).sin()) * self.size;
                (vec2(cx * aspect, cy), r, *c)
            })
            .collect();
        if self.interactive {
            let pc = Rgb::of(self.pointer_color.unwrap_or(t.accent));
            let u = (follow - rect.min) / rect.size();
            blobs.push((vec2(u.x * aspect, u.y), 0.4 * self.size, pc));
        }
        let dark = t.dark;
        shader_grid(&f.painter, rect, 56, 32, |uv| {
            let mut c = b0.lerp(b1, (uv.x * 0.5 + uv.y * 0.5).clamp(0.0, 1.0));
            let p = vec2(uv.x * aspect, uv.y);
            for (center, r, col) in &blobs {
                let d2 = (p - *center).length_sq() / (r * r);
                let w = (-d2 * 1.1).exp();
                c = c.lerp(*col, w * if dark { 0.8 } else { 0.7 });
            }
            c.to_color()
        });
    }
}

background!(GradientBlobs);

// ---------------------------------------------------------------------------------------------
// Mesh gradient
// ---------------------------------------------------------------------------------------------

/// A smoothly animated four-color mesh gradient: color points drift and swirl, blending into a
/// silky, organic gradient.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::MeshGradient;
/// use glowkit_core::hex;
/// ui.add(MeshGradient::new().colors([hex("#E0EAFF"), hex("#241D9A"), hex("#F75092"), hex("#9F50D3")]));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct MeshGradient {
    base: Base,
    colors: Option<[Color32; 4]>,
    speed: f32,
    distortion: f32,
    swirl: f32,
}

impl Default for MeshGradient {
    fn default() -> Self {
        Self::new()
    }
}

impl MeshGradient {
    /// A mesh gradient in the theme's colors.
    pub fn new() -> Self {
        Self { base: Base::default(), colors: None, speed: 1.0, distortion: 0.8, swirl: 0.6 }
    }
    /// The four colors (default: derived from the theme gradient).
    pub fn colors(mut self, colors: [Color32; 4]) -> Self {
        self.colors = Some(colors);
        self
    }
    /// Animation speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Organic noise distortion, `0..=2` (default 0.8).
    pub fn distortion(mut self, amount: f32) -> Self {
        self.distortion = amount.clamp(0.0, 2.0);
        self
    }
    /// Swirl strength, `0..=2` (default 0.6).
    pub fn swirl(mut self, amount: f32) -> Self {
        self.swirl = amount.clamp(0.0, 2.0);
        self
    }

    fn default_colors(t: &Theme) -> [Color32; 4] {
        if t.dark {
            [glowkit_core::color::shade(t.gradient[0], -0.25), t.gradient[1], t.chart[1], glowkit_core::color::shade(t.chart[5], -0.15)]
        } else {
            [glowkit_core::color::shade(t.gradient[0], 0.2), glowkit_core::color::shade(t.gradient[1], 0.25), glowkit_core::color::shade(t.chart[1], 0.3), t.chart[5]]
        }
    }

    /// Paint the gradient into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_mesh_gradient") else { return };
        let cols = self.colors.unwrap_or_else(|| Self::default_colors(&f.theme)).map(Rgb::of);
        let time = f.time * self.speed * 0.35;
        let aspect = rect.width() / rect.height().max(1.0);
        let pts: Vec<Vec2> = (0..4)
            .map(|i| {
                let fi = i as f32;
                let a = time * (0.6 + 0.15 * fi) + fi * std::f32::consts::FRAC_PI_2;
                vec2(0.5 + 0.32 * a.cos() + 0.08 * (time * 1.3 + fi).sin(), 0.5 + 0.32 * (a * 1.2 + fi).sin())
            })
            .collect();
        let (dist, swirl) = (self.distortion, self.swirl);
        shader_grid(&f.painter, rect, 56, 32, |uv| {
            let mut p = vec2((uv.x - 0.5) * aspect.clamp(1.0, 1.6) + 0.5, uv.y);
            // domain warp
            p += vec2(noise3(p.x * 2.0, p.y * 2.0, time), noise3(p.x * 2.0 + 7.3, p.y * 2.0 - 3.1, time)) * 0.18 * dist;
            // swirl around the center
            let d = p - vec2(0.5, 0.5);
            let r = d.length();
            let ang = swirl * (1.0 - r * 1.4).max(0.0).powi(2) * 2.2 * (time * 0.7).sin();
            let (s, c) = ang.sin_cos();
            p = vec2(0.5 + d.x * c - d.y * s, 0.5 + d.x * s + d.y * c);
            let mut acc = Rgb::default();
            let mut sum = 0.0;
            for (q, col) in pts.iter().zip(cols.iter()) {
                let w = 1.0 / ((p - *q).length_sq() + 0.02).powf(1.6);
                acc = acc.add(col.scale(w));
                sum += w;
            }
            acc.scale(1.0 / sum).to_color()
        });
    }
}

background!(MeshGradient);
