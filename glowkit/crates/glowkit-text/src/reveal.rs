use crate::util;
use egui::{Color32, Response, Sense, Ui, Widget};
use glowkit_core::{anim::smoothstep, ease, mix, Clock, Easing, Theme, Trigger};

/// Where [`ScrollRevealText`] gets its progress from.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Source {
    Time,
    Fixed(f32),
    Scroll,
}

/// A paragraph whose words light up one after another, from faint to full color (with a slight
/// blur clearing), driven by time, by your own `progress` value or by its scroll position.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let progress = 0.5;
/// ui.add(glowkit_text::ScrollRevealText::new("When does a man die?").progress(progress));
/// // or tie it to the enclosing ScrollArea:
/// ui.add(glowkit_text::ScrollRevealText::new("When does a man die?").scroll_linked());
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ScrollRevealText {
    text: String,
    size: f32,
    color: Option<Color32>,
    dim: f32,
    bold: bool,
    blur: bool,
    max_width: Option<f32>,
    softness: f32,
    duration: f32,
    trigger: Trigger,
    source: Source,
}

impl ScrollRevealText {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 30.0,
            color: None,
            dim: 0.15,
            bold: true,
            blur: true,
            max_width: None,
            softness: 4.0,
            duration: 4.0,
            trigger: Trigger::OnAppear,
            source: Source::Time,
        }
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Revealed text color (defaults to the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Opacity of not-yet-revealed words, 0..1 (default 0.15).
    pub fn dim(mut self, opacity: f32) -> Self {
        self.dim = opacity.clamp(0.0, 1.0);
        self
    }
    /// Fake-bold the text (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Blur the unrevealed words a little (default on).
    pub fn blur(mut self, blur: bool) -> Self {
        self.blur = blur;
        self
    }
    /// Wrap width in points (defaults to the available width).
    pub fn max_width(mut self, w: f32) -> Self {
        self.max_width = Some(w);
        self
    }
    /// How many words are mid-transition at once (default 4).
    pub fn softness(mut self, words: f32) -> Self {
        self.softness = words.max(0.5);
        self
    }
    /// Seconds for a full time-driven reveal.
    pub fn duration(mut self, seconds: f32) -> Self {
        self.duration = seconds.max(0.05);
        self
    }
    /// When the time-driven reveal plays.
    pub fn trigger(mut self, t: Trigger) -> Self {
        self.trigger = t;
        self
    }
    /// Drive the reveal yourself, `0..=1`.
    pub fn progress(mut self, p: f32) -> Self {
        self.source = Source::Fixed(p.clamp(0.0, 1.0));
        self
    }
    /// Drive the reveal from the widget's position in the enclosing scroll area / clip rect:
    /// it starts as the paragraph enters at the bottom and completes in the upper part.
    pub fn scroll_linked(mut self) -> Self {
        self.source = Source::Scroll;
        self
    }
}

impl Widget for ScrollRevealText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let wrap = self.max_width.unwrap_or(ui.available_width()).min(ui.available_width());
        let glyphs = crate::util::glyphs(ui.painter(), &self.text, font.clone(), Some(wrap));
        let (rect, resp) = ui.allocate_exact_size(glyphs.size + egui::vec2(bold, 0.0), Sense::click());
        let progress = match self.source {
            Source::Fixed(p) => p,
            Source::Time => {
                let e = Clock::triggered(ui, resp.id, self.trigger, self.duration, resp.hovered(), resp.clicked());
                ease(Easing::InOutSine, e / self.duration)
            }
            Source::Scroll => {
                let clip = ui.clip_rect();
                let start = clip.max.y;
                let end = clip.min.y + clip.height() * 0.35;
                let travel = (start - end + rect.height()).max(1.0);
                ((start - rect.min.y) / travel).clamp(0.0, 1.0)
            }
        };
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter();
        let color = self.color.unwrap_or(t.foreground);
        let words = glyphs.word_count() as f32;
        let front = progress * (words + self.softness);
        for g in &glyphs.glyphs {
            if g.ch.is_whitespace() {
                continue;
            }
            let b = smoothstep(0.0, 1.0, (front - g.word as f32) / self.softness);
            let c = mix(color.gamma_multiply(self.dim), color, b);
            let gal = util::layout_char(painter, g.ch, &font);
            let r = if self.blur { (1.0 - b) * self.size * 0.07 } else { 0.0 };
            util::paint_blurred(painter, rect.min + g.offset, &gal, c, r, bold);
        }
        resp
    }
}
