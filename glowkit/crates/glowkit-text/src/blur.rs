use egui::{vec2, Color32, FontId, Response, Sense, Ui, Widget};
use glowkit_core::{anim::stagger, ease, paint, Clock, Easing, Theme, Trigger};

/// How [`BlurText`] splits its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SplitBy {
    #[default]
    Words,
    Letters,
}

/// Direction the pieces travel from as they come into focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
    None,
}

impl Direction {
    pub(crate) fn offset(self, d: f32) -> egui::Vec2 {
        match self {
            Direction::Top => vec2(0.0, -d),
            Direction::Bottom => vec2(0.0, d),
            Direction::Left => vec2(-d, 0.0),
            Direction::Right => vec2(d, 0.0),
            Direction::None => egui::Vec2::ZERO,
        }
    }
}

/// Words (or letters) fade in from a blur, one after another.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_text::{BlurText, SplitBy};
/// ui.add(BlurText::new("Isn't this so cool?!").split_by(SplitBy::Letters).size(36.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BlurText {
    text: String,
    size: f32,
    color: Option<Color32>,
    split: SplitBy,
    direction: Direction,
    delay: f32,
    duration: f32,
    blur: f32,
    distance: f32,
    easing: Easing,
    trigger: Trigger,
}

impl BlurText {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 32.0,
            color: None,
            split: SplitBy::Words,
            direction: Direction::Top,
            delay: 0.12,
            duration: 0.7,
            blur: 8.0,
            distance: 18.0,
            easing: Easing::OutCubic,
            trigger: Trigger::OnAppear,
        }
    }
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    pub fn split_by(mut self, s: SplitBy) -> Self {
        self.split = s;
        self
    }
    pub fn direction(mut self, d: Direction) -> Self {
        self.direction = d;
        self
    }
    /// Seconds between consecutive pieces.
    pub fn delay(mut self, s: f32) -> Self {
        self.delay = s;
        self
    }
    /// Seconds each piece takes to come into focus.
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s;
        self
    }
    /// Starting blur radius in points.
    pub fn blur(mut self, b: f32) -> Self {
        self.blur = b;
        self
    }
    pub fn distance(mut self, d: f32) -> Self {
        self.distance = d;
        self
    }
    pub fn easing(mut self, e: Easing) -> Self {
        self.easing = e;
        self
    }
    pub fn trigger(mut self, t: Trigger) -> Self {
        self.trigger = t;
        self
    }
}

impl Widget for BlurText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let color = self.color.unwrap_or(t.foreground);
        let font = FontId::proportional(self.size);
        let glyphs = crate::util::glyphs(ui.painter(), &self.text, font.clone(), Some(ui.available_width()));
        let (rect, resp) = ui.allocate_exact_size(glyphs.size, Sense::click());
        let pieces = match self.split {
            SplitBy::Words => glyphs.word_count(),
            SplitBy::Letters => glyphs.glyphs.len(),
        };
        let total = self.duration + self.delay * pieces.saturating_sub(1) as f32;
        let elapsed = Clock::triggered(ui, resp.id, self.trigger, total, resp.hovered(), resp.clicked());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter();
        for g in &glyphs.glyphs {
            if g.ch.is_whitespace() {
                continue;
            }
            let idx = match self.split {
                SplitBy::Words => g.word,
                SplitBy::Letters => g.index,
            };
            let p = ease(self.easing, stagger(elapsed, idx, self.delay, self.duration));
            if p <= 0.0 {
                continue;
            }
            let off = self.direction.offset(self.distance * (1.0 - p));
            let galley = painter.layout_no_wrap(g.ch.to_string(), font.clone(), color);
            let pos = rect.min + g.offset + off;
            paint::blurred_galley(painter, pos, &galley, color.gamma_multiply(p), self.blur * (1.0 - p));
        }
        resp
    }
}
