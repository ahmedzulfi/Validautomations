//! [`KanbanBoard`]: columns of cards with spring-animated drag and drop.

use crate::common::{self, avatar, icon_button, qfont, Icon};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, Id, Pos2, Rect, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2};
use glowkit_core::{color::with_alpha, paint, Spring, Theme};
use std::collections::HashMap;

/// A colored tag on a card.
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    pub text: String,
    pub color: Color32,
}

impl Label {
    /// A label.
    pub fn new(text: impl Into<String>, color: Color32) -> Self {
        Self { text: text.into(), color }
    }
}

/// A card on a [`Board`].
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Card {
    pub id: u64,
    pub title: String,
    pub labels: Vec<Label>,
    /// Assignee initials (rendered as avatars).
    pub assignees: Vec<String>,
    /// Due date text (e.g. "Nov 14").
    pub due: String,
    /// Checklist progress `(done, total)`.
    pub checklist: Option<(u32, u32)>,
    /// Priority stripe color.
    pub priority: Option<Color32>,
}

impl Card {
    /// A card titled `title`.
    pub fn new(id: u64, title: impl Into<String>) -> Self {
        Self { id, title: title.into(), ..Default::default() }
    }
    /// Builder: add a label.
    pub fn label(mut self, text: impl Into<String>, color: Color32) -> Self {
        self.labels.push(Label::new(text, color));
        self
    }
    /// Builder: assignees (initials).
    pub fn assignees(mut self, a: &[&str]) -> Self {
        self.assignees = a.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Builder: due date text.
    pub fn due(mut self, d: impl Into<String>) -> Self {
        self.due = d.into();
        self
    }
    /// Builder: checklist progress.
    pub fn checklist(mut self, done: u32, total: u32) -> Self {
        self.checklist = Some((done, total));
        self
    }
    /// Builder: priority stripe.
    pub fn priority(mut self, c: Color32) -> Self {
        self.priority = Some(c);
        self
    }
}

/// A column (list) of cards.
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub id: u64,
    pub title: String,
    pub color: Color32,
    /// Work-in-progress limit.
    pub wip: Option<usize>,
    pub collapsed: bool,
    pub cards: Vec<Card>,
}

impl Column {
    /// A column.
    pub fn new(id: u64, title: impl Into<String>, color: Color32) -> Self {
        Self { id, title: title.into(), color, wip: None, collapsed: false, cards: Vec::new() }
    }
    /// Builder: WIP limit.
    pub fn wip(mut self, n: usize) -> Self {
        self.wip = Some(n);
        self
    }
    /// Builder: cards.
    pub fn cards(mut self, cards: Vec<Card>) -> Self {
        self.cards = cards;
        self
    }
    /// Is the column over its WIP limit?
    pub fn over_limit(&self) -> bool {
        self.wip.is_some_and(|w| self.cards.len() > w)
    }
}

/// The board: columns of cards. Owned by the caller.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Board {
    pub columns: Vec<Column>,
}

impl Board {
    /// Move card `card` to column index `to_col` at position `index` (clamped).
    /// Returns `false` if the card does not exist.
    pub fn move_card(&mut self, card: u64, to_col: usize, index: usize) -> bool {
        let Some((ci, ki)) = self.find(card) else { return false };
        if to_col >= self.columns.len() {
            return false;
        }
        let c = self.columns[ci].cards.remove(ki);
        let n = self.columns[to_col].cards.len();
        self.columns[to_col].cards.insert(index.min(n), c);
        true
    }
    /// `(column index, card index)` of a card.
    pub fn find(&self, card: u64) -> Option<(usize, usize)> {
        self.columns.iter().enumerate().find_map(|(ci, c)| c.cards.iter().position(|k| k.id == card).map(|ki| (ci, ki)))
    }
    /// A fresh card id.
    pub fn next_id(&self) -> u64 {
        self.columns.iter().flat_map(|c| c.cards.iter().map(|k| k.id)).max().unwrap_or(0) + 1
    }
}

/// Insertion index for a dragged item whose center is at `y`, among items with the given
/// vertical centers (the dragged item excluded).
pub fn insertion_index(centers: &[f32], y: f32) -> usize {
    centers.iter().filter(|c| **c < y).count()
}

#[derive(Clone, Debug)]
struct Drag {
    card: u64,
    grab: Vec2,
    pos: Pos2,
    width: f32,
}

/// Caller-owned state of a [`KanbanBoard`].
#[derive(Clone, Debug, Default)]
pub struct KanbanState {
    drag: Option<Drag>,
    springs: HashMap<u64, (Spring, Spring)>,
    widths: HashMap<u64, f32>,
    scroll: f32,
    adding: Option<(u64, String, bool)>,
    target: Option<(usize, usize)>,
}

/// A Trello / Linear style kanban board: drag cards within and between columns (other cards
/// spring aside to open a gap; dropped cards glide into place), add cards inline, WIP limits,
/// labels, avatars, due dates and collapsible columns.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_canvas::{Board, Card, Column, KanbanBoard, KanbanState};
/// # let mut state = KanbanState::default();
/// let mut board = Board { columns: vec![Column::new(1, "To do", egui::Color32::GRAY).cards(vec![Card::new(1, "Write docs")])] };
/// KanbanBoard::new(&mut board, &mut state).height(480.0).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct KanbanBoard<'a> {
    board: &'a mut Board,
    state: &'a mut KanbanState,
    height: f32,
    column_width: f32,
}

const HEADER: f32 = 46.0;
const GAP: f32 = 8.0;
const PAD: f32 = 8.0;

impl<'a> KanbanBoard<'a> {
    /// A board view.
    pub fn new(board: &'a mut Board, state: &'a mut KanbanState) -> Self {
        Self { board, state, height: 480.0, column_width: 248.0 }
    }
    /// Height of the board.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Width of an expanded column.
    pub fn column_width(mut self, w: f32) -> Self {
        self.column_width = w;
        self
    }

    fn card_height(painter: &egui::Painter, c: &Card, w: f32) -> f32 {
        let g = painter.layout(c.title.clone(), qfont(13.0), Color32::WHITE, w - 24.0);
        let mut h = 12.0 + g.size().y + 12.0;
        if !c.labels.is_empty() {
            h += 24.0;
        }
        if !c.assignees.is_empty() || !c.due.is_empty() || c.checklist.is_some() {
            h += 30.0;
        }
        h
    }

    /// Show the board. Returns `true` when cards moved or were added.
    pub fn show(self, ui: &mut Ui) -> bool {
        let KanbanBoard { board, state: st, height, column_width } = self;
        let t = Theme::of(ui);
        let id = ui.scope_id().with("glowkit_kanban");
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return false;
        }
        let painter = ui.painter_at(rect);
        let dt = glowkit_core::dt(ui);
        let mut changed = false;
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let col_fill = if t.dark { with_alpha(t.foreground, 0.035) } else { t.muted };
        let card_fill = if t.dark { t.popover } else { t.card };

        // ---- column widths (animated collapse)
        let mut xs = Vec::new();
        let mut x = 0.0;
        for c in &board.columns {
            let target = if c.collapsed { 48.0 } else { column_width };
            let w = st.widths.entry(c.id).or_insert(target);
            *w = glowkit_core::anim::approach(*w, target, 14.0, dt);
            if (*w - target).abs() > 0.3 {
                ui.ctx().request_repaint();
            } else {
                *w = target;
            }
            xs.push((x, *w));
            x += *w + 12.0;
        }
        let total = x - 12.0;
        let max_scroll = (total - rect.width()).max(0.0);
        if ui.rect_contains_pointer(rect) && max_scroll > 0.0 {
            let d = ui.input(|i| i.smooth_scroll_delta);
            if d != Vec2::ZERO {
                st.scroll -= if d.x.abs() > d.y.abs() { d.x } else { d.y };
                ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
            }
        }
        st.scroll = st.scroll.clamp(0.0, max_scroll);
        let origin = rect.min + vec2(-st.scroll, 0.0);
        let col_rect = |k: usize| Rect::from_min_size(origin + vec2(xs[k].0, 0.0), vec2(xs[k].1, height));

        // ---- drag target
        let dragging = st.drag.clone();
        st.target = None;
        if let (Some(d), Some(p)) = (&dragging, pointer) {
            let center = p - d.grab + vec2(d.width / 2.0, 0.0);
            let mut best = None;
            for k in 0..board.columns.len() {
                let r = col_rect(k);
                if center.x >= r.left() - 6.0 && center.x <= r.right() + 6.0 {
                    best = Some(k);
                }
            }
            if best.is_none() {
                best = (0..board.columns.len()).min_by(|a, b| (col_rect(*a).center().x - center.x).abs().total_cmp(&(col_rect(*b).center().x - center.x).abs()));
            }
            if let Some(k) = best {
                let col = &board.columns[k];
                let r = col_rect(k);
                let mut y = r.top() + HEADER;
                let mut centers = Vec::new();
                for c in col.cards.iter().filter(|c| c.id != d.card) {
                    let h = Self::card_height(&painter, c, xs[k].1 - PAD * 2.0);
                    centers.push(y + h / 2.0);
                    y += h + GAP;
                }
                let dragged_top = p.y - d.grab.y;
                let idx = if col.collapsed { centers.len() } else { insertion_index(&centers, dragged_top + 20.0) };
                st.target = Some((k, idx));
            }
        }
        let drag_h = dragging.as_ref().and_then(|d| board.find(d.card).map(|(ci, ki)| Self::card_height(&painter, &board.columns[ci].cards[ki], column_width - PAD * 2.0))).unwrap_or(0.0);

        // ---- columns
        let mut start_drag: Option<Drag> = None;
        let mut toggle_collapse: Option<usize> = None;
        let mut start_add: Option<u64> = None;
        for (k, col) in board.columns.iter().enumerate() {
            let r = col_rect(k);
            if !r.intersects(rect) {
                continue;
            }
            let over = col.over_limit();
            let is_target = st.target.is_some_and(|tg| tg.0 == k) && dragging.is_some();
            let tk = ui.ctx().animate_bool_with_time(id.with(("tgt", col.id)), is_target, 0.15);
            let fill = col_fill.lerp_to_gamma(with_alpha(if over { t.destructive } else { t.accent }, 0.10), tk * 0.6 + if over { 0.25 } else { 0.0 });
            painter.rect(r, CornerRadius::same(12), fill, Stroke::new(1.0, t.border.lerp_to_gamma(if over { t.destructive } else { t.accent }, tk * 0.7)), StrokeKind::Inside);
            let w = xs[k].1;
            let collapsed_k = ((column_width - w) / (column_width - 48.0)).clamp(0.0, 1.0);
            if collapsed_k > 0.5 {
                // collapsed strip
                let btn = Rect::from_center_size(pos2(r.center().x, r.top() + 22.0), Vec2::splat(26.0));
                if icon_button(ui, id.with(("col", col.id)), btn, Icon::ChevronRight, false, "Expand").clicked() {
                    toggle_collapse = Some(k);
                }
                painter.circle_filled(pos2(r.center().x, r.top() + 50.0), 4.0, col.color);
                let txt = format!("{}  ·  {}", col.title, col.cards.len());
                glowkit_core::text::paint_rotated(&painter, &txt, pos2(r.center().x, r.top() + 64.0 + txt.len() as f32 * 3.6), qfont(13.0), t.foreground, std::f32::consts::FRAC_PI_2);
                continue;
            }
            let cp = painter.with_clip_rect(r.intersect(rect));
            // header
            let hx = r.left() + 14.0;
            cp.circle_filled(pos2(hx + 4.0, r.top() + 23.0), 4.5, col.color);
            cp.text(pos2(hx + 16.0, r.top() + 23.0), Align2::LEFT_CENTER, &col.title, qfont(14.0), t.foreground);
            let tw = cp.layout_no_wrap(col.title.clone(), qfont(14.0), Color32::WHITE).size().x;
            let count = match col.wip {
                Some(wl) => format!("{} / {}", col.cards.len(), wl),
                None => col.cards.len().to_string(),
            };
            let chip_col = if over { t.destructive } else { t.muted_foreground };
            let cr = common::chip(&cp, pos2(hx + 24.0 + tw, r.top() + 14.0), &count, 11.0, chip_col);
            if over {
                cp.text(pos2(cr.right() + 6.0, r.top() + 23.0), Align2::LEFT_CENTER, "WIP", qfont(10.0), t.destructive);
            }
            let btn = Rect::from_center_size(pos2(r.right() - 22.0, r.top() + 23.0), Vec2::splat(26.0));
            if icon_button(ui, id.with(("col", col.id)), btn, Icon::ChevronLeft, false, "Collapse").clicked() {
                toggle_collapse = Some(k);
            }
            let addb = Rect::from_center_size(pos2(r.right() - 50.0, r.top() + 23.0), Vec2::splat(26.0));
            if icon_button(ui, id.with(("add", col.id)), addb, Icon::Plus, false, "Add card").clicked() {
                start_add = Some(col.id);
            }
            // cards
            let cw = w - PAD * 2.0;
            let mut y = r.top() + HEADER;
            let gap_at = st.target.filter(|tg| tg.0 == k).map(|tg| tg.1);
            let mut vis_index = 0;
            let mut placed_gap = false;
            for card in &col.cards {
                if dragging.as_ref().is_some_and(|d| d.card == card.id) {
                    continue;
                }
                if gap_at == Some(vis_index) && !placed_gap {
                    paint_placeholder(&cp, &t, Rect::from_min_size(pos2(r.left() + PAD, y), vec2(cw, drag_h)));
                    y += drag_h + GAP;
                    placed_gap = true;
                }
                let h = Self::card_height(&cp, card, cw);
                let target = pos2(r.left() + PAD, y) - origin;
                let sp = st.springs.entry(card.id).or_insert_with(|| (Spring::new(target.x).with(420.0, 34.0), Spring::new(target.y).with(420.0, 34.0)));
                sp.0.target = target.x;
                sp.1.target = target.y;
                sp.0.step(dt);
                sp.1.step(dt);
                if !(sp.0.is_settled() && sp.1.is_settled()) {
                    ui.ctx().request_repaint();
                }
                let pos = origin + vec2(sp.0.value, sp.1.value);
                let cr = Rect::from_min_size(pos, vec2(cw, h));
                y += h + GAP;
                vis_index += 1;
                if !cr.intersects(rect) {
                    continue;
                }
                let resp = ui.interact(cr.intersect(r), id.with(("card", card.id)), Sense::click_and_drag());
                let hov = ui.ctx().animate_bool_with_time(id.with(("ch", card.id)), resp.hovered() && dragging.is_none(), 0.12);
                if resp.hovered() && dragging.is_none() {
                    ui.ctx().set_cursor_icon(CursorIcon::Grab);
                }
                paint_card(&cp, &t, card, cr.translate(vec2(0.0, -hov)), card_fill, hov, 0.0);
                if resp.drag_started() {
                    let p = resp.interact_pointer_pos().unwrap_or(cr.min);
                    start_drag = Some(Drag { card: card.id, grab: p - cr.min, pos: p, width: cw });
                }
            }
            if gap_at.is_some() && !placed_gap {
                paint_placeholder(&cp, &t, Rect::from_min_size(pos2(r.left() + PAD, y), vec2(cw, drag_h)));
                y += drag_h + GAP;
            }
            // inline add
            if let Some((cid, text, focus)) = &mut st.adding {
                if *cid == col.id {
                    let ar = Rect::from_min_size(pos2(r.left() + PAD, y), vec2(cw, 64.0));
                    cp.rect(ar, CornerRadius::same(9), card_fill, Stroke::new(1.5, t.accent), StrokeKind::Inside);
                    let te = ui.put(ar.shrink2(vec2(10.0, 8.0)), TextEdit::multiline(text).frame(egui::Frame::NONE).font(qfont(13.0)).hint_text("Card title… (Enter to add)").desired_rows(2));
                    if *focus {
                        te.request_focus();
                        *focus = false;
                    }
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift);
                    let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
                    if enter || esc || te.lost_focus() {
                        let title = text.trim().to_owned();
                        let cid = *cid;
                        st.adding = None;
                        if !esc && !title.is_empty() {
                            let nid = board_next_id(board);
                            let ci = k;
                            // spawn the spring at the input position so it settles in nicely
                            st.springs.insert(nid, (Spring::new(ar.min.x - origin.x).with(420.0, 34.0), Spring::new(ar.min.y - origin.y + 8.0).with(420.0, 34.0)));
                            pending_add(ui, id, ci, cid, nid, title);
                        }
                    }
                    y += 72.0;
                }
            }
            // "add card" footer
            if st.adding.as_ref().is_none_or(|a| a.0 != col.id) && dragging.is_none() {
                let fr = Rect::from_min_size(pos2(r.left() + PAD, y), vec2(cw, 32.0));
                if fr.bottom() < r.bottom() - 4.0 {
                    let resp = ui.interact(fr, id.with(("foot", col.id)), Sense::click());
                    let h = ui.ctx().animate_bool_with_time(id.with(("fh", col.id)), resp.hovered(), 0.12);
                    cp.rect_filled(fr, CornerRadius::same(8), with_alpha(t.foreground, 0.05 * h));
                    common::paint_icon(&cp, Rect::from_center_size(pos2(fr.left() + 16.0, fr.center().y), Vec2::splat(12.0)), Icon::Plus, t.muted_foreground.lerp_to_gamma(t.foreground, h));
                    cp.text(pos2(fr.left() + 30.0, fr.center().y), Align2::LEFT_CENTER, "Add card", qfont(12.5), t.muted_foreground.lerp_to_gamma(t.foreground, h));
                    if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                        start_add = Some(col.id);
                    }
                }
            }
        }
        // apply pending adds (deferred so we do not mutate while iterating)
        if let Some((ci, cid, nid, title)) = take_pending_add(ui, id) {
            if let Some(col) = board.columns.get_mut(ci).filter(|c| c.id == cid) {
                col.cards.push(Card::new(nid, title));
                changed = true;
            }
        }
        if let Some(cid) = start_add {
            st.adding = Some((cid, String::new(), true));
        }
        if let Some(k) = toggle_collapse {
            board.columns[k].collapsed = !board.columns[k].collapsed;
        }
        if let Some(d) = start_drag {
            st.drag = Some(d);
        }

        // ---- dragged card on top
        if let Some(mut d) = st.drag.clone() {
            if let Some(p) = pointer {
                d.pos = p;
            }
            let released = ui.input(|i| !i.pointer.primary_down());
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            if let Some((ci, ki)) = board.find(d.card) {
                let card = board.columns[ci].cards[ki].clone();
                let h = Self::card_height(&painter, &card, d.width);
                let cr = Rect::from_min_size(d.pos - d.grab, vec2(d.width, h));
                let lp = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, id.with("drag"))).with_clip_rect(rect.expand(30.0));
                paint_card(&lp, &t, &card, cr, card_fill, 1.0, 1.0);
                if released {
                    if let Some((tc, ti)) = st.target {
                        let from = (ci, ki);
                        board.move_card(d.card, tc, ti);
                        if from != board.find(d.card).unwrap_or(from) {
                            changed = true;
                        }
                    }
                    // the drop animation: start the spring from where the card was released
                    let rel = cr.min - origin;
                    st.springs.insert(d.card, (Spring::new(rel.x).with(380.0, 30.0), Spring::new(rel.y).with(380.0, 30.0)));
                    st.drag = None;
                    st.target = None;
                    ui.ctx().request_repaint();
                } else {
                    st.drag = Some(d);
                }
            } else {
                st.drag = None;
            }
        }
        // edge fades when scrolled
        if max_scroll > 0.0 {
            let bg = ui.visuals().panel_fill;
            let _ = bg;
            let bar_w = rect.width() * rect.width() / total;
            let bx = rect.left() + (rect.width() - bar_w) * (st.scroll / max_scroll);
            painter.rect_filled(Rect::from_min_size(pos2(bx, rect.bottom() - 4.0), vec2(bar_w, 3.0)), CornerRadius::same(2), with_alpha(t.muted_foreground, 0.35));
        }
        changed
    }
}

fn board_next_id(b: &Board) -> u64 {
    b.next_id()
}

fn pending_add(ui: &Ui, id: Id, ci: usize, cid: u64, nid: u64, title: String) {
    ui.data_mut(|d| d.insert_temp(id.with("pending_add"), Some((ci, cid, nid, title))));
}

fn take_pending_add(ui: &Ui, id: Id) -> Option<(usize, u64, u64, String)> {
    ui.data_mut(|d| d.get_temp_mut_or_default::<Option<(usize, u64, u64, String)>>(id.with("pending_add")).take())
}

fn paint_placeholder(p: &egui::Painter, t: &Theme, r: Rect) {
    p.rect(r, CornerRadius::same(9), with_alpha(t.accent, 0.07), Stroke::NONE, StrokeKind::Inside);
    let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    let rr = common::round_corners(&pts, 9.0);
    common::dashed_polyline(p, &rr, 6.0, 4.0, 0.0, Stroke::new(1.3, with_alpha(t.accent, 0.6)));
}

fn paint_card(p: &egui::Painter, t: &Theme, c: &Card, r: Rect, fill: Color32, hov: f32, lifted: f32) {
    let shadow_a = if t.dark { 90.0 + 80.0 * lifted } else { 18.0 + 30.0 * lifted };
    paint::soft_shadow(p, r.translate(vec2(0.0, 1.0 + 6.0 * lifted)), 9.0, 6.0 + 4.0 * hov + 18.0 * lifted, vec2(0.0, 2.0 + 6.0 * lifted), Color32::from_black_alpha(shadow_a as u8));
    let border = t.border.lerp_to_gamma(t.muted_foreground, 0.35 * hov).lerp_to_gamma(t.accent, lifted * 0.8);
    p.rect(r, CornerRadius::same(9), fill, Stroke::new(1.0, border), StrokeKind::Inside);
    if let Some(pc) = c.priority {
        p.rect_filled(Rect::from_min_size(r.min + vec2(0.0, 10.0), vec2(3.0, r.height() - 20.0)), CornerRadius::same(2), pc);
    }
    let mut y = r.top() + 12.0;
    if !c.labels.is_empty() {
        let mut x = r.left() + 12.0;
        for l in &c.labels {
            let cr = common::chip(p, pos2(x, y), &l.text, 10.5, l.color);
            x = cr.right() + 5.0;
        }
        y += 24.0;
    }
    let g = p.layout(c.title.clone(), qfont(13.0), t.foreground, r.width() - 24.0);
    let gh = g.size().y;
    p.galley(pos2(r.left() + 12.0, y), g, t.foreground);
    y += gh + 10.0;
    if !c.assignees.is_empty() || !c.due.is_empty() || c.checklist.is_some() {
        let cy = y + 10.0;
        let mut x = r.left() + 12.0;
        if !c.due.is_empty() {
            let late = c.due.starts_with('!');
            let txt = c.due.trim_start_matches('!');
            let col = if late { t.destructive } else { t.muted_foreground };
            p.circle_stroke(pos2(x + 5.0, cy), 5.0, Stroke::new(1.2, col));
            p.line_segment([pos2(x + 5.0, cy), pos2(x + 5.0, cy - 3.0)], Stroke::new(1.2, col));
            p.line_segment([pos2(x + 5.0, cy), pos2(x + 7.5, cy)], Stroke::new(1.2, col));
            let tr = p.text(pos2(x + 14.0, cy), Align2::LEFT_CENTER, txt, qfont(11.0), col);
            x = tr.right() + 12.0;
        }
        if let Some((d, n)) = c.checklist {
            let col = if d == n { t.success } else { t.muted_foreground };
            common::paint_icon(p, Rect::from_center_size(pos2(x + 5.0, cy), Vec2::splat(11.0)), Icon::Check, col);
            p.text(pos2(x + 13.0, cy), Align2::LEFT_CENTER, format!("{d}/{n}"), qfont(11.0), col);
        }
        let mut ax = r.right() - 20.0;
        for a in c.assignees.iter().rev() {
            avatar(p, pos2(ax, cy), 10.0, a, common::color_for(t, a), fill);
            ax -= 15.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board() -> Board {
        Board {
            columns: vec![
                Column::new(1, "A", Color32::RED).cards(vec![Card::new(1, "one"), Card::new(2, "two"), Card::new(3, "three")]),
                Column::new(2, "B", Color32::BLUE).wip(1).cards(vec![Card::new(4, "four")]),
            ],
        }
    }

    #[test]
    fn move_between_columns() {
        let mut b = board();
        assert!(b.move_card(2, 1, 0));
        assert_eq!(b.columns[0].cards.len(), 2);
        assert_eq!(b.columns[1].cards[0].id, 2);
        assert!(b.columns[1].over_limit());
        assert!(!b.move_card(99, 0, 0));
    }

    #[test]
    fn move_within_column() {
        let mut b = board();
        b.move_card(1, 0, 99);
        let ids: Vec<u64> = b.columns[0].cards.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![2, 3, 1]);
        assert_eq!(b.next_id(), 5);
    }

    #[test]
    fn insertion() {
        assert_eq!(insertion_index(&[10.0, 50.0, 90.0], 0.0), 0);
        assert_eq!(insertion_index(&[10.0, 50.0, 90.0], 60.0), 2);
        assert_eq!(insertion_index(&[10.0, 50.0, 90.0], 200.0), 3);
    }
}
