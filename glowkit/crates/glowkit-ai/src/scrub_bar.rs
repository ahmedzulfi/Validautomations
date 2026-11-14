//! Timeline scrubber with buffered / played ranges, hover time and markers.

use crate::common;
use crate::media::fmt_clock;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke, Ui, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// A labelled point on a [`ScrubBar`] (chapter, highlight, comment).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrubMarker<'a> {
    /// Position in seconds.
    pub time: f32,
    /// Label shown on hover.
    pub label: &'a str,
    /// Optional color (default: theme accent).
    pub color: Option<Color32>,
}

impl<'a> ScrubMarker<'a> {
    /// A marker at `time` seconds.
    pub fn new(time: f32, label: &'a str) -> Self {
        Self { time, label, color: None }
    }
    /// Marker color.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

/// A media timeline scrubber: a slim track showing the buffered range(s) and the played portion,
/// a thumb that grows on hover, a floating time tooltip that follows the pointer, and marker
/// ticks with labels. Click or drag anywhere to seek; `position` (seconds) is caller-owned.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::{ScrubBar, ScrubMarker};
/// let mut pos = 42.0;
/// let markers = [ScrubMarker::new(30.0, "Intro"), ScrubMarker::new(95.0, "Demo")];
/// if ui.add(ScrubBar::new(&mut pos, 180.0).buffered(120.0).markers(&markers)).changed() {
///     // seek the player to pos
/// }
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ScrubBar<'a> {
    position: &'a mut f32,
    duration: f32,
    buffered: Vec<(f32, f32)>,
    markers: &'a [ScrubMarker<'a>],
    width: Option<f32>,
    height: f32,
    color: Option<Color32>,
    show_times: bool,
}

impl<'a> ScrubBar<'a> {
    /// A scrubber for a clip of `duration` seconds; `position` is updated while seeking.
    pub fn new(position: &'a mut f32, duration: f32) -> Self {
        Self {
            position,
            duration: duration.max(0.001),
            buffered: Vec::new(),
            markers: &[],
            width: None,
            height: 4.0,
            color: None,
            show_times: false,
        }
    }
    /// Buffered up to `end` seconds (from the start).
    pub fn buffered(mut self, end: f32) -> Self {
        self.buffered = vec![(0.0, end)];
        self
    }
    /// Several buffered ranges `(start, end)` in seconds.
    pub fn buffered_ranges(mut self, ranges: &[(f32, f32)]) -> Self {
        self.buffered = ranges.to_vec();
        self
    }
    /// Marker ticks.
    pub fn markers(mut self, m: &'a [ScrubMarker<'a>]) -> Self {
        self.markers = m;
        self
    }
    /// Width (default: available width).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Track thickness at rest (default 4; grows on hover).
    pub fn thickness(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Played-range color (default: theme accent).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Show elapsed / remaining time labels under the track.
    pub fn show_times(mut self, s: bool) -> Self {
        self.show_times = s;
        self
    }
}

impl Widget for ScrubBar<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = self.width.unwrap_or_else(|| ui.available_width());
        let h = 22.0 + if self.show_times { 18.0 } else { 0.0 };
        let (rect, mut resp) = ui.allocate_exact_size(vec2(w, h), Sense::click_and_drag());
        let resp_h = resp.hovered() || resp.dragged();
        let track = Rect::from_min_max(pos2(rect.min.x + 6.0, rect.min.y + 4.0), pos2(rect.max.x - 6.0, rect.min.y + 18.0));
        let to_x = |s: f32| track.min.x + (s / self.duration).clamp(0.0, 1.0) * track.width();
        let from_x = |x: f32| ((x - track.min.x) / track.width()).clamp(0.0, 1.0) * self.duration;
        if resp.is_pointer_button_down_on() || resp.dragged() {
            if let Some(p) = resp.interact_pointer_pos() {
                let v = from_x(p.x);
                if (v - *self.position).abs() > 1e-4 {
                    *self.position = v;
                    resp.mark_changed();
                }
            }
        }
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let hov = common::anim_bool(ui, resp.id.with("h"), resp_h, 0.14);
        let th = self.height + 2.0 * hov;
        let cy = track.center().y;
        let bar = Rect::from_min_max(pos2(track.min.x, cy - th / 2.0), pos2(track.max.x, cy + th / 2.0));
        let cr = CornerRadius::same(255);
        let col = self.color.unwrap_or(t.accent);
        let p = ui.painter();
        p.rect_filled(bar, cr, with_alpha(t.muted_foreground, if t.dark { 0.18 } else { 0.16 }));
        for (a, b) in &self.buffered {
            let r = Rect::from_min_max(pos2(to_x(*a), bar.min.y), pos2(to_x(*b), bar.max.y));
            if r.width() > 0.5 {
                p.rect_filled(r, cr, with_alpha(t.muted_foreground, if t.dark { 0.32 } else { 0.3 }));
            }
        }
        let px = to_x(*self.position);
        let played = Rect::from_min_max(bar.min, pos2(px, bar.max.y));
        if played.width() > 0.5 {
            glowkit_core::paint::gradient_rect(p, played, th / 2.0, 0.0, &[with_alpha(col, 0.75), col]);
        }
        // hover preview line
        let hover_pos = resp.hover_pos().filter(|_| !resp.dragged());
        if let Some(hp) = hover_pos {
            let hx = hp.x.clamp(track.min.x, track.max.x);
            if hx > px {
                let r = Rect::from_min_max(pos2(px, bar.min.y), pos2(hx, bar.max.y));
                p.rect_filled(r, cr, with_alpha(col, 0.22));
            }
        }
        // markers
        let mut hovered_marker = None;
        for (i, m) in self.markers.iter().enumerate() {
            let x = to_x(m.time);
            let mc = m.color.unwrap_or(t.foreground);
            let passed = m.time <= *self.position;
            let r = Rect::from_center_size(pos2(x, cy), vec2(3.0, th + 6.0));
            p.rect_filled(r, CornerRadius::same(2), if passed { with_alpha(mc, 0.95) } else { with_alpha(mc, 0.55) });
            if let Some(hp) = resp.hover_pos() {
                if (hp.x - x).abs() < 6.0 {
                    hovered_marker = Some(i);
                }
            }
        }
        // thumb
        let tr = 5.0 + 2.5 * hov;
        if hov > 0.01 || resp.dragged() {
            glowkit_core::paint::glow(p, pos2(px, cy), tr * 3.0, with_alpha(col, 0.25 * hov));
        }
        p.circle_filled(pos2(px, cy), tr, if t.dark { Color32::WHITE } else { t.card });
        p.circle_stroke(pos2(px, cy), tr, Stroke::new(1.5, col));
        // tooltip
        let tip_x = if resp.dragged() { Some(px) } else { hover_pos.map(|hp| hp.x.clamp(track.min.x, track.max.x)) };
        if let Some(x) = tip_x {
            let time = from_x(x);
            let mut text = fmt_clock(time);
            if let Some(i) = hovered_marker {
                text = format!("{}  ·  {}", self.markers[i].label, fmt_clock(self.markers[i].time));
            }
            let g = ui.painter().layout_no_wrap(text, FontId::proportional(11.5), t.background);
            let bw = g.size().x + 14.0;
            let bx = (x - bw / 2.0).clamp(ui.clip_rect().min.x + 2.0, ui.clip_rect().max.x - bw - 2.0);
            let br = Rect::from_min_size(pos2(bx, track.min.y - 30.0), vec2(bw, 22.0));
            let tp = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, resp.id.with("tip")));
            tp.rect_filled(br, CornerRadius::same(6), t.foreground);
            tp.add(egui::Shape::convex_polygon(
                vec![pos2(x - 4.0, br.max.y), pos2(x + 4.0, br.max.y), pos2(x, br.max.y + 4.0)],
                t.foreground,
                Stroke::NONE,
            ));
            tp.galley(pos2(br.min.x + 7.0, br.center().y - g.size().y / 2.0), g, t.background);
        }
        if self.show_times {
            let y = rect.max.y - 8.0;
            let p = ui.painter();
            common::mono(p, pos2(track.min.x, y), Align2::LEFT_CENTER, fmt_clock(*self.position), 11.0, t.muted_foreground);
            common::mono(
                p,
                pos2(track.max.x, y),
                Align2::RIGHT_CENTER,
                format!("-{}", fmt_clock(self.duration - *self.position)),
                11.0,
                t.muted_foreground,
            );
        }
        resp
    }
}
