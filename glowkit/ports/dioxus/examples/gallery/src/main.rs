//! glowkit for Dioxus: component gallery.
//!
//! Web: `cargo build -p glowkit-dioxus-gallery --target wasm32-unknown-unknown` then run
//! wasm-bindgen (see `examples/gallery/build-web.sh`). Desktop:
//! `cargo run -p glowkit-dioxus-gallery --no-default-features --features desktop`.

use dioxus::prelude::*;
use glowkit_dioxus::prelude::*;

mod demos;

const GALLERY_CSS: &str = include_str!("gallery.css");

/// One entry in the gallery.
#[derive(Clone, Copy, PartialEq)]
pub struct Entry {
    pub id: &'static str,
    pub title: &'static str,
    pub category: &'static str,
    pub desc: &'static str,
    pub mirrors: &'static str,
}

pub const ENTRIES: &[Entry] = &[
    Entry { id: "shiny-text", title: "Shiny Text", category: "Text Animations", desc: "A band of light sweeps across the text, like light catching metal.", mirrors: "glowkit_text::ShinyText" },
    Entry { id: "gradient-text", title: "Gradient Text", category: "Text Animations", desc: "Text filled with a flowing gradient, optionally inside an animated gradient pill.", mirrors: "glowkit_text::GradientText" },
    Entry { id: "blur-text", title: "Blur Text", category: "Text Animations", desc: "Words or letters come into focus from a blur, one after another.", mirrors: "glowkit_text::BlurText" },
    Entry { id: "typewriter", title: "Typewriter", category: "Text Animations", desc: "Types, holds, deletes and retypes a list of strings with a blinking cursor.", mirrors: "glowkit_text::TypewriterText" },
    Entry { id: "count-up", title: "Count Up", category: "Text Animations", desc: "Numbers count to their target with easing, separators and affixes.", mirrors: "glowkit_text::CountUp" },
    Entry { id: "aurora", title: "Aurora", category: "Backgrounds", desc: "Flowing curtains of northern light that drift and shimmer, fading into the background.", mirrors: "glowkit_backgrounds::Aurora" },
    Entry { id: "dot-grid", title: "Dot Grid", category: "Backgrounds", desc: "Dots light up and scatter around the pointer, then spring back. Click for a shockwave.", mirrors: "glowkit_backgrounds::DotGrid" },
    Entry { id: "shimmer-button", title: "Shimmer Button", category: "Buttons", desc: "A dark pill with a spark of light racing around its border and an inner glow that rises on hover.", mirrors: "glowkit_effects::ShimmerButton" },
    Entry { id: "rainbow-button", title: "Rainbow Button", category: "Buttons", desc: "A high-contrast button with a flowing rainbow border and a blurred rainbow glow.", mirrors: "glowkit_effects::RainbowButton" },
    Entry { id: "magnetic-button", title: "Magnetic Button", category: "Buttons", desc: "Leans toward the pointer as it approaches and springs back when it leaves.", mirrors: "glowkit_effects::MagneticButton" },
    Entry { id: "border-beam", title: "Border Beam", category: "Cards", desc: "A comet of light travels around the card's border, fading along its tail.", mirrors: "glowkit_effects::BorderBeam" },
    Entry { id: "spotlight-card", title: "Spotlight Card", category: "Cards", desc: "A soft spotlight follows the pointer across the card, optionally revealing dots.", mirrors: "glowkit_effects::SpotlightCard" },
    Entry { id: "tilt-card", title: "Tilt Card", category: "Cards", desc: "Tilts in 3D toward the pointer with a moving glare and parallax content.", mirrors: "glowkit_effects::TiltCard" },
    Entry { id: "marquee", title: "Marquee", category: "Motion & Layout", desc: "An infinitely scrolling strip with faded edges that pauses on hover.", mirrors: "glowkit_effects::Marquee" },
    Entry { id: "dock", title: "Dock", category: "Motion & Layout", desc: "A macOS-style dock: icons magnify around the pointer, show labels and bounce on click.", mirrors: "glowkit_effects::Dock" },
    Entry { id: "animated-tabs", title: "Animated Tabs", category: "Motion & Layout", desc: "A highlight that springs and stretches between tabs, with a gliding hover highlight.", mirrors: "glowkit_effects::AnimatedTabs" },
    Entry { id: "toast", title: "Toast", category: "Overlays & Feedback", desc: "A Sonner-style stack: newest in front, older ones tucked behind, fanning out on hover.", mirrors: "glowkit_components::Toaster" },
    Entry { id: "chat-message", title: "Chat Message", category: "AI", desc: "User bubbles, assistant Markdown with avatar and actions, system notices, streaming replies.", mirrors: "glowkit_ai::ChatMessage" },
    Entry { id: "streaming-text", title: "Streaming Text", category: "AI", desc: "Reveals a response token by token, each one fading in behind a caret.", mirrors: "glowkit_ai::StreamingText" },
    Entry { id: "prompt-input", title: "Prompt Input", category: "AI", desc: "The chat composer: auto-growing input, attachments, toolbar, send/stop and a focus glow.", mirrors: "glowkit_ai::PromptInput" },
    Entry { id: "approval-card", title: "Approval Card", category: "AI Agents", desc: "Human in the loop: approve, deny or edit a tool call an agent wants to run.", mirrors: "glowkit_agents::ApprovalQueue (row)" },
    Entry { id: "stat-tile", title: "Stat Tile", category: "Charts", desc: "A KPI card: a number that counts up, a delta badge and a sparkline that draws in.", mirrors: "glowkit_charts::StatTile" },
];

fn main() {
    dioxus::launch(App);
}

fn initial_route() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.location().hash().ok())
            .map(|h| h.trim_start_matches('#').to_string())
            .unwrap_or_default()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        String::new()
    }
}

fn set_route(r: &str) {
    #[cfg(target_arch = "wasm32")]
    if let Some(w) = web_sys::window() {
        let _ = w.location().set_hash(r);
        w.scroll_to_with_x_and_y(0.0, 0.0);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = r;
}

#[component]
fn App() -> Element {
    let route = initial_route();
    let (theme_name, route) = match route.split_once('@') {
        Some((r, t)) => (t.to_string(), r.to_string()),
        None => ("daylight".to_string(), route),
    };
    let mut route = use_signal(|| route);
    let mut theme = use_signal(|| Theme::by_name(&theme_name).unwrap_or_else(Theme::light));
    let t = theme();
    rsx! {
        style { dangerous_inner_html: GALLERY_CSS }
        GlowkitProvider { theme: t.clone(), class: "gallery",
            ToastProvider {
                div { class: "g-shell",
                    header { class: "g-top",
                        a { class: "g-logo", href: "#", onclick: move |_| { route.set(String::new()); set_route(""); },
                            span { class: "g-logo-mark", "✦" }
                            span { class: "g-logo-word", "glowkit" }
                            span { class: "g-badge", "for Dioxus" }
                        }
                        nav { class: "g-nav",
                            for cat in ["Text Animations", "Buttons", "Cards", "AI"] {
                                {
                                    let first = ENTRIES.iter().find(|e| e.category == cat).map(|e| e.id).unwrap_or("");
                                    rsx! { a { href: "#{first}", onclick: move |_| { route.set(first.to_string()); set_route(first); }, "{cat}" } }
                                }
                            }
                        }
                        div { class: "g-grow" }
                        ThemeSwatches { theme }
                        button { class: "g-mode", title: "Toggle light / dark",
                            onclick: move |_| {
                                let next = if theme.peek().dark { Theme::light() } else { Theme::dark() };
                                theme.set(next);
                            },
                            if t.dark { "☀" } else { "☾" }
                        }
                    }
                    div { class: "g-body",
                        aside { class: "g-side",
                            a { class: if route().is_empty() { "g-side-link g-active" } else { "g-side-link" }, href: "#",
                                onclick: move |_| { route.set(String::new()); set_route(""); }, "Overview" }
                            for cat in categories() {
                                div { class: "g-side-cat", "{cat}" }
                                for e in ENTRIES.iter().filter(|e| e.category == cat) {
                                    a {
                                        class: if route() == e.id { "g-side-link g-active" } else { "g-side-link" },
                                        href: "#{e.id}",
                                        onclick: move |_| { route.set(e.id.to_string()); set_route(e.id); },
                                        "{e.title}"
                                    }
                                }
                            }
                        }
                        main { class: "g-main",
                            match ENTRIES.iter().position(|e| e.id == route()) {
                                Some(i) => rsx! { Detail { key: "{ENTRIES[i].id}", index: i, route } },
                                None => rsx! { Overview { route } },
                            }
                        }
                    }
                }
            }
        }
    }
}

fn categories() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = Vec::new();
    for e in ENTRIES {
        if !v.contains(&e.category) {
            v.push(e.category);
        }
    }
    v
}

#[component]
fn ThemeSwatches(theme: Signal<Theme>) -> Element {
    let current = theme().name;
    rsx! {
        div { class: "g-swatches",
            for p in Theme::presets() {
                {
                    let name = p.name;
                    let bg = p.background.to_css();
                    let accent = p.accent.to_css();
                    rsx! {
                        button {
                            class: if name == current { "g-swatch g-on" } else { "g-swatch" },
                            title: "{name}",
                            style: "background:{bg}; --sw:{accent};",
                            onclick: move |_| theme.set(Theme::by_name(name).unwrap_or_default()),
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Detail(index: usize, route: Signal<String>) -> Element {
    let e = ENTRIES[index];
    let prev = ENTRIES[(index + ENTRIES.len() - 1) % ENTRIES.len()].id;
    let next = ENTRIES[(index + 1) % ENTRIES.len()].id;
    let mut replay = use_signal(|| 0u32);
    rsx! {
        div { class: "g-detail",
            section { class: "g-preview",
                div { class: "g-preview-bar",
                    span { class: "g-pill", "Preview" }
                    div { class: "g-grow" }
                    button { class: "g-round", title: "Replay", onclick: move |_| replay += 1, "↻" }
                    button { class: "g-round", title: "Previous", onclick: move |_| { route.set(prev.to_string()); set_route(prev); }, "←" }
                    button { class: "g-round", title: "Next", onclick: move |_| { route.set(next.to_string()); set_route(next); }, "→" }
                }
                div { class: "g-stage", key: "{replay}",
                    {demos::demo(e.id)}
                }
            }
            aside { class: "g-info",
                div { class: "g-info-cat", "{e.category}" }
                h1 { class: "g-info-title", "{e.title}" }
                p { class: "g-info-desc", "{e.desc}" }
                div { class: "g-info-label", "Ports" }
                div { class: "g-info-row", code { "{e.mirrors}" } span { "egui" } }
                div { class: "g-info-label", "Usage" }
                pre { class: "g-code", "{demos::snippet(e.id)}" }
            }
        }
    }
}

#[component]
fn Overview(route: Signal<String>) -> Element {
    rsx! {
        div { class: "g-overview",
            Aurora { class: "g-hero", style: "min-height: 340px;", height: 0.56,
                div { class: "g-hero-inner",
                    h1 { class: "g-hero-title", "Premium animated UI" }
                    div { class: "g-hero-sub", GradientText { text: "for Dioxus apps", size: 54.0, bold: true } }
                    p { class: "g-hero-copy",
                        "glowkit's design language, ported from egui to Dioxus 0.7: same tokens, presets, easings and springs. "
                        "{ENTRIES.len()} flagship components."
                    }
                    div { class: "g-hero-ctas",
                        ShimmerButton { label: "Browse components", onclick: move |_| { route.set("shiny-text".into()); set_route("shiny-text"); } }
                        RainbowButton { label: "cargo add glowkit-dioxus" }
                    }
                }
            }
            div { class: "g-grid",
                for e in ENTRIES.iter() {
                    a {
                        class: "g-tile",
                        href: "#{e.id}",
                        onclick: move |_| { route.set(e.id.to_string()); set_route(e.id); },
                        div { class: "g-tile-stage", {demos::thumb(e.id)} }
                        div { class: "g-tile-meta",
                            span { class: "g-tile-title", "{e.title}" }
                            span { class: "g-tile-cat", "{e.category}" }
                        }
                    }
                }
            }
        }
    }
}
