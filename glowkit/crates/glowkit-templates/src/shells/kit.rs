//! Crate-internal building blocks shared by the shells and the app templates: animation helpers,
//! surface colors, truncated text, rect splitting, small buttons and loading gates.

use egui::text::LayoutJob;
use egui::{
    pos2, vec2, Align2, Color32, Context, CursorIcon, FontId, Galley, Id, Painter, Pos2, Rect, Response, Sense, Stroke,
    StrokeKind, Ui, UiBuilder,
};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};
use std::sync::Arc;

use super::icon::AppIcon;

// ---------------------------------------------------------------------------------------------
// animation
// ---------------------------------------------------------------------------------------------

/// Frame-rate independent exponential follow stored in context memory.
pub(crate) fn follow(ctx: &Context, id: Id, target: f32, speed: f32) -> f32 {
    let dt = ctx.input(|i| i.stable_dt).clamp(0.0, 0.05);
    let v = ctx.data_mut(|d| {
        let v = d.get_temp_mut_or_insert_with(id.with("tpl_follow"), || target);
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

/// A spring stored in context memory (settles exactly and stops repainting).
pub(crate) fn spring(ctx: &Context, id: Id, target: f32, stiffness: f32, damping: f32) -> f32 {
    let dt = ctx.input(|i| i.stable_dt).clamp(0.0, 0.05);
    let (v, settled) = ctx.data_mut(|d| {
        let s = d.get_temp_mut_or_insert_with(id.with("tpl_spring"), || glowkit_core::Spring::new(target));
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

/// Smoothed hover amount (0..1) for a widget.
pub(crate) fn hover(ui: &Ui, id: Id, hovered: bool) -> f32 {
    follow(ui.ctx(), id.with("tpl_hover"), if hovered { 1.0 } else { 0.0 }, 18.0)
}

/// First-show loading gate: returns `true` for the first `secs` seconds after `id` was first
/// shown (show skeletons meanwhile). Call [`reset_loading`] to replay it.
pub(crate) fn loading(ui: &Ui, id: Id, secs: f32) -> bool {
    let now = ui.input(|i| i.time);
    let t0: f64 = glowkit_core::state(ui, id.with("tpl_loading"), || now);
    let e = (now - t0) as f32;
    if e < secs {
        ui.ctx().request_repaint();
        true
    } else {
        false
    }
}

/// Seconds since `id` first finished loading (for staggered appear animations).
pub(crate) fn since_loaded(ui: &Ui, id: Id, secs: f32) -> f32 {
    let now = ui.input(|i| i.time);
    let t0: f64 = glowkit_core::state(ui, id.with("tpl_loading"), || now);
    ((now - t0) as f32 - secs).max(0.0)
}

/// Replay the loading gate of `id`.
pub(crate) fn reset_loading(ui: &Ui, id: Id) {
    let now = ui.input(|i| i.time);
    glowkit_core::set_state(ui, id.with("tpl_loading"), now);
}

// ---------------------------------------------------------------------------------------------
// colors
// ---------------------------------------------------------------------------------------------

/// The slightly tinted surface sidebars and app chrome sit on.
pub(crate) fn chrome_bg(t: &Theme) -> Color32 {
    if t.dark {
        mix(t.background, t.foreground, 0.03)
    } else {
        mix(t.background, t.foreground, 0.025)
    }
}

/// Fill for the active row in navigation lists.
pub(crate) fn active_fill(t: &Theme, base: Color32) -> Color32 {
    if t.dark {
        mix(base, t.foreground, 0.08)
    } else {
        mix(base, t.foreground, 0.065)
    }
}

/// Hover fill (scaled by the hover amount).
pub(crate) fn hover_fill(t: &Theme, amount: f32) -> Color32 {
    with_alpha(t.foreground, if t.dark { 0.05 } else { 0.045 } * amount)
}

/// Soft shadow color for floating surfaces.
pub(crate) fn shadow(t: &Theme) -> Color32 {
    if t.dark {
        Color32::from_black_alpha(120)
    } else {
        Color32::from_black_alpha(26)
    }
}

/// Semantic status color by name ("success", "warning", ...).
pub(crate) fn tone(t: &Theme, tone: Tone) -> Color32 {
    match tone {
        Tone::Neutral => t.muted_foreground,
        Tone::Accent => t.accent,
        Tone::Success => t.success,
        Tone::Warning => t.warning,
        Tone::Danger => t.destructive,
        Tone::Info => t.info,
    }
}

/// Semantic tones used by status pills and dots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tone {
    Neutral,
    Accent,
    Success,
    Warning,
    Danger,
    Info,
}

// ---------------------------------------------------------------------------------------------
// surfaces
// ---------------------------------------------------------------------------------------------

/// A bordered card surface.
pub(crate) fn card(painter: &Painter, rect: Rect, t: &Theme) {
    painter.rect_filled(rect, t.radius, t.card);
    painter.rect_stroke(rect, t.radius, Stroke::new(1.0, t.border), StrokeKind::Inside);
}

/// A floating surface: soft shadow, popover fill and hairline border.
pub(crate) fn float_panel(painter: &Painter, rect: Rect, t: &Theme, radius: f32) {
    paint::soft_shadow(painter, rect, radius, 24.0, vec2(0.0, 8.0), shadow(t));
    painter.rect_filled(rect, radius, t.popover);
    painter.rect_stroke(rect, radius, Stroke::new(1.0, t.border), StrokeKind::Inside);
}

/// Run `add` in a child ui confined (and clipped) to `rect`.
pub(crate) fn child<R>(ui: &mut Ui, rect: Rect, add: impl FnOnce(&mut Ui) -> R) -> R {
    let clip = rect.intersect(ui.clip_rect());
    let mut c = ui.new_child(UiBuilder::new().max_rect(rect));
    c.set_clip_rect(clip);
    add(&mut c)
}

/// Paint a card at `rect` and run `add` inside it with `pad` padding. Returns the inner rect too.
pub(crate) fn card_ui<R>(ui: &mut Ui, rect: Rect, pad: f32, add: impl FnOnce(&mut Ui, Rect) -> R) -> R {
    let t = Theme::of(ui);
    card(ui.painter(), rect, &t);
    let inner = rect.shrink(pad);
    child(ui, inner, |ui| add(ui, inner))
}

/// A card with a title row (title, optional subtitle) and a body below. `add` receives the body rect.
pub(crate) fn titled_card<R>(
    ui: &mut Ui,
    rect: Rect,
    title: &str,
    subtitle: &str,
    add: impl FnOnce(&mut Ui, Rect) -> R,
) -> R {
    let t = Theme::of(ui);
    card(ui.painter(), rect, &t);
    let inner = rect.shrink(18.0);
    let p = ui.painter().clone();
    text_at(&p, pos2(inner.min.x, inner.min.y + 8.0), Align2::LEFT_CENTER, title, 15.0, t.foreground, inner.width() * 0.6);
    let mut top = inner.min.y + 22.0;
    if !subtitle.is_empty() {
        text_at(&p, pos2(inner.min.x, inner.min.y + 28.0), Align2::LEFT_CENTER, subtitle, 12.5, t.muted_foreground, inner.width() * 0.62);
        top += 20.0;
    }
    let body = Rect::from_min_max(pos2(inner.min.x, top + 10.0), inner.max);
    child(ui, body, |ui| add(ui, body))
}

// ---------------------------------------------------------------------------------------------
// text
// ---------------------------------------------------------------------------------------------

/// A single-line galley truncated with an ellipsis to `max_w`.
pub(crate) fn trunc(painter: &Painter, text: &str, size: f32, color: Color32, max_w: f32) -> Arc<Galley> {
    let mut job = LayoutJob::simple(text.to_owned(), FontId::proportional(size), color, max_w.max(8.0));
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    painter.layout_job(job)
}

/// Same as [`trunc`] in a monospace font.
pub(crate) fn trunc_mono(painter: &Painter, text: &str, size: f32, color: Color32, max_w: f32) -> Arc<Galley> {
    let mut job = LayoutJob::simple(text.to_owned(), FontId::monospace(size), color, max_w.max(8.0));
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    painter.layout_job(job)
}

/// Wrapped text limited to `rows` lines (ellipsis on the last).
pub(crate) fn wrapped(painter: &Painter, text: &str, size: f32, color: Color32, width: f32, rows: usize) -> Arc<Galley> {
    let mut job = LayoutJob::simple(text.to_owned(), FontId::proportional(size), color, width.max(8.0));
    job.wrap.max_rows = rows;
    job.wrap.break_anywhere = false;
    job.wrap.overflow_character = Some('…');
    painter.layout_job(job)
}

/// Paint truncated text anchored at `pos` with `align`. Returns the painted rect.
pub(crate) fn text_at(painter: &Painter, pos: Pos2, align: Align2, text: &str, size: f32, color: Color32, max_w: f32) -> Rect {
    let g = trunc(painter, text, size, color, max_w);
    let r = align.anchor_size(pos, g.size());
    painter.galley(r.min, g, color);
    r
}

/// Monospace variant of [`text_at`].
pub(crate) fn mono_at(painter: &Painter, pos: Pos2, align: Align2, text: &str, size: f32, color: Color32, max_w: f32) -> Rect {
    let g = trunc_mono(painter, text, size, color, max_w);
    let r = align.anchor_size(pos, g.size());
    painter.galley(r.min, g, color);
    r
}

/// Width of `text` at `size`.
pub(crate) fn text_w(painter: &Painter, text: &str, size: f32) -> f32 {
    painter.layout_no_wrap(text.to_owned(), FontId::proportional(size), Color32::WHITE).size().x
}

// ---------------------------------------------------------------------------------------------
// layout
// ---------------------------------------------------------------------------------------------

/// Split `rect` horizontally: entries > 0 are fixed widths, 0 entries share the rest equally.
pub(crate) fn split_x(rect: Rect, widths: &[f32], gap: f32) -> Vec<Rect> {
    let fixed: f32 = widths.iter().sum();
    let flex = widths.iter().filter(|w| **w <= 0.0).count().max(1) as f32;
    let rest = (rect.width() - fixed - gap * (widths.len().saturating_sub(1)) as f32).max(0.0) / flex;
    let mut x = rect.min.x;
    widths
        .iter()
        .map(|w| {
            let w = if *w > 0.0 { *w } else { rest };
            let r = Rect::from_min_size(pos2(x, rect.min.y), vec2(w, rect.height()));
            x += w + gap;
            r
        })
        .collect()
}

/// Split `rect` vertically: entries > 0 are fixed heights, 0 entries share the rest equally.
pub(crate) fn split_y(rect: Rect, heights: &[f32], gap: f32) -> Vec<Rect> {
    let fixed: f32 = heights.iter().sum();
    let flex = heights.iter().filter(|h| **h <= 0.0).count().max(1) as f32;
    let rest = (rect.height() - fixed - gap * (heights.len().saturating_sub(1)) as f32).max(0.0) / flex;
    let mut y = rect.min.y;
    heights
        .iter()
        .map(|h| {
            let h = if *h > 0.0 { *h } else { rest };
            let r = Rect::from_min_size(pos2(rect.min.x, y), vec2(rect.width(), h));
            y += h + gap;
            r
        })
        .collect()
}

/// `n` equal columns.
pub(crate) fn cols(rect: Rect, n: usize, gap: f32) -> Vec<Rect> {
    split_x(rect, &vec![0.0; n.max(1)], gap)
}

/// Allocate the full available width at `height` (the template canvas).
pub(crate) fn canvas(ui: &mut Ui, height: f32) -> Rect {
    let h = height.min(ui.available_height().max(360.0));
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    rect
}

// ---------------------------------------------------------------------------------------------
// interaction
// ---------------------------------------------------------------------------------------------

/// A clickable row with an animated hover fill. Returns the response and hover amount.
pub(crate) fn row(ui: &mut Ui, rect: Rect, id: Id, radius: f32) -> (Response, f32) {
    let t = Theme::of(ui);
    let resp = ui.interact(rect, id, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    let hv = hover(ui, id, resp.hovered());
    if hv > 0.01 {
        ui.painter().rect_filled(rect, radius, hover_fill(&t, hv));
    }
    (resp, hv)
}

/// A square ghost icon button.
pub(crate) fn icon_button(ui: &mut Ui, rect: Rect, id: Id, icon: AppIcon, active: bool) -> Response {
    let t = Theme::of(ui);
    let resp = ui.interact(rect, id, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    let hv = hover(ui, id, resp.hovered());
    let p = ui.painter();
    if active {
        p.rect_filled(rect, t.radius * 0.6, active_fill(&t, t.background));
    } else if hv > 0.01 {
        p.rect_filled(rect, t.radius * 0.6, hover_fill(&t, hv * 1.4));
    }
    let col = if active { t.foreground } else { t.muted_foreground.lerp_to_gamma(t.foreground, hv) };
    let s = (rect.height() * 0.55).min(18.0);
    icon.paint(p, Rect::from_center_size(rect.center(), vec2(s, s)), col);
    resp
}

/// Hover tooltip with an optional shortcut hint (e.g. `"Ctrl+B"`).
pub(crate) fn tip(resp: &Response, text: &str, shortcut: Option<&str>) {
    let s = match shortcut {
        Some(k) => format!("{text}   {k}"),
        None => text.to_owned(),
    };
    glowkit_components::Tooltip::new(s).delay(0.25).show(resp);
}

/// A keyboard hint chip whose right edge sits at `right_center`.
pub(crate) fn kbd(painter: &Painter, right_center: Pos2, text: &str, t: &Theme) -> Rect {
    let g = painter.layout_no_wrap(text.to_owned(), FontId::proportional(11.0), t.muted_foreground);
    let r = Rect::from_min_max(pos2(right_center.x - g.size().x - 10.0, right_center.y - 10.0), pos2(right_center.x, right_center.y + 10.0));
    painter.rect_filled(r, 5.0, t.muted);
    painter.rect_stroke(r, 5.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
    painter.galley(r.center() - g.size() / 2.0, g, t.muted_foreground);
    r
}

/// A small rounded pill (status / count). `pos` is the left-center (or right-center if `right`).
pub(crate) fn pill(painter: &Painter, pos: Pos2, right: bool, text: &str, fg: Color32, bg: Color32, size: f32) -> Rect {
    let g = painter.layout_no_wrap(text.to_owned(), FontId::proportional(size), fg);
    let w = g.size().x + size * 1.1;
    let h = size + 9.0;
    let x0 = if right { pos.x - w } else { pos.x };
    let r = Rect::from_min_size(pos2(x0, pos.y - h / 2.0), vec2(w, h));
    painter.rect_filled(r, h / 2.0, bg);
    painter.galley(r.center() - g.size() / 2.0, g, fg);
    r
}

/// A status pill with a dot in a semantic tone.
pub(crate) fn status_pill(painter: &Painter, pos: Pos2, right: bool, text: &str, tone_c: Color32, t: &Theme) -> Rect {
    let g = painter.layout_no_wrap(text.to_owned(), FontId::proportional(11.5), t.foreground);
    let w = g.size().x + 26.0;
    let x0 = if right { pos.x - w } else { pos.x };
    let r = Rect::from_min_size(pos2(x0, pos.y - 11.0), vec2(w, 22.0));
    painter.rect_filled(r, 11.0, with_alpha(tone_c, if t.dark { 0.14 } else { 0.1 }));
    painter.rect_stroke(r, 11.0, Stroke::new(1.0, with_alpha(tone_c, 0.28)), StrokeKind::Inside);
    painter.circle_filled(pos2(r.min.x + 10.0, r.center().y), 3.0, tone_c);
    let fg = if t.dark { mix(tone_c, Color32::WHITE, 0.35) } else { glowkit_core::color::shade(tone_c, -0.18) };
    painter.galley(pos2(r.min.x + 17.0, r.center().y - g.size().y / 2.0), g, fg);
    r
}

/// Paint a skeleton block.
pub(crate) fn skel(ui: &Ui, rect: Rect) {
    glowkit_components::Skeleton::new(rect.size()).paint(ui, rect);
}

/// Paint a skeleton block with a custom corner radius.
pub(crate) fn skel_r(ui: &Ui, rect: Rect, r: f32) {
    glowkit_components::Skeleton::new(rect.size()).radius(r).paint(ui, rect);
}

/// Paint an avatar (initials, color from name) into a square rect.
pub(crate) fn avatar(ui: &Ui, rect: Rect, name: &str) {
    glowkit_components::Avatar::new(name).paint(ui, rect);
}

/// A full-width ghost / outline / primary button at `rect`.
pub(crate) fn button_at(ui: &mut Ui, rect: Rect, label: &str, variant: glowkit_components::ButtonVariant) -> Response {
    ui.put(rect, glowkit_components::Button::new(label).variant(variant).size(glowkit_components::ControlSize::Sm).width(rect.width()))
}

/// Horizontal hairline.
pub(crate) fn hline(painter: &Painter, x0: f32, x1: f32, y: f32, t: &Theme) {
    painter.line_segment([pos2(x0, y), pos2(x1, y)], Stroke::new(1.0, t.border));
}

/// Vertical hairline.
pub(crate) fn vline(painter: &Painter, x: f32, y0: f32, y1: f32, t: &Theme) {
    painter.line_segment([pos2(x, y0), pos2(x, y1)], Stroke::new(1.0, t.border));
}

/// A logo tile: rounded gradient square with a letter or icon.
pub(crate) fn logo(painter: &Painter, rect: Rect, t: &Theme, letter: &str) {
    paint::gradient_rect(painter, rect, rect.height() * 0.26, 45.0, &t.gradient);
    painter.text(rect.center(), Align2::CENTER_CENTER, letter, FontId::proportional(rect.height() * 0.48), Color32::WHITE);
}

/// Page heading: big title + muted subtitle at the top-left of `rect`. Returns the bottom y.
pub(crate) fn heading(painter: &Painter, rect: Rect, title: &str, subtitle: &str, t: &Theme) -> f32 {
    text_at(painter, rect.min, Align2::LEFT_TOP, title, 22.0, t.foreground, rect.width());
    if subtitle.is_empty() {
        return rect.min.y + 30.0;
    }
    text_at(painter, rect.min + vec2(0.0, 32.0), Align2::LEFT_TOP, subtitle, 13.5, t.muted_foreground, rect.width());
    rect.min.y + 54.0
}

/// Format a number with thousands separators.
pub(crate) fn thousands(v: f64, decimals: usize) -> String {
    let s = format!("{:.*}", decimals, v.abs());
    let (int, frac) = s.split_once('.').map(|(a, b)| (a.to_owned(), Some(b.to_owned()))).unwrap_or((s.clone(), None));
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if let Some(f) = frac {
        out.push('.');
        out.push_str(&f);
    }
    if v < 0.0 {
        format!("-{out}")
    } else {
        out
    }
}

/// `$1,234.56`
pub(crate) fn money(v: f64) -> String {
    if v < 0.0 {
        format!("-${}", thousands(-v, 2))
    } else {
        format!("${}", thousands(v, 2))
    }
}

/// A vertical scroll area filling the ui, with an id salt.
pub(crate) fn scroll<R>(ui: &mut Ui, salt: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::ScrollArea::vertical().id_salt(salt).auto_shrink([false, false]).show(ui, add).inner
}

/// Allocate a fixed-height row of full width inside a layout ui and return its rect.
pub(crate) fn alloc_row(ui: &mut Ui, h: f32) -> Rect {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    r
}

/// A vertical two-color gradient rectangle (top color → bottom color).
pub(crate) fn vgrad(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut m = egui::Mesh::default();
    m.colored_vertex(rect.left_top(), top);
    m.colored_vertex(rect.right_top(), top);
    m.colored_vertex(rect.right_bottom(), bottom);
    m.colored_vertex(rect.left_bottom(), bottom);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(m));
}

/// A legend list: colored dot, label, value and share, one row each. Returns the used height.
pub(crate) fn legend_list(painter: &Painter, rect: Rect, items: &[(String, f64, Color32)], fmt: fn(f64) -> String, t: &Theme) -> f32 {
    let total: f64 = items.iter().map(|i| i.1).sum::<f64>().max(1e-9);
    let rh = 26.0;
    for (i, (label, v, c)) in items.iter().enumerate() {
        let y = rect.min.y + i as f32 * rh + rh / 2.0;
        painter.circle_filled(pos2(rect.min.x + 5.0, y), 4.0, *c);
        let vr = text_at(painter, pos2(rect.max.x, y), Align2::RIGHT_CENTER, &format!("{:.0}%", v / total * 100.0), 12.0, t.muted_foreground, 44.0);
        let vr2 = text_at(painter, pos2(vr.min.x - 10.0, y), Align2::RIGHT_CENTER, &fmt(*v), 12.5, t.foreground, 70.0);
        text_at(painter, pos2(rect.min.x + 16.0, y), Align2::LEFT_CENTER, label, 12.5, t.muted_foreground, vr2.min.x - rect.min.x - 22.0);
    }
    items.len() as f32 * rh
}

/// Place a widget at `rect` without moving the parent's layout cursor (unlike `Ui::put`).
pub(crate) fn put(ui: &mut Ui, rect: Rect, widget: impl egui::Widget) -> Response {
    child(ui, rect.expand(6.0), |ui| ui.put(rect, widget))
}
