//! Text transitions: state swaps, staggered reveals, a shimmering "thinking" label, a scrolling
//! reasoning card and word-by-word streaming.

use super::common::{change, cr, layer, material, phase, span, surface, MotionCurve};
use egui::emath::TSTransform;
use egui::{pos2, vec2, Align2, Color32, FontId, Galley, Rect, Response, Sense, Ui, Widget};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::text::Glyphs;
use glowkit_core::{paint, Theme};
use std::sync::Arc;

/// A status label that swaps its text with a tiny lift: the old words rise a few points while
/// blurring out, then the new words rise into place from below.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::StateText::new("Transaction processing..."));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct StateText {
    text: String,
    size: f32,
    color: Option<Color32>,
    dur: f32,
    distance: f32,
    blur: f32,
    curve: MotionCurve,
}

impl StateText {
    /// Show `text`; changing it plays the swap.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 17.0,
            color: None,
            dur: 0.15,
            distance: 4.0,
            blur: 2.0,
            curve: MotionCurve::EASE_IN_OUT,
        }
    }
    /// Font size in points (default 17).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(6.0);
        self
    }
    /// Text color (default: the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Seconds for each half of the swap (default 0.15).
    pub fn duration(mut self, s: f32) -> Self {
        self.dur = s.max(0.02);
        self
    }
    /// Lift distance in points (default 4).
    pub fn distance(mut self, d: f32) -> Self {
        self.distance = d;
        self
    }
    /// Blur in points at the ends of the swap (default 2).
    pub fn blur(mut self, b: f32) -> Self {
        self.blur = b.max(0.0);
        self
    }
    /// Timing curve (default ease-in-out).
    pub fn curve(mut self, c: MotionCurve) -> Self {
        self.curve = c;
        self
    }
}

impl Widget for StateText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = FontId::proportional(self.size);
        let color = self.color.unwrap_or(t.foreground);
        let g = ui.painter().layout_no_wrap(self.text.clone(), font.clone(), color);
        let (rect, response) =
            ui.allocate_exact_size(vec2(g.size().x + 8.0, g.size().y + self.distance * 2.0 + 6.0), Sense::hover());
        let ch = change(ui, response.id, &self.text, self.dur * 2.0 + 0.05);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let at = |g: &Arc<Galley>| rect.center() - g.size() / 2.0;
        if ch.age >= self.dur * 2.0 || ch.prev == self.text {
            ui.painter().galley_with_override_text_color(at(&g), g, color);
            return response;
        }
        if ch.age < self.dur {
            let k = self.curve.at(ch.age / self.dur);
            let og = ui.painter().layout_no_wrap(ch.prev.clone(), font, color);
            layer(ui, TSTransform::from_translation(vec2(0.0, -self.distance * k)), 1.0 - k, self.blur * k, |p| {
                p.galley_with_override_text_color(at(&og), og.clone(), color);
            });
        } else {
            let k = self.curve.at((ch.age - self.dur) / self.dur);
            layer(
                ui,
                TSTransform::from_translation(vec2(0.0, self.distance * (1.0 - k))),
                k,
                self.blur * (1.0 - k),
                |p| {
                    p.galley_with_override_text_color(at(&g), g.clone(), color);
                },
            );
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone)]
struct RevealLine {
    text: String,
    strong: bool,
}

/// A small block of text that reveals line by line (or word by word): each piece rises 12 points,
/// un-blurs and fades in on a long smooth curve, a beat after the one before. Hiding is a quick
/// fade.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(
///     glowkit_interactions::StaggerReveal::new(true)
///         .line("Pull request opened")
///         .secondary("Review requested from 3 teammates"),
/// );
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct StaggerReveal {
    shown: bool,
    lines: Vec<RevealLine>,
    size: f32,
    dur: f32,
    stagger: f32,
    distance: f32,
    blur: f32,
    words: bool,
    centered: bool,
    curve: MotionCurve,
}

impl StaggerReveal {
    /// A reveal that plays when `shown` turns true.
    pub fn new(shown: bool) -> Self {
        Self {
            shown,
            lines: Vec::new(),
            size: 16.0,
            dur: 0.5,
            stagger: 0.04,
            distance: 12.0,
            blur: 3.0,
            words: false,
            centered: false,
            curve: MotionCurve::SMOOTH_OUT,
        }
    }
    /// Add a primary (strong) line.
    pub fn line(mut self, s: impl Into<String>) -> Self {
        self.lines.push(RevealLine { text: s.into(), strong: true });
        self
    }
    /// Add a secondary (muted) line.
    pub fn secondary(mut self, s: impl Into<String>) -> Self {
        self.lines.push(RevealLine { text: s.into(), strong: false });
        self
    }
    /// Base font size (default 16).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(6.0);
        self
    }
    /// Seconds per piece (default 0.5).
    pub fn duration(mut self, s: f32) -> Self {
        self.dur = s.max(0.02);
        self
    }
    /// Delay between pieces in seconds (default 0.04).
    pub fn stagger(mut self, s: f32) -> Self {
        self.stagger = s.max(0.0);
        self
    }
    /// Rise distance in points (default 12).
    pub fn distance(mut self, d: f32) -> Self {
        self.distance = d;
        self
    }
    /// Starting blur in points (default 3).
    pub fn blur(mut self, b: f32) -> Self {
        self.blur = b.max(0.0);
        self
    }
    /// Stagger word by word instead of line by line.
    pub fn by_words(mut self, on: bool) -> Self {
        self.words = on;
        self
    }
    /// Center the lines (default left-aligned).
    pub fn centered(mut self, on: bool) -> Self {
        self.centered = on;
        self
    }
    /// Timing curve (default [`MotionCurve::SMOOTH_OUT`]).
    pub fn curve(mut self, c: MotionCurve) -> Self {
        self.curve = c;
        self
    }
}

impl Widget for StaggerReveal {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let laid: Vec<(Glyphs, Color32)> = self
            .lines
            .iter()
            .map(|l| {
                let (font, col) = if l.strong {
                    (FontId::proportional(self.size), t.foreground)
                } else {
                    (FontId::proportional(self.size * 0.86), t.muted_foreground)
                };
                (Glyphs::layout(ui.painter(), &l.text, font, None), col)
            })
            .collect();
        let gap = self.size * 0.3;
        let w = laid.iter().map(|(g, _)| g.size.x).fold(0.0, f32::max);
        let h = laid.iter().map(|(g, _)| g.size.y).sum::<f32>() + gap * (laid.len().saturating_sub(1)) as f32;
        let (rect, response) = ui.allocate_exact_size(vec2(w + 4.0, h + self.distance + 4.0), Sense::hover());
        let units: usize = if self.words { laid.iter().map(|(g, _)| g.word_count()).sum() } else { laid.len() };
        let ph = phase(
            ui,
            response.id,
            self.shown,
            self.dur + self.stagger * units as f32,
            0.2,
            MotionCurve::LINEAR,
            MotionCurve::LINEAR,
        );
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let hide = if ph.on { 1.0 } else { 1.0 - span(ph.age, 0.0, 0.2) };
        if hide <= 0.0 {
            return response;
        }
        let progress = |i: usize| -> f32 {
            if ph.on {
                self.curve.timed(ph.age, self.stagger * i as f32, self.dur)
            } else {
                1.0
            }
        };
        let mut y = rect.min.y + 2.0;
        let mut unit = 0usize;
        for (g, col) in &laid {
            let x0 = if self.centered { rect.center().x - g.size.x / 2.0 } else { rect.min.x + 2.0 };
            let origin = pos2(x0, y);
            if self.words {
                for wi in 0..g.word_count() {
                    let k = progress(unit);
                    unit += 1;
                    let pieces: Vec<_> = g.glyphs.iter().filter(|gb| gb.word == wi && !gb.ch.is_whitespace()).collect();
                    let Some(first) = pieces.first() else { continue };
                    let word: String = pieces.iter().map(|gb| gb.ch).collect();
                    let p0 = origin + first.offset;
                    layer(
                        ui,
                        TSTransform::from_translation(vec2(0.0, self.distance * (1.0 - k))),
                        k * hide,
                        self.blur * (1.0 - k),
                        |p| {
                            p.text(p0, Align2::LEFT_TOP, &word, g.font.clone(), *col);
                        },
                    );
                }
            } else {
                let k = progress(unit);
                unit += 1;
                layer(
                    ui,
                    TSTransform::from_translation(vec2(0.0, self.distance * (1.0 - k))),
                    k * hide,
                    self.blur * (1.0 - k),
                    |p| {
                        p.galley_with_override_text_color(origin, g.galley.clone(), *col);
                    },
                );
            }
            y += g.size.y + gap;
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// An AI "thinking" label: a soft highlight sweeps across muted text on a loop, and when the
/// state changes the old words lift away and the new ones rise in.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::ThinkingText::new("Reading the codebase"));
/// // or let it walk through a list on its own
/// ui.add(glowkit_interactions::ThinkingText::cycle(&["Planning", "Searching", "Writing"], 2.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ThinkingText {
    states: Vec<String>,
    hold: f32,
    size: f32,
    sweep: f32,
    swap: f32,
    distance: f32,
    base: Option<Color32>,
    highlight: Option<Color32>,
    shimmer: bool,
}

impl ThinkingText {
    /// Show `text` with the shimmer; changing it plays the swap.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            states: vec![text.into()],
            hold: 2.0,
            size: 17.0,
            sweep: 2.0,
            swap: 0.15,
            distance: 8.0,
            base: None,
            highlight: None,
            shimmer: true,
        }
    }
    /// Walk through `states` by itself, holding each for `hold` seconds.
    pub fn cycle(states: &[&str], hold: f32) -> Self {
        let mut s = Self::new("");
        s.states = states.iter().map(|s| s.to_string()).collect();
        s.hold = hold.max(0.2);
        s
    }
    /// Font size in points (default 17).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(6.0);
        self
    }
    /// Seconds for one shimmer sweep (default 2).
    pub fn sweep(mut self, s: f32) -> Self {
        self.sweep = s.max(0.2);
        self
    }
    /// Seconds for each half of a swap (default 0.15).
    pub fn swap(mut self, s: f32) -> Self {
        self.swap = s.max(0.02);
        self
    }
    /// Muted text color (default: between foreground and background).
    pub fn base(mut self, c: Color32) -> Self {
        self.base = Some(c);
        self
    }
    /// Highlight color (default: the foreground).
    pub fn highlight(mut self, c: Color32) -> Self {
        self.highlight = Some(c);
        self
    }
    /// Turn the shimmer on or off (on by default).
    pub fn shimmer(mut self, on: bool) -> Self {
        self.shimmer = on;
        self
    }
}

impl Widget for ThinkingText {
    fn ui(self, ui: &mut Ui) -> Response {
        let th = Theme::of(ui);
        let font = FontId::proportional(self.size);
        let now = glowkit_core::time(ui);
        let text = if self.states.len() > 1 {
            let i = (now / self.hold) as usize % self.states.len();
            self.states[i].clone()
        } else {
            self.states.first().cloned().unwrap_or_default()
        };
        let widest = self.states.iter().map(|s| super::common::text_width(ui.painter(), s, &font)).fold(0.0, f32::max);
        let (rect, response) =
            ui.allocate_exact_size(vec2(widest + 8.0, self.size * 1.4 + self.distance * 2.0), Sense::hover());
        let ch = change(ui, response.id, &text, self.swap * 2.0 + 0.1);
        if self.shimmer || self.states.len() > 1 {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let base = self.base.unwrap_or(mix(th.foreground, th.background, 0.52));
        let hi = self.highlight.unwrap_or(th.foreground);
        let shimmer = self.shimmer;
        let sweep = self.sweep;
        let paint = |p: &egui::Painter, s: &str| {
            let g = p.layout_no_wrap(s.to_owned(), font.clone(), Color32::WHITE);
            let pos = pos2(rect.center().x - g.size().x / 2.0, rect.center().y - g.size().y / 2.0);
            if !shimmer {
                p.galley_with_override_text_color(pos, g, base);
                return;
            }
            let w = g.size().x.max(1.0);
            let band = (w * 0.55).max(60.0);
            let u = (now / sweep).fract();
            let hx = pos.x - band + u * (w + band * 2.0);
            paint::galley_with(p, pos, &g, |q| {
                let d = ((q.x - hx) / band).abs();
                let k = (1.0 - d).clamp(0.0, 1.0);
                mix(base, hi, k * k * (3.0 - 2.0 * k))
            });
        };
        let sw = self.swap;
        if ch.age >= sw * 2.0 + 0.05 || ch.prev == text {
            paint(ui.painter(), &text);
        } else if ch.age < sw {
            let k = MotionCurve::EASE_IN_OUT.at(ch.age / sw);
            layer(ui, TSTransform::from_translation(vec2(0.0, -self.distance * k)), 1.0 - k, 2.0 * k, |p| {
                paint(p, &ch.prev)
            });
        } else {
            let k = MotionCurve::EASE_IN_OUT.at((ch.age - sw - 0.05) / sw);
            layer(ui, TSTransform::from_translation(vec2(0.0, self.distance * (1.0 - k))), k, 2.0 * (1.0 - k), |p| {
                paint(p, &text)
            });
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A card of agent reasoning that scrolls itself: every beat the text glides up by a couple of
/// lines on an ease-out, the top and bottom fade into the card, and it loops seamlessly.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::ReasoningScroll::new("First, check the cache...\n\nThen ..."));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ReasoningScroll {
    text: String,
    size: egui::Vec2,
    font: f32,
    lines: f32,
    step: f32,
    hold: f32,
    fade: f32,
    playing: bool,
}

impl ReasoningScroll {
    /// A scrolling card showing `text` (paragraphs separated by blank lines).
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: vec2(300.0, 140.0),
            font: 13.0,
            lines: 2.0,
            step: 0.5,
            hold: 0.84,
            fade: 28.0,
            playing: true,
        }
    }
    /// Card size (default 300 x 140).
    pub fn size(mut self, s: egui::Vec2) -> Self {
        self.size = s;
        self
    }
    /// Font size (default 13).
    pub fn font_size(mut self, s: f32) -> Self {
        self.font = s.max(6.0);
        self
    }
    /// Lines advanced per beat (default 2).
    pub fn lines(mut self, n: f32) -> Self {
        self.lines = n.max(0.25);
        self
    }
    /// Seconds each glide takes (default 0.5).
    pub fn step(mut self, s: f32) -> Self {
        self.step = s.max(0.05);
        self
    }
    /// Seconds to rest between glides (default 0.84).
    pub fn hold(mut self, s: f32) -> Self {
        self.hold = s.max(0.0);
        self
    }
    /// Height of the top / bottom fades (default 28).
    pub fn fade(mut self, f: f32) -> Self {
        self.fade = f.max(0.0);
        self
    }
    /// Pause or resume the scrolling.
    pub fn playing(mut self, on: bool) -> Self {
        self.playing = on;
        self
    }
}

#[derive(Clone, Copy)]
struct ScrollClock {
    offset_steps: f32,
    last: f64,
}

impl Widget for ReasoningScroll {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(self.size, Sense::hover());
        let now = ui.input(|i| i.time);
        let playing = self.playing;
        let cycle = self.step + self.hold;
        // accumulate time only while playing so pausing freezes it mid-glide
        let clock = glowkit_core::with_state(
            ui,
            response.id.with("scroll"),
            || ScrollClock { offset_steps: 0.0, last: now },
            |c| {
                let dt = ((now - c.last) as f32).clamp(0.0, 0.1);
                c.last = now;
                if playing {
                    c.offset_steps += dt;
                }
                *c
            },
        );
        if playing {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let bg = surface(&t);
        let r = 14.0;
        material(painter, &t, rect, r, bg);
        let pad = vec2(16.0, 12.0);
        let view = rect.shrink2(pad);
        let font = FontId::proportional(self.font);
        let galley = painter.layout(self.text.clone(), font.clone(), mix(t.foreground, bg, 0.12), view.width());
        let line_h = painter.fonts_mut(|f| f.row_height(&font)) * 1.0;
        let para_gap = line_h;
        let copy_h = galley.size().y + para_gap;
        // which beat and how far into it
        let e = clock.offset_steps;
        let beat = (e / cycle).floor();
        let u = ((e - beat * cycle) / self.step).clamp(0.0, 1.0);
        let k = MotionCurve::EASE_OUT.at(u);
        let travel = (beat + k) * self.lines * line_h;
        let off = travel.rem_euclid(copy_h);
        let clip = painter.with_clip_rect(rect.shrink(1.0));
        for i in 0..3 {
            let y = view.min.y - off + i as f32 * copy_h;
            if y > rect.max.y || y + galley.size().y < rect.min.y {
                continue;
            }
            clip.galley_with_override_text_color(pos2(view.min.x, y), galley.clone(), t.foreground);
        }
        // fades only span the text column so they never square off the rounded corners
        let inner = Rect::from_x_y_ranges(view.min.x - 2.0..=view.max.x + 2.0, rect.min.y + 1.0..=rect.max.y - 1.0);
        let fade = self.fade.min(inner.height() / 2.0);
        paint::fade_edges_vertical(&clip, inner, fade, bg);
        response
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct StreamState {
    text: String,
    times: Vec<f64>,
}

/// Text that streams in word by word, each word fading up out of a soft blur a beat after the
/// last, with a caret riding the end while words are still arriving. Feed it the text so far
/// (from a model, say) and only the new words animate; or hand it everything at once.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let reply_so_far = String::new();
/// ui.add(glowkit_interactions::StreamText::new(&reply_so_far).width(360.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct StreamText {
    text: String,
    width: f32,
    size: f32,
    gap: f32,
    fade: f32,
    blur: f32,
    rise: f32,
    caret: bool,
    color: Option<Color32>,
}

impl StreamText {
    /// Stream `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            width: 360.0,
            size: 15.0,
            gap: 0.06,
            fade: 0.32,
            blur: 2.5,
            rise: 3.0,
            caret: true,
            color: None,
        }
    }
    /// Wrap width (default 360).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(40.0);
        self
    }
    /// Font size (default 15).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(6.0);
        self
    }
    /// Seconds between words (default 0.06).
    pub fn word_gap(mut self, s: f32) -> Self {
        self.gap = s.max(0.0);
        self
    }
    /// Seconds each word takes to settle (default 0.32).
    pub fn fade(mut self, s: f32) -> Self {
        self.fade = s.max(0.02);
        self
    }
    /// Starting blur of each word (default 2.5).
    pub fn blur(mut self, b: f32) -> Self {
        self.blur = b.max(0.0);
        self
    }
    /// Show a caret while words are arriving (default on).
    pub fn caret(mut self, on: bool) -> Self {
        self.caret = on;
        self
    }
    /// Text color (default: the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

impl Widget for StreamText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = FontId::proportional(self.size);
        let g = Glyphs::layout(ui.painter(), &self.text, font.clone(), Some(self.width));
        let full_h = {
            // reserve the final height so the layout does not jump while streaming
            g.size.y.max(self.size * 1.3)
        };
        let (rect, response) = ui.allocate_exact_size(vec2(self.width, full_h + 4.0), Sense::hover());
        let now = ui.input(|i| i.time);
        let words = g.word_count();
        let gap = self.gap as f64;
        let text = self.text.clone();
        let times = glowkit_core::with_state(ui, response.id.with("stream"), StreamState::default, |s| {
            if !text.starts_with(&s.text) {
                s.times.clear();
            }
            s.text = text.clone();
            while s.times.len() < words {
                let next = s.times.last().map_or(now, |l| (l + gap).max(now));
                s.times.push(next);
            }
            s.times.truncate(words);
            s.times.clone()
        });
        let last = times.last().copied().unwrap_or(now);
        let streaming = now < last + self.fade as f64;
        if streaming {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let color = self.color.unwrap_or(t.foreground);
        let origin = rect.min + vec2(0.0, 2.0);
        let mut caret_at = None;
        for (wi, at) in times.iter().enumerate().take(words) {
            let age = (now - at) as f32;
            if age < 0.0 {
                continue;
            }
            let pieces: Vec<_> = g.glyphs.iter().filter(|gb| gb.word == wi && !gb.ch.is_whitespace()).collect();
            let Some(first) = pieces.first() else { continue };
            let lastg = pieces[pieces.len() - 1];
            let word: String = pieces.iter().map(|gb| gb.ch).collect();
            let k = MotionCurve::SMOOTH_OUT.at(age / self.fade);
            let p0 = origin + first.offset;
            caret_at = Some((origin + lastg.offset + vec2(lastg.size.x + 3.0, 0.0), lastg.size.y));
            if k >= 1.0 {
                ui.painter().text(p0, Align2::LEFT_TOP, &word, font.clone(), color);
                continue;
            }
            let a = span(age, 0.0, self.fade * 0.7);
            layer(ui, TSTransform::from_translation(vec2(0.0, self.rise * (1.0 - k))), a, self.blur * (1.0 - k), |p| {
                p.text(p0, Align2::LEFT_TOP, &word, font.clone(), color);
            });
        }
        if self.caret && streaming {
            if let Some((p, h)) = caret_at {
                let blink = 0.55 + 0.45 * (now as f32 * 7.0).cos().abs();
                ui.painter().rect_filled(
                    Rect::from_min_size(p + vec2(0.0, h * 0.18), vec2(self.size * 0.5, h * 0.68)),
                    cr(2.0),
                    with_alpha(t.foreground, 0.75 * blink),
                );
            }
        }
        response
    }
}
