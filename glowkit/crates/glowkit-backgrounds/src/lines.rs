//! Line-based backgrounds: sweeping beams, wavy ribbons, noisy line fields, threads and light
//! columns.

use crate::{background, ghost, h01, light, pointer, radial_mask, smoothstep, soft_ribbon, Base, Lut, Rgb};
use egui::epaint::{PathShape, PathStroke};
use egui::{pos2, vec2, Color32, Pos2, Rect, Ui, Vec2};
use glowkit_core::noise::{fbm3, noise3};
use glowkit_core::paint::{cubic_bezier, gradient_polyline, shader_grid};
use std::f32::consts::PI;

// ---------------------------------------------------------------------------------------------
// Background beams
// ---------------------------------------------------------------------------------------------

/// A sheaf of faint, gently curving S-shaped paths sweeping diagonally across the area, with
/// pulses of colored light racing along them.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::BackgroundBeams;
/// ui.add(BackgroundBeams::new().count(40));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct BackgroundBeams {
    base: Base,
    count: usize,
    colors: Option<Vec<Color32>>,
    line: Option<Color32>,
    speed: f32,
}

impl Default for BackgroundBeams {
    fn default() -> Self {
        Self::new()
    }
}

impl BackgroundBeams {
    /// 36 beams in the theme's colors.
    pub fn new() -> Self {
        Self { base: Base::default(), count: 36, colors: None, line: None, speed: 1.0 }
    }
    /// Number of paths (default 36, capped at 120).
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.min(120);
        self
    }
    /// Colors of the light pulses, from tail to head (default: cyan, accent, pink).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }
    /// Color of the faint paths (default: theme foreground at low opacity).
    pub fn line_color(mut self, c: Color32) -> Self {
        self.line = Some(c);
        self
    }
    /// Pulse speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// Paint the beams into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_background_beams") else { return };
        let t = &f.theme;
        let dark = f.dark();
        let colors = self.colors.clone().unwrap_or_else(|| vec![t.chart[1], t.accent, t.gradient[1]]);
        let lut = Lut::new(&colors, false);
        let line = self.line.unwrap_or_else(|| t.foreground.gamma_multiply(if dark { 0.07 } else { 0.09 }));
        let (w, h) = (rect.width(), rect.height());
        let at = |u: f32, v: f32| pos2(rect.min.x + u * w, rect.min.y + v * h);
        let p = &f.painter;
        let n = self.count.max(1);
        for i in 0..n {
            let s = i as f32 / n as f32;
            let o = vec2(s * 0.9, -s * 0.55);
            let pts = cubic_bezier(
                at(-0.45 + o.x, 0.15 + o.y),
                at(-0.2 + o.x, 1.05 + o.y),
                at(0.55 + o.x, 0.1 + o.y),
                at(0.85 + o.x, 1.25 + o.y),
                48,
            );
            p.add(PathShape::line(pts.clone(), PathStroke::new(0.8, line)));
            // every other path carries a pulse of light
            if i % 2 == 0 {
                let id = i as u32;
                let dur = (5.0 + 6.0 * h01(id, 1)) / self.speed.max(0.01);
                let head = ((f.time + h01(id, 2) * dur) / dur).fract() * 1.5 - 0.1;
                let len = 0.18 + 0.12 * h01(id, 3);
                let pulse = |u: f32| {
                    let d = head - u;
                    if d > 0.0 && d < len {
                        1.0 - d / len
                    } else {
                        0.0
                    }
                };
                gradient_polyline(p, &pts, false, 6.0, |u| {
                    let k = pulse(u);
                    light(lut.sample(k).to_color(), k * k * 0.18, dark)
                });
                gradient_polyline(p, &pts, false, 1.6, |u| {
                    let d = head - u;
                    if d > 0.0 && d < len {
                        let k = 1.0 - d / len;
                        light(lut.sample(k).to_color(), k * k, dark)
                    } else {
                        Color32::TRANSPARENT
                    }
                });
            }
        }
        radial_mask(p, rect, f.bg, 0.45, 1.15);
    }
}

background!(BackgroundBeams);

// ---------------------------------------------------------------------------------------------
// Wavy background
// ---------------------------------------------------------------------------------------------

/// Thick, blurred ribbons of color undulating across the middle of the area like slow ocean
/// waves.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::WavyBackground;
/// ui.add(WavyBackground::new().wave_width(50.0).blur(10.0));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct WavyBackground {
    base: Base,
    colors: Option<Vec<Color32>>,
    width: f32,
    blur: f32,
    speed: f32,
    opacity: f32,
    amplitude: f32,
}

impl Default for WavyBackground {
    fn default() -> Self {
        Self::new()
    }
}

impl WavyBackground {
    /// Five waves in the theme's colors.
    pub fn new() -> Self {
        Self { base: Base::default(), colors: None, width: 50.0, blur: 10.0, speed: 1.0, opacity: 0.5, amplitude: 1.0 }
    }
    /// One color per wave (default: five theme colors).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }
    /// Wave thickness in points (default 50).
    pub fn wave_width(mut self, w: f32) -> Self {
        self.width = w.max(1.0);
        self
    }
    /// Edge softness in points (default 10).
    pub fn blur(mut self, b: f32) -> Self {
        self.blur = b.max(0.0);
        self
    }
    /// Speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Wave opacity (default 0.5).
    pub fn opacity(mut self, a: f32) -> Self {
        self.opacity = a.clamp(0.0, 1.0);
        self
    }
    /// Wave height multiplier (default 1).
    pub fn amplitude(mut self, a: f32) -> Self {
        self.amplitude = a;
        self
    }

    /// Paint the waves into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_wavy_background") else { return };
        let t = &f.theme;
        let colors =
            self.colors.clone().unwrap_or_else(|| vec![t.chart[1], t.chart[5], t.accent, t.gradient[1], t.chart[1]]);
        let time = f.time * self.speed * 0.25;
        let cy = rect.center().y;
        let amp = rect.height() * 0.28 * self.amplitude;
        let n = ((rect.width() / 10.0) as usize).max(2);
        for (i, c) in colors.iter().enumerate() {
            let pts: Vec<Pos2> = (0..=n)
                .map(|k| {
                    let x = rect.min.x - 20.0 + (rect.width() + 40.0) * k as f32 / n as f32;
                    let y = cy + noise3(x / 700.0, 0.3 * i as f32, time) * amp * 1.4;
                    pos2(x, y)
                })
                .collect();
            let col = c.gamma_multiply(self.opacity);
            soft_ribbon(&f.painter, &pts, self.width - self.blur, self.blur * 2.0, |_| col);
        }
    }
}

background!(WavyBackground);

// ---------------------------------------------------------------------------------------------
// Waves
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct WaveField {
    off: Vec<Vec2>,
    vel: Vec<Vec2>,
    dims: (usize, usize),
    last: Option<Pos2>,
    pv: Vec2,
}

/// Dozens of thin horizontal lines warped by flowing noise, like a topographic field in the
/// wind. Moving the pointer through them drags the lines along and they spring back.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::Waves;
/// ui.add(Waves::new().gap(14.0).amplitude(1.2));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct Waves {
    base: Base,
    gap: f32,
    color: Option<Color32>,
    speed: f32,
    amplitude: f32,
    auto_hover: bool,
}

impl Default for Waves {
    fn default() -> Self {
        Self::new()
    }
}

impl Waves {
    /// Lines every 14 points.
    pub fn new() -> Self {
        Self { base: Base::default(), gap: 14.0, color: None, speed: 1.0, amplitude: 1.0, auto_hover: false }
    }
    /// Distance between lines in points (default 14).
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap.max(4.0);
        self
    }
    /// Line color (default: theme foreground at low opacity).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Noise displacement multiplier (default 1).
    pub fn amplitude(mut self, a: f32) -> Self {
        self.amplitude = a;
        self
    }
    /// When nobody hovers, a phantom pointer drifts through the lines (default false).
    pub fn auto_hover(mut self, on: bool) -> Self {
        self.auto_hover = on;
        self
    }

    /// Paint the lines into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_waves") else { return };
        let t = &f.theme;
        let color = self.color.unwrap_or_else(|| t.foreground.gamma_multiply(if t.dark { 0.4 } else { 0.45 }));
        let step_x = 10.0;
        let rows = (rect.height() / self.gap) as usize + 3;
        let cols = (rect.width() / step_x) as usize + 3;
        let hp = pointer(ui, rect).or_else(|| self.auto_hover.then(|| ghost(rect, f.time * 1.3)));
        let dt = f.dt;
        let time = f.time * self.speed;
        let amp = 22.0 * self.amplitude;
        let at = |i: usize, j: usize| {
            pos2(rect.min.x - step_x + i as f32 * step_x, rect.min.y - self.gap + j as f32 * self.gap)
        };
        let offs = glowkit_core::with_state(ui, f.id, WaveField::default, |s| {
            if s.dims != (cols, rows) {
                s.off = vec![Vec2::ZERO; cols * rows];
                s.vel = vec![Vec2::ZERO; cols * rows];
                s.dims = (cols, rows);
            }
            let raw = match (hp, s.last) {
                (Some(p), Some(l)) if dt > 0.0 => (p - l) / dt,
                _ => Vec2::ZERO,
            };
            s.pv = s.pv + (raw - s.pv) * 0.3;
            s.last = hp;
            for j in 0..rows {
                for i in 0..cols {
                    let k = j * cols + i;
                    if let Some(p) = hp {
                        let d = at(i, j).distance(p + s.off[k] * 0.0);
                        let r = 150.0;
                        if d < r {
                            let fall = 1.0 - d / r;
                            s.vel[k] += s.pv.clamp(Vec2::splat(-2500.0), Vec2::splat(2500.0)) * fall * fall * dt * 0.9;
                        }
                    }
                    let acc = -s.off[k] * 35.0 - s.vel[k] * 5.0;
                    s.vel[k] += acc * dt;
                    s.off[k] += s.vel[k] * dt;
                    s.off[k] = s.off[k].clamp(Vec2::splat(-70.0), Vec2::splat(70.0));
                }
            }
            s.off.clone()
        });
        for j in 0..rows {
            let pts: Vec<Pos2> = (0..cols)
                .map(|i| {
                    let p0 = at(i, j);
                    let n = noise3((p0.x - rect.min.x) * 0.004 + time * 0.12, (p0.y - rect.min.y) * 0.006, time * 0.08);
                    let m = noise3(
                        (p0.x - rect.min.x) * 0.002 - time * 0.05,
                        (p0.y - rect.min.y) * 0.003 + 5.0,
                        time * 0.05,
                    );
                    p0 + vec2(m * amp * 0.6, n * amp * 1.6) + offs[j * cols + i]
                })
                .collect();
            f.painter.add(PathShape::line(pts, PathStroke::new(1.0, color)));
        }
    }
}

background!(Waves);

// ---------------------------------------------------------------------------------------------
// Threads
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct Pull {
    pos: Option<Pos2>,
    strength: f32,
}

/// Fine threads that start as a tight bundle on the left and fan out into flowing, rippling
/// strands to the right. The strands lean toward the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::Threads;
/// ui.add(Threads::new().count(40).amplitude(1.2));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct Threads {
    base: Base,
    count: usize,
    color: Option<Color32>,
    speed: f32,
    amplitude: f32,
    distance: f32,
    interactive: bool,
}

impl Default for Threads {
    fn default() -> Self {
        Self::new()
    }
}

impl Threads {
    /// 40 threads.
    pub fn new() -> Self {
        Self {
            base: Base::default(),
            count: 40,
            color: None,
            speed: 1.0,
            amplitude: 1.0,
            distance: 1.0,
            interactive: true,
        }
    }
    /// Number of threads (default 40, capped at 150).
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.clamp(1, 150);
        self
    }
    /// Thread color (default: theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Ripple height multiplier (default 1).
    pub fn amplitude(mut self, a: f32) -> Self {
        self.amplitude = a;
        self
    }
    /// How far the threads fan apart (default 1).
    pub fn distance(mut self, d: f32) -> Self {
        self.distance = d.max(0.0);
        self
    }
    /// Whether the threads lean toward the pointer (default true).
    pub fn interactive(mut self, on: bool) -> Self {
        self.interactive = on;
        self
    }

    /// Paint the threads into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_threads") else { return };
        let t = &f.theme;
        let color = self.color.unwrap_or(t.foreground);
        let time = f.time * self.speed;
        let hp = if self.interactive { pointer(ui, rect) } else { None };
        let pull = glowkit_core::with_state(ui, f.id, Pull::default, |s| {
            let target = if hp.is_some() { 1.0 } else { 0.0 };
            s.strength += (target - s.strength) * (1.0 - (-f.dt * 3.0).exp());
            if let Some(p) = hp {
                s.pos = Some(s.pos.map_or(p, |q| q + (p - q) * (1.0 - (-f.dt * 5.0).exp())));
            }
            *s
        });
        let (w, h) = (rect.width(), rect.height());
        let cy = rect.center().y;
        let n = self.count;
        let samples = 110;
        for i in 0..n {
            let s = if n > 1 { i as f32 / (n - 1) as f32 - 0.5 } else { 0.0 };
            let pts: Vec<Pos2> = (0..=samples)
                .map(|k| {
                    let u = k as f32 / samples as f32;
                    let x = rect.min.x + u * w;
                    let spread = s * h * (0.04 + 0.7 * u.powf(1.2)) * self.distance;
                    let wave = fbm3(u * 1.8 - time * 0.2, s * 0.9 + time * 0.03, time * 0.08, 3)
                        * h
                        * 0.9
                        * u.powf(1.1)
                        * self.amplitude;
                    let ripple = (u * 7.0 + s * 5.0 - time * 1.1).sin() * h * 0.025 * u * self.amplitude;
                    let mut y = cy + spread + wave + ripple;
                    if let Some(m) = pull.pos {
                        let dx = (x - m.x) / (w * 0.18);
                        y += (m.y - y) * 0.35 * pull.strength * (-dx * dx).exp();
                    }
                    pos2(x, y)
                })
                .collect();
            let a = 0.18 + 0.4 * (1.0 - (s.abs() * 2.0)).powf(0.7);
            gradient_polyline(&f.painter, &pts, false, 1.0, |u| {
                color.gamma_multiply(a * (0.35 + 0.65 * smoothstep(0.0, 0.5, u)) * (1.0 - smoothstep(0.9, 1.0, u)))
            });
        }
    }
}

background!(Threads);

// ---------------------------------------------------------------------------------------------
// Beams (vertical light columns)
// ---------------------------------------------------------------------------------------------

/// Tall columns of soft light standing side by side, with noisy luminous texture flowing down
/// them. Rotate them for a dramatic diagonal look.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::Beams;
/// ui.add(Beams::new().count(12).rotation(30.0));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct Beams {
    base: Base,
    count: usize,
    width: f32,
    color: Option<Color32>,
    speed: f32,
    rotation: f32,
    noise: f32,
}

impl Default for Beams {
    fn default() -> Self {
        Self::new()
    }
}

impl Beams {
    /// 12 upright beams.
    pub fn new() -> Self {
        Self { base: Base::default(), count: 12, width: 0.35, color: None, speed: 1.0, rotation: 0.0, noise: 1.0 }
    }
    /// Number of beams across the rect (default 12).
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.clamp(1, 40);
        self
    }
    /// Beam width as a fraction of the spacing (default 0.35).
    pub fn beam_width(mut self, w: f32) -> Self {
        self.width = w.clamp(0.05, 1.0);
        self
    }
    /// Light color (default: theme foreground on dark themes, accent on light).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Flow speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Rotation in degrees (default 0 = upright).
    pub fn rotation(mut self, degrees: f32) -> Self {
        self.rotation = degrees;
        self
    }
    /// Strength of the flowing texture, `0..=1` (default 1).
    pub fn noise(mut self, n: f32) -> Self {
        self.noise = n.clamp(0.0, 1.0);
        self
    }

    /// Paint the beams into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_beams") else { return };
        let t = &f.theme;
        let dark = t.dark;
        let col = Rgb::of(self.color.unwrap_or(if dark { t.foreground } else { t.accent }));
        let bg = Rgb::of(f.bg);
        let aspect = rect.width() / rect.height().max(1.0);
        let (s, c) = self.rotation.to_radians().sin_cos();
        let n = self.count as f32;
        let time = f.time * self.speed;
        let (bw, noise) = (self.width, self.noise);
        shader_grid(&f.painter, rect, (self.count * 12).clamp(48, 150), 56, |uv| {
            let p = vec2((uv.x - 0.5) * aspect, uv.y - 0.5);
            let q = vec2(p.x * c + p.y * s, -p.x * s + p.y * c);
            let x = (q.x / aspect + 0.5) * n;
            let idx = x.floor();
            let fr = x - idx - 0.5;
            let core = (-(fr / (bw * 0.42)).powi(2)).exp();
            let halo = (-(fr / (bw * 0.9)).powi(2)).exp() * 0.35;
            let flow = fbm3(idx * 1.7, q.y * 1.6 - time * 0.5, time * 0.1, 2);
            let tex = 1.0 - noise * 0.6 + noise * 0.6 * smoothstep(-0.45, 0.5, flow);
            let fade = (PI * uv.y).sin().max(0.0).powf(0.5) * (0.7 + 0.3 * h01(idx as i32 as u32, 1));
            let i = (core * tex + halo * (0.5 + 0.5 * tex)) * fade;
            if dark {
                bg.add(col.scale(i * 0.85))
                    .lerp(Rgb(1.0, 1.0, 1.0), (core * tex * fade - 0.7).max(0.0) * 0.5)
                    .to_color()
            } else {
                bg.lerp(col, (i * 0.7).min(1.0)).to_color()
            }
        });
    }
}

background!(Beams);
