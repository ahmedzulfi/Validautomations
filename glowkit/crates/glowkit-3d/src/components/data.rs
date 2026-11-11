use super::{glow_c, salt_or};
use crate::{vec3, Background, Lights, Material, Mesh, OrbitControls, Transform, Vec3, Viewport3D};
use egui::{Align2, Color32, Response, Ui, Widget};
use glowkit_core::{color, ease, rng::hash01, Easing, Theme};
use std::f32::consts::{PI, TAU};

// ---------------------------------------------------------------------------------------------
// BarChart3D
// ---------------------------------------------------------------------------------------------

/// A 3D bar chart: rows of series by columns of categories, growing in with a staggered spring,
/// on a floor grid. Hover a bar to highlight it and read its value; drag to orbit;
/// double-click to replay the intro.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let data = vec![vec![3.0, 5.0, 2.0, 6.0], vec![4.0, 2.0, 5.0, 3.0]];
/// ui.add(glowkit_3d::BarChart3D::new(data).columns(&["Q1", "Q2", "Q3", "Q4"]).rows(&["2024", "2025"]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BarChart3D {
    data: Vec<Vec<f32>>,
    col_labels: Vec<String>,
    row_labels: Vec<String>,
    colors: Option<Vec<Color32>>,
    height: f32,
    id_salt: Option<String>,
}

impl BarChart3D {
    /// `data[row][column]`: each row is a series drawn front-to-back.
    pub fn new(data: Vec<Vec<f32>>) -> Self {
        Self { data, col_labels: Vec::new(), row_labels: Vec::new(), colors: None, height: 380.0, id_salt: None }
    }
    /// Category (column) labels along the front.
    pub fn columns(mut self, labels: &[&str]) -> Self {
        self.col_labels = labels.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Series (row) labels along the side.
    pub fn rows(mut self, labels: &[&str]) -> Self {
        self.row_labels = labels.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Series colors (defaults to the theme chart palette).
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

fn ray_box(o: Vec3, d: Vec3, lo: Vec3, hi: Vec3) -> Option<f32> {
    let mut t0 = f32::NEG_INFINITY;
    let mut t1 = f32::INFINITY;
    for (oo, dd, l, h) in [(o.x, d.x, lo.x, hi.x), (o.y, d.y, lo.y, hi.y), (o.z, d.z, lo.z, hi.z)] {
        if dd.abs() < 1e-8 {
            if oo < l || oo > h {
                return None;
            }
        } else {
            let (a, b) = ((l - oo) / dd, (h - oo) / dd);
            t0 = t0.max(a.min(b));
            t1 = t1.min(a.max(b));
        }
    }
    (t1 >= t0 && t1 > 0.0).then_some(t0.max(0.0))
}

impl Widget for BarChart3D {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let palette = self.colors.clone().unwrap_or_else(|| t.chart.to_vec());
        let rows = self.data.len();
        let cols = self.data.iter().map(|r| r.len()).max().unwrap_or(0);
        let max = self.data.iter().flatten().fold(0.0f32, |a, b| a.max(*b)).max(1e-6);
        let span = cols.max(rows) as f32;
        Viewport3D::new(salt_or(&self.id_salt, "bar-chart-3d"))
            .height(self.height)
            .fov(34.0)
            .controls(OrbitControls::new().distance(span * 1.75 + 3.0).pitch(0.5).yaw(0.6).auto_rotate(0.12).pitch_range(0.1, 1.3).target(vec3(0.0, 0.9, 0.0)))
            .show(ui, |scene, ctx| {
                if rows == 0 || cols == 0 {
                    return;
                }
                let cube = ctx.mesh("bar", || Mesh::cube(1.0));
                let restart = ctx.response.double_clicked();
                let start = ctx.state("start", || ctx.time, |s| {
                    if restart {
                        *s = ctx.time;
                    }
                    *s
                });
                let el = ctx.time - start;
                let pitch = 1.0;
                let foot = 0.62;
                let hmax = 2.6;
                let pos = |r: usize, c: usize| vec3((c as f32 - (cols - 1) as f32 / 2.0) * pitch, 0.0, (r as f32 - (rows - 1) as f32 / 2.0) * pitch);
                // hover pick
                let ray = ctx.pointer.map(|p| scene.ray(p));
                let mut hovered: Option<(usize, usize)> = None;
                let mut best = f32::INFINITY;
                let grow = |r: usize, c: usize| ease(Easing::OutBack, ((el - (r + c) as f32 * 0.06) / 0.9).clamp(0.0, 1.0));
                if let Some((o, d)) = ray {
                    for (r, row) in self.data.iter().enumerate() {
                        for (c, v) in row.iter().enumerate() {
                            let h = (v / max * hmax * grow(r, c)).max(0.02);
                            let p = pos(r, c);
                            if let Some(tt) = ray_box(o, d, p - vec3(foot / 2.0, 0.0, foot / 2.0), p + vec3(foot / 2.0, h, foot / 2.0)) {
                                if tt < best {
                                    best = tt;
                                    hovered = Some((r, c));
                                }
                            }
                        }
                    }
                }
                if el < 2.0 {
                    ctx.ui.ctx().request_repaint();
                }
                scene.lights = Lights::default();
                let floor_w = cols as f32 * pitch + 0.6;
                let floor_d = rows as f32 * pitch + 0.6;
                let plate = if t.dark { color::mix(t.card, t.muted, 0.6) } else { color::mix(t.background, t.muted, 0.9) };
                let gl = color::with_alpha(t.muted_foreground, if t.dark { 0.25 } else { 0.35 });
                scene.underlay(|s| {
                    let (hw, hd) = (floor_w / 2.0, floor_d / 2.0);
                    s.quad([vec3(-hw, 0.0, -hd), vec3(hw, 0.0, -hd), vec3(hw, 0.0, hd), vec3(-hw, 0.0, hd)], plate);
                    for c in 0..=cols {
                        let x = (c as f32 - cols as f32 / 2.0) * pitch;
                        s.line(vec3(x, 0.001, -hd), vec3(x, 0.001, hd), 1.0, gl);
                    }
                    for r in 0..=rows {
                        let z = (r as f32 - rows as f32 / 2.0) * pitch;
                        s.line(vec3(-hw, 0.001, z), vec3(hw, 0.001, z), 1.0, gl);
                    }
                    // back wall guide lines
                    for k in 1..=4 {
                        let y = hmax * k as f32 / 4.0;
                        s.line(vec3(-hw, y, -hd), vec3(hw, y, -hd), 1.0, gl.gamma_multiply(0.6));
                    }
                });
                for (r, row) in self.data.iter().enumerate() {
                    for (c, v) in row.iter().enumerate() {
                        let g = grow(r, c);
                        let h = (v / max * hmax * g).max(0.02);
                        let p = pos(r, c);
                        let base = palette[r % palette.len()];
                        let hot = hovered == Some((r, c));
                        let dim = hovered.is_some() && !hot;
                        let col = if hot { color::mix(base, Color32::WHITE, 0.25) } else if dim { color::mix(base, t.card, 0.35) } else { base };
                        let mat = Material::smooth(col).specular(0.35).shininess(30.0).rim(if hot { 0.6 } else { 0.15 }).emissive(if hot { 0.15 } else { 0.0 });
                        let tf = Transform::from_translation(p + vec3(0.0, h / 2.0, 0.0)).with_scale3(vec3(foot, h, foot));
                        scene.group(p + vec3(0.0, h / 2.0, 0.0), |s| s.draw(&cube, tf, &mat));
                    }
                }
                let lc = t.muted_foreground;
                for (c, l) in self.col_labels.iter().enumerate().take(cols) {
                    let p = pos(rows - 1, c) + vec3(0.0, 0.0, pitch * 0.85);
                    scene.text(l, p, Vec3::X, -Vec3::Z, 0.26, lc, Align2::CENTER_CENTER);
                }
                for (r, l) in self.row_labels.iter().enumerate().take(rows) {
                    let p = pos(r, cols - 1) + vec3(pitch * 0.7, 0.0, 0.0);
                    scene.text(l, p, Vec3::X, -Vec3::Z, 0.24, palette[r % palette.len()], Align2::LEFT_CENTER);
                }
                if let Some((r, c)) = hovered {
                    let v = self.data[r][c];
                    let h = v / max * hmax * grow(r, c);
                    let name = match (self.row_labels.get(r), self.col_labels.get(c)) {
                        (Some(a), Some(b)) => format!("{a} · {b}: {v}"),
                        _ => format!("{v}"),
                    };
                    scene.overlay(|s| s.label(&name, pos(r, c) + vec3(0.0, h + 0.25, 0.0), 13.0, t.foreground, Align2::CENTER_BOTTOM));
                }
            })
            .response
    }
}

// ---------------------------------------------------------------------------------------------
// Helix
// ---------------------------------------------------------------------------------------------

/// A rotating DNA double helix: two backbones of glossy spheres joined by colored base-pair
/// rungs, with depth fog. Drag to orbit.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_3d::Helix::new().pairs(28).speed(0.6).height(320.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Helix {
    pairs: usize,
    turns: f32,
    speed: f32,
    colors: Option<[Color32; 4]>,
    height: f32,
    id_salt: Option<String>,
}

impl Default for Helix {
    fn default() -> Self {
        Self::new()
    }
}

impl Helix {
    /// 26 base pairs over ~2.5 turns.
    pub fn new() -> Self {
        Self { pairs: 26, turns: 2.5, speed: 0.7, colors: None, height: 320.0, id_salt: None }
    }
    /// Number of base pairs.
    pub fn pairs(mut self, n: usize) -> Self {
        self.pairs = n.clamp(2, 400);
        self
    }
    /// Number of full turns.
    pub fn turns(mut self, t: f32) -> Self {
        self.turns = t;
        self
    }
    /// Rotation speed (radians/second).
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Colors: strand A, strand B, base 1, base 2.
    pub fn colors(mut self, c: [Color32; 4]) -> Self {
        self.colors = Some(c);
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

impl Widget for Helix {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let [ca, cb, b1, b2] = self.colors.unwrap_or([t.chart[0], t.chart[1], t.chart[2], t.chart[3]]);
        let aspect = ui.available_width() / self.height;
        let len = (aspect * 2.6).clamp(4.0, 10.0);
        Viewport3D::new(salt_or(&self.id_salt, "dna-helix"))
            .height(self.height)
            .fov(36.0)
            .fog(crate::Fog::fade(5.0, 11.0))
            .controls(OrbitControls::new().distance(7.0).pitch(0.15).yaw(0.0).distance_range(3.0, 14.0))
            .show(ui, |scene, ctx| {
                scene.lights = Lights::default();
                let n = self.pairs;
                let r = 0.85;
                let spin = ctx.time * self.speed;
                let strand = |k: f32, off: f32| -> Vec3 {
                    let u = k / (n - 1) as f32;
                    let a = u * self.turns * TAU + spin + off;
                    vec3((u - 0.5) * len, a.cos() * r, a.sin() * r)
                };
                for i in 0..n {
                    let k = i as f32;
                    let (pa, pb) = (strand(k, 0.0), strand(k, PI));
                    let mid = (pa + pb) * 0.5;
                    let pair = (hash01(i as u64 * 7) * 4.0) as usize;
                    let (c1, c2) = if pair % 2 == 0 { (b1, b2) } else { (b2, b1) };
                    scene.line_world(pa, mid, 0.07, c1, c1);
                    scene.line_world(mid, pb, 0.07, c2, c2);
                    scene.ball(pa, 0.17, ca);
                    scene.ball(pb, 0.17, cb);
                    if i + 1 < n {
                        // backbone links
                        for s in 1..3 {
                            let kk = k + s as f32 / 3.0;
                            scene.ball(strand(kk, 0.0), 0.08, color::mix(ca, t.card, 0.15));
                            scene.ball(strand(kk, PI), 0.08, color::mix(cb, t.card, 0.15));
                        }
                    }
                }
                scene.underlay(|s| s.glow_point(Vec3::ZERO, 3.2, glow_c(&t, ca, if t.dark { 0.1 } else { 0.06 })));
            })
            .response
    }
}

// ---------------------------------------------------------------------------------------------
// SolarSystem
// ---------------------------------------------------------------------------------------------

/// A planet of a [`SolarSystem`].
#[derive(Clone, Debug, PartialEq)]
pub struct Planet {
    pub name: String,
    pub color: Color32,
    /// Radius in world units.
    pub radius: f32,
    /// Orbit radius in world units.
    pub orbit: f32,
    /// Angular speed (radians/second).
    pub speed: f32,
    /// Saturn-style ring.
    pub ring: bool,
    /// A small moon.
    pub moon: bool,
}

impl Planet {
    /// A planet.
    pub fn new(name: impl Into<String>, color: Color32, radius: f32, orbit: f32, speed: f32) -> Self {
        Self { name: name.into(), color, radius, orbit, speed, ring: false, moon: false }
    }
    /// Give it a ring.
    pub fn ring(mut self) -> Self {
        self.ring = true;
        self
    }
    /// Give it a moon.
    pub fn moon(mut self) -> Self {
        self.moon = true;
        self
    }
    /// A stylized version of our solar system (not to scale).
    pub fn samples() -> Vec<Planet> {
        let h = color::hex;
        vec![
            Planet::new("Mercury", h("#A8A29E"), 0.12, 1.5, 1.6),
            Planet::new("Venus", h("#F5C77E"), 0.2, 2.1, 1.15),
            Planet::new("Earth", h("#3B82F6"), 0.22, 2.85, 0.9).moon(),
            Planet::new("Mars", h("#EF6B4A"), 0.16, 3.55, 0.72),
            Planet::new("Jupiter", h("#E8B98A"), 0.46, 4.7, 0.42),
            Planet::new("Saturn", h("#F1D9A0"), 0.36, 6.0, 0.3).ring(),
            Planet::new("Neptune", h("#6366F1"), 0.26, 7.1, 0.22),
        ]
    }
}

/// An animated solar system: a glowing sun, planets lit from the sun (true day/night sides)
/// on faint orbit rings, a ringed giant and an orbiting moon. Drag to orbit, hover to see names.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_3d::{SolarSystem, Planet};
/// ui.add(SolarSystem::new(Planet::samples()).labels(true).height(380.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SolarSystem {
    planets: Vec<Planet>,
    sun: Option<Color32>,
    labels: bool,
    speed: f32,
    height: f32,
    id_salt: Option<String>,
}

impl SolarSystem {
    /// A system with these planets.
    pub fn new(planets: Vec<Planet>) -> Self {
        Self { planets, sun: None, labels: true, speed: 1.0, height: 380.0, id_salt: None }
    }
    /// Sun color.
    pub fn sun_color(mut self, c: Color32) -> Self {
        self.sun = Some(c);
        self
    }
    /// Show planet names.
    pub fn labels(mut self, on: bool) -> Self {
        self.labels = on;
        self
    }
    /// Time multiplier.
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

impl Widget for SolarSystem {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let dark = t.dark;
        let sun = self.sun.unwrap_or(color::hex("#FDB813"));
        let outer = self.planets.iter().map(|p| p.orbit + p.radius).fold(2.0, f32::max);
        let bg = if dark { Background::Radial(color::mix(t.card, sun, 0.06), color::mix(t.card, Color32::BLACK, 0.35)) } else { Background::None };
        Viewport3D::new(salt_or(&self.id_salt, "solar-system"))
            .height(self.height)
            .background(bg)
            .fov(34.0)
            .controls(OrbitControls::new().distance(outer * 1.75).pitch(0.42).yaw(0.3).auto_rotate(0.05).distance_range(outer * 0.6, outer * 4.0))
            .show(ui, |scene, ctx| {
                let time = ctx.time * self.speed;
                let cam = scene.camera().position;
                // background stars
                if dark {
                    scene.underlay(|s| {
                        for i in 0..260 {
                            let h = |k: u64| hash01(i as u64 * 41 + k);
                            let d = Vec3::from_spherical(60.0, h(1) * TAU, (h(2) * 2.0 - 1.0).asin());
                            let tw = 0.6 + 0.4 * (ctx.time * (0.5 + h(3) * 2.0) + h(4) * 9.0).sin();
                            s.point_px(cam + d, 0.5 + h(5) * 0.8, crate::additive(Color32::WHITE.gamma_multiply((0.15 + h(6) * 0.45) * tw)));
                        }
                    });
                }
                // sun
                scene.glow_point(Vec3::ZERO, 2.4, glow_c(&t, sun, if dark { 0.35 } else { 0.25 }));
                scene.glow_point(Vec3::ZERO, 1.1, glow_c(&t, color::mix(sun, Color32::WHITE, 0.3), if dark { 0.55 } else { 0.4 }));
                scene.ball_lit(Vec3::ZERO, 0.7, color::mix(sun, Color32::WHITE, 0.25), (cam).normalized(), 0.85, 0.2);
                let ring_c = if dark { color::with_alpha(Color32::WHITE, 0.12) } else { color::with_alpha(t.foreground, 0.16) };
                for (i, p) in self.planets.iter().enumerate() {
                    scene.circle(Vec3::ZERO, p.orbit, Vec3::X, Vec3::Z, 96, 1.0, ring_c);
                    let a = time * p.speed + i as f32 * 2.1;
                    let pos = vec3(a.cos() * p.orbit, 0.0, a.sin() * p.orbit);
                    let to_sun = -pos.normalized();
                    scene.ball_lit(pos, p.radius, p.color, to_sun, if dark { 0.06 } else { 0.25 }, 0.35);
                    if p.ring {
                        let tilt = crate::Quat::from_axis_angle(vec3(1.0, 0.0, 0.4), 0.45);
                        let (u, v) = (tilt.rotate(Vec3::X), tilt.rotate(Vec3::Z));
                        let seg = 48;
                        let (r0, r1) = (p.radius * 1.35, p.radius * 2.2);
                        for k in 0..seg {
                            let (a0, a1) = (k as f32 / seg as f32 * TAU, (k + 1) as f32 / seg as f32 * TAU);
                            let d0 = u * a0.cos() + v * a0.sin();
                            let d1 = u * a1.cos() + v * a1.sin();
                            let lit = 0.55 + 0.45 * d0.dot(to_sun).max(0.0);
                            let ci = color::with_alpha(color::shade(p.color, -0.05 * lit), 0.75 * lit);
                            let co = color::with_alpha(color::shade(p.color, -0.15), 0.35 * lit);
                            scene.group(pos + (d0 + d1) * (r0 + r1) * 0.25, |s| {
                                s.triangle([pos + d0 * r0, pos + d0 * r1, pos + d1 * r1], [ci, co, co]);
                                s.triangle([pos + d0 * r0, pos + d1 * r1, pos + d1 * r0], [ci, co, ci]);
                            });
                        }
                    }
                    if p.moon {
                        let ma = time * 3.0;
                        let mp = pos + vec3(ma.cos(), 0.15 * ma.sin(), ma.sin()) * (p.radius * 2.0);
                        scene.ball_lit(mp, p.radius * 0.3, color::hex("#D4D4D8"), -mp.normalized(), if dark { 0.06 } else { 0.25 }, 0.2);
                    }
                    if self.labels {
                        let near = ctx.pointer.and_then(|m| scene.project(pos).map(|(s, _)| s.distance(m) < 40.0)).unwrap_or(false);
                        if near || ctx.pointer.is_none() && p.radius > 0.3 {
                            let lc = if near { t.foreground } else { t.muted_foreground };
                            scene.overlay(|s| s.label(&p.name, pos - vec3(0.0, p.radius + 0.12, 0.0), 11.0, lc, Align2::CENTER_TOP));
                        }
                    }
                }
            })
            .response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_box_hits() {
        let hit = ray_box(vec3(0.0, 0.5, 5.0), vec3(0.0, 0.0, -1.0), vec3(-0.5, 0.0, -0.5), vec3(0.5, 1.0, 0.5));
        assert!((hit.unwrap() - 4.5).abs() < 1e-5);
        assert!(ray_box(vec3(2.0, 0.5, 5.0), vec3(0.0, 0.0, -1.0), vec3(-0.5, 0.0, -0.5), vec3(0.5, 1.0, 0.5)).is_none());
    }
}
