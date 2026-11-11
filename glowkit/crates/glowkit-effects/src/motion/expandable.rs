use super::common::{icon_tile, theme_card};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::{paint, Theme};

/// One entry of an [`ExpandableCard`] list.
#[derive(Clone, Debug, PartialEq)]
pub struct ExpandableItem {
    /// Glyph on the item's artwork tile.
    pub icon: String,
    /// Title line.
    pub title: String,
    /// Secondary line.
    pub subtitle: String,
    /// Longer text revealed in the expanded panel.
    pub body: String,
    /// Label of the call-to-action button.
    pub action: String,
    /// Artwork gradient; `None` uses the theme chart palette.
    pub colors: Option<[Color32; 2]>,
}

impl ExpandableItem {
    /// An item with an icon, title and subtitle.
    pub fn new(icon: impl Into<String>, title: impl Into<String>, subtitle: impl Into<String>) -> Self {
        Self { icon: icon.into(), title: title.into(), subtitle: subtitle.into(), body: String::new(), action: "Open".into(), colors: None }
    }
    /// Text shown when expanded.
    pub fn body(mut self, s: impl Into<String>) -> Self {
        self.body = s.into();
        self
    }
    /// Call-to-action label.
    pub fn action(mut self, s: impl Into<String>) -> Self {
        self.action = s.into();
        self
    }
    /// Artwork gradient.
    pub fn colors(mut self, a: Color32, b: Color32) -> Self {
        self.colors = Some([a, b]);
        self
    }
}

#[derive(Clone, Copy, Default)]
struct ExpandState {
    open: Option<usize>,
    last: usize,
}

/// A list of compact cards; clicking one grows it in place into a large detail panel over a
/// dimmed backdrop, and clicking outside shrinks it back into its row.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{ExpandableCard, ExpandableItem};
/// let items = [ExpandableItem::new("🎧", "Lo-fi Beats", "Chillhop").body("Long description…")];
/// ui.add(ExpandableCard::new(&items));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ExpandableCard<'a> {
    items: &'a [ExpandableItem],
    row_height: f32,
    min_height: f32,
    panel_width: f32,
    autoplay: bool,
}

impl<'a> ExpandableCard<'a> {
    /// An expandable list over `items`.
    pub fn new(items: &'a [ExpandableItem]) -> Self {
        Self { items, row_height: 74.0, min_height: 380.0, panel_width: 460.0, autoplay: false }
    }
    /// Height of a collapsed row.
    pub fn row_height(mut self, h: f32) -> Self {
        self.row_height = h.max(40.0);
        self
    }
    /// Minimum height of the widget (room for the expanded panel).
    pub fn min_height(mut self, h: f32) -> Self {
        self.min_height = h.max(100.0);
        self
    }
    /// Maximum width of the expanded panel.
    pub fn panel_width(mut self, w: f32) -> Self {
        self.panel_width = w.max(160.0);
        self
    }
    /// Open and close items on its own while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for ExpandableCard<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.items.len();
        let gap = 8.0;
        let list_h = n as f32 * (self.row_height + gap);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), list_h.max(self.min_height)), Sense::click());
        let id = response.id;
        if n == 0 {
            return response;
        }
        let list_w = self.panel_width.min(rect.width());
        let list_top = rect.center().y - (list_h - gap) / 2.0;
        let rows: Vec<Rect> = (0..n)
            .map(|i| Rect::from_min_size(pos2(rect.center().x - list_w / 2.0, list_top + i as f32 * (self.row_height + gap)), vec2(list_w, self.row_height)))
            .collect();
        let mut st: ExpandState = glowkit_core::state(ui, id.with("st"), ExpandState::default);
        let target = Rect::from_center_size(rect.center(), vec2(list_w, (rect.height() - 16.0).min(560.0)));
        let pointer = response.hover_pos();
        if response.clicked() {
            if let Some(p) = response.interact_pointer_pos() {
                match st.open {
                    Some(_) => {
                        let close_btn = Rect::from_center_size(pos2(target.max.x - 22.0, target.min.y + 22.0), vec2(28.0, 28.0));
                        if !target.contains(p) || close_btn.contains(p) {
                            st.open = None;
                        }
                    }
                    None => {
                        if let Some(i) = rows.iter().position(|r| r.contains(p)) {
                            st.open = Some(i);
                            st.last = i;
                        }
                    }
                }
            }
        }
        if self.autoplay && !response.hovered() {
            let now = glowkit_core::time(ui);
            let slot = (now / 2.4).floor() as usize;
            st.open = if slot % 2 == 1 { Some((slot / 2) % n) } else { None };
            if let Some(i) = st.open {
                st.last = i;
            }
            glowkit_core::animate(ui);
        }
        glowkit_core::set_state(ui, id.with("st"), st);
        let p = glowkit_core::anim::spring(ui, id.with("p"), if st.open.is_some() { 1.0 } else { 0.0 }, 210.0, 24.0);
        let hover_row = if st.open.is_none() { pointer.and_then(|q| rows.iter().position(|r| r.contains(q))) } else { None };
        if hover_row.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let radius = (t.radius * 1.4).round();
        let expanding = p > 0.001;
        for (i, item) in self.items.iter().enumerate() {
            if expanding && i == st.last {
                // the travelling card is drawn last, leave a soft placeholder
                painter.rect_filled(rows[i], CornerRadius::same(radius as u8), with_alpha(t.muted, 0.5));
                continue;
            }
            let hv = glowkit_core::anim::follow(ui, id.with(("hv", i)), if hover_row == Some(i) { 1.0 } else { 0.0 }, 14.0);
            draw_row(&painter, &t, item, i, rows[i], radius, hv);
        }
        if expanding {
            let dim = if t.dark { Color32::BLACK } else { Color32::from_gray(250) };
            painter.rect_filled(rect, CornerRadius::same(t.radius as u8), with_alpha(dim, 0.62 * p.clamp(0.0, 1.0)));
            let i = st.last;
            let item = &self.items[i];
            let from = rows[i];
            let r = Rect::from_min_max(from.min.lerp(target.min, p), from.max.lerp(target.max, p));
            theme_card(&painter, &t, r, radius + 6.0 * p, 0.4 + p);
            let colors = item.colors.unwrap_or_else(|| {
                let a = t.chart[i % t.chart.len()];
                [shade(a, 0.08), shade(a, -0.15)]
            });
            // artwork grows from the row's icon tile into a header banner
            let tile_small = Rect::from_min_size(from.min + vec2(12.0, 11.0), vec2(from.height() - 22.0, from.height() - 22.0));
            let banner = Rect::from_min_size(target.min + vec2(12.0, 12.0), vec2(target.width() - 24.0, (target.height() * 0.42).min(210.0)));
            let art_min = tile_small.min.lerp(banner.min, p);
            let art_max = tile_small.max.lerp(banner.max, p);
            let art = Rect::from_min_max(art_min, art_max).translate(r.min - from.min.lerp(target.min, p));
            let art_r = 10.0 + 4.0 * p;
            paint::gradient_rect(&painter, art, art_r, 45.0, &colors);
            painter.text(art.center(), Align2::CENTER_CENTER, &item.icon, FontId::proportional(art.height() * 0.42), Color32::WHITE);
            // title block moves below the banner
            let row_text = pos2(tile_small.max.x + 14.0, from.center().y);
            let panel_text = pos2(target.min.x + 20.0, banner.max.y + 30.0);
            let tp = row_text.lerp(panel_text, p) + (r.min - from.min.lerp(target.min, p));
            painter.text(tp + vec2(0.0, -10.0 - 4.0 * p), Align2::LEFT_CENTER, &item.title, FontId::proportional(15.0 + 5.0 * p), t.foreground);
            painter.text(tp + vec2(0.0, 11.0 + 6.0 * p), Align2::LEFT_CENTER, &item.subtitle, FontId::proportional(13.0 + p), t.muted_foreground);
            // action button slides from the row's right edge
            let btn_row = Rect::from_center_size(pos2(from.max.x - 50.0, from.center().y), vec2(72.0, 32.0));
            let btn_panel = Rect::from_center_size(pos2(target.max.x - 62.0, banner.max.y + 34.0), vec2(92.0, 36.0));
            let btn = Rect::from_min_max(btn_row.min.lerp(btn_panel.min, p), btn_row.max.lerp(btn_panel.max, p));
            painter.rect_filled(btn, CornerRadius::same(99), mix(t.secondary, t.accent, p));
            painter.text(btn.center(), Align2::CENTER_CENTER, &item.action, FontId::proportional(13.0), mix(t.foreground, t.accent_foreground, p));
            // body fades in late
            let body_a = ((p - 0.55) / 0.45).clamp(0.0, 1.0);
            if body_a > 0.0 && !item.body.is_empty() {
                let wrap = target.width() - 40.0;
                let g = painter.layout(item.body.clone(), FontId::proportional(13.5), with_alpha(t.muted_foreground, body_a), wrap);
                let pos = pos2(target.min.x + 20.0, banner.max.y + 66.0 + (1.0 - body_a) * 10.0);
                let clip = painter.with_clip_rect(r.shrink(8.0));
                clip.galley(pos, g, t.muted_foreground);
                let close = Rect::from_center_size(pos2(target.max.x - 26.0, target.min.y + 26.0), vec2(26.0, 26.0));
                painter.circle(close.center(), 13.0, with_alpha(t.popover, 0.9 * body_a), Stroke::new(1.0, with_alpha(t.border, body_a)));
                painter.text(close.center(), Align2::CENTER_CENTER, "✕", FontId::proportional(12.0), with_alpha(t.foreground, body_a));
            }
        }
        response
    }
}

fn draw_row(painter: &egui::Painter, t: &Theme, item: &ExpandableItem, i: usize, r: Rect, radius: f32, hover: f32) {
    let bg = mix(Color32::TRANSPARENT, t.muted, hover);
    painter.rect(r, CornerRadius::same(radius as u8), bg, Stroke::new(1.0, with_alpha(t.border, hover)), StrokeKind::Inside);
    let colors = item.colors.unwrap_or_else(|| {
        let a = t.chart[i % t.chart.len()];
        [shade(a, 0.08), shade(a, -0.15)]
    });
    let tile = Rect::from_min_size(r.min + vec2(12.0, 11.0), vec2(r.height() - 22.0, r.height() - 22.0));
    icon_tile(painter, tile, 10.0, colors, &item.icon, 1.0);
    let tx = tile.max.x + 14.0;
    painter.text(pos2(tx, r.center().y - 10.0), Align2::LEFT_CENTER, &item.title, FontId::proportional(15.0), t.foreground);
    painter.text(pos2(tx, r.center().y + 11.0), Align2::LEFT_CENTER, &item.subtitle, FontId::proportional(13.0), t.muted_foreground);
    let btn = Rect::from_center_size(pos2(r.max.x - 50.0, r.center().y), vec2(72.0, 32.0));
    painter.rect_filled(btn, CornerRadius::same(99), mix(t.secondary, t.accent, hover));
    painter.text(btn.center(), Align2::CENTER_CENTER, &item.action, FontId::proportional(13.0), mix(t.foreground, t.accent_foreground, hover));
}
