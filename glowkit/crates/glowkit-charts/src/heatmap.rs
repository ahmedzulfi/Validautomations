//! Heatmaps: a labelled matrix and a GitHub-style contribution calendar.

use crate::cartesian::labels_vec;
use crate::common::*;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::anim::{ease, Easing};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// Color for intensity `k` (0..1): from a faint tint of `color` to full `color`.
fn ramp(t: &Theme, color: Color32, k: f32) -> Color32 {
    let base = if t.dark { mix(t.card, t.muted_foreground, 0.12) } else { mix(t.card, t.muted_foreground, 0.1) };
    if k <= 0.0 {
        return base;
    }
    mix(mix(base, color, 0.22), color, k.clamp(0.0, 1.0).powf(0.9))
}

/// A matrix of colored cells with row and column labels (e.g. activity by weekday × hour).
///
/// Cells fade in along a diagonal sweep; hovering outlines a cell and shows its value.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::Heatmap;
/// let rows = vec![vec![1.0, 4.0, 2.0], vec![3.0, 0.0, 5.0]];
/// ui.add(Heatmap::new(&rows).row_labels(&["Mon", "Tue"]).col_labels(&["9h", "10h", "11h"]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Heatmap<'a> {
    rows: &'a [Vec<f64>],
    row_labels: Vec<String>,
    col_labels: Vec<String>,
    base: Base,
    color: Option<Color32>,
    gap: f32,
    radius: f32,
    show_values: bool,
    scale: bool,
}

impl<'a> Heatmap<'a> {
    /// A heatmap of `rows` (each row a vector of column values).
    pub fn new(rows: &'a [Vec<f64>]) -> Self {
        Self {
            rows,
            row_labels: Vec::new(),
            col_labels: Vec::new(),
            base: Base::new(260.0),
            color: None,
            gap: 3.0,
            radius: 4.0,
            show_values: false,
            scale: true,
        }
    }
    base_setters!();
    /// Labels on the left, one per row.
    pub fn row_labels<S: AsRef<str>>(mut self, l: &[S]) -> Self {
        self.row_labels = labels_vec(l);
        self
    }
    /// Labels along the bottom, one per column.
    pub fn col_labels<S: AsRef<str>>(mut self, l: &[S]) -> Self {
        self.col_labels = labels_vec(l);
        self
    }
    /// Hot color (default: the first palette color).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Gap between cells (default 3).
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g;
        self
    }
    /// Cell corner radius (default 4).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r;
        self
    }
    /// Print values inside cells.
    pub fn show_values(mut self, on: bool) -> Self {
        self.show_values = on;
        self
    }
    /// Show the color scale below the grid (default `true`).
    pub fn scale(mut self, on: bool) -> Self {
        self.scale = on;
        self
    }
}

impl Widget for Heatmap<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp, id) = self.base.allocate(ui, Sense::hover());
        let nr = self.rows.len();
        let nc = self.rows.iter().map(|r| r.len()).max().unwrap_or(0);
        if nr == 0 || nc == 0 || !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter_at(rect);
        let font = axis_font();
        let color = self.color.unwrap_or(t.chart[0]);
        let left = self.row_labels.iter().map(|l| text_width(&painter, l, &font)).fold(0.0, f32::max)
            + if self.row_labels.is_empty() { 0.0 } else { 10.0 };
        let bottom = if self.col_labels.is_empty() { 0.0 } else { 22.0 } + if self.scale { 30.0 } else { 0.0 };
        let grid = Rect::from_min_max(
            pos2(rect.left() + left, rect.top() + 2.0),
            pos2(rect.right() - 2.0, rect.bottom() - bottom),
        );
        let cw = (grid.width() + self.gap) / nc as f32;
        let ch = (grid.height() + self.gap) / nr as f32;
        let (lo, hi) =
            self.rows.iter().flatten().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(*v), b.max(*v)));
        let span = (hi - lo).max(1e-9);
        let e = entry_time(ui, id, self.base.animate, 1.4);
        let hover = resp.hover_pos().filter(|p| grid.contains(*p)).map(|p| {
            let c = (((p.x - grid.left()) / cw).floor() as usize).min(nc - 1);
            let r = (((p.y - grid.top()) / ch).floor() as usize).min(nr - 1);
            (r, c)
        });
        for (r, row) in self.rows.iter().enumerate() {
            if let Some(l) = self.row_labels.get(r) {
                let hl = hover.map(|h| h.0 == r).unwrap_or(false);
                painter.text(
                    pos2(grid.left() - 10.0, grid.top() + r as f32 * ch + (ch - self.gap) / 2.0),
                    Align2::RIGHT_CENTER,
                    l,
                    font.clone(),
                    if hl { t.foreground } else { t.muted_foreground },
                );
            }
            for (c, v) in row.iter().enumerate() {
                let k = ((v - lo) / span) as f32;
                let delay = (r + c) as f32 / (nr + nc) as f32 * 0.7;
                let a = if self.base.animate { ease(Easing::OutCubic, (e - delay) / 0.5) } else { 1.0 };
                let cell = Rect::from_min_size(
                    pos2(grid.left() + c as f32 * cw, grid.top() + r as f32 * ch),
                    vec2(cw - self.gap, ch - self.gap),
                );
                let cell = Rect::from_center_size(cell.center(), cell.size() * (0.6 + 0.4 * a));
                let col = ramp(&t, color, k.max(0.001));
                let cr = CornerRadius::same(self.radius.min(cell.width().min(cell.height()) / 2.0) as u8);
                painter.rect_filled(cell, cr, col.gamma_multiply(a));
                if hover == Some((r, c)) {
                    painter.rect_stroke(cell.expand(1.5), cr, Stroke::new(1.5, t.foreground), StrokeKind::Outside);
                }
                if self.show_values && cell.width() > 26.0 && a > 0.9 {
                    let tc = if k > 0.55 {
                        glowkit_core::color::contrast_text(color)
                    } else {
                        t.foreground.gamma_multiply(0.8)
                    };
                    painter.text(
                        cell.center(),
                        Align2::CENTER_CENTER,
                        (self.base.format)(*v),
                        FontId::proportional(10.5),
                        tc,
                    );
                }
            }
        }
        let mut y = grid.bottom() + 8.0;
        if !self.col_labels.is_empty() {
            let stride = label_stride(&painter, &self.col_labels, cw, &font);
            for (c, l) in self.col_labels.iter().enumerate().take(nc) {
                if c % stride == 0 {
                    let hl = hover.map(|h| h.1 == c).unwrap_or(false);
                    painter.text(
                        pos2(grid.left() + c as f32 * cw + (cw - self.gap) / 2.0, y),
                        Align2::CENTER_TOP,
                        l,
                        font.clone(),
                        if hl { t.foreground } else { t.muted_foreground },
                    );
                }
            }
            y += 22.0;
        }
        if self.scale {
            // gradient key: "lo [=====] hi", right-aligned under the grid
            let (lo_s, hi_s) = ((self.base.format)(lo), (self.base.format)(hi));
            let hi_w = text_width(&painter, &hi_s, &font);
            let w = 140.0f32.min(grid.width() * 0.5);
            let bar = Rect::from_min_size(pos2(grid.right() - hi_w - 8.0 - w, y + 6.0), vec2(w, 8.0));
            let steps = 24;
            for s in 0..steps {
                let k = s as f32 / (steps - 1) as f32;
                let r = Rect::from_min_size(
                    pos2(bar.left() + bar.width() * s as f32 / steps as f32, bar.top()),
                    vec2(bar.width() / steps as f32 + 0.5, bar.height()),
                );
                painter.rect_filled(r, CornerRadius::ZERO, ramp(&t, color, k));
            }
            painter.rect_stroke(
                bar,
                CornerRadius::same(2),
                Stroke::new(1.0, with_alpha(t.border, 0.6)),
                StrokeKind::Outside,
            );
            painter.text(
                pos2(bar.left() - 8.0, bar.center().y),
                Align2::RIGHT_CENTER,
                lo_s,
                font.clone(),
                t.muted_foreground,
            );
            painter.text(
                pos2(bar.right() + 8.0, bar.center().y),
                Align2::LEFT_CENTER,
                hi_s,
                font.clone(),
                t.muted_foreground,
            );
        }
        if let (Some((r, c)), true) = (hover, self.base.tooltip) {
            let v = self.rows[r][c];
            let title = match (self.row_labels.get(r), self.col_labels.get(c)) {
                (Some(a), Some(b)) => format!("{a} · {b}"),
                (Some(a), None) => a.clone(),
                (None, Some(b)) => b.clone(),
                _ => String::new(),
            };
            let k = ((v - lo) / span) as f32;
            tooltip(
                ui,
                id,
                resp.hover_pos().unwrap_or(rect.center()),
                &title,
                &[TipRow::new(Some(ramp(&t, color, k)), "Value", (self.base.format)(v))],
                1.0,
            );
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// CalendarHeatmap
// ---------------------------------------------------------------------------------------------

/// A GitHub-style contribution calendar: one cell per day, weeks as columns, five intensity
/// levels, month and weekday labels, a "Less … More" key and day tooltips.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::CalendarHeatmap;
/// let days: Vec<f64> = (0..365).map(|i| (i % 7) as f64).collect();
/// ui.add(CalendarHeatmap::new(&days).start_date(2025, 1, 1));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CalendarHeatmap<'a> {
    values: &'a [f64],
    start: (i32, u32, u32),
    cell: f32,
    gap: f32,
    color: Option<Color32>,
    unit: String,
    animate: bool,
    legend: bool,
}

impl<'a> CalendarHeatmap<'a> {
    /// One value per consecutive day, starting at [`CalendarHeatmap::start_date`].
    pub fn new(values: &'a [f64]) -> Self {
        Self {
            values,
            start: (2025, 1, 1),
            cell: 12.0,
            gap: 3.0,
            color: None,
            unit: "contributions".into(),
            animate: true,
            legend: true,
        }
    }
    /// Date of the first value (year, month 1-12, day 1-31).
    pub fn start_date(mut self, year: i32, month: u32, day: u32) -> Self {
        self.start = (year, month, day);
        self
    }
    /// Maximum cell size in points (default 12; shrinks to fit the width).
    pub fn cell_size(mut self, s: f32) -> Self {
        self.cell = s;
        self
    }
    /// Gap between cells (default 3).
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g;
        self
    }
    /// Hot color (default: the success color, like GitHub).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Unit used in tooltips (default "contributions").
    pub fn unit(mut self, u: impl Into<String>) -> Self {
        self.unit = u.into();
        self
    }
    /// Entry animation (default `true`).
    pub fn animate(mut self, on: bool) -> Self {
        self.animate = on;
        self
    }
    /// Show the "Less … More" key (default `true`).
    pub fn legend(mut self, on: bool) -> Self {
        self.legend = on;
        self
    }
}

impl Widget for CalendarHeatmap<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let start = days_from_civil(self.start.0, self.start.1, self.start.2);
        let lead = weekday(start) as usize; // empty cells before the first day (Sunday-first rows)
        let total = lead + self.values.len();
        let weeks = total.div_ceil(7).max(1);
        let label_w = 30.0;
        let avail = ui.available_width();
        let cell = ((avail - label_w) / weeks as f32 - self.gap).clamp(4.0, self.cell);
        let step = cell + self.gap;
        let top = 20.0;
        let h = top + 7.0 * step + if self.legend { 28.0 } else { 0.0 };
        let w = label_w + weeks as f32 * step;
        let (rect, resp) = ui.allocate_exact_size(vec2(w.min(avail), h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        let painter = ui.painter_at(rect.expand(2.0));
        let color = self.color.unwrap_or(t.success);
        let font = FontId::proportional(10.5);
        let origin = pos2(rect.left() + label_w, rect.top() + top);
        let max = self.values.iter().fold(0.0f64, |m, v| m.max(*v)).max(1e-9);
        let level = |v: f64| -> usize {
            if v <= 0.0 {
                0
            } else {
                ((v / max * 4.0).ceil() as usize).clamp(1, 4)
            }
        };
        let level_color = |l: usize| ramp(&t, color, if l == 0 { 0.0 } else { l as f32 / 4.0 });
        for (r, name) in [(1, "Mon"), (3, "Wed"), (5, "Fri")] {
            painter.text(
                pos2(rect.left(), origin.y + r as f32 * step + cell / 2.0),
                Align2::LEFT_CENTER,
                name,
                font.clone(),
                t.muted_foreground,
            );
        }
        let e = entry_time(ui, id, self.animate, 1.5);
        let hover = resp.hover_pos().and_then(|p| {
            let c = ((p.x - origin.x) / step).floor();
            let r = ((p.y - origin.y) / step).floor();
            if c < 0.0 || !(0.0..7.0).contains(&r) {
                return None;
            }
            let idx = c as usize * 7 + r as usize;
            (idx >= lead && idx < total).then_some(idx - lead)
        });
        let mut last_month = 0;
        let mut last_label_x = f32::NEG_INFINITY;
        for (i, v) in self.values.iter().enumerate() {
            let cell_i = i + lead;
            let (wk, wd) = (cell_i / 7, cell_i % 7);
            let x = origin.x + wk as f32 * step;
            let y = origin.y + wd as f32 * step;
            let (_, m, _) = civil_from_days(start + i as i64);
            if m != last_month && wd <= 1 || i == 0 {
                if m != last_month && x - last_label_x > 28.0 {
                    painter.text(
                        pos2(x, rect.top() + 4.0),
                        Align2::LEFT_TOP,
                        month_name(m),
                        font.clone(),
                        t.muted_foreground,
                    );
                    last_label_x = x;
                }
                last_month = m;
            }
            let a = if self.animate { ease(Easing::OutCubic, (e - wk as f32 / weeks as f32 * 0.9) / 0.4) } else { 1.0 };
            let r = Rect::from_min_size(pos2(x, y), vec2(cell, cell));
            let r = Rect::from_center_size(r.center(), r.size() * (0.5 + 0.5 * a));
            let cr = CornerRadius::same((cell * 0.22).round() as u8);
            painter.rect_filled(r, cr, level_color(level(*v)).gamma_multiply(a));
            if hover == Some(i) {
                painter.rect_stroke(r.expand(1.0), cr, Stroke::new(1.5, t.foreground), StrokeKind::Outside);
            }
        }
        if self.legend {
            let y = origin.y + 7.0 * step + 14.0;
            let mut x = (origin.x + weeks as f32 * step - self.gap).min(rect.right()) - 5.0 * (cell + 3.0) - 30.0;
            painter.text(pos2(x - 6.0, y), Align2::RIGHT_CENTER, "Less", font.clone(), t.muted_foreground);
            for l in 0..5 {
                painter.rect_filled(
                    Rect::from_min_size(pos2(x, y - cell / 2.0), vec2(cell, cell)),
                    CornerRadius::same((cell * 0.22).round() as u8),
                    level_color(l),
                );
                x += cell + 3.0;
            }
            painter.text(pos2(x + 3.0, y), Align2::LEFT_CENTER, "More", font.clone(), t.muted_foreground);
            let total_sum: f64 = self.values.iter().sum();
            painter.text(
                pos2(origin.x, y),
                Align2::LEFT_CENTER,
                format!("{} {} in the last year", format_thousands(total_sum, 0), self.unit),
                font.clone(),
                t.muted_foreground,
            );
        }
        if let Some(i) = hover {
            let (y, m, d) = civil_from_days(start + i as i64);
            let v = self.values[i];
            let title = format!("{} {} on {} {}, {}", format_thousands(v, 0), self.unit, month_name(m), d, y);
            let cell_i = i + lead;
            let p = pos2(
                origin.x + (cell_i / 7) as f32 * step + cell / 2.0,
                origin.y + (cell_i % 7) as f32 * step + cell / 2.0,
            );
            tooltip(ui, id, p, &title, &[], 1.0);
        }
        resp
    }
}
