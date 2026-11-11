use super::{rounded_quad, salt_or};
use crate::{vec3, Lights, Material, Mesh, OrbitControls, Quat, Transform, Vec3, Viewport3D, Wire};
use egui::{Color32, Response, Ui, Widget};
use glowkit_core::{color, ease, rng::hash01, Easing, Rng, Theme};
use std::f32::consts::{FRAC_PI_2, TAU};

// ---------------------------------------------------------------------------------------------
// TiltCubes
// ---------------------------------------------------------------------------------------------

/// A grid of 3D cubes that tilt towards the pointer; clicking sends a ripple wave through the
/// grid. When idle, an invisible cursor wanders so it stays alive.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_3d::TiltCubes::new().grid(10, 6).max_tilt(45.0).height(340.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TiltCubes {
    cols: usize,
    rows: usize,
    gap: f32,
    max_tilt: f32,
    radius: f32,
    face: Option<Color32>,
    ripple: Option<Color32>,
    border: Option<Color32>,
    auto: bool,
    height: f32,
    id_salt: Option<String>,
}

impl Default for TiltCubes {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Default)]
struct CubesState {
    tilt: Vec<(f32, f32)>,
    ripples: Vec<(f32, f32, f32)>,
    next_auto: f32,
}

impl TiltCubes {
    /// A 10 x 6 grid.
    pub fn new() -> Self {
        Self { cols: 10, rows: 6, gap: 0.14, max_tilt: 50.0, radius: 2.2, face: None, ripple: None, border: None, auto: true, height: 340.0, id_salt: None }
    }
    /// Grid size.
    pub fn grid(mut self, cols: usize, rows: usize) -> Self {
        self.cols = cols.clamp(1, 40);
        self.rows = rows.clamp(1, 40);
        self
    }
    /// Gap between cubes (fraction of a cube).
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g.max(0.0);
        self
    }
    /// Maximum tilt in degrees.
    pub fn max_tilt(mut self, deg: f32) -> Self {
        self.max_tilt = deg;
        self
    }
    /// Pointer influence radius in cubes.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.max(0.1);
        self
    }
    /// Face color (defaults from the theme).
    pub fn face_color(mut self, c: Color32) -> Self {
        self.face = Some(c);
        self
    }
    /// Ripple flash color (defaults to the theme accent).
    pub fn ripple_color(mut self, c: Color32) -> Self {
        self.ripple = Some(c);
        self
    }
    /// Edge color.
    pub fn border_color(mut self, c: Color32) -> Self {
        self.border = Some(c);
        self
    }
    /// Wander and ripple on their own while the pointer is away.
    pub fn auto_animate(mut self, on: bool) -> Self {
        self.auto = on;
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

impl Widget for TiltCubes {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let face = self.face.unwrap_or(if t.dark { color::mix(t.card, t.accent, 0.12) } else { color::mix(t.background, t.muted, 0.7) });
        let ripple = self.ripple.unwrap_or(t.accent);
        let border = self.border.unwrap_or(if t.dark { color::with_alpha(color::mix(t.border, t.accent, 0.35), 0.9) } else { color::with_alpha(color::mix(t.border, t.foreground, 0.25), 0.9) });
        let (cols, rows) = (self.cols, self.rows);
        let pitch = 1.0 + self.gap;
        let (gw, gh) = (cols as f32 * pitch, rows as f32 * pitch);
        let aspect = ui.available_width() / self.height;
        let fov: f32 = 32.0;
        let fit = (gh / 2.0).max(gw / 2.0 / aspect) * 1.25;
        let dist = fit / (fov.to_radians() / 2.0).tan() + 0.5;
        Viewport3D::new(salt_or(&self.id_salt, "tilt-cubes"))
            .height(self.height)
            .fov(fov)
            .controls(OrbitControls::new().distance(dist).yaw(0.0).pitch(0.0).rotate(false).zoom(false))
            .show(ui, |scene, ctx| {
                let mut cam = *scene.camera();
                cam.position = vec3(0.0, -dist * 0.12, dist);
                scene.set_camera(cam);
                scene.lights = Lights::default();
                let mesh = ctx.mesh("cube", || Mesh::cube(1.0));
                let center = |i: usize, j: usize| vec3((i as f32 + 0.5) * pitch - gw / 2.0, gh / 2.0 - (j as f32 + 0.5) * pitch, 0.0);
                let plane_hit = |p: egui::Pos2| -> Option<Vec3> {
                    let (o, d) = scene.ray(p);
                    if d.z.abs() < 1e-5 {
                        return None;
                    }
                    let k = -o.z / d.z;
                    (k > 0.0).then(|| o + d * k)
                };
                let now = ctx.time;
                let auto_target = vec3((now * 0.55).cos() * gw * 0.38, (now * 0.83).sin() * gh * 0.36, 0.0);
                let pointer = match ctx.pointer.and_then(plane_hit) {
                    Some(p) => Some(p),
                    None if self.auto => Some(auto_target),
                    None => None,
                };
                let click = ctx.click().and_then(plane_hit);
                let n = cols * rows;
                let dt = ctx.dt;
                let max_tilt = self.max_tilt.to_radians();
                let radius = self.radius;
                let tilts = ctx.state("cubes", CubesState::default, |s| {
                    s.tilt.resize(n, (0.0, 0.0));
                    if let Some(c) = click {
                        s.ripples.push((c.x, c.y, now));
                    }
                    if self.auto && ctx.pointer.is_none() && now > s.next_auto {
                        if s.next_auto > 0.0 {
                            s.ripples.push((auto_target.x, auto_target.y, now));
                        }
                        s.next_auto = now + 3.2;
                    }
                    s.ripples.retain(|r| now - r.2 < 3.0);
                    for j in 0..rows {
                        for i in 0..cols {
                            let c = center(i, j);
                            let (mut tx, mut ty) = (0.0, 0.0);
                            if let Some(p) = pointer {
                                let d = p - c;
                                let dist = (d.x * d.x + d.y * d.y).sqrt();
                                let inf = (-(dist * dist) / (2.0 * radius * radius)).exp();
                                // rotate so the front face looks towards the pointer
                                tx = -d.y.clamp(-radius, radius) / radius * max_tilt * inf;
                                ty = d.x.clamp(-radius, radius) / radius * max_tilt * inf;
                            }
                            let k = 1.0 - (-14.0 * dt).exp();
                            let e = &mut s.tilt[j * cols + i];
                            e.0 += (tx - e.0) * k;
                            e.1 += (ty - e.1) * k;
                        }
                    }
                    (s.tilt.clone(), s.ripples.clone())
                });
                let (tilt, ripples) = tilts;
                for j in 0..rows {
                    for i in 0..cols {
                        let c = center(i, j);
                        let (mut rx, mut ry) = tilt[j * cols + i];
                        let mut lift = 0.0;
                        let mut flash = 0.0f32;
                        for &(x, y, t0) in &ripples {
                            let age = now - t0;
                            let (dx, dy) = (c.x - x, c.y - y);
                            let r = (dx * dx + dy * dy).sqrt();
                            let front = age * 6.0;
                            let w = (-(r - front).powi(2) / 1.2).exp() * (1.0 - age / 3.0).max(0.0);
                            if r > 1e-3 {
                                rx += -dy / r * w * 0.9;
                                ry += dx / r * w * 0.9;
                            }
                            lift += w * 0.6;
                            flash = flash.max(w);
                        }
                        let q = Quat::from_axis_angle(Vec3::X, rx) * Quat::from_axis_angle(Vec3::Y, ry);
                        let tf = Transform::from_translation(c + vec3(0.0, 0.0, lift)).with_rotation(q);
                        let base = color::mix(face, ripple, (flash * 0.9).min(1.0));
                        let mat = Material::smooth(base).specular(0.25).shininess(24.0).wire(Wire::new(border).width(1.0).depth_fade(0.0)).emissive(flash * 0.25);
                        scene.group(c + vec3(0.0, 0.0, lift), |s| s.draw(&mesh, tf, &mat));
                    }
                }
            })
            .response
    }
}

// ---------------------------------------------------------------------------------------------
// PuzzleCube
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Cubie {
    pos: [i8; 3],
    rot: Quat,
    home: [i8; 3],
}

#[derive(Clone, Copy)]
struct Turn {
    axis: usize,
    layer: i8,
    dir: i8,
    start: f32,
}

#[derive(Clone)]
struct PuzzleState {
    cubies: Vec<Cubie>,
    turn: Option<Turn>,
    last_axis: usize,
    idle_since: f32,
    rng: Rng,
}

fn rot90(p: [i8; 3], axis: usize, dir: i8) -> [i8; 3] {
    let mut p = p;
    let times = if dir > 0 { 1 } else { 3 };
    for _ in 0..times {
        p = match axis {
            0 => [p[0], -p[2], p[1]],
            1 => [p[2], p[1], -p[0]],
            _ => [-p[1], p[0], p[2]],
        };
    }
    p
}

const AXES: [Vec3; 3] = [Vec3::X, Vec3::Y, Vec3::Z];

/// A 3x3x3 twisty puzzle cube that scrambles itself with smooth, eased random face turns.
/// Drag to orbit.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_3d::PuzzleCube::new().turn_time(0.45).height(360.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PuzzleCube {
    colors: [Color32; 6],
    body: Option<Color32>,
    turn_time: f32,
    pause: f32,
    height: f32,
    id_salt: Option<String>,
}

impl Default for PuzzleCube {
    fn default() -> Self {
        Self::new()
    }
}

impl PuzzleCube {
    /// Classic sticker colors.
    pub fn new() -> Self {
        Self {
            colors: [color::hex("#EF4444"), color::hex("#F97316"), color::hex("#F8FAFC"), color::hex("#FACC15"), color::hex("#22C55E"), color::hex("#3B82F6")],
            body: None,
            turn_time: 0.45,
            pause: 0.25,
            height: 360.0,
            id_salt: None,
        }
    }
    /// Sticker colors for +X, -X, +Y, -Y, +Z, -Z.
    pub fn colors(mut self, c: [Color32; 6]) -> Self {
        self.colors = c;
        self
    }
    /// Body (plastic) color.
    pub fn body_color(mut self, c: Color32) -> Self {
        self.body = Some(c);
        self
    }
    /// Seconds per face turn.
    pub fn turn_time(mut self, s: f32) -> Self {
        self.turn_time = s.max(0.05);
        self
    }
    /// Pause between turns in seconds.
    pub fn pause(mut self, s: f32) -> Self {
        self.pause = s.max(0.0);
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

impl Widget for PuzzleCube {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let body = self.body.unwrap_or(color::hex("#141418"));
        Viewport3D::new(salt_or(&self.id_salt, "puzzle-cube"))
            .height(self.height)
            .fov(34.0)
            .controls(OrbitControls::new().distance(9.0).pitch(0.5).yaw(0.75).auto_rotate(0.35).distance_range(5.0, 16.0))
            .show(ui, |scene, ctx| {
                let now = ctx.time;
                let (tt, pause) = (self.turn_time, self.pause);
                let st = ctx.state(
                    "puzzle",
                    || {
                        let mut cubies = Vec::new();
                        for x in -1..=1i8 {
                            for y in -1..=1i8 {
                                for z in -1..=1i8 {
                                    cubies.push(Cubie { pos: [x, y, z], rot: Quat::IDENTITY, home: [x, y, z] });
                                }
                            }
                        }
                        PuzzleState { cubies, turn: None, last_axis: 9, idle_since: now, rng: Rng::new(7) }
                    },
                    |s| {
                        if let Some(turn) = s.turn {
                            if now - turn.start >= tt {
                                let q = Quat::from_axis_angle(AXES[turn.axis], FRAC_PI_2 * turn.dir as f32);
                                for c in &mut s.cubies {
                                    if c.pos[turn.axis] == turn.layer {
                                        c.pos = rot90(c.pos, turn.axis, turn.dir);
                                        c.rot = (q * c.rot).normalized();
                                    }
                                }
                                s.turn = None;
                                s.idle_since = now;
                            }
                        }
                        if s.turn.is_none() && now - s.idle_since >= pause {
                            let mut axis = s.rng.below(3);
                            if axis == s.last_axis {
                                axis = (axis + 1 + s.rng.below(2)) % 3;
                            }
                            s.last_axis = axis;
                            let layer = s.rng.below(3) as i8 - 1;
                            let dir = if s.rng.bool(0.5) { 1 } else { -1 };
                            s.turn = Some(Turn { axis, layer, dir, start: now });
                        }
                        s.clone()
                    },
                );
                let body_mesh = ctx.mesh("body", || Mesh::cube(0.97));
                let sticker = ctx.mesh("sticker", || rounded_quad(0.8, 0.8, 0.13, 4));
                scene.shadow(vec3(0.0, -2.6, 0.0), 2.6, super::shadow_c(&t, 0.9));
                let anim = st.turn.map(|tr| {
                    let k = ease(Easing::InOutCubic, ((now - tr.start) / tt).clamp(0.0, 1.0));
                    (tr, Quat::from_axis_angle(AXES[tr.axis], FRAC_PI_2 * tr.dir as f32 * k))
                });
                let body_mat = Material::smooth(body).specular(0.5).shininess(48.0);
                for c in &st.cubies {
                    let layer_q = match anim {
                        Some((tr, q)) if c.pos[tr.axis] == tr.layer => q,
                        _ => Quat::IDENTITY,
                    };
                    let p = vec3(c.pos[0] as f32, c.pos[1] as f32, c.pos[2] as f32);
                    let world_rot = (layer_q * c.rot).normalized();
                    let center = layer_q.rotate(p);
                    let tf = Transform::from_translation(center).with_rotation(world_rot);
                    let m = tf.matrix();
                    scene.group(center, |s| {
                        s.draw(&body_mesh, m, &body_mat);
                        for (fi, dir) in [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z].into_iter().enumerate() {
                            let comp = c.home[fi / 2];
                            let want = if fi % 2 == 0 { 1 } else { -1 };
                            if comp != want {
                                continue;
                            }
                            let local = Transform::from_translation(dir * 0.492).with_rotation(Quat::from_rotation_arc(Vec3::Z, dir)).matrix();
                            let mat = Material::smooth(self.colors[fi]).specular(0.6).shininess(60.0);
                            s.draw(&sticker, m * local, &mat);
                        }
                    });
                }
            })
            .response
    }
}

// ---------------------------------------------------------------------------------------------
// VoxelText
// ---------------------------------------------------------------------------------------------

/// 5x7 pixel font: each row is 5 bits (MSB = left column).
fn glyph(c: char) -> [u8; 7] {
    match c.to_ascii_uppercase() {
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' => [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' => [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        'J' => [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x11, 0x19, 0x15, 0x13, 0x11, 0x11],
        'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        '!' => [0x04, 0x04, 0x04, 0x04, 0x04, 0x00, 0x04],
        '?' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04],
        '-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
        '*' | '♥' => [0x00, 0x0A, 0x1F, 0x1F, 0x0E, 0x04, 0x00],
        _ => [0; 7],
    }
}

/// Voxel coordinates `(column, row)` for `text` in the built-in 5x7 pixel font (6 columns per
/// character, row 0 at the top).
pub fn voxel_font(text: &str) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for (ci, ch) in text.chars().enumerate() {
        let g = glyph(ch);
        for (row, bits) in g.iter().enumerate() {
            for col in 0..5 {
                if bits & (0x10 >> col) != 0 {
                    out.push((ci as i32 * 6 + col, row as i32));
                }
            }
        }
    }
    out
}

/// Text built from glossy 3D voxel cubes (a tiny 5x7 pixel font), rippling in a wave, gently
/// swaying, and periodically bursting apart and snapping back together. Drag to orbit.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_3d::VoxelText::new("HELLO").height(300.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct VoxelText {
    text: String,
    colors: Option<Vec<Color32>>,
    wave: f32,
    burst: bool,
    height: f32,
    id_salt: Option<String>,
}

impl VoxelText {
    /// Voxel text (A-Z, 0-9 and `! ? - . *`).
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), colors: None, wave: 1.0, burst: true, height: 300.0, id_salt: None }
    }
    /// Gradient across the text (defaults to the theme gradient).
    pub fn colors(mut self, c: &[Color32]) -> Self {
        self.colors = Some(c.to_vec());
        self
    }
    /// Wave amplitude (0 = still).
    pub fn wave(mut self, w: f32) -> Self {
        self.wave = w;
        self
    }
    /// Periodically burst apart and reassemble.
    pub fn burst(mut self, on: bool) -> Self {
        self.burst = on;
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

impl Widget for VoxelText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let stops = self.colors.clone().unwrap_or_else(|| t.gradient.to_vec());
        let vox = voxel_font(&self.text);
        let ncols = (self.text.chars().count() as i32 * 6 - 1).max(1) as f32;
        let aspect = ui.available_width() / self.height;
        let fov: f32 = 35.0;
        let half_w = ncols / 2.0 * 1.15;
        let dist = (half_w / aspect).max(5.5) / (fov.to_radians() / 2.0).tan();
        Viewport3D::new(salt_or(&self.id_salt, "voxel-text"))
            .height(self.height)
            .fov(fov)
            .controls(OrbitControls::new().distance(dist).yaw(0.0).pitch(0.12).distance_range(dist * 0.5, dist * 2.0))
            .show(ui, |scene, ctx| {
                let time = ctx.time;
                let cube = ctx.mesh("vox", || Mesh::cube(0.9));
                let cycle = 7.0;
                let ph = time % cycle;
                let burst = if !self.burst || ph < 5.0 {
                    0.0
                } else if ph < 5.7 {
                    ease(Easing::OutCubic, (ph - 5.0) / 0.7)
                } else {
                    1.0 - ease(Easing::OutBack, ((ph - 5.7) / 1.1).clamp(0.0, 1.0))
                };
                let sway = Quat::from_axis_angle(Vec3::Y, (time * 0.45).sin() * 0.35) * Quat::from_axis_angle(Vec3::X, (time * 0.6).sin() * 0.08);
                scene.lights = Lights::default();
                for (k, &(cx, cy)) in vox.iter().enumerate() {
                    let x = cx as f32 - ncols / 2.0 + 0.5;
                    let y = 3.0 - cy as f32;
                    let wave = (time * 2.4 - x * 0.35).sin() * 0.35 * self.wave;
                    let h = |s: u64| hash01(k as u64 * 31 + s) - 0.5;
                    let scatter = vec3(h(1) * 9.0, h(2) * 7.0, h(3) * 9.0 + 2.0);
                    let p = vec3(x, y, wave) + scatter * burst;
                    let spin = Quat::from_axis_angle(vec3(h(4), h(5), h(6)), burst * TAU * 0.6 + wave * 0.4);
                    let world = sway.rotate(p);
                    let tc = (x / ncols + 0.5).clamp(0.0, 1.0);
                    let c = color::gradient(&stops, tc);
                    let mat = Material::smooth(c).specular(0.6).shininess(36.0).rim(0.25).rim_color(color::mix(c, Color32::WHITE, 0.5));
                    let tf = Transform::from_translation(world).with_rotation(sway * spin);
                    scene.group(world, |s| s.draw(&cube, tf, &mat));
                }
                let span = ncols * 0.5;
                scene.underlay(|s| {
                    for i in 0..5 {
                        let x = (i as f32 / 4.0 - 0.5) * span * 1.6;
                        s.shadow(vec3(x, -5.2, 0.0), span * 0.45, super::shadow_c(&t, 0.45));
                    }
                });
            })
            .response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rot90_matches_quat() {
        for axis in 0..3 {
            for dir in [-1i8, 1] {
                let p = [1i8, -1, 0];
                let r = rot90(p, axis, dir);
                let q = Quat::from_axis_angle(AXES[axis], FRAC_PI_2 * dir as f32).rotate(vec3(1.0, -1.0, 0.0));
                assert!((q - vec3(r[0] as f32, r[1] as f32, r[2] as f32)).length() < 1e-4);
            }
        }
    }

    #[test]
    fn font_has_pixels() {
        assert_eq!(voxel_font("I").len(), 11);
        assert!(voxel_font(" ").is_empty());
        assert_eq!(voxel_font("AB").iter().map(|v| v.0).max(), Some(10));
    }
}
