use crate::util;
use egui::{pos2, vec2, Color32, CornerRadius, Rect, Response, Sense, Ui, Widget};
use glowkit_core::{ease, mix, Easing, Theme};

/// Where the underline of [`AnimatedUnderline`] grows from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum UnderlineOrigin {
    /// Draws in from the left and leaves to the right (default).
    #[default]
    Left,
    /// Grows from the center outward and shrinks back to it.
    Center,
    /// Draws in from the right and leaves to the left.
    Right,
}

#[derive(Clone, Copy, Default)]
struct LineState {
    /// How far the line has drawn in.
    p: f32,
    /// How far it has slid out (only while leaving).
    q: f32,
    leaving: bool,
}

/// Link-style text whose underline draws in on hover and slides out the far side when the
/// pointer leaves. Returns a clickable [`Response`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// if ui.add(glowkit_text::AnimatedUnderline::new("Read the docs")).clicked() {
///     // navigate
/// }
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AnimatedUnderline {
    text: String,
    size: f32,
    color: Option<Color32>,
    hover_color: Option<Color32>,
    line_color: Option<Color32>,
    thickness: Option<f32>,
    origin: UnderlineOrigin,
    active: bool,
    duration: f32,
    bold: bool,
}

impl AnimatedUnderline {
    /// Create a link with `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 18.0,
            color: None,
            hover_color: None,
            line_color: None,
            thickness: None,
            origin: UnderlineOrigin::Left,
            active: false,
            duration: 0.35,
            bold: false,
        }
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Resting text color (defaults to the theme muted foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Text color while hovered/active (defaults to the theme foreground).
    pub fn hover_color(mut self, c: Color32) -> Self {
        self.hover_color = Some(c);
        self
    }
    /// Underline color (defaults to the theme accent).
    pub fn line_color(mut self, c: Color32) -> Self {
        self.line_color = Some(c);
        self
    }
    /// Line thickness in points (default 0.1 x font size, at least 2).
    pub fn thickness(mut self, points: f32) -> Self {
        self.thickness = Some(points);
        self
    }
    /// Where the line grows from.
    pub fn origin(mut self, o: UnderlineOrigin) -> Self {
        self.origin = o;
        self
    }
    /// Keep the underline drawn (e.g. the current page in a nav bar).
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
    /// Seconds to draw the line.
    pub fn duration(mut self, seconds: f32) -> Self {
        self.duration = seconds.max(0.01);
        self
    }
    /// Fake-bold the text.
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
}

impl Widget for AnimatedUnderline {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let painter = ui.painter().clone();
        let galley = util::layout(&painter, self.text.clone(), &font);
        let th = self.thickness.unwrap_or((self.size * 0.1).max(2.0));
        let (rect, resp) = ui.allocate_exact_size(galley.size() + vec2(bold, th + self.size * 0.12), Sense::click());
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        let on = resp.hovered() || self.active || resp.has_focus();
        let dt = glowkit_core::dt(ui);
        let step = dt / self.duration;
        let st = glowkit_core::with_state(ui, resp.id.with("underline"), LineState::default, |s| {
            if on {
                s.leaving = false;
                s.q = 0.0;
                s.p = (s.p + step).min(1.0);
            } else if s.p > 0.0 {
                s.leaving = true;
                s.q = (s.q + step).min(1.0);
                if s.q >= 1.0 {
                    *s = LineState::default();
                }
            }
            *s
        });
        if st.leaving || (on && st.p < 1.0) {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let k = ease(Easing::InOutCubic, st.p);
        let out = if st.leaving { ease(Easing::InOutCubic, st.q) } else { 0.0 };
        let lit = k * (1.0 - out);
        let color = mix(self.color.unwrap_or(t.muted_foreground), self.hover_color.unwrap_or(t.foreground), lit);
        util::paint_galley(&painter, rect.min, &galley, color, bold);
        let (a, b) = match self.origin {
            UnderlineOrigin::Left => (out, k),
            UnderlineOrigin::Right => (1.0 - k, 1.0 - out),
            UnderlineOrigin::Center => (0.5 - lit / 2.0, 0.5 + lit / 2.0),
        };
        if b > a {
            let w = galley.size().x + bold;
            let y = rect.min.y + galley.size().y + self.size * 0.04;
            let line = Rect::from_min_max(pos2(rect.min.x + a * w, y), pos2(rect.min.x + b * w, y + th));
            let lc = self.line_color.unwrap_or(t.accent);
            painter.rect_filled(line, CornerRadius::same((th / 2.0) as u8), lc);
        }
        resp
    }
}
