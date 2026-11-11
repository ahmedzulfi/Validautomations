//! Scrolling message list that sticks to the bottom.

use crate::common;
use crate::icons::{paint_icon, Icon};
use egui::{pos2, vec2, Color32, Rect, Sense, Stroke, Ui};
use glowkit_core::color::with_alpha;
use glowkit_core::{paint, Theme};

#[derive(Clone, Copy, Default)]
struct ConvState {
    animating: bool,
}

/// A scroll container for chat messages. It stays pinned to the bottom while new content streams
/// in; once the user scrolls up it stops following and a round "scroll to bottom" button floats
/// above the content, which glides back down when clicked.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_ai::Conversation::new().height(400.0).show(ui, |ui| {
///     // add ChatMessage widgets here
/// });
/// ```
#[must_use = "call `.show(ui, ..)`"]
pub struct Conversation {
    height: Option<f32>,
    fade: Option<Color32>,
    id_salt: Option<egui::IdSalt>,
    max_width: Option<f32>,
}

impl Default for Conversation {
    fn default() -> Self {
        Self::new()
    }
}

impl Conversation {
    /// A conversation filling the available height.
    pub fn new() -> Self {
        Self { height: None, fade: None, id_salt: None, max_width: None }
    }
    /// Fixed viewport height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = Some(h);
        self
    }
    /// Fade the top and bottom edges into `background`.
    pub fn fade_edges(mut self, background: Color32) -> Self {
        self.fade = Some(background);
        self
    }
    /// Center the content column at this maximum width (like chat apps do on wide screens).
    pub fn max_width(mut self, w: f32) -> Self {
        self.max_width = Some(w);
        self
    }
    /// Stable id (keeps the scroll position).
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the conversation; `add` lays out the messages.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
        let t = Theme::of(ui);
        let id = self.id_salt.map(|s| ui.scope_id().with_salt(s)).unwrap_or_else(|| ui.scope_id().with("conversation"));
        let mut st: ConvState = glowkit_core::state(ui, id.with("st"), ConvState::default);
        let height = self.height.unwrap_or_else(|| ui.available_height());
        let mut sa = egui::ScrollArea::vertical()
            .id_salt(id.with("scroll"))
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .max_height(height)
            .min_scrolled_height(height);
        let prev: Option<(f32, f32, f32)> = glowkit_core::state(ui, id.with("metrics"), || None);
        if st.animating {
            if let Some((off, content_h, view_h)) = prev {
                let max = (content_h - view_h).max(0.0);
                let next = glowkit_core::anim::approach(off, max, 12.0, glowkit_core::dt(ui));
                let next = if (max - next).abs() < 1.0 { max } else { next };
                if next >= max {
                    st.animating = false;
                }
                sa = sa.vertical_scroll_offset(next);
                ui.ctx().request_repaint();
            }
        }
        let max_width = self.max_width;
        let out = sa.show(ui, |ui| {
            ui.add_space(4.0);
            match max_width {
                Some(mw) if ui.available_width() > mw => {
                    let pad = (ui.available_width() - mw) / 2.0;
                    let r = ui.horizontal(|ui| {
                        ui.add_space(pad);
                        ui.vertical(|ui| {
                            ui.set_width(mw);
                            add(ui)
                        })
                        .inner
                    });
                    r.inner
                }
                _ => add(ui),
            }
        });
        let off = out.state.clamped_offset().y;
        let content_h = out.content_size.y;
        let view = out.inner_rect;
        glowkit_core::set_state(ui, id.with("metrics"), Some((off, content_h, view.height())));
        let distance = (content_h - view.height() - off).max(0.0);
        let show_btn = distance > 60.0 && !st.animating;
        if let Some(bg) = self.fade {
            let p = ui.painter().with_clip_rect(view);
            let edge = 18.0;
            let top = (off / 30.0).clamp(0.0, 1.0);
            if top > 0.0 {
                paint::quad(&p, [view.left_top(), view.right_top(), pos2(view.max.x, view.min.y + edge), pos2(view.min.x, view.min.y + edge)], [bg.gamma_multiply(top), bg.gamma_multiply(top), Color32::TRANSPARENT, Color32::TRANSPARENT]);
            }
            let bot = (distance / 30.0).clamp(0.0, 1.0);
            if bot > 0.0 {
                paint::quad(&p, [pos2(view.min.x, view.max.y - edge), pos2(view.max.x, view.max.y - edge), view.right_bottom(), view.left_bottom()], [Color32::TRANSPARENT, Color32::TRANSPARENT, bg.gamma_multiply(bot), bg.gamma_multiply(bot)]);
            }
        }
        let a = common::anim_bool(ui, id.with("btn_a"), show_btn, 0.18);
        if a > 0.0 {
            let c = pos2(view.center().x, view.max.y - 26.0 + (1.0 - a) * 10.0);
            let r = Rect::from_center_size(c, vec2(34.0, 34.0));
            let resp = ui.interact(r, id.with("to_bottom"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Scroll to bottom");
            let h = common::anim_bool(ui, id.with("btn_h"), resp.hovered(), 0.12);
            let p = ui.painter();
            paint::soft_shadow(p, r, 17.0, 14.0, vec2(0.0, 4.0), Color32::from_black_alpha(((if t.dark { 110.0 } else { 30.0 }) * a) as u8));
            p.circle_filled(c, 17.0, with_alpha(glowkit_core::color::mix(t.popover, common::raised(&t), h), a));
            p.circle_stroke(c, 17.0, Stroke::new(1.0, with_alpha(t.border, a)));
            paint_icon(p, Icon::ArrowDown, Rect::from_center_size(c, vec2(15.0, 15.0)), with_alpha(t.foreground, a));
            if resp.clicked() && show_btn {
                st.animating = true;
                ui.ctx().request_repaint();
            }
        }
        glowkit_core::set_state(ui, id.with("st"), st);
        out.inner
    }
}
