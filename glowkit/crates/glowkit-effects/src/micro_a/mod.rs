#![warn(missing_docs)]
//! Micro-interaction blocks: small, tactile components with springy physics (a fanning photo
//! deck, an ASCII wake, sloshing and wake sliders, a folding bill, heat-map blobs, a generate
//! card, an image accordion, magnetic select, checklists, a liquid toggle, a now-playing bar,
//! stacked bars, emoji reactions, a voice note recorder and a time scrubber) plus the
//! [`ControlSlider`] parameter row used to tune them.

mod cards;
pub(crate) mod common;

pub use cards::{GenerateCard, ImageAccordion, PhotoFan};
pub use common::ControlSlider;

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
