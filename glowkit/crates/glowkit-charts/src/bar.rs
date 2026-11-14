//! Bar charts (vertical / horizontal, grouped / stacked / expanded) and the bar + line combo chart.

use crate::cartesian::{active_dot, cat_frame, data_range, labels_vec};
use crate::common::*;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::anim::{ease, stagger, Easing};
use glowkit_core::color::with_alpha;
use glowkit_core::paint::{fill_convex_fn, linear_fn};
use glowkit_core::Theme;

/// Bar direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Orientation {
    /// Bars grow upward from the x axis (columns).
    #[default]
    Vertical,
    /// Bars grow rightward from the y axis.
    Horizontal,
}

/// How a bar is filled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BarFill {
    /// Solid color.
    #[default]
    Solid,
    /// Gradient from the bar's color at its end to a faded tint at its base.
    Gradient,
    /// Tinted fill with animated diagonal stripes.
    Striped,
}

/// Paint one bar. `end` is the direction the bar's value end points to (for rounding & gradient).
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_bar(
    painter: &egui::Painter,
    r: Rect,
    radius: [f32; 4],
    color: Color32,
    fill: BarFill,
    grad_angle: f32,
    phase: f32,
    t: &Theme,
) {
    if r.width() < 0.3 || r.height() < 0.3 {
        return;
    }
    let lim = (r.width().min(r.height()) / 2.0).max(0.0);
    let rr = radius.map(|x| x.min(lim));
    match fill {
        BarFill::Solid => {
            let cr = CornerRadius {
                nw: rr[0].round() as u8,
                ne: rr[1].round() as u8,
                se: rr[2].round() as u8,
                sw: rr[3].round() as u8,
            };
            painter.rect_filled(r, cr, color);
        }
        BarFill::Gradient => {
            let pts = rounded_points(r, rr, 6);
            let a = color.a() as f32 / 255.0;
            let stops = [color, with_alpha(color, 0.28 * a)];
            fill_convex_fn(painter, &pts, 3, linear_fn(r, grad_angle, &stops));
        }
        BarFill::Striped => {
            let pts = rounded_points(r, rr, 6);
            let a = color.a() as f32 / 255.0;
            let cr = CornerRadius {
                nw: rr[0].round() as u8,
                ne: rr[1].round() as u8,
                se: rr[2].round() as u8,
                sw: rr[3].round() as u8,
            };
            painter.rect_filled(r, cr, with_alpha(color, if t.dark { 0.2 } else { 0.16 } * a));
            stripes(painter, &pts, r, with_alpha(color, 0.75 * a), 7.0, 2.0, phase);
            painter.rect_stroke(r, cr, Stroke::new(1.0, with_alpha(color, 0.9 * a)), StrokeKind::Inside);
        }
    }
}

/// Ticks for a secondary axis whose grid lines coincide with `primary`'s: same number of
/// intervals, a nice-ish step large enough to cover `lo..hi`.
pub(crate) fn aligned_ticks(primary: &Ticks, lo: f64, hi: f64) -> Ticks {
    let intervals = primary.values.len().saturating_sub(1).max(1);
    // where zero sits on the primary axis (in intervals from the bottom)
    let zero_at =
        if primary.min < 0.0 && primary.max > 0.0 { (-primary.min / primary.step).round() as usize } else { 0 };
    let above = (intervals - zero_at).max(1) as f64;
    let below = zero_at as f64;
    let need = (hi.max(0.0) / above).max(if below > 0.0 { (-lo.min(0.0)) / below } else { 0.0 });
    let step = nice_number(need.max(1e-9), false);
    let min = if zero_at > 0 {
        -step * below
    } else if lo < 0.0 {
        (lo / step).floor() * step
    } else {
        0.0
    };
    let values: Vec<f64> = (0..=intervals).map(|k| min + k as f64 * step).collect();
    Ticks { min, max: *values.last().unwrap(), step, values }
}

/// Corner radii `[nw, ne, se, sw]` rounding the value end of a bar.
pub(crate) fn end_radius(vertical: bool, positive: bool, r: f32) -> [f32; 4] {
    match (vertical, positive) {
        (true, true) => [r, r, 0.0, 0.0],
        (true, false) => [0.0, 0.0, r, r],
        (false, true) => [0.0, r, r, 0.0],
        (false, false) => [r, 0.0, 0.0, r],
    }
}

// ---------------------------------------------------------------------------------------------
// BarChart
// ---------------------------------------------------------------------------------------------

/// A bar chart: vertical or horizontal, grouped, stacked or expanded to 100%, with negative
/// values, rounded value ends, optional value labels, gradient or animated striped fills.
///
/// Bars grow in with a stagger; hovering highlights the category (or dims the others) and shows
/// a tooltip; the legend toggles series and the remaining bars re-flow smoothly.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::{BarChart, Series, Stacking};
/// let series = [Series::new("Desktop", [186.0, 305.0, 237.0]), Series::new("Mobile", [80.0, 200.0, 120.0])];
/// ui.add(BarChart::new(&series).labels(&["Jan", "Feb", "Mar"]).stacking(Stacking::Stacked).radius(6.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BarChart<'a> {
    series: &'a [Series],
    labels: Vec<String>,
    base: Base,
    orientation: Orientation,
    stacking: Stacking,
    radius: f32,
    band_gap: f32,
    bar_gap: f32,
    max_bar: f32,
    value_labels: bool,
    highlight_hover: bool,
    fill: BarFill,
    color_by_category: bool,
}

impl<'a> BarChart<'a> {
    /// A bar chart of `series`.
    pub fn new(series: &'a [Series]) -> Self {
        Self {
            series,
            labels: Vec::new(),
            base: Base::new(260.0),
            orientation: Orientation::Vertical,
            stacking: Stacking::None,
            radius: 6.0,
            band_gap: 0.3,
            bar_gap: 4.0,
            max_bar: 64.0,
            value_labels: false,
            highlight_hover: false,
            fill: BarFill::Solid,
            color_by_category: false,
        }
    }
    base_setters!();
    axis_setters!();
    /// Category labels.
    pub fn labels<S: AsRef<str>>(mut self, labels: &[S]) -> Self {
        self.labels = labels_vec(labels);
        self
    }
    /// Vertical columns (default) or horizontal bars.
    pub fn orientation(mut self, o: Orientation) -> Self {
        self.orientation = o;
        self
    }
    /// Shortcut for `orientation(Orientation::Horizontal)`.
    pub fn horizontal(self) -> Self {
        self.orientation(Orientation::Horizontal)
    }
    /// Grouped (default), stacked or expanded.
    pub fn stacking(mut self, s: Stacking) -> Self {
        self.stacking = s;
        self
    }
    /// Corner radius of the bar ends (default 6).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r;
        self
    }
    /// Gap between categories as a fraction of the category width (default 0.3).
    pub fn band_gap(mut self, fraction: f32) -> Self {
        self.band_gap = fraction.clamp(0.0, 0.9);
        self
    }
    /// Gap between bars of the same group in points (default 4).
    pub fn bar_gap(mut self, gap: f32) -> Self {
        self.bar_gap = gap.max(0.0);
        self
    }
    /// Maximum thickness of a single bar (default 64).
    pub fn max_bar_width(mut self, w: f32) -> Self {
        self.max_bar = w;
        self
    }
    /// Print values at the end of each bar (totals for stacked bars).
    pub fn value_labels(mut self, on: bool) -> Self {
        self.value_labels = on;
        self
    }
    /// On hover, keep the hovered category bright and dim the rest.
    pub fn highlight_hover(mut self, on: bool) -> Self {
        self.highlight_hover = on;
        self
    }
    /// Solid, gradient or animated striped fill.
    pub fn fill(mut self, fill: BarFill) -> Self {
        self.fill = fill;
        self
    }
    /// Color a single series by category instead of by series (uses the palette per bar).
    pub fn color_by_category(mut self, on: bool) -> Self {
        self.color_by_category = on;
        self
    }
}

impl Widget for BarChart<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp, id) = self.base.allocate(ui, Sense::hover());
        let k = self.series.len();
        let n = self.series.iter().map(|s| s.values.len()).max().unwrap_or(0);
        let vertical = self.orientation == Orientation::Vertical;
        let mut hidden = hidden_state(ui, id, k);
        let painter = ui.painter_at(rect);
        let legend_on = self.base.legend && k > 1 && !self.color_by_category;
        let legend_rect = Rect::from_min_max(pos2(rect.left(), rect.bottom() - LEGEND_H), rect.max);
        let mut focus = None;
        if legend_on {
            let entries: Vec<LegendEntry> = self
                .series
                .iter()
                .enumerate()
                .map(|(i, s)| LegendEntry { label: s.name.clone(), color: s.color.unwrap_or_else(|| palette(&t, i)) })
                .collect();
            let before = hidden.clone();
            focus = legend(ui, id, legend_rect, &entries, &mut hidden);
            if before != hidden {
                set_hidden_state(ui, id, hidden.clone());
            }
        }
        if n == 0 || !ui.is_rect_visible(rect) {
            return resp;
        }
        let raw: Vec<Vec<f64>> =
            self.series.iter().map(|s| (0..n).map(|i| s.values.get(i).copied().unwrap_or(0.0)).collect()).collect();
        let vis_t: Vec<f64> = hidden.iter().map(|h| if *h { 0.0 } else { 1.0 }).collect();
        let tstack = stack(&raw, &vis_t, self.stacking);
        let (lo, hi) = if self.stacking == Stacking::Expanded {
            (0.0, 1.0)
        } else {
            let vals: Vec<f64> = tstack
                .iter()
                .enumerate()
                .filter(|(i, _)| !hidden[*i])
                .flat_map(|(_, r)| r.iter().flat_map(|b| [b.0, b.1]))
                .collect();
            data_range(vals.iter(), true)
        };
        let axis_len =
            if vertical { rect.height() - 40.0 - if legend_on { LEGEND_H } else { 0.0 } } else { rect.width() - 120.0 };
        let ticks = nice_ticks(lo, hi, tick_count(axis_len, if vertical { 46.0 } else { 80.0 }));
        let mut targets: Vec<f32> = raw.iter().flat_map(|r| r.iter().map(|v| *v as f32)).collect();
        targets.extend(vis_t.iter().map(|v| *v as f32));
        targets.extend([ticks.min as f32, ticks.max as f32]);
        let cur = tween(ui, id, &targets);
        let (alo, ahi) = (cur[k * n + k] as f64, cur[k * n + k + 1] as f64);
        let vis: Vec<f64> = (0..k).map(|s| cur[k * n + s] as f64).collect();

        let pct = self.stacking == Stacking::Expanded;
        let fmt_default = tick_fmt(&ticks);
        let fmt_tick = |v: f64| if pct { format_percent(v) } else { fmt_default(v) };
        let font = axis_font();

        // layout
        let plot = if vertical {
            let y_w = y_label_width(&painter, &ticks, &fmt_tick);
            let fr = cat_frame(rect, &self.base, y_w, None, legend_on, 0.0);
            y_grid(
                &painter,
                &t,
                fr.plot,
                &ticks,
                alo,
                ahi,
                self.base.grid,
                self.base.y_axis.then_some((fr.y_label_x, false)),
                &fmt_tick,
            );
            fr.plot
        } else {
            let cat_w = if self.base.y_axis {
                self.labels.iter().map(|l| text_width(&painter, l, &font)).fold(0.0, f32::max) + 14.0
            } else {
                4.0
            };
            let bottom = if self.base.x_axis { 24.0 } else { 6.0 } + if legend_on { LEGEND_H } else { 0.0 };
            let right = if self.value_labels { 44.0 } else { 16.0 };
            let plot = Rect::from_min_max(
                pos2(rect.left() + cat_w, rect.top() + 4.0),
                pos2(rect.right() - right, rect.bottom() - bottom),
            );
            let labels_y = self.base.x_axis.then_some(plot.bottom() + 8.0);
            if pct {
                // percent labels for expanded
                for &v in &ticks.values {
                    let x = crisp(&painter, val_x(plot, alo, ahi, v));
                    if self.base.grid {
                        painter.extend(egui::Shape::dashed_line(
                            &[pos2(x, plot.top()), pos2(x, plot.bottom())],
                            Stroke::new(1.0, grid_color(&t)),
                            4.0,
                            4.0,
                        ));
                    }
                    if let Some(y) = labels_y {
                        painter.text(
                            pos2(x, y),
                            Align2::CENTER_TOP,
                            format_percent(v),
                            font.clone(),
                            t.muted_foreground,
                        );
                    }
                }
            } else {
                x_grid(&painter, &t, plot, &ticks, alo, ahi, self.base.grid, labels_y);
            }
            plot
        };
        let cat_len = if vertical { plot.width() } else { plot.height() };
        let bw = cat_len / n as f32;
        let cat_start = |i: usize| if vertical { plot.left() + i as f32 * bw } else { plot.top() + i as f32 * bw };
        // category labels
        if vertical && self.base.x_axis {
            x_labels(&painter, &t, plot, &self.labels, n, true, plot.bottom() + 8.0);
        } else if !vertical && self.base.y_axis {
            let stride = ((18.0 / bw).ceil() as usize).max(1);
            for (i, l) in self.labels.iter().enumerate().take(n) {
                if i % stride == 0 {
                    painter.text(
                        pos2(plot.left() - 10.0, cat_start(i) + bw / 2.0),
                        Align2::RIGHT_CENTER,
                        l,
                        font.clone(),
                        t.muted_foreground,
                    );
                }
            }
        }

        // hover
        let hover_pos = resp.hover_pos().filter(|p| plot.contains(*p));
        let hover = hover_pos.map(|p| {
            let c = if vertical { p.x - plot.left() } else { p.y - plot.top() };
            ((c / bw).floor().max(0.0) as usize).min(n - 1)
        });
        let halpha =
            glowkit_core::anim::follow(ui, id.with("hover_alpha"), if hover.is_some() { 1.0 } else { 0.0 }, 16.0);
        let last: usize = glowkit_core::state(ui, id.with("hover_last"), || 0usize);
        if let Some(h) = hover {
            glowkit_core::set_state(ui, id.with("hover_last"), h);
        }
        let shown = hover.unwrap_or(last);
        let hpos = follow_snap(ui, id.with("hover_pos"), cat_start(shown), 26.0, halpha < 0.05);
        if halpha > 0.01 && !self.highlight_hover && self.base.tooltip {
            let cursor = if vertical {
                Rect::from_min_size(pos2(hpos + bw * 0.06, plot.top()), vec2(bw * 0.88, plot.height()))
            } else {
                Rect::from_min_size(pos2(plot.left(), hpos + bw * 0.06), vec2(plot.width(), bw * 0.88))
            };
            painter.rect_filled(
                cursor,
                CornerRadius::same(6),
                t.muted.gamma_multiply(halpha * if t.dark { 0.9 } else { 0.8 }),
            );
        }

        // bars
        let e = entry_time(ui, id, self.base.animate, 0.7 + 0.05 * n as f32 + 0.2);
        let vis_sum: f64 = vis.iter().sum::<f64>().max(1e-6);
        let grouped = self.stacking == Stacking::None;
        let group_w = if grouped {
            (bw * (1.0 - self.band_gap))
                .min(self.max_bar * vis_sum as f32 + self.bar_gap * (vis_sum as f32 - 1.0).max(0.0))
        } else {
            (bw * (1.0 - self.band_gap)).min(self.max_bar)
        };
        let phase = if self.fill == BarFill::Striped {
            glowkit_core::animate(ui);
            glowkit_core::time(ui) * 1.2
        } else {
            0.0
        };
        let to_rect = |c0: f32, c1: f32, v0: f64, v1: f64| -> Rect {
            if vertical {
                let (y0, y1) = (val_y(plot, alo, ahi, v0), val_y(plot, alo, ahi, v1));
                Rect::from_min_max(pos2(c0, y0.min(y1)), pos2(c1, y0.max(y1)))
            } else {
                let (x0, x1) = (val_x(plot, alo, ahi, v0), val_x(plot, alo, ahi, v1));
                Rect::from_min_max(pos2(x0.min(x1), c0), pos2(x0.max(x1), c1))
            }
        };
        let tip_id = id;
        let mut tip_anchor: Option<Pos2> = None;
        for i in 0..n {
            let grow = if self.base.animate { ease(Easing::OutCubic, stagger(e, i, 0.05, 0.7)) as f64 } else { 1.0 };
            let vals: Vec<Vec<f64>> = (0..k).map(|s| vec![cur[s * n + i] as f64 * grow]).collect();
            let bands = stack(&vals, &vis, self.stacking);
            let g0 = cat_start(i) + (bw - group_w) / 2.0;
            // outermost visible segment in each direction (for rounding)
            let mut top_pos = None;
            let mut top_neg = None;
            for s in 0..k {
                if vis[s] > 0.01 {
                    if bands[s][0].1 >= bands[s][0].0 && vals[s][0] != 0.0 {
                        top_pos = Some(s);
                    } else if vals[s][0] != 0.0 {
                        top_neg = Some(s);
                    }
                }
            }
            let dim_cat = if self.highlight_hover && halpha > 0.01 && hover.is_some() && hover != Some(i) {
                1.0 - 0.7 * halpha
            } else {
                1.0
            };
            let mut offset = 0.0f32;
            let mut end_val: f64 = 0.0;
            for s in 0..k {
                let v = vis[s] as f32;
                let slot = if grouped { group_w / vis_sum as f32 * v } else { group_w };
                let (c0, c1) = if grouped {
                    let a = g0 + offset + self.bar_gap * 0.5 * v;
                    let b = g0 + offset + slot - self.bar_gap * 0.5 * v;
                    offset += slot;
                    if vis_sum <= 1.0 + 1e-3 {
                        (g0 + offset - slot, g0 + offset)
                    } else {
                        (a, b)
                    }
                } else {
                    (g0, g0 + group_w)
                };
                if v < 0.01 || c1 - c0 < 0.5 {
                    continue;
                }
                let (lo_v, hi_v) = bands[s][0];
                end_val = if hi_v.abs() > end_val.abs() { hi_v } else { end_val };
                let positive = hi_v >= lo_v;
                let r = to_rect(c0, c1, lo_v, hi_v);
                let outer = if grouped {
                    true
                } else if positive {
                    top_pos == Some(s)
                } else {
                    top_neg == Some(s)
                };
                let mut radius = if outer { end_radius(vertical, positive, self.radius) } else { [0.0; 4] };
                if !grouped && !positive && top_neg == Some(s) {
                    radius = end_radius(vertical, false, self.radius);
                }
                let mut color = if self.color_by_category {
                    palette(&t, i)
                } else {
                    self.series[s].color.unwrap_or_else(|| palette(&t, s))
                };
                let focus_dim = match focus {
                    Some(f) if f != s => 0.3,
                    _ => 1.0,
                };
                let a = (if grouped { v } else { 1.0 }) * dim_cat * focus_dim;
                color = color.gamma_multiply(a);
                let angle = match (vertical, positive) {
                    (true, true) => 90.0,
                    (true, false) => 270.0,
                    (false, true) => 180.0,
                    (false, false) => 0.0,
                };
                paint_bar(&painter, r, radius, color, self.fill, angle, phase, &t);
                if self.value_labels && grouped && grow > 0.98 {
                    let txt = (self.base.format)(raw[s][i]);
                    let (pos, align) = if vertical {
                        if positive {
                            (r.center_top() - vec2(0.0, 4.0), Align2::CENTER_BOTTOM)
                        } else {
                            (r.center_bottom() + vec2(0.0, 4.0), Align2::CENTER_TOP)
                        }
                    } else if positive {
                        (r.right_center() + vec2(6.0, 0.0), Align2::LEFT_CENTER)
                    } else {
                        (r.left_center() - vec2(6.0, 0.0), Align2::RIGHT_CENTER)
                    };
                    painter.text(pos, align, txt, axis_font(), t.foreground.gamma_multiply(0.85 * a.max(0.3)));
                }
                if hover == Some(i) {
                    let p = if vertical { pos2((c0 + c1) / 2.0, r.top()) } else { pos2(r.right(), (c0 + c1) / 2.0) };
                    tip_anchor = Some(match tip_anchor {
                        Some(q) if vertical && q.y < p.y => q,
                        Some(q) if !vertical && q.x > p.x => q,
                        _ => p,
                    });
                }
            }
            if self.value_labels && !grouped && !pct && grow > 0.98 {
                let total: f64 = (0..k).filter(|s| !hidden[*s]).map(|s| raw[s][i]).sum();
                let p = if vertical {
                    pos2(cat_start(i) + bw / 2.0, val_y(plot, alo, ahi, end_val) - 4.0)
                } else {
                    pos2(val_x(plot, alo, ahi, end_val) + 6.0, cat_start(i) + bw / 2.0)
                };
                let align = if vertical { Align2::CENTER_BOTTOM } else { Align2::LEFT_CENTER };
                painter.text(
                    p,
                    align,
                    (self.base.format)(total),
                    axis_font(),
                    t.foreground.gamma_multiply(0.85 * dim_cat),
                );
            }
        }
        if let (Some(i), true) = (hover, self.base.tooltip) {
            let mut rows = Vec::new();
            let total: f64 = (0..k).filter(|s| !hidden[*s]).map(|s| raw[s][i].abs()).sum();
            for s in 0..k {
                if hidden[s] {
                    continue;
                }
                let color = if self.color_by_category {
                    palette(&t, i)
                } else {
                    self.series[s].color.unwrap_or_else(|| palette(&t, s))
                };
                let v = raw[s][i];
                let value = if pct {
                    format!(
                        "{}  ·  {}",
                        (self.base.format)(v),
                        format_percent(if total > 0.0 { v.abs() / total } else { 0.0 })
                    )
                } else {
                    (self.base.format)(v)
                };
                rows.push(TipRow::new(Some(color), self.series[s].name.clone(), value));
            }
            if !grouped && rows.len() > 1 && !pct {
                rows.push(TipRow::new(
                    None,
                    "Total",
                    (self.base.format)((0..k).filter(|s| !hidden[*s]).map(|s| raw[s][i]).sum()),
                ));
            }
            let title = self.labels.get(i).cloned().unwrap_or_default();
            let anchor = resp.hover_pos().unwrap_or_else(|| tip_anchor.unwrap_or(plot.center()));
            tooltip(ui, tip_id, anchor, &title, &rows, halpha);
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// ComboChart
// ---------------------------------------------------------------------------------------------

/// Bars and lines on the same categories: e.g. revenue as columns with a margin or growth line.
/// With [`ComboChart::dual_axis`], the lines get their own scale on the right.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::{ComboChart, Series};
/// let bars = [Series::new("Revenue", [12.0, 15.0, 14.0, 18.0])];
/// let lines = [Series::new("Margin %", [22.0, 24.0, 21.0, 27.0])];
/// ui.add(ComboChart::new(&bars, &lines).labels(&["Q1", "Q2", "Q3", "Q4"]).dual_axis(true));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ComboChart<'a> {
    bars: &'a [Series],
    lines: &'a [Series],
    labels: Vec<String>,
    base: Base,
    dual: bool,
    line_format: Option<Formatter>,
    radius: f32,
    curve: Curve,
    fill: BarFill,
}

impl<'a> ComboChart<'a> {
    /// Bars from `bars`, lines from `lines`, sharing the same categories.
    pub fn new(bars: &'a [Series], lines: &'a [Series]) -> Self {
        Self {
            bars,
            lines,
            labels: Vec::new(),
            base: Base::new(280.0),
            dual: false,
            line_format: None,
            radius: 5.0,
            curve: Curve::Monotone,
            fill: BarFill::Solid,
        }
    }
    base_setters!();
    axis_setters!();
    /// Category labels.
    pub fn labels<S: AsRef<str>>(mut self, labels: &[S]) -> Self {
        self.labels = labels_vec(labels);
        self
    }
    /// Give the lines their own y axis on the right.
    pub fn dual_axis(mut self, on: bool) -> Self {
        self.dual = on;
        self
    }
    /// Formatter for line values (defaults to the chart formatter).
    pub fn line_format(mut self, f: Formatter) -> Self {
        self.line_format = Some(f);
        self
    }
    /// Bar corner radius (default 5).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r;
        self
    }
    /// Line interpolation.
    pub fn curve(mut self, c: Curve) -> Self {
        self.curve = c;
        self
    }
    /// Bar fill style.
    pub fn fill(mut self, fill: BarFill) -> Self {
        self.fill = fill;
        self
    }
}

impl Widget for ComboChart<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp, id) = self.base.allocate(ui, Sense::hover());
        let kb = self.bars.len();
        let kl = self.lines.len();
        let k = kb + kl;
        let n = self.bars.iter().chain(self.lines).map(|s| s.values.len()).max().unwrap_or(0);
        let mut hidden = hidden_state(ui, id, k);
        let painter = ui.painter_at(rect);
        let color_of = |s: usize| -> Color32 {
            if s < kb {
                self.bars[s].color.unwrap_or_else(|| palette(&t, s))
            } else {
                self.lines[s - kb].color.unwrap_or_else(|| palette(&t, s))
            }
        };
        let legend_on = self.base.legend && k > 0;
        let mut focus = None;
        if legend_on {
            let entries: Vec<LegendEntry> = self
                .bars
                .iter()
                .chain(self.lines)
                .enumerate()
                .map(|(i, s)| LegendEntry { label: s.name.clone(), color: color_of(i) })
                .collect();
            let before = hidden.clone();
            focus = legend(
                ui,
                id,
                Rect::from_min_max(pos2(rect.left(), rect.bottom() - LEGEND_H), rect.max),
                &entries,
                &mut hidden,
            );
            if before != hidden {
                set_hidden_state(ui, id, hidden.clone());
            }
        }
        if n == 0 || !ui.is_rect_visible(rect) {
            return resp;
        }
        let get = |s: &Series, i: usize| s.values.get(i).copied().unwrap_or(0.0);
        let bar_vals = self.bars.iter().enumerate().filter(|(i, _)| !hidden[*i]).flat_map(|(_, s)| s.values.iter());
        let line_vals =
            self.lines.iter().enumerate().filter(|(i, _)| !hidden[kb + *i]).flat_map(|(_, s)| s.values.iter());
        let tc = tick_count(rect.height() - 60.0, 46.0);
        let (ticks, ticks2) = if self.dual {
            let (a, b) = data_range(bar_vals, true);
            let (c, d) = data_range(line_vals, true);
            let left = nice_ticks(a, b, tc);
            (left.clone(), Some(aligned_ticks(&left, c, d)))
        } else {
            let (a, b) = data_range(bar_vals.chain(line_vals), true);
            (nice_ticks(a, b, tc), None)
        };
        let mut targets: Vec<f32> = Vec::new();
        for s in self.bars.iter().chain(self.lines) {
            targets.extend((0..n).map(|i| get(s, i) as f32));
        }
        targets.extend(hidden.iter().map(|h| if *h { 0.0 } else { 1.0 }));
        let t2 = ticks2.clone().unwrap_or(ticks.clone());
        targets.extend([ticks.min as f32, ticks.max as f32, t2.min as f32, t2.max as f32]);
        let cur = tween(ui, id, &targets);
        let o = k * n + k;
        let (alo, ahi, blo, bhi) = (cur[o] as f64, cur[o + 1] as f64, cur[o + 2] as f64, cur[o + 3] as f64);
        let vis: Vec<f32> = (0..k).map(|s| cur[k * n + s]).collect();
        let f1 = tick_fmt(&ticks);
        let f2 = tick_fmt(&t2);
        let y_w = y_label_width(&painter, &ticks, &f1);
        let y2_w = ticks2.as_ref().map(|t2| y_label_width(&painter, t2, &f2));
        let fr = cat_frame(rect, &self.base, y_w, y2_w, legend_on, 0.0);
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
            &f1,
        );
        if let Some(t2) = &ticks2 {
            y_grid(&painter, &t, plot, t2, blo, bhi, false, self.base.y_axis.then_some((fr.y2_label_x, true)), &f2);
        }
        if self.base.x_axis {
            x_labels(&painter, &t, plot, &self.labels, n, true, fr.x_label_y);
        }
        let (hover, hx, halpha) = if self.base.tooltip {
            crate::cartesian::cat_hover(ui, id, &resp, plot, n, true)
        } else {
            (None, 0.0, 0.0)
        };
        let bw = plot.width() / n as f32;
        if halpha > 0.01 {
            let cursor = Rect::from_center_size(pos2(hx, plot.center().y), vec2(bw * 0.88, plot.height()));
            painter.rect_filled(cursor, CornerRadius::same(6), t.muted.gamma_multiply(halpha * 0.85));
        }
        let e = entry_time(ui, id, self.base.animate, 1.6);
        let vb: f32 = vis[..kb].iter().sum::<f32>().max(1e-3);
        let group_w = (bw * 0.6).min(48.0 * vb);
        for i in 0..n {
            let grow = if self.base.animate { ease(Easing::OutCubic, stagger(e, i, 0.05, 0.7)) as f64 } else { 1.0 };
            let g0 = plot.left() + i as f32 * bw + (bw - group_w) / 2.0;
            let mut off = 0.0;
            for s in 0..kb {
                let slot = group_w / vb * vis[s];
                let (c0, c1) = (g0 + off + 2.0 * vis[s], g0 + off + slot - 2.0 * vis[s]);
                off += slot;
                if vis[s] < 0.01 {
                    continue;
                }
                let v = cur[s * n + i] as f64 * grow;
                let (y0, y1) = (val_y(plot, alo, ahi, 0.0), val_y(plot, alo, ahi, v));
                let r = Rect::from_min_max(pos2(c0, y0.min(y1)), pos2(c1, y0.max(y1)));
                let dim = match focus {
                    Some(f) if f != s => 0.3,
                    _ => 1.0,
                };
                paint_bar(
                    &painter,
                    r,
                    end_radius(true, v >= 0.0, self.radius),
                    color_of(s).gamma_multiply(vis[s] * dim),
                    self.fill,
                    if v >= 0.0 { 90.0 } else { 270.0 },
                    0.0,
                    &t,
                );
            }
        }
        let reveal = ease(Easing::InOutCubic, (e - 0.35) / 1.1);
        let lp = painter.with_clip_rect(
            Rect::from_min_max(rect.min, pos2(plot.left() + (plot.width() + 8.0) * reveal, rect.bottom()))
                .intersect(rect),
        );
        let (llo, lhi) = if self.dual { (blo, bhi) } else { (alo, ahi) };
        for l in 0..kl {
            let s = kb + l;
            if vis[s] < 0.01 {
                continue;
            }
            let dim = match focus {
                Some(f) if f != s => 0.3,
                _ => 1.0,
            };
            let c = color_of(s).gamma_multiply(vis[s] * dim);
            let pts: Vec<Pos2> =
                (0..n).map(|i| pos2(cat_x(plot, n, i, true), val_y(plot, llo, lhi, cur[s * n + i] as f64))).collect();
            let line = curve_points(&pts, self.curve);
            line_glow(&lp, &line, 2.0, c, 0.6);
            lp.add(egui::Shape::line(line, Stroke::new(2.5, c)));
            for p in &pts {
                lp.circle_filled(*p, 4.8, t.card.gamma_multiply(vis[s]));
                lp.circle_filled(*p, 3.2, c);
            }
        }
        if let (Some(i), true) = (hover, halpha > 0.01) {
            let mut rows = Vec::new();
            for s in 0..k {
                if hidden[s] {
                    continue;
                }
                let (series, fmt) = if s < kb {
                    (&self.bars[s], self.base.format)
                } else {
                    (&self.lines[s - kb], self.line_format.unwrap_or(self.base.format))
                };
                if s >= kb {
                    let y = val_y(plot, llo, lhi, cur[s * n + i] as f64);
                    active_dot(&painter, &t, pos2(hx, y), color_of(s), halpha);
                }
                rows.push(TipRow::new(Some(color_of(s)), series.name.clone(), fmt(get(series, i))));
            }
            let title = self.labels.get(i).cloned().unwrap_or_default();
            let py = resp.hover_pos().map(|p| p.y).unwrap_or(plot.center().y);
            tooltip(ui, id, pos2(hx, py), &title, &rows, halpha);
        }
        resp
    }
}
