//! Text effects: shiny, gradient, blur-in, typewriter and count-up.

use crate::anim::{use_clock, use_replay, use_tween};
use dioxus::prelude::*;
use glowkit_tokens::text::{format_number, type_state, TypeTiming};
use glowkit_tokens::{Easing, Trigger};

fn opt(c: &Option<String>, fallback: &str) -> String {
    c.clone().unwrap_or_else(|| fallback.to_string())
}

/// Join CSS colors into a cyclic gradient that tiles seamlessly (last stop = first stop).
pub(crate) fn cyclic_stops(colors: &[String]) -> String {
    let mut v: Vec<String> = colors.to_vec();
    if let Some(first) = colors.first() {
        v.push(first.clone());
    }
    v.join(", ")
}

pub(crate) fn theme_gradient() -> Vec<String> {
    vec!["var(--gk-gradient-1)".into(), "var(--gk-gradient-2)".into(), "var(--gk-gradient-3)".into()]
}

// ---------------------------------------------------------------------------------------------

/// Text with a band of light sweeping across it, like light catching metal.
///
/// Mirrors `glowkit_text::ShinyText`: `size`, `color` (dim base, default muted foreground),
/// `shine_color` (default foreground), `speed` (seconds per sweep), `band` (fraction of the text
/// width), `angle` (tilt in degrees) and `disabled`.
#[component]
pub fn ShinyText(
    #[props(into)] text: String,
    #[props(default = 28.0)] size: f32,
    #[props(into)] color: Option<String>,
    #[props(into)] shine_color: Option<String>,
    #[props(default = 3.0)] speed: f32,
    #[props(default = 0.18)] band: f32,
    #[props(default = 20.0)] angle: f32,
    #[props(default)] disabled: bool,
    #[props(default = 400)] weight: u16,
) -> Element {
    let base = opt(&color, "var(--gk-muted-foreground)");
    let shine = opt(&shine_color, "var(--gk-foreground)");
    let band = band.clamp(0.02, 1.0);
    // The gradient image is `s` text-widths wide; its middle (the highlight) travels from
    // `-band` to `1 + band` across the text, exactly like the egui sweep.
    let s = 2.0 + 2.0 * band;
    let half = band / s * 100.0;
    let style = format!(
        "font-size:{size}px; font-weight:{weight}; --gk-shiny-base:{base}; --gk-shiny-shine:{shine}; \
         --gk-shiny-speed:{speed}s; background-size:{w}% 100%; \
         background-image:linear-gradient({a}deg, var(--gk-shiny-base) 0%, var(--gk-shiny-base) {s0}%, \
         color-mix(in oklab, var(--gk-shiny-base), var(--gk-shiny-shine) 50%) {s1}%, var(--gk-shiny-shine) 50%, \
         color-mix(in oklab, var(--gk-shiny-base), var(--gk-shiny-shine) 50%) {s2}%, var(--gk-shiny-base) {s3}%, \
         var(--gk-shiny-base) 100%);",
        speed = speed.max(0.1),
        w = s * 100.0,
        a = 90.0 + angle,
        s0 = 50.0 - half,
        s1 = 50.0 - half * 0.5,
        s2 = 50.0 + half * 0.5,
        s3 = 50.0 + half,
    );
    let class = if disabled { "gk-shiny gk-shiny-off" } else { "gk-shiny" };
    rsx! {
        span { class, style, "{text}" }
    }
}

// ---------------------------------------------------------------------------------------------

/// Text filled with a flowing multi-color gradient, optionally inside a matching animated
/// gradient pill border.
///
/// Mirrors `glowkit_text::GradientText`: `size`, `colors` (CSS colors, wraps around; defaults to
/// the theme gradient), `speed` (seconds per cycle, 0 = static), `angle`, `border`, `bold`.
#[component]
pub fn GradientText(
    #[props(into)] text: String,
    #[props(default = 32.0)] size: f32,
    #[props(default)] colors: Vec<String>,
    #[props(default = 6.0)] speed: f32,
    #[props(default = 0.0)] angle: f32,
    #[props(default)] border: bool,
    #[props(default)] bold: bool,
) -> Element {
    let colors = if colors.is_empty() { theme_gradient() } else { colors };
    let grad = format!("linear-gradient(in oklab {}deg, {})", 90.0 + angle, cyclic_stops(&colors));
    let anim = if speed > 0.0 { format!("animation-duration:{speed}s;") } else { "animation:none;".into() };
    let weight = if bold { 700 } else { 500 };
    let text_style = format!("font-size:{size}px; font-weight:{weight}; --gk-grad:{grad}; {anim}");
    if border {
        let pad_y = size * 0.25;
        let pad_x = size * 0.6;
        rsx! {
            span {
                class: "gk-gradient-pill",
                style: "--gk-grad:{grad}; padding:{pad_y}px {pad_x}px; {anim}",
                span { class: "gk-gradient-text", style: text_style, "{text}" }
            }
        }
    } else {
        rsx! {
            span { class: "gk-gradient-text", style: text_style, "{text}" }
        }
    }
}

// ---------------------------------------------------------------------------------------------

/// How [`BlurText`] splits its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SplitBy {
    /// Each word animates as one piece (default).
    #[default]
    Words,
    /// Each letter animates on its own.
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
    fn offset(self, d: f32) -> (f32, f32) {
        match self {
            Direction::Top => (0.0, -d),
            Direction::Bottom => (0.0, d),
            Direction::Left => (-d, 0.0),
            Direction::Right => (d, 0.0),
            Direction::None => (0.0, 0.0),
        }
    }
}

/// Words (or letters) fade in from a blur, one after another.
///
/// Mirrors `glowkit_text::BlurText`: `size`, `color`, `split_by`, `direction`, `delay` (seconds
/// between pieces), `duration`, `blur` (starting radius), `distance`, `easing` and `trigger`.
#[component]
pub fn BlurText(
    #[props(into)] text: String,
    #[props(default = 32.0)] size: f32,
    #[props(into)] color: Option<String>,
    #[props(default)] split_by: SplitBy,
    #[props(default)] direction: Direction,
    #[props(default = 0.12)] delay: f32,
    #[props(default = 0.7)] duration: f32,
    #[props(default = 8.0)] blur: f32,
    #[props(default = 18.0)] distance: f32,
    #[props(default = Easing::OutCubic)] easing: Easing,
    #[props(default)] trigger: Trigger,
    #[props(default = 600)] weight: u16,
) -> Element {
    let pieces: Vec<String> = match split_by {
        SplitBy::Words => text.split_whitespace().map(str::to_owned).collect(),
        SplitBy::Letters => text.chars().map(|c| if c == ' ' { "\u{a0}".into() } else { c.to_string() }).collect(),
    };
    let n = pieces.len().max(1);
    let replay = use_replay(trigger, (n - 1) as f32 * delay + duration);
    let (dx, dy) = direction.offset(distance);
    let ease = easing.css();
    let color = opt(&color, "var(--gk-foreground)");
    let words = split_by == SplitBy::Words;
    let key = replay.key();
    rsx! {
        span {
            class: "gk-blur-text",
            style: "font-size:{size}px; font-weight:{weight}; color:{color}; --gk-blur:{blur}px; --gk-dx:{dx}px; --gk-dy:{dy}px; --gk-dur:{duration}s; --gk-ease:{ease};",
            onmouseenter: move |_| replay.hover(),
            onclick: move |_| replay.click(),
            span { key: "{key}", class: "gk-blur-run",
                for (i, p) in pieces.iter().enumerate() {
                    span { class: "gk-blur-piece", style: "animation-delay:{i as f32 * delay}s", "{p}" }
                    if words && i + 1 < pieces.len() { " " }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------

/// Shape of the typing cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CursorStyle {
    #[default]
    Bar,
    Block,
    Underscore,
    None,
}

/// Types out one or more strings with a blinking cursor, deleting and retyping in a loop.
///
/// Mirrors `glowkit_text::TypewriterText`: `prefix`, `size`, `color`, `prefix_color`, `colors`
/// (one per string), `cursor_color`, `cursor`, `bold`, `monospace`, `typing_speed`,
/// `deleting_speed` (chars/s), `hold`, `gap`, `looping`, `blink`.
#[component]
pub fn TypewriterText(
    strings: Vec<String>,
    #[props(into, default)] prefix: String,
    #[props(default = 32.0)] size: f32,
    #[props(into)] color: Option<String>,
    #[props(into)] prefix_color: Option<String>,
    #[props(default)] colors: Vec<String>,
    #[props(into)] cursor_color: Option<String>,
    #[props(default)] cursor: CursorStyle,
    #[props(default = true)] bold: bool,
    #[props(default)] monospace: bool,
    #[props(default = 14.0)] typing_speed: f32,
    #[props(default = 30.0)] deleting_speed: f32,
    #[props(default = 1.6)] hold: f32,
    #[props(default = 0.35)] gap: f32,
    #[props(default = true)] looping: bool,
    #[props(default = 1.0)] blink: f32,
) -> Element {
    let clock = use_clock(true);
    let chars: Vec<Vec<char>> = strings.iter().map(|s| s.chars().collect()).collect();
    let lens: Vec<usize> = chars.iter().map(Vec::len).collect();
    let tm = TypeTiming { type_cps: typing_speed, delete_cps: deleting_speed, hold, gap, looping };
    let st = type_state(clock() as f32, &lens, tm);
    let shown: String = chars.get(st.index).map(|c| c[..st.chars.min(c.len())].iter().collect()).unwrap_or_default();
    let color = if colors.is_empty() {
        opt(&color, "var(--gk-foreground)")
    } else {
        colors[st.index % colors.len()].clone()
    };
    let prefix_color = opt(&prefix_color, "var(--gk-foreground)");
    let cursor_color = opt(&cursor_color, "var(--gk-accent)");
    let family = if monospace { "var(--gk-font-mono)" } else { "inherit" };
    let weight = if bold { 700 } else { 400 };
    let blink_anim = if st.busy || blink <= 0.0 { "none".to_string() } else { format!("gk-blink {blink}s steps(1, end) infinite") };
    let cursor_class = match cursor {
        CursorStyle::Bar => "gk-tw-cursor gk-tw-bar",
        CursorStyle::Block => "gk-tw-cursor gk-tw-block",
        CursorStyle::Underscore => "gk-tw-cursor gk-tw-under",
        CursorStyle::None => "",
    };
    rsx! {
        span { class: "gk-typewriter", style: "font-size:{size}px; font-family:{family};",
            if !prefix.is_empty() {
                span { style: "color:{prefix_color}; font-weight:{weight};", "{prefix}" }
            }
            span { style: "color:{color}; font-weight:{weight};", "{shown}" }
            if cursor != CursorStyle::None {
                span { class: cursor_class, style: "--gk-cursor:{cursor_color}; animation:{blink_anim};" }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------

/// A number that counts up (or down) to its target with easing, thousands separators, decimals
/// and an optional prefix/suffix. Changing `to` later animates from the current value.
///
/// Mirrors `glowkit_text::CountUp`: `to`, `from`, `duration`, `easing`, `decimals`, `separator`,
/// `decimal_point`, `prefix`, `suffix`, `size`, `color`, `affix_color`, `bold`, plus `delay`.
#[component]
pub fn CountUp(
    to: f64,
    #[props(default = 0.0)] from: f64,
    #[props(default = 2.2)] duration: f32,
    #[props(default = Easing::OutExpo)] easing: Easing,
    #[props(default)] decimals: usize,
    #[props(into, default = ",".to_string())] separator: String,
    #[props(into, default = ".".to_string())] decimal_point: String,
    #[props(into, default)] prefix: String,
    #[props(into, default)] suffix: String,
    #[props(default = 48.0)] size: f32,
    #[props(into)] color: Option<String>,
    #[props(into)] affix_color: Option<String>,
    #[props(default = true)] bold: bool,
    #[props(default = 0.0)] delay: f32,
) -> Element {
    let v = use_tween(from, to, duration, easing, delay);
    let text = format_number(v(), decimals.min(10), &separator, &decimal_point);
    let color = opt(&color, "var(--gk-foreground)");
    let affix = opt(&affix_color, &color);
    let weight = if bold { 700 } else { 400 };
    rsx! {
        span { class: "gk-count", style: "font-size:{size}px; font-weight:{weight}; color:{color};",
            if !prefix.is_empty() { span { style: "color:{affix}", "{prefix}" } }
            span { class: "gk-count-num", "{text}" }
            if !suffix.is_empty() { span { style: "color:{affix}", "{suffix}" } }
        }
    }
}
