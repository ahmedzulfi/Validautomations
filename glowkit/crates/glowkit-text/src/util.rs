//! Private helpers shared by the text effects in this crate.

use egui::epaint::{Mesh, TextShape, Vertex};
use egui::emath::Rot2;
use egui::{pos2, vec2, Color32, FontId, Galley, Id, Painter, Pos2, Ui, Vec2};
use std::sync::Arc;

/// Proportional or monospace font of `size` points.
pub(crate) fn font(size: f32, mono: bool) -> FontId {
    if mono {
        FontId::monospace(size)
    } else {
        FontId::proportional(size)
    }
}

/// Lay out a single line with a placeholder color (so one cached galley serves every color).
pub(crate) fn layout(painter: &Painter, s: impl Into<String>, font: &FontId) -> Arc<Galley> {
    painter.layout_no_wrap(s.into(), font.clone(), Color32::PLACEHOLDER)
}

/// Lay out a single character.
pub(crate) fn layout_char(painter: &Painter, ch: char, font: &FontId) -> Arc<Galley> {
    let mut buf = [0u8; 4];
    layout(painter, &*ch.encode_utf8(&mut buf), font)
}

/// Fake-bold thickness (points) that reads well at `size`.
pub(crate) fn bold_px(size: f32) -> f32 {
    (size * 0.03).clamp(0.6, 2.8)
}

/// Thickness to use for a `bold` flag.
pub(crate) fn weight(bold: bool, size: f32) -> f32 {
    if bold {
        bold_px(size)
    } else {
        0.0
    }
}

/// Split a target opacity over `n` overlapping copies so the interior ends up at `alpha`.
fn per_copy(color: Color32, n: usize) -> Color32 {
    if n <= 1 {
        return color;
    }
    let a = color.a() as f32 / 255.0;
    if a >= 0.999 {
        return color;
    }
    let k = 1.0 - (1.0 - a).powf(1.0 / n as f32);
    color.gamma_multiply(k / a.max(1e-4))
}

/// Paint a galley at `pos` in `color`, thickened horizontally by `bold` points (0 = normal).
pub(crate) fn paint_galley(painter: &Painter, pos: Pos2, galley: &Arc<Galley>, color: Color32, bold: f32) {
    if color.a() == 0 {
        return;
    }
    let steps = (bold / 0.7).ceil().max(0.0) as usize;
    let c = per_copy(color, steps + 1);
    for i in 0..=steps {
        let dx = if steps == 0 { 0.0 } else { bold * i as f32 / steps as f32 };
        painter.add(TextShape::new(pos + vec2(dx, 0.0), galley.clone(), c).with_override_text_color(c));
    }
}

/// Paint a galley through an arbitrary (affine-ish) vertex mapping. `map` receives the vertex
/// position relative to the galley's top-left and returns a screen position. Colors come from
/// `color_at` evaluated at the mapped position.
pub(crate) fn paint_galley_mapped(
    painter: &Painter,
    galley: &Arc<Galley>,
    map: impl Fn(Vec2) -> Pos2,
    color_at: impl Fn(Pos2) -> Color32,
) {
    let [tw, th] = painter.fonts(|f| f.font_image_size());
    let norm = vec2(1.0 / tw.max(1) as f32, 1.0 / th.max(1) as f32);
    for row in &galley.rows {
        let src = &row.visuals.mesh;
        if src.is_empty() {
            continue;
        }
        let mut mesh = Mesh { indices: src.indices.clone(), vertices: Vec::with_capacity(src.vertices.len()), ..Default::default() };
        for v in &src.vertices {
            let p = map(row.pos.to_vec2() + v.pos.to_vec2());
            mesh.vertices.push(Vertex { pos: p, uv: pos2(v.uv.x * norm.x, v.uv.y * norm.y), color: color_at(p) });
        }
        painter.add(egui::Shape::mesh(mesh));
    }
}

/// Paint `galley` centered on `center`, scaled and rotated (radians, clockwise).
pub(crate) fn paint_galley_xf(
    painter: &Painter,
    galley: &Arc<Galley>,
    center: Pos2,
    scale: f32,
    angle: f32,
    color: Color32,
    bold: f32,
) {
    if color.a() == 0 || scale <= 0.01 {
        return;
    }
    if (scale - 1.0).abs() < 1e-3 && angle.abs() < 1e-4 {
        paint_galley(painter, center - galley.size() / 2.0, galley, color, bold);
        return;
    }
    let half = galley.size() / 2.0;
    let rot = Rot2::from_angle(angle);
    let steps = (bold * scale / 0.7).ceil().max(0.0) as usize;
    let c = per_copy(color, steps + 1);
    for i in 0..=steps {
        let dx = if steps == 0 { 0.0 } else { bold * i as f32 / steps as f32 };
        paint_galley_mapped(painter, galley, |p| center + rot * ((p - half + vec2(dx, 0.0)) * scale), |_| c);
    }
}

/// Paint a single character so its logical box is centered on `center`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_char_xf(
    painter: &Painter,
    ch: char,
    font: &FontId,
    center: Pos2,
    scale: f32,
    angle: f32,
    color: Color32,
    bold: f32,
) {
    if ch.is_whitespace() {
        return;
    }
    let g = layout_char(painter, ch, font);
    paint_galley_xf(painter, &g, center, scale, angle, color, bold);
}

/// Paint a single character with its logical box centered horizontally in a box of `width` at
/// `top_left` (used when a scrambled glyph replaces a glyph of a different width).
pub(crate) fn paint_char_in(
    painter: &Painter,
    ch: char,
    font: &FontId,
    top_left: Pos2,
    width: f32,
    color: Color32,
    bold: f32,
) {
    if ch.is_whitespace() {
        return;
    }
    let g = layout_char(painter, ch, font);
    let x = top_left.x + (width - g.size().x) / 2.0;
    paint_galley(painter, pos2(x, top_left.y), &g, color, bold);
}

/// A 0..1 value that ramps linearly toward `on` (1) or off (0) over `seconds`. Repaints while moving.
pub(crate) fn ramp(ui: &Ui, id: Id, on: bool, seconds: f32) -> f32 {
    let dt = glowkit_core::dt(ui);
    let target = if on { 1.0 } else { 0.0 };
    let v = glowkit_core::with_state(ui, id.with("glowkit_ramp"), || target, |v: &mut f32| {
        let step = dt / seconds.max(1e-3);
        *v = if *v < target { (*v + step).min(target) } else { (*v - step).max(target) };
        *v
    });
    if v != target {
        ui.ctx().request_repaint();
    }
    v
}

/// Widest of the ten digits, for tabular number layout.
pub(crate) fn digit_width(painter: &Painter, font: &FontId) -> f32 {
    ('0'..='9').map(|d| layout_char(painter, d, font).size().x).fold(0.0, f32::max)
}

/// Bump curve: 0 at 0 and 1, 1 at 0.5.
pub(crate) fn bump(t: f32) -> f32 {
    (t.clamp(0.0, 1.0) * std::f32::consts::PI).sin()
}

/// Fake gaussian blur for a placeholder-colored galley (see [`layout`]): rings of offset copies
/// at low alpha (more rings for bigger radii). `radius` near 0 paints the text normally.
pub(crate) fn paint_blurred(painter: &Painter, pos: Pos2, galley: &Arc<Galley>, color: Color32, radius: f32, bold: f32) {
    if color.a() == 0 {
        return;
    }
    if radius < 0.4 {
        paint_galley(painter, pos, galley, color, bold);
        return;
    }
    let rings = ((radius / 3.0).ceil() as usize).clamp(1, 4);
    let taps = 8;
    let copies = rings * taps;
    // total opacity spread over the copies, plus a faint sharp core that fades as blur grows
    let core = (1.0 - radius / 10.0).clamp(0.0, 1.0);
    let a = (1.0 - core * 0.6).clamp(0.2, 1.0) * 2.2 / copies as f32;
    let c = color.gamma_multiply(a.min(1.0));
    for ring in 0..rings {
        let rr = radius * (ring as f32 + 1.0) / rings as f32;
        let rot = ring as f32 * 0.4;
        for i in 0..taps {
            let ang = rot + i as f32 / taps as f32 * std::f32::consts::TAU;
            let p = pos + vec2(ang.cos(), ang.sin()) * rr + vec2(bold / 2.0, 0.0);
            painter.add(TextShape::new(p, galley.clone(), c).with_override_text_color(c));
        }
    }
    if core > 0.0 {
        paint_galley(painter, pos, galley, color.gamma_multiply(core * 0.6), bold);
    }
}
