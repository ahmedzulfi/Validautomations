use super::common::{cr, ghost_cursor, hairline, hover_fill, icon, idle_for, shadow_color, surface, BeIcon};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Sense, Stroke, Ui};
use glowkit_core::anim::spring;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// One row of a [`SortableStack`].
#[derive(Clone, Debug, PartialEq)]
pub struct SortableTask {
    /// Icon in the tinted tile.
    pub icon: BeIcon,
    /// Tile tint.
    pub color: Color32,
    /// First line.
    pub title: String,
    /// Second, dimmer line.
    pub subtitle: String,
    /// Ticked off.
    pub done: bool,
}

impl SortableTask {
    /// A task with an icon tile, a title and a subtitle.
    pub fn new(icon: BeIcon, color: Color32, title: impl Into<String>, subtitle: impl Into<String>) -> Self {
        Self { icon, color, title: title.into(), subtitle: subtitle.into(), done: false }
    }
}

#[derive(Clone, Debug, Default)]
struct SortState {
    /// (index in the list when grabbed, pointer y minus card top)
    drag: Option<(usize, f32)>,
    prev: Option<Vec<String>>,
}

/// A drag-to-reorder task list. Grab a card by its grip and it lifts off the stack (a touch
/// bigger, with a deeper shadow) while the other cards glide out of its way on springs; drop it
/// and it settles into its new slot. The numbers re-count themselves, the circle ticks a task off,
/// and "Undo reorder" puts the last order back.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BeIcon, SortableStack, SortableTask};
/// let mut tasks = vec![
///     SortableTask::new(BeIcon::Doc, egui::Color32::GOLD, "Collect references", "Find a direction"),
///     SortableTask::new(BeIcon::Rocket, egui::Color32::GREEN, "Ship it", "Polish and release"),
/// ];
/// ui.add(SortableStack::new(&mut tasks).heading("In the studio", "Make room for what matters."));
/// ```
pub struct SortableStack<'a> {
    items: &'a mut Vec<SortableTask>,
    width: f32,
    eyebrow: String,
    title: String,
    autoplay: bool,
}

impl<'a> SortableStack<'a> {
    /// A sortable list editing `items` in place.
    pub fn new(items: &'a mut Vec<SortableTask>) -> Self {
        Self { items, width: 440.0, eyebrow: String::new(), title: String::new(), autoplay: true }
    }
    /// Width of the list.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(240.0);
        self
    }
    /// Small caps eyebrow and a heading shown above the list.
    pub fn heading(mut self, eyebrow: impl Into<String>, title: impl Into<String>) -> Self {
        self.eyebrow = eyebrow.into();
        self.title = title.into();
        self
    }
    /// Drag a card around on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const CARD_H: f32 = 62.0;
const GAP: f32 = 8.0;

impl egui::Widget for SortableStack<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let n = self.items.len();
        let head_h = if self.title.is_empty() { 0.0 } else { 58.0 };
        let list_h = n as f32 * (CARD_H + GAP) - GAP;
        let size = vec2(self.width, head_h + list_h + 34.0);
        let (rect, mut resp) = ui.allocate_exact_size(size, Sense::hover());
        let id = resp.id;
        let top = rect.min.y + head_h;
        let slot_y = |i: usize| top + i as f32 * (CARD_H + GAP);
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let down = ui.input(|i| i.pointer.primary_down());
        let pressed = ui.input(|i| i.pointer.primary_pressed());
        let now = glowkit_core::time(ui);
        let hovering = ui.rect_contains_pointer(rect);
        let st0: SortState = glowkit_core::state(ui, id, SortState::default);
        let idle = idle_for(ui, id, hovering || st0.drag.is_some());
        let auto = self.autoplay && idle > 3.0 && n >= 3;

        // ---- autoplay: a ghost cursor picks up a card and moves it
        let cycle = 5.2;
        let (mut ptr, mut is_down, mut press) = (pointer, down, pressed);
        let mut ghost: Option<(egui::Pos2, f32, bool)> = None;
        if auto {
            glowkit_core::animate(ui);
            let ph = (idle - 3.0) % cycle;
            let k = ((idle - 3.0) / cycle) as u32;
            let (from, to) = if k.is_multiple_of(2) { (0usize, 2usize) } else { (2, 0) };
            let grip_x = rect.min.x + 26.0;
            let a = pos2(grip_x, slot_y(from) + CARD_H / 2.0);
            let b = pos2(grip_x, slot_y(to) + CARD_H / 2.0);
            let rest = pos2(rect.center().x + 60.0, rect.max.y + 10.0);
            let e = |x: f32| glowkit_core::ease(glowkit_core::Easing::InOutCubic, x);
            let (p, held) = if ph < 0.9 {
                (rest.lerp(a, e(ph / 0.9)), false)
            } else if ph < 1.2 {
                (a, ph > 1.05)
            } else if ph < 2.6 {
                (a.lerp(b, e((ph - 1.2) / 1.4)), true)
            } else if ph < 2.9 {
                (b, true)
            } else {
                (b.lerp(rest, e((ph - 2.9) / 1.6)), false)
            };
            let alpha = (ph / 0.3).min(1.0) * ((cycle - ph) / 0.4).min(1.0);
            ptr = Some(p);
            is_down = held;
            press = held && ph < 1.1;
            ghost = Some((p, alpha, held));
        }

        // ---- interaction
        let grip_hit =
            |i: usize, p: egui::Pos2| Rect::from_min_size(pos2(rect.min.x, slot_y(i)), vec2(52.0, CARD_H)).contains(p);
        let card_hit = |i: usize, p: egui::Pos2| {
            Rect::from_min_size(pos2(rect.min.x, slot_y(i)), vec2(self.width, CARD_H)).contains(p)
        };
        let mut drag = st0.drag;
        let mut reorder: Option<(usize, usize)> = None;
        let mut toggled: Option<usize> = None;
        if let Some(p) = ptr {
            if press && drag.is_none() {
                for i in 0..n {
                    if grip_hit(i, p) || (card_hit(i, p) && p.x < rect.max.x - 60.0) {
                        drag = Some((i, p.y - slot_y(i)));
                        break;
                    }
                }
            }
            if pressed && !auto {
                for i in 0..n {
                    let c = pos2(rect.max.x - 22.0, slot_y(i) + CARD_H / 2.0);
                    if c.distance(p) < 12.0 {
                        toggled = Some(i);
                    }
                }
            }
        }
        let drag_y = drag.and_then(|(_, off)| ptr.map(|p| p.y - off));
        let mut target_slot = None;
        if let (Some((from, _)), Some(y)) = (drag, drag_y) {
            let to = (((y - top) / (CARD_H + GAP)).round().max(0.0) as usize).min(n.saturating_sub(1));
            target_slot = Some((from, to));
            if !is_down {
                if from != to {
                    reorder = Some((from, to));
                }
                drag = None;
            }
        }
        if drag.is_some() && !auto {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if let Some(p) = pointer {
            if (0..n).any(|i| grip_hit(i, p)) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
        }
        // undo link
        let undo_rect = Rect::from_min_size(pos2(rect.min.x + 6.0, rect.max.y - 26.0), vec2(120.0, 22.0));
        let undo_resp = ui.interact(undo_rect, id.with("undo"), Sense::click());
        let mut undo = undo_resp.clicked() && st0.prev.is_some();
        if auto && st0.prev.is_none() {
            undo = false;
        }
        if undo_resp.hovered() && st0.prev.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let titles: Vec<String> = self.items.iter().map(|x| x.title.clone()).collect();
        if let Some((from, to)) = reorder {
            let it = self.items.remove(from);
            self.items.insert(to, it);
            resp.mark_changed();
        }
        let mut new_prev = st0.prev.clone();
        if reorder.is_some() {
            new_prev = Some(titles.clone());
        }
        if undo {
            if let Some(prev) = &st0.prev {
                let mut out = Vec::new();
                for tt in prev {
                    if let Some(p) = self.items.iter().position(|x| &x.title == tt) {
                        out.push(self.items.remove(p));
                    }
                }
                out.append(self.items);
                *self.items = out;
                resp.mark_changed();
            }
            new_prev = None;
        }
        if let Some(i) = toggled {
            self.items[i].done = !self.items[i].done;
            resp.mark_changed();
        }
        // remap drag index when order changed underneath
        glowkit_core::with_state(ui, id, SortState::default, |s| {
            s.drag = drag;
            s.prev = new_prev;
        });
        let _ = now;

        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter().clone();
        // heading
        if !self.title.is_empty() {
            painter.text(
                pos2(rect.min.x + 2.0, rect.min.y + 8.0),
                Align2::LEFT_CENTER,
                self.eyebrow.to_uppercase(),
                FontId::proportional(10.5),
                t.muted_foreground,
            );
            painter.text(
                pos2(rect.min.x + 2.0, rect.min.y + 32.0),
                Align2::LEFT_CENTER,
                &self.title,
                FontId::proportional(18.0),
                t.foreground,
            );
            painter.text(
                pos2(rect.max.x - 2.0, rect.min.y + 34.0),
                Align2::RIGHT_CENTER,
                format!("{:02} tasks", n),
                FontId::proportional(12.0),
                t.muted_foreground,
            );
        }
        // where each item should sit given the in-flight drag
        let order_target = |i: usize| -> usize {
            if let Some((from, to)) = target_slot {
                if i == from {
                    to
                } else if from < to && i > from && i <= to {
                    i - 1
                } else if from > to && i >= to && i < from {
                    i + 1
                } else {
                    i
                }
            } else {
                i
            }
        };
        let mut draw: Vec<(usize, f32, f32)> = Vec::new(); // (index, y, lift)
        for i in 0..n {
            let kid = id.with(("sort_item", &self.items[i].title));
            let dragged = drag.map(|d| d.0) == Some(i);
            let ty = if dragged { drag_y.unwrap_or(slot_y(i)) } else { slot_y(order_target(i)) };
            let y = if dragged {
                // keep the spring in step so the drop starts from here
                glowkit_core::set_state(ui, kid.with("y").with("glowkit_spring"), {
                    let mut s = glowkit_core::Spring::new(ty);
                    s.velocity = 0.0;
                    s
                });
                ty
            } else {
                spring(ui, kid.with("y"), ty, 340.0, 26.0)
            };
            let lift = spring(ui, kid.with("lift"), if dragged { 1.0 } else { 0.0 }, 380.0, 22.0);
            draw.push((i, y, lift));
        }
        draw.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        for (i, y, lift) in draw {
            let it = &self.items[i];
            let base = Rect::from_min_size(pos2(rect.min.x, y), vec2(self.width, CARD_H));
            let card = base.expand(lift * 5.0).translate(vec2(0.0, 0.0));
            let r = 16.0;
            paint::soft_shadow(
                &painter,
                card,
                r,
                8.0 + 22.0 * lift,
                vec2(0.0, 2.0 + 10.0 * lift),
                shadow_color(&t, 0.25 + 0.9 * lift),
            );
            let fill = if lift > 0.01 { mix(surface(&t), hover_fill(&t), 0.2 * lift) } else { surface(&t) };
            painter.rect(card, cr(r), fill, hairline(&t), egui::StrokeKind::Inside);
            let cy = card.center().y;
            // grip
            icon(
                &painter,
                BeIcon::Grip,
                pos2(card.min.x + 26.0, cy),
                12.0,
                mix(t.muted_foreground, t.foreground, lift),
                1.3,
            );
            // tile
            let tile = Rect::from_center_size(pos2(card.min.x + 76.0, cy), vec2(38.0, 38.0));
            painter.rect_filled(tile, cr(10.0), with_alpha(it.color, if t.dark { 0.18 } else { 0.14 }));
            icon(&painter, it.icon, tile.center(), 16.0, it.color, 1.6);
            let tx = tile.max.x + 14.0;
            let title_col = if it.done { t.muted_foreground } else { t.foreground };
            let tr =
                painter.text(pos2(tx, cy - 9.0), Align2::LEFT_CENTER, &it.title, FontId::proportional(14.5), title_col);
            if it.done {
                painter.line_segment(
                    [pos2(tr.min.x, tr.center().y + 1.0), pos2(tr.max.x, tr.center().y + 1.0)],
                    Stroke::new(1.2, t.muted_foreground),
                );
            }
            painter.text(
                pos2(tx, cy + 10.0),
                Align2::LEFT_CENTER,
                &it.subtitle,
                FontId::proportional(12.0),
                t.muted_foreground,
            );
            // live number for the slot it is heading to
            let shown = order_target(i);
            painter.text(
                pos2(card.max.x - 44.0, cy),
                Align2::RIGHT_CENTER,
                format!("{:02}", shown + 1),
                FontId::monospace(11.0),
                t.muted_foreground,
            );
            let cc = pos2(card.max.x - 22.0, cy);
            if it.done {
                painter.circle_filled(cc, 8.0, t.success);
                icon(&painter, BeIcon::Check, cc, 9.0, Color32::WHITE, 1.6);
            } else {
                painter.circle_stroke(cc, 7.5, Stroke::new(1.2, with_alpha(t.muted_foreground, 0.6)));
            }
        }
        // undo
        let has_prev = glowkit_core::state(ui, id, SortState::default).prev.is_some();
        let ua = spring(ui, id.with("undo_a"), if has_prev { 1.0 } else { 0.35 }, 200.0, 24.0);
        let uc = with_alpha(t.muted_foreground, ua);
        icon(&painter, BeIcon::Undo, pos2(undo_rect.min.x + 8.0, undo_rect.center().y), 11.0, uc, 1.3);
        painter.text(
            pos2(undo_rect.min.x + 20.0, undo_rect.center().y),
            Align2::LEFT_CENTER,
            "Undo reorder",
            FontId::proportional(12.0),
            uc,
        );
        if let Some((p, a, held)) = ghost {
            ghost_cursor(&painter, &t, p, a, held);
        }
        resp
    }
}
