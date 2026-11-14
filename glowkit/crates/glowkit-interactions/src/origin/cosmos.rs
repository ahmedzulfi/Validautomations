//! Particle "cosmos" pieces: a strand vortex, a point-cloud ringed planet, a black-hole accretion
//! disc and a glass orb with a galaxy inside.

use super::common::{h01, lerp_rgb, lum, pointer_or_ghost, ramp, strip, Cam, Dots, V3, v3};
use egui::epaint::Mesh;
use egui::{pos2, vec2, Color32, Pos2, Rect, Response, Sense, Shape, Ui, Vec2, Widget};
use glowkit_core::color::{hex, with_alpha};
use glowkit_core::noise::fbm3;
use glowkit_core::{paint, Theme};
use std::f32::consts::{PI, TAU};

fn alloc(ui: &mut Ui, size: Option<Vec2>, h: f32, sense: Sense) -> (Rect, Response) {
    let s = glowkit_core::desired_size(ui, size, h);
    ui.allocate_exact_size(s, sense)
}

/// Inertial drag-to-spin state shared by the 3D pieces.
#[derive(Clone, Copy, Debug, Default)]
struct Spin {
    yaw: f32,
    pitch: f32,
    vel: f32,
    pvel: f32,
}

fn spin_update(ui: &Ui, response: &Response, auto: f32, drag_k: f32) -> Spin {
    let dt = glowkit_core::dt(ui);
    let d = response.drag_delta();
    let dragging = response.dragged();
    glowkit_core::with_state(ui, response.id.with("origin_spin"), Spin::default, |s| {
        if dragging {
            s.vel = d.x * drag_k / dt.max(1e-3) * 0.6;
            s.pvel = d.y * drag_k / dt.max(1e-3) * 0.6;
        } else {
            s.vel += (auto - s.vel) * (1.0 - (-dt * 1.4).exp());
            s.pvel -= s.pvel * (1.0 - (-dt * 4.0).exp());
            s.pitch -= s.pitch * (1.0 - (-dt * 1.5).exp());
        }
        s.yaw += s.vel * dt;
        s.pitch = (s.pitch + s.pvel * dt).clamp(-0.6, 0.6);
        *s
    })
}

// ---------------------------------------------------------------------------------------------
// Tornado
// ---------------------------------------------------------------------------------------------

/// A strand vortex: hundreds of fine helical lines pinch into a narrow waist and flare out into a
/// swirling floor disc, with flickering motes riding up the funnel and comets racing along the
/// strands. The pointer pushes strands aside.
///
/// Light themes draw it in ink on paper; dark themes in additive glowing light.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::Tornado::new().lines(180).twist(3.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Tornado {
    size: Option<Vec2>,
    lines: usize,
    dots: usize,
    comets: usize,
    twist: f32,
    waist: f32,
    speed: f32,
    color: Option<Color32>,
    background: Option<Color32>,
    ghost: bool,
}

impl Default for Tornado {
    fn default() -> Self {
        Self::new()
    }
}

impl Tornado {
    /// A vortex with default settings.
    pub fn new() -> Self {
        Self {
            size: None,
            lines: 160,
            dots: 700,
            comets: 6,
            twist: 3.0,
            waist: 0.24,
            speed: 1.0,
            color: None,
            background: None,
            ghost: true,
        }
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Number of strands (20..400).
    pub fn lines(mut self, n: usize) -> Self {
        self.lines = n.clamp(20, 400);
        self
    }
    /// Number of drifting motes (0..2000).
    pub fn dots(mut self, n: usize) -> Self {
        self.dots = n.min(2000);
        self
    }
    /// Number of comets racing along strands.
    pub fn comets(mut self, n: usize) -> Self {
        self.comets = n.min(24);
        self
    }
    /// How far each strand winds around the axis (turns-ish, 0..8).
    pub fn twist(mut self, t: f32) -> Self {
        self.twist = t.clamp(0.0, 8.0);
        self
    }
    /// Waist radius relative to the top radius (0.05..0.8).
    pub fn waist(mut self, w: f32) -> Self {
        self.waist = w.clamp(0.05, 0.8);
        self
    }
    /// Spin speed multiplier.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Strand color (default: white glow on dark themes, ink on light ones).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Background fill (default: the theme background).
    pub fn background(mut self, c: Color32) -> Self {
        self.background = Some(c);
        self
    }
    /// Show a drifting ghost pointer while idle (default on).
    pub fn ghost(mut self, on: bool) -> Self {
        self.ghost = on;
        self
    }

    /// Paint the vortex into `rect`; `pointer` repels strands.
    pub fn paint(&self, ui: &Ui, rect: Rect, pointer: Option<Pos2>) {
        if !ui.is_rect_visible(rect) {
            return;
        }
        glowkit_core::animate(ui);
        let t = Theme::of(ui);
        let time = glowkit_core::time(ui) * self.speed;
        let painter = ui.painter_at(rect);
        let bg = self.background.unwrap_or(t.background);
        painter.rect_filled(rect, 0.0, bg);
        let add = t.dark;
        let col = self.color.unwrap_or(if add { Color32::WHITE } else { t.foreground });
        let unit = rect.height().min(rect.width() * 0.75);
        let cam = Cam { center: pos2(rect.center().x, rect.center().y + unit * 0.1), focal: unit * 2.25, dist: 6.0 };
        let pitch = 0.2;
        let rt = 1.7;
        let rw = 1.25 * self.waist;
        let rf = 3.6;
        let (y0, ytop) = (-0.3, 1.25);
        let surface = |u: f32, phase: f32| -> (V3, f32) {
            let (r, y, floor) = if u < 0.5 {
                let k = u / 0.5;
                let q = 1.0 - k;
                (rw + (rf - rw) * q * q * q, y0 - (y0 + 1.0) * q.powf(0.55), q * q)
            } else {
                let k = (u - 0.5) / 0.5;
                (rw + (rt - rw) * k * k, y0 + (ytop - y0) * k, 0.0)
            };
            let th = phase + self.twist * u * PI + floor * 4.5 + time * 0.55 * (1.0 + 0.6 * rw / r);
            (v3(r * th.cos(), y, r * th.sin()), r)
        };
        let view = |p: V3| p.rot_x(pitch);
        let push = |p: Pos2| -> Pos2 {
            if let Some(m) = pointer {
                let d = p - m;
                let l = d.length();
                let rad = unit * 0.16;
                if l < rad && l > 1e-3 {
                    let f = (1.0 - l / rad).powi(2) * rad * 0.22;
                    return p + d / l * f;
                }
            }
            p
        };
        let seg = 64;
        let mut mesh = Mesh::default();
        mesh.reserve_vertices(self.lines * (seg + 1) * 4);
        let base_a = if add { 0.32 } else { 0.30 };
        let density = (160.0 / self.lines as f32).sqrt().clamp(0.5, 1.6);
        let mut pts = Vec::with_capacity(seg + 1);
        let mut alphas = Vec::with_capacity(seg + 1);
        for i in 0..self.lines {
            let phase = i as f32 / self.lines as f32 * TAU + h01(i as u64 * 31) * 0.05;
            pts.clear();
            alphas.clear();
            for k in 0..=seg {
                let u = k as f32 / seg as f32;
                let (p, _r) = surface(u, phase);
                let v = view(p);
                if let Some((s, _, d)) = cam.project(v) {
                    pts.push(push(s));
                    // back strands dimmer; fade in from the far floor and out at the top rim
                    let depth = ((d - cam.dist) / 3.0).clamp(-1.0, 1.0);
                    let edge = (u * 6.0).min(1.0) * ((1.0 - u) * 10.0).min(1.0);
                    alphas.push(base_a * density * (0.55 - depth * 0.45) * edge);
                }
            }
            let flick = 0.8 + 0.2 * (time * 2.3 + i as f32 * 1.7).sin();
            strip(
                &mut mesh,
                &pts,
                |_| 0.7,
                |k| lum(col, alphas[k.min(alphas.len() - 1)] * flick, add),
                |_| Color32::TRANSPARENT,
            );
        }
        // comets: a bright segment sliding up a strand
        for c in 0..self.comets {
            let phase = h01(c as u64 * 977) * TAU;
            let head = ((time * 0.18 + h01(c as u64 * 13)) % 1.0) * 1.15;
            pts.clear();
            alphas.clear();
            for k in 0..=24 {
                let u = head - k as f32 * 0.012;
                if !(0.02..=0.98).contains(&u) {
                    continue;
                }
                let (p, _) = surface(u, phase);
                if let Some((s, _, _)) = cam.project(view(p)) {
                    pts.push(push(s));
                    alphas.push((1.0 - k as f32 / 24.0).powf(1.5));
                }
            }
            if pts.len() > 1 {
                strip(
                    &mut mesh,
                    &pts,
                    |k| 1.8 * alphas[k],
                    |k| lum(col, alphas[k] * if add { 0.95 } else { 0.6 }, add),
                    |_| Color32::TRANSPARENT,
                );
            }
        }
        painter.add(Shape::mesh(mesh));
        // motes riding up the funnel
        let mut dots = Dots::new(self.dots);
        for i in 0..self.dots {
            let seed = i as u64 * 7919 + 11;
            let u = (h01(seed) + time * (0.02 + 0.03 * h01(seed + 1))) % 1.0;
            let phase = h01(seed + 2) * TAU;
            let (p, r) = surface(u, phase);
            let jitter = 1.0 + (h01(seed + 3) - 0.5) * 0.18;
            let p = v3(p.x * jitter, p.y, p.z * jitter);
            if let Some((s, sc, d)) = cam.project(view(p)) {
                let tw = 0.5 + 0.5 * (time * (3.0 + 5.0 * h01(seed + 4)) + h01(seed + 5) * TAU).sin();
                let depth = ((d - cam.dist) / 3.0).clamp(-1.0, 1.0);
                let a = (0.25 + 0.75 * tw) * (0.7 - depth * 0.3) * (u * 8.0).min(1.0);
                let size = ((0.45 + 0.9 * h01(seed + 6).powi(2)) * (sc / (unit * 0.3)).clamp(0.6, 1.3) * (1.0 + 0.1 / r)).min(1.6);
                let a = if add { a } else { a * 0.55 };
                dots.add(push(s), size, lum(col, a, add));
            }
        }
        dots.paint(&painter);
        // soft glow where the funnel pinches
        if add {
            if let Some((s, sc, _)) = cam.project(view(v3(0.0, y0 - 0.55, 0.0))) {
                paint::glow_ellipse(&painter, s, vec2(sc * rw * 4.0, sc * 0.18), lum(col, 0.18, true));
            }
        }
        paint::vignette(&painter, rect, bg, 0.6);
    }
}

impl Widget for Tornado {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = alloc(ui, self.size, 380.0, Sense::hover());
        let time = glowkit_core::time(ui);
        let (p, _) = pointer_or_ghost(ui, rect, &response, time, self.ghost);
        self.paint(ui, rect, p);
        response
    }
}

// ---------------------------------------------------------------------------------------------
// Particle Saturn
// ---------------------------------------------------------------------------------------------

/// A point-cloud ringed planet: a Fibonacci-sphere body of fine dots wrapped in wide particle rings
/// with dark division gaps. Inner ring particles orbit faster than outer ones; drag to spin the
/// planet with inertia.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::ParticleSaturn::new().tilt(0.4));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ParticleSaturn {
    size: Option<Vec2>,
    body: usize,
    ring: usize,
    tilt: f32,
    roll: f32,
    spin: f32,
    dot: f32,
    core: Option<Color32>,
    rings: Option<Color32>,
    background: Option<Color32>,
}

impl Default for ParticleSaturn {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticleSaturn {
    /// A planet with default settings.
    pub fn new() -> Self {
        Self {
            size: None,
            body: 3000,
            ring: 4200,
            tilt: 0.24,
            roll: -0.12,
            spin: 1.0,
            dot: 1.0,
            core: None,
            rings: None,
            background: None,
        }
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Number of particles on the planet body (200..3000).
    pub fn body_particles(mut self, n: usize) -> Self {
        self.body = n.clamp(200, 3000);
        self
    }
    /// Number of ring particles (200..3000).
    pub fn ring_particles(mut self, n: usize) -> Self {
        self.ring = n.clamp(200, 3000);
        self
    }
    /// Forward tilt of the ring plane in radians.
    pub fn tilt(mut self, a: f32) -> Self {
        self.tilt = a;
        self
    }
    /// Sideways roll of the ring plane in radians.
    pub fn roll(mut self, a: f32) -> Self {
        self.roll = a;
        self
    }
    /// Idle spin speed multiplier.
    pub fn spin(mut self, s: f32) -> Self {
        self.spin = s;
        self
    }
    /// Particle size multiplier.
    pub fn dot_size(mut self, k: f32) -> Self {
        self.dot = k.clamp(0.3, 3.0);
        self
    }
    /// Planet body color.
    pub fn core_color(mut self, c: Color32) -> Self {
        self.core = Some(c);
        self
    }
    /// Ring color.
    pub fn ring_color(mut self, c: Color32) -> Self {
        self.rings = Some(c);
        self
    }
    /// Background fill (default: the theme background).
    pub fn background(mut self, c: Color32) -> Self {
        self.background = Some(c);
        self
    }

    fn draw(&self, ui: &Ui, rect: Rect, yaw: f32, pitch: f32) {
        let t = Theme::of(ui);
        let time = glowkit_core::time(ui);
        let painter = ui.painter_at(rect);
        let bg = self.background.unwrap_or(t.background);
        painter.rect_filled(rect, 0.0, bg);
        let add = t.dark;
        let core = self.core.unwrap_or(if add { hex("#E0B21C") } else { hex("#8A6A00") });
        let ringc = self.rings.unwrap_or(if add { hex("#FFD400") } else { hex("#C99700") });
        let unit = rect.height().min(rect.width() * 0.62);
        let cam = Cam { center: rect.center(), focal: unit * 2.5, dist: 8.0 };
        let tilt = self.tilt + pitch;
        let tf = |p: V3| p.rot_x(tilt).rot_z(self.roll);
        let pr = 1.0;
        let (center, csc, _) = cam.project(v3(0.0, 0.0, 0.0)).unwrap_or((rect.center(), 1.0, cam.dist));
        let disc_r = pr * csc * 1.01;
        let mut back = Dots::new(self.ring);
        let mut front = Dots::new(self.ring + self.body);
        let ds = self.dot * (unit / 360.0).clamp(0.7, 1.6);
        // body: fibonacci sphere, rotating about its own axis
        let ga = PI * (3.0 - 5f32.sqrt());
        for i in 0..self.body {
            let y = 1.0 - 2.0 * (i as f32 + 0.5) / self.body as f32;
            let r = (1.0 - y * y).sqrt();
            let th = ga * i as f32 + yaw;
            let p = tf(v3(r * th.cos(), y, r * th.sin()) * pr);
            if p.z < -0.05 {
                continue; // hidden hemisphere
            }
            if let Some((s, _, _)) = cam.project(p) {
                let lit = 0.45 + 0.55 * p.z.max(0.0);
                let band = 0.75 + 0.25 * (y * 14.0).sin();
                front.add(s, 0.9 * ds, lum(core, (lit * band * 1.25).min(1.0), add));
            }
        }
        // rings: radii with divisions
        for i in 0..self.ring {
            let seed = i as u64 * 104_729 + 3;
            let mut r = 1.36 + h01(seed).powf(0.85) * 0.92;
            // carve the gaps (push particles out of them)
            if (1.86..1.96).contains(&r) {
                r += 0.1;
            }
            if (1.5..1.53).contains(&r) {
                r -= 0.03;
            }
            let w = 0.55 / r.powf(1.5);
            let th = h01(seed + 1) * TAU + time * w * self.spin + yaw * 0.25;
            let yj = (h01(seed + 2) - 0.5) * 0.025;
            let p = tf(v3(r * th.cos(), yj, r * th.sin()));
            if let Some((s, sc, _)) = cam.project(p) {
                let behind = p.z < 0.0 && s.distance(center) < disc_r;
                if behind {
                    continue;
                }
                let dens = if r > 2.0 { 0.6 } else if r < 1.5 { 0.55 } else { 1.0 };
                let a = (0.55 + 0.45 * h01(seed + 4)) * dens;
                let size = (0.55 + 0.75 * h01(seed + 5)) * ds * (sc / csc).clamp(0.8, 1.2);
                let c = lum(ringc, a, add);
                if p.z < 0.0 {
                    back.add(s, size, c);
                } else {
                    front.add(s, size, c);
                }
            }
        }
        if add {
            paint::glow(&painter, center, disc_r * 1.6, lum(core, 0.12, true));
        } else {
            paint::glow(&painter, center, disc_r * 1.5, with_alpha(core, 0.06));
        }
        back.paint(&painter);
        // faint body disc so the back of the rings reads as hidden
        painter.circle_filled(center, disc_r, with_alpha(bg, 0.55));
        painter.circle_filled(center, disc_r, if add { lum(core, 0.06, true) } else { with_alpha(core, 0.06) });
        front.paint(&painter);
    }
}

impl Widget for ParticleSaturn {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = alloc(ui, self.size, 380.0, Sense::drag());
        let response = response.on_hover_cursor(egui::CursorIcon::Grab);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let s = spin_update(ui, &response, 0.25 * self.spin, 0.01);
        self.draw(ui, rect, s.yaw, s.pitch * 0.5);
        response
    }
}

// ---------------------------------------------------------------------------------------------
// Accretion Disc
// ---------------------------------------------------------------------------------------------

/// A black-hole background: thousands of particles orbit an occluding dark core along glowing
/// log-spiral lanes, with faint polar jets streaming from the poles. Drag to orbit the camera.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::AccretionDisc::new().arms(3));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AccretionDisc {
    size: Option<Vec2>,
    particles: usize,
    arms: usize,
    wind: f32,
    speed: f32,
    jets: bool,
    base: Option<Color32>,
    accent: Option<Color32>,
    background: Option<Color32>,
}

impl Default for AccretionDisc {
    fn default() -> Self {
        Self::new()
    }
}

impl AccretionDisc {
    /// A disc with default settings.
    pub fn new() -> Self {
        Self {
            size: None,
            particles: 4800,
            arms: 2,
            wind: 1.8,
            speed: 1.0,
            jets: true,
            base: None,
            accent: None,
            background: None,
        }
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Number of disc particles (500..5000).
    pub fn particles(mut self, n: usize) -> Self {
        self.particles = n.clamp(500, 5000);
        self
    }
    /// Number of spiral lanes (1..6).
    pub fn arms(mut self, n: usize) -> Self {
        self.arms = n.clamp(1, 6);
        self
    }
    /// How tightly the lanes wind (0.5..4).
    pub fn wind(mut self, k: f32) -> Self {
        self.wind = k.clamp(0.5, 4.0);
        self
    }
    /// Orbit speed multiplier.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Show the polar jets (default on).
    pub fn jets(mut self, on: bool) -> Self {
        self.jets = on;
        self
    }
    /// Base (outer, dim) color.
    pub fn base_color(mut self, c: Color32) -> Self {
        self.base = Some(c);
        self
    }
    /// Accent (lane-center, hot) color.
    pub fn accent_color(mut self, c: Color32) -> Self {
        self.accent = Some(c);
        self
    }
    /// Background fill (default: the theme background).
    pub fn background(mut self, c: Color32) -> Self {
        self.background = Some(c);
        self
    }

    fn draw(&self, ui: &Ui, rect: Rect, yaw: f32, pitch: f32) {
        let t = Theme::of(ui);
        let time = glowkit_core::time(ui) * self.speed;
        let painter = ui.painter_at(rect);
        let bg = self.background.unwrap_or(t.background);
        painter.rect_filled(rect, 0.0, bg);
        let add = t.dark;
        let base = self.base.unwrap_or(if add { hex("#FF5F00") } else { hex("#C2410C") });
        let accent = self.accent.unwrap_or(if add { hex("#FFD9A0") } else { hex("#7C2D12") });
        let unit = rect.height().min(rect.width() * 0.5);
        let cam = Cam { center: pos2(rect.center().x, rect.center().y - unit * 0.02), focal: unit * 2.1, dist: 7.5 };
        let tilt = 0.42 + pitch;
        let tf = |p: V3| p.rot_y(yaw).rot_x(tilt);
        let core_r = 0.32;
        let (center, csc, _) = cam.project(tf(v3(0.0, 0.0, 0.0))).unwrap_or((rect.center(), 1.0, 0.0));
        let disc_r = core_r * csc;
        let mut back = Dots::new(self.particles / 2);
        let mut front = Dots::new(self.particles);
        let ds = (unit / 300.0).clamp(0.7, 1.5);
        let arms = self.arms as f32;
        for i in 0..self.particles {
            let seed = i as u64 * 6151 + 7;
            let u = h01(seed);
            let r = 0.42 + 2.6 * u.powf(1.15);
            let arm = (h01(seed + 1) * arms).floor();
            let spread = (h01(seed + 2) - 0.5) * (h01(seed + 3) - 0.5) * 3.0; // peaked distribution
            let lane = arm / arms * TAU + self.wind * r.ln() * 3.2 + spread * 1.1;
            let w = 0.45 / r.powf(0.9);
            let th = lane + time * w * 0.6 - time * 0.25;
            let yj = (h01(seed + 4) - 0.5) * 0.04 * r;
            let p = tf(v3(r * th.cos(), yj, r * th.sin()));
            if let Some((s, sc, _)) = cam.project(p) {
                if p.z < 0.0 && s.distance(center) < disc_r * 1.02 {
                    continue;
                }
                let hot = (1.0 - spread.abs() * 1.4).clamp(0.0, 1.0);
                let inner = (1.0 - (r - 0.42) / 2.5).clamp(0.0, 1.0);
                let c = lerp_rgb(base, accent, hot * 0.8 + inner * 0.2);
                let a = (0.25 + 0.75 * hot) * (0.4 + 0.6 * inner) * (0.6 + 0.4 * h01(seed + 5));
                let a = if add { a } else { a * 0.9 };
                let size = (0.55 + 0.8 * h01(seed + 6)) * ds * (sc / csc).clamp(0.7, 1.3);
                let c = lum(c, a, add);
                if p.z < 0.0 {
                    back.add(s, size, c);
                } else {
                    front.add(s, size, c);
                }
            }
        }
        // halo + photon ring
        if add {
            paint::glow(&painter, center, disc_r * 3.5, lum(base, 0.22, true));
        } else {
            paint::glow(&painter, center, disc_r * 3.0, with_alpha(base, 0.10));
        }
        back.paint(&painter);
        // jets
        if self.jets {
            let mut jet = Dots::new(400);
            for i in 0..360 {
                let seed = i as u64 * 811 + 5;
                let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
                let h = (h01(seed) + time * 0.12 * (0.6 + h01(seed + 1))) % 1.0;
                let y = sign * (core_r + h * 2.6);
                let rr = 0.03 + h * 0.18 * h01(seed + 2);
                let a0 = h01(seed + 3) * TAU + time;
                let p = tf(v3(rr * a0.cos(), y, rr * a0.sin()));
                if let Some((s, _, _)) = cam.project(p) {
                    if p.z < 0.0 && s.distance(center) < disc_r {
                        continue;
                    }
                    let a = (1.0 - h).powf(1.5) * 0.45;
                    jet.add(s, 0.7 * ds, lum(accent, a, add));
                }
            }
            jet.paint(&painter);
        }
        // the core
        let ink = if add { Color32::BLACK } else { hex("#0B0B0F") };
        painter.circle_filled(center, disc_r, ink);
        painter.circle_stroke(center, disc_r + 0.5, egui::Stroke::new(1.2, lum(accent, 0.55, add)));
        front.paint(&painter);
        paint::vignette(&painter, rect, bg, 0.5);
    }
}

impl Widget for AccretionDisc {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = alloc(ui, self.size, 380.0, Sense::drag());
        let response = response.on_hover_cursor(egui::CursorIcon::Grab);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let s = spin_update(ui, &response, 0.0, 0.008);
        self.draw(ui, rect, s.yaw, s.pitch);
        response
    }
}

// ---------------------------------------------------------------------------------------------
// Cosmic Orb
// ---------------------------------------------------------------------------------------------

/// A glass sphere with a slowly turning galaxy inside: drifting nebula clouds, twinkling stars,
/// a lensed rim with chromatic fringes and a soft specular highlight. The orb leans toward the
/// pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::CosmicOrb::new().radius(140.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CosmicOrb {
    size: Option<Vec2>,
    radius: f32,
    palette: Option<[Color32; 4]>,
    speed: f32,
    lens: f32,
    stars: usize,
}

impl Default for CosmicOrb {
    fn default() -> Self {
        Self::new()
    }
}

impl CosmicOrb {
    /// An orb with default settings.
    pub fn new() -> Self {
        Self { size: None, radius: 140.0, palette: None, speed: 1.0, lens: 0.45, stars: 240 }
    }
    /// Widget size (default: a square around the orb).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Orb radius in points.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.max(20.0);
        self
    }
    /// Four nebula colors.
    pub fn palette(mut self, p: [Color32; 4]) -> Self {
        self.palette = Some(p);
        self
    }
    /// Animation speed multiplier.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Lens (edge magnification and fringe) strength, 0..1.
    pub fn lens(mut self, k: f32) -> Self {
        self.lens = k.clamp(0.0, 1.0);
        self
    }
    /// Number of stars inside.
    pub fn stars(mut self, n: usize) -> Self {
        self.stars = n.min(800);
        self
    }

    /// Paint the orb centered at `c`; `lean` (-1..1 each axis) turns it toward the pointer.
    pub fn paint(&self, ui: &Ui, c: Pos2, lean: Vec2) {
        let t = Theme::of(ui);
        let time = glowkit_core::time(ui) * self.speed;
        let painter = ui.painter();
        let r = self.radius;
        let pal = self.palette.unwrap_or([hex("#7C3AED"), hex("#EC4899"), hex("#38BDF8"), hex("#F0ABFC")]);
        let deep = hex("#05030C");
        // drop shadow / outer glow
        if t.dark {
            paint::glow(painter, c, r * 1.5, lum(pal[0], 0.22, true));
        } else {
            paint::glow(painter, c + vec2(0.0, r * 0.18), r * 1.35, with_alpha(hex("#1E1B4B"), 0.18));
        }
        let yaw = time * 0.12 + lean.x * 0.5;
        let pitch = lean.y * 0.35;
        let lens = self.lens;
        // map a disc point to a direction inside the sphere (with lens magnification near the rim)
        let dir = |x: f32, y: f32| -> V3 {
            let rr = (x * x + y * y).min(1.0);
            let k = 1.0 - lens * 0.35 * rr * rr;
            let (x, y) = (x * k, y * k);
            let z = (1.0 - (x * x + y * y)).max(0.0).sqrt();
            v3(x, y, z).rot_x(pitch).rot_y(yaw)
        };
        let nebula = |d: V3| -> Color32 {
            let tt = time * 0.05;
            let n1 = fbm3(d.x * 1.6 + tt, d.y * 1.6, d.z * 1.6 - tt, 3);
            let n2 = fbm3(d.x * 2.4 - 3.0, d.y * 2.4 + tt, d.z * 2.4 + 1.7, 3);
            let streak = (1.0 - ((d.y * 3.0 + n1 * 2.0).sin()).abs()).powf(3.0);
            let a = (n1 * 0.5 + 0.5).powf(2.2) * 1.4;
            let b = (n2 * 0.5 + 0.5).powf(3.0) * 1.6;
            let mut col = lerp_rgb(deep, pal[0], (a * 0.9).min(1.0));
            col = lerp_rgb(col, pal[1], (b * 0.7).min(1.0));
            col = lerp_rgb(col, pal[2], (streak * 0.35 * (1.0 - a)).min(1.0));
            col = lerp_rgb(col, pal[3], ((a * b) * 0.6).min(1.0));
            col
        };
        // polar mesh
        let rings = 14;
        let segs = 72;
        let mut m = Mesh::default();
        let cd = dir(0.0, 0.0);
        m.colored_vertex(c, nebula(cd));
        for k in 1..=rings {
            let s = (k as f32 / rings as f32).powf(0.8);
            for j in 0..segs {
                let a = j as f32 / segs as f32 * TAU;
                let (x, y) = (a.cos() * s, a.sin() * s);
                let mut col = nebula(dir(x, -y));
                // darken towards the rim (thickness) then the fresnel brightening is added below
                col = lerp_rgb(col, deep, (s * s * 0.35).min(1.0));
                m.colored_vertex(c + vec2(x, y) * r, col);
            }
        }
        for j in 0..segs as u32 {
            m.add_triangle(0, 1 + j, 1 + (j + 1) % segs as u32);
        }
        for k in 1..rings as u32 {
            let a0 = 1 + (k - 1) * segs as u32;
            let b0 = 1 + k * segs as u32;
            for j in 0..segs as u32 {
                let jn = (j + 1) % segs as u32;
                m.add_triangle(a0 + j, b0 + j, b0 + jn);
                m.add_triangle(a0 + j, b0 + jn, a0 + jn);
            }
        }
        // feather
        let f0 = m.vertices.len() as u32;
        let o0 = 1 + (rings as u32 - 1) * segs as u32;
        for j in 0..segs {
            let a = j as f32 / segs as f32 * TAU;
            m.colored_vertex(c + vec2(a.cos(), a.sin()) * (r + 1.0), Color32::TRANSPARENT);
        }
        for j in 0..segs as u32 {
            let jn = (j + 1) % segs as u32;
            m.add_triangle(o0 + j, f0 + j, f0 + jn);
            m.add_triangle(o0 + j, f0 + jn, o0 + jn);
        }
        painter.add(Shape::mesh(m));
        // stars
        let mut dots = Dots::new(self.stars);
        for i in 0..self.stars {
            let seed = i as u64 * 1543 + 9;
            let z = h01(seed) * 2.0 - 1.0;
            let a = h01(seed + 1) * TAU;
            let rr = (1.0 - z * z).sqrt();
            let depth = 0.35 + 0.6 * h01(seed + 2);
            let p = v3(rr * a.cos(), z, rr * a.sin()) * depth;
            let p = p.rot_y(-yaw).rot_x(-pitch);
            let xy = vec2(p.x, -p.y);
            let l = xy.length();
            if l > 0.97 {
                continue;
            }
            let mag = 1.0 + lens * 0.35 * l.powi(4);
            let s = c + xy * r * mag.min(1.04);
            let tw = 0.5 + 0.5 * (time * (1.5 + 3.0 * h01(seed + 3)) + h01(seed + 4) * TAU).sin();
            let front = (p.z * 0.5 + 0.5).clamp(0.2, 1.0);
            let size = (0.5 + 1.2 * h01(seed + 5).powi(3)) * (r / 140.0).clamp(0.6, 1.6);
            dots.add(s, size, lum(Color32::WHITE, (0.35 + 0.65 * tw) * front, true));
        }
        dots.paint(painter);
        // rim: fresnel glow + chromatic fringes
        let rim = |rad: f32, w: f32, col: Color32| {
            painter.circle_stroke(c, rad, egui::Stroke::new(w, col));
        };
        for k in 0..6 {
            let f = k as f32 / 6.0;
            rim(r - 1.0 - f * r * 0.09, 2.2, lum(Color32::WHITE, 0.12 * (1.0 - f), true));
        }
        if lens > 0.0 {
            let off = vec2(1.0, 0.6) * (1.5 * lens);
            painter.circle_stroke(c + off, r - 2.0, egui::Stroke::new(1.4, lum(hex("#FF4D6D"), 0.35 * lens, true)));
            painter.circle_stroke(c - off, r - 2.0, egui::Stroke::new(1.4, lum(hex("#4DA3FF"), 0.35 * lens, true)));
        }
        rim(r - 0.6, 1.2, lum(Color32::WHITE, 0.55, true));
        if !t.dark {
            painter.circle_stroke(c, r + 0.5, egui::Stroke::new(1.0, with_alpha(hex("#1E1B4B"), 0.3)));
        }
        // specular
        paint::glow_ellipse(painter, c + vec2(-r * 0.38, -r * 0.48), vec2(r * 0.34, r * 0.16), lum(Color32::WHITE, 0.22, true));
        paint::glow(painter, c + vec2(-r * 0.45, -r * 0.5), r * 0.08, lum(Color32::WHITE, 0.55, true));
        let _ = PI;
    }
}

impl Widget for CosmicOrb {
    fn ui(self, ui: &mut Ui) -> Response {
        let side = self.radius * 2.0 + 60.0;
        let (rect, response) = alloc(ui, Some(self.size.unwrap_or(vec2(ui.available_width().max(side), side))), side, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let time = glowkit_core::time(ui);
        let target = match response.hover_pos() {
            Some(p) => ((p - rect.center()) / (rect.size() * 0.5)).clamp(Vec2::splat(-1.0), Vec2::splat(1.0)),
            None => vec2((time * 0.4).sin() * 0.5, (time * 0.31).cos() * 0.4),
        };
        let lean = glowkit_core::anim::spring2(ui, response.id.with("lean"), target, 60.0, 14.0);
        self.paint(ui, rect.center(), lean);
        let _ = ramp;
        response
    }
}
