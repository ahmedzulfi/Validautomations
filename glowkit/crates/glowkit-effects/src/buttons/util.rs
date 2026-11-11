//! Shared painting and layout helpers for buttons and cards (crate-internal).

use egui::epaint::Mesh;
use egui::{pos2, vec2, Color32, CornerRadius, FontId, Galley, Painter, Pos2, Rect, Response, Role, Sense, Shape, Ui, Vec2, WidgetInfo};
use glowkit_core::paint::{rounded_rect_points, subdivide_closed};
use std::f32::consts::TAU;
use std::sync::Arc;

/// `CornerRadius` from a float, clamped to what egui can store.
pub(crate) fn cr(r: f32) -> CornerRadius {
    CornerRadius::same(r.round().clamp(0.0, 255.0) as u8)
}

/// Rounded-rect outline, subdivided finely enough for smooth per-vertex color.
pub(crate) fn outline(rect: Rect, radius: f32, max_seg: f32) -> Vec<Pos2> {
    let segs = ((radius / 2.0).ceil() as usize).clamp(3, 16);
    subdivide_closed(&rounded_rect_points(rect, radius, segs), max_seg)
}

/// A closed anti-aliased ribbon along `pts` with a per-vertex color `f(point, t)`, where `t` is
/// 0..1 along the path length.
pub(crate) fn ribbon(painter: &Painter, pts: &[Pos2], width: f32, f: impl Fn(Pos2, f32) -> Color32) {
    let n = pts.len();
    if n < 3 {
        return;
    }
    let mut lens = Vec::with_capacity(n + 1);
    let mut total = 0.0;
    lens.push(0.0);
    for i in 0..n {
        total += pts[i].distance(pts[(i + 1) % n]);
        lens.push(total);
    }
    let hw = width / 2.0;
    let feather = 0.8;
    let mut mesh = Mesh::default();
    mesh.reserve_vertices((n + 1) * 4);
    for k in 0..=n {
        let i = k % n;
        let p = pts[i];
        let prev = pts[(i + n - 1) % n];
        let next = pts[(i + 1) % n];
        let d1 = (p - prev).normalized();
        let d2 = (next - p).normalized();
        let mut dir = d1 + d2;
        if dir.length_sq() < 1e-6 {
            dir = d2;
        }
        let normal = dir.normalized().rot90();
        let miter = 1.0 / normal.dot(d2.rot90()).abs().max(0.3);
        let t = lens[k] / total.max(1e-3);
        let col = f(p, t);
        let w = hw * miter;
        mesh.colored_vertex(p + normal * (w + feather), Color32::TRANSPARENT);
        mesh.colored_vertex(p + normal * w, col);
        mesh.colored_vertex(p - normal * w, col);
        mesh.colored_vertex(p - normal * (w + feather), Color32::TRANSPARENT);
    }
    for k in 0..n as u32 {
        let a = k * 4;
        let b = (k + 1) * 4;
        for s in 0..3 {
            mesh.add_triangle(a + s, b + s, b + s + 1);
            mesh.add_triangle(a + s, b + s + 1, a + s + 1);
        }
    }
    painter.add(Shape::mesh(mesh));
}

/// Stroke the border of a rounded rect (inside `rect`) with a color per point.
pub(crate) fn border_fn(painter: &Painter, rect: Rect, radius: f32, width: f32, f: impl Fn(Pos2, f32) -> Color32) {
    let r = rect.shrink(width / 2.0);
    let pts = outline(r, (radius - width / 2.0).max(0.0), 4.0);
    ribbon(painter, &pts, width, f);
}

/// Point on a rounded-rect perimeter at `t` (0..1), starting at the top edge and running
/// clockwise. Uses the same parametrization as [`border_fn`] (approximately).
pub(crate) fn perimeter_point(rect: Rect, radius: f32, t: f32) -> Pos2 {
    let pts = outline(rect, radius, 4.0);
    let n = pts.len();
    let mut lens = Vec::with_capacity(n + 1);
    let mut total = 0.0;
    lens.push(0.0);
    for i in 0..n {
        total += pts[i].distance(pts[(i + 1) % n]);
        lens.push(total);
    }
    let target = t.rem_euclid(1.0) * total;
    for i in 0..n {
        if lens[i + 1] >= target {
            let seg = (lens[i + 1] - lens[i]).max(1e-4);
            return pts[i].lerp(pts[(i + 1) % n], (target - lens[i]) / seg);
        }
    }
    pts[0]
}

/// Clip a polygon against a **convex** clip polygon (Sutherland-Hodgman). Both may be in
/// either winding order.
pub(crate) fn clip_convex(subject: &[Pos2], clip: &[Pos2]) -> Vec<Pos2> {
    if subject.len() < 3 || clip.len() < 3 {
        return Vec::new();
    }
    // orientation of the clip polygon
    let mut area = 0.0;
    for i in 0..clip.len() {
        let a = clip[i];
        let b = clip[(i + 1) % clip.len()];
        area += a.x * b.y - b.x * a.y;
    }
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let inside = |a: Pos2, b: Pos2, p: Pos2| ((b - a).x * (p - a).y - (b - a).y * (p - a).x) * sign >= 0.0;
    let intersect = |a: Pos2, b: Pos2, p: Pos2, q: Pos2| {
        let r = b - a;
        let s = q - p;
        let den = r.x * s.y - r.y * s.x;
        if den.abs() < 1e-9 {
            return p;
        }
        let u = ((a - p).x * r.y - (a - p).y * r.x) / den;
        p + s * u
    };
    let mut out = subject.to_vec();
    for i in 0..clip.len() {
        if out.is_empty() {
            break;
        }
        let a = clip[i];
        let b = clip[(i + 1) % clip.len()];
        let input = std::mem::take(&mut out);
        for j in 0..input.len() {
            let p = input[j];
            let q = input[(j + 1) % input.len()];
            let pin = inside(a, b, p);
            let qin = inside(a, b, q);
            if pin {
                out.push(p);
                if !qin {
                    out.push(intersect(a, b, p, q));
                }
            } else if qin {
                out.push(intersect(a, b, p, q));
            }
        }
    }
    out
}

/// Fill the intersection of a circle and a rounded rect (ripples, expanding dots).
pub(crate) fn circle_in_rounded(painter: &Painter, rect: Rect, radius: f32, center: Pos2, r: f32, color: Color32) {
    if r <= 0.3 || color.a() == 0 {
        return;
    }
    let n = ((r * 0.6) as usize).clamp(24, 96);
    let circle = glowkit_core::paint::circle_points(center, r, n);
    let clip = rounded_rect_points(rect, radius, 10);
    let poly = clip_convex(&circle, &clip);
    if poly.len() >= 3 {
        fill_poly(painter, &poly, color);
    }
}

/// Fill a convex polygon with a solid color and an anti-aliased edge.
pub(crate) fn fill_poly(painter: &Painter, poly: &[Pos2], color: Color32) {
    glowkit_core::paint::fill_convex_fn(painter, poly, 1, |_| color);
}

/// Fill the part of a rounded rect below `top(x)`, column by column (liquid fills, waves).
/// `color(y_fraction)` gives the fill color by height.
pub(crate) fn fill_below(painter: &Painter, rect: Rect, radius: f32, cols: usize, top: impl Fn(f32) -> f32, color: impl Fn(f32) -> Color32) {
    let r = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let cols = cols.max(2);
    let mut mesh = Mesh::default();
    let col_range = |x: f32| -> (f32, f32) {
        let dx = if x < rect.min.x + r {
            rect.min.x + r - x
        } else if x > rect.max.x - r {
            x - (rect.max.x - r)
        } else {
            0.0
        };
        let inset = r - (r * r - dx.min(r) * dx.min(r)).max(0.0).sqrt();
        (rect.min.y + inset, rect.max.y - inset)
    };
    let yf = |y: f32| ((y - rect.min.y) / rect.height().max(1.0)).clamp(0.0, 1.0);
    for i in 0..=cols {
        let x = rect.min.x + rect.width() * i as f32 / cols as f32;
        let (y0, y1) = col_range(x);
        let ty = top(x).clamp(y0, y1);
        let c0 = color(yf(ty));
        let feather = if ty > y0 + 0.5 { 0.9 } else { 0.0 };
        mesh.colored_vertex(pos2(x, ty - feather), Color32::TRANSPARENT);
        mesh.colored_vertex(pos2(x, ty), c0);
        mesh.colored_vertex(pos2(x, y1), color(1.0));
    }
    for i in 0..cols as u32 {
        let a = i * 3;
        let b = (i + 1) * 3;
        mesh.add_triangle(a, b, b + 1);
        mesh.add_triangle(a, b + 1, a + 1);
        mesh.add_triangle(a + 1, b + 1, b + 2);
        mesh.add_triangle(a + 1, b + 2, a + 2);
    }
    painter.add(Shape::mesh(mesh));
}

/// Lay out a single line of text.
pub(crate) fn text_galley(ui: &Ui, text: &str, size: f32, color: Color32) -> Arc<Galley> {
    ui.painter().layout_no_wrap(text.to_owned(), FontId::proportional(size), color)
}

/// Paint a galley centered on `center`, pixel-snapped.
pub(crate) fn galley_centered(painter: &Painter, galley: &Arc<Galley>, center: Pos2, color: Color32) {
    let ppp = painter.pixels_per_point();
    let p = center - galley.size() / 2.0;
    let p = pos2((p.x * ppp).round() / ppp, (p.y * ppp).round() / ppp);
    painter.galley_with_override_text_color(p, galley.clone(), color);
}

/// Allocate a button: returns the rect and a click response with accessibility info and a hand
/// cursor.
pub(crate) fn alloc_button(ui: &mut Ui, size: Vec2, label: &str) -> (Rect, Response) {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let enabled = ui.is_enabled();
    let label_owned = label.to_owned();
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, &label_owned));
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    (rect, resp)
}

/// A smoothed 0..1 hover value for widget `id`.
pub(crate) fn hover_t(ui: &Ui, id: egui::Id, on: bool, speed: f32) -> f32 {
    glowkit_core::anim::follow(ui, id.with("hover_t"), if on { 1.0 } else { 0.0 }, speed)
}

/// A smoothed 0..1 press value for widget `id`.
pub(crate) fn press_t(ui: &Ui, id: egui::Id, on: bool) -> f32 {
    glowkit_core::anim::follow(ui, id.with("press_t"), if on { 1.0 } else { 0.0 }, 22.0)
}

/// Right-pointing arrow (shaft + head) centered on `center`.
pub(crate) fn arrow_right(painter: &Painter, center: Pos2, len: f32, width: f32, color: Color32, shaft: f32) {
    let head = len * 0.36;
    let tip = center + vec2(len / 2.0, 0.0);
    let stroke = egui::Stroke::new(width, color);
    if shaft > 0.01 {
        let start = tip - vec2(len * shaft, 0.0);
        painter.line_segment([start, tip], stroke);
    }
    painter.add(Shape::line(vec![tip + vec2(-head, -head), tip, tip + vec2(-head, head)], stroke));
}

/// A check mark drawn up to `progress` (0..1) of its length.
pub(crate) fn check_mark(painter: &Painter, center: Pos2, size: f32, width: f32, color: Color32, progress: f32) {
    let a = center + vec2(-0.5, 0.02) * size;
    let b = center + vec2(-0.14, 0.36) * size;
    let c = center + vec2(0.52, -0.34) * size;
    let l1 = a.distance(b);
    let l2 = b.distance(c);
    let d = progress.clamp(0.0, 1.0) * (l1 + l2);
    if d <= 0.01 {
        return;
    }
    let stroke = egui::Stroke::new(width, color);
    if d <= l1 {
        painter.add(Shape::line(vec![a, a.lerp(b, d / l1)], stroke));
    } else {
        painter.add(Shape::line(vec![a, b, b.lerp(c, (d - l1) / l2)], stroke));
    }
}

/// Spinner arc rotating with time.
pub(crate) fn spinner(painter: &Painter, center: Pos2, radius: f32, width: f32, color: Color32, time: f32) {
    let a0 = time * TAU * 1.1;
    let sweep = TAU * (0.55 + 0.2 * (time * 2.3).sin());
    glowkit_core::paint::arc_gradient(painter, center, radius, a0, a0 + sweep, width, |t| color.gamma_multiply(t.powf(0.8)));
}
