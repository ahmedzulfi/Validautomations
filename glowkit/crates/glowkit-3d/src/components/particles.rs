use super::{glow_c, salt_or};
use crate::{vec3, Background, Camera, Fog, OrbitControls, Vec3, Viewport3D};
use egui::{Color32, Response, Ui, Widget};
use glowkit_core::{color, ease, rng::hash01, Easing, Theme};
use std::f32::consts::{PI, TAU};

/// A target shape for [`ParticleMorph`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloudShape {
    Sphere,
    Torus,
    Cube,
    Wave,
    Helix,
    Galaxy,
}

impl CloudShape {
    /// The default morph cycle.
    pub const ALL: [CloudShape; 6] = [
        CloudShape::Sphere,
        CloudShape::Torus,
        CloudShape::Wave,
        CloudShape::Cube,
        CloudShape::Helix,
        CloudShape::Galaxy,
    ];

    /// Position of particle `i` of `n` on this shape (roughly within a radius of 1.6).
    pub fn point(self, i: usize, n: usize, time: f32) -> Vec3 {
        let u = (i as f32 + 0.5) / n as f32;
        let h1 = hash01(i as u64 * 7 + 1);
        let h2 = hash01(i as u64 * 13 + 5);
        match self {
            CloudShape::Sphere => {
                // fibonacci sphere: even coverage
                let y = 1.0 - 2.0 * u;
                let r = (1.0 - y * y).max(0.0).sqrt();
                let a = i as f32 * PI * (3.0 - 5f32.sqrt());
                vec3(a.cos() * r, y, a.sin() * r) * 1.35
            }
            CloudShape::Torus => {
                let a = u * TAU * 1.0;
                let b = i as f32 * 2.399_963 * 7.0;
                let ring = vec3(a.cos(), 0.0, a.sin());
                (ring * 1.05 + (ring * b.cos() + Vec3::Y * b.sin()) * 0.42) * 1.15
            }
            CloudShape::Cube => {
                let face = i % 6;
                let (s, t) = (h1 * 2.0 - 1.0, h2 * 2.0 - 1.0);
                let sg = if face % 2 == 0 { 1.0 } else { -1.0 };
                let p = match face / 2 {
                    0 => vec3(sg, s, t),
                    1 => vec3(s, sg, t),
                    _ => vec3(s, t, sg),
                };
                p * 0.95
            }
            CloudShape::Wave => {
                let side = (n as f32).sqrt().ceil() as usize;
                let (gx, gz) = ((i % side) as f32 / side as f32, (i / side) as f32 / side as f32);
                let (x, z) = ((gx - 0.5) * 3.4, (gz - 0.5) * 3.4);
                let r = (x * x + z * z).sqrt();
                vec3(x, (r * 3.2 - time * 2.4).sin() * 0.28 * (1.0 - r / 3.0).max(0.2), z)
            }
            CloudShape::Helix => {
                let strand = i % 2;
                let a = u * TAU * 3.0 + strand as f32 * PI;
                let y = (u - 0.5) * 3.2;
                let r = 0.75 + (h1 - 0.5) * 0.16;
                vec3(a.cos() * r, y, a.sin() * r)
            }
            CloudShape::Galaxy => {
                let arm = (i % 3) as f32;
                let r = h1.powf(0.7) * 1.7;
                let a = arm * TAU / 3.0 + r * 2.4 + (h2 - 0.5) * 0.5;
                vec3(a.cos() * r, (hash01(i as u64 * 31) - 0.5) * 0.15 * (1.8 - r), a.sin() * r)
            }
        }
    }
}

/// A cloud of thousands of glowing points that morphs between shapes (sphere, torus, wave,
/// cube, helix, galaxy) with a staggered, eased transition. The pointer pushes particles away.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_3d::{ParticleMorph, CloudShape};
/// ui.add(ParticleMorph::new().count(2000).shapes(&[CloudShape::Sphere, CloudShape::Torus]).height(380.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ParticleMorph {
    count: usize,
    shapes: Vec<CloudShape>,
    hold: f32,
    morph: f32,
    colors: Option<Vec<Color32>>,
    size: f32,
    height: f32,
    id_salt: Option<String>,
}

impl Default for ParticleMorph {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticleMorph {
    /// Default: 1600 points cycling through every shape.
    pub fn new() -> Self {
        Self {
            count: 1600,
            shapes: CloudShape::ALL.to_vec(),
            hold: 2.2,
            morph: 1.8,
            colors: None,
            size: 0.022,
            height: 380.0,
            id_salt: None,
        }
    }
    /// Number of particles.
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.clamp(10, 20_000);
        self
    }
    /// The shapes to cycle through.
    pub fn shapes(mut self, s: &[CloudShape]) -> Self {
        if !s.is_empty() {
            self.shapes = s.to_vec();
        }
        self
    }
    /// Seconds each shape is held, and seconds a morph takes.
    pub fn timing(mut self, hold: f32, morph: f32) -> Self {
        self.hold = hold.max(0.0);
        self.morph = morph.max(0.1);
        self
    }
    /// Gradient colors (defaults to the theme gradient).
    pub fn colors(mut self, c: &[Color32]) -> Self {
        self.colors = Some(c.to_vec());
        self
    }
    /// Particle radius in world units.
    pub fn point_size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    /// Height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Id salt for several instances.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.id_salt = Some(s.into());
        self
    }
}

impl Widget for ParticleMorph {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let stops = self.colors.clone().unwrap_or_else(|| t.gradient.to_vec());
        let bg = if t.dark { Background::Radial(color::mix(t.card, stops[0], 0.12), t.card) } else { Background::None };
        Viewport3D::new(salt_or(&self.id_salt, "particle-morph"))
            .height(self.height)
            .background(bg)
            .fov(40.0)
            .controls(OrbitControls::new().distance(4.7).pitch(0.3).auto_rotate(0.25).distance_range(2.5, 12.0))
            .show(ui, |scene, ctx| {
                let n = self.count;
                let shapes = &self.shapes;
                let cycle = self.hold + self.morph;
                let time = ctx.time;
                let k = (time / cycle).floor() as usize;
                let phase = time - k as f32 * cycle;
                let (a, b) = (shapes[k % shapes.len()], shapes[(k + 1) % shapes.len()]);
                let m = ((phase - self.hold) / self.morph).clamp(0.0, 1.0);
                let ray = ctx.pointer.map(|p| scene.ray(p));
                let spin = time * 0.15;
                let (sn, cs) = spin.sin_cos();
                let dark = t.dark;
                for i in 0..n {
                    let d = hash01(i as u64 * 3 + 11) * 0.35;
                    let local = ease(Easing::InOutCubic, ((m - d) / 0.65).clamp(0.0, 1.0));
                    let pa = a.point(i, n, time);
                    let pb = b.point(i, n, time);
                    let mut p = pa.lerp(pb, local);
                    // a little lift in the middle of the morph for a swirly feel
                    let lift = (local * PI).sin();
                    p += vec3(p.z, 0.0, -p.x) * (lift * 0.35);
                    p = vec3(p.x * cs - p.z * sn, p.y, p.x * sn + p.z * cs);
                    if let Some((o, dir)) = ray {
                        let rel = p - o;
                        let along = rel.dot(dir);
                        let off = rel - dir * along;
                        let dist = off.length();
                        if dist < 0.6 {
                            p += off.normalized() * (0.6 - dist) * 0.9;
                        }
                    }
                    let tcol = ((p.length() / 1.9) * 0.65 + (p.y / 3.2 + 0.5) * 0.35 + hash01(i as u64) * 0.12)
                        .clamp(0.0, 1.0);
                    let c = color::gradient(&stops, tcol);
                    let col = if dark {
                        crate::additive(c.gamma_multiply(0.95))
                    } else {
                        color::with_alpha(color::shade(c, -0.08), 0.9)
                    };
                    scene.point(p, self.size * (0.7 + hash01(i as u64 * 5) * 0.6), col);
                }
                // soft core glow
                scene.underlay(|s| s.glow_point(Vec3::ZERO, 1.6, glow_c(&t, stops[stops.len() / 2], 0.16)));
            })
            .response
    }
}

/// A spiral galaxy of thousands of stars with differential rotation, a glowing core and nebula
/// dust; the camera drifts through it on a slow cinematic path. Drag to look around.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_3d::Galaxy::new().arms(4).stars(3000).height(380.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Galaxy {
    stars: usize,
    arms: usize,
    twist: f32,
    core: Option<Color32>,
    colors: Option<Vec<Color32>>,
    speed: f32,
    height: f32,
    id_salt: Option<String>,
}

impl Default for Galaxy {
    fn default() -> Self {
        Self::new()
    }
}

impl Galaxy {
    /// Default: 2600 stars, 3 arms.
    pub fn new() -> Self {
        Self { stars: 2600, arms: 3, twist: 3.2, core: None, colors: None, speed: 1.0, height: 380.0, id_salt: None }
    }
    /// Number of stars.
    pub fn stars(mut self, n: usize) -> Self {
        self.stars = n.clamp(100, 20_000);
        self
    }
    /// Number of spiral arms.
    pub fn arms(mut self, n: usize) -> Self {
        self.arms = n.clamp(1, 8);
        self
    }
    /// How tightly the arms wind.
    pub fn twist(mut self, t: f32) -> Self {
        self.twist = t;
        self
    }
    /// Core color (defaults to warm white).
    pub fn core_color(mut self, c: Color32) -> Self {
        self.core = Some(c);
        self
    }
    /// Arm gradient (inner → outer). Defaults to the theme gradient.
    pub fn colors(mut self, c: &[Color32]) -> Self {
        self.colors = Some(c.to_vec());
        self
    }
    /// Rotation / camera speed multiplier.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Id salt for several instances.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.id_salt = Some(s.into());
        self
    }
}

impl Widget for Galaxy {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let core = self.core.unwrap_or(if t.dark { color::hex("#FFE8C2") } else { color::hex("#F59E0B") });
        let stops = self.colors.clone().unwrap_or_else(|| {
            vec![color::mix(core, t.gradient[1], 0.5), t.gradient[1], t.gradient[0], color::hex("#38BDF8")]
        });
        let bg = if t.dark {
            Background::Radial(color::mix(t.card, stops[2], 0.1), color::mix(t.card, Color32::BLACK, 0.3))
        } else {
            Background::None
        };
        Viewport3D::new(salt_or(&self.id_salt, "galaxy"))
            .height(self.height)
            .background(bg)
            .fov(50.0)
            .controls(OrbitControls::new().distance(6.0).pitch(0.0).yaw(0.0).zoom(false))
            .show(ui, |scene, ctx| {
                let time = ctx.time * self.speed;
                let c = ctx.controls;
                // cinematic path: slow orbit, swooping pitch and distance
                let dist = 6.6 + 1.6 * (time * 0.11).sin();
                let pitch = (0.55 + 0.35 * (time * 0.07).sin() + c.pitch).clamp(-1.2, 1.3);
                let yaw = time * 0.06 + c.yaw;
                let eye = Vec3::from_spherical(dist, yaw, pitch);
                scene.set_camera(Camera::looking_at(eye, vec3(0.0, -0.2, 0.0)).fov_degrees(50.0));
                scene.fog = Some(Fog::fade(dist + 2.0, dist + 9.0));
                let dark = t.dark;
                let n = self.stars;
                let arms = self.arms as f32;
                for i in 0..n {
                    let h = |k: u64| hash01(i as u64 * 97 + k);
                    let r = h(1).powf(1.35) * 4.2 + 0.05;
                    let arm = (i % self.arms) as f32;
                    let spread = (h(2) - 0.5) * (0.7 + r * 0.22) + (h(3) - 0.5) * 0.45;
                    let a = arm / arms * TAU + r * self.twist * 0.35 + spread - time * 0.35 / (r + 0.6);
                    let y = (h(4) + h(5) - 1.0) * 0.22 * (1.0 - r / 4.6).max(0.05);
                    let off = (h(6) - 0.5) * 0.25;
                    let p = vec3(a.cos() * (r + off), y, a.sin() * (r + off));
                    let tt = (r / 4.2).clamp(0.0, 1.0);
                    let base = if tt < 0.12 {
                        color::mix(core, stops[0], tt / 0.12)
                    } else {
                        color::gradient(&stops, (tt - 0.12) / 0.88)
                    };
                    let tw = 0.6 + 0.4 * (time * (1.5 + h(7) * 3.0) + h(8) * TAU).sin();
                    let bright = (0.45 + h(9) * 0.55) * tw;
                    let col = if dark {
                        crate::additive(base.gamma_multiply(bright))
                    } else {
                        color::with_alpha(color::shade(base, -0.12), bright * 0.85)
                    };
                    scene.point(p, 0.012 + h(10) * 0.014, col);
                    // nebula dust along arms
                    if i % 23 == 0 && r > 0.6 {
                        scene.glow_point(p, 0.35 + h(11) * 0.35, glow_c(&t, base, 0.07));
                    }
                }
                // core
                scene.glow_point(Vec3::ZERO, 1.4, glow_c(&t, core, 0.45));
                scene.glow_point(Vec3::ZERO, 0.45, glow_c(&t, Color32::WHITE, 0.7));
                // distant background stars
                scene.underlay(|s| {
                    for i in 0..220 {
                        let h = |k: u64| hash01(i as u64 * 131 + k);
                        let p = Vec3::from_spherical(30.0, h(1) * TAU, (h(2) * 2.0 - 1.0).asin());
                        let c = if dark {
                            crate::additive(Color32::WHITE.gamma_multiply(0.25 + h(3) * 0.5))
                        } else {
                            color::with_alpha(t.muted_foreground, 0.35)
                        };
                        s.point_px(eye + p, 0.6 + h(4) * 0.8, c);
                    }
                });
            })
            .response
    }
}

/// A true-3D warp-speed starfield: you fly forward through streaking stars. Hovering
/// steers gently; pressing boosts to warp speed.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_3d::Starfield::new().speed(1.0).height(320.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Starfield {
    count: usize,
    speed: f32,
    colors: Option<Vec<Color32>>,
    height: f32,
    id_salt: Option<String>,
}

impl Default for Starfield {
    fn default() -> Self {
        Self::new()
    }
}

impl Starfield {
    /// Default: 900 stars.
    pub fn new() -> Self {
        Self { count: 1400, speed: 1.0, colors: None, height: 340.0, id_salt: None }
    }
    /// Number of stars.
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.clamp(50, 10_000);
        self
    }
    /// Cruise speed multiplier.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Star tint gradient (defaults to white + theme gradient).
    pub fn colors(mut self, c: &[Color32]) -> Self {
        self.colors = Some(c.to_vec());
        self
    }
    /// Height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Id salt for several instances.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.id_salt = Some(s.into());
        self
    }
}

#[derive(Clone, Copy, Default)]
struct WarpState {
    travel: f32,
    boost: f32,
    steer: egui::Vec2,
}

impl Widget for Starfield {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let stops =
            self.colors.clone().unwrap_or_else(|| vec![Color32::WHITE, t.gradient[0], t.gradient[1], Color32::WHITE]);
        let bg = if t.dark {
            Background::Radial(color::mix(t.card, t.gradient[0], 0.15), color::mix(t.card, Color32::BLACK, 0.4))
        } else {
            Background::None
        };
        Viewport3D::new(salt_or(&self.id_salt, "starfield"))
            .height(self.height)
            .background(bg)
            .camera(Camera::looking_at(Vec3::ZERO, vec3(0.0, 0.0, -1.0)).fov_degrees(70.0))
            .show(ui, |scene, ctx| {
                let pressed = ctx.response.is_pointer_button_down_on();
                let target_steer = ctx.pointer_ndc.unwrap_or_default() * 0.25;
                let st = ctx.state("warp", WarpState::default, |s| {
                    s.boost = glowkit_core::anim::approach(s.boost, if pressed { 1.0 } else { 0.0 }, 3.0, ctx.dt);
                    s.steer = s.steer + (target_steer - s.steer) * (1.0 - (-3.0 * ctx.dt).exp());
                    s.travel += ctx.dt * self.speed * (6.0 + s.boost * 30.0);
                    *s
                });
                let roll = ctx.time * 0.05;
                let look = vec3(st.steer.x, st.steer.y, -1.0);
                let up = vec3(roll.sin(), roll.cos(), 0.0);
                let mut cam = Camera::looking_at(Vec3::ZERO, look).fov_degrees(70.0 + st.boost * 18.0);
                cam.up = up;
                scene.set_camera(cam);
                let depth = 40.0;
                let dark = t.dark;
                let streak = 0.25 + st.boost * 2.4 + self.speed * 0.35;
                for i in 0..self.count {
                    let h = |k: u64| hash01(i as u64 * 59 + k);
                    let a = h(1) * TAU;
                    let r = 0.35 + h(2).powf(0.8) * 7.0;
                    let z = -((h(3) * depth - st.travel).rem_euclid(depth)) - 0.5;
                    let p = vec3(a.cos() * r, a.sin() * r, z);
                    let near = 1.0 - (-z / depth);
                    let fade = near.clamp(0.0, 1.0).powf(1.5) * ((-z - 0.5) / 2.0).clamp(0.0, 1.0);
                    let base = color::gradient(&stops, h(4));
                    let tail = p + vec3(0.0, 0.0, -streak * (1.0 + h(5)));
                    let (ch, ct) = if dark {
                        (crate::additive(base.gamma_multiply(fade)), crate::additive(Color32::TRANSPARENT))
                    } else {
                        (color::with_alpha(color::shade(base, -0.25), fade * 0.9), Color32::TRANSPARENT)
                    };
                    scene.line_world(p, tail, 0.016 + h(6) * 0.02, ch, ct);
                    if h(7) > 0.9 {
                        scene.glow_point(
                            p,
                            0.12,
                            if dark {
                                crate::additive(base.gamma_multiply(fade * 0.35))
                            } else {
                                color::with_alpha(base, fade * 0.3)
                            },
                        );
                    }
                }
                let center = scene.rect().center();
                let _ = center;
            })
            .response
    }
}
