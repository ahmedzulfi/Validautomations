#![warn(missing_docs)]
//! Mobile-grade gesture interactions in the spirit of native iOS patterns (a swipe-paging photo
//! viewer, a live island, OTP autofill, preview menus, reorderable / swipeable / selectable
//! lists, tips, banners, undo bars, action sheets) plus a set of tactile showcase pieces
//! (draggable marquee, flip letters, hover image list, parallax slider, split showcase, a glass
//! prism, uptime bars, a text reel, a lensed gallery and an active-sessions list).
//!
//! Everything here is driven by real gesture physics: drags track the finger 1:1, overscroll
//! rubber-bands, and releases settle on springs that carry the flick's velocity.

mod banner;
mod gallery;
pub(crate) mod common;
mod index;
mod island;
mod otp;
mod preview_menu;
mod reel;
mod reorder;
mod search;
mod select;
mod sheets;
mod showcase;
mod status;
mod swipe;
mod tip;
mod trail;
mod undo;
mod viewer;

pub use banner::{BannerSpec, TopBanner};
pub use gallery::{DraggableMarquee, LensGallery, ParallaxSlider};
pub use index::SectionIndex;
pub use island::{IslandNotice, LiveIsland};
pub use otp::OtpAutofill;
pub use preview_menu::PreviewMenu;
pub use reel::{FlipLetters, TextReel};
pub use reorder::{ReorderList, TouchRow};
pub use search::SearchReveal;
pub use select::SelectionList;
pub use sheets::{ActionBarSheet, ConfirmSheet, ConfirmStyle};
pub use showcase::{HoverImageList, PrismBeam, ShowcaseSide, SplitShowcase};
pub use status::{ActiveSessions, DeviceSession, UptimeBars};
pub use swipe::SwipeActions;
pub use tip::CoachTip;
pub use trail::TrailNav;
pub use undo::UndoBar;
pub use viewer::SwipeImageViewer;

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
