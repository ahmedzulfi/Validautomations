use super::kit::{self, ControlSize};
use egui::{
    pos2, vec2, Color32, CornerRadius, CursorIcon, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget,
};
use glowkit_core::anim::spring;
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// Lay out `[control] label / description` and return (control rect, full response).
fn labeled(
    ui: &mut Ui,
    control: egui::Vec2,
    label: &Option<String>,
    desc: &Option<String>,
    gap: f32,
    enabled: bool,
) -> (Rect, Response) {
    let t = Theme::of(ui);
    let font = FontId::proportional(14.0);
    let lg = label.as_ref().map(|l| ui.painter().layout_no_wrap(l.clone(), font, t.foreground));
    let dg = desc.as_ref().map(|d| {
        ui.painter().layout(
            d.clone(),
            FontId::proportional(12.5),
            t.muted_foreground,
            (ui.available_width() - control.x - gap).max(120.0),
        )
    });
    let tw = lg.as_ref().map(|g| g.size().x).unwrap_or(0.0).max(dg.as_ref().map(|g| g.size().x).unwrap_or(0.0));
    let th = lg.as_ref().map(|g| g.size().y).unwrap_or(0.0) + dg.as_ref().map(|g| g.size().y + 3.0).unwrap_or(0.0);
    let w = control.x + if tw > 0.0 { gap + tw } else { 0.0 };
    let h = control.y.max(th);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), if enabled { Sense::click() } else { Sense::hover() });
    let resp = if enabled { resp.on_hover_cursor(CursorIcon::PointingHand) } else { resp };
    let first_line_h = lg.as_ref().map(|g| g.size().y).unwrap_or(control.y);
    let cy = rect.min.y + first_line_h.max(control.y) / 2.0;
    let crect = Rect::from_center_size(pos2(rect.min.x + control.x / 2.0, cy), control);
    let alpha = if enabled { 1.0 } else { 0.5 };
    let mut y = rect.min.y + (first_line_h.max(control.y) - first_line_h) / 2.0;
    if let Some(g) = lg {
        let gh = g.size().y;
        ui.painter().galley(pos2(crect.max.x + gap, y), g, t.foreground.gamma_multiply(alpha));
        y += gh + 3.0;
    }
    if let Some(g) = dg {
        ui.painter().galley(pos2(crect.max.x + gap, y), g, t.muted_foreground.gamma_multiply(alpha));
    }
    (crect, resp)
}

/// An on/off switch with a springy thumb that stretches while pressed.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut on = true;
/// use glowkit_components::Switch;
/// ui.add(Switch::new(&mut on).label("Airplane mode"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Switch<'a> {
    on: &'a mut bool,
    label: Option<String>,
    description: Option<String>,
    size: ControlSize,
    color: Option<Color32>,
    disabled: bool,
}

impl<'a> Switch<'a> {
    /// A switch bound to `on`.
    pub fn new(on: &'a mut bool) -> Self {
        Self { on, label: None, description: None, size: ControlSize::Md, color: None, disabled: false }
    }
    /// Text to the right of the switch.
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }
    /// Muted second line under the label.
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
    /// Size: Sm 28×16, Md 36×20, Lg 44×24.
    pub fn size(mut self, s: ControlSize) -> Self {
        self.size = s;
        self
    }
    /// Track color when on (default: theme primary).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Dim and ignore clicks.
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }
}

impl Widget for Switch<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (w, h) = match self.size {
            ControlSize::Sm => (28.0, 16.0),
            ControlSize::Md => (36.0, 20.0),
            ControlSize::Lg => (44.0, 24.0),
        };
        let (track, mut resp) = labeled(ui, vec2(w, h), &self.label, &self.description, 10.0, !self.disabled);
        if resp.clicked() {
            *self.on = !*self.on;
            resp.mark_changed();
        }
        if !ui.is_rect_visible(track) {
            return resp;
        }
        let id = resp.id;
        let on = spring(ui, id.with("pos"), if *self.on { 1.0 } else { 0.0 }, 420.0, 24.0);
        let col_k = kit::anim(ui, id.with("col"), *self.on, 0.18);
        let pressed = resp.is_pointer_button_down_on();
        let stretch = spring(ui, id.with("stretch"), if pressed { 1.0 } else { 0.0 }, 500.0, 30.0);
        let hover = kit::anim(ui, id.with("h"), resp.hovered(), 0.12);
        let alpha = if self.disabled { 0.5 } else { 1.0 };
        let on_col = self.color.unwrap_or(t.primary);
        let off_col = if t.dark {
            t.input.lerp_to_gamma(t.muted_foreground, 0.15 + 0.1 * hover)
        } else {
            t.input.lerp_to_gamma(t.muted_foreground, 0.1 * hover)
        };
        let painter = ui.painter();
        let r = CornerRadius::same((h / 2.0) as u8);
        painter.rect_filled(track, r, off_col.lerp_to_gamma(on_col, col_k).gamma_multiply(alpha));
        let pad = 2.0;
        let d = h - pad * 2.0;
        let extra = d * 0.3 * stretch;
        let travel = w - pad * 2.0 - d - extra;
        let x0 = track.min.x + pad + travel * on;
        let thumb = Rect::from_min_size(pos2(x0, track.min.y + pad), vec2(d + extra, d));
        let thumb_col = if t.dark {
            t.foreground.lerp_to_gamma(if self.color.is_some() { Color32::WHITE } else { t.primary_foreground }, col_k)
        } else {
            t.background
        };
        glowkit_core::paint::soft_shadow(
            painter,
            thumb,
            d / 2.0,
            3.0,
            vec2(0.0, 1.0),
            Color32::from_black_alpha(if t.dark { 50 } else { 40 }),
        );
        painter.rect_filled(thumb, CornerRadius::same((d / 2.0) as u8), thumb_col.gamma_multiply(alpha));
        kit::focus_halo(painter, track, r, t.ring, kit::anim(ui, id.with("f"), resp.has_focus(), 0.15));
        resp
    }
}

/// A checkbox whose check mark draws itself like a pen stroke; supports an indeterminate state.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut agree = false;
/// use glowkit_components::Checkbox;
/// ui.add(Checkbox::new(&mut agree).label("Accept terms and conditions"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    label: Option<String>,
    description: Option<String>,
    indeterminate: bool,
    disabled: bool,
    color: Option<Color32>,
    size: f32,
}

impl<'a> Checkbox<'a> {
    /// A checkbox bound to `checked`.
    pub fn new(checked: &'a mut bool) -> Self {
        Self { checked, label: None, description: None, indeterminate: false, disabled: false, color: None, size: 16.0 }
    }
    /// Text to the right.
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }
    /// Muted second line under the label.
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
    /// Show a dash instead of a check (e.g. "some rows selected"). Clicking sets it to checked.
    pub fn indeterminate(mut self, i: bool) -> Self {
        self.indeterminate = i;
        self
    }
    /// Dim and ignore clicks.
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }
    /// Fill color when checked (default theme primary).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Box size in points (default 16).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
}

/// Paint a checkbox box at `rect` (shared with tables).
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_checkbox(
    ui: &Ui,
    id: egui::Id,
    rect: Rect,
    checked: bool,
    indeterminate: bool,
    hover: bool,
    color: Option<Color32>,
    alpha: f32,
) {
    let t = Theme::of(ui);
    let on = checked || indeterminate;
    let k = kit::anim(ui, id.with("fill"), on, 0.14);
    let draw = kit::anim(ui, id.with("draw"), checked && !indeterminate, 0.22);
    let dash = kit::anim(ui, id.with("dash"), indeterminate, 0.18);
    let h = kit::anim(ui, id.with("hov"), hover, 0.12);
    let fill_on = color.unwrap_or(t.primary);
    let fg = if color.is_some() { Color32::WHITE } else { t.primary_foreground };
    let painter = ui.painter();
    let r = CornerRadius::same(((rect.width() * 0.25).round()) as u8);
    let border =
        t.input.lerp_to_gamma(t.muted_foreground, 0.35 * h + if t.dark { 0.15 } else { 0.1 }).lerp_to_gamma(fill_on, k);
    let bg = kit::field_fill(&t).lerp_to_gamma(fill_on, k);
    if !t.dark {
        glowkit_core::paint::soft_shadow(painter, rect, 4.0, 2.0, vec2(0.0, 1.0), Color32::from_black_alpha(10));
    }
    painter.rect(rect, r, bg.gamma_multiply(alpha), Stroke::new(1.0, border.gamma_multiply(alpha)), StrokeKind::Inside);
    let inner = rect.shrink(rect.width() * 0.12);
    kit::check_path(painter, inner, fg.gamma_multiply(alpha), (rect.width() / 8.0).max(1.6), draw);
    if dash > 0.0 {
        let c = rect.center();
        let hw = rect.width() * 0.28 * dash;
        painter.line_segment(
            [pos2(c.x - hw, c.y), pos2(c.x + hw, c.y)],
            Stroke::new((rect.width() / 8.0).max(1.6), fg.gamma_multiply(alpha)),
        );
    }
}

impl Widget for Checkbox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let s = self.size;
        let (bx, mut resp) = labeled(ui, vec2(s, s), &self.label, &self.description, 9.0, !self.disabled);
        if resp.clicked() {
            *self.checked = if self.indeterminate { true } else { !*self.checked };
            resp.mark_changed();
        }
        if ui.is_rect_visible(bx) {
            let pressed = resp.is_pointer_button_down_on();
            let sc = spring(ui, resp.id.with("press"), if pressed { 0.88 } else { 1.0 }, 600.0, 20.0);
            let bx = Rect::from_center_size(bx.center(), bx.size() * sc);
            paint_checkbox(
                ui,
                resp.id,
                bx,
                *self.checked,
                self.indeterminate && !resp.clicked(),
                resp.hovered(),
                self.color,
                if self.disabled { 0.5 } else { 1.0 },
            );
            let t = Theme::of(ui);
            kit::focus_halo(
                ui.painter(),
                bx,
                CornerRadius::same(4),
                t.ring,
                kit::anim(ui, resp.id.with("f"), resp.has_focus(), 0.15),
            );
        }
        resp
    }
}
