//! Motion paths: move things along a polyline / bezier with rotation following the tangent, and
//! "draw" any path progressively (`pathLength` animations).

use egui::{Color32, Painter, Pos2, Vec2};
use glowkit_core::paint;

/// A sampled point on a [`MotionPath`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathPoint {
    /// Position on the path.
    pub pos: Pos2,
    /// Unit tangent (direction of travel).
    pub tangent: Vec2,
    /// Tangent angle in radians (0 = pointing right, clockwise positive).
    pub angle: f32,
}

/// A path parameterized by arc length, so `t = 0.5` is always halfway along it no matter how
/// unevenly its points are spaced.
///
/// ```no_run
/// use glowkit_motion::MotionPath;
/// use egui::pos2;
/// let path = MotionPath::smooth(&[pos2(0.0, 0.0), pos2(100.0, 60.0), pos2(200.0, 0.0)], false);
/// let p = path.sample(0.5);
/// // draw a rocket at p.pos rotated by p.angle
/// ```
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MotionPath {
    points: Vec<Pos2>,
    cumulative: Vec<f32>,
    closed: bool,
}

impl MotionPath {
    /// A path through `points` with straight segments.
    pub fn new(points: impl Into<Vec<Pos2>>, closed: bool) -> Self {
        let mut points: Vec<Pos2> = points.into();
        if closed && points.len() > 1 {
            points.push(points[0]);
        }
        let mut cumulative = Vec::with_capacity(points.len());
        let mut acc = 0.0;
        for (i, p) in points.iter().enumerate() {
            if i > 0 {
                acc += points[i - 1].distance(*p);
            }
            cumulative.push(acc);
        }
        Self { points, cumulative, closed }
    }

    /// A smooth Catmull-Rom curve through `points`.
    pub fn smooth(points: &[Pos2], closed: bool) -> Self {
        if points.len() < 3 {
            return Self::new(points.to_vec(), closed);
        }
        let pts = if closed {
            // wrap so the curve is smooth across the seam
            let n = points.len();
            let mut ext = vec![points[n - 1]];
            ext.extend_from_slice(points);
            ext.push(points[0]);
            ext.push(points[1 % n]);
            const PER: usize = 16;
            let curve = paint::catmull_rom(&ext, PER);
            // drop the leading / trailing helper segments (and the final duplicate point; `new`
            // closes the loop again)
            curve[PER..curve.len() - PER - 1].to_vec()
        } else {
            paint::catmull_rom(points, 16)
        };
        Self::new(pts, closed)
    }

    /// A cubic bezier from `a` to `b` with control points `c1`, `c2`.
    pub fn cubic(a: Pos2, c1: Pos2, c2: Pos2, b: Pos2) -> Self {
        Self::new(paint::cubic_bezier(a, c1, c2, b, 64), false)
    }

    /// Append another path (its first point is joined to this path's last point).
    pub fn then(self, other: &MotionPath) -> Self {
        let mut pts = self.points;
        pts.extend_from_slice(&other.points);
        Self::new(pts, false)
    }

    /// The polyline points.
    pub fn points(&self) -> &[Pos2] {
        &self.points
    }

    /// Total length in points.
    pub fn length(&self) -> f32 {
        self.cumulative.last().copied().unwrap_or(0.0)
    }

    /// Is this a closed loop?
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    fn locate(&self, t: f32) -> (usize, f32) {
        let n = self.points.len();
        let target = t * self.length();
        let i = self.cumulative.partition_point(|&c| c < target).clamp(1, n - 1);
        let (c0, c1) = (self.cumulative[i - 1], self.cumulative[i]);
        let k = if c1 - c0 > 1e-6 { (target - c0) / (c1 - c0) } else { 0.0 };
        (i, k)
    }

    /// Position, tangent and angle at `t` (`0..=1` along the length; wraps for closed paths).
    pub fn sample(&self, t: f32) -> PathPoint {
        let t = if self.closed { t.rem_euclid(1.0) } else { t.clamp(0.0, 1.0) };
        match self.points.len() {
            0 => PathPoint { pos: Pos2::ZERO, tangent: Vec2::X, angle: 0.0 },
            1 => PathPoint { pos: self.points[0], tangent: Vec2::X, angle: 0.0 },
            _ => {
                let (i, k) = self.locate(t);
                let (a, b) = (self.points[i - 1], self.points[i]);
                let pos = a.lerp(b, k);
                // smooth the tangent with the neighbouring segment to avoid jitter at corners
                let d = (b - a).normalized();
                let tangent = if k > 0.5 && i + 1 < self.points.len() {
                    let d2 = (self.points[i + 1] - b).normalized();
                    (d * (1.5 - k) + d2 * (k - 0.5)).normalized()
                } else if k < 0.5 && i >= 2 {
                    let d0 = (a - self.points[i - 2]).normalized();
                    (d * (0.5 + k) + d0 * (0.5 - k)).normalized()
                } else {
                    d
                };
                let tangent = if tangent.is_finite() && tangent.length_sq() > 0.0 { tangent } else { Vec2::X };
                PathPoint { pos, tangent, angle: tangent.y.atan2(tangent.x) }
            }
        }
    }

    /// Position at `t`.
    pub fn point_at(&self, t: f32) -> Pos2 {
        self.sample(t).pos
    }

    /// The part of the path between `t0` and `t1` (`0..=1`) as a polyline.
    pub fn slice(&self, t0: f32, t1: f32) -> Vec<Pos2> {
        let (t0, t1) = (t0.clamp(0.0, 1.0), t1.clamp(0.0, 1.0));
        if self.points.len() < 2 || t1 <= t0 {
            return Vec::new();
        }
        let (i0, k0) = self.locate(t0);
        let (i1, k1) = self.locate(t1);
        let mut out = Vec::with_capacity(i1 - i0 + 2);
        out.push(self.points[i0 - 1].lerp(self.points[i0], k0));
        out.extend_from_slice(&self.points[i0..i1]);
        out.push(self.points[i1 - 1].lerp(self.points[i1], k1));
        out
    }

    /// Stroke the first `progress` (`0..=1`) of the path with a color function over the drawn
    /// length (`f(0)` at the start, `f(1)` at the pen tip). Like animating SVG `pathLength`.
    pub fn paint_progress(&self, painter: &Painter, progress: f32, width: f32, f: impl Fn(f32) -> Color32) {
        let pts = self.slice(0.0, progress);
        if pts.len() >= 2 {
            paint::gradient_polyline(painter, &pts, false, width, f);
        }
    }

    /// Stroke the window `t0..t1` of the path (e.g. a moving dash / comet).
    pub fn paint_range(&self, painter: &Painter, t0: f32, t1: f32, width: f32, f: impl Fn(f32) -> Color32) {
        let pts = self.slice(t0, t1);
        if pts.len() >= 2 {
            paint::gradient_polyline(painter, &pts, false, width, f);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    #[test]
    fn arc_length_sampling() {
        // uneven spacing: 10 units, then 90 units
        let p = MotionPath::new(vec![pos2(0.0, 0.0), pos2(10.0, 0.0), pos2(100.0, 0.0)], false);
        assert_eq!(p.length(), 100.0);
        assert!((p.point_at(0.5).x - 50.0).abs() < 1e-4);
        assert!((p.sample(0.5).angle).abs() < 1e-5);
        let s = p.slice(0.0, 0.5);
        assert_eq!(s.first(), Some(&pos2(0.0, 0.0)));
        assert!((s.last().unwrap().x - 50.0).abs() < 1e-4);
    }

    #[test]
    fn closed_paths_wrap() {
        let sq = MotionPath::new(vec![pos2(0.0, 0.0), pos2(10.0, 0.0), pos2(10.0, 10.0), pos2(0.0, 10.0)], true);
        assert_eq!(sq.length(), 40.0);
        assert!((sq.point_at(1.25) - sq.point_at(0.25)).length() < 1e-4);
    }
}
