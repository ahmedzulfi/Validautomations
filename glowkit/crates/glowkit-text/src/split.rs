use crate::util;
use crate::{Direction, SplitBy};
use egui::{pos2, Color32, Rect, Response, Sense, Ui, Widget};
use glowkit_core::{anim::stagger, ease, text::Glyphs, Clock, Easing, Theme, Trigger};

/// Letters (or words) rise into place one after another while fading in.
///
/// With [`SplitText::mask`] each line clips its letters, so they appear to slide up from behind
/// an invisible edge ("letter pull up").
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_text::{SplitText, SplitBy};
/// ui.add(SplitText::new("Hello, you!").size(48.0));
/// ui.add(SplitText::new("Word by word").split_by(SplitBy::Words).mask(true));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SplitText {
    text: String,
    size: f32,
    color: Option<Color32>,
    bold: bool,
    split: SplitBy,
    direction: Direction,
    distance: Option<f32>,
    delay: f32,
    duration: f32,
    easing: Easing,
    trigger: Trigger,
    mask: bool,
    wrap: Option<f32>,
}

impl SplitText {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 40.0,
            color: None,
            bold: true,
            split: SplitBy::Letters,
            direction: Direction::Bottom,
            distance: None,
            delay: 0.04,
            duration: 0.7,
            easing: Easing::OutBack,
            trigger: Trigger::OnAppear,
            mask: false,
            wrap: None,
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
    /// Animate letter by letter (default) or word by word.
    pub fn split_by(mut self, s: SplitBy) -> Self {
        self.split = s;
        self
    }
    /// Side the pieces come from (default: from below).
    pub fn direction(mut self, d: Direction) -> Self {
        self.direction = d;
        self
    }
    /// Travel distance in points (defaults to 0.8 x the font size).
    pub fn distance(mut self, d: f32) -> Self {
        self.distance = Some(d);
        self
    }
    /// Seconds between consecutive pieces.
    pub fn delay(mut self, s: f32) -> Self {
        self.delay = s;
        self
    }
    /// Seconds each piece takes to settle.
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s;
        self
    }
    /// Easing curve for each piece (default `OutBack` for a little overshoot).
    pub fn easing(mut self, e: Easing) -> Self {
        self.easing = e;
        self
    }
    /// When the animation plays.
    pub fn trigger(mut self, t: Trigger) -> Self {
        self.trigger = t;
        self
    }
    /// Clip each line so pieces slide out from behind its edge.
    pub fn mask(mut self, mask: bool) -> Self {
        self.mask = mask;
        self
    }
    /// Wrap width in points (defaults to the available width).
    pub fn wrap(mut self, width: f32) -> Self {
        self.wrap = Some(width);
        self
    }
}

impl Widget for SplitText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let color = self.color.unwrap_or(t.foreground);
        let font = util::font(self.size, false);
        let wrap = self.wrap.unwrap_or(ui.available_width());
        let glyphs = Glyphs::layout(ui.painter(), &self.text, font.clone(), Some(wrap));
        let bold = util::weight(self.bold, self.size);
        let (rect, resp) = ui.allocate_exact_size(glyphs.size + egui::vec2(bold, 0.0), Sense::click());
        let pieces = match self.split {
            SplitBy::Words => glyphs.word_count(),
            SplitBy::Letters => glyphs.glyphs.len(),
        };
        let total = self.duration + self.delay * pieces.saturating_sub(1) as f32;
        let elapsed = Clock::triggered(ui, resp.id, self.trigger, total, resp.hovered(), resp.clicked());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let dist = self.distance.unwrap_or(self.size * 0.8);
        for g in &glyphs.glyphs {
            if g.ch.is_whitespace() {
                continue;
            }
            let idx = match self.split {
                SplitBy::Words => g.word,
                SplitBy::Letters => g.index,
            };
            let lin = stagger(elapsed, idx, self.delay, self.duration);
            if lin <= 0.0 {
                continue;
            }
            let p = ease(self.easing, lin);
            let alpha = ease(Easing::OutCubic, (lin * 1.6).min(1.0));
            let off = self.direction.offset(dist * (1.0 - p));
            let pos = rect.min + g.offset + off;
            let painter = if self.mask {
                let line = Rect::from_min_max(
                    pos2(rect.min.x - self.size, rect.min.y + g.offset.y),
                    pos2(rect.max.x + self.size, rect.min.y + g.offset.y + g.size.y),
                );
                ui.painter().with_clip_rect(line.intersect(ui.clip_rect()))
            } else {
                ui.painter().clone()
            };
            let galley = util::layout_char(&painter, g.ch, &font);
            util::paint_galley(&painter, pos, &galley, color.gamma_multiply(alpha), bold);
        }
        resp
    }
}
