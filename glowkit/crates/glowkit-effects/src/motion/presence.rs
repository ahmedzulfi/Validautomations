use egui::emath::TSTransform;
use egui::{pos2, vec2, Rect, Sense, Ui, UiBuilder};
use glowkit_core::anim::{ease, Easing};

/// How content enters and leaves in [`SpringPresence`] and [`TransitionPanel`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PresenceEffect {
    /// Opacity only.
    Fade,
    /// Fade while scaling up from 90 %.
    Scale,
    /// Fade while rising from below.
    #[default]
    SlideUp,
    /// Fade while dropping from above.
    SlideDown,
    /// Fade while sliding in from the right.
    SlideLeft,
    /// Fade while sliding in from the left.
    SlideRight,
    /// Springy scale from 50 % with overshoot.
    Pop,
}

impl PresenceEffect {
    fn transform(self, p: f32, distance: f32, center: egui::Pos2) -> TSTransform {
        let q = 1.0 - p;
        let scale_about = |s: f32| TSTransform::from_translation(center.to_vec2()) * TSTransform::from_scaling(s) * TSTransform::from_translation(-center.to_vec2());
        match self {
            PresenceEffect::Fade => TSTransform::IDENTITY,
            PresenceEffect::Scale => scale_about(0.9 + 0.1 * p),
            PresenceEffect::Pop => scale_about((0.5 + 0.5 * p).max(0.01)),
            PresenceEffect::SlideUp => TSTransform::from_translation(vec2(0.0, distance * q)),
            PresenceEffect::SlideDown => TSTransform::from_translation(vec2(0.0, -distance * q)),
            PresenceEffect::SlideLeft => TSTransform::from_translation(vec2(distance * q, 0.0)),
            PresenceEffect::SlideRight => TSTransform::from_translation(vec2(-distance * q, 0.0)),
        }
    }
}

/// Animate content in and out when a condition changes, like `AnimatePresence`: content fades
/// / slides / scales in on a spring when `visible` becomes true, plays the reverse when it
/// becomes false, and is removed once hidden. Optionally the space it takes animates too.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let show = true;
/// glowkit_effects::SpringPresence::new(show).effect(glowkit_effects::PresenceEffect::Pop).show(ui, |ui| {
///     ui.label("Hello!");
/// });
/// ```
#[must_use = "call `.show(ui, |ui| ...)`"]
pub struct SpringPresence {
    visible: bool,
    effect: PresenceEffect,
    distance: f32,
    stiffness: f32,
    damping: f32,
    animate_height: bool,
    salt: u64,
}

impl SpringPresence {
    /// Present the content while `visible` is true.
    pub fn new(visible: bool) -> Self {
        Self { visible, effect: PresenceEffect::SlideUp, distance: 16.0, stiffness: 260.0, damping: 24.0, animate_height: true, salt: 0 }
    }
    /// Enter / exit effect.
    pub fn effect(mut self, e: PresenceEffect) -> Self {
        self.effect = e;
        self
    }
    /// Travel distance of slide effects.
    pub fn distance(mut self, d: f32) -> Self {
        self.distance = d;
        self
    }
    /// Spring stiffness and damping.
    pub fn spring(mut self, stiffness: f32, damping: f32) -> Self {
        self.stiffness = stiffness.max(1.0);
        self.damping = damping.max(0.1);
        self
    }
    /// Also animate the height the content occupies so the layout below glides (default on).
    pub fn animate_height(mut self, on: bool) -> Self {
        self.animate_height = on;
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }

    /// Show the content while present. Returns `None` once fully hidden.
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let id = ui.scope_id().with(("presence", self.salt));
        let target = if self.visible { 1.0 } else { 0.0 };
        let (st, dm) = if self.effect == PresenceEffect::Pop { (self.stiffness * 1.3, self.damping * 0.6) } else { (self.stiffness, self.damping) };
        let p = glowkit_core::anim::spring(ui, id.with("p"), target, st, dm);
        if !self.visible && p < 0.002 {
            return None;
        }
        let alpha = p.clamp(0.0, 1.0);
        let measured: f32 = glowkit_core::state(ui, id.with("h"), || 0.0);
        let width = ui.available_width();
        let height = if self.animate_height { measured * ease(Easing::OutCubic, alpha) } else { measured };
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let center = rect.min + vec2(width / 2.0, measured / 2.0);
        let tf = self.effect.transform(p, self.distance, center);
        let mut out = None;
        let mut new_h = measured;
        ui.with_visual_transform(tf, |ui| {
            let mut child = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(rect.min, vec2(width, 10_000.0))).id_salt("presence_body"));
            if self.animate_height {
                let clip = Rect::from_min_size(rect.min - vec2(40.0, 40.0), vec2(width + 80.0, height + 80.0));
                child.set_clip_rect(clip.intersect(ui.clip_rect()));
            }
            child.multiply_opacity(alpha);
            out = Some(add_contents(&mut child));
            new_h = child.min_rect().height();
        });
        if (new_h - measured).abs() > 0.5 {
            glowkit_core::set_state(ui, id.with("h"), new_h);
            ui.ctx().request_repaint();
        }
        out
    }
}

#[derive(Clone, Copy)]
struct PanelAnim {
    from: usize,
    to: usize,
    t0: f32,
}

/// Switches between panels with a transition: the old panel exits while the new one enters
/// (direction-aware for slides) and the container height springs to fit.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let tab = 0usize;
/// glowkit_effects::TransitionPanel::new(tab).show(ui, |ui, i| {
///     ui.label(format!("Panel {i}"));
/// });
/// ```
#[must_use = "call `.show(ui, |ui, index| ...)`"]
pub struct TransitionPanel {
    active: usize,
    effect: PresenceEffect,
    duration: f32,
    distance: f32,
    salt: u64,
}

impl TransitionPanel {
    /// Show panel `active`.
    pub fn new(active: usize) -> Self {
        Self { active, effect: PresenceEffect::SlideLeft, duration: 0.4, distance: 40.0, salt: 0 }
    }
    /// Transition effect (slides reverse direction when going back).
    pub fn effect(mut self, e: PresenceEffect) -> Self {
        self.effect = e;
        self
    }
    /// Transition duration in seconds.
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s.max(0.05);
        self
    }
    /// Travel distance of slide effects.
    pub fn distance(mut self, d: f32) -> Self {
        self.distance = d;
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }

    /// Show the panels; `panel(ui, index)` lays out one panel.
    pub fn show(self, ui: &mut Ui, mut panel: impl FnMut(&mut Ui, usize)) {
        let id = ui.scope_id().with(("transition_panel", self.salt));
        let now = glowkit_core::time(ui);
        let mut anim: PanelAnim = glowkit_core::state(ui, id.with("anim"), || PanelAnim { from: self.active, to: self.active, t0: -10.0 });
        if anim.to != self.active {
            anim = PanelAnim { from: anim.to, to: self.active, t0: now };
            glowkit_core::set_state(ui, id.with("anim"), anim);
        }
        let p = ((now - anim.t0) / self.duration).clamp(0.0, 1.0);
        if p < 1.0 {
            ui.ctx().request_repaint();
        }
        let e = ease(Easing::OutCubic, p);
        let last: f32 = glowkit_core::state(ui, id.with("last_h"), || 0.0);
        let measured: f32 = glowkit_core::state(ui, id.with(("h", self.active)), || last);
        let h = glowkit_core::anim::spring(ui, id.with("hs"), measured, 260.0, 28.0);
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
        let back = anim.to < anim.from;
        let mut effect = self.effect;
        if back {
            effect = match effect {
                PresenceEffect::SlideLeft => PresenceEffect::SlideRight,
                PresenceEffect::SlideRight => PresenceEffect::SlideLeft,
                PresenceEffect::SlideUp => PresenceEffect::SlideDown,
                PresenceEffect::SlideDown => PresenceEffect::SlideUp,
                other => other,
            };
        }
        let clip = rect.expand2(vec2(30.0, 20.0)).intersect(ui.clip_rect());
        let mut new_h = measured;
        let mut render = |ui: &mut Ui, index: usize, prog: f32, exiting: bool, measure: bool| {
            let c = pos2(rect.center().x, rect.min.y + h / 2.0);
            // exiting panels move the opposite way
            let tf = if exiting {
                let mirrored = match effect {
                    PresenceEffect::SlideLeft => PresenceEffect::SlideRight,
                    PresenceEffect::SlideRight => PresenceEffect::SlideLeft,
                    PresenceEffect::SlideUp => PresenceEffect::SlideDown,
                    PresenceEffect::SlideDown => PresenceEffect::SlideUp,
                    other => other,
                };
                mirrored.transform(prog, self.distance, c)
            } else {
                effect.transform(prog, self.distance, c)
            };
            ui.with_visual_transform(tf, |ui| {
                let mut child = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(rect.min, vec2(width, 10_000.0))).id_salt(("panel", index)));
                child.set_clip_rect(clip);
                child.multiply_opacity(prog.clamp(0.0, 1.0));
                panel(&mut child, index);
                if measure {
                    new_h = child.min_rect().height();
                }
            });
        };
        if p < 1.0 && anim.from != anim.to {
            render(ui, anim.from, 1.0 - e, true, false);
        }
        render(ui, self.active, if p < 1.0 { e } else { 1.0 }, false, true);
        if (new_h - measured).abs() > 0.5 {
            glowkit_core::set_state(ui, id.with(("h", self.active)), new_h);
            ui.ctx().request_repaint();
        }
        glowkit_core::set_state(ui, id.with("last_h"), new_h);
    }
}
