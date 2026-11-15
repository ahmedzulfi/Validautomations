//! Shared look and helpers for the agent supervision blocks: the [`RunState`] vocabulary, soft
//! panels, tone badges, status icons, pill buttons, switches and number formatting.

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Galley, Id, Painter, Pos2, Rect, Response, Sense,
    Shape, Stroke, StrokeKind, Ui,
};
use glowkit_ai::{paint_icon, Icon};
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::TAU;
use std::sync::Arc;

/// The life-cycle state of an agent, a run, a step or a session. Every supervision block uses
/// the same words and colors for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum RunState {
    /// Waiting for a free worker.
    #[default]
    Queued,
    /// Working right now.
    Running,
    /// Blocked on a person (approval, question, review).
    NeedsInput,
    /// Finished successfully.
    Done,
    /// Finished with an error.
    Failed,
    /// Paused by a person.
    Paused,
    /// Alive but doing nothing.
    Idle,
}

impl RunState {
    /// Short label, e.g. `"Running"`.
    pub fn label(self) -> &'static str {
        match self {
            RunState::Queued => "Queued",
            RunState::Running => "Running",
            RunState::NeedsInput => "Needs you",
            RunState::Done => "Done",
            RunState::Failed => "Failed",
            RunState::Paused => "Paused",
            RunState::Idle => "Idle",
        }
    }
    /// The state's signal color in theme `t`.
    pub fn color(self, t: &Theme) -> Color32 {
        match self {
            RunState::Queued | RunState::Idle => t.muted_foreground,
            RunState::Running => t.info,
            RunState::NeedsInput => t.warning,
            RunState::Done => t.success,
            RunState::Failed => t.destructive,
            RunState::Paused => mix(t.muted_foreground, t.foreground, 0.35),
        }
    }
    /// True while the state still changes by itself (running or queued).
    pub fn is_active(self) -> bool {
        matches!(self, RunState::Running | RunState::Queued)
    }
}

// ---------------------------------------------------------------------------------------------
// Palette

/// Panel fill: white on the light stage, a lifted tone in dark themes.
pub(crate) fn surface(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.muted, 0.06)
    } else {
        t.card
    }
}

/// A faint inset fill for rows, wells and tracks inside a panel.
pub(crate) fn well(t: &Theme) -> Color32 {
    if t.dark {
        mix(surface(t), t.foreground, 0.045)
    } else {
        mix(t.card, t.muted, 0.75)
    }
}

/// Track color for meters and bars.
pub(crate) fn track(t: &Theme) -> Color32 {
    mix(surface(t), t.foreground, if t.dark { 0.1 } else { 0.075 })
}

/// Hairline color.
pub(crate) fn hair(t: &Theme) -> Color32 {
    if t.dark {
        mix(t.border, t.foreground, 0.04)
    } else {
        t.border
    }
}

/// Secondary text.
pub(crate) fn dim(t: &Theme) -> Color32 {
    t.muted_foreground
}

/// Tertiary text (timestamps, ids).
pub(crate) fn faint(t: &Theme) -> Color32 {
    mix(t.muted_foreground, surface(t), 0.3)
}

/// Background tint for a soft badge of color `c`.
pub(crate) fn tint(t: &Theme, c: Color32) -> Color32 {
    with_alpha(c, if t.dark { 0.17 } else { 0.1 })
}

/// Readable text color for a soft badge of color `c`.
pub(crate) fn ink(t: &Theme, c: Color32) -> Color32 {
    if t.dark {
        mix(c, Color32::WHITE, 0.25)
    } else {
        shade(c, -0.12)
    }
}

/// Rounded-rect corner radius from a float.
pub(crate) fn cr(r: f32) -> CornerRadius {
    CornerRadius::same(r.round().clamp(0.0, 255.0) as u8)
}

/// A soft elevated panel: diffuse shadow, surface fill and a hairline.
pub(crate) fn panel(painter: &Painter, t: &Theme, rect: Rect, radius: f32) {
    panel_fill(painter, t, rect, radius, surface(t), 1.0);
}

/// [`panel`] with an explicit fill and shadow strength.
pub(crate) fn panel_fill(painter: &Painter, t: &Theme, rect: Rect, radius: f32, fill: Color32, elevation: f32) {
    if elevation > 0.0 {
        let sc = if t.dark { Color32::from_black_alpha(90) } else { Color32::from_rgba_unmultiplied(24, 24, 40, 14) };
        paint::soft_shadow(painter, rect, radius, 14.0 * elevation, vec2(0.0, 3.0 * elevation), sc);
    }
    painter.rect(rect, cr(radius), fill, Stroke::new(1.0, hair(t)), StrokeKind::Inside);
}

/// Horizontal hairline.
pub(crate) fn hline(painter: &Painter, t: &Theme, x0: f32, x1: f32, y: f32) {
    painter.line_segment([pos2(x0, y), pos2(x1, y)], Stroke::new(1.0, hair(t)));
}

// ---------------------------------------------------------------------------------------------
// Text

/// Proportional font of `size`.
pub(crate) fn font(size: f32) -> FontId {
    FontId::proportional(size)
}

/// Monospace font of `size`.
pub(crate) fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}

/// Lay out `text` on one row, elided with "…" past `max_w`.
pub(crate) fn galley1(painter: &Painter, text: &str, font: FontId, color: Color32, max_w: f32) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(text.to_owned(), TextFormat::simple(font, color));
    job.wrap = TextWrapping { max_width: max_w.max(4.0), max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
    painter.layout_job(job)
}

/// Paint one elided row of text anchored at `pos` with `align`; returns its rect.
pub(crate) fn text(
    painter: &Painter,
    pos: Pos2,
    align: Align2,
    s: &str,
    font: FontId,
    color: Color32,
    max_w: f32,
) -> Rect {
    let g = galley1(painter, s, font, color, max_w);
    let r = align.anchor_size(pos, g.size());
    painter.galley(r.min, g, color);
    r
}

/// Paint wrapped text inside `width`, at most `rows` rows; returns its rect.
pub(crate) fn para(
    painter: &Painter,
    pos: Pos2,
    s: &str,
    font: FontId,
    color: Color32,
    width: f32,
    rows: usize,
) -> Rect {
    let mut job = LayoutJob::single_section(s.to_owned(), TextFormat::simple(font, color));
    job.wrap = TextWrapping { max_width: width.max(4.0), max_rows: rows, break_anywhere: false, overflow_character: Some('…') };
    let g = painter.layout_job(job);
    let r = Rect::from_min_size(pos, g.size());
    painter.galley(pos, g, color);
    r
}

/// Width of `s` in `font`.
pub(crate) fn text_w(painter: &Painter, s: &str, font: FontId) -> f32 {
    painter.layout_no_wrap(s.to_owned(), font, Color32::WHITE).size().x
}

// ---------------------------------------------------------------------------------------------
// Badges and status marks

/// A soft rounded badge of color `c` (tinted fill, colored text, optional leading dot). Anchored
/// at `pos` with `align`; returns its rect.
pub(crate) fn badge(painter: &Painter, t: &Theme, pos: Pos2, align: Align2, label: &str, c: Color32, dot: bool) -> Rect {
    let f = font(11.0);
    let tw = text_w(painter, label, f.clone());
    let pad = 7.0;
    let dw = if dot { 10.0 } else { 0.0 };
    let size = vec2(tw + pad * 2.0 + dw, 19.0);
    let r = align.anchor_size(pos, size);
    painter.rect_filled(r, cr(6.0), tint(t, c));
    if dot {
        painter.circle_filled(pos2(r.min.x + pad + 2.5, r.center().y), 2.6, c);
    }
    painter.text(pos2(r.min.x + pad + dw, r.center().y), Align2::LEFT_CENTER, label, f, ink(t, c));
    r
}

/// A neutral outline badge (for counts, ids, model names).
pub(crate) fn chip(painter: &Painter, t: &Theme, pos: Pos2, align: Align2, label: &str, monospace: bool) -> Rect {
    let f = if monospace { mono(10.5) } else { font(11.0) };
    let tw = text_w(painter, label, f.clone());
    let r = align.anchor_size(pos, vec2(tw + 12.0, 19.0));
    painter.rect(r, cr(6.0), well(t), Stroke::new(1.0, hair(t)), StrokeKind::Inside);
    painter.text(r.center(), Align2::CENTER_CENTER, label, f, dim(t));
    r
}

/// A [`RunState`] badge with a dot that pulses while running.
pub(crate) fn state_badge(painter: &Painter, t: &Theme, pos: Pos2, align: Align2, s: RunState, now: f32) -> Rect {
    let c = s.color(t);
    let r = badge(painter, t, pos, align, s.label(), c, true);
    if s == RunState::Running || s == RunState::NeedsInput {
        let p = (now * 1.4).fract();
        painter.circle_filled(pos2(r.min.x + 9.5, r.center().y), 2.6 + p * 4.0, with_alpha(c, 0.35 * (1.0 - p)));
    }
    r
}

/// A rotating spinner arc.
pub(crate) fn spinner(painter: &Painter, c: Pos2, r: f32, color: Color32, now: f32, width: f32) {
    painter.circle_stroke(c, r, Stroke::new(width, with_alpha(color, 0.18)));
    let a0 = now * 5.2;
    let len = 1.6 + 0.9 * (now * 2.1).sin();
    paint::arc(painter, c, r, a0, a0 + len, width, color);
}

/// A round status mark: spinner, check, cross, pause bars, amber "!" or a hollow ring.
pub(crate) fn state_icon(painter: &Painter, t: &Theme, c: Pos2, r: f32, s: RunState, now: f32) {
    let col = s.color(t);
    let w = (r * 0.22).clamp(1.2, 2.2);
    match s {
        RunState::Running => spinner(painter, c, r * 0.82, col, now, w),
        RunState::Done => {
            painter.circle_filled(c, r, col);
            let ir = Rect::from_center_size(c, vec2(r * 1.25, r * 1.25));
            glowkit_ai::paint_icon_width(painter, Icon::Check, ir, Color32::WHITE, w.max(1.5));
        }
        RunState::Failed => {
            painter.circle_filled(c, r, col);
            let ir = Rect::from_center_size(c, vec2(r * 1.1, r * 1.1));
            glowkit_ai::paint_icon_width(painter, Icon::X, ir, Color32::WHITE, w.max(1.5));
        }
        RunState::NeedsInput => {
            let p = 0.5 + 0.5 * (now * 3.0).sin();
            painter.circle_filled(c, r + 2.0 * p, with_alpha(col, 0.18 * p));
            painter.circle_filled(c, r, col);
            painter.line_segment([c - vec2(0.0, r * 0.45), c + vec2(0.0, r * 0.1)], Stroke::new(w.max(1.5), Color32::WHITE));
            painter.circle_filled(c + vec2(0.0, r * 0.45), w * 0.6, Color32::WHITE);
        }
        RunState::Paused => {
            painter.circle_stroke(c, r * 0.9, Stroke::new(w, col));
            for dx in [-1.0, 1.0] {
                painter.line_segment(
                    [c + vec2(dx * r * 0.22, -r * 0.35), c + vec2(dx * r * 0.22, r * 0.35)],
                    Stroke::new(w, col),
                );
            }
        }
        RunState::Queued | RunState::Idle => {
            dashed_circle(painter, c, r * 0.85, Stroke::new(w, with_alpha(col, 0.8)), 10);
        }
    }
}

/// A dashed circle outline of `n` dashes.
pub(crate) fn dashed_circle(painter: &Painter, c: Pos2, r: f32, stroke: Stroke, n: usize) {
    for i in 0..n {
        let a0 = i as f32 / n as f32 * TAU;
        let a1 = a0 + TAU / n as f32 * 0.55;
        let pts: Vec<Pos2> = (0..=4).map(|k| c + egui::Vec2::angled(a0 + (a1 - a0) * k as f32 / 4.0) * r).collect();
        painter.add(Shape::line(pts, stroke));
    }
}

/// Paint a line icon centered at `c` of `size`.
pub(crate) fn icon(painter: &Painter, ic: Icon, c: Pos2, size: f32, color: Color32) {
    paint_icon(painter, ic, Rect::from_center_size(c, vec2(size, size)), color);
}

/// A rounded-square agent mark with initials on a soft tone of `c`.
pub(crate) fn avatar(painter: &Painter, t: &Theme, c: Pos2, size: f32, initials: &str, col: Color32) {
    let r = Rect::from_center_size(c, vec2(size, size));
    painter.rect_filled(r, cr(size * 0.3), tint(t, col));
    painter.rect_stroke(r, cr(size * 0.3), Stroke::new(1.0, with_alpha(col, 0.25)), StrokeKind::Inside);
    painter.text(c, Align2::CENTER_CENTER, initials, FontId::proportional(size * 0.4), ink(t, col));
}

/// Color for item `i` from the theme chart palette.
pub(crate) fn series(t: &Theme, i: usize) -> Color32 {
    t.chart[i % t.chart.len()]
}

// ---------------------------------------------------------------------------------------------
// Controls

/// Look of a [`button`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Btn {
    /// Solid foreground fill.
    Primary,
    /// Surface fill with a hairline.
    Secondary,
    /// Text only, fill on hover.
    Ghost,
    /// Soft red.
    Danger,
}

/// A rounded button drawn in `rect`; returns the click response. Hover eases in.
pub(crate) fn button(ui: &Ui, rect: Rect, id: Id, label: &str, ic: Option<Icon>, kind: Btn) -> Response {
    let t = Theme::of(ui);
    let resp = ui.interact(rect, id, Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let h = glowkit_core::anim::follow(ui, id.with("hov"), if resp.hovered() { 1.0 } else { 0.0 }, 14.0);
    let press = if resp.is_pointer_button_down_on() { 0.97 } else { 1.0 };
    let r = Rect::from_center_size(rect.center(), rect.size() * press);
    let p = ui.painter();
    let (fill, fg, stroke) = match kind {
        Btn::Primary => (mix(t.primary, t.muted_foreground, 0.12 * h), t.primary_foreground, Stroke::NONE),
        Btn::Secondary => (mix(surface(&t), well(&t), 0.3 + h), t.foreground, Stroke::new(1.0, hair(&t))),
        Btn::Ghost => (with_alpha(t.foreground, 0.06 * h), dim(&t), Stroke::NONE),
        Btn::Danger => (with_alpha(t.destructive, 0.1 + 0.06 * h), ink(&t, t.destructive), Stroke::NONE),
    };
    p.rect(r, cr(r.height().min(16.0) * 0.5), fill, stroke, StrokeKind::Inside);
    let f = font(12.0);
    let tw = if label.is_empty() { 0.0 } else { text_w(p, label, f.clone()) };
    let iw = if ic.is_some() { 13.0 + if label.is_empty() { 0.0 } else { 5.0 } } else { 0.0 };
    let x0 = r.center().x - (tw + iw) / 2.0;
    if let Some(ic) = ic {
        icon(p, ic, pos2(x0 + 6.5, r.center().y), 13.0, fg);
    }
    if !label.is_empty() {
        p.text(pos2(x0 + iw, r.center().y), Align2::LEFT_CENTER, label, f, fg);
    }
    resp
}

/// Width a [`button`] needs for `label` (plus an icon).
pub(crate) fn button_w(painter: &Painter, label: &str, with_icon: bool) -> f32 {
    let tw = if label.is_empty() { 0.0 } else { text_w(painter, label, font(12.0)) };
    tw + if with_icon { 18.0 } else { 0.0 } + if label.is_empty() { 12.0 } else { 22.0 }
}

/// A small sliding switch in `rect` (springy knob). Flips `on` when clicked.
pub(crate) fn switch(ui: &Ui, rect: Rect, id: Id, on: &mut bool, accent: Color32) -> Response {
    let t = Theme::of(ui);
    let resp = ui.interact(rect, id, Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if resp.clicked() {
        *on = !*on;
    }
    let k = glowkit_core::anim::spring(ui, id.with("knob"), if *on { 1.0 } else { 0.0 }, 380.0, 22.0);
    let p = ui.painter();
    let fill = mix(track(&t), accent, k.clamp(0.0, 1.0));
    p.rect_filled(rect, cr(rect.height() / 2.0), fill);
    let kr = rect.height() / 2.0 - 2.0;
    let x = egui::lerp(rect.min.x + kr + 2.0..=rect.max.x - kr - 2.0, k);
    paint::soft_shadow(p, Rect::from_center_size(pos2(x, rect.center().y), vec2(kr * 2.0, kr * 2.0)), kr, 4.0, vec2(0.0, 1.0), Color32::from_black_alpha(40));
    p.circle_filled(pos2(x, rect.center().y), kr, Color32::WHITE);
    resp
}

/// A horizontal meter: track plus a filled part of `frac` in `color`.
pub(crate) fn meter(painter: &Painter, t: &Theme, rect: Rect, frac: f32, color: Color32) {
    painter.rect_filled(rect, cr(rect.height() / 2.0), track(t));
    let f = frac.clamp(0.0, 1.0);
    if f > 0.001 {
        let w = (rect.width() * f).max(rect.height());
        painter.rect_filled(Rect::from_min_size(rect.min, vec2(w, rect.height())), cr(rect.height() / 2.0), color);
    }
}

/// A small sparkline with a soft area under it.
pub(crate) fn sparkline(painter: &Painter, rect: Rect, values: &[f32], color: Color32) {
    if values.len() < 2 {
        return;
    }
    let (lo, hi) = values.iter().fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
    let span = (hi - lo).max(1e-3);
    let pts: Vec<Pos2> = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            pos2(
                rect.min.x + rect.width() * i as f32 / (values.len() - 1) as f32,
                rect.max.y - (v - lo) / span * rect.height(),
            )
        })
        .collect();
    let curve = paint::monotone(&pts, 6);
    paint::area_fill(painter, &curve, rect.max.y, with_alpha(color, 0.22), with_alpha(color, 0.0));
    painter.add(Shape::line(curve.clone(), Stroke::new(1.5, color)));
    if let Some(l) = curve.last() {
        painter.circle_filled(*l, 2.4, color);
    }
}

/// Center a box of `w` (clamped to the available width) in a freshly allocated row of height `h`.
pub(crate) fn alloc_centered(ui: &mut Ui, w: f32, h: f32, sense: Sense) -> (Rect, Response) {
    let avail = ui.available_width();
    let (outer, resp) = ui.allocate_exact_size(vec2(avail, h), sense);
    let w = w.min(avail);
    (Rect::from_center_size(outer.center(), vec2(w, h)), resp)
}

// ---------------------------------------------------------------------------------------------
// Formatting

/// `"8.2s"`, `"1m 05s"`, `"2h 03m"`.
pub fn fmt_secs(s: f32) -> String {
    let s = s.max(0.0);
    if s < 10.0 {
        format!("{s:.1}s")
    } else if s < 60.0 {
        format!("{}s", s as u32)
    } else if s < 3600.0 {
        format!("{}m {:02}s", s as u32 / 60, s as u32 % 60)
    } else {
        format!("{}h {:02}m", s as u32 / 3600, (s as u32 % 3600) / 60)
    }
}

/// `"$0.042"` / `"$1.20"` / `"$1,240"`.
pub fn fmt_usd(v: f32) -> String {
    if v < 0.1 {
        format!("${v:.3}")
    } else if v < 1000.0 {
        format!("${v:.2}")
    } else {
        let n = v.round() as u64;
        format!("${},{:03}", n / 1000, n % 1000)
    }
}

/// `"12.4K"` style token counts.
pub fn fmt_tok(n: f32) -> String {
    glowkit_ai::format_tokens(n.max(0.0) as u64)
}

/// `"just now"`, `"4m ago"`, `"2h ago"`, `"3d ago"` from an age in seconds.
pub fn fmt_ago(age: f32) -> String {
    let a = age.max(0.0);
    if a < 45.0 {
        "just now".into()
    } else if a < 3600.0 {
        format!("{}m ago", (a / 60.0).round().max(1.0) as u32)
    } else if a < 86400.0 {
        format!("{}h ago", (a / 3600.0).round() as u32)
    } else {
        format!("{}d ago", (a / 86400.0).round() as u32)
    }
}

// ---------------------------------------------------------------------------------------------
// Paths

/// Turn an orthogonal polyline into a smooth one with rounded corners of `radius`.
pub(crate) fn rounded_path(pts: &[Pos2], radius: f32) -> Vec<Pos2> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let mut out = vec![pts[0]];
    for i in 1..pts.len() - 1 {
        let (a, b, c) = (pts[i - 1], pts[i], pts[i + 1]);
        let d1 = (b - a).length();
        let d2 = (c - b).length();
        let r = radius.min(d1 / 2.0).min(d2 / 2.0);
        if r < 0.5 {
            out.push(b);
            continue;
        }
        let p0 = b - (b - a).normalized() * r;
        let p1 = b + (c - b).normalized() * r;
        out.extend(paint::quad_bezier(p0, b, p1, 8));
    }
    out.push(*pts.last().unwrap());
    out
}

/// Total length of a polyline.
pub(crate) fn path_len(pts: &[Pos2]) -> f32 {
    pts.windows(2).map(|w| (w[1] - w[0]).length()).sum()
}

/// The point a fraction `f` (0..1) of the way along a polyline.
pub(crate) fn path_at(pts: &[Pos2], f: f32) -> Pos2 {
    let total = path_len(pts);
    let mut want = total * f.clamp(0.0, 1.0);
    for w in pts.windows(2) {
        let l = (w[1] - w[0]).length();
        if want <= l && l > 0.0 {
            return w[0] + (w[1] - w[0]) * (want / l);
        }
        want -= l;
    }
    *pts.last().unwrap_or(&Pos2::ZERO)
}

/// The part of a polyline between fractions `f0` and `f1`.
pub(crate) fn path_slice(pts: &[Pos2], f0: f32, f1: f32) -> Vec<Pos2> {
    let total = path_len(pts);
    let (a, b) = (total * f0.clamp(0.0, 1.0), total * f1.clamp(0.0, 1.0));
    let mut out = Vec::new();
    let mut acc = 0.0;
    for w in pts.windows(2) {
        let l = (w[1] - w[0]).length();
        let (s, e) = (acc, acc + l);
        if e >= a && s <= b && l > 0.0 {
            let p0 = w[0] + (w[1] - w[0]) * ((a - s).max(0.0) / l);
            let p1 = w[0] + (w[1] - w[0]) * ((b.min(e) - s) / l);
            if out.is_empty() {
                out.push(p0);
            }
            out.push(p1);
        }
        acc = e;
    }
    out
}

/// A glowing comet running along `pts`, head at fraction `head`, tail `len` long (fraction).
pub(crate) fn comet(painter: &Painter, pts: &[Pos2], head: f32, len: f32, color: Color32, width: f32) {
    let tail = path_slice(pts, head - len, head);
    if tail.len() >= 2 {
        paint::gradient_polyline(painter, &tail, false, width, |u| with_alpha(color, u * u));
    }
    if (0.0..=1.0).contains(&head) {
        let p = path_at(pts, head);
        paint::glow(painter, p, width * 4.0, with_alpha(color, 0.35));
        painter.circle_filled(p, width * 0.9, color);
    }
}

/// A copy of `p` whose shapes are drawn at `a` opacity.
pub(crate) fn faded(p: &Painter, a: f32) -> Painter {
    let mut q = p.clone();
    q.multiply_opacity(a.clamp(0.0, 1.0));
    q
}
