//! A contributions heat grid card (53 weeks of day cells with intensity steps).

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Id, Painter, Pos2, Rect, Response, Sense, Ui};
use glowkit_core::color::{fade, mix};
use glowkit_core::Theme;

/// A headline stat tile: a bold value over a small label ("9B" / "Lifetime tokens").
#[derive(Clone, Debug, PartialEq)]
pub struct BoardStat {
    /// The value ("562.7M").
    pub value: String,
    /// What it measures ("Peak tokens").
    pub label: String,
}

impl BoardStat {
    /// A tile showing `value` over `label`.
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self { value: value.into(), label: label.into() }
    }
}

/// Deterministic sample year of daily counts (busier mid-week, a few streaks and quiet spells).
pub(crate) fn sample_counts(seed: u64) -> Vec<u32> {
    let mut rng = glowkit_core::Rng::new(seed);
    let mut out = Vec::with_capacity(365);
    let mut streak = 0.0_f32;
    for d in 0..365 {
        let wd = d % 7;
        streak = (streak * 0.82 + rng.range(-0.6, 1.0)).clamp(-1.0, 2.2);
        let base = if wd >= 5 { 0.6 } else { 1.6 } + streak + glowkit_core::noise::noise2(d as f32 * 0.07, seed as f32) * 1.6;
        let v = (base * 2.2 + rng.range(-1.0, 1.4)).max(0.0);
        out.push(if rng.bool(0.16) { 0 } else { v.round() as u32 });
    }
    out
}

const MONTH_START: [usize; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

fn date_of(day: usize) -> String {
    let m = MONTH_START.iter().rposition(|s| *s <= day).unwrap_or(0);
    format!("{} {}", MONTHS[m], day - MONTH_START[m] + 1)
}

#[allow(clippy::needless_range_loop)]
/// Grid aggregation: 0 = day cells, 1 = monthly averages, 2 = weekly totals.
fn levels(counts: &[u32], first_wd: usize, mode: usize) -> Vec<f32> {
    let n = counts.len().min(371);
    let max_day = counts.iter().copied().max().unwrap_or(1).max(1) as f32;
    let lvl = |v: f32, max: f32| -> f32 {
        if v <= 0.0 {
            0.0
        } else {
            (v / max * 4.0).ceil().clamp(1.0, 4.0)
        }
    };
    let mut out = vec![0.0; 53 * 7];
    match mode {
        0 => {
            for (d, c) in counts.iter().take(n).enumerate() {
                let s = d + first_wd;
                if s < out.len() {
                    out[s] = lvl(*c as f32, max_day);
                }
            }
        }
        1 => {
            let mut avg = [0.0_f32; 12];
            for m in 0..12 {
                let a = MONTH_START[m];
                let b = if m == 11 { 365 } else { MONTH_START[m + 1] };
                let slice = &counts[a.min(n)..b.min(n)];
                avg[m] = slice.iter().sum::<u32>() as f32 / slice.len().max(1) as f32;
            }
            let mx = avg.iter().copied().fold(1e-3, f32::max);
            for d in 0..n {
                let m = MONTH_START.iter().rposition(|s| *s <= d).unwrap_or(0);
                let s = d + first_wd;
                if s < out.len() {
                    // a little per-weekday texture keeps the blocks from looking flat
                    let tex = if counts[d] == 0 { -1.0 } else { 0.0 };
                    out[s] = (lvl(avg[m], mx) + tex).max(1.0);
                }
            }
        }
        _ => {
            let mut wk = [0.0_f32; 53];
            for (d, c) in counts.iter().take(n).enumerate() {
                wk[((d + first_wd) / 7).min(52)] += *c as f32;
            }
            let mx = wk.iter().copied().fold(1e-3, f32::max);
            for d in 0..n {
                let s = d + first_wd;
                if s < out.len() {
                    let w = s / 7;
                    let row = s % 7;
                    let l = lvl(wk[w], mx);
                    // fill each column from the bottom like a bar
                    out[s] = if (6 - row) as f32 <= l * 1.75 - 0.5 { l } else { 0.0 };
                }
            }
        }
    }
    out
}

#[derive(Clone)]
struct HeatAnim {
    mode: usize,
    at: f32,
    prev: Vec<f32>,
    cur: Vec<f32>,
}

/// Colour for a fractional level 0..4.
fn level_color(t: &Theme, accent: Color32, l: f32) -> Color32 {
    let base = track(t);
    let steps = [
        base,
        mix(base, accent, if t.dark { 0.3 } else { 0.24 }),
        mix(base, accent, 0.5),
        mix(base, accent, 0.76),
        accent,
    ];
    let l = l.clamp(0.0, 4.0);
    let i = (l.floor() as usize).min(3);
    mix(steps[i], steps[i + 1], l - i as f32)
}

/// Paint the heat grid + month labels in `area`; handles tooltip. Returns the hovered day.
#[allow(clippy::too_many_arguments)]
pub(crate) fn heat_grid(
    ui: &Ui,
    painter: &Painter,
    id: Id,
    area: Rect,
    counts: &[u32],
    first_wd: usize,
    mode: usize,
    accent: Color32,
    el: f32,
    hover: Option<Pos2>,
) -> Option<usize> {
    let t = Theme::of(ui);
    let now = glowkit_core::time(ui);
    let target = levels(counts, first_wd, mode);
    let anim = glowkit_core::with_state(
        ui,
        id.with("heat"),
        || HeatAnim { mode, at: now - 10.0, prev: target.clone(), cur: target.clone() },
        |a| {
            if a.mode != mode {
                a.prev = a.cur.clone();
                a.cur = target.clone();
                a.mode = mode;
                a.at = now;
            } else {
                a.cur = target.clone();
            }
            a.clone()
        },
    );
    let since = now - anim.at;
    let label_h = 22.0;
    let gap = ((area.width() / 53.0) * 0.26).clamp(2.0, 4.5);
    let cell = ((area.width() + gap) / 53.0 - gap).min((area.height() - label_h + gap) / 7.0 - gap).max(3.0);
    let gw = 53.0 * (cell + gap) - gap;
    let ox = area.min.x + (area.width() - gw) / 2.0;
    let rad = (cell * 0.22).min(3.0);
    let mut hovered = None;
    let mut busy = since < 1.2;
    for col in 0..53 {
        for row in 0..7 {
            let s = col * 7 + row;
            let r = Rect::from_min_size(pos2(ox + col as f32 * (cell + gap), area.min.y + row as f32 * (cell + gap)), vec2(cell, cell));
            let day = s as isize - first_wd as isize;
            if day < 0 || day as usize >= counts.len() {
                continue;
            }
            let delay = (col + row) as f32 * 0.012;
            let k = ease_out(((since - delay) / 0.32).clamp(0.0, 1.0));
            let l = anim.prev[s] + (anim.cur[s] - anim.prev[s]) * k;
            // entry: diagonal fade-in
            let e = ((el - delay * 0.8) / 0.4).clamp(0.0, 1.0);
            if e < 1.0 {
                busy = true;
            }
            let hov = hover.is_some_and(|p| r.expand(gap / 2.0).contains(p));
            if hov {
                hovered = Some(day as usize);
            }
            let rr = if hov { r.expand(1.0) } else { r };
            painter.rect_filled(rr, cr(rad), fade(level_color(&t, accent, l), ease_out(e)));
        }
    }
    if busy {
        ui.ctx().request_repaint();
    }
    // month labels
    for (m, start) in MONTH_START.iter().enumerate() {
        let col = (start + first_wd) / 7;
        let x = ox + col as f32 * (cell + gap) + if m == 0 { 0.0 } else { cell * 0.2 };
        text(painter, pos2(x.max(ox), area.min.y + 7.0 * (cell + gap) + 9.0), Align2::LEFT_CENTER, MONTHS[m], f(12.0), faint(&t));
    }
    // tooltip
    if let Some(d) = hovered {
        let col = (d + first_wd) / 7;
        let row = (d + first_wd) % 7;
        let c = pos2(ox + col as f32 * (cell + gap) + cell / 2.0, area.min.y + row as f32 * (cell + gap));
        let s = format!("{} contributions · {}", counts[d], date_of(d));
        let w = text_w(painter, &s, f(12.0)) + 18.0;
        let x = (c.x - w / 2.0).clamp(area.min.x - 8.0, area.max.x + 8.0 - w);
        let tip = Rect::from_min_size(pos2(x, c.y - 34.0), vec2(w, 26.0));
        let bg = if t.dark { mix(t.muted, t.foreground, 0.12) } else { Color32::from_rgb(24, 24, 27) };
        glowkit_core::paint::soft_shadow(painter, tip, 7.0, 10.0, vec2(0.0, 3.0), Color32::from_black_alpha(40));
        painter.rect_filled(tip, cr(7.0), bg);
        text(painter, tip.center(), Align2::CENTER_CENTER, s, f(12.0), Color32::WHITE);
    }
    hovered
}

/// Height of a heat grid for a given width.
pub(crate) fn heat_height(width: f32) -> f32 {
    let gap = ((width / 53.0) * 0.26).clamp(2.0, 4.5);
    let cell = (width + gap) / 53.0 - gap;
    7.0 * (cell + gap) + 22.0
}

/// A contributions card: a yearly headline with a delta chip, a row of stat tiles and a
/// 53-week heat grid of day cells with intensity steps and month labels. Weekly / Monthly /
/// Yearly tabs re-aggregate the grid (days, monthly averages, weekly totals) with a diagonal
/// colour ripple; hovering a cell shows its date and count. The accent colour family is
/// swappable.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::BoardContributions;
/// let counts: Vec<u32> = (0..365).map(|d| (d * 7 % 11) as u32).collect();
/// ui.add(BoardContributions::new("Contributions this year").counts(counts, 3));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoardContributions {
    title: String,
    counts: Vec<u32>,
    first_wd: usize,
    stats: Vec<BoardStat>,
    delta: String,
    accent: Option<Color32>,
    width: f32,
    autoplay: bool,
}

impl BoardContributions {
    /// A contributions card titled `title` with a sample year.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            counts: sample_counts(11),
            first_wd: 3,
            stats: vec![
                BoardStat::new("9B", "Lifetime tokens"),
                BoardStat::new("562.7M", "Peak tokens"),
                BoardStat::new("12h 54m", "Longest task"),
                BoardStat::new("62 days", "Top streak"),
            ],
            delta: "+14.8%".into(),
            accent: None,
            width: 680.0,
            autoplay: false,
        }
    }
    /// Daily counts for the year (up to 365) and the weekday of Jan 1 (0 = Sunday row on top).
    pub fn counts(mut self, c: Vec<u32>, first_weekday: usize) -> Self {
        self.counts = c;
        self.first_wd = first_weekday % 7;
        self
    }
    /// Stat tiles under the headline (empty hides the row).
    pub fn stats(mut self, s: Vec<BoardStat>) -> Self {
        self.stats = s;
        self
    }
    /// Delta chip text (`""` hides it).
    pub fn delta(mut self, d: impl Into<String>) -> Self {
        self.delta = d.into();
        self
    }
    /// Accent colour the intensity steps are mixed toward (default purple).
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = Some(c);
        self
    }
    /// Card width in points (default 680).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(360.0);
        self
    }
    /// Switch tabs and hover cells by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

/// Body shared with the profile page. `rect` = full card. Returns the used height.
#[allow(clippy::too_many_arguments)]
pub(crate) fn contributions_body(
    ui: &Ui,
    painter: &Painter,
    id: Id,
    rect: Rect,
    title: &str,
    headline: &str,
    counts: &[u32],
    first_wd: usize,
    stats: &[BoardStat],
    delta: &str,
    accent: Color32,
    autoplay: bool,
    hovered: bool,
    inverted: bool,
) {
    let t = Theme::of(ui);
    let el = entry(ui, id);
    let pad = 16.0;
    let total: u32 = counts.iter().sum();
    let shown = tween(ui, id.with("head"), total as f32, 0.0, 1.1);
    text(painter, pos2(rect.min.x + pad, rect.min.y + 24.0), Align2::LEFT_CENTER, title, f(13.0), dim(&t));
    let head = if headline.is_empty() { thousands(shown as f64) } else { headline.to_string() };
    let hr = text(painter, pos2(rect.min.x + pad, rect.min.y + 52.0), Align2::LEFT_CENTER, head, f(26.0), t.foreground);
    if !delta.is_empty() {
        let c = chip(painter, &t, pos2(hr.max.x + 8.0, hr.center().y + 1.0), delta, !delta.starts_with('-'));
        let _ = c;
    }
    let mut y = rect.min.y + 80.0;
    if !stats.is_empty() {
        let n = stats.len();
        let gap = 8.0;
        let tw = (rect.width() - 16.0 - gap * (n as f32 - 1.0)) / n as f32;
        for (i, s) in stats.iter().enumerate() {
            let k = ease_out(entry_k(ui, el, 0.06 * i as f32, 0.45));
            let r = Rect::from_min_size(pos2(rect.min.x + 8.0 + i as f32 * (tw + gap), y + (1.0 - k) * 8.0), vec2(tw, 58.0));
            let mut tp = painter.clone();
            tp.multiply_opacity(k.max(0.001));
            if inverted {
                tp.rect_filled(r, cr(10.0), frame_bg(&t));
                text(&tp, pos2(r.min.x + 10.0, r.min.y + 19.0), Align2::LEFT_CENTER, &s.value, f(13.5), t.foreground);
                text(&tp, pos2(r.min.x + 10.0, r.min.y + 39.0), Align2::LEFT_CENTER, &s.label, f(12.5), dim(&t));
            } else {
                stat_tile(&tp, &t, r, &s.label, &s.value, None, 0.0, true);
            }
        }
        y += 58.0 + 14.0;
    }
    // tabs
    text(painter, pos2(rect.min.x + pad, y + 12.0), Align2::LEFT_CENTER, "Activity", f(13.0), dim(&t));
    let labels = ["Weekly", "Monthly", "Yearly"];
    let rects = tabs_layout(painter, pos2(rect.max.x - pad + 8.0, y + 12.0), &labels, 13.0);
    let stored: usize = glowkit_core::state(ui, id.with("tab"), || 0);
    let (sel, _knob) = tabs_logic(ui, id, &rects, stored);
    let over = rects.iter().any(|r| ui.rect_contains_pointer(*r));
    let idle = idle_for(ui, id.with("auto"), hovered || over);
    let mut mode = sel;
    if autoplay && idle > 2.5 && el > 1.5 {
        mode = (sel + ((idle - 2.5) / 2.6) as usize) % 3;
        glowkit_core::animate(ui);
    } else {
        glowkit_core::set_state(ui, id.with("tab"), sel);
    }
    for (i, r) in rects.iter().enumerate() {
        let a = flag(ui, id.with(("tabc", i)), i == mode, 12.0);
        text(painter, r.center(), Align2::CENTER_CENTER, labels[i], f(13.0), mix(faint(&t), t.foreground, a));
    }
    y += 30.0;
    let area = Rect::from_min_max(pos2(rect.min.x + pad, y), pos2(rect.max.x - pad, rect.max.y - 10.0));
    let mut hover = ui.ctx().pointer_hover_pos().filter(|p| area.contains(*p) && hovered);
    if hover.is_none() && autoplay && idle > 1.2 && el > 1.0 {
        // a ghost pointer drifting along the grid
        let ph = (idle * 0.11).fract();
        let row = 1.5 + 2.5 * (1.0 + (idle * 0.9).sin());
        hover = Some(pos2(area.min.x + area.width() * (0.08 + 0.84 * ph), area.min.y + row * (area.height() - 22.0) / 7.0));
    }
    heat_grid(ui, painter, id, area, counts, first_wd, mode, accent, el, hover);
}

/// Height of [`contributions_body`] for a width.
pub(crate) fn contributions_height(width: f32, stats: bool) -> f32 {
    80.0 + if stats { 72.0 } else { 0.0 } + 30.0 + heat_height(width - 32.0) + 10.0
}

impl egui::Widget for BoardContributions {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let h = contributions_height(self.width, !self.stats.is_empty());
        let (rect, response) = ui.allocate_exact_size(vec2(self.width, h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(40.0));
        frame(&painter, &t, rect, 16.0);
        let accent = self.accent.unwrap_or(pal(&t)[PURPLE]);
        contributions_body(
            ui,
            &painter,
            response.id,
            rect,
            &self.title,
            "",
            &self.counts,
            self.first_wd,
            &self.stats,
            &self.delta,
            accent,
            self.autoplay,
            response.hovered(),
            false,
        );
        response
    }
}
