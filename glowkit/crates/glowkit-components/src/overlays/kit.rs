//! Internal building blocks shared by the navigation and overlay components: floating panels,
//! presence animations, small buttons and keyboard chips. Not part of the public API.

use egui::emath::TSTransform;
use egui::{
    vec2, Align2, Color32, Context, CursorIcon, FontId, Id, InnerResponse, LayerId, Order, Pos2, Rect, Response, Sense,
    Stroke, StrokeKind, Ui, UiBuilder,
};
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::{paint, Theme};

/// Shadow tint for floating surfaces.
pub(crate) fn shadow_color(t: &Theme) -> Color32 {
    if t.dark {
        Color32::from_black_alpha(130)
    } else {
        Color32::from_black_alpha(34)
    }
}

/// Backdrop tint behind modal surfaces at full strength.
pub(crate) fn backdrop_color(t: &Theme) -> Color32 {
    if t.dark {
        Color32::from_black_alpha(150)
    } else {
        Color32::from_black_alpha(90)
    }
}

/// Paint a floating surface: soft shadow, popover fill, 1px border.
pub(crate) fn panel(painter: &egui::Painter, rect: Rect, t: &Theme, radius: f32) {
    paint::soft_shadow(painter, rect, radius, 28.0, vec2(0.0, 10.0), shadow_color(t));
    paint::soft_shadow(
        painter,
        rect,
        radius,
        6.0,
        vec2(0.0, 2.0),
        with_alpha(Color32::BLACK, if t.dark { 0.25 } else { 0.05 }),
    );
    painter.rect_filled(rect, radius, t.popover);
    painter.rect_stroke(rect, radius, Stroke::new(1.0, t.border), StrokeKind::Inside);
}

/// A subtle hover / highlight fill for rows inside floating surfaces.
pub(crate) fn row_highlight(t: &Theme) -> Color32 {
    if t.dark {
        mix(t.popover, t.foreground, 0.08)
    } else {
        mix(t.popover, t.foreground, 0.055)
    }
}

/// Frame-rate independent exponential follow stored in context memory.
pub(crate) fn follow_ctx(ctx: &Context, id: Id, target: f32, speed: f32) -> f32 {
    let dt = ctx.input(|i| i.stable_dt).clamp(0.0, 0.05);
    let v = ctx.data_mut(|d| {
        let v = d.get_temp_mut_or_insert_with(id.with("kit_follow"), || target);
        *v = glowkit_core::anim::approach(*v, target, speed, dt);
        if (*v - target).abs() < 1e-3 {
            *v = target;
        }
        *v
    });
    if v != target {
        ctx.request_repaint();
    }
    v
}

/// A spring stored in context memory.
pub(crate) fn spring_ctx(ctx: &Context, id: Id, target: f32, stiffness: f32, damping: f32) -> f32 {
    let dt = ctx.input(|i| i.stable_dt).clamp(0.0, 0.05);
    let (v, settled) = ctx.data_mut(|d| {
        let s = d.get_temp_mut_or_insert_with(id.with("kit_spring"), || glowkit_core::Spring::new(target));
        s.target = target;
        s.stiffness = stiffness;
        s.damping = damping;
        s.step(dt);
        if s.is_settled() {
            s.value = target;
            s.velocity = 0.0;
        }
        (s.value, s.is_settled())
    });
    if !settled {
        ctx.request_repaint();
    }
    v
}

/// Exponential presence animation (0 = hidden, 1 = shown).
pub(crate) fn presence(ui: &Ui, id: Id, open: bool, speed: f32) -> f32 {
    follow_ctx(ui.ctx(), id.with("presence"), if open { 1.0 } else { 0.0 }, speed)
}

/// A spring that overshoots slightly when opening (for scale-in effects).
pub(crate) fn pop_spring(ui: &Ui, id: Id, open: bool) -> f32 {
    spring_ctx(ui.ctx(), id.with("pop"), if open { 1.0 } else { 0.0 }, 420.0, 30.0)
}

/// Smoothly animated hover amount for a widget.
pub(crate) fn hover_anim(ui: &Ui, id: Id, hovered: bool) -> f32 {
    follow_ctx(ui.ctx(), id.with("hover_anim"), if hovered { 1.0 } else { 0.0 }, 18.0)
}

/// Show `add` inside a floating layer at exactly `rect`, scaled by `scale` around `origin` and
/// faded by `opacity`. Content is clipped to `clip`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn float_layer<R>(
    ctx: &Context,
    id: Id,
    order: Order,
    rect: Rect,
    clip: Rect,
    scale: f32,
    origin: Pos2,
    opacity: f32,
    add: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let resp = egui::Area::new(id)
        .order(order)
        .fixed_pos(rect.min)
        .constrain(false)
        .fade_in(false)
        .interactable(opacity > 0.05)
        .show(ctx, |ui| {
            ui.set_clip_rect(clip);
            ui.multiply_opacity(opacity.clamp(0.0, 1.0));
            ui.set_min_size(rect.size());
            let mut child = ui.new_child(UiBuilder::new().max_rect(rect));
            child.set_clip_rect(clip);
            add(&mut child)
        });
    let layer = LayerId::new(order, id);
    let s = scale.max(0.01);
    ctx.set_transform_layer(layer, TSTransform::new(origin.to_vec2() * (1.0 - s), s));
    resp
}

/// The full drawable area for overlays: either a caller-supplied rect or the whole screen.
pub(crate) fn screen(ctx: &Context, within: Option<Rect>) -> Rect {
    within.unwrap_or_else(|| ctx.content_rect())
}

/// Visual style of the small built-in buttons.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Btn {
    Primary,
    Outline,
    Ghost,
    Destructive,
}

/// Measure a button label.
pub(crate) fn button_width(ui: &Ui, label: &str, size: f32) -> f32 {
    let g = ui.painter().layout_no_wrap(label.to_owned(), FontId::proportional(size), Color32::WHITE);
    g.size().x + 28.0
}

/// A compact themed button that allocates its own space.
#[cfg_attr(not(feature = "demos"), allow(dead_code))]
pub(crate) fn button(ui: &mut Ui, label: &str, kind: Btn) -> Response {
    let w = button_width(ui, label, 13.5);
    let (rect, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
    let id = ui.scope_id().with(("kit_btn", label));
    button_at(ui, rect, id, label, kind)
}

/// A compact themed button painted in `rect`.
pub(crate) fn button_at(ui: &mut Ui, rect: Rect, id: Id, label: &str, kind: Btn) -> Response {
    let t = Theme::of(ui);
    let resp = ui.interact(rect, id, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    let h = hover_anim(ui, id, resp.hovered());
    let press = if resp.is_pointer_button_down_on() { 1.0 } else { 0.0 };
    let r = t.radius * 0.75;
    let (fill, fg, stroke) = match kind {
        Btn::Primary => (shade(t.primary, if t.dark { -0.08 } else { 0.12 } * h), t.primary_foreground, None),
        Btn::Destructive => (shade(t.destructive, -0.06 * h), Color32::WHITE, None),
        Btn::Outline => (mix(t.background, t.foreground, 0.06 * h), t.foreground, Some(t.border)),
        Btn::Ghost => (with_alpha(t.foreground, 0.07 * h), t.foreground, None),
    };
    let rect = rect.shrink(press * 0.6);
    let p = ui.painter();
    if kind == Btn::Primary || kind == Btn::Destructive {
        paint::soft_shadow(p, rect, r, 6.0, vec2(0.0, 1.0), Color32::from_black_alpha(if t.dark { 60 } else { 25 }));
    }
    p.rect_filled(rect, r, fill);
    if let Some(s) = stroke {
        p.rect_stroke(rect, r, Stroke::new(1.0, s), StrokeKind::Inside);
    }
    p.text(rect.center(), Align2::CENTER_CENTER, label, FontId::proportional(13.5), fg);
    resp
}

/// A small square icon button (ghost style).
pub(crate) fn icon_button(ui: &mut Ui, rect: Rect, id: Id, icon: super::LineIcon, color: Color32) -> Response {
    let t = Theme::of(ui);
    let resp = ui.interact(rect, id, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    let h = hover_anim(ui, id, resp.hovered());
    let p = ui.painter();
    p.rect_filled(rect, t.radius * 0.6, with_alpha(t.foreground, 0.08 * h));
    let s = rect.height().min(rect.width()) * 0.55;
    icon.paint(p, Rect::from_center_size(rect.center(), vec2(s, s)), color.lerp_to_gamma(t.foreground, h));
    resp
}

/// Paint a keyboard-shortcut chip right-aligned at `right_center`. Returns its rect.
pub(crate) fn kbd(painter: &egui::Painter, right_center: Pos2, text: &str, t: &Theme) -> Rect {
    let g = painter.layout_no_wrap(text.to_owned(), FontId::proportional(11.0), t.muted_foreground);
    let size = vec2(g.size().x + 10.0, 18.0);
    let rect = Rect::from_min_size(right_center - vec2(size.x, size.y / 2.0), size);
    painter.rect_filled(rect, 4.0, t.muted);
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
    painter.galley(rect.center() - g.size() / 2.0, g, t.muted_foreground);
    rect
}

/// Plain shortcut text (no chip), right-aligned.
pub(crate) fn shortcut_text(painter: &egui::Painter, right_center: Pos2, text: &str, t: &Theme) {
    painter.text(
        right_center,
        Align2::RIGHT_CENTER,
        text,
        FontId::proportional(11.5),
        with_alpha(t.muted_foreground, 0.85),
    );
}

/// Paint a small arrow (triangle) pointing from a floating panel edge toward `tip`.
pub(crate) fn arrow(painter: &egui::Painter, tip: Pos2, dir: egui::Vec2, size: f32, fill: Color32, border: Color32) {
    let n = dir.normalized();
    let side = n.rot90();
    let base = tip - n * size;
    let a = base + side * size;
    let b = base - side * size;
    painter.add(egui::Shape::convex_polygon(vec![a, tip, b], fill, Stroke::NONE));
    painter.line_segment([a, tip], Stroke::new(1.0, border));
    painter.line_segment([tip, b], Stroke::new(1.0, border));
    // cover the panel border under the arrow base
    painter.line_segment([a - n * 0.5 + side * -0.5, b - n * 0.5 + side * 0.5], Stroke::new(1.6, fill));
}

/// Paint a simple spinner.
pub(crate) fn spinner(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32, time: f32) {
    for i in 0..8 {
        let a = i as f32 / 8.0 * std::f32::consts::TAU;
        let phase = ((time * 1.2 - i as f32 / 8.0).rem_euclid(1.0) * 1.0).powf(1.0);
        let alpha = 0.18 + 0.82 * (1.0 - phase);
        let d = vec2(a.cos(), a.sin());
        painter.line_segment(
            [center + d * radius * 0.5, center + d * radius],
            Stroke::new(radius * 0.22, with_alpha(color, alpha)),
        );
    }
}

/// Clamp `rect` so it lies inside `bounds` (with a margin), preserving size when possible.
pub(crate) fn clamp_into(rect: Rect, bounds: Rect, margin: f32) -> Rect {
    let b = bounds.shrink(margin);
    let mut r = rect;
    if r.max.x > b.max.x {
        r = r.translate(vec2(b.max.x - r.max.x, 0.0));
    }
    if r.min.x < b.min.x {
        r = r.translate(vec2(b.min.x - r.min.x, 0.0));
    }
    if r.max.y > b.max.y {
        r = r.translate(vec2(0.0, b.max.y - r.max.y));
    }
    if r.min.y < b.min.y {
        r = r.translate(vec2(0.0, b.min.y - r.min.y));
    }
    r
}

/// Lay out text wrapped to `width` and return the galley.
pub(crate) fn wrap(ui: &Ui, text: &str, size: f32, color: Color32, width: f32) -> std::sync::Arc<egui::Galley> {
    ui.painter().layout(text.to_owned(), FontId::proportional(size), color, width)
}

/// A fake app screen used as the backdrop of overlay demos (skeleton content).
#[cfg(feature = "demos")]
pub(crate) fn demo_canvas(ui: &mut Ui, height: f32) -> Rect {
    let t = Theme::of(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, t.radius, t.background);
    p.rect_stroke(rect, t.radius, Stroke::new(1.0, t.border), StrokeKind::Inside);
    let sk = mix(t.background, t.foreground, if t.dark { 0.07 } else { 0.06 });
    // top bar
    let bar = Rect::from_min_size(rect.min, vec2(rect.width(), 44.0));
    p.line_segment([bar.left_bottom(), bar.right_bottom()], Stroke::new(1.0, t.border));
    p.circle_filled(bar.left_center() + vec2(22.0, 0.0), 8.0, with_alpha(t.accent, 0.8));
    for i in 0..3 {
        p.rect_filled(
            Rect::from_min_size(bar.left_center() + vec2(44.0 + i as f32 * 62.0, -4.0), vec2(48.0, 8.0)),
            4.0,
            sk,
        );
    }
    p.circle_filled(bar.right_center() - vec2(22.0, 0.0), 11.0, sk);
    // sidebar + content blocks
    let body = Rect::from_min_max(bar.left_bottom() + vec2(18.0, 18.0), rect.max - vec2(18.0, 18.0));
    let side_w = (body.width() * 0.2).min(170.0);
    for i in 0..6 {
        p.rect_filled(
            Rect::from_min_size(
                body.min + vec2(0.0, i as f32 * 26.0),
                vec2(side_w * (0.6 + 0.07 * (i % 3) as f32), 10.0),
            ),
            5.0,
            sk,
        );
    }
    let main = Rect::from_min_max(body.min + vec2(side_w + 24.0, 0.0), body.max);
    p.rect_filled(Rect::from_min_size(main.min, vec2(main.width() * 0.4, 16.0)), 6.0, sk);
    p.rect_filled(Rect::from_min_size(main.min + vec2(0.0, 28.0), vec2(main.width() * 0.65, 9.0)), 4.0, sk);
    let cw = (main.width() - 24.0) / 3.0;
    for i in 0..3 {
        let r = Rect::from_min_size(
            main.min + vec2(i as f32 * (cw + 12.0), 56.0),
            vec2(cw, (main.height() - 56.0).max(40.0)),
        );
        p.rect_stroke(r, t.radius * 0.8, Stroke::new(1.0, t.border), StrokeKind::Inside);
        p.rect_filled(Rect::from_min_size(r.min + vec2(14.0, 16.0), vec2(cw * 0.5, 9.0)), 4.0, sk);
        p.rect_filled(Rect::from_min_size(r.min + vec2(14.0, 34.0), vec2(cw * 0.3, 18.0)), 5.0, sk);
    }
    rect
}
