//! Pressure-like variable-width ink strokes (in the spirit of "perfect freehand"): the pointer
//! path is streamlined, speed is turned into simulated pressure (fast = thin), the ends taper,
//! and the result is painted as a feathered, anti-aliased ribbon.

use egui::{epaint::Mesh, vec2, Color32, Painter, Pos2, Vec2};

/// One captured input sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InkPoint {
    /// Position (any coordinate space; world space in the editors).
    pub pos: Pos2,
    /// Capture time in seconds.
    pub t: f32,
    /// Real pressure in `0..=1`, or a negative value to simulate it from speed.
    pub pressure: f32,
}

impl InkPoint {
    /// A sample with simulated pressure.
    pub fn new(pos: Pos2, t: f32) -> Self {
        Self { pos, t, pressure: -1.0 }
    }
}

/// Shape parameters of a freehand stroke.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InkStyle {
    /// Base diameter.
    pub size: f32,
    /// How much speed thins the line (`0` = constant width, `0.7` = very expressive).
    pub thinning: f32,
    /// Streamline factor `0..1`: higher lags the pointer more and smooths jitter.
    pub streamline: f32,
    /// Taper length at the start, in units of `size`.
    pub taper_start: f32,
    /// Taper length at the end, in units of `size`.
    pub taper_end: f32,
}

impl Default for InkStyle {
    fn default() -> Self {
        Self { size: 4.0, thinning: 0.55, streamline: 0.45, taper_start: 2.0, taper_end: 3.0 }
    }
}

impl InkStyle {
    /// A pen with the given diameter.
    pub fn pen(size: f32) -> Self {
        Self { size, ..Default::default() }
    }
    /// Constant-width marker (no thinning, no taper).
    pub fn marker(size: f32) -> Self {
        Self { size, thinning: 0.0, streamline: 0.35, taper_start: 0.0, taper_end: 0.0 }
    }
}

/// Turn raw samples into a smoothed centerline with a radius per point.
pub fn stroke_radii(points: &[InkPoint], style: &InkStyle) -> Vec<(Pos2, f32)> {
    if points.is_empty() {
        return Vec::new();
    }
    // 1. streamline
    let mut line: Vec<(Pos2, f32, f32)> = Vec::with_capacity(points.len());
    let mut prev = points[0].pos;
    let k = 1.0 - style.streamline.clamp(0.0, 0.95);
    for (i, p) in points.iter().enumerate() {
        let q = if i == 0 { p.pos } else { prev + (p.pos - prev) * k };
        if i > 0 && q.distance(prev) < 0.3 && i + 1 != points.len() {
            continue;
        }
        line.push((q, p.t, p.pressure));
        prev = q;
    }
    // always end exactly on the last sample
    if let (Some(last), Some(lp)) = (line.last_mut(), points.last()) {
        last.0 = last.0 + (lp.pos - last.0) * 0.6;
    }
    // 2. subdivide long segments so width changes stay smooth
    let mut dense: Vec<(Pos2, f32, f32)> = Vec::with_capacity(line.len() * 2);
    for i in 0..line.len() {
        if i > 0 {
            let (a, ta, pa) = line[i - 1];
            let (b, tb, pb) = line[i];
            let n = (a.distance(b) / (style.size * 0.75).max(2.0)).floor() as usize;
            for s in 1..=n {
                let f = s as f32 / (n + 1) as f32;
                dense.push((a.lerp(b, f), ta + (tb - ta) * f, pa + (pb - pa) * f));
            }
        }
        dense.push(line[i]);
    }
    // 3. pressure from speed + taper
    let total: f32 = dense.windows(2).map(|w| w[0].0.distance(w[1].0)).sum();
    let mut pressure = 0.5f32;
    let mut run = 0.0f32;
    let r0 = style.size * 0.5;
    let mut out = Vec::with_capacity(dense.len());
    for i in 0..dense.len() {
        let (p, t, real) = dense[i];
        if i > 0 {
            let d = p.distance(dense[i - 1].0);
            run += d;
            let dt = (t - dense[i - 1].1).max(1.0 / 240.0);
            let speed = d / dt / (style.size * 60.0).max(1.0);
            let target = (1.0 - speed).clamp(0.0, 1.0);
            pressure += (target - pressure) * 0.25;
        }
        let pr = if real >= 0.0 { real } else { pressure };
        let mut r = r0 * (1.0 - style.thinning * (1.0 - pr) * 1.2).clamp(0.2, 1.4);
        if style.taper_start > 0.0 {
            let len = style.taper_start * style.size;
            r *= ease_out((run / len).clamp(0.0, 1.0)).max(0.15);
        }
        if style.taper_end > 0.0 {
            let len = style.taper_end * style.size;
            r *= ease_out(((total - run) / len).clamp(0.0, 1.0)).max(0.15);
        }
        out.push((p, r.max(0.35)));
    }
    out
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

/// Paint a variable-width ribbon (from [`stroke_radii`]) as a feathered mesh.
pub fn paint_ribbon(painter: &Painter, pts: &[(Pos2, f32)], color: Color32) {
    match pts.len() {
        0 => {}
        1 => {
            painter.circle_filled(pts[0].0, pts[0].1, color);
        }
        n => {
            let feather = 0.9;
            let clear = Color32::TRANSPARENT;
            let mut mesh = Mesh::default();
            mesh.reserve_vertices(n * 4);
            for i in 0..n {
                let (p, r) = pts[i];
                let a = pts[i.saturating_sub(1)].0;
                let b = pts[(i + 1).min(n - 1)].0;
                let mut d = b - a;
                if d.length_sq() < 1e-6 {
                    d = Vec2::X;
                }
                let nrm = vec2(-d.y, d.x).normalized();
                mesh.colored_vertex(p + nrm * (r + feather), clear);
                mesh.colored_vertex(p + nrm * r, color);
                mesh.colored_vertex(p - nrm * r, color);
                mesh.colored_vertex(p - nrm * (r + feather), clear);
                if i > 0 {
                    let b0 = ((i - 1) * 4) as u32;
                    let b1 = (i * 4) as u32;
                    for k in 0..3u32 {
                        mesh.add_triangle(b0 + k, b0 + k + 1, b1 + k);
                        mesh.add_triangle(b0 + k + 1, b1 + k + 1, b1 + k);
                    }
                }
            }
            painter.add(mesh);
            // round caps
            painter.circle_filled(pts[0].0, pts[0].1, color);
            painter.circle_filled(pts[n - 1].0, pts[n - 1].1, color);
        }
    }
}

/// Convenience: compute and paint a stroke, mapping each point through `map` first
/// (e.g. world → screen) and scaling the width by `scale`.
pub fn paint_ink(
    painter: &Painter,
    points: &[InkPoint],
    style: &InkStyle,
    color: Color32,
    map: impl Fn(Pos2) -> Pos2,
    scale: f32,
) {
    let pts: Vec<(Pos2, f32)> = stroke_radii(points, style).into_iter().map(|(p, r)| (map(p), r * scale)).collect();
    paint_ribbon(painter, &pts, color);
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    fn line(speed: f32) -> Vec<InkPoint> {
        (0..60).map(|i| InkPoint::new(pos2(i as f32 * speed, 0.0), i as f32 / 60.0)).collect()
    }

    #[test]
    fn tapers_at_ends() {
        let style = InkStyle::pen(8.0);
        let r = stroke_radii(&line(2.0), &style);
        let mid = r[r.len() / 2].1;
        assert!(r[0].1 < mid);
        assert!(r[r.len() - 1].1 < mid);
    }

    #[test]
    fn fast_is_thinner_than_slow() {
        let style = InkStyle { taper_start: 0.0, taper_end: 0.0, ..InkStyle::pen(8.0) };
        let slow = stroke_radii(&line(0.6), &style);
        let fast = stroke_radii(&line(25.0), &style);
        let avg = |v: &[(Pos2, f32)]| v.iter().map(|p| p.1).sum::<f32>() / v.len() as f32;
        assert!(avg(&fast) < avg(&slow));
    }

    #[test]
    fn marker_is_constant() {
        let r = stroke_radii(&line(5.0), &InkStyle::marker(6.0));
        assert!(r.iter().all(|p| (p.1 - 3.0).abs() < 1e-3));
    }
}
