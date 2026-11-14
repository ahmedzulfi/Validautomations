//! Animated buttons, borders, cards and cursor effects for egui: shimmer, border beams, spotlight cards, tilt, sparks, docks and more.
//!
//! Modules are re-exported at the crate root, so `glowkit_effects::Thing` works for every component.

pub mod buttons;
pub mod cards;
pub mod cursor;
pub mod menus;
pub mod motion;
pub mod scroll;

pub use buttons::*;
pub use cards::*;
pub use cursor::*;
pub use menus::*;
pub use motion::*;
pub use scroll::*;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    let mut v = Vec::new();
    v.extend(buttons::demos());
    v.extend(cards::demos());
    v.extend(cursor::demos());
    v.extend(motion::demos());
    v.extend(menus::demos());
    v.extend(scroll::demos());
    v
}
