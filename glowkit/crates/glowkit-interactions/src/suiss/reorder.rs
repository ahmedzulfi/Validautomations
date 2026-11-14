//! Rows you drag by a handle to reorder, with neighbours springing out of the way.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Rect, Response, Sense, Ui};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// A list row: a title and an optional detail line. Used by the suiss list components.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TouchRow {
    /// Main line.
    pub title: String,
    /// Optional second, dimmer line (empty for none).
    pub detail: String,
}

impl TouchRow {
    /// A row with a title and a detail line (pass `""` for none).
    pub fn new(title: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { title: title.into(), detail: detail.into() }
    }
    pub(crate) fn key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.title.hash(&mut h);
        h.finish()
    }
}

/// A few sample rows used by the demos.
pub(crate) fn sample_lists() -> Vec<TouchRow> {
    vec![
        TouchRow::new("Inbox", "12 messages"),
        TouchRow::new("Today", ""),
        TouchRow::new("Upcoming", "Next 7 days"),
        TouchRow::new("Someday", ""),
        TouchRow::new("Archive", "Everything finished, kept for later"),
    ]
}

#[derive(Clone, Copy, Default)]
struct RowAnim {
    key: u64,
    y: f32,
    v: f32,
    bump: f32,
    bump_v: f32,
}

#[derive(Clone, Default)]
struct ReorderState {
    rows: Vec<RowAnim>,
    drag: Option<(usize, f32)>,
    target: usize,
    lift: f32,
    lift_v: f32,
    lifted_key: u64,
    g: Gesture,
}

/// A grouped list whose rows are dragged by a grip handle to change their order.
///
/// Press a row's handle and the row lifts off the list: it grows a touch, a soft shadow fades in
/// beneath it and it follows the finger exactly, rubber-banding when pulled past the ends. The
/// other rows spring out of its way as it crosses their midpoints (with a tiny "tick" bump, the
/// visual version of a haptic), and on release it settles into its new slot carrying the
/// finger's velocity. Rows can have different heights. The order is written back into the
/// `Vec` you pass in.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{ReorderList, TouchRow};
/// let mut rows = vec![TouchRow::new("Inbox", "12 messages"), TouchRow::new("Today", "")];
/// ui.add(ReorderList::new(&mut rows).bounce(40.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ReorderList<'a> {
    rows: &'a mut Vec<TouchRow>,
    width: Option<f32>,
    bounce: f32,
    lift: f32,
    footer: bool,
    autoplay: bool,
}

impl<'a> ReorderList<'a> {
    /// A reorderable list over `rows`.
    pub fn new(rows: &'a mut Vec<TouchRow>) -> Self {
        Self { rows, width: None, bounce: 40.0, lift: 50.0, footer: true, autoplay: false }
    }
    /// Width in points (default: available width).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Overshoot when rows settle, 0..100.
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 100.0);
        self
    }
    /// How much the dragged row grows and how deep its shadow is, 0..100.
    pub fn lift(mut self, l: f32) -> Self {
        self.lift = l.clamp(0.0, 100.0);
        self
    }
    /// Show the current order as a caption under the list.
    pub fn footer(mut self, on: bool) -> Self {
        self.footer = on;
        self
    }
    /// Drag rows around on their own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn row_h(r: &TouchRow) -> f32 {
    if r.detail.is_empty() {
        46.0
    } else {
        58.0
    }
}

impl egui::Widget for ReorderList<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = self.width.unwrap_or_else(|| ui.available_width());
        let total: f32 = self.rows.iter().map(row_h).sum();
        let footer_h = if self.footer { 34.0 } else { 0.0 };
        let (rect, mut response) = ui.allocate_exact_size(vec2(w, total + footer_h + 8.0), Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let n = self.rows.len();
        let list = Rect::from_min_size(rect.min + vec2(0.0, 4.0), vec2(w, total));
        // script positions are expressed for the rows' current layout
        let mut tops = Vec::with_capacity(n);
        let mut y = 0.0;
        for r in self.rows.iter() {
            tops.push(y);
            y += row_h(r);
        }
        let ny = |i: usize| ((list.min.y + tops.get(i).copied().unwrap_or(0.0) + 26.0) - rect.min.y) / rect.height();
        let hx = (w - 24.0) / w;
        let script = if n >= 4 {
            vec![
                hold_drag(0.8, 0.25, 2.0, (hx, ny(0)), (hx, ny(3) + 0.06)),
                hold_drag(3.4, 0.25, 4.6, (hx, ny(n - 1)), (hx, ny(1) - 0.02)),
            ]
        } else {
            Vec::new()
        };
        let touch = touch(ui, &response, rect, self.autoplay, &script, 6.4);
        let mut st: ReorderState = glowkit_core::state(ui, id.with("reorder"), ReorderState::default);

        // sync row animations with the data
        let keys: Vec<u64> = self.rows.iter().map(|r| r.key()).collect();
        if st.rows.len() != n || (st.drag.is_none() && st.rows.iter().zip(&keys).any(|(a, k)| a.key != *k)) {
            let old = std::mem::take(&mut st.rows);
            st.rows = keys
                .iter()
                .enumerate()
                .map(|(i, k)| {
                    old.iter().find(|a| a.key == *k).copied().unwrap_or(RowAnim {
                        key: *k,
                        y: tops[i],
                        ..Default::default()
                    })
                })
                .collect();
        }

        let g = st.g.update(&touch, now, dt);
        if g.pressed {
            let p = st.g.start;
            if p.x > list.max.x - 48.0 {
                if let Some(i) = (0..n).find(|&i| {
                    let top = list.min.y + tops[i];
                    p.y >= top && p.y < top + row_h(&self.rows[i])
                }) {
                    st.drag = Some((i, tops[i]));
                    st.target = i;
                    st.lifted_key = keys[i];
                }
            }
        }
        if let Some((di, start_top)) = st.drag {
            let dh = row_h(&self.rows[di]);
            let raw = start_top + (st.g.last.y - st.g.start.y);
            let y = rubber_clamp(raw, 0.0, total - dh, 40.0);
            st.rows[di].y = y;
            st.rows[di].v = st.g.vel.y;
            // provisional order: where does the dragged row's center fall?
            let center = y + dh / 2.0;
            let mut others: Vec<usize> = (0..n).filter(|&i| i != di).collect();
            let mut acc = 0.0;
            let mut slot = others.len();
            for (k, &i) in others.iter().enumerate() {
                let h = row_h(&self.rows[i]);
                if center < acc + h / 2.0 {
                    slot = k;
                    break;
                }
                acc += h;
            }
            if slot != st.target {
                // tick: bump the row we just passed
                let passed = if slot < st.target { others.get(slot) } else { others.get(slot.saturating_sub(1)) };
                if let Some(&pi) = passed {
                    st.rows[pi].bump_v = 6.0;
                }
                st.target = slot;
            }
            others.insert(slot, di);
            if g.released || !st.g.down {
                // commit the new order
                let new_rows: Vec<TouchRow> = others.iter().map(|&i| self.rows[i].clone()).collect();
                let new_anims: Vec<RowAnim> = others.iter().map(|&i| st.rows[i]).collect();
                if others.iter().enumerate().any(|(k, &i)| k != i) {
                    response.mark_changed();
                }
                *self.rows = new_rows;
                st.rows = new_anims;
                st.drag = None;
            } else {
                let mut acc = 0.0;
                for &i in &others {
                    if i != di {
                        let target = acc;
                        let a = &mut st.rows[i];
                        spring_step(&mut a.y, &mut a.v, target, 380.0, damping_for(self.bounce, 380.0), dt);
                    }
                    acc += row_h(&self.rows[i]);
                }
            }
        }
        if st.drag.is_none() {
            let mut acc = 0.0;
            for (i, r) in self.rows.iter().enumerate() {
                let a = &mut st.rows[i];
                spring_step(&mut a.y, &mut a.v, acc, 300.0, damping_for(self.bounce, 300.0), dt);
                acc += row_h(r);
            }
        }
        for a in st.rows.iter_mut() {
            spring_step(&mut a.bump, &mut a.bump_v, 0.0, 600.0, 18.0, dt);
        }
        let lift_target = if st.drag.is_some() { 1.0 } else { 0.0 };
        spring_step(&mut st.lift, &mut st.lift_v, lift_target, 420.0, damping_for(self.bounce, 420.0), dt);
        let moving = st.drag.is_some()
            || st.lift.abs() > 0.002
            || st
                .rows
                .iter()
                .zip(&tops)
                .any(|(a, t)| (a.y - t).abs() > 0.1 || a.v.abs() > 0.5 || a.bump_v.abs() > 0.01);
        let s = st.clone();
        glowkit_core::set_state(ui, id.with("reorder"), st);
        if moving || touch.ghost {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }

        // paint
        let painter = ui.painter_at(rect.expand(20.0));
        let fill = surface(&t);
        card(&painter, &t, list, 18.0, 0.7, fill);
        let lifted = s.drag.map(|(i, _)| i).or_else(|| {
            if s.lift > 0.01 {
                keys.iter().position(|k| *k == s.lifted_key)
            } else {
                None
            }
        });
        let draw_row = |i: usize, elevated: f32| {
            let r = &self.rows[i];
            let a = s.rows[i];
            let h = row_h(r);
            let grow = 1.0 + elevated * 0.035 * self.lift / 50.0 + a.bump * 0.01;
            let base = Rect::from_min_size(pos2(list.min.x, list.min.y + a.y), vec2(w, h));
            let rr = Rect::from_center_size(base.center(), vec2(w * grow, h * (1.0 + a.bump * 0.02)));
            if elevated > 0.01 {
                shadow(&painter, &t, rr, 14.0, elevated.clamp(0.0, 1.3) * (0.4 + self.lift / 70.0));
                painter.rect_filled(
                    rr,
                    cr(14.0 * elevated.clamp(0.0, 1.0)),
                    mix(fill, Color32::WHITE, if t.dark { 0.04 } else { 0.0 }),
                );
            }
            let x = rr.min.x + 18.0;
            if r.detail.is_empty() {
                text(&painter, pos2(x, rr.center().y), Align2::LEFT_CENTER, &r.title, 14.0, t.foreground);
            } else {
                text(&painter, pos2(x, rr.center().y - 9.0), Align2::LEFT_CENTER, &r.title, 14.0, t.foreground);
                text(&painter, pos2(x, rr.center().y + 10.0), Align2::LEFT_CENTER, &r.detail, 11.5, t.muted_foreground);
            }
            // grip: 2 x 3 dots
            let gc = pos2(rr.max.x - 24.0, rr.center().y);
            let dot = with_alpha(t.muted_foreground, 0.55 + 0.45 * elevated.clamp(0.0, 1.0));
            for dx in [-3.0, 3.0] {
                for dy in [-5.0, 0.0, 5.0] {
                    painter.circle_filled(gc + vec2(dx, dy), 1.5, dot);
                }
            }
        };
        // separators follow the resting layout
        for i in 1..n {
            if Some(i) == lifted || Some(i - 1) == lifted {
                continue;
            }
            let y = list.min.y + s.rows[i].y;
            painter.line_segment(
                [pos2(list.min.x + 18.0, y), pos2(list.max.x - 12.0, y)],
                egui::Stroke::new(1.0, hairline(&t)),
            );
        }
        for i in 0..n {
            if Some(i) != lifted {
                draw_row(i, 0.0);
            }
        }
        if let Some(i) = lifted {
            draw_row(i, s.lift.max(0.0));
        }
        if self.footer {
            let order: Vec<&str> = self.rows.iter().map(|r| r.title.as_str()).collect();
            text(
                &painter,
                pos2(list.min.x + 14.0, list.max.y + 20.0),
                Align2::LEFT_CENTER,
                order.join(", "),
                11.5,
                t.muted_foreground,
            );
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}
