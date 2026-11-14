//! Liquid Ether: a small real-time fluid simulation (stable-fluids style) rendered as glowing,
//! swirling color.

use crate::{background, ghost, pointer, smoothstep, Base, Lut, Rgb};
use egui::{vec2, Color32, Pos2, Rect, Ui, Vec2};
use glowkit_core::{noise::noise3, paint::shader_grid};

#[derive(Clone, Debug, Default)]
struct Fluid {
    nx: usize,
    ny: usize,
    u: Vec<f32>,
    v: Vec<f32>,
    /// Dye: palette position (weighted) and density, advected with the flow.
    hue: Vec<f32>,
    ink: Vec<f32>,
    p: Vec<f32>,
    tmp: Vec<f32>,
    tmp2: Vec<f32>,
    last: Vec<Pos2>,
    hue_clock: f32,
}

fn sample(f: &[f32], nx: usize, ny: usize, x: f32, y: f32) -> f32 {
    let x = x.clamp(0.0, (nx - 1) as f32);
    let y = y.clamp(0.0, (ny - 1) as f32);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(nx - 1), (y0 + 1).min(ny - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let a = f[y0 * nx + x0] + (f[y0 * nx + x1] - f[y0 * nx + x0]) * fx;
    let b = f[y1 * nx + x0] + (f[y1 * nx + x1] - f[y1 * nx + x0]) * fx;
    a + (b - a) * fy
}

impl Fluid {
    fn resize(&mut self, nx: usize, ny: usize) {
        if self.nx == nx && self.ny == ny {
            return;
        }
        let n = nx * ny;
        *self = Fluid {
            nx,
            ny,
            u: vec![0.0; n],
            v: vec![0.0; n],
            hue: vec![0.0; n],
            ink: vec![0.0; n],
            p: vec![0.0; n],
            tmp: vec![0.0; n],
            tmp2: vec![0.0; n],
            last: Vec::new(),
            hue_clock: 0.0,
        };
    }

    /// Push the fluid at `pos` (cells) with velocity `force` (cells/s), dyeing it with `hue`.
    fn splat(&mut self, pos: Vec2, force: Vec2, radius: f32, hue: f32) {
        let (nx, ny) = (self.nx as i32, self.ny as i32);
        let r = radius.max(1.0);
        let reach = (r * 2.5).ceil() as i32;
        let amount = (force.length() / 30.0).min(1.0);
        for y in (pos.y as i32 - reach).max(0)..=(pos.y as i32 + reach).min(ny - 1) {
            for x in (pos.x as i32 - reach).max(0)..=(pos.x as i32 + reach).min(nx - 1) {
                let d = vec2(x as f32, y as f32) - pos;
                let w = (-d.length_sq() / (r * r)).exp();
                if w < 0.01 {
                    continue;
                }
                let i = y as usize * self.nx + x as usize;
                self.u[i] += force.x * w * 0.5;
                self.v[i] += force.y * w * 0.5;
                let add = w * amount;
                let total = self.ink[i] + add;
                if total > 1e-4 {
                    // blend hue weighted by density (hue is a palette position)
                    self.hue[i] = (self.hue[i] * self.ink[i] + hue * add) / total;
                }
                self.ink[i] = total.min(1.6);
            }
        }
    }

    fn step(&mut self, dt: f32, swirl: f32, time: f32, ambient: f32) {
        let (nx, ny) = (self.nx, self.ny);
        let idx = |x: usize, y: usize| y * nx + x;
        // gentle ambient curl-noise stirring so the ether never fully settles
        if ambient > 0.0 {
            for y in 1..ny - 1 {
                for x in 1..nx - 1 {
                    let (fx, fy) = (x as f32 * 0.08, y as f32 * 0.08);
                    let e = 0.5;
                    let n1 = noise3(fx, fy + e, time * 0.1);
                    let n2 = noise3(fx, fy - e, time * 0.1);
                    let n3 = noise3(fx + e, fy, time * 0.1);
                    let n4 = noise3(fx - e, fy, time * 0.1);
                    let i = idx(x, y);
                    self.u[i] += (n1 - n2) * ambient * dt;
                    self.v[i] -= (n3 - n4) * ambient * dt;
                }
            }
        }
        // vorticity confinement keeps the swirls tight and lively
        if swirl > 0.0 {
            let curl = &mut self.tmp;
            for y in 1..ny - 1 {
                for x in 1..nx - 1 {
                    curl[idx(x, y)] = 0.5
                        * ((self.v[idx(x + 1, y)] - self.v[idx(x - 1, y)])
                            - (self.u[idx(x, y + 1)] - self.u[idx(x, y - 1)]));
                }
            }
            for y in 2..ny - 2 {
                for x in 2..nx - 2 {
                    let gx = curl[idx(x + 1, y)].abs() - curl[idx(x - 1, y)].abs();
                    let gy = curl[idx(x, y + 1)].abs() - curl[idx(x, y - 1)].abs();
                    let len = (gx * gx + gy * gy).sqrt() + 1e-5;
                    let w = curl[idx(x, y)];
                    self.u[idx(x, y)] += swirl * (gy / len) * w * dt;
                    self.v[idx(x, y)] -= swirl * (gx / len) * w * dt;
                }
            }
        }
        // semi-Lagrangian advection of velocity and dye
        let (u, v) = (self.u.clone(), self.v.clone());
        let (hue, ink) = (self.hue.clone(), self.ink.clone());
        let fade_v = (-dt * 0.55).exp();
        let fade_ink = (-dt * 0.45).exp();
        for y in 0..ny {
            for x in 0..nx {
                let i = idx(x, y);
                let bx = x as f32 - u[i] * dt;
                let by = y as f32 - v[i] * dt;
                self.u[i] = sample(&u, nx, ny, bx, by) * fade_v;
                self.v[i] = sample(&v, nx, ny, bx, by) * fade_v;
                self.hue[i] = sample(&hue, nx, ny, bx, by);
                self.ink[i] = sample(&ink, nx, ny, bx, by) * fade_ink;
            }
        }
        // walls
        for x in 0..nx {
            self.v[idx(x, 0)] = 0.0;
            self.v[idx(x, ny - 1)] = 0.0;
        }
        for y in 0..ny {
            self.u[idx(0, y)] = 0.0;
            self.u[idx(nx - 1, y)] = 0.0;
        }
        // pressure projection (Jacobi, warm-started) so the flow stays incompressible
        let div = &mut self.tmp;
        for y in 1..ny - 1 {
            for x in 1..nx - 1 {
                div[idx(x, y)] = 0.5
                    * (self.u[idx(x + 1, y)] - self.u[idx(x - 1, y)] + self.v[idx(x, y + 1)] - self.v[idx(x, y - 1)]);
            }
        }
        for _ in 0..18 {
            let p = &self.p;
            let out = &mut self.tmp2;
            for y in 1..ny - 1 {
                for x in 1..nx - 1 {
                    out[idx(x, y)] = (p[idx(x - 1, y)] + p[idx(x + 1, y)] + p[idx(x, y - 1)] + p[idx(x, y + 1)]
                        - div[idx(x, y)])
                        * 0.25;
                }
            }
            std::mem::swap(&mut self.p, &mut self.tmp2);
        }
        for y in 1..ny - 1 {
            for x in 1..nx - 1 {
                let i = idx(x, y);
                self.u[i] -= 0.5 * (self.p[idx(x + 1, y)] - self.p[idx(x - 1, y)]);
                self.v[i] -= 0.5 * (self.p[idx(x, y + 1)] - self.p[idx(x, y - 1)]);
            }
        }
    }
}

/// Swirling, fluid-like ribbons of color that react to the pointer: moving it stirs a small
/// real-time fluid simulation, leaving glowing eddies that curl, mix and slowly fade. When
/// nobody is pointing, a ghost pointer keeps stirring.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::LiquidEther;
/// ui.add(LiquidEther::new());
/// ui.add(LiquidEther::new().colors(&[glowkit_core::hex("#5227FF"), glowkit_core::hex("#FF9FFC"), glowkit_core::hex("#B19EEF")]));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct LiquidEther {
    base: Base,
    colors: Vec<Color32>,
    force: f32,
    radius: f32,
    swirl: f32,
    resolution: usize,
    autoplay: bool,
}

impl Default for LiquidEther {
    fn default() -> Self {
        Self::new()
    }
}

impl LiquidEther {
    /// Liquid ether in the theme gradient colors.
    pub fn new() -> Self {
        Self {
            base: Base::default(),
            colors: Vec::new(),
            force: 1.0,
            radius: 1.0,
            swirl: 1.0,
            resolution: 96,
            autoplay: true,
        }
    }
    /// Dye colors, cycled as you stir (default: the theme gradient).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    /// How hard the pointer pushes the fluid (default 1).
    pub fn force(mut self, k: f32) -> Self {
        self.force = k.max(0.0);
        self
    }
    /// Size of the pointer's splat, relative (default 1).
    pub fn cursor_size(mut self, k: f32) -> Self {
        self.radius = k.clamp(0.2, 4.0);
        self
    }
    /// Strength of the curl that keeps eddies tight (default 1).
    pub fn swirl(mut self, k: f32) -> Self {
        self.swirl = k.max(0.0);
        self
    }
    /// Simulation columns (default 96; rows follow the aspect ratio). Higher is finer and costlier.
    pub fn resolution(mut self, cols: usize) -> Self {
        self.resolution = cols.clamp(24, 140);
        self
    }
    /// Stir with a ghost pointer when the real one is away (default true).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Paint the ether into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_liquid_ether") else { return };
        let t = &f.theme;
        let stops: Vec<Color32> = if self.colors.len() >= 2 {
            self.colors.clone()
        } else {
            vec![t.gradient[0], t.gradient[1], glowkit_core::mix(t.gradient[0], Color32::WHITE, 0.45)]
        };
        let lut = Lut::new(&stops, true);
        let nx = self.resolution;
        let ny = ((nx as f32 * rect.height() / rect.width().max(1.0)).round() as usize).clamp(20, 120);
        let cell = vec2(rect.width() / (nx - 1) as f32, rect.height() / (ny - 1) as f32);
        let real = pointer(ui, rect);
        // with nobody pointing, two ghost stirrers wander in different rhythms
        let stirrers: Vec<Pos2> = match real {
            Some(p) => vec![p],
            None if self.autoplay => vec![ghost(rect, f.time * 1.7), ghost(rect, f.time * 1.3 + 40.0)],
            None => Vec::new(),
        };
        let dt = f.dt.max(1.0 / 240.0);
        let (force, radius, swirl, time) = (self.force, self.radius, self.swirl, f.time);
        let field = glowkit_core::with_state(ui, f.id, Fluid::default, |s| {
            s.resize(nx, ny);
            s.hue_clock += dt * 0.22;
            let lasts = std::mem::take(&mut s.last);
            for (k, p) in stirrers.iter().enumerate() {
                let pos = (*p - rect.min) / cell;
                if let Some(last) = lasts.get(k) {
                    let delta = (*p - *last) / cell;
                    if delta.length() > 0.01 && delta.length() < 30.0 {
                        let vel = delta / dt * force;
                        let hue = (s.hue_clock + k as f32 * 0.5).rem_euclid(1.0);
                        s.splat(pos, vel * 0.9, 3.4 * radius, hue);
                    }
                }
            }
            s.last = stirrers.clone();
            s.step(dt, 22.0 * swirl, time, 18.0);
            // output per node: (palette position, brightness), lightly blurred for display
            let mut raw = Vec::with_capacity(nx * ny);
            for i in 0..nx * ny {
                let speed = (s.u[i] * s.u[i] + s.v[i] * s.v[i]).sqrt();
                let b = (s.ink[i] * 0.85 + (speed / 30.0).min(1.0) * 0.35).min(1.2);
                raw.push((s.hue[i] + (speed / 60.0).min(0.3), b));
            }
            // two soft blur passes (brightness, and hue weighted by brightness) hide the grid
            let mut out = raw;
            for _ in 0..2 {
                let src = out.clone();
                for y in 1..ny - 1 {
                    for x in 1..nx - 1 {
                        let (mut b, mut hw, mut wsum) = (0.0, 0.0, 0.0);
                        for (dx, dy, w) in [
                            (0i32, 0i32, 4.0f32),
                            (1, 0, 2.0),
                            (-1, 0, 2.0),
                            (0, 1, 2.0),
                            (0, -1, 2.0),
                            (1, 1, 1.0),
                            (-1, 1, 1.0),
                            (1, -1, 1.0),
                            (-1, -1, 1.0),
                        ] {
                            let (h, v) = src[(y as i32 + dy) as usize * nx + (x as i32 + dx) as usize];
                            b += v * w;
                            let ww = w * (v + 0.02);
                            hw += h * ww;
                            wsum += ww;
                        }
                        out[y * nx + x] = (hw / wsum, b / 16.0);
                    }
                }
            }
            out
        });
        let bg = Rgb::of(f.bg);
        let dark = f.dark();
        shader_grid(&f.painter, rect, nx - 1, ny - 1, |uv| {
            let x = ((uv.x * (nx - 1) as f32).round() as usize).min(nx - 1);
            let y = ((uv.y * (ny - 1) as f32).round() as usize).min(ny - 1);
            let (hue, b) = field[y * nx + x];
            let a = smoothstep(0.0, 0.9, b);
            let c = lut.sample(hue);
            if dark {
                // glowing: add light, with a hot white core where dense
                let hot = smoothstep(0.75, 1.2, b) * 0.35;
                bg.add(c.scale(a)).lerp(Rgb(1.0, 1.0, 1.0), hot).to_color()
            } else {
                bg.lerp(c, a * 0.9).to_color()
            }
        });
    }
}

background!(LiquidEther);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fluid_stays_finite_and_moves() {
        let mut f = Fluid::default();
        f.resize(40, 24);
        for k in 0..120 {
            f.splat(vec2(10.0 + k as f32 * 0.1, 12.0), vec2(20.0, 5.0), 3.0, 0.3);
            f.step(1.0 / 60.0, 14.0, k as f32 / 60.0, 6.0);
        }
        assert!(f.u.iter().chain(&f.v).chain(&f.ink).all(|v| v.is_finite()));
        assert!(f.ink.iter().any(|v| *v > 0.05));
    }
}
