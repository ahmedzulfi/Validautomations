//! A list whose deletions wait in a floating undo bar before they are committed.

use super::common::*;
use super::reorder::TouchRow;
use egui::{pos2, vec2, Align2, Color32, Rect, Response, Sense, Ui, Vec2};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Default)]
struct RowFold {
    key: u64,
    h: f32,
    hv: f32,
    glow: f32,
}

#[derive(Clone, Default)]
struct UndoState {
    rows: Vec<RowFold>,
    pending: Option<(u64, String)>,
    left: f32,
    bar: f32,
    bar_v: f32,
    drag: f32,
    swap: f32,
    swap_v: f32,
    commit: Option<u64>,
    title: String,
    g: Gesture,
    grabbed: bool,
}

/// A list with delete buttons whose deletions can be taken back from a floating bar.
///
/// Tap a row's trash can and the row folds away on a spring while a dark pill rises from the
/// bottom: "“Groceries” deleted" with an **Undo** button wrapped in a countdown ring. Undo
/// within the timeout and the row unfolds back into place with a soft highlight; let the ring
/// run out, swipe the bar down, or delete something else, and the deletion is committed (removed
/// from your `Vec`). The countdown pauses while the pointer rests on the bar. A second delete
/// while the bar is up commits the first and the bar gives a little bump as its text changes.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{UndoBar, TouchRow};
/// let mut notes = vec![TouchRow::new("Groceries", "Milk, eggs, bread")];
/// ui.add(UndoBar::new(&mut notes).timeout(5.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct UndoBar<'a> {
    rows: &'a mut Vec<TouchRow>,
    size: Option<Vec2>,
    timeout: f32,
    bounce: f32,
    autoplay: bool,
}

impl<'a> UndoBar<'a> {
    /// An undoable list over `rows`.
    pub fn new(rows: &'a mut Vec<TouchRow>) -> Self {
        Self { rows, size: None, timeout: 5.0, bounce: 35.0, autoplay: false }
    }
    /// Widget size (default: all available space; the bar floats at the bottom).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Seconds before a deletion is committed.
    pub fn timeout(mut self, s: f32) -> Self {
        self.timeout = s.clamp(1.0, 30.0);
        self
    }
    /// Overshoot of the bar and the restored row, 0..100.
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 100.0);
        self
    }
    /// Delete and undo on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const ROW_H: f32 = 58.0;
const SCRIPT: [Stroke2; 5] = [
    tap(0.8, 0.87, 0.19),
    tap(2.5, 0.8, 0.917),
    tap(4.0, 0.87, 0.3),
    tap(5.1, 0.87, 0.3),
    drag(6.6, 6.9, (0.45, 0.917), (0.45, 1.02)),
];

impl egui::Widget for UndoBar<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = self.size.unwrap_or(ui.available_size());
        let (rect, mut response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let touch = touch(ui, &response, rect, self.autoplay, &SCRIPT, 9.0);
        let mut st: UndoState = glowkit_core::state(ui, id.with("undo"), UndoState::default);
        let keys: Vec<u64> = self.rows.iter().map(|r| r.key()).collect();
        let old = std::mem::take(&mut st.rows);
        st.rows = keys
            .iter()
            .map(|k| {
                old.iter().find(|a| a.key == *k).copied().unwrap_or(RowFold { key: *k, h: 1.0, ..Default::default() })
            })
            .collect();
        // commit a finished deletion
        if let Some(k) = st.commit.take() {
            if let Some(i) = keys.iter().position(|x| *x == k) {
                self.rows.remove(i);
                st.rows.remove(i);
                response.mark_changed();
            }
        }
        let n = self.rows.len();
        let top = rect.min.y + 70.0;
        let list_x = rect.min.x + 12.0;
        let list_w = rect.width() - 24.0;
        let row_y =
            |rows: &[RowFold], i: usize| top + rows[..i].iter().map(|r| r.h.clamp(0.0, 1.0) * ROW_H).sum::<f32>();
        let bar = Rect::from_center_size(pos2(rect.center().x, rect.max.y - 44.0), vec2(rect.width() - 28.0, 52.0));
        let undo_btn = Rect::from_min_max(pos2(bar.max.x - 96.0, bar.min.y), bar.max);
        let pending_key = st.pending.as_ref().map(|p| p.0);

        let g = st.g.update(&touch, now, dt);
        let bar_live = st.pending.is_some() && st.bar > 0.5;
        if g.pressed {
            st.grabbed = bar_live && bar.translate(vec2(0.0, st.drag)).contains(st.g.start);
        }
        if st.grabbed && st.g.down {
            let d = st.g.last.y - st.g.start.y;
            st.drag = if d > 0.0 { d } else { rubber(d, 16.0) };
        }
        if g.released {
            if st.grabbed && (st.drag > 30.0 || st.g.vel.y > 500.0) {
                st.commit = pending_key;
                st.pending = None;
                st.bar_v = -st.g.vel.y.max(0.0) / 120.0;
            } else if g.tap && bar_live && undo_btn.contains(st.g.last) {
                if let Some((k, _)) = st.pending.take() {
                    if let Some(r) = st.rows.iter_mut().find(|r| r.key == k) {
                        r.hv = 2.0;
                        r.glow = 1.0;
                    }
                }
            } else if g.tap && !st.grabbed {
                // trash buttons
                let p = st.g.last;
                if p.x > list_x + list_w - 46.0 {
                    if let Some(i) = (0..n).find(|&i| {
                        let y = row_y(&st.rows, i);
                        Some(st.rows[i].key) != pending_key
                            && p.y >= y
                            && p.y < y + ROW_H * st.rows[i].h.clamp(0.0, 1.0)
                    }) {
                        if let Some((k, _)) = st.pending.take() {
                            st.commit = Some(k);
                            st.swap_v = 7.0;
                        }
                        st.pending = Some((st.rows[i].key, self.rows[i].title.clone()));
                        st.title = self.rows[i].title.clone();
                        st.left = self.timeout;
                    }
                }
            }
            st.grabbed = false;
        }
        let hover_bar = !touch.ghost && touch.pos.is_some_and(|p| bar.contains(p));
        if st.pending.is_some() && !st.grabbed && !hover_bar {
            st.left -= dt;
            if st.left <= 0.0 {
                st.commit = st.pending.take().map(|p| p.0);
            }
        }
        // physics
        let pending_key = st.pending.as_ref().map(|p| p.0);
        let committing = st.commit;
        let mut moving = st.g.down;
        for r in st.rows.iter_mut() {
            let target = if Some(r.key) == pending_key || Some(r.key) == committing { 0.0 } else { 1.0 };
            spring_step(&mut r.h, &mut r.hv, target, 260.0, damping_for(self.bounce, 260.0), dt);
            r.glow = (r.glow - dt * 1.2).max(0.0);
            moving |= !settled(r.h * 50.0, r.hv * 50.0, target * 50.0) || r.glow > 0.0;
        }
        let bt = if st.pending.is_some() { 1.0 } else { 0.0 };
        spring_step(&mut st.bar, &mut st.bar_v, bt, 240.0, damping_for(self.bounce, 240.0), dt);
        spring_step(&mut st.swap, &mut st.swap_v, 0.0, 500.0, 16.0, dt);
        if !st.grabbed {
            st.drag *= (1.0 - dt * 10.0).max(0.0);
        }
        moving |=
            st.pending.is_some() || !settled(st.bar, st.bar_v, bt) || st.swap_v.abs() > 0.01 || st.commit.is_some();
        let s = st.clone();
        glowkit_core::set_state(ui, id.with("undo"), st);
        if moving || touch.ghost {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }

        // paint
        let painter = ui.painter_at(rect);
        text_bold(
            &painter,
            pos2(rect.min.x + 18.0, rect.min.y + 46.0),
            Align2::LEFT_CENTER,
            "Notes",
            24.0,
            t.foreground,
        );
        let total: f32 = s.rows.iter().map(|r| r.h.clamp(0.0, 1.0) * ROW_H).sum();
        let list = Rect::from_min_size(pos2(list_x, top), vec2(list_w, total.max(0.0)));
        if total > 1.0 {
            card(&painter, &t, list, 14.0, 0.4, surface(&t));
        }
        let clip = painter.with_clip_rect(list);
        let del = if t.dark { hexa(0xFF, 0x45, 0x3A) } else { hexa(0xE5, 0x30, 0x2A) };
        for i in 0..n {
            let r = s.rows[i];
            let h = r.h.clamp(0.0, 1.0) * ROW_H;
            if h < 0.5 {
                continue;
            }
            let y = row_y(&s.rows, i);
            let row = Rect::from_min_size(pos2(list_x, y), vec2(list_w, h));
            let rc = clip.with_clip_rect(row.intersect(list));
            if r.glow > 0.0 {
                rc.rect_filled(row, cr(0.0), with_alpha(system_blue(&t), 0.12 * r.glow));
            }
            let a = r.h.clamp(0.0, 1.0);
            let cy = y + h / 2.0;
            let tr = &self.rows[i];
            text_bold(
                &rc,
                pos2(row.min.x + 16.0, cy - 9.0),
                Align2::LEFT_CENTER,
                &tr.title,
                14.0,
                with_alpha(t.foreground, a),
            );
            text(
                &rc,
                pos2(row.min.x + 16.0, cy + 10.0),
                Align2::LEFT_CENTER,
                &tr.detail,
                12.0,
                with_alpha(t.muted_foreground, a),
            );
            glyph(
                &rc,
                Glyph::Trash,
                pos2(row.max.x - 24.0, cy),
                15.0,
                with_alpha(mix(t.muted_foreground, del, 0.35), a),
            );
            if i + 1 < n {
                rc.line_segment(
                    [pos2(row.min.x + 16.0, row.max.y - 0.5), pos2(row.max.x, row.max.y - 0.5)],
                    egui::Stroke::new(1.0, hairline(&t)),
                );
            }
        }
        if s.bar > 0.01 {
            let off = (1.0 - s.bar) * 110.0 + s.drag;
            let b = Rect::from_center_size(bar.center() + vec2(0.0, off), bar.size() * (1.0 + s.swap * 0.02));
            let fill = if t.dark { hexa(0x3A, 0x3A, 0x40) } else { hexa(0x1C, 0x1C, 0x1F) };
            shadow(&painter, &t, b, 26.0, 1.2);
            painter.rect_filled(b, cr(26.0), fill);
            let label = format!("“{}” deleted", s.title);
            text(
                &painter,
                pos2(b.min.x + 20.0, b.center().y - s.swap * 2.0),
                Align2::LEFT_CENTER,
                label,
                13.0,
                Color32::WHITE,
            );
            let uc = pos2(b.max.x - 56.0, b.center().y);
            text_bold(&painter, uc + vec2(-6.0, 0.0), Align2::CENTER_CENTER, "Undo", 13.5, hexa(0x64, 0xB5, 0xFF));
            let frac = (s.left / self.timeout).clamp(0.0, 1.0);
            let rc = pos2(b.max.x - 20.0, b.center().y);
            paint::arc(&painter, rc, 9.0, 0.0, TAU, 2.2, with_alpha(Color32::WHITE, 0.18));
            paint::arc(&painter, rc, 9.0, -PI / 2.0, -PI / 2.0 + TAU * frac, 2.2, Color32::WHITE);
            text(
                &painter,
                rc,
                Align2::CENTER_CENTER,
                format!("{}", s.left.ceil().max(0.0) as i32),
                9.0,
                Color32::WHITE,
            );
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}
