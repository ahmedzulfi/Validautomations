//! Ready-made page blocks and showcase widgets for egui: hero sections, feature grids, bento,
//! pricing, testimonials, terminals, globes, device mockups and more.
//!
//! Modules are re-exported at the crate root, so `glowkit_blocks::Thing` works for every component.

pub mod showcase;

pub use showcase::*;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    let mut v = Vec::new();
    v.extend(showcase::demos());
    v
}
