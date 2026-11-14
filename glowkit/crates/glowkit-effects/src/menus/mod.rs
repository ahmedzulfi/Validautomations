#![warn(missing_docs)]
//! Menus, navigation and showcase cards: bubble, card, pill and staggered menus, the infinite
//! sphere menu, bento and chroma grids, profile and glass cards, glass surfaces, decay cards,
//! sticker peel, magnet lines, pixel transitions and blurred shapes.

pub(crate) mod common;

mod bento;
mod cards;
mod field;
mod glass;
mod infinite;
mod nav;

pub use bento::{BentoItem, ChromaGrid, ChromaItem, MagicBento};
pub use cards::{DecayCard, GlassIcon, GlassIcons, ProfileCard, StickerPeel};
pub use field::{MagnetLines, PixelTransition, ShapeBlur, ShapeBlurKind};
pub use glass::GlassSurface;
pub use infinite::InfiniteMenu;
pub use nav::{BubbleMenu, CardNav, CardNavItem, PillNav, StaggeredMenu};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
