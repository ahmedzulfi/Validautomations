use crate::util;
use egui::{vec2, Color32, Response, Sense, Ui, Widget};
use glowkit_core::text::{scramble_char, SCRAMBLE_CHARS};
use glowkit_core::{mix, Theme};

#[derive(Clone, Copy, Default)]
struct ScrambleState {
    h: f32,
    leaving: bool,
}

/// Text that dissolves into flickering random symbols while hovered (left to right) and resolves
/// back, again left to right, when the pointer leaves.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::ScrambleHover::new("Hover to scramble").monospace(true));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ScrambleHover {
    text: String,
    size: f32,
    color: Option<Color32>,
    scramble_color: Option<Color32>,
    mono: bool,
    bold: bool,
    tick: f32,
    stagger: f32,
    charset: String,
    auto: Option<f32>,
}

impl ScrambleHover {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 28.0,
            color: None,
            scramble_color: None,
            mono: false,
            bold: false,
            tick: 0.05,
            stagger: 0.035,
            charset: SCRAMBLE_CHARS.to_owned(),
            auto: None,
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
    /// Color of scrambled symbols (defaults to the theme accent).
    pub fn scramble_color(mut self, c: Color32) -> Self {
        self.scramble_color = Some(c);
        self
    }
    /// Use the monospace font.
    pub fn monospace(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }
    /// Fake-bold the text.
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Seconds between symbol changes.
    pub fn tick(mut self, seconds: f32) -> Self {
        self.tick = seconds.max(0.01);
        self
    }
    /// Seconds between letters as the scramble sweeps across.
    pub fn stagger(mut self, seconds: f32) -> Self {
        self.stagger = seconds.max(0.0);
        self
    }
    /// Symbols to scramble through.
    pub fn charset(mut self, chars: impl Into<String>) -> Self {
        self.charset = chars.into();
        self
    }
    /// Also scramble by itself every `seconds` (for showcases and idle hints).
    pub fn auto(mut self, seconds: f32) -> Self {
        self.auto = Some(seconds.max(1.0));
        self
    }
}

impl Widget for ScrambleHover {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, self.mono);
        let bold = util::weight(self.bold, self.size);
        let glyphs = crate::util::glyphs(ui.painter(), &self.text, font.clone(), None);
        let (rect, resp) = ui.allocate_exact_size(glyphs.size + vec2(bold, 0.0), Sense::hover());
        let time = glowkit_core::time(ui);
        let auto_on = self.auto.is_some_and(|p| time % p < 1.2);
        if self.auto.is_some() {
            glowkit_core::animate(ui);
        }
        let on = resp.hovered() || auto_on;
        let n = glyphs.glyphs.len();
        let span = self.stagger * n as f32 + 0.05;
        let dt = glowkit_core::dt(ui);
        let st = glowkit_core::with_state(ui, resp.id.with("scramble"), ScrambleState::default, |s| {
            if on {
                if s.leaving {
                    s.leaving = false;
                    s.h = 0.0;
                }
                s.h = (s.h + dt / span).min(1.0);
            } else if s.h > 0.0 || s.leaving {
                if !s.leaving {
                    s.leaving = true;
                    s.h = 0.0;
                }
                s.h = (s.h + dt / span).min(1.0);
                if s.h >= 1.0 {
                    s.leaving = false;
                    s.h = 0.0;
                }
            }
            *s
        });
        let busy = on || st.leaving;
        if busy {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter();
        let color = self.color.unwrap_or(t.foreground);
        let scol = self.scramble_color.unwrap_or(t.accent);
        let front = st.h * span;
        let tick = (time / self.tick) as u64;
        for g in &glyphs.glyphs {
            if g.ch.is_whitespace() {
                continue;
            }
            let at = g.index as f32 * self.stagger;
            let scrambled = if st.leaving { front < at } else { on && front >= at };
            let pos = rect.min + g.offset;
            if scrambled {
                let ch = scramble_char(resp.id.value(), g.index, tick, &self.charset);
                util::paint_char_in(painter, ch, &font, pos, g.size.x, mix(scol, color, 0.15), bold);
            } else {
                let gal = util::layout_char(painter, g.ch, &font);
                util::paint_galley(painter, pos, &gal, color, bold);
            }
        }
        resp
    }
}
