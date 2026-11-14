//! Shared building blocks for the BoardUI-style dashboard cards: the soft frame and tiles,
//! the signature palette, count-up tweens, entry clocks, chips, pills, tabs and small painted
//! icons.

use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Id, Painter, Pos2, Rect, Shape, Stroke, Ui};
use glowkit_core::color::{fade, mix, shade};
use glowkit_core::{hex, paint, Theme};
use std::f32::consts::{PI, TAU};

// ---------------------------------------------------------------------------------------------
// Palette

/// The BoardUI accent family (lime, blue, purple, pink, yellow, emerald, teal, indigo, red,
/// orange), tuned for the active theme: identical in light themes, slightly lifted in dark ones.
pub(crate) fn pal(t: &Theme) -> [Color32; 10] {
    let base = [
        hex("#8EDB00"),
        hex("#2B7FFF"),
        hex("#C27AFF"),
        hex("#FB64B6"),
        hex("#FDC700"),
        hex("#00C951"),
        hex("#00D3BB"),
        hex("#615FFF"),
        hex("#F0174B"),
        hex("#FF8A1F"),
    ];
    if t.dark {
        base.map(|c| shade(c, 0.03))
    } else {
        base
    }
}

/// Palette indices.
pub(crate) const LIME: usize = 0;
pub(crate) const BLUE: usize = 1;
pub(crate) const PURPLE: usize = 2;
pub(crate) const PINK: usize = 3;
pub(crate) const YELLOW: usize = 4;
pub(crate) const EMERALD: usize = 5;
pub(crate) const TEAL: usize = 6;
pub(crate) const INDIGO: usize = 7;
pub(crate) const RED: usize = 8;
pub(crate) const ORANGE: usize = 9;

/// The soft panel every card sits on (light grey in light themes, a lifted tone in dark).
pub(crate) fn frame_bg(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.muted, 0.025)
    } else {
        mix(t.muted, t.card, 0.3)
    }
}

/// Inner white tiles.
pub(crate) fn tile_bg(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.muted, 0.075)
    } else {
        t.card
    }
}

/// Recessed chart track (the grey pills behind bars, empty heat cells).
pub(crate) fn track(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.muted, 0.06)
    } else {
        mix(t.muted, t.foreground, 0.045)
    }
}

/// A slightly stronger neutral (last-year bars, idle stubs).
pub(crate) fn neutral(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.muted, 0.16)
    } else {
        mix(t.muted, t.foreground, 0.13)
    }
}

/// Secondary text.
pub(crate) fn dim(t: &Theme) -> Color32 {
    t.muted_foreground
}

/// Tertiary text (axis labels, units).
pub(crate) fn faint(t: &Theme) -> Color32 {
    mix(t.muted_foreground, frame_bg(t), 0.3)
}

/// Hairline between rows.
pub(crate) fn hairline(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.muted, 0.09)
    } else {
        mix(t.border, t.card, 0.25)
    }
}

/// Rounded corner radius from a float.
pub(crate) fn cr(r: f32) -> CornerRadius {
    CornerRadius::same(r.round().clamp(0.0, 255.0) as u8)
}

/// Paint the outer soft frame.
pub(crate) fn frame(painter: &Painter, t: &Theme, rect: Rect, radius: f32) {
    if !t.dark {
        paint::soft_shadow(painter, rect, radius, 18.0, vec2(0.0, 3.0), Color32::from_rgba_unmultiplied(20, 20, 40, 6));
    }
    painter.rect_filled(rect, cr(radius), frame_bg(t));
    let edge = if t.dark { fade(t.border, 0.5) } else { fade(t.border, 0.75) };
    painter.rect_stroke(rect, cr(radius), Stroke::new(1.0, edge), egui::StrokeKind::Inside);
}

/// Paint an inner tile.
pub(crate) fn tile(painter: &Painter, t: &Theme, rect: Rect, radius: f32) {
    painter.rect_filled(rect, cr(radius), tile_bg(t));
    if !t.dark {
        painter.rect_stroke(rect, cr(radius), Stroke::new(0.6, fade(t.border, 0.5)), egui::StrokeKind::Inside);
    }
}

/// A white card with a hairline border and soft shadow (popover-style components).
pub(crate) fn sheet(painter: &Painter, t: &Theme, rect: Rect, radius: f32) {
    let sc = if t.dark { Color32::from_black_alpha(90) } else { Color32::from_rgba_unmultiplied(20, 20, 40, 14) };
    paint::soft_shadow(painter, rect, radius, 22.0, vec2(0.0, 6.0), sc);
    painter.rect(rect, cr(radius), tile_bg(t), Stroke::new(1.0, hairline(t)), egui::StrokeKind::Inside);
}

// ---------------------------------------------------------------------------------------------
// Text

/// Proportional font.
pub(crate) fn f(size: f32) -> FontId {
    FontId::proportional(size)
}

/// Monospace font.
pub(crate) fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}

/// Paint text and return its rect.
pub(crate) fn text(painter: &Painter, pos: Pos2, align: Align2, s: impl ToString, font: FontId, color: Color32) -> Rect {
    painter.text(pos, align, s.to_string(), font, color)
}

/// Width of a single-line string.
pub(crate) fn text_w(painter: &Painter, s: &str, font: FontId) -> f32 {
    painter.layout_no_wrap(s.to_string(), font, Color32::WHITE).size().x
}

/// Text with a soft highlight sweeping across it (a "shimmer" label).
pub(crate) fn shimmer_text(painter: &Painter, pos: Pos2, s: &str, font: FontId, base: Color32, hi: Color32, phase: f32) {
    let g = painter.layout_no_wrap(s.to_string(), font, Color32::WHITE);
    let w = g.size().x.max(1.0);
    let band = w * 0.45 + 30.0;
    let x0 = pos.x - band + (w + band * 2.0) * phase.rem_euclid(1.0);
    paint::galley_with(painter, pos, &g, |p| {
        let d = ((p.x - x0) / band).abs();
        let k = (1.0 - d).clamp(0.0, 1.0);
        mix(base, hi, k * k * (3.0 - 2.0 * k))
    });
}

/// `1234567` -> `1,234,567`.
pub(crate) fn thousands(v: f64) -> String {
    let neg = v < 0.0;
    let s = (v.abs().round() as u64).to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if neg {
        format!("-{out}")
    } else {
        out
    }
}

/// `2700` -> `2.7k`, `10000` -> `10K` style compact numbers (`upper` picks K vs k).
pub(crate) fn compact(v: f32, upper: bool) -> String {
    let k = if upper { "K" } else { "k" };
    let a = v.abs();
    if a >= 1_000_000.0 {
        trim(format!("{:.1}", v / 1_000_000.0)) + "M"
    } else if a >= 1000.0 {
        trim(format!("{:.1}", v / 1000.0)) + k
    } else {
        format!("{}", v.round() as i64)
    }
}

fn trim(s: String) -> String {
    if let Some(stripped) = s.strip_suffix(".0") {
        stripped.to_string()
    } else {
        s
    }
}

// ---------------------------------------------------------------------------------------------
// Motion

#[derive(Clone, Copy)]
struct Tween {
    from: f32,
    to: f32,
    start: f32,
}

/// Ease-out cubic.
pub(crate) fn ease_out(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    1.0 - (1.0 - x).powi(3)
}

/// Smooth count-up / count-to: when `target` changes the value glides there over `dur` seconds
/// with an ease-out. The first value counts up from `from`.
pub(crate) fn tween(ui: &Ui, id: Id, target: f32, from: f32, dur: f32) -> f32 {
    let now = glowkit_core::time(ui);
    let (v, done) = glowkit_core::with_state(
        ui,
        id.with("board_tween"),
        || Tween { from, to: target, start: now },
        |tw| {
            let k = ((now - tw.start) / dur.max(1e-3)).clamp(0.0, 1.0);
            let cur = tw.from + (tw.to - tw.from) * ease_out(k);
            if (tw.to - target).abs() > 1e-4 {
                tw.from = cur;
                tw.to = target;
                tw.start = now;
                (cur, false)
            } else {
                (cur, k >= 1.0)
            }
        },
    );
    if !done {
        ui.ctx().request_repaint();
    }
    v
}

#[derive(Clone, Copy)]
struct Seen {
    start: f32,
    pass: u64,
}

/// Seconds since the widget appeared. The clock restarts when the widget was not painted for a
/// few frames (so switching back to a page replays the entry animation).
pub(crate) fn entry(ui: &Ui, id: Id) -> f32 {
    let now = glowkit_core::time(ui);
    let pass = ui.ctx().cumulative_pass_nr();
    glowkit_core::with_state(
        ui,
        id.with("board_entry"),
        || Seen { start: now, pass },
        |s| {
            if pass > s.pass + 3 || now < s.start {
                s.start = now;
            }
            s.pass = pass;
            now - s.start
        },
    )
}

/// Progress `0..1` of an entry animation of `dur` seconds after `delay`; requests repaints while
/// running.
pub(crate) fn entry_k(ui: &Ui, elapsed: f32, delay: f32, dur: f32) -> f32 {
    let k = ((elapsed - delay) / dur).clamp(0.0, 1.0);
    if k < 1.0 {
        ui.ctx().request_repaint();
    }
    k
}

/// A springy ease (slight overshoot) for entry motion.
pub(crate) fn ease_back(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let c1 = 1.4;
    let c3 = c1 + 1.0;
    1.0 + c3 * (x - 1.0).powi(3) + c1 * (x - 1.0).powi(2)
}

/// Seconds since the pointer last interacted with the widget (or since it appeared).
pub(crate) fn idle_for(ui: &Ui, id: Id, interacting: bool) -> f32 {
    let now = glowkit_core::time(ui);
    let since = glowkit_core::with_state(
        ui,
        id.with("board_idle"),
        || now,
        |t| {
            if interacting || now < *t {
                *t = now;
            }
            *t
        },
    );
    now - since
}

/// Smoothly follow a 0/1 flag.
pub(crate) fn flag(ui: &Ui, id: Id, on: bool, speed: f32) -> f32 {
    glowkit_core::anim::follow(ui, id, if on { 1.0 } else { 0.0 }, speed)
}

// ---------------------------------------------------------------------------------------------
// Chips, pills and tabs

/// A delta chip ("+14.8%"): lime tint for positive, rose for negative. Returns its rect.
pub(crate) fn chip(painter: &Painter, t: &Theme, left_center: Pos2, s: &str, positive: bool) -> Rect {
    let font = f(12.0);
    let w = text_w(painter, s, font.clone()) + 12.0;
    let r = Rect::from_min_size(pos2(left_center.x, left_center.y - 10.0), vec2(w, 20.0));
    let (bg, fg) = if positive {
        if t.dark {
            (fade(hex("#9AE600"), 0.2), hex("#BBF451"))
        } else {
            (hex("#D8F999"), hex("#35530E"))
        }
    } else if t.dark {
        (fade(hex("#FF2056"), 0.2), hex("#FF8FA6"))
    } else {
        (hex("#FFE0E6"), hex("#A50036"))
    };
    painter.rect_filled(r, cr(6.0), bg);
    text(painter, r.center(), Align2::CENTER_CENTER, s, font, fg);
    r
}

/// A white pill button with optional leading calendar icon and trailing chevron. `rect` is the
/// pill; `hover` brightens the border.
pub(crate) fn pill(painter: &Painter, t: &Theme, rect: Rect, label: &str, calendar: bool, hover: f32) {
    painter.rect(
        rect,
        cr(8.0),
        tile_bg(t),
        Stroke::new(1.0, mix(hairline(t), t.muted_foreground, 0.25 * hover)),
        egui::StrokeKind::Inside,
    );
    let mut x = rect.min.x + 10.0;
    if calendar {
        icon_calendar(painter, pos2(x + 6.0, rect.center().y), 12.0, dim(t));
        x += 18.0;
    }
    text(painter, pos2(x, rect.center().y), Align2::LEFT_CENTER, label, f(13.0), t.foreground);
    chevron(painter, pos2(rect.max.x - 13.0, rect.center().y), 7.0, Dir::Down, dim(t));
}

/// Preferred width of [`pill`].
pub(crate) fn pill_w(painter: &Painter, label: &str, calendar: bool) -> f32 {
    text_w(painter, label, f(13.0)) + if calendar { 18.0 } else { 0.0 } + 40.0
}

/// A date-range pill: chevrons on both ends, label centered. Returns (left, right) click rects.
pub(crate) fn range_pill(painter: &Painter, t: &Theme, rect: Rect, label: &str, hl: [f32; 2]) -> (Rect, Rect) {
    painter.rect(rect, cr(8.0), tile_bg(t), Stroke::new(1.0, hairline(t)), egui::StrokeKind::Inside);
    let l = Rect::from_min_size(rect.min, vec2(26.0, rect.height()));
    let r = Rect::from_min_size(pos2(rect.max.x - 26.0, rect.min.y), vec2(26.0, rect.height()));
    for (i, (rr, d)) in [(l, Dir::Left), (r, Dir::Right)].into_iter().enumerate() {
        if hl[i] > 0.01 {
            painter.rect_filled(rr.shrink(3.0), cr(6.0), fade(track(t), hl[i]));
        }
        chevron(painter, rr.center(), 7.0, d, mix(dim(t), t.foreground, hl[i]));
    }
    text(painter, rect.center(), Align2::CENTER_CENTER, label, f(13.0), t.foreground);
    (l, r)
}

/// Segmented tabs drawn as plain labels with a sliding white knob behind the active one.
/// Returns the label rects so the caller can hit-test them. `knob` is the animated
/// (x, width) of the knob.
pub(crate) fn tabs_layout(painter: &Painter, right: Pos2, labels: &[&str], size: f32) -> Vec<Rect> {
    let font = f(size);
    let ws: Vec<f32> = labels.iter().map(|l| text_w(painter, l, font.clone()) + 20.0).collect();
    let total: f32 = ws.iter().sum();
    let mut x = right.x - total;
    let mut out = Vec::new();
    for w in ws {
        out.push(Rect::from_min_size(pos2(x, right.y - 13.0), vec2(w, 26.0)));
        x += w;
    }
    out
}

/// Paint segmented tabs from [`tabs_layout`] rects with the knob at `knob` (x, w).
pub(crate) fn tabs_paint(painter: &Painter, t: &Theme, rects: &[Rect], labels: &[&str], sel: usize, knob: (f32, f32), size: f32) {
    if let Some(r0) = rects.first() {
        let k = Rect::from_min_size(pos2(knob.0, r0.min.y), vec2(knob.1, r0.height()));
        if t.dark {
            painter.rect_filled(k, cr(7.0), shade(t.muted, 0.12));
        } else {
            paint::soft_shadow(painter, k, 7.0, 4.0, vec2(0.0, 1.0), Color32::from_rgba_unmultiplied(0, 0, 0, 14));
            painter.rect_filled(k, cr(7.0), t.card);
        }
    }
    for (i, r) in rects.iter().enumerate() {
        let c = if i == sel { t.foreground } else { dim(t) };
        text(painter, r.center(), Align2::CENTER_CENTER, labels[i], f(size), c);
    }
}

/// Handle tab clicks + knob springs; returns (selected, knob).
pub(crate) fn tabs_logic(ui: &Ui, id: Id, rects: &[Rect], sel: usize) -> (usize, (f32, f32)) {
    let mut sel = sel.min(rects.len().saturating_sub(1));
    for (i, r) in rects.iter().enumerate() {
        let resp = ui.interact(*r, id.with(("tab", i)), egui::Sense::click());
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            sel = i;
        }
    }
    let target = rects.get(sel).map(|r| (r.min.x, r.width())).unwrap_or((0.0, 0.0));
    let x = glowkit_core::anim::spring(ui, id.with("knob_x"), target.0, 380.0, 30.0);
    let w = glowkit_core::anim::spring(ui, id.with("knob_w"), target.1, 380.0, 30.0);
    (sel, (x, w))
}

/// A small coloured square legend dot.
pub(crate) fn swatch(painter: &Painter, c: Pos2, color: Color32) {
    painter.rect_filled(Rect::from_center_size(c, vec2(9.0, 9.0)), cr(2.5), color);
}

#[allow(clippy::too_many_arguments)]
/// A white stat tile: optional colour swatch + label on top, value below.
pub(crate) fn stat_tile(painter: &Painter, t: &Theme, rect: Rect, label: &str, value: &str, color: Option<Color32>, focus: f32, value_first: bool) {
    tile(painter, t, rect, 10.0);
    if focus > 0.01 {
        let c = color.unwrap_or(t.foreground);
        painter.rect_stroke(rect, cr(10.0), Stroke::new(1.5, fade(c, focus * 0.9)), egui::StrokeKind::Inside);
    }
    let x = rect.min.x + 10.0;
    let (top, bottom) = if value_first { (value, label) } else { (label, value) };
    let mut lx = x;
    if let Some(c) = color {
        swatch(painter, pos2(x + 4.5, rect.min.y + 17.0), c);
        lx += 15.0;
    }
    let (ct, cb) = if value_first { (t.foreground, dim(t)) } else { (dim(t), t.foreground) };
    text(painter, pos2(lx, rect.min.y + 17.0), Align2::LEFT_CENTER, top, f(13.5), ct);
    text(painter, pos2(x, rect.min.y + 38.0), Align2::LEFT_CENTER, bottom, f(13.5), cb);
}

// ---------------------------------------------------------------------------------------------
// Icons

/// Chevron direction.
#[derive(Clone, Copy)]
pub(crate) enum Dir {
    Left,
    Right,
    Down,
}

/// A thin chevron.
pub(crate) fn chevron(painter: &Painter, c: Pos2, size: f32, d: Dir, color: Color32) {
    let s = size / 2.0;
    let pts = match d {
        Dir::Down => vec![c + vec2(-s, -s * 0.5), c + vec2(0.0, s * 0.5), c + vec2(s, -s * 0.5)],
        Dir::Left => vec![c + vec2(s * 0.5, -s), c + vec2(-s * 0.5, 0.0), c + vec2(s * 0.5, s)],
        Dir::Right => vec![c + vec2(-s * 0.5, -s), c + vec2(s * 0.5, 0.0), c + vec2(-s * 0.5, s)],
    };
    painter.add(Shape::line(pts, Stroke::new(1.4, color)));
}

/// A small calendar.
pub(crate) fn icon_calendar(painter: &Painter, c: Pos2, size: f32, color: Color32) {
    let r = Rect::from_center_size(c + vec2(0.0, size * 0.05), vec2(size, size * 0.9));
    let st = Stroke::new(1.2, color);
    painter.rect_stroke(r, cr(2.0), st, egui::StrokeKind::Middle);
    painter.line_segment([pos2(r.min.x, r.min.y + size * 0.3), pos2(r.max.x, r.min.y + size * 0.3)], st);
    for dx in [-0.22, 0.22] {
        let x = c.x + dx * size;
        painter.line_segment([pos2(x, r.min.y - size * 0.12), pos2(x, r.min.y + size * 0.12)], st);
    }
}

/// A magnifier.
pub(crate) fn icon_search(painter: &Painter, c: Pos2, size: f32, color: Color32) {
    let st = Stroke::new(1.3, color);
    let r = size * 0.32;
    let o = c + vec2(-size * 0.08, -size * 0.08);
    painter.circle_stroke(o, r, st);
    painter.line_segment([o + vec2(r * 0.72, r * 0.72), c + vec2(size * 0.42, size * 0.42)], st);
}

/// A light bulb.
pub(crate) fn icon_bulb(painter: &Painter, c: Pos2, size: f32, color: Color32) {
    let st = Stroke::new(1.2, color);
    let r = size * 0.3;
    let o = c + vec2(0.0, -size * 0.12);
    let pts: Vec<Pos2> = (0..=24)
        .map(|i| {
            let a = PI * 0.75 + (PI * 1.5) * i as f32 / 24.0;
            o + vec2(a.cos(), a.sin()) * r
        })
        .collect();
    let (a, b) = (pts[0], pts[pts.len() - 1]);
    painter.add(Shape::line(pts, st));
    let y1 = c.y + size * 0.28;
    painter.line_segment([a, pos2(a.x + r * 0.15, y1)], st);
    painter.line_segment([b, pos2(b.x - r * 0.15, y1)], st);
    painter.line_segment([pos2(a.x + r * 0.15, y1), pos2(b.x - r * 0.15, y1)], st);
    painter.line_segment([pos2(c.x - r * 0.45, y1 + size * 0.13), pos2(c.x + r * 0.45, y1 + size * 0.13)], st);
}

/// A four-point sparkle cluster.
pub(crate) fn icon_sparkles(painter: &Painter, c: Pos2, size: f32, color: Color32, phase: f32) {
    let k1 = 0.75 + 0.25 * (phase * TAU).sin();
    let k2 = 0.75 + 0.25 * (phase * TAU + 2.0).sin();
    let k3 = 0.75 + 0.25 * (phase * TAU + 4.0).sin();
    paint::sparkle(painter, c + vec2(-size * 0.08, size * 0.08), size * 0.42 * k1, 0.0, color);
    paint::sparkle(painter, c + vec2(size * 0.3, -size * 0.28), size * 0.2 * k2, 0.0, color);
    paint::sparkle(painter, c + vec2(-size * 0.34, -size * 0.32), size * 0.14 * k3, 0.0, color);
}

/// A tick mark.
pub(crate) fn icon_check(painter: &Painter, c: Pos2, size: f32, color: Color32, width: f32) {
    let s = size / 2.0;
    painter.add(Shape::line(
        vec![c + vec2(-0.62 * s, 0.02 * s), c + vec2(-0.18 * s, 0.46 * s), c + vec2(0.66 * s, -0.48 * s)],
        Stroke::new(width, color),
    ));
}

/// An x.
pub(crate) fn icon_close(painter: &Painter, c: Pos2, size: f32, color: Color32) {
    let s = size / 2.0;
    let st = Stroke::new(1.4, color);
    painter.line_segment([c + vec2(-s, -s), c + vec2(s, s)], st);
    painter.line_segment([c + vec2(-s, s), c + vec2(s, -s)], st);
}

/// A plus.
pub(crate) fn icon_plus(painter: &Painter, c: Pos2, size: f32, color: Color32) {
    let s = size / 2.0;
    let st = Stroke::new(1.5, color);
    painter.line_segment([c + vec2(-s, 0.0), c + vec2(s, 0.0)], st);
    painter.line_segment([c + vec2(0.0, -s), c + vec2(0.0, s)], st);
}

/// An upward arrow.
pub(crate) fn icon_arrow_up(painter: &Painter, c: Pos2, size: f32, color: Color32) {
    let s = size / 2.0;
    let st = Stroke::new(1.7, color);
    painter.line_segment([c + vec2(0.0, s), c + vec2(0.0, -s)], st);
    painter.add(Shape::line(vec![c + vec2(-s * 0.7, -s * 0.25), c + vec2(0.0, -s), c + vec2(s * 0.7, -s * 0.25)], st));
}

/// A right arrow.
pub(crate) fn icon_arrow_right(painter: &Painter, c: Pos2, size: f32, color: Color32) {
    let s = size / 2.0;
    let st = Stroke::new(1.3, color);
    painter.line_segment([c + vec2(-s, 0.0), c + vec2(s, 0.0)], st);
    painter.add(Shape::line(vec![c + vec2(s * 0.3, -s * 0.65), c + vec2(s, 0.0), c + vec2(s * 0.3, s * 0.65)], st));
}

/// A microphone.
pub(crate) fn icon_mic(painter: &Painter, c: Pos2, size: f32, color: Color32) {
    let st = Stroke::new(1.3, color);
    let body = Rect::from_center_size(c + vec2(0.0, -size * 0.12), vec2(size * 0.36, size * 0.55));
    painter.rect_stroke(body, cr(size * 0.18), st, egui::StrokeKind::Middle);
    let pts: Vec<Pos2> = (0..=16)
        .map(|i| {
            let a = PI * i as f32 / 16.0;
            c + vec2(-a.cos() * size * 0.32, a.sin() * size * 0.3 - size * 0.08)
        })
        .collect();
    painter.add(Shape::line(pts, st));
    painter.line_segment([c + vec2(0.0, size * 0.22), c + vec2(0.0, size * 0.42)], st);
}

/// Simple glyph icons used inside stage pills and notification badges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Glyph {
    Eye,
    UserPlus,
    Bolt,
    Crown,
    Users,
    Building,
    Cloud,
    At,
    Gear,
}

/// Paint a [`Glyph`] centered at `c`.
pub(crate) fn glyph(painter: &Painter, g: Glyph, c: Pos2, size: f32, color: Color32) {
    let s = size / 2.0;
    let st = Stroke::new((size * 0.1).clamp(1.0, 1.6), color);
    let p = |x: f32, y: f32| c + vec2(x * s, y * s);
    match g {
        Glyph::Eye => {
            let up: Vec<Pos2> = (0..=16).map(|i| {
                let u = i as f32 / 16.0 * 2.0 - 1.0;
                p(u * 0.95, -(1.0 - u * u) * 0.55)
            }).collect();
            let dn: Vec<Pos2> = up.iter().map(|q| pos2(q.x, c.y + (c.y - q.y))).collect();
            painter.add(Shape::line(up, st));
            painter.add(Shape::line(dn, st));
            painter.circle_stroke(c, s * 0.26, st);
        }
        Glyph::UserPlus => {
            painter.circle_stroke(p(-0.2, -0.38), s * 0.3, st);
            let arc: Vec<Pos2> = (0..=12).map(|i| {
                let a = PI + PI * i as f32 / 12.0;
                p(-0.2, 0.75) + vec2(a.cos() * s * 0.62, a.sin() * s * 0.6)
            }).collect();
            painter.add(Shape::line(arc, st));
            painter.line_segment([p(0.55, 0.0), p(0.95, 0.0)], st);
            painter.line_segment([p(0.75, -0.2), p(0.75, 0.2)], st);
        }
        Glyph::Bolt => {
            painter.add(Shape::closed_line(
                vec![p(0.15, -0.95), p(-0.55, 0.12), p(-0.02, 0.12), p(-0.15, 0.95), p(0.55, -0.12), p(0.02, -0.12)],
                st,
            ));
        }
        Glyph::Crown => {
            painter.add(Shape::closed_line(
                vec![p(-0.85, 0.55), p(-0.85, -0.45), p(-0.4, 0.0), p(0.0, -0.65), p(0.4, 0.0), p(0.85, -0.45), p(0.85, 0.55)],
                st,
            ));
        }
        Glyph::Users => {
            for (dx, k) in [(-0.3, 1.0), (0.38, 0.85)] {
                painter.circle_stroke(p(dx, -0.35), s * 0.27 * k, st);
                let arc: Vec<Pos2> = (0..=12).map(|i| {
                    let a = PI + PI * i as f32 / 12.0;
                    p(dx, 0.75) + vec2(a.cos() * s * 0.5 * k, a.sin() * s * 0.55 * k)
                }).collect();
                painter.add(Shape::line(arc, st));
            }
        }
        Glyph::Building => {
            painter.rect_stroke(Rect::from_min_max(p(-0.75, -0.8), p(0.15, 0.85)), cr(1.0), st, egui::StrokeKind::Middle);
            painter.rect_stroke(Rect::from_min_max(p(0.15, -0.2), p(0.8, 0.85)), cr(1.0), st, egui::StrokeKind::Middle);
            for y in [-0.45, 0.0, 0.45] {
                painter.line_segment([p(-0.48, y), p(-0.15, y)], st);
            }
        }
        Glyph::Cloud => {
            let pts: Vec<Pos2> = (0..=28).map(|i| {
                let a = PI * 0.85 + PI * 1.3 * i as f32 / 28.0;
                let bump = 1.0 + 0.25 * (a * 3.0).sin().abs();
                p(0.0, 0.05) + vec2(a.cos() * s * 0.8, a.sin() * s * 0.55 * bump)
            }).collect();
            painter.add(Shape::line(pts, st));
            painter.line_segment([p(0.0, -0.05), p(0.0, 0.85)], st);
            painter.add(Shape::line(vec![p(-0.3, 0.55), p(0.0, 0.85), p(0.3, 0.55)], st));
        }
        Glyph::At => {
            painter.circle_stroke(c, s * 0.35, st);
            let pts: Vec<Pos2> = (0..=28).map(|i| {
                let a = 0.2 + TAU * 0.8 * i as f32 / 28.0;
                c + vec2(a.cos(), a.sin()) * s * 0.85
            }).collect();
            painter.add(Shape::line(pts, st));
            painter.line_segment([p(0.35, -0.35), p(0.35, 0.3)], st);
        }
        Glyph::Gear => {
            painter.circle_stroke(c, s * 0.3, st);
            for i in 0..8 {
                let a = TAU * i as f32 / 8.0;
                let d = vec2(a.cos(), a.sin());
                painter.line_segment([c + d * s * 0.55, c + d * s * 0.85], Stroke::new(st.width * 1.6, color));
            }
            painter.circle_stroke(c, s * 0.62, st);
        }
    }
}

/// Paint a round-capped arc from angle `a0` sweeping `sweep` radians (clockwise from 3 o'clock).
pub(crate) fn ring_arc(painter: &Painter, c: Pos2, r: f32, a0: f32, sweep: f32, width: f32, color: Color32) {
    if sweep.abs() < 1e-3 {
        painter.circle_filled(c + vec2(a0.cos(), a0.sin()) * r, width / 2.0, color);
        return;
    }
    paint::arc(painter, c, r, a0, a0 + sweep, width, color);
}

/// Concentric activity rings. `rings` = (progress, colour, emphasis 0..1) from outside in.
pub(crate) fn paint_rings(painter: &Painter, t: &Theme, c: Pos2, radius: f32, width: f32, gap: f32, rings: &[(f32, Color32, f32)]) {
    let start = -PI / 2.0;
    for (i, (p, col, emph)) in rings.iter().enumerate() {
        let r = radius - width / 2.0 - i as f32 * (width + gap);
        if r <= 1.0 {
            break;
        }
        let a = 0.35 + 0.65 * emph;
        let track_a = if t.dark { 0.22 } else { 0.2 };
        paint::arc_band(painter, c, r - width / 2.0, r + width / 2.0, 0.0, TAU, fade(*col, track_a * a));
        let p = p.max(0.0);
        if p > 0.0005 {
            let col = fade(*col, a);
            let first = p.min(1.0);
            ring_arc(painter, c, r, start, TAU * first, width, col);
            if p > 1.0 {
                // second lap: soft shadow at the head so the overlap reads
                let head = start + TAU * p;
                let hp = c + vec2(head.cos(), head.sin()) * r;
                paint::glow(painter, hp + vec2(-head.sin(), head.cos()) * width * 0.3, width * 0.9, Color32::from_black_alpha(40));
                ring_arc(painter, c, r, start + TAU, TAU * (p - 1.0).min(1.0), width, shade(col, 0.03));
            }
        }
    }
}
