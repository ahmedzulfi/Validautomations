use super::common::{card, cr, hairline, hover_fill, icon, idle_for, text_width, well, BeIcon};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Sense, Stroke, Ui};
use glowkit_core::anim::spring;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Copy, Debug)]
struct BarState {
    open: bool,
    since: f64,
}

struct BarItem {
    icon: BeIcon,
    label: String,
    shortcut: String,
    badge: u32,
}

/// A floating toolbar of icons that unfolds into labelled actions. Each slot widens on its own
/// slightly staggered spring, so the bar ripples open from left to right: labels and keyboard
/// hints slide out from behind the icons, notification badges ride along, and a soft highlight
/// glides between whichever action is under the pointer. A pill below toggles it.
///
/// [`ExpandableActionBar::show`] returns the index of the action clicked this frame.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BeIcon, ExpandableActionBar};
/// let clicked = ExpandableActionBar::new()
///     .item(BeIcon::Send, "Send", "S")
///     .item(BeIcon::Copy, "Copy", "C")
///     .badge_item(BeIcon::Bell, "Alerts", 3)
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct ExpandableActionBar {
    items: Vec<BarItem>,
    toggle: bool,
    stagger: f32,
    autoplay: bool,
}

impl Default for ExpandableActionBar {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpandableActionBar {
    /// An empty bar.
    pub fn new() -> Self {
        Self { items: Vec::new(), toggle: true, stagger: 0.03, autoplay: true }
    }
    /// Add an action with a keyboard hint (use `""` for none).
    pub fn item(mut self, icon: BeIcon, label: impl Into<String>, shortcut: impl Into<String>) -> Self {
        self.items.push(BarItem { icon, label: label.into(), shortcut: shortcut.into(), badge: 0 });
        self
    }
    /// Add an action that carries a red count badge.
    pub fn badge_item(mut self, icon: BeIcon, label: impl Into<String>, count: u32) -> Self {
        self.items.push(BarItem { icon, label: label.into(), shortcut: String::new(), badge: count });
        self
    }
    /// Show the Expand / Collapse pill under the bar (default on).
    pub fn toggle_button(mut self, on: bool) -> Self {
        self.toggle = on;
        self
    }
    /// Delay between neighbouring slots as the bar ripples open, in seconds.
    pub fn stagger(mut self, s: f32) -> Self {
        self.stagger = s.clamp(0.0, 0.2);
        self
    }
    /// Expand and collapse on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the bar. Returns the action clicked this frame.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = Theme::of(ui);
        let painter = ui.painter().clone();
        let n = self.items.len();
        let slot = 34.0;
        let pad = 5.0;
        let open_w: Vec<f32> = self
            .items
            .iter()
            .map(|it| {
                let mut w = slot + text_width(&painter, &it.label, 13.0) + 4.0;
                if !it.shortcut.is_empty() {
                    w += text_width(&painter, &it.shortcut, 10.0) + 8.0;
                }
                if it.badge > 0 {
                    w += 20.0;
                }
                w + 8.0
            })
            .collect();
        let max_w = open_w.iter().sum::<f32>() + pad * 2.0;
        let bar_h = slot + pad * 2.0;
        let total_h = bar_h + if self.toggle { 58.0 } else { 0.0 };
        let (outer, resp) = ui.allocate_exact_size(vec2(max_w.max(160.0), total_h), Sense::hover());
        let id = resp.id;
        let now = ui.input(|i| i.time);
        let st0: BarState = glowkit_core::state(ui, id, || BarState { open: false, since: now - 10.0 });
        // current slot widths
        let widths: Vec<f32> = (0..n)
            .map(|i| {
                let delay = (if st0.open { i } else { n - 1 - i }) as f32 * self.stagger;
                let go = (now - st0.since) as f32 >= delay;
                let target = if st0.open == go { open_w[i] } else { slot };
                // springs settle the width, a little bounce on the way out
                spring(ui, id.with(("w", i)), target, 380.0, 26.0)
            })
            .collect();
        let bar_w = widths.iter().sum::<f32>() + pad * 2.0;
        let bar = Rect::from_center_size(pos2(outer.center().x, outer.min.y + bar_h / 2.0), vec2(bar_w, bar_h));
        // hit testing
        let ptr = ui.input(|i| i.pointer.hover_pos());
        let mut x = bar.min.x + pad;
        let mut slots = Vec::new();
        for w in &widths {
            slots.push(Rect::from_min_size(pos2(x, bar.min.y + pad), vec2(*w, slot)));
            x += w;
        }
        let hov = ptr.and_then(|p| slots.iter().position(|r| r.contains(p)));
        let mut clicked = None;
        for (i, r) in slots.iter().enumerate() {
            let resp = ui.interact(*r, id.with(("slot", i)), Sense::click());
            if resp.clicked() {
                clicked = Some(i);
            }
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
        let label = if st0.open { "Collapse" } else { "Expand" };
        let bw = spring(ui, id.with("bw"), text_width(&painter, label, 12.5) + 46.0, 400.0, 28.0);
        let btn = Rect::from_center_size(pos2(outer.center().x, bar.max.y + 32.0), vec2(bw, 32.0));
        let mut toggle = false;
        if self.toggle {
            let r = ui.interact(btn, id.with("toggle"), Sense::click());
            toggle = r.clicked();
            if r.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
        let active = ui.rect_contains_pointer(outer);
        let idle = idle_for(ui, id, active);
        let auto = self.autoplay && idle > 2.0;
        let st = glowkit_core::with_state(
            ui,
            id,
            || st0,
            |s| {
                if toggle || (auto && now - s.since > if s.open { 2.8 } else { 1.8 }) {
                    s.open = !s.open;
                    s.since = now;
                }
                *s
            },
        );
        if auto || (now - st.since) < n as f64 * self.stagger as f64 + 0.1 {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(outer) {
            return clicked;
        }
        card(&painter, &t, bar, bar_h / 2.0, 0.6);
        // gliding hover highlight
        let auto_hover = if auto && st.open { Some(((now * 0.9) as usize) % n.max(1)) } else { None };
        let hi = hov.or(auto_hover);
        let hx = spring(ui, id.with("hx"), hi.map(|i| slots[i].min.x).unwrap_or(bar.center().x), 420.0, 32.0);
        let hw = spring(ui, id.with("hw"), hi.map(|i| slots[i].width()).unwrap_or(0.0), 420.0, 32.0);
        let ha = spring(ui, id.with("ha"), if hi.is_some() { 1.0 } else { 0.0 }, 300.0, 26.0);
        if ha > 0.01 {
            let hr = Rect::from_min_size(pos2(hx, bar.min.y + pad), vec2(hw, slot));
            painter.rect_filled(hr, cr(slot / 2.0), with_alpha(hover_fill(&t), ha));
        }
        let clip = painter.with_clip_rect(bar.shrink(2.0));
        for (i, it) in self.items.iter().enumerate() {
            let r = slots[i];
            let k = ((r.width() - slot) / (open_w[i] - slot).max(1.0)).clamp(0.0, 1.0);
            let hot = hi == Some(i);
            let fg = if hot || i == 0 { t.foreground } else { mix(t.muted_foreground, t.foreground, 0.35) };
            let ic = pos2(r.min.x + slot / 2.0, r.center().y);
            icon(&clip, it.icon, ic, 15.0, fg, 1.4);
            let ir = clip.with_clip_rect(r.intersect(bar.shrink(2.0)));
            let lx = r.min.x + slot - 4.0 + (1.0 - k) * -8.0;
            let lr = ir.text(pos2(lx, r.center().y), Align2::LEFT_CENTER, &it.label, FontId::proportional(13.0), with_alpha(fg, k));
            let mut ex = lr.max.x + 6.0;
            if !it.shortcut.is_empty() {
                ir.text(
                    pos2(ex, r.center().y + 0.5),
                    Align2::LEFT_CENTER,
                    &it.shortcut,
                    FontId::proportional(10.0),
                    with_alpha(t.muted_foreground, k),
                );
                ex += 14.0;
            }
            if it.badge > 0 {
                // the badge rides from the icon's shoulder to the end of the label
                let from = ic + vec2(7.0, -7.0);
                let to = pos2(ex + 2.0, r.center().y);
                let bc = from.lerp(to, k);
                let br = 7.5 - 0.5 * k;
                clip.circle_filled(bc, br + 1.5, super::common::surface(&t));
                clip.circle_filled(bc, br, t.destructive);
                clip.text(bc, Align2::CENTER_CENTER, it.badge.to_string(), FontId::proportional(9.5), Color32::WHITE);
            }
        }
        if self.toggle {
            let hovb = ptr.is_some_and(|p| btn.contains(p));
            painter.rect(btn, cr(16.0), if hovb { hover_fill(&t) } else { well(&t) }, hairline(&t), egui::StrokeKind::Inside);
            // arrows that point out (expand) or in (collapse)
            let ic = pos2(btn.min.x + 18.0, btn.center().y);
            let k = spring(ui, id.with("arrow"), if st.open { 1.0 } else { 0.0 }, 300.0, 22.0);
            let st_ = Stroke::new(1.3, t.foreground);
            for sgn in [-1.0f32, 1.0] {
                let d = vec2(1.0, -1.0) * sgn;
                let a = ic + d * 1.5;
                let b = ic + d * 5.5;
                painter.line_segment([a, b], st_);
                let tip = if k < 0.5 { b } else { a };
                let back = if k < 0.5 { -d } else { d };
                let n1 = vec2(back.x, 0.0) * 3.0;
                let n2 = vec2(0.0, back.y) * 3.0;
                painter.line_segment([tip, tip + n1], st_);
                painter.line_segment([tip, tip + n2], st_);
            }
            painter.text(pos2(btn.min.x + 30.0, btn.center().y), Align2::LEFT_CENTER, label, FontId::proportional(12.5), t.foreground);
        }
        clicked
    }
}
