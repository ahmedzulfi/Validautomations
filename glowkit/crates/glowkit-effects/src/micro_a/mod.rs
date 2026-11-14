#![warn(missing_docs)]
//! Micro-interaction blocks: small, tactile components with springy physics (a fanning photo
//! deck, an ASCII wake, sloshing and wake sliders, a folding bill, heat-map blobs, a generate
//! card, an image accordion, magnetic select, a checklist, a liquid toggle, a now-playing bar,
//! stacked bars, emoji reactions, a voice note recorder and a time scrubber) plus the
//! [`ControlSlider`] parameter row used to tune them.

mod ascii;
mod bars;
mod bill;
mod cards;
pub(crate) mod common;
mod heat;
mod media;
mod select;
mod sliders;

pub use ascii::{AsciiWake, GlyphSet};
pub use bars::StackedBars;
pub use bill::{BillPrint, FoldingBill};
pub use cards::{GenerateCard, ImageAccordion, PhotoFan};
pub use common::ControlSlider;
pub use heat::{HeatBlobs, HeatPalette, HeatShape};
pub use media::{EmojiReactions, NowPlaying, ReactionSet, VoiceNote};
pub use select::{LiquidToggle, MagneticSelect, TaskChecklist, TodoItem};
pub use sliders::{SloshSlider, TimeScrubber, WakeSlider};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
