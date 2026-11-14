//! Painting primitives egui doesn't ship: gradient fills for any convex shape, conic and linear
//! gradients, radial glows, soft shadows, gradient strokes, gradient text, and a cheap blur.

use crate::color::{gradient, gradient_cyclic, with_alpha};
use egui::epaint::{Mesh, Shadow, Vertex, WHITE_UV};
use egui::{pos2, vec2, Color32, CornerRadius, Galley, Painter, Pos2, Rect, Shape, Stroke, Vec2};
use std::f32::consts::TAU;
use std::sync::Arc;

/// Outline of a rounded rectangle (clockwise), `segs` points per corner.
pub fn rounded_rect_points(rect: Rect, radius: f32, segs: usize) -> Vec<Pos2> {
    let r = radius.min(rect.width() / 2.0).min(rect.height() / 2.0).max(0.0);
    let segs = segs.max(1);
    let mut pts = Vec::with_capacity(segs * 4 + 4);
    let corners = [
        (pos2(rect.max.x - r, rect.min.y + r), -0.25),
        (pos2(rect.max.x - r, rect.max.y - r), 0.0),
        (pos2(rect.min.x + r, rect.max.y - r), 0.25),
        (pos2(rect.min.x + r, rect.min.y + r), 0.5),
    ];
    for (c, start) in corners {
        for i in 0..=segs {
            let a = (start + 0.25 * i as f32 / segs as f32) * TAU;
            pts.push(c + vec2(a.cos(), a.sin()) * r);
        }
    }
    pts
}

/// Outline of a circle.
pub fn circle_points(center: Pos2, radius: f32, n: usize) -> Vec<Pos2> {
    (0..n.max(3))
        .map(|i| {
            let a = i as f32 / n as f32 * TAU;
            center + vec2(a.cos(), a.sin()) * radius
        })
        .collect()
}

/// Subdivide a closed polyline so no edge is longer than `max_len`.
pub fn subdivide_closed(pts: &[Pos2], max_len: f32) -> Vec<Pos2> {
    let mut out = Vec::with_capacity(pts.len() * 2);
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        let n = ((a.distance(b) / max_len.max(1.0)).ceil() as usize).max(1);
        for k in 0..n {
            out.push(a.lerp(b, k as f32 / n as f32));
        }
    }
    out
}

/// Fill a **convex** outline with a color function evaluated per vertex.
///
/// The shape is meshed as concentric rings so multi-stop gradients stay accurate, and gets a
/// one-pixel transparent feather for anti-aliasing.
pub fn fill_convex_fn(painter: &Painter, outline: &[Pos2], rings: usize, f: impl Fn(Pos2) -> Color32) {
    if outline.len() < 3 {
        return;
    }
    let outline = subdivide_closed(outline, 24.0);
    let n = outline.len();
    let c = outline.iter().fold(Vec2::ZERO, |acc, p| acc + p.to_vec2()) / n as f32;
    let c = c.to_pos2();
    let rings = rings.max(1);
    let mut mesh = Mesh::default();
    mesh.reserve_vertices(n * (rings + 1) + 1);
    mesh.colored_vertex(c, f(c));
    for k in 1..=rings {
        let s = k as f32 / rings as f32;
        for p in &outline {
            let q = c + (*p - c) * s;
            mesh.colored_vertex(q, f(q));
        }
    }
    // center fan
    for i in 0..n {
        mesh.add_triangle(0, 1 + i as u32, 1 + ((i + 1) % n) as u32);
    }
    // ring strips
    for k in 1..rings {
        let a0 = 1 + (k - 1) * n;
        let b0 = 1 + k * n;
        for i in 0..n {
            let j = (i + 1) % n;
            let (a, an, b, bn) = ((a0 + i) as u32, (a0 + j) as u32, (b0 + i) as u32, (b0 + j) as u32);
            mesh.add_triangle(a, b, bn);
            mesh.add_triangle(a, bn, an);
        }
    }
    // feather
    let outer0 = 1 + (rings - 1) * n;
    let f0 = mesh.vertices.len();
    for i in 0..n {
        let p = outline[i];
        let prev = outline[(i + n - 1) % n];
        let next = outline[(i + 1) % n];
        let normal = ((p - prev).rot90().normalized() + (next - p).rot90().normalized()).normalized();
        let dir = if normal.dot(p - c) < 0.0 { -normal } else { normal };
        let col = mesh.vertices[outer0 + i].color;
        mesh.colored_vertex(p + dir * 0.75, Color32::TRANSPARENT.lerp_to_gamma(col, 0.0));
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let (a, an) = ((outer0 + i) as u32, (outer0 + j) as u32);
        let (b, bn) = ((f0 + i) as u32, (f0 + j) as u32);
        mesh.add_triangle(a, b, bn);
        mesh.add_triangle(a, bn, an);
    }
    painter.add(Shape::mesh(mesh));
}

/// A linear gradient function across `rect` at `angle_deg` (0 = left→right, 90 = top→bottom).
pub fn linear_fn(rect: Rect, angle_deg: f32, stops: &[Color32]) -> impl Fn(Pos2) -> Color32 + '_ {
    let a = angle_deg.to_radians();
    let dir = vec2(a.cos(), a.sin());
    let half = (rect.width() * dir.x.abs() + rect.height() * dir.y.abs()) / 2.0;
    let c = rect.center();
    move |p: Pos2| gradient(stops, ((p - c).dot(dir) / half.max(1e-3)) * 0.5 + 0.5)
}

/// A radial gradient function: `stops[0]` at `center`, last stop at `radius`.
pub fn radial_fn(center: Pos2, radius: f32, stops: &[Color32]) -> impl Fn(Pos2) -> Color32 + '_ {
    move |p: Pos2| gradient(stops, p.distance(center) / radius.max(1e-3))
}

/// A conic (angular) gradient around `center`, rotated by `rotation` turns. Wraps seamlessly.
pub fn conic_fn(center: Pos2, rotation: f32, stops: &[Color32]) -> impl Fn(Pos2) -> Color32 + '_ {
    move |p: Pos2| {
        let d = p - center;
        let t = d.y.atan2(d.x) / TAU + 0.5 - rotation;
        gradient_cyclic(stops, t)
    }
}

/// Rounded rect filled with a linear gradient.
pub fn gradient_rect(painter: &Painter, rect: Rect, radius: f32, angle_deg: f32, stops: &[Color32]) {
    let pts = rounded_rect_points(rect, radius, 8);
    fill_convex_fn(painter, &pts, 6, linear_fn(rect, angle_deg, stops));
}

/// Circle filled with a radial gradient.
pub fn gradient_circle(painter: &Painter, center: Pos2, radius: f32, stops: &[Color32]) {
    let pts = circle_points(center, radius, 64);
    fill_convex_fn(painter, &pts, 8, radial_fn(center, radius, stops));
}

/// A soft radial glow: `color` at the center fading smoothly to transparent at `radius`.
pub fn glow(painter: &Painter, center: Pos2, radius: f32, color: Color32) {
    if radius <= 0.5 {
        return;
    }
    let n = 48;
    let rings = 8;
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, color);
    for k in 1..=rings {
        let s = k as f32 / rings as f32;
        let falloff = (1.0 - s).powf(2.2);
        let col = color.gamma_multiply(falloff);
        for i in 0..n {
            let a = i as f32 / n as f32 * TAU;
            mesh.colored_vertex(center + vec2(a.cos(), a.sin()) * radius * s, col);
        }
    }
    for i in 0..n {
        mesh.add_triangle(0, 1 + i as u32, 1 + ((i + 1) % n) as u32);
    }
    for k in 1..rings {
        let a0 = 1 + (k - 1) * n;
        let b0 = 1 + k * n;
        for i in 0..n {
            let j = (i + 1) % n;
            mesh.add_triangle((a0 + i) as u32, (b0 + i) as u32, (b0 + j) as u32);
            mesh.add_triangle((a0 + i) as u32, (b0 + j) as u32, (a0 + j) as u32);
        }
    }
    painter.add(Shape::mesh(mesh));
}

/// An elliptical glow (stretch a radial glow by `scale`).
pub fn glow_ellipse(painter: &Painter, center: Pos2, radii: Vec2, color: Color32) {
    let n = 48;
    let rings = 8;
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, color);
    for k in 1..=rings {
        let s = k as f32 / rings as f32;
        let col = color.gamma_multiply((1.0 - s).powf(2.2));
        for i in 0..n {
            let a = i as f32 / n as f32 * TAU;
            mesh.colored_vertex(center + vec2(a.cos() * radii.x, a.sin() * radii.y) * s, col);
        }
    }
    for i in 0..n {
        mesh.add_triangle(0, 1 + i as u32, 1 + ((i + 1) % n) as u32);
    }
    for k in 1..rings {
        let a0 = 1 + (k - 1) * n;
        let b0 = 1 + k * n;
        for i in 0..n {
            let j = (i + 1) % n;
            mesh.add_triangle((a0 + i) as u32, (b0 + i) as u32, (b0 + j) as u32);
            mesh.add_triangle((a0 + i) as u32, (b0 + j) as u32, (a0 + j) as u32);
        }
    }
    painter.add(Shape::mesh(mesh));
}

/// A blurred drop shadow / outer glow behind a rounded rect.
pub fn soft_shadow(painter: &Painter, rect: Rect, radius: f32, blur: f32, offset: Vec2, color: Color32) {
    let s = Shadow {
        offset: [offset.x.round().clamp(-127.0, 127.0) as i8, offset.y.round().clamp(-127.0, 127.0) as i8],
        blur: blur.round().clamp(0.0, 255.0) as u8,
        spread: 0,
        color,
    };
    painter.add(s.as_shape(rect, CornerRadius::same(radius.round().clamp(0.0, 255.0) as u8)));
}

/// Stroke an open or closed polyline with a per-vertex color function `f(t)`, `t` = 0..1 along
/// its length. Produces a smooth anti-aliased ribbon.
pub fn gradient_polyline(painter: &Painter, pts: &[Pos2], closed: bool, width: f32, f: impl Fn(f32) -> Color32) {
    let n = pts.len();
    if n < 2 {
        return;
    }
    let mut lens = Vec::with_capacity(n + 1);
    let mut total = 0.0;
    lens.push(0.0);
    let segs = if closed { n } else { n - 1 };
    for i in 0..segs {
        total += pts[i].distance(pts[(i + 1) % n]);
        lens.push(total);
    }
    let hw = width / 2.0;
    let feather = 0.8;
    let mut mesh = Mesh::default();
    let count = if closed { n + 1 } else { n };
    for k in 0..count {
        let i = k % n;
        let p = pts[i];
        let prev = if closed || i > 0 { pts[(i + n - 1) % n] } else { p };
        let next = if closed || i + 1 < n { pts[(i + 1) % n] } else { p };
        let d1 = (p - prev).normalized();
        let d2 = (next - p).normalized();
        let mut dir = d1 + d2;
        if dir.length_sq() < 1e-6 {
            dir = if d2.length_sq() > 0.0 { d2 } else { d1 };
        }
        let normal = dir.normalized().rot90();
        let miter = 1.0 / normal.dot((if d2.length_sq() > 0.0 { d2 } else { d1 }).rot90()).abs().max(0.3);
        let t = if total > 0.0 { lens[k.min(lens.len() - 1)] / total } else { 0.0 };
        let col = f(t);
        let w = hw * miter;
        mesh.colored_vertex(p + normal * (w + feather), Color32::TRANSPARENT);
        mesh.colored_vertex(p + normal * w, col);
        mesh.colored_vertex(p - normal * w, col);
        mesh.colored_vertex(p - normal * (w + feather), Color32::TRANSPARENT);
    }
    for k in 0..count - 1 {
        let a = (k * 4) as u32;
        let b = ((k + 1) * 4) as u32;
        for s in 0..3 {
            mesh.add_triangle(a + s, b + s, b + s + 1);
            mesh.add_triangle(a + s, b + s + 1, a + s + 1);
        }
    }
    painter.add(Shape::mesh(mesh));
}

/// Stroke a rounded rect border with `f(t)`, where `t` runs 0..1 clockwise from the top-left.
pub fn gradient_border(painter: &Painter, rect: Rect, radius: f32, width: f32, f: impl Fn(f32) -> Color32) {
    let r = rect.shrink(width / 2.0);
    let pts = subdivide_closed(&rounded_rect_points(r, (radius - width / 2.0).max(0.0), 10), 6.0);
    // rotate so t=0 starts at the top-left corner end
    gradient_polyline(painter, &pts, true, width, f);
}

/// A "comet" along a closed path: bright head at `head` (0..1 along the path) with a tail of
/// `length` (fraction of the path) fading out. Used by border beams and orbit effects.
pub fn comet_fn(head: f32, length: f32, color: Color32) -> impl Fn(f32) -> Color32 {
    move |t: f32| {
        let d = (head - t).rem_euclid(1.0);
        if d < length {
            let k = 1.0 - d / length;
            color.gamma_multiply(k * k)
        } else {
            Color32::TRANSPARENT
        }
    }
}

/// Paint a galley with a color function evaluated at every glyph vertex (gradient text).
///
/// `pos` is the top-left of the galley; `f` receives absolute screen positions.
pub fn galley_with(painter: &Painter, pos: Pos2, galley: &Arc<Galley>, f: impl Fn(Pos2) -> Color32) {
    // Galley meshes store UVs in texels; the tessellator normally normalizes them.
    let [tw, th] = painter.fonts(|fo| fo.font_image_size());
    let norm = vec2(1.0 / tw.max(1) as f32, 1.0 / th.max(1) as f32);
    let ppp = painter.pixels_per_point();
    let pos = pos2((pos.x * ppp).round() / ppp, (pos.y * ppp).round() / ppp);
    for row in &galley.rows {
        let mut mesh = row.visuals.mesh.clone();
        if mesh.is_empty() {
            continue;
        }
        let offset = pos.to_vec2() + row.pos.to_vec2();
        for v in &mut mesh.vertices {
            let p = v.pos + offset;
            let a = v.color.a();
            let c = f(p);
            *v = Vertex {
                pos: p,
                uv: pos2(v.uv.x * norm.x, v.uv.y * norm.y),
                color: c.gamma_multiply(a as f32 / 255.0),
            };
        }
        painter.add(Shape::mesh(mesh));
    }
}

/// Fake gaussian blur for a galley: draws `taps` offset copies at low alpha.
/// `radius` 0 paints the text normally.
pub fn blurred_galley(painter: &Painter, pos: Pos2, galley: &Arc<Galley>, color: Color32, radius: f32) {
    if radius < 0.3 {
        painter.galley(pos, galley.clone(), color);
        return;
    }
    let taps = 10;
    let a = (1.6 / taps as f32).min(1.0);
    for i in 0..taps {
        let ang = i as f32 / taps as f32 * TAU;
        let rr = radius * if i % 2 == 0 { 1.0 } else { 0.5 };
        painter.galley(pos + vec2(ang.cos(), ang.sin()) * rr, galley.clone(), color.gamma_multiply(a));
    }
    painter.galley(pos, galley.clone(), color.gamma_multiply((1.0 - radius / 12.0).clamp(0.15, 1.0) * 0.5));
}

/// Draw a filled quad with four corner colors (bilinear-ish via two triangles).
pub fn quad(painter: &Painter, p: [Pos2; 4], c: [Color32; 4]) {
    let mut m = Mesh::default();
    for i in 0..4 {
        m.vertices.push(Vertex { pos: p[i], uv: WHITE_UV, color: c[i] });
    }
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(m));
}

/// A grid of quads covering `rect`, colored by `f(uv)` where `uv` is in `0..1`. The workhorse of
/// shader-like backgrounds (aurora, silk, iridescence, plasma...).
pub fn shader_grid(painter: &Painter, rect: Rect, cols: usize, rows: usize, f: impl Fn(Vec2) -> Color32) {
    let cols = cols.max(1);
    let rows = rows.max(1);
    let mut m = Mesh::default();
    m.reserve_vertices((cols + 1) * (rows + 1));
    m.reserve_triangles(cols * rows * 2);
    for y in 0..=rows {
        for x in 0..=cols {
            let uv = vec2(x as f32 / cols as f32, y as f32 / rows as f32);
            let p = pos2(rect.min.x + uv.x * rect.width(), rect.min.y + uv.y * rect.height());
            m.colored_vertex(p, f(uv));
        }
    }
    let w = (cols + 1) as u32;
    for y in 0..rows as u32 {
        for x in 0..cols as u32 {
            let i = y * w + x;
            m.add_triangle(i, i + 1, i + w + 1);
            m.add_triangle(i, i + w + 1, i + w);
        }
    }
    painter.add(Shape::mesh(m));
}

/// Vertical fade mask: paints `bg` over the top and bottom `edge` points so content appears to
/// fade out. Use after painting the content.
pub fn fade_edges_vertical(painter: &Painter, rect: Rect, edge: f32, bg: Color32) {
    let t = Rect::from_min_size(rect.min, vec2(rect.width(), edge));
    let b = Rect::from_min_max(pos2(rect.min.x, rect.max.y - edge), rect.max);
    let clear = with_alpha(bg, 0.0);
    quad(painter, [t.left_top(), t.right_top(), t.right_bottom(), t.left_bottom()], [bg, bg, clear, clear]);
    quad(painter, [b.left_top(), b.right_top(), b.right_bottom(), b.left_bottom()], [clear, clear, bg, bg]);
}

/// Horizontal fade mask (for marquees).
pub fn fade_edges_horizontal(painter: &Painter, rect: Rect, edge: f32, bg: Color32) {
    let l = Rect::from_min_size(rect.min, vec2(edge, rect.height()));
    let r = Rect::from_min_max(pos2(rect.max.x - edge, rect.min.y), rect.max);
    let clear = with_alpha(bg, 0.0);
    quad(painter, [l.left_top(), l.right_top(), l.right_bottom(), l.left_bottom()], [bg, clear, clear, bg]);
    quad(painter, [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()], [clear, bg, bg, clear]);
}

/// Vignette: darkens toward the edges of `rect`.
pub fn vignette(painter: &Painter, rect: Rect, color: Color32, strength: f32) {
    let c = rect.center();
    let r = rect.size().length() / 2.0;
    shader_grid(painter, rect, 16, 16, |uv| {
        let p = pos2(rect.min.x + uv.x * rect.width(), rect.min.y + uv.y * rect.height());
        let d = (p.distance(c) / r).clamp(0.0, 1.0);
        color.gamma_multiply((d * d * strength).clamp(0.0, 1.0))
    });
}

/// Simple solid star polygon (for sparkles and ratings).
pub fn star(painter: &Painter, center: Pos2, outer: f32, inner: f32, points: usize, rotation: f32, color: Color32) {
    let n = points.max(2) * 2;
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, color);
    for i in 0..n {
        let a = rotation + i as f32 / n as f32 * TAU - TAU / 4.0;
        let r = if i % 2 == 0 { outer } else { inner };
        mesh.colored_vertex(center + vec2(a.cos(), a.sin()) * r, color);
    }
    for i in 0..n {
        mesh.add_triangle(0, 1 + i as u32, 1 + ((i + 1) % n) as u32);
    }
    painter.add(Shape::mesh(mesh));
}

/// Four-pointed sparkle (the classic "magic" glint).
pub fn sparkle(painter: &Painter, center: Pos2, size: f32, rotation: f32, color: Color32) {
    star(painter, center, size, size * 0.22, 4, rotation, color);
}

/// A thin line with round caps (convenience).
pub fn line(painter: &Painter, a: Pos2, b: Pos2, width: f32, color: Color32) {
    painter.line_segment([a, b], Stroke::new(width, color));
}

/// Quadratic bezier sampled into points.
pub fn quad_bezier(a: Pos2, ctrl: Pos2, b: Pos2, n: usize) -> Vec<Pos2> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            pos2(u * u * a.x + 2.0 * u * t * ctrl.x + t * t * b.x, u * u * a.y + 2.0 * u * t * ctrl.y + t * t * b.y)
        })
        .collect()
}

/// Cubic bezier sampled into points.
pub fn cubic_bezier(a: Pos2, c1: Pos2, c2: Pos2, b: Pos2, n: usize) -> Vec<Pos2> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            pos2(
                w[0] * a.x + w[1] * c1.x + w[2] * c2.x + w[3] * b.x,
                w[0] * a.y + w[1] * c1.y + w[2] * c2.y + w[3] * b.y,
            )
        })
        .collect()
}

/// Catmull-Rom smoothing through `pts` (used by smooth chart lines and waves).
pub fn catmull_rom(pts: &[Pos2], per_seg: usize) -> Vec<Pos2> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let mut out = Vec::with_capacity(pts.len() * per_seg);
    for i in 0..pts.len() - 1 {
        let p0 = pts[i.saturating_sub(1)];
        let p1 = pts[i];
        let p2 = pts[i + 1];
        let p3 = pts[(i + 2).min(pts.len() - 1)];
        for s in 0..per_seg {
            let t = s as f32 / per_seg as f32;
            let t2 = t * t;
            let t3 = t2 * t;
            let f = |a: f32, b: f32, c: f32, d: f32| {
                0.5 * ((2.0 * b)
                    + (-a + c) * t
                    + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
                    + (-a + 3.0 * b - 3.0 * c + d) * t3)
            };
            out.push(pos2(f(p0.x, p1.x, p2.x, p3.x), f(p0.y, p1.y, p2.y, p3.y)));
        }
    }
    out.push(*pts.last().unwrap());
    out
}

/// Monotone cubic interpolation for charts (never overshoots between data points).
pub fn monotone(pts: &[Pos2], per_seg: usize) -> Vec<Pos2> {
    let n = pts.len();
    if n < 3 {
        return pts.to_vec();
    }
    let mut d = vec![0.0f32; n - 1];
    let mut m = vec![0.0f32; n];
    for i in 0..n - 1 {
        let dx = (pts[i + 1].x - pts[i].x).max(1e-4);
        d[i] = (pts[i + 1].y - pts[i].y) / dx;
    }
    m[0] = d[0];
    m[n - 1] = d[n - 2];
    for i in 1..n - 1 {
        m[i] = if d[i - 1] * d[i] <= 0.0 { 0.0 } else { (d[i - 1] + d[i]) / 2.0 };
    }
    for i in 0..n - 1 {
        if d[i].abs() < 1e-6 {
            m[i] = 0.0;
            m[i + 1] = 0.0;
            continue;
        }
        let a = m[i] / d[i];
        let b = m[i + 1] / d[i];
        let s = a * a + b * b;
        if s > 9.0 {
            let t = 3.0 / s.sqrt();
            m[i] = t * a * d[i];
            m[i + 1] = t * b * d[i];
        }
    }
    let mut out = Vec::with_capacity(n * per_seg);
    for i in 0..n - 1 {
        let (p0, p1) = (pts[i], pts[i + 1]);
        let h = p1.x - p0.x;
        for s in 0..per_seg {
            let t = s as f32 / per_seg as f32;
            let (t2, t3) = (t * t, t * t * t);
            let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
            let h10 = t3 - 2.0 * t2 + t;
            let h01 = -2.0 * t3 + 3.0 * t2;
            let h11 = t3 - t2;
            out.push(pos2(p0.x + h * t, h00 * p0.y + h10 * h * m[i] + h01 * p1.y + h11 * h * m[i + 1]));
        }
    }
    out.push(pts[n - 1]);
    out
}

/// Fill the area between a polyline and a baseline `y` with a vertical gradient (top → bottom).
pub fn area_fill(painter: &Painter, pts: &[Pos2], baseline: f32, top: Color32, bottom: Color32) {
    if pts.len() < 2 {
        return;
    }
    let min_y = pts.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let span = (baseline - min_y).abs().max(1.0);
    let mut m = Mesh::default();
    for p in pts {
        let t = ((p.y - min_y) / span).clamp(0.0, 1.0);
        m.colored_vertex(*p, crate::color::mix(top, bottom, t));
        m.colored_vertex(pos2(p.x, baseline), bottom);
    }
    for i in 0..pts.len() as u32 - 1 {
        let a = i * 2;
        m.add_triangle(a, a + 2, a + 3);
        m.add_triangle(a, a + 3, a + 1);
    }
    painter.add(Shape::mesh(m));
}

/// Annular sector (donut slice), angles in radians, 0 = 3 o'clock, clockwise.
pub fn arc_band(painter: &Painter, center: Pos2, r_in: f32, r_out: f32, a0: f32, a1: f32, color: Color32) {
    let steps = (((a1 - a0).abs() / TAU) * 128.0).ceil().max(2.0) as usize;
    let mut m = Mesh::default();
    for i in 0..=steps {
        let a = a0 + (a1 - a0) * i as f32 / steps as f32;
        let d = vec2(a.cos(), a.sin());
        m.colored_vertex(center + d * r_out, color);
        m.colored_vertex(center + d * r_in, color);
    }
    for i in 0..steps as u32 {
        let k = i * 2;
        m.add_triangle(k, k + 2, k + 3);
        m.add_triangle(k, k + 3, k + 1);
    }
    painter.add(Shape::mesh(m));
}

/// Arc stroke with round-ish ends (progress rings, gauges, spinners).
pub fn arc(painter: &Painter, center: Pos2, radius: f32, a0: f32, a1: f32, width: f32, color: Color32) {
    let steps = (((a1 - a0).abs() / TAU) * 96.0).ceil().max(2.0) as usize;
    let pts: Vec<Pos2> = (0..=steps)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / steps as f32;
            center + vec2(a.cos(), a.sin()) * radius
        })
        .collect();
    gradient_polyline(painter, &pts, false, width, |_| color);
    painter.circle_filled(pts[0], width / 2.0, color);
    painter.circle_filled(*pts.last().unwrap(), width / 2.0, color);
}

/// Arc stroke with a gradient along its length.
pub fn arc_gradient(
    painter: &Painter,
    center: Pos2,
    radius: f32,
    a0: f32,
    a1: f32,
    width: f32,
    f: impl Fn(f32) -> Color32,
) {
    let steps = (((a1 - a0).abs() / TAU) * 128.0).ceil().max(2.0) as usize;
    let pts: Vec<Pos2> = (0..=steps)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / steps as f32;
            center + vec2(a.cos(), a.sin()) * radius
        })
        .collect();
    gradient_polyline(painter, &pts, false, width, f);
}
