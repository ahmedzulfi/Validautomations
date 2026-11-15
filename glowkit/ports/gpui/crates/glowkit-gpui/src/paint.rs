//! Low-level painting helpers used inside `canvas(..)` paint callbacks: rounded-rect outlines
//! with per-point colors (beams, sparks, spotlight borders), soft glows and polylines.
//!
//! GPUI draws paths with a single color each, so gradients *along* a stroke are built from many
//! short segments, the same way glowkit's egui painters do it with per-vertex colors.

use gpui::{point, px, size, Bounds, BoxShadow, Corners, Hsla, PathBuilder, Pixels, Point, Window};
use std::f32::consts::{FRAC_PI_2, PI};

/// Points around a rounded rectangle, clockwise from the top-left corner's end, roughly every
/// `step` pixels. Returned as `(x, y)` in window space.
pub fn rounded_outline(bounds: Bounds<Pixels>, radius: f32, step: f32) -> Vec<(f32, f32)> {
    let (x0, y0) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
    let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    let r = radius.clamp(0.0, w.min(h) / 2.0);
    let mut pts = Vec::new();
    let arc_steps = ((r * FRAC_PI_2) / step).ceil().max(2.0) as usize;
    let edge = |a: (f32, f32), b: (f32, f32), pts: &mut Vec<(f32, f32)>| {
        let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let n = (len / step).ceil().max(1.0) as usize;
        for i in 0..n {
            let t = i as f32 / n as f32;
            pts.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
        }
    };
    let arc = |c: (f32, f32), start: f32, pts: &mut Vec<(f32, f32)>| {
        for i in 0..arc_steps {
            let a = start + FRAC_PI_2 * i as f32 / arc_steps as f32;
            pts.push((c.0 + r * a.cos(), c.1 + r * a.sin()));
        }
    };
    // top edge, top-right arc, right edge, bottom-right arc, bottom edge, bottom-left, left, top-left
    edge((x0 + r, y0), (x0 + w - r, y0), &mut pts);
    arc((x0 + w - r, y0 + r), -FRAC_PI_2, &mut pts);
    edge((x0 + w, y0 + r), (x0 + w, y0 + h - r), &mut pts);
    arc((x0 + w - r, y0 + h - r), 0.0, &mut pts);
    edge((x0 + w - r, y0 + h), (x0 + r, y0 + h), &mut pts);
    arc((x0 + r, y0 + h - r), FRAC_PI_2, &mut pts);
    edge((x0, y0 + h - r), (x0, y0 + r), &mut pts);
    arc((x0 + r, y0 + r), PI, &mut pts);
    pts
}

fn cumulative(pts: &[(f32, f32)]) -> (Vec<f32>, f32) {
    let n = pts.len();
    let mut lens = Vec::with_capacity(n + 1);
    let mut total = 0.0;
    lens.push(0.0);
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        total += ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        lens.push(total);
    }
    (lens, total)
}

/// The point at fraction `t` (0..1, wraps) of the way around a rounded rectangle.
pub fn perimeter_point(bounds: Bounds<Pixels>, radius: f32, t: f32) -> Point<Pixels> {
    let pts = rounded_outline(bounds, radius, 4.0);
    let (lens, total) = cumulative(&pts);
    let target = t.rem_euclid(1.0) * total;
    let n = pts.len();
    for i in 0..n {
        if lens[i + 1] >= target {
            let seg = (lens[i + 1] - lens[i]).max(1e-4);
            let k = (target - lens[i]) / seg;
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            return point(px(a.0 + (b.0 - a.0) * k), px(a.1 + (b.1 - a.1) * k));
        }
    }
    point(px(pts[0].0), px(pts[0].1))
}

/// Stroke the inside edge of a rounded rectangle where each short segment gets its own color
/// from `color(s, p)`, `s` being the segment's position around the perimeter (0..1, clockwise
/// from the top-left) and `p` its midpoint in window space. Segments whose alpha is ~0 are
/// skipped, so a short beam is cheap.
pub fn stroke_outline_fn(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    radius: f32,
    width: f32,
    color: impl Fn(f32, Point<Pixels>) -> Hsla,
) {
    let inner = bounds.dilate(px(-width / 2.0));
    let pts = rounded_outline(inner, (radius - width / 2.0).max(0.0), 3.0);
    let (lens, total) = cumulative(&pts);
    let n = pts.len();
    for i in 0..n {
        let s = (lens[i] + lens[i + 1]) * 0.5 / total.max(1e-3);
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let c = color(s, point(px((a.0 + b.0) * 0.5), px((a.1 + b.1) * 0.5)));
        if c.a < 0.01 {
            continue;
        }
        let mut pb = PathBuilder::stroke(px(width));
        pb.move_to(point(px(a.0), px(a.1)));
        pb.line_to(point(px(b.0), px(b.1)));
        if let Ok(path) = pb.build() {
            window.paint_path(path, c);
        }
    }
}

/// A soft round glow (a blurred disc) centered on `center`. Uses GPUI's analytic shadow
/// primitive, so it is cheap and perfectly smooth.
pub fn glow(window: &mut Window, center: Point<Pixels>, radius: f32, blur: f32, color: Hsla) {
    let b = Bounds::centered_at(center, size(px(radius * 2.0), px(radius * 2.0)));
    window.paint_shadows(
        b,
        Corners::all(px(radius)),
        &[BoxShadow { color, offset: point(px(0.), px(0.)), blur_radius: px(blur), spread_radius: px(0.) }],
    );
}

/// Stroke an open polyline.
pub fn polyline(window: &mut Window, pts: &[Point<Pixels>], width: f32, color: Hsla) {
    if pts.len() < 2 {
        return;
    }
    let mut pb = PathBuilder::stroke(px(width));
    pb.move_to(pts[0]);
    for p in &pts[1..] {
        pb.line_to(*p);
    }
    if let Ok(path) = pb.build() {
        window.paint_path(path, color);
    }
}

/// Sweep profile used by sparks and beams: a bright, soft head at `head` with a fading tail of
/// length `tail` behind it (both as fractions of the perimeter). Returns 0..1.
pub fn sweep_profile(head: f32, s: f32, head_width: f32, tail: f32) -> f32 {
    let mut d = (head - s).rem_euclid(1.0);
    if d > 0.5 + tail / 2.0 {
        d -= 1.0;
    }
    let front = (-(d / head_width.max(1e-4)).powi(2)).exp();
    let k = if (0.0..tail).contains(&d) {
        let u = 1.0 - d / tail;
        u * u * (3.0 - 2.0 * u)
    } else {
        0.0
    };
    k.max(if d < 0.0 { front } else { 0.0 })
}
