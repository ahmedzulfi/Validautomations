//! Showcase widgets: terminals, globes, device mockups, reveal effects, hover cards and
//! shader-style backgrounds.

mod cards;
mod code;
mod devices;
mod fields;
mod globe;
mod highlight;
mod people;
mod progress;
mod reveal;
mod shaders;
mod terminal;
mod toggles;
mod vanish;
mod util;

pub use cards::{CanvasRevealCard, DirectionAwareHover, GlowingStarsCard, Pin3D, TextHoverEffect};
pub use code::{CodeComparison, ScriptCopyBox, StickyBanner};
pub use devices::{LaptopFrame, PhoneFrame, PhoneStyle, SafariWindow};
pub use fields::{BallPit, MetaBalls, RippleGrid};
pub use globe::{CloudIcon, Globe, GlobeMarker, IconCloud};
pub use highlight::{Cover, GeminiEffect, PointerHighlight};
pub use people::{AnimatedTestimonials, AnimatedTooltip, Person, Testimonial, Tweet, TweetCard};
pub use progress::{MultiStepLoader, TracingBeam};
pub use reveal::{CompareSlider, ScratchResponse, ScratchToReveal};
pub use shaders::{ColorBends, DarkVeil, FaultyTerminal, GradientBlinds, PixelBlast, PixelPattern, PixelShape};
pub use terminal::{Terminal, TerminalLine};
pub use toggles::{DayNightSwitch, ThemeToggler};
pub use vanish::{VanishInput, VanishOutput};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
