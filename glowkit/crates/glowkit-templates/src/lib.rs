//! Page sections (MVP-Blocks style), dashboard shells and complete app templates for egui, built
//! from every glowkit crate.
//!
//! Modules are re-exported at the crate root, so `glowkit_templates::Thing` works for everything.

pub mod apps;
pub mod pages;
pub mod sections;
pub mod shells;

pub use apps::*;
pub use pages::*;
pub use sections::*;
pub use shells::*;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    let mut v = Vec::new();
    v.extend(sections::demos());
    v.extend(pages::demos());
    v.extend(shells::demos());
    v.extend(apps::demos());
    v
}
