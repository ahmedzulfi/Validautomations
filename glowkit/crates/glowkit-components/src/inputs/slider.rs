use super::kit;
use egui::{
    pos2, vec2, Color32, CornerRadius, CursorIcon, FontId, Id, Pos2, Rect, Response, Sense, Stroke, Ui, Widget,
};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;
use std::ops::RangeInclusive;

/// Shared slider look & behavior.
#[derive(Clone)]
struct SliderStyle {
    range: RangeInclusive<f32>,
    step: Option<f32>,
    width: Option<f32>,
    tooltip: bool,
    ticks: Option<usize>,
    decimals: usize,
    prefix: String,
    suffix: String,
    color: Option<Color32>,
    disabled: bool,
}

impl SliderStyle {
    fn new(range: RangeInclusive<f32>) -> Self {
        Self {
            range,
            step: None,
            width: None,
            tooltip: true,
            ticks: None,
            decimals: 0,
            prefix: String::new(),
            suffix: String::new(),
            color: None,
            disabled: false,
        }
    }
    fn fmt(&self, v: f32) -> String {
        format!("{}{:.*}{}", self.prefix, self.decimals, v, self.suffix)
    }
    fn quantize(&self, v: f32) -> f32 {
        let (a, b) = (*self.range.start(), *self.range.end());
        let v = v.clamp(a.min(b), a.max(b));
        match self.step {
            Some(s) if s > 0.0 => (a + ((v - a) / s).round() * s).clamp(a.min(b), a.max(b)),
            _ => v,
        }
    }
    fn to_t(&self, v: f32) -> f32 {
        glowkit_core::anim::unlerp(*self.range.start(), *self.range.end(), v)
    }
    fn value_at_x(&self, track: Rect, x: f32) -> f32 {
        let t = ((x - track.min.x) / track.width().max(1.0)).clamp(0.0, 1.0);
        self.quantize(glowkit_core::anim::lerp(*self.range.start(), *self.range.end(), t))
    }
}

macro_rules! style_setters {
    () => {
        /// Snap values to multiples of `step`.
        pub fn step(mut self, step: f32) -> Self {
            self.style.step = Some(step);
            self
        }
        /// Track width in points (default `min(available, 320)`).
        pub fn width(mut self, w: f32) -> Self {
            self.style.width = Some(w);
            self
        }
        /// Show a value bubble above the thumb while hovering / dragging (default on).
        pub fn tooltip(mut self, on: bool) -> Self {
            self.style.tooltip = on;
            self
        }
        /// Draw `n` evenly spaced tick marks under the track.
        pub fn ticks(mut self, n: usize) -> Self {
            self.style.ticks = Some(n);
            self
        }
        /// Digits after the decimal point in the tooltip.
        pub fn decimals(mut self, d: usize) -> Self {
            self.style.decimals = d;
            self
        }
        /// Text before the value in the tooltip (e.g. `"$"`).
        pub fn prefix(mut self, p: impl Into<String>) -> Self {
            self.style.prefix = p.into();
            self
        }
        /// Text after the value in the tooltip (e.g. `"%"`).
        pub fn suffix(mut self, s: impl Into<String>) -> Self {
            self.style.suffix = s.into();
            self
        }
        /// Fill color (default: theme primary).
        pub fn color(mut self, c: Color32) -> Self {
            self.style.color = Some(c);
            self
        }
        /// Dim and ignore input.
        pub fn disabled(mut self, d: bool) -> Self {
            self.style.disabled = d;
            self
        }
    };
}

/// A slider with a rounded track, primary fill, a thumb with an animated halo, and a value
/// bubble that pops up while hovering or dragging.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut volume = 50.0;
/// use glowkit_components::Slider;
/// ui.add(Slider::new(&mut volume, 0.0..=100.0).suffix("%"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Slider<'a> {
    value: &'a mut f32,
    style: SliderStyle,
}

impl<'a> Slider<'a> {
    /// A slider editing `value` within `range`.
    pub fn new(value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        Self { value, style: SliderStyle::new(range) }
    }
    style_setters!();
}

/// A two-thumb slider selecting a `[low, high]` interval.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut price = [20.0, 80.0];
/// use glowkit_components::RangeSlider;
/// ui.add(RangeSlider::new(&mut price, 0.0..=100.0).prefix("$"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct RangeSlider<'a> {
    values: &'a mut [f32; 2],
    style: SliderStyle,
}

impl<'a> RangeSlider<'a> {
    /// A range slider editing `values` (`[low, high]`) within `range`.
    pub fn new(values: &'a mut [f32; 2], range: RangeInclusive<f32>) -> Self {
        Self { values, style: SliderStyle::new(range) }
    }
    style_setters!();
}

const THUMB_R: f32 = 8.0;
const TRACK_H: f32 = 6.0;

fn layout(ui: &mut Ui, style: &SliderStyle) -> (Rect, Rect, Response) {
    let w = style.width.unwrap_or_else(|| ui.available_width().min(320.0));
    let h = 22.0 + if style.ticks.is_some() { 14.0 } else { 0.0 };
    let (rect, resp) =
        ui.allocate_exact_size(vec2(w, h), if style.disabled { Sense::hover() } else { Sense::click_and_drag() });
    let resp = if style.disabled { resp } else { resp.on_hover_cursor(CursorIcon::PointingHand) };
    let cy = rect.min.y + 11.0;
    let track = Rect::from_min_max(
        pos2(rect.min.x + THUMB_R, cy - TRACK_H / 2.0),
        pos2(rect.max.x - THUMB_R, cy + TRACK_H / 2.0),
    );
    (rect, track, resp)
}

fn paint_track(ui: &Ui, track: Rect, a: f32, b: f32, style: &SliderStyle) {
    let t = Theme::of(ui);
    let painter = ui.painter();
    let alpha = if style.disabled { 0.5 } else { 1.0 };
    let r = CornerRadius::same((TRACK_H / 2.0) as u8);
    let bg = if t.dark { t.secondary } else { t.muted.lerp_to_gamma(t.input, 0.6) };
    painter.rect_filled(track.expand2(vec2(THUMB_R * 0.5, 0.0)), r, bg.gamma_multiply(alpha));
    let x0 = track.min.x + track.width() * a;
    let x1 = track.min.x + track.width() * b;
    let fill =
        Rect::from_min_max(pos2(x0 - if a <= 0.0 { THUMB_R * 0.5 } else { 0.0 }, track.min.y), pos2(x1, track.max.y));
    painter.rect_filled(fill, r, style.color.unwrap_or(t.primary).gamma_multiply(alpha));
    if let Some(n) = style.ticks {
        let n = n.max(2);
        for i in 0..n {
            let x = track.min.x + track.width() * i as f32 / (n - 1) as f32;
            painter.line_segment(
                [pos2(x, track.max.y + 8.0), pos2(x, track.max.y + 12.0)],
                Stroke::new(1.0, with_alpha(t.muted_foreground, 0.6)),
            );
        }
    }
}

fn paint_thumb(ui: &Ui, id: Id, c: Pos2, active: bool, hover: bool, value: &str, style: &SliderStyle) {
    let t = Theme::of(ui);
    let painter = ui.painter();
    let h = kit::anim(ui, id.with("h"), hover || active, 0.14);
    let a = glowkit_core::anim::spring(ui, id.with("a"), if active { 1.0 } else { 0.0 }, 400.0, 26.0);
    let accent = style.color.unwrap_or(t.primary);
    if h > 0.0 {
        painter.circle_filled(c, THUMB_R + 4.0 * h, with_alpha(t.ring, 0.25 * h));
    }
    glowkit_core::paint::soft_shadow(
        painter,
        Rect::from_center_size(c, vec2(THUMB_R * 2.0, THUMB_R * 2.0)),
        THUMB_R,
        4.0,
        vec2(0.0, 1.0),
        Color32::from_black_alpha(if t.dark { 60 } else { 30 }),
    );
    let r = THUMB_R * (1.0 + 0.08 * a);
    painter.circle(c, r, t.background, Stroke::new(1.5, accent));
    if style.tooltip && h > 0.01 && !style.disabled {
        let font = FontId::proportional(12.0);
        let g = painter.layout_no_wrap(value.to_owned(), font, t.primary_foreground);
        let size = vec2(g.size().x + 14.0, 22.0);
        let lift = 8.0 * h;
        let center = pos2(c.x, c.y - THUMB_R - 6.0 - size.y / 2.0 - lift + 6.0);
        let br = Rect::from_center_size(center, size * (0.85 + 0.15 * h));
        let fill = with_alpha(t.primary, h);
        let p = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, id.with("tip")));
        glowkit_core::paint::soft_shadow(&p, br, 6.0, 8.0, vec2(0.0, 2.0), Color32::from_black_alpha((50.0 * h) as u8));
        p.rect_filled(br, CornerRadius::same(6), fill);
        let tri = vec![pos2(c.x - 4.0, br.max.y - 0.5), pos2(c.x + 4.0, br.max.y - 0.5), pos2(c.x, br.max.y + 4.0)];
        p.add(egui::Shape::convex_polygon(tri, fill, Stroke::NONE));
        p.galley(br.center() - g.size() / 2.0, g, with_alpha(t.primary_foreground, h));
    }
}

impl Widget for Slider<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (_rect, track, mut resp) = layout(ui, &self.style);
        if let Some(p) = resp.interact_pointer_pos() {
            if resp.dragged() || resp.clicked() {
                let v = self.style.value_at_x(track, p.x);
                if v != *self.value {
                    *self.value = v;
                    resp.mark_changed();
                }
            }
        }
        if resp.has_focus() {
            let step = self.style.step.unwrap_or((*self.style.range.end() - *self.style.range.start()) / 100.0);
            let d = ui.input(|i| {
                (i.key_pressed(egui::Key::ArrowRight) as i32 - i.key_pressed(egui::Key::ArrowLeft) as i32) as f32
            });
            if d != 0.0 {
                *self.value = self.style.quantize(*self.value + d * step);
                resp.mark_changed();
            }
        }
        if ui.is_rect_visible(track.expand(20.0)) {
            let tv = self.style.to_t(*self.value);
            let tv = glowkit_core::anim::follow(ui, resp.id.with("vis"), tv, 30.0);
            paint_track(ui, track, 0.0, tv, &self.style);
            let c = pos2(track.min.x + track.width() * tv, track.center().y);
            paint_thumb(ui, resp.id, c, resp.dragged(), resp.hovered(), &self.style.fmt(*self.value), &self.style);
        }
        resp
    }
}

impl Widget for RangeSlider<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (_rect, track, mut resp) = layout(ui, &self.style);
        let active_id = resp.id.with("active");
        let mut active: usize = glowkit_core::state(ui, active_id, || 0);
        let x_of = |v: f32| track.min.x + track.width() * self.style.to_t(v);
        if let Some(p) = resp.interact_pointer_pos() {
            if resp.drag_started() || resp.clicked() || (resp.is_pointer_button_down_on() && !resp.dragged()) {
                let d0 = (p.x - x_of(self.values[0])).abs();
                let d1 = (p.x - x_of(self.values[1])).abs();
                active = if d0 < d1 || (d0 == d1 && p.x < x_of(self.values[0])) { 0 } else { 1 };
                glowkit_core::set_state(ui, active_id, active);
            }
            if resp.dragged() || resp.clicked() {
                let mut v = self.style.value_at_x(track, p.x);
                if active == 0 {
                    v = v.min(self.values[1]);
                } else {
                    v = v.max(self.values[0]);
                }
                if v != self.values[active] {
                    self.values[active] = v;
                    resp.mark_changed();
                }
            }
        }
        if ui.is_rect_visible(track.expand(20.0)) {
            let a = glowkit_core::anim::follow(ui, resp.id.with("va"), self.style.to_t(self.values[0]), 30.0);
            let b = glowkit_core::anim::follow(ui, resp.id.with("vb"), self.style.to_t(self.values[1]), 30.0);
            paint_track(ui, track, a, b, &self.style);
            let hover_pos = resp.hover_pos();
            for (i, tv) in [a, b].into_iter().enumerate() {
                let c = pos2(track.min.x + track.width() * tv, track.center().y);
                let near = hover_pos.map(|p| (p.x - c.x).abs() < 14.0).unwrap_or(false);
                paint_thumb(
                    ui,
                    resp.id.with(i),
                    c,
                    resp.dragged() && active == i,
                    near,
                    &self.style.fmt(self.values[i]),
                    &self.style,
                );
            }
        }
        resp
    }
}
