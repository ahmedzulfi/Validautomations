//! Dismissible feature tips: a popover with an arrow, or an inline card that folds into layout.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Context, Id, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;
use glowkit_effects::MicroIcon;

#[derive(Clone, Copy, Default)]
struct TipState {
    dismissed: bool,
    closing: bool,
    shown_at: f32,
    started: bool,
    appear: f32,
    appear_v: f32,
    off: Vec2,
    vel: Vec2,
    flying: bool,
    fade: f32,
    ghost_done: bool,
}

/// A TipKit-style feature tip that points at a control (or sits inline in a list).
///
/// As a popover it grows out of its arrow on a bouncy spring, anchored under the control you
/// point it at. It is tactile: drag it and it follows with rubber-band resistance, fling it and it
/// flies off in that direction and is dismissed; let go gently and it springs home. The close
/// button shrinks it back into its arrow. As an inline card ([`CoachTip::inline`]) it unfolds
/// into the layout, pushing content down, and folds away when closed. Dismissal is remembered
/// per id ([`CoachTip::dismissed`], [`CoachTip::reset`]), so several tips can be shown one after
/// another.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::CoachTip;
/// let anchor = egui::pos2(300.0, 40.0);
/// ui.add(CoachTip::new("new-note", "Start a note fast", "Tap here any time to begin a new note.").anchor(anchor));
/// if !CoachTip::dismissed(ui.ctx(), "new-note") { /* ... */ }
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CoachTip {
    key: Id,
    title: String,
    body: String,
    icon: MicroIcon,
    accent: Option<Color32>,
    anchor: Option<Pos2>,
    bounds: Option<Rect>,
    width: f32,
    inline: bool,
    bounce: f32,
    autoplay: bool,
}

fn key_of(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Id {
    Id::unique(id_salt).with("coach_tip")
}

impl CoachTip {
    /// A tip remembered under `id_salt`.
    pub fn new(
        id_salt: impl std::hash::Hash + std::fmt::Debug,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            key: key_of(id_salt),
            title: title.into(),
            body: body.into(),
            icon: MicroIcon::Doc,
            accent: None,
            anchor: None,
            bounds: None,
            width: 236.0,
            inline: false,
            bounce: 45.0,
            autoplay: false,
        }
    }
    /// Whether the tip under `id_salt` has been dismissed.
    pub fn dismissed(ctx: &Context, id_salt: impl std::hash::Hash + std::fmt::Debug) -> bool {
        let k = key_of(id_salt);
        ctx.data(|d| d.get_temp::<TipState>(k).is_some_and(|s| s.dismissed))
    }
    /// Show the tip under `id_salt` again.
    pub fn reset(ctx: &Context, id_salt: impl std::hash::Hash + std::fmt::Debug) {
        let k = key_of(id_salt);
        ctx.data_mut(|d| d.remove::<TipState>(k));
    }
    /// Point the popover's arrow at this screen position (it hangs below it).
    pub fn anchor(mut self, p: Pos2) -> Self {
        self.anchor = Some(p);
        self
    }
    /// Keep the popover inside this rect (default: the ui's clip rect).
    pub fn bounds(mut self, r: Rect) -> Self {
        self.bounds = Some(r);
        self
    }
    /// Show as an inline card that takes space in the layout instead of a popover.
    pub fn inline(mut self, on: bool) -> Self {
        self.inline = on;
        self
    }
    /// Leading icon.
    pub fn icon(mut self, i: MicroIcon) -> Self {
        self.icon = i;
        self
    }
    /// Icon color (default: the system blue).
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = Some(c);
        self
    }
    /// Popover width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(140.0);
        self
    }
    /// Overshoot of the pop-in, 0..100.
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 100.0);
        self
    }
    /// Flick itself away (with a ghost finger) after a few seconds.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for CoachTip {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let mut st: TipState = glowkit_core::state(ui, self.key, TipState::default);
        if !st.started {
            st.started = true;
            st.shown_at = now;
        }
        let accent = self.accent.unwrap_or_else(|| system_blue(&t));
        let pad = 14.0;
        let text_w = if self.inline { ui.available_width() - 2.0 * pad - 40.0 } else { self.width - 2.0 * pad - 40.0 };
        let body_g = ui.painter().layout(
            self.body.clone(),
            egui::FontId::proportional(12.5),
            t.muted_foreground,
            text_w.max(60.0),
        );
        let card_h = (pad * 2.0 + 20.0 + body_g.size().y).max(64.0);
        let visible_t = if st.dismissed || st.closing || st.flying { 0.0 } else { 1.0 };
        let stiff = 300.0;
        spring_step(&mut st.appear, &mut st.appear_v, visible_t, stiff, damping_for(self.bounce, stiff), dt);

        // layout
        let (rect, card) = if self.inline {
            let h = (card_h + 10.0) * st.appear.clamp(0.0, 1.05);
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), h.max(0.0)), Sense::hover());
            (rect, Rect::from_min_size(rect.min, vec2(rect.width(), card_h)))
        } else {
            let a = self.anchor.unwrap_or_else(|| ui.cursor().min);
            let clip = self.bounds.unwrap_or_else(|| ui.clip_rect());
            let x =
                (a.x - self.width * 0.7).clamp(clip.min.x + 8.0, (clip.max.x - 8.0 - self.width).max(clip.min.x + 8.0));
            let card = Rect::from_min_size(pos2(x, a.y + 10.0), vec2(self.width, card_h));
            let (rect, _) = ui.allocate_exact_size(Vec2::ZERO, Sense::hover());
            (rect, card)
        };
        let card = card.translate(st.off);
        let resp = ui.interact(card, self.key.with("hit"), Sense::click_and_drag());
        let close_c = pos2(card.max.x - 18.0, card.min.y + 18.0);

        // interaction (real)
        if !st.dismissed && !st.closing && !st.flying && !self.inline {
            if resp.dragged() {
                let d = resp.drag_delta();
                st.off += d * (1.0 / (1.0 + st.off.length() / 50.0));
                st.vel = d / dt.max(1e-3);
            }
            if resp.drag_stopped() && (st.off.length() > 55.0 || st.vel.length() > 800.0) {
                st.flying = true;
                st.vel = st.vel.normalized() * st.vel.length().max(900.0);
            }
        }
        if resp.clicked() && resp.interact_pointer_pos().is_some_and(|p| (p - close_c).length() < 16.0) {
            st.closing = true;
        }
        // ghost: flick the tip away after a while
        let mut ghost: Option<(Pos2, f32, f32)> = None;
        if self.autoplay && !st.dismissed && !self.inline && !resp.hovered() {
            let e = now - st.shown_at;
            let (t0, t1) = (2.6, 2.95);
            let start = card.center() - st.off;
            if e > t0 - 0.6 && e < t1 + 0.6 {
                let alpha = ramp(e, t0 - 0.6, t0 - 0.2) * (1.0 - ramp(e, t1 + 0.2, t1 + 0.6));
                let pressed = if e >= t0 && e <= t1 { 1.0 } else { 0.0 };
                if e >= t0 && e <= t1 && !st.flying {
                    let u = (e - t0) / (t1 - t0);
                    let target = vec2(130.0, -24.0) * u * u;
                    let d = target - st.off;
                    st.vel = d / dt.max(1e-3);
                    st.off = target;
                }
                if e > t1 && !st.flying && !st.ghost_done {
                    st.ghost_done = true;
                    st.flying = true;
                    st.vel = vec2(1100.0, -200.0);
                }
                ghost = Some((start + if st.flying { vec2(140.0, -26.0) } else { st.off }, pressed, alpha));
            }
            ui.ctx().request_repaint();
        } else if self.autoplay && self.inline && !st.dismissed && !st.closing {
            let e = now - st.shown_at;
            if e > 3.2 {
                st.closing = true;
            }
            if e > 2.4 {
                ghost = Some((close_c, if e > 3.1 { 1.0 } else { 0.0 }, ramp(e, 2.4, 2.8)));
            }
            ui.ctx().request_repaint();
        }

        // physics
        if st.flying {
            st.off += st.vel * dt;
            st.vel *= 0.995;
            st.fade += dt * 3.5;
            if st.fade >= 1.0 {
                st.dismissed = true;
            }
        } else if !resp.dragged() {
            let mut vx = st.vel.x;
            let mut vy = st.vel.y;
            spring_step(&mut st.off.x, &mut vx, 0.0, 300.0, damping_for(self.bounce, 300.0), dt);
            spring_step(&mut st.off.y, &mut vy, 0.0, 300.0, damping_for(self.bounce, 300.0), dt);
            st.vel = vec2(vx, vy);
        }
        if st.closing && st.appear < 0.02 {
            st.dismissed = true;
        }
        let animating = st.flying
            || st.closing && !st.dismissed
            || st.off.length() > 0.2
            || !settled(st.appear, st.appear_v, visible_t);
        let s = st;
        glowkit_core::set_state(ui, self.key, st);
        if animating {
            ui.ctx().request_repaint();
        }
        if s.dismissed && !s.flying {
            let _ = rect;
            return resp;
        }

        // paint
        let painter = match self.bounds {
            Some(b) if !self.inline => ui.painter().with_clip_rect(b.intersect(ui.clip_rect())),
            _ => ui.painter().clone(),
        };
        let k = s.appear.max(0.0);
        let alpha = (k * 2.0).clamp(0.0, 1.0) * (1.0 - s.fade).clamp(0.0, 1.0);
        if alpha <= 0.01 {
            return resp;
        }
        let fill = if t.dark { hexa(0x2A, 0x2A, 0x2F) } else { Color32::WHITE };
        let (pivot, scale) = if self.inline {
            (card.center(), 1.0)
        } else {
            (self.anchor.unwrap_or(card.center_top()) + s.off, 0.55 + 0.45 * k)
        };
        let tr = |p: Pos2| pivot + (p - pivot) * scale;
        let r = Rect::from_min_max(tr(card.min), tr(card.max));
        shadow(&painter, &t, r, 16.0, 0.9 * alpha);
        if !self.inline {
            if let Some(a) = self.anchor {
                let ax = (a.x + s.off.x).clamp(r.min.x + 22.0, r.max.x - 22.0);
                let tip = tr(pos2(ax, a.y + 2.0 + s.off.y));
                let base_y = r.min.y + 1.0;
                painter.add(Shape::convex_polygon(
                    vec![tip, pos2(ax + 9.0 * scale, base_y), pos2(ax - 9.0 * scale, base_y)],
                    with_alpha(fill, alpha),
                    Stroke::NONE,
                ));
            }
        }
        let stroke = if t.dark { Stroke::new(1.0, with_alpha(t.border, alpha)) } else { Stroke::NONE };
        painter.rect(r, cr(16.0 * scale), with_alpha(fill, alpha), stroke, egui::StrokeKind::Inside);
        if scale > 0.8 {
            let ca = alpha * ((scale - 0.8) / 0.2).clamp(0.0, 1.0);
            let icon_c = pos2(r.min.x + pad + 13.0, r.min.y + pad + 13.0);
            self.icon.paint(&painter, icon_c, 24.0 * scale, with_alpha(accent, ca));
            let tx = r.min.x + pad + 38.0;
            text_bold(
                &painter,
                pos2(tx, r.min.y + pad + 9.0),
                Align2::LEFT_CENTER,
                &self.title,
                14.0,
                with_alpha(t.foreground, ca),
            );
            let bg = painter.layout(
                self.body.clone(),
                egui::FontId::proportional(12.5),
                with_alpha(t.muted_foreground, ca),
                text_w.max(60.0),
            );
            painter.galley(pos2(tx, r.min.y + pad + 20.0), bg, t.muted_foreground);
            let cc = pos2(r.max.x - 18.0, r.min.y + 18.0);
            glyph(&painter, Glyph::Close, cc, 9.0, with_alpha(t.muted_foreground, ca));
        }
        if let Some((p, pressed, a)) = ghost {
            ghost_touch(&painter, p, pressed, a);
        }
        resp
    }
}
