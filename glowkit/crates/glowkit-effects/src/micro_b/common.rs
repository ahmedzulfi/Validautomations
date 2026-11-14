//! Shared plumbing for the `micro_b` components: soft surfaces, painted line icons, avatars,
//! idle timers for autoplay and a ghost cursor.

use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Shape, Stroke, Ui, Vec2};
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::{PI, TAU};

/// Corner radius from points.
#[inline]
pub(crate) fn cr(r: f32) -> CornerRadius {
    CornerRadius::same(r.round().clamp(0.0, 255.0) as u8)
}

/// The raised "white card" color: the card in light themes, a lifted muted tone in dark ones
/// (dark themes have a card darker than the stage, which would read as a hole).
pub(crate) fn surface(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.muted, 0.07)
    } else {
        t.card
    }
}

/// A slightly darker / lighter variant of [`surface`] for hover rows and wells.
pub(crate) fn surface_hover(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.muted, 0.12)
    } else {
        mix(t.card, t.muted, 0.9)
    }
}

/// Hairline used around surfaces (very faint in light themes).
pub(crate) fn hairline(t: &Theme) -> Stroke {
    Stroke::new(1.0, if t.dark { with_alpha(Color32::WHITE, 0.06) } else { with_alpha(t.border, 0.7) })
}

/// Shadow color scaled by `k` (0..1).
pub(crate) fn shadow_color(t: &Theme, k: f32) -> Color32 {
    if t.dark {
        Color32::from_black_alpha((150.0 * k).clamp(0.0, 255.0) as u8)
    } else {
        Color32::from_rgba_unmultiplied(24, 24, 40, (30.0 * k).clamp(0.0, 255.0) as u8)
    }
}

/// A soft raised card: blurred shadow, fill and hairline.
pub(crate) fn soft_card(painter: &Painter, t: &Theme, rect: Rect, radius: f32, elevation: f32, fill: Color32) {
    if elevation > 0.0 {
        paint::soft_shadow(
            painter,
            rect,
            radius,
            10.0 + 14.0 * elevation,
            vec2(0.0, 2.0 + 4.0 * elevation),
            shadow_color(t, elevation),
        );
    }
    painter.rect(rect, cr(radius), fill, hairline(t), egui::StrokeKind::Inside);
}

/// Ink fill for "dark" variants (the primary color) and the text on it.
pub(crate) fn ink(t: &Theme) -> (Color32, Color32) {
    (t.primary, t.primary_foreground)
}

/// Seconds since `active` was last true for widget `id` (starts large so autoplay begins at once).
pub(crate) fn idle_for(ui: &Ui, id: egui::Id, active: bool) -> f32 {
    let now = ui.input(|i| i.time);
    glowkit_core::with_state(
        ui,
        id.with("micro_b_idle"),
        || now - 1000.0,
        |last| {
            if active {
                *last = now;
            }
            (now - *last) as f32
        },
    )
}

/// Paint text anchored at `pos`.
pub(crate) fn text(painter: &Painter, pos: Pos2, align: Align2, s: &str, size: f32, color: Color32) -> Rect {
    painter.text(pos, align, s, FontId::proportional(size), color)
}

/// Width of a single line of text.
pub(crate) fn text_width(painter: &Painter, s: &str, size: f32) -> f32 {
    painter.layout_no_wrap(s.to_owned(), FontId::proportional(size), Color32::WHITE).size().x
}

/// A small arrow cursor (used to show autoplay "ghost" interactions).
pub(crate) fn ghost_cursor(painter: &Painter, t: &Theme, tip: Pos2, alpha: f32, pressed: bool) {
    if alpha <= 0.01 {
        return;
    }
    let s = if pressed { 0.9 } else { 1.0 };
    let pts: Vec<Pos2> = [(0.0, 0.0), (0.0, 16.0), (4.2, 12.3), (7.0, 18.2), (9.6, 17.0), (6.9, 11.3), (12.0, 11.3)]
        .iter()
        .map(|(x, y)| tip + vec2(*x, *y) * s)
        .collect();
    let (fill, line) = if t.dark { (Color32::WHITE, Color32::BLACK) } else { (Color32::BLACK, Color32::WHITE) };
    let mut closed = pts.clone();
    closed.push(pts[0]);
    // concave outline: fill as a fan from a point inside the main triangle
    let c = tip + vec2(3.5, 9.0) * s;
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        painter.add(Shape::convex_polygon(vec![c, a, b], with_alpha(fill, alpha), Stroke::NONE));
    }
    painter.add(Shape::line(closed, Stroke::new(1.2, with_alpha(line, alpha))));
}

/// A gradient avatar disc with a ring in the surface color (so stacked avatars separate).
pub(crate) fn avatar(painter: &Painter, center: Pos2, r: f32, colors: [Color32; 2], ring: Option<Color32>) {
    if let Some(ring) = ring {
        painter.circle_filled(center, r + 2.0, ring);
    }
    let pts = paint::circle_points(center, r, 40);
    let rect = Rect::from_center_size(center, Vec2::splat(r * 2.0));
    paint::fill_convex_fn(painter, &pts, 3, paint::linear_fn(rect, 135.0, &colors));
    paint::arc(painter, center, r - 1.0, -2.6, -1.4, 1.2, with_alpha(Color32::WHITE, 0.35));
}

// ---------------------------------------------------------------------------------------------
// Icons

/// Small line icons used by the micro-interaction components, painted with strokes so they look
/// the same on every platform (no font glyphs needed).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_effects::MicroIcon::Folder.paint(ui.painter(), egui::pos2(20.0, 20.0), 16.0, egui::Color32::GRAY);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MicroIcon {
    /// A plus sign.
    Plus,
    /// Arrow pointing up.
    ArrowUp,
    /// Arrow pointing down.
    ArrowDown,
    /// Arrow pointing right.
    ArrowRight,
    /// Diagonal arrow (up and right).
    ArrowUpRight,
    /// A check mark.
    Check,
    /// Two chain links.
    Link,
    /// Two overlapping squares.
    Copy,
    /// Two slider rails with knobs.
    Sliders,
    /// A page with a folded corner.
    Doc,
    /// A spreadsheet grid.
    Sheet,
    /// Four tiles.
    Board,
    /// A folder.
    Folder,
    /// Two circling arrows.
    Refresh,
    /// An x.
    Close,
    /// A downward chevron.
    ChevronDown,
    /// A circle on a wire.
    Node,
    /// A heart outline.
    Heart,
}

impl MicroIcon {
    /// Paint the icon centered at `center` in a box of `size` points.
    pub fn paint(self, painter: &Painter, center: Pos2, size: f32, color: Color32) {
        icon(painter, self, center, size, color, (size * 0.11).clamp(1.2, 2.4));
    }
}

/// Paint `icon` centered at `c` within a box of `size` points.
pub(crate) fn icon(painter: &Painter, ic: MicroIcon, c: Pos2, size: f32, color: Color32, width: f32) {
    let s = size / 2.0;
    let st = Stroke::new(width, color);
    let p = |x: f32, y: f32| c + vec2(x * s, y * s);
    let line = |pts: Vec<Pos2>| {
        painter.add(Shape::line(pts, st));
    };
    match ic {
        MicroIcon::Plus => {
            line(vec![p(-0.8, 0.0), p(0.8, 0.0)]);
            line(vec![p(0.0, -0.8), p(0.0, 0.8)]);
        }
        MicroIcon::ArrowUp | MicroIcon::ArrowDown | MicroIcon::ArrowRight | MicroIcon::ArrowUpRight => {
            let ang = match ic {
                MicroIcon::ArrowUp => -PI / 2.0,
                MicroIcon::ArrowDown => PI / 2.0,
                MicroIcon::ArrowRight => 0.0,
                _ => -PI / 4.0,
            };
            arrow(painter, c, s * 1.6, ang, st);
        }
        MicroIcon::Check => line(vec![p(-0.7, 0.05), p(-0.22, 0.52), p(0.72, -0.5)]),
        MicroIcon::Link => {
            let d = vec2(0.42, -0.42) * s;
            for k in [-1.0, 1.0] {
                let cc = c + d * k;
                let rect = Rect::from_center_size(cc, vec2(s * 1.25, s * 0.72));
                // rotate a capsule outline by -45 degrees
                let pts: Vec<Pos2> = paint::rounded_rect_points(rect, s * 0.36, 6)
                    .into_iter()
                    .map(|q| {
                        let v = q - cc;
                        let (sn, cs) = (-PI / 4.0).sin_cos();
                        cc + vec2(v.x * cs - v.y * sn, v.x * sn + v.y * cs)
                    })
                    .collect();
                painter.add(Shape::closed_line(pts, st));
            }
        }
        MicroIcon::Copy => {
            let a = Rect::from_min_size(p(-0.75, -0.35), vec2(s * 1.1, s * 1.1));
            let b = Rect::from_min_size(p(-0.35, -0.75), vec2(s * 1.1, s * 1.1));
            painter.rect_stroke(b, cr(s * 0.25), st, egui::StrokeKind::Middle);
            painter.rect_filled(a.expand(width * 0.5), cr(s * 0.25), Color32::TRANSPARENT);
            painter.rect_stroke(a, cr(s * 0.25), st, egui::StrokeKind::Middle);
        }
        MicroIcon::Sliders => {
            for (y, x) in [(-0.45, 0.3), (0.45, -0.3)] {
                line(vec![p(-0.8, y), p(0.8, y)]);
                painter.circle_filled(p(x, y), s * 0.24, color);
            }
        }
        MicroIcon::Doc => {
            line(vec![p(0.2, -0.85), p(-0.65, -0.85), p(-0.65, 0.85), p(0.65, 0.85), p(0.65, -0.4), p(0.2, -0.85)]);
            line(vec![p(0.2, -0.85), p(0.2, -0.4), p(0.65, -0.4)]);
            line(vec![p(-0.3, 0.15), p(0.3, 0.15)]);
            line(vec![p(-0.3, 0.45), p(0.3, 0.45)]);
        }
        MicroIcon::Sheet => {
            let r = Rect::from_center_size(c, vec2(s * 1.6, s * 1.6));
            painter.rect_stroke(r, cr(s * 0.25), st, egui::StrokeKind::Middle);
            line(vec![p(-0.8, -0.27), p(0.8, -0.27)]);
            line(vec![p(-0.8, 0.27), p(0.8, 0.27)]);
            line(vec![p(-0.27, -0.27), p(-0.27, 0.8)]);
        }
        MicroIcon::Board => {
            for (x, y) in [(-0.42, -0.42), (0.42, -0.42), (-0.42, 0.42), (0.42, 0.42)] {
                painter.rect_stroke(
                    Rect::from_center_size(p(x, y), vec2(s * 0.62, s * 0.62)),
                    cr(s * 0.12),
                    st,
                    egui::StrokeKind::Middle,
                );
            }
        }
        MicroIcon::Folder => {
            line(vec![
                p(-0.85, -0.6),
                p(-0.25, -0.6),
                p(-0.05, -0.35),
                p(0.85, -0.35),
                p(0.85, 0.7),
                p(-0.85, 0.7),
                p(-0.85, -0.6),
            ]);
        }
        MicroIcon::Refresh => {
            paint::arc(painter, c, s * 0.72, -PI * 0.15, PI * 0.95, width, color);
            paint::arc(painter, c, s * 0.72, PI * 0.85, PI * 1.95, width, color);
            for (a, dir) in [(PI * 0.95, 1.0f32), (PI * 1.95, 1.0)] {
                let tip = c + vec2(a.cos(), a.sin()) * s * 0.72;
                let tan = vec2(-a.sin(), a.cos()) * dir;
                let n = vec2(a.cos(), a.sin());
                line(vec![tip - tan * s * 0.32 + n * s * 0.28, tip, tip - tan * s * 0.32 - n * s * 0.28]);
            }
        }
        MicroIcon::Close => {
            line(vec![p(-0.6, -0.6), p(0.6, 0.6)]);
            line(vec![p(0.6, -0.6), p(-0.6, 0.6)]);
        }
        MicroIcon::ChevronDown => line(vec![p(-0.55, -0.25), p(0.0, 0.3), p(0.55, -0.25)]),
        MicroIcon::Node => {
            line(vec![p(-0.9, 0.0), p(-0.4, 0.0)]);
            line(vec![p(0.4, 0.0), p(0.9, 0.0)]);
            painter.circle_stroke(c, s * 0.4, st);
        }
        MicroIcon::Heart => {
            let pts = heart_points(c, s * 1.7, 40);
            painter.add(Shape::closed_line(pts, st));
        }
    }
}

/// An arrow of length `len` centered at `c`, pointing at angle `ang` (radians).
pub(crate) fn arrow(painter: &Painter, c: Pos2, len: f32, ang: f32, st: Stroke) {
    let d = vec2(ang.cos(), ang.sin());
    let tip = c + d * len / 2.0;
    let tail = c - d * len / 2.0;
    let h = len * 0.38;
    let n = d.rot90();
    painter.line_segment([tail, tip], st);
    painter.add(Shape::line(vec![tip - d * h + n * h, tip, tip - d * h - n * h], st));
}

/// Points of a heart outline that fits a box of `size` centered on `c` (clockwise).
pub(crate) fn heart_points(c: Pos2, size: f32, n: usize) -> Vec<Pos2> {
    (0..n)
        .map(|i| {
            let a = i as f32 / n as f32 * TAU;
            let x = 16.0 * a.sin().powi(3);
            let y = -(13.0 * a.cos() - 5.0 * (2.0 * a).cos() - 2.0 * (3.0 * a).cos() - (4.0 * a).cos());
            c + vec2(x, y + 1.5) * (size / 34.0)
        })
        .collect()
}

/// Filled heart (concave, so it is painted as a fan from its center).
pub(crate) fn heart_filled(painter: &Painter, c: Pos2, size: f32, color: Color32) {
    let pts = heart_points(c, size, 48);
    let center = c + vec2(0.0, size * 0.05);
    let mut mesh = egui::epaint::Mesh::default();
    mesh.colored_vertex(center, color);
    for q in &pts {
        mesh.colored_vertex(*q, color);
    }
    let n = pts.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(Shape::mesh(mesh));
    let mut rim = pts;
    rim.push(rim[0]);
    painter.add(Shape::line(rim, Stroke::new(0.8, color)));
}

/// A smooth point on a slow figure-eight inside `rect` (autoplay paths).
pub(crate) fn lissajous(rect: Rect, time: f32, seed: f32) -> Pos2 {
    let c = rect.center();
    pos2(
        c.x + (time * 0.7 + seed).sin() * rect.width() * 0.38,
        c.y + (time * 1.13 + seed * 1.7).sin() * rect.height() * 0.34,
    )
}
