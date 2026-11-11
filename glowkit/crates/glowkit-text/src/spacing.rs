use crate::util;
use egui::{vec2, Color32, Response, Sense, Ui, Widget};
use glowkit_core::{anim::stagger, ease, text::Glyphs, Clock, Easing, Theme, Trigger};

/// Letters start squeezed together at the center and fade in while the letter spacing opens up
/// to its natural width.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::GradualSpacing::new("Gradual Spacing").size(48.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct GradualSpacing {
    text: String,
    size: f32,
    color: Option<Color32>,
    bold: bool,
    collapse: f32,
    duration: f32,
    stagger: f32,
    easing: Easing,
    trigger: Trigger,
}

impl GradualSpacing {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 44.0,
            color: None,
            bold: true,
            collapse: 0.75,
            duration: 1.1,
            stagger: 0.02,
            easing: Easing::OutQuart,
            trigger: Trigger::OnAppear,
        }
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Text color (defaults to the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Fake-bold the text (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// How collapsed the letters start, 0..1 (1 = all on top of each other, default 0.75).
    pub fn collapse(mut self, k: f32) -> Self {
        self.collapse = k.clamp(0.0, 1.0);
        self
    }
    /// Seconds each letter takes.
    pub fn duration(mut self, seconds: f32) -> Self {
        self.duration = seconds.max(0.01);
        self
    }
    /// Seconds between letters; the middle letters move first and the wave spreads outwards.
    pub fn stagger(mut self, seconds: f32) -> Self {
        self.stagger = seconds.max(0.0);
        self
    }
    /// Easing curve (try `OutBack` for a spring).
    pub fn easing(mut self, e: Easing) -> Self {
        self.easing = e;
        self
    }
    /// When the animation plays.
    pub fn trigger(mut self, t: Trigger) -> Self {
        self.trigger = t;
        self
    }
}

impl Widget for GradualSpacing {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let glyphs = Glyphs::layout(ui.painter(), &self.text, font.clone(), None);
        let (rect, resp) = ui.allocate_exact_size(glyphs.size + vec2(bold, 0.0), Sense::click());
        let n = glyphs.glyphs.len();
        let total = self.duration + self.stagger * (n / 2) as f32;
        let e = Clock::triggered(ui, resp.id, self.trigger, total, resp.hovered(), resp.clicked());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter();
        let color = self.color.unwrap_or(t.foreground);
        let cx = glyphs.size.x / 2.0;
        let mid = (n as f32 - 1.0) / 2.0;
        for g in &glyphs.glyphs {
            if g.ch.is_whitespace() {
                continue;
            }
            // inner letters move first, the wave spreads outwards
            let rank = (g.index as f32 - mid).abs().round() as usize;
            let lin = stagger(e, rank, self.stagger, self.duration);
            let p = ease(self.easing, lin);
            let gx = g.offset.x + g.size.x / 2.0;
            let x = cx + (gx - cx) * (1.0 - self.collapse * (1.0 - p));
            let alpha = ease(Easing::OutCubic, (lin * 1.8).min(1.0));
            let gal = util::layout_char(painter, g.ch, &font);
            let pos = rect.min + vec2(x - g.size.x / 2.0, g.offset.y);
            util::paint_blurred(painter, pos, &gal, color.gamma_multiply(alpha), (1.0 - alpha) * self.size * 0.08, bold);
        }
        resp
    }
}
