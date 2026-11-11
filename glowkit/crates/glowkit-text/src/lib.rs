//! # glowkit-text
//!
//! Animated text effects for egui. Every effect is a builder that implements [`egui::Widget`]:
//!
//! ```no_run
//! # let ui: &mut egui::Ui = todo!();
//! ui.add(glowkit_text::ShinyText::new("Hello").size(32.0));
//! ```

mod blur;
mod decrypt;
mod gradient;
mod rotating;
mod shiny;
mod split;
mod stagger;
mod typewriter;
mod circular;
mod count;
mod focus;
mod fuzzy;
mod glitch;
mod proximity;
mod sliding;
mod velocity;
mod wave;
mod util;

pub use blur::{BlurText, Direction, SplitBy};
pub use circular::{CircularText, HoverSpin};
pub use count::{format_number, CountUp};
pub use decrypt::DecryptedText;
pub use focus::TrueFocus;
pub use fuzzy::FuzzyText;
pub use glitch::GlitchText;
pub use proximity::{Falloff, ProximityText};
pub use sliding::SlidingNumber;
pub use velocity::ScrollVelocity;
pub use wave::WaveText;
pub use gradient::GradientText;
pub use rotating::{RotateMode, RotatingText};
pub use shiny::ShinyText;
pub use split::SplitText;
pub use stagger::StaggerFrom;
pub use typewriter::{CursorStyle, TypewriterText};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
