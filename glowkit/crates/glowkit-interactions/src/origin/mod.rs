//! origin components: particle cosmos pieces, glowing fields, tactile 3D-ish buttons, image
//! galleries and painted scenery.

mod buttons;
mod common;
mod cosmos;
mod fields;
mod galleries;
mod scenery;
mod tiles;

pub use buttons::{CobaltSprite, GlassKeycap, SpiderButton, VelourBolster, VenomButton};
pub use cosmos::{AccretionDisc, CosmicOrb, ParticleSaturn, Tornado};
pub use fields::{CursorRingField, FluidTrail, LightCables, MatrixJunction, RibbonGlow, WireTerrain};
pub use galleries::{CoverflowGallery, GalleryTunnel, HoverImageReveal, SpiralGallery};
pub use scenery::{CloudSky, GlimmerVale, VoxelMeadow};
pub use tiles::{CorollaGleam, PlushMark, PlushTile};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
