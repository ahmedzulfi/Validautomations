//! Model selector, suggestions, checkpoints and the web preview frame.

use crate::common;
use crate::icons::{paint_icon, Icon};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Order, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, Widget};
use glowkit_core::color::{hsv, mix, with_alpha};
use glowkit_core::{paint, Theme};

/// A model offered by a [`ModelSelector`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelInfo<'a> {
    /// Display name, e.g. "Nova 2".
    pub name: &'a str,
    /// Provider, e.g. "Lumen Labs" (drives the badge color and letter).
    pub provider: &'a str,
    /// One-line description.
    pub description: &'a str,
    /// Optional tag such as "New" or "Fast".
    pub tag: Option<&'a str>,
}

impl<'a> ModelInfo<'a> {
    /// A model with a name and provider.
    pub const fn new(name: &'a str, provider: &'a str) -> Self {
        Self { name, provider, description: "", tag: None }
    }
    /// One-line description.
    pub const fn description(mut self, d: &'a str) -> Self {
        self.description = d;
        self
    }
    /// Small tag next to the name.
    pub const fn tag(mut self, tag: &'a str) -> Self {
        self.tag = Some(tag);
        self
    }
}

fn hash_str(s: &str) -> u32 {
    s.bytes().fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619))
}

/// A provider badge: rounded square with the provider's initial in a stable color.
pub(crate) fn provider_badge(p: &egui::Painter, center: Pos2, size: f32, provider: &str, t: &Theme) {
    let hue = (hash_str(provider) % 360) as f32;
    let c = hsv(hue, 0.55, if t.dark { 0.9 } else { 0.62 });
    let r = Rect::from_center_size(center, vec2(size, size));
    p.rect_filled(r, CornerRadius::same((size * 0.28) as u8), with_alpha(c, if t.dark { 0.2 } else { 0.14 }));
    p.rect_stroke(r, CornerRadius::same((size * 0.28) as u8), Stroke::new(1.0, with_alpha(c, 0.35)), StrokeKind::Inside);
    let letter: String = provider.chars().next().map(|c| c.to_ascii_uppercase()).into_iter().collect();
    p.text(center, Align2::CENTER_CENTER, letter, FontId::proportional(size * 0.58), c);
}

/// A pill button showing the current model with its provider badge; clicking opens a dropdown
/// listing every model with provider badges, descriptions, tags and a check on the selection.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::{ModelInfo, ModelSelector};
/// let models = [ModelInfo::new("Nova 2", "Lumen Labs"), ModelInfo::new("Orbit 5", "Orbit AI")];
/// let mut selected = 0;
/// ui.add(ModelSelector::new(&models, &mut selected));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ModelSelector<'a> {
    models: &'a [ModelInfo<'a>],
    selected: &'a mut usize,
    ghost: bool,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> ModelSelector<'a> {
    /// Choose among `models`; `selected` is an index into it.
    pub fn new(models: &'a [ModelInfo<'a>], selected: &'a mut usize) -> Self {
        Self { models, selected, ghost: false, id_salt: None }
    }
    /// Stable id for the open state (needed when several selectors share a parent).
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }
    /// Borderless button (for use inside a prompt toolbar).
    pub fn ghost(mut self, on: bool) -> Self {
        self.ghost = on;
        self
    }
}

impl Widget for ModelSelector<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        if self.models.is_empty() {
            return ui.allocate_response(vec2(0.0, 0.0), Sense::hover());
        }
        *self.selected = (*self.selected).min(self.models.len() - 1);
        let cur = self.models[*self.selected];
        let g = common::galley(ui, cur.name, 13.0, t.foreground);
        let id = self.id_salt.map(|s| ui.scope_id().with_salt(s)).unwrap_or_else(|| ui.scope_id().with("glowkit_model_selector"));
        let (rect, _) = ui.allocate_exact_size(vec2(g.size().x + 58.0, 32.0), Sense::hover());
        let mut resp = ui.interact(rect, id.with("button"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let mut open: bool = glowkit_core::state(ui, id.with("open"), || false);
        if resp.clicked() {
            open = !open;
        }
        let h = common::anim_bool(ui, id.with("h"), resp.hovered() || open, 0.12);
        let p = ui.painter();
        let cr = CornerRadius::same(16);
        if !self.ghost {
            p.rect_filled(rect, cr, mix(t.card, common::hover_fill(&t), 0.0));
            p.rect_stroke(rect, cr, Stroke::new(1.0, t.border), StrokeKind::Inside);
        }
        if h > 0.0 {
            p.rect_filled(rect, cr, common::hover_fill(&t).gamma_multiply(h));
        }
        provider_badge(p, pos2(rect.min.x + 18.0, rect.center().y), 18.0, cur.provider, &t);
        p.galley(pos2(rect.min.x + 33.0, rect.center().y - g.size().y / 2.0), g, t.foreground);
        let open_t = common::anim_bool(ui, id.with("o"), open, 0.15);
        let cc = pos2(rect.max.x - 15.0, rect.center().y);
        // chevron down rotating up when open
        let flip = 1.0 - 2.0 * open_t;
        let pts = vec![cc + vec2(-4.0, -2.0 * flip), cc + vec2(0.0, 2.0 * flip), cc + vec2(4.0, -2.0 * flip)];
        ui.painter().add(egui::Shape::line(pts, Stroke::new(1.5, t.muted_foreground)));
        if open || open_t > 0.01 {
            let width = 300.0;
            // open upward when there is no room below
            let providers = self.models.windows(2).filter(|w| w[0].provider != w[1].provider).count() + 1;
            let est_h = self.models.iter().map(|m| if m.description.is_empty() { 38.0 } else { 52.0 }).sum::<f32>() + providers as f32 * 28.0 + 14.0;
            let below = ui.ctx().content_rect().max.y - rect.max.y;
            let up = below < est_h + 12.0 && rect.min.y > est_h + 12.0;
            let slide = (1.0 - open_t) * 6.0;
            let (pos, pivot) = if up { (pos2(rect.min.x, rect.min.y - 6.0 + slide), Align2::LEFT_BOTTOM) } else { (pos2(rect.min.x, rect.max.y + 6.0 - slide), Align2::LEFT_TOP) };
            let area = egui::Area::new(id.with("menu"))
                .order(Order::Foreground)
                .fade_in(false)
                .pivot(pivot)
                .fixed_pos(pos)
                .interactable(open)
                .show(ui.ctx(), |ui| {
                    ui.multiply_opacity(open_t);
                    egui::Frame::new()
                        .fill(t.popover)
                        .stroke(Stroke::new(1.0, t.border))
                        .corner_radius(t.corner_scaled(1.2))
                        .shadow(egui::Shadow { offset: [0, 10], blur: 30, spread: 0, color: Color32::from_black_alpha(if t.dark { 140 } else { 36 }) })
                        .inner_margin(egui::Margin::same(6))
                        .show(ui, |ui| {
                            ui.set_width(width);
                            ui.spacing_mut().item_spacing.y = 2.0;
                            let mut last_provider = "";
                            for (i, m) in self.models.iter().enumerate() {
                                if m.provider != last_provider {
                                    let (r, _) = ui.allocate_exact_size(vec2(width, 26.0), Sense::hover());
                                    common::text(ui.painter(), pos2(r.min.x + 10.0, r.center().y + 2.0), Align2::LEFT_CENTER, m.provider, 11.5, t.muted_foreground);
                                    last_provider = m.provider;
                                }
                                let row_h = if m.description.is_empty() { 36.0 } else { 50.0 };
                                let (r, rr) = ui.allocate_exact_size(vec2(width, row_h), Sense::click());
                                let rr = rr.on_hover_cursor(egui::CursorIcon::PointingHand);
                                let hh = common::anim_bool(ui, id.with(("row", i)), rr.hovered(), 0.08);
                                let p = ui.painter();
                                if hh > 0.0 {
                                    p.rect_filled(r, CornerRadius::same(8), common::hover_fill(&t).gamma_multiply(hh));
                                }
                                provider_badge(p, pos2(r.min.x + 22.0, r.center().y), 24.0, m.provider, &t);
                                let ty = if m.description.is_empty() { r.center().y } else { r.min.y + 17.0 };
                                let ng = common::galley(ui, m.name, 13.5, t.foreground);
                                let nx = r.min.x + 44.0;
                                ui.painter().galley(pos2(nx, ty - ng.size().y / 2.0), ng.clone(), t.foreground);
                                if let Some(tag) = m.tag {
                                    common::badge(ui.painter(), pos2(nx + ng.size().x + 8.0, ty), tag, t.accent, &t);
                                }
                                if !m.description.is_empty() {
                                    common::text(ui.painter(), pos2(nx, r.min.y + 35.0), Align2::LEFT_CENTER, m.description, 12.0, t.muted_foreground);
                                }
                                if i == *self.selected {
                                    paint_icon(ui.painter(), Icon::Check, Rect::from_center_size(pos2(r.max.x - 18.0, r.center().y), vec2(14.0, 14.0)), t.foreground);
                                }
                                if rr.clicked() {
                                    *self.selected = i;
                                    open = false;
                                    resp.mark_changed();
                                }
                            }
                        });
                });
            // click outside closes
            if open && !resp.clicked() && ui.input(|i| i.pointer.any_pressed()) {
                let p = ui.input(|i| i.pointer.interact_pos());
                if p.is_some_and(|p| !area.response.rect.contains(p) && !rect.contains(p)) {
                    open = false;
                }
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                open = false;
            }
        }
        glowkit_core::set_state(ui, id.with("open"), open);
        resp
    }
}

/// A wrap of rounded suggestion chips ("Explain quantum computing", ...). Returns the index of
/// the chip clicked this frame.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// if let Some(i) = glowkit_ai::Suggestions::new(&["Summarize this", "Write a test"]).show(ui) {
///     // use suggestion i
/// }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct Suggestions<'a> {
    items: &'a [&'a str],
    icon: Option<Icon>,
    centered: bool,
    size: f32,
}

impl<'a> Suggestions<'a> {
    /// Chips for `items`.
    pub fn new(items: &'a [&'a str]) -> Self {
        Self { items, icon: None, centered: false, size: 13.0 }
    }
    /// Leading icon on every chip.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
    /// Center the rows.
    pub fn centered(mut self, on: bool) -> Self {
        self.centered = on;
        self
    }
    /// Font size.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Show the chips; returns the clicked index.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = Theme::of(ui);
        let mut clicked = None;
        let gap = 8.0;
        let h = self.size * 2.6;
        let icon_w = if self.icon.is_some() { self.size + 6.0 } else { 0.0 };
        let gs: Vec<_> = self.items.iter().map(|s| common::galley(ui, *s, self.size, t.foreground)).collect();
        let widths: Vec<f32> = gs.iter().map(|g| g.size().x + 28.0 + icon_w).collect();
        // greedy rows
        let avail = ui.available_width();
        let mut rows: Vec<Vec<usize>> = vec![Vec::new()];
        let mut x = 0.0;
        for (i, w) in widths.iter().enumerate() {
            if x + w > avail && !rows.last().unwrap().is_empty() {
                rows.push(Vec::new());
                x = 0.0;
            }
            rows.last_mut().unwrap().push(i);
            x += w + gap;
        }
        let total_h = rows.len() as f32 * h + (rows.len() as f32 - 1.0).max(0.0) * gap;
        let (rect, base) = ui.allocate_exact_size(vec2(avail, total_h), Sense::hover());
        for (ri, row) in rows.iter().enumerate() {
            let row_w: f32 = row.iter().map(|&i| widths[i]).sum::<f32>() + gap * (row.len() as f32 - 1.0);
            let mut x = if self.centered { rect.center().x - row_w / 2.0 } else { rect.min.x };
            let y = rect.min.y + ri as f32 * (h + gap);
            for &i in row {
                let r = Rect::from_min_size(pos2(x, y), vec2(widths[i], h));
                let resp = ui.interact(r, base.id.with(("sugg", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                let hv = common::anim_bool(ui, resp.id.with("h"), resp.hovered(), 0.14);
                let lift = hv * 1.0;
                let r = r.translate(vec2(0.0, -lift));
                let p = ui.painter();
                let cr = CornerRadius::same((h / 2.0) as u8);
                if hv > 0.0 {
                    paint::soft_shadow(p, r, h / 2.0, 10.0, vec2(0.0, 2.0), Color32::from_black_alpha(((if t.dark { 70.0 } else { 18.0 }) * hv) as u8));
                }
                p.rect_filled(r, cr, mix(t.card, common::raised(&t), hv));
                p.rect_stroke(r, cr, Stroke::new(1.0, mix(t.border, common::border_strong(&t), hv)), StrokeKind::Inside);
                let mut tx = r.min.x + 14.0;
                if let Some(icon) = self.icon {
                    paint_icon(p, icon, Rect::from_center_size(pos2(tx + self.size / 2.0, r.center().y), vec2(self.size, self.size)), t.muted_foreground.lerp_to_gamma(t.accent, hv));
                    tx += icon_w;
                }
                let col = mix(t.foreground, t.muted_foreground, 0.25 * (1.0 - hv));
                p.galley(pos2(tx, r.center().y - gs[i].size().y / 2.0), gs[i].clone(), col);
                if resp.clicked() {
                    clicked = Some(i);
                }
                x += widths[i] + gap;
            }
        }
        clicked
    }
}

/// A divider marking a restorable point in the conversation: hairlines with a centered
/// "Restore checkpoint" pill. Returns a response whose `clicked()` means "restore".
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// if ui.add(glowkit_ai::Checkpoint::new().label("Restore checkpoint").time("2 min ago")).clicked() {
///     // roll back
/// }
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Checkpoint<'a> {
    label: &'a str,
    time: Option<&'a str>,
}

impl Default for Checkpoint<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Checkpoint<'a> {
    /// A "Restore checkpoint" divider.
    pub fn new() -> Self {
        Self { label: "Restore checkpoint", time: None }
    }
    /// Button label.
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = label;
        self
    }
    /// Muted time label after the button.
    pub fn time(mut self, time: &'a str) -> Self {
        self.time = Some(time);
        self
    }
}

impl Widget for Checkpoint<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = ui.available_width();
        let (rect, base) = ui.allocate_exact_size(vec2(w, 30.0), Sense::hover());
        let g = common::galley(ui, self.label, 12.5, t.muted_foreground);
        let tg = self.time.map(|s| common::galley(ui, s, 11.5, t.muted_foreground));
        let bw = g.size().x + 36.0;
        let tw = tg.as_ref().map(|g| g.size().x + 10.0).unwrap_or(0.0);
        let total = bw + tw;
        let br = Rect::from_min_size(pos2(rect.center().x - total / 2.0, rect.center().y - 14.0), vec2(bw, 28.0));
        let resp = ui.interact(br, base.id.with("checkpoint"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let h = common::anim_bool(ui, resp.id.with("h"), resp.hovered(), 0.12);
        let p = ui.painter();
        let y = rect.center().y;
        let line = with_alpha(t.border, 1.0);
        p.line_segment([pos2(rect.min.x, y), pos2(br.min.x - 12.0, y)], Stroke::new(1.0, line));
        p.line_segment([pos2(br.max.x + tw + 12.0, y), pos2(rect.max.x, y)], Stroke::new(1.0, line));
        p.rect_filled(br, CornerRadius::same(14), mix(t.card, common::raised(&t), 0.5 + h * 0.5));
        p.rect_stroke(br, CornerRadius::same(14), Stroke::new(1.0, mix(t.border, common::border_strong(&t), h)), StrokeKind::Inside);
        let col = t.muted_foreground.lerp_to_gamma(t.foreground, h);
        paint_icon(p, Icon::History, Rect::from_center_size(pos2(br.min.x + 15.0, br.center().y), vec2(13.0, 13.0)), col);
        p.galley(pos2(br.min.x + 26.0, br.center().y - g.size().y / 2.0), g, col);
        if let Some(tg) = tg {
            p.galley(pos2(br.max.x + 10.0, br.center().y - tg.size().y / 2.0), tg, with_alpha(t.muted_foreground, 0.75));
        }
        resp
    }
}

/// What happened in a [`WebPreview`] toolbar this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WebPreviewOutput {
    /// Back clicked.
    pub back: bool,
    /// Forward clicked.
    pub forward: bool,
    /// Reload clicked (or Enter pressed in the address bar).
    pub reload: bool,
    /// "Open in new tab" clicked.
    pub open_external: bool,
}

/// A browser-like frame for previewing generated web apps: back / forward / reload, an editable
/// address bar with a lock icon, an "open" button, then your content in the panel.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut url = String::from("https://preview.local/app");
/// glowkit_ai::WebPreview::new(&mut url).height(280.0).show(ui, |ui| {
///     ui.label("page content");
/// });
/// ```
#[must_use = "call `.show(ui, ..)`"]
pub struct WebPreview<'a> {
    url: &'a mut String,
    height: f32,
    loading: bool,
}

impl<'a> WebPreview<'a> {
    /// A preview frame showing `url` (editable).
    pub fn new(url: &'a mut String) -> Self {
        Self { url, height: 300.0, loading: false }
    }
    /// Height of the content panel.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Show a loading bar under the toolbar.
    pub fn loading(mut self, on: bool) -> Self {
        self.loading = on;
        self
    }

    /// Show the frame; `content` paints the page.
    pub fn show(self, ui: &mut Ui, content: impl FnOnce(&mut Ui)) -> WebPreviewOutput {
        let t = Theme::of(ui);
        let mut out = WebPreviewOutput::default();
        let w = ui.available_width();
        let bar_h = 48.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, bar_h + self.height), Sense::hover());
        let cr = t.corner_scaled(1.2);
        let p = ui.painter();
        p.rect_filled(rect, cr, t.card);
        let bar = Rect::from_min_size(rect.min, vec2(w, bar_h));
        p.rect_filled(bar, CornerRadius { nw: cr.nw, ne: cr.ne, sw: 0, se: 0 }, common::raised(&t));
        p.line_segment([pos2(rect.min.x, bar.max.y), pos2(rect.max.x, bar.max.y)], Stroke::new(1.0, t.border));
        // nav buttons
        let mut x = bar.min.x + 10.0;
        for (icon, tip, k) in [(Icon::ChevronLeft, "Back", 0), (Icon::ChevronRight, "Forward", 1), (Icon::Refresh, "Reload", 2)] {
            let r = Rect::from_min_size(pos2(x, bar.center().y - 14.0), vec2(28.0, 28.0));
            let mut child = ui.new_child(UiBuilder::new().max_rect(r));
            if common::icon_button(&mut child, icon, 28.0, None, tip).clicked() {
                match k {
                    0 => out.back = true,
                    1 => out.forward = true,
                    _ => out.reload = true,
                }
            }
            x += 30.0;
        }
        // open external on the right
        let er = Rect::from_min_size(pos2(bar.max.x - 38.0, bar.center().y - 14.0), vec2(28.0, 28.0));
        let mut child = ui.new_child(UiBuilder::new().max_rect(er));
        if common::icon_button(&mut child, Icon::External, 28.0, None, "Open in new tab").clicked() {
            out.open_external = true;
            ui.ctx().open_url(egui::OpenUrl::new_tab(self.url.as_str()));
        }
        // address bar
        let ar = Rect::from_min_max(pos2(x + 6.0, bar.center().y - 15.0), pos2(er.min.x - 8.0, bar.center().y + 15.0));
        let p = ui.painter();
        p.rect_filled(ar, CornerRadius::same(15), t.card);
        p.rect_stroke(ar, CornerRadius::same(15), Stroke::new(1.0, t.border), StrokeKind::Inside);
        paint_icon(p, Icon::Lock, Rect::from_center_size(pos2(ar.min.x + 16.0, ar.center().y), vec2(12.0, 12.0)), t.muted_foreground);
        let te_rect = Rect::from_min_max(pos2(ar.min.x + 28.0, ar.min.y), pos2(ar.max.x - 10.0, ar.max.y));
        let mut child = ui.new_child(UiBuilder::new().max_rect(te_rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
        let row_h = ui.fonts_mut(|f| f.row_height(&FontId::proportional(13.0)));
        let my = ((te_rect.height() - row_h) / 2.0).max(0.0) as i8;
        let te = egui::TextEdit::singleline(self.url)
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::symmetric(0, my))
            .font(FontId::proportional(13.0))
            .text_color(t.foreground)
            .desired_width(te_rect.width())
            .id_salt(resp.id.with("url"))
            .show(&mut child);
        if te.response.response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            out.reload = true;
        }
        if self.loading {
            let time = glowkit_core::time(ui);
            let lb = Rect::from_min_size(pos2(rect.min.x, bar.max.y - 1.0), vec2(w, 2.0));
            let head = (time * 0.7).fract();
            let x0 = lb.min.x + lb.width() * (head * 1.4 - 0.4).max(0.0);
            let x1 = lb.min.x + lb.width() * (head * 1.4).min(1.0);
            ui.painter().rect_filled(Rect::from_min_max(pos2(x0, lb.min.y), pos2(x1, lb.max.y)), CornerRadius::same(1), t.accent);
            glowkit_core::animate(ui);
        }
        let body = Rect::from_min_max(pos2(rect.min.x + 1.0, bar.max.y + 1.0), rect.max - vec2(1.0, 1.0));
        let mut child = ui.new_child(UiBuilder::new().max_rect(body.shrink(0.0)).layout(egui::Layout::top_down(egui::Align::Min)));
        child.set_clip_rect(body.intersect(ui.clip_rect()));
        content(&mut child);
        ui.painter().rect_stroke(rect, cr, Stroke::new(1.0, t.border), StrokeKind::Inside);
        out
    }
}
