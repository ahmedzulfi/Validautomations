//! Effect cards: border beam, spotlight and 3D tilt.
//!
//! Every card shares glowkit's card base options: `width` (default: fill), `min_height`,
//! `padding` (24), `radius` (1.4x the theme radius) and `background` (theme card).

use crate::anim::{next_frame, now_ms, sleep_ms, use_springs, SpringConfig};
use crate::pointer::use_element_rect;
use dioxus::prelude::*;

fn base_style(
    width: Option<f32>,
    min_height: f32,
    padding: f32,
    radius: Option<f32>,
    background: &Option<String>,
    default_bg: &str,
) -> String {
    let w = width.map(|w| format!("width:{w}px;")).unwrap_or_default();
    let r = radius.map(|r| format!("{r}px")).unwrap_or_else(|| "var(--gk-radius-lg)".into());
    let bg = background.clone().unwrap_or_else(|| default_bg.to_string());
    format!("{w} min-height:{min_height}px; --gk-pad:{padding}px; --gk-r:{r}; --gk-card-bg:{bg};")
}

/// A comet of light that travels around the border of the card, fading from one color into
/// another along its tail.
///
/// Mirrors `glowkit_effects::BorderBeam`: card base options plus `from` (tail, default theme
/// gradient[2]), `to` (head, default gradient[0]), `duration` (seconds per lap), `length`
/// (fraction of the perimeter), `thickness`, `delay`, `reverse`.
#[component]
pub fn BorderBeam(
    width: Option<f32>,
    #[props(default)] min_height: f32,
    #[props(default = 24.0)] padding: f32,
    radius: Option<f32>,
    #[props(into)] background: Option<String>,
    #[props(into)] from: Option<String>,
    #[props(into)] to: Option<String>,
    #[props(default = 6.0)] duration: f32,
    #[props(default = 0.22)] length: f32,
    #[props(default = 1.6)] thickness: f32,
    #[props(default)] delay: f32,
    #[props(default)] reverse: bool,
    #[props(into, default)] class: String,
    children: Element,
) -> Element {
    let rect = use_element_rect();
    let b = rect.bounds();
    let perimeter = if b.width > 0.0 { 2.0 * (b.width + b.height) } else { 1200.0 };
    let len = (length.clamp(0.02, 0.9) as f64 * perimeter).round();
    let from = from.unwrap_or_else(|| "var(--gk-gradient-3)".into());
    let to = to.unwrap_or_else(|| "var(--gk-gradient-1)".into());
    let dir = if reverse { "reverse" } else { "normal" };
    let base = base_style(width, min_height, padding, radius, &background, "var(--gk-card)");
    rsx! {
        div {
            class: "gk-card gk-beam-card {class}",
            style: "{base} --gk-beam-from:{from}; --gk-beam-to:{to}; --gk-beam-len:{len}px; --gk-beam-w:{thickness}px; --gk-dur:{duration}s; --gk-delay:-{delay}s; --gk-dir:{dir};",
            onmounted: move |e| rect.mounted(e),
            onmouseenter: move |_| rect.refresh(),
            div { class: "gk-beam-glow-track", div { class: "gk-beam-glow" } }
            div { class: "gk-beam-track", div { class: "gk-beam-head" } }
            div { class: "gk-card-content", {children} }
        }
    }
}

/// A card lit by a soft spotlight that follows the pointer (optionally revealing a dot grid).
///
/// Mirrors `glowkit_effects::SpotlightCard`: card base options plus `color` (default soft white
/// in dark themes, accent in light), `radius_light`, `dots`, `ambient`.
#[component]
pub fn SpotlightCard(
    width: Option<f32>,
    #[props(default)] min_height: f32,
    #[props(default = 24.0)] padding: f32,
    radius: Option<f32>,
    #[props(into)] background: Option<String>,
    #[props(into)] color: Option<String>,
    #[props(default = 200.0)] radius_light: f32,
    #[props(default)] dots: bool,
    #[props(default)] ambient: bool,
    #[props(into, default)] class: String,
    children: Element,
) -> Element {
    let rect = use_element_rect();
    let mut pos = use_signal(|| (0.0f64, 0.0f64));
    let mut hovered = use_signal(|| false);
    let mut amb = use_signal(|| ambient);
    if *amb.peek() != ambient {
        amb.set(ambient);
    }
    use_future(move || async move {
        let t0 = now_ms();
        loop {
            if !*amb.peek() || *hovered.peek() {
                sleep_ms(150.0).await;
                continue;
            }
            next_frame().await;
            let b = rect.peek();
            let t = (now_ms() - t0) / 1000.0;
            pos.set((b.width * (0.5 + 0.38 * (t * 0.5).sin()), b.height * (0.5 + 0.35 * (t * 0.83 + 0.6).cos())));
        }
    });
    let (x, y) = pos();
    let lit = hovered() || ambient;
    let color = color.unwrap_or_else(|| "var(--gk-spotlight)".into());
    let base = base_style(width, min_height, padding, radius, &background, "var(--gk-spotlight-bg)");
    rsx! {
        div {
            class: if lit { "gk-card gk-spot-card gk-lit {class}" } else { "gk-card gk-spot-card {class}" },
            style: "{base} --gk-mx:{x}px; --gk-my:{y}px; --gk-spot:{color}; --gk-spot-r:{radius_light}px;",
            onmounted: move |e| rect.mounted(e),
            onmouseenter: move |_| {
                hovered.set(true);
                rect.refresh();
            },
            onmousemove: move |e| pos.set(rect.local(&e)),
            onmouseleave: move |_| hovered.set(false),
            div { class: "gk-spot-light" }
            if dots { div { class: "gk-spot-dots" } }
            div { class: "gk-card-content", {children} }
        }
    }
}

/// A card that tilts in 3D toward the pointer, with a moving glare and parallax content.
///
/// Mirrors `glowkit_effects::TiltCard`: card base options plus `max_tilt` (degrees), `scale`
/// (while hovered), `glare`, `parallax` (points the content shifts), `ambient`, and
/// `gradient` (two CSS colors for a diagonal fill).
#[component]
pub fn TiltCard(
    width: Option<f32>,
    #[props(default)] min_height: f32,
    #[props(default = 24.0)] padding: f32,
    radius: Option<f32>,
    #[props(into)] background: Option<String>,
    #[props(default = 14.0)] max_tilt: f32,
    #[props(default = 1.04)] scale: f32,
    #[props(default = true)] glare: bool,
    #[props(default = 10.0)] parallax: f32,
    #[props(default)] ambient: bool,
    gradient: Option<(String, String)>,
    #[props(into, default)] class: String,
    children: Element,
) -> Element {
    let rect = use_element_rect();
    // springs: tilt x, tilt y (normalized -1..1), hover amount, glare x, glare y (0..1)
    let s = use_springs(vec![0.0, 0.0, 0.0, 0.5, 0.5], SpringConfig::new(200.0, 18.0));
    let mut hovered = use_signal(|| false);
    let mut amb = use_signal(|| ambient);
    if *amb.peek() != ambient {
        amb.set(ambient);
    }
    use_future(move || async move {
        let t0 = now_ms();
        loop {
            if !*amb.peek() || *hovered.peek() {
                sleep_ms(150.0).await;
                continue;
            }
            next_frame().await;
            let t = ((now_ms() - t0) / 1000.0) as f32;
            let (nx, ny) = ((t * 0.9).sin() * 0.6, (t * 1.3 + 0.5).cos() * 0.5);
            s.set(0, nx);
            s.set(1, ny);
            s.set(3, 0.5 + nx * 0.5);
            s.set(4, 0.5 + ny * 0.5);
        }
    });
    let (tx, ty, hk, gx, gy) = (s.get(0), s.get(1), s.get(2), s.get(3), s.get(4));
    let rx = -ty * max_tilt;
    let ry = tx * max_tilt;
    let sc = 1.0 + (scale - 1.0) * hk;
    let bg_default = match &gradient {
        Some((a, b)) => format!("linear-gradient(135deg, {a}, {b})"),
        None => "var(--gk-card)".into(),
    };
    let base = base_style(width, min_height, padding, radius, &background, &bg_default);
    let glare_a = if glare { 0.10 + 0.22 * hk.max(if ambient { 0.6 } else { 0.0 }) } else { 0.0 };
    let shadow_x = -tx * 10.0;
    let shadow_y = 14.0 - ty * 10.0;
    rsx! {
        div {
            class: "gk-tilt-scene {class}",
            style: if let Some(w) = width { format!("width:{w}px") } else { String::new() },
            onmounted: move |e| rect.mounted(e),
            onmouseenter: move |_| {
                hovered.set(true);
                rect.refresh();
                s.set(2, 1.0);
            },
            onmousemove: move |e| {
                let (px, py) = rect.local(&e);
                let b = rect.peek();
                let nx = ((px / b.width.max(1.0)) * 2.0 - 1.0).clamp(-1.0, 1.0) as f32;
                let ny = ((py / b.height.max(1.0)) * 2.0 - 1.0).clamp(-1.0, 1.0) as f32;
                s.set(0, nx);
                s.set(1, ny);
                s.set(3, (nx + 1.0) / 2.0);
                s.set(4, (ny + 1.0) / 2.0);
            },
            onmouseleave: move |_| {
                hovered.set(false);
                for (i, v) in [0.0, 0.0, 0.0, 0.5, 0.5].into_iter().enumerate() {
                    s.set(i, v);
                }
            },
            div {
                class: "gk-card gk-tilt-card",
                style: "{base} transform: perspective(900px) rotateX({rx}deg) rotateY({ry}deg) scale({sc}); box-shadow: {shadow_x}px {shadow_y}px 40px -12px var(--gk-shadow-color);",
                if glare {
                    div { class: "gk-tilt-glare", style: "--gk-gx:{gx * 100.0}%; --gk-gy:{gy * 100.0}%; opacity:{glare_a};" }
                }
                div {
                    class: "gk-card-content",
                    style: "transform: translate({tx * parallax}px, {ty * parallax}px);",
                    {children}
                }
            }
        }
    }
}
