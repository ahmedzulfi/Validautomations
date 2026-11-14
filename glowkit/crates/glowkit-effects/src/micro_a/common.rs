//! Shared helpers for the micro-interaction blocks: the [`ControlSlider`] parameter row, soft
//! surfaces, small painted icons and an idle-autoplay clock.

use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Id, Painter, Pos2, Rect, Response, Sense, Shape,
    Stroke, Ui,
};
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::{paint, Theme};
use std::ops::RangeInclusive;

// ---------------------------------------------------------------------------------------------
// Palette helpers

/// Surface color for cards sitting on a `muted` stage: white in light themes, a lifted tone in
/// dark ones (dark themes' `card` is darker than `muted`, which would look like a hole).
pub(crate) fn surface(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.muted, 0.07)
    } else {
        t.card
    }
}

/// A raised knob / selection color that stands out from [`track`] in light and dark themes.
pub(crate) fn raised(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.muted, 0.17)
    } else {
        t.card
    }
}

/// A recessed track color (slightly darker / lighter than the stage).
pub(crate) fn track(t: &Theme) -> Color32 {
    mix(t.muted, t.foreground, if t.dark { 0.09 } else { 0.065 })
}

/// A stronger neutral used for "filled" parts of tracks.
pub(crate) fn track_fill(t: &Theme) -> Color32 {
    mix(t.muted, t.foreground, if t.dark { 0.2 } else { 0.16 })
}

/// Rounded-rect corner radius from a float.
pub(crate) fn cr(r: f32) -> CornerRadius {
    CornerRadius::same(r.round().clamp(0.0, 255.0) as u8)
}

/// A soft elevated card: diffuse shadow, surface fill and an optional hairline.
pub(crate) fn soft_card(painter: &Painter, t: &Theme, rect: Rect, radius: f32, elevation: f32, stroke: bool) {
    soft_card_fill(painter, t, rect, radius, elevation, stroke, surface(t));
}

/// [`soft_card`] with an explicit fill.
pub(crate) fn soft_card_fill(
    painter: &Painter,
    t: &Theme,
    rect: Rect,
    radius: f32,
    elevation: f32,
    stroke: bool,
    fill: Color32,
) {
    let radius = radius.min(rect.height() / 2.0).min(rect.width() / 2.0).max(0.0);
    if elevation > 0.0 {
        let sc = if t.dark { Color32::from_black_alpha(110) } else { Color32::from_rgba_unmultiplied(30, 30, 50, 18) };
        paint::soft_shadow(painter, rect, radius, 16.0 * elevation, vec2(0.0, 4.0 * elevation), sc);
    }
    let s = if stroke || t.dark {
        Stroke::new(1.0, with_alpha(t.border, if stroke { 1.0 } else { 0.6 }))
    } else {
        Stroke::NONE
    };
    painter.rect(rect, cr(radius), fill, s, egui::StrokeKind::Inside);
}

/// Seconds since the widget was last interacted with (or first shown). Used to start autoplay
/// only while the user leaves the component alone.
pub(crate) fn idle_for(ui: &Ui, id: Id, interacting: bool) -> f32 {
    let now = glowkit_core::time(ui);
    let since = glowkit_core::with_state(
        ui,
        id.with("micro_idle"),
        || now,
        |t| {
            if interacting {
                *t = now;
            }
            *t
        },
    );
    now - since
}

// ---------------------------------------------------------------------------------------------
// Icons (painted, so they look identical everywhere)

/// Solid play triangle centered in a box of `size`.
pub(crate) fn icon_play(painter: &Painter, c: Pos2, size: f32, color: Color32) {
    let h = size * 0.5;
    let pts =
        vec![pos2(c.x - h * 0.55, c.y - h * 0.62), pos2(c.x + h * 0.7, c.y), pos2(c.x - h * 0.55, c.y + h * 0.62)];
    painter.add(Shape::convex_polygon(pts, color, Stroke::NONE));
}

/// Two pause bars.
pub(crate) fn icon_pause(painter: &Painter, c: Pos2, size: f32, color: Color32) {
    let w = size * 0.13;
    let h = size * 0.34;
    for dx in [-1.0, 1.0] {
        let r = Rect::from_center_size(pos2(c.x + dx * size * 0.13, c.y), vec2(w, h * 2.0));
        painter.rect_filled(r, cr(w * 0.4), color);
    }
}

/// Skip-back / skip-forward icon: an outlined triangle plus a bar. `dir` = 1 for forward.
pub(crate) fn icon_skip(painter: &Painter, c: Pos2, size: f32, dir: f32, color: Color32) {
    let h = size * 0.3;
    let s = Stroke::new((size * 0.08).max(1.2), color);
    let tip = pos2(c.x + dir * h * 0.6, c.y);
    let back = c.x - dir * h * 0.6;
    let tri = vec![pos2(back, c.y - h), tip, pos2(back, c.y + h), pos2(back, c.y - h)];
    painter.add(Shape::line(tri, s));
    painter.line_segment([pos2(tip.x + dir * h * 0.1, c.y - h), pos2(tip.x + dir * h * 0.1, c.y + h)], s);
}

/// A microphone glyph.
pub(crate) fn icon_mic(painter: &Painter, c: Pos2, size: f32, color: Color32) {
    let w = size * 0.32;
    let body = Rect::from_center_size(pos2(c.x, c.y - size * 0.12), vec2(w, size * 0.5));
    painter.rect_filled(body, cr(w / 2.0), color);
    let s = Stroke::new((size * 0.08).max(1.3), color);
    let r = size * 0.27;
    let cy = c.y - size * 0.05;
    let arc: Vec<Pos2> = (0..=12)
        .map(|i| {
            let a = std::f32::consts::PI * i as f32 / 12.0;
            pos2(c.x + r * a.cos(), cy + r * a.sin())
        })
        .collect();
    painter.add(Shape::line(arc, s));
    painter.line_segment([pos2(c.x, cy + r), pos2(c.x, c.y + size * 0.36)], s);
}

/// A thin "x".
pub(crate) fn icon_x(painter: &Painter, c: Pos2, size: f32, width: f32, color: Color32) {
    let h = size * 0.5;
    let s = Stroke::new(width, color);
    painter.line_segment([pos2(c.x - h, c.y - h), pos2(c.x + h, c.y + h)], s);
    painter.line_segment([pos2(c.x - h, c.y + h), pos2(c.x + h, c.y - h)], s);
}

/// An outlined heart (or filled when `fill` is set).
pub(crate) fn icon_heart(painter: &Painter, c: Pos2, size: f32, color: Color32, fill: f32) {
    let pts: Vec<Pos2> = (0..48)
        .map(|i| {
            let a = std::f32::consts::TAU * i as f32 / 48.0;
            let x = 16.0 * a.sin().powi(3);
            let y = 13.0 * a.cos() - 5.0 * (2.0 * a).cos() - 2.0 * (3.0 * a).cos() - (4.0 * a).cos();
            pos2(c.x + x / 34.0 * size, c.y - y / 34.0 * size - size * 0.03)
        })
        .collect();
    if fill > 0.01 {
        // the heart is star-shaped around a point slightly above center: fan triangulation works
        let mut m = egui::epaint::Mesh::default();
        let fc = with_alpha(color, fill);
        m.colored_vertex(pos2(c.x, c.y - size * 0.05), fc);
        for p in &pts {
            m.colored_vertex(*p, fc);
        }
        for i in 0..pts.len() as u32 {
            m.add_triangle(0, 1 + i, 1 + (i + 1) % pts.len() as u32);
        }
        painter.add(Shape::mesh(m));
    }
    let mut closed = pts;
    closed.push(closed[0]);
    painter.add(Shape::line(closed, Stroke::new((size * 0.08).max(1.2), color)));
}

/// A smiley with a small "+" (add reaction).
pub(crate) fn icon_add_reaction(painter: &Painter, c: Pos2, size: f32, color: Color32) {
    let r = size * 0.4;
    let s = Stroke::new((size * 0.075).max(1.2), color);
    // circle with a gap at the top-right for the plus
    let pts: Vec<Pos2> = (0..=40)
        .map(|i| {
            let a = -0.2 + (std::f32::consts::TAU - 1.3) * i as f32 / 40.0;
            pos2(c.x + r * a.cos(), c.y + r * a.sin())
        })
        .collect();
    painter.add(Shape::line(pts, s));
    painter.circle_filled(pos2(c.x - r * 0.35, c.y - r * 0.2), size * 0.055, color);
    painter.circle_filled(pos2(c.x + r * 0.35, c.y - r * 0.2), size * 0.055, color);
    let smile: Vec<Pos2> = (0..=10)
        .map(|i| {
            let a = 0.5 + 2.14 * i as f32 / 10.0;
            pos2(c.x + r * 0.55 * a.cos(), c.y + r * 0.1 + r * 0.45 * a.sin())
        })
        .collect();
    painter.add(Shape::line(smile, s));
    let p = pos2(c.x + r * 0.95, c.y - r * 0.95);
    painter.line_segment([p - vec2(r * 0.32, 0.0), p + vec2(r * 0.32, 0.0)], s);
    painter.line_segment([p - vec2(0.0, r * 0.32), p + vec2(0.0, r * 0.32)], s);
}

/// Format `m:ss`.
pub(crate) fn mmss(secs: f32) -> String {
    let s = secs.max(0.0) as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

// ---------------------------------------------------------------------------------------------
// ControlSlider

#[derive(Clone, Copy, Debug, PartialEq)]
enum ValueFormat {
    Integer,
    Decimals(usize),
}

#[derive(Clone, Copy, Default)]
struct SliderDrag {
    anchor_value: f32,
    anchor_x: f32,
    moved: bool,
}

/// A parameter row in the style of a soft design-tool side panel: a full-width rounded bar
/// whose darker fill shows the value, with the label on the left and the formatted value on the
/// right. Press and drag anywhere on the bar to change it (relative drag, so it never jumps);
/// a plain click sets the value where you clicked. The fill glides with a spring.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut bounce = 50.0_f32; let mut corner = 14.0_f32; let mut trail = 1.2_f32;
/// use glowkit_effects::ControlSlider;
/// ui.add(ControlSlider::new("Bounce", &mut bounce, 0.0..=100.0));
/// ui.add(ControlSlider::new("Corner", &mut corner, 0.0..=32.0).suffix("px"));
/// ui.add(ControlSlider::new("Trail", &mut trail, 0.2..=3.0).decimals(1).suffix("s"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ControlSlider<'a> {
    label: String,
    value: &'a mut f32,
    range: RangeInclusive<f32>,
    format: ValueFormat,
    suffix: String,
    step: Option<f32>,
    width: Option<f32>,
    height: f32,
    radius: Option<f32>,
}

impl<'a> ControlSlider<'a> {
    /// A slider labelled `label` editing `value` within `range` (integer formatting by default).
    pub fn new(label: impl Into<String>, value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        Self {
            label: label.into(),
            value,
            range,
            format: ValueFormat::Integer,
            suffix: String::new(),
            step: None,
            width: None,
            height: 34.0,
            radius: None,
        }
    }
    /// Show the value rounded to an integer (the default; also snaps the value to whole numbers).
    pub fn integer(mut self) -> Self {
        self.format = ValueFormat::Integer;
        self
    }
    /// Show the value as a float with `n` decimals.
    pub fn decimals(mut self, n: usize) -> Self {
        self.format = ValueFormat::Decimals(n);
        self
    }
    /// Unit appended to the value, e.g. `"px"` or `"s"`.
    pub fn suffix(mut self, s: impl Into<String>) -> Self {
        self.suffix = s.into();
        self
    }
    /// Snap the value to multiples of `step`.
    pub fn step(mut self, step: f32) -> Self {
        self.step = Some(step.abs().max(1e-6));
        self
    }
    /// Width in points (default: all available width).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Bar height in points (default 34).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(16.0);
        self
    }
    /// Corner radius (default: from the theme).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = Some(r.max(0.0));
        self
    }
    fn text(&self, v: f32) -> String {
        let n = match self.format {
            ValueFormat::Integer => format!("{}", v.round() as i64),
            ValueFormat::Decimals(d) => format!("{v:.d$}"),
        };
        format!("{n}{}", self.suffix)
    }
}

impl egui::Widget for ControlSlider<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = self.width.unwrap_or_else(|| ui.available_width()).max(60.0);
        let (rect, mut response) = ui.allocate_exact_size(vec2(w, self.height), Sense::click_and_drag());
        let id = response.id;
        let (lo, hi) = (*self.range.start(), *self.range.end());
        let span = (hi - lo).abs().max(1e-6);
        let snap = |v: f32| {
            let v = v.clamp(lo.min(hi), lo.max(hi));
            let v = if let Some(s) = self.step { lo + ((v - lo) / s).round() * s } else { v };
            if self.format == ValueFormat::Integer {
                v.round()
            } else {
                v
            }
        };
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
        }
        let ptr = response.interact_pointer_pos();
        let cur = *self.value;
        let mut new_value = None;
        if response.drag_started() {
            if let Some(p) = ptr {
                glowkit_core::set_state(
                    ui,
                    id.with("drag"),
                    SliderDrag { anchor_value: cur, anchor_x: p.x, moved: false },
                );
            }
        }
        if response.dragged() {
            if let Some(p) = ptr {
                let mut d: SliderDrag = glowkit_core::state(ui, id.with("drag"), SliderDrag::default);
                if (p.x - d.anchor_x).abs() > 1.0 {
                    d.moved = true;
                }
                let v = d.anchor_value + (p.x - d.anchor_x) / rect.width() * (hi - lo);
                new_value = Some(snap(v));
                glowkit_core::set_state(ui, id.with("drag"), d);
            }
        }
        if response.clicked() {
            if let Some(p) = ptr {
                let f = ((p.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
                new_value = Some(snap(lo + f * (hi - lo)));
            }
        }
        if let Some(v) = new_value {
            if (v - cur).abs() > f32::EPSILON {
                *self.value = v;
                response.mark_changed();
            }
        }
        let v = *self.value;
        let frac = ((v - lo) / span).clamp(0.0, 1.0);
        let shown = glowkit_core::anim::spring(ui, id.with("fill"), frac, 420.0, 34.0).clamp(0.0, 1.0);
        let active = glowkit_core::anim::follow(
            ui,
            id.with("hover"),
            if response.hovered() || response.dragged() { 1.0 } else { 0.0 },
            14.0,
        );
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let radius = self.radius.unwrap_or(t.radius * 0.8).min(rect.height() / 2.0);
        painter.rect_filled(rect, cr(radius), track(&t));
        let fill_col = mix(track_fill(&t), t.foreground, 0.05 * active);
        if shown > 0.001 {
            let fw = (rect.width() * shown).max(radius.min(6.0));
            let fr = Rect::from_min_size(rect.min, vec2(fw, rect.height()));
            let cl = CornerRadius {
                nw: cr(radius).nw,
                sw: cr(radius).sw,
                ne: if shown > 0.995 { cr(radius).ne } else { 0 },
                se: if shown > 0.995 { cr(radius).se } else { 0 },
            };
            painter.rect_filled(fr, cl, fill_col);
        }
        let font = FontId::proportional((self.height * 0.4).clamp(11.0, 15.0));
        let pad = 12.0;
        painter.text(
            pos2(rect.min.x + pad, rect.center().y),
            Align2::LEFT_CENTER,
            &self.label,
            font.clone(),
            mix(t.muted_foreground, t.foreground, 0.55),
        );
        painter.text(pos2(rect.max.x - pad, rect.center().y), Align2::RIGHT_CENTER, self.text(v), font, t.foreground);
        response
    }
}
