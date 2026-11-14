//! Floating icon toolbar for canvases and nodes.

use crate::common;
use crate::icons::{paint_icon, Icon};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// One button of an [`AiToolbar`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToolbarItem<'a> {
    /// The icon.
    pub icon: Icon,
    /// Tooltip text.
    pub tooltip: &'a str,
    /// Keyboard shortcut shown in the tooltip.
    pub shortcut: Option<&'a str>,
    /// Drawn as toggled on.
    pub active: bool,
    /// Draw a divider before this item.
    pub separator: bool,
    /// Tint (used for destructive actions); default is the foreground color.
    pub color: Option<Color32>,
}

impl<'a> ToolbarItem<'a> {
    /// A button with `icon` and a `tooltip`.
    pub fn new(icon: Icon, tooltip: &'a str) -> Self {
        Self { icon, tooltip, shortcut: None, active: false, separator: false, color: None }
    }
    /// Keyboard shortcut hint.
    pub fn shortcut(mut self, s: &'a str) -> Self {
        self.shortcut = Some(s);
        self
    }
    /// Toggled-on state.
    pub fn active(mut self, a: bool) -> Self {
        self.active = a;
        self
    }
    /// Divider before this item.
    pub fn separator(mut self) -> Self {
        self.separator = true;
        self
    }
    /// Icon tint.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

/// A floating toolbar for canvases and graph nodes: a rounded, shadowed bar of icon buttons
/// with dividers, toggled states and quick tooltips (with shortcut hints) that fade in above
/// the hovered button. Lay it out inline with [`AiToolbar::show`] or float it over a canvas with
/// [`AiToolbar::show_at`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::{AiToolbar, Icon, ToolbarItem};
/// let items = [ToolbarItem::new(Icon::Pencil, "Edit"), ToolbarItem::new(Icon::Copy, "Duplicate").shortcut("⌘D")];
/// if let Some(i) = AiToolbar::new(&items).show(ui) {
///     // handle items[i]
/// }
/// ```
#[must_use = "call `.show(ui)` or `.show_at(ui, pos)`"]
pub struct AiToolbar<'a> {
    items: &'a [ToolbarItem<'a>],
    vertical: bool,
    button: f32,
    force_tooltip: Option<usize>,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> AiToolbar<'a> {
    /// A toolbar of `items`.
    pub fn new(items: &'a [ToolbarItem<'a>]) -> Self {
        Self { items, vertical: false, button: 32.0, force_tooltip: None, id_salt: None }
    }
    /// Stack the buttons vertically (tooltips open to the right).
    pub fn vertical(mut self, v: bool) -> Self {
        self.vertical = v;
        self
    }
    /// Button size in points (default 32).
    pub fn button_size(mut self, s: f32) -> Self {
        self.button = s;
        self
    }
    /// Always show the tooltip of this item (handy for previews and onboarding).
    pub fn force_tooltip(mut self, i: Option<usize>) -> Self {
        self.force_tooltip = i;
        self
    }
    /// Stable id.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// The size the toolbar takes.
    pub fn size(&self) -> egui::Vec2 {
        let pad = 4.0;
        let seps = self.items.iter().filter(|i| i.separator).count() as f32;
        let along = self.items.len() as f32 * (self.button + 2.0) - 2.0 + seps * 9.0 + pad * 2.0;
        let across = self.button + pad * 2.0;
        if self.vertical {
            vec2(across, along)
        } else {
            vec2(along, across)
        }
    }

    /// Float the toolbar so its bottom-center (top-left-center when vertical) sits at `anchor`.
    pub fn show_at(self, ui: &mut Ui, anchor: Pos2) -> Option<usize> {
        let s = self.size();
        let min = if self.vertical { pos2(anchor.x, anchor.y - s.y / 2.0) } else { pos2(anchor.x - s.x / 2.0, anchor.y - s.y) };
        let mut child = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(min, s)));
        self.show(&mut child)
    }

    /// Lay the toolbar out inline. Returns the index of the item clicked this frame.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = Theme::of(ui);
        let size = self.size();
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        let id = self.id_salt.map(|s| ui.scope_id().with_salt(s)).unwrap_or(resp.id).with("ai_toolbar");
        let cr = CornerRadius::same(((self.button + 8.0) * 0.32) as u8);
        let p = ui.painter();
        glowkit_core::paint::soft_shadow(p, rect, cr.nw as f32, 14.0, vec2(0.0, 6.0), Color32::from_black_alpha(if t.dark { 90 } else { 22 }));
        p.rect(rect, cr, t.popover, Stroke::new(1.0, t.border), StrokeKind::Inside);
        let mut cursor = if self.vertical { rect.min.y + 4.0 } else { rect.min.x + 4.0 };
        let mut clicked = None;
        let mut tip: Option<(Rect, usize, f32)> = None;
        for (i, item) in self.items.iter().enumerate() {
            if item.separator && i > 0 {
                let c = cursor + 3.5;
                let p = ui.painter();
                if self.vertical {
                    p.line_segment([pos2(rect.min.x + 9.0, c), pos2(rect.max.x - 9.0, c)], Stroke::new(1.0, t.border));
                } else {
                    p.line_segment([pos2(c, rect.min.y + 9.0), pos2(c, rect.max.y - 9.0)], Stroke::new(1.0, t.border));
                }
                cursor += 9.0;
            }
            let br = if self.vertical {
                Rect::from_min_size(pos2(rect.min.x + 4.0, cursor), vec2(self.button, self.button))
            } else {
                Rect::from_min_size(pos2(cursor, rect.min.y + 4.0), vec2(self.button, self.button))
            };
            cursor += self.button + 2.0;
            let r = ui.interact(br, id.with(i), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            let h = common::anim_bool(ui, id.with(("h", i)), r.hovered(), 0.1);
            let p = ui.painter();
            let bcr = CornerRadius::same((self.button * 0.26) as u8);
            if item.active {
                p.rect_filled(br, bcr, with_alpha(t.accent, if t.dark { 0.18 } else { 0.12 }));
            }
            if h > 0.0 {
                p.rect_filled(br, bcr, common::hover_fill(&t).gamma_multiply(h));
            }
            let base = item.color.unwrap_or(if item.active { t.accent } else { t.muted_foreground });
            let col = if item.color.is_some() || item.active { base } else { base.lerp_to_gamma(t.foreground, h) };
            let press = if r.is_pointer_button_down_on() { 0.86 } else { 1.0 };
            paint_icon(p, item.icon, Rect::from_center_size(br.center(), vec2(self.button * 0.5 * press, self.button * 0.5 * press)), col);
            if r.clicked() {
                clicked = Some(i);
            }
            let show = r.hovered() || self.force_tooltip == Some(i);
            let k = common::anim_bool(ui, id.with(("tip", i)), show, 0.12);
            if k > 0.0 {
                tip = Some((br, i, k));
            }
        }
        if let Some((br, i, k)) = tip {
            let item = &self.items[i];
            let tp = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, id.with("tip_layer")));
            let g = tp.layout_no_wrap(item.tooltip.to_owned(), FontId::proportional(12.0), t.background);
            let sg = item.shortcut.map(|s| tp.layout_no_wrap(s.to_owned(), FontId::proportional(11.0), with_alpha(t.background, 0.6)));
            let w = g.size().x + 16.0 + sg.as_ref().map(|s| s.size().x + 8.0).unwrap_or(0.0);
            let hgt = 26.0;
            let off = (1.0 - k) * 4.0;
            let tr = if self.vertical {
                Rect::from_min_size(pos2(rect.max.x + 8.0 + off, br.center().y - hgt / 2.0), vec2(w, hgt))
            } else {
                Rect::from_min_size(pos2(br.center().x - w / 2.0, rect.min.y - hgt - 8.0 - off), vec2(w, hgt))
            };
            let bg = with_alpha(t.foreground, k);
            tp.rect_filled(tr, CornerRadius::same(6), bg);
            let arrow = if self.vertical {
                vec![pos2(tr.min.x, tr.center().y - 4.0), pos2(tr.min.x - 4.0, tr.center().y), pos2(tr.min.x, tr.center().y + 4.0)]
            } else {
                vec![pos2(tr.center().x - 4.0, tr.max.y), pos2(tr.center().x + 4.0, tr.max.y), pos2(tr.center().x, tr.max.y + 4.0)]
            };
            tp.add(egui::Shape::convex_polygon(arrow, bg, Stroke::NONE));
            tp.galley_with_override_text_color(pos2(tr.min.x + 8.0, tr.center().y - g.size().y / 2.0), g, t.background.gamma_multiply(k));
            if let Some(s) = item.shortcut {
                tp.text(pos2(tr.max.x - 8.0, tr.center().y), Align2::RIGHT_CENTER, s, FontId::proportional(11.0), with_alpha(t.background, 0.6 * k));
            }
        }
        clicked
    }
}
