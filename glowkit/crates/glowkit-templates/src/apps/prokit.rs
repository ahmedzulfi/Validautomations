//! Compact building blocks for the dense "pro tool" templates (agent sessions, fleet, local LLM,
//! trading terminal, inspector): section labels, live dots, meters, mini tabs, chips and small
//! buttons with hover states.

use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Id, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui};
use glowkit_components::{ControlSize, Input, UiIcon};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

use crate::shells::kit;
use crate::shells::AppIcon;

/// Small uppercase section label. Returns its rect.
pub(crate) fn label(p: &Painter, pos: Pos2, text: &str, t: &Theme, max_w: f32) -> Rect {
    kit::text_at(p, pos, Align2::LEFT_CENTER, &text.to_uppercase(), 10.5, t.muted_foreground, max_w)
}

/// A tinted background for a semantic color.
pub(crate) fn soft(t: &Theme, c: Color32, strength: f32) -> Color32 {
    with_alpha(c, if t.dark { 0.16 } else { 0.11 } * strength)
}

/// Readable text color for a semantic tone on a soft background.
pub(crate) fn tone_fg(t: &Theme, c: Color32) -> Color32 {
    if t.dark {
        mix(c, Color32::WHITE, 0.3)
    } else {
        glowkit_core::color::shade(c, -0.22)
    }
}

/// A status dot; `pulse` adds an expanding ring (live).
pub(crate) fn dot(p: &Painter, c: Pos2, r: f32, col: Color32, pulse: Option<f64>) {
    if let Some(time) = pulse {
        let ph = (time * 0.9).fract() as f32;
        p.circle_filled(c, r + ph * r * 2.2, with_alpha(col, 0.35 * (1.0 - ph)));
    }
    p.circle_filled(c, r, col);
}

/// A horizontal meter (track + fill).
pub(crate) fn meter(p: &Painter, rect: Rect, frac: f32, col: Color32, t: &Theme) {
    let r = rect.height() / 2.0;
    p.rect_filled(rect, r, t.muted);
    let f = frac.clamp(0.0, 1.0);
    if f > 0.0 {
        let w = (rect.width() * f).max(rect.height());
        p.rect_filled(Rect::from_min_size(rect.min, vec2(w, rect.height())), r, col);
    }
}

/// A tiny tag (rounded, tinted, optional mono). Anchored at left-center (or right-center).
pub(crate) fn tag(p: &Painter, pos: Pos2, right: bool, text: &str, col: Color32, t: &Theme, mono: bool) -> Rect {
    let font = if mono { FontId::monospace(10.5) } else { FontId::proportional(10.5) };
    let fg = tone_fg(t, col);
    let g = p.layout_no_wrap(text.to_owned(), font, fg);
    let w = g.size().x + 10.0;
    let x0 = if right { pos.x - w } else { pos.x };
    let r = Rect::from_min_size(pos2(x0, pos.y - 9.0), vec2(w, 18.0));
    p.rect_filled(r, 4.0, soft(t, col, 1.0));
    p.rect_stroke(r, 4.0, Stroke::new(1.0, with_alpha(col, 0.22)), StrokeKind::Inside);
    p.galley(r.center() - g.size() / 2.0, g, fg);
    r
}

/// Compact underline tabs. Returns true when the selection changed.
pub(crate) fn mini_tabs(ui: &mut Ui, rect: Rect, id: Id, labels: &[&str], sel: &mut usize) -> bool {
    let t = Theme::of(ui);
    let p = ui.painter().clone();
    let mut x = rect.min.x;
    let mut changed = false;
    let mut spans = Vec::new();
    for (i, l) in labels.iter().enumerate() {
        let w = kit::text_w(&p, l, 12.5) + 18.0;
        let r = Rect::from_min_size(pos2(x, rect.min.y), vec2(w, rect.height()));
        let resp = ui.interact(r, id.with(i), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        let hv = kit::hover(ui, id.with(("h", i)), resp.hovered());
        if resp.clicked() && *sel != i {
            *sel = i;
            changed = true;
        }
        let active = *sel == i;
        let col = if active { t.foreground } else { t.muted_foreground.lerp_to_gamma(t.foreground, hv * 0.7) };
        p.text(r.center(), Align2::CENTER_CENTER, *l, FontId::proportional(12.5), col);
        spans.push((r.min.x + 6.0, r.max.x - 6.0));
        x += w;
    }
    if let Some(&(a, b)) = spans.get(*sel) {
        let xa = kit::follow(ui.ctx(), id.with("ia"), a, 20.0);
        let xb = kit::follow(ui.ctx(), id.with("ib"), b, 20.0);
        p.line_segment([pos2(xa, rect.max.y - 1.0), pos2(xb, rect.max.y - 1.0)], Stroke::new(2.0, t.foreground));
    }
    changed
}

/// A toggle chip (filter). Returns the response (`clicked` toggles at the caller).
pub(crate) fn chip(ui: &mut Ui, rect: Rect, id: Id, text: &str, on: bool, accent: Color32) -> Response {
    let t = Theme::of(ui);
    let resp = ui.interact(rect, id, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    let hv = kit::hover(ui, id, resp.hovered());
    let p = ui.painter();
    let r = rect.height() / 2.0;
    if on {
        p.rect_filled(rect, r, soft(&t, accent, 1.2));
        p.rect_stroke(rect, r, Stroke::new(1.0, with_alpha(accent, 0.4)), StrokeKind::Inside);
    } else {
        p.rect_filled(rect, r, kit::hover_fill(&t, hv * 1.5));
        p.rect_stroke(rect, r, Stroke::new(1.0, t.border), StrokeKind::Inside);
    }
    let fg = if on { tone_fg(&t, accent) } else { t.muted_foreground.lerp_to_gamma(t.foreground, hv) };
    kit::text_at(p, rect.center(), Align2::CENTER_CENTER, text, 11.5, fg, rect.width() - 8.0);
    resp
}

/// Width a [`chip`] needs for `text`.
pub(crate) fn chip_w(p: &Painter, text: &str) -> f32 {
    kit::text_w(p, text, 11.5) + 20.0
}

/// A compact button: outline (or filled with `fill`), optional icon and a trailing key hint.
pub(crate) fn small_btn(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    text: &str,
    icon: Option<AppIcon>,
    fill: Option<Color32>,
    key: Option<&str>,
) -> Response {
    let t = Theme::of(ui);
    let resp = ui.interact(rect, id, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    let hv = kit::hover(ui, id, resp.hovered());
    let press = if resp.is_pointer_button_down_on() { 1.0 } else { 0.0 };
    let p = ui.painter();
    let rr = 6.0;
    let fg = match fill {
        Some(c) => {
            let c = mix(c, if t.dark { Color32::WHITE } else { Color32::BLACK }, hv * 0.08 + press * 0.08);
            p.rect_filled(rect, rr, c);
            glowkit_core::color::contrast_text(c)
        }
        None => {
            p.rect_filled(rect, rr, mix(t.background, t.foreground, 0.02 + hv * 0.04 + press * 0.03));
            p.rect_stroke(rect, rr, Stroke::new(1.0, t.border), StrokeKind::Inside);
            t.foreground
        }
    };
    let key_w = key.map_or(0.0, |k| kit::text_w(p, k, 10.5) + 10.0);
    let icon_w = if icon.is_some() { 18.0 } else { 0.0 };
    let tw = kit::text_w(p, text, 12.5).min(rect.width() - icon_w - key_w - 16.0);
    let total = icon_w + tw + if key_w > 0.0 { key_w + 6.0 } else { 0.0 };
    let mut x = rect.center().x - total / 2.0;
    let cy = rect.center().y;
    if let Some(i) = icon {
        i.paint(p, Rect::from_center_size(pos2(x + 7.0, cy), vec2(14.0, 14.0)), fg);
        x += icon_w;
    }
    let r = kit::text_at(p, pos2(x, cy), Align2::LEFT_CENTER, text, 12.5, fg, tw + 1.0);
    if let Some(k) = key {
        let kr = Rect::from_min_size(pos2(r.max.x + 6.0, cy - 8.0), vec2(key_w, 16.0));
        p.rect_filled(kr, 4.0, with_alpha(fg, 0.12));
        p.text(kr.center(), Align2::CENTER_CENTER, k, FontId::proportional(10.5), with_alpha(fg, 0.75));
    }
    resp
}

/// A square ghost icon button with a tooltip.
pub(crate) fn icon_btn(ui: &mut Ui, rect: Rect, id: Id, icon: AppIcon, tip: &str, key: Option<&str>) -> Response {
    let r = kit::icon_button(ui, rect, id, icon, false);
    kit::tip(&r, tip, key);
    r
}

/// A small search input placed at `rect` (height ~32).
pub(crate) fn search(ui: &mut Ui, rect: Rect, text: &mut String, hint: &str, salt: &str) -> Response {
    kit::put(
        ui,
        rect,
        Input::new(text)
            .placeholder(hint)
            .icon(UiIcon::Search)
            .size(ControlSize::Sm)
            .clearable(true)
            .width(rect.width())
            .id_salt(salt),
    )
}

/// A plain text field placed at `rect` (height ~32) with an optional suffix.
pub(crate) fn field(ui: &mut Ui, rect: Rect, text: &mut String, suffix: &str, salt: &str) -> Response {
    let mut i = Input::new(text).size(ControlSize::Sm).width(rect.width()).id_salt(salt);
    if !suffix.is_empty() {
        i = i.suffix(suffix);
    }
    kit::put(ui, rect, i)
}

/// A one-line monospace text painted left-center at `pos`, clipped to `max_w`.
pub(crate) fn mono(p: &Painter, pos: Pos2, align: Align2, text: &str, size: f32, col: Color32, max_w: f32) -> Rect {
    kit::mono_at(p, pos, align, text, size, col, max_w)
}

/// `12.4 GB` style formatting from gigabytes.
pub(crate) fn gb(v: f64) -> String {
    if v < 1.0 {
        format!("{:.0} MB", v * 1024.0)
    } else {
        format!("{v:.1} GB")
    }
}

/// Mini sparkline painted into `rect` (no animation, cheap).
pub(crate) fn spark(p: &Painter, rect: Rect, values: &[f64], col: Color32, fill: bool) {
    if values.len() < 2 {
        return;
    }
    let (lo, hi) = values.iter().fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
    let span = (hi - lo).max(1e-9);
    let n = values.len() - 1;
    let pts: Vec<Pos2> = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            pos2(
                rect.min.x + rect.width() * i as f32 / n as f32,
                rect.max.y - ((v - lo) / span) as f32 * rect.height(),
            )
        })
        .collect();
    if fill {
        let mut m = egui::Mesh::default();
        for (i, q) in pts.iter().enumerate() {
            m.colored_vertex(*q, with_alpha(col, 0.22));
            m.colored_vertex(pos2(q.x, rect.max.y), with_alpha(col, 0.0));
            if i > 0 {
                let b = (i as u32) * 2;
                m.add_triangle(b - 2, b - 1, b);
                m.add_triangle(b - 1, b + 1, b);
            }
        }
        p.add(egui::Shape::mesh(m));
    }
    p.add(egui::Shape::line(pts, Stroke::new(1.4, col)));
}

/// A key hint chip anchored at left-center.
pub(crate) fn key_hint(p: &Painter, pos: Pos2, key: &str, t: &Theme) -> Rect {
    let g = p.layout_no_wrap(key.to_owned(), FontId::proportional(10.5), t.muted_foreground);
    let r = Rect::from_min_size(pos2(pos.x, pos.y - 9.0), vec2(g.size().x + 10.0, 18.0));
    p.rect_filled(r, 4.0, t.muted);
    p.rect_stroke(r, 4.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
    p.galley(r.center() - g.size() / 2.0, g, t.muted_foreground);
    r
}

/// "key  description" hints in a row starting at `pos`; returns the end x.
pub(crate) fn hints(p: &Painter, pos: Pos2, items: &[(&str, &str)], t: &Theme, max_x: f32) -> f32 {
    let mut x = pos.x;
    for (k, d) in items {
        let need = kit::text_w(p, k, 10.5) + 16.0 + kit::text_w(p, d, 11.5) + 14.0;
        if x + need > max_x {
            break;
        }
        let r = key_hint(p, pos2(x, pos.y), k, t);
        let r2 = kit::text_at(p, pos2(r.max.x + 6.0, pos.y), Align2::LEFT_CENTER, d, 11.5, t.muted_foreground, 200.0);
        x = r2.max.x + 14.0;
    }
    x
}

/// A clipped panel surface with the chrome tint and a border.
pub(crate) fn panel(p: &Painter, rect: Rect, t: &Theme) {
    p.rect_filled(rect, t.radius, t.card);
    p.rect_stroke(rect, t.radius, Stroke::new(1.0, t.border), StrokeKind::Inside);
}

/// A framed single-line text field painted at `rect` (frameless `TextEdit` inside, focus ring).
/// Returns `(submitted with Enter, focused)`.
pub(crate) fn text_line(ui: &mut Ui, rect: Rect, id: Id, text: &mut String, hint: &str, mono: bool) -> (bool, bool) {
    let t = Theme::of(ui);
    let focused = ui.memory(|m| m.has_focus(id));
    let fe = kit::follow(ui.ctx(), id.with("focus_ring"), if focused { 1.0 } else { 0.0 }, 14.0);
    let p = ui.painter().clone();
    p.rect_filled(rect, 8.0, if t.dark { mix(t.background, t.foreground, 0.035) } else { t.background });
    p.rect_stroke(rect, 8.0, Stroke::new(1.0, t.border.lerp_to_gamma(t.ring, fe * 0.8)), StrokeKind::Inside);
    if fe > 0.01 {
        p.rect_stroke(rect.expand(2.0), 10.0, Stroke::new(2.0, with_alpha(t.ring, 0.18 * fe)), StrokeKind::Inside);
    }
    let font = if mono { FontId::monospace(12.5) } else { FontId::proportional(13.0) };
    let h = font.size + 6.0;
    let er = Rect::from_min_max(pos2(rect.min.x + 12.0, rect.center().y - h / 2.0), pos2(rect.max.x - 10.0, rect.center().y + h / 2.0));
    let mut submitted = false;
    kit::child(ui, er, |ui| {
        let te = egui::TextEdit::singleline(text)
            .id(id)
            .hint_text(hint)
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .font(font)
            .desired_width(er.width());
        let resp = ui.add(te);
        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            submitted = true;
        }
    });
    (submitted, focused)
}

/// Whether any text field has keyboard focus (to gate single-key shortcuts).
pub(crate) fn typing(ui: &Ui) -> bool {
    ui.memory(|m| m.focused().is_some())
}
