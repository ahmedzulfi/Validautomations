//! Line and area charts over categories.

use crate::common::*;
use egui::{pos2, vec2, Color32, Id, Painter, Rect, Response, Sense, Ui, Widget};
use glowkit_core::anim::{ease, Easing};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// Where things go inside a category chart.
pub(crate) struct CatFrame {
    pub plot: Rect,
    pub x_label_y: f32,
    pub y_label_x: f32,
    pub y2_label_x: f32,
    pub legend: Option<Rect>,
}

/// Lay out a category chart: y labels on the left (and optionally right), x labels below, legend
/// at the bottom.
pub(crate) fn cat_frame(rect: Rect, base: &Base, y_w: f32, y2_w: Option<f32>, legend: bool, x_pad: f32) -> CatFrame {
    let left = if base.y_axis { y_w + 12.0 } else { 4.0 };
    let right = y2_w.map(|w| w + 12.0).unwrap_or(8.0);
    let bottom = if base.x_axis { 24.0 } else { 6.0 } + if legend { LEGEND_H } else { 0.0 };
    let plot = Rect::from_min_max(
        pos2(rect.left() + left + x_pad, rect.top() + 8.0),
        pos2(rect.right() - right - x_pad, rect.bottom() - bottom),
    );
    CatFrame {
        plot,
        x_label_y: plot.bottom() + 8.0,
        y_label_x: rect.left() + left - 10.0,
        y2_label_x: rect.right() - right + 10.0,
        legend: legend.then(|| Rect::from_min_max(pos2(rect.left(), rect.bottom() - LEGEND_H), rect.max)),
    }
}

/// Value range of visible series (raw, before nice rounding).
pub(crate) fn data_range<'a>(values: impl Iterator<Item = &'a f64>, include_zero: bool) -> (f64, f64) {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for &v in values {
        if v.is_finite() {
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    if !lo.is_finite() {
        return (0.0, 1.0);
    }
    if include_zero {
        lo = lo.min(0.0);
        hi = hi.max(0.0);
    }
    (lo, hi)
}

pub(crate) fn labels_vec<S: AsRef<str>>(labels: &[S]) -> Vec<String> {
    labels.iter().map(|s| s.as_ref().to_owned()).collect()
}

/// Shared hover logic for category charts: returns `(index, crosshair_x, alpha)`.
pub(crate) fn cat_hover(
    ui: &Ui,
    id: Id,
    resp: &Response,
    plot: Rect,
    n: usize,
    band: bool,
) -> (Option<usize>, f32, f32) {
    let pos = resp.hover_pos().filter(|p| plot.expand2(vec2(8.0, 4.0)).contains(*p));
    let idx = pos.map(|p| cat_index(plot, n, p.x, band));
    let alpha = glowkit_core::anim::follow(ui, id.with("hover_alpha"), if idx.is_some() { 1.0 } else { 0.0 }, 16.0);
    let target = idx
        .map(|i| cat_x(plot, n, i, band))
        .unwrap_or_else(|| glowkit_core::state(ui, id.with("hover_last_x"), || plot.center().x));
    if idx.is_some() {
        glowkit_core::set_state(ui, id.with("hover_last_x"), target);
    }
    let x = follow_snap(ui, id.with("hover_x"), target, 28.0, alpha < 0.05);
    (idx.or(if alpha > 0.02 { Some(cat_index(plot, n, x, band)) } else { None }), x, alpha)
}

/// Active dot: halo, card-colored ring, filled center.
pub(crate) fn active_dot(painter: &Painter, t: &Theme, p: egui::Pos2, color: Color32, alpha: f32) {
    painter.circle_filled(p, 10.0, with_alpha(color, 0.18 * alpha));
    painter.circle_filled(p, 5.5, t.card.gamma_multiply(alpha));
    painter.circle_filled(p, 4.0, color.gamma_multiply(alpha));
}

// ---------------------------------------------------------------------------------------------
// LineChart
// ---------------------------------------------------------------------------------------------

/// A multi-series line chart over categories (months, days...).
///
/// Lines draw in from left to right, a dashed crosshair snaps to the nearest category with a
/// tooltip card, and the clickable legend toggles series (the y axis re-scales smoothly). When
/// the data changes, lines glide to their new values.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::{LineChart, Series, Curve};
/// let series = [Series::new("Desktop", [186.0, 305.0, 237.0, 73.0, 209.0, 214.0]),
///               Series::new("Mobile", [80.0, 200.0, 120.0, 190.0, 130.0, 140.0])];
/// ui.add(LineChart::new(&series).labels(&["Jan", "Feb", "Mar", "Apr", "May", "Jun"])
///     .curve(Curve::Monotone).dots(true).glow(true).height(260.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct LineChart<'a> {
    series: &'a [Series],
    labels: Vec<String>,
    base: Base,
    curve: Curve,
    line_width: f32,
    dots: bool,
    glow: bool,
    gradient_stroke: bool,
    style: LineStyle,
    series_styles: Vec<(usize, LineStyle)>,
    include_zero: bool,
    y_range: Option<(f64, f64)>,
}

impl<'a> LineChart<'a> {
    /// A line chart of `series`; every series should have one value per category.
    pub fn new(series: &'a [Series]) -> Self {
        Self {
            series,
            labels: Vec::new(),
            base: Base::new(260.0),
            curve: Curve::Monotone,
            line_width: 2.0,
            dots: false,
            glow: false,
            gradient_stroke: false,
            style: LineStyle::Solid,
            series_styles: Vec::new(),
            include_zero: true,
            y_range: None,
        }
    }
    base_setters!();
    axis_setters!();
    /// Category labels along the x axis.
    pub fn labels<S: AsRef<str>>(mut self, labels: &[S]) -> Self {
        self.labels = labels_vec(labels);
        self
    }
    /// Interpolation between points (default [`Curve::Monotone`]).
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }
    /// Stroke width in points (default 2).
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }
    /// Draw a dot at every data point.
    pub fn dots(mut self, dots: bool) -> Self {
        self.dots = dots;
        self
    }
    /// Soft neon glow under each line.
    pub fn glow(mut self, glow: bool) -> Self {
        self.glow = glow;
        self
    }
    /// Stroke each line with a left-to-right gradient into the next palette color.
    pub fn gradient_stroke(mut self, on: bool) -> Self {
        self.gradient_stroke = on;
        self
    }
    /// Stroke style for all series.
    pub fn line_style(mut self, style: LineStyle) -> Self {
        self.style = style;
        self
    }
    /// Stroke style for one series (e.g. a dashed "forecast").
    pub fn series_style(mut self, index: usize, style: LineStyle) -> Self {
        self.series_styles.push((index, style));
        self
    }
    /// Always include zero in the y axis (default `true`).
    pub fn include_zero(mut self, on: bool) -> Self {
        self.include_zero = on;
        self
    }
    /// Fix the y axis range instead of fitting the data.
    pub fn y_range(mut self, min: f64, max: f64) -> Self {
        self.y_range = Some((min, max));
        self
    }
}

impl Widget for LineChart<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp, id) = self.base.allocate(ui, Sense::hover());
        let k = self.series.len();
        let n = self.series.iter().map(|s| s.values.len()).max().unwrap_or(0);
        let mut hidden = hidden_state(ui, id, k);
        let painter = ui.painter_at(rect);
        let legend_on = self.base.legend && k > 0;
        // legend first: it can toggle series
        let frame_guess = cat_frame(rect, &self.base, 0.0, None, legend_on, 0.0);
        let mut focus = None;
        if legend_on {
            let entries: Vec<LegendEntry> = self
                .series
                .iter()
                .enumerate()
                .map(|(i, s)| LegendEntry { label: s.name.clone(), color: s.color.unwrap_or_else(|| palette(&t, i)) })
                .collect();
            let before = hidden.clone();
            focus = legend(ui, id, frame_guess.legend.unwrap(), &entries, &mut hidden);
            if before != hidden {
                set_hidden_state(ui, id, hidden.clone());
            }
        }
        if n == 0 || !ui.is_rect_visible(rect) {
            return resp;
        }
        let any_visible = hidden.iter().any(|h| !h);
        let (lo, hi) = self.y_range.unwrap_or_else(|| {
            data_range(
                self.series
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !any_visible || !hidden[*i])
                    .flat_map(|(_, s)| s.values.iter()),
                self.include_zero,
            )
        });
        let ticks = nice_ticks(lo, hi, tick_count(frame_guess.plot.height(), 46.0));
        let (dlo, dhi) = if self.y_range.is_some() { (lo, hi) } else { (ticks.min, ticks.max) };
        // animated values: [values..., visibility..., domain lo, domain hi]
        let mut targets: Vec<f32> = Vec::with_capacity(k * n + k + 2);
        for s in self.series {
            targets.extend((0..n).map(|i| s.values.get(i).copied().unwrap_or(0.0) as f32));
        }
        targets.extend(hidden.iter().map(|h| if *h { 0.0 } else { 1.0 }));
        targets.extend([dlo as f32, dhi as f32]);
        let cur = tween(ui, id, &targets);
        let (alo, ahi) = (cur[k * n + k] as f64, cur[k * n + k + 1] as f64);
        let fmt_tick = tick_fmt(&ticks);
        let y_w = y_label_width(&painter, &ticks, &fmt_tick);
        let fr = cat_frame(rect, &self.base, y_w, None, legend_on, 10.0);
        let plot = fr.plot;
        let grid_plot =
            Rect::from_min_max(pos2(plot.left() - 10.0, plot.top()), pos2(plot.right() + 10.0, plot.bottom()));
        y_grid(
            &painter,
            &t,
            grid_plot,
            &ticks,
            alo,
            ahi,
            self.base.grid,
            self.base.y_axis.then_some((fr.y_label_x, false)),
            &fmt_tick,
        );
        if self.base.x_axis {
            x_labels(&painter, &t, plot, &self.labels, n, false, fr.x_label_y);
        }

        let e = entry_time(ui, id, self.base.animate, 1.4);
        let reveal = ease(Easing::InOutCubic, e / 1.25);
        let clip =
            Rect::from_min_max(rect.min, pos2(grid_plot.left() + (grid_plot.width() + 12.0) * reveal, rect.bottom()));
        let lp = painter.with_clip_rect(clip.intersect(rect));
        let (hover, hx, halpha) =
            if self.base.tooltip { cat_hover(ui, id, &resp, plot, n, false) } else { (None, 0.0, 0.0) };
        if halpha > 0.01 {
            crosshair_v(&painter, &t, hx, plot.top(), plot.bottom(), halpha);
        }
        let mut pts_all = Vec::with_capacity(k);
        for (si, s) in self.series.iter().enumerate() {
            let vis = cur[k * n + si];
            let color = s.color.unwrap_or_else(|| palette(&t, si));
            let pts: Vec<egui::Pos2> =
                (0..n).map(|i| pos2(cat_x(plot, n, i, false), val_y(plot, alo, ahi, cur[si * n + i] as f64))).collect();
            pts_all.push((pts.clone(), color, vis));
            if vis < 0.01 {
                continue;
            }
            let dim = match focus {
                Some(f) if f != si => 0.25,
                _ => 1.0,
            };
            let a = vis * dim;
            let line = curve_points(&pts, self.curve);
            let style =
                self.series_styles.iter().rev().find(|(i, _)| *i == si).map(|(_, st)| *st).unwrap_or(self.style);
            if self.glow {
                line_glow(&lp, &line, self.line_width, color, a);
            }
            if self.gradient_stroke && style == LineStyle::Solid {
                let c2 = palette(&t, si + 1);
                glowkit_core::paint::gradient_polyline(&lp, &line, false, self.line_width + 0.4, |u| {
                    mix(color, c2, u).gamma_multiply(a)
                });
            } else {
                stroke_line(&lp, &line, self.line_width, color.gamma_multiply(a), style);
            }
            if self.dots {
                for p in &pts {
                    lp.circle_filled(*p, 4.6, t.card.gamma_multiply(a));
                    lp.circle_filled(*p, 3.2, color.gamma_multiply(a));
                }
            }
        }
        if let (Some(i), true) = (hover, halpha > 0.01) {
            let mut rows = Vec::new();
            for (si, s) in self.series.iter().enumerate() {
                let (pts, color, vis) = &pts_all[si];
                if *vis < 0.5 {
                    continue;
                }
                let y = pts[i].y;
                active_dot(&painter, &t, pos2(hx, y), *color, halpha * vis);
                rows.push(TipRow::new(
                    Some(*color),
                    s.name.clone(),
                    (self.base.format)(s.values.get(i).copied().unwrap_or(0.0)),
                ));
            }
            let title = self.labels.get(i).cloned().unwrap_or_default();
            let py = resp.hover_pos().map(|p| p.y).unwrap_or(plot.center().y);
            tooltip(ui, id, pos2(hx, py), &title, &rows, halpha);
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// AreaChart
// ---------------------------------------------------------------------------------------------

/// An area chart: lines with gradient fills underneath, optionally stacked or expanded to 100%.
///
/// Areas rise from the baseline on appear, the crosshair tooltip snaps to categories, and the
/// legend toggles series (stacks re-flow smoothly).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::{AreaChart, Series, Stacking};
/// let series = [Series::new("Desktop", [186.0, 305.0, 237.0]), Series::new("Mobile", [80.0, 200.0, 120.0])];
/// ui.add(AreaChart::new(&series).labels(&["Jan", "Feb", "Mar"]).stacking(Stacking::Stacked));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AreaChart<'a> {
    series: &'a [Series],
    labels: Vec<String>,
    base: Base,
    curve: Curve,
    stacking: Stacking,
    fill_top: f32,
    fill_bottom: f32,
    line: bool,
    line_width: f32,
    dots: bool,
    glow: bool,
    style: LineStyle,
}

impl<'a> AreaChart<'a> {
    /// An area chart of `series`.
    pub fn new(series: &'a [Series]) -> Self {
        Self {
            series,
            labels: Vec::new(),
            base: Base::new(260.0),
            curve: Curve::Monotone,
            stacking: Stacking::None,
            fill_top: 0.45,
            fill_bottom: 0.02,
            line: true,
            line_width: 2.0,
            dots: false,
            glow: false,
            style: LineStyle::Solid,
        }
    }
    base_setters!();
    axis_setters!();
    /// Category labels along the x axis.
    pub fn labels<S: AsRef<str>>(mut self, labels: &[S]) -> Self {
        self.labels = labels_vec(labels);
        self
    }
    /// Interpolation between points (default [`Curve::Monotone`]).
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }
    /// Overlapping, stacked or expanded (100%) areas.
    pub fn stacking(mut self, stacking: Stacking) -> Self {
        self.stacking = stacking;
        self
    }
    /// Fill opacity at the top and bottom of the gradient (default 0.45 → 0.02).
    pub fn fill_opacity(mut self, top: f32, bottom: f32) -> Self {
        self.fill_top = top;
        self.fill_bottom = bottom;
        self
    }
    /// Draw the line on top of each area (default `true`).
    pub fn line(mut self, on: bool) -> Self {
        self.line = on;
        self
    }
    /// Stroke width (default 2).
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }
    /// Dots at every data point.
    pub fn dots(mut self, on: bool) -> Self {
        self.dots = on;
        self
    }
    /// Neon glow under the lines.
    pub fn glow(mut self, on: bool) -> Self {
        self.glow = on;
        self
    }
    /// Stroke style of the lines.
    pub fn line_style(mut self, style: LineStyle) -> Self {
        self.style = style;
        self
    }
}

impl Widget for AreaChart<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp, id) = self.base.allocate(ui, Sense::hover());
        let k = self.series.len();
        let n = self.series.iter().map(|s| s.values.len()).max().unwrap_or(0);
        let mut hidden = hidden_state(ui, id, k);
        let painter = ui.painter_at(rect);
        let legend_on = self.base.legend && k > 0;
        let frame_guess = cat_frame(rect, &self.base, 0.0, None, legend_on, 0.0);
        let mut focus = None;
        if legend_on {
            let entries: Vec<LegendEntry> = self
                .series
                .iter()
                .enumerate()
                .map(|(i, s)| LegendEntry { label: s.name.clone(), color: s.color.unwrap_or_else(|| palette(&t, i)) })
                .collect();
            let before = hidden.clone();
            focus = legend(ui, id, frame_guess.legend.unwrap(), &entries, &mut hidden);
            if before != hidden {
                set_hidden_state(ui, id, hidden.clone());
            }
        }
        if n == 0 || !ui.is_rect_visible(rect) {
            return resp;
        }
        let raw: Vec<Vec<f64>> =
            self.series.iter().map(|s| (0..n).map(|i| s.values.get(i).copied().unwrap_or(0.0)).collect()).collect();
        let vis_target: Vec<f64> = hidden.iter().map(|h| if *h { 0.0 } else { 1.0 }).collect();
        let target_stack = stack(&raw, &vis_target, self.stacking);
        let (lo, hi) = if self.stacking == Stacking::Expanded {
            (0.0, 1.0)
        } else {
            data_range(
                target_stack
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !hidden[*i])
                    .flat_map(|(_, r)| r.iter().flat_map(|b| [b.0, b.1]))
                    .collect::<Vec<_>>()
                    .iter(),
                true,
            )
        };
        let ticks = nice_ticks(lo, hi, tick_count(frame_guess.plot.height(), 46.0));
        let mut targets: Vec<f32> = raw.iter().flat_map(|r| r.iter().map(|v| *v as f32)).collect();
        targets.extend(vis_target.iter().map(|v| *v as f32));
        targets.extend([ticks.min as f32, ticks.max as f32]);
        let cur = tween(ui, id, &targets);
        let (alo, ahi) = (cur[k * n + k] as f64, cur[k * n + k + 1] as f64);
        let e = entry_time(ui, id, self.base.animate, 1.3);
        let rise = ease(Easing::OutCubic, e / 1.1) as f64;
        let vals: Vec<Vec<f64>> = (0..k).map(|si| (0..n).map(|i| cur[si * n + i] as f64 * rise).collect()).collect();
        let vis: Vec<f64> = (0..k).map(|si| cur[k * n + si] as f64).collect();
        let bands = stack(&vals, &vis, self.stacking);

        let pct = self.stacking == Stacking::Expanded;
        let fmt_default = tick_fmt(&ticks);
        let fmt_tick = |v: f64| if pct { format_percent(v) } else { fmt_default(v) };
        let y_w = y_label_width(&painter, &ticks, &fmt_tick);
        let fr = cat_frame(rect, &self.base, y_w, None, legend_on, 0.0);
        let plot = fr.plot;
        y_grid(
            &painter,
            &t,
            plot,
            &ticks,
            alo,
            ahi,
            self.base.grid,
            self.base.y_axis.then_some((fr.y_label_x, false)),
            &fmt_tick,
        );
        if self.base.x_axis {
            x_labels(&painter, &t, plot, &self.labels, n, false, fr.x_label_y);
        }
        let (hover, hx, halpha) =
            if self.base.tooltip { cat_hover(ui, id, &resp, plot, n, false) } else { (None, 0.0, 0.0) };
        let lp = painter.with_clip_rect(plot.expand2(vec2(12.0, 8.0)).intersect(rect));
        let stacked = self.stacking != Stacking::None;
        let (top_a, bot_a) = if stacked {
            (self.fill_top.max(0.5), self.fill_bottom.max(0.12))
        } else {
            (self.fill_top, self.fill_bottom)
        };
        let mut uppers = Vec::with_capacity(k);
        for si in 0..k {
            let color = self.series[si].color.unwrap_or_else(|| palette(&t, si));
            let up: Vec<egui::Pos2> =
                (0..n).map(|i| pos2(cat_x(plot, n, i, false), val_y(plot, alo, ahi, bands[si][i].1))).collect();
            let lowp: Vec<egui::Pos2> =
                (0..n).map(|i| pos2(cat_x(plot, n, i, false), val_y(plot, alo, ahi, bands[si][i].0))).collect();
            uppers.push((up.clone(), color));
            let v = vis[si] as f32;
            if v < 0.01 {
                continue;
            }
            let dim = match focus {
                Some(f) if f != si => 0.3,
                _ => 1.0,
            };
            let a = if stacked { dim } else { v * dim };
            let cu = curve_points(&up, self.curve);
            let cl = curve_points(&lowp, self.curve);
            band_fill(&lp, &cu, &cl, plot, color, top_a * a, bot_a * a);
            if self.glow {
                line_glow(&lp, &cu, self.line_width, color, a);
            }
            if self.line {
                stroke_line(&lp, &cu, self.line_width, color.gamma_multiply(a), self.style);
            }
            if self.dots {
                for p in &up {
                    lp.circle_filled(*p, 4.6, t.card.gamma_multiply(a));
                    lp.circle_filled(*p, 3.2, color.gamma_multiply(a));
                }
            }
        }
        if halpha > 0.01 {
            crosshair_v(&painter, &t, hx, plot.top(), plot.bottom(), halpha);
        }
        if let (Some(i), true) = (hover, halpha > 0.01) {
            let mut rows = Vec::new();
            let total: f64 = (0..k).filter(|si| !hidden[*si]).map(|si| raw[si][i].abs()).sum();
            for si in 0..k {
                if hidden[si] {
                    continue;
                }
                let (up, color) = &uppers[si];
                active_dot(&painter, &t, pos2(hx, up[i].y), *color, halpha);
                let v = raw[si][i];
                let value = if pct {
                    format!(
                        "{}  ·  {}",
                        (self.base.format)(v),
                        format_percent(if total > 0.0 { v / total } else { 0.0 })
                    )
                } else {
                    (self.base.format)(v)
                };
                rows.push(TipRow::new(Some(*color), self.series[si].name.clone(), value));
            }
            if stacked && rows.len() > 1 {
                rows.push(TipRow::new(None, "Total", (self.base.format)(total)));
            }
            let title = self.labels.get(i).cloned().unwrap_or_default();
            let py = resp.hover_pos().map(|p| p.y).unwrap_or(plot.center().y);
            tooltip(ui, id, pos2(hx, py), &title, &rows, halpha);
        }
        resp
    }
}
