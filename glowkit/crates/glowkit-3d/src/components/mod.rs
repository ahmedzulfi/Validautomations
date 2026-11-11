//! Ready-made animated 3D components built on [`crate::Viewport3D`].

mod cards;
mod cubes;
mod data;
mod globe;
mod hero;
mod particles;
mod terrain;
mod viewer;

pub use cards::{Card3D, CardCarousel3D, FlipCard3D, Marquee3D};
pub use cubes::{voxel_font, PuzzleCube, TiltCubes, VoxelText};
pub use data::{BarChart3D, Helix, Planet, SolarSystem};
pub use globe::{Globe3D, GlobeArc};
pub use hero::{FloatingShapes, GlowKnot};
pub use particles::{CloudShape, Galaxy, ParticleMorph, Starfield};
pub use terrain::{TerrainStyle, WaveTerrain};
pub use viewer::{Look, Model, ModelViewer};

use crate::{Mesh, Vec3};
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

pub(crate) fn salt_or(s: &Option<String>, default: &str) -> String {
    s.clone().unwrap_or_else(|| default.to_owned())
}

/// Rounded rectangle outline in 2D (counter-clockwise, y up), `segs` points per corner.
pub(crate) fn rounded_outline(w: f32, h: f32, r: f32, segs: usize) -> Vec<(f32, f32)> {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let (hw, hh) = (w / 2.0 - r, h / 2.0 - r);
    let mut out = Vec::with_capacity(segs * 4 + 4);
    for (k, (cx, cy)) in [(hw, hh), (-hw, hh), (-hw, -hh), (hw, -hh)].into_iter().enumerate() {
        for i in 0..=segs {
            let a = (k as f32 + i as f32 / segs as f32) * std::f32::consts::FRAC_PI_2;
            out.push((cx + a.cos() * r, cy + a.sin() * r));
        }
    }
    out
}

/// A flat rounded rectangle in the XY plane facing +Z.
pub(crate) fn rounded_quad(w: f32, h: f32, r: f32, segs: usize) -> Mesh {
    let mut m = Mesh::new();
    let c = m.vertex(Vec3::ZERO, Vec3::Z);
    let pts = rounded_outline(w, h, r, segs);
    let n = pts.len() as u32;
    for (x, y) in &pts {
        m.vertex(Vec3::new(*x, *y, 0.0), Vec3::Z);
    }
    for i in 0..n {
        m.indices.push([c, 1 + i, 1 + (i + 1) % n]);
        m.edges.push([1 + i, 1 + (i + 1) % n]);
    }
    m.finish()
}
