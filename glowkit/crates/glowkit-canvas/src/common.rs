//! Shared geometry, vector icons and small chrome helpers used by every canvas component.

use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Id, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};
use glowkit_core::{color::with_alpha, paint, Theme};
use std::f32::consts::TAU;

// ---------------------------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------------------------

/// Total length of an open polyline.
pub fn polyline_length(pts: &[Pos2]) -> f32 {
    pts.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// Point (and unit tangent) at arc-length `dist` along an open polyline (clamped to its ends).
pub fn point_along(pts: &[Pos2], dist: f32) -> (Pos2, Vec2) {
    if pts.is_empty() {
        return (Pos2::ZERO, Vec2::X);
    }
    if pts.len() == 1 {
        return (pts[0], Vec2::X);
    }
    let mut left = dist.max(0.0);
    for w in pts.windows(2) {
        let l = w[0].distance(w[1]);
        if left <= l && l > 0.0 {
            let dir = (w[1] - w[0]) / l;
            return (w[0] + dir * left, dir);
        }
        left -= l;
    }
    let n = pts.len();
    let dir = (pts[n - 1] - pts[n - 2]).normalized();
    (pts[n - 1], if dir.is_finite() { dir } else { Vec2::X })
}

/// Distance from `p` to the segment `a..b`.
pub fn dist_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let l2 = ab.length_sq();
    if l2 <= f32::EPSILON {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// Distance from `p` to an open polyline.
pub fn dist_to_polyline(p: Pos2, pts: &[Pos2]) -> f32 {
    match pts.len() {
        0 => f32::INFINITY,
        1 => p.distance(pts[0]),
        _ => pts.windows(2).map(|w| dist_to_segment(p, w[0], w[1])).fold(f32::INFINITY, f32::min),
    }
}

/// Rotate `p` around `c` by `angle` radians.
pub fn rotate_about(p: Pos2, c: Pos2, angle: f32) -> Pos2 {
    if angle == 0.0 {
        return p;
    }
    let (s, co) = angle.sin_cos();
    let d = p - c;
    c + vec2(d.x * co - d.y * s, d.x * s + d.y * co)
}

/// Point-in-polygon test (even-odd rule).
pub fn point_in_polygon(p: Pos2, poly: &[Pos2]) -> bool {
    let mut inside = false;
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Bounding rect of a set of points (`Rect::NOTHING` when empty).
pub fn bounds_of(pts: impl IntoIterator<Item = Pos2>) -> Rect {
    let mut r = Rect::NOTHING;
    for p in pts {
        r.extend_with(p);
    }
    r
}

/// Snap `v` to the nearest multiple of `step` (no-op when `step <= 0`).
pub fn snap(v: f32, step: f32) -> f32 {
    if step > 0.0 {
        (v / step).round() * step
    } else {
        v
    }
}

/// Orthogonal route between two points leaving `a` along `dir_a` and entering `b` against
/// `dir_b` (both axis-aligned unit vectors), with corners rounded by `radius`.
pub fn orthogonal_route(a: Pos2, dir_a: Vec2, b: Pos2, dir_b: Vec2, offset: f32, radius: f32) -> Vec<Pos2> {
    let horizontal = dir_a.x.abs() > dir_a.y.abs();
    // work in a frame where the main axis is x
    let sw = |p: Pos2| if horizontal { p } else { pos2(p.y, p.x) };
    let swv = |v: Vec2| if horizontal { v } else { vec2(v.y, v.x) };
    let (a2, b2, da, db) = (sw(a), sw(b), swv(dir_a), swv(dir_b));
    let a1 = a2 + da * offset;
    let b1 = b2 + db * offset;
    let mut corners = vec![a2, a1];
    let forward = (b1.x - a1.x) * da.x >= 0.0;
    if forward {
        let mx = (a1.x + b1.x) * 0.5;
        corners.push(pos2(mx, a1.y));
        corners.push(pos2(mx, b1.y));
    } else {
        let my = (a1.y + b1.y) * 0.5;
        corners.push(pos2(a1.x, my));
        corners.push(pos2(b1.x, my));
    }
    corners.push(b1);
    corners.push(b2);
    corners.dedup_by(|p, q| p.distance(*q) < 0.01);
    let pts: Vec<Pos2> = corners.into_iter().map(sw).collect();
    round_corners(&pts, radius)
}

/// Replace the interior corners of a polyline with small quadratic arcs.
pub fn round_corners(pts: &[Pos2], radius: f32) -> Vec<Pos2> {
    if pts.len() < 3 || radius <= 0.0 {
        return pts.to_vec();
    }
    let mut out = vec![pts[0]];
    for i in 1..pts.len() - 1 {
        let (p0, p1, p2) = (pts[i - 1], pts[i], pts[i + 1]);
        let l1 = p0.distance(p1);
        let l2 = p1.distance(p2);
        let r = radius.min(l1 * 0.5).min(l2 * 0.5);
        if r < 0.5 {
            out.push(p1);
            continue;
        }
        let a = p1 + (p0 - p1).normalized() * r;
        let b = p1 + (p2 - p1).normalized() * r;
        out.extend(paint::quad_bezier(a, p1, b, 6));
    }
    out.push(*pts.last().unwrap_or(&Pos2::ZERO));
    out
}

// ---------------------------------------------------------------------------------------------
// Painting
// ---------------------------------------------------------------------------------------------

/// Stroke a polyline as dashes. `offset` scrolls the dash pattern (animate it for "flow").
pub fn dashed_polyline(painter: &Painter, pts: &[Pos2], dash: f32, gap: f32, offset: f32, stroke: Stroke) {
    let total = polyline_length(pts);
    if total <= 0.0 || dash <= 0.0 {
        return;
    }
    let period = dash + gap;
    let mut s = -(offset.rem_euclid(period));
    while s < total {
        let a = s.max(0.0);
        let b = (s + dash).min(total);
        if b > a {
            let mut seg = vec![point_along(pts, a).0];
            // include interior vertices so dashes bend around corners
            let mut acc = 0.0;
            for w in pts.windows(2) {
                let l = w[0].distance(w[1]);
                if acc + l > a && acc + l < b {
                    seg.push(w[1]);
                }
                acc += l;
            }
            seg.push(point_along(pts, b).0);
            painter.add(Shape::line(seg, stroke));
        }
        s += period;
    }
}

/// A filled arrow head with its tip at `tip`, pointing along `dir`.
pub fn arrow_head(painter: &Painter, tip: Pos2, dir: Vec2, size: f32, color: Color32) {
    let d = dir.normalized();
    if !d.is_finite() {
        return;
    }
    let n = vec2(-d.y, d.x);
    let base = tip - d * size;
    painter.add(Shape::convex_polygon(vec![tip, base + n * size * 0.55, base - n * size * 0.55], color, Stroke::NONE));
}

/// Quantize a font size so zooming does not fill the font atlas with hundreds of sizes.
pub fn qfont(size: f32) -> FontId {
    FontId::proportional(((size * 2.0).round() / 2.0).max(1.0))
}

/// Monospace variant of [`qfont`].
pub fn qmono(size: f32) -> FontId {
    FontId::monospace(((size * 2.0).round() / 2.0).max(1.0))
}

/// Text clipped to `max_width` with an ellipsis.
pub fn elide(painter: &Painter, text: &str, font: FontId, max_width: f32) -> String {
    let w = painter.layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE).size().x;
    if w <= max_width {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut lo, mut hi) = (0usize, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let s: String = chars[..mid].iter().collect::<String>() + "…";
        if painter.layout_no_wrap(s, font.clone(), Color32::WHITE).size().x <= max_width {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    chars[..lo].iter().collect::<String>() + "…"
}

/// A rounded pill with centered text; returns its rect.
pub fn pill(painter: &Painter, center: Pos2, text: &str, font: FontId, fg: Color32, bg: Color32, stroke: Stroke) -> Rect {
    let g = painter.layout_no_wrap(text.to_owned(), font, fg);
    let size = g.size() + vec2(14.0, 6.0);
    let r = Rect::from_center_size(center, size);
    painter.rect(r, CornerRadius::same((size.y / 2.0) as u8), bg, stroke, StrokeKind::Inside);
    painter.galley(r.center() - g.size() / 2.0, g, fg);
    r
}

/// Small avatar circle with initials.
pub fn avatar(painter: &Painter, center: Pos2, radius: f32, initials: &str, color: Color32, ring: Color32) {
    painter.circle(center, radius, color, Stroke::new(1.5, ring));
    painter.text(center, Align2::CENTER_CENTER, initials, FontId::proportional(radius * 0.85), glowkit_core::color::contrast_text(color));
}

/// Deterministic color for a string (stable across runs), picked from the theme chart palette.
pub fn color_for(t: &Theme, s: &str) -> Color32 {
    let h = s.bytes().fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619));
    t.chart[(h % t.chart.len() as u32) as usize]
}

// ---------------------------------------------------------------------------------------------
// Icons
// ---------------------------------------------------------------------------------------------

/// Vector icons painted with strokes so they look identical on every platform and scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    Select,
    Hand,
    Pen,
    Highlighter,
    Rect,
    Ellipse,
    Diamond,
    Arrow,
    Line,
    Text,
    Sticky,
    Eraser,
    Plus,
    Minus,
    Fit,
    Lock,
    Unlock,
    Undo,
    Redo,
    Trash,
    Copy,
    Map,
    Layout,
    Up,
    Down,
    ChevronLeft,
    ChevronRight,
    ChevronDown,
    Play,
    Pause,
    Help,
    Sketch,
    Fill,
    Export,
    Import,
    Magnet,
    Dots,
    Close,
    Check,
}

/// Paint `icon` centered in `rect` with `color`.
pub fn paint_icon(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let s = rect.width().min(rect.height());
    let c = rect.center();
    let u = s / 16.0; // design grid: 16x16
    let p = |x: f32, y: f32| c + vec2((x - 8.0) * u, (y - 8.0) * u);
    let st = Stroke::new((1.5 * u).max(1.0), color);
    let line = |pts: &[(f32, f32)]| {
        painter.add(Shape::line(pts.iter().map(|&(x, y)| p(x, y)).collect(), st));
    };
    let closed = |pts: &[(f32, f32)]| {
        painter.add(Shape::closed_line(pts.iter().map(|&(x, y)| p(x, y)).collect(), st));
    };
    match icon {
        Icon::Select => {
            let pts = vec![p(4.0, 2.5), p(4.0, 13.0), p(6.8, 10.4), p(8.8, 14.2), p(10.4, 13.4), p(8.5, 9.8), p(12.2, 9.6)];
            painter.add(Shape::closed_line(pts, st));
        }
        Icon::Hand => {
            line(&[(5.0, 9.0), (5.0, 4.5)]);
            line(&[(7.5, 8.0), (7.5, 3.0)]);
            line(&[(10.0, 8.0), (10.0, 3.5)]);
            line(&[(12.3, 8.5), (12.3, 5.0)]);
            line(&[(5.0, 9.0), (3.2, 7.8), (2.6, 9.0), (5.6, 13.2), (7.5, 14.2), (10.5, 14.2), (12.3, 12.0), (12.3, 8.5)]);
        }
        Icon::Pen => {
            closed(&[(3.0, 13.0), (3.6, 10.2), (11.2, 2.6), (13.4, 4.8), (5.8, 12.4)]);
            line(&[(9.8, 4.0), (12.0, 6.2)]);
        }
        Icon::Highlighter => {
            closed(&[(4.0, 12.0), (5.0, 9.0), (10.5, 3.5), (12.5, 5.5), (7.0, 11.0)]);
            line(&[(3.0, 14.0), (13.0, 14.0)]);
        }
        Icon::Rect => {
            painter.rect_stroke(Rect::from_min_max(p(2.5, 3.5), p(13.5, 12.5)), CornerRadius::same((1.5 * u) as u8), st, StrokeKind::Middle);
        }
        Icon::Ellipse => {
            painter.add(Shape::ellipse_stroke(c, vec2(5.8 * u, 4.8 * u), st));
        }
        Icon::Diamond => closed(&[(8.0, 2.0), (14.0, 8.0), (8.0, 14.0), (2.0, 8.0)]),
        Icon::Arrow => {
            line(&[(3.0, 13.0), (13.0, 3.0)]);
            line(&[(7.5, 3.0), (13.0, 3.0), (13.0, 8.5)]);
        }
        Icon::Line => line(&[(3.0, 13.0), (13.0, 3.0)]),
        Icon::Text => {
            line(&[(3.5, 4.5), (3.5, 3.0), (12.5, 3.0), (12.5, 4.5)]);
            line(&[(8.0, 3.0), (8.0, 13.0)]);
            line(&[(6.0, 13.0), (10.0, 13.0)]);
        }
        Icon::Sticky => {
            closed(&[(2.5, 2.5), (13.5, 2.5), (13.5, 9.5), (9.5, 13.5), (2.5, 13.5)]);
            line(&[(13.5, 9.5), (9.5, 9.5), (9.5, 13.5)]);
        }
        Icon::Eraser => {
            closed(&[(2.5, 10.0), (8.5, 4.0), (13.0, 8.5), (8.0, 13.5), (5.5, 13.5)]);
            line(&[(5.5, 7.0), (10.0, 11.5)]);
            line(&[(8.0, 13.5), (14.0, 13.5)]);
        }
        Icon::Plus => {
            line(&[(8.0, 3.0), (8.0, 13.0)]);
            line(&[(3.0, 8.0), (13.0, 8.0)]);
        }
        Icon::Minus => line(&[(3.0, 8.0), (13.0, 8.0)]),
        Icon::Fit => {
            line(&[(2.5, 6.0), (2.5, 2.5), (6.0, 2.5)]);
            line(&[(10.0, 2.5), (13.5, 2.5), (13.5, 6.0)]);
            line(&[(13.5, 10.0), (13.5, 13.5), (10.0, 13.5)]);
            line(&[(6.0, 13.5), (2.5, 13.5), (2.5, 10.0)]);
        }
        Icon::Lock | Icon::Unlock => {
            painter.rect_stroke(Rect::from_min_max(p(3.5, 7.5), p(12.5, 14.0)), CornerRadius::same((1.5 * u) as u8), st, StrokeKind::Middle);
            if icon == Icon::Lock {
                line(&[(5.5, 7.5), (5.5, 5.0), (6.5, 3.0), (9.5, 3.0), (10.5, 5.0), (10.5, 7.5)]);
            } else {
                line(&[(5.5, 7.5), (5.5, 5.0), (6.5, 3.0), (9.5, 3.0), (10.5, 4.5)]);
            }
        }
        Icon::Undo | Icon::Redo => {
            let f = if icon == Icon::Undo { 1.0 } else { -1.0 };
            let q = |x: f32, y: f32| (8.0 + (x - 8.0) * f, y);
            let arc: Vec<(f32, f32)> = (0..=10)
                .map(|i| {
                    let a = -TAU * 0.25 + i as f32 / 10.0 * TAU * 0.5;
                    q(8.5 + a.cos() * 4.5, 8.0 + a.sin() * 4.0)
                })
                .collect();
            let mut pts = vec![q(3.5, 4.0)];
            pts.extend(arc);
            pts.push(q(6.0, 12.0));
            line(&pts);
            line(&[q(6.0, 1.8), q(3.5, 4.0), q(6.0, 6.2)]);
        }
        Icon::Trash => {
            line(&[(2.5, 4.5), (13.5, 4.5)]);
            line(&[(6.0, 4.5), (6.5, 2.5), (9.5, 2.5), (10.0, 4.5)]);
            line(&[(4.0, 4.5), (5.0, 13.5), (11.0, 13.5), (12.0, 4.5)]);
        }
        Icon::Copy => {
            painter.rect_stroke(Rect::from_min_max(p(5.5, 5.5), p(13.5, 13.5)), CornerRadius::same((1.5 * u) as u8), st, StrokeKind::Middle);
            line(&[(3.5, 10.5), (2.5, 10.5), (2.5, 2.5), (10.5, 2.5), (10.5, 3.5)]);
        }
        Icon::Map => {
            closed(&[(2.0, 4.0), (6.0, 2.5), (10.0, 4.0), (14.0, 2.5), (14.0, 12.0), (10.0, 13.5), (6.0, 12.0), (2.0, 13.5)]);
            line(&[(6.0, 2.5), (6.0, 12.0)]);
            line(&[(10.0, 4.0), (10.0, 13.5)]);
        }
        Icon::Layout => {
            painter.rect_stroke(Rect::from_min_max(p(1.5, 6.0), p(5.5, 10.0)), CornerRadius::same(1), st, StrokeKind::Middle);
            painter.rect_stroke(Rect::from_min_max(p(10.5, 2.0), p(14.5, 6.0)), CornerRadius::same(1), st, StrokeKind::Middle);
            painter.rect_stroke(Rect::from_min_max(p(10.5, 10.0), p(14.5, 14.0)), CornerRadius::same(1), st, StrokeKind::Middle);
            line(&[(5.5, 8.0), (8.0, 8.0), (8.0, 4.0), (10.5, 4.0)]);
            line(&[(8.0, 8.0), (8.0, 12.0), (10.5, 12.0)]);
        }
        Icon::Up => line(&[(3.5, 10.0), (8.0, 5.5), (12.5, 10.0)]),
        Icon::Down | Icon::ChevronDown => line(&[(3.5, 6.0), (8.0, 10.5), (12.5, 6.0)]),
        Icon::ChevronLeft => line(&[(10.0, 3.5), (5.5, 8.0), (10.0, 12.5)]),
        Icon::ChevronRight => line(&[(6.0, 3.5), (10.5, 8.0), (6.0, 12.5)]),
        Icon::Play => {
            painter.add(Shape::convex_polygon(vec![p(4.5, 3.0), p(13.0, 8.0), p(4.5, 13.0)], color, Stroke::NONE));
        }
        Icon::Pause => {
            painter.rect_filled(Rect::from_min_max(p(4.0, 3.0), p(6.8, 13.0)), CornerRadius::same(1), color);
            painter.rect_filled(Rect::from_min_max(p(9.2, 3.0), p(12.0, 13.0)), CornerRadius::same(1), color);
        }
        Icon::Help => {
            painter.text(c, Align2::CENTER_CENTER, "?", FontId::proportional(13.0 * u), color);
        }
        Icon::Sketch => {
            let pts: Vec<(f32, f32)> = (0..=16).map(|i| (2.0 + i as f32 * 0.75, 8.0 + ((i as f32) * 1.3).sin() * 3.0)).collect();
            line(&pts);
        }
        Icon::Fill => {
            painter.rect_filled(Rect::from_min_max(p(3.0, 3.0), p(13.0, 13.0)), CornerRadius::same((2.0 * u) as u8), with_alpha(color, 0.45));
            painter.rect_stroke(Rect::from_min_max(p(3.0, 3.0), p(13.0, 13.0)), CornerRadius::same((2.0 * u) as u8), st, StrokeKind::Middle);
        }
        Icon::Export | Icon::Import => {
            line(&[(2.5, 10.0), (2.5, 13.5), (13.5, 13.5), (13.5, 10.0)]);
            line(&[(8.0, 2.5), (8.0, 10.0)]);
            if icon == Icon::Export {
                line(&[(5.0, 5.5), (8.0, 2.5), (11.0, 5.5)]);
            } else {
                line(&[(5.0, 7.0), (8.0, 10.0), (11.0, 7.0)]);
            }
        }
        Icon::Magnet => {
            line(&[(4.0, 3.0), (4.0, 9.0), (5.5, 12.0), (8.0, 13.2), (10.5, 12.0), (12.0, 9.0), (12.0, 3.0)]);
            line(&[(2.5, 5.5), (5.5, 5.5)]);
            line(&[(10.5, 5.5), (13.5, 5.5)]);
        }
        Icon::Dots => {
            for x in [3.5, 8.0, 12.5] {
                painter.circle_filled(p(x, 8.0), 1.3 * u, color);
            }
        }
        Icon::Close => {
            line(&[(4.0, 4.0), (12.0, 12.0)]);
            line(&[(12.0, 4.0), (4.0, 12.0)]);
        }
        Icon::Check => line(&[(3.0, 8.5), (6.5, 12.0), (13.0, 4.5)]),
    }
}

// ---------------------------------------------------------------------------------------------
// Chrome: floating panels and icon buttons
// ---------------------------------------------------------------------------------------------

/// Paint a floating panel (card with border and soft shadow).
pub fn panel(painter: &Painter, t: &Theme, rect: Rect, radius: f32) {
    paint::soft_shadow(painter, rect, radius, 16.0, vec2(0.0, 4.0), Color32::from_black_alpha(if t.dark { 110 } else { 28 }));
    painter.rect(rect, CornerRadius::same(radius as u8), t.popover, Stroke::new(1.0, t.border), StrokeKind::Inside);
}

/// A square icon button painted with hover/press/active states. Returns its response.
pub fn icon_button(ui: &mut Ui, id: Id, rect: Rect, icon: Icon, active: bool, tooltip: &str) -> Response {
    let t = Theme::of(ui);
    let resp = ui.interact(rect, id, Sense::click());
    let hov = ui.ctx().animate_bool_with_time(id.with("hov"), resp.hovered(), 0.12);
    let act = ui.ctx().animate_bool_with_time(id.with("act"), active, 0.15);
    let painter = ui.painter();
    let bg = if active {
        with_alpha(t.accent, 0.18 + 0.06 * hov)
    } else {
        with_alpha(t.foreground, 0.07 * hov + if resp.is_pointer_button_down_on() { 0.06 } else { 0.0 })
    };
    let r = rect.shrink(if resp.is_pointer_button_down_on() { 1.0 } else { 0.0 });
    painter.rect_filled(r, CornerRadius::same(7), bg);
    let col = t.muted_foreground.lerp_to_gamma(t.foreground, hov).lerp_to_gamma(t.accent, act);
    paint_icon(painter, r.shrink(r.width() * 0.25), icon, col);
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if tooltip.is_empty() {
        resp
    } else {
        resp.on_hover_text(tooltip)
    }
}

/// Small label chip painted directly with the painter. Returns its rect.
pub fn chip(painter: &Painter, min: Pos2, text: &str, size: f32, color: Color32) -> Rect {
    let font = FontId::proportional(size);
    let g = painter.layout_no_wrap(text.to_owned(), font, color);
    let r = Rect::from_min_size(min, g.size() + vec2(size * 0.9, size * 0.35));
    painter.rect_filled(r, CornerRadius::same((r.height() / 2.0) as u8), with_alpha(color, 0.16));
    painter.galley(r.center() - g.size() / 2.0, g, color);
    r
}

/// Draw a keycap (`⌘`, `Shift`, `A`...) and return its width.
pub fn keycap(painter: &Painter, t: &Theme, left_center: Pos2, key: &str) -> f32 {
    let font = FontId::monospace(10.5);
    let g = painter.layout_no_wrap(key.to_owned(), font, t.foreground);
    let w = (g.size().x + 10.0).max(20.0);
    let r = Rect::from_min_size(left_center - vec2(0.0, 10.0), vec2(w, 20.0));
    painter.rect(r, CornerRadius::same(5), t.muted, Stroke::new(1.0, t.border), StrokeKind::Inside);
    painter.line_segment([r.left_bottom() + vec2(3.0, -0.5), r.right_bottom() + vec2(-3.0, -0.5)], Stroke::new(1.0, with_alpha(t.foreground, 0.12)));
    painter.galley(r.center() - g.size() / 2.0, g, t.foreground);
    w
}

/// An animated keyboard-shortcut cheat sheet. `open` (0..1) drives the fade/slide.
///
/// Each entry is `(keys, description)`, where keys are separated by `+`.
pub fn shortcut_overlay(painter: &Painter, t: &Theme, area: Rect, title: &str, entries: &[(&str, &str)], open: f32) {
    if open <= 0.01 {
        return;
    }
    let cols = if entries.len() > 8 { 2 } else { 1 };
    let rows = entries.len().div_ceil(cols);
    let col_w = 250.0;
    let size = vec2(col_w * cols as f32 + 32.0, rows as f32 * 28.0 + 62.0);
    let center = area.center() + vec2(0.0, (1.0 - open) * 14.0);
    let rect = Rect::from_center_size(center, size);
    painter.rect_filled(area, CornerRadius::ZERO, Color32::from_black_alpha((70.0 * open) as u8));
    let p = painter.clone().with_clip_rect(area);
    let mut p2 = p.clone();
    p2.set_opacity(open);
    panel(&p2, t, rect, 12.0);
    p2.text(rect.left_top() + vec2(16.0, 18.0), Align2::LEFT_CENTER, title, FontId::proportional(14.0), t.foreground);
    p2.text(rect.right_top() + vec2(-16.0, 18.0), Align2::RIGHT_CENTER, "press ? to close", FontId::proportional(11.0), t.muted_foreground);
    p2.line_segment([rect.left_top() + vec2(12.0, 34.0), rect.right_top() + vec2(-12.0, 34.0)], Stroke::new(1.0, t.border));
    for (i, (keys, desc)) in entries.iter().enumerate() {
        let col = i / rows;
        let row = i % rows;
        let y = rect.top() + 52.0 + row as f32 * 28.0;
        let x0 = rect.left() + 16.0 + col as f32 * col_w;
        let mut x = x0;
        for (k, key) in keys.split('+').enumerate() {
            if k > 0 {
                x += 3.0;
            }
            x += keycap(&p2, t, pos2(x, y), key.trim());
        }
        p2.text(pos2(x0 + 104.0, y), Align2::LEFT_CENTER, *desc, FontId::proportional(12.5), t.muted_foreground);
    }
}

/// Paint a small "press ? for shortcuts" hint chip at `pos` (bottom-right anchor).
pub fn shortcut_hint_chip(painter: &Painter, t: &Theme, right_bottom: Pos2) {
    let font = FontId::proportional(11.0);
    let g = painter.layout_no_wrap("shortcuts".to_owned(), font, t.muted_foreground);
    let w = g.size().x + 40.0;
    let r = Rect::from_min_max(right_bottom - vec2(w, 26.0), right_bottom);
    painter.rect(r, CornerRadius::same(13), with_alpha(t.popover, 0.92), Stroke::new(1.0, t.border), StrokeKind::Inside);
    keycap(painter, t, pos2(r.left() + 4.0, r.center().y), "?");
    painter.galley(pos2(r.left() + 30.0, r.center().y - g.size().y / 2.0), g, t.muted_foreground);
}

/// True when the command key (ctrl / cmd) is held.
pub fn command(ui: &Ui) -> bool {
    ui.input(|i| i.modifiers.command)
}

/// Read clipboard-style events this frame: (copy, cut, paste).
pub fn clipboard_events(ui: &Ui) -> (bool, bool, bool) {
    ui.input(|i| {
        let mut out = (false, false, false);
        for e in &i.events {
            match e {
                egui::Event::Copy => out.0 = true,
                egui::Event::Cut => out.1 = true,
                egui::Event::Paste(_) => out.2 = true,
                egui::Event::Key { key, pressed: true, modifiers, .. } if modifiers.command => match key {
                    egui::Key::C => out.0 = true,
                    egui::Key::X => out.1 = true,
                    egui::Key::V => out.2 = true,
                    _ => {}
                },
                _ => {}
            }
        }
        out
    })
}

/// Was `key` pressed this frame with no command modifier?
pub fn pressed(ui: &Ui, key: egui::Key) -> bool {
    ui.input(|i| i.key_pressed(key) && !i.modifiers.command)
}

/// Was `key` pressed with the command modifier (and optionally shift)?
pub fn pressed_cmd(ui: &Ui, key: egui::Key, shift: bool) -> bool {
    ui.input(|i| i.key_pressed(key) && i.modifiers.command && i.modifiers.shift == shift)
}

/// Was the `?` key pressed (handles both `Questionmark` and `Shift+/`)?
pub fn help_pressed(ui: &Ui) -> bool {
    ui.input(|i| {
        i.events.iter().any(|e| matches!(e, egui::Event::Text(s) if s == "?"))
            || i.key_pressed(egui::Key::Questionmark)
    })
}

/// Keep Tab/arrows/Escape for this widget while it has focus.
pub fn lock_focus(ui: &Ui, id: Id) {
    ui.memory_mut(|m| {
        m.set_focus_lock_filter(id, egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true })
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polyline_helpers() {
        let pts = [pos2(0.0, 0.0), pos2(10.0, 0.0), pos2(10.0, 10.0)];
        assert!((polyline_length(&pts) - 20.0).abs() < 1e-4);
        let (p, d) = point_along(&pts, 15.0);
        assert!((p - pos2(10.0, 5.0)).length() < 1e-4);
        assert!((d - Vec2::Y).length() < 1e-4);
        assert!((dist_to_polyline(pos2(5.0, 3.0), &pts) - 3.0).abs() < 1e-4);
    }

    #[test]
    fn polygon_contains() {
        let sq = [pos2(0.0, 0.0), pos2(10.0, 0.0), pos2(10.0, 10.0), pos2(0.0, 10.0)];
        assert!(point_in_polygon(pos2(5.0, 5.0), &sq));
        assert!(!point_in_polygon(pos2(15.0, 5.0), &sq));
    }

    #[test]
    fn rotation_roundtrip() {
        let c = pos2(3.0, 4.0);
        let p = pos2(10.0, -2.0);
        let q = rotate_about(rotate_about(p, c, 0.7), c, -0.7);
        assert!((p - q).length() < 1e-4);
    }

    #[test]
    fn orthogonal_route_is_axis_aligned_ends() {
        let r = orthogonal_route(pos2(0.0, 0.0), Vec2::X, pos2(100.0, 50.0), -Vec2::X, 20.0, 0.0);
        assert_eq!(r.first().copied(), Some(pos2(0.0, 0.0)));
        assert_eq!(r.last().copied(), Some(pos2(100.0, 50.0)));
        for w in r.windows(2) {
            assert!((w[0].x - w[1].x).abs() < 1e-3 || (w[0].y - w[1].y).abs() < 1e-3);
        }
    }

    #[test]
    fn snapping() {
        assert_eq!(snap(13.0, 10.0), 10.0);
        assert_eq!(snap(16.0, 10.0), 20.0);
        assert_eq!(snap(16.0, 0.0), 16.0);
    }
}
