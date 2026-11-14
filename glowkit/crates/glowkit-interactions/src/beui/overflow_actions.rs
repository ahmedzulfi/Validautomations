use super::common::{card, cr, hover_fill, icon, idle_for, text_width, well, BeIcon};
use egui::{pos2, vec2, Align2, FontId, Rect, Sense, Stroke, Ui};
use glowkit_core::anim::spring;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Copy, Debug)]
struct OverflowState {
    open: bool,
    since: f64,
}

/// A small action group with an overflow button. The primary actions sit in a pill next to a
/// round "more" button; press it and the hidden actions slide out of it one by one, the pill
/// stretching to make room, while the three dots fold into a close cross. Press again and
/// everything tucks back in.
///
/// [`OverflowActions::show`] returns the index of the action clicked (primary first, then the
/// overflow ones).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BeIcon, OverflowActions};
/// let clicked = OverflowActions::new()
///     .action(BeIcon::Eye, "Preview")
///     .action(BeIcon::Pin, "Pin")
///     .more(BeIcon::Share, "Share")
///     .more(BeIcon::Trash, "Delete")
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct OverflowActions {
    primary: Vec<(BeIcon, String)>,
    more: Vec<(BeIcon, String)>,
    stagger: f32,
    autoplay: bool,
}

impl Default for OverflowActions {
    fn default() -> Self {
        Self::new()
    }
}

impl OverflowActions {
    /// An empty group.
    pub fn new() -> Self {
        Self { primary: Vec::new(), more: Vec::new(), stagger: 0.05, autoplay: true }
    }
    /// An always-visible action.
    pub fn action(mut self, icon: BeIcon, label: impl Into<String>) -> Self {
        self.primary.push((icon, label.into()));
        self
    }
    /// An action hidden behind the overflow button.
    pub fn more(mut self, icon: BeIcon, label: impl Into<String>) -> Self {
        self.more.push((icon, label.into()));
        self
    }
    /// Delay between hidden actions sliding out, in seconds.
    pub fn stagger(mut self, s: f32) -> Self {
        self.stagger = s.clamp(0.0, 0.3);
        self
    }
    /// Open and close on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the group. Returns the action clicked this frame.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = Theme::of(ui);
        let painter = ui.painter().clone();
        let h = 48.0;
        let pad = 5.0;
        let btn_h = h - pad * 2.0;
        let widths: Vec<f32> =
            self.primary.iter().chain(self.more.iter()).map(|(_, l)| text_width(&painter, l, 13.5) + 46.0).collect();
        let np = self.primary.len();
        let total_open = widths.iter().sum::<f32>() + btn_h + pad * 2.0 + 4.0 * widths.len() as f32;
        let (outer, resp) = ui.allocate_exact_size(vec2(total_open, h), Sense::hover());
        let id = resp.id;
        let now = ui.input(|i| i.time);
        let st0: OverflowState = glowkit_core::state(ui, id, || OverflowState { open: false, since: now - 10.0 });
        // each hidden slot opens on its own spring, staggered
        let slot_w: Vec<f32> = widths
            .iter()
            .enumerate()
            .map(|(i, w)| {
                if i < np {
                    *w
                } else {
                    let j = i - np;
                    let order = if st0.open { j } else { self.more.len() - 1 - j };
                    let go = (now - st0.since) as f32 >= order as f32 * self.stagger;
                    let tgt = if st0.open == go { *w } else { 0.0 };
                    spring(ui, id.with(("w", i)), tgt, 360.0, 24.0)
                }
            })
            .collect();
        let bar_w =
            slot_w.iter().sum::<f32>() + btn_h + pad * 2.0 + 4.0 * slot_w.iter().filter(|w| **w > 1.0).count() as f32;
        let bar = Rect::from_center_size(outer.center(), vec2(bar_w, h));
        let mut x = bar.min.x + pad;
        let mut rects = Vec::new();
        for w in &slot_w {
            rects.push(Rect::from_min_size(pos2(x, bar.min.y + pad), vec2(*w, btn_h)));
            if *w > 1.0 {
                x += w + 4.0;
            }
        }
        let more_c = pos2(bar.max.x - pad - btn_h / 2.0, bar.center().y);
        let more_r = Rect::from_center_size(more_c, vec2(btn_h, btn_h));
        let mut clicked = None;
        for (i, r) in rects.iter().enumerate() {
            if r.width() < 20.0 {
                continue;
            }
            let rr = ui.interact(*r, id.with(("a", i)), Sense::click());
            if rr.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if rr.clicked() {
                clicked = Some(i);
            }
        }
        let mr = ui.interact(more_r, id.with("more"), Sense::click());
        if mr.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let idle = idle_for(ui, id, ui.rect_contains_pointer(outer));
        let auto = self.autoplay && idle > 2.0;
        let st = glowkit_core::with_state(
            ui,
            id,
            || st0,
            |s| {
                if mr.clicked()
                    || (clicked.is_some_and(|c| c >= np))
                    || (auto && now - s.since > if s.open { 2.6 } else { 1.6 })
                {
                    s.open = !s.open;
                    s.since = now;
                }
                *s
            },
        );
        if auto || (now - st.since) < 1.0 {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(outer) {
            return clicked;
        }
        card(&painter, &t, bar, h / 2.0, 0.5);
        let ptr = ui.input(|i| i.pointer.hover_pos());
        let clip = painter.with_clip_rect(bar.shrink(1.0));
        for (i, (ic, label)) in self.primary.iter().chain(self.more.iter()).enumerate() {
            let r = rects[i];
            let full = widths[i];
            let k = (r.width() / full).clamp(0.0, 1.0);
            if k < 0.02 {
                continue;
            }
            let hov = ptr.is_some_and(|p| r.contains(p));
            let rc = clip.with_clip_rect(r.intersect(bar.shrink(1.0)));
            rc.rect_filled(r, cr(btn_h / 2.0), if hov { hover_fill(&t) } else { well(&t) });
            let a = ((k - 0.3) / 0.7).clamp(0.0, 1.0);
            let off = (1.0 - k) * 14.0;
            let danger = *ic == BeIcon::Trash;
            let col = if danger { t.destructive } else { t.foreground };
            icon(&rc, *ic, pos2(r.min.x + 20.0 + off, r.center().y), 14.0, with_alpha(col, a), 1.3);
            rc.text(
                pos2(r.min.x + 35.0 + off, r.center().y),
                Align2::LEFT_CENTER,
                label,
                FontId::proportional(13.5),
                with_alpha(col, a),
            );
        }
        // more button: three dots fold into a cross
        let k = spring(ui, id.with("x"), if st.open { 1.0 } else { 0.0 }, 300.0, 18.0);
        let hov = mr.hovered();
        painter.circle_filled(
            more_c,
            btn_h / 2.0 - if mr.is_pointer_button_down_on() { 1.5 } else { 0.0 },
            if hov { mix(t.info, egui::Color32::WHITE, 0.1) } else { t.info },
        );
        let fg = egui::Color32::WHITE;
        let kk = k.clamp(0.0, 1.0);
        // the outer dots travel to the cross' ends, the middle one shrinks away
        let ang = kk * std::f32::consts::FRAC_PI_4;
        for sgn in [-1.0f32, 1.0] {
            let d = egui::Vec2::angled(ang * sgn) * 6.0 * sgn;
            let p = more_c + d;
            if kk < 0.05 {
                painter.circle_filled(p, 1.9, fg);
            } else {
                let len = 6.0 * kk;
                let a = more_c + egui::Vec2::angled(std::f32::consts::FRAC_PI_4 * sgn) * len;
                let b = more_c - egui::Vec2::angled(std::f32::consts::FRAC_PI_4 * sgn) * len;
                painter.line_segment([a, b], Stroke::new(1.6, with_alpha(fg, kk)));
                painter.circle_filled(p, 1.9 * (1.0 - kk), fg);
            }
        }
        painter.circle_filled(more_c, 1.9 * (1.0 - kk), fg);
        clicked
    }
}
