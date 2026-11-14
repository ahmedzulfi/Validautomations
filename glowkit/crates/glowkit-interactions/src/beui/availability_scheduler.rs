use super::common::{card, cr, hairline, hover_fill, icon, idle_for, morph, BeIcon};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Sense, Stroke, Ui};
use glowkit_core::anim::spring;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// One weekday in an [`AvailabilityScheduler`].
#[derive(Clone, Debug, PartialEq)]
pub struct DayAvailability {
    /// Day name.
    pub name: String,
    /// Whether the day is available at all.
    pub enabled: bool,
    /// Time ranges as (start, end) minutes after midnight.
    pub slots: Vec<(u16, u16)>,
}

impl DayAvailability {
    /// A day with one slot from `start` to `end` (minutes after midnight).
    pub fn new(name: impl Into<String>, enabled: bool, start: u16, end: u16) -> Self {
        Self { name: name.into(), enabled, slots: vec![(start, end)] }
    }
    /// Monday to Friday 9 to 5, weekend off.
    pub fn work_week() -> Vec<Self> {
        ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"]
            .iter()
            .enumerate()
            .map(|(i, d)| Self::new(*d, i < 5, 9 * 60, 17 * 60))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct SchedState {
    /// (day, slot, 0 = start / 1 = end)
    popover: Option<(usize, usize, u8)>,
    last_auto: f64,
    auto_step: u32,
    copied: Option<(usize, f64)>,
}

fn fmt_time(m: u16) -> String {
    let h = m / 60;
    let mm = m % 60;
    let (h12, ap) = match h {
        0 => (12, "AM"),
        1..=11 => (h, "AM"),
        12 => (12, "PM"),
        _ => (h - 12, "PM"),
    };
    format!("{h12}:{mm:02} {ap}")
}

/// A weekly availability editor. Each day has a switch and one or more time ranges; switching a
/// day off folds its time pickers away into "Unavailable", adding a range grows the row on a
/// spring and the new pickers drop in, and a time picker springs open into a short list of
/// times right where it sits. The copy button stamps a day's hours onto every other open day.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{AvailabilityScheduler, DayAvailability};
/// let mut week = DayAvailability::work_week();
/// ui.add(AvailabilityScheduler::new(&mut week));
/// ```
pub struct AvailabilityScheduler<'a> {
    days: &'a mut Vec<DayAvailability>,
    width: f32,
    step: u16,
    autoplay: bool,
}

impl<'a> AvailabilityScheduler<'a> {
    /// An editor over `days`.
    pub fn new(days: &'a mut Vec<DayAvailability>) -> Self {
        Self { days, width: 560.0, step: 30, autoplay: true }
    }
    /// Width of the editor.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(420.0);
        self
    }
    /// Minutes between choices in the time picker (default 30).
    pub fn step(mut self, minutes: u16) -> Self {
        self.step = minutes.clamp(5, 120);
        self
    }
    /// Flip a day and add a range now and then while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const SLOT_H: f32 = 38.0;
const ROW_PAD: f32 = 6.0;

impl egui::Widget for AvailabilityScheduler<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let painter = ui.painter().clone();
        let n = self.days.len();
        // animated row heights
        let id = ui.next_auto_id().with("beui_sched");
        let heights: Vec<f32> = (0..n)
            .map(|i| {
                let d = &self.days[i];
                let slots = if d.enabled { d.slots.len().max(1) } else { 1 };
                let target = slots as f32 * SLOT_H + ROW_PAD * 2.0;
                spring(ui, id.with(("h", i)), target, 320.0, 24.0)
            })
            .collect();
        let total: f32 = heights.iter().sum::<f32>();
        let (rect, mut resp) = ui.allocate_exact_size(vec2(self.width, total.max(10.0) + 16.0), Sense::click());
        let now = ui.input(|i| i.time);
        let st0: SchedState = glowkit_core::state(ui, id, SchedState::default);
        let ptr = ui.input(|i| i.pointer.hover_pos());
        let click = ui.input(|i| if i.pointer.primary_clicked() { i.pointer.interact_pos() } else { None });
        let idle = idle_for(ui, id, ptr.is_some_and(|p| rect.contains(p)));
        let auto = self.autoplay && idle > 2.5;

        let x_switch = rect.min.x + 26.0;
        let x_name = rect.min.x + 52.0;
        let x_slots = rect.min.x + 170.0;
        let pill_w = 116.0;
        let x_end = x_slots + pill_w + 22.0;
        let x_remove = x_end + pill_w + 22.0;
        let x_add = rect.max.x - 50.0;
        let x_copy = rect.max.x - 20.0;
        // row tops
        let mut tops = Vec::with_capacity(n);
        let mut y = rect.min.y + 8.0;
        for h in &heights {
            tops.push(y);
            y += h;
        }
        let pill = |day: usize, slot: usize, which: u8| -> Rect {
            let x = if which == 0 { x_slots } else { x_end };
            Rect::from_min_size(pos2(x, tops[day] + ROW_PAD + slot as f32 * SLOT_H + 4.0), vec2(pill_w, SLOT_H - 8.0))
        };
        // popover geometry
        let step = self.step;
        let options_for = |m: u16| -> Vec<u16> {
            let base = m as i32 - 2 * step as i32;
            (0..6).map(|k| (base + k * step as i32).clamp(0, 24 * 60 - step as i32) as u16).collect::<Vec<_>>()
        };
        let pop_rect = |anchor: Rect| Rect::from_min_size(anchor.min, vec2(anchor.width(), 6.0 * 30.0 + 8.0));
        // ---- input
        let mut st = st0;
        let mut changed = false;
        if let Some(p) = click {
            let mut consumed = false;
            if let Some((d, s, w)) = st.popover {
                if d < n && s < self.days[d].slots.len() {
                    let anchor = pill(d, s, w);
                    let pr = pop_rect(anchor);
                    if pr.contains(p) {
                        let k = (((p.y - pr.min.y - 4.0) / 30.0).floor() as i32).clamp(0, 5) as usize;
                        let cur = self.days[d].slots[s];
                        let v = options_for(if w == 0 { cur.0 } else { cur.1 })[k];
                        if w == 0 {
                            self.days[d].slots[s].0 = v;
                        } else {
                            self.days[d].slots[s].1 = v;
                        }
                        changed = true;
                    }
                }
                st.popover = None;
                consumed = true;
            }
            if !consumed {
                for d in 0..n {
                    let row = Rect::from_min_size(pos2(rect.min.x, tops[d]), vec2(self.width, heights[d]));
                    if !row.contains(p) {
                        continue;
                    }
                    let mid = pos2(0.0, tops[d] + ROW_PAD + SLOT_H / 2.0);
                    if (p.x - x_switch).abs() < 20.0 && (p.y - mid.y).abs() < 14.0 {
                        self.days[d].enabled = !self.days[d].enabled;
                        if self.days[d].slots.is_empty() {
                            self.days[d].slots.push((9 * 60, 17 * 60));
                        }
                        changed = true;
                    } else if (p.x - x_add).abs() < 13.0 && (p.y - mid.y).abs() < 13.0 {
                        let d_ = &mut self.days[d];
                        if !d_.enabled {
                            d_.enabled = true;
                            if d_.slots.is_empty() {
                                d_.slots.push((9 * 60, 17 * 60));
                            }
                        } else if d_.slots.len() < 3 {
                            let last = d_.slots.last().map(|s| s.1).unwrap_or(9 * 60);
                            let s0 = (last + 60).min(22 * 60);
                            d_.slots.push((s0, (s0 + 120).min(23 * 60 + 30)));
                        }
                        changed = true;
                    } else if (p.x - x_copy).abs() < 13.0 && (p.y - mid.y).abs() < 13.0 {
                        let src = self.days[d].clone();
                        for (j, o) in self.days.iter_mut().enumerate() {
                            if j != d && o.enabled {
                                o.slots = src.slots.clone();
                            }
                        }
                        st.copied = Some((d, now));
                        changed = true;
                    } else if self.days[d].enabled {
                        for s in 0..self.days[d].slots.len() {
                            for w in 0..2u8 {
                                if pill(d, s, w).contains(p) {
                                    st.popover = Some((d, s, w));
                                }
                            }
                            let xr = pos2(x_remove, tops[d] + ROW_PAD + s as f32 * SLOT_H + SLOT_H / 2.0);
                            if xr.distance(p) < 12.0 {
                                if self.days[d].slots.len() > 1 {
                                    self.days[d].slots.remove(s);
                                } else {
                                    self.days[d].enabled = false;
                                }
                                changed = true;
                                break;
                            }
                        }
                    }
                }
            }
        }
        if auto && now - st.last_auto > 1.6 && n >= 6 {
            st.last_auto = now;
            match st.auto_step % 4 {
                0 => self.days[5].enabled = true,
                1 => {
                    let d = &mut self.days[0];
                    if d.enabled && d.slots.len() < 2 {
                        let last = d.slots.last().map(|s| s.1).unwrap_or(17 * 60);
                        d.slots.push(((last + 60).min(22 * 60), (last + 180).min(23 * 60 + 30)));
                    }
                }
                2 => self.days[5].enabled = false,
                _ => {
                    if self.days[0].slots.len() > 1 {
                        self.days[0].slots.truncate(1);
                    }
                }
            }
            st.auto_step += 1;
        }
        if auto {
            glowkit_core::animate(ui);
        }
        glowkit_core::set_state(ui, id, st);
        if changed {
            resp.mark_changed();
        }
        if ptr.is_some_and(|p| rect.contains(p)) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        // ---- paint
        for d in 0..n {
            let day = &self.days[d];
            let top = tops[d];
            let h = heights[d];
            let row = Rect::from_min_size(pos2(rect.min.x, top), vec2(self.width, h));
            let clip = painter.with_clip_rect(row);
            if d + 1 < n {
                painter.line_segment([pos2(rect.min.x, row.max.y), pos2(rect.max.x, row.max.y)], hairline(&t));
            }
            let my = top + ROW_PAD + SLOT_H / 2.0;
            let on = spring(ui, id.with(("on", d)), if day.enabled { 1.0 } else { 0.0 }, 360.0, 24.0);
            // switch
            let sw = Rect::from_center_size(pos2(x_switch, my), vec2(40.0, 24.0));
            let track = mix(if t.dark { mix(t.muted, Color32::WHITE, 0.15) } else { mix(t.muted, t.border, 0.9) }, t.accent, on.clamp(0.0, 1.0));
            painter.rect_filled(sw, cr(12.0), track);
            let kx = sw.min.x + 12.0 + 16.0 * on;
            painter.circle_filled(pos2(kx, my), 9.5, Color32::WHITE);
            painter.text(pos2(x_name, my), Align2::LEFT_CENTER, &day.name, FontId::proportional(14.0), mix(t.muted_foreground, t.foreground, 0.4 + 0.6 * on));
            // "Unavailable" fades in as the pickers fold away
            let ua = (1.0 - on * 1.6).clamp(0.0, 1.0);
            if ua > 0.0 {
                clip.text(pos2(x_slots + (1.0 - ua) * -10.0, my), Align2::LEFT_CENTER, "Unavailable", FontId::proportional(13.5), with_alpha(t.muted_foreground, ua));
            }
            let slot_a = ((on - 0.2) / 0.8).clamp(0.0, 1.0);
            if slot_a > 0.0 {
                for (s, (a, b)) in day.slots.iter().enumerate() {
                    let enter = spring(ui, id.with(("slot", d, s)), 1.0, 300.0, 20.0);
                    let sa = slot_a * enter;
                    let sy = top + ROW_PAD + s as f32 * SLOT_H + SLOT_H / 2.0;
                    for (w, m) in [(0u8, *a), (1u8, *b)] {
                        let r0 = pill(d, s, w);
                        // pickers fold in from the left when the day turns on
                        let r = Rect::from_min_size(r0.min, vec2(r0.width() * (0.6 + 0.4 * sa), r0.height()));
                        let hov = ptr.is_some_and(|p| r0.contains(p));
                        clip.rect(r, cr(10.0), with_alpha(if hov { hover_fill(&t) } else { super::common::well(&t) }, sa), Stroke::new(1.0, with_alpha(hairline(&t).color, sa)), egui::StrokeKind::Inside);
                        clip.text(pos2(r.min.x + 12.0, sy), Align2::LEFT_CENTER, fmt_time(m), FontId::proportional(13.0), with_alpha(t.foreground, sa));
                        icon(&clip, BeIcon::ChevronDown, pos2(r.max.x - 16.0, sy), 9.0, with_alpha(t.muted_foreground, sa), 1.3);
                    }
                    clip.text(pos2(x_slots + pill_w + 11.0, sy), Align2::CENTER_CENTER, "–", FontId::proportional(13.0), with_alpha(t.muted_foreground, sa));
                    icon(&clip, BeIcon::Close, pos2(x_remove, sy), 9.0, with_alpha(t.muted_foreground, sa), 1.3);
                }
            }
            icon(&painter, BeIcon::Plus, pos2(x_add, my), 11.0, t.muted_foreground, 1.4);
            let flash = st.copied.filter(|c| c.0 == d).map(|c| (1.0 - (now - c.1) as f32 / 0.9).clamp(0.0, 1.0)).unwrap_or(0.0);
            if flash > 0.0 {
                glowkit_core::animate(ui);
                icon(&painter, BeIcon::Check, pos2(x_copy, my), 11.0, mix(t.muted_foreground, t.success, flash), 1.5);
            } else {
                icon(&painter, BeIcon::Copy, pos2(x_copy, my), 11.0, t.muted_foreground, 1.3);
            }
        }
        // popover springs out of the picker it belongs to
        let pop = st.popover.filter(|(d, s, _)| *d < n && *s < self.days[*d].slots.len());
        let pid = id.with("pop");
        let open = pop.is_some();
        let last_anchor: Option<Rect> = glowkit_core::with_state(ui, pid.with("anchor"), || None, |a| {
            if let Some((d, s, w)) = pop {
                *a = Some(pill(d, s, w));
            }
            *a
        });
        if let Some(anchor) = last_anchor {
            let target = if open { pop_rect(anchor) } else { anchor };
            let (r, rad) = morph(ui, pid, target, if open { 12.0 } else { 10.0 }, 420.0, 30.0);
            let k = ((r.height() - anchor.height()) / (pop_rect(anchor).height() - anchor.height())).clamp(0.0, 1.0);
            if k > 0.02 {
                card(&painter, &t, r, rad, 0.8 * k);
                let clip = painter.with_clip_rect(r.shrink(1.0));
                if let Some((d, s, w)) = pop.or(None) {
                    let cur = self.days[d].slots[s];
                    let m = if w == 0 { cur.0 } else { cur.1 };
                    for (k_, v) in options_for(m).iter().enumerate() {
                        let rr = Rect::from_min_size(pos2(r.min.x + 4.0, r.min.y + 4.0 + k_ as f32 * 30.0), vec2(r.width() - 8.0, 30.0));
                        let hov = ptr.is_some_and(|p| rr.contains(p));
                        if *v == m || hov {
                            clip.rect_filled(rr, cr(8.0), if *v == m { with_alpha(t.accent, 0.14) } else { hover_fill(&t) });
                        }
                        clip.text(
                            pos2(rr.min.x + 10.0, rr.center().y),
                            Align2::LEFT_CENTER,
                            fmt_time(*v),
                            FontId::proportional(13.0),
                            with_alpha(if *v == m { t.accent } else { t.foreground }, k),
                        );
                    }
                }
            }
        }
        resp
    }
}
