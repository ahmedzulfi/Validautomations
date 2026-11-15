//! Animated buttons: shimmer, rainbow and magnetic.

use crate::anim::{next_frame, now_ms, use_springs, SpringConfig};
use crate::components::text::cyclic_stops;
use crate::pointer::use_element_rect;
use dioxus::prelude::*;
use glowkit_tokens::color::palettes;

fn radius_css(radius: Option<f32>, fallback: &str) -> String {
    radius.map(|r| format!("{r}px")).unwrap_or_else(|| fallback.to_string())
}

fn min_w(min_width: Option<f32>) -> String {
    min_width.map(|w| format!("min-width:{w}px;")).unwrap_or_default()
}

/// A dark pill with a spark of light racing around its border and a soft inner glow that
/// rises on hover.
///
/// Mirrors `glowkit_effects::ShimmerButton`: `size` (font size; the button scales with it),
/// `background`, `shimmer_color`, `text_color`, `radius` (default: full pill), `duration`
/// (seconds per lap), `min_width`.
#[component]
pub fn ShimmerButton(
    #[props(into)] label: String,
    #[props(default = 15.0)] size: f32,
    #[props(into)] background: Option<String>,
    #[props(into)] shimmer_color: Option<String>,
    #[props(into)] text_color: Option<String>,
    radius: Option<f32>,
    #[props(default = 3.0)] duration: f32,
    min_width: Option<f32>,
    onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let bg = background.unwrap_or_else(|| "var(--gk-shimmer-bg)".into());
    let spark = shimmer_color.unwrap_or_else(|| "#ffffff".into());
    let text = text_color.unwrap_or_else(|| "#ffffff".into());
    let r = radius_css(radius, "9999px");
    let style = format!(
        "--gk-size:{size}px; --gk-bg:{bg}; --gk-spark:{spark}; color:{text}; --gk-r:{r}; --gk-dur:{d}s; {mw}",
        d = duration.max(0.2),
        mw = min_w(min_width)
    );
    rsx! {
        button {
            class: "gk-btn gk-shimmer-btn",
            style,
            onclick: move |e| if let Some(h) = onclick { h.call(e) },
            span { class: "gk-shimmer-glow" }
            span { class: "gk-shimmer-spark" }
            span { class: "gk-btn-label", "{label}" {children} }
        }
    }
}

/// A high-contrast button with a flowing rainbow border and a blurred rainbow glow underneath.
///
/// Mirrors `glowkit_effects::RainbowButton`: `size`, `colors` (wraps around; default rainbow),
/// `background` (default primary), `text_color`, `radius`, `speed` (seconds per cycle),
/// `min_width`.
#[component]
pub fn RainbowButton(
    #[props(into)] label: String,
    #[props(default = 15.0)] size: f32,
    #[props(default)] colors: Vec<String>,
    #[props(into)] background: Option<String>,
    #[props(into)] text_color: Option<String>,
    radius: Option<f32>,
    #[props(default = 2.0)] speed: f32,
    min_width: Option<f32>,
    onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let colors = if colors.is_empty() { palettes::RAINBOW.iter().map(|c| c.to_css()).collect() } else { colors };
    let grad = format!("linear-gradient(in oklab 90deg, {})", cyclic_stops(&colors));
    let bg = background.unwrap_or_else(|| "var(--gk-primary)".into());
    let text = text_color.unwrap_or_else(|| "var(--gk-primary-foreground)".into());
    let r = radius_css(radius, "calc(var(--gk-radius) * 1.2)");
    let style = format!(
        "--gk-size:{size}px; --gk-bg:{bg}; color:{text}; --gk-r:{r}; --gk-grad:{grad}; --gk-dur:{d}s; {mw}",
        d = speed.max(0.1),
        mw = min_w(min_width)
    );
    rsx! {
        span { class: "gk-rainbow-wrap", style,
            span { class: "gk-rainbow-glow" }
            button {
                class: "gk-btn gk-rainbow-btn",
                onclick: move |e| if let Some(h) = onclick { h.call(e) },
                span { class: "gk-btn-label", "{label}" {children} }
            }
        }
    }
}

/// A button that leans toward the pointer as it approaches and springs back when it leaves.
///
/// Mirrors `glowkit_effects::MagneticButton`: `size`, `strength` (fraction of the pointer offset
/// followed), `range` (how far outside the button the pull starts), `background`, `text_color`,
/// `radius`, `ambient` (drift on its own when the pointer is away), `min_width`.
#[component]
pub fn MagneticButton(
    #[props(into)] label: String,
    #[props(default = 15.0)] size: f32,
    #[props(default = 0.35)] strength: f32,
    #[props(default = 60.0)] range: f32,
    #[props(into)] background: Option<String>,
    #[props(into)] text_color: Option<String>,
    radius: Option<f32>,
    #[props(default)] ambient: bool,
    min_width: Option<f32>,
    onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let zone = use_element_rect();
    let off = use_springs(vec![0.0, 0.0], SpringConfig::new(170.0, 13.0));
    let mut inside = use_signal(|| false);
    let mut amb = use_signal(|| ambient);
    if *amb.peek() != ambient {
        amb.set(ambient);
    }
    let mut strength_sig = use_signal(|| strength);
    if *strength_sig.peek() != strength {
        strength_sig.set(strength);
    }
    use_future(move || async move {
        let t0 = now_ms();
        loop {
            next_frame().await;
            if !*amb.peek() || *inside.peek() {
                crate::anim::sleep_ms(120.0).await;
                continue;
            }
            let b = zone.peek();
            let tm = ((now_ms() - t0) / 1000.0) as f32;
            let (w, h) = ((b.width as f32 - 2.0 * range).max(1.0), (b.height as f32 - 2.0 * range).max(1.0));
            let s = *strength_sig.peek();
            off.set(0, ((tm * 1.3).sin() * 0.9 + (tm * 0.7).cos() * 0.4) * w * 0.5 * s);
            off.set(1, (tm * 1.9).cos() * 0.7 * h * 0.8 * s);
        }
    });
    let (x, y) = (off.get(0), off.get(1));
    let bg = background.unwrap_or_else(|| "var(--gk-primary)".into());
    let text = text_color.unwrap_or_else(|| "var(--gk-primary-foreground)".into());
    let r = radius_css(radius, "9999px");
    rsx! {
        span {
            class: "gk-magnetic-zone",
            style: "padding:{range}px; margin:-{range}px;",
            onmounted: move |e| zone.mounted(e),
            onmouseenter: move |_| {
                inside.set(true);
                zone.refresh();
            },
            onmousemove: move |e| {
                zone.refresh();
                let b = zone.peek();
                if b.width < 1.0 {
                    return;
                }
                let (px, py) = zone.local(&e);
                let max = (range as f64 + b.width / 2.0) as f32;
                let dx = (((px - b.width / 2.0) as f32) * strength).clamp(-max, max);
                let dy = (((py - b.height / 2.0) as f32) * strength).clamp(-max, max);
                off.set(0, dx);
                off.set(1, dy);
            },
            onmouseleave: move |_| {
                inside.set(false);
                off.set(0, 0.0);
                off.set(1, 0.0);
            },
            button {
                class: "gk-btn gk-magnetic-btn",
                style: "--gk-size:{size}px; --gk-bg:{bg}; color:{text}; --gk-r:{r}; {min_w(min_width)} transform: translate({x}px, {y}px); box-shadow: {x * 0.25}px {6.0 + y * 0.25}px 22px var(--gk-shadow-color);",
                onclick: move |e| if let Some(h) = onclick { h.call(e) },
                span { class: "gk-btn-label", style: "transform: translate({x * 0.35}px, {y * 0.35}px)", "{label}" {children} }
            }
        }
    }
}
