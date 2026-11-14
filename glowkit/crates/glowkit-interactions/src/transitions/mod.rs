//! Transitions: buttery, precisely timed UI transitions (cards that resize, numbers that pop,
//! badges, menus that morph, modals, tooltips, toasts, checkboxes, switches, loaders and AI
//! "thinking" text), each a reusable widget with its timing exposed as setters.

mod common;
mod controls;
#[cfg(feature = "demos")]
mod demos;
mod numbers;
mod overlays;
mod surfaces;
mod text;

pub use common::MotionCurve;
pub use controls::{AvatarStack, BellBadge, BounceSwitch, DissolveSearch, DrawnCheckbox, LearnMoreLink, ShakeField, SuccessCheck, SwapIcon, SwapIconPair};
pub use numbers::{DrumCounter, PopNumber, SubtleNumber};
pub use overlays::{BannerStack, DropdownSurface, MorphMenu, PopModal, RevealPanel, SlideToast, StackBanner, TooltipBar};
pub use surfaces::{DotMatrixLoader, DotPattern, ResizeCard, SidePager, SkeletonReveal};
pub use text::{ReasoningScroll, StaggerReveal, StateText, StreamText, ThinkingText};

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
