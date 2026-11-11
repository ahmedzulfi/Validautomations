//! Polar charts: radar, radial bars, gauge and donut progress.

use crate::cartesian::labels_vec;
use crate::common::*;
use egui::epaint::Mesh;
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Widget};
use glowkit_core::anim::{ease, stagger, Easing};
use glowkit_core::color::{gradient, with_alpha};
use glowkit_core::paint::{arc, arc_gradient};
use glowkit_core::Theme;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

// ---------------------------------------------------------------------------------------------
// RadarChart
// ---------------------------------------------------------------------------------------------

/// Shape of the radar grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RadarGrid {
    /// Concentric polygons (a web).
    #[default]
    Polygon,
    /// Concentric circles.
    Circle,
}

/// A radar (spider) chart: one axis per category, each series a translucent polygon.
///
/// Polygons unfold from the center on appear; hovering an axis highlights it and lists every
/// series' value; the legend toggles series.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::{RadarChart, Series};
/// let s = [Series::new("2024", [80.0, 95.0, 70.0, 60.0, 88.0, 75.0])];
/// ui.add(RadarChart::new(&s).labels(&["Speed", "Power", "Range", "Comfort", "Safety", "Price"]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct RadarChart<'a> {
    series: &'a [Series],
    labels: Vec<String>,
    base: Base,
    levels: usize,
    grid_shape: RadarGrid,
    dots: bool,
    fill_opacity: f32,
    max: Option<f64>,
}

impl<'a> RadarChart<'a> {
    /// A radar chart; each series needs one value per axis.
    pub fn new(series: &'a [Series]) -> Self {
        Self { series, labels: Vec::new(), base: Base::new(300.0), levels: 4, grid_shape: RadarGrid::Polygon, dots: true, fill_opacity: 0.28, max: None }
    }
    base_setters!();
    /// Axis labels.
    pub fn labels<S: AsRef<str>>(mut self, labels: &[S]) -> Self {
        self.labels = labels_vec(labels);
        self
    }
    /// Number of grid rings (default 4).
    pub fn levels(mut self, n: usize) -> Self {
        self.levels = n.max(1);
        self
    }
    /// Polygon web or circles.
    pub fn grid_shape(mut self, g: RadarGrid) -> Self {
        self.grid_shape = g;
        self
    }
    /// Dots at the vertices (default `true`).
    pub fn dots(mut self, on: bool) -> Self {
        self.dots = on;
        self
    }
    /// Fill opacity of the polygons (default 0.28).
    pub fn fill_opacity(mut self, a: f32) -> Self {
        self.fill_opacity = a;
        self
    }
    /// Fixed outer value (defaults to a nice number above the data).
    pub fn max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }
}

fn fan_fill(painter: &egui::Painter, c: Pos2, pts: &[Pos2], color: Color32) {
    let mut m = Mesh::default();
    m.colored_vertex(c, color);
    for p in pts {
        m.colored_vertex(*p, color);
    }
    let n = pts.len() as u32;
    for i in 0..n {
        m.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(Shape::mesh(m));
}

impl Widget for RadarChart<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp, id) = self.base.allocate(ui, Sense::hover());
        let k = self.series.len();
        let n = self.series.iter().map(|s| s.values.len()).max().unwrap_or(0).max(self.labels.len());
        let mut hidden = hidden_state(ui, id, k);
        let painter = ui.painter_at(rect);
        let color_of = |i: usize| self.series[i].color.unwrap_or_else(|| palette(&t, i));
        let legend_on = self.base.legend && k > 1;
        let mut focus = None;
        if legend_on {
            let entries: Vec<LegendEntry> = self.series.iter().enumerate().map(|(i, s)| LegendEntry { label: s.name.clone(), color: color_of(i) }).collect();
            let before = hidden.clone();
            focus = legend(ui, id, Rect::from_min_max(pos2(rect.left(), rect.bottom() - LEGEND_H), rect.max), &entries, &mut hidden);
            if before != hidden {
                set_hidden_state(ui, id, hidden.clone());
            }
        }
        if n < 3 || !ui.is_rect_visible(rect) {
            return resp;
        }
        let area = Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() - if legend_on { LEGEND_H } else { 0.0 }));
        let c = area.center();
        let font = axis_font();
        let label_w = self.labels.iter().map(|l| text_width(&painter, l, &font)).fold(0.0, f32::max);
        let radius = ((area.width() / 2.0 - label_w - 14.0).min(area.height() / 2.0 - 28.0)).max(10.0);
        let data_max = self.series.iter().flat_map(|s| s.values.iter()).fold(0.0f64, |m, v| m.max(*v));
        let max = self.max.unwrap_or_else(|| nice_ticks(0.0, data_max, self.levels + 1).max).max(1e-9);
        let mut targets: Vec<f32> = self.series.iter().flat_map(|s| (0..n).map(|i| s.values.get(i).copied().unwrap_or(0.0) as f32)).collect();
        targets.extend(hidden.iter().map(|h| if *h { 0.0 } else { 1.0 }));
        let cur = tween(ui, id, &targets);
        let angle = |i: usize| -FRAC_PI_2 + i as f32 / n as f32 * TAU;
        let dir = |i: usize| vec2(angle(i).cos(), angle(i).sin());
        // grid
        let gc = grid_color(&t);
        for l in 1..=self.levels {
            let r = radius * l as f32 / self.levels as f32;
            match self.grid_shape {
                RadarGrid::Polygon => {
                    let pts: Vec<Pos2> = (0..n).map(|i| c + dir(i) * r).collect();
                    painter.add(Shape::closed_line(pts, Stroke::new(1.0, gc)));
                }
                RadarGrid::Circle => {
                    painter.circle_stroke(c, r, Stroke::new(1.0, gc));
                }
            }
        }
        // hover axis
        let hover = resp.hover_pos().filter(|p| p.distance(c) < radius + 30.0 && p.distance(c) > 4.0).map(|p| {
            let a = ((p - c).y.atan2((p - c).x) + FRAC_PI_2).rem_euclid(TAU);
            ((a / TAU * n as f32).round() as usize) % n
        });
        let halpha = glowkit_core::anim::follow(ui, id.with("hover_alpha"), if hover.is_some() { 1.0 } else { 0.0 }, 16.0);
        for i in 0..n {
            let hl = hover == Some(i);
            let col = if hl { t.muted_foreground.gamma_multiply(0.7) } else { gc };
            painter.line_segment([c, c + dir(i) * radius], Stroke::new(1.0, col));
            if let Some(l) = self.labels.get(i) {
                let d = dir(i);
                let p = c + d * (radius + 10.0);
                let align = if d.x.abs() < 0.2 {
                    if d.y < 0.0 { Align2::CENTER_BOTTOM } else { Align2::CENTER_TOP }
                } else if d.x > 0.0 {
                    Align2::LEFT_CENTER
                } else {
                    Align2::RIGHT_CENTER
                };
                painter.text(p, align, l, font.clone(), if hl { t.foreground } else { t.muted_foreground });
            }
        }
        // polygons
        let e = entry_time(ui, id, self.base.animate, 1.4);
        for s in 0..k {
            let vis = cur[k * n + s];
            if vis < 0.01 {
                continue;
            }
            let grow = ease(Easing::OutBack, stagger(e, s, 0.15, 0.9)) * vis;
            let dim = match focus {
                Some(f) if f != s => 0.3,
                _ => 1.0,
            };
            let color = color_of(s);
            let pts: Vec<Pos2> = (0..n).map(|i| c + dir(i) * radius * (cur[s * n + i] / max as f32).max(0.0) * grow).collect();
            fan_fill(&painter, c, &pts, with_alpha(color, self.fill_opacity * dim * vis));
            painter.add(Shape::closed_line(pts.clone(), Stroke::new(2.0, color.gamma_multiply(dim * vis))));
            if self.dots {
                for (i, p) in pts.iter().enumerate() {
                    let big = hover == Some(i);
                    painter.circle_filled(*p, if big { 5.5 } else { 4.0 }, t.card.gamma_multiply(vis));
                    painter.circle_filled(*p, if big { 4.0 } else { 2.6 }, color.gamma_multiply(dim * vis));
                }
            }
        }
        if let (Some(i), true) = (hover, self.base.tooltip) {
            let rows: Vec<TipRow> = (0..k).filter(|s| !hidden[*s]).map(|s| TipRow::new(Some(color_of(s)), self.series[s].name.clone(), (self.base.format)(self.series[s].values.get(i).copied().unwrap_or(0.0)))).collect();
            tooltip(ui, id, resp.hover_pos().unwrap_or(c), self.labels.get(i).map(|s| s.as_str()).unwrap_or(""), &rows, halpha);
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// RadialBarChart
// ---------------------------------------------------------------------------------------------

/// Concentric radial bars: each datum is a ring whose arc length is its value, over a faint
/// track. Rings sweep in one after another.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::{RadialBarChart, Datum};
/// let d = [Datum::new("Chrome", 275.0), Datum::new("Safari", 200.0), Datum::new("Firefox", 187.0)];
/// ui.add(RadialBarChart::new(&d).sweep(270.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct RadialBarChart<'a> {
    data: &'a [Datum],
    base: Base,
    max: Option<f64>,
    sweep: f32,
    start: f32,
    inner: f32,
    track: bool,
    labels: bool,
    center_title: Option<String>,
    center_caption: Option<String>,
}

impl<'a> RadialBarChart<'a> {
    /// One ring per datum, outermost first.
    pub fn new(data: &'a [Datum]) -> Self {
        Self { data, base: Base::new(280.0), max: None, sweep: 270.0, start: 0.0, inner: 0.3, track: true, labels: true, center_title: None, center_caption: None }
    }
    base_setters!();
    /// Value of a full ring (default: the largest value).
    pub fn max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }
    /// Degrees a full ring covers (default 270).
    pub fn sweep(mut self, degrees: f32) -> Self {
        self.sweep = degrees.clamp(10.0, 360.0);
        self
    }
    /// Start angle in degrees clockwise from 12 o'clock.
    pub fn start_angle(mut self, degrees: f32) -> Self {
        self.start = degrees;
        self
    }
    /// Inner hole as a fraction of the radius (default 0.3).
    pub fn inner_radius(mut self, f: f32) -> Self {
        self.inner = f.clamp(0.0, 0.9);
        self
    }
    /// Draw the faint background track (default `true`).
    pub fn track(mut self, on: bool) -> Self {
        self.track = on;
        self
    }
    /// Labels at the start of each ring (default `true`).
    pub fn ring_labels(mut self, on: bool) -> Self {
        self.labels = on;
        self
    }
    /// Text in the center.
    pub fn center_title(mut self, text: impl Into<String>) -> Self {
        self.center_title = Some(text.into());
        self
    }
    /// Caption under the center text.
    pub fn center_caption(mut self, text: impl Into<String>) -> Self {
        self.center_caption = Some(text.into());
        self
    }
}

impl Widget for RadialBarChart<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp, id) = self.base.allocate(ui, Sense::hover());
        let n = self.data.len();
        if n == 0 || !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter_at(rect);
        let c = rect.center();
        let radius = rect.width().min(rect.height()) / 2.0 - 6.0;
        let r_in = radius * self.inner;
        let ring = (radius - r_in) / n as f32;
        let w = ring * 0.72;
        let max = self.max.unwrap_or_else(|| self.data.iter().fold(0.0f64, |m, d| m.max(d.value))).max(1e-9);
        let targets: Vec<f32> = self.data.iter().map(|d| (d.value / max).clamp(0.0, 1.0) as f32).collect();
        let cur = tween(ui, id, &targets);
        let e = entry_time(ui, id, self.base.animate, 1.6);
        let a0 = -FRAC_PI_2 + self.start.to_radians();
        let full = self.sweep.to_radians();
        let hover = resp.hover_pos().and_then(|p| {
            let d = p.distance(c);
            if d < r_in || d > radius {
                return None;
            }
            Some((((radius - d) / ring).floor() as usize).min(n - 1))
        });
        let halpha = glowkit_core::anim::follow(ui, id.with("hover_alpha"), if hover.is_some() { 1.0 } else { 0.0 }, 16.0);
        let font = axis_font();
        for (i, d) in self.data.iter().enumerate() {
            let rm = radius - ring * (i as f32 + 0.5);
            let color = d.color.unwrap_or_else(|| palette(&t, i));
            if self.track {
                arc(&painter, c, rm, a0, a0 + full, w, track_color(&t));
            }
            let p = ease(Easing::OutCubic, stagger(e, i, 0.1, 1.0));
            let dim = if hover.is_some() && hover != Some(i) { 1.0 - 0.45 * halpha } else { 1.0 };
            let span = full * cur[i] * p;
            if span > 1e-3 {
                arc(&painter, c, rm, a0, a0 + span, w, color.gamma_multiply(dim));
            }
            if self.labels && self.sweep < 359.0 {
                let tangent = vec2(-a0.sin(), a0.cos());
                let anchor = c + vec2(a0.cos(), a0.sin()) * rm - tangent * (w / 2.0 + 6.0);
                painter.text(anchor, Align2::RIGHT_CENTER, &d.label, font.clone(), t.muted_foreground.lerp_to_gamma(t.foreground, if hover == Some(i) { 1.0 } else { 0.0 }));
            }
        }
        if self.center_title.is_some() || self.center_caption.is_some() {
            let size = (r_in * 0.5).clamp(14.0, 30.0);
            if let Some(tt) = &self.center_title {
                painter.text(c - vec2(0.0, size * 0.2), Align2::CENTER_CENTER, tt, FontId::proportional(size), t.foreground);
            }
            if let Some(cc) = &self.center_caption {
                painter.text(c + vec2(0.0, size * 0.6), Align2::CENTER_CENTER, cc, FontId::proportional(11.5), t.muted_foreground);
            }
        }
        if let (Some(i), true) = (hover, self.base.tooltip) {
            let d = &self.data[i];
            let row = TipRow::new(Some(d.color.unwrap_or_else(|| palette(&t, i))), d.label.clone(), (self.base.format)(d.value));
            tooltip(ui, id, resp.hover_pos().unwrap_or(c), "", &[row], halpha);
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Gauge
// ---------------------------------------------------------------------------------------------

/// A semicircular gauge with a gradient arc, tick marks, a needle that springs to the value,
/// and a big value readout.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_charts::Gauge::new(72.0).range(0.0, 100.0).label("CPU load").unit("%"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Gauge {
    value: f64,
    min: f64,
    max: f64,
    label: String,
    unit: String,
    colors: Option<Vec<Color32>>,
    size: f32,
    thickness: f32,
    needle: bool,
    ticks: bool,
    animate: bool,
    format: Formatter,
}

impl Gauge {
    /// A gauge showing `value` (0..100 by default).
    pub fn new(value: f64) -> Self {
        Self {
            value,
            min: 0.0,
            max: 100.0,
            label: String::new(),
            unit: String::new(),
            colors: None,
            size: 240.0,
            thickness: 16.0,
            needle: true,
            ticks: true,
            animate: true,
            format: |v| format!("{v:.0}"),
        }
    }
    /// Value range (default 0..100).
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }
    /// Caption under the value.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
    /// Unit appended to the value (e.g. `"%"`).
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }
    /// Gradient stops along the arc (default: success → warning → destructive).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }
    /// Width of the gauge in points (default 240).
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Arc thickness (default 16).
    pub fn thickness(mut self, th: f32) -> Self {
        self.thickness = th;
        self
    }
    /// Show the needle (default `true`).
    pub fn needle(mut self, on: bool) -> Self {
        self.needle = on;
        self
    }
    /// Show tick marks (default `true`).
    pub fn ticks(mut self, on: bool) -> Self {
        self.ticks = on;
        self
    }
    /// Sweep from the minimum on appear (default `true`).
    pub fn animate(mut self, on: bool) -> Self {
        self.animate = on;
        self
    }
    /// Value formatter for the readout.
    pub fn format(mut self, f: Formatter) -> Self {
        self.format = f;
        self
    }
}

impl Widget for Gauge {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let h = self.size * 0.5 + 58.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(self.size, h), Sense::hover());
        let id = resp.id;
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter_at(rect.expand(4.0));
        let span = (self.max - self.min).max(1e-9);
        let frac_t = ((self.value - self.min) / span).clamp(0.0, 1.0) as f32;
        let e = entry_time(ui, id, self.animate, 1.6);
        let intro = ease(Easing::OutCubic, e / 1.3);
        // spring towards the value (with a little overshoot on the needle)
        let frac = glowkit_core::anim::spring(ui, id.with("needle"), frac_t, 90.0, 13.0) * intro;
        let frac = if self.animate { frac } else { frac_t };
        let r = self.size / 2.0 - self.thickness / 2.0 - 4.0;
        let c = pos2(rect.center().x, rect.top() + r + self.thickness / 2.0 + 4.0);
        let colors = self.colors.clone().unwrap_or_else(|| vec![t.success, t.warning, t.destructive]);
        let (a0, a1) = (PI, TAU);
        arc(&painter, c, r, a0, a1, self.thickness, track_color(&t));
        let fv = frac.clamp(0.0, 1.0);
        if fv > 0.002 {
            let end = a0 + PI * fv;
            arc_gradient(&painter, c, r, a0, end, self.thickness, |u| gradient(&colors, u * fv));
            painter.circle_filled(c + vec2(a0.cos(), a0.sin()) * r, self.thickness / 2.0, gradient(&colors, 0.0));
            painter.circle_filled(c + vec2(end.cos(), end.sin()) * r, self.thickness / 2.0, gradient(&colors, fv));
            glowkit_core::paint::glow(&painter, c + vec2(end.cos(), end.sin()) * r, self.thickness * 1.6, with_alpha(gradient(&colors, fv), 0.35));
        }
        if self.ticks {
            for i in 0..=20 {
                let a = a0 + PI * i as f32 / 20.0;
                let d = vec2(a.cos(), a.sin());
                let major = i % 5 == 0;
                let r0 = r - self.thickness / 2.0 - 5.0;
                let r1 = r0 - if major { 7.0 } else { 3.5 };
                painter.line_segment([c + d * r0, c + d * r1], Stroke::new(if major { 1.4 } else { 1.0 }, t.muted_foreground.gamma_multiply(if major { 0.7 } else { 0.4 })));
            }
        }
        if self.needle {
            let a = a0 + PI * fv;
            let d = vec2(a.cos(), a.sin());
            let len = r - self.thickness / 2.0 - 16.0;
            let tip = c + d * len;
            let side = d.rot90() * 4.0;
            let back = c - d * 10.0;
            painter.add(Shape::convex_polygon(vec![back + side, tip, back - side], t.foreground, Stroke::NONE));
            painter.circle_filled(c, 7.0, t.foreground);
            painter.circle_filled(c, 3.0, t.card);
        }
        let shown = self.min + span * fv as f64;
        let value_y = c.y + 26.0;
        painter.text(pos2(c.x, value_y), Align2::CENTER_CENTER, format!("{}{}", (self.format)(shown), self.unit), FontId::proportional(26.0), t.foreground);
        if !self.label.is_empty() {
            painter.text(pos2(c.x, value_y + 22.0), Align2::CENTER_CENTER, &self.label, FontId::proportional(12.0), t.muted_foreground);
        }
        let font = axis_font();
        painter.text(c + vec2(-r, self.thickness / 2.0 + 6.0), Align2::CENTER_TOP, (self.format)(self.min), font.clone(), t.muted_foreground);
        painter.text(c + vec2(r, self.thickness / 2.0 + 6.0), Align2::CENTER_TOP, (self.format)(self.max), font, t.muted_foreground);
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// DonutProgress
// ---------------------------------------------------------------------------------------------

/// A circular progress ring with the percentage in the middle (Tremor-style progress circle).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_charts::DonutProgress::new(0.72).label("Storage"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DonutProgress {
    value: f32,
    size: f32,
    thickness: f32,
    color: Option<Color32>,
    gradient: Option<Vec<Color32>>,
    label: String,
    text: Option<String>,
    animate: bool,
}

impl DonutProgress {
    /// Progress in `0..=1`.
    pub fn new(fraction: f32) -> Self {
        Self { value: fraction.clamp(0.0, 1.0), size: 120.0, thickness: 10.0, color: None, gradient: None, label: String::new(), text: None, animate: true }
    }
    /// Diameter in points (default 120).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    /// Ring thickness (default 10).
    pub fn thickness(mut self, th: f32) -> Self {
        self.thickness = th;
        self
    }
    /// Ring color (default: the theme accent).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Gradient along the ring instead of a solid color.
    pub fn gradient(mut self, stops: &[Color32]) -> Self {
        self.gradient = Some(stops.to_vec());
        self
    }
    /// Caption under the ring.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
    /// Center text (defaults to the percentage).
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }
    /// Sweep in on appear (default `true`).
    pub fn animate(mut self, on: bool) -> Self {
        self.animate = on;
        self
    }
}

impl Widget for DonutProgress {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let label_h = if self.label.is_empty() { 0.0 } else { 24.0 };
        let (rect, resp) = ui.allocate_exact_size(vec2(self.size, self.size + label_h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        let painter = ui.painter_at(rect.expand(6.0));
        let c = pos2(rect.center().x, rect.top() + self.size / 2.0);
        let r = self.size / 2.0 - self.thickness / 2.0 - 1.0;
        let e = entry_time(ui, id, self.animate, 1.4);
        let target = tween(ui, id, &[self.value])[0];
        let v = target * ease(Easing::OutCubic, e / 1.2);
        arc(&painter, c, r, 0.0, TAU, self.thickness, track_color(&t));
        let color = self.color.unwrap_or(t.accent);
        let a0 = -FRAC_PI_2;
        if v > 0.002 {
            match &self.gradient {
                Some(stops) => {
                    arc_gradient(&painter, c, r, a0, a0 + TAU * v, self.thickness, |u| gradient(stops, u * v));
                    painter.circle_filled(c + vec2(a0.cos(), a0.sin()) * r, self.thickness / 2.0, gradient(stops, 0.0));
                    let ea = a0 + TAU * v;
                    painter.circle_filled(c + vec2(ea.cos(), ea.sin()) * r, self.thickness / 2.0, gradient(stops, v));
                }
                None => arc(&painter, c, r, a0, a0 + TAU * v, self.thickness, color),
            }
        }
        let txt = self.text.clone().unwrap_or_else(|| format!("{:.0}%", v * 100.0));
        painter.text(c, Align2::CENTER_CENTER, txt, FontId::proportional((self.size * 0.2).clamp(12.0, 30.0)), t.foreground);
        if !self.label.is_empty() {
            painter.text(pos2(c.x, rect.bottom() - 8.0), Align2::CENTER_CENTER, &self.label, FontId::proportional(12.0), t.muted_foreground);
        }
        resp
    }
}
