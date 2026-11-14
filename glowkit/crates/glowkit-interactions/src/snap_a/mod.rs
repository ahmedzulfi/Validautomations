#![warn(missing_docs)]
#![allow(clippy::needless_range_loop)]
//! snap_a components: cinematic paper, liquid and cloth effects — crumpled sheets, tearing
//! tickets, printers, pouring ink, dials, clay critters, clothes lines, scrolls, pleats, elastic
//! frames, reeded glass, liquid glass, ink, petals and scroll-driven reveals.

mod clothes;
mod common;
mod crumple;
mod frog;
mod posters;
mod pour;
mod receipt;
mod tear;
mod tuner;
mod turtle;
mod unfurl;

pub use clothes::{ClothesLine, LineClip};
pub use crumple::{CrumpleCarousel, CrumpleItem, CrumpleReveal};
pub use frog::CookieFrog;
pub use pour::PourFilter;
pub use receipt::{ReceiptPlan, ReceiptPrinter};
pub use tear::TearTicket;
pub use tuner::TunerDial;
pub use turtle::TurtlePassword;
pub use unfurl::UnfurlScroll;

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
