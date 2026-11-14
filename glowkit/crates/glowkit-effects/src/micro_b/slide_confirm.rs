use super::common::{cr, ghost_cursor, hairline, icon, idle_for, ink, shadow_color, surface, MicroIcon};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Stroke, Ui, Widget};
use glowkit_core::anim::{smoothstep, Spring};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

#[derive(Clone, Copy, Debug)]
struct SlideState {
    knob: Spring,
    grab: Option<f32>,
    confirmed: bool,
    since: f64,
    ghost: f32,
}

/// A "slide to confirm" pill: drag the round handle across the track. The label fades and
/// shimmers as the handle passes over it, and once the handle is past the mark it takes over and
/// springs to the end on its own, turns into a check and the label becomes the confirmation.
/// Let go early and it springs back with a little bounce.
///
/// The response reports `changed()` on the frame the slide is confirmed.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// if ui.add(glowkit_effects::SlideToConfirm::new("Slide to confirm")).changed() {
///     // do the thing
/// }
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SlideToConfirm {
    label: String,
    done_label: String,
    width: f32,
    height: f32,
    corner: f32,
    dark: bool,
    stroke: bool,
    speed: f32,
    threshold: f32,
    reset_after: Option<f32>,
    autoplay: bool,
}

impl SlideToConfirm {
    /// A slider labelled `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            done_label: "Confirmed".into(),
            width: 280.0,
            height: 56.0,
            corner: 28.0,
            dark: false,
            stroke: false,
            speed: 0.5,
            threshold: 0.7,
            reset_after: Some(2.2),
            autoplay: true,
        }
    }
    /// Text shown after confirming.
    pub fn confirmed_label(mut self, s: impl Into<String>) -> Self {
        self.done_label = s.into();
        self
    }
    /// Track width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(120.0);
        self
    }
    /// Track height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.clamp(32.0, 96.0);
        self
    }
    /// Corner radius (default fully round).
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// Dark (ink) track with a light handle.
    pub fn dark(mut self, on: bool) -> Self {
        self.dark = on;
        self
    }
    /// Hairline outline around the track.
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// How quickly the handle finishes / returns (0..1).
    pub fn speed(mut self, k: f32) -> Self {
        self.speed = k.clamp(0.0, 1.0);
        self
    }
    /// Fraction of the track past which the slide completes itself (default 0.7).
    pub fn threshold(mut self, k: f32) -> Self {
        self.threshold = k.clamp(0.2, 1.0);
        self
    }
    /// Slide back to the start this many seconds after confirming (`None` stays confirmed).
    pub fn reset_after(mut self, secs: Option<f32>) -> Self {
        self.reset_after = secs;
        self
    }
    /// Demonstrate the gesture with a ghost cursor while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for SlideToConfirm {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, mut response) = ui.allocate_exact_size(vec2(self.width, self.height), Sense::hover());
        let id = response.id;
        let pad = 4.0;
        let knob_d = self.height - pad * 2.0;
        let travel = rect.width() - pad * 2.0 - knob_d;
        let st0: SlideState = glowkit_core::state(ui, id, || SlideState {
            knob: Spring::new(0.0),
            grab: None,
            confirmed: false,
            since: 0.0,
            ghost: -1.0,
        });
        let knob_rect = |p: f32| Rect::from_min_size(pos2(rect.min.x + pad + p * travel, rect.min.y + pad), vec2(knob_d, knob_d));
        let kr = knob_rect(st0.knob.value);
        let k_resp = ui.interact(kr, id.with("knob"), Sense::drag());
        if k_resp.hovered() || k_resp.dragged() {
            ui.ctx().set_cursor_icon(if k_resp.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        let now = ui.input(|i| i.time);
        let dt = glowkit_core::dt(ui);
        let ptr = k_resp.interact_pointer_pos();
        let user = k_resp.dragged() || k_resp.hovered();
        let idle = idle_for(ui, id, user || ui.rect_contains_pointer(rect));
        let (started, dragging, released) = (k_resp.drag_started(), k_resp.dragged(), k_resp.drag_stopped());
        let stiff = 120.0 + 380.0 * self.speed;
        let auto = self.autoplay && idle > 2.5;
        let (threshold, reset_after) = (self.threshold, self.reset_after);
        let mut just_confirmed = false;
        let st = glowkit_core::with_state(ui, id, || st0, |s| {
            if s.confirmed {
                s.grab = None;
                if s.ghost >= 0.0 {
                    s.ghost += dt;
                    if s.ghost > 2.6 {
                        s.ghost = -1.0;
                    }
                }
                if reset_after.is_some_and(|r| now - s.since > r as f64) {
                    s.confirmed = false;
                }
            } else {
                if started {
                    if let Some(p) = ptr {
                        s.grab = Some(p.x - (rect.min.x + pad + s.knob.value * travel));
                    }
                }
                if let (true, Some(g), Some(p)) = (dragging, s.grab, ptr) {
                    let raw = (p.x - g - rect.min.x - pad) / travel.max(1.0);
                    // rubber band before the start
                    let v = if raw < 0.0 { raw * 0.15 } else { raw.min(1.0) };
                    s.knob.velocity = (v - s.knob.value) / dt.max(1e-3);
                    s.knob.value = v;
                    s.knob.target = v;
                    s.ghost = -1.0;
                } else if s.ghost >= 0.0 || (auto && s.grab.is_none()) {
                    // ghost demo: ease out to just past the mark, then let go
                    if s.ghost < 0.0 {
                        s.ghost = 0.0;
                    }
                    s.ghost += dt;
                    let g = s.ghost;
                    if (1.0..2.1).contains(&g) {
                        let k = smoothstep(1.0, 2.1, g);
                        s.knob.value = k * (threshold + 0.06);
                        s.knob.target = s.knob.value;
                        s.knob.velocity = 0.0;
                    } else if g >= 2.1 {
                        s.knob.target = 1.0;
                    }
                }
                if released {
                    s.grab = None;
                }
                if s.knob.value >= threshold && !s.confirmed {
                    s.confirmed = true;
                    s.since = now;
                    s.grab = None;
                    just_confirmed = true;
                }
            }
            if s.grab.is_none() && !(1.0..2.1).contains(&s.ghost) {
                s.knob.target = if s.confirmed { 1.0 } else { 0.0 };
                s.knob.stiffness = stiff;
                s.knob.damping = if s.confirmed { 22.0 } else { 13.0 };
                s.knob.step(dt);
            }
            *s
        });
        if just_confirmed {
            response.mark_changed();
        }
        let moving = !st.knob.is_settled() || st.grab.is_some() || st.ghost >= 0.0 || st.confirmed;
        if moving || auto {
            glowkit_core::animate(ui);
        }
        let done = glowkit_core::anim::spring(ui, id.with("done"), if st.confirmed { 1.0 } else { 0.0 }, 220.0, 20.0);
        let press = glowkit_core::anim::spring(ui, id.with("press"), if st.grab.is_some() { 1.0 } else { 0.0 }, 400.0, 24.0);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let p = st.knob.value;
        let (ink_bg, ink_fg) = ink(&t);
        let (track, track_fg, knob_bg, knob_fg) =
            if self.dark { (ink_bg, ink_fg, ink_fg, ink_bg) } else { (surface(&t), t.foreground, ink_bg, ink_fg) };
        let radius = self.corner.min(self.height / 2.0);
        paint::soft_shadow(painter, rect, radius, 16.0, vec2(0.0, 4.0), shadow_color(&t, 0.7));
        painter.rect(rect, cr(radius), track, if self.stroke { hairline(&t) } else { Stroke::NONE }, egui::StrokeKind::Inside);
        let kr = knob_rect(p.max(0.0)).expand(press * 1.5);
        // trail behind the handle
        let trail = Rect::from_min_max(rect.min + vec2(pad, pad), pos2(kr.max.x, rect.max.y - pad));
        painter.rect_filled(trail, cr(radius - pad), with_alpha(track_fg, 0.06 + 0.04 * done));
        // label: dims and shimmers as the handle passes, then swaps for the confirmation
        let label_c = pos2(rect.center().x + knob_d * 0.35, rect.center().y);
        let fade = (1.0 - p * 1.6).clamp(0.0, 1.0) * (1.0 - done);
        if fade > 0.01 {
            let time = glowkit_core::time(ui);
            let galley = painter.layout_no_wrap(self.label.clone(), FontId::proportional(14.5), Color32::WHITE);
            let pos = label_c - galley.size() / 2.0;
            let sweep = ((time * 0.6) % 1.6) - 0.3;
            let base = with_alpha(track_fg, 0.45 * fade);
            let hi = with_alpha(track_fg, 0.95 * fade);
            let x0 = pos.x;
            let w = galley.size().x.max(1.0);
            paint::galley_with(painter, pos, &galley, |q| {
                let u = (q.x - x0) / w;
                let d = ((u - sweep) / 0.22).abs();
                mix(base, hi, (1.0 - d).clamp(0.0, 1.0))
            });
            if auto {
                glowkit_core::animate(ui);
            }
        }
        if done > 0.01 {
            let c = pos2(rect.center().x - knob_d * 0.35, rect.center().y + (1.0 - done) * 6.0);
            painter.text(c, Align2::CENTER_CENTER, &self.done_label, FontId::proportional(14.5), with_alpha(track_fg, done.min(1.0)));
        }
        // handle
        paint::soft_shadow(painter, kr, kr.width() / 2.0, 8.0, vec2(0.0, 2.0), shadow_color(&t, 0.8));
        painter.circle_filled(kr.center(), kr.width() / 2.0, knob_bg);
        let s = kr.width() * 0.36;
        let a = (1.0 - done).clamp(0.0, 1.0);
        if a > 0.01 {
            icon(painter, MicroIcon::ArrowRight, kr.center() + vec2(-done * 4.0, 0.0), s, with_alpha(knob_fg, a), 2.0);
        }
        if done > 0.01 {
            let k = done.clamp(0.0, 1.0);
            icon(painter, MicroIcon::Check, kr.center(), s * (0.6 + 0.4 * k), with_alpha(knob_fg, k), 2.0);
        }
        if st.ghost >= 0.0 {
            let g = st.ghost;
            let a = smoothstep(0.5, 0.9, g) * (1.0 - smoothstep(2.2, 2.6, g));
            ghost_cursor(painter, &t, kr.center() + vec2(4.0, 6.0), a, (0.95..2.1).contains(&g));
        }
        response.union(k_resp)
    }
}
