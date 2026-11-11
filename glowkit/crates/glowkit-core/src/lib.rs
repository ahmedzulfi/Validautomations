//! # glowkit-core
//!
//! The foundation every glowkit crate is built on:
//!
//! * [`theme`]: design tokens (colors, radii, gradients) with dark and light presets.
//! * [`anim`]: easing curves, springs, tweens, and per-widget animation clocks.
//! * [`color`]: HSV / OKLab helpers and perceptual color mixing.
//! * [`paint`]: gradient fills, radial glows, soft shadows, and anti-aliased meshes.
//! * [`noise`]: value / gradient noise and fractal Brownian motion for organic motion.
//! * [`rng`]: a tiny deterministic random number generator (no dependencies).
//! * [`text`]: per-glyph layout so text can be animated letter by letter.
//! * [`demo`]: the registry the gallery uses to list components.

pub mod anim;
pub mod color;
pub mod demo;
pub mod noise;
pub mod paint;
pub mod rng;
pub mod text;
pub mod theme;

pub use anim::{ease, Clock, Easing, Spring, Trigger};
pub use color::{hex, hsv, mix, with_alpha};
pub use rng::Rng;
pub use theme::Theme;

/// Re-export of the `egui` version glowkit is built against.
pub use egui;

/// Current time in seconds (as `f32`), read from the egui input state.
#[inline]
pub fn time(ui: &egui::Ui) -> f32 {
    ui.input(|i| i.time) as f32
}

/// Frame delta time in seconds, clamped so a long pause never makes physics explode.
#[inline]
pub fn dt(ui: &egui::Ui) -> f32 {
    ui.input(|i| i.stable_dt).clamp(0.0, 1.0 / 20.0)
}

/// Ask egui to keep repainting: used by every continuously animated component.
#[inline]
pub fn animate(ui: &egui::Ui) {
    ui.ctx().request_repaint();
}

/// Read persistent per-widget state, creating it with `init` on first use.
pub fn state<T: Clone + Send + Sync + 'static>(
    ui: &egui::Ui,
    id: egui::Id,
    init: impl FnOnce() -> T,
) -> T {
    ui.data_mut(|d| d.get_temp_mut_or_insert_with(id, init).clone())
}

/// Store per-widget state.
pub fn set_state<T: Clone + Send + Sync + 'static>(ui: &egui::Ui, id: egui::Id, value: T) {
    ui.data_mut(|d| d.insert_temp(id, value));
}

/// Mutate per-widget state in place, creating it with `init` on first use.
///
/// **Do not call any `ui` / `ctx` method inside `f`.** The closure runs while egui's context lock
/// is held, so reading input, time, or requesting a repaint there will deadlock (on wasm it shows
/// up as a panic). Read what you need *before* calling `with_state`, and act on the result after.
pub fn with_state<T: Clone + Send + Sync + 'static, R>(
    ui: &egui::Ui,
    id: egui::Id,
    init: impl FnOnce() -> T,
    f: impl FnOnce(&mut T) -> R,
) -> R {
    ui.data_mut(|d| f(d.get_temp_mut_or_insert_with(id, init)))
}

/// Size a widget: `requested` if set, otherwise all available width and `default_height`.
pub fn desired_size(ui: &egui::Ui, requested: Option<egui::Vec2>, default_height: f32) -> egui::Vec2 {
    requested.unwrap_or_else(|| egui::vec2(ui.available_width(), default_height))
}
