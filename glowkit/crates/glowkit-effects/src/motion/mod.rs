//! Motion & layout components: docks, marquees, animated lists, card stacks, carousels,
//! tabs, steppers, beams, orbits and presence transitions.

pub(crate) mod common;
mod carousel;
mod dock;
mod expandable;
mod flowing;
mod folder;
mod gooey;
mod list;
mod marquee;
mod orbit;
mod presence;
mod scroll;
mod slider;
mod stack;
mod stepper;
mod tabs;

pub use carousel::{Carousel, CarouselResponse, RingCarousel};
pub use common::CardFace;
pub use dock::{Dock, DockItem, DockResponse, DockStyle};
pub use expandable::{ExpandableCard, ExpandableItem};
pub use flowing::FlowingMenu;
pub use folder::Folder;
pub use gooey::GooeyNav;
pub use list::AnimatedList;
pub use marquee::Marquee;
pub use orbit::{AnimatedBeam, OrbitingCircles};
pub use presence::{PresenceEffect, SpringPresence, TransitionPanel};
pub use scroll::{InfiniteScroll, Masonry};
pub use slider::ElasticSlider;
pub use stack::{BounceCards, CardSwap, StackCards};
pub use stepper::Stepper;
pub use tabs::{AnimatedTabs, TabHighlight};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
