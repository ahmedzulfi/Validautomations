//! # glowkit-text
//!
//! Animated text effects for egui: shiny, gradient, blur-in, split, decrypt, typewriter,
//! count-up, odometer, rotating words, glitch, fuzzy, circular, marquee, wave, proximity,
//! focus, sparkles, morphing, highlight, curved loops, ASCII art, comic and video-filled text
//! and many more. Every effect is a builder that implements
//! [`egui::Widget`]:
//!
//! ```no_run
//! # let ui: &mut egui::Ui = todo!();
//! ui.add(glowkit_text::ShinyText::new("Hello").size(32.0));
//! ui.add(glowkit_text::SplitText::new("Letters rise").size(48.0));
//! ```
//!
//! Replayable intro animations take a [`glowkit_core::Trigger`] (`OnAppear`, `Loop`, `OnHover`,
//! `OnClick`); continuous effects animate on their own.

mod ascii_text;
mod blur;
mod box_reveal;
mod circular;
mod colorful;
mod comic_text;
mod count;
mod curved_loop;
mod decrypt;
mod falling;
mod focus;
mod fuzzy;
mod generate;
mod glitch;
mod gradient;
mod highlight;
mod line_shadow;
mod morph;
mod proximity;
mod raster;
mod reveal;
mod roll;
mod rotating;
mod scramble;
mod scrambled_text;
mod scroll_float;
mod shiny;
mod shuffle;
mod sliding;
mod spacing;
mod sparkles;
mod split;
mod stagger;
mod text_cursor;
mod typewriter;
mod underline;
mod util;
mod velocity;
mod video_text;
mod wave;

pub use ascii_text::AsciiText;
pub use blur::{BlurText, Direction, SplitBy};
pub use box_reveal::BoxReveal;
pub use circular::{CircularText, HoverSpin};
pub use colorful::ColorfulText;
pub use comic_text::ComicText;
pub use count::{format_number, CountUp};
pub use curved_loop::CurvedLoopText;
pub use decrypt::DecryptedText;
pub use falling::FallingText;
pub use focus::TrueFocus;
pub use fuzzy::FuzzyText;
pub use generate::TextGenerate;
pub use glitch::GlitchText;
pub use gradient::GradientText;
pub use highlight::{HighlightStyle, HighlightText};
pub use line_shadow::LineShadowText;
pub use morph::MorphingText;
pub use proximity::{Falloff, ProximityText};
pub use reveal::ScrollRevealText;
pub use roll::TextRoll;
pub use rotating::{RotateMode, RotatingText};
pub use scramble::ScrambleHover;
pub use scrambled_text::ScrambledText;
pub use scroll_float::ScrollFloat;
pub use shiny::ShinyText;
pub use shuffle::{ShuffleDirection, ShuffleText};
pub use sliding::SlidingNumber;
pub use spacing::GradualSpacing;
pub use sparkles::SparklesText;
pub use split::SplitText;
pub use stagger::StaggerFrom;
pub use text_cursor::TextCursor;
pub use typewriter::{CursorStyle, TypewriterText};
pub use underline::{AnimatedUnderline, UnderlineOrigin};
pub use velocity::ScrollVelocity;
pub use video_text::{VideoScene, VideoText};
pub use wave::WaveText;

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
