// Some painting helpers are only used by gallery demos.
#![cfg_attr(not(feature = "demos"), allow(dead_code))]
//! Distinctive micro-interactions, transitions and cinematic effects for egui.
//!
//! Modules are re-exported at the crate root.

pub mod beui;
pub mod game;
pub mod obsidian_board;
pub mod origin;
pub mod snap_a;
pub mod snap_b;
pub mod suiss;
pub mod transitions;

pub use beui::*;
pub use game::*;
pub use obsidian_board::*;
pub use origin::*;
pub use snap_a::*;
pub use snap_b::*;
pub use suiss::*;
pub use transitions::*;

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
    v.extend(game::demos());
    v
}
