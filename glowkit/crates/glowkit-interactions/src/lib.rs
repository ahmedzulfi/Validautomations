//! Distinctive micro-interactions, transitions and cinematic effects for egui.
//!
//! Modules are re-exported at the crate root.

pub mod transitions;
pub mod suiss;
pub mod snap_a;
pub mod snap_b;
pub mod origin;
pub mod beui;
pub mod obsidian_board;

pub use transitions::*;
pub use suiss::*;
pub use snap_a::*;
pub use snap_b::*;
pub use origin::*;
pub use beui::*;
pub use obsidian_board::*;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    let mut v = Vec::new();
    v.extend(transitions::demos());
    v.extend(suiss::demos());
    v.extend(snap_a::demos());
    v.extend(snap_b::demos());
    v.extend(origin::demos());
    v.extend(beui::demos());
    v.extend(obsidian_board::demos());
    v
}
