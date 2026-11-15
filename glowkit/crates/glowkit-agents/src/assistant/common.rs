//! Shared plumbing for the assistant blocks: calm surfaces, text helpers, small buttons, the
//! tool-result card frame, idle timers and a ghost cursor for autoplay.

use egui::emath::TSTransform;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Galley, Id, Painter, Pos2, Rect, Response, Sense, Shape,
    Stroke, StrokeKind, Ui, UiBuilder, Vec2,
};
use glowkit_ai::{paint_icon, Icon};
use glowkit_core::anim::spring;
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::{paint, Theme};
use std::sync::Arc;

/// Corner radius from points.
#[inline]
pub(crate) fn cr(r: f32) -> CornerRadius {
    CornerRadius::same(r.round().clamp(0.0, 255.0) as u8)
}

/// The raised card color (lifted muted tone in dark themes so cards never read as holes).
pub(crate) fn surface(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.muted, 0.05)
    } else {
        t.card
    }
}

/// A slightly tinted well / hover row color.
pub(crate) fn well(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.muted, 0.11)
    } else {
        mix(t.card, t.muted, 0.75)
    }
}

/// Recessed track for segmented controls (visible against light stages).
pub(crate) fn track(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.muted, 0.06)
    } else {
        mix(t.muted, t.border, 0.45)
    }
}

/// The stage behind cards: a barely-warm off-white in light themes.
pub(crate) fn stage(t: &Theme) -> Color32 {
    if t.dark {
        t.background
    } else {
        mix(t.background, t.muted, 0.45)
    }
}

/// Hairline around surfaces.
pub(crate) fn hairline(t: &Theme) -> Stroke {
    Stroke::new(1.0, if t.dark { with_alpha(Color32::WHITE, 0.08) } else { with_alpha(t.border, 0.95) })
}

/// Shadow color scaled by `k`.
pub(crate) fn shadow(t: &Theme, k: f32) -> Color32 {
    if t.dark {
        Color32::from_black_alpha((160.0 * k).clamp(0.0, 255.0) as u8)
    } else {
        Color32::from_rgba_unmultiplied(20, 20, 35, (26.0 * k).clamp(0.0, 255.0) as u8)
    }
}

/// Lay out a single line of text.
pub(crate) fn galley(ui: &Ui, s: impl Into<String>, size: f32, color: Color32) -> Arc<Galley> {
    ui.painter().layout_no_wrap(s.into(), FontId::proportional(size), color)
}

/// Lay out wrapped text.
pub(crate) fn galley_wrap(ui: &Ui, s: impl Into<String>, size: f32, color: Color32, wrap: f32) -> Arc<Galley> {
    ui.painter().layout(s.into(), FontId::proportional(size), color, wrap.max(10.0))
}

/// Paint text and return its rect.
pub(crate) fn text(p: &Painter, pos: Pos2, align: Align2, s: &str, size: f32, color: Color32) -> Rect {
    p.text(pos, align, s, FontId::proportional(size), color)
}

/// Paint "semi-bold" text (two slightly offset passes).
pub(crate) fn text_bold(p: &Painter, pos: Pos2, align: Align2, s: &str, size: f32, color: Color32) -> Rect {
    let r = p.text(pos, align, s, FontId::proportional(size), color);
    p.text(pos + vec2(0.45, 0.0), align, s, FontId::proportional(size), color);
    r
}

/// Paint monospace text.
pub(crate) fn mono(p: &Painter, pos: Pos2, align: Align2, s: &str, size: f32, color: Color32) -> Rect {
    p.text(pos, align, s, FontId::monospace(size), color)
}

/// Width of one line of text.
pub(crate) fn text_w(ui: &Ui, s: &str, size: f32) -> f32 {
    galley(ui, s, size, Color32::WHITE).size().x
}

/// Paint one of the glowkit-ai line icons centered at `c`.
pub(crate) fn icon(p: &Painter, ic: Icon, c: Pos2, size: f32, color: Color32) {
    paint_icon(p, ic, Rect::from_center_size(c, Vec2::splat(size)), color);
}

/// A 0..1 spring toward `on`.
pub(crate) fn toggle(ui: &Ui, id: Id, on: bool) -> f32 {
    spring(ui, id, if on { 1.0 } else { 0.0 }, 320.0, 28.0)
}

/// Seconds since `active` was last true for `id` (starts large so autoplay begins at once).
pub(crate) fn idle_for(ui: &Ui, id: Id, active: bool) -> f32 {
    let now = ui.input(|i| i.time);
    glowkit_core::with_state(
        ui,
        id.with("asst_idle"),
        || now - 1000.0,
        |last| {
            if active {
                *last = now;
            }
            (now - *last) as f32
        },
    )
}

/// True while the pointer is over `rect` or a button is pressed there (used to pause autoplay).
pub(crate) fn user_active(ui: &Ui, rect: Rect) -> bool {
    let (pos, any, typed) = ui.input(|i| {
        (
            i.pointer.latest_pos(),
            i.pointer.any_down(),
            i.events.iter().any(|e| matches!(e, egui::Event::Text(_) | egui::Event::Key { .. })),
        )
    });
    let inside = pos.is_some_and(|p| rect.contains(p));
    inside && (any || typed) || (typed && ui.memory(|m| m.focused().is_some()))
}

/// The first `n` characters of `s`.
pub(crate) fn take_chars(s: &str, n: usize) -> &str {
    match s.char_indices().nth(n) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

/// A small arrow cursor for autoplay.
pub(crate) fn ghost_cursor(p: &Painter, t: &Theme, tip: Pos2, alpha: f32, pressed: bool) {
    if alpha <= 0.01 {
        return;
    }
    let s = if pressed { 0.88 } else { 1.0 };
    let pts: Vec<Pos2> = [(0.0, 0.0), (0.0, 16.0), (4.2, 12.3), (7.0, 18.2), (9.6, 17.0), (6.9, 11.3), (12.0, 11.3)]
        .iter()
        .map(|(x, y)| tip + vec2(*x, *y) * s)
        .collect();
    let (fill, line) = if t.dark { (Color32::WHITE, Color32::BLACK) } else { (Color32::BLACK, Color32::WHITE) };
    let c = tip + vec2(3.5, 9.0) * s;
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        p.add(Shape::convex_polygon(vec![c, a, b], with_alpha(fill, alpha), Stroke::NONE));
    }
    let mut closed = pts.clone();
    closed.push(pts[0]);
    p.add(Shape::line(closed, Stroke::new(1.2, with_alpha(line, alpha))));
    if pressed {
        p.circle_stroke(tip, 9.0, Stroke::new(1.5, with_alpha(t.accent, alpha * 0.5)));
    }
}

/// Button flavours used by the blocks.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Btn {
    /// Ink fill (theme primary).
    Primary,
    /// Hairline outline.
    Outline,
    /// No chrome until hovered.
    Ghost,
}

/// Paint and interact with a pill button in `rect`.
pub(crate) fn button(ui: &mut Ui, id: Id, rect: Rect, label: &str, kind: Btn, icon_l: Option<Icon>) -> Response {
    let t = Theme::of(ui);
    let r = ui.interact(rect, id, Sense::click());
    let h = toggle(ui, id.with("h"), r.hovered());
    let press = toggle(ui, id.with("p"), r.is_pointer_button_down_on());
    let p = ui.painter();
    let rad = rect.height() / 2.0;
    let rr = rect.shrink(press * 0.8);
    let (fill, fg) = match kind {
        Btn::Primary => (shade(t.primary, if t.dark { -0.08 * h } else { 0.12 * h }), t.primary_foreground),
        Btn::Outline => (mix(surface(&t), well(&t), h), t.foreground),
        Btn::Ghost => (with_alpha(well(&t), h), mix(t.muted_foreground, t.foreground, 0.5 + 0.5 * h)),
    };
    p.rect_filled(rr, cr(rad), fill);
    if kind == Btn::Outline {
        p.rect_stroke(rr, cr(rad), hairline(&t), StrokeKind::Inside);
    }
    let size = (rect.height() * 0.42).clamp(11.0, 14.0);
    let tw = text_w(ui, label, size);
    let iw = if icon_l.is_some() { size + 6.0 } else { 0.0 };
    let x0 = rect.center().x - (tw + iw) / 2.0;
    if let Some(ic) = icon_l {
        icon(p, ic, pos2(x0 + size / 2.0, rect.center().y), size, fg);
    }
    text(p, pos2(x0 + iw, rect.center().y), Align2::LEFT_CENTER, label, size, fg);
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Width a [`button`] needs for `label`.
pub(crate) fn button_w(ui: &Ui, label: &str, h: f32, has_icon: bool) -> f32 {
    let size = (h * 0.42).clamp(11.0, 14.0);
    text_w(ui, label, size) + h * 0.9 + if has_icon { size + 6.0 } else { 0.0 }
}

/// A round icon button.
pub(crate) fn icon_button(ui: &mut Ui, id: Id, rect: Rect, ic: Icon, filled: bool) -> Response {
    let t = Theme::of(ui);
    let r = ui.interact(rect, id, Sense::click());
    let h = toggle(ui, id.with("h"), r.hovered());
    let p = ui.painter();
    let rad = rect.width().min(rect.height()) / 2.0;
    if filled {
        p.rect_filled(rect, cr(rad), shade(t.primary, if t.dark { -0.08 * h } else { 0.14 * h }));
        icon(p, ic, rect.center(), rad * 1.05, t.primary_foreground);
    } else {
        if h > 0.01 {
            p.rect_filled(rect, cr(rad), with_alpha(well(&t), h));
        }
        icon(p, ic, rect.center(), rad * 1.0, mix(t.muted_foreground, t.foreground, h));
    }
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A rounded tag / chip, returns its rect.
pub(crate) fn chip(p: &Painter, t: &Theme, ui: &Ui, pos: Pos2, label: &str, size: f32, tint: Option<Color32>) -> Rect {
    let w = text_w(ui, label, size) + size * 1.2;
    let rect = Rect::from_min_size(pos, vec2(w, size * 1.75));
    let (bg, fg) = match tint {
        Some(c) => (with_alpha(c, if t.dark { 0.18 } else { 0.1 }), if t.dark { shade(c, 0.25) } else { shade(c, -0.15) }),
        None => (well(t), t.muted_foreground),
    };
    p.rect_filled(rect, cr(rect.height() / 2.0), bg);
    text(p, rect.center(), Align2::CENTER_CENTER, label, size, fg);
    rect
}

/// A keyboard key cap.
pub(crate) fn kbd(p: &Painter, t: &Theme, ui: &Ui, right_center: Pos2, label: &str) -> Rect {
    let w = text_w(ui, label, 10.5).max(8.0) + 10.0;
    let rect = Rect::from_min_size(right_center - vec2(w, 9.0), vec2(w, 18.0));
    p.rect_filled(rect, cr(4.0), well(t));
    p.rect_stroke(rect, cr(4.0), hairline(t), StrokeKind::Inside);
    mono(p, rect.center(), Align2::CENTER_CENTER, label, 10.0, t.muted_foreground);
    rect
}

/// A small spinner arc.
pub(crate) fn spinner(p: &Painter, c: Pos2, r: f32, time: f32, color: Color32) {
    p.circle_stroke(c, r, Stroke::new(1.6, with_alpha(color, 0.18)));
    let a0 = time * 5.0;
    paint::arc(p, c, r, a0, a0 + 1.7, 1.6, color);
}

/// A check inside a filled disc.
pub(crate) fn check_disc(p: &Painter, c: Pos2, r: f32, fill: Color32, fg: Color32, k: f32) {
    p.circle_filled(c, r * (0.6 + 0.4 * k), fill);
    if k > 0.05 {
        let pts = [c + vec2(-0.45, 0.02) * r, c + vec2(-0.12, 0.36) * r, c + vec2(0.48, -0.32) * r];
        // draw the tick progressively
        let l1 = (k * 2.0).min(1.0);
        let l2 = (k * 2.0 - 1.0).clamp(0.0, 1.0);
        let m = pts[0] + (pts[1] - pts[0]) * l1;
        p.line_segment([pts[0], m], Stroke::new(r * 0.22, fg));
        if l2 > 0.0 {
            p.line_segment([pts[1], pts[1] + (pts[2] - pts[1]) * l2], Stroke::new(r * 0.22, fg));
        }
    }
}

/// Header used by every tool-result card: tool glyph, title, optional status on the right.
pub(crate) struct ToolHead<'a> {
    pub icon: Icon,
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub status: Option<(&'a str, Color32)>,
}

/// Show `body` inside a calm tool-result card with an appear animation (fade, rise and a tiny
/// scale) driven by `appear` (0..1). Returns the card rect.
pub(crate) fn tool_card(
    ui: &mut Ui,
    appear: f32,
    head: Option<ToolHead<'_>>,
    width: f32,
    body: impl FnOnce(&mut Ui),
) -> Rect {
    let t = Theme::of(ui);
    let a = appear.clamp(0.0, 1.2);
    let start = ui.cursor().min;
    let bg = ui.painter().add(Shape::Noop);
    let scale = 0.96 + 0.04 * a.min(1.0);
    let rise = (1.0 - a) * 14.0;
    let pivot = start + vec2(width / 2.0, 0.0);
    let tr = TSTransform::from_translation(pivot.to_vec2() + vec2(0.0, rise))
        * TSTransform::from_scaling(scale)
        * TSTransform::from_translation(-pivot.to_vec2());
    let inner = ui.with_visual_transform(tr, |ui| {
        ui.multiply_opacity(a.clamp(0.0, 1.0));
        let max = Rect::from_min_size(start, vec2(width, 4000.0));
        ui.scope_builder(UiBuilder::new().max_rect(max).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
            ui.set_width(width);
            let pad = 14.0;
            ui.add_space(pad);
            ui.horizontal(|ui| {
                ui.add_space(pad);
                ui.vertical(|ui| {
                    ui.set_width(width - pad * 2.0);
                    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                    if let Some(h) = head {
                        let (r, _) = ui.allocate_exact_size(vec2(width - pad * 2.0, 26.0), Sense::hover());
                        let p = ui.painter();
                        let ib = Rect::from_min_size(r.min + vec2(0.0, 1.0), vec2(24.0, 24.0));
                        p.rect_filled(ib, cr(7.0), well(&t));
                        icon(p, h.icon, ib.center(), 14.0, t.foreground);
                        let tx = ib.max.x + 9.0;
                        if let Some(sub) = h.subtitle {
                            text_bold(p, pos2(tx, r.min.y + 7.0), Align2::LEFT_CENTER, h.title, 12.5, t.foreground);
                            text(p, pos2(tx, r.min.y + 20.0), Align2::LEFT_CENTER, sub, 10.5, t.muted_foreground);
                        } else {
                            text_bold(p, pos2(tx, r.center().y), Align2::LEFT_CENTER, h.title, 13.0, t.foreground);
                        }
                        if let Some((s, c)) = h.status {
                            let w = text_w(ui, s, 10.5) + 22.0;
                            let sr = Rect::from_min_size(pos2(r.max.x - w, r.center().y - 10.0), vec2(w, 20.0));
                            p.rect_filled(sr, cr(10.0), with_alpha(c, if t.dark { 0.16 } else { 0.09 }));
                            p.circle_filled(pos2(sr.min.x + 9.0, sr.center().y), 3.0, c);
                            text(p, pos2(sr.min.x + 15.0, sr.center().y), Align2::LEFT_CENTER, s, 10.5, c);
                        }
                    }
                    body(ui);
                });
            });
            ui.add_space(pad);
        })
        .response
        .rect
    });
    let rect = Rect::from_min_size(start, vec2(width, inner.inner.height()));
    let card_rect = tr.mul_rect(rect);
    let p = ui.painter();
    let mut shapes = Vec::new();
    let al = a.clamp(0.0, 1.0);
    // layered soft shadow + card behind the content
    let sh = egui::epaint::Shadow { offset: [0, 4], blur: 18, spread: 0, color: shadow(&t, 0.9 * al) };
    shapes.push(sh.as_shape(card_rect, cr(14.0)).into());
    shapes.push(Shape::rect_filled(card_rect, cr(14.0), with_alpha(surface(&t), al)));
    shapes.push(Shape::rect_stroke(
        card_rect,
        cr(14.0),
        Stroke::new(1.0, hairline(&t).color.gamma_multiply(al)),
        StrokeKind::Inside,
    ));
    p.set(bg, Shape::Vec(shapes));
    ui.allocate_rect(rect, Sense::hover());
    rect
}

/// Seconds since `key` last changed for `id` (restarts an appear animation when content changes).
pub(crate) fn since_change(ui: &Ui, id: Id, key: u64) -> f32 {
    let now = ui.input(|i| i.time);
    let (k, at) = glowkit_core::with_state(
        ui,
        id.with("asst_since"),
        || (key, now),
        |s| {
            if s.0 != key {
                *s = (key, now);
            }
            *s
        },
    );
    let _ = k;
    (now - at) as f32
}

/// Spring-like appear curve from elapsed seconds (overshoots slightly, settles at 1).
pub(crate) fn appear_curve(e: f32) -> f32 {
    if e <= 0.0 {
        return 0.0;
    }
    let k = 1.0 - (-e * 9.0).exp() * (e * 11.0).cos();
    k.clamp(0.0, 1.08)
}

/// Request repaints while an appear curve is still moving.
pub(crate) fn keep_alive(ui: &Ui, e: f32, dur: f32) {
    if e < dur {
        ui.ctx().request_repaint();
    }
}

/// Paint the assistant avatar (gradient disc with a sparkle).
pub(crate) fn bot_avatar(p: &Painter, t: &Theme, c: Pos2, r: f32) {
    let pts = paint::circle_points(c, r, 36);
    let rect = Rect::from_center_size(c, Vec2::splat(r * 2.0));
    if t.dark {
        paint::fill_convex_fn(p, &pts, 3, paint::linear_fn(rect, 135.0, &[t.gradient[0], t.gradient[1]]));
        paint::sparkle(p, c, r * 0.95, 0.0, Color32::WHITE);
    } else {
        p.circle_filled(c, r, t.primary);
        paint::sparkle(p, c, r * 0.95, 0.0, t.primary_foreground);
    }
}

/// A neutral initials avatar.
pub(crate) fn initials_avatar(p: &Painter, t: &Theme, c: Pos2, r: f32, s: &str, tint: Color32) {
    p.circle_filled(c, r, with_alpha(tint, if t.dark { 0.3 } else { 0.16 }));
    text_bold(p, c, Align2::CENTER_CENTER, s, r * 0.85, if t.dark { shade(tint, 0.35) } else { shade(tint, -0.3) });
}

/// Simple deterministic hash of a string (for appear keys).
pub(crate) fn hash_str(s: &str) -> u64 {
    let mut h: u64 = 1469598103934665603;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

/// Resolve a widget id from an optional salt, falling back to `default` under the current scope.
pub(crate) fn wid(ui: &Ui, salt: Option<egui::IdSalt>, default: &str) -> Id {
    match salt {
        Some(s) => ui.scope_id().with_salt(s),
        None => ui.scope_id().with(default),
    }
}
