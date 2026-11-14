use egui::{pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Rect, Response, Sense, Ui, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::{paint, Theme};
use std::ops::RangeInclusive;

/// A slider with rubber-band physics: drag past either end and the track stretches toward the
/// pointer, then snaps back with a wobble on release. The end icons swell as you push them.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut volume = 50.0;
/// ui.add(glowkit_effects::ElasticSlider::new(&mut volume, 0.0..=100.0).step(1.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ElasticSlider<'a> {
    value: &'a mut f32,
    range: RangeInclusive<f32>,
    width: f32,
    step: Option<f32>,
    icons: (String, String),
    show_value: bool,
    color: Option<Color32>,
    max_stretch: f32,
}

impl<'a> ElasticSlider<'a> {
    /// A slider editing `value` within `range`.
    pub fn new(value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        Self {
            value,
            range,
            width: 260.0,
            step: None,
            icons: ("🔈".into(), "🔊".into()),
            show_value: true,
            color: None,
            max_stretch: 36.0,
        }
    }
    /// Track width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(40.0);
        self
    }
    /// Snap the value to multiples of `step`.
    pub fn step(mut self, step: f32) -> Self {
        self.step = (step > 0.0).then_some(step);
        self
    }
    /// Glyphs at the left and right ends (empty strings hide them).
    pub fn icons(mut self, left: impl Into<String>, right: impl Into<String>) -> Self {
        self.icons = (left.into(), right.into());
        self
    }
    /// Show the value under the track (default on).
    pub fn show_value(mut self, on: bool) -> Self {
        self.show_value = on;
        self
    }
    /// Fill color. Defaults to the theme foreground.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Maximum stretch past the ends, in points.
    pub fn max_stretch(mut self, s: f32) -> Self {
        self.max_stretch = s.max(0.0);
        self
    }
}

impl Widget for ElasticSlider<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let icon_w = if self.icons.0.is_empty() && self.icons.1.is_empty() { 0.0 } else { 34.0 };
        let total = vec2(self.width + icon_w * 2.0, if self.show_value { 64.0 } else { 36.0 });
        let (rect, mut response) = ui.allocate_exact_size(total, Sense::click_and_drag());
        let id = response.id;
        let track_y = rect.min.y + 18.0;
        let track =
            Rect::from_min_max(pos2(rect.min.x + icon_w, track_y - 3.0), pos2(rect.max.x - icon_w, track_y + 3.0));
        let (lo, hi) = (*self.range.start(), *self.range.end());
        let mut overflow_target = 0.0;
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(if response.dragged() { CursorIcon::Grabbing } else { CursorIcon::Grab });
        }
        if let Some(p) = response.interact_pointer_pos() {
            if response.dragged() || response.clicked() || response.drag_started() {
                let u = ((p.x - track.min.x) / track.width()).clamp(0.0, 1.0);
                let mut v = lo + (hi - lo) * u;
                if let Some(s) = self.step {
                    v = lo + ((v - lo) / s).round() * s;
                }
                let v = v.clamp(lo.min(hi), hi.max(lo));
                if (v - *self.value).abs() > f32::EPSILON {
                    *self.value = v;
                    response.mark_changed();
                }
                if response.dragged() {
                    let d = if p.x < track.min.x {
                        p.x - track.min.x
                    } else if p.x > track.max.x {
                        p.x - track.max.x
                    } else {
                        0.0
                    };
                    // exponential decay: the further you pull, the harder it resists
                    overflow_target =
                        d.signum() * self.max_stretch * (1.0 - (-d.abs() / (self.max_stretch * 2.5)).exp());
                }
            }
        }
        let overflow = if response.dragged() {
            glowkit_core::set_state(ui, id.with("of_direct"), overflow_target);
            // follow the pointer tightly while dragging
            glowkit_core::anim::spring(ui, id.with("of"), overflow_target, 900.0, 40.0)
        } else {
            glowkit_core::anim::spring(ui, id.with("of"), 0.0, 260.0, 9.0)
        };
        let active = response.hovered() || response.dragged();
        let thick = glowkit_core::anim::spring(ui, id.with("th"), if active { 12.0 } else { 6.0 }, 320.0, 22.0);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(self.max_stretch + 10.0));
        let stretch_ratio = overflow.abs() / self.width;
        let th = thick * (1.0 - stretch_ratio * 1.6).max(0.4);
        let mut tr = Rect::from_min_max(pos2(track.min.x, track_y - th / 2.0), pos2(track.max.x, track_y + th / 2.0));
        if overflow < 0.0 {
            tr.min.x += overflow;
        } else {
            tr.max.x += overflow;
        }
        let cr = CornerRadius::same((th / 2.0).round() as u8);
        painter.rect_filled(tr, cr, with_alpha(t.muted_foreground, if t.dark { 0.28 } else { 0.22 }));
        let u = if (hi - lo).abs() > f32::EPSILON { ((*self.value - lo) / (hi - lo)).clamp(0.0, 1.0) } else { 0.0 };
        let fill_end = tr.min.x + tr.width() * u;
        let fill = Rect::from_min_max(tr.min, pos2(fill_end.max(tr.min.x + th), tr.max.y));
        let col = self.color.unwrap_or(t.foreground);
        if u > 0.0 {
            paint::gradient_rect(&painter, fill, th / 2.0, 0.0, &[with_alpha(col, 0.75), col]);
        }
        // icons swell on the side being pulled
        let left_scale = 1.0 + (-overflow).max(0.0) / self.max_stretch.max(1.0) * 0.45;
        let right_scale = 1.0 + overflow.max(0.0) / self.max_stretch.max(1.0) * 0.45;
        let icol = if active { t.foreground } else { t.muted_foreground };
        if !self.icons.0.is_empty() {
            painter.text(
                pos2(tr.min.x - icon_w / 2.0, track_y),
                Align2::CENTER_CENTER,
                &self.icons.0,
                FontId::proportional(16.0 * left_scale),
                icol,
            );
        }
        if !self.icons.1.is_empty() {
            painter.text(
                pos2(tr.max.x + icon_w / 2.0, track_y),
                Align2::CENTER_CENTER,
                &self.icons.1,
                FontId::proportional(16.0 * right_scale),
                icol,
            );
        }
        if self.show_value {
            let txt = match self.step {
                Some(s) if s >= 1.0 => format!("{:.0}", *self.value),
                _ => format!("{:.2}", *self.value),
            };
            painter.text(
                pos2(rect.center().x, rect.max.y - 12.0),
                Align2::CENTER_CENTER,
                txt,
                FontId::monospace(13.0),
                t.muted_foreground,
            );
        }
        response
    }
}
