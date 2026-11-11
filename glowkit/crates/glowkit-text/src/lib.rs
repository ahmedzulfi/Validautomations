//! # glowkit-text
//!
//! Animated text effects for egui. Every effect is a builder that implements [`egui::Widget`]:
//!
//! ```no_run
//! # let ui: &mut egui::Ui = todo!();
//! ui.add(glowkit_text::ShinyText::new("Hello").size(32.0));
//! ```

mod blur;
mod gradient;
mod shiny;

pub use blur::{BlurText, Direction, SplitBy};
pub use gradient::GradientText;
pub use shiny::ShinyText;

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
