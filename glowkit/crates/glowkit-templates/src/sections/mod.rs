//! Marketing page sections in the MVP Blocks style: heroes, feature grids, pricing, FAQ,
//! footers and more. Every section is a builder with sensible defaults, rendered with
//! `.show(ui)` which returns a [`SectionResponse`].

mod kit;
mod hero;

pub use hero::{HeroLayout, HeroSection, HeroVisual};

pub use kit::{SectionAlign, SectionBackground, SectionCtaStyle, SectionResponse, SectionSocial, SECTION_MAX_WIDTH, SECTION_STACK_BELOW};
#[allow(unused_imports)]
pub(crate) use kit::*;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}

#[cfg(feature = "demos")]
mod demos;
