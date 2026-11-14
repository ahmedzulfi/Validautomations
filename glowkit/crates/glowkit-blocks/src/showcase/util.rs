//! Small painting / interaction helpers shared by the showcase widgets.

use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{pos2, vec2, Align2, Color32, FontId, Galley, Id, Painter, Pos2, Rect, Shape, Stroke, Ui, Vec2};
use std::f32::consts::{PI, TAU};
use std::sync::Arc;

/// Paint text twice with a hairline offset to fake a bolder weight (the default font is light).
pub(crate) fn bold_text(p: &Painter, pos: Pos2, anchor: Align2, text: &str, font: FontId, color: Color32) -> Rect {
    p.text(pos + vec2(0.45, 0.0), anchor, text, font.clone(), color);
    p.text(pos, anchor, text, font, color)
}

/// Seconds since the widget was last interacted with (`active` = hovered / dragged / focused).
pub(crate) fn idle_time(ui: &Ui, id: Id, active: bool) -> f32 {
    let now = ui.input(|i| i.time);
    let last = glowkit_core::with_state(
        ui,
        id.with("gk_idle"),
        || now - 100.0,
        |v| {
            if active {
                *v = now;
            }
            *v
        },
    );
    (now - last) as f32
}

/// Add an anti-aliased filled circle (octagon-ish fan with a feather ring) to a mesh.
pub(crate) fn mesh_dot(m: &mut Mesh, c: Pos2, r: f32, color: Color32) {
    let n = if r < 2.5 {
        8
    } else if r < 6.0 {
        14
    } else {
        24
    };
    let base = m.vertices.len() as u32;
    m.colored_vertex(c, color);
    let inner = (r - 0.5).max(0.1);
    let outer = r + 0.5;
    for i in 0..n {
        let a = i as f32 / n as f32 * TAU;
        let d = vec2(a.cos(), a.sin());
        m.colored_vertex(c + d * inner, color);
        m.colored_vertex(c + d * outer, Color32::TRANSPARENT);
    }
    for i in 0..n as u32 {
        let j = (i + 1) % n as u32;
        let (a, ao, b, bo) = (base + 1 + i * 2, base + 2 + i * 2, base + 1 + j * 2, base + 2 + j * 2);
        m.add_triangle(base, a, b);
        m.add_triangle(a, ao, bo);
        m.add_triangle(a, bo, b);
    }
}

/// Add an axis-aligned quad to a mesh.
pub(crate) fn mesh_rect(m: &mut Mesh, r: Rect, color: Color32) {
    let i = m.vertices.len() as u32;
    for p in [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()] {
        m.vertices.push(Vertex { pos: p, uv: WHITE_UV, color });
    }
    m.add_triangle(i, i + 1, i + 2);
    m.add_triangle(i, i + 2, i + 3);
}

/// Paint the four corner regions of `rect` that lie outside a rounded rect of radius `r`
/// (used to round the corners of clipped content).
pub(crate) fn corner_mask(p: &Painter, rect: Rect, r: f32, color: Color32) {
    let r = r.min(rect.width() / 2.0).min(rect.height() / 2.0);
    if r <= 0.5 {
        return;
    }
    let mut m = Mesh::default();
    let corners = [
        (rect.left_top(), pos2(rect.min.x + r, rect.min.y + r), PI),
        (rect.right_top(), pos2(rect.max.x - r, rect.min.y + r), 1.5 * PI),
        (rect.right_bottom(), pos2(rect.max.x - r, rect.max.y - r), 0.0),
        (rect.left_bottom(), pos2(rect.min.x + r, rect.max.y - r), 0.5 * PI),
    ];
    let segs = 12;
    for (corner, c, a0) in corners {
        let base = m.vertices.len() as u32;
        m.colored_vertex(corner, color);
        for i in 0..=segs {
            let a = a0 + 0.5 * PI * i as f32 / segs as f32;
            // pull the arc very slightly inward for an AA-safe overlap
            m.colored_vertex(c + vec2(a.cos(), a.sin()) * (r - 0.3), color);
        }
        for i in 0..segs as u32 {
            m.add_triangle(base, base + 1 + i, base + 2 + i);
        }
    }
    p.add(Shape::mesh(m));
}

/// A ring between two rounded rects (`outer` must contain `inner`): fills the bezel of a device.
pub(crate) fn rounded_ring(p: &Painter, outer: Rect, r_out: f32, inner: Rect, r_in: f32, color: Color32) {
    let segs = 12;
    let a = glowkit_core::paint::rounded_rect_points(outer, r_out, segs);
    let b = glowkit_core::paint::rounded_rect_points(inner, r_in, segs);
    let mut m = Mesh::default();
    let n = a.len();
    for i in 0..n {
        m.colored_vertex(a[i], color);
        m.colored_vertex(b[i], color);
    }
    for i in 0..n as u32 {
        let j = (i + 1) % n as u32;
        m.add_triangle(i * 2, i * 2 + 1, j * 2 + 1);
        m.add_triangle(i * 2, j * 2 + 1, j * 2);
    }
    p.add(Shape::mesh(m));
}

/// Paint a galley with a per-position color, subdividing every glyph quad into `n x n` cells so
/// radial masks and spotlights stay smooth even on huge text.
pub(crate) fn galley_fine(p: &Painter, pos: Pos2, g: &Arc<Galley>, n: usize, f: impl Fn(Pos2) -> Color32) {
    let [tw, th] = p.fonts(|fo| fo.font_image_size());
    let norm = vec2(1.0 / tw.max(1) as f32, 1.0 / th.max(1) as f32);
    let ppp = p.pixels_per_point();
    let pos = pos2((pos.x * ppp).round() / ppp, (pos.y * ppp).round() / ppp);
    let n = n.max(1);
    for row in &g.rows {
        let src = &row.visuals.mesh;
        if src.is_empty() || src.vertices.len() % 4 != 0 {
            continue;
        }
        let off = pos.to_vec2() + row.pos.to_vec2();
        let mut m = Mesh::default();
        for q in src.vertices.chunks(4) {
            let (p0, p1, p2) = (q[0].pos + off, q[1].pos + off, q[2].pos + off);
            let (u0, u1, u2) = (q[0].uv, q[1].uv, q[2].uv);
            let alpha = q[0].color.a() as f32 / 255.0;
            let base = m.vertices.len() as u32;
            for y in 0..=n {
                for x in 0..=n {
                    let (sx, sy) = (x as f32 / n as f32, y as f32 / n as f32);
                    let pp = p0 + (p1 - p0) * sx + (p2 - p0) * sy;
                    let uv = u0 + (u1 - u0) * sx + (u2 - u0) * sy;
                    m.vertices.push(Vertex {
                        pos: pp,
                        uv: pos2(uv.x * norm.x, uv.y * norm.y),
                        color: f(pp).gamma_multiply(alpha),
                    });
                }
            }
            let w = (n + 1) as u32;
            for y in 0..n as u32 {
                for x in 0..n as u32 {
                    let i = base + y * w + x;
                    m.add_triangle(i, i + 1, i + w + 1);
                    m.add_triangle(i, i + w + 1, i + w);
                }
            }
        }
        p.add(Shape::mesh(m));
    }
}

/// A gradient-filled avatar circle with initials.
pub(crate) fn avatar(p: &Painter, c: Pos2, r: f32, colors: [Color32; 2], initials: &str) {
    let pts = glowkit_core::paint::circle_points(c, r, 48);
    let rect = Rect::from_center_size(c, Vec2::splat(r * 2.0));
    let stops = [colors[0], colors[1]];
    glowkit_core::paint::fill_convex_fn(p, &pts, 3, glowkit_core::paint::linear_fn(rect, 45.0, &stops));
    // soft top highlight
    glowkit_core::paint::glow(p, c - vec2(r * 0.3, r * 0.4), r * 0.9, Color32::from_white_alpha(28));
    if !initials.is_empty() {
        bold_text(p, c, Align2::CENTER_CENTER, initials, FontId::proportional(r * 0.78), Color32::WHITE);
    }
}

/// A checkmark drawn with `progress` 0..1 (stroke grows along the tick).
pub(crate) fn check(p: &Painter, c: Pos2, s: f32, stroke: Stroke, progress: f32) {
    let a = c + vec2(-0.42, 0.02) * s;
    let b = c + vec2(-0.12, 0.32) * s;
    let d = c + vec2(0.45, -0.3) * s;
    let l1 = a.distance(b);
    let l2 = b.distance(d);
    let len = (l1 + l2) * progress.clamp(0.0, 1.0);
    if len <= 0.0 {
        return;
    }
    let mut pts = vec![a];
    if len <= l1 {
        pts.push(a.lerp(b, len / l1));
    } else {
        pts.push(b);
        pts.push(b.lerp(d, (len - l1) / l2));
    }
    p.add(Shape::line(pts, stroke));
}

/// Simple chevron arrow pointing in `dir` (unit vector).
pub(crate) fn chevron(p: &Painter, c: Pos2, s: f32, dir: Vec2, stroke: Stroke) {
    let n = dir.rot90();
    let tip = c + dir * s * 0.25;
    p.add(Shape::line(vec![tip - dir * s * 0.5 + n * s * 0.5, tip, tip - dir * s * 0.5 - n * s * 0.5], stroke));
}

/// Allocate a block of `size` centered horizontally in the available width, with `top` spacing.
#[cfg_attr(not(feature = "demos"), allow(dead_code))]
pub(crate) fn centered_rect(ui: &mut Ui, size: Vec2, top: f32) -> Rect {
    let avail = ui.available_rect_before_wrap();
    let x = avail.center().x - size.x / 2.0;
    let r = Rect::from_min_size(pos2(x, avail.min.y + top), size);
    ui.allocate_rect(Rect::from_min_max(avail.min, pos2(avail.max.x, r.max.y)), egui::Sense::hover());
    r
}

/// Rounded-rect outline of a heart shape for like buttons (convex-safe: returns two lobes + tip as points).
pub(crate) fn heart_points(c: Pos2, s: f32) -> Vec<Pos2> {
    (0..48)
        .map(|i| {
            let t = i as f32 / 48.0 * TAU;
            let x = 16.0 * t.sin().powi(3);
            let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
            c + vec2(x, -y) * (s / 34.0)
        })
        .collect()
}

/// Rotate `p` around `c` by `a` radians.
pub(crate) fn rotate(p: Pos2, c: Pos2, a: f32) -> Pos2 {
    let (s, co) = a.sin_cos();
    let d = p - c;
    c + vec2(d.x * co - d.y * s, d.x * s + d.y * co)
}

/// Fill a polygon that is star-shaped around `center` (every vertex visible from it), e.g. a heart.
pub(crate) fn fill_star_shaped(p: &Painter, center: Pos2, pts: &[Pos2], color: Color32) {
    let mut m = Mesh::default();
    m.colored_vertex(center, color);
    for q in pts {
        m.colored_vertex(*q, color);
    }
    let n = pts.len() as u32;
    for i in 0..n {
        m.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    p.add(Shape::mesh(m));
    p.add(Shape::closed_line(pts.to_vec(), Stroke::new(0.8, color)));
}

/// A grid mesh over a rounded rect: vertices outside the rounded corners are projected onto
/// the outline, so per-vertex effects get smooth rounded edges.
pub(crate) fn rounded_grid(p: &Painter, rect: Rect, radius: f32, cell: f32, f: impl Fn(Pos2) -> Color32) {
    let cols = (rect.width() / cell).ceil().max(1.0) as usize;
    let rows = (rect.height() / cell).ceil().max(1.0) as usize;
    let mut m = Mesh::default();
    m.reserve_vertices((cols + 1) * (rows + 1));
    for y in 0..=rows {
        for x in 0..=cols {
            let mut q = pos2(
                rect.min.x + rect.width() * x as f32 / cols as f32,
                rect.min.y + rect.height() * y as f32 / rows as f32,
            );
            let c = pos2(
                q.x.clamp(rect.min.x + radius, rect.max.x - radius),
                q.y.clamp(rect.min.y + radius, rect.max.y - radius),
            );
            let d = q - c;
            if d.length() > radius {
                q = c + d.normalized() * radius;
            }
            m.colored_vertex(q, f(q));
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
    p.add(Shape::mesh(m));
}
