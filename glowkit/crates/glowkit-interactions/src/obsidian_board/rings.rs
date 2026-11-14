//! Ring-based cards: activity rings, a segmented sleep-score gauge and a month calendar of
//! mini activity rings.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Id, Painter, Pos2, Rect, Response, Sense, Stroke, Ui};
use glowkit_core::color::{fade, mix};
use glowkit_core::Theme;
use std::f32::consts::{PI, TAU};

// =============================================================================================
// Activity rings

/// One ring of a [`BoardActivityRings`] card.
#[derive(Clone, Debug, PartialEq)]
pub struct BoardRing {
    /// Label shown in the stat tile ("Move").
    pub label: String,
    /// Formatted value shown under the label ("1,592 kcal").
    pub value: String,
    /// Goal progress; `1.0` closes the ring, values above 1 start a second lap.
    pub progress: f32,
    /// Ring colour (`None` = the BoardUI accent for its position).
    pub color: Option<Color32>,
}

impl BoardRing {
    /// A ring labelled `label` showing `value`, at `progress` of its goal.
    pub fn new(label: impl Into<String>, value: impl Into<String>, progress: f32) -> Self {
        Self { label: label.into(), value: value.into(), progress: progress.max(0.0), color: None }
    }
    /// Set the ring colour.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

fn ring_color(t: &Theme, r: &BoardRing, i: usize) -> Color32 {
    let p = pal(t);
    r.color.unwrap_or([p[PINK], p[LIME], p[BLUE], p[ORANGE], p[PURPLE]][i % 5])
}

/// Paint the activity panel body (title, tiles, rings) into `rect` and handle ring focus.
/// Returns the focused ring.
#[allow(clippy::too_many_arguments)]
fn activity_panel(
    ui: &Ui,
    painter: &Painter,
    id: Id,
    rect: Rect,
    title: &str,
    rings: &[BoardRing],
    width: f32,
    hover: Option<Pos2>,
    autoplay: bool,
    hovered: bool,
) -> Option<usize> {
    let t = Theme::of(ui);
    let el = entry(ui, id);
    frame(painter, &t, rect, 16.0);
    let pad = 12.0;
    text(painter, pos2(rect.min.x + pad + 4.0, rect.min.y + 24.0), Align2::LEFT_CENTER, title, f(13.5), dim(&t));
    let n = rings.len().max(1);
    let gap = 8.0;
    let tw = (rect.width() - pad * 2.0 - gap * (n as f32 - 1.0)) / n as f32;
    let tiles: Vec<Rect> =
        (0..n).map(|i| Rect::from_min_size(pos2(rect.min.x + pad + i as f32 * (tw + gap), rect.min.y + 46.0), vec2(tw, 56.0))).collect();
    let ring_top = rect.min.y + 46.0 + 56.0 + 14.0;
    let avail = (rect.max.y - pad - ring_top).max(40.0);
    let radius = (avail / 2.0).min(rect.width() * 0.28);
    let c = pos2(rect.center().x, ring_top + avail / 2.0);
    let step = width + width * 0.28;

    // focus from hover (tile or ring band)
    let mut focus = None;
    if let Some(mp) = hover {
        for (i, r) in tiles.iter().enumerate() {
            if r.contains(mp) {
                focus = Some(i);
            }
        }
        let d = mp.distance(c);
        for i in 0..rings.len() {
            let rr = radius - width / 2.0 - i as f32 * step;
            if (d - rr).abs() <= width * 0.5 + 2.0 {
                focus = Some(i);
            }
        }
    }
    let idle = idle_for(ui, id, hovered);
    if focus.is_none() && autoplay && idle > 2.5 && el > 2.0 && !rings.is_empty() {
        let k = ((idle - 2.5) / 1.6) as usize % (rings.len() + 1);
        focus = (k < rings.len()).then_some(k);
        glowkit_core::animate(ui);
    }
    let mut rs = Vec::new();
    for (i, r) in rings.iter().enumerate() {
        let col = ring_color(&t, r, i);
        let em = flag(ui, id.with(("em", i)), focus.is_none() || focus == Some(i), 10.0);
        let fo = flag(ui, id.with(("fo", i)), focus == Some(i), 12.0);
        let ta = ease_out(entry_k(ui, el, 0.05 * i as f32, 0.4));
        let tr = tiles[i].translate(vec2(0.0, (1.0 - ta) * 8.0));
        let mut painter_t = painter.clone();
        painter_t.multiply_opacity(ta.max(0.001));
        stat_tile(&painter_t, &t, tr, &r.label, &r.value, Some(col), fo, false);
        let grow = ease_out(entry_k(ui, el, 0.2 + 0.14 * i as f32, 1.3));
        let shown = tween(ui, id.with(("p", i)), r.progress, 0.0, 0.8) * grow;
        rs.push((shown, col, em));
    }
    paint_rings(painter, &t, c, radius, width, width * 0.28, &rs);
    focus
}

/// An activity card in the style of watch goal rings: a row of stat tiles (colour swatch,
/// label, value) above concentric rings that close as each goal is reached and lap past 100%.
/// Rings sweep in with a stagger, glide when the data changes, and hovering a ring or its tile
/// focuses it while the others step back.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BoardActivityRings, BoardRing};
/// ui.add(BoardActivityRings::new("Activity").rings(vec![
///     BoardRing::new("Move", "1,592 kcal", 0.86),
///     BoardRing::new("Exercise", "1h 45m", 0.74),
///     BoardRing::new("Running", "5.2 km", 0.62),
/// ]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoardActivityRings {
    title: String,
    rings: Vec<BoardRing>,
    width: f32,
    thickness: f32,
    autoplay: bool,
}

impl BoardActivityRings {
    /// An activity card titled `title` with sample move / exercise / running rings.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            rings: vec![
                BoardRing::new("Move", "1,592 kcal", 0.84),
                BoardRing::new("Exercise", "1h 45m", 0.76),
                BoardRing::new("Running", "5.2 km", 0.66),
            ],
            width: 440.0,
            thickness: 18.0,
            autoplay: false,
        }
    }
    /// The rings, outermost first.
    pub fn rings(mut self, r: Vec<BoardRing>) -> Self {
        self.rings = r;
        self
    }
    /// Card width in points (default 440).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(300.0);
        self
    }
    /// Ring thickness in points (default 18).
    pub fn thickness(mut self, w: f32) -> Self {
        self.thickness = w.clamp(6.0, 30.0);
        self
    }
    /// Cycle the ring focus by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for BoardActivityRings {
    fn ui(self, ui: &mut Ui) -> Response {
        let n = self.rings.len() as f32;
        let ring_d = (n * self.thickness * 1.28 * 2.0 + 60.0).max(170.0);
        let h = 46.0 + 56.0 + 14.0 + ring_d + 14.0;
        let (rect, response) = ui.allocate_exact_size(vec2(self.width, h), Sense::hover());
        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect.expand(20.0));
            activity_panel(
                ui,
                &painter,
                response.id,
                rect,
                &self.title,
                &self.rings,
                self.thickness,
                response.hover_pos(),
                self.autoplay,
                response.hovered(),
            );
        }
        response
    }
}

// =============================================================================================
// Sleep score

/// One sub-score of a [`BoardSleepScore`] gauge.
#[derive(Clone, Debug, PartialEq)]
pub struct BoardSleepMetric {
    /// Row label, e.g. `"Duration: 7h 50m"`.
    pub label: String,
    /// Points earned.
    pub score: f32,
    /// Points available (also sets the segment's share of the ring).
    pub max: f32,
    /// Segment colour (`None` = BoardUI accent for its position).
    pub color: Option<Color32>,
}

impl BoardSleepMetric {
    /// A metric row `label` with `score` out of `max` points.
    pub fn new(label: impl Into<String>, score: f32, max: f32) -> Self {
        Self { label: label.into(), score: score.max(0.0), max: max.max(1e-3), color: None }
    }
    /// Set the segment colour.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

/// A week of sleep data for [`BoardSleepScore`]: a range label and its metrics.
pub type BoardSleepWeek = (String, Vec<BoardSleepMetric>);

/// A sleep-score card: a segmented ring gauge (one arc per sub-score, sized by its share of
/// the total and filled by how much of it was earned) around a count-up score, the grade as a
/// headline, a date-range pill with arrows to step through weeks, and a tile of metric rows.
/// Hovering a row thickens its segment and fades the rest; switching weeks re-fills every arc.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BoardSleepMetric, BoardSleepScore};
/// ui.add(BoardSleepScore::new().weeks(vec![(
///     "29 Jun - 5 Jul".into(),
///     vec![
///         BoardSleepMetric::new("Duration: 7h 50m", 49.0, 50.0),
///         BoardSleepMetric::new("Bedtime: 20m earlier", 29.0, 30.0),
///         BoardSleepMetric::new("Interruptions: 5m awake", 20.0, 20.0),
///     ],
/// )]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoardSleepScore {
    title: String,
    weeks: Vec<BoardSleepWeek>,
    width: f32,
    thickness: f32,
    gap: f32,
    autoplay: bool,
}

impl Default for BoardSleepScore {
    fn default() -> Self {
        Self::new()
    }
}

impl BoardSleepScore {
    /// A sleep card with three sample weeks.
    pub fn new() -> Self {
        let wk = |label: &str, d: (&str, f32), b: (&str, f32), i: (&str, f32)| -> BoardSleepWeek {
            (
                label.into(),
                vec![
                    BoardSleepMetric::new(format!("Duration: {}", d.0), d.1, 50.0),
                    BoardSleepMetric::new(format!("Bedtime: {}", b.0), b.1, 30.0),
                    BoardSleepMetric::new(format!("Interruptions: {}", i.0), i.1, 20.0),
                ],
            )
        };
        Self {
            title: "Sleep score".into(),
            weeks: vec![
                wk("15 - 21 Jun", ("6h 40m", 38.0), ("35m later", 17.0), ("22m awake", 12.0)),
                wk("22 - 28 Jun", ("7h 15m", 44.0), ("10m later", 24.0), ("12m awake", 16.0)),
                wk("29 Jun - 5 Jul", ("7h 50m", 49.0), ("20m earlier", 29.0), ("5m awake", 20.0)),
            ],
            width: 440.0,
            thickness: 13.0,
            gap: 16.0,
            autoplay: false,
        }
    }
    /// Card title (default "Sleep score").
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Weeks to page through (the last one is shown first).
    pub fn weeks(mut self, w: Vec<BoardSleepWeek>) -> Self {
        if !w.is_empty() {
            self.weeks = w;
        }
        self
    }
    /// Card width in points (default 440).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(300.0);
        self
    }
    /// Gauge thickness in points (default 13).
    pub fn thickness(mut self, w: f32) -> Self {
        self.thickness = w.clamp(4.0, 26.0);
        self
    }
    /// Gap between segments in degrees (default 16).
    pub fn gap(mut self, deg: f32) -> Self {
        self.gap = deg.clamp(0.0, 40.0);
        self
    }
    /// Focus rows and page weeks by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

/// Grade for a 0..100 score.
fn grade(s: f32) -> &'static str {
    if s >= 90.0 {
        "Excellent"
    } else if s >= 78.0 {
        "Good"
    } else if s >= 62.0 {
        "Fair"
    } else {
        "Poor"
    }
}

impl egui::Widget for BoardSleepScore {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let p = pal(&t);
        let rows_n = self.weeks.iter().map(|w| w.1.len()).max().unwrap_or(0);
        let h = 214.0 + rows_n as f32 * 40.0 + 12.0;
        let (rect, response) = ui.allocate_exact_size(vec2(self.width, h), Sense::hover());
        let id = response.id;
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(20.0));
        let el = entry(ui, id);
        let pad = 16.0;
        let mut week: usize = glowkit_core::state(ui, id.with("week"), || self.weeks.len() - 1);
        week = week.min(self.weeks.len() - 1);

        // pill + navigation
        let pr = Rect::from_min_size(pos2(rect.max.x - pad - 156.0, rect.min.y + 14.0), vec2(156.0, 30.0));
        let lr = Rect::from_min_size(pr.min, vec2(26.0, pr.height()));
        let rr = Rect::from_min_size(pos2(pr.max.x - 26.0, pr.min.y), vec2(26.0, pr.height()));
        let lresp = ui.interact(lr, id.with("prev"), Sense::click());
        let rresp = ui.interact(rr, id.with("next"), Sense::click());
        let idle = idle_for(ui, id, response.hovered() || lresp.hovered() || rresp.hovered());
        if lresp.clicked() && week > 0 {
            week -= 1;
        }
        if rresp.clicked() && week + 1 < self.weeks.len() {
            week += 1;
        }
        // autoplay: page weeks every few seconds
        let mut auto_row = None;
        if self.autoplay && idle > 2.0 && el > 1.5 {
            let cyc = (idle - 2.0) / 1.5;
            let n = rows_n.max(1);
            let step = cyc as usize % (n + 2);
            if step < n {
                auto_row = Some(step);
            }
            let page: usize = ((idle - 2.0) / (1.5 * (n + 2) as f32)) as usize;
            let len = self.weeks.len();
            week = (len - 1 + page) % len;
            glowkit_core::animate(ui);
        }
        glowkit_core::set_state(ui, id.with("week"), week);
        let (label, metrics) = &self.weeks[week];

        frame(&painter, &t, rect, 16.0);
        text(&painter, pos2(rect.min.x + pad, rect.min.y + 26.0), Align2::LEFT_CENTER, &self.title, f(13.0), dim(&t));
        let total: f32 = metrics.iter().map(|m| m.score).sum();
        let max_total: f32 = metrics.iter().map(|m| m.max).sum::<f32>().max(1e-3);
        let score100 = total / max_total * 100.0;
        text(&painter, pos2(rect.min.x + pad, rect.min.y + 52.0), Align2::LEFT_CENTER, grade(score100), f(22.0), t.foreground);
        let hl = [flag(ui, id.with("lh"), lresp.hovered(), 14.0), flag(ui, id.with("rh"), rresp.hovered(), 14.0)];
        range_pill(&painter, &t, pr, label, hl);

        // rows
        let rows_r = Rect::from_min_size(pos2(rect.min.x + 8.0, rect.min.y + 214.0), vec2(rect.width() - 16.0, rows_n as f32 * 40.0));
        let mut focus = None;
        if let Some(mp) = response.hover_pos() {
            if rows_r.contains(mp) {
                focus = Some(((mp.y - rows_r.min.y) / 40.0) as usize);
            }
        }
        focus = focus.or(auto_row).filter(|f| *f < metrics.len());
        tile(&painter, &t, rows_r, 10.0);
        let cols = [p[PURPLE], p[PINK], p[BLUE], p[TEAL], p[YELLOW]];
        for (i, m) in metrics.iter().enumerate() {
            let y = rows_r.min.y + i as f32 * 40.0;
            let col = m.color.unwrap_or(cols[i % cols.len()]);
            let fo = flag(ui, id.with(("rowf", i)), focus == Some(i), 12.0);
            let ra = ease_out(entry_k(ui, el, 0.35 + 0.08 * i as f32, 0.45));
            let rr = Rect::from_min_size(pos2(rows_r.min.x, y), vec2(rows_r.width(), 40.0));
            if fo > 0.01 {
                painter.rect_filled(rr.shrink2(vec2(4.0, 3.0)), cr(8.0), fade(track(&t), fo));
            }
            if i > 0 {
                painter.line_segment([pos2(rr.min.x + 10.0, y), pos2(rr.max.x - 10.0, y)], Stroke::new(1.0, hairline(&t)));
            }
            let a = ra * (1.0 - 0.45 * flag(ui, id.with(("rowd", i)), focus.is_some() && focus != Some(i), 10.0));
            painter.rect_filled(Rect::from_center_size(pos2(rr.min.x + 16.0, rr.center().y), vec2(10.0, 10.0)), cr(3.0), fade(col, a));
            text(&painter, pos2(rr.min.x + 30.0, rr.center().y), Align2::LEFT_CENTER, &m.label, f(13.5), fade(dim(&t), a));
            let sc = tween(ui, id.with(("sc", i)), m.score, 0.0, 0.7);
            text(
                &painter,
                pos2(rr.max.x - 12.0, rr.center().y),
                Align2::RIGHT_CENTER,
                format!("{}/{}", sc.round(), m.max.round()),
                f(13.5),
                fade(t.foreground, a),
            );
        }

        // gauge
        let c = pos2(rect.center().x, rect.min.y + 134.0);
        let r = 52.0;
        let gap = self.gap.to_radians();
        let usable = TAU - gap * metrics.len() as f32;
        let mut a = -PI / 2.0 + gap / 2.0;
        for (i, m) in metrics.iter().enumerate() {
            let span = usable * m.max / max_total;
            let col = m.color.unwrap_or(cols[i % cols.len()]);
            let fo = flag(ui, id.with(("rowf", i)), focus == Some(i), 12.0);
            let other = flag(ui, id.with(("rowd", i)), focus.is_some() && focus != Some(i), 10.0);
            let w = self.thickness + 5.0 * fo;
            let grow = ease_out(entry_k(ui, el, 0.15 + 0.15 * i as f32, 0.9));
            let frac = (tween(ui, id.with(("arc", i)), m.score / m.max, 0.0, 0.8) * grow).clamp(0.0, 1.0);
            // round caps eat into the gap: inset by half the width in angle
            let inset = (w * 0.5 / r).min(span * 0.45);
            let a0 = a + inset;
            let full = (span - inset * 2.0).max(0.0);
            ring_arc(&painter, c, r, a0, full, w, fade(track(&t), 1.0));
            let alpha = 1.0 - 0.6 * other;
            if frac > 0.0 {
                ring_arc(&painter, c, r, a0, full * frac, w, fade(col, alpha));
            }
            a += span + gap;
        }
        let shown = tween(ui, id.with("score"), score100, 0.0, 1.1);
        text(&painter, c, Align2::CENTER_CENTER, format!("{}", shown.round()), f(32.0), t.foreground);
        for r in [&lresp, &rresp] {
            if r.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
        response
    }
}

// =============================================================================================
// Most active days

/// One day of a [`BoardMonth`].
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct BoardDay {
    /// Move, exercise and running goal progress (`1.0` = goal reached). All zero = no data.
    pub progress: [f32; 3],
    /// Steps walked that day.
    pub steps: u32,
}

/// A month of daily activity for [`BoardActiveDays`].
#[derive(Clone, Debug, PartialEq)]
pub struct BoardMonth {
    /// Full month name ("July").
    pub name: String,
    /// Weekday of the 1st, `0` = Monday.
    pub first_weekday: usize,
    /// One entry per day of the month.
    pub days: Vec<BoardDay>,
}

impl BoardMonth {
    /// A month named `name` starting on `first_weekday` (0 = Monday) with explicit days.
    pub fn new(name: impl Into<String>, first_weekday: usize, days: Vec<BoardDay>) -> Self {
        Self { name: name.into(), first_weekday: first_weekday % 7, days }
    }
    /// A month of `len` days with plausible generated activity for the first `filled` days.
    pub fn generated(name: impl Into<String>, first_weekday: usize, len: usize, filled: usize, seed: u64) -> Self {
        let mut rng = glowkit_core::Rng::new(seed);
        let days = (0..len)
            .map(|i| {
                if i >= filled {
                    return BoardDay::default();
                }
                let weekend = (first_weekday + i) % 7 >= 5;
                let base = if weekend { 0.75 } else { 0.55 };
                let progress = [
                    (base + rng.range(-0.25, 0.55)).max(0.08),
                    (base + rng.range(-0.3, 0.6)).max(0.05),
                    (base - 0.1 + rng.range(-0.3, 0.6)).max(0.05),
                ];
                BoardDay { progress, steps: (4200.0 + progress[0] * 6500.0 + rng.range(0.0, 1800.0)) as u32 }
            })
            .collect();
        Self::new(name, first_weekday, days)
    }
}

/// A "most active days" pair of cards: a scrolling month calendar where every day shows a mini
/// activity ring set (selected day outlined, month switcher with arrows that slides the grid,
/// total steps as the headline), and beside it the activity rings and tiles of the selected
/// day. Click a day to drive the rings; scroll the calendar with the wheel. While idle the
/// selection walks through the month by itself when autoplay is on.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BoardActiveDays, BoardMonth};
/// ui.add(BoardActiveDays::new(vec![BoardMonth::generated("July", 2, 31, 12, 7)]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoardActiveDays {
    months: Vec<BoardMonth>,
    year: i32,
    goals: [f32; 3],
    width: f32,
    height: f32,
    autoplay: bool,
}

impl BoardActiveDays {
    /// Cards for `months` (the last month is shown first).
    pub fn new(months: Vec<BoardMonth>) -> Self {
        let months = if months.is_empty() { vec![BoardMonth::generated("July", 2, 31, 12, 7)] } else { months };
        Self { months, year: 2026, goals: [600.0, 60.0, 5.0], width: 740.0, height: 340.0, autoplay: false }
    }
    /// Year shown in the day title.
    pub fn year(mut self, y: i32) -> Self {
        self.year = y;
        self
    }
    /// Daily goals: move kcal, exercise minutes, running km (default 600, 60, 5).
    pub fn goals(mut self, g: [f32; 3]) -> Self {
        self.goals = g;
        self
    }
    /// Overall size of the pair in points (default 740 x 340).
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.width = w.max(520.0);
        self.height = h.max(280.0);
        self
    }
    /// Walk the selection through the month by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

#[derive(Clone, Copy)]
struct DaysState {
    month: usize,
    day: usize,
    scroll: f32,
    slide: f32,
}

impl egui::Widget for BoardActiveDays {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let p = pal(&t);
        let (rect, response) = ui.allocate_exact_size(vec2(self.width, self.height), Sense::hover());
        let id = response.id;
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(20.0));
        let el = entry(ui, id);
        let gap = 16.0;
        let lw = (self.width - gap) * 0.5;
        let left = Rect::from_min_size(rect.min, vec2(lw, self.height));
        let right = Rect::from_min_max(pos2(left.max.x + gap, rect.min.y), rect.max);
        let last = self.months.len() - 1;
        let default_day = self.months[last].days.iter().rposition(|d| d.progress.iter().any(|v| *v > 0.0)).unwrap_or(0);
        let mut st: DaysState =
            glowkit_core::state(ui, id.with("st"), || DaysState { month: last, day: default_day, scroll: f32::NAN, slide: 0.0 });
        st.month = st.month.min(last);

        // header + month pill
        frame(&painter, &t, left, 16.0);
        let pad = 16.0;
        let pr = Rect::from_min_size(pos2(left.max.x - pad - 128.0, left.min.y + 14.0), vec2(128.0, 30.0));
        let lr = Rect::from_min_size(pr.min, vec2(26.0, 30.0));
        let rr = Rect::from_min_size(pos2(pr.max.x - 26.0, pr.min.y), vec2(26.0, 30.0));
        let lresp = ui.interact(lr, id.with("pm"), Sense::click());
        let rresp = ui.interact(rr, id.with("nm"), Sense::click());
        if lresp.clicked() && st.month > 0 {
            st.month -= 1;
            st.slide = -1.0;
            st.day = 0;
            st.scroll = f32::NAN;
        }
        if rresp.clicked() && st.month < last {
            st.month += 1;
            st.slide = 1.0;
            st.day = 0;
            st.scroll = f32::NAN;
        }
        let m = &self.months[st.month];
        text(&painter, pos2(left.min.x + pad, left.min.y + 26.0), Align2::LEFT_CENTER, "Most active days", f(13.0), dim(&t));
        let total: u32 = m.days.iter().map(|d| d.steps).sum();
        let shown = tween(ui, id.with(("steps", st.month)), total as f32, 0.0, 1.0);
        let hr = text(&painter, pos2(left.min.x + pad, left.min.y + 54.0), Align2::LEFT_CENTER, thousands(shown as f64), f(24.0), t.foreground);
        text(&painter, pos2(hr.max.x + 6.0, hr.max.y - 9.0), Align2::LEFT_CENTER, "total steps", f(12.0), dim(&t));
        let hl = [flag(ui, id.with("lh"), lresp.hovered(), 14.0), flag(ui, id.with("rh"), rresp.hovered(), 14.0)];
        range_pill(&painter, &t, pr, &m.name, hl);

        // calendar viewport
        let vp = Rect::from_min_max(pos2(left.min.x + 10.0, left.min.y + 82.0), pos2(left.max.x - 10.0, left.max.y - 10.0));
        tile(&painter, &t, vp, 12.0);
        let cw = (vp.width() - 16.0) / 7.0;
        let ch = 62.0;
        let rows = (m.first_weekday + m.days.len()).div_ceil(7);
        let head = 34.0;
        let content_h = head + rows as f32 * ch + 8.0;
        let max_scroll = (content_h - vp.height()).max(0.0);
        let vresp = ui.interact(vp, id.with("vp"), Sense::click());
        let idle = idle_for(ui, id, response.hovered() || vresp.hovered());
        // autoplay: walk the selection through the filled days
        let filled = m.days.iter().rposition(|d| d.progress.iter().any(|v| *v > 0.0)).map(|i| i + 1).unwrap_or(0);
        if self.autoplay && idle > 2.0 && el > 1.0 && filled > 0 {
            let k = ((idle - 2.0) / 1.3) as usize;
            st.day = (k * 3 + 2) % filled;
            glowkit_core::animate(ui);
        }
        let sel_row = (m.first_weekday + st.day) / 7;
        let want = if sel_row < 2 { 0.0 } else { (head + (sel_row - 1) as f32 * ch - 4.0).clamp(0.0, max_scroll) };
        if st.scroll.is_nan() {
            st.scroll = want;
        }
        if vresp.hovered() {
            let wheel = ui.input(|i| i.smooth_scroll_delta.y);
            if wheel != 0.0 {
                st.scroll = (st.scroll - wheel).clamp(0.0, max_scroll);
            }
        } else if self.autoplay && idle > 2.0 {
            st.scroll += (want - st.scroll) * 0.12;
            if (want - st.scroll).abs() > 0.5 {
                glowkit_core::animate(ui);
            }
        }
        let scroll = glowkit_core::anim::spring(ui, id.with("scroll"), st.scroll, 260.0, 30.0);
        let dt = glowkit_core::dt(ui);
        st.slide = glowkit_core::anim::approach(st.slide, 0.0, 9.0, dt);
        if st.slide.abs() < 0.002 {
            st.slide = 0.0;
        } else {
            glowkit_core::animate(ui);
        }
        let slide = st.slide;
        let cal = painter.with_clip_rect(vp.shrink(1.0).intersect(painter.clip_rect()));
        let ox = slide * vp.width() * 0.6;
        let ca = 1.0 - slide.abs().min(1.0);
        let y0 = vp.min.y - scroll;
        let short: String = m.name.chars().take(3).collect();
        text(&cal, pos2(vp.min.x + 14.0 + ox, y0 + 20.0), Align2::LEFT_CENTER, short, f(16.0), fade(t.foreground, ca));
        let ptr = vresp.hover_pos();
        let mut hovered_day = None;
        for (i, d) in m.days.iter().enumerate() {
            let slot = m.first_weekday + i;
            let (col, row) = (slot % 7, slot / 7);
            let cell = Rect::from_min_size(pos2(vp.min.x + 8.0 + col as f32 * cw + ox, y0 + head + row as f32 * ch), vec2(cw, ch));
            if cell.max.y < vp.min.y || cell.min.y > vp.max.y {
                continue;
            }
            let hov = ptr.is_some_and(|q| cell.contains(q));
            if hov {
                hovered_day = Some(i);
            }
            let sel = st.day == i;
            let sf = flag(ui, id.with(("sel", st.month, i)), sel, 14.0);
            let hf = flag(ui, id.with(("hov", st.month, i)), hov, 14.0);
            let box_r = cell.shrink2(vec2(3.0, 2.0));
            if sf > 0.01 {
                cal.rect_stroke(box_r, cr(10.0), Stroke::new(1.5, fade(mix(t.border, t.muted_foreground, 0.5), sf * ca)), egui::StrokeKind::Inside);
            }
            if hf > 0.01 {
                cal.rect_filled(box_r, cr(10.0), fade(track(&t), hf * 0.7 * ca));
            }
            text(&cal, pos2(cell.center().x, cell.min.y + 13.0), Align2::CENTER_CENTER, i + 1, f(13.0), fade(t.foreground, ca));
            let rc = pos2(cell.center().x, cell.min.y + 41.0);
            let grow = ease_out(entry_k(ui, el, 0.2 + 0.015 * i as f32, 0.8));
            let empty = d.progress.iter().all(|v| *v <= 0.0);
            let rs: Vec<(f32, Color32, f32)> = (0..3)
                .map(|k| (d.progress[k] * grow, [p[PINK], p[LIME], p[BLUE]][k], if empty { 0.0 } else { 1.0 }))
                .collect();
            let mut rp = cal.clone();
            rp.multiply_opacity(ca.max(0.001));
            paint_rings(&rp, &t, rc, 14.0, 3.2, 1.3, &rs);
        }
        if let Some(d) = hovered_day {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            if vresp.clicked() {
                st.day = d;
            }
        }
        // scroll shadow fades
        paint_fade(&painter, &t, vp, scroll > 1.0, scroll < max_scroll - 1.0);
        glowkit_core::set_state(ui, id.with("st"), st);

        // right: selected day rings
        let d = m.days.get(st.day).copied().unwrap_or_default();
        let mins = d.progress[1] * self.goals[1];
        let rings = vec![
            BoardRing::new("Move", format!("{} kcal", thousands((d.progress[0] * self.goals[0]) as f64)), d.progress[0]),
            BoardRing::new("Exercise", format!("{}h {:02}m", (mins / 60.0) as u32, (mins % 60.0) as u32), d.progress[1]),
            BoardRing::new("Running", format!("{:.1} km", d.progress[2] * self.goals[2]), d.progress[2]),
        ];
        let title = format!("Activity for {} {}, {}", m.name, st.day + 1, self.year);
        activity_panel(ui, &painter, id.with("rings"), right, &title, &rings, 17.0, response.hover_pos().filter(|q| right.contains(*q)), false, false);
        for r in [&lresp, &rresp] {
            if r.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
        response
    }
}

fn paint_fade(painter: &Painter, t: &Theme, vp: Rect, top: bool, bottom: bool) {
    let bg = tile_bg(t);
    let h = 18.0;
    let inner = vp.shrink2(vec2(1.0, 1.0));
    if top {
        let r = Rect::from_min_size(inner.min + vec2(0.0, 0.0), vec2(inner.width(), h));
        glowkit_core::paint::quad(painter, [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()], [bg, bg, Color32::TRANSPARENT, Color32::TRANSPARENT]);
    }
    if bottom {
        let r = Rect::from_min_max(pos2(inner.min.x + 6.0, inner.max.y - h), pos2(inner.max.x - 6.0, inner.max.y - 6.0));
        glowkit_core::paint::quad(painter, [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()], [Color32::TRANSPARENT, Color32::TRANSPARENT, bg, bg]);
    }
}
