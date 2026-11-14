//! AI agent interface blocks for egui.
//!
//! Modules are re-exported at the crate root.
// Some painting helpers are only used by gallery demos.
#![cfg_attr(not(feature = "demos"), allow(dead_code))]

pub mod supervise;
pub mod assistant;

pub use supervise::*;
pub use assistant::*;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    let mut v = Vec::new();
    v.extend(supervise::demos());
    v.extend(assistant::demos());
    v
}
