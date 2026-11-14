use crate::util;
use egui::{pos2, Color32, Response, Sense, Ui, Widget};
use glowkit_core::{mix, text::scramble_char, Theme};

/// A paragraph whose letters scramble into random symbols when the pointer passes near them,
/// then settle back into the original text after a moment. Without a pointer, a ghost one
/// wanders across the text.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::ScrambledText::new("Lorem ipsum dolor sit amet").radius(90.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ScrambledText {
    text: String,
    size: f32,
    radius: f32,
    duration: f32,
    speed: f32,
    charset: String,
    color: Option<Color32>,
    highlight: Option<Color32>,
    max_width: Option<f32>,
    mono: bool,
    autoplay: bool,
}

impl ScrambledText {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 22.0,
            radius: 90.0,
            duration: 1.0,
            speed: 18.0,
            charset: ".:".into(),
            color: None,
            highlight: None,
            max_width: None,
            mono: true,
            autoplay: true,
        }
    }
    /// Font size in points (default 22).
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Radius around the pointer that scrambles letters, in points (default 90).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.max(1.0);
        self
    }
    /// Seconds a letter stays scrambled after the pointer leaves it (default 1).
    pub fn duration(mut self, seconds: f32) -> Self {
        self.duration = seconds.max(0.05);
        self
    }
    /// Symbol changes per second while scrambled (default 18).
    pub fn speed(mut self, per_sec: f32) -> Self {
        self.speed = per_sec.max(1.0);
        self
    }
    /// Characters to scramble through (default `".:"` for a subtle dotted look; try
    /// [`glowkit_core::text::SCRAMBLE_CHARS`]).
    pub fn charset(mut self, chars: impl Into<String>) -> Self {
        self.charset = chars.into();
        self
    }
    /// Text color (defaults to the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Color scrambled letters blend toward (defaults to the theme accent).
    pub fn highlight(mut self, c: Color32) -> Self {
        self.highlight = Some(c);
        self
    }
    /// Wrap width in points (defaults to the available width).
    pub fn max_width(mut self, w: f32) -> Self {
        self.max_width = Some(w);
        self
    }
    /// Use the monospace font so scrambled symbols never change the layout (default true).
    pub fn monospace(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }
    /// Let a ghost pointer wander over the text when the real one is away (default true).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for ScrambledText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, self.mono);
        let wrap = self.max_width.unwrap_or(ui.available_width()).min(ui.available_width());
        let glyphs = util::glyphs(ui.painter(), &self.text, font.clone(), Some(wrap));
        let (rect, resp) = ui.allocate_exact_size(glyphs.size, Sense::hover());
        let time = glowkit_core::time(ui);
        let dt = glowkit_core::dt(ui);
        let pointer = resp.hover_pos().or_else(|| {
            self.autoplay.then(|| {
                let a = time * 0.55;
                pos2(
                    rect.center().x + rect.width() * 0.42 * a.sin(),
                    rect.center().y + rect.height() * 0.42 * (a * 1.7 + 0.6).sin(),
                )
            })
        });
        let n = glyphs.glyphs.len();
        let (radius, duration) = (self.radius, self.duration);
        // per-letter remaining scramble time
        let timers = glowkit_core::with_state(
            ui,
            resp.id.with("scrambled"),
            || vec![0.0f32; n],
            |v| {
                v.resize(n, 0.0);
                for (tm, g) in v.iter_mut().zip(&glyphs.glyphs) {
                    *tm = (*tm - dt).max(0.0);
                    if let Some(p) = pointer {
                        let c = rect.min + g.offset + g.size / 2.0;
                        let d = p.distance(c);
                        if d < radius {
                            // letters closest to the pointer stay scrambled the longest
                            *tm = tm.max(duration * (1.0 - d / radius).powf(0.5));
                        }
                    }
                }
                v.clone()
            },
        );
        if pointer.is_some() || timers.iter().any(|v| *v > 0.0) {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter();
        let color = self.color.unwrap_or(t.foreground);
        let hi = self.highlight.unwrap_or(t.accent);
        let tick = (time * self.speed) as u64;
        let charset = if self.charset.is_empty() { glowkit_core::text::SCRAMBLE_CHARS } else { &self.charset };
        for (i, (g, tm)) in glyphs.glyphs.iter().zip(&timers).enumerate() {
            if g.ch.is_whitespace() {
                continue;
            }
            let k = (tm / duration).clamp(0.0, 1.0);
            let pos = rect.min + g.offset;
            if k > 0.02 {
                let ch = scramble_char(0x5C4A, i, tick + i as u64 * 7, charset);
                let c = mix(color, hi, (k * 1.4).min(1.0));
                util::paint_char_in(painter, ch, &font, pos, g.size.x, c, 0.0);
            } else {
                util::paint_char_in(painter, g.ch, &font, pos, g.size.x, color, 0.0);
            }
        }
        resp
    }
}
