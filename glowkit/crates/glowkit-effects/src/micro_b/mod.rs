//! Micro-interaction components: small, tactile widgets with springy physics (floating-label
//! fields, slide-to-confirm, pull-to-refresh, slingshots, squishy balls, like bursts and more).

mod action_node;
mod asset_swap;
mod assignees;
pub(crate) mod common;
mod create_menu;
#[cfg(feature = "demos")]
mod demos;
mod drag_ball;
mod escape_button;
mod eye_tracker;
mod glass_bubble;
mod label_input;
mod like_button;
mod palette;
mod pull_refresh;
mod slide_confirm;
mod slingshot;
mod step_player;
mod todo_tower;

pub use action_node::ActionNode;
pub use asset_swap::{AssetSwap, SwapAsset};
pub use assignees::{Assignee, AssigneePicker, AvatarLayout};
pub use common::MicroIcon;
pub use create_menu::CreateMenu;
pub use drag_ball::DragBall;
pub use escape_button::EscapeButton;
pub use eye_tracker::{EyeShape, EyeTracker};
pub use glass_bubble::GlassBubble;
pub use label_input::LabelInput;
pub use like_button::{LikeBurst, LikeButton};
pub use palette::PaletteSwatches;
pub use pull_refresh::{PullOutput, PullToRefresh};
pub use slide_confirm::SlideToConfirm;
pub use slingshot::Slingshot;
pub use step_player::StepPlayer;
pub use todo_tower::TodoTower;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
