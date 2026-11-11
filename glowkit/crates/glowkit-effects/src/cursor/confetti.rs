use super::area_input;
use super::trail::{area_setters, area_widget};
use egui::epaint::Mesh;
use egui::{pos2, vec2, Color32, Id, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
use glowkit_core::color::{hsv, mix, shade, with_alpha};
use glowkit_core::{paint, Rng, Theme};
use std::f32::consts::TAU;

#[derive(Clone, Copy)]
struct Piece {
    pos: Pos2,
    vel: Vec2,
    rot: f32,
    spin: f32,
    flip: f32,
    flip_speed: f32,
    wobble: f32,
    color: Color32,
    size: Vec2,
    round: bool,
    t0: f32,
}

/// Confetti that bursts from a point, tumbles in 3D, flutters and falls under gravity.
///
/// Call [`ConfettiBurst::fire`] whenever something worth celebrating happens and
/// [`ConfettiBurst::show`] every frame over the area the confetti may fly through.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let area = ui.max_rect();
/// let confetti = glowkit_effects::ConfettiBurst::new().count(120);
/// if ui.button("🎉 Celebrate").clicked() {
///     confetti.fire(ui, area.center());
/// }
/// confetti.show(ui, area);
/// ```
#[must_use = "call `.show(ui, rect)` every frame"]
#[derive(Clone)]
pub struct ConfettiBurst {
    colors: Option<Vec<Color32>>,
    count: usize,
    angle: f32,
    spread: f32,
    velocity: f32,
    gravity: f32,
    lifetime: f32,
    scalar: f32,
    fire_on_click: bool,
    autoplay: bool,
    area: Option<Vec2>,
    salt: u64,
}

impl Default for ConfettiBurst {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfettiBurst {
    /// 90 pieces shot upward with a 70 degree spread; clicks in the area fire too.
    pub fn new() -> Self {
        Self {
            colors: None,
            count: 90,
            angle: -90.0,
            spread: 70.0,
            velocity: 620.0,
            gravity: 900.0,
            lifetime: 3.0,
            scalar: 1.0,
            fire_on_click: true,
            autoplay: false,
            area: None,
            salt: 0,
        }
    }
    /// Confetti colors. Defaults to the theme chart palette.
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        if !colors.is_empty() {
            self.colors = Some(colors.to_vec());
        }
        self
    }
    /// Pieces per burst.
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.clamp(1, 600);
        self
    }
    /// Launch direction in degrees (0 = right, -90 = up).
    pub fn angle(mut self, deg: f32) -> Self {
        self.angle = deg;
        self
    }
    /// Cone width in degrees (360 = all directions).
    pub fn spread(mut self, deg: f32) -> Self {
        self.spread = deg.clamp(0.0, 360.0);
        self
    }
    /// Launch speed in points per second.
    pub fn velocity(mut self, v: f32) -> Self {
        self.velocity = v.max(0.0);
        self
    }
    /// Downward acceleration in points per second squared.
    pub fn gravity(mut self, g: f32) -> Self {
        self.gravity = g;
        self
    }
    /// Seconds before a piece disappears.
    pub fn lifetime(mut self, s: f32) -> Self {
        self.lifetime = s.clamp(0.3, 10.0);
        self
    }
    /// Scale of each piece.
    pub fn scalar(mut self, s: f32) -> Self {
        self.scalar = s.clamp(0.2, 4.0);
        self
    }
    /// Fire a burst wherever the area is clicked (default on).
    pub fn fire_on_click(mut self, on: bool) -> Self {
        self.fire_on_click = on;
        self
    }
    area_setters!();

    fn id(&self, ui: &Ui) -> Id {
        ui.scope_id().with(("confetti", self.salt))
    }

    /// Fire a burst from `origin` using this burst's settings.
    pub fn fire(&self, ui: &Ui, origin: Pos2) {
        self.fire_with(ui, origin, self.angle, self.spread);
    }

    /// Fire a burst from `origin` in a custom direction (degrees) and spread.
    pub fn fire_with(&self, ui: &Ui, origin: Pos2, angle: f32, spread: f32) {
        let id = self.id(ui);
        let t = Theme::of(ui);
        let colors = self.colors.clone().unwrap_or_else(|| t.chart.to_vec());
        // simulation clock (advanced by `show`), so low frame rates never cut lifetimes short
        let now: f32 = glowkit_core::state(ui, id.with("clock"), || 0.0);
        let seed = (glowkit_core::time(ui) * 1000.0) as u64 ^ (origin.x as u64).wrapping_mul(31) ^ (origin.y as u64).wrapping_mul(131);
        let mut rng = Rng::new(seed);
        let count = self.count;
        let sc = self.scalar;
        let v0 = self.velocity;
        glowkit_core::with_state(ui, id.with("pieces"), Vec::<Piece>::new, |v| {
            for _ in 0..count {
                let a = (angle + rng.range(-0.5, 0.5) * spread).to_radians();
                let speed = v0 * rng.range(0.45, 1.05);
                let round = rng.bool(0.25);
                let w = rng.range(6.0, 10.0) * sc;
                v.push(Piece {
                    pos: origin,
                    vel: vec2(a.cos(), a.sin()) * speed,
                    rot: rng.range(0.0, TAU),
                    spin: rng.range(-8.0, 8.0),
                    flip: rng.range(0.0, TAU),
                    flip_speed: rng.range(6.0, 14.0),
                    wobble: rng.range(0.0, TAU),
                    color: colors[rng.below(colors.len())],
                    size: if round { Vec2::splat(w * 0.75) } else { vec2(w, w * rng.range(0.45, 0.7)) },
                    round,
                    t0: now,
                });
            }
            if v.len() > 1500 {
                let e = v.len() - 1500;
                v.drain(0..e);
            }
        });
        ui.ctx().request_repaint();
    }

    /// Simulate and paint the confetti over `rect` (and fire on clicks inside it).
    pub fn show(self, ui: &mut Ui, rect: Rect) {
        let id = self.id(ui);
        let input = area_input(ui, rect, id, self.autoplay, 1.6);
        if self.fire_on_click || !input.real {
            for c in &input.clicks {
                if input.real {
                    self.fire_with(ui, *c, -90.0, 360.0);
                } else {
                    // autoplay: alternate cannons from the bottom corners
                    let n = (glowkit_core::time(ui) / 1.6).floor() as i32;
                    let left = n % 2 == 0;
                    let o = if left { pos2(rect.min.x + 20.0, rect.max.y) } else { pos2(rect.max.x - 20.0, rect.max.y) };
                    self.fire_with(ui, o, if left { -60.0 } else { -120.0 }, 45.0);
                }
            }
        }
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::with_state(ui, id.with("clock"), || 0.0f32, |c| {
            *c += dt;
            *c
        });
        let g = self.gravity;
        let life = self.lifetime;
        let pieces = glowkit_core::with_state(ui, id.with("pieces"), Vec::<Piece>::new, |v| {
            for p in v.iter_mut() {
                let drag = (-2.2 * dt).exp();
                p.vel *= drag;
                p.vel.y += g * dt * 0.55;
                // terminal velocity flutter
                p.vel.y = p.vel.y.min(260.0);
                p.wobble += dt * 6.0;
                p.pos += (p.vel + vec2(p.wobble.sin() * 30.0, 0.0)) * dt;
                p.rot += p.spin * dt;
                p.flip += p.flip_speed * dt;
            }
            v.retain(|p| now - p.t0 < life && p.pos.y < rect.max.y + 60.0);
            v.clone()
        });
        if pieces.is_empty() {
            return;
        }
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return;
        }
        let painter = ui.painter_at(rect);
        let mut mesh = Mesh::default();
        for p in &pieces {
            let age = now - p.t0;
            let a = (1.0 - ((age - (life - 0.7)) / 0.7).clamp(0.0, 1.0)).min((age / 0.05).min(1.0));
            let flip = p.flip.cos();
            let col = if flip < 0.0 { shade(p.color, -0.12) } else { p.color };
            let col = col.gamma_multiply(a);
            if p.round {
                let r = p.size.x / 2.0;
                painter.add(Shape::ellipse_filled(p.pos, vec2(r, r * flip.abs().max(0.15)), col));
                continue;
            }
            let half = vec2(p.size.x / 2.0, p.size.y / 2.0 * flip.abs().max(0.12));
            let (s, c) = p.rot.sin_cos();
            let tf = |x: f32, y: f32| p.pos + vec2(x * c - y * s, x * s + y * c);
            let base = mesh.vertices.len() as u32;
            for q in [tf(-half.x, -half.y), tf(half.x, -half.y), tf(half.x, half.y), tf(-half.x, half.y)] {
                mesh.colored_vertex(q, col);
            }
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base, base + 2, base + 3);
        }
        painter.add(Shape::mesh(mesh));
    }
}
area_widget!(ConfettiBurst);

// ---------------------------------------------------------------------------------------------

const ROCKET_G: f32 = 520.0;

#[derive(Clone, Copy)]
struct Rocket {
    pos: Pos2,
    vel: Vec2,
    target_y: f32,
    hue: f32,
}

#[derive(Clone, Copy)]
struct Ember {
    pos: Pos2,
    prev: Pos2,
    vel: Vec2,
    color: Color32,
    t0: f32,
    life: f32,
    twinkle: f32,
}

/// Rockets streak up from the bottom of the area and burst into glowing, falling sparks.
/// Clicking launches a rocket toward the click.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let rect = ui.max_rect();
/// glowkit_effects::Fireworks::new().interval(0.8).show(ui, rect);
/// ```
#[must_use = "call `.show(ui, rect)` or add it with `ui.add(...)`"]
pub struct Fireworks {
    colors: Option<Vec<Color32>>,
    interval: f32,
    sparks: usize,
    power: f32,
    autoplay: bool,
    area: Option<Vec2>,
    salt: u64,
}

impl Default for Fireworks {
    fn default() -> Self {
        Self::new()
    }
}

impl Fireworks {
    /// Auto-launching fireworks in random hues.
    pub fn new() -> Self {
        Self { colors: None, interval: 0.9, sparks: 70, power: 1.0, autoplay: true, area: None, salt: 0 }
    }
    /// Pick burst colors from this list instead of random hues.
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        if !colors.is_empty() {
            self.colors = Some(colors.to_vec());
        }
        self
    }
    /// Seconds between automatic launches (`0` = only launch on click).
    pub fn interval(mut self, s: f32) -> Self {
        self.interval = s.max(0.0);
        self
    }
    /// Sparks per explosion.
    pub fn sparks(mut self, n: usize) -> Self {
        self.sparks = n.clamp(4, 300);
        self
    }
    /// Explosion size multiplier.
    pub fn power(mut self, p: f32) -> Self {
        self.power = p.clamp(0.2, 4.0);
        self
    }
    area_setters!();

    /// Simulate and paint over `rect`.
    pub fn show(self, ui: &mut Ui, rect: Rect) {
        let id = ui.scope_id().with(("fireworks", self.salt));
        let input = area_input(ui, rect, id, false, 0.0);
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::with_state(ui, id.with("clock"), || 0.0f32, |c| {
            *c += dt;
            *c
        });
        let auto = self.autoplay && self.interval > 0.0;
        let colors = self.colors.clone();
        let n_sparks = self.sparks;
        let power = self.power * rect.height().min(420.0) / 360.0;
        let interval = self.interval;
        let (rockets, embers) = glowkit_core::with_state(
            ui,
            id.with("st"),
            || (Vec::<Rocket>::new(), Vec::<Ember>::new(), now, 1u64),
            |(rockets, embers, next, seed)| {
                let launch = |target: Pos2, seed: &mut u64, rockets: &mut Vec<Rocket>| {
                    *seed += 1;
                    let mut r = Rng::new(*seed * 977);
                    let x0 = (target.x + r.range(-60.0, 60.0)).clamp(rect.min.x + 10.0, rect.max.x - 10.0);
                    let rise = (rect.max.y - target.y).max(40.0);
                    let vy = -(2.0 * ROCKET_G * rise).sqrt();
                    let t_up = -vy / ROCKET_G;
                    rockets.push(Rocket { pos: pos2(x0, rect.max.y), vel: vec2((target.x - x0) / t_up, vy), target_y: target.y, hue: r.range(0.0, 360.0) });
                };
                for c in &input.clicks {
                    launch(*c, seed, rockets);
                }
                if auto && now >= *next {
                    let mut r = Rng::new(*seed * 31 + 7);
                    let target = pos2(r.range(rect.min.x + rect.width() * 0.15, rect.max.x - rect.width() * 0.15), r.range(rect.min.y + rect.height() * 0.15, rect.min.y + rect.height() * 0.5));
                    launch(target, seed, rockets);
                    *next = now + interval * r.range(0.6, 1.3);
                }
                let mut boom = Vec::new();
                rockets.retain_mut(|r| {
                    r.vel.y += ROCKET_G * dt;
                    r.pos += r.vel * dt;
                    if r.vel.y >= -20.0 || r.pos.y <= r.target_y {
                        boom.push(*r);
                        false
                    } else {
                        true
                    }
                });
                for b in boom {
                    *seed += 1;
                    let mut rng = Rng::new(*seed * 7331);
                    let base = match &colors {
                        Some(c) => c[rng.below(c.len())],
                        None => hsv(b.hue, 0.7, 1.0),
                    };
                    let accent = match &colors {
                        Some(c) => c[rng.below(c.len())],
                        None => hsv((b.hue + 40.0) % 360.0, 0.55, 1.0),
                    };
                    let ring = rng.bool(0.35);
                    for k in 0..n_sparks {
                        let a = k as f32 / n_sparks as f32 * TAU + rng.range(-0.05, 0.05);
                        let sp = if ring { 170.0 * rng.range(0.92, 1.0) } else { 190.0 * rng.range(0.15, 1.0).sqrt() } * power;
                        embers.push(Ember {
                            pos: b.pos,
                            prev: b.pos,
                            vel: vec2(a.cos(), a.sin()) * sp + b.vel * 0.15,
                            color: if rng.bool(0.25) { accent } else { base },
                            t0: now,
                            life: rng.range(1.0, 1.7),
                            twinkle: rng.range(0.0, TAU),
                        });
                    }
                }
                for e in embers.iter_mut() {
                    e.prev = e.pos;
                    e.vel *= (-1.6 * dt).exp();
                    e.vel.y += 70.0 * dt;
                    e.pos += e.vel * dt;
                }
                embers.retain(|e| now - e.t0 < e.life);
                if embers.len() > 2400 {
                    let x = embers.len() - 2400;
                    embers.drain(0..x);
                }
                (rockets.clone(), embers.clone())
            },
        );
        if auto || !rockets.is_empty() || !embers.is_empty() {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return;
        }
        let painter = ui.painter_at(rect);
        for r in &rockets {
            let tail: Vec<Pos2> = (0..10).map(|i| r.pos - r.vel * 0.006 * i as f32).collect();
            let c = hsv(r.hue, 0.35, 1.0);
            crate::motion::common::tapered_ribbon(&painter, &tail, |u| 3.0 * (1.0 - u), |u| with_alpha(c, 1.0 - u));
            paint::glow(&painter, r.pos, 10.0, with_alpha(c, 0.6));
        }
        let mut glow_budget = 30;
        for e in &embers {
            let age = (now - e.t0) / e.life;
            let tw = 0.75 + 0.25 * (e.twinkle + now * 18.0).sin();
            let a = (1.0 - age).powf(1.4) * tw;
            let hot = mix(e.color, Color32::WHITE, (1.0 - age * 3.0).clamp(0.0, 1.0) * 0.6);
            let tail = e.prev - (e.pos - e.prev) * 3.0;
            painter.line_segment([tail, e.pos], Stroke::new(1.8, with_alpha(hot, a)));
            if glow_budget > 0 && age < 0.12 {
                glow_budget -= 1;
                paint::glow(&painter, e.pos, 16.0, with_alpha(e.color, 0.3 * a));
            }
        }
    }
}
area_widget!(Fireworks);
