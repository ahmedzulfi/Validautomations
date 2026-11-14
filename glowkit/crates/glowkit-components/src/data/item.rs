//! A flexible list-item row: media, title, description and actions.

use crate::icon::Icon;
use crate::inputs::kit;
use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, FontId, Layout, Rect, Response, Sense, Stroke, StrokeKind, Ui,
    UiBuilder, Widget,
};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

type UiFn<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// Surface style of an [`Item`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ItemVariant {
    /// Transparent until hovered (for lists inside cards).
    #[default]
    Default,
    /// Hairline border, card fill.
    Outline,
    /// Soft muted fill, no border.
    Muted,
}

/// Density of an [`Item`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ItemSize {
    /// Roomy: 16 pt padding, 40 pt media.
    #[default]
    Default,
    /// Compact: 10 pt padding, 32 pt media.
    Sm,
}

/// What sits on the left of an [`Item`].
#[derive(Clone, Debug, PartialEq)]
pub enum ItemMedia {
    /// An icon in a rounded tile.
    Icon(Icon),
    /// An icon in a tile tinted with a color.
    TintedIcon(Icon, Color32),
    /// Round avatar with initials of the given name.
    Avatar(String),
    /// A gradient "image" thumbnail (placeholder art) seeded by the index.
    Image(u32),
}

/// A list row in the shadcn "Item" layout: optional media (icon tile, avatar or thumbnail), a
/// title with a muted description, and trailing actions (buttons, badges, a chevron). Comes in
/// default, outline and muted variants and two sizes; clickable rows highlight on hover.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::{Button, ButtonVariant, Item, ItemVariant, UiIcon};
/// Item::new("Two-factor authentication")
///     .description("Verify via email or phone number.")
///     .icon(UiIcon::Lock)
///     .variant(ItemVariant::Outline)
///     .actions(|ui| { ui.add(Button::new("Enable").small()); })
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)` or add it with `ui.add(...)`"]
pub struct Item<'a> {
    title: String,
    description: Option<String>,
    media: Option<ItemMedia>,
    variant: ItemVariant,
    size: ItemSize,
    actions: Option<UiFn<'a>>,
    actions_width: f32,
    chevron: bool,
    clickable: bool,
    selected: bool,
    width: Option<f32>,
}

impl<'a> Item<'a> {
    /// A row titled `title`.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            media: None,
            variant: ItemVariant::Default,
            size: ItemSize::Default,
            actions: None,
            actions_width: 140.0,
            chevron: false,
            clickable: false,
            selected: false,
            width: None,
        }
    }
    /// Muted second line (wraps).
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
    /// Icon tile on the left.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.media = Some(ItemMedia::Icon(icon.into()));
        self
    }
    /// Any media on the left.
    pub fn media(mut self, m: ItemMedia) -> Self {
        self.media = Some(m);
        self
    }
    /// Surface style.
    pub fn variant(mut self, v: ItemVariant) -> Self {
        self.variant = v;
        self
    }
    /// Density.
    pub fn size(mut self, s: ItemSize) -> Self {
        self.size = s;
        self
    }
    /// Shortcut for `size(ItemSize::Sm)`.
    pub fn small(self) -> Self {
        self.size(ItemSize::Sm)
    }
    /// Trailing actions, laid out right-to-left within [`Item::actions_width`].
    pub fn actions(mut self, f: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.actions = Some(Box::new(f));
        self
    }
    /// Width reserved for the actions (default 140).
    pub fn actions_width(mut self, w: f32) -> Self {
        self.actions_width = w;
        self
    }
    /// Trailing chevron (implies clickable).
    pub fn chevron(mut self, c: bool) -> Self {
        self.chevron = c;
        self.clickable |= c;
        self
    }
    /// Highlight on hover and show a pointer cursor.
    pub fn clickable(mut self, c: bool) -> Self {
        self.clickable = c;
        self
    }
    /// Draw as selected (accent tint and ring).
    pub fn selected(mut self, s: bool) -> Self {
        self.selected = s;
        self
    }
    /// Outer width (default: available width).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }

    /// Show the row. The response is clickable when [`Item::clickable`] or [`Item::chevron`] is set.
    pub fn show(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = self.width.unwrap_or_else(|| ui.available_width());
        let (pad, media_s, title_s, desc_s, gap) = match self.size {
            ItemSize::Default => (16.0, 40.0, 14.5, 13.0, 14.0),
            ItemSize::Sm => (10.0, 32.0, 13.5, 12.5, 12.0),
        };
        let pad_x = pad;
        let pad_y = if self.size == ItemSize::Sm { 9.0 } else { pad - 2.0 };
        let has_media = self.media.is_some();
        let trailing = if self.actions.is_some() { self.actions_width } else { 0.0 }
            + if self.chevron { 24.0 } else { 0.0 };
        let text_x = pad_x + if has_media { media_s + gap } else { 0.0 };
        let text_w = (w - text_x - pad_x - trailing - if trailing > 0.0 { 10.0 } else { 0.0 }).max(40.0);
        let title_g = ui.painter().layout(self.title.clone(), FontId::proportional(title_s), t.foreground, text_w);
        let desc_g = self
            .description
            .as_ref()
            .map(|d| ui.painter().layout(d.clone(), FontId::proportional(desc_s), t.muted_foreground, text_w));
        let text_h = title_g.size().y + desc_g.as_ref().map(|g| g.size().y + 3.0).unwrap_or(0.0);
        let h = (text_h.max(if has_media { media_s } else { 0.0 }).max(28.0)) + pad_y * 2.0;
        let sense = if self.clickable { Sense::click() } else { Sense::hover() };
        let (rect, mut resp) = ui.allocate_exact_size(vec2(w, h), sense);
        if self.clickable {
            resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        let hov = kit::anim(ui, resp.id.with("item_h"), self.clickable && resp.hovered(), 0.14);
        let cr = t.corner_scaled(0.9);
        if ui.is_rect_visible(rect) {
            let p = ui.painter();
            let hover_c = with_alpha(t.foreground, if t.dark { 0.05 } else { 0.035 });
            match self.variant {
                ItemVariant::Default => {}
                ItemVariant::Outline => {
                    p.rect(rect, cr, t.card, Stroke::new(1.0, t.border), StrokeKind::Inside);
                }
                ItemVariant::Muted => {
                    p.rect_filled(rect, cr, if t.dark { mix(t.muted, t.card, 0.3) } else { t.muted });
                }
            }
            if hov > 0.0 {
                p.rect_filled(rect, cr, hover_c.gamma_multiply(hov));
            }
            if self.selected {
                p.rect_filled(rect, cr, with_alpha(t.accent, if t.dark { 0.10 } else { 0.07 }));
                p.rect_stroke(rect, cr, Stroke::new(1.0, with_alpha(t.accent, 0.55)), StrokeKind::Inside);
            }
            // media
            if let Some(m) = &self.media {
                let mr = Rect::from_min_size(pos2(rect.min.x + pad_x, rect.center().y - media_s / 2.0), vec2(media_s, media_s));
                paint_media(ui, m, mr, &t, self.variant);
            }
            // text block, vertically centered
            let ty = rect.center().y - text_h / 2.0;
            let p = ui.painter();
            p.galley(pos2(rect.min.x + text_x, ty), title_g.clone(), t.foreground);
            if let Some(g) = desc_g {
                p.galley(pos2(rect.min.x + text_x, ty + title_g.size().y + 3.0), g, t.muted_foreground);
            }
            if self.chevron {
                let c = pos2(rect.max.x - pad_x - 6.0 + 3.0 * hov, rect.center().y);
                crate::inputs::UiIcon::ChevronRight.paint(
                    p,
                    Rect::from_center_size(c, vec2(16.0, 16.0)),
                    t.muted_foreground.lerp_to_gamma(t.foreground, hov),
                );
            }
        }
        if let Some(actions) = self.actions {
            let right = rect.max.x - pad_x - if self.chevron { 24.0 } else { 0.0 };
            let ar = Rect::from_min_max(pos2(right - self.actions_width, rect.min.y), pos2(right, rect.max.y));
            let mut child =
                ui.new_child(UiBuilder::new().max_rect(ar).layout(Layout::right_to_left(Align::Center)));
            child.spacing_mut().item_spacing.x = 6.0;
            actions(&mut child);
        }
        resp
    }
}

impl Widget for Item<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui)
    }
}

fn paint_media(ui: &Ui, m: &ItemMedia, r: Rect, t: &Theme, variant: ItemVariant) {
    let p = ui.painter();
    let cr = CornerRadius::same((r.width() * 0.24) as u8);
    match m {
        ItemMedia::Icon(icon) => {
            let fill = match variant {
                ItemVariant::Muted => t.card,
                _ => t.muted,
            };
            p.rect(r, cr, fill, Stroke::new(1.0, t.border), StrokeKind::Inside);
            icon.paint(p, r.shrink(r.width() * 0.27), t.foreground);
        }
        ItemMedia::TintedIcon(icon, c) => {
            p.rect(
                r,
                cr,
                with_alpha(*c, if t.dark { 0.16 } else { 0.11 }),
                Stroke::new(1.0, with_alpha(*c, 0.28)),
                StrokeKind::Inside,
            );
            icon.paint(p, r.shrink(r.width() * 0.27), *c);
        }
        ItemMedia::Avatar(name) => {
            let initials: String = name.split_whitespace().filter_map(|w| w.chars().next()).take(2).collect();
            let hue = name.bytes().fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32)) % 360;
            let c = glowkit_core::hsv(hue as f32, 0.5, if t.dark { 0.85 } else { 0.6 });
            p.circle_filled(r.center(), r.width() / 2.0, with_alpha(c, if t.dark { 0.22 } else { 0.16 }));
            p.circle_stroke(r.center(), r.width() / 2.0, Stroke::new(1.0, with_alpha(c, 0.35)));
            p.text(r.center(), Align2::CENTER_CENTER, initials, FontId::proportional(r.width() * 0.36), c);
        }
        ItemMedia::Image(seed) => {
            let h = (*seed as f32 * 67.0) % 360.0;
            let stops = [glowkit_core::hsv(h, 0.6, 0.95), glowkit_core::hsv(h + 50.0, 0.7, 0.75)];
            glowkit_core::paint::gradient_rect(p, r, r.width() * 0.24, 135.0, &stops);
            // a little "landscape" glyph
            let c = Color32::from_white_alpha(170);
            p.circle_filled(r.min + vec2(r.width() * 0.68, r.height() * 0.32), r.width() * 0.09, c);
            let base = r.max.y - r.height() * 0.22;
            p.add(egui::Shape::convex_polygon(
                vec![
                    pos2(r.min.x + r.width() * 0.16, base),
                    pos2(r.min.x + r.width() * 0.42, r.min.y + r.height() * 0.42),
                    pos2(r.min.x + r.width() * 0.66, base),
                ],
                c,
                Stroke::NONE,
            ));
        }
    }
}
