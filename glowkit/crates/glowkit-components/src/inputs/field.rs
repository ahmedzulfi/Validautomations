use super::kit::UiIcon;
use egui::{vec2, Response, Sense, Ui, Widget};
use glowkit_core::Theme;

/// A form label (14 pt, medium contrast) with an optional required asterisk and hint on the right.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::FieldLabel;
/// ui.add(FieldLabel::new("Email").required(true));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct FieldLabel {
    text: String,
    required: bool,
    hint: Option<String>,
    size: f32,
}

impl FieldLabel {
    /// A label with the given text.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), required: false, hint: None, size: 13.5 }
    }
    /// Append a red asterisk.
    pub fn required(mut self, r: bool) -> Self {
        self.required = r;
        self
    }
    /// Muted text aligned to the right (e.g. "Optional").
    pub fn hint(mut self, h: impl Into<String>) -> Self {
        self.hint = Some(h.into());
        self
    }
    /// Font size in points.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
}

impl Widget for FieldLabel {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let painter = ui.painter();
        let g = painter.layout_no_wrap(self.text.clone(), egui::FontId::proportional(self.size), t.foreground);
        let w = if self.hint.is_some() { ui.available_width() } else { g.size().x + if self.required { 10.0 } else { 0.0 } };
        let (rect, resp) = ui.allocate_exact_size(vec2(w, g.size().y), Sense::hover());
        let painter = ui.painter();
        painter.galley(rect.min, g.clone(), t.foreground);
        if self.required {
            painter.text(rect.min + vec2(g.size().x + 3.0, 0.0), egui::Align2::LEFT_TOP, "*", egui::FontId::proportional(self.size), t.destructive);
        }
        if let Some(h) = self.hint {
            painter.text(rect.right_center(), egui::Align2::RIGHT_CENTER, h, egui::FontId::proportional(self.size - 1.0), t.muted_foreground);
        }
        resp
    }
}

/// Muted helper text shown under a form control.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_components::FieldDescription::new("We'll never share your email."));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct FieldDescription {
    text: String,
}

impl FieldDescription {
    /// Helper text.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl Widget for FieldDescription {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        ui.add(egui::Label::new(egui::RichText::new(self.text).size(12.5).color(t.muted_foreground)).wrap())
    }
}

/// Red validation message with a small alert icon, shown under a form control.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_components::FieldError::new("Password must be at least 8 characters."));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct FieldError {
    text: String,
}

impl FieldError {
    /// Error text.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl Widget for FieldError {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let g = ui.painter().layout(self.text, egui::FontId::proportional(12.5), t.destructive, ui.available_width() - 18.0);
        let (rect, resp) = ui.allocate_exact_size(vec2(g.size().x + 18.0, g.size().y.max(14.0)), Sense::hover());
        UiIcon::AlertCircle.paint(ui.painter(), egui::Rect::from_min_size(rect.min + vec2(0.0, 1.0), vec2(13.0, 13.0)), t.destructive);
        ui.painter().galley(rect.min + vec2(18.0, 0.0), g, t.destructive);
        resp
    }
}
