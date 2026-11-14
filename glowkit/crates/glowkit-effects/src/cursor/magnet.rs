use egui::emath::TSTransform;
use egui::{vec2, InnerResponse, Rect, Ui, Vec2};

/// Wraps any content and pulls it toward the pointer when the pointer comes near, springing
/// back when it leaves.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_effects::Magnet::new().strength(0.4).padding(80.0).show(ui, |ui| {
///     ui.button("Hover near me")
/// });
/// ```
#[must_use = "call `.show(ui, |ui| ...)`"]
pub struct Magnet {
    strength: f32,
    padding: f32,
    stiffness: f32,
    damping: f32,
    max_offset: f32,
    autoplay: bool,
    salt: u64,
}

impl Default for Magnet {
    fn default() -> Self {
        Self::new()
    }
}

impl Magnet {
    /// Moderate pull within 80 pt of the content.
    pub fn new() -> Self {
        Self {
            strength: 0.35,
            padding: 80.0,
            stiffness: 220.0,
            damping: 16.0,
            max_offset: 40.0,
            autoplay: false,
            salt: 0,
        }
    }
    /// Fraction of the pointer's offset from the content center that the content follows.
    pub fn strength(mut self, s: f32) -> Self {
        self.strength = s.clamp(0.0, 2.0);
        self
    }
    /// How far around the content the magnetic field reaches.
    pub fn padding(mut self, p: f32) -> Self {
        self.padding = p.max(0.0);
        self
    }
    /// Spring stiffness and damping of the motion.
    pub fn spring(mut self, stiffness: f32, damping: f32) -> Self {
        self.stiffness = stiffness.max(1.0);
        self.damping = damping.max(0.1);
        self
    }
    /// Clamp the displacement to this many points.
    pub fn max_offset(mut self, o: f32) -> Self {
        self.max_offset = o.max(0.0);
        self
    }
    /// When the pointer is away, gently orbit as if a pointer circled the content.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }

    /// Show `add_contents` with the magnetic pull applied.
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let id = ui.scope_id().with(("magnet", self.salt));
        let prev: Option<Rect> = glowkit_core::state(ui, id.with("rect"), || None);
        let mut target = Vec2::ZERO;
        if let Some(r) = prev {
            let field = r.expand(self.padding);
            let pointer =
                ui.input(|i| i.pointer.hover_pos()).filter(|p| field.contains(*p) && ui.rect_contains_pointer(field));
            if let Some(p) = pointer {
                target = (p - r.center()) * self.strength;
            } else if self.autoplay {
                let t = glowkit_core::time(ui);
                let a = t * 1.4;
                let reach = (r.size() / 2.0 + Vec2::splat(self.padding * 0.5)) * (0.6 + 0.4 * (t * 0.7).sin());
                target = vec2(a.cos() * reach.x, (a * 1.3).sin() * reach.y) * self.strength;
                glowkit_core::animate(ui);
            }
        }
        if target.length() > self.max_offset {
            target = target.normalized() * self.max_offset;
        }
        let off = glowkit_core::anim::spring2(ui, id.with("off"), target, self.stiffness, self.damping);
        let res = ui.with_visual_transform(TSTransform::from_translation(off), add_contents);
        glowkit_core::set_state(ui, id.with("rect"), Some(res.response.rect));
        res
    }
}
