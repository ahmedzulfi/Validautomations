use super::common::{card, cr, ghost_cursor, hairline, hover_fill, icon, idle_for, surface, well, BeIcon};
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Sense, Ui};
use glowkit_core::anim::Spring;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Debug)]
struct SwipeState {
    off: Vec<Spring>,
    height: Vec<Spring>,
    removed: Vec<bool>,
    drag: Option<(usize, f32, f32)>,
    status: String,
}

/// A list of rows you swipe to act on, like a mail inbox. Drag a row left and it slides over a
/// pair of actions (snooze and delete) that grow into view; let go past the first stop and it
/// rests open, fling it all the way and it flies off while the rows below spring up to close
/// the gap. "Reset" brings everything back.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BeIcon, SwipeableList};
/// ui.add(
///     SwipeableList::new("Priority queue")
///         .row(BeIcon::Doc, "Launch brief", "Finalize the copy", "9:41")
///         .row(BeIcon::Mail, "Client feedback", "Three comments", "11:08"),
/// );
/// ```
pub struct SwipeableList {
    title: String,
    rows: Vec<(BeIcon, String, String, String)>,
    width: f32,
    autoplay: bool,
}

impl SwipeableList {
    /// An empty list titled `title`.
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), rows: Vec::new(), width: 380.0, autoplay: true }
    }
    /// Add a row: icon, title, subtitle and a time stamp.
    pub fn row(mut self, icon: BeIcon, title: impl Into<String>, subtitle: impl Into<String>, time: impl Into<String>) -> Self {
        self.rows.push((icon, title.into(), subtitle.into(), time.into()));
        self
    }
    /// Width of the card.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(260.0);
        self
    }
    /// Swipe rows on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const ROW_H: f32 = 70.0;
const GAP: f32 = 8.0;
const OPEN: f32 = -116.0;

impl egui::Widget for SwipeableList {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let n = self.rows.len();
        let head = 58.0;
        let foot = 34.0;
        let pad = 12.0;
        let full_h = head + n as f32 * (ROW_H + GAP) + foot;
        let (rect, resp) = ui.allocate_exact_size(vec2(self.width, full_h), Sense::hover());
        let id = resp.id;
        let dt = glowkit_core::dt(ui);
        let fresh = || SwipeState {
            off: vec![Spring::new(0.0).with(320.0, 26.0); n],
            height: vec![Spring::new(1.0).with(260.0, 24.0); n],
            removed: vec![false; n],
            drag: None,
            status: "Ready".into(),
        };
        let mut st: SwipeState = glowkit_core::state(ui, id, fresh);
        if st.off.len() != n {
            st = fresh();
        }
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let hovering = pointer.is_some_and(|p| rect.contains(p));
        let idle = idle_for(ui, id, hovering || st.drag.is_some());
        let auto = self.autoplay && idle > 2.5;
        let (mut ptr, mut down, mut pressed) = (pointer, ui.input(|i| i.pointer.primary_down()), ui.input(|i| i.pointer.primary_pressed()));
        let click = ui.input(|i| i.pointer.primary_clicked());
        // row tops from animated heights
        let mut tops = Vec::with_capacity(n);
        let mut y = rect.min.y + head;
        for i in 0..n {
            tops.push(y);
            y += (ROW_H + GAP) * st.height[i].value.max(0.0);
        }
        let row_rect = |i: usize| Rect::from_min_size(pos2(rect.min.x + pad, tops[i]), vec2(self.width - pad * 2.0, ROW_H));
        let reset_r = Rect::from_min_size(pos2(rect.max.x - pad - 78.0, rect.min.y + 14.0), vec2(78.0, 30.0));
        // ---- autoplay: a ghost finger swipes
        let mut ghost: Option<(Pos2, f32, bool)> = None;
        if auto {
            glowkit_core::animate(ui);
            let cyc = 3.4;
            let ph = (idle - 2.5) % cyc;
            let k = ((idle - 2.5) / cyc) as usize;
            let alive: Vec<usize> = (0..n).filter(|i| !st.removed[*i]).collect();
            if alive.len() <= 2 && ph < 0.05 {
                st = fresh();
            } else if let Some(&row) = alive.first() {
                let fling = k % 2 == 1;
                let r = row_rect(row);
                let a = pos2(r.max.x - 70.0, r.center().y);
                let b = a + vec2(if fling { -230.0 } else { -130.0 }, 0.0);
                let rest = pos2(r.max.x + 20.0, r.max.y + 40.0);
                let e = |x: f32| glowkit_core::ease(glowkit_core::Easing::InOutCubic, x.clamp(0.0, 1.0));
                let (p, held) = if ph < 0.7 {
                    (rest.lerp(a, e(ph / 0.7)), false)
                } else if ph < 0.85 {
                    (a, true)
                } else if ph < 1.5 {
                    (a.lerp(b, e((ph - 0.85) / (if fling { 0.4 } else { 0.65 }))), true)
                } else if ph < 2.2 {
                    (b, false)
                } else {
                    (b.lerp(rest, e((ph - 2.2) / 1.0)), false)
                };
                if !fling && (2.3..2.35).contains(&ph) {
                    // close the opened row again
                    st.off[row].target = 0.0;
                }
                ptr = Some(p);
                pressed = held && ph < 0.86 && st.drag.is_none();
                down = held;
                ghost = Some((p, (ph / 0.3).min(1.0) * ((cyc - ph) / 0.4).min(1.0), held));
            }
        }
        // ---- interaction
        if let Some(p) = ptr {
            if pressed && st.drag.is_none() {
                if let Some(i) = (0..n).find(|&i| !st.removed[i] && row_rect(i).contains(p)) {
                    // a press on an open row's action buttons is handled as a click below
                    let on_actions = st.off[i].value < -40.0 && p.x > row_rect(i).max.x + st.off[i].value;
                    if !on_actions {
                        st.drag = Some((i, p.x, st.off[i].value));
                        for (j, o) in st.off.iter_mut().enumerate() {
                            if j != i {
                                o.target = 0.0;
                            }
                        }
                    }
                }
            }
        }
        if let Some((i, x0, o0)) = st.drag {
            if let (Some(p), true) = (ptr, down) {
                let raw = o0 + p.x - x0;
                // rubber band past the open stop and to the right
                let v = if raw > 0.0 { raw * 0.25 } else { raw };
                st.off[i].value = v.max(-self.width);
                st.off[i].target = st.off[i].value;
                st.off[i].velocity = 0.0;
            } else {
                let v = st.off[i].value;
                if v < -190.0 {
                    st.off[i].target = -self.width - 40.0;
                    st.removed[i] = true;
                    st.height[i].target = 0.0;
                    st.status = format!("Archived “{}”", self.rows[i].1);
                } else if v < OPEN * 0.5 {
                    st.off[i].target = OPEN;
                } else {
                    st.off[i].target = 0.0;
                }
                st.drag = None;
            }
        }
        if click && !auto {
            if let Some(p) = pointer {
                if reset_r.contains(p) {
                    st = fresh();
                } else {
                    for i in 0..n {
                        let r = row_rect(i);
                        if st.removed[i] || st.off[i].value > -40.0 || !r.contains(p) {
                            continue;
                        }
                        let x_del = r.max.x - 29.0;
                        let x_snooze = r.max.x - 87.0;
                        if (p.x - x_del).abs() < 26.0 {
                            st.removed[i] = true;
                            st.off[i].target = -self.width - 40.0;
                            st.height[i].target = 0.0;
                            st.status = format!("Deleted “{}”", self.rows[i].1);
                        } else if (p.x - x_snooze).abs() < 26.0 {
                            st.off[i].target = 0.0;
                            st.status = format!("Snoozed “{}”", self.rows[i].1);
                        }
                    }
                }
            }
        }
        let mut busy = st.drag.is_some();
        for k in 0..n {
            if st.drag.map(|d| d.0) != Some(k) {
                st.off[k].step(dt);
            }
            if st.removed[k] && st.off[k].value < -self.width * 0.6 {
                st.height[k].target = 0.0;
            }
            st.height[k].step(dt);
            busy |= !st.off[k].is_settled() || !st.height[k].is_settled();
        }
        glowkit_core::set_state(ui, id, st.clone());
        if busy {
            glowkit_core::animate(ui);
        }
        if hovering {
            if pointer.is_some_and(|p| reset_r.contains(p)) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            } else if st.drag.is_some() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter().clone();
        let alive = st.removed.iter().filter(|r| !**r).count();
        let card_h = head + (0..n).map(|i| (ROW_H + GAP) * st.height[i].value.max(0.0)).sum::<f32>() + foot;
        let body = Rect::from_min_size(rect.min, vec2(self.width, card_h.max(head + foot)));
        card(&painter, &t, body, 26.0, 0.6);
        painter.text(pos2(body.min.x + 18.0, body.min.y + 24.0), Align2::LEFT_CENTER, &self.title, FontId::proportional(14.5), t.foreground);
        painter.text(pos2(body.min.x + 18.0, body.min.y + 42.0), Align2::LEFT_CENTER, &st.status, FontId::proportional(11.5), t.muted_foreground);
        let hov_reset = pointer.is_some_and(|p| reset_r.contains(p));
        painter.rect(reset_r, cr(15.0), if hov_reset { hover_fill(&t) } else { well(&t) }, hairline(&t), egui::StrokeKind::Inside);
        icon(&painter, BeIcon::Refresh, pos2(reset_r.min.x + 18.0, reset_r.center().y), 11.0, t.muted_foreground, 1.3);
        painter.text(pos2(reset_r.min.x + 30.0, reset_r.center().y), Align2::LEFT_CENTER, "Reset", FontId::proportional(12.5), t.muted_foreground);
        let clip = painter.with_clip_rect(body.shrink(1.0));
        for (i, (ic, title, sub, time)) in self.rows.iter().enumerate() {
            let hk = st.height[i].value;
            if hk < 0.02 {
                continue;
            }
            let r = row_rect(i);
            let off = st.off[i].value;
            let rc = clip.with_clip_rect(Rect::from_min_max(r.min, pos2(r.max.x, r.min.y + ROW_H * hk.min(1.0))).intersect(body.shrink(1.0)));
            // actions behind the row grow into view
            let reveal = (-off / -OPEN).clamp(0.0, 1.6);
            if reveal > 0.01 {
                let amber = t.warning;
                let red = t.destructive;
                let del_w = if reveal > 1.0 { 52.0 + (reveal - 1.0) * 160.0 } else { 52.0 };
                for (k, (col, ic2, cx)) in [(amber, BeIcon::Clock, r.max.x - 87.0), (red, BeIcon::Trash, r.max.x - 29.0 - (del_w - 52.0) / 2.0)].iter().enumerate() {
                    let s = (reveal * 1.2 - k as f32 * 0.15).clamp(0.0, 1.0);
                    let w = if k == 1 { del_w } else { 52.0 };
                    let br = Rect::from_center_size(pos2(*cx, r.center().y), vec2(w * s.max(0.01), (ROW_H - 12.0) * s));
                    rc.rect_filled(br, cr(14.0), with_alpha(*col, 0.16 * s));
                    icon(&rc, *ic2, br.center(), 15.0 * s, with_alpha(*col, s), 1.5);
                }
            }
            let row = r.translate(vec2(off, 0.0));
            let fill = if t.dark { mix(surface(&t), Color32::WHITE, 0.03) } else { mix(surface(&t), t.muted, 0.35) };
            rc.rect(row, cr(18.0), fill, hairline(&t), egui::StrokeKind::Inside);
            let tile = Rect::from_center_size(pos2(row.min.x + 34.0, row.center().y), vec2(38.0, 38.0));
            rc.rect_filled(tile, cr(11.0), well(&t));
            icon(&rc, *ic, tile.center(), 15.0, t.muted_foreground, 1.3);
            rc.text(pos2(row.min.x + 66.0, row.center().y - 9.0), Align2::LEFT_CENTER, title, FontId::proportional(14.0), t.foreground);
            rc.text(pos2(row.min.x + 66.0, row.center().y + 10.0), Align2::LEFT_CENTER, sub, FontId::proportional(12.0), t.muted_foreground);
            rc.text(pos2(row.max.x - 16.0, row.center().y), Align2::RIGHT_CENTER, time, FontId::proportional(12.5), t.muted_foreground);
        }
        let fy = body.max.y - foot / 2.0 - 2.0;
        painter.text(pos2(body.min.x + 18.0, fy), Align2::LEFT_CENTER, format!("{alive} open"), FontId::proportional(11.5), t.muted_foreground);
        painter.text(pos2(body.max.x - 18.0, fy), Align2::RIGHT_CENTER, "Today", FontId::proportional(11.5), t.muted_foreground);
        if let Some((p, a, held)) = ghost {
            if held {
                painter.circle_filled(p, 14.0, with_alpha(t.foreground, 0.12 * a));
            }
            ghost_cursor(&painter, &t, p, a, held);
        }
        resp
    }
}
