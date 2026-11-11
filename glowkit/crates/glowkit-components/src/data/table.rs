use super::avatar::Avatar;
use super::badge::{badge_colors, BadgeVariant};
use crate::inputs::kit::{self, UiIcon};
use crate::inputs::paint_checkbox;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Id, Painter, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;
use std::f32::consts::PI;

/// Content of one [`DataTable`] cell. Each kind renders with the right typography and sorts
/// sensibly.
#[derive(Clone, Debug, PartialEq)]
pub enum TableCell {
    /// Regular text.
    Text(String),
    /// Muted (secondary) text.
    Muted(String),
    /// Emphasized text.
    Strong(String),
    /// Monospace text (ids, hashes).
    Mono(String),
    /// A badge with a variant.
    Badge(String, BadgeVariant),
    /// A colored dot followed by text.
    Status(String, Color32),
    /// Avatar + name + muted email.
    User {
        /// Display name.
        name: String,
        /// Secondary line.
        email: String,
    },
    /// A number with its display text (sorted by the number).
    Number(f64, String),
    /// A small progress bar, 0..=1.
    Progress(f32),
}

impl From<&str> for TableCell {
    fn from(s: &str) -> Self {
        TableCell::Text(s.to_owned())
    }
}

impl From<String> for TableCell {
    fn from(s: String) -> Self {
        TableCell::Text(s)
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
enum SortKey {
    Num(f64),
    Str(String),
}

impl TableCell {
    fn sort_key(&self) -> SortKey {
        match self {
            TableCell::Text(s) | TableCell::Muted(s) | TableCell::Strong(s) | TableCell::Mono(s) | TableCell::Badge(s, _) | TableCell::Status(s, _) => SortKey::Str(s.to_lowercase()),
            TableCell::User { name, .. } => SortKey::Str(name.to_lowercase()),
            TableCell::Number(n, _) => SortKey::Num(*n),
            TableCell::Progress(p) => SortKey::Num(*p as f64),
        }
    }
}

/// Column definition for a [`DataTable`].
#[derive(Clone, Debug)]
pub struct TableColumn {
    title: String,
    fixed: Option<f32>,
    flex: f32,
    right: bool,
    sortable: bool,
}

impl TableColumn {
    /// A flexible, left-aligned, sortable column.
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), fixed: None, flex: 1.0, right: false, sortable: true }
    }
    /// Fixed width in points.
    pub fn fixed(mut self, w: f32) -> Self {
        self.fixed = Some(w);
        self
    }
    /// Share of the remaining width (default 1).
    pub fn flex(mut self, f: f32) -> Self {
        self.flex = f;
        self
    }
    /// Right-align (numbers, actions).
    pub fn right(mut self) -> Self {
        self.right = true;
        self
    }
    /// Whether clicking the header sorts by this column (default true).
    pub fn sortable(mut self, s: bool) -> Self {
        self.sortable = s;
        self
    }
}

#[derive(Clone, Copy, Default)]
struct TableState {
    sort: Option<(usize, bool)>,
    page: usize,
    changed_at: f64,
}

/// A data table in the Kibo / Origin UI style: sortable headers with an animated indicator,
/// row hover, optional striping, selection checkboxes with a select-all header, and a
/// pagination footer. Rows fade in when the page or sort changes.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut selected = vec![false; 2];
/// use glowkit_components::{DataTable, TableCell, TableColumn, BadgeVariant};
/// let cols = vec![TableColumn::new("Name").flex(2.0), TableColumn::new("Status"), TableColumn::new("Amount").right()];
/// let rows = vec![
///     vec![TableCell::Strong("INV001".into()), TableCell::Badge("Paid".into(), BadgeVariant::Success), TableCell::Number(250.0, "$250.00".into())],
///     vec![TableCell::Strong("INV002".into()), TableCell::Badge("Pending".into(), BadgeVariant::Warning), TableCell::Number(150.0, "$150.00".into())],
/// ];
/// ui.add(DataTable::new(cols, &rows).selection(&mut selected).page_size(5));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DataTable<'a> {
    columns: Vec<TableColumn>,
    rows: &'a [Vec<TableCell>],
    selection: Option<&'a mut Vec<bool>>,
    page_size: Option<usize>,
    striped: bool,
    width: Option<f32>,
    row_height: Option<f32>,
    id_salt: Option<Id>,
}

impl<'a> DataTable<'a> {
    /// A table with these columns and rows (each row has one cell per column).
    pub fn new(columns: Vec<TableColumn>, rows: &'a [Vec<TableCell>]) -> Self {
        Self { columns, rows, selection: None, page_size: None, striped: false, width: None, row_height: None, id_salt: None }
    }
    /// Add a checkbox column bound to per-row selection flags (resized to the row count).
    pub fn selection(mut self, s: &'a mut Vec<bool>) -> Self {
        self.selection = Some(s);
        self
    }
    /// Paginate with this many rows per page.
    pub fn page_size(mut self, n: usize) -> Self {
        self.page_size = Some(n.max(1));
        self
    }
    /// Alternate row backgrounds.
    pub fn striped(mut self, s: bool) -> Self {
        self.striped = s;
        self
    }
    /// Width (default: available).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Row height (default 48, or 56 when a row contains a user cell).
    pub fn row_height(mut self, h: f32) -> Self {
        self.row_height = Some(h);
        self
    }
    /// Stable id salt for sort / page state.
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(Id::unique(salt));
        self
    }
}

fn paint_cell(painter: &Painter, ui: &Ui, rect: Rect, cell: &TableCell, right: bool, t: &Theme, alpha: f32) {
    let al = |c: Color32| c.gamma_multiply(alpha);
    let (x, align) = if right { (rect.max.x, Align2::RIGHT_CENTER) } else { (rect.min.x, Align2::LEFT_CENTER) };
    let cy = rect.center().y;
    match cell {
        TableCell::Text(s) => {
            painter.text(pos2(x, cy), align, s, FontId::proportional(13.5), al(t.foreground));
        }
        TableCell::Muted(s) => {
            painter.text(pos2(x, cy), align, s, FontId::proportional(13.5), al(t.muted_foreground));
        }
        TableCell::Strong(s) => {
            painter.text(pos2(x, cy), align, s, FontId::proportional(14.0), al(t.foreground));
            painter.text(pos2(x + if right { -0.35 } else { 0.35 }, cy), align, s, FontId::proportional(14.0), al(with_alpha(t.foreground, 0.5)));
        }
        TableCell::Mono(s) => {
            painter.text(pos2(x, cy), align, s, FontId::monospace(12.5), al(t.muted_foreground));
        }
        TableCell::Number(_, s) => {
            painter.text(pos2(x, cy), align, s, FontId::proportional(13.5), al(t.foreground));
        }
        TableCell::Badge(s, v) => {
            let (fill, text, border) = badge_colors(t, *v);
            let g = painter.layout_no_wrap(s.clone(), FontId::proportional(12.0), al(text));
            let w = g.size().x + 16.0;
            let bx = if right { x - w } else { x };
            let br = Rect::from_min_size(pos2(bx, cy - 11.0), vec2(w, 22.0));
            painter.rect(br, CornerRadius::same(6), al(fill), Stroke::new(1.0, al(border)), StrokeKind::Inside);
            painter.galley(br.center() - g.size() / 2.0, g, al(text));
        }
        TableCell::Status(s, c) => {
            let g = painter.layout_no_wrap(s.clone(), FontId::proportional(13.5), al(t.foreground));
            let w = g.size().x + 16.0;
            let bx = if right { x - w } else { x };
            painter.circle_filled(pos2(bx + 4.0, cy), 4.0, al(with_alpha(*c, 0.25)));
            painter.circle_filled(pos2(bx + 4.0, cy), 2.5, al(*c));
            painter.galley(pos2(bx + 14.0, cy - g.size().y / 2.0), g, al(t.foreground));
        }
        TableCell::User { name, email } => {
            let ar = Rect::from_center_size(pos2(rect.min.x + 16.0, cy), vec2(32.0, 32.0));
            Avatar::new(name.clone()).paint(ui, ar);
            painter.text(pos2(ar.max.x + 10.0, cy - 8.5), Align2::LEFT_CENTER, name, FontId::proportional(13.5), al(t.foreground));
            painter.text(pos2(ar.max.x + 10.0, cy + 9.0), Align2::LEFT_CENTER, email, FontId::proportional(12.5), al(t.muted_foreground));
        }
        TableCell::Progress(p) => {
            let w = rect.width().min(120.0);
            let bx = if right { x - w - 40.0 } else { x };
            let track = Rect::from_min_size(pos2(bx, cy - 3.0), vec2(w, 6.0));
            painter.rect_filled(track, CornerRadius::same(3), al(if t.dark { t.secondary } else { mix(t.muted, t.input, 0.5) }));
            painter.rect_filled(Rect::from_min_size(track.min, vec2(w * p.clamp(0.0, 1.0), 6.0)), CornerRadius::same(3), al(t.primary));
            painter.text(pos2(track.max.x + 8.0, cy), Align2::LEFT_CENTER, format!("{:.0}%", p * 100.0), FontId::proportional(12.5), al(t.muted_foreground));
        }
    }
}

fn icon_btn(ui: &mut Ui, rect: Rect, id: Id, icon: UiIcon, enabled: bool) -> bool {
    let t = Theme::of(ui);
    let r = ui.interact(rect, id, if enabled { Sense::click() } else { Sense::hover() });
    let r = if enabled { r.on_hover_cursor(CursorIcon::PointingHand) } else { r };
    let h = kit::anim(ui, id.with("h"), r.hovered() && enabled, 0.1);
    let fill = if t.dark { with_alpha(t.input, 0.3) } else { t.background }.lerp_to_gamma(t.secondary, h);
    ui.painter().rect(rect, CornerRadius::same(7), fill, Stroke::new(1.0, t.input), StrokeKind::Inside);
    icon.paint(ui.painter(), Rect::from_center_size(rect.center(), vec2(15.0, 15.0)), if enabled { t.foreground } else { with_alpha(t.muted_foreground, 0.4) });
    r.clicked()
}

impl Widget for DataTable<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = self.width.unwrap_or_else(|| ui.available_width());
        let id = match self.id_salt {
            Some(s) => ui.make_persistent_id(s),
            None => {
                let id = ui.next_auto_id();
                ui.skip_ahead_auto_ids(1);
                id
            }
        };
        let now = ui.input(|i| i.time);
        let mut st: TableState = glowkit_core::state(ui, id, TableState::default);
        let n = self.rows.len();
        let mut selection = self.selection;
        if let Some(s) = selection.as_deref_mut() {
            s.resize(n, false);
        }
        let has_sel = selection.is_some();
        let has_user = self.rows.iter().any(|r| r.iter().any(|c| matches!(c, TableCell::User { .. })));
        let row_h = self.row_height.unwrap_or(if has_user { 56.0 } else { 48.0 });
        let head_h = 42.0;
        // order
        let mut order: Vec<usize> = (0..n).collect();
        if let Some((c, asc)) = st.sort {
            order.sort_by(|&a, &b| {
                let ka = self.rows[a].get(c).map(|x| x.sort_key());
                let kb = self.rows[b].get(c).map(|x| x.sort_key());
                let o = ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal);
                if asc {
                    o
                } else {
                    o.reverse()
                }
            });
        }
        let page_size = self.page_size.unwrap_or(n.max(1));
        let pages = n.div_ceil(page_size).max(1);
        st.page = st.page.min(pages - 1);
        let visible: Vec<usize> = order.iter().skip(st.page * page_size).take(page_size).copied().collect();
        let footer_h = if self.page_size.is_some() || has_sel { 56.0 } else { 0.0 };
        let body_rows = if self.page_size.is_some() { page_size } else { n };
        let table_h = head_h + row_h * body_rows as f32;
        let (rect, mut resp) = ui.allocate_exact_size(vec2(w, table_h + footer_h), Sense::hover());
        let trect = Rect::from_min_size(rect.min, vec2(w, (head_h + row_h * visible.len() as f32).max(head_h + row_h)));
        let cr = t.corner();
        let p = ui.painter().clone();
        p.rect(trect, cr, t.card, Stroke::NONE, StrokeKind::Inside);
        // column layout
        let pad = 16.0;
        let sel_w = if has_sel { 44.0 } else { 0.0 };
        let fixed: f32 = self.columns.iter().filter_map(|c| c.fixed).sum();
        let flex_total: f32 = self.columns.iter().filter(|c| c.fixed.is_none()).map(|c| c.flex).sum::<f32>().max(1e-3);
        let avail = (w - pad * 2.0 - sel_w - fixed - 16.0 * (self.columns.len().saturating_sub(1)) as f32).max(0.0);
        let mut xs = Vec::new();
        let mut x = rect.min.x + pad + sel_w;
        for c in &self.columns {
            let cw = c.fixed.unwrap_or(avail * c.flex / flex_total);
            xs.push((x, cw));
            x += cw + 16.0;
        }
        // header
        let header = Rect::from_min_size(rect.min, vec2(w, head_h));
        p.rect_filled(header.shrink(1.0), CornerRadius { nw: cr.nw, ne: cr.ne, sw: 0, se: 0 }, with_alpha(t.muted, if t.dark { 0.55 } else { 0.7 }));
        let y = kit::snap(ui, header.max.y) - 0.5;
        p.line_segment([pos2(rect.min.x, y), pos2(rect.max.x, y)], Stroke::new(1.0, t.border));
        if let Some(sel) = selection.as_deref_mut() {
            let all = !sel.is_empty() && sel.iter().all(|s| *s);
            let some = sel.iter().any(|s| *s);
            let cb = Rect::from_center_size(pos2(rect.min.x + pad + 8.0, header.center().y), vec2(16.0, 16.0));
            let r = ui.interact(cb.expand(6.0), id.with("all"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            if r.clicked() {
                let v = !all;
                sel.iter_mut().for_each(|s| *s = v);
                resp.mark_changed();
            }
            paint_checkbox(ui, id.with("allcb"), cb, all, some && !all, r.hovered(), None, 1.0);
        }
        for (ci, col) in self.columns.iter().enumerate() {
            let (cx, cw) = xs[ci];
            let hr = Rect::from_min_size(pos2(cx - 8.0, header.min.y + 6.0), vec2(cw + 16.0, head_h - 12.0));
            let sorted = st.sort.map(|(c, a)| (c == ci).then_some(a)).unwrap_or(None);
            let hov = if col.sortable {
                let r = ui.interact(hr, id.with(("h", ci)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                if r.clicked() {
                    st.sort = match sorted {
                        None => Some((ci, true)),
                        Some(true) => Some((ci, false)),
                        Some(false) => None,
                    };
                    st.changed_at = now;
                }
                kit::anim(ui, id.with(("hh", ci)), r.hovered(), 0.12)
            } else {
                0.0
            };
            if hov > 0.0 {
                p.rect_filled(hr, CornerRadius::same(6), with_alpha(if t.dark { t.secondary } else { t.input }, 0.7 * hov));
            }
            let tc = t.muted_foreground.lerp_to_gamma(t.foreground, if sorted.is_some() { 1.0 } else { hov });
            let g = p.layout_no_wrap(col.title.clone(), FontId::proportional(13.0), tc);
            let gx = if col.right { cx + cw - g.size().x - if col.sortable { 18.0 } else { 0.0 } } else { cx };
            let gw = g.size().x;
            p.galley(pos2(gx, header.center().y - g.size().y / 2.0), g, tc);
            if col.sortable {
                let vis = kit::anim(ui, id.with(("sv", ci)), sorted.is_some(), 0.15).max(hov * 0.5);
                let ang = glowkit_core::anim::spring(ui, id.with(("sa", ci)), if sorted == Some(false) { PI } else { 0.0 }, 300.0, 22.0);
                if vis > 0.01 {
                    let c = pos2(gx + gw + 10.0, header.center().y);
                    let rot = |v: egui::Vec2| c + vec2(v.x * ang.cos() - v.y * ang.sin(), v.x * ang.sin() + v.y * ang.cos());
                    let col_a = with_alpha(t.foreground, vis);
                    p.line_segment([rot(vec2(0.0, 5.0)), rot(vec2(0.0, -5.0))], Stroke::new(1.4, col_a));
                    p.add(egui::Shape::line(vec![rot(vec2(-3.5, -1.5)), rot(vec2(0.0, -5.0)), rot(vec2(3.5, -1.5))], Stroke::new(1.4, col_a)));
                }
            }
        }
        // rows
        let since = (now - st.changed_at) as f32;
        for (vi, &ri) in visible.iter().enumerate() {
            let rr = Rect::from_min_size(pos2(rect.min.x, header.max.y + vi as f32 * row_h), vec2(w, row_h));
            let appear = glowkit_core::ease(glowkit_core::Easing::OutCubic, glowkit_core::anim::stagger(since, vi, 0.03, 0.25));
            if appear < 1.0 {
                ui.ctx().request_repaint();
            }
            let row_resp = ui.interact(rr, id.with(("row", ri)), Sense::click());
            let hov = kit::anim(ui, id.with(("rh", ri)), row_resp.hovered(), 0.1);
            let selected = selection.as_deref().map(|s| s[ri]).unwrap_or(false);
            let sk = kit::anim(ui, id.with(("rs", ri)), selected, 0.14);
            let last = vi + 1 == visible.len();
            let rcr = if last { CornerRadius { nw: 0, ne: 0, sw: cr.sw.saturating_sub(1), se: cr.se.saturating_sub(1) } } else { CornerRadius::ZERO };
            let inner = Rect::from_min_max(pos2(rr.min.x + 1.0, rr.min.y), pos2(rr.max.x - 1.0, rr.max.y - if last { 1.0 } else { 0.0 }));
            if self.striped && vi % 2 == 1 {
                p.rect_filled(inner, rcr, with_alpha(t.muted, if t.dark { 0.35 } else { 0.45 }));
            }
            if sk > 0.0 {
                p.rect_filled(inner, rcr, with_alpha(t.accent, (if t.dark { 0.10 } else { 0.06 }) * sk));
            }
            if hov > 0.0 {
                p.rect_filled(inner, rcr, with_alpha(t.muted, (if t.dark { 0.6 } else { 0.8 }) * hov));
            }
            if !last {
                let yy = kit::snap(ui, rr.max.y) - 0.5;
                p.line_segment([pos2(rect.min.x + 1.0, yy), pos2(rect.max.x - 1.0, yy)], Stroke::new(1.0, t.border));
            }
            let dy = 6.0 * (1.0 - appear);
            if let Some(sel) = selection.as_deref_mut() {
                let cb = Rect::from_center_size(pos2(rect.min.x + pad + 8.0, rr.center().y + dy), vec2(16.0, 16.0));
                if row_resp.clicked() {
                    sel[ri] = !sel[ri];
                    resp.mark_changed();
                }
                paint_checkbox(ui, id.with(("cb", ri)), cb, sel[ri], false, row_resp.hovered(), None, appear);
            }
            for (ci, col) in self.columns.iter().enumerate() {
                let (cx, cw) = xs[ci];
                let cell_r = Rect::from_min_size(pos2(cx, rr.min.y + dy), vec2(cw, row_h));
                if let Some(cell) = self.rows[ri].get(ci) {
                    let cp = p.with_clip_rect(cell_r.expand2(vec2(2.0, 0.0)).intersect(p.clip_rect()));
                    paint_cell(&cp, ui, cell_r, cell, col.right, &t, appear);
                }
            }
        }
        p.rect_stroke(trect, cr, Stroke::new(1.0, t.border), StrokeKind::Inside);
        // footer
        if footer_h > 0.0 {
            let fy = rect.min.y + table_h + footer_h / 2.0 + 4.0;
            let left = match selection.as_deref() {
                Some(s) => format!("{} of {} row(s) selected.", s.iter().filter(|x| **x).count(), n),
                None => format!("Showing {}–{} of {}", (st.page * page_size + 1).min(n), (st.page * page_size + visible.len()).min(n), n),
            };
            p.text(pos2(rect.min.x + 2.0, fy), Align2::LEFT_CENTER, left, FontId::proportional(13.0), t.muted_foreground);
            if self.page_size.is_some() {
                let nb = Rect::from_center_size(pos2(rect.max.x - 16.0, fy), vec2(32.0, 32.0));
                let pb = nb.translate(vec2(-40.0, 0.0));
                if icon_btn(ui, nb, id.with("next"), UiIcon::ChevronRight, st.page + 1 < pages) {
                    st.page += 1;
                    st.changed_at = now;
                }
                if icon_btn(ui, pb, id.with("prev"), UiIcon::ChevronLeft, st.page > 0) {
                    st.page -= 1;
                    st.changed_at = now;
                }
                p.text(pos2(pb.min.x - 14.0, fy), Align2::RIGHT_CENTER, format!("Page {} of {}", st.page + 1, pages), FontId::proportional(13.0), t.foreground);
            }
        }
        glowkit_core::set_state(ui, id, st);
        resp
    }
}
