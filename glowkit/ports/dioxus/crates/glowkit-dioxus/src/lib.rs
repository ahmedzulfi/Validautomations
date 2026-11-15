//! # glowkit-dioxus
//!
//! glowkit's design language and flagship animated components for [Dioxus](https://dioxuslabs.com)
//! 0.7. Same tokens, presets, easings and spring model as the egui original (shared through
//! [`glowkit_tokens`]); rendering is HTML + CSS, with CSS keyframes for ambient motion and
//! frame-driven springs ([`anim::use_spring`]) for interactive motion.
//!
//! ```ignore
//! use dioxus::prelude::*;
//! use glowkit_dioxus::prelude::*;
//!
//! fn app() -> Element {
//!     rsx! {
//!         GlowkitProvider { theme: Theme::dark(),
//!             ShinyText { text: "Hello from Rust", size: 48.0 }
//!             ShimmerButton { label: "Get started" }
//!         }
//!     }
//! }
//! ```

pub mod anim;
pub mod components;
pub mod pointer;
pub mod theme;

pub use glowkit_tokens as tokens;

/// Everything you need in one import.
pub mod prelude {
    pub use crate::anim::{use_clock, use_replay, use_spring, use_springs, use_tween, SpringConfig, SpringValue, Springs};
    pub use crate::components::agents::*;
    pub use crate::components::ai::*;
    pub use crate::components::backgrounds::{Aurora, DotGrid};
    pub use crate::components::buttons::*;
    pub use crate::components::cards::*;
    pub use crate::components::charts::*;
    pub use crate::components::motion::*;
    pub use crate::components::text::{BlurText, CountUp, CursorStyle, Direction, GradientText, ShinyText, SplitBy, TypewriterText};
    pub use crate::components::toast::*;
    pub use crate::pointer::{use_element_rect, ElementRect};
    pub use crate::theme::{use_theme, GlowkitProvider, GLOWKIT_CSS};
    pub use glowkit_tokens::color::{hex, palettes, Color};
    pub use glowkit_tokens::{Easing, Theme, Trigger};
}
