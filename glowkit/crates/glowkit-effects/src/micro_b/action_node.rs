use super::common::{avatar, cr, hairline, icon, idle_for, shadow_color, surface, surface_hover, MicroIcon};
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use glowkit_core::anim::{spring, stagger};
use glowkit_core::color::{hex, with_alpha};
use glowkit_core::{paint, Theme};

#[derive(Clone, Copy, Debug)]
struct NodeState {
    open: bool,
    since: f64,
}

/// An automation-step card (think one node of a workflow editor) with its controls folded into
/// the top-right corner. Hover it and the round action buttons fan out around the corner one by
/// one on springs; move away and they tuck back in, last one first.
///
/// [`ActionNode::show`] returns the index of the action button clicked this frame.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let clicked = glowkit_effects::ActionNode::new("Node 07", "Summarises the thread and posts it back.")
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct ActionNode {
    title: String,
    body: String,
    actions: Vec<MicroIcon>,
    people: Vec<[Color32; 2]>,
    extra: usize,
    width: f32,
    corner: f32,
    bounce: f32,
    stagger: f32,
    dark: bool,
    stroke: bool,
    autoplay: bool,
}

impl ActionNode {
    /// A node card with a title and a short description.
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            actions: vec![MicroIcon::Link, MicroIcon::Plus, MicroIcon::Copy, MicroIcon::Sliders],
            people: vec![
                [hex("#60A5FA"), hex("#6366F1")],
                [hex("#FB7185"), hex("#F97316")],
                [hex("#F472B6"), hex("#C084FC")],
            ],
            extra: 3,
            width: 272.0,
            corner: 20.0,
            bounce: 0.2,
            stagger: 0.55,
            dark: false,
            stroke: true,
            autoplay: true,
        }
    }
    /// The corner action buttons (default: link, add, duplicate, settings).
    pub fn actions(mut self, icons: Vec<MicroIcon>) -> Self {
        self.actions = icons;
        self
    }
    /// Show only the first `n` action buttons.
    pub fn buttons(mut self, n: usize) -> Self {
        self.actions.truncate(n);
        self
    }
    /// Avatar gradients in the footer and how many more people to count after them.
    pub fn people(mut self, colors: Vec<[Color32; 2]>, more: usize) -> Self {
        self.people = colors;
        self.extra = more;
        self
    }
    /// Card width.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(160.0);
        self
    }
    /// Corner radius.
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// Overshoot of the buttons as they pop out (0..1).
    pub fn bounce(mut self, k: f32) -> Self {
        self.bounce = k.clamp(0.0, 1.0);
        self
    }
    /// Delay between buttons (0..1).
    pub fn stagger(mut self, k: f32) -> Self {
        self.stagger = k.clamp(0.0, 1.0);
        self
    }
    /// Dark card.
    pub fn dark(mut self, on: bool) -> Self {
        self.dark = on;
        self
    }
    /// Hairline outline (default on).
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Unfold and fold on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the node. Returns the index of the action button clicked this frame.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = Theme::of(ui);
        let (bg, fg, sub) = if self.dark {
            (t.primary, t.primary_foreground, with_alpha(t.primary_foreground, 0.6))
        } else {
            (surface(&t), t.foreground, t.muted_foreground)
        };
        let painter = ui.painter().clone();
        let inner_w = self.width - 32.0;
        let body = painter.layout(self.body.clone(), FontId::proportional(12.5), sub, inner_w);
        let card_h = 16.0 + 26.0 + 10.0 + body.size().y + 14.0 + 22.0 + 16.0;
        let margin = 48.0;
        let (outer, _) = ui.allocate_exact_size(vec2(self.width + margin * 2.0, card_h + margin * 2.0), Sense::hover());
        let card = Rect::from_center_size(outer.center(), vec2(self.width, card_h));
        let id = ui.scope_id().with(("glowkit_action_node", &self.title));
        let card_resp = ui.interact(card, id.with("card"), Sense::hover());
        let now = ui.input(|i| i.time);
        let n = self.actions.len();
        let corner = card.right_top() + vec2(-14.0, 14.0);
        // the buttons sit on an L just outside the corner: along the top edge, then down the side
        const SPOTS: [(f32, f32); 6] = [(-52.0, -20.0), (-18.0, -20.0), (20.0, 14.0), (20.0, 48.0), (20.0, 82.0), (-86.0, -20.0)];
        let spot = |i: usize| -> Pos2 {
            let (x, y) = SPOTS[i % SPOTS.len()];
            card.right_top() + vec2(x, y)
        };
        // hovering the fan keeps it open
        let fan = Rect::from_center_size(corner, Vec2::splat(110.0));
        let hovering = card_resp.hovered() || ui.rect_contains_pointer(fan) && ui.rect_contains_pointer(outer);
        let idle = idle_for(ui, id, hovering);
        let auto = self.autoplay && idle > 2.0;
        let st = glowkit_core::with_state(ui, id, || NodeState { open: false, since: now }, |s| {
            let want = if auto {
                if s.open { now - s.since < 2.6 } else { now - s.since > 1.4 }
            } else {
                hovering
            };
            if want != s.open {
                s.open = want;
                s.since = now;
            }
            *s
        });
        if auto {
            glowkit_core::animate(ui);
        }
        let lift = spring(ui, id.with("lift"), if st.open { 1.0 } else { 0.0 }, 260.0, 24.0);
        let since = (now - st.since) as f32;
        let delay = 0.02 + 0.09 * self.stagger;
        let mut clicked = None;
        let mut ps = Vec::with_capacity(n);
        for i in 0..n {
            let order = if st.open { i } else { n - 1 - i };
            let gate = stagger(since, order, delay, 0.001);
            let on = if st.open { gate >= 1.0 } else { gate < 1.0 };
            let k = spring(ui, id.with(("btn", i)), if on { 1.0 } else { 0.0 }, 360.0, 26.0 - 18.0 * self.bounce);
            let r = Rect::from_center_size(spot(i), Vec2::splat(30.0));
            let resp = ui.interact(r, id.with(("act", i)), Sense::click());
            let hov = spring(ui, id.with(("hov", i)), if resp.hovered() && k > 0.5 { 1.0 } else { 0.0 }, 400.0, 22.0);
            if resp.clicked() && k > 0.5 {
                clicked = Some(i);
            }
            if resp.hovered() && k > 0.5 {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            ps.push((k, hov));
        }
        if !ui.is_rect_visible(outer) {
            return clicked;
        }
        // card
        let line = if self.stroke { hairline(&t) } else { Stroke::NONE };
        let card_l = card.translate(vec2(0.0, -2.0 * lift));
        paint::soft_shadow(&painter, card_l, self.corner, 16.0 + 10.0 * lift, vec2(0.0, 4.0 + 4.0 * lift), shadow_color(&t, 0.5 + 0.4 * lift));
        painter.rect(card_l, cr(self.corner), bg, line, egui::StrokeKind::Inside);
        let x0 = card_l.min.x + 16.0;
        let tile = Rect::from_min_size(pos2(x0, card_l.min.y + 16.0), vec2(26.0, 26.0));
        painter.rect_filled(tile, cr(8.0), if self.dark { with_alpha(fg, 0.1) } else { surface_hover(&t) });
        icon(&painter, MicroIcon::Node, tile.center(), 13.0, fg, 1.4);
        painter.text(pos2(tile.max.x + 10.0, tile.center().y), Align2::LEFT_CENTER, &self.title, FontId::proportional(14.5), fg);
        let by = tile.max.y + 10.0;
        painter.galley(pos2(x0, by), body.clone(), sub);
        let fy = by + body.size().y + 14.0 + 11.0;
        let ring = bg;
        for (i, cols) in self.people.iter().enumerate() {
            avatar(&painter, pos2(x0 + 10.0 + i as f32 * 13.0, fy), 9.0, *cols, Some(ring));
        }
        if self.extra > 0 {
            let ex = x0 + 10.0 + self.people.len() as f32 * 13.0 + 4.0;
            painter.text(pos2(ex, fy), Align2::LEFT_CENTER, format!("+{}", self.extra), FontId::proportional(11.5), sub);
        }
        // action buttons grow out of the corner along their arc
        for (i, (k, hov)) in ps.iter().copied().enumerate() {
            if k <= 0.01 {
                continue;
            }
            let p = corner + (spot(i) - corner) * k;
            let r = (12.0 + 1.5 * hov) * k.clamp(0.0, 1.2);
            let a = k.clamp(0.0, 1.0);
            let br = Rect::from_center_size(p, Vec2::splat(r * 2.0));
            paint::soft_shadow(&painter, br, r, 8.0, vec2(0.0, 2.0), shadow_color(&t, 0.6 * a));
            painter.circle(p, r, with_alpha(bg, a), Stroke::new(1.0, with_alpha(line.color, a)));
            icon(&painter, self.actions[i], p, 11.0 * k.clamp(0.3, 1.1), with_alpha(fg, a), 1.4);
        }
        clicked
    }
}
