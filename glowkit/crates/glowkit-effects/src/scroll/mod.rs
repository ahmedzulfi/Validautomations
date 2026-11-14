#![warn(missing_docs)]
//! Scroll-driven and image-centric effects: content that animates in on view, scroll stacks,
//! progressive blur, flying posters, parallax heroes, sliders, lenses, mask reveals and more.

pub(crate) mod common;
mod gallery;
mod pointer;
mod reveal;
mod stack;
mod sticky;

pub use common::PaintedScene;
pub use gallery::{AppleCard, AppleCardsCarousel, HeroVideoDialog, ImagesSlider, PixelImage, VideoDialogAnimation};
pub use reveal::{AnimatedContent, BlurEdge, FadeContent, GradualBlur, SlideFrom};
pub use pointer::{Lens, LinkPreview, SvgMaskEffect, TextRevealCard};
pub use stack::{ContainerScroll, FlyingPosters, HeroParallax, ParallaxProduct, ScrollStack};
pub use sticky::{StickyScrollReveal, StickySection};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
