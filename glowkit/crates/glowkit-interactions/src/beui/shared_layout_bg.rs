use super::common::{cr, ghost_cursor, hover_fill, icon, idle_for, morph, text_width, BeIcon};
use egui::{pos2, vec2, Align2, FontId, Pos2, Rect, Sense, Ui};
use glowkit_core::anim::spring;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// A list (or a row of nav links) sharing one hover background. Instead of each item lighting up
/// on its own, a single soft pill slides and resizes from item to item under the pointer, the
/// way a shared-layout animation moves one element between places, and fades away when the
/// pointer leaves. The hovered item's arrow nudges up and to the right.
///
/// [`SharedLayoutBg::show`] returns the item clicked this frame.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::SharedLayoutBg;
/// let clicked = SharedLayoutBg::new()
///     .item("Inbox", "12 unread threads")
///     .item("Drafts", "4 posts waiting")
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct SharedLayoutBg {
    items: Vec<(String, String)>,
    horizontal: bool,
    width: f32,
    stiffness: f32,
    autoplay: bool,
}

impl Default for SharedLayoutBg {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedLayoutBg {
    /// An empty list.
    pub fn new() -> Self {
        Self { items: Vec::new(), horizontal: false, width: 440.0, stiffness: 380.0, autoplay: true }
    }
    /// Add an item with a title and a (possibly empty) description.
    pub fn item(mut self, title: impl Into<String>, subtitle: impl Into<String>) -> Self {
        self.items.push((title.into(), subtitle.into()));
        self
    }
    /// Lay the items out in a row like nav links (descriptions are not shown).
    pub fn horizontal(mut self, on: bool) -> Self {
        self.horizontal = on;
        self
    }
    /// Width of the vertical list.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(160.0);
        self
    }
    /// Spring stiffness of the gliding background (higher = snappier).
    pub fn stiffness(mut self, k: f32) -> Self {
        self.stiffness = k.clamp(60.0, 1200.0);
        self
    }
    /// Glide over the items with a ghost cursor while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the list. Returns the item clicked this frame.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = Theme::of(ui);
        let painter = ui.painter().clone();
        let n = self.items.len();
        let base_id = ui.next_auto_id();
        let rects: Vec<Rect>;
        let size;
        if self.horizontal {
            let ws: Vec<f32> = self.items.iter().map(|it| text_width(&painter, &it.0, 14.0) + 30.0).collect();
            size = vec2(ws.iter().sum::<f32>() + 8.0, 46.0);
            let (outer, _) = ui.allocate_exact_size(size, Sense::hover());
            let mut x = outer.min.x + 4.0;
            rects = ws
                .iter()
                .map(|w| {
                    let r = Rect::from_min_size(pos2(x, outer.min.y + 5.0), vec2(*w, 36.0));
                    x += w;
                    r
                })
                .collect();
        } else {
            let row_h = 60.0;
            size = vec2(self.width, row_h * n as f32);
            let (outer, _) = ui.allocate_exact_size(size, Sense::hover());
            rects = (0..n)
                .map(|i| {
                    Rect::from_min_size(
                        pos2(outer.min.x, outer.min.y + i as f32 * row_h),
                        vec2(self.width, row_h - 4.0),
                    )
                })
                .collect();
        }
        if n == 0 {
            return None;
        }
        let outer = rects.iter().fold(rects[0], |a, b| a.union(*b));
        let id = base_id.with("beui_shared_bg");
        let mut clicked = None;
        let mut hov = None;
        for (i, r) in rects.iter().enumerate() {
            let resp = ui.interact(*r, id.with(i), Sense::click());
            if resp.hovered() {
                hov = Some(i);
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
                clicked = Some(i);
            }
        }
        let idle = idle_for(ui, id, hov.is_some());
        let mut ghost: Option<(Pos2, f32)> = None;
        if self.autoplay && idle > 2.0 && hov.is_none() {
            glowkit_core::animate(ui);
            let time = idle - 2.0;
            // dwell on each item, glide between them, step out now and then
            let per = 0.9;
            let cyc = n as f32 + 1.0;
            let ph = time / per % cyc;
            let i = ph.floor() as usize;
            let f = ph.fract();
            let e = glowkit_core::ease(glowkit_core::Easing::InOutCubic, ((f - 0.55) / 0.45).clamp(0.0, 1.0));
            let anchor = |j: usize| -> Pos2 {
                if j >= n {
                    if self.horizontal {
                        pos2(outer.max.x + 30.0, outer.max.y + 20.0)
                    } else {
                        pos2(outer.max.x - 40.0, outer.max.y + 26.0)
                    }
                } else if self.horizontal {
                    rects[j].center() + vec2(4.0, 4.0)
                } else {
                    pos2(rects[j].center().x + 50.0, rects[j].center().y + 4.0)
                }
            };
            let p = anchor(i).lerp(anchor((i + 1) % (n + 1)), e);
            if let Some(j) = rects.iter().position(|r| r.contains(p)) {
                hov = Some(j);
            }
            let a = (time / 0.4).min(1.0);
            ghost = Some((p, a));
        }
        // the shared background: one rect that springs between items
        let target = hov.map(|i| rects[i]);
        let last: Option<Rect> = glowkit_core::with_state(
            ui,
            id.with("last"),
            || None,
            |l| {
                if target.is_some() {
                    *l = target;
                }
                *l
            },
        );
        let to = target.or(last).unwrap_or(rects[0]);
        let (bg, r) = morph(
            ui,
            id.with("bg"),
            to,
            if self.horizontal { 18.0 } else { 14.0 },
            self.stiffness,
            2.0 * self.stiffness.sqrt() * 0.85,
        );
        let a = spring(ui, id.with("bga"), if hov.is_some() { 1.0 } else { 0.0 }, 260.0, 26.0);
        if !ui.is_rect_visible(outer) {
            return clicked;
        }
        if a > 0.01 {
            let bgr = bg.expand((1.0 - a) * -4.0);
            if t.dark {
                painter.rect_filled(bgr, cr(r), with_alpha(hover_fill(&t), a));
            } else {
                glowkit_core::paint::soft_shadow(
                    &painter,
                    bgr,
                    r,
                    10.0,
                    vec2(0.0, 2.0),
                    super::common::shadow_color(&t, 0.35 * a),
                );
                painter.rect(
                    bgr,
                    cr(r),
                    with_alpha(super::common::surface(&t), a),
                    egui::Stroke::new(1.0, with_alpha(t.border, 0.8 * a)),
                    egui::StrokeKind::Inside,
                );
            }
        }
        for (i, (title, sub)) in self.items.iter().enumerate() {
            let rr = rects[i];
            let h = spring(ui, id.with(("h", i)), if hov == Some(i) { 1.0 } else { 0.0 }, 300.0, 22.0);
            if self.horizontal {
                let col = mix(t.muted_foreground, t.foreground, h.clamp(0.0, 1.0));
                painter.text(rr.center(), Align2::CENTER_CENTER, title, FontId::proportional(14.0), col);
            } else {
                let x = rr.min.x + 16.0;
                painter.text(
                    pos2(x, rr.center().y - 9.0),
                    Align2::LEFT_CENTER,
                    title,
                    FontId::proportional(14.5),
                    t.foreground,
                );
                painter.text(
                    pos2(x, rr.center().y + 10.0),
                    Align2::LEFT_CENTER,
                    sub,
                    FontId::proportional(12.5),
                    t.muted_foreground,
                );
                let nudge = vec2(1.0, -1.0) * 3.0 * h;
                let col = mix(t.muted_foreground, t.foreground, h.clamp(0.0, 1.0));
                icon(&painter, BeIcon::ArrowUpRight, pos2(rr.max.x - 20.0, rr.center().y - 9.0) + nudge, 9.0, col, 1.4);
            }
        }
        if let Some((p, a)) = ghost {
            ghost_cursor(&painter, &t, p, a, false);
        }
        clicked
    }
}
