//! Shared chart building blocks: data types, nice axis ticks, number formatting, stacking,
//! animation state, legends, tooltips, grids and a few painting helpers.

use egui::epaint::Mesh;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Id, LayerId, Order, Painter, Pos2, Rect, Response, Sense, Shape,
    Stroke, StrokeKind, Ui, Vec2,
};
use glowkit_core::anim::Spring;
use glowkit_core::color::{shade, with_alpha};
use glowkit_core::Theme;
use std::f32::consts::TAU;

// ---------------------------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------------------------

/// A named series of values, one per category (x position).
///
/// ```no_run
/// use glowkit_charts::Series;
/// let s = Series::new("Desktop", [186.0, 305.0, 237.0]).color(egui::Color32::LIGHT_BLUE);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Series {
    /// Display name (legend and tooltip).
    pub name: String,
    /// One value per category.
    pub values: Vec<f64>,
    /// Color override; defaults to the theme's chart palette.
    pub color: Option<Color32>,
}

impl Series {
    /// A new series. Its color comes from the theme palette unless set with [`Series::color`].
    pub fn new(name: impl Into<String>, values: impl Into<Vec<f64>>) -> Self {
        Self { name: name.into(), values: values.into(), color: None }
    }
    /// Override the series color.
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }
}

/// A single labelled value: a pie slice, a funnel stage, a bar-list row, a treemap tile...
///
/// ```no_run
/// use glowkit_charts::Datum;
/// let d = Datum::new("Chrome", 275.0);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Datum {
    /// Label.
    pub label: String,
    /// Value (most charts expect it to be non-negative).
    pub value: f64,
    /// Color override; defaults to the theme's chart palette.
    pub color: Option<Color32>,
}

impl Datum {
    /// A new labelled value.
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self { label: label.into(), value, color: None }
    }
    /// Override the color.
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }
}

/// Formats a value for labels and tooltips. Non-capturing closures coerce to it:
/// `.format(|v| format!("${}", format_compact(v)))`.
pub type Formatter = fn(f64) -> String;

/// How the stroke of a line is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LineStyle {
    /// Continuous line.
    #[default]
    Solid,
    /// Dashes.
    Dashed,
    /// Round dots.
    Dotted,
}

/// How a line passes through its data points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Curve {
    /// Straight segments.
    Linear,
    /// Smooth monotone cubic (never overshoots the data). The default.
    #[default]
    Monotone,
    /// Catmull-Rom spline: rounder, may overshoot slightly.
    Natural,
    /// Horizontal steps (value holds until the next point).
    Step,
}

/// How multiple series share the value axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Stacking {
    /// Side by side (bars) or overlapping (areas).
    #[default]
    None,
    /// Stacked on top of each other.
    Stacked,
    /// Stacked and normalized so every category sums to 100%.
    Expanded,
}

/// Palette color `i` for the theme, shading the palette for indices beyond its length.
pub fn palette(t: &Theme, i: usize) -> Color32 {
    let base = t.chart[i % t.chart.len()];
    let round = (i / t.chart.len()) as f32;
    if round == 0.0 {
        base
    } else {
        shade(base, if t.dark { -0.12 } else { 0.12 } * round)
    }
}

// ---------------------------------------------------------------------------------------------
// Ticks & formatting
// ---------------------------------------------------------------------------------------------

/// Axis ticks produced by [`nice_ticks`].
#[derive(Clone, Debug, PartialEq)]
pub struct Ticks {
    /// Lowest tick (the axis minimum).
    pub min: f64,
    /// Highest tick (the axis maximum).
    pub max: f64,
    /// Distance between ticks.
    pub step: f64,
    /// Every tick value, ascending.
    pub values: Vec<f64>,
}

/// Round `x` to a "nice" number: 1, 2, 5 or 10 times a power of ten.
///
/// With `round`, picks the closest nice number; otherwise the smallest nice number `>= x`.
pub fn nice_number(x: f64, round: bool) -> f64 {
    if !x.is_finite() || x <= 0.0 {
        return 1.0;
    }
    let exp = x.log10().floor();
    let p = 10f64.powf(exp);
    // round away floating point noise (0.7 / 0.1 = 6.9999999)
    let f = (x / p * 1e9).round() / 1e9;
    let nf = if round {
        if f < 1.5 {
            1.0
        } else if f < 3.0 {
            2.0
        } else if f < 7.0 {
            5.0
        } else {
            10.0
        }
    } else if f <= 1.0 {
        1.0
    } else if f <= 2.0 {
        2.0
    } else if f <= 5.0 {
        5.0
    } else {
        10.0
    };
    nf * p
}

/// "Nice numbers" axis ticks covering `min..=max` with roughly `max_ticks` ticks.
///
/// The returned range is expanded outward to whole steps, and steps are 1, 2 or 5 × 10ⁿ.
///
/// ```
/// let t = glowkit_charts::nice_ticks(0.0, 4300.0, 5);
/// assert_eq!(t.values, vec![0.0, 1000.0, 2000.0, 3000.0, 4000.0, 5000.0]);
/// ```
pub fn nice_ticks(min: f64, max: f64, max_ticks: usize) -> Ticks {
    let (mut lo, mut hi) = if min <= max { (min, max) } else { (max, min) };
    if !lo.is_finite() || !hi.is_finite() {
        lo = 0.0;
        hi = 1.0;
    }
    if (hi - lo).abs() < 1e-12 {
        if lo == 0.0 {
            hi = 1.0;
        } else {
            let pad = lo.abs() * 0.1;
            lo -= pad;
            hi += pad;
        }
    }
    let n = max_ticks.max(2) as f64;
    let step = nice_number((hi - lo) / (n - 1.0), true);
    let start = (lo / step).floor() * step;
    let end = (hi / step).ceil() * step;
    let count = ((end - start) / step).round() as usize;
    let decimals = (-(step.log10().floor()) as i32 + 1).clamp(0, 15);
    let values = (0..=count)
        .map(|k| {
            let v = start + k as f64 * step;
            // kill floating point noise like 0.30000000000000004 and -0
            let v = round_to(v, decimals);
            if v.abs() < step * 1e-9 {
                0.0
            } else {
                v
            }
        })
        .collect::<Vec<_>>();
    Ticks { min: values[0], max: *values.last().unwrap(), step, values }
}

fn round_to(v: f64, decimals: i32) -> f64 {
    let m = 10f64.powi(decimals);
    (v * m).round() / m
}

fn trim_zeros(s: String) -> String {
    if s.contains('.') {
        let s = s.trim_end_matches('0').trim_end_matches('.');
        s.to_string()
    } else {
        s
    }
}

/// Short human number: `950`, `1.2k`, `15.3k`, `3.4M`, `2.1B`, `0.25`.
///
/// ```
/// assert_eq!(glowkit_charts::format_compact(1234.0), "1.2k");
/// assert_eq!(glowkit_charts::format_compact(3_400_000.0), "3.4M");
/// ```
pub fn format_compact(v: f64) -> String {
    if !v.is_finite() {
        return "–".into();
    }
    const UNITS: [(f64, &str); 5] = [(1.0, ""), (1e3, "k"), (1e6, "M"), (1e9, "B"), (1e12, "T")];
    let a = v.abs();
    let mut u = 0;
    while u + 1 < UNITS.len() && a >= UNITS[u + 1].0 {
        u += 1;
    }
    // rounding may push us to the next unit (999_950 -> "1000k" -> "1M")
    if u > 0 && u + 1 < UNITS.len() && (a / UNITS[u].0 * 10.0).round() / 10.0 >= 1000.0 {
        u += 1;
    }
    let (div, suf) = UNITS[u];
    let x = v / div;
    let s = if u == 0 {
        if a >= 100.0 || a.fract() == 0.0 {
            format!("{x:.0}")
        } else if a >= 10.0 {
            trim_zeros(format!("{x:.1}"))
        } else {
            trim_zeros(format!("{x:.2}"))
        }
    } else if x.abs() >= 100.0 {
        format!("{x:.0}")
    } else {
        trim_zeros(format!("{x:.1}"))
    };
    let s = if s == "-0" { "0".to_string() } else { s };
    format!("{s}{suf}")
}

/// Number with thousands separators and up to `decimals` decimals: `12,345.6`.
pub fn format_thousands(v: f64, decimals: usize) -> String {
    if !v.is_finite() {
        return "–".into();
    }
    let s = format!("{:.*}", decimals, v.abs());
    let (int, frac) = match s.split_once('.') {
        Some((a, b)) => (a.to_string(), Some(b.to_string())),
        None => (s, None),
    };
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if let Some(f) = frac {
        out.push('.');
        out.push_str(&f);
    }
    if v < 0.0 && out.chars().any(|c| c.is_ascii_digit() && c != '0') {
        out.insert(0, '-');
    }
    out
}

/// Format an axis tick so every label of an axis uses the same unit and enough decimals for
/// `step`: `0, 2k, 4k` or `0, 0.5, 1` or `0, 2.5M, 5M`.
pub fn format_tick(v: f64, step: f64, max_abs: f64) -> String {
    let unit = if max_abs >= 1e12 {
        (1e12, "T")
    } else if max_abs >= 1e9 {
        (1e9, "B")
    } else if max_abs >= 1e6 {
        (1e6, "M")
    } else if max_abs >= 1e4 {
        (1e3, "k")
    } else {
        (1.0, "")
    };
    let rel = (step / unit.0).abs();
    let decimals = if rel > 0.0 { (-(rel.log10() + 1e-9).floor()).clamp(0.0, 6.0) as usize } else { 0 };
    let x = v / unit.0;
    let s = format!("{:.*}", decimals, x);
    let s = if s.starts_with('-') && s.trim_start_matches('-').chars().all(|c| c == '0' || c == '.') {
        s.trim_start_matches('-').to_string()
    } else {
        s
    };
    if v == 0.0 {
        return "0".into();
    }
    format!("{s}{}", unit.1)
}

/// Percent with no decimals for big values and one for small: `42%`, `3.5%`.
pub fn format_percent(fraction: f64) -> String {
    let p = fraction * 100.0;
    if p.abs() >= 10.0 || p.fract().abs() < 0.05 {
        format!("{p:.0}%")
    } else {
        format!("{p:.1}%")
    }
}

// ---------------------------------------------------------------------------------------------
// Stacking
// ---------------------------------------------------------------------------------------------

/// Stack series values. Returns, per series and per category, the `(lower, upper)` band.
///
/// Positive values stack upward from zero and negative values downward. `weights` (0..1, one per
/// series) scale each series, which is how hidden series smoothly collapse. With
/// [`Stacking::Expanded`] every category is normalized so its positive total is `1.0`.
/// With [`Stacking::None`] each band is simply `(0, value)`.
pub fn stack(values: &[Vec<f64>], weights: &[f64], mode: Stacking) -> Vec<Vec<(f64, f64)>> {
    let n = values.iter().map(|v| v.len()).max().unwrap_or(0);
    let w = |k: usize| weights.get(k).copied().unwrap_or(1.0);
    let get = |k: usize, i: usize| values[k].get(i).copied().unwrap_or(0.0) * w(k);
    let mut out = vec![vec![(0.0, 0.0); n]; values.len()];
    if mode == Stacking::None {
        for (k, row) in out.iter_mut().enumerate() {
            for (i, b) in row.iter_mut().enumerate() {
                *b = (0.0, get(k, i));
            }
        }
        return out;
    }
    for i in 0..n {
        let (mut pos, mut neg) = (0.0, 0.0);
        let total: f64 = (0..values.len()).map(|k| get(k, i).abs()).sum();
        let scale = if mode == Stacking::Expanded && total > 0.0 { 1.0 / total } else { 1.0 };
        for (k, row) in out.iter_mut().enumerate() {
            let v = get(k, i) * scale;
            if v >= 0.0 {
                row[i] = (pos, pos + v);
                pos += v;
            } else {
                row[i] = (neg, neg + v);
                neg += v;
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Animation state
// ---------------------------------------------------------------------------------------------

#[derive(Clone)]
struct EntryState {
    start: f64,
    last_frame: u64,
}

/// Seconds since this chart (re)appeared. The clock restarts when the chart was not drawn for a
/// couple of frames (e.g. the user navigated away and back), so the entry animation replays.
/// Keeps requesting repaints for `duration` seconds and then stops.
pub(crate) fn entry_time(ui: &Ui, id: Id, enabled: bool, duration: f32) -> f32 {
    if !enabled {
        return 1e6;
    }
    let now = ui.input(|i| i.time);
    let frame = ui.ctx().cumulative_frame_nr();
    let start = glowkit_core::with_state(
        ui,
        id.with("glowkit_chart_entry"),
        || EntryState { start: now, last_frame: frame },
        |s| {
            if frame > s.last_frame + 2 {
                s.start = now;
            }
            s.last_frame = frame;
            s.start
        },
    );
    let e = (now - start) as f32;
    if e < duration {
        ui.ctx().request_repaint();
    }
    e
}

#[derive(Clone)]
struct TweenState {
    from: Vec<f32>,
    to: Vec<f32>,
    spring: Spring,
}

/// Animate a vector of values toward `targets` with a spring. Whenever the targets change, the
/// values glide from wherever they are now. The first call (or a length change) snaps.
pub(crate) fn tween(ui: &Ui, id: Id, targets: &[f32]) -> Vec<f32> {
    let dt = glowkit_core::dt(ui);
    let (out, settled) = glowkit_core::with_state(
        ui,
        id.with("glowkit_chart_tween"),
        || TweenState {
            from: targets.to_vec(),
            to: targets.to_vec(),
            spring: Spring { value: 1.0, target: 1.0, ..Spring::new(1.0) },
        },
        |s| {
            let cur = |s: &TweenState| -> Vec<f32> {
                s.from.iter().zip(&s.to).map(|(a, b)| a + (b - a) * s.spring.value).collect()
            };
            if s.to.len() != targets.len() {
                s.from = targets.to_vec();
                s.to = targets.to_vec();
                s.spring.value = 1.0;
                s.spring.velocity = 0.0;
            } else if s.to.iter().zip(targets).any(|(a, b)| (a - b).abs() > 1e-6 * (1.0 + b.abs())) {
                s.from = cur(s);
                s.to = targets.to_vec();
                s.spring =
                    Spring { value: 0.0, velocity: 0.0, target: 1.0, stiffness: 140.0, damping: 20.0, mass: 1.0 };
            }
            s.spring.step(dt);
            let settled = s.spring.is_settled();
            if settled {
                s.spring.value = 1.0;
            }
            (cur(s), settled)
        },
    );
    if !settled {
        ui.ctx().request_repaint();
    }
    out
}

/// Exponential follow that snaps instead of sliding when `snap` is true.
pub(crate) fn follow_snap(ui: &Ui, id: Id, target: f32, speed: f32, snap: bool) -> f32 {
    if snap {
        glowkit_core::set_state(ui, id.with("glowkit_follow"), target);
        return target;
    }
    glowkit_core::anim::follow(ui, id, target, speed)
}

/// Visibility toggles for a chart's series/slices, persisted per chart.
pub(crate) fn hidden_state(ui: &Ui, id: Id, n: usize) -> Vec<bool> {
    let mut h: Vec<bool> = glowkit_core::state(ui, id.with("glowkit_chart_hidden"), || vec![false; n]);
    h.resize(n, false);
    h
}

pub(crate) fn set_hidden_state(ui: &Ui, id: Id, h: Vec<bool>) {
    glowkit_core::set_state(ui, id.with("glowkit_chart_hidden"), h);
}

// ---------------------------------------------------------------------------------------------
// Common options
// ---------------------------------------------------------------------------------------------

/// Options shared by every chart (size, legend, grid, tooltip, animation, formatting).
#[derive(Clone)]
pub(crate) struct Base {
    pub width: Option<f32>,
    pub height: f32,
    pub legend: bool,
    pub grid: bool,
    pub tooltip: bool,
    pub animate: bool,
    pub format: Formatter,
    pub id_salt: Option<egui::IdSalt>,
    pub x_axis: bool,
    pub y_axis: bool,
}

impl Base {
    pub fn new(height: f32) -> Self {
        Self {
            width: None,
            height,
            legend: true,
            grid: true,
            tooltip: true,
            animate: true,
            format: format_compact,
            id_salt: None,
            x_axis: true,
            y_axis: true,
        }
    }

    /// Allocate space and resolve the chart's state id.
    pub fn allocate(&self, ui: &mut Ui, sense: Sense) -> (Rect, Response, Id) {
        let w = self.width.unwrap_or_else(|| ui.available_width()).max(40.0);
        let (rect, resp) = ui.allocate_exact_size(vec2(w, self.height), sense);
        let id = self.id_salt.map(|s| ui.scope_id().with_salt(s)).unwrap_or(resp.id);
        (rect, resp, id)
    }
}

/// Expands to the setters every chart shares. The struct must have a `base: Base` field.
macro_rules! base_setters {
    () => {
        /// Height in points.
        pub fn height(mut self, height: f32) -> Self {
            self.base.height = height;
            self
        }
        /// Width in points (defaults to all available width).
        pub fn width(mut self, width: f32) -> Self {
            self.base.width = Some(width);
            self
        }
        /// Show the legend (default `true`). Clicking a legend entry toggles it.
        pub fn legend(mut self, show: bool) -> Self {
            self.base.legend = show;
            self
        }
        /// Show grid lines (default `true`).
        pub fn grid(mut self, show: bool) -> Self {
            self.base.grid = show;
            self
        }
        /// Show the hover tooltip (default `true`).
        pub fn tooltip(mut self, show: bool) -> Self {
            self.base.tooltip = show;
            self
        }
        /// Play the entry animation (default `true`).
        pub fn animate(mut self, animate: bool) -> Self {
            self.base.animate = animate;
            self
        }
        /// Formatter for values in labels and tooltips (default [`format_compact`](crate::format_compact)).
        pub fn format(mut self, format: $crate::Formatter) -> Self {
            self.base.format = format;
            self
        }
        /// Use an explicit id for animation/legend state (needed when the widget moves around).
        pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
            self.base.id_salt = Some(egui::IdSalt::new(salt));
            self
        }
    };
}
pub(crate) use base_setters;

macro_rules! axis_setters {
    () => {
        /// Show the category (x) axis labels (default `true`).
        pub fn x_axis(mut self, show: bool) -> Self {
            self.base.x_axis = show;
            self
        }
        /// Show the value (y) axis labels (default `true`).
        pub fn y_axis(mut self, show: bool) -> Self {
            self.base.y_axis = show;
            self
        }
    };
}
pub(crate) use axis_setters;

// ---------------------------------------------------------------------------------------------
// Layout helpers
// ---------------------------------------------------------------------------------------------

/// Axis label font.
pub(crate) fn axis_font() -> FontId {
    FontId::proportional(11.0)
}

pub(crate) fn text_width(painter: &Painter, text: &str, font: &FontId) -> f32 {
    painter.layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE).size().x
}

/// Truncate `text` with an ellipsis so it fits in `max_w` points (empty if nothing fits).
pub(crate) fn truncate(painter: &Painter, text: &str, font: &FontId, max_w: f32) -> String {
    if text_width(painter, text, font) <= max_w {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    for n in (1..chars.len()).rev() {
        let s: String = chars[..n].iter().collect::<String>() + "…";
        if text_width(painter, &s, font) <= max_w {
            return s;
        }
    }
    String::new()
}

/// Snap a coordinate to a pixel center so 1px lines are crisp.
pub(crate) fn crisp(painter: &Painter, v: f32) -> f32 {
    painter.round_to_pixel_center(v)
}

/// Every how many category labels to show so none overlap.
pub(crate) fn label_stride(painter: &Painter, labels: &[String], slot: f32, font: &FontId) -> usize {
    if labels.is_empty() || slot <= 0.0 {
        return 1;
    }
    let maxw = labels.iter().map(|l| text_width(painter, l, font)).fold(0.0, f32::max) + 10.0;
    ((maxw / slot).ceil() as usize).max(1)
}

/// x of category `i` of `n`. Band layout centers in equal slots; point layout spans edge to edge.
pub(crate) fn cat_x(plot: Rect, n: usize, i: usize, band: bool) -> f32 {
    if band {
        plot.left() + (i as f32 + 0.5) * plot.width() / n.max(1) as f32
    } else if n <= 1 {
        plot.center().x
    } else {
        plot.left() + i as f32 * plot.width() / (n - 1) as f32
    }
}

/// Nearest category index to screen x.
pub(crate) fn cat_index(plot: Rect, n: usize, x: f32, band: bool) -> usize {
    if n == 0 {
        return 0;
    }
    let f = if band {
        ((x - plot.left()) / (plot.width() / n as f32)).floor()
    } else if n == 1 {
        0.0
    } else {
        ((x - plot.left()) / (plot.width() / (n - 1) as f32)).round()
    };
    (f.max(0.0) as usize).min(n - 1)
}

/// Map a value to screen y within `plot` for the domain `lo..hi`.
pub(crate) fn val_y(plot: Rect, lo: f64, hi: f64, v: f64) -> f32 {
    let t = if (hi - lo).abs() < 1e-12 { 0.0 } else { (v - lo) / (hi - lo) };
    plot.bottom() - t as f32 * plot.height()
}

/// Map a value to screen x within `plot` for the domain `lo..hi`.
pub(crate) fn val_x(plot: Rect, lo: f64, hi: f64, v: f64) -> f32 {
    let t = if (hi - lo).abs() < 1e-12 { 0.0 } else { (v - lo) / (hi - lo) };
    plot.left() + t as f32 * plot.width()
}

/// How many value ticks fit along an axis of `len` points.
pub(crate) fn tick_count(len: f32, spacing: f32) -> usize {
    ((len / spacing).floor() as usize).clamp(2, 8)
}

/// Height reserved for a one-row legend.
pub(crate) const LEGEND_H: f32 = 28.0;

/// Default tick label formatter for `ticks`: shared unit, enough decimals for the step.
pub(crate) fn tick_fmt(ticks: &Ticks) -> impl Fn(f64) -> String {
    let max_abs = ticks.values.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let step = ticks.step;
    move |v| format_tick(v, step, max_abs)
}

/// Draw horizontal value grid lines and their labels. `labels` is the x where labels end (or
/// start, when `labels_right`); `None` hides labels.
#[allow(clippy::too_many_arguments)]
pub(crate) fn y_grid(
    painter: &Painter,
    t: &Theme,
    plot: Rect,
    ticks: &Ticks,
    lo: f64,
    hi: f64,
    grid: bool,
    labels: Option<(f32, bool)>,
    fmt: &dyn Fn(f64) -> String,
) {
    let font = axis_font();
    for &v in &ticks.values {
        let y = val_y(plot, lo, hi, v);
        if y < plot.top() - 1.0 || y > plot.bottom() + 1.0 {
            continue;
        }
        let y = crisp(painter, y);
        if grid {
            if v == 0.0 && lo < 0.0 {
                painter.hline(plot.x_range(), y, Stroke::new(1.0, t.muted_foreground.gamma_multiply(0.45)));
            } else {
                painter.extend(Shape::dashed_line(
                    &[pos2(plot.left(), y), pos2(plot.right(), y)],
                    Stroke::new(1.0, grid_color(t)),
                    4.0,
                    4.0,
                ));
            }
        }
        if let Some((x, right)) = labels {
            let align = if right { Align2::LEFT_CENTER } else { Align2::RIGHT_CENTER };
            painter.text(pos2(x, y), align, fmt(v), font.clone(), t.muted_foreground);
        }
    }
}

/// Vertical value grid (for horizontal bar charts and scatter x axes).
#[allow(clippy::too_many_arguments)]
pub(crate) fn x_grid(
    painter: &Painter,
    t: &Theme,
    plot: Rect,
    ticks: &Ticks,
    lo: f64,
    hi: f64,
    grid: bool,
    labels_y: Option<f32>,
) {
    let font = axis_font();
    let max_abs = ticks.values.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let labels: Vec<String> = ticks.values.iter().map(|&v| format_tick(v, ticks.step, max_abs)).collect();
    let slot = plot.width() / ticks.values.len().max(1) as f32;
    let stride = label_stride(painter, &labels, slot, &font);
    for (k, &v) in ticks.values.iter().enumerate() {
        let x = val_x(plot, lo, hi, v);
        if x < plot.left() - 1.0 || x > plot.right() + 1.0 {
            continue;
        }
        let x = crisp(painter, x);
        if grid {
            let col = if v == 0.0 && lo < 0.0 { t.muted_foreground.gamma_multiply(0.45) } else { grid_color(t) };
            painter.extend(Shape::dashed_line(
                &[pos2(x, plot.top()), pos2(x, plot.bottom())],
                Stroke::new(1.0, col),
                4.0,
                4.0,
            ));
        }
        if let Some(y) = labels_y {
            if k % stride == 0 {
                painter.text(pos2(x, y), Align2::CENTER_TOP, &labels[k], font.clone(), t.muted_foreground);
            }
        }
    }
}

/// Opaque faint track color (for rings and progress tracks), so overlapping caps don't show.
pub(crate) fn track_color(t: &Theme) -> Color32 {
    glowkit_core::color::mix(t.card, t.muted_foreground, if t.dark { 0.13 } else { 0.16 })
}

/// Grid line color: the border token, a touch stronger in light themes.
pub(crate) fn grid_color(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.border, 0.03)
    } else {
        t.border
    }
}

/// Category labels along the bottom, thinned so they never overlap.
pub(crate) fn x_labels(painter: &Painter, t: &Theme, plot: Rect, labels: &[String], n: usize, band: bool, y: f32) {
    if labels.is_empty() || n == 0 {
        return;
    }
    let font = axis_font();
    let slot = if band || n <= 1 { plot.width() / n as f32 } else { plot.width() / (n - 1) as f32 };
    let stride = label_stride(painter, labels, slot, &font);
    for (i, l) in labels.iter().enumerate().take(n) {
        if i % stride != 0 {
            continue;
        }
        let mut x = cat_x(plot, n, i, band);
        // keep edge labels inside the chart
        let w = text_width(painter, l, &font);
        if !band {
            x = x.clamp(plot.left() - 6.0 + w / 2.0, plot.right() + 6.0 - w / 2.0);
        }
        painter.text(pos2(x, y), Align2::CENTER_TOP, l, font.clone(), t.muted_foreground);
    }
}

/// Width needed for the y-axis labels of `ticks`.
pub(crate) fn y_label_width(painter: &Painter, ticks: &Ticks, fmt: &dyn Fn(f64) -> String) -> f32 {
    let font = axis_font();
    ticks.values.iter().map(|&v| text_width(painter, &fmt(v), &font)).fold(0.0, f32::max)
}

// ---------------------------------------------------------------------------------------------
// Legend
// ---------------------------------------------------------------------------------------------

/// One legend entry.
pub(crate) struct LegendEntry {
    pub label: String,
    pub color: Color32,
}

/// Draw a centered, one-row, clickable legend in `area`. Clicking toggles `hidden`.
/// Returns the hovered entry, if any.
pub(crate) fn legend(ui: &mut Ui, id: Id, area: Rect, entries: &[LegendEntry], hidden: &mut [bool]) -> Option<usize> {
    let t = Theme::of(ui);
    let font = FontId::proportional(12.0);
    let painter = ui.painter_at(area.expand(4.0));
    let mut widths: Vec<f32> = entries.iter().map(|e| text_width(&painter, &e.label, &font)).collect();
    let item_w = |w: f32| 10.0 + 6.0 + w;
    let n = entries.len();
    let mut gap = 16.0;
    let sum =
        |widths: &[f32], gap: f32| widths.iter().map(|w| item_w(*w)).sum::<f32>() + gap * (n.saturating_sub(1)) as f32;
    // Too wide for the area: tighten the gaps, then truncate the longest labels with "…".
    let avail = area.width() - 8.0;
    if sum(&widths, gap) > avail {
        gap = 10.0;
    }
    let mut truncated = vec![false; n];
    if sum(&widths, gap) > avail && n > 0 {
        let fixed = n as f32 * 16.0 + gap * (n - 1) as f32;
        let mut budget = ((avail - fixed) / n as f32).max(18.0);
        // give the slack of short labels to the long ones
        let short: f32 = widths.iter().filter(|w| **w <= budget).sum();
        let long = widths.iter().filter(|w| **w > budget).count();
        if long > 0 {
            budget = ((avail - fixed - short) / long as f32).max(budget);
        }
        for (i, w) in widths.iter_mut().enumerate() {
            if *w > budget {
                *w = budget;
                truncated[i] = true;
            }
        }
    }
    let total = sum(&widths, gap);
    let mut x = area.center().x - total / 2.0;
    let cy = area.center().y;
    let mut hovered = None;
    for (i, e) in entries.iter().enumerate() {
        let w = item_w(widths[i]);
        let r = Rect::from_min_size(pos2(x - 4.0, cy - 10.0), vec2(w + 8.0, 20.0));
        let resp = ui.interact(r, id.with(("glowkit_legend", i)), Sense::click());
        let off = hidden.get(i).copied().unwrap_or(false);
        if resp.clicked() {
            if let Some(h) = hidden.get_mut(i) {
                *h = !*h;
            }
        }
        let hov = glowkit_core::anim::follow(
            ui,
            id.with(("glowkit_legend_hov", i)),
            if resp.hovered() { 1.0 } else { 0.0 },
            18.0,
        );
        if resp.hovered() {
            // a hidden entry has nothing to focus on
            if !hidden.get(i).copied().unwrap_or(false) {
                hovered = Some(i);
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if hov > 0.01 {
            painter.rect_filled(r, CornerRadius::same(6), t.muted.gamma_multiply(hov));
        }
        let dot = Rect::from_center_size(pos2(x + 5.0, cy), vec2(10.0, 10.0));
        if off {
            painter.rect_stroke(
                dot.shrink(0.5),
                CornerRadius::same(3),
                Stroke::new(1.5, e.color.gamma_multiply(0.6)),
                StrokeKind::Inside,
            );
        } else {
            painter.rect_filled(dot, CornerRadius::same(3), e.color);
        }
        let col = if off {
            t.muted_foreground.gamma_multiply(0.55)
        } else {
            t.muted_foreground.lerp_to_gamma(t.foreground, 0.35 + 0.65 * hov)
        };
        let tr = if truncated[i] {
            let mut job = egui::text::LayoutJob::simple_singleline(e.label.clone(), font.clone(), col);
            job.wrap.max_width = widths[i];
            job.wrap.max_rows = 1;
            job.wrap.break_anywhere = true;
            job.wrap.overflow_character = Some('…');
            let g = painter.layout_job(job);
            let r = Rect::from_min_size(pos2(x + 16.0, cy - g.size().y / 2.0), g.size());
            painter.galley(r.min, g, col);
            if resp.hovered() {
                resp.clone().on_hover_text(&e.label);
            }
            r
        } else {
            painter.text(pos2(x + 16.0, cy), Align2::LEFT_CENTER, &e.label, font.clone(), col)
        };
        if off {
            painter.hline(tr.x_range(), cy, Stroke::new(1.0, col));
        }
        x += w + gap;
    }
    hovered
}

// ---------------------------------------------------------------------------------------------
// Tooltip
// ---------------------------------------------------------------------------------------------

/// One row of a tooltip.
pub(crate) struct TipRow {
    pub color: Option<Color32>,
    pub label: String,
    pub value: String,
}

impl TipRow {
    pub fn new(color: Option<Color32>, label: impl Into<String>, value: impl Into<String>) -> Self {
        Self { color, label: label.into(), value: value.into() }
    }
}

/// Paint a tooltip card near `anchor` on the tooltip layer: rounded, bordered, soft shadow,
/// optional bold title, then rows with a color indicator, a muted label and a right-aligned value.
pub(crate) fn tooltip(ui: &Ui, id: Id, anchor: Pos2, title: &str, rows: &[TipRow], alpha: f32) {
    if alpha <= 0.01 {
        return;
    }
    let t = Theme::of(ui);
    let painter = ui.ctx().layer_painter(LayerId::new(Order::Tooltip, id.with("glowkit_tip")));
    let title_font = FontId::proportional(12.5);
    let font = FontId::proportional(12.0);
    let pad = vec2(11.0, 9.0);
    let row_h = 19.0;
    let title_g =
        (!title.is_empty()).then(|| painter.layout_no_wrap(title.to_owned(), title_font.clone(), t.foreground));
    let mut label_w: f32 = 0.0;
    let mut value_w: f32 = 0.0;
    for r in rows {
        label_w = label_w.max(text_width(&painter, &r.label, &font) + if r.color.is_some() { 16.0 } else { 0.0 });
        value_w = value_w.max(text_width(&painter, &r.value, &font));
    }
    let gap = if label_w > 0.0 && value_w > 0.0 { 20.0 } else { 0.0 };
    let title_h = title_g.as_ref().map(|g| g.size().y + if rows.is_empty() { 0.0 } else { 6.0 }).unwrap_or(0.0);
    let w = (label_w + gap + value_w).max(title_g.as_ref().map(|g| g.size().x).unwrap_or(0.0)).max(90.0) + pad.x * 2.0;
    let h = title_h + rows.len() as f32 * row_h + pad.y * 2.0 - if rows.is_empty() { 0.0 } else { 2.0 };
    let screen = ui.ctx().content_rect();
    let mut pos = pos2(anchor.x + 16.0, anchor.y - h / 2.0);
    if pos.x + w > screen.right() - 4.0 {
        pos.x = anchor.x - 16.0 - w;
    }
    pos.y = pos.y.clamp(screen.top() + 4.0, (screen.bottom() - h - 4.0).max(screen.top() + 4.0));
    pos.x = pos.x.max(screen.left() + 4.0);
    let rect = Rect::from_min_size(pos, vec2(w, h));
    let a = alpha.clamp(0.0, 1.0);
    let shadow = if t.dark { Color32::from_black_alpha(140) } else { Color32::from_black_alpha(36) };
    glowkit_core::paint::soft_shadow(
        &painter,
        rect.translate(vec2(0.0, 4.0)),
        8.0,
        18.0,
        Vec2::ZERO,
        shadow.gamma_multiply(a),
    );
    painter.rect_filled(rect, CornerRadius::same(8), t.popover.gamma_multiply(a));
    painter.rect_stroke(rect, CornerRadius::same(8), Stroke::new(1.0, t.border.gamma_multiply(a)), StrokeKind::Inside);
    let mut y = rect.top() + pad.y;
    if let Some(g) = title_g {
        let gh = g.size().y;
        painter.galley(pos2(rect.left() + pad.x, y), g, t.foreground.gamma_multiply(a));
        y += gh + 6.0;
    }
    for r in rows {
        let cy = y + row_h / 2.0 - 1.0;
        let mut x = rect.left() + pad.x;
        if let Some(c) = r.color {
            painter.rect_filled(
                Rect::from_center_size(pos2(x + 4.5, cy), vec2(9.0, 9.0)),
                CornerRadius::same(2),
                c.gamma_multiply(a),
            );
            x += 16.0;
        }
        painter.text(pos2(x, cy), Align2::LEFT_CENTER, &r.label, font.clone(), t.muted_foreground.gamma_multiply(a));
        painter.text(
            pos2(rect.right() - pad.x, cy),
            Align2::RIGHT_CENTER,
            &r.value,
            font.clone(),
            t.foreground.gamma_multiply(a),
        );
        y += row_h;
    }
}

/// Dashed vertical crosshair.
pub(crate) fn crosshair_v(painter: &Painter, t: &Theme, x: f32, top: f32, bottom: f32, alpha: f32) {
    let x = crisp(painter, x);
    painter.extend(Shape::dashed_line(
        &[pos2(x, top), pos2(x, bottom)],
        Stroke::new(1.0, t.muted_foreground.gamma_multiply(0.55 * alpha)),
        4.0,
        3.0,
    ));
}

// ---------------------------------------------------------------------------------------------
// Painting helpers
// ---------------------------------------------------------------------------------------------

/// Apply a [`Curve`] to screen points.
pub(crate) fn curve_points(pts: &[Pos2], curve: Curve) -> Vec<Pos2> {
    match curve {
        Curve::Linear => pts.to_vec(),
        Curve::Monotone => glowkit_core::paint::monotone(pts, 14),
        Curve::Natural => glowkit_core::paint::catmull_rom(pts, 14),
        Curve::Step => {
            let mut out = Vec::with_capacity(pts.len() * 2);
            for (i, p) in pts.iter().enumerate() {
                if i > 0 {
                    out.push(pos2((pts[i - 1].x + p.x) / 2.0, pts[i - 1].y));
                    out.push(pos2((pts[i - 1].x + p.x) / 2.0, p.y));
                }
                out.push(*p);
            }
            out
        }
    }
}

/// Fill between two point lists of equal length (same x's) with a vertical gradient: alpha
/// `top_a` at the top of `plot` fading to `bottom_a` at the bottom.
pub(crate) fn band_fill(
    painter: &Painter,
    upper: &[Pos2],
    lower: &[Pos2],
    plot: Rect,
    color: Color32,
    top_a: f32,
    bottom_a: f32,
) {
    let n = upper.len().min(lower.len());
    if n < 2 {
        return;
    }
    let col = |y: f32| {
        let k = ((y - plot.top()) / plot.height().max(1.0)).clamp(0.0, 1.0);
        with_alpha(color, top_a + (bottom_a - top_a) * k)
    };
    let mut m = Mesh::default();
    m.reserve_vertices(n * 2);
    for i in 0..n {
        m.colored_vertex(upper[i], col(upper[i].y));
        m.colored_vertex(lower[i], col(lower[i].y));
    }
    for i in 0..n as u32 - 1 {
        let a = i * 2;
        m.add_triangle(a, a + 2, a + 3);
        m.add_triangle(a, a + 3, a + 1);
    }
    painter.add(Shape::mesh(m));
}

/// Stroke a polyline in one of the [`LineStyle`]s.
pub(crate) fn stroke_line(painter: &Painter, pts: &[Pos2], width: f32, color: Color32, style: LineStyle) {
    if pts.len() < 2 {
        return;
    }
    match style {
        LineStyle::Solid => {
            painter.add(Shape::line(pts.to_vec(), Stroke::new(width, color)));
        }
        LineStyle::Dashed => painter.extend(Shape::dashed_line(pts, Stroke::new(width, color), 7.0, 5.0)),
        LineStyle::Dotted => painter.extend(Shape::dotted_line(pts, color, width * 2.6, width * 0.62)),
    }
}

/// A soft neon glow under a line: wide, low-alpha passes.
pub(crate) fn line_glow(painter: &Painter, pts: &[Pos2], width: f32, color: Color32, strength: f32) {
    for (w, a) in [(width + 14.0, 0.05), (width + 8.0, 0.08), (width + 3.5, 0.16)] {
        glowkit_core::paint::gradient_polyline(painter, pts, false, w, |_| with_alpha(color, a * strength));
    }
}

/// Anti-aliased annular sector (donut slice). Angles in radians, 0 = 3 o'clock, clockwise.
pub(crate) fn sector(painter: &Painter, c: Pos2, r_in: f32, r_out: f32, a0: f32, a1: f32, color: Color32) {
    if a1 - a0 <= 1e-4 || r_out <= r_in {
        return;
    }
    let f = 0.7; // feather in points
    let steps = (((a1 - a0) / TAU) * 180.0).ceil().max(2.0) as usize;
    let inner = r_in > 0.5;
    let mut m = Mesh::default();
    let clear = Color32::TRANSPARENT;
    // columns of 4 vertices: outer feather, outer, inner, inner feather
    let push_col = |m: &mut Mesh, a: f32, shift: Vec2, solid: bool| {
        let d = vec2(a.cos(), a.sin());
        let cc = if solid { color } else { clear };
        m.colored_vertex(c + d * (r_out + f) + shift, clear);
        m.colored_vertex(c + d * r_out + shift, cc);
        m.colored_vertex(c + d * if inner { r_in } else { 0.0 } + shift, cc);
        m.colored_vertex(c + d * if inner { (r_in - f).max(0.0) } else { 0.0 } + shift, clear);
    };
    let t0 = vec2(a0.cos(), a0.sin()).rot90() * -f;
    let t1 = vec2(a1.cos(), a1.sin()).rot90() * f;
    push_col(&mut m, a0, t0, false);
    for i in 0..=steps {
        let a = a0 + (a1 - a0) * i as f32 / steps as f32;
        push_col(&mut m, a, Vec2::ZERO, true);
    }
    push_col(&mut m, a1, t1, false);
    let cols = steps + 3;
    for k in 0..cols as u32 - 1 {
        let a = k * 4;
        let b = (k + 1) * 4;
        for s in 0..3 {
            if !inner && s == 2 {
                continue;
            }
            m.add_triangle(a + s, b + s, b + s + 1);
            m.add_triangle(a + s, b + s + 1, a + s + 1);
        }
    }
    painter.add(Shape::mesh(m));
}

/// Rounded rect outline with independent corner radii (clockwise from top-left).
pub(crate) fn rounded_points(rect: Rect, cr: [f32; 4], segs: usize) -> Vec<Pos2> {
    let lim = (rect.width().min(rect.height()) / 2.0).max(0.0);
    let [nw, ne, se, sw] = cr.map(|r| r.clamp(0.0, lim));
    let mut pts = Vec::with_capacity(segs * 4 + 8);
    let mut corner = |c: Pos2, r: f32, start: f32| {
        if r < 0.5 {
            pts.push(c);
            return;
        }
        for i in 0..=segs {
            let a = (start + 0.25 * i as f32 / segs as f32) * TAU;
            pts.push(c + vec2(a.cos(), a.sin()) * r);
        }
    };
    corner(pos2(rect.left() + nw, rect.top() + nw), nw, 0.5);
    corner(pos2(rect.right() - ne, rect.top() + ne), ne, 0.75);
    corner(pos2(rect.right() - se, rect.bottom() - se), se, 0.0);
    corner(pos2(rect.left() + sw, rect.bottom() - sw), sw, 0.25);
    pts
}

/// Clip segment `a..b` to a convex polygon (Cyrus–Beck).
pub(crate) fn clip_segment_convex(poly: &[Pos2], a: Pos2, b: Pos2) -> Option<(Pos2, Pos2)> {
    let n = poly.len();
    if n < 3 {
        return None;
    }
    // orientation
    let area: f32 = (0..n)
        .map(|i| {
            let (p, q) = (poly[i], poly[(i + 1) % n]);
            p.x * q.y - q.x * p.y
        })
        .sum();
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let d = b - a;
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let e = q - p;
        // inward normal
        let nrm = vec2(-e.y, e.x) * sign;
        let num = nrm.dot(a - p);
        let den = nrm.dot(d);
        if den.abs() < 1e-9 {
            if num < 0.0 {
                return None;
            }
        } else {
            let t = -num / den;
            if den > 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
            if t0 > t1 {
                return None;
            }
        }
    }
    Some((a + d * t0, a + d * t1))
}

/// Animated diagonal stripes inside a convex outline (pattern fill).
pub(crate) fn stripes(
    painter: &Painter,
    outline: &[Pos2],
    bounds: Rect,
    color: Color32,
    spacing: f32,
    width: f32,
    phase: f32,
) {
    let span = bounds.width() + bounds.height();
    let off = (phase * spacing).rem_euclid(spacing);
    let mut s = -bounds.height() - spacing + off;
    while s < bounds.width() + spacing {
        let a = pos2(bounds.left() + s, bounds.bottom());
        let b = a + vec2(span, -span);
        if let Some((p, q)) = clip_segment_convex(outline, a, b) {
            painter.line_segment([p, q], Stroke::new(width, color));
        }
        s += spacing;
    }
}

/// Small filled triangle pointing up or down (delta badges).
pub(crate) fn triangle(painter: &Painter, c: Pos2, size: f32, up: bool, color: Color32) {
    let h = size * 0.85;
    let pts = if up {
        vec![pos2(c.x, c.y - h / 2.0), pos2(c.x + size / 2.0, c.y + h / 2.0), pos2(c.x - size / 2.0, c.y + h / 2.0)]
    } else {
        vec![pos2(c.x - size / 2.0, c.y - h / 2.0), pos2(c.x + size / 2.0, c.y - h / 2.0), pos2(c.x, c.y + h / 2.0)]
    };
    painter.add(Shape::convex_polygon(pts, color, Stroke::NONE));
}

// ---------------------------------------------------------------------------------------------
// Squarified treemap
// ---------------------------------------------------------------------------------------------

/// Squarified treemap layout: splits `rect` into one rect per value (in input order) with areas
/// proportional to the values, keeping tiles as square as possible.
///
/// ```
/// let r = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 100.0));
/// let tiles = glowkit_charts::squarify(&[50.0, 25.0, 25.0], r);
/// assert!((tiles[0].area() - 5000.0).abs() < 1.0);
/// ```
pub fn squarify(values: &[f64], rect: Rect) -> Vec<Rect> {
    let mut out = vec![Rect::from_min_size(rect.min, Vec2::ZERO); values.len()];
    let total: f64 = values.iter().map(|v| v.max(0.0)).sum();
    if total <= 0.0 || rect.area() <= 0.0 {
        return out;
    }
    let mut order: Vec<usize> = (0..values.len()).filter(|&i| values[i] > 0.0).collect();
    order.sort_by(|&a, &b| values[b].partial_cmp(&values[a]).unwrap_or(std::cmp::Ordering::Equal));
    let scale = rect.area() as f64 / total;
    let areas: Vec<f64> = order.iter().map(|&i| values[i] * scale).collect();
    let mut free = rect;
    let mut i = 0;
    let worst = |row: &[f64], side: f64| -> f64 {
        let s: f64 = row.iter().sum();
        let (mx, mn) = row.iter().fold((0.0f64, f64::INFINITY), |(a, b), &v| (a.max(v), b.min(v)));
        let s2 = s * s;
        let w2 = side * side;
        (w2 * mx / s2).max(s2 / (w2 * mn))
    };
    while i < areas.len() {
        let side = free.width().min(free.height()) as f64;
        let mut j = i + 1;
        while j < areas.len() && worst(&areas[i..=j], side) <= worst(&areas[i..j], side) {
            j += 1;
        }
        let row = &areas[i..j];
        let sum: f64 = row.iter().sum();
        let last = j == areas.len();
        if free.width() >= free.height() {
            // column on the left
            let w = if last { free.width() } else { (sum / free.height() as f64) as f32 };
            let mut y = free.top();
            for (k, a) in row.iter().enumerate() {
                let h = if k + 1 == row.len() { free.bottom() - y } else { (a / sum) as f32 * free.height() };
                out[order[i + k]] = Rect::from_min_size(pos2(free.left(), y), vec2(w, h));
                y += h;
            }
            free.min.x += w;
        } else {
            let h = if last { free.height() } else { (sum / free.width() as f64) as f32 };
            let mut x = free.left();
            for (k, a) in row.iter().enumerate() {
                let w = if k + 1 == row.len() { free.right() - x } else { (a / sum) as f32 * free.width() };
                out[order[i + k]] = Rect::from_min_size(pos2(x, free.top()), vec2(w, h));
                x += w;
            }
            free.min.y += h;
        }
        i = j;
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Dates (for the calendar heatmap)
// ---------------------------------------------------------------------------------------------

/// Days since 1970-01-01 for a civil date (proleptic Gregorian).
pub(crate) fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Civil date for days since 1970-01-01.
pub(crate) fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((if m <= 2 { y + 1 } else { y }) as i32, m, d)
}

/// Weekday for days since 1970-01-01, 0 = Sunday.
pub(crate) fn weekday(days: i64) -> u32 {
    (days + 4).rem_euclid(7) as u32
}

/// Short English month name (1-based).
pub(crate) fn month_name(m: u32) -> &'static str {
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][((m.max(1) - 1) % 12) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nice_numbers() {
        assert_eq!(nice_number(0.7, true), 1.0);
        assert_eq!(nice_number(1.6, true), 2.0);
        assert_eq!(nice_number(4.0, true), 5.0);
        assert_eq!(nice_number(8.0, true), 10.0);
        assert_eq!(nice_number(1250.0, true), 1000.0);
        assert_eq!(nice_number(4300.0, false), 5000.0);
        assert_eq!(nice_number(0.03, false), 0.05);
    }

    #[test]
    fn ticks_basic() {
        let t = nice_ticks(0.0, 4300.0, 5);
        assert_eq!(t.values, vec![0.0, 1000.0, 2000.0, 3000.0, 4000.0, 5000.0]);
        assert_eq!(t.step, 1000.0);
        let t = nice_ticks(0.0, 1.0, 6);
        assert_eq!(t.values, vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0]);
    }

    #[test]
    fn ticks_cover_range_and_are_nice() {
        for &(lo, hi, n) in &[(0.0, 305.0, 5), (-120.0, 480.0, 6), (12.5, 13.1, 4), (-3.0, -1.0, 5), (0.0, 1e7, 5)] {
            let t = nice_ticks(lo, hi, n);
            assert!(t.min <= lo && t.max >= hi, "{lo}..{hi} -> {t:?}");
            assert!(t.values.len() >= 2 && t.values.len() <= n + 3, "{t:?}");
            let m = t.step / 10f64.powf(t.step.log10().floor());
            assert!([1.0, 2.0, 5.0].iter().any(|x| (m - x).abs() < 1e-9), "step {}", t.step);
            for w in t.values.windows(2) {
                assert!(((w[1] - w[0]) - t.step).abs() < t.step * 1e-6);
            }
        }
    }

    #[test]
    fn ticks_degenerate() {
        let t = nice_ticks(0.0, 0.0, 5);
        assert_eq!(t.min, 0.0);
        assert!(t.max > 0.0);
        let t = nice_ticks(50.0, 50.0, 5);
        assert!(t.min < 50.0 && t.max > 50.0);
        let t = nice_ticks(f64::NAN, 3.0, 5);
        assert!(t.values.iter().all(|v| v.is_finite()));
        assert!(!nice_ticks(-0.0001, 0.0, 5).values.iter().any(|v| v.to_string() == "-0"));
    }

    #[test]
    fn compact_format() {
        assert_eq!(format_compact(0.0), "0");
        assert_eq!(format_compact(950.0), "950");
        assert_eq!(format_compact(1000.0), "1k");
        assert_eq!(format_compact(1200.0), "1.2k");
        assert_eq!(format_compact(15_300.0), "15.3k");
        assert_eq!(format_compact(250_000.0), "250k");
        assert_eq!(format_compact(999_950.0), "1M");
        assert_eq!(format_compact(3_400_000.0), "3.4M");
        assert_eq!(format_compact(2.1e9), "2.1B");
        assert_eq!(format_compact(-2500.0), "-2.5k");
        assert_eq!(format_compact(0.25), "0.25");
        assert_eq!(format_compact(12.5), "12.5");
        assert_eq!(format_compact(f64::NAN), "–");
    }

    #[test]
    fn tick_format() {
        assert_eq!(format_tick(0.0, 1000.0, 5000.0), "0");
        assert_eq!(format_tick(2000.0, 1000.0, 5000.0), "2000");
        assert_eq!(format_tick(20_000.0, 10_000.0, 50_000.0), "20k");
        assert_eq!(format_tick(2_500_000.0, 500_000.0, 5_000_000.0), "2.5M");
        assert_eq!(format_tick(0.4, 0.2, 1.0), "0.4");
        assert_eq!(format_tick(-40.0, 20.0, 100.0), "-40");
    }

    #[test]
    fn thousands_and_percent() {
        assert_eq!(format_thousands(1234567.891, 2), "1,234,567.89");
        assert_eq!(format_thousands(-950.0, 0), "-950");
        assert_eq!(format_thousands(100.0, 0), "100");
        assert_eq!(format_percent(0.426), "43%");
        assert_eq!(format_percent(0.035), "3.5%");
    }

    #[test]
    fn stacking() {
        let v = vec![vec![1.0, 2.0, -1.0], vec![3.0, 2.0, -2.0], vec![1.0, -1.0, 4.0]];
        let s = stack(&v, &[1.0, 1.0, 1.0], Stacking::Stacked);
        assert_eq!(s[0][0], (0.0, 1.0));
        assert_eq!(s[1][0], (1.0, 4.0));
        assert_eq!(s[2][0], (4.0, 5.0));
        // negatives stack downward separately
        assert_eq!(s[2][1], (0.0, -1.0));
        assert_eq!(s[0][2], (0.0, -1.0));
        assert_eq!(s[1][2], (-1.0, -3.0));
        assert_eq!(s[2][2], (0.0, 4.0));
        // hidden series collapse
        let s = stack(&v, &[1.0, 0.0, 1.0], Stacking::Stacked);
        assert_eq!(s[2][0], (1.0, 2.0));
        // expanded sums to one
        let s = stack(&[vec![1.0, 3.0], vec![3.0, 1.0]], &[1.0, 1.0], Stacking::Expanded);
        assert!((s[1][0].1 - 1.0).abs() < 1e-12 && (s[0][0].1 - 0.25).abs() < 1e-12);
        assert!((s[1][1].1 - 1.0).abs() < 1e-12);
        // none = independent bands
        let s = stack(&v, &[1.0; 3], Stacking::None);
        assert_eq!(s[1][0], (0.0, 3.0));
    }

    #[test]
    fn treemap_layout() {
        let r = Rect::from_min_size(pos2(10.0, 20.0), vec2(300.0, 200.0));
        let vals = [6.0, 6.0, 4.0, 3.0, 2.0, 2.0, 1.0, 0.0];
        let tiles = squarify(&vals, r);
        assert_eq!(tiles.len(), vals.len());
        let total: f64 = vals.iter().sum();
        let mut covered = 0.0;
        for (i, t) in tiles.iter().enumerate() {
            let want = vals[i] / total * r.area() as f64;
            assert!((t.area() as f64 - want).abs() < 1.0, "tile {i}: {} vs {want}", t.area());
            assert!(r.expand(0.01).contains_rect(*t), "tile {i} outside");
            covered += t.area();
        }
        assert!((covered - r.area()).abs() < 1.0);
        for i in 0..tiles.len() {
            for j in i + 1..tiles.len() {
                let o = tiles[i].intersect(tiles[j]);
                assert!(!o.is_positive() || o.area() < 0.01, "{i} overlaps {j}");
            }
        }
        // squarified tiles stay reasonably square
        let worst = tiles[..7].iter().map(|t| (t.width() / t.height()).max(t.height() / t.width())).fold(0.0, f32::max);
        assert!(worst < 4.0, "aspect {worst}");
    }

    #[test]
    fn dates() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(weekday(0), 4); // Thursday
        let d = days_from_civil(2025, 3, 4);
        assert_eq!(civil_from_days(d), (2025, 3, 4));
        assert_eq!(weekday(days_from_civil(2026, 11, 11)), 3); // Wednesday
        assert_eq!(civil_from_days(days_from_civil(2024, 2, 29) + 1), (2024, 3, 1));
    }

    #[test]
    fn clip_segment() {
        let sq = [pos2(0.0, 0.0), pos2(10.0, 0.0), pos2(10.0, 10.0), pos2(0.0, 10.0)];
        let (a, b) = clip_segment_convex(&sq, pos2(-5.0, 5.0), pos2(15.0, 5.0)).unwrap();
        assert!((a.x - 0.0).abs() < 1e-4 && (b.x - 10.0).abs() < 1e-4);
        assert!(clip_segment_convex(&sq, pos2(-5.0, -5.0), pos2(-1.0, 20.0)).is_none());
    }
}
