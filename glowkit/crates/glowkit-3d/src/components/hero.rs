use super::{glow_c, salt_or};
use crate::{vec3, Background, Camera, Material, Mesh, Quat, Shading, Transform, Vec3, Viewport3D, Wire};
use egui::{Align2, Color32, Response, Ui, Widget};
use glowkit_core::{color, Theme};
use std::f32::consts::{PI, TAU};

/// A Spline-style hero: glossy gradient shapes (torus, sphere, knot, gems, capsule...) float
/// and spin around a headline, some in front of it and some behind, with a subtle parallax that
/// follows the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_3d::FloatingShapes::new("Build in 3D", "Pure Rust, no GPU pipeline").height(380.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct FloatingShapes {
    title: String,
    subtitle: String,
    colors: Option<[Color32; 3]>,
    title_color: Option<Color32>,
    height: f32,
    id_salt: Option<String>,
}

impl FloatingShapes {
    /// A hero with a headline and a subtitle (either may be empty).
    pub fn new(title: impl Into<String>, subtitle: impl Into<String>) -> Self {
        Self { title: title.into(), subtitle: subtitle.into(), colors: None, title_color: None, height: 380.0, id_salt: None }
    }
    /// Brand gradient for the shapes (defaults to the theme gradient).
    pub fn colors(mut self, c: [Color32; 3]) -> Self {
        self.colors = Some(c);
        self
    }
    /// Headline color (defaults to the theme foreground).
    pub fn title_color(mut self, c: Color32) -> Self {
        self.title_color = Some(c);
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

impl Widget for FloatingShapes {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let g = self.colors.unwrap_or(t.gradient);
        let aspect = ui.available_width() / self.height;
        let bg = if t.dark {
            Background::Radial(color::mix(t.card, g[0], 0.16), t.card)
        } else {
            Background::Radial(color::mix(t.background, g[1], 0.07), color::mix(t.background, g[0], 0.04))
        };
        Viewport3D::new(salt_or(&self.id_salt, "floating-shapes"))
            .height(self.height)
            .background(bg)
            .camera(Camera::default())
            .show(ui, |scene, ctx| {
                let time = ctx.time;
                let target = ctx.pointer_ndc.unwrap_or(egui::vec2((time * 0.3).sin() * 0.4, (time * 0.23).cos() * 0.3));
                let par = ctx.state("parallax", || egui::Vec2::ZERO, |p| {
                    *p += (target - *p) * (1.0 - (-3.0 * ctx.dt).exp());
                    *p
                });
                let dist = 9.0;
                let fov = 38.0f32;
                scene.set_camera(Camera::looking_at(vec3(par.x * 0.9, par.y * 0.6, dist), Vec3::ZERO).fov_degrees(fov));
                let half_h = dist * (fov.to_radians() / 2.0).tan();
                let half_w = half_h * aspect;
                let sx = |f: f32| f * half_w * 0.86;
                let sy = |f: f32| f * half_h * 0.86;
                let shades = |i: usize| -> [Color32; 3] {
                    let a = g[i % 3];
                    let b = g[(i + 1) % 3];
                    [color::shade(color::mix(a, b, 0.2), -0.28), color::mix(a, b, 0.35), color::mix(color::mix(a, b, 0.6), Color32::WHITE, 0.45)]
                };
                struct S {
                    mesh: &'static str,
                    x: f32,
                    y: f32,
                    z: f32,
                    scale: f32,
                    speed: f32,
                    phase: f32,
                }
                let shapes = [
                    S { mesh: "torus", x: -0.78, y: 0.52, z: -1.0, scale: 0.85, speed: 0.6, phase: 0.0 },
                    S { mesh: "sphere", x: 0.8, y: -0.45, z: 0.4, scale: 0.75, speed: 0.4, phase: 1.3 },
                    S { mesh: "ico", x: -0.62, y: -0.62, z: 1.0, scale: 0.55, speed: 0.8, phase: 2.1 },
                    S { mesh: "knot", x: 0.66, y: 0.58, z: -1.6, scale: 0.7, speed: 0.5, phase: 3.4 },
                    S { mesh: "capsule", x: -0.1, y: 0.86, z: -2.8, scale: 0.55, speed: 0.7, phase: 4.2 },
                    S { mesh: "cone", x: 0.26, y: -0.86, z: 1.6, scale: 0.4, speed: 0.9, phase: 5.0 },
                    S { mesh: "octa", x: 0.98, y: 0.12, z: -2.2, scale: 0.42, speed: 1.0, phase: 0.7 },
                    S { mesh: "dodeca", x: -1.0, y: -0.05, z: -1.8, scale: 0.42, speed: 0.75, phase: 2.8 },
                ];
                for (i, s) in shapes.iter().enumerate() {
                    let mesh = ctx.mesh(s.mesh, || match s.mesh {
                        "torus" => Mesh::torus(0.8, 0.32, 40, 16),
                        "sphere" => Mesh::icosphere(0.85, 3),
                        "ico" => Mesh::icosahedron(0.9),
                        "knot" => Mesh::torus_knot(2, 3, 0.85, 0.24, 140, 12),
                        "capsule" => Mesh::capsule(0.38, 0.9, 24, 6),
                        "cone" => Mesh::cone(0.6, 1.2, 28),
                        "octa" => Mesh::octahedron(0.9),
                        _ => Mesh::dodecahedron(0.9),
                    });
                    let bob = (time * 0.9 + s.phase).sin() * 0.16;
                    let p = vec3(sx(s.x), sy(s.y) + bob, s.z);
                    let q = Quat::from_axis_angle(vec3(0.3, 1.0, 0.2), time * s.speed + s.phase) * Quat::from_axis_angle(Vec3::X, s.phase);
                    let mut mat = Material::matcap(shades(i)).shininess(22.0).specular(0.8).rim(0.3).rim_color(color::mix(g[(i + 2) % 3], Color32::WHITE, 0.5));
                    if matches!(s.mesh, "ico" | "octa" | "dodeca") {
                        mat = Material::flat(color::mix(g[i % 3], g[(i + 1) % 3], 0.4)).specular(0.6).shininess(30.0).rim(0.25);
                    }
                    if s.mesh == "cone" {
                        mat.shading = Shading::Toon { bands: 3 };
                        mat.color = g[(i + 1) % 3];
                    }
                    scene.draw(&mesh, Transform::from_translation(p).with_rotation(q).with_scale(s.scale), &mat);
                }
                let tc = self.title_color.unwrap_or(t.foreground);
                let title_h = (half_h * 0.24).min(half_w * 2.4 / self.title.chars().count().max(4) as f32);
                if !self.title.is_empty() {
                    scene.underlay(|_| {});
                    scene.text(&self.title, vec3(0.0, title_h * 0.35, 0.0), Vec3::X, Vec3::Y, title_h, tc, Align2::CENTER_CENTER);
                }
                if !self.subtitle.is_empty() {
                    scene.text(&self.subtitle, vec3(0.0, -title_h * 0.62, 0.0), Vec3::X, Vec3::Y, title_h * 0.36, t.muted_foreground, Align2::CENTER_CENTER);
                }
                // soft glow behind the headline
                scene.underlay(|s| s.glow_point(vec3(0.0, 0.0, -3.0), half_h * 1.2, glow_c(&t, g[1], if t.dark { 0.12 } else { 0.1 })));
            })
            .response
    }
}

/// A torus knot rendered as glowing neon: a bright core line wrapped in layered bloom, with
/// light pulses racing along it and a faint wireframe tube around it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_3d::GlowKnot::new().knot(3, 5).height(360.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct GlowKnot {
    p: u32,
    q: u32,
    colors: Option<Vec<Color32>>,
    wireframe: bool,
    height: f32,
    id_salt: Option<String>,
}

impl Default for GlowKnot {
    fn default() -> Self {
        Self::new()
    }
}

impl GlowKnot {
    /// A (2, 3) trefoil knot.
    pub fn new() -> Self {
        Self { p: 2, q: 3, colors: None, wireframe: true, height: 360.0, id_salt: None }
    }
    /// Knot parameters (`p` windings around the axis, `q` through the hole).
    pub fn knot(mut self, p: u32, q: u32) -> Self {
        self.p = p.max(1);
        self.q = q.max(1);
        self
    }
    /// Neon colors cycling along the knot (defaults to the theme gradient).
    pub fn colors(mut self, c: &[Color32]) -> Self {
        self.colors = Some(c.to_vec());
        self
    }
    /// Show the faint wireframe tube.
    pub fn wireframe(mut self, on: bool) -> Self {
        self.wireframe = on;
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

impl Widget for GlowKnot {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let dark = t.dark;
        let stops = self.colors.clone().unwrap_or_else(|| t.gradient.to_vec());
        let bg = if dark { Background::Radial(color::mix(t.card, stops[0], 0.1), color::mix(t.card, Color32::BLACK, 0.3)) } else { Background::None };
        Viewport3D::new(salt_or(&self.id_salt, "glow-knot"))
            .height(self.height)
            .background(bg)
            .fov(36.0)
            .controls(crate::OrbitControls::new().distance(5.6).pitch(0.2).auto_rotate(0.3).distance_range(3.0, 12.0))
            .show(ui, |scene, ctx| {
                let time = ctx.time;
                let (p, q) = (self.p, self.q);
                let n = 260;
                let rot = Quat::from_axis_angle(Vec3::X, 1.1 + (time * 0.3).sin() * 0.2) * Quat::from_axis_angle(Vec3::Z, time * 0.25);
                let path: Vec<Vec3> = (0..n)
                    .map(|i| {
                        let u = i as f32 / n as f32 * TAU;
                        let (pt, qt) = (p as f32 * u, q as f32 * u);
                        let r = 2.0 + qt.cos();
                        rot.rotate(vec3(r * pt.cos(), r * pt.sin(), -qt.sin()) * 0.55)
                    })
                    .collect();
                if self.wireframe {
                    let mesh = ctx.mesh(("tube", p, q), || Mesh::torus_knot(p, q, 1.65, 0.2, 140, 6));
                    let wc = if dark { color::with_alpha(stops[0], 0.35) } else { color::with_alpha(stops[0], 0.3) };
                    let mat = Material::wireframe(wc).wire(Wire::new(wc).width(0.8).depth_fade(0.85)).additive(dark);
                    // torus_knot uses (x, -sin, z); rotate it into the path's frame
                    let align = Quat::from_axis_angle(Vec3::X, -PI / 2.0);
                    scene.draw(&mesh, Transform::IDENTITY.with_rotation(rot * align), &mat);
                }
                let col = |u: f32| color::gradient_cyclic(&stops, u - time * 0.08);
                for i in 0..n {
                    let (a, b) = (path[i], path[(i + 1) % n]);
                    let u = i as f32 / n as f32;
                    let c = col(u);
                    if dark {
                        scene.line_world(a, b, 0.22, crate::additive(c.gamma_multiply(0.07)), crate::additive(c.gamma_multiply(0.07)));
                        scene.line_world(a, b, 0.08, crate::additive(c.gamma_multiply(0.3)), crate::additive(c.gamma_multiply(0.3)));
                        let core = color::mix(c, Color32::WHITE, 0.55);
                        scene.line_world(a, b, 0.025, crate::additive(core), crate::additive(core));
                    } else {
                        scene.line_world(a, b, 0.2, color::with_alpha(c, 0.08), color::with_alpha(c, 0.08));
                        scene.line_world(a, b, 0.07, color::with_alpha(c, 0.5), color::with_alpha(c, 0.5));
                        let core = color::shade(c, -0.08);
                        scene.line_world(a, b, 0.025, core, core);
                    }
                }
                // racing pulses
                for k in 0..4 {
                    let u = (time * 0.11 + k as f32 / 4.0).fract();
                    let i = (u * n as f32) as usize % n;
                    let c = col(u);
                    for tail in 0..14 {
                        let j = (i + n - tail) % n;
                        let f = 1.0 - tail as f32 / 14.0;
                        scene.glow_point(path[j], 0.12 * f + 0.04, glow_c(&t, color::mix(c, Color32::WHITE, 0.5), 0.5 * f));
                    }
                }
                scene.underlay(|s| s.glow_point(Vec3::ZERO, 2.6, glow_c(&t, stops[1 % stops.len()], if dark { 0.12 } else { 0.06 })));
            })
            .response
    }
}
