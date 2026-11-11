//! [`DataGrid`]: a virtualized spreadsheet-like grid (frozen header and first column, range
//! selection, inline editing, column resize, TSV copy / paste).

use crate::common::{self, qfont, qmono};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, Event, Key, Rect, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2};
use glowkit_core::{color::with_alpha, Theme};

/// How a column renders its cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CellKind {
    #[default]
    Text,
    /// Right-aligned number with thousands separators.
    Number,
    /// Right-aligned `$` amount.
    Currency,
    /// `0..100` rendered with a mini bar.
    Percent,
    /// Colored status pill.
    Status,
    /// Monospace text (ids, dates).
    Mono,
}

/// A grid column.
#[derive(Clone, Debug, PartialEq)]
pub struct GridColumn {
    pub name: String,
    pub width: f32,
    pub kind: CellKind,
}

impl GridColumn {
    /// A column.
    pub fn new(name: impl Into<String>, width: f32, kind: CellKind) -> Self {
        Self { name: name.into(), width, kind }
    }
}

/// Grid data: columns and rows of string cells. Owned by the caller.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct GridData {
    pub columns: Vec<GridColumn>,
    pub rows: Vec<Vec<String>>,
}

impl GridData {
    /// Cell text (empty when out of range).
    pub fn cell(&self, r: usize, c: usize) -> &str {
        self.rows.get(r).and_then(|row| row.get(c)).map(|s| s.as_str()).unwrap_or("")
    }
    /// Set a cell (grows the row if needed).
    pub fn set(&mut self, r: usize, c: usize, v: impl Into<String>) {
        if let Some(row) = self.rows.get_mut(r) {
            if row.len() <= c {
                row.resize(c + 1, String::new());
            }
            row[c] = v.into();
        }
    }
    /// Selection as tab-separated values.
    pub fn to_tsv(&self, sel: CellRange) -> String {
        let mut out = String::new();
        for r in sel.r0..=sel.r1 {
            let line: Vec<&str> = (sel.c0..=sel.c1).map(|c| self.cell(r, c)).collect();
            out.push_str(&line.join("\t"));
            if r != sel.r1 {
                out.push('\n');
            }
        }
        out
    }
    /// Paste tab-separated values with the top-left at `(r, c)`; returns the pasted range.
    pub fn paste_tsv(&mut self, r: usize, c: usize, tsv: &str) -> CellRange {
        let mut r1 = r;
        let mut c1 = c;
        for (dr, line) in tsv.lines().enumerate() {
            if r + dr >= self.rows.len() {
                break;
            }
            for (dc, v) in line.split('\t').enumerate() {
                if c + dc < self.columns.len() {
                    self.set(r + dr, c + dc, v);
                    c1 = c1.max(c + dc);
                }
            }
            r1 = r + dr;
        }
        CellRange { r0: r, c0: c, r1, c1 }
    }
}

/// An inclusive, normalized rectangular range of cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRange {
    pub r0: usize,
    pub c0: usize,
    pub r1: usize,
    pub c1: usize,
}

impl CellRange {
    /// Range spanning two corner cells (any order).
    pub fn new(a: (usize, usize), b: (usize, usize)) -> Self {
        Self { r0: a.0.min(b.0), c0: a.1.min(b.1), r1: a.0.max(b.0), c1: a.1.max(b.1) }
    }
    /// Does it contain `(r, c)`?
    pub fn contains(&self, r: usize, c: usize) -> bool {
        r >= self.r0 && r <= self.r1 && c >= self.c0 && c <= self.c1
    }
    /// Number of cells.
    pub fn count(&self) -> usize {
        (self.r1 - self.r0 + 1) * (self.c1 - self.c0 + 1)
    }
}

/// Parse a cell as a number (ignores `$`, `,` and `%`).
pub fn parse_number(s: &str) -> Option<f64> {
    let cleaned: String = s.chars().filter(|c| !matches!(c, '$' | ',' | '%' | ' ')).collect();
    if cleaned.is_empty() {
        None
    } else {
        cleaned.parse().ok()
    }
}

/// Format with thousands separators.
pub fn group_thousands(v: f64, decimals: usize) -> String {
    let s = format!("{:.*}", decimals, v.abs());
    let (int, frac) = s.split_once('.').map(|(a, b)| (a.to_owned(), format!(".{b}"))).unwrap_or((s.clone(), String::new()));
    let mut out = String::new();
    for (i, ch) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    format!("{}{}{}", if v < 0.0 { "-" } else { "" }, out, frac)
}

/// Caller-owned state of a [`DataGrid`].
#[derive(Clone, Debug)]
pub struct GridState {
    /// Anchor cell of the selection.
    pub anchor: (usize, usize),
    /// Moving end of the selection (the active cell).
    pub cursor: (usize, usize),
    /// Scroll offset in points.
    pub scroll: Vec2,
    editing: Option<(usize, usize, String, bool)>,
    resizing: Option<(usize, f32, f32)>,
    selecting: bool,
    scroll_drag: Option<(bool, f32, f32)>,
}

impl Default for GridState {
    fn default() -> Self {
        Self { anchor: (0, 0), cursor: (0, 0), scroll: Vec2::ZERO, editing: None, resizing: None, selecting: false, scroll_drag: None }
    }
}

impl GridState {
    /// The selected range.
    pub fn selection(&self) -> CellRange {
        CellRange::new(self.anchor, self.cursor)
    }
}

/// A virtualized data grid / spreadsheet (Glide Data Grid / AG Grid / Excel pattern). Only
/// visible cells are painted, so it handles 100k rows. Frozen header row, row numbers and
/// first column; click-drag range selection; type or double-click to edit; Enter / Tab / arrow
/// navigation; resize columns by dragging header borders; TSV copy / paste; live sum / average.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_canvas::{CellKind, DataGrid, GridColumn, GridData, GridState};
/// # let mut state = GridState::default();
/// let mut data = GridData { columns: vec![GridColumn::new("Name", 160.0, CellKind::Text)], rows: vec![vec!["Ada".into()]] };
/// DataGrid::new(&mut data, &mut state).height(480.0).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct DataGrid<'a> {
    data: &'a mut GridData,
    state: &'a mut GridState,
    height: f32,
    row_h: f32,
    frozen: usize,
}

const HEAD_H: f32 = 34.0;
const GUTTER: f32 = 56.0;
const STATUS: f32 = 30.0;

impl<'a> DataGrid<'a> {
    /// A grid editing `data`.
    pub fn new(data: &'a mut GridData, state: &'a mut GridState) -> Self {
        Self { data, state, height: 480.0, row_h: 30.0, frozen: 1 }
    }
    /// Height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Row height.
    pub fn row_height(mut self, h: f32) -> Self {
        self.row_h = h;
        self
    }
    /// Number of frozen data columns on the left.
    pub fn frozen_columns(mut self, n: usize) -> Self {
        self.frozen = n;
        self
    }

    /// Show the grid. Returns `true` when a cell changed.
    pub fn show(self, ui: &mut Ui) -> bool {
        let DataGrid { data, state: st, height, row_h, frozen } = self;
        let t = Theme::of(ui);
        let id = ui.scope_id().with("glowkit_grid");
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        if !ui.is_rect_visible(rect) || data.columns.is_empty() {
            return false;
        }
        let mut changed = false;
        let nrows = data.rows.len();
        let ncols = data.columns.len();
        let frozen = frozen.min(ncols);
        let painter = ui.painter_at(rect);
        painter.rect(rect, CornerRadius::same(10), t.card, Stroke::new(1.0, t.border), StrokeKind::Inside);
        let body = Rect::from_min_max(pos2(rect.left() + GUTTER, rect.top() + HEAD_H), pos2(rect.right() - 10.0, rect.bottom() - STATUS - 10.0));
        let frozen_w: f32 = data.columns[..frozen].iter().map(|c| c.width).sum();
        let scroll_area = Rect::from_min_max(pos2(body.left() + frozen_w, body.top()), body.max);
        let total_w: f32 = data.columns[frozen..].iter().map(|c| c.width).sum();
        let total_h = nrows as f32 * row_h;
        let max_scroll = vec2((total_w - scroll_area.width()).max(0.0), (total_h - body.height()).max(0.0));
        let resp = ui.interact(Rect::from_min_max(rect.min, pos2(rect.right(), body.bottom())), id, Sense::click_and_drag());
        if resp.clicked() || resp.drag_started() {
            resp.request_focus();
        }
        let focused = resp.has_focus() || st.editing.is_some();
        if resp.has_focus() {
            common::lock_focus(ui, id);
        }
        // scrolling
        if ui.rect_contains_pointer(rect) {
            let d = ui.input(|i| i.smooth_scroll_delta);
            if d != Vec2::ZERO {
                st.scroll -= d;
                ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
            }
        }
        st.scroll = st.scroll.clamp(Vec2::ZERO, max_scroll);
        let pointer = ui.input(|i| i.pointer.hover_pos());

        let mut geo = Geo::new(data, st, body, scroll_area, frozen, row_h);
        // ---- column resize handles (header borders)
        let mut border_hover = None;
        for c in 0..ncols {
            let x = geo.col_x(c) + data.columns[c].width;
            if (c >= frozen && (x < scroll_area.left() || x > body.right())) || (c < frozen && x > body.right()) {
                continue;
            }
            let hr = Rect::from_center_size(pos2(x, rect.top() + HEAD_H / 2.0), vec2(8.0, HEAD_H));
            let r = ui.interact(hr, id.with(("rs", c)), Sense::drag());
            if r.hovered() || r.dragged() {
                border_hover = Some(c);
                ui.ctx().set_cursor_icon(CursorIcon::ResizeColumn);
            }
            if r.drag_started() {
                st.resizing = Some((c, data.columns[c].width, pointer.map(|p| p.x).unwrap_or(x)));
            }
            if r.double_clicked() {
                // auto-fit to content of the first rows
                let font = qfont(12.5);
                let mut w = painter.layout_no_wrap(data.columns[c].name.clone(), font.clone(), Color32::WHITE).size().x + 34.0;
                for row in data.rows.iter().take(300) {
                    if let Some(v) = row.get(c) {
                        w = w.max(painter.layout_no_wrap(v.clone(), font.clone(), Color32::WHITE).size().x + 24.0);
                    }
                }
                data.columns[c].width = w.clamp(48.0, 480.0);
            }
        }
        if let Some((c, w0, x0)) = st.resizing {
            if let Some(p) = pointer {
                data.columns[c].width = (w0 + p.x - x0).clamp(40.0, 600.0);
            }
            ui.ctx().set_cursor_icon(CursorIcon::ResizeColumn);
            if ui.input(|i| !i.pointer.primary_down()) {
                st.resizing = None;
            }
        }

        // ---- selection by mouse
        geo = Geo::new(data, st, body, scroll_area, frozen, row_h);
        let shift = ui.input(|i| i.modifiers.shift);
        if st.resizing.is_none() && border_hover.is_none() {
            if let Some(p) = resp.interact_pointer_pos().filter(|_| resp.is_pointer_button_down_on()) {
                let press = ui.input(|i| i.pointer.press_origin()).unwrap_or(p);
                if resp.drag_started() || ui.input(|i| i.pointer.primary_pressed()) {
                    st.selecting = true;
                    if press.y < body.top() && press.x >= body.left() {
                        // header click: select column
                        if let Some((_, c)) = geo.cell_at(pos2(press.x, body.top() + 1.0)) {
                            st.anchor = (0, c);
                            st.cursor = (nrows.saturating_sub(1), c);
                            st.selecting = false;
                        }
                    } else if press.x < body.left() && press.y >= body.top() {
                        if let Some((r, _)) = geo.cell_at(pos2(body.left() + 1.0, press.y)) {
                            st.anchor = (r, 0);
                            st.cursor = (r, ncols - 1);
                            st.selecting = false;
                        }
                    } else if press.x < body.left() && press.y < body.top() {
                        st.anchor = (0, 0);
                        st.cursor = (nrows.saturating_sub(1), ncols - 1);
                        st.selecting = false;
                    } else if let Some(cell) = geo.cell_at(press) {
                        if st.editing.as_ref().is_some_and(|e| (e.0, e.1) != cell) {
                            commit(data, st, &mut changed);
                        }
                        if !shift {
                            st.anchor = cell;
                        }
                        st.cursor = cell;
                    }
                } else if st.selecting {
                    if let Some(cell) = geo.cell_at(p) {
                        st.cursor = cell;
                    }
                    // auto-scroll near edges
                    let edge = 24.0;
                    let mut d = Vec2::ZERO;
                    if p.y > body.bottom() - edge {
                        d.y = 8.0;
                    } else if p.y < body.top() + edge && p.y > body.top() {
                        d.y = -8.0;
                    }
                    if p.x > body.right() - edge {
                        d.x = 8.0;
                    }
                    st.scroll = (st.scroll + d).clamp(Vec2::ZERO, max_scroll);
                    if d != Vec2::ZERO {
                        ui.ctx().request_repaint();
                    }
                }
            }
        }
        if ui.input(|i| !i.pointer.primary_down()) {
            st.selecting = false;
        }
        if resp.double_clicked() {
            if let Some(cell) = pointer.and_then(|p| geo.cell_at(p)) {
                if pointer.is_some_and(|p| p.y > body.top() && p.x > body.left()) {
                    st.editing = Some((cell.0, cell.1, data.cell(cell.0, cell.1).to_owned(), true));
                }
            }
        }

        // ---- keyboard
        let mut moved = false;
        if focused && st.editing.is_none() {
            let (events, mods) = ui.input(|i| (i.events.clone(), i.modifiers));
            let sel = st.selection();
            for e in &events {
                match e {
                    Event::Key { key, pressed: true, modifiers, .. } => {
                        let (dr, dc): (i64, i64) = match key {
                            Key::ArrowUp => (-1, 0),
                            Key::ArrowDown => (1, 0),
                            Key::ArrowLeft => (0, -1),
                            Key::ArrowRight => (0, 1),
                            Key::Tab => (0, if modifiers.shift { -1 } else { 1 }),
                            Key::Enter => (if modifiers.shift { -1 } else { 1 }, 0),
                            Key::PageDown => (((body.height() / row_h) as i64).max(1), 0),
                            Key::PageUp => (-((body.height() / row_h) as i64).max(1), 0),
                            _ => (0, 0),
                        };
                        if *key == Key::F2 {
                            st.editing = Some((st.cursor.0, st.cursor.1, data.cell(st.cursor.0, st.cursor.1).to_owned(), true));
                        }
                        if matches!(key, Key::Delete | Key::Backspace) {
                            for r in sel.r0..=sel.r1 {
                                for c in sel.c0..=sel.c1 {
                                    data.set(r, c, "");
                                }
                            }
                            changed = true;
                        }
                        if modifiers.command && *key == Key::A {
                            st.anchor = (0, 0);
                            st.cursor = (nrows.saturating_sub(1), ncols - 1);
                        }
                        if (dr, dc) != (0, 0) && !modifiers.command {
                            let r = (st.cursor.0 as i64 + dr).clamp(0, nrows as i64 - 1) as usize;
                            let c = (st.cursor.1 as i64 + dc).clamp(0, ncols as i64 - 1) as usize;
                            st.cursor = (r, c);
                            if !(modifiers.shift && matches!(key, Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight)) {
                                st.anchor = st.cursor;
                            }
                            moved = true;
                        }
                    }
                    Event::Copy | Event::Cut => {
                        ui.ctx().copy_text(data.to_tsv(sel));
                        if matches!(e, Event::Cut) {
                            for r in sel.r0..=sel.r1 {
                                for c in sel.c0..=sel.c1 {
                                    data.set(r, c, "");
                                }
                            }
                            changed = true;
                        }
                    }
                    Event::Paste(text) => {
                        let rng = data.paste_tsv(sel.r0, sel.c0, text);
                        st.anchor = (rng.r0, rng.c0);
                        st.cursor = (rng.r1, rng.c1);
                        changed = true;
                    }
                    Event::Text(txt) if !mods.command && !txt.trim().is_empty() => {
                        // typing starts editing, replacing the cell
                        st.editing = Some((st.cursor.0, st.cursor.1, txt.clone(), true));
                    }
                    _ => {}
                }
            }
        }
        if moved {
            // keep the active cell visible
            let (r, c) = st.cursor;
            let y0 = r as f32 * row_h;
            if y0 < st.scroll.y {
                st.scroll.y = y0;
            } else if y0 + row_h > st.scroll.y + body.height() {
                st.scroll.y = y0 + row_h - body.height();
            }
            if c >= frozen {
                let x0: f32 = data.columns[frozen..c].iter().map(|k| k.width).sum();
                let x1 = x0 + data.columns[c].width;
                if x0 < st.scroll.x {
                    st.scroll.x = x0;
                } else if x1 > st.scroll.x + scroll_area.width() {
                    st.scroll.x = x1 - scroll_area.width();
                }
            }
            st.scroll = st.scroll.clamp(Vec2::ZERO, max_scroll);
        }

        // ---- paint body (virtualized)
        geo = Geo::new(data, st, body, scroll_area, frozen, row_h);
        let sel = st.selection();
        let first = (st.scroll.y / row_h).floor() as usize;
        let last = (((st.scroll.y + body.height()) / row_h).ceil() as usize).min(nrows);
        let hover_row = pointer.filter(|p| body.contains(*p)).and_then(|p| geo.cell_at(p)).map(|c| c.0);
        for pass in 0..2 {
            // pass 0: scrolling columns, pass 1: frozen columns on top
            let clip = if pass == 0 { scroll_area } else { Rect::from_min_max(body.min, pos2(scroll_area.left(), body.bottom())) };
            let p = painter.with_clip_rect(clip);
            let cols: Vec<usize> = if pass == 0 { (frozen..ncols).collect() } else { (0..frozen).collect() };
            if pass == 1 {
                p.rect_filled(clip, CornerRadius::ZERO, t.card);
            }
            for r in first..last {
                let y = geo.row_y(r);
                let row_rect = Rect::from_min_max(pos2(clip.left(), y), pos2(clip.right(), y + row_h));
                if r % 2 == 1 {
                    p.rect_filled(row_rect, CornerRadius::ZERO, with_alpha(t.foreground, 0.018));
                }
                if hover_row == Some(r) {
                    p.rect_filled(row_rect, CornerRadius::ZERO, with_alpha(t.foreground, 0.035));
                }
                for &c in &cols {
                    let x = geo.col_x(c);
                    let w = data.columns[c].width;
                    if x + w < clip.left() || x > clip.right() {
                        continue;
                    }
                    let cr = Rect::from_min_size(pos2(x, y), vec2(w, row_h));
                    if sel.contains(r, c) && sel.count() > 1 {
                        p.rect_filled(cr, CornerRadius::ZERO, with_alpha(t.accent, 0.10));
                    }
                    if st.editing.as_ref().is_some_and(|e| e.0 == r && e.1 == c) {
                        continue;
                    }
                    paint_cell(&p, &t, data.columns[c].kind, data.cell(r, c), cr);
                    p.line_segment([cr.right_top(), cr.right_bottom()], Stroke::new(1.0, with_alpha(t.border, 0.5)));
                }
                p.line_segment([row_rect.left_bottom(), row_rect.right_bottom()], Stroke::new(1.0, with_alpha(t.border, 0.5)));
            }
            if pass == 1 && frozen > 0 {
                // shadow on the frozen edge when scrolled horizontally
                let k = (st.scroll.x / 30.0).clamp(0.0, 1.0);
                if k > 0.0 {
                    let x = scroll_area.left();
                    for i in 0..8 {
                        let a = (1.0 - i as f32 / 8.0) * 0.12 * k;
                        painter.line_segment([pos2(x + i as f32, body.top()), pos2(x + i as f32, body.bottom())], Stroke::new(1.0, Color32::from_black_alpha((a * 255.0) as u8)));
                    }
                }
                painter.line_segment([pos2(scroll_area.left(), body.top()), pos2(scroll_area.left(), body.bottom())], Stroke::new(1.0, t.border));
            }
        }
        // selection outline + active cell + fill handle
        let range_rect = |rg: CellRange| -> Rect {
            let x0 = geo.col_x(rg.c0);
            let x1 = geo.col_x(rg.c1) + data.columns[rg.c1].width;
            Rect::from_min_max(pos2(x0, geo.row_y(rg.r0)), pos2(x1, geo.row_y(rg.r1) + row_h))
        };
        let sp = painter.with_clip_rect(body.expand(1.0));
        let rr = range_rect(sel);
        sp.rect_stroke(rr, CornerRadius::same(2), Stroke::new(1.5, t.accent), StrokeKind::Middle);
        let ac = range_rect(CellRange::new(st.cursor, st.cursor));
        sp.rect_stroke(ac.shrink(1.0), CornerRadius::same(2), Stroke::new(2.0, t.accent), StrokeKind::Inside);
        sp.rect(Rect::from_center_size(rr.right_bottom(), Vec2::splat(7.0)), CornerRadius::same(1), t.accent, Stroke::new(1.5, t.card), StrokeKind::Outside);

        // ---- editing
        if let Some((r, c, mut text, focus)) = st.editing.take() {
            let cr = range_rect(CellRange::new((r, c), (r, c)));
            if body.intersects(cr) {
                painter.rect(cr.expand(1.0), CornerRadius::same(2), t.popover, Stroke::new(2.0, t.accent), StrokeKind::Inside);
                let te = ui.put(cr.shrink2(vec2(8.0, 0.0)), TextEdit::singleline(&mut text).frame(egui::Frame::NONE).font(qfont(12.5)).align(Align2::LEFT_CENTER));
                if focus {
                    te.request_focus();
                    if let Some(mut s) = TextEdit::load_state(ui.ctx(), te.id) {
                        let end = egui::text::CCursor::new(text.chars().count());
                        s.cursor.set_char_range(Some(egui::text::CCursorRange::one(end)));
                        s.store(ui.ctx(), te.id);
                    }
                }
                let (enter, tab, esc, shift) = ui.input(|i| (i.key_pressed(Key::Enter), i.key_pressed(Key::Tab), i.key_pressed(Key::Escape), i.modifiers.shift));
                if esc {
                    resp.request_focus();
                } else if enter || tab || (te.lost_focus() && !focus) {
                    if data.cell(r, c) != text {
                        data.set(r, c, text);
                        changed = true;
                    }
                    if enter {
                        let nr = if shift { r.saturating_sub(1) } else { (r + 1).min(nrows.saturating_sub(1)) };
                        st.cursor = (nr, c);
                    } else if tab {
                        st.cursor = (r, if shift { c.saturating_sub(1) } else { (c + 1).min(ncols - 1) });
                    }
                    st.anchor = st.cursor;
                    resp.request_focus();
                } else {
                    st.editing = Some((r, c, text, false));
                    common::lock_focus(ui, te.id);
                }
            }
        }

        // ---- header + gutter (frozen)
        let header = Rect::from_min_max(rect.min, pos2(rect.right(), rect.top() + HEAD_H));
        painter.rect_filled(header.shrink(1.0), CornerRadius { nw: 9, ne: 9, sw: 0, se: 0 }, t.muted);
        painter.line_segment([pos2(rect.left(), body.top()), pos2(rect.right(), body.top())], Stroke::new(1.0, t.border));
        for pass in 0..2 {
            let clip = if pass == 0 { Rect::from_min_max(pos2(scroll_area.left(), header.top()), pos2(body.right(), header.bottom())) } else { Rect::from_min_max(pos2(body.left(), header.top()), pos2(scroll_area.left(), header.bottom())) };
            let hp = painter.with_clip_rect(clip);
            let cols: Vec<usize> = if pass == 0 { (frozen..ncols).collect() } else { (0..frozen).collect() };
            for c in cols {
                let x = geo.col_x(c);
                let w = data.columns[c].width;
                let hr = Rect::from_min_size(pos2(x, header.top()), vec2(w, HEAD_H));
                if c >= sel.c0 && c <= sel.c1 {
                    hp.rect_filled(hr.shrink2(vec2(0.0, 1.0)), CornerRadius::ZERO, with_alpha(t.accent, 0.12));
                    hp.line_segment([hr.left_bottom(), hr.right_bottom()], Stroke::new(2.0, t.accent));
                }
                let col = &data.columns[c];
                let right = matches!(col.kind, CellKind::Number | CellKind::Currency | CellKind::Percent);
                let label = common::elide(&hp, &col.name, qfont(12.0), w - 20.0);
                if right {
                    hp.text(pos2(hr.right() - 10.0, hr.center().y), Align2::RIGHT_CENTER, label, qfont(12.0), t.muted_foreground);
                } else {
                    hp.text(pos2(hr.left() + 10.0, hr.center().y), Align2::LEFT_CENTER, label, qfont(12.0), t.muted_foreground);
                }
                let bh = border_hover == Some(c) || st.resizing.is_some_and(|r| r.0 == c);
                hp.line_segment([hr.right_top() + vec2(0.0, if bh { 4.0 } else { 9.0 }), hr.right_bottom() - vec2(0.0, if bh { 4.0 } else { 9.0 })], Stroke::new(if bh { 2.5 } else { 1.0 }, if bh { t.accent } else { t.border }));
            }
        }
        // gutter
        let gutter = Rect::from_min_max(pos2(rect.left(), body.top()), pos2(body.left(), body.bottom()));
        let gp = painter.with_clip_rect(gutter);
        gp.rect_filled(gutter, CornerRadius::ZERO, if t.dark { with_alpha(t.muted, 0.6) } else { with_alpha(t.muted, 0.7) });
        for r in first..last {
            let y = geo.row_y(r);
            let in_sel = r >= sel.r0 && r <= sel.r1;
            let rr = Rect::from_min_size(pos2(gutter.left(), y), vec2(GUTTER, row_h));
            if in_sel {
                gp.rect_filled(rr, CornerRadius::ZERO, with_alpha(t.accent, 0.12));
                gp.line_segment([rr.right_top(), rr.right_bottom()], Stroke::new(2.0, t.accent));
            }
            gp.text(pos2(rr.right() - 10.0, rr.center().y), Align2::RIGHT_CENTER, (r + 1).to_string(), qmono(10.5), if in_sel { t.foreground } else { t.muted_foreground });
        }
        painter.line_segment([gutter.right_top() - vec2(0.0, HEAD_H), gutter.right_bottom()], Stroke::new(1.0, t.border));

        // ---- scrollbars
        let bars = [
            (false, Rect::from_min_max(pos2(scroll_area.left(), body.bottom() + 2.0), pos2(body.right(), body.bottom() + 8.0)), scroll_area.width(), total_w, st.scroll.x, max_scroll.x),
            (true, Rect::from_min_max(pos2(body.right() + 2.0, body.top()), pos2(body.right() + 8.0, body.bottom())), body.height(), total_h, st.scroll.y, max_scroll.y),
        ];
        for (vertical, track, view, total, pos, maxs) in bars {
            if maxs <= 0.0 {
                continue;
            }
            let len = if vertical { track.height() } else { track.width() };
            let thumb_len = (len * view / total).max(28.0);
            let off = (len - thumb_len) * pos / maxs;
            let thumb = if vertical { Rect::from_min_size(pos2(track.left(), track.top() + off), vec2(track.width(), thumb_len)) } else { Rect::from_min_size(pos2(track.left() + off, track.top()), vec2(thumb_len, track.height())) };
            let r = ui.interact(thumb.expand(2.0), id.with(("sb", vertical)), Sense::drag());
            let h = ui.ctx().animate_bool_with_time(id.with(("sbh", vertical)), r.hovered() || r.dragged(), 0.12);
            painter.rect_filled(thumb, CornerRadius::same(3), with_alpha(t.muted_foreground, 0.3 + 0.35 * h));
            if r.drag_started() {
                st.scroll_drag = Some((vertical, pointer.map(|p| if vertical { p.y } else { p.x }).unwrap_or(0.0), pos));
            }
            if let (Some((v, p0, s0)), Some(p)) = (st.scroll_drag, pointer) {
                if v == vertical && r.dragged() {
                    let d = (if vertical { p.y } else { p.x }) - p0;
                    let ns = (s0 + d * maxs / (len - thumb_len).max(1.0)).clamp(0.0, maxs);
                    if vertical {
                        st.scroll.y = ns;
                    } else {
                        st.scroll.x = ns;
                    }
                }
            }
            if r.drag_stopped() {
                st.scroll_drag = None;
            }
        }

        // ---- status bar
        let status = Rect::from_min_max(pos2(rect.left(), rect.bottom() - STATUS), rect.max);
        painter.line_segment([status.left_top(), status.right_top()], Stroke::new(1.0, t.border));
        let col_letter = |c: usize| -> String {
            let mut s = String::new();
            let mut n = c + 1;
            while n > 0 {
                let rem = (n - 1) % 26;
                s.insert(0, (b'A' + rem as u8) as char);
                n = (n - 1) / 26;
            }
            s
        };
        let addr = if sel.count() == 1 { format!("{}{}", col_letter(sel.c0), sel.r0 + 1) } else { format!("{}{}:{}{}", col_letter(sel.c0), sel.r0 + 1, col_letter(sel.c1), sel.r1 + 1) };
        painter.text(pos2(status.left() + 14.0, status.center().y), Align2::LEFT_CENTER, addr, qmono(11.0), t.foreground);
        let mut sum = 0.0;
        let mut n = 0usize;
        if sel.count() <= 200_000 {
            for r in sel.r0..=sel.r1 {
                for c in sel.c0..=sel.c1 {
                    if matches!(data.columns[c].kind, CellKind::Number | CellKind::Currency | CellKind::Percent) {
                        if let Some(v) = parse_number(data.cell(r, c)) {
                            sum += v;
                            n += 1;
                        }
                    }
                }
            }
        }
        let info = if n > 1 { format!("Sum {}   ·   Avg {}   ·   Count {}", group_thousands(sum, 2), group_thousands(sum / n as f64, 2), n) } else { format!("{} rows  ·  {} columns  ·  {} visible", group_thousands(nrows as f64, 0), ncols, last.saturating_sub(first)) };
        painter.text(pos2(status.right() - 14.0, status.center().y), Align2::RIGHT_CENTER, info, qfont(11.5), t.muted_foreground);
        if focused {
            painter.text(pos2(status.left() + 110.0, status.center().y), Align2::LEFT_CENTER, "Type to edit · Enter/Tab to move · Shift+arrows to extend · Ctrl+C / Ctrl+V", qfont(11.0), with_alpha(t.muted_foreground, 0.7));
        }
        changed
    }
}

struct Geo {
    widths: Vec<f32>,
    frozen: usize,
    body: Rect,
    scroll_left: f32,
    scroll: Vec2,
    row_h: f32,
    nrows: usize,
}

impl Geo {
    fn new(data: &GridData, st: &GridState, body: Rect, scroll_area: Rect, frozen: usize, row_h: f32) -> Self {
        Self { widths: data.columns.iter().map(|c| c.width).collect(), frozen, body, scroll_left: scroll_area.left(), scroll: st.scroll, row_h, nrows: data.rows.len() }
    }
    fn col_x(&self, c: usize) -> f32 {
        if c < self.frozen {
            self.body.left() + self.widths[..c].iter().sum::<f32>()
        } else {
            self.scroll_left - self.scroll.x + self.widths[self.frozen..c].iter().sum::<f32>()
        }
    }
    fn row_y(&self, r: usize) -> f32 {
        self.body.top() - self.scroll.y + r as f32 * self.row_h
    }
    fn cell_at(&self, p: egui::Pos2) -> Option<(usize, usize)> {
        if self.nrows == 0 || self.widths.is_empty() {
            return None;
        }
        let r = ((p.y - self.body.top() + self.scroll.y) / self.row_h).floor().clamp(0.0, (self.nrows - 1) as f32) as usize;
        let n = self.widths.len();
        let mut c = n - 1;
        for k in 0..n {
            if k >= self.frozen && p.x < self.scroll_left {
                continue;
            }
            let x0 = self.col_x(k);
            if p.x >= x0 && p.x < x0 + self.widths[k] {
                c = k;
                break;
            }
            if p.x < x0 {
                c = k.saturating_sub(1);
                break;
            }
        }
        Some((r, c))
    }
}

fn commit(data: &mut GridData, st: &mut GridState, changed: &mut bool) {
    if let Some((r, c, text, _)) = st.editing.take() {
        if data.cell(r, c) != text {
            data.set(r, c, text);
            *changed = true;
        }
    }
}

fn paint_cell(p: &egui::Painter, t: &Theme, kind: CellKind, v: &str, r: Rect) {
    let font = qfont(12.5);
    let w = r.width() - 20.0;
    match kind {
        CellKind::Text => {
            p.text(pos2(r.left() + 10.0, r.center().y), Align2::LEFT_CENTER, common::elide(p, v, font.clone(), w), font, t.foreground);
        }
        CellKind::Mono => {
            p.text(pos2(r.left() + 10.0, r.center().y), Align2::LEFT_CENTER, v, qmono(11.5), t.muted_foreground);
        }
        CellKind::Number | CellKind::Currency => {
            let txt = match parse_number(v) {
                Some(n) if kind == CellKind::Currency => format!("${}", group_thousands(n, 2)),
                Some(n) => group_thousands(n, 0),
                None => v.to_owned(),
            };
            let neg = parse_number(v).is_some_and(|n| n < 0.0);
            p.text(pos2(r.right() - 10.0, r.center().y), Align2::RIGHT_CENTER, txt, qmono(12.0), if neg { t.destructive } else { t.foreground });
        }
        CellKind::Percent => {
            let n = parse_number(v).unwrap_or(0.0) as f32;
            let bar = Rect::from_min_size(pos2(r.left() + 10.0, r.center().y - 3.0), vec2((r.width() - 64.0).max(10.0), 6.0));
            p.rect_filled(bar, CornerRadius::same(3), with_alpha(t.foreground, 0.08));
            let mut fill = bar;
            fill.set_width(bar.width() * (n / 100.0).clamp(0.0, 1.0));
            let col = if n >= 80.0 { t.success } else if n >= 40.0 { t.chart[1] } else { t.warning };
            p.rect_filled(fill, CornerRadius::same(3), col);
            p.text(pos2(r.right() - 10.0, r.center().y), Align2::RIGHT_CENTER, format!("{n:.0}%"), qmono(11.5), t.muted_foreground);
        }
        CellKind::Status => {
            if v.is_empty() {
                return;
            }
            let col = match v.to_lowercase().as_str() {
                "active" | "paid" | "done" | "enterprise" => t.success,
                "trial" | "pending" | "pro" => t.info,
                "churned" | "failed" | "overdue" => t.destructive,
                "paused" | "starter" => t.warning,
                _ => t.muted_foreground,
            };
            let g = p.layout_no_wrap(v.to_owned(), qfont(11.0), col);
            let pr = Rect::from_min_size(pos2(r.left() + 10.0, r.center().y - 10.0), vec2(g.size().x + 22.0, 20.0));
            p.rect_filled(pr, CornerRadius::same(10), with_alpha(col, 0.14));
            p.circle_filled(pos2(pr.left() + 9.0, pr.center().y), 3.0, col);
            p.galley(pos2(pr.left() + 16.0, pr.center().y - g.size().y / 2.0), g, col);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> GridData {
        GridData {
            columns: vec![GridColumn::new("a", 50.0, CellKind::Text), GridColumn::new("b", 50.0, CellKind::Number), GridColumn::new("c", 50.0, CellKind::Number)],
            rows: (0..5).map(|r| vec![format!("r{r}"), format!("{}", r * 10), format!("{}", r)]).collect(),
        }
    }

    #[test]
    fn ranges() {
        let r = CellRange::new((3, 2), (1, 0));
        assert_eq!((r.r0, r.c0, r.r1, r.c1), (1, 0, 3, 2));
        assert!(r.contains(2, 1));
        assert!(!r.contains(4, 1));
        assert_eq!(r.count(), 9);
    }

    #[test]
    fn tsv_roundtrip() {
        let mut d = data();
        let tsv = d.to_tsv(CellRange::new((0, 0), (1, 2)));
        assert_eq!(tsv, "r0\t0\t0\nr1\t10\t1");
        let rng = d.paste_tsv(3, 1, "x\ty\nz\tw\nignored\trow");
        assert_eq!(d.cell(3, 1), "x");
        assert_eq!(d.cell(4, 2), "w");
        assert_eq!((rng.r1, rng.c1), (4, 2)); // clipped to the grid
    }

    #[test]
    fn numbers() {
        assert_eq!(parse_number("$1,234.50"), Some(1234.5));
        assert_eq!(parse_number("42%"), Some(42.0));
        assert_eq!(parse_number("abc"), None);
        assert_eq!(group_thousands(1234567.891, 2), "1,234,567.89");
        assert_eq!(group_thousands(-999.0, 0), "-999");
        assert_eq!(group_thousands(1000.0, 0), "1,000");
    }
}
