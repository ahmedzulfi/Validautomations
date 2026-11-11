//! Motion & layout components: docks, marquees, animated lists, card stacks, carousels,
//! tabs, steppers, beams, orbits and presence transitions.

pub(crate) mod common;
mod carousel;
mod dock;
mod list;
mod marquee;
mod stack;

pub use carousel::{Carousel, CarouselResponse, RingCarousel};
pub use common::CardFace;
pub use dock::{Dock, DockItem, DockResponse, DockStyle};
pub use list::AnimatedList;
pub use marquee::Marquee;
pub use stack::{BounceCards, CardSwap, StackCards};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
