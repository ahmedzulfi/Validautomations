use super::util::idle_time;
use egui::{pos2, vec2, Color32, Pos2, Rect, Response, Sense, Ui, Vec2, Widget};
use glowkit_core::{anim::smoothstep, color::with_alpha, hex, mix, paint, rng::hash01, Theme};
use std::f32::consts::TAU;
use std::sync::Arc;

/// Pointer position inside `rect` (smoothed), or an autoplay Lissajous path when idle.
fn pointer_or_auto(ui: &Ui, resp: &Response, rect: Rect, speed: f32) -> (Pos2, bool) {
    let idle = idle_time(ui, resp.id, resp.hovered());
    let time = glowkit_core::time(ui);
    let auto = pos2(
        rect.center().x + rect.width() * 0.32 * (time * 0.55 * speed).sin(),
        rect.center().y + rect.height() * 0.28 * (time * 0.83 * speed + 0.6).sin(),
    );
    let (target, real) = match resp.hover_pos() {
        Some(p) => (p, true),
        None if idle > 0.5 => (auto, false),
        None => (auto, false),
    };
    let x = glowkit_core::anim::follow(ui, resp.id.with("px"), target.x, 7.0);
    let y = glowkit_core::anim::follow(ui, resp.id.with("py"), target.y, 7.0);
    (pos2(x, y), real)
}

/// Organic, merging blobs: a metaball field thresholded with a soft edge, colored by whichever
/// blob dominates. One blob follows the pointer (or wanders when idle).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_blocks::MetaBalls::new().count(8));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(...)`"]
pub struct MetaBalls {
    size: Option<Vec2>,
    count: usize,
    colors: Vec<Color32>,
    speed: f32,
    cell: f32,
}

impl Default for MetaBalls {
    fn default() -> Self {
        Self::new()
    }
}

impl MetaBalls {
    /// Default blobs in the theme gradient.
    pub fn new() -> Self {
        Self { size: None, count: 9, colors: Vec::new(), speed: 1.0, cell: 6.0 }
    }
    /// Size (defaults to available width x 360).
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.size = Some(vec2(w, h));
        self
    }
    /// Number of wandering blobs (plus the pointer blob).
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.clamp(1, 24);
        self
    }
    /// Blob colors (cycled).
    pub fn colors(mut self, c: &[Color32]) -> Self {
        self.colors = c.to_vec();
        self
    }
    /// Animation speed multiplier.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Field resolution in points (smaller = smoother edges, more vertices).
    pub fn resolution(mut self, cell: f32) -> Self {
        self.cell = cell.clamp(3.0, 20.0);
        self
    }
    /// Paint into `rect`, with `pointer` as the position of the pointer blob.
    pub fn paint_at(&self, ui: &Ui, rect: Rect, pointer: Pos2) {
        let t = Theme::of(ui);
        let colors = if self.colors.is_empty() { vec![t.gradient[0], t.gradient[1], t.gradient[2]] } else { self.colors.clone() };
        let time = glowkit_core::time(ui) * self.speed;
        let m = rect.width().min(rect.height());
        let mut balls: Vec<(Pos2, f32, Color32)> = (0..self.count)
            .map(|i| {
                let h = |k: u64| hash01(i as u64 * 97 + k);
                let c = pos2(
                    rect.center().x + rect.width() * 0.38 * (time * (0.2 + 0.25 * h(1)) + h(2) * TAU).sin(),
                    rect.center().y + rect.height() * 0.36 * (time * (0.18 + 0.3 * h(3)) + h(4) * TAU).cos(),
                );
                (c, m * (0.07 + 0.07 * h(5)), colors[i % colors.len()])
            })
            .collect();
        balls.push((pointer, m * 0.13, colors[colors.len() - 1]));
        let p = ui.painter_at(rect);
        let cols = (rect.width() / self.cell).ceil() as usize;
        let rows = (rect.height() / self.cell).ceil() as usize;
        let size = rect.size();
        let rgb: Vec<[f32; 3]> = balls.iter().map(|b| b.2.to_normalized_gamma_f32()).map(|c| [c[0], c[1], c[2]]).collect();
        paint::shader_grid(&p, rect, cols, rows, |uv| {
            let q = pos2(rect.min.x + uv.x * size.x, rect.min.y + uv.y * size.y);
            let mut f = 0.0;
            let mut acc = [0.0f32; 3];
            let mut wsum = 1e-6f32;
            for (i, (c, r, _)) in balls.iter().enumerate() {
                let d2 = (q - *c).length_sq().max(1.0);
                let w = r * r / d2;
                f += w;
                let w2 = w * w;
                wsum += w2;
                acc[0] += rgb[i][0] * w2;
                acc[1] += rgb[i][1] * w2;
                acc[2] += rgb[i][2] * w2;
            }
            let a = smoothstep(0.9, 1.1, f) * 0.95 + smoothstep(0.35, 0.9, f) * 0.18;
            if a <= 0.001 {
                return Color32::TRANSPARENT;
            }
            // brighter core
            let core = smoothstep(1.2, 3.0, f) * 0.25;
            let c = Color32::from_rgb(
                ((acc[0] / wsum + core).min(1.0) * 255.0) as u8,
                ((acc[1] / wsum + core).min(1.0) * 255.0) as u8,
                ((acc[2] / wsum + core).min(1.0) * 255.0) as u8,
            );
            with_alpha(c, a.min(1.0))
        });
    }
}

impl Widget for MetaBalls {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = glowkit_core::desired_size(ui, self.size, 360.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        if ui.is_rect_visible(rect) {
            glowkit_core::animate(ui);
            let (ptr, _) = pointer_or_auto(ui, &resp, rect, 1.0);
            self.paint_at(ui, rect, ptr);
        }
        resp
    }
}

#[derive(Clone, Copy, Debug)]
struct Ball {
    pos: Vec2,
    vel: Vec2,
    r: f32,
    color: usize,
}

/// A pit of glossy balls with simple 2D physics (gravity, walls, ball-ball collisions). The
/// pointer pushes them around; when idle, an invisible hand stirs the pit.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_blocks::BallPit::new().count(40));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BallPit {
    size: Option<Vec2>,
    count: usize,
    colors: Vec<Color32>,
    gravity: f32,
}

impl Default for BallPit {
    fn default() -> Self {
        Self::new()
    }
}

impl BallPit {
    /// A pit with 36 balls.
    pub fn new() -> Self {
        Self { size: None, count: 36, colors: Vec::new(), gravity: 900.0 }
    }
    /// Size (defaults to available width x 360).
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.size = Some(vec2(w, h));
        self
    }
    /// Number of balls.
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.clamp(1, 120);
        self
    }
    /// Ball colors (defaults to vivid theme chart colors).
    pub fn colors(mut self, c: &[Color32]) -> Self {
        self.colors = c.to_vec();
        self
    }
    /// Gravity in points / s².
    pub fn gravity(mut self, g: f32) -> Self {
        self.gravity = g;
        self
    }
}

impl Widget for BallPit {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = glowkit_core::desired_size(ui, self.size, 360.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        glowkit_core::animate(ui);
        let id = resp.id.with(("pit", self.count));
        let w = rect.width();
        let h = rect.height();
        let n = self.count;
        let mut balls: Arc<Vec<Ball>> = glowkit_core::state(ui, id, || {
            Arc::new(
                (0..n)
                    .map(|i| {
                        let hh = |k: u64| hash01(i as u64 * 31 + k);
                        let r = (w.min(h) * (0.045 + 0.05 * hh(1))).max(8.0);
                        Ball { pos: vec2(r + hh(2) * (w - 2.0 * r), -hh(3) * h), vel: vec2(hh(4) * 200.0 - 100.0, 0.0), r, color: i }
                    })
                    .collect(),
            )
        });
        let (ptr, real) = pointer_or_auto(ui, &resp, rect, 1.4);
        let ptr_v = ptr - rect.min;
        let dt = glowkit_core::dt(ui);
        let time = glowkit_core::time(ui);
        {
            let bs = Arc::make_mut(&mut balls);
            let steps = 3;
            let h_dt = dt / steps as f32;
            for _ in 0..steps {
                for b in bs.iter_mut() {
                    b.vel.y += self.gravity * h_dt;
                    // pointer push
                    let d = b.pos - ptr_v;
                    let dist = d.length();
                    let reach = 110.0 + b.r;
                    if dist < reach && dist > 0.1 {
                        let k = (1.0 - dist / reach) * if real { 5200.0 } else { 3600.0 };
                        b.vel += d / dist * k * h_dt;
                    }
                    b.vel *= (-0.4 * h_dt).exp();
                    b.pos += b.vel * h_dt;
                    if b.pos.x < b.r {
                        b.pos.x = b.r;
                        b.vel.x = b.vel.x.abs() * 0.75;
                    }
                    if b.pos.x > w - b.r {
                        b.pos.x = w - b.r;
                        b.vel.x = -b.vel.x.abs() * 0.75;
                    }
                    if b.pos.y > h - b.r {
                        b.pos.y = h - b.r;
                        b.vel.y = -b.vel.y.abs() * 0.6;
                        b.vel.x *= 0.985;
                    }
                }
                for i in 0..bs.len() {
                    for j in (i + 1)..bs.len() {
                        let d = bs[j].pos - bs[i].pos;
                        let min = bs[i].r + bs[j].r;
                        let dist2 = d.length_sq();
                        if dist2 < min * min && dist2 > 1e-4 {
                            let dist = dist2.sqrt();
                            let nrm = d / dist;
                            let overlap = min - dist;
                            let (mi, mj) = (bs[i].r * bs[i].r, bs[j].r * bs[j].r);
                            let tot = mi + mj;
                            bs[i].pos -= nrm * overlap * (mj / tot);
                            bs[j].pos += nrm * overlap * (mi / tot);
                            let rel = (bs[j].vel - bs[i].vel).dot(nrm);
                            if rel < 0.0 {
                                let imp = -(1.0 + 0.7) * rel / (1.0 / mi + 1.0 / mj);
                                bs[i].vel -= nrm * imp / mi;
                                bs[j].vel += nrm * imp / mj;
                            }
                        }
                    }
                }
            }
            // occasional stir so the pit never sleeps
            if !real && (time % 4.0) < dt {
                for (i, b) in bs.iter_mut().enumerate() {
                    if hash01(i as u64 + (time as u64) * 13) < 0.4 {
                        b.vel += vec2(hash01(i as u64 * 3) * 300.0 - 150.0, -500.0 - 300.0 * hash01(i as u64 * 7));
                    }
                }
            }
        }
        let colors = if self.colors.is_empty() {
            vec![hex("#FF4D8D"), hex("#3DB2FF"), hex("#FFD23F"), hex("#7CE577"), hex("#A26BFF"), t.accent]
        } else {
            self.colors.clone()
        };
        let p = ui.painter_at(rect);
        for b in balls.iter() {
            let c = rect.min + b.pos;
            let base = colors[b.color % colors.len()];
            // contact shadow
            paint::glow_ellipse(&p, c + vec2(0.0, b.r * 0.85), vec2(b.r * 0.9, b.r * 0.25), Color32::from_black_alpha(if t.dark { 70 } else { 30 }));
            let hl = c + vec2(-b.r * 0.35, -b.r * 0.4);
            let light = mix(base, Color32::WHITE, 0.45);
            let dark = mix(base, Color32::BLACK, 0.35);
            let pts = paint::circle_points(c, b.r, 40);
            let rr = b.r;
            paint::fill_convex_fn(&p, &pts, 4, |q| {
                let k = (q.distance(hl) / (rr * 1.6)).clamp(0.0, 1.0);
                if k < 0.5 {
                    light.lerp_to_gamma(base, k * 2.0)
                } else {
                    base.lerp_to_gamma(dark, (k - 0.5) * 2.0)
                }
            });
            // specular
            paint::glow(&p, hl, b.r * 0.35, Color32::from_white_alpha(170));
        }
        glowkit_core::set_state(ui, id, balls);
        resp
    }
}

/// A grid of lines that ripples: waves emanate from the center and from the pointer, displacing
/// the lines and lighting up the crests, with a soft vignette.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_blocks::RippleGrid::new().spacing(28.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct RippleGrid {
    size: Option<Vec2>,
    spacing: f32,
    color: Option<Color32>,
    amplitude: f32,
}

impl Default for RippleGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl RippleGrid {
    /// A ripple grid with default spacing.
    pub fn new() -> Self {
        Self { size: None, spacing: 30.0, color: None, amplitude: 10.0 }
    }
    /// Size (defaults to available width x 360).
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.size = Some(vec2(w, h));
        self
    }
    /// Grid spacing in points.
    pub fn spacing(mut self, s: f32) -> Self {
        self.spacing = s.max(8.0);
        self
    }
    /// Crest color (defaults to the accent).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Wave amplitude in points.
    pub fn amplitude(mut self, a: f32) -> Self {
        self.amplitude = a;
        self
    }
}

impl Widget for RippleGrid {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = glowkit_core::desired_size(ui, self.size, 360.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        glowkit_core::animate(ui);
        let (ptr, _) = pointer_or_auto(ui, &resp, rect, 0.8);
        let time = glowkit_core::time(ui);
        let crest = self.color.unwrap_or(t.accent);
        let base = if t.dark { mix(t.background, t.foreground, 0.12) } else { mix(t.background, t.foreground, 0.14) };
        let c = rect.center();
        let amp = self.amplitude;
        let wave = |q: Pos2| -> (Vec2, f32) {
            let mut disp = Vec2::ZERO;
            let mut e = 0.0;
            for (src, strength, k, w) in [(c, 1.0, 0.045, 3.0), (ptr, 0.9, 0.07, 5.0)] {
                let d = q - src;
                let dist = d.length().max(1e-3);
                let env = (-dist / 260.0).exp();
                let s = (dist * k - time * w).sin() * env * strength;
                disp += d / dist * s * amp;
                e += s.max(0.0) * env;
            }
            (disp, e.min(1.0))
        };
        let p = ui.painter_at(rect);
        let edge = |q: Pos2| {
            let ex = smoothstep(0.0, 80.0, (q.x - rect.min.x).min(rect.max.x - q.x));
            let ey = smoothstep(0.0, 60.0, (q.y - rect.min.y).min(rect.max.y - q.y));
            ex * ey
        };
        let draw = |pts: Vec<Pos2>| {
            let mut out = Vec::with_capacity(pts.len());
            let mut cols = Vec::with_capacity(pts.len());
            for q in pts {
                let (d, e) = wave(q);
                let qq = q + d;
                out.push(qq);
                let col = base.lerp_to_gamma(crest, e.powf(0.8));
                cols.push(with_alpha(col, edge(q) * (0.55 + 0.45 * e)));
            }
            let n = cols.len();
            paint::gradient_polyline(&p, &out, false, 1.2, |u| cols[((u * (n - 1) as f32).round() as usize).min(n - 1)]);
        };
        let sp = self.spacing;
        let nx = (rect.width() / sp).ceil() as i32;
        let ny = (rect.height() / sp).ceil() as i32;
        let ox = rect.center().x - (nx as f32 / 2.0).floor() * sp;
        let oy = rect.center().y - (ny as f32 / 2.0).floor() * sp;
        let step = 8.0;
        for i in -1..=nx + 1 {
            let x = ox + i as f32 * sp;
            if x < rect.min.x || x > rect.max.x {
                continue;
            }
            let m = (rect.height() / step) as usize;
            draw((0..=m).map(|k| pos2(x, rect.min.y + rect.height() * k as f32 / m as f32)).collect());
        }
        for j in -1..=ny + 1 {
            let y = oy + j as f32 * sp;
            if y < rect.min.y || y > rect.max.y {
                continue;
            }
            let m = (rect.width() / step) as usize;
            draw((0..=m).map(|k| pos2(rect.min.x + rect.width() * k as f32 / m as f32, y)).collect());
        }
        paint::glow(&p, ptr, 60.0, with_alpha(crest, 0.18));
        resp
    }
}
