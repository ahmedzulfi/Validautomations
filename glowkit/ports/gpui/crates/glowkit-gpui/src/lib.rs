//! # glowkit for GPUI
//!
//! glowkit's design language (theme tokens, easings, springs) and a set of its flagship animated
//! components, rebuilt for [GPUI](https://www.gpui.rs), Zed's GPU-accelerated UI framework.
//!
//! ```no_run
//! use gpui::{prelude::*, App, Application};
//! use glowkit_gpui::prelude::*;
//!
//! Application::new().run(|cx: &mut App| {
//!     Theme::dark().install(cx);
//!     // open a window whose view renders e.g.
//!     // ShinyText::new("hero", "Hello from GPUI").size(48.0)
//! });
//! ```

pub mod agents;
pub mod ai;
pub mod anim;
pub mod buttons;
pub mod cards;
pub mod charts;
pub mod color;
pub mod motion;
pub mod paint;
pub mod text;
pub mod theme;

pub use agents::{AgentSession, AgentSessionList, Device, RunState};
pub use ai::{ApprovalCard, ChatMessage, Decision, PromptInput, PromptStop, PromptSubmit, Risk, Role};
pub use anim::{ease, Easing, FrameClock, Spring, Spring2};
pub use charts::{Sparkline, StatTile};
pub use buttons::{MagneticButton, ShimmerButton};
pub use cards::{BorderBeam, SpotlightCard, TiltCard};
pub use motion::{AnimatedTabs, Marquee, TabHighlight, Toast, ToastKind, ToastStack};
pub use text::{format_thousands, CountUp, GradientText, ShinyText, StreamingText};
pub use theme::{ActiveTheme, Theme};

/// Everything you need in one import.
pub mod prelude {
    pub use crate::agents::*;
    pub use crate::ai::*;
    pub use crate::anim::{ease, Easing, Spring};
    pub use crate::charts::*;
    pub use crate::buttons::*;
    pub use crate::cards::*;
    pub use crate::motion::*;
    pub use crate::text::*;
    pub use crate::theme::{ActiveTheme, Theme};
}
