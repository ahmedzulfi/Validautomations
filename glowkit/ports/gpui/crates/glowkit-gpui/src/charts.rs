//! [`Sparkline`] and [`StatTile`]: a KPI card with a count-up number, a delta badge and a
//! sparkline that draws itself in. Port of `glowkit_charts::{Sparkline, StatTile}`.

use crate::anim::{ease, secs, Easing};
use crate::color::with_alpha;
use crate::paint::{glow, polyline};
use crate::text::{sub_id, CountUp};
use crate::theme::ActiveTheme;
use gpui::prelude::*;
use gpui::{
    canvas, div, linear_color_stop, linear_gradient, point, px, Animation, AnimationExt, App, Bounds, ElementId,
    FontWeight, Hsla, PathBuilder, Pixels, Point, SharedString, Window,
};

/// Map `values` into `b` (min at the bottom, max at the top, with a little headroom).
fn layout_points(values: &[f64], b: Bounds<Pixels>) -> Vec<Point<Pixels>> {
    let (lo, hi) = values.iter().fold((f64::MAX, f64::MIN), |(l, h), v| (l.min(*v), h.max(*v)));
    let span = (hi - lo).max(1e-9);
    let n = values.len().max(2) - 1;
    let (x0, y0) = (f32::from(b.origin.x), f32::from(b.origin.y));
    // keep the end dot (r = 3 px + glow) inside the bounds
    let (w, h) = (f32::from(b.size.width) - 6.0, f32::from(b.size.height));
    values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let u = i as f32 / n as f32;
            let k = ((v - lo) / span) as f32;
            point(px(x0 + u * w), px(y0 + 4.0 + (1.0 - k) * (h - 8.0)))
        })
        .collect()
}

/// The prefix of a polyline covering fraction `k` of its x-extent (the last point interpolated).
fn reveal(pts: &[Point<Pixels>], k: f32) -> Vec<Point<Pixels>> {
    if pts.len() < 2 || k >= 1.0 {
        return pts.to_vec();
    }
    let f = k.max(0.0) * (pts.len() - 1) as f32;
    let i = f.floor() as usize;
    let mut out = pts[..=i].to_vec();
    let fr = f - i as f32;
    if i + 1 < pts.len() && fr > 0.0 {
        let (a, b) = (pts[i], pts[i + 1]);
        out.push(point(a.x + (b.x - a.x) * fr, a.y + (b.y - a.y) * fr));
    }
    out
}

/// Paint an area sparkline into `b`, revealed up to `k` (0..1).
pub fn paint_sparkline(window: &mut Window, b: Bounds<Pixels>, values: &[f64], color: Hsla, k: f32, area: bool) {
    if values.len() < 2 {
        return;
    }
    let pts = reveal(&layout_points(values, b), k);
    if pts.len() < 2 {
        return;
    }
    if area {
        let bottom = b.origin.y + b.size.height;
        let mut pb = PathBuilder::fill();
        pb.move_to(point(pts[0].x, bottom));
        for p in &pts {
            pb.line_to(*p);
        }
        pb.line_to(point(pts[pts.len() - 1].x, bottom));
        pb.close();
        if let Ok(path) = pb.build() {
            window.paint_path(
                path,
                linear_gradient(
                    180.,
                    linear_color_stop(with_alpha(color, 0.32), 0.0),
                    linear_color_stop(with_alpha(color, 0.0), 1.0),
                ),
            );
        }
    }
    polyline(window, &pts, 1.8, color);
    let end = pts[pts.len() - 1];
    glow(window, end, 3.0, 8.0, with_alpha(color, 0.7));
    let r = 3.0;
    let ring: Vec<_> = (0..16)
        .map(|i| {
            let a = i as f32 / 16.0 * std::f32::consts::TAU;
            point(end.x + px(r * a.cos()), end.y + px(r * a.sin()))
        })
        .collect();
    let mut pb = PathBuilder::fill();
    pb.add_polygon(&ring, true);
    if let Ok(path) = pb.build() {
        window.paint_path(path, color);
    }
}

/// A small line/area chart that draws itself in from the left when it appears.
#[derive(IntoElement)]
pub struct Sparkline {
    id: ElementId,
    values: Vec<f64>,
    color: Option<Hsla>,
    area: bool,
    height: f32,
    duration: f32,
}

impl Sparkline {
    pub fn new(id: impl Into<ElementId>, values: impl Into<Vec<f64>>) -> Self {
        Self { id: id.into(), values: values.into(), color: None, area: true, height: 48.0, duration: 1.2 }
    }
    pub fn color(mut self, c: Hsla) -> Self {
        self.color = Some(c);
        self
    }
    /// Fill the area under the line with a fading gradient (default on).
    pub fn area(mut self, a: bool) -> Self {
        self.area = a;
        self
    }
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Draw-in duration in seconds.
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s;
        self
    }
}

impl RenderOnce for Sparkline {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = self.color.unwrap_or(cx.theme().accent);
        let (values, area) = (self.values, self.area);
        div().w_full().h(px(self.height)).with_animation(self.id, Animation::new(secs(self.duration)), move |el, k| {
            let values = values.clone();
            let k = ease(Easing::OutCubic, k);
            el.child(
                canvas(|_, _, _| (), move |b, _, w, _| paint_sparkline(w, b, &values, color, k, area))
                    .absolute()
                    .inset_0(),
            )
            .relative()
        })
    }
}

/// A KPI tile: title, count-up value, delta badge (green up / red down, or inverted), caption and
/// a sparkline along the bottom.
#[derive(IntoElement)]
pub struct StatTile {
    id: SharedString,
    title: SharedString,
    value: f64,
    prefix: SharedString,
    suffix: SharedString,
    decimals: usize,
    delta: Option<f64>,
    invert_delta: bool,
    caption: Option<SharedString>,
    spark: Option<Vec<f64>>,
    color: Option<Hsla>,
    width: f32,
}

impl StatTile {
    pub fn new(id: impl Into<SharedString>, title: impl Into<SharedString>, value: f64) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            value,
            prefix: "".into(),
            suffix: "".into(),
            decimals: 0,
            delta: None,
            invert_delta: false,
            caption: None,
            spark: None,
            color: None,
            width: 260.0,
        }
    }
    pub fn prefix(mut self, p: impl Into<SharedString>) -> Self {
        self.prefix = p.into();
        self
    }
    pub fn suffix(mut self, s: impl Into<SharedString>) -> Self {
        self.suffix = s.into();
        self
    }
    pub fn decimals(mut self, d: usize) -> Self {
        self.decimals = d;
        self
    }
    /// Percent change for the badge.
    pub fn delta(mut self, pct: f64) -> Self {
        self.delta = Some(pct);
        self
    }
    /// Treat a decrease as good (churn, latency).
    pub fn invert_delta(mut self, on: bool) -> Self {
        self.invert_delta = on;
        self
    }
    pub fn caption(mut self, c: impl Into<SharedString>) -> Self {
        self.caption = Some(c.into());
        self
    }
    pub fn spark(mut self, values: impl Into<Vec<f64>>) -> Self {
        self.spark = Some(values.into());
        self
    }
    /// Sparkline color (defaults to the delta's color, or the accent).
    pub fn color(mut self, c: Hsla) -> Self {
        self.color = Some(c);
        self
    }
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
}

impl RenderOnce for StatTile {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.theme().clone();
        let good = self.delta.map(|d| (d >= 0.0) != self.invert_delta);
        let delta_c = match good {
            Some(true) => t.success,
            Some(false) => t.destructive,
            None => t.accent,
        };
        let spark_c = self.color.unwrap_or(delta_c);
        let id: ElementId = self.id.clone().into();
        div()
            .id(id.clone())
            .w(px(self.width))
            .flex()
            .flex_col()
            .gap(px(6.))
            .pt(px(16.))
            .rounded(px(t.radius))
            .bg(t.surface())
            .border_1()
            .border_color(t.border)
            .shadow(t.card_shadow(0.0))
            .hover(|s| s.border_color(crate::color::mix(t.border, t.muted_foreground, 0.2)))
            .overflow_hidden()
            .child(div().px(px(16.)).text_size(px(12.5)).text_color(t.muted_foreground).child(self.title))
            .child(
                div().px(px(16.)).child(
                    CountUp::new(format!("{}-count", self.id), self.value)
                        .prefix(self.prefix)
                        .suffix(self.suffix)
                        .decimals(self.decimals)
                        .size(28.0)
                        .duration(1.1)
                        .easing(Easing::OutCubic),
                ),
            )
            .child(
                div()
                    .px(px(16.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .children(self.delta.map(|d| {
                        let arrow = if d >= 0.0 { "\u{25B2}" } else { "\u{25BC}" };
                        div()
                            .px(px(6.))
                            .py(px(1.))
                            .rounded(px(6.))
                            .bg(with_alpha(delta_c, 0.14))
                            .text_color(delta_c)
                            .text_size(px(11.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("{arrow} {:.1}%", d.abs()))
                    }))
                    .children(self.caption.map(|c| div().text_size(px(12.)).text_color(t.muted_foreground).child(c))),
            )
            .children(self.spark.map(|v| Sparkline::new(sub_id(&id, "spark"), v).color(spark_c).height(54.0)))
    }
}
