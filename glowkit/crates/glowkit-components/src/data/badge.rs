use crate::inputs::kit::{self, UiIcon};
use egui::{pos2, vec2, Color32, CornerRadius, CursorIcon, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// Visual style of a [`Badge`] or [`Chip`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BadgeVariant {
    /// Solid primary.
    #[default]
    Default,
    /// Soft neutral.
    Secondary,
    /// Hairline border only.
    Outline,
    /// Soft red.
    Destructive,
    /// Soft green.
    Success,
    /// Soft amber.
    Warning,
    /// Soft blue.
    Info,
    /// Soft accent (brand) tint.
    Accent,
}

/// (fill, text, border) for a variant.
pub(crate) fn badge_colors(t: &Theme, v: BadgeVariant) -> (Color32, Color32, Color32) {
    let soft = |c: Color32| {
        let fill = with_alpha(c, if t.dark { 0.16 } else { 0.10 });
        let text = if t.dark { mix(c, Color32::WHITE, 0.25) } else { mix(c, Color32::BLACK, 0.15) };
        (fill, text, with_alpha(c, if t.dark { 0.28 } else { 0.22 }))
    };
    match v {
        BadgeVariant::Default => (t.primary, t.primary_foreground, t.primary),
        BadgeVariant::Secondary => (t.secondary, t.secondary_foreground, t.secondary),
        BadgeVariant::Outline => (Color32::TRANSPARENT, t.foreground, t.border),
        BadgeVariant::Destructive => soft(t.destructive),
        BadgeVariant::Success => soft(t.success),
        BadgeVariant::Warning => soft(t.warning),
        BadgeVariant::Info => soft(t.info),
        BadgeVariant::Accent => soft(t.accent),
    }
}

#[derive(Clone)]
struct CountState {
    shown: i64,
    prev: i64,
    at: f64,
}

/// A small status label. Can carry a leading dot or icon, and animate a numeric count (digits
/// roll up or down when the number changes).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::{Badge, BadgeVariant};
/// ui.add(Badge::new("Active").variant(BadgeVariant::Success).dot(true));
/// ui.add(Badge::count(12));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Badge {
    text: String,
    variant: BadgeVariant,
    dot: bool,
    icon: Option<UiIcon>,
    count: Option<i64>,
    pill: bool,
    color: Option<Color32>,
    size: f32,
}

impl Badge {
    /// A text badge.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), variant: BadgeVariant::Default, dot: false, icon: None, count: None, pill: false, color: None, size: 12.0 }
    }
    /// A numeric badge that animates when `n` changes (shows "99+" above 99).
    pub fn count(n: i64) -> Self {
        Self { count: Some(n), pill: true, variant: BadgeVariant::Default, ..Self::new("") }
    }
    /// Visual variant.
    pub fn variant(mut self, v: BadgeVariant) -> Self {
        self.variant = v;
        self
    }
    /// Leading status dot in the text color.
    pub fn dot(mut self, d: bool) -> Self {
        self.dot = d;
        self
    }
    /// Leading icon.
    pub fn icon(mut self, i: UiIcon) -> Self {
        self.icon = Some(i);
        self
    }
    /// Fully rounded ends.
    pub fn pill(mut self, p: bool) -> Self {
        self.pill = p;
        self
    }
    /// Custom tint (soft fill + colored text).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Font size (default 12).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
}

fn count_text(n: i64) -> String {
    if n > 99 {
        "99+".into()
    } else {
        n.to_string()
    }
}

impl Widget for Badge {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (mut fill, mut text_col, mut border) = badge_colors(&t, self.variant);
        if let Some(c) = self.color {
            fill = with_alpha(c, if t.dark { 0.16 } else { 0.1 });
            text_col = if t.dark { mix(c, Color32::WHITE, 0.25) } else { mix(c, Color32::BLACK, 0.15) };
            border = with_alpha(c, 0.25);
        }
        let font = FontId::proportional(self.size);
        let label = match self.count {
            Some(n) => count_text(n),
            None => self.text.clone(),
        };
        let g = ui.painter().layout_no_wrap(label.clone(), font.clone(), text_col);
        let h = (self.size + 10.0).round();
        let lead = if self.dot { 12.0 } else if self.icon.is_some() { self.size + 4.0 } else { 0.0 };
        let px = if self.count.is_some() { 6.0 } else { 8.0 };
        let w = (g.size().x + px * 2.0 + lead).max(h);
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter();
        let r = if self.pill || self.count.is_some() { CornerRadius::same((h / 2.0) as u8) } else { CornerRadius::same(6) };
        painter.rect(rect, r, fill, Stroke::new(1.0, border), StrokeKind::Inside);
        let mut x = rect.center().x - (g.size().x + lead) / 2.0;
        if self.dot {
            painter.circle_filled(pos2(x + 3.0, rect.center().y), 3.0, text_col);
            x += 12.0;
        } else if let Some(i) = self.icon {
            i.paint(painter, Rect::from_center_size(pos2(x + self.size / 2.0, rect.center().y), vec2(self.size, self.size)), text_col);
            x += self.size + 4.0;
        }
        match self.count {
            Some(n) => {
                let id = resp.id.with("count");
                let now = ui.input(|i| i.time);
                let mut st: CountState = glowkit_core::state(ui, id, || CountState { shown: n, prev: n, at: -10.0 });
                if st.shown != n {
                    st.prev = st.shown;
                    st.shown = n;
                    st.at = now;
                    glowkit_core::set_state(ui, id, st.clone());
                }
                let k = glowkit_core::ease(glowkit_core::Easing::OutBack, ((now - st.at) / 0.35) as f32);
                if k < 1.0 {
                    ui.ctx().request_repaint();
                }
                let dir = if st.shown > st.prev { 1.0 } else { -1.0 };
                let clip = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
                let cy = rect.center().y;
                clip.text(pos2(rect.center().x, cy + dir * h * (1.0 - k)), egui::Align2::CENTER_CENTER, count_text(st.shown), font.clone(), text_col);
                if k < 1.0 {
                    clip.text(pos2(rect.center().x, cy - dir * h * k), egui::Align2::CENTER_CENTER, count_text(st.prev), font, with_alpha(text_col, (1.0 - k).clamp(0.0, 1.0)));
                }
            }
            None => {
                painter.galley(pos2(x, rect.center().y - g.size().y / 2.0), g, text_col);
            }
        }
        resp
    }
}

/// A compact, optionally removable or selectable chip (tag).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::Chip;
/// if ui.add(Chip::new("Design").removable(true)).clicked() { /* removed */ }
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Chip {
    text: String,
    icon: Option<UiIcon>,
    removable: bool,
    selected: Option<bool>,
    color: Option<Color32>,
    avatar: Option<String>,
}

impl Chip {
    /// A chip with a label.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), icon: None, removable: false, selected: None, color: None, avatar: None }
    }
    /// Leading icon.
    pub fn icon(mut self, i: UiIcon) -> Self {
        self.icon = Some(i);
        self
    }
    /// Show an × button; the response's `.clicked()` is true when it is pressed.
    pub fn removable(mut self, r: bool) -> Self {
        self.removable = r;
        self
    }
    /// Filter-chip mode: highlighted when selected; the response's `.clicked()` reports toggles.
    pub fn selected(mut self, s: bool) -> Self {
        self.selected = Some(s);
        self
    }
    /// Tint color for the selected state and dot.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Leading mini avatar with initials from this name.
    pub fn avatar(mut self, name: impl Into<String>) -> Self {
        self.avatar = Some(name.into());
        self
    }
}

impl Widget for Chip {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = FontId::proportional(13.0);
        let g = ui.painter().layout_no_wrap(self.text.clone(), font, Color32::PLACEHOLDER);
        let h = 28.0;
        let lead = if self.avatar.is_some() { 24.0 } else if self.icon.is_some() || self.selected.is_some() { 20.0 } else { 0.0 };
        let trail = if self.removable { 20.0 } else { 0.0 };
        let w = g.size().x + 22.0 + lead + trail - if self.avatar.is_some() { 6.0 } else { 0.0 };
        let sense = if self.removable && self.selected.is_none() { Sense::hover() } else { Sense::click() };
        let (rect, mut resp) = ui.allocate_exact_size(vec2(w, h), sense);
        let sel = self.selected.unwrap_or(false);
        let k = kit::anim(ui, resp.id.with("sel"), sel, 0.16);
        let hov = kit::anim(ui, resp.id.with("h"), resp.hovered(), 0.12);
        let tint = self.color.unwrap_or(t.primary);
        let base_fill = if t.dark { with_alpha(t.secondary, 0.6 + 0.4 * hov) } else { mix(t.background, t.secondary, 0.5 + 0.5 * hov) };
        let sel_fill = if self.color.is_some() { with_alpha(tint, if t.dark { 0.2 } else { 0.12 }) } else { t.primary };
        let sel_text = if self.color.is_some() { if t.dark { mix(tint, Color32::WHITE, 0.3) } else { mix(tint, Color32::BLACK, 0.2) } } else { t.primary_foreground };
        let fill = base_fill.lerp_to_gamma(sel_fill, k);
        let text = t.foreground.lerp_to_gamma(sel_text, k);
        let border = t.border.lerp_to_gamma(if self.color.is_some() { with_alpha(tint, 0.4) } else { t.primary }, k);
        let painter = ui.painter();
        let r = CornerRadius::same(14);
        painter.rect(rect, r, fill, Stroke::new(1.0, border), StrokeKind::Inside);
        let mut x = rect.min.x + 11.0;
        let cy = rect.center().y;
        if let Some(name) = &self.avatar {
            let c = pos2(rect.min.x + 14.0, cy);
            let col = super::avatar::name_color(&t, name);
            painter.circle_filled(c, 10.0, col);
            painter.text(c, egui::Align2::CENTER_CENTER, super::avatar::initials(name), FontId::proportional(9.0), Color32::WHITE);
            x = rect.min.x + 29.0;
        } else if self.selected.is_some() && self.icon.is_none() {
            // check slides in when selected
            let ir = Rect::from_center_size(pos2(x + 7.0, cy), vec2(14.0, 14.0));
            if k > 0.01 {
                kit::check_path(painter, ir, text, 1.8, k);
            } else {
                UiIcon::Plus.paint(painter, ir, t.muted_foreground);
            }
            x += 20.0;
        } else if let Some(i) = self.icon {
            i.paint(painter, Rect::from_center_size(pos2(x + 7.0, cy), vec2(14.0, 14.0)), text);
            x += 20.0;
        }
        painter.galley(pos2(x, cy - g.size().y / 2.0), g, text);
        if self.removable {
            let xr = Rect::from_center_size(pos2(rect.max.x - 15.0, cy), vec2(18.0, 18.0));
            let xresp = ui.interact(xr, resp.id.with("x"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            if xresp.hovered() {
                ui.painter().circle_filled(xr.center(), 9.0, with_alpha(t.muted_foreground, 0.2));
            }
            UiIcon::X.paint(ui.painter(), xr.shrink(4.0), if xresp.hovered() { t.foreground } else { t.muted_foreground });
            if xresp.clicked() {
                resp = xresp;
            }
        } else {
            resp = resp.on_hover_cursor(CursorIcon::PointingHand);
        }
        resp
    }
}

/// A keyboard key cap, e.g. `⌘` `K`.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_components::Kbd::new("⌘K"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Kbd {
    text: String,
    size: f32,
}

impl Kbd {
    /// A key cap showing `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), size: 12.0 }
    }
    /// Font size (default 12).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
}

impl Widget for Kbd {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let g = ui.painter().layout_no_wrap(self.text, FontId::proportional(self.size), t.muted_foreground);
        let h = self.size + 10.0;
        let (rect, resp) = ui.allocate_exact_size(vec2((g.size().x + 10.0).max(h), h), Sense::hover());
        let painter = ui.painter();
        let r = CornerRadius::same(5);
        painter.rect_filled(rect.translate(vec2(0.0, 1.0)), r, if t.dark { mix(t.border, Color32::BLACK, 0.3) } else { t.border });
        painter.rect(rect, r, t.muted, Stroke::new(1.0, t.border), StrokeKind::Inside);
        painter.galley(rect.center() - g.size() / 2.0, g, t.muted_foreground);
        resp
    }
}

/// A hairline divider, horizontal or vertical, optionally with a centered label ("OR").
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_components::Separator::horizontal().label("or continue with"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Separator {
    vertical: bool,
    label: Option<String>,
    length: Option<f32>,
    margin: f32,
}

impl Separator {
    /// A horizontal line filling the available width.
    pub fn horizontal() -> Self {
        Self { vertical: false, label: None, length: None, margin: 0.0 }
    }
    /// A vertical line (default 20 pt tall).
    pub fn vertical() -> Self {
        Self { vertical: true, label: None, length: Some(20.0), margin: 0.0 }
    }
    /// Text centered on the line.
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }
    /// Line length (default: available width, or 20 for vertical).
    pub fn length(mut self, l: f32) -> Self {
        self.length = Some(l);
        self
    }
    /// Extra space before and after the line.
    pub fn margin(mut self, m: f32) -> Self {
        self.margin = m;
        self
    }
}

impl Widget for Separator {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        if self.vertical {
            let h = self.length.unwrap_or(20.0);
            let (rect, resp) = ui.allocate_exact_size(vec2(1.0 + self.margin * 2.0, h), Sense::hover());
            let x = kit::snap(ui, rect.center().x) + 0.5;
            ui.painter().line_segment([pos2(x, rect.min.y), pos2(x, rect.max.y)], Stroke::new(1.0, t.border));
            return resp;
        }
        let w = self.length.unwrap_or_else(|| ui.available_width());
        let lh = if self.label.is_some() { 16.0 } else { 1.0 };
        let (rect, resp) = ui.allocate_exact_size(vec2(w, lh + self.margin * 2.0), Sense::hover());
        let y = kit::snap(ui, rect.center().y) + 0.5;
        match &self.label {
            Some(l) => {
                let g = ui.painter().layout_no_wrap(l.to_uppercase(), FontId::proportional(11.0), t.muted_foreground);
                let gx = rect.center().x - g.size().x / 2.0;
                ui.painter().line_segment([pos2(rect.min.x, y), pos2(gx - 10.0, y)], Stroke::new(1.0, t.border));
                ui.painter().line_segment([pos2(gx + g.size().x + 10.0, y), pos2(rect.max.x, y)], Stroke::new(1.0, t.border));
                ui.painter().galley(pos2(gx, rect.center().y - g.size().y / 2.0), g, t.muted_foreground);
            }
            None => {
                ui.painter().line_segment([pos2(rect.min.x, y), pos2(rect.max.x, y)], Stroke::new(1.0, t.border));
            }
        }
        resp
    }
}
