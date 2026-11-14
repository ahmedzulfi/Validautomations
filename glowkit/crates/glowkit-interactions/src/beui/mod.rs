//! beui components: Framer-Motion style layout morphs (pills that swell into menus, buttons that
//! become modals, highlights that glide between rows), pickers, stacks and wallet / market blocks.

mod adaptive_stepper;
mod arc_picker;
mod availability_scheduler;
mod bloom_menu;
mod bounce_sidebar;
mod bouncy_accordion;
mod cylinder_carousel;
mod expandable_action_bar;
mod expandable_control;
mod expanding_arrow_button;
mod feedback_widget;
mod file_diff;
mod knockout_bracket;
mod morphing_modal;
mod morphing_search;
mod notification_stack;
mod overflow_actions;
mod prediction_market;
mod project_folder;
mod shared_layout_bg;
mod swipeable_list;
mod wallet_card;
mod card_dealer;
mod center_morph_modal;
mod sortable_stack;
mod wheel_picker;
pub(crate) mod common;
#[cfg(feature = "demos")]
mod demos;

pub use adaptive_stepper::AdaptiveStepper;
pub use arc_picker::{ArcPicker, ArcSide};
pub use availability_scheduler::{AvailabilityScheduler, DayAvailability};
pub use bloom_menu::BloomMenu;
pub use bounce_sidebar::BounceSidebar;
pub use bouncy_accordion::BouncyAccordion;
pub use card_dealer::CardDealer;
pub use center_morph_modal::CenterMorphModal;
pub use cylinder_carousel::{CylinderCarousel, CylinderMode};
pub use expandable_action_bar::ExpandableActionBar;
pub use expandable_control::ExpandableControl;
pub use expanding_arrow_button::ExpandingArrowButton;
pub use feedback_widget::FeedbackWidget;
pub use file_diff::FileDiff;
pub use knockout_bracket::KnockoutBracket;
pub use morphing_modal::MorphingModal;
pub use morphing_search::MorphingSearch;
pub use notification_stack::NotificationStack;
pub use overflow_actions::OverflowActions;
pub use prediction_market::PredictionMarket;
pub use project_folder::ProjectFolder;
pub use shared_layout_bg::SharedLayoutBg;
pub use swipeable_list::SwipeableList;
pub use wallet_card::WalletCard;
pub use common::BeIcon;
pub use sortable_stack::{SortableStack, SortableTask};
pub use wheel_picker::WheelPicker;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
