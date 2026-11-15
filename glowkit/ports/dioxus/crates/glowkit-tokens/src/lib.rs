//! # glowkit-tokens
//!
//! The framework-agnostic half of glowkit: design tokens ([`Theme`] with the five glowkit
//! presets), color math ([`color`]: hex, HSV, OKLab mixing, gradients, CSS output), the 31
//! easing curves and the spring model ([`anim`]), and smooth noise ([`noise`]).
//!
//! No UI framework dependency: `glowkit-dioxus` builds on it, and any other port can too.

pub mod anim;
pub mod color;
pub mod noise;
pub mod text;
pub mod theme;

pub use anim::{ease, lerp, smoothstep, stagger, unlerp, approach, Easing, Spring, Trigger};
pub use color::{hex, mix, with_alpha, Color};
pub use theme::Theme;
