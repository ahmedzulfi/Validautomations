//! Shape morphing: resample two outlines to the same number of points, align them, and
//! interpolate. [`MorphShape`] cycles through built-in shapes.

use egui::{vec2, Color32, Painter, Pos2, Response, Sense, Ui, Vec2, Widget};
use glowkit_core::anim::{ease, Easing};
use glowkit_core::{color, paint, Rng, Theme};
use std::f32::consts::{PI, TAU};

/// Resample a closed outline to `n` points evenly spaced by arc length.
pub fn resample_closed(pts: &[Pos2], n: usize) -> Vec<Pos2> {
    let m = pts.len();
    if m == 0 || n == 0 {
        return Vec::new();
    }
    if m == 1 {
        return vec![pts[0]; n];
    }
    let mut cum = Vec::with_capacity(m + 1);
    let mut acc = 0.0;
    cum.push(0.0);
    for i in 0..m {
        acc += pts[i].distance(pts[(i + 1) % m]);
        cum.push(acc);
    }
    let total = acc.max(1e-6);
    let mut out = Vec::with_capacity(n);
    let mut seg = 0;
    for k in 0..n {
        let d = total * k as f32 / n as f32;
        while seg + 1 < m && cum[seg + 1] < d {
            seg += 1;
        }
        let len = cum[seg + 1] - cum[seg];
        let t = if len > 1e-6 { (d - cum[seg]) / len } else { 0.0 };
        out.push(pts[seg].lerp(pts[(seg + 1) % m], t));
    }
    out
}

fn signed_area(pts: &[Pos2]) -> f32 {
    let n = pts.len();
    (0..n).map(|i| pts[i].x * pts[(i + 1) % n].y - pts[(i + 1) % n].x * pts[i].y).sum::<f32>() * 0.5
}

/// Resample `a` and `b` to `n` points each, with `b` rotated (and, if needed, reversed) so its
/// points line up with `a`'s, minimizing travel. Do this once, then [`lerp_points`] per frame.
pub fn align_outlines(a: &[Pos2], b: &[Pos2], n: usize) -> (Vec<Pos2>, Vec<Pos2>) {
    let ra = resample_closed(a, n);
    let mut rb = resample_closed(b, n);
    if ra.is_empty() || rb.is_empty() {
        return (ra, rb);
    }
    if signed_area(&ra).signum() != signed_area(&rb).signum() {
        rb.reverse();
    }
    let ca = centroid(&ra).to_vec2();
    let cb = centroid(&rb).to_vec2();
    let mut best = (f32::INFINITY, 0);
    for off in 0..n {
        let cost: f32 = (0..n).map(|i| ((ra[i] - ca) - (rb[(i + off) % n] - cb)).length_sq()).sum();
        if cost < best.0 {
            best = (cost, off);
        }
    }
    let rb = (0..n).map(|i| rb[(i + best.1) % n]).collect();
    (ra, rb)
}

/// Point-wise interpolation of two equally long point lists.
pub fn lerp_points(a: &[Pos2], b: &[Pos2], t: f32) -> Vec<Pos2> {
    a.iter().zip(b).map(|(p, q)| p.lerp(*q, t)).collect()
}

/// Morph polygon `a` into `b` at `t` (`0..=1`): both are resampled to the same number of points
/// and aligned, then interpolated. For per-frame use with fixed shapes, cache
/// [`align_outlines`] and call [`lerp_points`].
///
/// ```
/// use egui::pos2;
/// let square = [pos2(0.0, 0.0), pos2(10.0, 0.0), pos2(10.0, 10.0), pos2(0.0, 10.0)];
/// let tri = [pos2(5.0, 0.0), pos2(10.0, 10.0), pos2(0.0, 10.0)];
/// let mid = glowkit_motion::morph_polygon(&square, &tri, 0.5);
/// assert_eq!(mid.len(), 96);
/// ```
pub fn morph_polygon(a: &[Pos2], b: &[Pos2], t: f32) -> Vec<Pos2> {
    let (ra, rb) = align_outlines(a, b, 96);
    lerp_points(&ra, &rb, t)
}

/// Average of the points.
pub fn centroid(pts: &[Pos2]) -> Pos2 {
    if pts.is_empty() {
        return Pos2::ZERO;
    }
    (pts.iter().fold(Vec2::ZERO, |a, p| a + p.to_vec2()) / pts.len() as f32).to_pos2()
}

/// Built-in outlines for morphing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShapeKind {
    /// A circle.
    Circle,
    /// A rounded square.
    Square,
    /// An equilateral triangle (point up).
    Triangle,
    /// A star with this many points.
    Star(u8),
    /// A heart.
    Heart,
    /// A regular polygon with this many sides.
    Polygon(u8),
    /// An organic blob (different seeds give different blobs).
    Blob(u32),
}

impl ShapeKind {
    /// The outline (clockwise on screen), fitting a circle of `radius` around `center`, sampled
    /// densely enough to be resampled.
    pub fn points(self, center: Pos2, radius: f32) -> Vec<Pos2> {
        let polar = |n: usize, f: &dyn Fn(f32) -> f32| -> Vec<Pos2> {
            (0..n)
                .map(|i| {
                    let a = i as f32 / n as f32 * TAU - PI / 2.0;
                    center + vec2(a.cos(), a.sin()) * radius * f(a)
                })
                .collect()
        };
        match self {
            ShapeKind::Circle => polar(96, &|_| 1.0),
            ShapeKind::Square => {
                let r = radius * 0.86;
                paint::rounded_rect_points(egui::Rect::from_center_size(center, Vec2::splat(r * 2.0)), r * 0.22, 6)
            }
            ShapeKind::Triangle => ShapeKind::Polygon(3).points(center + vec2(0.0, radius * 0.12), radius * 1.05),
            ShapeKind::Polygon(n) => {
                let n = n.max(3) as usize;
                (0..n)
                    .map(|i| {
                        let a = i as f32 / n as f32 * TAU - PI / 2.0;
                        center + vec2(a.cos(), a.sin()) * radius
                    })
                    .collect()
            }
            ShapeKind::Star(n) => {
                let n = n.max(3) as usize;
                (0..n * 2)
                    .map(|i| {
                        let a = i as f32 / (n * 2) as f32 * TAU - PI / 2.0;
                        let r = if i % 2 == 0 { radius } else { radius * 0.45 };
                        center + vec2(a.cos(), a.sin()) * r
                    })
                    .collect()
            }
            ShapeKind::Heart => (0..120)
                .map(|i| {
                    let t = i as f32 / 120.0 * TAU;
                    let x = 16.0 * t.sin().powi(3);
                    let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
                    center + vec2(x, -y - 2.5) * (radius / 16.5)
                })
                .collect(),
            ShapeKind::Blob(seed) => {
                let mut rng = Rng::new(seed as u64 + 7);
                let k: Vec<(f32, f32)> = (0..4).map(|i| (rng.range(0.05, 0.13) / (1.0 + i as f32 * 0.4), rng.range(0.0, TAU))).collect();
                polar(96, &move |a| 0.9 + k.iter().enumerate().map(|(i, (amp, ph))| amp * ((i as f32 + 2.0) * a + ph).sin()).sum::<f32>())
            }
        }
    }
}

/// Fill a star-shaped outline (every point visible from its centroid, true for all
/// [`ShapeKind`]s and their morphs) with a per-vertex color function, anti-aliased.
pub fn fill_outline(painter: &Painter, outline: &[Pos2], f: impl Fn(Pos2) -> Color32) {
    paint::fill_convex_fn(painter, outline, 4, f);
}

/// A shape that continuously morphs through a list of outlines (circle → square → star → heart
/// → blob ...), filled with a gradient and a soft glow.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_motion::{MorphShape, ShapeKind};
/// ui.add(MorphShape::new().shapes(&[ShapeKind::Circle, ShapeKind::Star(5), ShapeKind::Heart]).size(160.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
#[derive(Clone, Debug)]
pub struct MorphShape {
    shapes: Vec<ShapeKind>,
    size: f32,
    hold: f32,
    duration: f32,
    easing: Easing,
    colors: Option<Vec<Color32>>,
    spin: bool,
    phase: f32,
}

impl Default for MorphShape {
    fn default() -> Self {
        Self::new()
    }
}

impl MorphShape {
    /// Circle → square → star → heart → blob, 0.8 s hold, 0.9 s morph.
    pub fn new() -> Self {
        Self {
            shapes: vec![ShapeKind::Circle, ShapeKind::Square, ShapeKind::Star(5), ShapeKind::Heart, ShapeKind::Blob(3), ShapeKind::Triangle],
            size: 150.0,
            hold: 0.8,
            duration: 0.9,
            easing: Easing::InOutCubic,
            colors: None,
            spin: true,
            phase: 0.0,
        }
    }
    /// The shapes to cycle through.
    pub fn shapes(mut self, shapes: &[ShapeKind]) -> Self {
        if !shapes.is_empty() {
            self.shapes = shapes.to_vec();
        }
        self
    }
    /// Width and height in points.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(8.0);
        self
    }
    /// Seconds each shape is held.
    pub fn hold(mut self, s: f32) -> Self {
        self.hold = s.max(0.0);
        self
    }
    /// Seconds per morph.
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s.max(0.05);
        self
    }
    /// Easing of each morph.
    pub fn easing(mut self, e: Easing) -> Self {
        self.easing = e;
        self
    }
    /// Gradient colors (defaults to the theme gradient).
    pub fn colors(mut self, c: &[Color32]) -> Self {
        self.colors = Some(c.to_vec());
        self
    }
    /// Twist slightly while morphing (default on).
    pub fn spin(mut self, on: bool) -> Self {
        self.spin = on;
        self
    }
    /// Offset in seconds, to desynchronize several shapes.
    pub fn phase(mut self, seconds: f32) -> Self {
        self.phase = seconds;
        self
    }
}

impl Widget for MorphShape {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(self.size), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        glowkit_core::animate(ui);
        let time = glowkit_core::time(ui) + self.phase;
        let cycle = self.hold + self.duration;
        let n = self.shapes.len();
        let step = (time / cycle).floor();
        let i = (step as i64).rem_euclid(n as i64) as usize;
        let local = time - step * cycle;
        let k = ease(self.easing, ((local - self.hold) / self.duration).clamp(0.0, 1.0));
        let c = rect.center();
        let r = self.size * 0.42;
        let a = self.shapes[i].points(c, r);
        let b = self.shapes[(i + 1) % n].points(c, r);
        let (ra, rb) = align_outlines(&a, &b, 128);
        let mut pts = lerp_points(&ra, &rb, k);
        if self.spin {
            let ang = (k * PI).sin() * 0.3;
            let rot = egui::emath::Rot2::from_angle(ang);
            for p in &mut pts {
                *p = c + rot * (*p - c);
            }
        }
        let stops = self.colors.clone().unwrap_or_else(|| t.gradient.to_vec());
        let painter = ui.painter();
        let glow_col = color::with_alpha(color::gradient(&stops, 0.5), if t.dark { 0.35 } else { 0.22 });
        paint::glow(painter, c + vec2(0.0, r * 0.1), r * 1.5, glow_col);
        let f = paint::linear_fn(rect, 45.0 + time * 20.0, &stops);
        fill_outline(painter, &pts, f);
        // subtle top highlight
        let hl = color::with_alpha(Color32::WHITE, 0.16);
        paint::gradient_polyline(painter, &pts, true, 1.5, |_| hl);
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    #[test]
    fn resample_is_even() {
        let sq = [pos2(0.0, 0.0), pos2(10.0, 0.0), pos2(10.0, 10.0), pos2(0.0, 10.0)];
        let r = resample_closed(&sq, 8);
        assert_eq!(r.len(), 8);
        assert_eq!(r[1], pos2(5.0, 0.0));
        assert_eq!(r[2], pos2(10.0, 0.0));
    }

    #[test]
    fn morph_endpoints_match_shapes() {
        let a = ShapeKind::Circle.points(Pos2::ZERO, 10.0);
        let b = ShapeKind::Star(5).points(Pos2::ZERO, 10.0);
        let (ra, rb) = align_outlines(&a, &b, 64);
        assert_eq!(lerp_points(&ra, &rb, 0.0), ra);
        assert_eq!(lerp_points(&ra, &rb, 1.0), rb);
        for p in lerp_points(&ra, &rb, 0.5) {
            assert!(p.to_vec2().length() <= 10.01);
        }
    }
}
