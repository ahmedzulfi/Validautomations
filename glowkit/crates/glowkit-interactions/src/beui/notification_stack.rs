use super::common::{card, cr, hairline, icon, idle_for, lerp, morph, shadow_color, well, BeIcon};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Sense, Ui};
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::{paint, Theme};

#[derive(Clone, Copy, Debug)]
struct StackState {
    open: bool,
    since: f64,
}

/// Notifications stacked like a deck. Collapsed, the newest card sits on top with the others
/// peeking out underneath, narrower and dimmer, over a count badge; press the stack and the
/// cards fan down into a full list (each card's position, width and shade morph to its slot)
/// while the container grows to hold them. Press again to stack them back up.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::NotificationStack;
/// ui.add(
///     NotificationStack::new()
///         .notice("Orders import failed", "42s · TimeoutError at step 2", 2)
///         .notice("SLA breach", "2m 11s · Data enrichment", 0),
/// );
/// ```
pub struct NotificationStack {
    items: Vec<(String, String, u32)>,
    width: f32,
    badge: Color32,
    autoplay: bool,
}

impl Default for NotificationStack {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationStack {
    /// An empty stack.
    pub fn new() -> Self {
        Self { items: Vec::new(), width: 340.0, badge: hex("#F97316"), autoplay: true }
    }
    /// Add a notification: title, detail line and a retry count (0 hides it).
    pub fn notice(mut self, title: impl Into<String>, detail: impl Into<String>, retries: u32) -> Self {
        self.items.push((title.into(), detail.into(), retries));
        self
    }
    /// Width of the stack.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(240.0);
        self
    }
    /// Color of the count badge.
    pub fn badge(mut self, c: Color32) -> Self {
        self.badge = c;
        self
    }
    /// Expand and collapse on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for NotificationStack {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let n = self.items.len();
        let pad = 12.0;
        let card_h = 66.0;
        let gap = 8.0;
        let foot = 46.0;
        let open_h = pad + n as f32 * (card_h + gap) - gap + foot + 4.0;
        let closed_h = pad + card_h + 2.0 * 7.0 + foot;
        let (outer, resp) = ui.allocate_exact_size(vec2(self.width, open_h), Sense::click());
        let id = resp.id;
        let now = ui.input(|i| i.time);
        let idle = idle_for(ui, id, resp.hovered());
        let auto = self.autoplay && idle > 2.0;
        let st = glowkit_core::with_state(ui, id, || StackState { open: false, since: now }, |s| {
            if resp.clicked() || (auto && now - s.since > if s.open { 2.8 } else { 1.8 }) {
                s.open = !s.open;
                s.since = now;
            }
            *s
        });
        if auto {
            glowkit_core::animate(ui);
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let top = outer.min.y;
        let container_t = Rect::from_min_size(pos2(outer.min.x, top), vec2(self.width, if st.open { open_h } else { closed_h }));
        let (container, cr_) = morph(ui, id.with("box"), container_t, 28.0, 300.0, 24.0);
        let k = ((container.height() - closed_h) / (open_h - closed_h)).clamp(-0.1, 1.1);
        if !ui.is_rect_visible(outer) {
            return resp;
        }
        let painter = ui.painter().clone();
        card(&painter, &t, container, cr_, 0.5);
        let inner_w = self.width - pad * 2.0;
        // back to front so the newest card paints last
        for i in (0..n).rev() {
            let (title, detail, retries) = &self.items[i];
            // stacked slot: shrinking, sliding down under the front card
            let depth = (i as f32).min(3.0);
            let stacked = Rect::from_min_size(
                pos2(container.min.x + pad + depth * 10.0, top + pad + depth * 7.0),
                vec2(inner_w - depth * 20.0, card_h),
            );
            let listed = Rect::from_min_size(pos2(container.min.x + pad, top + pad + i as f32 * (card_h + gap)), vec2(inner_w, card_h));
            // each card follows on its own spring, a little later the deeper it is
            let target = if st.open { listed } else { stacked };
            let (r, _) = morph(ui, id.with(("c", i)), target, 18.0, 320.0 - i as f32 * 30.0, 24.0 - i as f32 * 1.5);
            let hidden = if st.open { 0.0 } else { ((depth - 2.0) / 1.0).clamp(0.0, 1.0) };
            let dim = lerp(depth * 0.18, 0.0, k.clamp(0.0, 1.0)).min(0.6);
            if hidden >= 1.0 && i > 2 && k < 0.05 {
                continue;
            }
            let fill = mix(well(&t), if t.dark { t.background } else { t.muted }, dim);
            paint::soft_shadow(&painter, r, 18.0, 10.0, vec2(0.0, 2.0), shadow_color(&t, 0.25 * (1.0 - dim)));
            painter.rect(r, cr(18.0), fill, hairline(&t), egui::StrokeKind::Inside);
            // content fades on the cards that are tucked under
            let ca = if i == 0 { 1.0 } else { k.clamp(0.0, 1.0) };
            if ca > 0.02 {
                let clip = painter.with_clip_rect(r);
                clip.text(pos2(r.min.x + 16.0, r.min.y + 24.0), Align2::LEFT_CENTER, title, FontId::proportional(14.0), with_alpha(t.foreground, ca));
                clip.text(pos2(r.min.x + 16.0, r.min.y + 45.0), Align2::LEFT_CENTER, detail, FontId::proportional(11.5), with_alpha(t.muted_foreground, ca));
                if *retries > 0 {
                    let amber = t.warning;
                    let ic = pos2(r.max.x - 34.0, r.min.y + 24.0);
                    super::common::icon(&clip, BeIcon::Refresh, ic, 12.0, with_alpha(amber, ca), 1.4);
                    clip.text(ic + vec2(10.0, 0.0), Align2::LEFT_CENTER, retries.to_string(), FontId::proportional(12.0), with_alpha(amber, ca));
                }
            }
        }
        // footer: badge + label crossfade
        let fy = container.max.y - foot / 2.0 - 2.0;
        let bc = pos2(container.min.x + pad + 14.0, fy);
        painter.circle_filled(bc, 13.0, self.badge);
        painter.text(bc, Align2::CENTER_CENTER, n.to_string(), FontId::proportional(12.0), Color32::WHITE);
        let kk = k.clamp(0.0, 1.0);
        painter.text(pos2(bc.x + 22.0, fy - kk * 6.0), Align2::LEFT_CENTER, "Notifications", FontId::proportional(14.0), with_alpha(t.foreground, 1.0 - kk));
        let vr = painter.text(pos2(bc.x + 22.0, fy + (1.0 - kk) * 6.0), Align2::LEFT_CENTER, "View all", FontId::proportional(14.0), with_alpha(t.foreground, kk));
        icon(&painter, BeIcon::ArrowUpRight, pos2(vr.max.x + 10.0, fy + (1.0 - kk) * 6.0), 8.0, with_alpha(t.foreground, kk), 1.4);
        resp
    }
}
