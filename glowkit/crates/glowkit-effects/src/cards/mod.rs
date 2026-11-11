//! Cards and borders: border beams, shine and electric borders, spotlight / magic / tilt /
//! glare cards, encrypted and pixel cards, glass, wobble, hover grids and focus cards.
//!
//! Cards are containers: `SpotlightCard::new().show(ui, |ui| { ... })` paints the effect and
//! lays your content out inside with padding. Border effects also offer `paint(ui, rect)` so
//! they can decorate any rect.

pub(crate) mod shell;

mod borders;
mod grid;
mod pointer;
mod tilt;

pub use borders::{AnimatedGradientBorder, BorderBeam, DashedMarchingBorder, ElectricBorder, NeonGradientCard, ShineBorder};
pub use grid::{CardHoverEffect, FocusCards};
pub use pointer::{EncryptedCard, GlareCard, GlowingBorderCard, MagicCard, PixelCard, SpotlightCard};
pub use tilt::{GlassCard, HoverLiftCard, TiltCard, WobbleCard};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
