//! Pie and donut charts.

use crate::common::*;
use egui::{pos2, vec2, Align2, FontId, Rect, Response, Sense, Stroke, Ui, Widget};
use glowkit_core::anim::{ease, Easing};
use glowkit_core::color::contrast_text;
use glowkit_core::Theme;
use std::f32::consts::{FRAC_PI_2, TAU};

/// Slice labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PieLabels {
    /// No labels on the chart (the legend and tooltip still name slices).
    #[default]
    None,
    /// Percentages inside the slices.
    Percent,
    /// Values inside the slices.
    Value,
    /// Name and value outside the ring with leader lines.
    Outside,
}

/// A pie chart, or a donut when given an inner radius ([`DonutChart`] is a preset).
///
/// Slices sweep in on appear, the hovered slice pops out, the legend toggles slices (the rest
/// re-flow smoothly), and donuts show a center total that switches to the hovered slice.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::{PieChart, Datum, PieLabels};
/// let data = [Datum::new("Chrome", 275.0), Datum::new("Safari", 200.0), Datum::new("Firefox", 187.0)];
/// ui.add(PieChart::new(&data).labels(PieLabels::Percent).height(260.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PieChart<'a> {
    data: &'a [Datum],
    base: Base,
    inner: f32,
    pad_angle: f32,
    labels: PieLabels,
    rounded: bool,
    center_title: Option<String>,
    center_caption: Option<String>,
    pop_out: f32,
    start_angle: f32,
}

/// Donut chart preset: a [`PieChart`] with a 62% inner radius, padding between slices and a
/// center total.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::{DonutChart, Datum};
/// let data = [Datum::new("Desktop", 1260.0), Datum::new("Mobile", 940.0), Datum::new("Tablet", 310.0)];
/// ui.add(DonutChart::new(&data).center_caption("Visitors"));
/// ```
pub struct DonutChart;

impl DonutChart {
    /// A donut chart of `data` (returns a configured [`PieChart`]).
    #[allow(clippy::new_ret_no_self)]
    pub fn new(data: &[Datum]) -> PieChart<'_> {
        PieChart::new(data).inner_radius(0.62).pad_angle(2.0)
    }
}

impl<'a> PieChart<'a> {
    /// A pie chart of `data`.
    pub fn new(data: &'a [Datum]) -> Self {
        Self {
            data,
            base: Base::new(260.0),
            inner: 0.0,
            pad_angle: 0.0,
            labels: PieLabels::None,
            rounded: false,
            center_title: None,
            center_caption: None,
            pop_out: 7.0,
            start_angle: 0.0,
        }
    }
    base_setters!();
    /// Inner radius as a fraction of the outer radius (0 = pie, ~0.6 = donut).
    pub fn inner_radius(mut self, fraction: f32) -> Self {
        self.inner = fraction.clamp(0.0, 0.95);
        self
    }
    /// Gap between slices in degrees.
    pub fn pad_angle(mut self, degrees: f32) -> Self {
        self.pad_angle = degrees.max(0.0);
        self
    }
    /// Slice labels.
    pub fn labels(mut self, labels: PieLabels) -> Self {
        self.labels = labels;
        self
    }
    /// Round slice ends (donuts only): pill-shaped segments.
    pub fn rounded(mut self, on: bool) -> Self {
        self.rounded = on;
        self
    }
    /// Big text in the donut center (defaults to the formatted total).
    pub fn center_title(mut self, text: impl Into<String>) -> Self {
        self.center_title = Some(text.into());
        self
    }
    /// Small caption under the center title (e.g. "Visitors").
    pub fn center_caption(mut self, text: impl Into<String>) -> Self {
        self.center_caption = Some(text.into());
        self
    }
    /// How far the hovered slice pops out, in points (default 7).
    pub fn pop_out(mut self, points: f32) -> Self {
        self.pop_out = points;
        self
    }
    /// Rotate the start of the first slice, in degrees clockwise from 12 o'clock.
    pub fn start_angle(mut self, degrees: f32) -> Self {
        self.start_angle = degrees;
        self
    }
}

impl Widget for PieChart<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp, id) = self.base.allocate(ui, Sense::hover());
        let n = self.data.len();
        let mut hidden = hidden_state(ui, id, n);
        let painter = ui.painter_at(rect);
        let color_of = |i: usize| self.data[i].color.unwrap_or_else(|| palette(&t, i));
        let legend_on = self.base.legend && n > 0;
        let mut focus = None;
        if legend_on {
            let entries: Vec<LegendEntry> = self.data.iter().enumerate().map(|(i, d)| LegendEntry { label: d.label.clone(), color: color_of(i) }).collect();
            let before = hidden.clone();
            focus = legend(ui, id, Rect::from_min_max(pos2(rect.left(), rect.bottom() - LEGEND_H), rect.max), &entries, &mut hidden);
            if before != hidden {
                set_hidden_state(ui, id, hidden.clone());
            }
        }
        if n == 0 || !ui.is_rect_visible(rect) {
            return resp;
        }
        let targets: Vec<f32> = self.data.iter().enumerate().map(|(i, d)| if hidden[i] { 0.0 } else { d.value.max(0.0) as f32 }).collect();
        let cur = tween(ui, id, &targets);
        let total: f32 = cur.iter().sum::<f32>().max(1e-9);
        let total_raw: f64 = self.data.iter().enumerate().filter(|(i, _)| !hidden[*i]).map(|(_, d)| d.value.max(0.0)).sum();
        let area = Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() - if legend_on { LEGEND_H + 4.0 } else { 0.0 }));
        let outside = self.labels == PieLabels::Outside;
        let label_w = if outside {
            let f = FontId::proportional(11.5);
            let tot: f64 = self.data.iter().map(|d| d.value.max(0.0)).sum::<f64>().max(1e-9);
            self.data.iter().map(|d| text_width(&painter, &format!("{} {}", d.label, format_percent(d.value.max(0.0) / tot)), &f)).fold(0.0, f32::max)
        } else {
            0.0
        };
        let r_out = if outside {
            (area.width() / 2.0 - label_w - 26.0).min(area.height() / 2.0 - 24.0).max(8.0)
        } else {
            (area.width().min(area.height()) / 2.0 - self.pop_out - 4.0).max(8.0)
        };
        let r_in = r_out * self.inner;
        let c = area.center();

        let e = entry_time(ui, id, self.base.animate, 1.3);
        let sweep = ease(Easing::InOutCubic, e / 1.1);
        let scale = 0.92 + 0.08 * ease(Easing::OutBack, e / 0.9);
        let start = -FRAC_PI_2 + self.start_angle.to_radians();
        let pad = self.pad_angle.to_radians();

        // hit test
        let hover = resp.hover_pos().and_then(|p| {
            let d = p - c;
            let dist = d.length();
            if dist < r_in * 0.95 || dist > r_out + self.pop_out + 2.0 {
                return None;
            }
            let a = (d.y.atan2(d.x) - start).rem_euclid(TAU) / sweep.max(1e-3);
            let mut acc = 0.0;
            for (i, v) in cur.iter().enumerate() {
                let span = v / total * TAU;
                if a >= acc && a < acc + span && *v > 0.0 {
                    return Some(i);
                }
                acc += span;
            }
            None
        });
        let active = hover.or(focus.filter(|i| !hidden[*i]));
        let mut acc = start;
        let mut slices = Vec::with_capacity(n);
        for v in cur.iter() {
            let span = v / total * TAU * sweep;
            slices.push((acc, acc + span));
            acc += span;
        }
        let card = t.card;
        for (i, &(a0, a1)) in slices.iter().enumerate() {
            if a1 - a0 < 1e-4 {
                continue;
            }
            let pop = glowkit_core::anim::follow(ui, id.with(("pop", i)), if active == Some(i) { 1.0 } else { 0.0 }, 14.0);
            let mid = (a0 + a1) / 2.0;
            let dir = vec2(mid.cos(), mid.sin());
            let off = if r_in > 0.0 { egui::Vec2::ZERO } else { dir * self.pop_out * pop };
            let ro = r_out * scale + if r_in > 0.0 { self.pop_out * pop } else { 0.0 };
            let ri = r_in * scale;
            let dim = if active.is_some() && active != Some(i) { 0.6 } else { 1.0 };
            let col = color_of(i).gamma_multiply(dim);
            let p = pad.min((a1 - a0) * 0.5);
            if self.rounded && r_in > 0.0 {
                let w = ro - ri;
                let rm = (ro + ri) / 2.0;
                let cap = (w / 2.0) / rm;
                let (b0, b1) = (a0 + p / 2.0 + cap, a1 - p / 2.0 - cap);
                if b1 > b0 {
                    glowkit_core::paint::arc(&painter, c + off, rm, b0, b1, w, col);
                } else {
                    painter.circle_filled(c + off + vec2(mid.cos(), mid.sin()) * rm, w / 2.0 * ((a1 - a0) / (2.0 * cap)).min(1.0), col);
                }
            } else {
                sector(&painter, c + off, ri, ro, a0 + p / 2.0, a1 - p / 2.0, col);
                if r_in == 0.0 && pad == 0.0 && n > 1 {
                    // crisp separators in the card color
                    let d0 = vec2(a0.cos(), a0.sin());
                    painter.line_segment([c + off, c + off + d0 * (ro + 1.0)], Stroke::new(1.5, card));
                }
            }
        }
        if r_in == 0.0 && pad == 0.0 && n > 1 {
            painter.circle_filled(c, 1.5, card);
        }

        // labels
        let font = FontId::proportional(11.5);
        if sweep > 0.98 {
            for (i, &(a0, a1)) in slices.iter().enumerate() {
                let frac = cur[i] / total;
                if a1 - a0 < 0.2 || hidden[i] {
                    continue;
                }
                let mid = (a0 + a1) / 2.0;
                let dir = vec2(mid.cos(), mid.sin());
                match self.labels {
                    PieLabels::None => {}
                    PieLabels::Percent | PieLabels::Value => {
                        let txt = if self.labels == PieLabels::Percent { format_percent(frac as f64) } else { (self.base.format)(self.data[i].value) };
                        let rr = if r_in > 0.0 { (r_in + r_out) / 2.0 } else { r_out * 0.64 };
                        painter.text(c + dir * rr, Align2::CENTER_CENTER, txt, FontId::proportional(12.0), contrast_text(color_of(i)));
                    }
                    PieLabels::Outside => {
                        let p0 = c + dir * (r_out + 3.0);
                        let p1 = c + dir * (r_out + 10.0);
                        let right = dir.x >= 0.0;
                        let p2 = p1 + vec2(if right { 8.0 } else { -8.0 }, 0.0);
                        painter.add(egui::Shape::line(vec![p0, p1, p2], Stroke::new(1.0, t.muted_foreground.gamma_multiply(0.6))));
                        let txt = format!("{} {}", self.data[i].label, format_percent(frac as f64));
                        painter.text(p2 + vec2(if right { 4.0 } else { -4.0 }, 0.0), if right { Align2::LEFT_CENTER } else { Align2::RIGHT_CENTER }, txt, font.clone(), t.foreground.gamma_multiply(0.85));
                    }
                }
            }
        }

        // center label
        if r_in > 26.0 {
            let (title, caption) = match active {
                Some(i) => ((self.base.format)(self.data[i].value), self.data[i].label.clone()),
                None => (self.center_title.clone().unwrap_or_else(|| (self.base.format)(total_raw * ease(Easing::OutCubic, e / 1.1) as f64)), self.center_caption.clone().unwrap_or_else(|| "Total".into())),
            };
            let size = (r_in * 0.42).clamp(14.0, 34.0);
            painter.text(c - vec2(0.0, size * 0.18), Align2::CENTER_CENTER, title, FontId::proportional(size), t.foreground);
            painter.text(c + vec2(0.0, size * 0.62), Align2::CENTER_CENTER, caption, FontId::proportional((size * 0.42).clamp(11.0, 13.0)), t.muted_foreground);
        } else if let (Some(i), true) = (hover, self.base.tooltip) {
            let a = glowkit_core::anim::follow(ui, id.with("tip_a"), 1.0, 16.0);
            let row = TipRow::new(Some(color_of(i)), self.data[i].label.clone(), format!("{}  ·  {}", (self.base.format)(self.data[i].value), format_percent((cur[i] / total) as f64)));
            tooltip(ui, id, resp.hover_pos().unwrap_or(c), "", &[row], a);
        }
        if hover.is_none() {
            glowkit_core::set_state(ui, id.with("tip_a").with("glowkit_follow"), 0.0f32);
        }
        resp
    }
}
