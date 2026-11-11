//! # glowkit
//!
//! A big, beautiful, animated component library for Rust GUI apps on Windows, macOS, Linux and
//! the web, built on [egui](https://github.com/emilk/egui).
//!
//! ```no_run
//! use glowkit::prelude::*;
//!
//! fn ui(ui: &mut egui::Ui) {
//!     Theme::dark().install(ui.ctx());
//!     ui.add(glowkit::text::ShinyText::new("Hello, glowkit"));
//! }
//! ```
//!
//! Every category lives in its own crate and is enabled by a cargo feature (all on by default):
//! `text`, `backgrounds`, `effects`, `components`, `charts`, `ai`, `blocks`, `motion`, `three-d`.

pub use glowkit_core as core;
pub use glowkit_core::{anim, color, egui, noise, paint, rng, Theme};

#[cfg(feature = "ai")]
pub use glowkit_ai as ai;
#[cfg(feature = "backgrounds")]
pub use glowkit_backgrounds as backgrounds;
#[cfg(feature = "blocks")]
pub use glowkit_blocks as blocks;
#[cfg(feature = "motion")]
pub use glowkit_motion as motion;
#[cfg(feature = "three-d")]
pub use glowkit_3d as three_d;
#[cfg(feature = "charts")]
pub use glowkit_charts as charts;
#[cfg(feature = "components")]
pub use glowkit_components as components;
#[cfg(feature = "effects")]
pub use glowkit_effects as effects;
#[cfg(feature = "text")]
pub use glowkit_text as text;

/// Common imports.
pub mod prelude {
    pub use glowkit_core::anim::{Easing, Spring, Trigger};
    pub use glowkit_core::color::{hex, hsv, mix, with_alpha};
    pub use glowkit_core::egui;
    pub use glowkit_core::Theme;
}

/// Every demo from every enabled crate (used by the gallery).
#[cfg(feature = "demos")]
pub fn all_demos() -> Vec<glowkit_core::demo::Demo> {
    let mut v = Vec::new();
    #[cfg(feature = "text")]
    v.extend(glowkit_text::demos());
    #[cfg(feature = "backgrounds")]
    v.extend(glowkit_backgrounds::demos());
    #[cfg(feature = "effects")]
    v.extend(glowkit_effects::demos());
    #[cfg(feature = "components")]
    v.extend(glowkit_components::demos());
    #[cfg(feature = "charts")]
    v.extend(glowkit_charts::demos());
    #[cfg(feature = "ai")]
    v.extend(glowkit_ai::demos());
    #[cfg(feature = "blocks")]
    v.extend(glowkit_blocks::demos());
    #[cfg(feature = "motion")]
    v.extend(glowkit_motion::demos());
    #[cfg(feature = "three-d")]
    v.extend(glowkit_3d::demos());
    v.sort_by_key(|d| d.category);
    v
}
