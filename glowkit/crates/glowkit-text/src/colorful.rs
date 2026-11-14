use crate::util;
use egui::{vec2, Color32, Response, Sense, Ui, Widget};
use glowkit_core::{anim::stagger, mix, Clock, Theme};

/// Every letter gets its own color from a palette; every few seconds the colors reshuffle and a
/// wave of tiny blur-and-bounce passes through the letters as they change.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::ColorfulText::new("colorful").size(56.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ColorfulText {
    text: String,
    size: f32,
    colors: Vec<Color32>,
    bold: bool,
    interval: f32,
    stagger: f32,
    duration: f32,
}

impl ColorfulText {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 48.0,
            colors: Vec::new(),
            bold: true,
            interval: 3.0,
            stagger: 0.05,
            duration: 0.55,
        }
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Palette to pick letter colors from (defaults to the theme chart palette).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    /// Fake-bold the text (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Seconds between reshuffles.
    pub fn interval(mut self, seconds: f32) -> Self {
        self.interval = seconds.max(0.2);
        self
    }
    /// Seconds between letters in the change wave.
    pub fn stagger(mut self, seconds: f32) -> Self {
        self.stagger = seconds.max(0.0);
        self
    }
}

/// Palette index for letter `i` in generation `gen`. Each letter walks the palette with its own
/// stride, so it never keeps the same color two generations in a row.
fn pick(seed: u64, i: usize, gen: u64, n: usize) -> usize {
    if n <= 1 {
        return 0;
    }
    let mut r = glowkit_core::Rng::new(seed ^ (i as u64).wrapping_mul(0x9E37_79B9));
    let start = r.below(n) as u64;
    let stride = 1 + r.below(n - 1) as u64;
    ((start + gen.wrapping_mul(stride)) % n as u64) as usize
}

impl Widget for ColorfulText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let glyphs = crate::util::glyphs(ui.painter(), &self.text, font.clone(), Some(ui.available_width()));
        let (rect, resp) = ui.allocate_exact_size(glyphs.size + vec2(bold, self.size * 0.16), Sense::hover());
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let palette = if self.colors.is_empty() { t.chart.to_vec() } else { self.colors };
        let n = palette.len();
        let e = Clock::elapsed(ui, resp.id);
        let gen = (e / self.interval) as u64;
        let local = e - gen as f32 * self.interval;
        let seed = 0x5EED;
        let painter = ui.painter();
        for g in &glyphs.glyphs {
            if g.ch.is_whitespace() {
                continue;
            }
            let p = stagger(local, g.index, self.stagger, self.duration);
            let now = palette[pick(seed, g.index, gen, n)];
            let before = if gen == 0 { now } else { palette[pick(seed, g.index, gen - 1, n)] };
            let c = mix(before, now, p);
            let bump = util::bump(p);
            let gal = util::layout_char(painter, g.ch, &font);
            let pos = rect.min + vec2(0.0, self.size * 0.08) + g.offset + vec2(0.0, -bump * self.size * 0.08);
            util::paint_blurred(painter, pos, &gal, c.gamma_multiply(1.0 - 0.2 * bump), bump * self.size * 0.07, bold);
        }
        resp
    }
}
