//! AI agent interface blocks for egui.
//!
//! Modules are re-exported at the crate root.
// Some painting helpers are only used by gallery demos.
#![cfg_attr(not(feature = "demos"), allow(dead_code))]

pub mod assistant;
pub mod supervise;

pub use assistant::*;
pub use supervise::*;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    let mut v = Vec::new();
    v.extend(supervise::demos());
    v.extend(assistant::demos());
    v
}
