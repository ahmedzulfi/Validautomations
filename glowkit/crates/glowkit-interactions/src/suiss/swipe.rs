//! List rows that reveal action buttons when swiped sideways, with full-swipe to run one.

use super::common::*;
use super::reorder::TouchRow;
use egui::{pos2, vec2, Align2, Color32, Rect, Response, Sense, Ui};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Copy, Default)]
struct SwipeAnim {
    key: u64,
    x: f32,
    v: f32,
    height: f32,
    height_v: f32,
    removing: bool,
    full: f32,
    full_v: f32,
}

#[derive(Clone, Default)]
struct SwipeState {
    rows: Vec<SwipeAnim>,
    active: Option<u64>,
    axis: u8,
    start_x: f32,
    open: Option<u64>,
    g: Gesture,
    toast: Option<(String, f32)>,
}

struct Act {
    label: &'static str,
    glyph: Glyph,
    color: Color32,
}

/// A message list whose rows slide sideways to reveal actions, like a mail app.
///
/// Drag a row left to uncover Flag and Delete, or right to uncover Pin. The row tracks the finger
/// 1:1; let go past half the buttons' width and it springs open (carrying your flick), otherwise
/// it springs shut. Dragging further than the buttons meets growing rubber-band resistance,
/// except on the delete side: past 60% of the row the Delete button stretches across the whole
/// row with a little lurch, and releasing there collapses the row away. Only one row stays open
/// at a time; tapping a row closes it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{SwipeActions, TouchRow};
/// let mut rows = vec![TouchRow::new("Maya Chen", "Dinner on Friday?")];
/// ui.add(SwipeActions::new(&mut rows).full_swipe(true));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SwipeActions<'a> {
    rows: &'a mut Vec<TouchRow>,
    width: Option<f32>,
    full_swipe: bool,
    resistance: f32,
    bounce: f32,
    autoplay: bool,
}

impl<'a> SwipeActions<'a> {
    /// Swipeable rows over `rows` (full-swipe deletes remove entries from the `Vec`).
    pub fn new(rows: &'a mut Vec<TouchRow>) -> Self {
        Self { rows, width: None, full_swipe: true, resistance: 50.0, bounce: 30.0, autoplay: false }
    }
    /// Width in points (default: available width).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Allow a long swipe to run the outermost action (delete).
    pub fn full_swipe(mut self, on: bool) -> Self {
        self.full_swipe = on;
        self
    }
    /// How hard over-dragging pushes back, 0..100.
    pub fn resistance(mut self, r: f32) -> Self {
        self.resistance = r.clamp(0.0, 100.0);
        self
    }
    /// Overshoot of the open / close snap, 0..100.
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 100.0);
        self
    }
    /// Swipe rows on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const ROW_H: f32 = 66.0;
const BTN_W: f32 = 70.0;

/// Sample message rows for demos.
pub(crate) fn sample_messages() -> Vec<TouchRow> {
    vec![
        TouchRow::new("Maya Chen", "Dinner on Friday? I found a place"),
        TouchRow::new("Northwind Store", "Your parcel is on the way"),
        TouchRow::new("Cloud Backup", "Storage is almost full"),
        TouchRow::new("Jordan Lee", "Slides for Monday's review"),
        TouchRow::new("Studio Weekly", "Five new templates this week"),
    ]
}

impl egui::Widget for SwipeActions<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = self.width.unwrap_or_else(|| ui.available_width());
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let id = ui.next_auto_id();
        let mut st: SwipeState = glowkit_core::state(ui, id.with("swipe"), SwipeState::default);
        // sync
        let keys: Vec<u64> = self.rows.iter().map(|r| r.key()).collect();
        let old = std::mem::take(&mut st.rows);
        let mut rows: Vec<SwipeAnim> = keys
            .iter()
            .map(|k| {
                old.iter().find(|a| a.key == *k).copied().unwrap_or(SwipeAnim {
                    key: *k,
                    height: 1.0,
                    ..Default::default()
                })
            })
            .collect();
        let total: f32 = rows.iter().map(|a| a.height.clamp(0.0, 1.0) * ROW_H).sum::<f32>().max(1.0);
        let (rect, response) = ui.allocate_exact_size(vec2(w, total + 34.0), Sense::click_and_drag());
        let list = Rect::from_min_size(rect.min, vec2(w, total));
        let ry = |i: usize| (ROW_H * (i as f32 + 0.5)) / rect.height();
        let script = [
            drag(0.8, 1.25, (0.82, ry(0)), (0.42, ry(0))),
            tap(2.4, 0.3, ry(0)),
            drag(3.4, 3.85, (0.14, ry(1)), (0.5, ry(1))),
            tap(5.0, 0.6, ry(1)),
            drag(6.0, 6.5, (0.92, ry(2)), (0.1, ry(2))),
        ];
        let touch = touch(ui, &response, rect, self.autoplay, &script, 8.5);

        let trailing = [
            Act { label: "Flag", glyph: Glyph::Flag, color: hexa(0xFF, 0x9F, 0x0A) },
            Act { label: "Delete", glyph: Glyph::Trash, color: hexa(0xFF, 0x3B, 0x30) },
        ];
        let leading = [Act { label: "Pin", glyph: Glyph::Pin, color: hexa(0x0A, 0x84, 0xFF) }];
        let trail_w = BTN_W * trailing.len() as f32;
        let lead_w = BTN_W * leading.len() as f32;
        let full_at = w * 0.6;

        let row_top =
            |rows: &[SwipeAnim], i: usize| -> f32 { rows[..i].iter().map(|a| a.height.clamp(0.0, 1.0) * ROW_H).sum() };
        let g = st.g.update(&touch, now, dt);
        if g.pressed {
            let p = st.g.start;
            st.axis = 0;
            st.active = (0..rows.len())
                .find(|&i| {
                    let top = list.min.y + row_top(&rows, i);
                    p.y >= top && p.y < top + ROW_H && !rows[i].removing
                })
                .map(|i| rows[i].key);
            if let Some(k) = st.active {
                st.start_x = rows.iter().find(|a| a.key == k).map(|a| a.x).unwrap_or(0.0);
            }
            if st.open.is_some() && st.open != st.active {
                st.open = None; // pressing elsewhere closes the open row
            }
        }
        let mut removed: Option<usize> = None;
        if let Some(k) = st.active {
            if let Some(i) = rows.iter().position(|a| a.key == k) {
                let d = st.g.last - st.g.start;
                if st.axis == 0 && d.length() > 6.0 {
                    st.axis = if d.x.abs() > d.y.abs() { 1 } else { 2 };
                }
                if st.g.down && st.axis == 1 {
                    let raw = st.start_x + d.x;
                    let res = 1.0 + self.resistance / 25.0;
                    let x = if raw < -trail_w {
                        if self.full_swipe {
                            -trail_w + (raw + trail_w) * 0.92
                        } else {
                            -trail_w + rubber(raw + trail_w, 120.0 / res)
                        }
                    } else if raw > lead_w {
                        lead_w + rubber(raw - lead_w, 120.0 / res)
                    } else {
                        raw
                    };
                    rows[i].x = x;
                    rows[i].v = st.g.vel.x;
                    let full_t = if self.full_swipe && -x > full_at { 1.0 } else { 0.0 };
                    let a = &mut rows[i];
                    spring_step(&mut a.full, &mut a.full_v, full_t, 500.0, 24.0, dt);
                }
                if g.released {
                    let x = rows[i].x;
                    let proj = x + project(st.g.vel.x, 0.99);
                    if st.axis == 1 {
                        if self.full_swipe && -x > full_at {
                            rows[i].removing = true;
                            st.open = None;
                            st.toast = Some((format!("Deleted “{}”", self.rows[i].title), now));
                        } else if proj < -trail_w * 0.5 || proj > lead_w * 0.5 {
                            st.open = Some(k);
                        } else {
                            st.open = None;
                        }
                    } else if g.tap {
                        // tap on an open row: hit a button or close
                        if st.open == Some(k) {
                            let px = st.g.last.x - list.min.x;
                            if x < -1.0 && px > w + x {
                                let b =
                                    (((px - (w + x)) / (-x / trailing.len() as f32)) as usize).min(trailing.len() - 1);
                                if b + 1 == trailing.len() {
                                    rows[i].removing = true;
                                    st.toast = Some((format!("Deleted “{}”", self.rows[i].title), now));
                                } else {
                                    st.toast = Some((format!("{} “{}”", trailing[b].label, self.rows[i].title), now));
                                }
                            } else if x > 1.0 && px < x {
                                st.toast = Some((format!("Pinned “{}”", self.rows[i].title), now));
                            }
                        }
                        st.open = None;
                    }
                    st.active = None;
                }
            }
        }
        // springs
        let stiff = 320.0;
        let damp = damping_for(self.bounce, stiff);
        let mut moving = st.g.down;
        for (i, a) in rows.iter_mut().enumerate() {
            let dragging = st.active == Some(a.key) && st.g.down && st.axis == 1;
            let target = if a.removing {
                -w * 1.1
            } else if st.open == Some(a.key) {
                if a.x < 0.0 {
                    -trail_w
                } else {
                    lead_w
                }
            } else {
                0.0
            };
            if !dragging {
                spring_step(&mut a.x, &mut a.v, target, stiff, damp, dt);
                let ft = if a.removing { 1.0 } else { 0.0 };
                spring_step(&mut a.full, &mut a.full_v, ft, 500.0, 26.0, dt);
            }
            if a.removing && a.x < -w * 0.9 {
                spring_step(&mut a.height, &mut a.height_v, 0.0, 260.0, 30.0, dt);
                if a.height < 0.01 {
                    removed = Some(i);
                }
            }
            moving |= !settled(a.x, a.v, target) || a.removing || a.full_v.abs() > 0.01;
        }
        if let Some(i) = removed {
            if i < self.rows.len() {
                self.rows.remove(i);
                rows.remove(i);
            }
        }
        if st.toast.as_ref().is_some_and(|(_, t0)| now - t0 > 2.2) {
            st.toast = None;
        }
        st.rows = rows.clone();
        let toast = st.toast.clone();
        glowkit_core::set_state(ui, id.with("swipe"), st);
        if moving || touch.ghost || toast.is_some() {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }

        // paint
        let painter = ui.painter_at(rect.expand(16.0));
        let fill = surface(&t);
        card(&painter, &t, list, 16.0, 0.6, fill);
        let clip_all = painter.with_clip_rect(list.shrink(0.5));
        let mut y = list.min.y;
        for (i, a) in rows.iter().enumerate() {
            let h = a.height.clamp(0.0, 1.0) * ROW_H;
            if h < 0.5 || i >= self.rows.len() {
                y += h;
                continue;
            }
            let row = Rect::from_min_size(pos2(list.min.x, y), vec2(w, h));
            let clip = clip_all.with_clip_rect(row.intersect(list));
            let first = i == 0;
            let last = i + 1 == rows.len();
            let rad = |left: bool, right: bool| egui::CornerRadius {
                nw: if first && left { 16 } else { 0 },
                ne: if first && right { 16 } else { 0 },
                sw: if last && left { 16 } else { 0 },
                se: if last && right { 16 } else { 0 },
            };
            let x = a.x;
            // actions behind
            if x < -0.5 {
                let revealed = Rect::from_min_max(pos2(row.max.x + x, row.min.y), row.max);
                let n = trailing.len();
                let full = a.full.clamp(0.0, 1.0);
                for (b, act) in trailing.iter().enumerate() {
                    let share = revealed.width() / n as f32;
                    let mut bx0 = revealed.min.x + share * b as f32;
                    let mut bx1 = bx0 + share;
                    if b + 1 == n {
                        bx0 += (revealed.min.x - bx0) * full;
                    } else {
                        bx1 -= (bx1 - bx0) * full;
                    }
                    let br = Rect::from_min_max(pos2(bx0, row.min.y), pos2(bx1.max(bx0), row.max.y));
                    clip.rect_filled(br.expand2(vec2(0.5, 0.0)), rad(false, b + 1 == n), act.color);
                    let cx = if b + 1 == n { br.min.x + (BTN_W / 2.0).min(br.width() / 2.0) } else { br.center().x };
                    let cx = if b + 1 == n && full > 0.5 { br.min.x + BTN_W / 2.0 } else { cx };
                    let alpha = ((br.width() - 20.0) / 30.0).clamp(0.0, 1.0);
                    glyph(&clip, act.glyph, pos2(cx, row.center().y - 8.0), 15.0, with_alpha(Color32::WHITE, alpha));
                    text(
                        &clip,
                        pos2(cx, row.center().y + 13.0),
                        Align2::CENTER_CENTER,
                        act.label,
                        11.0,
                        with_alpha(Color32::WHITE, alpha),
                    );
                }
            } else if x > 0.5 {
                let revealed = Rect::from_min_max(row.min, pos2(row.min.x + x, row.max.y));
                let act = &leading[0];
                clip.rect_filled(revealed, rad(true, false), act.color);
                let cx = revealed.max.x - (BTN_W / 2.0).min(revealed.width() / 2.0);
                let alpha = ((revealed.width() - 20.0) / 30.0).clamp(0.0, 1.0);
                glyph(&clip, act.glyph, pos2(cx, row.center().y - 8.0), 15.0, with_alpha(Color32::WHITE, alpha));
                text(
                    &clip,
                    pos2(cx, row.center().y + 13.0),
                    Align2::CENTER_CENTER,
                    act.label,
                    11.0,
                    with_alpha(Color32::WHITE, alpha),
                );
            }
            // content
            let content = row.translate(vec2(x, 0.0));
            clip.rect_filled(content, rad(true, true), fill);
            let r = &self.rows[i];
            let unread = i % 2 == 0;
            if unread {
                clip.circle_filled(pos2(content.min.x + 11.0, content.min.y + 22.0), 4.0, system_blue(&t));
            }
            text_bold(
                &clip,
                pos2(content.min.x + 22.0, content.min.y + 22.0),
                Align2::LEFT_CENTER,
                &r.title,
                14.0,
                t.foreground,
            );
            text(
                &clip,
                pos2(content.max.x - 14.0, content.min.y + 22.0),
                Align2::RIGHT_CENTER,
                ["9:41", "8:02", "Yesterday", "Mon", "Sun", "Sat"][i % 6],
                11.0,
                t.muted_foreground,
            );
            text(
                &clip,
                pos2(content.min.x + 22.0, content.min.y + 44.0),
                Align2::LEFT_CENTER,
                &r.detail,
                12.5,
                mix(t.muted_foreground, t.foreground, 0.25),
            );
            if i + 1 < rows.len() {
                clip.line_segment(
                    [pos2(content.min.x + 22.0, row.max.y - 0.5), pos2(row.max.x, row.max.y - 0.5)],
                    egui::Stroke::new(1.0, hairline(&t)),
                );
            }
            y += h;
        }
        if let Some((msg, t0)) = toast {
            let a = ramp(now - t0, 0.0, 0.2) * (1.0 - ramp(now - t0, 1.8, 2.2));
            text(
                &painter,
                pos2(list.min.x + 6.0, list.max.y + 18.0),
                Align2::LEFT_CENTER,
                msg,
                11.5,
                with_alpha(t.muted_foreground, a),
            );
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}
