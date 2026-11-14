use egui::emath::TSTransform;
use egui::{vec2, Rect, Response, Sense, Ui, UiBuilder};
use glowkit_core::Clock;

/// A list whose items pop in one after another, newest on top, pushing older ones down with
/// springy layout animation: like a notification feed.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let messages = ["Payment received", "New follower", "Build passed"];
/// glowkit_effects::AnimatedList::new(messages.len()).delay(1.2).show(ui, |ui, i| {
///     ui.label(messages[i]);
/// });
/// ```
#[must_use = "call `.show(ui, |ui, index| ...)`"]
pub struct AnimatedList {
    count: usize,
    delay: f32,
    looping: bool,
    hold: f32,
    gap: f32,
    height: f32,
    newest_first: bool,
    salt: u64,
}

impl AnimatedList {
    /// A list that will reveal `count` items.
    pub fn new(count: usize) -> Self {
        Self { count, delay: 1.0, looping: true, hold: 2.5, gap: 10.0, height: 340.0, newest_first: true, salt: 0 }
    }
    /// Seconds between two items appearing.
    pub fn delay(mut self, s: f32) -> Self {
        self.delay = s.max(0.05);
        self
    }
    /// Start over after everything has been shown (default on).
    pub fn looping(mut self, on: bool) -> Self {
        self.looping = on;
        self
    }
    /// Seconds to hold the full list before looping.
    pub fn hold(mut self, s: f32) -> Self {
        self.hold = s.max(0.0);
        self
    }
    /// Vertical gap between items.
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g.max(0.0);
        self
    }
    /// Height of the list viewport; older items slide out of the bottom.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(20.0);
        self
    }
    /// Put new items on top (default) or append them at the bottom.
    pub fn newest_first(mut self, on: bool) -> Self {
        self.newest_first = on;
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }

    /// Show the list; `item(ui, index)` lays out one item (index 0 appears first).
    pub fn show(self, ui: &mut Ui, mut item: impl FnMut(&mut Ui, usize)) -> Response {
        let id = ui.scope_id().with(("animated_list", self.salt));
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        if self.count == 0 {
            return response;
        }
        let elapsed = Clock::elapsed(ui, id);
        let reveal_end = self.count as f32 * self.delay;
        let cycle = reveal_end + self.hold;
        if self.looping && elapsed > cycle + 0.45 {
            Clock::restart(ui, id);
            let gen: u32 = glowkit_core::state(ui, id.with("gen"), || 0);
            glowkit_core::set_state(ui, id.with("gen"), gen + 1);
        }
        let gen: u32 = glowkit_core::state(ui, id.with("gen"), || 0);
        let gid = id.with(gen);
        let shown = ((elapsed / self.delay).floor() as usize + 1).min(self.count);
        let fade_all = if self.looping { 1.0 - ((elapsed - cycle) / 0.4).clamp(0.0, 1.0) } else { 1.0 };
        if shown < self.count || self.looping {
            glowkit_core::animate(ui);
        }
        let heights: Vec<f32> = glowkit_core::state(ui, id.with("h"), || vec![56.0; self.count]);
        let mut heights = if heights.len() == self.count { heights } else { vec![56.0; self.count] };
        let order: Vec<usize> = if self.newest_first { (0..shown).rev().collect() } else { (0..shown).collect() };
        let clip = rect.intersect(ui.clip_rect());
        let mut y = 0.0;
        for &i in &order {
            let target_y = y;
            y += heights[i] + self.gap;
            let appear_t = elapsed - i as f32 * self.delay;
            // new items start where they will be; older items spring down to make room
            let sy = glowkit_core::anim::spring(ui, gid.with(("y", i)), target_y, 260.0, 24.0);
            let pop = glowkit_core::anim::spring(ui, gid.with(("pop", i)), 1.0, 380.0, 22.0);
            let pop = if appear_t < 0.0 { 0.0 } else { pop };
            let alpha = (appear_t / 0.25).clamp(0.0, 1.0) * fade_all;
            let top = rect.min.y + sy;
            if top > rect.max.y {
                continue;
            }
            let item_rect = Rect::from_min_size(egui::pos2(rect.min.x, top), vec2(rect.width(), heights[i]));
            let scale = 0.7 + 0.3 * pop;
            let c = item_rect.center();
            let tf = TSTransform::from_translation(c.to_vec2())
                * TSTransform::from_scaling(scale)
                * TSTransform::from_translation(-c.to_vec2());
            let mut measured = heights[i];
            ui.with_visual_transform(tf, |ui| {
                let mut child = ui.new_child(
                    UiBuilder::new()
                        .max_rect(Rect::from_min_size(item_rect.min, vec2(rect.width(), 2000.0)))
                        .id_salt(("item", i)),
                );
                child.set_clip_rect(clip);
                child.multiply_opacity(alpha);
                item(&mut child, i);
                measured = child.min_rect().height();
            });
            if (measured - heights[i]).abs() > 0.5 {
                heights[i] = measured;
                ui.ctx().request_repaint();
            }
        }
        glowkit_core::set_state(ui, id.with("h"), heights);
        response
    }
}
