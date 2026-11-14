//! Animated buttons: shimmer, shiny, rainbow, pulsating, ripple, moving borders, magnetic,
//! neon, liquid fill, stateful, subscribe, copy, particle bursts, 3D push, pixel and more.
//!
//! Every button implements [`egui::Widget`] and returns a normal [`egui::Response`], so
//! `ui.add(ShimmerButton::new("Get started")).clicked()` works like any egui button.

/// Generates a documented builder setter that stores `Some(value)`.
macro_rules! opt_setter {
    ($(#[$m:meta])* $name:ident: $ty:ty) => {
        $(#[$m])*
        pub fn $name(mut self, value: $ty) -> Self {
            self.$name = Some(value);
            self
        }
    };
}
/// Generates a documented builder setter that stores the value as is.
macro_rules! setter {
    ($(#[$m:meta])* $name:ident: $ty:ty) => {
        $(#[$m])*
        pub fn $name(mut self, value: $ty) -> Self {
            self.$name = value;
            self
        }
    };
}
pub(crate) use opt_setter;
pub(crate) use setter;

pub(crate) mod util;

mod border;
mod glow;
mod interactive;
mod shimmer;
mod stateful;

pub use border::{HoverBorderGradient, MovingBorderButton, StarBorderButton};
pub use glow::{GlowButton, NeonButton, PulsatingButton};
pub use interactive::{
    ArrowSlideButton, FillHoverButton, InteractiveHoverButton, MagneticButton, PixelButton, PushButton3D, PushStyle,
    RippleButton,
};
pub use shimmer::{GradientOutlineButton, RainbowButton, ShimmerButton, ShinyButton};
pub use stateful::{CopyButton, ParticleBurstButton, StatefulButton, StatefulButtonState, SubscribeButton};

#[cfg(feature = "demos")]
pub(crate) mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
