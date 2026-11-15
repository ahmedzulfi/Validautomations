//! Animated text: [`ShinyText`], [`GradientText`], [`CountUp`] and [`StreamingText`].
//!
//! Per-glyph color effects are done with `StyledText::with_highlights`, one highlight per
//! character, recomputed every animation frame inside `with_animation`.

use crate::anim::{secs, Easing};
use crate::color::{gradient_cyclic, lerp_srgb, with_alpha};
use crate::theme::ActiveTheme;
use gpui::prelude::*;
use gpui::{
    div, px, Animation, AnimationExt, App, ElementId, FontWeight, HighlightStyle, Hsla, Pixels, SharedString,
    StyledText, Window,
};
use std::ops::Range;

fn char_ranges(text: &str) -> Vec<Range<usize>> {
    text.char_indices().map(|(i, c)| i..i + c.len_utf8()).collect()
}

fn colored(text: &SharedString, ranges: &[Range<usize>], color: impl Fn(usize, usize) -> Hsla) -> StyledText {
    let n = ranges.len();
    StyledText::new(text.clone()).with_highlights(
        ranges
            .iter()
            .enumerate()
            .map(|(i, r)| (r.clone(), HighlightStyle { color: Some(color(i, n)), ..Default::default() })),
    )
}

/// Child id helper: a stable sub-id of a component id.
pub(crate) fn sub_id(id: &ElementId, name: &'static str) -> ElementId {
    ElementId::NamedChild(Box::new(id.clone()), name.into())
}

// ---------------------------------------------------------------------------------------------

/// Text with a band of light sweeping across it, like light catching metal.
///
/// ```no_run
/// # use glowkit_gpui::ShinyText;
/// ShinyText::new("shiny", "Shimmering text").size(32.0).speed(2.5);
/// ```
#[derive(IntoElement)]
pub struct ShinyText {
    id: ElementId,
    text: SharedString,
    size: f32,
    weight: FontWeight,
    color: Option<Hsla>,
    shine: Option<Hsla>,
    speed: f32,
    band: f32,
    disabled: bool,
}

impl ShinyText {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            size: 28.0,
            weight: FontWeight::SEMIBOLD,
            color: None,
            shine: None,
            speed: 3.0,
            band: 0.18,
            disabled: false,
        }
    }
    /// Font size in pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    pub fn weight(mut self, w: FontWeight) -> Self {
        self.weight = w;
        self
    }
    /// Base (dim) color. Defaults to the theme's muted foreground.
    pub fn color(mut self, c: Hsla) -> Self {
        self.color = Some(c);
        self
    }
    /// Color of the moving highlight. Defaults to the theme's foreground.
    pub fn shine_color(mut self, c: Hsla) -> Self {
        self.shine = Some(c);
        self
    }
    /// Seconds per sweep.
    pub fn speed(mut self, seconds: f32) -> Self {
        self.speed = seconds.max(0.1);
        self
    }
    /// Width of the highlight band as a fraction of the text width.
    pub fn band(mut self, fraction: f32) -> Self {
        self.band = fraction.clamp(0.02, 1.0);
        self
    }
    /// Stop the animation.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for ShinyText {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.theme();
        let base = self.color.unwrap_or(t.muted_foreground);
        let shine = self.shine.unwrap_or(t.foreground);
        let el = div().text_size(px(self.size)).font_weight(self.weight).line_height(px(self.size * 1.25));
        if self.disabled {
            return el.text_color(base).child(self.text).into_any_element();
        }
        let (text, band) = (self.text, self.band);
        let ranges = char_ranges(&text);
        el.with_animation(self.id, Animation::new(secs(self.speed)).repeat(), move |el, phase| {
            // the band travels from fully left of the text to fully right of it
            let center = -band + phase * (1.0 + band * 2.0);
            el.child(colored(&text, &ranges, |i, n| {
                let u = (i as f32 + 0.5) / n.max(1) as f32;
                let k = (1.0 - ((u - center) / band).abs()).clamp(0.0, 1.0);
                lerp_srgb(base, shine, k * k * (3.0 - 2.0 * k))
            }))
        })
        .into_any_element()
    }
}

// ---------------------------------------------------------------------------------------------

/// Text filled with a slowly flowing multi-stop gradient (defaults to the theme's brand
/// gradient).
#[derive(IntoElement)]
pub struct GradientText {
    id: ElementId,
    text: SharedString,
    size: f32,
    weight: FontWeight,
    colors: Option<Vec<Hsla>>,
    speed: f32,
    spread: f32,
}

impl GradientText {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            size: 36.0,
            weight: FontWeight::BOLD,
            colors: None,
            speed: 6.0,
            spread: 0.5,
        }
    }
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    pub fn weight(mut self, w: FontWeight) -> Self {
        self.weight = w;
        self
    }
    /// Gradient stops (cycled). Defaults to the theme gradient.
    pub fn colors(mut self, colors: Vec<Hsla>) -> Self {
        self.colors = Some(colors);
        self
    }
    /// Seconds for the gradient to flow one full cycle.
    pub fn speed(mut self, seconds: f32) -> Self {
        self.speed = seconds.max(0.2);
        self
    }
    /// How much of the gradient cycle spans the text (0..1).
    pub fn spread(mut self, spread: f32) -> Self {
        self.spread = spread.clamp(0.05, 1.0);
        self
    }
}

impl RenderOnce for GradientText {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let stops = self.colors.unwrap_or_else(|| cx.theme().gradient.to_vec());
        let (text, spread) = (self.text, self.spread);
        let ranges = char_ranges(&text);
        div()
            .text_size(px(self.size))
            .font_weight(self.weight)
            .line_height(px(self.size * 1.2))
            .with_animation(self.id, Animation::new(secs(self.speed)).repeat(), move |el, phase| {
                el.child(colored(&text, &ranges, |i, n| {
                    let u = i as f32 / n.max(1) as f32;
                    gradient_cyclic(&stops, u * spread - phase)
                }))
            })
    }
}

// ---------------------------------------------------------------------------------------------

/// Format with thousands separators and fixed decimals: `1234567.8` -> `"1,234,567.80"`.
pub fn format_thousands(v: f64, decimals: usize) -> String {
    let s = format!("{:.*}", decimals, v.abs());
    let (int, frac) = s.split_once('.').map(|(a, b)| (a, Some(b))).unwrap_or((&s, None));
    let mut out = String::new();
    for (i, ch) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if let Some(f) = frac {
        out.push('.');
        out.push_str(f);
    }
    if v < 0.0 && v.abs() >= 0.5 * 10f64.powi(-(decimals as i32)) {
        out.insert(0, '-');
    }
    out
}

/// A number that counts up to its value when it first appears (and again whenever the value
/// changes), with an ease-out curve.
#[derive(IntoElement)]
pub struct CountUp {
    id: SharedString,
    value: f64,
    from: f64,
    duration: f32,
    decimals: usize,
    prefix: SharedString,
    suffix: SharedString,
    size: f32,
    weight: FontWeight,
    color: Option<Hsla>,
    easing: Easing,
}

impl CountUp {
    pub fn new(id: impl Into<SharedString>, value: f64) -> Self {
        Self {
            id: id.into(),
            value,
            from: 0.0,
            duration: 1.6,
            decimals: 0,
            prefix: "".into(),
            suffix: "".into(),
            size: 40.0,
            weight: FontWeight::BOLD,
            color: None,
            easing: Easing::OutExpo,
        }
    }
    pub fn from(mut self, v: f64) -> Self {
        self.from = v;
        self
    }
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s;
        self
    }
    pub fn decimals(mut self, d: usize) -> Self {
        self.decimals = d;
        self
    }
    pub fn prefix(mut self, p: impl Into<SharedString>) -> Self {
        self.prefix = p.into();
        self
    }
    pub fn suffix(mut self, s: impl Into<SharedString>) -> Self {
        self.suffix = s.into();
        self
    }
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    pub fn weight(mut self, w: FontWeight) -> Self {
        self.weight = w;
        self
    }
    pub fn color(mut self, c: Hsla) -> Self {
        self.color = Some(c);
        self
    }
    pub fn easing(mut self, e: Easing) -> Self {
        self.easing = e;
        self
    }
}

impl RenderOnce for CountUp {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = self.color.unwrap_or(cx.theme().foreground);
        // keying the animation on the value replays it whenever the value changes
        let id = ElementId::NamedInteger(self.id, self.value.to_bits());
        let (from, to, dec, pre, suf) = (self.from, self.value, self.decimals, self.prefix, self.suffix);
        div()
            .text_size(px(self.size))
            .font_weight(self.weight)
            .text_color(color)
            .line_height(px(self.size * 1.15))
            .whitespace_nowrap()
            .with_animation(
                id,
                Animation::new(secs(self.duration)).with_easing(self.easing.curve()),
                move |el, k| {
                    let v = from + (to - from) * k as f64;
                    el.child(format!("{pre}{}{suf}", format_thousands(v, dec)))
                },
            )
    }
}

// ---------------------------------------------------------------------------------------------

/// Text that types itself out at `cps` characters per second with a blinking caret, or (with
/// [`StreamingText::live`]) shows text that is still arriving from a model with a pulsing caret.
#[derive(IntoElement)]
pub struct StreamingText {
    id: ElementId,
    text: SharedString,
    cps: f32,
    live: bool,
    caret: bool,
    size: Pixels,
    color: Option<Hsla>,
}

impl StreamingText {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            cps: 45.0,
            live: false,
            caret: true,
            size: px(14.0),
            color: None,
        }
    }
    /// Characters revealed per second (typewriter mode).
    pub fn cps(mut self, cps: f32) -> Self {
        self.cps = cps.max(1.0);
        self
    }
    /// The text is being streamed by the caller: show it all, with a live caret.
    pub fn live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }
    /// Show the caret while typing / streaming (default on).
    pub fn caret(mut self, on: bool) -> Self {
        self.caret = on;
        self
    }
    pub fn text_size(mut self, s: Pixels) -> Self {
        self.size = s;
        self
    }
    pub fn color(mut self, c: Hsla) -> Self {
        self.color = Some(c);
        self
    }
}

const CARET: &str = "\u{258D}"; // ▍

impl RenderOnce for StreamingText {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.theme();
        let fg = self.color.unwrap_or(t.foreground);
        let accent = t.accent;
        let el = div().text_size(self.size).text_color(fg).line_height(self.size * 1.6);
        let text = self.text;
        let caret = self.caret;
        if self.live {
            if !caret {
                return el.child(text).into_any_element();
            }
            let full: SharedString = format!("{text}{CARET}").into();
            let caret_range = text.len()..full.len();
            return el
                .with_animation(
                    self.id,
                    Animation::new(secs(0.9)).repeat().with_easing(gpui::pulsating_between(0.25, 1.0)),
                    move |el, k| {
                        el.child(StyledText::new(full.clone()).with_highlights([(
                            caret_range.clone(),
                            HighlightStyle { color: Some(with_alpha(accent, k)), ..Default::default() },
                        )]))
                    },
                )
                .into_any_element();
        }
        let ranges = char_ranges(&text);
        let n = ranges.len();
        let dur = n as f32 / self.cps;
        el.with_animation(self.id, Animation::new(secs(dur)), move |el, k| {
            let shown = ((k * n as f32).round() as usize).min(n);
            let end = if shown == 0 { 0 } else { ranges[shown - 1].end };
            if shown >= n || !caret {
                // done: drop the caret, but keep the not-yet-revealed tail laid out invisibly so
                // the block never reflows while typing
                return el.child(StyledText::new(text.clone()).with_highlights([(
                    end..text.len(),
                    HighlightStyle { color: Some(with_alpha(fg, 0.0)), ..Default::default() },
                )]));
            }
            let full: SharedString = format!("{}{CARET}{}", &text[..end], &text[end..]).into();
            let c = end..end + CARET.len();
            el.child(StyledText::new(full.clone()).with_highlights([
                (c.clone(), HighlightStyle { color: Some(accent), ..Default::default() }),
                (c.end..full.len(), HighlightStyle { color: Some(with_alpha(fg, 0.0)), ..Default::default() }),
            ]))
        })
        .into_any_element()
    }
}
