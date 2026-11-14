//! Crate-internal form and navigation controls used by the shells and the app templates: a
//! breadcrumb trail, pagination, a labeled divider, toggle chips, a multi-line field, a radio
//! list, a numeric field and an anchored floating panel.

use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Id, Layout, Order, Rect, Response, RichText,
    Sense, Shape, Stroke, StrokeKind, TextEdit, Ui, UiBuilder, Widget,
};
use glowkit_components::{ControlSize, DropdownMenu, FieldLabel, Input, LineIcon, MenuEntry};
use glowkit_core::color::with_alpha;
use glowkit_core::{paint, Theme};

use super::kit;

fn control_radius(t: &Theme) -> CornerRadius {
    t.corner_scaled(0.8)
}

fn field_fill(t: &Theme) -> Color32 {
    if t.dark {
        with_alpha(t.input, 0.30)
    } else {
        t.background
    }
}

fn snap(ui: &Ui, v: f32) -> f32 {
    let ppp = ui.ctx().pixels_per_point();
    (v * ppp).round() / ppp
}

// ---------------------------------------------------------------------------------------------
// breadcrumb
// ---------------------------------------------------------------------------------------------

/// A breadcrumb trail with chevron separators. Middle items collapse into a "…" menu when there
/// are more than `max_items`. Returns the index of a clicked (non-current) item.
pub(crate) fn breadcrumb(ui: &mut Ui, items: &[&str], size: f32, max_items: usize) -> Option<usize> {
    let t = Theme::of(ui);
    let id = ui.scope_id().with(("tpl_breadcrumb", items));
    let n = items.len();
    if n == 0 {
        return None;
    }
    let max_items = max_items.max(2);
    let mut shown: Vec<Option<usize>> = (0..n).map(Some).collect();
    let mut hidden: Vec<usize> = Vec::new();
    if n > max_items {
        let tail = max_items - 1;
        hidden = (1..n - tail).collect();
        shown = vec![Some(0), None];
        shown.extend((n - tail..n).map(Some));
    }
    let font = FontId::proportional(size);
    let height = size + 14.0;
    let mut clicked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (k, item) in shown.iter().enumerate() {
            if k > 0 {
                let (r, _) = ui.allocate_exact_size(vec2(22.0, height), Sense::hover());
                LineIcon::ChevronRight.paint(
                    ui.painter(),
                    Rect::from_center_size(r.center(), vec2(13.0, 13.0)),
                    with_alpha(t.muted_foreground, 0.7),
                );
            }
            match item {
                None => {
                    let (r, resp) = ui.allocate_exact_size(vec2(30.0, height), Sense::click());
                    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
                    let h = kit::hover(ui, id.with("dots"), resp.hovered());
                    let br = Rect::from_center_size(r.center(), vec2(28.0, 24.0));
                    ui.painter().rect_filled(br, 6.0, with_alpha(t.foreground, 0.08 * h));
                    LineIcon::Dots.paint(
                        ui.painter(),
                        Rect::from_center_size(r.center(), vec2(16.0, 16.0)),
                        t.muted_foreground.lerp_to_gamma(t.foreground, h),
                    );
                    let entries: Vec<MenuEntry> = hidden.iter().map(|&i| MenuEntry::item(items[i])).collect();
                    if let Some(label) = DropdownMenu::new(entries).width(160.0).show(&resp) {
                        clicked = hidden.iter().copied().find(|&i| items[i] == label);
                    }
                }
                Some(i) => {
                    let i = *i;
                    let current = i + 1 == n;
                    let g = ui.painter().layout_no_wrap(items[i].to_owned(), font.clone(), Color32::WHITE);
                    let (r, resp) = ui.allocate_exact_size(
                        vec2(g.size().x + 4.0, height),
                        if current { Sense::hover() } else { Sense::click() },
                    );
                    let resp = if current { resp } else { resp.on_hover_cursor(CursorIcon::PointingHand) };
                    let h = if current { 0.0 } else { kit::hover(ui, id.with(("i", i)), resp.hovered()) };
                    let col = if current { t.foreground } else { t.muted_foreground.lerp_to_gamma(t.foreground, h) };
                    let tp = pos2(r.min.x + 2.0, r.center().y - g.size().y / 2.0);
                    ui.painter().text(tp, Align2::LEFT_TOP, items[i], font.clone(), col);
                    if h > 0.01 {
                        let y = tp.y + g.size().y + 1.0;
                        let w = g.size().x * glowkit_core::ease(glowkit_core::Easing::OutCubic, h);
                        ui.painter().line_segment([pos2(tp.x, y), pos2(tp.x + w, y)], Stroke::new(1.0, col));
                    }
                    if resp.clicked() {
                        clicked = Some(i);
                    }
                }
            }
        }
    });
    clicked
}

// ---------------------------------------------------------------------------------------------
// pagination
// ---------------------------------------------------------------------------------------------

fn page_slots(page: usize, total: usize) -> Vec<Option<usize>> {
    if total <= 7 {
        return (1..=total).map(Some).collect();
    }
    let left = page.saturating_sub(1).max(1);
    let right = (page + 1).min(total);
    let (show_l, show_r) = (left > 2, right < total - 1);
    let mut v = Vec::new();
    if !show_l && show_r {
        v.extend((1..=5).map(Some));
        v.extend([None, Some(total)]);
    } else if show_l && !show_r {
        v.extend([Some(1), None]);
        v.extend((total - 4..=total).map(Some));
    } else {
        v.extend([Some(1), None]);
        v.extend((left..=right).map(Some));
        v.extend([None, Some(total)]);
    }
    v
}

/// Previous / numbered pages with ellipses / Next. `page` is 1-based; the active page's outline
/// slides between numbers on a spring.
pub(crate) fn pagination(ui: &mut Ui, page: &mut usize, total: usize) -> Response {
    let t = Theme::of(ui);
    let total = total.max(1);
    *page = (*page).clamp(1, total);
    let slots = page_slots(*page, total);
    let (cell, gap) = (36.0, 4.0);
    let font = FontId::proportional(13.5);
    let nav_w =
        |label: &str| ui.painter().layout_no_wrap(label.to_owned(), font.clone(), Color32::WHITE).size().x + 40.0;
    let (prev_w, next_w) = (nav_w("Previous"), nav_w("Next"));
    let total_w = prev_w + next_w + slots.len() as f32 * (cell + gap) + gap;
    let (rect, mut resp) = ui.allocate_exact_size(vec2(total_w, cell), Sense::hover());
    let id = resp.id.with("tpl_pagination");
    let mut new_page = *page;
    let btn = |ui: &mut Ui, r: Rect, key: usize, enabled: bool| -> Response {
        let resp = ui.interact(r, id.with(("b", key)), if enabled { Sense::click() } else { Sense::hover() });
        let resp = if enabled { resp.on_hover_cursor(CursorIcon::PointingHand) } else { resp };
        let h = kit::hover(ui, id.with(("b", key)), resp.hovered() && enabled);
        ui.painter().rect_filled(r, t.radius * 0.7, with_alpha(t.foreground, 0.07 * h));
        resp
    };
    let mut x = rect.min.x;
    let pr = Rect::from_min_size(pos2(x, rect.min.y), vec2(prev_w, cell));
    let can_prev = *page > 1;
    let pcol = if can_prev { t.foreground } else { with_alpha(t.muted_foreground, 0.5) };
    if btn(ui, pr, 0, can_prev).clicked() {
        new_page -= 1;
    }
    LineIcon::ChevronLeft.paint(
        ui.painter(),
        Rect::from_center_size(pos2(pr.min.x + 16.0, pr.center().y), vec2(14.0, 14.0)),
        pcol,
    );
    ui.painter().text(pos2(pr.min.x + 28.0, pr.center().y), Align2::LEFT_CENTER, "Previous", font.clone(), pcol);
    x += prev_w + gap;
    let mut active_x = None;
    let mut cells = Vec::new();
    for (k, s) in slots.iter().enumerate() {
        let r = Rect::from_min_size(pos2(x, rect.min.y), vec2(cell, cell));
        if let Some(pg) = s {
            if btn(ui, r, 10 + k, true).clicked() {
                new_page = *pg;
            }
            if *pg == *page {
                active_x = Some(r.min.x);
            }
        }
        cells.push((r, *s));
        x += cell + gap;
    }
    if let Some(ax) = active_x {
        let sx = glowkit_core::anim::spring(ui, id.with("ax"), ax, 400.0, 32.0);
        let ar = Rect::from_min_size(pos2(sx, rect.min.y), vec2(cell, cell));
        ui.painter().rect_filled(ar, t.radius * 0.7, t.background);
        ui.painter().rect_stroke(ar, t.radius * 0.7, Stroke::new(1.0, t.border), StrokeKind::Inside);
    }
    for (r, s) in cells {
        match s {
            Some(pg) => {
                ui.painter().text(r.center(), Align2::CENTER_CENTER, pg.to_string(), font.clone(), t.foreground);
            }
            None => LineIcon::Dots.paint(
                ui.painter(),
                Rect::from_center_size(r.center(), vec2(16.0, 16.0)),
                t.muted_foreground,
            ),
        }
    }
    let nr = Rect::from_min_size(pos2(x, rect.min.y), vec2(next_w, cell));
    let can_next = *page < total;
    let ncol = if can_next { t.foreground } else { with_alpha(t.muted_foreground, 0.5) };
    if btn(ui, nr, 1, can_next).clicked() {
        new_page += 1;
    }
    ui.painter().text(pos2(nr.max.x - 28.0, nr.center().y), Align2::RIGHT_CENTER, "Next", font, ncol);
    LineIcon::ChevronRight.paint(
        ui.painter(),
        Rect::from_center_size(pos2(nr.max.x - 16.0, nr.center().y), vec2(14.0, 14.0)),
        ncol,
    );
    let new_page = new_page.clamp(1, total);
    if new_page != *page {
        *page = new_page;
        resp.mark_changed();
    }
    resp
}

// ---------------------------------------------------------------------------------------------
// divider
// ---------------------------------------------------------------------------------------------

/// A full-width hairline with a small uppercase label in the middle ("OR CONTINUE WITH").
pub(crate) fn labeled_divider(ui: &mut Ui, label: &str) -> Response {
    let t = Theme::of(ui);
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 16.0), Sense::hover());
    let y = snap(ui, rect.center().y) + 0.5;
    let p = ui.painter();
    let g = p.layout_no_wrap(label.to_uppercase(), FontId::proportional(11.0), t.muted_foreground);
    let gx = rect.center().x - g.size().x / 2.0;
    let stroke = Stroke::new(1.0, t.border);
    p.line_segment([pos2(rect.min.x, y), pos2(gx - 10.0, y)], stroke);
    p.line_segment([pos2(gx + g.size().x + 10.0, y), pos2(rect.max.x, y)], stroke);
    p.galley(pos2(gx, rect.center().y - g.size().y / 2.0), g, t.muted_foreground);
    resp
}

// ---------------------------------------------------------------------------------------------
// toggle group
// ---------------------------------------------------------------------------------------------

/// A joined row of text toggles where any number can be pressed (`pressed` is resized to fit).
pub(crate) fn toggle_row(ui: &mut Ui, pressed: &mut Vec<bool>, items: &[&str], size: ControlSize) -> Response {
    let t = Theme::of(ui);
    let n = items.len();
    pressed.resize(n, false);
    let pad = if size == ControlSize::Sm { 10.0 } else { 12.0 };
    let widths: Vec<f32> = items.iter().map(|s| kit::text_w(ui.painter(), s, size.font()) + pad * 2.0).collect();
    let h = size.height();
    let (rect, mut resp) = ui.allocate_exact_size(vec2(widths.iter().sum(), h), Sense::hover());
    let r = control_radius(&t);
    if !t.dark {
        paint::soft_shadow(ui.painter(), rect, r.nw as f32, 2.0, vec2(0.0, 1.0), Color32::from_black_alpha(10));
    }
    ui.painter().rect(rect, r, field_fill(&t), Stroke::new(1.0, t.input), StrokeKind::Inside);
    let mut x = rect.min.x;
    for (i, item) in items.iter().enumerate() {
        let ir = Rect::from_min_size(pos2(x, rect.min.y), vec2(widths[i], h));
        let iresp = ui.interact(ir, resp.id.with(i), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        if iresp.clicked() {
            pressed[i] = !pressed[i];
            resp.mark_changed();
        }
        let on = ui.ctx().animate_bool_with_time(resp.id.with(("on", i)), pressed[i], 0.16);
        let hover = kit::hover(ui, resp.id.with(("h", i)), iresp.hovered());
        let rr = r.nw;
        let corner = CornerRadius {
            nw: if i == 0 { rr } else { 0 },
            sw: if i == 0 { rr } else { 0 },
            ne: if i + 1 == n { rr } else { 0 },
            se: if i + 1 == n { rr } else { 0 },
        };
        let on_fill = if t.dark { t.secondary.lerp_to_gamma(t.foreground, 0.08) } else { t.secondary };
        let fill = Color32::TRANSPARENT
            .lerp_to_gamma(with_alpha(t.secondary, 0.6), hover * (1.0 - on))
            .lerp_to_gamma(on_fill, on);
        ui.painter().rect_filled(ir.shrink(1.0), corner, fill);
        if i + 1 < n {
            let xx = snap(ui, ir.max.x) - 0.5;
            ui.painter()
                .line_segment([pos2(xx, rect.min.y + 1.0), pos2(xx, rect.max.y - 1.0)], Stroke::new(1.0, t.input));
        }
        let col = t.muted_foreground.lerp_to_gamma(t.foreground, on.max(hover));
        ui.painter().text(ir.center(), Align2::CENTER_CENTER, *item, FontId::proportional(size.font()), col);
        x += widths[i];
    }
    resp
}

// ---------------------------------------------------------------------------------------------
// multi-line field
// ---------------------------------------------------------------------------------------------

/// A multi-line text field with label, auto-growing height, helper / error text and an optional
/// character counter, styled like the single-line `Input`.
#[must_use = "add it with `ui.add(...)`"]
pub(crate) struct MultilineField<'a> {
    text: &'a mut String,
    label: Option<String>,
    description: Option<String>,
    error: Option<String>,
    placeholder: String,
    rows: usize,
    max_len: Option<usize>,
    width: Option<f32>,
    id_salt: Option<Id>,
}

impl<'a> MultilineField<'a> {
    pub(crate) fn new(text: &'a mut String) -> Self {
        Self {
            text,
            label: None,
            description: None,
            error: None,
            placeholder: String::new(),
            rows: 3,
            max_len: None,
            width: None,
            id_salt: None,
        }
    }
    pub(crate) fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }
    pub(crate) fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
    pub(crate) fn error(mut self, e: impl Into<String>) -> Self {
        self.error = Some(e.into());
        self
    }
    pub(crate) fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }
    pub(crate) fn rows(mut self, r: usize) -> Self {
        self.rows = r.max(1);
        self
    }
    pub(crate) fn max_len(mut self, n: usize) -> Self {
        self.max_len = Some(n);
        self
    }
    pub(crate) fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    pub(crate) fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(Id::unique(salt));
        self
    }
}

impl Widget for MultilineField<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let width = match self.width {
            Some(w) if w.is_finite() => w,
            _ => ui.available_width(),
        };
        ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            if let Some(l) = &self.label {
                ui.add(FieldLabel::new(l.clone()));
            }
            let base_id = match self.id_salt {
                Some(s) => ui.make_persistent_id(s),
                None => {
                    let id = ui.next_auto_id();
                    ui.skip_ahead_auto_ids(1);
                    id
                }
            };
            let te_id = base_id.with("te");
            let chrome_idx = ui.painter().add(Shape::Noop);
            let focused = ui.memory(|m| m.has_focus(te_id));
            let focus = ui.ctx().animate_bool_with_time(base_id.with("f"), focused, 0.16);
            let text = self.text;
            let mut te = TextEdit::multiline(&mut *text)
                .id(te_id)
                .frame(egui::Frame::NONE)
                .margin(egui::Margin::ZERO)
                .font(FontId::proportional(14.0))
                .text_color(t.foreground)
                .hint_text(RichText::new(self.placeholder).color(with_alpha(t.muted_foreground, 0.85)).size(14.0))
                .desired_rows(self.rows)
                .desired_width(width - 24.0);
            if let Some(n) = self.max_len {
                te = te.char_limit(n);
            }
            let inner = egui::Frame::NONE.inner_margin(egui::Margin::symmetric(12, 9)).show(ui, |ui| {
                ui.set_width(width - 24.0);
                ui.add(te)
            });
            let rect = inner.response.rect;
            let bg = ui.interact(rect, base_id.with("bg"), Sense::click()).on_hover_cursor(CursorIcon::Text);
            if bg.clicked() {
                ui.memory_mut(|m| m.request_focus(te_id));
            }
            let hover = kit::hover(ui, base_id.with("h"), ui.rect_contains_pointer(rect));
            let error = self.error.is_some();
            // chrome beneath the text
            let r = control_radius(&t);
            let base = t.input.lerp_to_gamma(t.muted_foreground, 0.25 * hover);
            let accent = if error { t.destructive } else { t.ring };
            let border = if error { t.destructive } else { base.lerp_to_gamma(accent, focus) };
            let mut shapes: Vec<Shape> = Vec::new();
            if !t.dark {
                shapes.push(
                    egui::epaint::Shadow { offset: [0, 1], blur: 2, spread: 0, color: Color32::from_black_alpha(10) }
                        .as_shape(rect, r)
                        .into(),
                );
            }
            shapes.push(
                egui::epaint::RectShape::new(rect, r, field_fill(&t), Stroke::new(1.0, border), StrokeKind::Inside)
                    .into(),
            );
            let k = if error { focus.max(0.4) } else { focus };
            if k > 0.01 {
                shapes.push(
                    egui::epaint::RectShape::stroke(
                        rect,
                        r,
                        Stroke::new(1.0 + 2.0 * k, with_alpha(accent, 0.32 * k)),
                        StrokeKind::Outside,
                    )
                    .into(),
                );
            }
            ui.painter().set(chrome_idx, Shape::Vec(shapes));
            // footer: description / error on the left, counter on the right
            let msg = match (&self.error, &self.description) {
                (Some(e), _) => Some((e.clone(), t.destructive)),
                (None, Some(d)) => Some((d.clone(), t.muted_foreground)),
                _ => None,
            };
            if msg.is_some() || self.max_len.is_some() {
                let (row, _) = ui.allocate_exact_size(vec2(width, 16.0), Sense::hover());
                let p = ui.painter();
                let mut right = row.max.x;
                if let Some(max) = self.max_len {
                    let n = text.chars().count();
                    let near = n as f32 / max as f32;
                    let col = if near >= 1.0 {
                        t.destructive
                    } else if near > 0.9 {
                        t.warning
                    } else {
                        t.muted_foreground
                    };
                    let cr = p.text(
                        row.right_center(),
                        Align2::RIGHT_CENTER,
                        format!("{n}/{max}"),
                        FontId::proportional(12.0),
                        col,
                    );
                    right = cr.min.x - 8.0;
                }
                if let Some((m, mcol)) = msg {
                    p.with_clip_rect(Rect::from_min_max(row.min, pos2(right, row.max.y))).text(
                        row.left_center(),
                        Align2::LEFT_CENTER,
                        m,
                        FontId::proportional(12.5),
                        mcol,
                    );
                }
            }
            inner.inner
        })
        .inner
    }
}

// ---------------------------------------------------------------------------------------------
// radio list
// ---------------------------------------------------------------------------------------------

/// A vertical list of radio options, each with a title and a muted description line. The
/// selected dot springs in.
pub(crate) fn radio_list(ui: &mut Ui, selected: &mut usize, options: &[(&str, &str)]) -> Response {
    let t = Theme::of(ui);
    let w = ui.available_width();
    let mut inner = ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 12.0;
        let mut changed = false;
        for (i, (title, desc)) in options.iter().enumerate() {
            let tw = w - 26.0;
            let tg = ui.painter().layout(title.to_string(), FontId::proportional(14.0), t.foreground, tw);
            let dg = ui.painter().layout(desc.to_string(), FontId::proportional(12.5), t.muted_foreground, tw);
            let h = tg.size().y + 3.0 + dg.size().y;
            let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
            let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
            if resp.clicked() && *selected != i {
                *selected = i;
                changed = true;
            }
            let sel = *selected == i;
            let k = ui.ctx().animate_bool_with_time(resp.id.with("k"), sel, 0.14);
            let dot = glowkit_core::anim::spring(ui, resp.id.with("dot"), if sel { 1.0 } else { 0.0 }, 520.0, 18.0);
            let hv = kit::hover(ui, resp.id, resp.hovered());
            let c = pos2(rect.min.x + 8.0, rect.min.y + tg.size().y / 2.0);
            let border = t
                .input
                .lerp_to_gamma(t.muted_foreground, 0.35 * hv + if t.dark { 0.15 } else { 0.1 })
                .lerp_to_gamma(t.primary, k);
            let p = ui.painter();
            p.circle(c, 8.0, field_fill(&t), Stroke::new(1.0, border));
            if dot > 0.01 {
                p.circle_filled(c, 4.0 * dot.max(0.0), t.primary);
            }
            p.galley(pos2(rect.min.x + 26.0, rect.min.y), tg.clone(), t.foreground);
            p.galley(pos2(rect.min.x + 26.0, rect.min.y + tg.size().y + 3.0), dg, t.muted_foreground);
        }
        changed
    });
    if inner.inner {
        inner.response.mark_changed();
    }
    inner.response
}

// ---------------------------------------------------------------------------------------------
// numeric field
// ---------------------------------------------------------------------------------------------

/// A money / number field built on `Input`: the typed text is kept while editing and the value is
/// clamped to `range` and reformatted with two decimals when focus leaves.
pub(crate) fn amount_field(
    ui: &mut Ui,
    value: &mut f64,
    label: &str,
    prefix: &str,
    range: std::ops::RangeInclusive<f64>,
    width: f32,
    id_salt: &str,
) -> Response {
    let id = ui.make_persistent_id(("tpl_amount", id_salt));
    let fmt = |v: f64| format!("{v:.2}");
    let mut buf: String = ui.data_mut(|d| d.get_temp(id).unwrap_or_else(|| fmt(*value)));
    let resp = ui.add(Input::new(&mut buf).label(label).prefix(prefix).width(width).id_salt(id_salt));
    if resp.changed() {
        if let Ok(v) = buf.replace(',', "").trim().parse::<f64>() {
            *value = v.clamp(*range.start(), *range.end());
        }
    }
    if !resp.has_focus() {
        buf = fmt(*value);
    }
    ui.data_mut(|d| d.insert_temp(id, buf));
    resp
}

// ---------------------------------------------------------------------------------------------
// floating panel
// ---------------------------------------------------------------------------------------------

fn panel_open_id(trigger: Id) -> Id {
    trigger.with("tpl_panel_open")
}

/// Is the floating panel attached to `trigger` open?
pub(crate) fn panel_is_open(ctx: &egui::Context, trigger: Id) -> bool {
    ctx.data_mut(|d| d.get_temp::<bool>(panel_open_id(trigger)).unwrap_or(false))
}

/// Open or close the floating panel attached to `trigger`.
pub(crate) fn set_panel_open(ctx: &egui::Context, trigger: Id, open: bool) {
    ctx.data_mut(|d| d.insert_temp(panel_open_id(trigger), open));
}

/// A card that drops down below `trigger` (toggled by clicking it; closed by Escape or a click
/// outside), fading and sliding in. It stays inside `bounds`. Returns the body's result while
/// visible.
pub(crate) fn floating_panel<R>(
    trigger: &Response,
    width: f32,
    bounds: Rect,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let ctx = trigger.ctx.clone();
    let tid = trigger.id;
    let id = tid.with("tpl_panel");
    let mut open = panel_is_open(&ctx, tid);
    if trigger.clicked() {
        open = !open;
    }
    let last: Option<Rect> = ctx.data_mut(|d| d.get_temp(id.with("rect")));
    if open && !trigger.clicked() {
        let (pressed, pos, esc) =
            ctx.input(|i| (i.pointer.any_pressed(), i.pointer.interact_pos(), i.key_pressed(egui::Key::Escape)));
        if esc {
            open = false;
        }
        if let (true, Some(pos), Some(r)) = (pressed, pos, last) {
            if !r.contains(pos) && !trigger.rect.contains(pos) {
                open = false;
            }
        }
    }
    set_panel_open(&ctx, tid, open);
    let p = kit::follow(&ctx, id.with("p"), if open { 1.0 } else { 0.0 }, 16.0);
    if p < 0.01 && !open {
        return None;
    }
    let t = Theme::current(&ctx);
    let pad = 16.0;
    let h: f32 = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(id.with("h"), || 120.0));
    let x = (trigger.rect.center().x - width / 2.0)
        .clamp(bounds.min.x + 8.0, (bounds.max.x - 8.0 - width).max(bounds.min.x));
    let below = trigger.rect.max.y + 6.0;
    let y = if below + h > bounds.max.y - 8.0 && trigger.rect.min.y - 6.0 - h > bounds.min.y {
        trigger.rect.min.y - 6.0 - h
    } else {
        below
    };
    let rect = Rect::from_min_size(pos2(x, y - (1.0 - p) * 6.0), vec2(width, h));
    let res = egui::Area::new(id).order(Order::Foreground).fixed_pos(rect.min).constrain(false).show(&ctx, |ui| {
        ui.multiply_opacity(p);
        ui.set_clip_rect(bounds.expand(30.0));
        ui.allocate_rect(rect, Sense::click());
        let painter = ui.painter();
        let shadow = if t.dark { Color32::from_black_alpha(130) } else { Color32::from_black_alpha(34) };
        paint::soft_shadow(painter, rect, t.radius, 28.0, vec2(0.0, 10.0), shadow);
        painter.rect_filled(rect, t.radius, t.popover);
        painter.rect_stroke(rect, t.radius, Stroke::new(1.0, t.border), StrokeKind::Inside);
        let inner = Rect::from_min_max(rect.min + vec2(pad, pad), pos2(rect.max.x - pad, rect.max.y + 1000.0));
        let mut child = ui.new_child(UiBuilder::new().max_rect(inner));
        child.set_clip_rect(rect.intersect(bounds.expand(30.0)));
        let r = add(&mut child);
        (r, child.min_rect().height())
    });
    let (r, used_h) = res.inner;
    let new_h = (used_h + pad * 2.0).max(24.0);
    if (new_h - h).abs() > 0.5 {
        ctx.data_mut(|d| d.insert_temp(id.with("h"), new_h));
        ctx.request_repaint();
    }
    ctx.data_mut(|d| d.insert_temp(id.with("rect"), rect));
    Some(r)
}
