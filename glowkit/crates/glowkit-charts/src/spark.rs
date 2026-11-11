//! Tiny charts for tables and KPI cards: sparklines, stat tiles and status trackers.

use crate::common::*;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::anim::{ease, stagger, Easing};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// Sparkline flavor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SparkKind {
    /// A line with an end dot.
    #[default]
    Line,
    /// A line with a gradient fill.
    Area,
    /// Tiny columns (negative values in the destructive color).
    Bar,
}

/// A tiny chart without axes, for tables and cards.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::{Sparkline, SparkKind};
/// ui.add(Sparkline::new(&[3.0, 5.0, 4.0, 7.0, 6.0, 9.0]).kind(SparkKind::Area));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Sparkline<'a> {
    values: &'a [f64],
    kind: SparkKind,
    size: egui::Vec2,
    color: Option<Color32>,
    end_dot: bool,
    animate: bool,
    curve: Curve,
}

impl<'a> Sparkline<'a> {
    /// A sparkline of `values`.
    pub fn new(values: &'a [f64]) -> Self {
        Self { values, kind: SparkKind::Line, size: vec2(120.0, 32.0), color: None, end_dot: true, animate: true, curve: Curve::Monotone }
    }
    /// Line, area or bars.
    pub fn kind(mut self, kind: SparkKind) -> Self {
        self.kind = kind;
        self
    }
    /// Size in points (default 120 × 32).
    pub fn size(mut self, size: egui::Vec2) -> Self {
        self.size = size;
        self
    }
    /// Color (default: the theme accent).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Dot on the last value (default `true`, lines only).
    pub fn end_dot(mut self, on: bool) -> Self {
        self.end_dot = on;
        self
    }
    /// Entry animation (default `true`).
    pub fn animate(mut self, on: bool) -> Self {
        self.animate = on;
        self
    }
    /// Interpolation (default monotone).
    pub fn curve(mut self, c: Curve) -> Self {
        self.curve = c;
        self
    }
}

/// Paint a sparkline into `rect` (shared with [`StatTile`]).
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_spark(ui: &Ui, painter: &egui::Painter, rect: Rect, values: &[f64], kind: SparkKind, color: Color32, end_dot: bool, progress: f32, curve: Curve) {
    let t = Theme::of(ui);
    let n = values.len();
    if n == 0 {
        return;
    }
    let (lo, hi) = values.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(*v), b.max(*v)));
    let (lo, hi) = if kind == SparkKind::Bar { (lo.min(0.0), hi.max(0.0)) } else { (lo, hi) };
    let pad = if kind == SparkKind::Bar { 0.0 } else { (hi - lo) * 0.08 };
    let (lo, hi) = (lo - pad, hi + pad + if (hi - lo).abs() < 1e-12 { 1.0 } else { 0.0 });
    let inner = rect.shrink2(vec2(if end_dot && kind != SparkKind::Bar { 4.0 } else { 0.0 }, 3.0));
    match kind {
        SparkKind::Bar => {
            let bw = inner.width() / n as f32;
            let base = val_y(inner, lo, hi, 0.0);
            for (i, v) in values.iter().enumerate() {
                let g = ease(Easing::OutCubic, stagger(progress * 1.4, i, 0.4 / n as f32, 0.6));
                let y = val_y(inner, lo, hi, v * g as f64);
                let x0 = inner.left() + i as f32 * bw + bw * 0.15;
                let r = Rect::from_min_max(pos2(x0, y.min(base)), pos2(x0 + bw * 0.7, y.max(base).max(y.min(base) + 1.0)));
                let col = if *v < 0.0 { t.destructive } else { color };
                painter.rect_filled(r, CornerRadius::same((bw * 0.2).min(2.0) as u8), col);
            }
        }
        _ => {
            let pts: Vec<Pos2> = values.iter().enumerate().map(|(i, v)| pos2(cat_x(inner, n, i, false), val_y(inner, lo, hi, *v))).collect();
            let line = curve_points(&pts, curve);
            let clip = Rect::from_min_max(rect.min - vec2(2.0, 2.0), pos2(rect.left() + (rect.width() + 4.0) * ease(Easing::InOutCubic, progress), rect.bottom() + 2.0));
            let lp = painter.with_clip_rect(clip.intersect(painter.clip_rect()));
            if kind == SparkKind::Area {
                let lower: Vec<Pos2> = line.iter().map(|p| pos2(p.x, rect.bottom())).collect();
                band_fill(&lp, &line, &lower, rect, color, 0.35, 0.0);
            }
            lp.add(Shape::line(line, Stroke::new(1.6, color)));
            if end_dot && progress > 0.98 {
                let p = *pts.last().unwrap();
                painter.circle_filled(p, 5.5, with_alpha(color, 0.22));
                painter.circle_filled(p, 2.6, color);
            }
        }
    }
}

impl Widget for Sparkline<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, resp) = ui.allocate_exact_size(self.size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let t = Theme::of(ui);
        let e = entry_time(ui, resp.id, self.animate, 1.0);
        let painter = ui.painter_at(rect.expand(6.0));
        paint_spark(ui, &painter, rect, self.values, self.kind, self.color.unwrap_or(t.accent), self.end_dot, (e / 0.9).min(1.0), self.curve);
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// StatTile
// ---------------------------------------------------------------------------------------------

/// A KPI card: title, big number that counts up, a colored delta badge and a sparkline.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::StatTile;
/// ui.add(StatTile::new("Revenue", 45_231.89).prefix("$").delta(20.1).caption("vs last month")
///     .spark(&[12.0, 18.0, 15.0, 22.0, 28.0, 26.0, 34.0]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct StatTile<'a> {
    title: String,
    value: f64,
    prefix: String,
    suffix: String,
    decimals: usize,
    delta: Option<f64>,
    invert_delta: bool,
    caption: String,
    spark: Option<&'a [f64]>,
    kind: SparkKind,
    color: Option<Color32>,
    size: egui::Vec2,
    animate: bool,
}

impl<'a> StatTile<'a> {
    /// A tile titled `title` showing `value`.
    pub fn new(title: impl Into<String>, value: f64) -> Self {
        Self {
            title: title.into(),
            value,
            prefix: String::new(),
            suffix: String::new(),
            decimals: 0,
            delta: None,
            invert_delta: false,
            caption: String::new(),
            spark: None,
            kind: SparkKind::Area,
            color: None,
            size: vec2(260.0, 150.0),
            animate: true,
        }
    }
    /// Text before the number (e.g. `"$"`).
    pub fn prefix(mut self, p: impl Into<String>) -> Self {
        self.prefix = p.into();
        self
    }
    /// Text after the number (e.g. `"%"`).
    pub fn suffix(mut self, s: impl Into<String>) -> Self {
        self.suffix = s.into();
        self
    }
    /// Decimals shown (default 0).
    pub fn decimals(mut self, d: usize) -> Self {
        self.decimals = d;
        self
    }
    /// Percent change for the badge (positive = green ▲, negative = red ▼).
    pub fn delta(mut self, pct: f64) -> Self {
        self.delta = Some(pct);
        self
    }
    /// Treat a decrease as good (e.g. churn, latency).
    pub fn invert_delta(mut self, on: bool) -> Self {
        self.invert_delta = on;
        self
    }
    /// Small muted text next to the badge.
    pub fn caption(mut self, c: impl Into<String>) -> Self {
        self.caption = c.into();
        self
    }
    /// Sparkline values at the bottom of the card.
    pub fn spark(mut self, values: &'a [f64]) -> Self {
        self.spark = Some(values);
        self
    }
    /// Sparkline flavor (default area).
    pub fn spark_kind(mut self, k: SparkKind) -> Self {
        self.kind = k;
        self
    }
    /// Sparkline color (defaults to the delta's color, or the accent).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Card size (default 260 × 150).
    pub fn size(mut self, s: egui::Vec2) -> Self {
        self.size = s;
        self
    }
    /// Count-up and draw-in animation (default `true`).
    pub fn animate(mut self, on: bool) -> Self {
        self.animate = on;
        self
    }
}

impl Widget for StatTile<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(self.size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        let painter = ui.painter_at(rect.expand(8.0));
        let hov = glowkit_core::anim::follow(ui, id.with("hov"), if resp.hovered() { 1.0 } else { 0.0 }, 14.0);
        let r = t.corner();
        let shadow = if t.dark { Color32::from_black_alpha(90) } else { Color32::from_black_alpha(18) };
        glowkit_core::paint::soft_shadow(&painter, rect.translate(vec2(0.0, 2.0)), t.radius, 10.0 + 8.0 * hov, egui::Vec2::ZERO, shadow);
        painter.rect_filled(rect, r, if t.dark { glowkit_core::color::shade(t.card, 0.015) } else { t.card });
        painter.rect_stroke(rect, r, Stroke::new(1.0, t.border.lerp_to_gamma(t.muted_foreground, 0.15 * hov)), StrokeKind::Inside);
        let pad = 16.0;
        let e = entry_time(ui, id, self.animate, 1.3);
        let shown = tween(ui, id, &[self.value as f32])[0] as f64 * ease(Easing::OutCubic, e / 1.1) as f64;
        painter.text(rect.left_top() + vec2(pad, pad), Align2::LEFT_TOP, &self.title, FontId::proportional(12.5), t.muted_foreground);
        let num = format!("{}{}{}", self.prefix, format_thousands(shown, self.decimals), self.suffix);
        let vr = painter.text(rect.left_top() + vec2(pad, pad + 22.0), Align2::LEFT_TOP, num, FontId::proportional(26.0), t.foreground);
        let good_color = t.success;
        let bad_color = t.destructive;
        let mut accent = self.color.unwrap_or(t.accent);
        let row_y = vr.bottom() + 14.0;
        if let Some(d) = self.delta {
            let up = d >= 0.0;
            let good = up != self.invert_delta;
            let col = if good { good_color } else { bad_color };
            if self.color.is_none() {
                accent = col;
            }
            let txt = format!("{:.1}%", d.abs());
            let font = FontId::proportional(11.5);
            let tw = text_width(&painter, &txt, &font);
            let badge = Rect::from_min_size(pos2(rect.left() + pad, row_y - 10.0), vec2(tw + 26.0, 20.0));
            painter.rect_filled(badge, CornerRadius::same(10), with_alpha(col, if t.dark { 0.16 } else { 0.12 }));
            triangle(&painter, pos2(badge.left() + 10.0, badge.center().y), 7.0, up, col);
            painter.text(pos2(badge.left() + 17.0, badge.center().y), Align2::LEFT_CENTER, txt, font, col);
            if !self.caption.is_empty() {
                painter.text(pos2(badge.right() + 8.0, badge.center().y), Align2::LEFT_CENTER, &self.caption, FontId::proportional(11.5), t.muted_foreground);
            }
        } else if !self.caption.is_empty() {
            painter.text(pos2(rect.left() + pad, row_y), Align2::LEFT_CENTER, &self.caption, FontId::proportional(11.5), t.muted_foreground);
        }
        if let Some(sp) = self.spark {
            // sparkline in the top-right corner area if the tile is wide, else along the bottom
            let sr = Rect::from_min_max(pos2(rect.right() - pad - (rect.width() * 0.38), rect.top() + pad + 22.0), pos2(rect.right() - pad, vr.bottom() + 2.0));
            paint_spark(ui, &painter, sr, sp, self.kind, accent, true, (e / 1.0).min(1.0), Curve::Monotone);
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Tracker
// ---------------------------------------------------------------------------------------------

/// Status of one [`Tracker`] block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Status {
    /// Operational (success color).
    #[default]
    Ok,
    /// Degraded (warning color).
    Warning,
    /// Down (destructive color).
    Down,
    /// No data (muted).
    Empty,
}

/// One block of a [`Tracker`].
#[derive(Clone, Debug, PartialEq)]
pub struct TrackerBlock {
    /// Status (sets the color unless overridden).
    pub status: Status,
    /// Tooltip text.
    pub tooltip: String,
    /// Color override.
    pub color: Option<Color32>,
}

impl TrackerBlock {
    /// A block with a status and tooltip.
    pub fn new(status: Status, tooltip: impl Into<String>) -> Self {
        Self { status, tooltip: tooltip.into(), color: None }
    }
    /// Override the color.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

/// A row of small colored blocks, e.g. 90 days of uptime (Tremor-style tracker). Blocks fade in
/// left to right, the hovered block lifts and shows its tooltip.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::{Tracker, TrackerBlock, Status};
/// let blocks: Vec<_> = (0..60).map(|i| TrackerBlock::new(if i == 41 { Status::Down } else { Status::Ok }, format!("Day {i}"))).collect();
/// ui.add(Tracker::new(&blocks).height(32.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Tracker<'a> {
    blocks: &'a [TrackerBlock],
    height: f32,
    gap: f32,
    radius: f32,
    animate: bool,
}

impl<'a> Tracker<'a> {
    /// A tracker of `blocks`, filling the available width.
    pub fn new(blocks: &'a [TrackerBlock]) -> Self {
        Self { blocks, height: 32.0, gap: 3.0, radius: 2.0, animate: true }
    }
    /// Block height (default 32).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Gap between blocks (default 3).
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g;
        self
    }
    /// Corner radius (default 2).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r;
        self
    }
    /// Entry animation (default `true`).
    pub fn animate(mut self, on: bool) -> Self {
        self.animate = on;
        self
    }
}

impl Widget for Tracker<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(vec2(w, self.height + 4.0), Sense::hover());
        let n = self.blocks.len();
        if n == 0 || !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        let painter = ui.painter_at(rect.expand(4.0));
        let bw = ((rect.width() + self.gap) / n as f32 - self.gap).max(1.0);
        let e = entry_time(ui, id, self.animate, 1.2);
        let hover = resp.hover_pos().map(|p| (((p.x - rect.left()) / (bw + self.gap)).floor().max(0.0) as usize).min(n - 1));
        for (i, b) in self.blocks.iter().enumerate() {
            let col = b.color.unwrap_or(match b.status {
                Status::Ok => t.success,
                Status::Warning => t.warning,
                Status::Down => t.destructive,
                Status::Empty => with_alpha(t.muted_foreground, 0.2),
            });
            let a = ease(Easing::OutCubic, stagger(e, i, 0.6 / n as f32, 0.35));
            let lift = glowkit_core::anim::follow(ui, id.with(("lift", i)), if hover == Some(i) { 1.0 } else { 0.0 }, 18.0);
            let x = rect.left() + i as f32 * (bw + self.gap);
            let h = self.height * (0.4 + 0.6 * a);
            let r = Rect::from_min_size(pos2(x, rect.top() + 2.0 + (self.height - h) - 2.0 * lift), vec2(bw, h));
            let dim = if hover.is_some() && hover != Some(i) { 0.75 } else { 1.0 };
            painter.rect_filled(r, CornerRadius::same(self.radius.round() as u8), col.gamma_multiply(a * dim));
        }
        if let Some(i) = hover {
            let b = &self.blocks[i];
            let col = b.color.unwrap_or(match b.status {
                Status::Ok => t.success,
                Status::Warning => t.warning,
                Status::Down => t.destructive,
                Status::Empty => t.muted_foreground,
            });
            let x = rect.left() + i as f32 * (bw + self.gap) + bw / 2.0;
            tooltip(ui, id, pos2(x, rect.center().y), "", &[TipRow::new(Some(col), b.tooltip.clone(), "")], 1.0);
        }
        resp
    }
}
