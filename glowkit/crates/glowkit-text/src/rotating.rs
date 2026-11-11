use crate::stagger::{stagger_ranks, StaggerFrom};
use crate::util;
use egui::{pos2, vec2, Color32, CornerRadius, Rect, Response, Sense, Ui, Widget};
use glowkit_core::{anim::stagger, ease, paint, text::Glyphs, Clock, Easing, Theme};

/// How [`RotatingText`] swaps words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RotateMode {
    /// Letters slide up out of a clipped line while the next word's letters slide in from below.
    #[default]
    Slide,
    /// The old word lifts away into a blur while the new one's letters come into focus.
    Blur,
}

/// Cycles through a list of words with a per-letter staggered transition, optionally on a
/// colored pill whose width animates between words.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.horizontal(|ui| {
///     ui.label("Creative");
///     ui.add(glowkit_text::RotatingText::new(["thinking", "coding", "components"]).pill(true));
/// });
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct RotatingText {
    words: Vec<String>,
    size: f32,
    color: Option<Color32>,
    pill: bool,
    pill_color: Option<Color32>,
    bold: bool,
    interval: f32,
    transition: f32,
    stagger: f32,
    from: StaggerFrom,
    mode: RotateMode,
    easing: Easing,
}

impl RotatingText {
    /// Cycle through `words`.
    pub fn new<S: Into<String>>(words: impl IntoIterator<Item = S>) -> Self {
        Self {
            words: words.into_iter().map(Into::into).collect(),
            size: 36.0,
            color: None,
            pill: false,
            pill_color: None,
            bold: true,
            interval: 2.2,
            transition: 0.55,
            stagger: 0.025,
            from: StaggerFrom::Start,
            mode: RotateMode::Slide,
            easing: Easing::OutBack,
        }
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Text color (defaults to the foreground, or the accent foreground on a pill).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Draw the words on a filled pill whose width follows the current word.
    pub fn pill(mut self, pill: bool) -> Self {
        self.pill = pill;
        self
    }
    /// Pill fill (defaults to the theme accent).
    pub fn pill_color(mut self, c: Color32) -> Self {
        self.pill_color = Some(c);
        self
    }
    /// Fake-bold the text (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Seconds each word stays (including its transition).
    pub fn interval(mut self, seconds: f32) -> Self {
        self.interval = seconds.max(0.2);
        self
    }
    /// Seconds one letter takes to transition.
    pub fn transition(mut self, seconds: f32) -> Self {
        self.transition = seconds.max(0.05);
        self
    }
    /// Seconds between letters.
    pub fn stagger(mut self, seconds: f32) -> Self {
        self.stagger = seconds.max(0.0);
        self
    }
    /// Which letter moves first.
    pub fn stagger_from(mut self, from: StaggerFrom) -> Self {
        self.from = from;
        self
    }
    /// Transition style.
    pub fn mode(mut self, mode: RotateMode) -> Self {
        self.mode = mode;
        self
    }
    /// Easing for each letter.
    pub fn easing(mut self, e: Easing) -> Self {
        self.easing = e;
        self
    }
}

impl Widget for RotatingText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let painter = ui.painter().clone();
        let n = self.words.len().max(1);
        let id = ui.next_auto_id();
        let elapsed = Clock::elapsed(ui, id);
        let cycle = (elapsed / self.interval).floor() as usize;
        let local = elapsed - cycle as f32 * self.interval;
        let cur = cycle % n;
        let prev = (cycle + n - 1) % n;
        let has_prev = cycle > 0;
        let layout = |i: usize| crate::util::glyphs(&painter, self.words.get(i).map(String::as_str).unwrap_or(""), font.clone(), None);
        let g_cur = layout(cur);
        let g_prev = layout(prev);
        let w_cur = g_cur.size.x + bold;
        let w_prev = if has_prev { g_prev.size.x + bold } else { w_cur };
        let span = self.transition + self.stagger * g_cur.glyphs.len().max(g_prev.glyphs.len()).saturating_sub(1) as f32;
        let wk = ease(Easing::OutCubic, (local / (span * 0.9).max(1e-3)).min(1.0));
        let text_w = w_prev + (w_cur - w_prev) * wk;
        let line_h = g_cur.size.y.max(self.size * 1.2);
        let pad = if self.pill { vec2(self.size * 0.42, self.size * 0.16) } else { vec2(0.0, 0.0) };
        let (rect, resp) = ui.allocate_exact_size(vec2(text_w, line_h) + pad * 2.0, Sense::hover());
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let fill = self.pill_color.unwrap_or(t.accent);
        let color = self.color.unwrap_or(if self.pill { glowkit_core::color::contrast_text(fill) } else { t.foreground });
        if self.pill {
            let r = (rect.height() * 0.28).min(t.radius * 1.2);
            paint::soft_shadow(&painter, rect, r, 14.0, vec2(0.0, 4.0), fill.gamma_multiply(0.35));
            painter.rect_filled(rect, CornerRadius::same(r as u8), fill);
        }
        let inner = rect.shrink2(pad);
        let clip = match self.mode {
            RotateMode::Slide => Rect::from_min_max(pos2(rect.min.x - bold, inner.min.y - pad.y * 0.5), pos2(rect.max.x + bold, inner.max.y + pad.y * 0.5)),
            RotateMode::Blur => ui.clip_rect(),
        };
        let p = painter.with_clip_rect(clip.intersect(ui.clip_rect()));
        let draw = |g: &Glyphs, entering: bool| {
            let ranks = stagger_ranks(g.glyphs.len(), self.from, 7);
            let x0 = inner.center().x - (g.size.x + bold) / 2.0;
            let y0 = inner.center().y - g.size.y / 2.0;
            for (gi, gl) in g.glyphs.iter().enumerate() {
                if gl.ch.is_whitespace() {
                    continue;
                }
                let lin = stagger(local, ranks[gi], self.stagger, self.transition);
                let galley = util::layout_char(&p, gl.ch, &font);
                let base = pos2(x0, y0) + gl.offset;
                match (self.mode, entering) {
                    (RotateMode::Slide, true) => {
                        let k = ease(self.easing, lin);
                        let off = vec2(0.0, (1.0 - k) * line_h * 1.05);
                        util::paint_galley(&p, base + off, &galley, color.gamma_multiply(lin.min(1.0).sqrt()), bold);
                    }
                    (RotateMode::Slide, false) => {
                        let k = ease(Easing::InOutCubic, lin);
                        let off = vec2(0.0, -k * line_h * 1.05);
                        util::paint_galley(&p, base + off, &galley, color.gamma_multiply(1.0 - k * k), bold);
                    }
                    (RotateMode::Blur, true) => {
                        let k = ease(Easing::OutCubic, lin);
                        let off = vec2(0.0, (1.0 - k) * self.size * 0.35);
                        util::paint_blurred(&p, base + off, &galley, color.gamma_multiply(k), (1.0 - k) * self.size * 0.2, bold);
                    }
                    (RotateMode::Blur, false) => {
                        let k = ease(Easing::OutCubic, (local / (self.transition * 0.9)).min(1.0));
                        let center = inner.center() + (base + vec2(gl.size.x / 2.0, gl.size.y / 2.0) - inner.center()) * (1.0 + k * 0.4) + vec2(k * self.size * 0.6, -k * self.size * 0.9);
                        let c = color.gamma_multiply((1.0 - k).powi(2));
                        util::paint_blurred(&p, center - galley.size() / 2.0, &galley, c, k * self.size * 0.25, bold);
                    }
                }
            }
        };
        if has_prev && local < span + 0.2 {
            draw(&g_prev, false);
        }
        draw(&g_cur, true);
        resp
    }
}
