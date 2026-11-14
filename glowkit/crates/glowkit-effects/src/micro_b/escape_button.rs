use super::common::{cr, ghost_cursor, hairline, idle_for, ink, shadow_color, surface, text_width};
use egui::{pos2, vec2, Align2, FontId, Pos2, Rect, Response, Sense, Stroke, Ui, Widget};
use glowkit_core::anim::Spring;
use glowkit_core::color::with_alpha;
use glowkit_core::{paint, Theme};

#[derive(Clone, Copy, Debug)]
struct EscapeState {
    x: Spring,
    y: Spring,
    inside: bool,
    dodges: u32,
    caught: Option<f64>,
    ghost: Pos2,
    ghost_on: bool,
}

/// A button that slides away from your cursor. Every time the pointer comes within `radius` it
/// springs off in the other direction (squashing as it goes, and jumping across the stage when
/// cornered). After `patience` escapes it gives up, says so, and lets itself be clicked.
///
/// While idle a ghost cursor chases it around so the stage is never still.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// if ui.add(glowkit_effects::EscapeButton::new("Touch me")).clicked() {
///     // caught it
/// }
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct EscapeButton {
    label: String,
    tired_label: String,
    caught_label: String,
    height: f32,
    skittish: f32,
    patience: u32,
    radius: f32,
    corner: f32,
    dark: bool,
    stroke: bool,
    autoplay: bool,
}

impl EscapeButton {
    /// A runaway button labelled `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            tired_label: "Okay, fine".into(),
            caught_label: "You got me".into(),
            height: 320.0,
            skittish: 0.55,
            patience: 4,
            radius: 120.0,
            corner: 28.0,
            dark: false,
            stroke: false,
            autoplay: true,
        }
    }
    /// Labels shown once it gives up, and after it is clicked.
    pub fn give_up_labels(mut self, tired: impl Into<String>, caught: impl Into<String>) -> Self {
        self.tired_label = tired.into();
        self.caught_label = caught.into();
        self
    }
    /// Height of the area it can run around in (width is all available).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(80.0);
        self
    }
    /// How far and fast it jumps away (0..1).
    pub fn skittishness(mut self, k: f32) -> Self {
        self.skittish = k.clamp(0.0, 1.0);
        self
    }
    /// How many times it escapes before giving up.
    pub fn patience(mut self, n: u32) -> Self {
        self.patience = n;
        self
    }
    /// How close (points) the pointer may come before it runs.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.max(10.0);
        self
    }
    /// Corner radius of the button.
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// Dark (ink) button.
    pub fn dark(mut self, on: bool) -> Self {
        self.dark = on;
        self
    }
    /// Hairline outline.
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Let a ghost cursor chase it while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for EscapeButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (stage, stage_resp) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let id = stage_resp.id;
        let now = ui.input(|i| i.time);
        let dt = glowkit_core::dt(ui);
        let st0: EscapeState = glowkit_core::state(ui, id, || EscapeState {
            x: Spring::new(0.0),
            y: Spring::new(0.0),
            inside: false,
            dodges: 0,
            caught: None,
            ghost: stage.center() + vec2(-stage.width() * 0.35, stage.height() * 0.3),
            ghost_on: false,
        });
        let tired = st0.dodges >= self.patience;
        let label = if st0.caught.is_some() {
            &self.caught_label
        } else if tired {
            &self.tired_label
        } else {
            &self.label
        };
        let painter = ui.painter_at(stage);
        let size = vec2(text_width(&painter, label, 14.0) + 36.0, 40.0);
        let center = stage.center() + vec2(st0.x.value, st0.y.value);
        let btn = Rect::from_center_size(center, size);
        let resp = ui.interact(btn, id.with("btn"), Sense::click());
        let real =
            ui.input(|i| i.pointer.hover_pos()).filter(|p| stage.contains(*p) && ui.rect_contains_pointer(stage));
        let idle = idle_for(ui, id, real.is_some());
        let ghost_on = self.autoplay && idle > 2.0;
        let clicked = resp.clicked() && tired;
        if tired && resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let half = stage.size() / 2.0 - size / 2.0 - vec2(12.0, 12.0);
        let (radius, skittish, patience) = (self.radius, self.skittish, self.patience);
        let st = glowkit_core::with_state(
            ui,
            id,
            || st0,
            |s| {
                if clicked && s.caught.is_none() {
                    s.caught = Some(now);
                }
                if let Some(at) = s.caught {
                    if now - at > 1.8 {
                        // start over from the middle
                        s.caught = None;
                        s.dodges = 0;
                        s.x.target = 0.0;
                        s.y.target = 0.0;
                    }
                }
                s.ghost_on = ghost_on;
                let pos = stage.center() + vec2(s.x.value, s.y.value);
                if ghost_on {
                    // the ghost chases the button; once it is tired it goes in for the click
                    let to = pos - s.ghost;
                    let speed = if s.dodges >= patience { 160.0 } else { 210.0 + 90.0 * skittish };
                    let step = to.length().min(speed * dt);
                    if to.length() > 1.0 {
                        s.ghost += to.normalized() * step;
                    }
                    if s.dodges >= patience && to.length() < 6.0 && s.caught.is_none() {
                        s.caught = Some(now);
                    }
                }
                let pointer = real.or(if ghost_on { Some(s.ghost) } else { None });
                let inside = pointer
                    .is_some_and(|p| p.distance(pos) < radius * if ghost_on && real.is_none() { 0.45 } else { 1.0 });
                if inside && s.dodges < patience && s.caught.is_none() {
                    let p = pointer.unwrap_or(pos);
                    let mut away = pos - p;
                    if away.length() < 1.0 {
                        away = vec2(1.0, -0.4);
                    }
                    let jump = radius * (0.9 + 0.9 * skittish);
                    let mut target = (pos - stage.center()) + away.normalized() * jump;
                    // cornered: leap to the far side of the stage instead
                    if target.x.abs() > half.x || target.y.abs() > half.y {
                        let mirrored = -(p - stage.center()) * 0.85;
                        target = vec2(mirrored.x.clamp(-half.x, half.x), mirrored.y.clamp(-half.y, half.y));
                    }
                    s.x.target = target.x.clamp(-half.x, half.x);
                    s.y.target = target.y.clamp(-half.y, half.y);
                    if !s.inside {
                        s.dodges += 1;
                    }
                }
                s.inside = inside;
                let k = 140.0 + 260.0 * skittish;
                for sp in [&mut s.x, &mut s.y] {
                    sp.stiffness = k;
                    sp.damping = 2.0 * k.sqrt() * 0.62;
                    sp.step(dt);
                }
                *s
            },
        );
        let moving = !(st.x.is_settled() && st.y.is_settled());
        if moving || ghost_on || st.caught.is_some() {
            glowkit_core::animate(ui);
        }
        let center = stage.center() + vec2(st.x.value, st.y.value);
        let vel = vec2(st.x.velocity, st.y.velocity);
        let speed = vel.length();
        let squash = (speed / 1800.0).min(0.22);
        let sz = if speed > 1.0 {
            let d = vel / speed;
            vec2(
                size.x * (1.0 + squash * d.x.abs()) / (1.0 + squash * d.y.abs() * 0.5),
                size.y * (1.0 + squash * d.y.abs() * 0.6) / (1.0 + squash * d.x.abs() * 0.6),
            )
        } else {
            size
        };
        let pressed = resp.is_pointer_button_down_on() && tired;
        let caught_k =
            glowkit_core::anim::spring(ui, id.with("caught"), if st.caught.is_some() { 1.0 } else { 0.0 }, 300.0, 14.0);
        let scale = if pressed { 0.95 } else { 1.0 } + caught_k * 0.06;
        let btn = Rect::from_center_size(center, sz * scale);
        let (ink_bg, ink_fg) = ink(&t);
        let caught_fill = t.success;
        let (mut bg, mut fg) = if self.dark { (ink_bg, ink_fg) } else { (surface(&t), t.foreground) };
        if caught_k > 0.01 {
            bg = bg.lerp_to_gamma(caught_fill, caught_k.clamp(0.0, 1.0));
            fg = fg.lerp_to_gamma(egui::Color32::WHITE, caught_k.clamp(0.0, 1.0));
        }
        let r = self.corner.min(btn.height() / 2.0);
        paint::soft_shadow(
            &painter,
            btn,
            r,
            14.0 + squash * 40.0,
            vec2(-vel.x, -vel.y) * 0.004 + vec2(0.0, 4.0),
            shadow_color(&t, 0.7),
        );
        painter.rect(btn, cr(r), bg, if self.stroke { hairline(&t) } else { Stroke::NONE }, egui::StrokeKind::Inside);
        painter.text(btn.center(), Align2::CENTER_CENTER, label, FontId::proportional(14.0), fg);
        // patience pips under the button
        if self.patience > 0 && st.caught.is_none() {
            let n = self.patience;
            let left = st.dodges.min(n);
            let w = n as f32 * 8.0;
            for i in 0..n {
                let p = pos2(center.x - w / 2.0 + 4.0 + i as f32 * 8.0, btn.max.y + 10.0);
                let used = i < left;
                painter.circle_filled(p, 2.0, with_alpha(t.muted_foreground, if used { 0.2 } else { 0.55 }));
            }
        }
        if st.ghost_on && real.is_none() {
            ghost_cursor(&painter, &t, st.ghost, 0.9, st.caught.is_some());
        }
        resp
    }
}
