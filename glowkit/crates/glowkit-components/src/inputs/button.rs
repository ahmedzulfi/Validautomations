use super::kit::{self, control_radius, ControlSize, UiIcon};
use egui::{
    vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget,
};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// Visual style of a [`Button`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonVariant {
    /// Solid primary fill: the main call to action.
    #[default]
    Default,
    /// Soft neutral fill.
    Secondary,
    /// Hairline border, transparent fill.
    Outline,
    /// No chrome until hovered.
    Ghost,
    /// Solid red, for destructive actions.
    Destructive,
    /// Looks like a text link, underlines on hover.
    Link,
}

/// A design-system button with variants, sizes, icons and a loading state.
///
/// Presses with a subtle spring "tap" scale, lifts its fill on hover, and swaps its icon for a
/// spinner while loading.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::{Button, ButtonVariant, UiIcon};
/// if ui.add(Button::new("Save changes")).clicked() {}
/// ui.add(Button::new("Delete").variant(ButtonVariant::Destructive).icon(UiIcon::Trash));
/// ui.add(Button::new("Saving").loading(true));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Button {
    text: String,
    variant: ButtonVariant,
    size: ControlSize,
    icon: Option<UiIcon>,
    trailing: Option<UiIcon>,
    loading: bool,
    disabled: bool,
    width: Option<f32>,
    corners: Option<CornerRadius>,
    fill: Option<Color32>,
    selected: bool,
}

impl Button {
    /// A button with a text label (pass `""` together with [`Button::icon`] for an icon button).
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            variant: ButtonVariant::Default,
            size: ControlSize::Md,
            icon: None,
            trailing: None,
            loading: false,
            disabled: false,
            width: None,
            corners: None,
            fill: None,
            selected: false,
        }
    }
    /// A square icon-only button.
    pub fn icon_only(icon: UiIcon) -> Self {
        Self::new("").icon(icon)
    }
    /// Visual variant.
    pub fn variant(mut self, v: ButtonVariant) -> Self {
        self.variant = v;
        self
    }
    /// Size: 32 / 36 / 40 points tall.
    pub fn size(mut self, s: ControlSize) -> Self {
        self.size = s;
        self
    }
    /// Shortcut for [`ControlSize::Sm`].
    pub fn small(self) -> Self {
        self.size(ControlSize::Sm)
    }
    /// Shortcut for [`ControlSize::Lg`].
    pub fn large(self) -> Self {
        self.size(ControlSize::Lg)
    }
    /// Leading icon.
    pub fn icon(mut self, icon: UiIcon) -> Self {
        self.icon = Some(icon);
        self
    }
    /// Trailing icon (e.g. an arrow or chevron).
    pub fn trailing_icon(mut self, icon: UiIcon) -> Self {
        self.trailing = Some(icon);
        self
    }
    /// Show a spinner and ignore clicks.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
    /// Dim the button and ignore clicks.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    /// Fixed width (default: fit content). Use `f32::INFINITY` to fill the row.
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Override corner radii (used by [`ButtonGroup`] to join buttons).
    pub fn corners(mut self, c: CornerRadius) -> Self {
        self.corners = Some(c);
        self
    }
    /// Override the fill color of solid variants.
    pub fn fill(mut self, c: Color32) -> Self {
        self.fill = Some(c);
        self
    }
    /// Render in the "active" state (used by groups and toolbars).
    pub fn selected(mut self, s: bool) -> Self {
        self.selected = s;
        self
    }
}

struct Palette {
    fill: Color32,
    hover_fill: Color32,
    text: Color32,
    border: Option<Color32>,
}

fn palette(t: &Theme, v: ButtonVariant, fill: Option<Color32>) -> Palette {
    let hover_neutral =
        if t.dark { mix(t.secondary, t.foreground, 0.06) } else { mix(t.secondary, t.foreground, 0.03) };
    match v {
        ButtonVariant::Default => {
            let f = fill.unwrap_or(t.primary);
            Palette {
                fill: f,
                hover_fill: mix(f, t.background, 0.12),
                text: if fill.is_some() { glowkit_core::color::contrast_text(f) } else { t.primary_foreground },
                border: None,
            }
        }
        ButtonVariant::Secondary => {
            Palette { fill: t.secondary, hover_fill: hover_neutral, text: t.secondary_foreground, border: None }
        }
        ButtonVariant::Outline => Palette {
            fill: if t.dark { with_alpha(t.input, 0.3) } else { t.background },
            hover_fill: if t.dark { with_alpha(t.input, 0.6) } else { t.secondary },
            text: t.foreground,
            border: Some(t.input),
        },
        ButtonVariant::Ghost => Palette {
            fill: Color32::TRANSPARENT,
            hover_fill: if t.dark { with_alpha(t.secondary, 0.8) } else { t.secondary },
            text: t.foreground,
            border: None,
        },
        ButtonVariant::Destructive => {
            let f = fill.unwrap_or(t.destructive);
            Palette { fill: f, hover_fill: mix(f, t.background, 0.12), text: Color32::WHITE, border: None }
        }
        ButtonVariant::Link => {
            Palette { fill: Color32::TRANSPARENT, hover_fill: Color32::TRANSPARENT, text: t.foreground, border: None }
        }
    }
}

impl Widget for Button {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let h = self.size.height();
        let fs = self.size.font();
        let is = self.size.icon();
        let font = FontId::proportional(fs);
        let galley = ui.painter().layout_no_wrap(self.text.clone(), font, Color32::PLACEHOLDER);
        let has_text = !self.text.is_empty();
        let lead = self.icon.is_some() || self.loading;
        let gap = 8.0;
        let mut content = galley.size().x;
        if lead {
            content += is + if has_text { gap } else { 0.0 };
        }
        if self.trailing.is_some() {
            content += is + if has_text { gap } else { 0.0 };
        }
        let pad = if has_text { self.size.pad_x() - if lead { 2.0 } else { 0.0 } } else { (h - is) / 2.0 };
        let mut w = if has_text { content + pad * 2.0 } else { h };
        if self.variant == ButtonVariant::Link {
            w = content + 4.0;
        }
        if let Some(fw) = self.width {
            w = if fw.is_finite() { fw } else { ui.available_width() };
        }
        let sense = if self.disabled || self.loading { Sense::hover() } else { Sense::click() };
        let (rect, mut resp) = ui.allocate_exact_size(vec2(w, h), sense);
        if !self.disabled && !self.loading {
            resp = resp.on_hover_cursor(CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        let hover = kit::anim(ui, id.with("h"), resp.hovered() || self.selected, 0.12);
        let pressed = resp.is_pointer_button_down_on();
        let scale = glowkit_core::anim::spring(ui, id.with("press"), if pressed { 0.965 } else { 1.0 }, 500.0, 22.0);
        let focus = kit::anim(ui, id.with("f"), resp.has_focus(), 0.15);
        let p = palette(&t, self.variant, self.fill);
        let alpha = if self.disabled {
            0.5
        } else if self.loading {
            0.9
        } else {
            1.0
        };
        let fade = |c: Color32| c.gamma_multiply(alpha);
        let r = self.corners.unwrap_or_else(|| control_radius(&t));
        let painter = ui.painter();
        let body = Rect::from_center_size(rect.center(), rect.size() * scale);
        let fill = p.fill.lerp_to_gamma(p.hover_fill, hover);
        if matches!(self.variant, ButtonVariant::Default | ButtonVariant::Destructive | ButtonVariant::Outline)
            && !self.disabled
        {
            glowkit_core::paint::soft_shadow(
                painter,
                body,
                r.nw as f32,
                3.0,
                vec2(0.0, 1.0),
                Color32::from_black_alpha(if t.dark { 30 } else { 14 }),
            );
        }
        painter.rect(
            body,
            r,
            fade(fill),
            p.border.map(|b| Stroke::new(1.0, fade(b))).unwrap_or(Stroke::NONE),
            StrokeKind::Inside,
        );
        kit::focus_halo(painter, body, r, t.ring, focus);

        // content
        let mut x = body.center().x - content / 2.0;
        let cy = body.center().y;
        let text_col = fade(p.text);
        if lead {
            let ir = Rect::from_center_size(egui::pos2(x + is / 2.0, cy), vec2(is, is));
            if self.loading {
                kit::mini_spinner(painter, ir.center(), is * 0.42, text_col, glowkit_core::time(ui));
                glowkit_core::animate(ui);
            } else if let Some(icon) = self.icon {
                icon.paint(painter, ir, text_col);
            }
            x += is + if has_text { gap } else { 0.0 };
        }
        if has_text {
            let gpos = egui::pos2(x, cy - galley.size().y / 2.0);
            painter.galley(gpos, galley.clone(), text_col);
            if self.variant == ButtonVariant::Link && hover > 0.0 {
                let y = gpos.y + galley.size().y - 1.0;
                painter.line_segment(
                    [egui::pos2(x, y), egui::pos2(x + galley.size().x * hover, y)],
                    Stroke::new(1.0, text_col),
                );
            }
            x += galley.size().x + gap;
        }
        if let Some(icon) = self.trailing {
            let nudge =
                if self.variant != ButtonVariant::Link && matches!(icon, UiIcon::ArrowRight | UiIcon::ChevronRight) {
                    2.0 * hover
                } else {
                    0.0
                };
            icon.paint(painter, Rect::from_center_size(egui::pos2(x + is / 2.0 + nudge, cy), vec2(is, is)), text_col);
        }
        let _ = Align2::CENTER_CENTER;
        resp
    }
}
