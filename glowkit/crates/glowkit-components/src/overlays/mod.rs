//! Overlay & feedback components: dialogs, sheets, toasts, tooltips, dropdown menus, guided tours
//! and a morphing "island".

mod dialog;
mod icons;
mod island;
pub(crate) mod kit;
pub(crate) mod menu;
pub(crate) mod popover;
mod toast;
mod tour;

pub use dialog::{AlertDialog, AlertResponse, Dialog, DialogResponse, Sheet, SheetSide};
pub use icons::LineIcon;
pub use island::{IslandView, MorphingIsland};
pub use menu::{DropdownMenu, MenuEntry};
pub use popover::{tooltip, Placement, Tooltip};
pub use toast::{toast, Toast, ToastId, ToastKind, ToastPosition, Toaster};
pub use tour::{SpotlightTour, TourStep};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
