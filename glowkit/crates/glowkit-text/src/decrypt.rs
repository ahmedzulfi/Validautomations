use crate::util;
use egui::{Color32, Response, Sense, Ui, Widget};
use glowkit_core::text::{scramble_char, Glyphs, SCRAMBLE_CHARS};
use glowkit_core::{mix, Clock, Theme, Trigger};
use crate::stagger::{stagger_ranks, StaggerFrom};

/// Characters scramble through random symbols, then lock into the real text one by one.
///
/// Use [`Trigger::OnHover`] for the "hyper text" behaviour where hovering replays the decode.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_core::Trigger;
/// ui.add(glowkit_text::DecryptedText::new("ACCESS GRANTED").monospace(true).trigger(Trigger::OnHover));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DecryptedText {
    text: String,
    size: f32,
    color: Option<Color32>,
    scramble_color: Option<Color32>,
    mono: bool,
    bold: bool,
    tick: f32,
    step: f32,
    lead: f32,
    from: StaggerFrom,
    charset: String,
    trigger: Trigger,
}

impl DecryptedText {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 32.0,
            color: None,
            scramble_color: None,
            mono: false,
            bold: false,
            tick: 0.05,
            step: 0.055,
            lead: 0.35,
            from: StaggerFrom::Start,
            charset: SCRAMBLE_CHARS.to_owned(),
            trigger: Trigger::OnAppear,
        }
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Color of settled characters (defaults to the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Color of still-scrambling characters (defaults to the theme accent).
    pub fn scramble_color(mut self, c: Color32) -> Self {
        self.scramble_color = Some(c);
        self
    }
    /// Use the monospace font (classic terminal look, no width wobble).
    pub fn monospace(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }
    /// Fake-bold the text.
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Seconds between scramble symbol changes.
    pub fn tick(mut self, seconds: f32) -> Self {
        self.tick = seconds.max(0.01);
        self
    }
    /// Seconds between consecutive characters settling.
    pub fn step(mut self, seconds: f32) -> Self {
        self.step = seconds.max(0.0);
        self
    }
    /// Seconds of pure scrambling before the first character settles.
    pub fn lead(mut self, seconds: f32) -> Self {
        self.lead = seconds.max(0.0);
        self
    }
    /// Order in which characters settle.
    pub fn reveal_from(mut self, from: StaggerFrom) -> Self {
        self.from = from;
        self
    }
    /// Symbols to scramble through.
    pub fn charset(mut self, chars: impl Into<String>) -> Self {
        self.charset = chars.into();
        self
    }
    /// When the decode plays (`OnHover` replays it whenever the pointer enters).
    pub fn trigger(mut self, t: Trigger) -> Self {
        self.trigger = t;
        self
    }
}

impl Widget for DecryptedText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let color = self.color.unwrap_or(t.foreground);
        let scol = self.scramble_color.unwrap_or(t.accent);
        let font = util::font(self.size, self.mono);
        let glyphs = Glyphs::layout(ui.painter(), &self.text, font.clone(), Some(ui.available_width()));
        let bold = util::weight(self.bold, self.size);
        let (rect, resp) = ui.allocate_exact_size(glyphs.size + egui::vec2(bold, 0.0), Sense::click());
        let n = glyphs.visible_count();
        let total = self.lead + self.step * n as f32 + 0.2;
        let elapsed = Clock::triggered(ui, resp.id, self.trigger, total, resp.hovered(), resp.clicked());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let ranks = stagger_ranks(n, self.from, resp.id.value());
        let tick = (elapsed / self.tick) as u64;
        let painter = ui.painter();
        let mut vi = 0;
        for g in &glyphs.glyphs {
            if g.ch.is_whitespace() {
                continue;
            }
            let at = self.lead + ranks[vi] as f32 * self.step;
            vi += 1;
            let pos = rect.min + g.offset;
            if elapsed >= at {
                // settled: flash from the scramble color into the final color
                let k = ((elapsed - at) / 0.25).clamp(0.0, 1.0);
                let c = mix(scol, color, k);
                let gal = util::layout_char(painter, g.ch, &font);
                util::paint_galley(painter, pos, &gal, c, bold);
            } else {
                let ch = scramble_char(resp.id.value(), g.index, tick, &self.charset);
                util::paint_char_in(painter, ch, &font, pos, g.size.x, scol.gamma_multiply(0.85), bold);
            }
        }
        resp
    }
}
