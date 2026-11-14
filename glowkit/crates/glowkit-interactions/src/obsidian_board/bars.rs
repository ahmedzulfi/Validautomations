//! Bar-chart cards: pill bars (earnings, steps), grouped year-over-year bars (orders) and a
//! funnel drawn as stage bars.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Id, Painter, Rect, Response, Sense, Stroke, Ui};
use glowkit_core::color::{fade, mix, shade};
use glowkit_core::Theme;

/// A "nice" axis maximum and tick step for `max` with about `n` ticks.
fn nice_axis(max: f32, n: usize) -> (f32, f32) {
    let raw = (max.max(1e-3) / n as f32).max(1e-6);
    let mag = 10f32.powf(raw.log10().floor());
    let norm = raw / mag;
    let step = if norm <= 1.0 {
        1.0
    } else if norm <= 2.0 {
        2.0
    } else if norm <= 2.5 {
        2.5
    } else if norm <= 5.0 {
        5.0
    } else {
        10.0
    } * mag;
    ((max / step).ceil().max(1.0) * step, step)
}

/// Per-bar staggered targets: when the dataset key changes, bar `i` switches to its new value
/// `i * delay` seconds later. Returns the animated value of every bar.
fn stagger_values(ui: &Ui, id: Id, key: usize, values: &[f32], el: f32, delay: f32) -> Vec<f32> {
    let now = glowkit_core::time(ui);
    #[derive(Clone)]
    struct Sw {
        key: usize,
        at: f32,
        prev: Vec<f32>,
        cur: Vec<f32>,
    }
    let vals = values.to_vec();
    let sw = glowkit_core::with_state(
        ui,
        id.with("stagger"),
        || Sw { key, at: now, prev: vec![0.0; vals.len()], cur: vals.clone() },
        |s| {
            if s.key != key {
                s.prev = s.cur.clone();
                s.cur = vals.clone();
                s.key = key;
                s.at = now;
            } else {
                s.cur = vals.clone();
            }
            s.clone()
        },
    );
    let since = (now - sw.at).min(el);
    let mut out = Vec::with_capacity(values.len());
    for (i, v) in values.iter().enumerate() {
        let started = since > i as f32 * delay;
        let target = if started { *v } else { sw.prev.get(i).copied().unwrap_or(0.0) };
        out.push(glowkit_core::anim::spring(ui, id.with(("bar", i)), target, 170.0, 19.0).max(0.0));
    }
    if since < values.len() as f32 * delay + 0.1 {
        ui.ctx().request_repaint();
    }
    out
}

/// Paint pill bars (grey track + coloured fill) in `area`; returns the hovered bar.
#[allow(clippy::too_many_arguments)]
fn pill_bars(
    painter: &Painter,
    t: &Theme,
    area: Rect,
    shown: &[f32],
    labels: &[String],
    axis_max: f32,
    color: Color32,
    hover: Option<egui::Pos2>,
    forced: Option<usize>,
    hl: &dyn Fn(usize, bool) -> f32,
) -> Option<usize> {
    let n = shown.len().max(1);
    let slot = area.width() / n as f32;
    let bw = (slot * 0.64).min(44.0);
    let label_h = 22.0;
    let plot = Rect::from_min_max(area.min, pos2(area.max.x, area.max.y - label_h));
    let mut hovered = None;
    for i in 0..shown.len() {
        let cx = area.min.x + slot * (i as f32 + 0.5);
        let col_r = Rect::from_min_max(pos2(cx - slot / 2.0, area.min.y), pos2(cx + slot / 2.0, area.max.y));
        if hover.is_some_and(|p| col_r.contains(p)) {
            hovered = Some(i);
        }
    }
    let active = hovered.or(forced);
    for (i, v) in shown.iter().enumerate() {
        let cx = area.min.x + slot * (i as f32 + 0.5);
        let tr = Rect::from_min_max(pos2(cx - bw / 2.0, plot.min.y), pos2(cx + bw / 2.0, plot.max.y));
        let r = (bw * 0.28).min(9.0);
        painter.rect_filled(tr, cr(r), track(t));
        let h = (v / axis_max.max(1e-6)).clamp(0.0, 1.0) * tr.height();
        if h > 0.5 {
            let fr = Rect::from_min_max(pos2(tr.min.x, tr.max.y - h.max(r * 2.0).min(tr.height())), tr.max);
            let k = hl(i, active == Some(i));
            painter.rect_filled(fr, cr(r), mix(color, shade(color, -0.08), k));
        }
        let o = hl(i, active == Some(i));
        if o > 0.01 {
            painter.rect_stroke(tr.expand(3.0), cr(r + 3.0), Stroke::new(1.5, fade(mix(t.border, t.foreground, 0.55), o)), egui::StrokeKind::Inside);
        }
        if let Some(l) = labels.get(i) {
            let c = if active == Some(i) { t.foreground } else { faint(t) };
            text(painter, pos2(cx, area.max.y - 8.0), Align2::CENTER_CENTER, l, f(12.0), c);
        }
    }
    hovered
}

/// Autoplay ghost hover index: walks the bars while idle.
fn ghost(ui: &Ui, id: Id, on: bool, interacting: bool, n: usize, period: f32) -> Option<usize> {
    let idle = idle_for(ui, id, interacting);
    if !on || idle < 2.0 || n == 0 {
        return None;
    }
    glowkit_core::animate(ui);
    let k = ((idle - 2.0) / period) as usize % (n + 3);
    (k < n).then_some(k)
}

// =============================================================================================
// Earnings

/// One dataset of [`BoardEarnings`]: a tab label, bar labels, values and the delta chip text.
#[derive(Clone, Debug, PartialEq)]
pub struct BoardSeries {
    /// Tab label ("Monthly").
    pub tab: String,
    /// Bar labels ("Jan", ...).
    pub labels: Vec<String>,
    /// Bar values.
    pub values: Vec<f32>,
    /// Delta chip text next to the headline (`""` hides it).
    pub delta: String,
}

impl BoardSeries {
    /// A dataset shown under tab `tab`.
    pub fn new(tab: impl Into<String>, labels: &[&str], values: Vec<f32>) -> Self {
        Self { tab: tab.into(), labels: labels.iter().map(|s| s.to_string()).collect(), values, delta: String::new() }
    }
    /// Delta chip text, e.g. `"+14.8%"` (a leading `-` turns it rose).
    pub fn delta(mut self, d: impl Into<String>) -> Self {
        self.delta = d.into();
        self
    }
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// An "earned so far" card: rounded pill bars (a grey track with a coloured fill) over a money
/// axis, a Weekly / Monthly / Yearly switch that regrows every bar with a stagger, a count-up
/// dollar headline with a delta chip, and an outline ring on the hovered bar while the
/// headline follows it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BoardEarnings, BoardSeries};
/// ui.add(BoardEarnings::new("Earned so far").series(vec![
///     BoardSeries::new("Monthly", &["Jan", "Feb", "Mar"], vec![2400.0, 6100.0, 8300.0]).delta("+14.8%"),
/// ]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoardEarnings {
    title: String,
    series: Vec<BoardSeries>,
    prefix: String,
    color: Option<Color32>,
    size: egui::Vec2,
    autoplay: bool,
}

impl BoardEarnings {
    /// An earnings card titled `title` with sample weekly, monthly and yearly data.
    pub fn new(title: impl Into<String>) -> Self {
        let weeks: Vec<String> = (27..39).map(|w| format!("W{w}")).collect();
        let wl: Vec<&str> = weeks.iter().map(|s| s.as_str()).collect();
        Self {
            title: title.into(),
            series: vec![
                BoardSeries::new("Weekly", &wl, vec![420.0, 610.0, 380.0, 720.0, 690.0, 540.0, 810.0, 760.0, 590.0, 880.0, 940.0, 712.0]).delta("+6.2%"),
                BoardSeries::new(
                    "Monthly",
                    &MONTHS,
                    vec![2900.0, 7200.0, 9400.0, 7000.0, 3700.0, 2200.0, 3900.0, 5000.0, 6800.0, 4400.0, 11400.0, 8200.0],
                )
                .delta("+14.8%"),
                BoardSeries::new("Yearly", &["2021", "2022", "2023", "2024", "2025", "2026"], vec![31000.0, 42000.0, 39000.0, 56000.0, 64000.0, 72100.0]).delta("+12.6%"),
            ],
            prefix: "$".into(),
            color: None,
            size: vec2(560.0, 330.0),
            autoplay: false,
        }
    }
    /// Datasets, one per tab (replaces the samples).
    pub fn series(mut self, s: Vec<BoardSeries>) -> Self {
        if !s.is_empty() {
            self.series = s;
        }
        self
    }
    /// Currency / unit prefix for the headline and axis (default `"$"`).
    pub fn prefix(mut self, p: impl Into<String>) -> Self {
        self.prefix = p.into();
        self
    }
    /// Bar colour (default lime).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Card size in points (default 560 x 330).
    pub fn size(mut self, s: egui::Vec2) -> Self {
        self.size = vec2(s.x.max(320.0), s.y.max(240.0));
        self
    }
    /// Sweep the hover and switch tabs by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for BoardEarnings {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(self.size, Sense::hover());
        let id = response.id;
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(16.0));
        let el = entry(ui, id);
        let pad = 16.0;
        frame(&painter, &t, rect, 16.0);
        let labels: Vec<&str> = self.series.iter().map(|s| s.tab.as_str()).collect();
        let rects = tabs_layout(&painter, pos2(rect.max.x - pad + 4.0, rect.min.y + 30.0), &labels, 13.0);
        let stored: usize = glowkit_core::state(ui, id.with("tab"), || self.series.len().saturating_sub(2).min(1));
        let (mut sel, knob) = tabs_logic(ui, id, &rects, stored);
        let over_tabs = rects.iter().any(|r| ui.rect_contains_pointer(*r));
        let idle = idle_for(ui, id.with("tabs"), response.hovered() || over_tabs);
        if self.autoplay && idle > 2.0 && el > 1.0 {
            let n = self.series[sel].values.len();
            let per = 0.55 * (n + 3) as f32 + 1.0;
            let k = ((idle - 2.0) / per) as usize;
            sel = (stored + k) % self.series.len();
        }
        if !(self.autoplay && idle > 2.0) {
            glowkit_core::set_state(ui, id.with("tab"), sel);
        }
        tabs_paint(&painter, &t, &rects, &labels, sel, knob, 13.0);
        let s = &self.series[sel.min(self.series.len() - 1)];
        let color = self.color.unwrap_or(pal(&t)[LIME]);

        // chart
        let axis_w = 46.0;
        let area = Rect::from_min_max(pos2(rect.min.x + pad + axis_w, rect.min.y + 88.0), pos2(rect.max.x - pad, rect.max.y - 12.0));
        let max = s.values.iter().copied().fold(0.0, f32::max);
        let (axis_max, step) = nice_axis(max, 4);
        let axis_max_s = glowkit_core::anim::spring(ui, id.with("axis"), axis_max, 160.0, 22.0);
        let plot_h = area.height() - 22.0;
        let mut v = 0.0;
        while v <= axis_max + 1e-3 {
            let y = area.min.y + plot_h * (1.0 - v / axis_max_s);
            if y >= area.min.y - 4.0 {
                text(&painter, pos2(area.min.x - 12.0, y), Align2::RIGHT_CENTER, format!("{}{}", self.prefix, compact(v, true)), f(11.5), faint(&t));
            }
            v += step;
        }
        let shown = stagger_values(ui, id, sel, &s.values, el, 0.045);
        let interacting = response.hovered();
        let forced = ghost(ui, id.with("ghost"), self.autoplay && !over_tabs, interacting, s.values.len(), 0.55);
        let ptr = response.hover_pos();
        let hovered_now = {
            // hit-test first so highlight springs know the target
            let n = s.values.len().max(1);
            let slot = area.width() / n as f32;
            ptr.filter(|p| area.contains(*p)).map(|p| (((p.x - area.min.x) / slot) as usize).min(n - 1))
        };
        let active = hovered_now.or(forced);
        let hls: Vec<f32> = (0..s.values.len()).map(|i| flag(ui, id.with(("hl", i)), active == Some(i), 14.0)).collect();
        pill_bars(&painter, &t, area, &shown, &s.labels, axis_max_s, color, ptr, forced, &|i, _| hls[i]);

        // headline
        let (title, value) = match active {
            Some(i) => (format!("{} · {}", self.title, s.labels.get(i).cloned().unwrap_or_default()), s.values[i]),
            None => (self.title.clone(), s.values.iter().sum::<f32>()),
        };
        let shown_v = tween(ui, id.with("head"), value, 0.0, 0.9);
        text(&painter, pos2(rect.min.x + pad, rect.min.y + 26.0), Align2::LEFT_CENTER, &title, f(13.0), dim(&t));
        let hr = text(&painter, pos2(rect.min.x + pad, rect.min.y + 54.0), Align2::LEFT_CENTER, format!("{}{}", self.prefix, thousands(shown_v as f64)), f(26.0), t.foreground);
        if !s.delta.is_empty() && active.is_none() {
            chip(&painter, &t, pos2(hr.max.x + 8.0, hr.center().y + 1.0), &s.delta, !s.delta.starts_with('-'));
        }
        response
    }
}

// =============================================================================================
// Steps

/// A weekly steps card: seven rounded pill bars (grey track, teal fill) under a count-up step
/// total, with a date-range pill whose arrows page through weeks (bars regrow with a stagger)
/// and an outline ring on the hovered day while the headline follows it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::BoardSteps;
/// ui.add(BoardSteps::new().weeks(vec![
///     ("29 Jun - 5 Jul".into(), [6200.0, 2700.0, 2400.0, 7900.0, 8800.0, 6500.0, 4100.0]),
/// ]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoardSteps {
    title: String,
    weeks: Vec<(String, [f32; 7])>,
    goal: Option<f32>,
    color: Option<Color32>,
    size: egui::Vec2,
    autoplay: bool,
}

impl Default for BoardSteps {
    fn default() -> Self {
        Self::new()
    }
}

impl BoardSteps {
    /// A steps card with three sample weeks.
    pub fn new() -> Self {
        Self {
            title: "Steps".into(),
            weeks: vec![
                ("15 - 21 Jun".into(), [5400.0, 7100.0, 6800.0, 3900.0, 9100.0, 10400.0, 5600.0]),
                ("22 - 28 Jun".into(), [8200.0, 4300.0, 6100.0, 7400.0, 3800.0, 8900.0, 9700.0]),
                ("29 Jun - 5 Jul".into(), [6200.0, 2700.0, 2400.0, 7900.0, 8800.0, 6500.0, 4100.0]),
            ],
            goal: Some(10_000.0),
            color: None,
            size: vec2(540.0, 320.0),
            autoplay: false,
        }
    }
    /// Card title (default "Steps").
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Weeks to page through: range label and Monday..Sunday steps (the last is shown first).
    pub fn weeks(mut self, w: Vec<(String, [f32; 7])>) -> Self {
        if !w.is_empty() {
            self.weeks = w;
        }
        self
    }
    /// The value a full track represents (default 10,000; `None` = the week's maximum).
    pub fn goal(mut self, g: Option<f32>) -> Self {
        self.goal = g;
        self
    }
    /// Bar colour (default teal).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Card size in points (default 540 x 320).
    pub fn size(mut self, s: egui::Vec2) -> Self {
        self.size = vec2(s.x.max(300.0), s.y.max(220.0));
        self
    }
    /// Sweep the hover and page weeks by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

impl egui::Widget for BoardSteps {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(self.size, Sense::hover());
        let id = response.id;
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(16.0));
        let el = entry(ui, id);
        let pad = 16.0;
        frame(&painter, &t, rect, 16.0);
        let last = self.weeks.len() - 1;
        let mut week: usize = glowkit_core::state(ui, id.with("wk"), || last);
        let pr = Rect::from_min_size(pos2(rect.max.x - pad - 152.0, rect.min.y + 14.0), vec2(152.0, 30.0));
        let lr = Rect::from_min_size(pr.min, vec2(26.0, 30.0));
        let rr = Rect::from_min_size(pos2(pr.max.x - 26.0, pr.min.y), vec2(26.0, 30.0));
        let l = ui.interact(lr, id.with("l"), Sense::click());
        let r = ui.interact(rr, id.with("r"), Sense::click());
        if l.clicked() && week > 0 {
            week -= 1;
        }
        if r.clicked() && week < last {
            week += 1;
        }
        week = week.min(last);
        glowkit_core::set_state(ui, id.with("wk"), week);
        let idle = idle_for(ui, id.with("nav"), response.hovered() || l.hovered() || r.hovered());
        let mut shown_week = week;
        if self.autoplay && idle > 2.0 {
            let k = ((idle - 2.0) / (0.5 * 10.0 + 1.0)) as usize;
            shown_week = (week + k) % self.weeks.len();
        }
        let (label, vals) = &self.weeks[shown_week];
        let hl = [flag(ui, id.with("lh"), l.hovered(), 14.0), flag(ui, id.with("rh"), r.hovered(), 14.0)];
        range_pill(&painter, &t, pr, label, hl);
        let color = self.color.unwrap_or(pal(&t)[TEAL]);
        let area = Rect::from_min_max(pos2(rect.min.x + pad, rect.min.y + 84.0), pos2(rect.max.x - pad, rect.max.y - 10.0));
        let axis_max = self.goal.unwrap_or_else(|| vals.iter().copied().fold(1.0, f32::max)).max(1.0);
        let shown = stagger_values(ui, id, shown_week, vals, el, 0.05);
        let forced = ghost(ui, id.with("ghost"), self.autoplay, response.hovered(), 7, 0.5);
        let ptr = response.hover_pos();
        let hov = ptr.filter(|p| area.contains(*p)).map(|p| (((p.x - area.min.x) / (area.width() / 7.0)) as usize).min(6));
        let active = hov.or(forced);
        let hls: Vec<f32> = (0..7).map(|i| flag(ui, id.with(("hl", i)), active == Some(i), 14.0)).collect();
        let labels: Vec<String> = DAYS.iter().map(|s| s.to_string()).collect();
        pill_bars(&painter, &t, area, &shown, &labels, axis_max, color, ptr, forced, &|i, _| hls[i]);
        let (title, value, unit) = match active {
            Some(i) => (format!("{} · {}", self.title, DAYS[i]), vals[i], "steps"),
            None => (self.title.clone(), vals.iter().sum(), "total steps"),
        };
        let v = tween(ui, id.with("head"), value, 0.0, 0.9);
        text(&painter, pos2(rect.min.x + pad, rect.min.y + 26.0), Align2::LEFT_CENTER, &title, f(13.0), dim(&t));
        let hr = text(&painter, pos2(rect.min.x + pad, rect.min.y + 54.0), Align2::LEFT_CENTER, thousands(v as f64), f(26.0), t.foreground);
        text(&painter, pos2(hr.max.x + 6.0, hr.max.y - 9.0), Align2::LEFT_CENTER, unit, f(12.0), dim(&t));
        for q in [&l, &r] {
            if q.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
        response
    }
}

// =============================================================================================
// Orders

/// A year-over-year orders card: for every month a thin bar for this year beside a neutral bar
/// for last year, a value axis, a legend, and a headline that counts up to the yearly total with
/// a delta chip. Hovering a month bands its column and swaps the headline to that month's
/// figure and what it was a year earlier.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::BoardOrders;
/// let this = [1700.0, 1800.0, 2000.0, 1900.0, 2100.0, 2200.0, 2400.0, 2300.0, 2400.0, 2500.0, 2400.0, 2450.0];
/// let last = [1500.0, 1500.0, 1700.0, 1650.0, 1750.0, 1800.0, 1850.0, 1900.0, 1950.0, 2000.0, 1850.0, 1850.0];
/// ui.add(BoardOrders::new("Orders").data(this, last));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoardOrders {
    title: String,
    this: [f32; 12],
    last: [f32; 12],
    colors: Option<(Color32, Color32)>,
    size: egui::Vec2,
    autoplay: bool,
}

impl BoardOrders {
    /// An orders card titled `title` with sample data.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            this: [1720.0, 1790.0, 1960.0, 1880.0, 2100.0, 2210.0, 2400.0, 2290.0, 2420.0, 2540.0, 2410.0, 2452.0],
            last: [1540.0, 1510.0, 1690.0, 1640.0, 1750.0, 1810.0, 1870.0, 1930.0, 1960.0, 2020.0, 1850.0, 1838.0],
            colors: None,
            size: vec2(600.0, 340.0),
            autoplay: false,
        }
    }
    /// Monthly values for this year and last year.
    pub fn data(mut self, this: [f32; 12], last: [f32; 12]) -> Self {
        self.this = this;
        self.last = last;
        self
    }
    /// Colours of this year's and last year's bars.
    pub fn colors(mut self, this: Color32, last: Color32) -> Self {
        self.colors = Some((this, last));
        self
    }
    /// Card size in points (default 600 x 340).
    pub fn size(mut self, s: egui::Vec2) -> Self {
        self.size = vec2(s.x.max(360.0), s.y.max(240.0));
        self
    }
    /// Sweep the month hover by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn pct(a: f32, b: f32) -> String {
    let d = (a / b.max(1e-3) - 1.0) * 100.0;
    format!("{}{:.0}%", if d >= 0.0 { "+" } else { "-" }, d.abs())
}

impl egui::Widget for BoardOrders {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(self.size, Sense::hover());
        let id = response.id;
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(16.0));
        let el = entry(ui, id);
        let pad = 16.0;
        frame(&painter, &t, rect, 16.0);
        let (c_this, c_last) = self.colors.unwrap_or((pal(&t)[INDIGO], neutral(&t)));

        // legend
        let lx = rect.max.x - pad;
        let r2 = text(&painter, pos2(lx, rect.min.y + 26.0), Align2::RIGHT_CENTER, "Last year", f(12.5), dim(&t));
        painter.circle_filled(pos2(r2.min.x - 9.0, r2.center().y), 3.5, c_last);
        let r1 = text(&painter, pos2(r2.min.x - 24.0, rect.min.y + 26.0), Align2::RIGHT_CENTER, "This year", f(12.5), dim(&t));
        painter.circle_filled(pos2(r1.min.x - 9.0, r1.center().y), 3.5, c_this);

        // plot
        let axis_w = 40.0;
        let area = Rect::from_min_max(pos2(rect.min.x + pad + axis_w, rect.min.y + 112.0), pos2(rect.max.x - pad, rect.max.y - 12.0));
        let plot = Rect::from_min_max(area.min, pos2(area.max.x, area.max.y - 24.0));
        let max = self.this.iter().chain(self.last.iter()).copied().fold(0.0, f32::max);
        let (axis_max, step) = nice_axis(max, 3);
        let mut v = 0.0;
        while v <= axis_max + 1e-3 {
            let y = plot.max.y - plot.height() * v / axis_max;
            text(&painter, pos2(plot.min.x - 14.0, y), Align2::RIGHT_CENTER, compact(v, false), f(11.5), faint(&t));
            painter.line_segment([pos2(plot.min.x, y), pos2(plot.max.x, y)], Stroke::new(1.0, fade(hairline(&t), if v == 0.0 { 1.0 } else { 0.45 })));
            v += step;
        }
        let slot = plot.width() / 12.0;
        let ptr = response.hover_pos();
        let hov = ptr.filter(|p| area.contains(*p)).map(|p| (((p.x - area.min.x) / slot) as usize).min(11));
        let forced = ghost(ui, id.with("ghost"), self.autoplay, response.hovered(), 12, 0.6);
        let active = hov.or(forced);
        let bw = (slot * 0.2).clamp(4.0, 12.0);
        for i in 0..12 {
            let cx = plot.min.x + slot * (i as f32 + 0.5);
            let hf = flag(ui, id.with(("hl", i)), active == Some(i), 14.0);
            if hf > 0.01 {
                let band = Rect::from_min_max(pos2(cx - slot * 0.46, plot.min.y - 6.0), pos2(cx + slot * 0.46, plot.max.y + 4.0));
                painter.rect_filled(band, cr(8.0), fade(tile_bg(&t), hf * if t.dark { 0.7 } else { 1.0 }));
            }
            let dimk = 1.0 - 0.45 * flag(ui, id.with(("dm", i)), active.is_some() && active != Some(i), 12.0);
            for (k, (vals, col)) in [(&self.last, c_last), (&self.this, c_this)].into_iter().enumerate() {
                let g = ease_out(entry_k(ui, el, 0.04 * i as f32 + 0.08 * k as f32, 0.7));
                let val = tween(ui, id.with(("v", i, k)), vals[i], 0.0, 0.7) * g;
                let h = plot.height() * val / axis_max;
                let x = cx + if k == 0 { -bw - 1.5 } else { 1.5 };
                let r = Rect::from_min_max(pos2(x, plot.max.y - h), pos2(x + bw, plot.max.y));
                painter.rect_filled(r, egui::CornerRadius { nw: 3, ne: 3, sw: 0, se: 0 }, fade(col, dimk));
            }
            let lc = if active == Some(i) { t.foreground } else { faint(&t) };
            text(&painter, pos2(cx, area.max.y - 9.0), Align2::CENTER_CENTER, MONTHS[i], f(12.0), lc);
        }

        // headline
        let tot_this: f32 = self.this.iter().sum();
        let tot_last: f32 = self.last.iter().sum();
        let (title, a, b) = match active {
            Some(i) => (format!("{} · {}", self.title, MONTHS[i]), self.this[i], self.last[i]),
            None => (self.title.clone(), tot_this, tot_last),
        };
        let va = tween(ui, id.with("ha"), a, 0.0, 0.9);
        let vb = tween(ui, id.with("hb"), b, 0.0, 0.9);
        text(&painter, pos2(rect.min.x + pad, rect.min.y + 26.0), Align2::LEFT_CENTER, &title, f(13.0), dim(&t));
        let hr = text(&painter, pos2(rect.min.x + pad, rect.min.y + 56.0), Align2::LEFT_CENTER, thousands(va as f64), f(26.0), t.foreground);
        chip(&painter, &t, pos2(hr.max.x + 8.0, hr.center().y + 1.0), &pct(a, b), a >= b);
        text(&painter, pos2(rect.min.x + pad, rect.min.y + 84.0), Align2::LEFT_CENTER, format!("{} last year", thousands(vb as f64)), f(12.5), faint(&t));
        response
    }
}

// =============================================================================================
// Stage bars

/// One stage of a [`BoardStageBars`] funnel.
#[derive(Clone, Debug, PartialEq)]
pub struct BoardStage {
    /// Stage name ("Sign-up").
    pub label: String,
    /// Count at this stage.
    pub value: f32,
    /// Colour (`None` = BoardUI accent for its position).
    pub color: Option<Color32>,
}

impl BoardStage {
    /// A stage `label` with `value`.
    pub fn new(label: impl Into<String>, value: f32) -> Self {
        Self { label: label.into(), value: value.max(0.0), color: None }
    }
    /// Set the stage colour.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

/// A funnel drawn as a list: one rounded pill per stage over a grey track (stage name on the
/// left, a small glyph at the start of the fill, value and share of the top stage on the
/// right), with a grid of stat tiles below. Widths grow in with a stagger; hovering a stage (or
/// its tile) deepens its pill and fades the rest while the headline follows.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BoardStage, BoardStageBars};
/// ui.add(BoardStageBars::new("Pipeline").stages(vec![
///     BoardStage::new("Visits", 1180.0),
///     BoardStage::new("Sign-up", 790.0),
///     BoardStage::new("Active", 460.0),
/// ]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoardStageBars {
    title: String,
    stages: Vec<BoardStage>,
    delta: String,
    period: String,
    width: f32,
    bar_h: f32,
    tiles: bool,
    autoplay: bool,
}

impl BoardStageBars {
    /// A funnel card titled `title` with sample stages.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            stages: vec![
                BoardStage::new("Visits", 1180.0),
                BoardStage::new("Sign-up", 790.0),
                BoardStage::new("Active", 460.0),
                BoardStage::new("Pro", 250.0),
                BoardStage::new("Team", 120.0),
                BoardStage::new("Enterprise", 40.0),
            ],
            delta: "+2.4%".into(),
            period: "Last 7 days".into(),
            width: 450.0,
            bar_h: 20.0,
            tiles: true,
            autoplay: false,
        }
    }
    /// The stages, top of the funnel first.
    pub fn stages(mut self, s: Vec<BoardStage>) -> Self {
        if !s.is_empty() {
            self.stages = s;
        }
        self
    }
    /// Delta chip text (`""` hides it).
    pub fn delta(mut self, d: impl Into<String>) -> Self {
        self.delta = d.into();
        self
    }
    /// Period pill label.
    pub fn period(mut self, p: impl Into<String>) -> Self {
        self.period = p.into();
        self
    }
    /// Card width in points (default 450).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(320.0);
        self
    }
    /// Pill height in points (default 20).
    pub fn bar_height(mut self, h: f32) -> Self {
        self.bar_h = h.clamp(10.0, 32.0);
        self
    }
    /// Show the tile grid under the bars (default on).
    pub fn tiles(mut self, on: bool) -> Self {
        self.tiles = on;
        self
    }
    /// Walk the stage focus by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const STAGE_GLYPHS: [Glyph; 6] = [Glyph::Eye, Glyph::UserPlus, Glyph::Bolt, Glyph::Crown, Glyph::Users, Glyph::Building];

impl egui::Widget for BoardStageBars {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let p = pal(&t);
        let n = self.stages.len();
        let row_h = self.bar_h + 12.0;
        let tile_rows = if self.tiles { n.div_ceil(3) } else { 0 };
        let h = 94.0 + n as f32 * row_h + 10.0 + tile_rows as f32 * 63.0 + 8.0;
        let (rect, response) = ui.allocate_exact_size(vec2(self.width, h), Sense::hover());
        let id = response.id;
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(16.0));
        let el = entry(ui, id);
        let pad = 16.0;
        frame(&painter, &t, rect, 16.0);
        let cols = [p[LIME], p[BLUE], p[PURPLE], p[PINK], p[YELLOW], p[EMERALD], p[TEAL], p[ORANGE]];
        let col = |i: usize| self.stages[i].color.unwrap_or(cols[i % cols.len()]);
        let top = self.stages.first().map(|s| s.value).unwrap_or(1.0).max(1e-3);

        // geometry
        let label_w = self.stages.iter().map(|s| text_w(&painter, &s.label, f(13.5))).fold(0.0, f32::max) + 14.0;
        let val_w = 92.0;
        let x0 = rect.min.x + pad + label_w;
        let x1 = rect.max.x - pad - val_w;
        let rows_top = rect.min.y + 94.0;
        let tiles_top = rows_top + n as f32 * row_h + 10.0;
        let tw = (rect.width() - 16.0 - 16.0) / 3.0;
        let tile_rect = |i: usize| Rect::from_min_size(pos2(rect.min.x + 8.0 + (i % 3) as f32 * (tw + 8.0), tiles_top + (i / 3) as f32 * 63.0), vec2(tw, 55.0));

        // focus
        let mut focus = None;
        if let Some(mp) = response.hover_pos() {
            for i in 0..n {
                let rr = Rect::from_min_max(pos2(rect.min.x, rows_top + i as f32 * row_h), pos2(rect.max.x, rows_top + (i + 1) as f32 * row_h));
                if rr.contains(mp) || (self.tiles && tile_rect(i).contains(mp)) {
                    focus = Some(i);
                }
            }
        }
        let forced = ghost(ui, id.with("ghost"), self.autoplay, response.hovered(), n, 1.1);
        let focus = focus.or(forced);

        for (i, s) in self.stages.iter().enumerate() {
            let y = rows_top + i as f32 * row_h + row_h / 2.0;
            let fo = flag(ui, id.with(("fo", i)), focus == Some(i), 12.0);
            let other = flag(ui, id.with(("ot", i)), focus.is_some() && focus != Some(i), 10.0);
            let a = 1.0 - 0.6 * other;
            text(&painter, pos2(x0 - 12.0, y), Align2::RIGHT_CENTER, &s.label, f(13.5), fade(mix(dim(&t), t.foreground, fo), a.max(0.5)));
            let tr = Rect::from_min_max(pos2(x0, y - self.bar_h / 2.0), pos2(x1, y + self.bar_h / 2.0));
            painter.rect_filled(tr, cr(self.bar_h / 2.0), track(&t));
            let g = ease_out(entry_k(ui, el, 0.06 * i as f32, 0.9));
            let share = tween(ui, id.with(("w", i)), s.value / top, 0.0, 0.8) * g;
            let fw = (tr.width() * share).max(self.bar_h);
            let fr = Rect::from_min_size(tr.min, vec2(fw, self.bar_h));
            let c = mix(col(i), shade(col(i), -0.1), fo);
            painter.rect_filled(fr, cr(self.bar_h / 2.0), fade(c, a));
            let gc = if t.dark { fade(egui::Color32::from_gray(20), a) } else { fade(egui::Color32::from_gray(25), a * 0.9) };
            glyph(&painter, STAGE_GLYPHS[i % STAGE_GLYPHS.len()], pos2(fr.min.x + self.bar_h / 2.0 + 1.0, y), self.bar_h * 0.55, gc);
            text(&painter, pos2(x1 + 52.0, y), Align2::RIGHT_CENTER, thousands(s.value as f64), f(13.5), fade(t.foreground, a));
            text(&painter, pos2(x1 + 56.0, y), Align2::LEFT_CENTER, format!("{:.0}%", s.value / top * 100.0), f(11.5), fade(faint(&t), a));
            if self.tiles {
                let ta = ease_out(entry_k(ui, el, 0.4 + 0.05 * i as f32, 0.45));
                let mut tp = painter.clone();
                tp.multiply_opacity(ta.max(0.001) * (1.0 - 0.4 * other));
                stat_tile(&tp, &t, tile_rect(i).translate(vec2(0.0, (1.0 - ta) * 6.0)), &s.label, &thousands(s.value as f64), Some(col(i)), fo, false);
            }
        }

        // header
        let (title, value) = match focus {
            Some(i) => (self.stages[i].label.clone(), self.stages[i].value),
            None => (self.title.clone(), top),
        };
        let v = tween(ui, id.with("head"), value, 0.0, 0.9);
        text(&painter, pos2(rect.min.x + pad, rect.min.y + 26.0), Align2::LEFT_CENTER, &title, f(13.0), dim(&t));
        let hr = text(&painter, pos2(rect.min.x + pad, rect.min.y + 54.0), Align2::LEFT_CENTER, thousands(v as f64), f(26.0), t.foreground);
        let chip_s = match focus {
            Some(i) if i > 0 => format!("{:.0}%", self.stages[i].value / top * 100.0),
            _ => self.delta.clone(),
        };
        if !chip_s.is_empty() {
            chip(&painter, &t, pos2(hr.max.x + 8.0, hr.center().y + 1.0), &chip_s, !chip_s.starts_with('-'));
        }
        let w = pill_w(&painter, &self.period, true);
        let pr = Rect::from_min_size(pos2(rect.max.x - pad - w, rect.min.y + 14.0), vec2(w, 30.0));
        let hov = ui.rect_contains_pointer(pr);
        pill(&painter, &t, pr, &self.period, true, flag(ui, id.with("pill"), hov, 12.0));
        response
    }
}
