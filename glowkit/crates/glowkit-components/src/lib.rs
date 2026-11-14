//! Polished everyday components for egui: buttons, inputs, switches, tabs, accordions, dialogs, toasts, tables, loaders and more.
//!
//! Modules are re-exported at the crate root, so `glowkit_components::Thing` works for every component.

pub mod data;
pub mod feedback;
pub mod icon;
pub mod inputs;
pub mod navigation;
pub mod overlays;

pub use data::*;
pub use feedback::*;
pub use icon::Icon;
pub use inputs::*;
pub use navigation::*;
pub use overlays::*;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    let mut v = Vec::new();
    v.extend(inputs::demos());
    v.extend(feedback::demos());
    v.extend(navigation::demos());
    v.extend(overlays::demos());
    v.extend(data::demos());
    v
}
