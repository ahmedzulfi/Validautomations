//! Night-sky backgrounds: meteors, twinkling stars and shooting stars.

use crate::{background, h01, light, Base, Lut};
use egui::{vec2, Color32, Painter, Pos2, Rect, Ui, Vec2};
use glowkit_core::paint::{gradient_polyline, sparkle};
use glowkit_core::Rng;
use std::f32::consts::TAU;

// ---------------------------------------------------------------------------------------------
// Meteors
// ---------------------------------------------------------------------------------------------

/// Meteors streaking diagonally across the area, each with a glowing head and a fading tail,
/// at random speeds and delays.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::Meteors;
/// ui.add(Meteors::new().count(20));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct Meteors {
    base: Base,
    count: usize,
    color: Option<Color32>,
    angle: f32,
    tail: f32,
    speed: f32,
}

impl Default for Meteors {
    fn default() -> Self {
        Self::new()
    }
}

impl Meteors {
    /// 20 meteors falling down and to the left.
    pub fn new() -> Self {
        Self { base: Base::default(), count: 20, color: None, angle: 145.0, tail: 90.0, speed: 1.0 }
    }
    /// Number of meteors (default 20, capped at 200).
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.min(200);
        self
    }
    /// Meteor color (default: theme foreground blended toward muted).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Travel direction in degrees, clockwise from pointing right (default 145 = down-left).
    pub fn angle(mut self, degrees: f32) -> Self {
        self.angle = degrees;
        self
    }
    /// Tail length in points (default 90).
    pub fn tail(mut self, length: f32) -> Self {
        self.tail = length.max(4.0);
        self
    }
    /// Speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed.max(0.01);
        self
    }

    /// Paint the meteors into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_meteors") else { return };
        let t = &f.theme;
        let dark = f.dark();
        let color = self.color.unwrap_or_else(|| glowkit_core::mix(t.muted_foreground, t.foreground, 0.4));
        let dir = Vec2::angled(self.angle.to_radians());
        let span = rect.width() + rect.height();
        let p = &f.painter;
        for i in 0..self.count as u32 {
            let dur = (2.0 + 6.0 * h01(i, 1)) / self.speed;
            let delay = 0.2 + 2.0 * h01(i, 2);
            let cycle = dur + delay;
            let local = (f.time + h01(i, 3) * cycle).rem_euclid(cycle);
            if local < delay {
                continue;
            }
            let k = (local - delay) / dur;
            // start somewhere along the edge the meteors come from
            let u = h01(i, 4);
            let start = if dir.x < 0.0 {
                rect.left_top() + vec2(rect.width() * (0.15 + u * 1.1), -10.0 - h01(i, 5) * rect.height() * 0.3)
            } else {
                rect.left_top() + vec2(rect.width() * (u * 1.1 - 0.25), -10.0 - h01(i, 5) * rect.height() * 0.3)
            };
            let head = start + dir * span * 0.8 * k;
            let a = (1.0 - ((k - 0.6) / 0.4).max(0.0)) * (k / 0.05).min(1.0);
            let tail = head - dir * self.tail * (0.7 + 0.6 * h01(i, 6));
            gradient_polyline(p, &[tail, tail.lerp(head, 0.5), head], false, 1.2, |s| color.gamma_multiply(a * s * s));
            p.circle_filled(head, 4.0, light(color, a * 0.18, dark));
            p.circle_filled(head, 1.3, color.gamma_multiply(a));
        }
    }
}

background!(Meteors);

// ---------------------------------------------------------------------------------------------
// Star field
// ---------------------------------------------------------------------------------------------

/// A calm field of stars at random sizes, most of them gently twinkling. The brightest ones get
/// a little four-pointed glint.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::StarField;
/// ui.add(StarField::new().density(1.2));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct StarField {
    base: Base,
    density: f32,
    twinkle: f32,
    speed: f32,
    color: Option<Color32>,
}

impl Default for StarField {
    fn default() -> Self {
        Self::new()
    }
}

impl StarField {
    /// About one star per 100 x 100 points... times 1.0 density.
    pub fn new() -> Self {
        Self { base: Base::default(), density: 1.0, twinkle: 0.7, speed: 1.0, color: None }
    }
    /// Stars per 10 000 square points (default 1).
    pub fn density(mut self, d: f32) -> Self {
        self.density = d.max(0.0);
        self
    }
    /// Fraction of stars that twinkle, `0..=1` (default 0.7).
    pub fn twinkle(mut self, fraction: f32) -> Self {
        self.twinkle = fraction.clamp(0.0, 1.0);
        self
    }
    /// Twinkle speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Star color (default: theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }

    fn paint_stars(&self, painter: &Painter, rect: Rect, time: f32, color: Color32, dark: bool) {
        let n = ((rect.area() / 10_000.0) * self.density * 1.2).min(1500.0) as u32;
        for i in 0..n {
            let p = rect.min + vec2(h01(i, 21) * rect.width(), h01(i, 22) * rect.height());
            let size = 0.45 + 1.1 * h01(i, 23).powi(3);
            let mut a = 0.35 + 0.55 * h01(i, 24);
            if h01(i, 25) < self.twinkle {
                let s = (0.5 + 1.0 * h01(i, 26)) * self.speed;
                a *= 0.35 + 0.65 * (0.5 + 0.5 * (time * s * TAU * 0.5 + h01(i, 27) * TAU).sin());
            }
            if size > 1.2 {
                painter.circle_filled(p, size * 3.0, light(color, a * 0.12, dark));
                sparkle(painter, p, size * 3.2, 0.0, color.gamma_multiply(a * 0.7));
            }
            painter.circle_filled(p, size, color.gamma_multiply(a));
        }
    }

    /// Paint the stars into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_star_field") else { return };
        let color = self.color.unwrap_or(f.theme.foreground);
        self.paint_stars(&f.painter, rect, f.time, color, f.dark());
    }
}

background!(StarField);

// ---------------------------------------------------------------------------------------------
// Shooting stars
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Shot {
    pos: Pos2,
    dir: Vec2,
    speed: f32,
    len: f32,
    age: f32,
}

#[derive(Clone, Default)]
struct ShotState {
    shots: Vec<Shot>,
    next: f32,
    rng: Option<Rng>,
}

/// Shooting stars: every couple of seconds a bright streak with a colorful gradient trail
/// shoots across a twinkling star field.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::{ShootingStars, StarField};
/// ui.add(ShootingStars::new().stars(StarField::new().density(1.4)).delay(0.8, 2.5));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct ShootingStars {
    base: Base,
    stars: Option<StarField>,
    colors: Option<[Color32; 2]>,
    delay: (f32, f32),
    speed: f32,
    trail: f32,
    width: f32,
}

impl Default for ShootingStars {
    fn default() -> Self {
        Self::new()
    }
}

impl ShootingStars {
    /// Shooting stars over a default [`StarField`].
    pub fn new() -> Self {
        Self { base: Base::default(), stars: Some(StarField::new()), colors: None, delay: (1.0, 3.5), speed: 1.0, trail: 170.0, width: 2.0 }
    }
    /// The star field painted underneath (`StarField::new()` by default).
    pub fn stars(mut self, stars: StarField) -> Self {
        self.stars = Some(stars);
        self
    }
    /// Paint only the shooting stars, no star field.
    pub fn no_stars(mut self) -> Self {
        self.stars = None;
        self
    }
    /// Trail gradient: tail color, head color (default: theme gradient start and a cyan).
    pub fn colors(mut self, tail: Color32, head: Color32) -> Self {
        self.colors = Some([tail, head]);
        self
    }
    /// Random wait between shooting stars, in seconds (default 1 to 3.5).
    pub fn delay(mut self, min: f32, max: f32) -> Self {
        self.delay = (min.max(0.05), max.max(min.max(0.05)));
        self
    }
    /// Speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed.max(0.05);
        self
    }
    /// Trail length in points (default 170).
    pub fn trail(mut self, length: f32) -> Self {
        self.trail = length.max(10.0);
        self
    }
    /// Trail width in points (default 2).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.max(0.5);
        self
    }

    /// Paint the sky into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_shooting_stars") else { return };
        let t = &f.theme;
        let dark = f.dark();
        if let Some(stars) = &self.stars {
            stars.paint_stars(&f.painter, rect, f.time, stars.color.unwrap_or(t.foreground), dark);
        }
        let [c0, c1] = self.colors.unwrap_or([t.gradient[0], t.chart[1]]);
        let lut = Lut::new(&[c0, c1], false);
        let (dmin, dmax, speed, trail) = (self.delay.0, self.delay.1, self.speed, self.trail);
        let dt = f.dt;
        let shots = glowkit_core::with_state(ui, f.id, ShotState::default, |s| {
            let rng = s.rng.get_or_insert_with(|| Rng::new(4242));
            s.next -= dt;
            if s.next <= 0.0 && s.shots.len() < 4 {
                s.next = rng.range(dmin, dmax);
                let ang = rng.range(18.0f32, 38.0).to_radians();
                let from_left = rng.bool(0.4);
                let pos = if from_left {
                    rect.left_top() + vec2(-20.0, rng.range(0.0, rect.height() * 0.6))
                } else {
                    rect.left_top() + vec2(rng.range(0.0, rect.width() * 0.75), -20.0)
                };
                s.shots.push(Shot { pos, dir: Vec2::angled(ang), speed: rng.range(500.0, 900.0), len: trail * rng.range(0.7, 1.2), age: 0.0 });
            }
            for sh in &mut s.shots {
                sh.pos += sh.dir * sh.speed * speed * dt;
                sh.age += dt;
            }
            s.shots.retain(|sh| rect.expand(sh.len + 40.0).contains(sh.pos));
            s.shots.clone()
        });
        let p = &f.painter;
        for sh in shots {
            let grow = (sh.age * 3.0).min(1.0);
            let tail = sh.pos - sh.dir * sh.len * grow;
            let pts: Vec<Pos2> = (0..=12).map(|k| tail.lerp(sh.pos, k as f32 / 12.0)).collect();
            gradient_polyline(p, &pts, false, self.width * 3.5, |s| light(lut.sample(s).to_color(), s.powi(3) * 0.25, dark));
            gradient_polyline(p, &pts, false, self.width, |s| lut.sample(s).to_color().gamma_multiply(s.powf(1.5)));
            p.circle_filled(sh.pos, self.width * 5.0, light(c1, 0.25, dark));
            p.circle_filled(sh.pos, self.width * 0.8, if dark { Color32::WHITE } else { c1 });
        }
    }
}

background!(ShootingStars);
