//! Shared painting helpers for the AI components (crate-private).

use crate::icons::{paint_icon, Icon};
use egui::epaint::Vertex;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Galley, Id, Painter, Pos2, Rect, Response, Sense, Shape, Stroke,
    StrokeKind, Ui, UiBuilder, Vec2,
};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;
use std::ops::Range;
use std::sync::Arc;

/// Per-glyph effect used by [`paint_galley_fx`].
#[derive(Clone, Copy)]
pub(crate) struct Fx {
    pub alpha: f32,
    pub bold: bool,
    pub color: Option<Color32>,
    pub offset: Vec2,
}

impl Default for Fx {
    fn default() -> Self {
        Self { alpha: 1.0, bold: false, color: None, offset: Vec2::ZERO }
    }
}

/// Paint a galley, letting `fx(char_index)` change opacity, color, offset and (fake) weight of
/// every glyph. `char_index` counts characters of the galley's text, newlines included.
pub(crate) fn paint_galley_fx(painter: &Painter, pos: Pos2, galley: &Arc<Galley>, fx: impl Fn(usize) -> Fx) {
    let [tw, th] = painter.fonts(|fo| fo.font_image_size());
    let norm = vec2(1.0 / tw.max(1) as f32, 1.0 / th.max(1) as f32);
    let ppp = painter.pixels_per_point();
    let pos = pos2((pos.x * ppp).round() / ppp, (pos.y * ppp).round() / ppp);
    let bold_dx = 0.55_f32.max(0.5 / ppp);
    let mut base = 0usize;
    for row in &galley.rows {
        let src = &row.visuals.mesh;
        if src.is_empty() {
            base += row.char_count_including_newline().0;
            continue;
        }
        let offset = pos.to_vec2() + row.pos.to_vec2();
        let mut mesh = src.clone();
        for v in &mut mesh.vertices {
            v.pos += offset;
            v.uv = pos2(v.uv.x * norm.x, v.uv.y * norm.y);
        }
        let mut extra: Vec<[Vertex; 4]> = Vec::new();
        for (i, g) in row.glyphs.iter().enumerate() {
            if g.uv_rect.is_nothing() {
                continue;
            }
            let f = fx(base + i);
            let v0 = g.first_vertex as usize;
            if v0 + 4 > mesh.vertices.len() {
                continue;
            }
            for v in &mut mesh.vertices[v0..v0 + 4] {
                let c = f.color.unwrap_or(v.color);
                v.color = c.gamma_multiply(f.alpha.clamp(0.0, 1.0));
                v.pos += f.offset;
            }
            if f.bold && f.alpha > 0.0 {
                let mut q = [mesh.vertices[v0], mesh.vertices[v0 + 1], mesh.vertices[v0 + 2], mesh.vertices[v0 + 3]];
                for v in &mut q {
                    v.pos.x += bold_dx;
                }
                extra.push(q);
            }
        }
        for q in extra {
            let idx = mesh.vertices.len() as u32;
            mesh.vertices.extend_from_slice(&q);
            mesh.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx + 2, idx + 1, idx + 3]);
        }
        painter.add(Shape::mesh(mesh));
        base += row.char_count_including_newline().0;
    }
}

/// Rectangles (relative to the galley origin) covering the characters in `range`, one per row.
pub(crate) fn char_rects(galley: &Galley, range: Range<usize>) -> Vec<Rect> {
    let mut out = Vec::new();
    let mut base = 0usize;
    for row in &galley.rows {
        let mut r = Rect::NOTHING;
        for (i, g) in row.glyphs.iter().enumerate() {
            let idx = base + i;
            if idx >= range.start && idx < range.end {
                let gr = Rect::from_min_size(row.pos + vec2(g.pos.x, 0.0), vec2(g.advance_width, row.row.size.y));
                r = r.union(gr);
            }
        }
        if r.is_positive() {
            out.push(r);
        }
        base += row.char_count_including_newline().0;
    }
    out
}

/// Position (relative to the galley origin) just after the last glyph.
pub(crate) fn galley_end(galley: &Galley) -> Rect {
    match galley.rows.last() {
        Some(row) => {
            let x = row.pos.x + row.row.size.x;
            Rect::from_min_size(pos2(x, row.pos.y), vec2(0.0, row.row.size.y))
        }
        None => Rect::from_min_size(Pos2::ZERO, vec2(0.0, 16.0)),
    }
}

/// Smooth 0..1 hover/active value.
pub(crate) fn anim_bool(ui: &Ui, id: Id, on: bool, secs: f32) -> f32 {
    ui.ctx().animate_bool_with_time(id, on, secs)
}

/// A rotating arc spinner.
pub(crate) fn spinner(painter: &Painter, center: Pos2, radius: f32, width: f32, color: Color32, time: f32) {
    painter.circle_stroke(center, radius, Stroke::new(width, with_alpha(color, 0.18)));
    let a0 = time * 5.2;
    let sweep = 1.3 + (time * 2.1).sin() * 0.6;
    glowkit_core::paint::arc_gradient(painter, center, radius, a0, a0 + sweep, width, |t| {
        color.gamma_multiply(0.25 + 0.75 * t)
    });
}

/// Single-line text galley.
pub(crate) fn galley(ui: &Ui, text: impl Into<String>, size: f32, color: Color32) -> Arc<Galley> {
    ui.painter().layout_no_wrap(text.into(), FontId::proportional(size), color)
}

/// Wrapped text galley.
pub(crate) fn galley_wrapped(ui: &Ui, text: impl Into<String>, size: f32, color: Color32, width: f32) -> Arc<Galley> {
    ui.painter().layout(text.into(), FontId::proportional(size), color, width)
}

/// Paint text and return its rect.
pub(crate) fn text(painter: &Painter, pos: Pos2, align: Align2, s: impl ToString, size: f32, color: Color32) -> Rect {
    painter.text(pos, align, s, FontId::proportional(size), color)
}

/// Paint text in monospace and return its rect.
pub(crate) fn mono(painter: &Painter, pos: Pos2, align: Align2, s: impl ToString, size: f32, color: Color32) -> Rect {
    painter.text(pos, align, s, FontId::monospace(size), color)
}

/// A slightly raised surface color, for nested panels inside cards.
pub(crate) fn raised(t: &Theme) -> Color32 {
    if t.dark {
        mix(t.card, t.foreground, 0.035)
    } else {
        mix(t.card, t.muted, 0.6)
    }
}

/// Very subtle hover fill.
pub(crate) fn hover_fill(t: &Theme) -> Color32 {
    if t.dark {
        with_alpha(t.foreground, 0.07)
    } else {
        with_alpha(t.foreground, 0.055)
    }
}

/// Border that reads a little stronger than `t.border` (focused / hovered outlines).
pub(crate) fn border_strong(t: &Theme) -> Color32 {
    mix(t.border, t.muted_foreground, 0.35)
}

/// A square ghost icon button. Returns the click response.
pub(crate) fn icon_button(ui: &mut Ui, icon: Icon, size: f32, active: Option<Color32>, tooltip: &str) -> Response {
    let t = Theme::of(ui);
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    if ui.is_rect_visible(rect) {
        let h = anim_bool(ui, resp.id.with("h"), resp.hovered(), 0.12);
        let p = ui.painter();
        if h > 0.0 {
            p.rect_filled(rect, CornerRadius::same((size * 0.25) as u8), hover_fill(&t).gamma_multiply(h));
        }
        let col = active.unwrap_or_else(|| t.muted_foreground.lerp_to_gamma(t.foreground, h));
        let press = if resp.is_pointer_button_down_on() { 0.9 } else { 1.0 };
        let ir = Rect::from_center_size(rect.center(), Vec2::splat(size * 0.5 * press));
        paint_icon(p, icon, ir, col);
    }
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if tooltip.is_empty() {
        resp
    } else {
        resp.on_hover_text(tooltip)
    }
}

/// A chevron that rotates from pointing right (closed, `open = 0`) to down (`open = 1`).
pub(crate) fn chevron(painter: &Painter, center: Pos2, size: f32, color: Color32, open: f32) {
    let a = open * std::f32::consts::FRAC_PI_2;
    let (s, c) = a.sin_cos();
    let rot = |v: Vec2| center + vec2(v.x * c - v.y * s, v.x * s + v.y * c);
    let h = size * 0.5;
    let pts = vec![rot(vec2(-h * 0.45, -h)), rot(vec2(h * 0.55, 0.0)), rot(vec2(-h * 0.45, h))];
    painter.add(Shape::line(pts, Stroke::new(1.5, color)));
}

/// A rounded "pill" badge: tinted background, colored text. Returns its rect.
pub(crate) fn badge(painter: &Painter, left_center: Pos2, label: &str, color: Color32, t: &Theme) -> Rect {
    let g = painter.layout_no_wrap(label.to_owned(), FontId::proportional(11.5), color);
    let size = vec2(g.size().x + 14.0, 20.0);
    let rect = Rect::from_min_size(pos2(left_center.x, left_center.y - size.y / 2.0), size);
    painter.rect_filled(rect, CornerRadius::same(10), with_alpha(color, if t.dark { 0.14 } else { 0.1 }));
    painter.rect_stroke(rect, CornerRadius::same(10), Stroke::new(1.0, with_alpha(color, 0.25)), StrokeKind::Inside);
    painter.galley(pos2(rect.min.x + 7.0, rect.center().y - g.size().y / 2.0), g, color);
    rect
}

/// Lay out `add` in a child ui that is revealed vertically by `open` (0..1): height and opacity
/// animate together, so collapsible panels slide open smoothly.
pub(crate) fn reveal(ui: &mut Ui, id: Id, open: f32, add: impl FnOnce(&mut Ui)) {
    if open <= 0.001 {
        return;
    }
    let full_h: f32 = glowkit_core::state(ui, id.with("reveal_h"), || 0.0);
    let avail = ui.available_rect_before_wrap();
    let max = Rect::from_min_size(avail.min, vec2(avail.width(), f32::INFINITY));
    let mut child = ui.new_child(UiBuilder::new().max_rect(max).layout(egui::Layout::top_down(egui::Align::Min)));
    if open < 1.0 {
        let h = full_h * open;
        child.set_clip_rect(Rect::from_min_size(avail.min, vec2(avail.width(), h)).intersect(ui.clip_rect()));
        child.multiply_opacity(open * open);
    }
    add(&mut child);
    let content = child.min_rect();
    glowkit_core::set_state(ui, id.with("reveal_h"), content.height());
    let h = if open < 1.0 { (if full_h > 0.0 { full_h } else { content.height() }) * open } else { content.height() };
    ui.allocate_rect(Rect::from_min_size(avail.min, vec2(content.width(), h)), Sense::hover());
}

/// A collapsible state stored in egui memory: returns (open, animated openness).
pub(crate) fn toggle_state(ui: &Ui, id: Id, default_open: bool, toggled: bool) -> (bool, f32) {
    let open = glowkit_core::with_state(
        ui,
        id.with("open"),
        || default_open,
        |o| {
            if toggled {
                *o = !*o;
            }
            *o
        },
    );
    let t = ui.ctx().animate_bool_with_time(id.with("open_anim"), open, 0.22);
    (open, glowkit_core::ease(glowkit_core::Easing::OutCubic, t))
}

/// Format seconds as "4s" / "1m 12s".
pub(crate) fn fmt_secs(s: f32) -> String {
    let s = s.max(0.0).round() as u32;
    if s < 60 {
        format!("{s}s")
    } else {
        format!("{}m {}s", s / 60, s % 60)
    }
}

/// Avatar circle with an icon (assistant) or initials.
pub(crate) fn avatar(painter: &Painter, center: Pos2, radius: f32, t: &Theme, kind: AvatarKind<'_>) {
    match kind {
        AvatarKind::Assistant => {
            let stops = [t.gradient[0], t.gradient[1]];
            glowkit_core::paint::fill_convex_fn(
                painter,
                &glowkit_core::paint::circle_points(center, radius, 40),
                3,
                glowkit_core::paint::linear_fn(Rect::from_center_size(center, Vec2::splat(radius * 2.0)), 45.0, &stops),
            );
            glowkit_core::paint::sparkle(painter, center, radius * 0.62, 0.0, Color32::WHITE);
        }
        AvatarKind::Initials(s) => {
            painter.circle_filled(center, radius, t.secondary);
            painter.circle_stroke(center, radius, Stroke::new(1.0, t.border));
            painter.text(center, Align2::CENTER_CENTER, s, FontId::proportional(radius * 0.8), t.foreground);
        }
        AvatarKind::Icon(icon) => {
            painter.circle_filled(center, radius, t.secondary);
            painter.circle_stroke(center, radius, Stroke::new(1.0, t.border));
            paint_icon(painter, icon, Rect::from_center_size(center, Vec2::splat(radius * 1.1)), t.foreground);
        }
    }
}

/// What to draw inside an avatar.
#[derive(Clone, Copy, Debug)]
pub(crate) enum AvatarKind<'a> {
    Assistant,
    Initials(&'a str),
    #[allow(dead_code)]
    Icon(Icon),
}

/// Resolve a widget's state id: the caller's salt (stable) or the next auto id (positional).
pub(crate) fn widget_id(ui: &Ui, salt: Option<egui::IdSalt>) -> Id {
    salt.map(|s| ui.scope_id().with_salt(s)).unwrap_or_else(|| ui.next_auto_id())
}
