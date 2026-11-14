//! A list with an edit mode for picking several rows and acting on them from a floating bar.

use super::common::*;
use super::reorder::TouchRow;
use egui::{pos2, vec2, Align2, Color32, Rect, Response, Sense, Ui, Vec2};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Copy, Default)]
struct RowSel {
    key: u64,
    on: bool,
    pop: f32,
    pop_v: f32,
    height: f32,
    height_v: f32,
    removing: bool,
    slide: f32,
    slide_v: f32,
}

#[derive(Clone, Default)]
struct SelectState {
    edit_t: f32,
    editing: bool,
    edit: f32,
    edit_v: f32,
    bar: f32,
    bar_v: f32,
    rows: Vec<RowSel>,
    sweep: Option<(bool, usize)>,
    count_pop: f32,
    count_pop_v: f32,
    last_count: usize,
    g: Gesture,
    cycle: u32,
}

/// A notes list with an iOS-style Select mode.
///
/// Tap **Select** and every row slides aside on a spring to make room for a round checkbox that
/// slides in from the edge (rows follow one another in a quick cascade), while a floating toolbar
/// rises from the bottom. Tap rows to toggle them (the check pops in), or press in the checkbox
/// column and sweep up or down to select a whole run in one stroke. The toolbar counts the
/// selection and Delete collapses the chosen rows away. **Done** leaves edit mode and clears it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{SelectionList, TouchRow};
/// let mut notes = vec![TouchRow::new("Groceries", "Milk, eggs, bread")];
/// ui.add(SelectionList::new(&mut notes).title("Notes"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SelectionList<'a> {
    rows: &'a mut Vec<TouchRow>,
    title: String,
    size: Option<Vec2>,
    bounce: f32,
    cascade: f32,
    autoplay: bool,
}

impl<'a> SelectionList<'a> {
    /// A selectable list over `rows` (Delete removes entries from the `Vec`).
    pub fn new(rows: &'a mut Vec<TouchRow>) -> Self {
        Self { rows, title: "Notes".into(), size: None, bounce: 30.0, cascade: 40.0, autoplay: false }
    }
    /// Large title above the list.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    /// Widget size (default: all available space).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Overshoot of the edit-mode slide, 0..100.
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 100.0);
        self
    }
    /// Delay between rows as they slide, 0..100.
    pub fn cascade(mut self, c: f32) -> Self {
        self.cascade = c.clamp(0.0, 100.0);
        self
    }
    /// Select rows on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

/// Sample notes for demos.
pub(crate) fn sample_notes() -> Vec<TouchRow> {
    vec![
        TouchRow::new("Groceries", "Oat milk, eggs, sourdough, coffee"),
        TouchRow::new("Trip ideas", "Lisbon, Porto and the coast road"),
        TouchRow::new("Book list", "Three novels for the summer"),
        TouchRow::new("Meeting notes", "Ship the beta on Friday"),
        TouchRow::new("Gift ideas", "Headphones, a plant, a scarf"),
        TouchRow::new("Recipes", "Lemon pasta and a quick curry"),
    ]
}

const ROW_H: f32 = 58.0;
const SCRIPT: [Stroke2; 5] = [
    tap(0.8, 0.86, 0.087),
    tap(1.8, 0.55, 0.301),
    drag(2.6, 3.4, (0.09, 0.411), (0.09, 0.64)),
    tap(4.3, 0.55, 0.411),
    tap(5.6, 0.86, 0.087),
];

impl egui::Widget for SelectionList<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = self.size.unwrap_or(ui.available_size());
        let (rect, mut response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let touch = touch(ui, &response, rect, self.autoplay, &SCRIPT, 7.2);
        let mut st: SelectState = glowkit_core::state(ui, id.with("sel"), SelectState::default);
        if touch.ghost && touch.cycle != st.cycle {
            st.cycle = touch.cycle;
            st.editing = false;
            for r in st.rows.iter_mut() {
                r.on = false;
            }
        }
        let keys: Vec<u64> = self.rows.iter().map(|r| r.key()).collect();
        let old = std::mem::take(&mut st.rows);
        st.rows = keys
            .iter()
            .map(|k| old.iter().find(|a| a.key == *k).copied().unwrap_or(RowSel { key: *k, height: 1.0, ..Default::default() }))
            .collect();
        let n = st.rows.len();

        let btn = Rect::from_center_size(pos2(rect.max.x - 36.0, rect.min.y + 46.0), vec2(64.0, 30.0));
        let list_top = rect.min.y + 72.0;
        let list_x = rect.min.x + 12.0;
        let list_w = rect.width() - 24.0;
        let row_y = |rows: &[RowSel], i: usize| -> f32 { list_top + rows[..i].iter().map(|r| r.height.clamp(0.0, 1.0) * ROW_H).sum::<f32>() };
        let bar = Rect::from_center_size(pos2(rect.center().x, rect.max.y - 40.0 + (1.0 - st.bar) * 90.0), vec2(rect.width() - 36.0, 48.0));
        let del_c = pos2(bar.max.x - 28.0, bar.center().y);
        let arch_c = pos2(bar.max.x - 66.0, bar.center().y);

        let g = st.g.update(&touch, now, dt);
        let hit_row = |rows: &[RowSel], p: egui::Pos2| (0..rows.len()).find(|&i| {
            let y = row_y(rows, i);
            p.y >= y && p.y < y + ROW_H * rows[i].height.clamp(0.0, 1.0) && p.x > list_x && p.x < list_x + list_w
        });
        if g.pressed && st.editing && st.g.start.x < list_x + 40.0 {
            if let Some(i) = hit_row(&st.rows, st.g.start) {
                let on = !st.rows[i].on;
                st.sweep = Some((on, i));
                st.rows[i].on = on;
                st.rows[i].pop_v = 8.0;
            }
        }
        if let Some((on, _)) = st.sweep {
            if st.g.down {
                if let Some(i) = hit_row(&st.rows, st.g.last) {
                    if st.rows[i].on != on {
                        st.rows[i].on = on;
                        st.rows[i].pop_v = 8.0;
                    }
                }
            } else {
                st.sweep = None;
            }
        } else if g.tap {
            let p = st.g.last;
            if btn.contains(p) {
                st.editing = !st.editing;
                st.edit_t = now;
                if !st.editing {
                    for r in st.rows.iter_mut() {
                        r.on = false;
                    }
                }
            } else if st.editing && st.bar > 0.5 && (p - del_c).length() < 20.0 {
                for r in st.rows.iter_mut().filter(|r| r.on) {
                    r.removing = true;
                }
            } else if st.editing && st.bar > 0.5 && (p - arch_c).length() < 20.0 {
                for r in st.rows.iter_mut() {
                    r.on = false;
                }
            } else if st.editing {
                if let Some(i) = hit_row(&st.rows, p) {
                    st.rows[i].on = !st.rows[i].on;
                    st.rows[i].pop_v = 8.0;
                }
            }
        }
        let count = st.rows.iter().filter(|r| r.on && !r.removing).count();
        if count != st.last_count {
            st.count_pop_v = 6.0;
            st.last_count = count;
        }
        let et = if st.editing { 1.0 } else { 0.0 };
        spring_step(&mut st.edit, &mut st.edit_v, et, 260.0, damping_for(self.bounce, 260.0), dt);
        spring_step(&mut st.bar, &mut st.bar_v, et, 220.0, damping_for(self.bounce, 220.0), dt);
        spring_step(&mut st.count_pop, &mut st.count_pop_v, 0.0, 500.0, 18.0, dt);
        let mut removed = Vec::new();
        let delay = self.cascade / 100.0 * 0.05;
        let since = now - st.edit_t;
        let editing = st.editing;
        let mut sliding = false;
        for (i, r) in st.rows.iter_mut().enumerate() {
            let tgt = if (since > i as f32 * delay) == editing { 1.0 } else { 0.0 };
            spring_step(&mut r.slide, &mut r.slide_v, tgt, 300.0, damping_for(self.bounce, 300.0), dt);
            sliding |= !settled(r.slide, r.slide_v, et);
            spring_step(&mut r.pop, &mut r.pop_v, 0.0, 520.0, 16.0, dt);
            if r.removing {
                spring_step(&mut r.height, &mut r.height_v, 0.0, 240.0, 30.0, dt);
                if r.height < 0.01 {
                    removed.push(i);
                }
            }
        }
        for &i in removed.iter().rev() {
            if i < self.rows.len() {
                self.rows.remove(i);
                st.rows.remove(i);
                response.mark_changed();
            }
        }
        let moving = sliding
            || !settled(st.edit, st.edit_v, et)
            || !settled(st.bar, st.bar_v, et)
            || st.rows.iter().any(|r| r.pop_v.abs() > 0.01 || r.removing)
            || st.count_pop_v.abs() > 0.01
            || st.g.down;
        let s = st.clone();
        glowkit_core::set_state(ui, id.with("sel"), st);
        if moving || touch.ghost {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }

        // paint
        let painter = ui.painter_at(rect);
        let blue = system_blue(&t);
        text_bold(&painter, pos2(rect.min.x + 18.0, rect.min.y + 46.0), Align2::LEFT_CENTER, &self.title, 24.0, t.foreground);
        text(&painter, btn.center(), Align2::CENTER_CENTER, if s.editing { "Done" } else { "Select" }, 14.0, blue);
        let fill = surface(&t);
        let total: f32 = s.rows.iter().map(|r| r.height.clamp(0.0, 1.0) * ROW_H).sum();
        let list = Rect::from_min_size(pos2(list_x, list_top), vec2(list_w, total.max(1.0)));
        if n > 0 {
            card(&painter, &t, list, 14.0, 0.5, fill);
        }
        let clip = painter.with_clip_rect(list);
        for i in 0..n {
            let r = s.rows[i];
            let h = r.height.clamp(0.0, 1.0) * ROW_H;
            if h < 0.5 || i >= self.rows.len() {
                continue;
            }
            let y = row_y(&s.rows, i);
            let row = Rect::from_min_size(pos2(list_x, y), vec2(list_w, h));
            let e = r.slide;
            let shift = 38.0 * e;
            if r.on {
                clip.rect_filled(row, cr(0.0), mix(fill, blue, if t.dark { 0.14 } else { 0.07 }));
            }
            // checkbox
            let cc = pos2(row.min.x + 22.0 - (1.0 - e) * 40.0, row.center().y);
            let rad = 10.0 * (1.0 + r.pop * 0.06);
            if r.on {
                clip.circle_filled(cc, rad, blue);
                glyph(&clip, Glyph::Check, cc, 10.0 * (1.0 + r.pop * 0.12), Color32::WHITE);
            } else {
                clip.circle_stroke(cc, rad, egui::Stroke::new(1.5, with_alpha(t.muted_foreground, 0.7)));
            }
            let x = row.min.x + 16.0 + shift;
            let row_r = &self.rows[i];
            text_bold(&clip, pos2(x, row.center().y - 9.0), Align2::LEFT_CENTER, &row_r.title, 14.0, t.foreground);
            text(&clip, pos2(x, row.center().y + 10.0), Align2::LEFT_CENTER, &row_r.detail, 12.0, t.muted_foreground);
            let chev_a = (1.0 - e).clamp(0.0, 1.0);
            glyph(&clip, Glyph::ChevronRight, pos2(row.max.x - 18.0, row.center().y), 10.0, with_alpha(t.muted_foreground, 0.6 * chev_a));
            if i + 1 < n {
                clip.line_segment(
                    [pos2(row.min.x + 16.0 + shift.max(0.0), row.max.y - 0.5), pos2(row.max.x, row.max.y - 0.5)],
                    egui::Stroke::new(1.0, hairline(&t)),
                );
            }
        }
        // floating toolbar
        if s.bar > 0.01 {
            let bar = Rect::from_center_size(pos2(rect.center().x, rect.max.y - 40.0 + (1.0 - s.bar) * 90.0), vec2(rect.width() - 36.0, 48.0));
            let bfill = if t.dark { hexa(0x2C, 0x2C, 0x30) } else { Color32::WHITE };
            card(&painter, &t, bar, 24.0, 1.2, bfill);
            let label = if count == 0 { "Select items".to_string() } else { format!("{count} selected") };
            let sc = 1.0 + s.count_pop * 0.05;
            painter.text(
                pos2(bar.min.x + 20.0, bar.center().y - s.count_pop * 1.5),
                Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(14.0 * sc),
                if count == 0 { t.muted_foreground } else { t.foreground },
            );
            let on_a = if count > 0 { 1.0 } else { 0.35 };
            let del = if t.dark { hexa(0xFF, 0x45, 0x3A) } else { hexa(0xE5, 0x30, 0x2A) };
            glyph(&painter, Glyph::Archive, arch_c, 17.0, with_alpha(blue, on_a));
            glyph(&painter, Glyph::Trash, del_c, 17.0, with_alpha(del, on_a));
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}
