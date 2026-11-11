//! Showcase widgets: terminals, globes, device mockups, reveal effects, hover cards and
//! shader-style backgrounds.

mod devices;
mod globe;
mod terminal;
mod util;

pub use devices::{LaptopFrame, PhoneFrame, PhoneStyle, SafariWindow};
pub use globe::{CloudIcon, Globe, GlobeMarker, IconCloud};
pub use terminal::{Terminal, TerminalLine};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
