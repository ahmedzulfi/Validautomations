//! Theme provider: exposes a glowkit [`Theme`] as CSS custom properties and injects the
//! component stylesheet.

use dioxus::prelude::*;
use glowkit_tokens::Theme;

/// The stylesheet every glowkit-dioxus component relies on (keyframes, layout, effects).
/// [`GlowkitProvider`] injects it; include it yourself if you skip the provider.
pub const GLOWKIT_CSS: &str = include_str!("glowkit.css");

/// Root of a glowkit UI: provides `theme` to every component below it (as CSS custom
/// properties like `--gk-accent`, and as context for [`use_theme`]) and injects
/// [`GLOWKIT_CSS`] once. Change `theme` to restyle everything live.
///
/// ```ignore
/// rsx! { GlowkitProvider { theme: Theme::dark(), ShimmerButton { label: "Hello" } } }
/// ```
#[component]
pub fn GlowkitProvider(
    #[props(default)] theme: Theme,
    #[props(into, default)] class: String,
    #[props(into, default)] style: String,
    /// Skip injecting the stylesheet (when you ship `GLOWKIT_CSS` yourself).
    #[props(default = true)]
    inject_css: bool,
    children: Element,
) -> Element {
    let current = use_memo(use_reactive((&theme,), |(t,)| t));
    use_context_provider(|| current);
    let vars = theme.css_declarations();
    rsx! {
        if inject_css {
            style { dangerous_inner_html: GLOWKIT_CSS }
        }
        div { class: "gk-root {class}", style: "{vars} {style}", {children} }
    }
}

/// The theme of the nearest [`GlowkitProvider`] (or [`Theme::dark`] without one).
/// Reactive: components re-render when the provider's theme changes.
pub fn use_theme() -> Theme {
    match try_use_context::<Memo<Theme>>() {
        Some(m) => m(),
        None => Theme::dark(),
    }
}
