//! Particle backgrounds: floating particles, vortex, snow, galaxy, warp speed.

use crate::{background, h01, light, pointer, Base, Lut};
use egui::{vec2, Color32, Pos2, Rect, Stroke, Ui, Vec2};
use glowkit_core::noise::noise3;
use glowkit_core::paint::{glow, glow_ellipse};
use glowkit_core::Rng;
use std::f32::consts::TAU;

// ---------------------------------------------------------------------------------------------
// Particles
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Dot {
    pos: Vec2,
    push: Vec2,
    dir: Vec2,
    z: f32,
    seed: f32,
    color: u8,
}

#[derive(Clone, Default)]
struct DotsState {
    dots: Vec<Dot>,
    size: Vec2,
}

/// Softly drifting particles with depth: near ones are bigger, brighter and faster. Optionally
/// connected by faint lines when close (a "constellation" network), twinkling like sparkles, and
/// pushed away by the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::Particles;
/// ui.add(Particles::new().count(140).links(true));
/// ui.add(Particles::new().count(260).twinkle(true).particle_size(1.2));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct Particles {
    base: Base,
    count: usize,
    colors: Option<Vec<Color32>>,
    speed: f32,
    size: f32,
    links: bool,
    link_distance: f32,
    twinkle: bool,
    repel: f32,
}

impl Default for Particles {
    fn default() -> Self {
        Self::new()
    }
}

impl Particles {
    /// 120 drifting particles.
    pub fn new() -> Self {
        Self {
            base: Base::default(),
            count: 120,
            colors: None,
            speed: 1.0,
            size: 1.0,
            links: false,
            link_distance: 110.0,
            twinkle: false,
            repel: 1.0,
        }
    }
    /// Number of particles (default 120, capped at 600).
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.min(600);
        self
    }
    /// Particle colors, picked at random (default: theme foreground with a few accent ones).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }
    /// Drift speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Particle size multiplier (default 1).
    pub fn particle_size(mut self, size: f32) -> Self {
        self.size = size.max(0.1);
        self
    }
    /// Connect nearby particles with faint lines (default off).
    pub fn links(mut self, on: bool) -> Self {
        self.links = on;
        self
    }
    /// Maximum distance for a link, in points (default 110).
    pub fn link_distance(mut self, d: f32) -> Self {
        self.link_distance = d.max(10.0);
        self
    }
    /// Make particles twinkle like sparkles (default off).
    pub fn twinkle(mut self, on: bool) -> Self {
        self.twinkle = on;
        self
    }
    /// Pointer repulsion strength, 0 to disable (default 1).
    pub fn repel(mut self, strength: f32) -> Self {
        self.repel = strength;
        self
    }

    /// Paint the particles into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_particles") else { return };
        let t = &f.theme;
        let colors = self.colors.clone().unwrap_or_else(|| vec![t.foreground, t.foreground, t.foreground, t.accent, t.chart[1]]);
        let size = rect.size();
        let mouse = pointer(ui, rect).map(|p| p - rect.min);
        let (dt, time) = (f.dt, f.time);
        let count = self.count;
        let ncol = colors.len().max(1);
        let dots = glowkit_core::with_state(ui, f.id, DotsState::default, |s| {
            if s.dots.len() != count || (s.size - size).length() > 1.0 {
                let mut rng = Rng::new(0x5EED + count as u64);
                s.dots = (0..count)
                    .map(|_| {
                        let a = rng.range(0.0, TAU);
                        Dot {
                            pos: vec2(rng.range(0.0, size.x), rng.range(0.0, size.y)),
                            push: Vec2::ZERO,
                            dir: vec2(a.cos(), a.sin()),
                            z: rng.f32().powf(1.6),
                            seed: rng.f32(),
                            color: rng.below(ncol) as u8,
                        }
                    })
                    .collect();
                s.size = size;
            }
            for d in &mut s.dots {
                let wob = noise3(d.seed * 37.0, time * 0.15, d.z) * 1.6;
                let dir = egui::Vec2::angled(d.dir.angle() + wob);
                d.pos += dir * (6.0 + 22.0 * d.z) * self.speed * dt;
                if let Some(m) = mouse {
                    let off = d.pos + d.push - m;
                    let dist = off.length();
                    let r = 120.0;
                    if dist < r && dist > 0.01 {
                        let k = (1.0 - dist / r).powi(2) * 900.0 * self.repel * (0.4 + d.z);
                        d.push += off / dist * k * dt;
                    }
                }
                d.push *= (-dt * 2.5).exp();
                d.pos.x = (d.pos.x + 10.0).rem_euclid(size.x + 20.0) - 10.0;
                d.pos.y = (d.pos.y + 10.0).rem_euclid(size.y + 20.0) - 10.0;
            }
            s.dots.clone()
        });
        let p = &f.painter;
        let dark = f.dark();
        let pts: Vec<Pos2> = dots.iter().map(|d| rect.min + d.pos + d.push).collect();
        if self.links {
            let maxd = self.link_distance;
            let lc = t.muted_foreground;
            for i in 0..dots.len() {
                for j in i + 1..dots.len() {
                    let d = pts[i].distance(pts[j]);
                    if d < maxd {
                        let a = (1.0 - d / maxd) * 0.45 * (0.35 + 0.65 * dots[i].z.min(dots[j].z));
                        p.line_segment([pts[i], pts[j]], Stroke::new(0.8, lc.gamma_multiply(a)));
                    }
                }
                if let Some(m) = mouse {
                    let m = rect.min + m;
                    let d = pts[i].distance(m);
                    if d < maxd * 1.4 {
                        p.line_segment([pts[i], m], Stroke::new(0.8, t.accent.gamma_multiply((1.0 - d / (maxd * 1.4)) * 0.5)));
                    }
                }
            }
        }
        for (d, &pos) in dots.iter().zip(&pts) {
            let mut a = 0.25 + 0.75 * d.z;
            if self.twinkle {
                a *= (0.5 + 0.5 * (time * (1.5 + d.seed * 3.0) + d.seed * 40.0).sin()).powf(1.5);
            }
            let r = self.size * (0.6 + 1.5 * d.z);
            let c = colors[d.color as usize % ncol];
            if d.z > 0.55 {
                p.circle_filled(pos, r * 3.2, light(c, a * 0.12, dark));
            }
            if self.twinkle && d.z > 0.75 {
                glowkit_core::paint::sparkle(p, pos, r * 3.0 * a, time * 0.3 + d.seed * 6.0, c.gamma_multiply(a));
            } else {
                p.circle_filled(pos, r, c.gamma_multiply(a));
            }
        }
    }
}

background!(Particles);

// ---------------------------------------------------------------------------------------------
// Vortex
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Swirler {
    angle: f32,
    radius: f32,
    life: f32,
    ttl: f32,
    speed: f32,
    hue: f32,
    width: f32,
}

#[derive(Clone, Default)]
struct VortexState {
    p: Vec<Swirler>,
    rng: Option<Rng>,
}

/// Hundreds of glowing particles swirling into a vortex, leaving short colorful trails.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::Vortex;
/// ui.add(Vortex::new().count(500).speed(1.2));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct Vortex {
    base: Base,
    count: usize,
    colors: Option<Vec<Color32>>,
    speed: f32,
    tilt: f32,
}

impl Default for Vortex {
    fn default() -> Self {
        Self::new()
    }
}

impl Vortex {
    /// A vortex of 450 particles in the theme's colors.
    pub fn new() -> Self {
        Self { base: Base::default(), count: 450, colors: None, speed: 1.0, tilt: 0.42 }
    }
    /// Number of particles (default 450, capped at 1000).
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.min(1000);
        self
    }
    /// Particle colors, blended by distance from the center (default: theme gradient).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }
    /// Rotation speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Vertical squash of the vortex, 1 = circular (default 0.42).
    pub fn tilt(mut self, tilt: f32) -> Self {
        self.tilt = tilt.clamp(0.1, 1.0);
        self
    }

    fn spawn(rng: &mut Rng, fresh: bool) -> Swirler {
        Swirler {
            angle: rng.range(0.0, TAU),
            radius: if fresh { rng.range(0.08, 1.0) } else { rng.range(0.75, 1.05) },
            life: if fresh { rng.range(0.0, 6.0) } else { 0.0 },
            ttl: rng.range(4.0, 9.0),
            speed: rng.range(0.6, 1.4),
            hue: rng.f32(),
            width: rng.range(0.8, 2.4),
        }
    }

    /// Paint the vortex into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_vortex") else { return };
        let t = &f.theme;
        let colors = self.colors.clone().unwrap_or_else(|| vec![t.chart[1], t.gradient[0], t.gradient[1]]);
        let lut = Lut::new(&colors, false);
        let dt = f.dt * self.speed;
        let count = self.count;
        let ps = glowkit_core::with_state(ui, f.id, VortexState::default, |s| {
            let rng = s.rng.get_or_insert_with(|| Rng::new(77));
            if s.p.len() != count {
                s.p = (0..count).map(|_| Self::spawn(rng, true)).collect();
            }
            for p in &mut s.p {
                let w = p.speed * (0.35 + 0.25 / (p.radius + 0.15));
                p.angle += w * dt;
                p.radius -= dt * 0.035 * p.speed * (1.0 + 0.6 * (p.angle * 3.0).sin());
                p.life += dt;
                if p.life > p.ttl || p.radius < 0.04 {
                    *p = Self::spawn(rng, false);
                }
            }
            s.p.clone()
        });
        let c = rect.center();
        let rx = rect.width() * 0.5;
        let ry = rx * self.tilt;
        let painter = &f.painter;
        let dark = f.dark();
        glow_ellipse(painter, c, vec2(rx * 0.35, ry * 0.35), light(lut.sample(0.0).to_color(), 0.35, dark));
        for p in &ps {
            let fade = (p.life / 0.8).min(1.0) * ((p.ttl - p.life) / 1.0).clamp(0.0, 1.0);
            let w = p.speed * (0.35 + 0.25 / (p.radius + 0.15));
            let pos = |a: f32, r: f32| c + vec2(a.cos() * rx * r, a.sin() * ry * r);
            let head = pos(p.angle, p.radius);
            let tail = pos(p.angle - w * 0.16, p.radius + 0.006);
            let mid = pos(p.angle - w * 0.08, p.radius + 0.003);
            let col = lut.sample(p.radius * 0.8 + p.hue * 0.35 - 0.1).to_color();
            let depth = 0.55 + 0.45 * p.angle.sin(); // front of the ring is brighter
            let a = fade * (0.35 + 0.65 * depth);
            painter.line_segment([tail, mid], Stroke::new(p.width, light(col, a * 0.35, dark)));
            painter.line_segment([mid, head], Stroke::new(p.width, light(col, a * 0.9, dark)));
            if p.width > 2.0 {
                painter.circle_filled(head, p.width * 2.5, light(col, a * 0.15, dark));
            }
        }
    }
}

background!(Vortex);

// ---------------------------------------------------------------------------------------------
// Snowfall
// ---------------------------------------------------------------------------------------------

/// Gently falling snow with depth: distant flakes are small and slow, near ones big, soft and
/// swaying.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::Snowfall;
/// ui.add(Snowfall::new().count(220).wind(0.3));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct Snowfall {
    base: Base,
    count: usize,
    color: Option<Color32>,
    speed: f32,
    wind: f32,
    size: f32,
}

impl Default for Snowfall {
    fn default() -> Self {
        Self::new()
    }
}

impl Snowfall {
    /// 180 flakes.
    pub fn new() -> Self {
        Self { base: Base::default(), count: 180, color: None, speed: 1.0, wind: 0.15, size: 1.0 }
    }
    /// Number of flakes (default 180, capped at 800).
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.min(800);
        self
    }
    /// Flake color (default: theme foreground on dark themes, muted foreground on light ones).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Fall speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Sideways wind, negative blows left (default 0.15).
    pub fn wind(mut self, wind: f32) -> Self {
        self.wind = wind;
        self
    }
    /// Flake size multiplier (default 1).
    pub fn flake_size(mut self, size: f32) -> Self {
        self.size = size.max(0.1);
        self
    }

    /// Paint the snow into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_snowfall") else { return };
        let t = &f.theme;
        let color = self.color.unwrap_or(if t.dark { t.foreground } else { glowkit_core::mix(t.muted_foreground, t.accent, 0.25) });
        let (w, h) = (rect.width() + 40.0, rect.height() + 40.0);
        let time = f.time;
        let dark = f.dark();
        for i in 0..self.count as u32 {
            let z = h01(i, 1).powf(1.5);
            let fall = (18.0 + 55.0 * z) * self.speed;
            let y = (h01(i, 2) * h + time * fall).rem_euclid(h) - 20.0;
            let sway = (time * (0.4 + 0.6 * h01(i, 4)) + h01(i, 5) * TAU).sin() * (6.0 + 14.0 * z);
            let x = (h01(i, 3) * w + time * self.wind * fall + sway).rem_euclid(w) - 20.0;
            let pos = rect.min + vec2(x, y);
            let r = self.size * (0.7 + 2.6 * z);
            let a = 0.3 + 0.65 * z;
            if z > 0.5 {
                f.painter.circle_filled(pos, r * 2.4, light(color, a * 0.12, dark));
            }
            f.painter.circle_filled(pos, r, color.gamma_multiply(a));
        }
    }
}

background!(Snowfall);

// ---------------------------------------------------------------------------------------------
// Galaxy
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct Push {
    pos: Option<Pos2>,
    strength: f32,
}

/// A slowly rotating spiral galaxy: a bright core, glowing arms of colored stars and a dusting
/// of background stars. Stars scatter away from the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::Galaxy;
/// ui.add(Galaxy::new().arms(3).stars(800));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct Galaxy {
    base: Base,
    stars: usize,
    arms: usize,
    colors: Option<Vec<Color32>>,
    speed: f32,
    twist: f32,
    tilt: f32,
}

impl Default for Galaxy {
    fn default() -> Self {
        Self::new()
    }
}

impl Galaxy {
    /// A two-armed galaxy of 700 stars.
    pub fn new() -> Self {
        Self { base: Base::default(), stars: 700, arms: 2, colors: None, speed: 1.0, twist: 3.6, tilt: 0.5 }
    }
    /// Number of stars in the galaxy (default 700, capped at 2000).
    pub fn stars(mut self, n: usize) -> Self {
        self.stars = n.min(2000);
        self
    }
    /// Number of spiral arms (default 2).
    pub fn arms(mut self, n: usize) -> Self {
        self.arms = n.clamp(1, 8);
        self
    }
    /// Colors from the core outwards (default: warm white to the theme gradient).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }
    /// Rotation speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// How tightly the arms wind, in radians from core to rim (default 3.6).
    pub fn twist(mut self, twist: f32) -> Self {
        self.twist = twist;
        self
    }
    /// Vertical squash, 1 = seen face-on (default 0.5).
    pub fn tilt(mut self, tilt: f32) -> Self {
        self.tilt = tilt.clamp(0.1, 1.0);
        self
    }

    /// Paint the galaxy into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_galaxy") else { return };
        let t = &f.theme;
        let dark = f.dark();
        let core = if dark { glowkit_core::hex("#FFF4E0") } else { t.gradient[2] };
        let colors = self.colors.clone().unwrap_or_else(|| vec![core, t.gradient[1], t.accent, t.chart[1]]);
        let lut = Lut::new(&colors, false);
        let painter = &f.painter;
        let time = f.time * self.speed;
        let c = rect.center();
        let radius = (rect.width() * 0.42).min(rect.height() * 0.95 / self.tilt.max(0.3));
        // pointer push (smoothed so stars ease back when it leaves)
        let ptr = pointer(ui, rect);
        let push = glowkit_core::with_state(ui, f.id, Push::default, |s| {
            let target = if ptr.is_some() { 1.0 } else { 0.0 };
            s.strength += (target - s.strength) * (1.0 - (-f.dt * 4.0).exp());
            if let Some(p) = ptr {
                s.pos = Some(s.pos.map_or(p, |q| q + (p - q) * (1.0 - (-f.dt * 10.0).exp())));
            }
            *s
        });
        // background stars
        let star_c = t.foreground;
        for i in 0..160u32 {
            let p = rect.min + vec2(h01(i, 11) * rect.width(), h01(i, 12) * rect.height());
            let tw = 0.5 + 0.5 * (time * (0.8 + h01(i, 13) * 2.0) + h01(i, 14) * TAU).sin();
            painter.circle_filled(p, 0.5 + h01(i, 15) * 0.8, star_c.gamma_multiply(0.15 + 0.4 * tw));
        }
        let cc = lut.sample(0.0).to_color();
        glow_ellipse(painter, c, vec2(radius * 0.75, radius * 0.75 * self.tilt), light(lut.sample(0.45).to_color(), 0.22, dark));
        glow_ellipse(painter, c, vec2(radius * 0.32, radius * 0.32 * self.tilt), light(cc, 0.55, dark));
        let rot = time * 0.08;
        // soft dust glow along the arms
        for arm in 0..self.arms {
            for k in 1..12 {
                let d = k as f32 / 12.0;
                let a = arm as f32 * TAU / self.arms as f32 + d * self.twist + rot * (1.0 + 0.3 / (d + 0.3));
                let pos = c + vec2(a.cos() * d * radius, a.sin() * d * radius * self.tilt);
                let col = lut.sample(d).to_color();
                glow_ellipse(painter, pos, vec2(radius * 0.22, radius * 0.22 * self.tilt.max(0.35)), light(col, 0.13 * (1.0 - d * 0.6), dark));
            }
        }
        for i in 0..self.stars as u32 {
            let arm = (i as usize % self.arms) as f32;
            let d = h01(i, 1).powf(0.75);
            let spread = (h01(i, 2) - 0.5 + h01(i, 3) - 0.5 + h01(i, 9) - 0.5) * (0.9 - d * 0.35);
            let a = arm * TAU / self.arms as f32 + d * self.twist + spread + rot * (1.0 + 0.3 / (d + 0.3));
            let rr = d * radius * (1.0 + (h01(i, 4) - 0.5) * 0.15);
            let mut pos = c + vec2(a.cos() * rr, a.sin() * rr * self.tilt);
            if let Some(m) = push.pos {
                let off = pos - m;
                let dist2 = off.length_sq();
                pos += off.normalized() * push.strength * 60.0 * (-dist2 / (90.0 * 90.0)).exp();
            }
            let tw = 0.65 + 0.35 * (time * (1.0 + h01(i, 5) * 3.0) + h01(i, 6) * TAU).sin();
            let col = lut.sample(d + (h01(i, 7) - 0.5) * 0.25).to_color();
            let size = 0.5 + 1.5 * h01(i, 8).powi(3) + (1.0 - d) * 0.6;
            let a = tw * (1.0 - d * 0.55);
            if size > 1.6 {
                painter.circle_filled(pos, size * 2.6, light(col, a * 0.18, dark));
            }
            painter.circle_filled(pos, size, if dark { light(col, a, true) } else { col.gamma_multiply(a) });
        }
        glow(painter, c, radius * 0.1, light(Color32::WHITE, if dark { 0.7 } else { 0.0 }, dark));
    }
}

background!(Galaxy);

// ---------------------------------------------------------------------------------------------
// Warp speed
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Star3 {
    x: f32,
    y: f32,
    z: f32,
    c: u8,
}

#[derive(Clone, Default)]
struct WarpState {
    stars: Vec<Star3>,
    boost: f32,
    rng: Option<Rng>,
}

/// Hyperspace: star streaks rushing out of a vanishing point toward the viewer. Press and hold
/// to engage warp and the streaks stretch into long beams.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::WarpSpeed;
/// ui.add(WarpSpeed::new().speed(1.5));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct WarpSpeed {
    base: Base,
    count: usize,
    colors: Option<Vec<Color32>>,
    speed: f32,
    warp: Option<bool>,
}

impl Default for WarpSpeed {
    fn default() -> Self {
        Self::new()
    }
}

impl WarpSpeed {
    /// 380 stars.
    pub fn new() -> Self {
        Self { base: Base::default(), count: 380, colors: None, speed: 1.0, warp: None }
    }
    /// Number of stars (default 380, capped at 1200).
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.min(1200);
        self
    }
    /// Star colors (default: mostly foreground with theme tints).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }
    /// Cruise speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Force warp on or off instead of using the pointer press.
    pub fn warp(mut self, on: bool) -> Self {
        self.warp = Some(on);
        self
    }

    /// Paint the star streaks into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_warp_speed") else { return };
        let t = &f.theme;
        let colors = self.colors.clone().unwrap_or_else(|| vec![t.foreground, t.foreground, t.foreground, t.accent, t.chart[1], t.gradient[1]]);
        let ncol = colors.len().max(1);
        let pressed = self.warp.unwrap_or_else(|| pointer(ui, rect).is_some() && ui.input(|i| i.pointer.primary_down()));
        let dt = f.dt;
        let count = self.count;
        let speed = self.speed;
        let (stars, boost) = glowkit_core::with_state(ui, f.id, WarpState::default, |s| {
            let rng = s.rng.get_or_insert_with(|| Rng::new(99));
            if s.stars.len() != count {
                s.stars = (0..count)
                    .map(|_| Star3 { x: rng.range(-1.0, 1.0), y: rng.range(-1.0, 1.0), z: rng.range(0.02, 1.0), c: rng.below(ncol) as u8 })
                    .collect();
            }
            let target = if pressed { 1.0 } else { 0.0 };
            s.boost += (target - s.boost) * (1.0 - (-dt * if pressed { 1.6 } else { 2.5 }).exp());
            let v = speed * (0.12 + s.boost * 0.9);
            for st in &mut s.stars {
                st.z -= v * dt;
                if st.z <= 0.02 {
                    *st = Star3 { x: rng.range(-1.0, 1.0), y: rng.range(-1.0, 1.0), z: rng.range(0.85, 1.0), c: rng.below(ncol) as u8 };
                }
            }
            (s.stars.clone(), s.boost)
        });
        let c = rect.center();
        let scale = rect.width().max(rect.height()) * 0.5;
        let dark = f.dark();
        let painter = &f.painter;
        glow(painter, c, scale * (0.25 + boost * 0.25), light(t.accent, 0.25 + boost * 0.3, dark));
        let streak = 0.025 + boost * 0.3;
        for st in &stars {
            let proj = |z: f32| c + vec2(st.x / z, st.y / z) * scale * 0.18;
            let head = proj(st.z);
            let tail = proj((st.z + streak * (1.0 - st.z * 0.5)).min(1.2));
            let near = (1.0 - st.z).clamp(0.0, 1.0);
            let a = near.powf(1.3).min(1.0) * (st.z / 0.06).clamp(0.3, 1.0);
            let w = 0.6 + near * 2.2;
            let col = colors[st.c as usize % ncol];
            painter.line_segment([tail, head], Stroke::new(w, light(col, a, dark)));
            if near > 0.7 {
                painter.circle_filled(head, w * 0.6, light(col, a, dark));
            }
        }
    }
}

background!(WarpSpeed);
