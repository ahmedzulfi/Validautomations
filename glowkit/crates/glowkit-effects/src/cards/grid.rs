use crate::buttons::setter;
use crate::buttons::util::cr;
use egui::{vec2, Align, Color32, Id, Layout, Rect, Response, Sense, Ui, UiBuilder};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// Allocate a uniform grid; returns the whole rect, the cell rects and the grid id. Cell height
/// comes from the tallest content measured last frame.
fn grid_layout(ui: &mut Ui, count: usize, columns: usize, gap: f32, min_h: f32) -> (Rect, Vec<Rect>, Id, Response) {
    let id = ui.next_auto_id();
    let cols = columns.clamp(1, count.max(1));
    let rows = count.div_ceil(cols);
    let w = ui.available_width();
    let cw = ((w - gap * (cols as f32 - 1.0)) / cols as f32).floor();
    let ch: f32 = glowkit_core::state(ui, id.with("glowkit_grid_h"), || min_h).max(min_h);
    let total = vec2(w, rows as f32 * ch + (rows as f32 - 1.0).max(0.0) * gap);
    let (rect, resp) = ui.allocate_exact_size(total, Sense::hover());
    let cells = (0..count)
        .map(|i| {
            let (c, r) = (i % cols, i / cols);
            Rect::from_min_size(rect.min + vec2(c as f32 * (cw + gap), r as f32 * (ch + gap)), vec2(cw, ch))
        })
        .collect();
    (rect, cells, id, resp)
}

fn store_height(ui: &Ui, id: Id, measured: f32, min_h: f32) {
    let key = id.with("glowkit_grid_h");
    let cur: f32 = glowkit_core::state(ui, key, || min_h);
    let m = measured.max(min_h);
    if (m - cur).abs() > 0.5 {
        glowkit_core::set_state(ui, key, m);
        ui.ctx().request_repaint();
    }
}

fn hovered_index(ui: &Ui, cells: &[Rect], expand: f32) -> Option<usize> {
    let p = ui.input(|i| i.pointer.hover_pos())?;
    if !ui.clip_rect().contains(p) {
        return None;
    }
    cells.iter().position(|r| r.expand(expand).contains(p))
}

// ---------------------------------------------------------------------------------------------

/// A grid of cards where a soft highlight slab slides from card to card, following the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::CardHoverEffect;
/// let items = ["Stripe", "Netflix", "Google"];
/// CardHoverEffect::new(items.len()).columns(3).show(ui, |ui, i| { ui.heading(items[i]); });
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct CardHoverEffect {
    count: usize,
    columns: usize,
    gap: f32,
    padding: f32,
    min_height: f32,
    radius: Option<f32>,
    highlight: Option<Color32>,
    ambient: bool,
}

impl CardHoverEffect {
    /// A grid of `count` cards.
    pub fn new(count: usize) -> Self {
        Self { count, columns: 3, gap: 16.0, padding: 20.0, min_height: 120.0, radius: None, highlight: None, ambient: false }
    }
    setter!(/// Number of columns.
        columns: usize);
    setter!(/// Gap between cards in points.
        gap: f32);
    setter!(/// Padding inside each card.
        padding: f32);
    setter!(/// Minimum card height.
        min_height: f32);
    /// Corner radius of the cards.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }
    /// Highlight slab color. Defaults to the theme's muted surface.
    pub fn highlight(mut self, color: Color32) -> Self {
        self.highlight = Some(color);
        self
    }
    setter!(/// When the pointer is away, let the highlight hop between cards on its own.
        ambient: bool);

    /// Lay out the grid; `add(ui, index)` fills each card.
    pub fn show(self, ui: &mut Ui, mut add: impl FnMut(&mut Ui, usize)) -> Response {
        let t = Theme::of(ui);
        let (_, cells, id, resp) = grid_layout(ui, self.count, self.columns, self.gap, self.min_height);
        let time = glowkit_core::time(ui);
        let mut target = hovered_index(ui, &cells, self.gap / 2.0);
        if target.is_none() && self.ambient && !cells.is_empty() {
            let order = [0usize, 1, 2, 5, 4, 3];
            let step = (time / 1.3) as usize;
            target = Some(order[step % order.len()] % cells.len());
            glowkit_core::animate(ui);
        }
        let radius = self.radius.unwrap_or(t.radius * 1.6);
        let last: usize = glowkit_core::state(ui, id.with("last"), || 0);
        let to = target.unwrap_or(last);
        glowkit_core::set_state(ui, id.with("last"), to);
        let vis = glowkit_core::anim::follow(ui, id.with("vis"), if target.is_some() { 1.0 } else { 0.0 }, 9.0);
        if let Some(cell) = cells.get(to) {
            let tr = cell.expand(self.gap * 0.45);
            let k = (170.0, 22.0);
            let min = glowkit_core::anim::spring2(ui, id.with("min"), tr.min.to_vec2(), k.0, k.1);
            let max = glowkit_core::anim::spring2(ui, id.with("max"), tr.max.to_vec2(), k.0, k.1);
            if vis > 0.01 {
                let hr = Rect::from_min_max(min.to_pos2(), max.to_pos2());
                let hc = self.highlight.unwrap_or(if t.dark { mix(t.muted, t.foreground, 0.06) } else { mix(t.muted, t.foreground, 0.03) });
                ui.painter().rect_filled(hr, cr(radius + self.gap * 0.45), hc.gamma_multiply(vis));
            }
        }
        let mut tallest: f32 = 0.0;
        for (i, cell) in cells.iter().enumerate() {
            let p = ui.painter();
            let on = Some(i) == target;
            let k = glowkit_core::anim::follow(ui, id.with(("on", i)), if on { 1.0 } else { 0.0 }, 10.0);
            let fill = with_alpha(if t.dark { mix(t.background, t.card, 0.5) } else { t.card }, 0.55);
            p.rect_filled(*cell, cr(radius), fill);
            p.rect_stroke(*cell, cr(radius), egui::Stroke::new(1.0, t.border.lerp_to_gamma(mix(t.border, t.foreground, 0.25), k)), egui::StrokeKind::Inside);
            let inner = cell.shrink(self.padding);
            let mut child = ui.new_child(UiBuilder::new().id_salt(id.with(("cell", i))).max_rect(inner).layout(Layout::top_down(Align::Min)));
            child.set_clip_rect(cell.intersect(ui.clip_rect()));
            add(&mut child, i);
            tallest = tallest.max(child.min_rect().height() + self.padding * 2.0);
        }
        store_height(ui, id, tallest, self.min_height);
        resp
    }
}

// ---------------------------------------------------------------------------------------------

/// A grid of cards where the hovered card stays in sharp focus while the others shrink back,
/// dim and haze over.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::FocusCards;
/// FocusCards::new(3).columns(3).show(ui, |ui, i, focused| { ui.label(format!("Card {i}")); });
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct FocusCards {
    count: usize,
    columns: usize,
    gap: f32,
    padding: f32,
    min_height: f32,
    radius: Option<f32>,
    dim: f32,
    ambient: bool,
}

impl FocusCards {
    /// A grid of `count` focus cards.
    pub fn new(count: usize) -> Self {
        Self { count, columns: 3, gap: 16.0, padding: 0.0, min_height: 200.0, radius: None, dim: 0.65, ambient: false }
    }
    setter!(/// Number of columns.
        columns: usize);
    setter!(/// Gap between cards in points.
        gap: f32);
    setter!(/// Padding inside each card (0 lets content paint edge to edge).
        padding: f32);
    setter!(/// Minimum card height.
        min_height: f32);
    /// Corner radius of the cards.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }
    setter!(/// How strongly unfocused cards fade (0..1).
        dim: f32);
    setter!(/// When the pointer is away, cycle the focus on its own.
        ambient: bool);

    /// Lay out the grid; `add(ui, index, focused)` fills each card. Content may paint the full
    /// card area with `ui.max_rect()` (e.g. an image or gradient).
    pub fn show(self, ui: &mut Ui, mut add: impl FnMut(&mut Ui, usize, bool)) -> Response {
        let t = Theme::of(ui);
        let (_, cells, id, resp) = grid_layout(ui, self.count, self.columns, self.gap, self.min_height);
        let time = glowkit_core::time(ui);
        let mut target = hovered_index(ui, &cells, 0.0);
        if target.is_none() && self.ambient && !cells.is_empty() {
            let ph = (time / 1.6) as usize;
            // focus each card in turn, with a "nothing focused" beat in between
            if ph % (cells.len() + 1) < cells.len() {
                target = Some(ph % (cells.len() + 1));
            }
            glowkit_core::animate(ui);
        }
        let radius = self.radius.unwrap_or(t.radius * 1.4);
        let mut tallest: f32 = 0.0;
        for (i, cell) in cells.iter().enumerate() {
            let dimmed = target.is_some() && target != Some(i);
            let focused = target == Some(i);
            let k = glowkit_core::anim::follow(ui, id.with(("dim", i)), if dimmed { 1.0 } else { 0.0 }, 8.0);
            let f = glowkit_core::anim::follow(ui, id.with(("focus", i)), if focused { 1.0 } else { 0.0 }, 8.0);
            let body = cell.shrink2(cell.size() * 0.02 * k);
            let p = ui.painter();
            paint::soft_shadow(p, body, radius, 16.0 + 18.0 * f, vec2(0.0, 6.0 + 8.0 * f), with_alpha(Color32::BLACK, if t.dark { 0.3 + 0.2 * f } else { 0.06 + 0.08 * f }));
            p.rect_filled(body, cr(radius), t.card);
            let inner = body.shrink(self.padding);
            let mut child = ui.new_child(UiBuilder::new().id_salt(id.with(("cell", i))).max_rect(inner).layout(Layout::top_down(Align::Min)));
            child.set_clip_rect(body.intersect(ui.clip_rect()));
            add(&mut child, i, focused);
            tallest = tallest.max(child.min_rect().height() + self.padding * 2.0);
            // haze: a veil of the page color plus a soft blur-like rim
            if k > 0.01 {
                let veil = with_alpha(t.background, self.dim * k);
                ui.painter().rect_filled(body, cr(radius), veil);
                paint::soft_shadow(ui.painter(), body.shrink(6.0), radius, 18.0, egui::Vec2::ZERO, with_alpha(t.background, 0.25 * k));
            }
            ui.painter().rect_stroke(body, cr(radius), egui::Stroke::new(1.0, t.border.lerp_to_gamma(with_alpha(t.foreground, 0.4), f)), egui::StrokeKind::Inside);
        }
        store_height(ui, id, tallest, self.min_height);
        resp
    }
}
