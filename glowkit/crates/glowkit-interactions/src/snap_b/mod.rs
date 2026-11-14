#![warn(missing_docs)]
//! snap_b components: material headlines (molten metal, liquid chrome, film grain, ordered
//! dither), a dither lens, a halftone logo, a holographic card, liquid-glass tabs / buttons, a
//! morphing navbar, 3D carousels (DNA helix, arc, wave ribbon, image tunnel, spiral), a warp
//! grid, a marbling scroll story, a mosaic trail, a step path, quote and story decks and a
//! particle globe.

pub(crate) mod common;
mod carousels;
mod decks;
mod globe;
mod nav;
mod scrolly;
mod surfaces;
mod text;

pub use carousels::{ArcCarousel, ArcItem, DnaCarousel, ImageTunnel, SpiralSlider, WaveGallery};
pub use decks::{Quote, QuoteDeck, Story, StoryDeck};
pub use globe::ParticleGlobe;
pub use nav::{GlassGlyph, GlassTabs, GlassVariant, LiquidGlassButton, MorphNavbar};
pub use scrolly::{MarbleScroll, MosaicTrail, PathStep, StepPath, WarpGrid};
pub use surfaces::{DitherLens, HalftoneLogo, HoloCard};
pub use text::{ChromeText, DitherText, GrainText, MoltenText};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
