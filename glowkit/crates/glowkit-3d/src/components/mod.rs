//! Ready-made animated 3D components built on [`crate::Viewport3D`].

mod viewer;

pub use viewer::{Look, Model, ModelViewer};

use egui::Color32;
use glowkit_core::{color, Theme};

/// Pick a color that reads as "glow" on the current theme: additive on dark themes,
/// a normal (alpha) blend on light ones.
pub(crate) fn glow_c(t: &Theme, c: Color32, strength: f32) -> Color32 {
    if t.dark {
        crate::additive(c.gamma_multiply(strength.clamp(0.0, 1.0)))
    } else {
        color::with_alpha(c, (strength * 0.8).clamp(0.0, 1.0))
    }
}

/// A shadow tint that works on the current theme.
pub(crate) fn shadow_c(t: &Theme, strength: f32) -> Color32 {
    let base = if t.dark { Color32::BLACK } else { color::mix(t.foreground, t.accent, 0.15) };
    color::with_alpha(base, (if t.dark { 0.55 } else { 0.22 }) * strength)
}
