//! Small "the model is working" indicators.

use crate::common;
use egui::{pos2, vec2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Response, Sense, Stroke, Ui, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::{paint, Theme};
use std::f32::consts::TAU;

/// Text with a soft highlight sweeping across it: the "Thinking…" / "Searching the web…" label.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_ai::ShimmerText::new("Thinking…"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ShimmerText<'a> {
    text: &'a str,
    size: f32,
    speed: f32,
    base: Option<Color32>,
    shine: Option<Color32>,
    active: bool,
}

impl<'a> ShimmerText<'a> {
    /// Shimmering `text`.
    pub fn new(text: &'a str) -> Self {
        Self { text, size: 14.0, speed: 1.8, base: None, shine: None, active: true }
    }
    /// Font size.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Seconds per sweep.
    pub fn speed(mut self, secs: f32) -> Self {
        self.speed = secs.max(0.2);
        self
    }
    /// Base (dim) color. Defaults to the muted foreground.
    pub fn color(mut self, c: Color32) -> Self {
        self.base = Some(c);
        self
    }
    /// Highlight color. Defaults to the foreground.
    pub fn shine(mut self, c: Color32) -> Self {
        self.shine = Some(c);
        self
    }
    /// Turn the sweep off (renders plain text in the base color).
    pub fn active(mut self, on: bool) -> Self {
        self.active = on;
        self
    }
}

/// Paint shimmering text with its top-left at `pos`; returns the rect.
pub(crate) fn paint_shimmer(
    ui: &Ui,
    pos: Pos2,
    text: &str,
    size: f32,
    base: Color32,
    shine: Color32,
    speed: f32,
) -> Rect {
    let g = ui.painter().layout_no_wrap(text.to_owned(), FontId::proportional(size), base);
    let rect = Rect::from_min_size(pos, g.size());
    let time = glowkit_core::time(ui);
    let band = 0.35;
    let phase = (time / speed).fract() * (1.0 + band * 2.0) - band;
    let r = rect;
    paint::galley_with(ui.painter(), pos, &g, |p| {
        let u = (p.x - r.min.x) / r.width().max(1.0);
        let k = (1.0 - ((u - phase) / band).abs()).clamp(0.0, 1.0);
        base.lerp_to_gamma(shine, k * k * (3.0 - 2.0 * k))
    });
    glowkit_core::animate(ui);
    rect
}

impl Widget for ShimmerText<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let base = self.base.unwrap_or(t.muted_foreground);
        let shine = self.shine.unwrap_or(t.foreground);
        let g = ui.painter().layout_no_wrap(self.text.to_owned(), FontId::proportional(self.size), base);
        let (rect, resp) = ui.allocate_exact_size(g.size(), Sense::hover());
        if ui.is_rect_visible(rect) {
            if self.active {
                paint_shimmer(ui, rect.min, self.text, self.size, base, shine, self.speed);
            } else {
                ui.painter().galley(rect.min, g, base);
            }
        }
        resp
    }
}

/// Three dots bouncing in sequence, optionally inside a chat bubble: "the assistant is typing".
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_ai::TypingIndicator::new().bubble(true));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TypingIndicator {
    size: f32,
    bubble: bool,
    color: Option<Color32>,
}

impl Default for TypingIndicator {
    fn default() -> Self {
        Self::new()
    }
}

impl TypingIndicator {
    /// Bare dots.
    pub fn new() -> Self {
        Self { size: 7.0, bubble: false, color: None }
    }
    /// Dot diameter.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Draw a rounded bubble behind the dots.
    pub fn bubble(mut self, on: bool) -> Self {
        self.bubble = on;
        self
    }
    /// Dot color. Defaults to the muted foreground.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

impl Widget for TypingIndicator {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let d = self.size;
        let gap = d * 0.7;
        let inner = vec2(d * 3.0 + gap * 2.0, d * 2.4);
        let size = if self.bubble { inner + vec2(30.0, 18.0) } else { inner };
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        if ui.is_rect_visible(rect) {
            let p = ui.painter();
            if self.bubble {
                p.rect_filled(
                    rect,
                    CornerRadius { nw: 18, ne: 18, sw: 6, se: 18 },
                    if t.dark { t.secondary } else { t.muted },
                );
            }
            let col = self.color.unwrap_or(t.muted_foreground);
            let time = glowkit_core::time(ui);
            let c = rect.center();
            for i in 0..3 {
                let ph = ((time * 1.25 - i as f32 * 0.16).rem_euclid(1.0) * TAU).sin().max(0.0);
                let ph = ph * ph;
                let x = c.x + (i as f32 - 1.0) * (d + gap);
                p.circle_filled(pos2(x, c.y + d * 0.35 - ph * d * 0.75), d / 2.0, with_alpha(col, 0.45 + 0.55 * ph));
            }
            glowkit_core::animate(ui);
        }
        resp
    }
}

/// Visual style of a [`Loader`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoaderKind {
    /// Three dots pulsing in turn.
    #[default]
    Dots,
    /// Three dots bouncing (typing).
    Typing,
    /// One dot breathing with an expanding ring.
    Pulse,
    /// Five bars rising and falling like a wave.
    Wave,
    /// A spinning arc.
    Circular,
    /// A row of bars with a travelling highlight.
    Bars,
    /// Text with a sweeping shimmer.
    TextShimmer,
    /// Text whose letters fade in and out one after another.
    TextBlink,
}

/// Compact loading indicators for AI states, in several styles ([`LoaderKind`]).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::{Loader, LoaderKind};
/// ui.add(Loader::new(LoaderKind::Wave));
/// ui.add(Loader::new(LoaderKind::TextShimmer).text("Generating"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Loader<'a> {
    kind: LoaderKind,
    size: f32,
    color: Option<Color32>,
    text: &'a str,
}

impl<'a> Loader<'a> {
    /// A loader of the given kind.
    pub fn new(kind: LoaderKind) -> Self {
        Self { kind, size: 22.0, color: None, text: "Thinking" }
    }
    /// Overall height in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Main color. Defaults to the foreground (text kinds use muted/foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Label for the text kinds.
    pub fn text(mut self, text: &'a str) -> Self {
        self.text = text;
        self
    }
}

fn dots(p: &Painter, c: Pos2, s: f32, col: Color32, time: f32) {
    let d = s * 0.28;
    for i in 0..3 {
        let ph = ((time * 1.4 - i as f32 * 0.2).rem_euclid(1.0) * TAU).cos() * 0.5 + 0.5;
        let x = c.x + (i as f32 - 1.0) * d * 1.7;
        p.circle_filled(pos2(x, c.y), d / 2.0 * (0.75 + 0.35 * ph), with_alpha(col, 0.3 + 0.7 * ph));
    }
}

impl Widget for Loader<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let s = self.size;
        let col = self.color.unwrap_or(t.foreground);
        match self.kind {
            LoaderKind::TextShimmer => {
                let text = format!("{}…", self.text);
                return ui.add(ShimmerText::new(&text).size(s * 0.68).shine(col));
            }
            LoaderKind::TextBlink => {
                let g = ui.painter().layout_no_wrap(format!("{}…", self.text), FontId::proportional(s * 0.68), col);
                let (rect, resp) = ui.allocate_exact_size(g.size(), Sense::hover());
                let time = glowkit_core::time(ui);
                let n = self.text.chars().count() + 1;
                common::paint_galley_fx(ui.painter(), rect.min, &g, |i| {
                    let ph = ((time * 0.9 - i as f32 / n as f32 * 0.6).rem_euclid(1.0) * TAU).cos() * 0.5 + 0.5;
                    common::Fx { alpha: 0.3 + 0.7 * ph, ..Default::default() }
                });
                glowkit_core::animate(ui);
                return resp;
            }
            _ => {}
        }
        let w = match self.kind {
            LoaderKind::Wave => s * 1.3,
            LoaderKind::Bars => s * 1.6,
            LoaderKind::Dots | LoaderKind::Typing => s * 1.6,
            _ => s,
        };
        let (rect, resp) = ui.allocate_exact_size(vec2(w, s), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let p = ui.painter();
        let c = rect.center();
        let time = glowkit_core::time(ui);
        match self.kind {
            LoaderKind::Dots => dots(p, c, s, col, time),
            LoaderKind::Typing => {
                let d = s * 0.26;
                for i in 0..3 {
                    let ph = ((time * 1.25 - i as f32 * 0.16).rem_euclid(1.0) * TAU).sin().max(0.0);
                    let x = c.x + (i as f32 - 1.0) * d * 1.75;
                    p.circle_filled(
                        pos2(x, c.y + d * 0.5 - ph * ph * d * 1.1),
                        d / 2.0,
                        with_alpha(col, 0.5 + 0.5 * ph),
                    );
                }
            }
            LoaderKind::Pulse => {
                let ph = (time * 0.9).fract();
                p.circle_stroke(c, s * 0.15 + ph * s * 0.35, Stroke::new(1.5, with_alpha(col, (1.0 - ph) * 0.6)));
                let breathe = (time * TAU * 0.9).sin() * 0.5 + 0.5;
                p.circle_filled(c, s * (0.17 + 0.05 * breathe), col);
            }
            LoaderKind::Wave => {
                let bw = s * 0.12;
                for i in 0..5 {
                    let ph = ((time * 1.3 - i as f32 * 0.12).rem_euclid(1.0) * TAU).sin() * 0.5 + 0.5;
                    let h = s * (0.25 + 0.65 * ph);
                    let x = rect.min.x + bw / 2.0 + i as f32 * (rect.width() - bw) / 4.0;
                    p.rect_filled(
                        Rect::from_center_size(pos2(x, c.y), vec2(bw, h)),
                        CornerRadius::same((bw / 2.0) as u8),
                        with_alpha(col, 0.5 + 0.5 * ph),
                    );
                }
            }
            LoaderKind::Circular => {
                common::spinner(p, c, s * 0.4, (s * 0.1).max(1.8), col, time);
            }
            LoaderKind::Bars => {
                let n = 4;
                let bw = rect.width() / (n as f32 * 1.6);
                let head = (time * 1.2).fract() * (n as f32 + 1.0) - 0.5;
                for i in 0..n {
                    let k = (1.0 - (i as f32 - head).abs()).clamp(0.0, 1.0);
                    let x = rect.min.x + bw / 2.0 + i as f32 * bw * 1.6 + bw * 0.3;
                    p.rect_filled(
                        Rect::from_center_size(pos2(x, c.y), vec2(bw, s * 0.5)),
                        CornerRadius::same(2),
                        with_alpha(col, 0.2 + 0.8 * k),
                    );
                }
            }
            LoaderKind::TextShimmer | LoaderKind::TextBlink => {}
        }
        glowkit_core::animate(ui);
        resp
    }
}
