//! Numeric x/y charts: scatter & bubble, and candlesticks.

use crate::cartesian::{cat_frame, labels_vec};
use crate::common::*;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Widget};
use glowkit_core::anim::{ease, stagger, Easing};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// A series of `(x, y)` points for [`ScatterChart`], optionally with a size per point (bubbles).
#[derive(Clone, Debug, PartialEq)]
pub struct ScatterSeries {
    /// Name (legend, tooltip).
    pub name: String,
    /// Points as `[x, y]`.
    pub points: Vec<[f64; 2]>,
    /// Optional size per point (bubble charts).
    pub sizes: Option<Vec<f64>>,
    /// Optional label per point (tooltip title).
    pub labels: Option<Vec<String>>,
    /// Color override.
    pub color: Option<Color32>,
}

impl ScatterSeries {
    /// A series of points.
    pub fn new(name: impl Into<String>, points: impl Into<Vec<[f64; 2]>>) -> Self {
        Self { name: name.into(), points: points.into(), sizes: None, labels: None, color: None }
    }
    /// Size per point; turns the scatter into a bubble chart.
    pub fn sizes(mut self, sizes: impl Into<Vec<f64>>) -> Self {
        self.sizes = Some(sizes.into());
        self
    }
    /// Label per point (shown as the tooltip title).
    pub fn labels<S: AsRef<str>>(mut self, labels: &[S]) -> Self {
        self.labels = Some(labels_vec(labels));
        self
    }
    /// Override the color.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

/// A scatter or bubble chart over numeric x and y axes.
///
/// Points pop in with a stagger; the nearest point to the pointer is ringed and described in a
/// tooltip; the legend toggles series.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::{ScatterChart, ScatterSeries};
/// let s = [ScatterSeries::new("Deals", vec![[12.0, 3.4], [20.0, 5.1], [31.0, 4.2]]).sizes(vec![4.0, 9.0, 6.0])];
/// ui.add(ScatterChart::new(&s).x_title("Sales cycle (days)").y_title("Deal size ($k)"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ScatterChart<'a> {
    series: &'a [ScatterSeries],
    base: Base,
    x_title: String,
    y_title: String,
    radius: (f32, f32),
    x_format: Formatter,
}

impl<'a> ScatterChart<'a> {
    /// A scatter chart of `series`.
    pub fn new(series: &'a [ScatterSeries]) -> Self {
        Self {
            series,
            base: Base::new(300.0),
            x_title: String::new(),
            y_title: String::new(),
            radius: (4.0, 22.0),
            x_format: format_compact,
        }
    }
    base_setters!();
    axis_setters!();
    /// Title under the x axis.
    pub fn x_title(mut self, t: impl Into<String>) -> Self {
        self.x_title = t.into();
        self
    }
    /// Title for the y axis (shown above it).
    pub fn y_title(mut self, t: impl Into<String>) -> Self {
        self.y_title = t.into();
        self
    }
    /// Point radius, or the min/max bubble radius when sizes are given (default 4..22).
    pub fn radius(mut self, min: f32, max: f32) -> Self {
        self.radius = (min, max);
        self
    }
    /// Formatter for x values in the tooltip.
    pub fn x_format(mut self, f: Formatter) -> Self {
        self.x_format = f;
        self
    }
}

impl Widget for ScatterChart<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp, id) = self.base.allocate(ui, Sense::hover());
        let k = self.series.len();
        let mut hidden = hidden_state(ui, id, k);
        let painter = ui.painter_at(rect);
        let color_of = |i: usize| self.series[i].color.unwrap_or_else(|| palette(&t, i));
        let legend_on = self.base.legend && k > 1;
        let mut focus = None;
        if legend_on {
            let entries: Vec<LegendEntry> = self
                .series
                .iter()
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
        let pts = || self.series.iter().flat_map(|s| s.points.iter());
        if pts().next().is_none() || !ui.is_rect_visible(rect) {
            return resp;
        }
        let (xlo, xhi) = pts().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| (a.min(p[0]), b.max(p[0])));
        let (ylo, yhi) = pts().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| (a.min(p[1]), b.max(p[1])));
        let smax =
            self.series.iter().filter_map(|s| s.sizes.as_ref()).flatten().fold(0.0f64, |m, v| m.max(*v)).max(1e-9);
        let title_h = if self.y_title.is_empty() { 0.0 } else { 18.0 };
        let xt_h = if self.x_title.is_empty() { 0.0 } else { 18.0 };
        let xt = nice_ticks(xlo.min(0.0).max(xlo - (xhi - xlo) * 0.1), xhi, tick_count(rect.width() - 80.0, 90.0));
        let yt = nice_ticks(ylo.min(0.0).max(ylo - (yhi - ylo) * 0.1), yhi, tick_count(rect.height() - 70.0, 46.0));
        let fy = tick_fmt(&yt);
        let y_w = y_label_width(&painter, &yt, &fy);
        let mut base = self.base.clone();
        base.legend = legend_on;
        let inner =
            Rect::from_min_max(pos2(rect.left(), rect.top() + title_h), pos2(rect.right(), rect.bottom() - xt_h));
        let fr = cat_frame(inner, &base, y_w, None, legend_on, 0.0);
        let plot = Rect::from_min_max(fr.plot.min + vec2(0.0, 4.0), fr.plot.max - vec2(12.0, 0.0));
        y_grid(
            &painter,
            &t,
            plot,
            &yt,
            yt.min,
            yt.max,
            self.base.grid,
            self.base.y_axis.then_some((fr.y_label_x, false)),
            &fy,
        );
        x_grid(
            &painter,
            &t,
            plot,
            &xt,
            xt.min,
            xt.max,
            self.base.grid,
            self.base.x_axis.then_some(plot.bottom() + 8.0),
        );
        if !self.y_title.is_empty() {
            painter.text(
                pos2(rect.left(), rect.top() + 2.0),
                Align2::LEFT_TOP,
                &self.y_title,
                FontId::proportional(11.5),
                t.muted_foreground,
            );
        }
        if !self.x_title.is_empty() {
            painter.text(
                pos2(plot.center().x, plot.bottom() + 26.0),
                Align2::CENTER_TOP,
                &self.x_title,
                FontId::proportional(11.5),
                t.muted_foreground,
            );
        }
        let vis_t: Vec<f32> = hidden.iter().map(|h| if *h { 0.0 } else { 1.0 }).collect();
        let vis = tween(ui, id, &vis_t);
        let e = entry_time(ui, id, self.base.animate, 1.6);
        let total_pts: usize = self.series.iter().map(|s| s.points.len()).sum();
        let hover_p = resp.hover_pos().filter(|p| plot.expand(10.0).contains(*p));
        let mut best: Option<(usize, usize, f32, Pos2, f32)> = None;
        let mut idx = 0;
        for (s, ser) in self.series.iter().enumerate() {
            let c = color_of(s);
            let dim = match focus {
                Some(f) if f != s => 0.25,
                _ => 1.0,
            };
            for (j, p) in ser.points.iter().enumerate() {
                let q = pos2(val_x(plot, xt.min, xt.max, p[0]), val_y(plot, yt.min, yt.max, p[1]));
                let r = match &ser.sizes {
                    Some(sz) => {
                        let v = sz.get(j).copied().unwrap_or(0.0).max(0.0) / smax;
                        self.radius.0 + (self.radius.1 - self.radius.0) * v.sqrt() as f32
                    }
                    None => self.radius.0,
                };
                let g = ease(Easing::OutBack, stagger(e, idx, 0.9 / total_pts.max(1) as f32, 0.5)) * vis[s];
                idx += 1;
                if g <= 0.01 {
                    continue;
                }
                if !hidden[s] {
                    if let Some(hp) = hover_p {
                        let d = hp.distance(q) - r;
                        if d < 16.0 && best.map(|b| d < b.2).unwrap_or(true) {
                            best = Some((s, j, d, q, r));
                        }
                    }
                }
                let bubble = ser.sizes.is_some();
                let a = dim * vis[s];
                painter.circle_filled(q, r * g, with_alpha(c, if bubble { 0.42 } else { 0.85 } * a));
                painter.circle_stroke(q, r * g, Stroke::new(if bubble { 1.5 } else { 1.0 }, c.gamma_multiply(a)));
            }
        }
        let halpha =
            glowkit_core::anim::follow(ui, id.with("hover_alpha"), if best.is_some() { 1.0 } else { 0.0 }, 18.0);
        if let Some((s, j, _, q, r)) = best {
            let c = color_of(s);
            painter.circle_stroke(q, r + 4.0, Stroke::new(2.0, c.gamma_multiply(halpha)));
            crosshair_v(&painter, &t, q.x, q.y + r + 4.0, plot.bottom(), halpha * 0.8);
            let ser = &self.series[s];
            let p = ser.points[j];
            let mut rows = vec![
                TipRow::new(Some(c), "x", (self.x_format)(p[0])),
                TipRow::new(None, "y", (self.base.format)(p[1])),
            ];
            if !self.x_title.is_empty() {
                rows[0].label = self.x_title.clone();
            }
            if !self.y_title.is_empty() {
                rows[1].label = self.y_title.clone();
            }
            if let Some(sz) = ser.sizes.as_ref().and_then(|v| v.get(j)) {
                rows.push(TipRow::new(None, "Size", format_compact(*sz)));
            }
            let title = ser.labels.as_ref().and_then(|l| l.get(j)).cloned().unwrap_or_else(|| ser.name.clone());
            tooltip(ui, id, q + vec2(r, 0.0), &title, &rows, halpha);
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// CandlestickChart
// ---------------------------------------------------------------------------------------------

/// One OHLC candle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candle {
    /// Opening price.
    pub open: f64,
    /// Highest price.
    pub high: f64,
    /// Lowest price.
    pub low: f64,
    /// Closing price.
    pub close: f64,
}

impl Candle {
    /// A candle from open, high, low, close.
    pub fn new(open: f64, high: f64, low: f64, close: f64) -> Self {
        Self { open, high, low, close }
    }
}

/// A candlestick (OHLC) chart: green rising and red falling candles with wicks, a crosshair with
/// a price tag on the axis, and an OHLC tooltip. Candles grow out of their midpoints on appear.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::{CandlestickChart, Candle};
/// let c = [Candle::new(100.0, 108.0, 97.0, 105.0), Candle::new(105.0, 110.0, 101.0, 102.0)];
/// ui.add(CandlestickChart::new(&c).labels(&["Mon", "Tue"]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CandlestickChart<'a> {
    candles: &'a [Candle],
    labels: Vec<String>,
    base: Base,
    up: Option<Color32>,
    down: Option<Color32>,
    hollow_up: bool,
}

impl<'a> CandlestickChart<'a> {
    /// A candlestick chart of `candles`.
    pub fn new(candles: &'a [Candle]) -> Self {
        let mut base = Base::new(300.0);
        base.format = |v| format_thousands(v, 2);
        Self { candles, labels: Vec::new(), base, up: None, down: None, hollow_up: false }
    }
    base_setters!();
    axis_setters!();
    /// Category labels (dates).
    pub fn labels<S: AsRef<str>>(mut self, labels: &[S]) -> Self {
        self.labels = labels_vec(labels);
        self
    }
    /// Rising candle color (default: success).
    pub fn up_color(mut self, c: Color32) -> Self {
        self.up = Some(c);
        self
    }
    /// Falling candle color (default: destructive).
    pub fn down_color(mut self, c: Color32) -> Self {
        self.down = Some(c);
        self
    }
    /// Draw rising candles hollow (outline only).
    pub fn hollow_up(mut self, on: bool) -> Self {
        self.hollow_up = on;
        self
    }
}

impl Widget for CandlestickChart<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp, id) = self.base.allocate(ui, Sense::hover());
        let n = self.candles.len();
        if n == 0 || !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter_at(rect);
        let (lo, hi) =
            self.candles.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), c| (a.min(c.low), b.max(c.high)));
        let ticks = nice_ticks(lo, hi, tick_count(rect.height() - 40.0, 44.0));
        let ft = tick_fmt(&ticks);
        let mut base = self.base.clone();
        base.legend = false;
        let y_w = y_label_width(&painter, &ticks, &ft);
        // price axis on the right, like trading terminals
        let fr = cat_frame(
            rect,
            &{
                let mut b = base.clone();
                b.y_axis = false;
                b
            },
            0.0,
            Some(y_w.max(54.0)),
            false,
            0.0,
        );
        let plot = fr.plot;
        let targets = [ticks.min as f32, ticks.max as f32];
        let dom = tween(ui, id, &targets);
        let (dlo, dhi) = (dom[0] as f64, dom[1] as f64);
        y_grid(
            &painter,
            &t,
            plot,
            &ticks,
            dlo,
            dhi,
            self.base.grid,
            self.base.y_axis.then_some((fr.y2_label_x, true)),
            &ft,
        );
        if self.base.x_axis {
            x_labels(&painter, &t, plot, &self.labels, n, true, fr.x_label_y);
        }
        let up_c = self.up.unwrap_or(t.success);
        let down_c = self.down.unwrap_or(t.destructive);
        let bw = plot.width() / n as f32;
        let body_w = (bw * 0.62).clamp(1.0, 18.0);
        let e = entry_time(ui, id, self.base.animate, 0.6 + n as f32 * 0.02 + 0.5);
        let (hover, hx, halpha) = if self.base.tooltip {
            crate::cartesian::cat_hover(ui, id, &resp, plot, n, true)
        } else {
            (None, 0.0, 0.0)
        };
        if halpha > 0.01 {
            crosshair_v(&painter, &t, hx, plot.top(), plot.bottom(), halpha);
        }
        for (i, c) in self.candles.iter().enumerate() {
            let g = ease(Easing::OutCubic, stagger(e, i, 0.6 / n as f32, 0.5));
            if g <= 0.0 {
                continue;
            }
            let up = c.close >= c.open;
            let col = if up { up_c } else { down_c };
            let dim = if hover.is_some() && hover != Some(i) { 1.0 - 0.35 * halpha } else { 1.0 };
            let col = col.gamma_multiply(dim);
            let x = crisp(&painter, cat_x(plot, n, i, true));
            let mid = (c.open + c.close) / 2.0;
            let s = |v: f64| val_y(plot, dlo, dhi, mid + (v - mid) * g as f64);
            painter.line_segment([pos2(x, s(c.high)), pos2(x, s(c.low))], Stroke::new(1.0, col));
            let (y0, y1) = (s(c.open), s(c.close));
            let body = Rect::from_min_max(
                pos2(x - body_w / 2.0, y0.min(y1)),
                pos2(x + body_w / 2.0, y0.max(y1).max(y0.min(y1) + 1.0)),
            );
            let cr = CornerRadius::same((body_w * 0.15).min(2.0) as u8);
            if up && self.hollow_up {
                painter.rect_filled(body, cr, t.card);
                painter.rect_stroke(body, cr, Stroke::new(1.0, col), egui::StrokeKind::Inside);
            } else {
                painter.rect_filled(body, cr, col);
            }
        }
        if let Some(hp) = resp.hover_pos().filter(|p| plot.contains(*p)) {
            // horizontal crosshair with a price tag on the axis
            let y = crisp(&painter, hp.y);
            painter.extend(Shape::dashed_line(
                &[pos2(plot.left(), y), pos2(plot.right(), y)],
                Stroke::new(1.0, t.muted_foreground.gamma_multiply(0.5)),
                4.0,
                3.0,
            ));
            let price = dlo + (plot.bottom() - hp.y) as f64 / plot.height() as f64 * (dhi - dlo);
            let txt = ft(price);
            let font = axis_font();
            let w = text_width(&painter, &txt, &font) + 10.0;
            let tag = Rect::from_min_size(pos2(plot.right() + 4.0, y - 9.0), vec2(w, 18.0));
            painter.rect_filled(tag, CornerRadius::same(4), t.foreground);
            painter.text(tag.center(), Align2::CENTER_CENTER, txt, font, t.background);
        }
        if let (Some(i), true) = (hover, halpha > 0.01) {
            let c = self.candles[i];
            let col = if c.close >= c.open { up_c } else { down_c };
            let chg = (c.close - c.open) / c.open.abs().max(1e-9);
            let f = self.base.format;
            let rows = [
                TipRow::new(None, "Open", f(c.open)),
                TipRow::new(None, "High", f(c.high)),
                TipRow::new(None, "Low", f(c.low)),
                TipRow::new(None, "Close", f(c.close)),
                TipRow::new(
                    Some(col),
                    "Change",
                    format!("{}{}", if chg >= 0.0 { "+" } else { "" }, format_percent(chg)),
                ),
            ];
            let py = resp.hover_pos().map(|p| p.y).unwrap_or(plot.center().y);
            tooltip(ui, id, pos2(hx, py), self.labels.get(i).map(|s| s.as_str()).unwrap_or(""), &rows, halpha);
        }
        resp
    }
}
