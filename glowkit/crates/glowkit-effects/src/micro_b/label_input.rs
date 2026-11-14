use super::common::cr;
use egui::{pos2, vec2, Align, Align2, Color32, FontId, Layout, Rect, Response, Sense, Stroke, Ui, UiBuilder, Vec2, Widget};
use glowkit_core::anim::spring;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// A text field whose label sits inside like a placeholder and lifts into the top edge when the
/// field is focused or filled. The label shrinks as it rises and the outline opens a notch behind
/// it, all on a soft spring.
///
/// The notch is filled with the color behind the field (the theme `muted` stage by default), so
/// set [`LabelInput::notch_fill`] when the field sits on another background.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut email = String::new();
/// ui.add(glowkit_effects::LabelInput::new(&mut email, "Email"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct LabelInput<'a> {
    text: &'a mut String,
    label: String,
    password: bool,
    size: Vec2,
    radius: f32,
    notch: Option<Color32>,
    accent: Option<Color32>,
    highlight: bool,
}

impl<'a> LabelInput<'a> {
    /// A field editing `text`, labelled `label`.
    pub fn new(text: &'a mut String, label: impl Into<String>) -> Self {
        Self {
            text,
            label: label.into(),
            password: false,
            size: vec2(280.0, 52.0),
            radius: 20.0,
            notch: None,
            accent: None,
            highlight: false,
        }
    }
    /// Hide the characters (password field).
    pub fn password(mut self, on: bool) -> Self {
        self.password = on;
        self
    }
    /// Field width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.size.x = w.max(80.0);
        self
    }
    /// Field height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.size.y = h.max(32.0);
        self
    }
    /// Corner radius (default 20).
    pub fn corner(mut self, r: f32) -> Self {
        self.radius = r.max(0.0);
        self
    }
    /// Color painted behind the lifted label (match what the field sits on).
    pub fn notch_fill(mut self, c: Color32) -> Self {
        self.notch = Some(c);
        self
    }
    /// Outline color while focused (default: the theme foreground).
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = Some(c);
        self
    }
    /// Draw as focused even when it is not (useful for showcases).
    pub fn highlight(mut self, on: bool) -> Self {
        self.highlight = on;
        self
    }
}

impl Widget for LabelInput<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, outer) = ui.allocate_exact_size(self.size, Sense::click());
        let id = outer.id;
        let te_id = id.with("te");
        let radius = self.radius.min(rect.height() / 2.0);
        let pad = 14.0 + radius * 0.3;
        let text_rect = Rect::from_min_max(pos2(rect.min.x + pad, rect.min.y), pos2(rect.max.x - pad, rect.max.y));
        let bg_idx = ui.painter().add(egui::Shape::Noop);
        let resp = ui
            .scope_builder(UiBuilder::new().max_rect(text_rect).layout(Layout::left_to_right(Align::Center)), |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut *self.text)
                        .id(te_id)
                        .frame(egui::Frame::NONE)
                        .margin(egui::Margin::ZERO)
                        .font(FontId::proportional(15.0))
                        .text_color(t.foreground)
                        .desired_width(text_rect.width())
                        .password(self.password)
                        .align(Align2::LEFT_CENTER),
                )
            })
            .inner;
        if outer.clicked() {
            ui.memory_mut(|m| m.request_focus(te_id));
        }
        let focused = resp.has_focus() || self.highlight;
        let hovered = resp.hovered() || outer.hovered();
        let lift = spring(ui, id.with("lift"), if focused || !self.text.is_empty() { 1.0 } else { 0.0 }, 260.0, 21.0);
        let focus = spring(ui, id.with("focus"), if focused { 1.0 } else { 0.0 }, 300.0, 26.0);
        let hov = spring(ui, id.with("hov"), if hovered { 1.0 } else { 0.0 }, 300.0, 26.0);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter();
        let accent = self.accent.unwrap_or(t.foreground);
        let idle = with_alpha(t.muted_foreground, 0.38 + 0.2 * hov);
        let line_col = mix(idle, accent, focus);
        let width = 1.0 + 0.5 * focus;
        // label geometry
        let size = 15.0 + (11.5 - 15.0) * lift;
        let lab = painter.layout_no_wrap(self.label.clone(), FontId::proportional(size), Color32::WHITE);
        let x = rect.min.x + pad + (radius * 0.15 - 2.0) * lift;
        let y = rect.center().y + (rect.min.y - rect.center().y) * lift;
        let lab_rect = Rect::from_min_size(pos2(x, y - lab.size().y / 2.0), lab.size());
        let notch = self.notch.unwrap_or(t.muted);
        let mut shapes = vec![egui::Shape::rect_stroke(
            rect.shrink(width / 2.0),
            cr(radius),
            Stroke::new(width, line_col),
            egui::StrokeKind::Middle,
        )];
        if lift > 0.01 {
            let w = (lab_rect.width() + 8.0) * lift.clamp(0.0, 1.0);
            let gap = Rect::from_center_size(
                pos2(lab_rect.center().x, rect.min.y + width / 2.0),
                vec2(w, width + 3.0),
            );
            shapes.push(egui::Shape::rect_filled(gap, cr(1.0), notch));
        }
        painter.set(bg_idx, egui::Shape::Vec(shapes));
        let lab_col = mix(t.muted_foreground, t.foreground, focus * 0.85);
        painter.text(lab_rect.left_center(), Align2::LEFT_CENTER, &self.label, FontId::proportional(size), lab_col);
        resp
    }
}
