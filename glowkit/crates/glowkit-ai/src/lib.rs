//! # glowkit-ai
//!
//! AI interface components for egui.

mod code;
mod common;
mod icons;
pub mod markdown;

pub use code::{highlight, CodeBlock, TokenKind};
pub use icons::{paint_icon, paint_icon_width, Icon};
pub use markdown::Markdown;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}

#[cfg(feature = "demos")]
mod demos;
