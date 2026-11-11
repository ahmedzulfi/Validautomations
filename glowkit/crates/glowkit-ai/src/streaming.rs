//! Token-by-token response reveal.

use crate::common::{self, Fx};
use crate::markdown::Markdown;
use egui::{pos2, vec2, Color32, CornerRadius, FontId, Rect, Response, Sense, Ui, Widget};
use glowkit_core::anim::Clock;
use glowkit_core::color::with_alpha;
use glowkit_core::{Theme, Trigger};

/// How each newly revealed token appears.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RevealStyle {
    /// Tokens fade in (and rise a pixel or two).
    #[default]
    Fade,
    /// Tokens appear instantly, typewriter style.
    Typewriter,
    /// Tokens sharpen from a soft blur while fading in.
    Blur,
}

/// Split text into token-ish chunks: a word plus its trailing whitespace.
pub(crate) fn token_ends(text: &str) -> Vec<usize> {
    // char index (exclusive) where each token ends
    let mut ends = Vec::new();
    let mut prev_space = false;
    let mut count = 0;
    for (i, c) in text.chars().enumerate() {
        let space = c.is_whitespace();
        if !space && prev_space {
            ends.push(i);
        }
        prev_space = space;
        count = i + 1;
    }
    if count > 0 {
        ends.push(count);
    }
    ends
}

/// Reveals a full response token by token, the way chat UIs show a streaming answer: every new
/// token fades in, and a blinking caret rides along the end. Drive it by time
/// ([`StreamingText::tokens_per_second`]) or by an explicit [`StreamingText::progress`].
///
/// With [`StreamingText::markdown`] the revealed prefix is rendered as Markdown (headings, code
/// blocks, tables all appear as they stream in).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::StreamingText;
/// ui.add(StreamingText::new("Hello! How can I help you today?").tokens_per_second(12.0));
/// ui.add(StreamingText::new("Rendered from **Markdown**").markdown(true).progress(0.5));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct StreamingText<'a> {
    text: &'a str,
    progress: Option<f32>,
    tps: f32,
    style: RevealStyle,
    fade_secs: f32,
    size: f32,
    color: Option<Color32>,
    caret: bool,
    markdown: bool,
    trigger: Trigger,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> StreamingText<'a> {
    /// Reveal `text`.
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            progress: None,
            tps: 14.0,
            style: RevealStyle::Fade,
            fade_secs: 0.45,
            size: 15.0,
            color: None,
            caret: true,
            markdown: false,
            trigger: Trigger::OnAppear,
            id_salt: None,
        }
    }
    /// Drive the reveal yourself: 0 = nothing shown, 1 = everything.
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = Some(p.clamp(0.0, 1.0));
        self
    }
    /// Time-driven speed (ignored when [`Self::progress`] is set).
    pub fn tokens_per_second(mut self, tps: f32) -> Self {
        self.tps = tps.max(0.1);
        self
    }
    /// How tokens appear.
    pub fn style(mut self, style: RevealStyle) -> Self {
        self.style = style;
        self
    }
    /// Seconds each token takes to fade in.
    pub fn fade_duration(mut self, secs: f32) -> Self {
        self.fade_secs = secs.max(0.01);
        self
    }
    /// Font size.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Text color. Defaults to the theme foreground.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Blinking caret while revealing (default on).
    pub fn caret(mut self, on: bool) -> Self {
        self.caret = on;
        self
    }
    /// Render the revealed prefix as Markdown.
    pub fn markdown(mut self, on: bool) -> Self {
        self.markdown = on;
        self
    }
    /// When the reveal (re)starts: once, looping, on hover or on click.
    pub fn trigger(mut self, trigger: Trigger) -> Self {
        self.trigger = trigger;
        self
    }
    /// Stable id for the reveal clock.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }
}

impl Widget for StreamingText<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let color = self.color.unwrap_or(t.foreground);
        let id = common::widget_id(ui, self.id_salt);
        let ends = token_ends(self.text);
        let n_tok = ends.len().max(1) as f32;
        // position in "tokens": how many tokens have started revealing (fractional)
        let pos = match self.progress {
            Some(p) => p * (n_tok + 1.0),
            None => {
                let duration = n_tok / self.tps + self.fade_secs;
                let prev_hover: bool = glowkit_core::state(ui, id.with("hov"), || false);
                let e = Clock::triggered(ui, id, self.trigger, duration, prev_hover, false);
                e * self.tps
            }
        };
        let fade_tokens = (self.fade_secs * self.tps).max(0.01);
        let done = pos >= n_tok + fade_tokens;
        if !done {
            glowkit_core::animate(ui);
        }
        let resp = if self.markdown {
            let shown = (pos.floor() as usize).min(ends.len());
            let chars = if shown == 0 { 0 } else { ends[shown - 1] };
            let prefix: String = self.text.chars().take(chars).collect();
            let tail = if self.style == RevealStyle::Typewriter || done { 0 } else { 24 };
            ui.add(Markdown::new(&prefix).size(self.size * 0.95).caret(self.caret && !done).fade_tail(tail))
        } else {
            let w = ui.available_width();
            let galley = ui.painter().layout(self.text.to_owned(), FontId::proportional(self.size), color, w);
            let (rect, resp) = ui.allocate_exact_size(vec2(w, galley.size().y), Sense::hover());
            // char -> token
            let mut tok_of = Vec::with_capacity(self.text.len());
            let mut k = 0;
            for i in 0..self.text.chars().count() {
                while k < ends.len() && i >= ends[k] {
                    k += 1;
                }
                tok_of.push(k);
            }
            let style = self.style;
            let alpha_of = |ci: usize| -> f32 {
                let tok = tok_of.get(ci).copied().unwrap_or(usize::MAX) as f32;
                match style {
                    RevealStyle::Typewriter => {
                        if pos > tok {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    _ => ((pos - tok) / fade_tokens).clamp(0.0, 1.0),
                }
            };
            if ui.is_rect_visible(rect) {
                if style == RevealStyle::Blur {
                    // soft halo copies behind tokens that are still fading in
                    for (dx, dy) in [(-1.6f32, 0.0f32), (1.6, 0.0), (0.0, -1.4), (0.0, 1.4)] {
                        common::paint_galley_fx(ui.painter(), rect.min + vec2(dx, dy), &galley, |i| {
                            let a = alpha_of(i);
                            Fx { alpha: if a > 0.0 && a < 1.0 { (1.0 - a) * a * 0.9 } else { 0.0 }, ..Default::default() }
                        });
                    }
                }
                common::paint_galley_fx(ui.painter(), rect.min, &galley, |i| {
                    let a = alpha_of(i);
                    let a = if style == RevealStyle::Blur { a * a } else { a };
                    let rise = if style == RevealStyle::Fade { (1.0 - a) * 3.0 } else { 0.0 };
                    Fx { alpha: a, offset: vec2(0.0, rise), ..Default::default() }
                });
                if self.caret && !done {
                    // caret after the last (partially) visible char
                    let visible = tok_of.iter().take_while(|&&k| (k as f32) < pos).count();
                    let cr = if visible == 0 {
                        Rect::from_min_size(rect.min, vec2(0.0, self.size * 1.2))
                    } else {
                        common::char_rects(&galley, visible - 1..visible).last().copied().map(|r| r.translate(rect.min.to_vec2())).unwrap_or(rect)
                    };
                    let time = glowkit_core::time(ui);
                    let blink = 0.55 + 0.45 * (time * 6.0).cos();
                    let r = Rect::from_min_size(pos2(cr.max.x + 2.0, cr.center().y - self.size * 0.55), vec2(2.0, self.size * 1.1));
                    ui.painter().rect_filled(r, CornerRadius::same(1), with_alpha(color, blink));
                }
            }
            resp
        };
        glowkit_core::set_state(ui, id.with("hov"), resp.hovered());
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::token_ends;

    #[test]
    fn tokens() {
        assert_eq!(token_ends("hi there  you"), vec![3, 10, 13]);
        assert_eq!(token_ends(""), Vec::<usize>::new());
        assert_eq!(token_ends("one"), vec![3]);
    }
}
