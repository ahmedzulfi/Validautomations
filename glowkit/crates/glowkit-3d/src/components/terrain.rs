use super::salt_or;
use crate::{vec3, Background, Camera, Fog, Lights, Material, Mesh, Vec3, Viewport3D, Wire};
use egui::{Color32, Response, Ui, Widget};
use glowkit_core::{color, noise, rng::hash01, Theme};
use std::f32::consts::TAU;

/// Visual style of [`WaveTerrain`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TerrainStyle {
    /// Retro synthwave: dark ground, neon grid, striped sun on the horizon.
    #[default]
    Synthwave,
    /// Glowing height-colored wireframe lines only.
    Wireframe,
    /// Flat-shaded low-poly hills with fog.
    Solid,
}

/// An endless animated noise terrain the camera flies over: mountains on both sides of a flat
/// valley, rolling towards you.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_3d::{WaveTerrain, TerrainStyle};
/// ui.add(WaveTerrain::new().style(TerrainStyle::Synthwave).speed(1.0).height(340.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct WaveTerrain {
    style: TerrainStyle,
    speed: f32,
    amplitude: f32,
    cols: usize,
    rows: usize,
    colors: Option<[Color32; 3]>,
    height: f32,
    id_salt: Option<String>,
}

impl Default for WaveTerrain {
    fn default() -> Self {
        Self::new()
    }
}

impl WaveTerrain {
    /// Default synthwave terrain.
    pub fn new() -> Self {
        Self { style: TerrainStyle::Synthwave, speed: 1.0, amplitude: 1.0, cols: 36, rows: 30, colors: None, height: 340.0, id_salt: None }
    }
    /// Visual style.
    pub fn style(mut self, s: TerrainStyle) -> Self {
        self.style = s;
        self
    }
    /// Flight speed multiplier.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Mountain height multiplier.
    pub fn amplitude(mut self, a: f32) -> Self {
        self.amplitude = a;
        self
    }
    /// Grid resolution (columns across, rows into the distance).
    pub fn resolution(mut self, cols: usize, rows: usize) -> Self {
        self.cols = cols.clamp(4, 160);
        self.rows = rows.clamp(4, 160);
        self
    }
    /// Colors: (low, high, sky/sun). Defaults derive from the theme.
    pub fn colors(mut self, c: [Color32; 3]) -> Self {
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

fn terrain_h(x: f32, z: f32, amp: f32) -> f32 {
    let valley = glowkit_core::anim::smoothstep(0.6, 3.2, x.abs());
    let n = noise::fbm3(x * 0.32, z * 0.32, 0.7, 4) * 0.5 + 0.5;
    (n * n * 2.6 + 0.08 * (x * 1.3).sin()) * valley * amp
}

impl Widget for WaveTerrain {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let dark = t.dark;
        let [low, high, sun] = self.colors.unwrap_or(match self.style {
            TerrainStyle::Synthwave => [color::hex("#FF2BD6"), color::hex("#22D3EE"), color::hex("#FFB547")],
            _ => [t.gradient[0], t.gradient[1], t.gradient[2]],
        });
        let synth = self.style == TerrainStyle::Synthwave;
        let (sky_top, horizon) = if synth {
            if dark {
                (color::hex("#07021A"), color::hex("#3B0A4F"))
            } else {
                (color::hex("#F5E8FF"), color::hex("#FFD3EC"))
            }
        } else if dark {
            (t.card, color::mix(t.card, low, 0.18))
        } else {
            (color::mix(t.background, high, 0.06), color::mix(t.background, low, 0.14))
        };
        Viewport3D::new(salt_or(&self.id_salt, "wave-terrain"))
            .height(self.height)
            .background(Background::Vertical(sky_top, horizon))
            .camera(Camera::looking_at(vec3(0.0, 1.6, 4.0), vec3(0.0, 0.9, -6.0)).fov_degrees(62.0))
            .show(ui, |scene, ctx| {
                let travel = ctx.time * self.speed * 2.2;
                let sway = (ctx.time * 0.35).sin() * 0.35 + ctx.pointer_ndc.map(|p| p.x * 0.6).unwrap_or(0.0);
                let mut cam = Camera::looking_at(vec3(sway, 1.55, 4.0), vec3(sway * 0.4, 0.95, -6.0)).fov_degrees(62.0);
                cam.up = vec3((ctx.time * 0.35).cos() * -0.04, 1.0, 0.0);
                scene.set_camera(cam);
                let (cols, rows) = (self.cols, self.rows);
                let (w, depth) = (16.0f32, 26.0f32);
                let cell = depth / rows as f32;
                let shift = travel.rem_euclid(cell);
                let base_row = (travel / cell).floor();
                let amp = self.amplitude;
                let pos = |i: usize, j: usize| -> Vec3 {
                    let x = (i as f32 / cols as f32 - 0.5) * w;
                    let z = 4.5 - j as f32 * cell + shift;
                    let wz = -(j as f32 + base_row) * cell; // world-stable coordinate for the noise
                    vec3(x, terrain_h(x, wz, amp), z)
                };
                scene.fog = Some(if synth || dark { Fog::new(horizon, 9.0, depth - 2.0) } else { Fog::fade(10.0, depth - 2.0) });
                // sun / moon on the horizon
                let sun_c = vec3(0.0, 3.4, -24.0);
                scene.underlay(|s| {
                    if synth {
                        let r = 5.0;
                        let strips = 14;
                        for k in 0..strips {
                            let (y0, y1) = (r - 2.0 * r * k as f32 / strips as f32, r - 2.0 * r * (k + 1) as f32 / strips as f32);
                            let gap = if y1 < 0.6 { ((0.6 - y1) / (2.0 * r) * 0.9).min(0.75) * (y0 - y1) } else { 0.0 };
                            let y1 = y1 + gap;
                            let half = |y: f32| (r * r - y * y).max(0.0).sqrt();
                            let col = |y: f32| color::mix(sun, low, ((r - y) / (2.0 * r)).clamp(0.0, 1.0));
                            let segs = 6;
                            for q in 0..segs {
                                let (a0, a1) = (q as f32 / segs as f32 * 2.0 - 1.0, (q + 1) as f32 / segs as f32 * 2.0 - 1.0);
                                let p = |y: f32, a: f32| sun_c + vec3(half(y) * a, y, 0.0);
                                let (c0, c1) = (col(y0), col(y1));
                                s.triangle([p(y0, a0), p(y1, a0), p(y1, a1)], [c0, c1, c1]);
                                s.triangle([p(y0, a0), p(y1, a1), p(y0, a1)], [c0, c1, c0]);
                            }
                        }
                        s.glow_point(sun_c, 9.0, super::glow_c(&t, sun, if dark { 0.35 } else { 0.25 }));
                        if dark {
                            for i in 0..90 {
                                let h = |k: u64| hash01(i * 17 + k);
                                let p = vec3((h(1) - 0.5) * 70.0, 2.0 + h(2) * 18.0, -30.0);
                                let tw = 0.5 + 0.5 * (ctx.time * (1.0 + h(3) * 2.0) + h(4) * TAU).sin();
                                s.point_px(p, 0.6 + h(5) * 0.7, crate::additive(Color32::WHITE.gamma_multiply(0.2 + 0.5 * tw)));
                            }
                        }
                    } else {
                        s.glow_point(sun_c, 7.0, super::glow_c(&t, sun, 0.22));
                        s.ball_lit(sun_c, 1.6, color::mix(sun, Color32::WHITE, 0.4), vec3(0.0, 0.0, 1.0), 1.0, 0.0);
                    }
                });
                match self.style {
                    TerrainStyle::Wireframe => {
                        let lc = |p: Vec3| -> Color32 {
                            let k = (p.y / (2.2 * amp.max(0.1))).clamp(0.0, 1.0);
                            let c = color::mix(low, high, k);
                            let fade = (1.0 - (4.5 - p.z) / depth).clamp(0.0, 1.0);
                            if dark {
                                crate::additive(c.gamma_multiply(0.35 + 0.65 * fade))
                            } else {
                                color::with_alpha(color::shade(c, -0.1), 0.25 + 0.6 * fade)
                            }
                        };
                        for j in 0..=rows {
                            for i in 0..=cols {
                                let p = pos(i, j);
                                if i < cols {
                                    let q = pos(i + 1, j);
                                    scene.line_gradient(p, q, 1.2, lc(p), lc(q));
                                }
                                if j < rows {
                                    let q = pos(i, j + 1);
                                    scene.line_gradient(p, q, 1.2, lc(p), lc(q));
                                }
                            }
                        }
                    }
                    TerrainStyle::Synthwave | TerrainStyle::Solid => {
                        let mut m = Mesh::new();
                        let mut colors = Vec::with_capacity((cols + 1) * (rows + 1));
                        for j in 0..=rows {
                            for i in 0..=cols {
                                let p = pos(i, j);
                                m.vertex(p, Vec3::Y);
                                let k = (p.y / (2.4 * amp.max(0.1))).clamp(0.0, 1.0);
                                colors.push(if synth { Color32::WHITE } else { color::gradient(&[color::mix(low, t.card, 0.2), low, high], k) });
                            }
                        }
                        m.colors = colors;
                        let wv = (cols + 1) as u32;
                        for j in 0..rows as u32 {
                            for i in 0..cols as u32 {
                                let a = j * wv + i;
                                m.quad(a, a + 1, a + wv + 1, a + wv);
                            }
                        }
                        m.compute_normals();
                        // compute_normals follows the winding; make sure up is up
                        if m.normals.first().map(|n| n.y < 0.0).unwrap_or(false) {
                            for n in &mut m.normals {
                                *n = -*n;
                            }
                        }
                        let mesh = m.finish();
                        if synth {
                            scene.lights = Lights::ambient(1.0);
                            let ground = if dark { color::hex("#12021F") } else { color::hex("#FBEFFF") };
                            let grid = if dark { color::hex("#FF3BD4") } else { color::hex("#C026D3") };
                            let mat = Material::flat(ground).specular(0.0).double_sided(true).wire(Wire::new(grid).width(1.3).depth_fade(0.0));
                            scene.draw(&mesh, crate::Transform::IDENTITY, &mat);
                        } else {
                            scene.lights = Lights::default().with_directional(vec3(0.0, 0.3, -1.0), sun, 0.35);
                            let mat = Material::flat(Color32::WHITE).specular(0.1).double_sided(true).wire(Wire::new(color::with_alpha(if dark { Color32::WHITE } else { t.foreground }, 0.12)).width(0.8).depth_fade(0.5));
                            scene.draw(&mesh, crate::Transform::IDENTITY, &mat);
                        }
                    }
                }
            })
            .response
    }
}
