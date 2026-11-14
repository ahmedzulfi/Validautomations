//! Micro-interaction components: small, tactile widgets with springy physics (floating-label
//! fields, slide-to-confirm, pull-to-refresh, slingshots, squishy balls, like bursts and more).

pub(crate) mod common;
#[cfg(feature = "demos")]
mod demos;
mod create_menu;
mod escape_button;
mod glass_bubble;
mod label_input;
mod pull_refresh;
mod slide_confirm;
mod slingshot;
mod step_player;

pub use common::MicroIcon;
pub use create_menu::CreateMenu;
pub use escape_button::EscapeButton;
pub use glass_bubble::GlassBubble;
pub use label_input::LabelInput;
pub use pull_refresh::{PullOutput, PullToRefresh};
pub use slide_confirm::SlideToConfirm;
pub use slingshot::Slingshot;
pub use step_player::StepPlayer;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
