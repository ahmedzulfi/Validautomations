//! # glowkit-motion
//!
//! A motion engine for egui in the spirit of the animation libraries web developers love:
//! animated values with springs, tweens and inertia, keyframes, GSAP-style timelines,
//! `motion.div`-like containers with hover / tap states, variants with staggered children,
//! enter / exit presence, FLIP layout animations and shared elements, drag gestures with
//! momentum and snapping, drag-to-reorder lists, scroll-linked motion, motion paths, ropes and
//! spring chains, and shape morphing.
//!
//! ```no_run
//! # let ui: &mut egui::Ui = todo!();
//! use glowkit_motion::{animate, MotionBox, Props, Transition};
//!
//! // 1. animate any value: f32, Vec2, Pos2, Rect, Color32, Props, or your own `Animatable`
//! let open = true;
//! let w: f32 = animate(ui, ui.scope_id().with("w"), if open { 280.0 } else { 80.0 }, Transition::BOUNCY);
//!
//! // 2. animate a whole container like `motion.div`
//! MotionBox::new(ui.scope_id().with("card"))
//!     .initial(Props::new().opacity(0.0).y(24.0))
//!     .while_hover(Props::new().scale(1.04))
//!     .show(ui, |ui| ui.label("Hello"));
//! ```
//!
//! Everything is immediate-mode: call the functions every frame with the state you want, and
//! the animation state lives in egui memory under the ids you pass. Repaints are only requested
//! while something moves.
//!
//! ## Modules at a glance
//!
//! | Web concept                         | glowkit-motion                                              |
//! |-------------------------------------|-------------------------------------------------------------|
//! | `animate()`, `useSpring`            | [`animate`], [`animate_from`], [`MotionValue`], [`Transition`] |
//! | keyframes, repeat, yoyo/mirror      | [`Keyframes`], [`Repeat`], [`RepeatType`]                    |
//! | GSAP timeline                       | [`Timeline`], [`Tween`], [`At`], [`Playback`], [`Scrubber`] |
//! | `motion.div`, whileHover / whileTap | [`MotionBox`], [`Props`], [`transformed`]                   |
//! | variants, staggerChildren           | [`Variants`], [`MotionGroup`]                               |
//! | `AnimatePresence`                   | [`Presence`], [`PresenceList`]                              |
//! | `layout`, `layoutId`                | [`LayoutAnim`]                                              |
//! | drag, `Reorder.Group`               | [`Draggable`], [`Reorder`], [`VelocityTracker`]              |
//! | `useScroll`, `useTransform`, whileInView | [`scroll_progress`], [`transform`], [`parallax`], [`InView`] |
//! | motion path, `pathLength`           | [`MotionPath`]                                              |
//! | physics                             | [`SpringChain`], [`Rope`], [`Lanyard`]                       |
//! | shape morphing                      | [`morph_polygon`], [`MorphShape`], [`ShapeKind`]             |
//!
//! ## Limitations
//!
//! * Translation and scale are applied with egui layer transforms, so painting **and** input
//!   follow them. Rotation is applied to the painted shapes only: hit-testing uses the
//!   un-rotated rect, and nested motion containers are not rotated by their parent.
//! * Scale is uniform (egui's `TSTransform` has a single scale factor). Use [`LayoutAnim`] to
//!   animate real sizes: it lays content out inside the animated rect.

// index loops read better than iterator chains in the numeric code here
#![allow(clippy::needless_range_loop)]

mod gesture;
mod group;
mod keyframes;
mod layout;
mod morph;
mod motion_box;
mod path;
mod physics;
mod presence;
mod scroll;
mod timeline;
mod transform;
mod value;

pub use gesture::{rubber_band, Axis, DragResponse, Draggable, Reorder, ReorderItem, VelocityTracker};
pub use group::{GroupScope, MotionGroup, StaggerOrder, Variants};
pub use keyframes::{Keyframes, Repeat, RepeatType};
pub use layout::{LayoutAnim, LayoutFrame, LayoutScope};
pub use morph::{
    align_outlines, centroid, fill_outline, lerp_points, morph_polygon, resample_closed, MorphShape, ShapeKind,
};
pub use motion_box::{MotionBox, MotionResponse};
pub use path::{MotionPath, PathPoint};
pub use physics::{Lanyard, Rope, SpringChain};
pub use presence::{Presence, PresenceList};
pub use scroll::{
    in_view, parallax, scroll_offset, scroll_progress, scroll_progress_of, transform, transform_with, visible_fraction,
    InView,
};
pub use timeline::{At, Playback, Scrubber, Timeline, TimelineEntry, Tween};
pub use transform::{rotate_layer, rotate_shape, transformed, Props};
pub use value::{
    animate, animate_from, motion_value, tick, with_motion_value, Animatable, Components, MotionValue, Transition,
    MAX_DIM,
};

/// Re-export of the easing curves (from `glowkit-core`).
pub use glowkit_core::anim::Easing;

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
