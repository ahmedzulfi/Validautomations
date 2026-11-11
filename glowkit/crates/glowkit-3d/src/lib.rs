//! # glowkit-3d
//!
//! Real-time 3D inside egui with no extra GPU pipeline: a small software 3D engine that
//! projects, lights, clips and depth-sorts triangles, lines, points and text into batched egui
//! meshes, plus ready-made animated 3D components.
//!
//! * [`math`]: [`Vec3`], [`Vec4`], [`Mat4`], [`Quat`], [`Transform`].
//! * [`Camera`] (perspective / orthographic) and [`OrbitControls`] (drag, inertia, zoom,
//!   auto-rotate).
//! * [`Mesh`]: primitives (cube, spheres, torus, torus knot, cylinder, cone, capsule, plane,
//!   platonic solids, tubes, lathes), an OBJ loader and built-in [`models`].
//! * [`Material`]: flat, smooth, toon, normal, matcap, wireframe, glow; rim light, emission.
//!   [`Lights`] (ambient, sky, directional, point) and [`Fog`].
//! * [`Scene`]: the renderer. [`Viewport3D`]: the widget that hosts it.
//!
//! ```no_run
//! # let ui: &mut egui::Ui = todo!();
//! use glowkit_3d::*;
//! Viewport3D::new("hello-3d").height(300.0).controls(OrbitControls::new().auto_rotate(0.4)).show(ui, |scene, ctx| {
//!     let mesh = ctx.mesh("ico", || Mesh::icosphere(1.2, 2));
//!     scene.draw(&mesh, Transform::IDENTITY, &Material::matcap(ctx.theme.gradient));
//! });
//! ```
//!
//! Performance: the renderer comfortably handles ~20k triangles per frame in release builds.
//! Painter's-algorithm sorting is per primitive, so intersecting geometry can show artifacts.

pub mod camera;
pub mod material;
pub mod math;
pub mod mesh;
pub mod scene;
pub mod viewport;

mod components;

pub use camera::{Camera, OrbitControls, Projection};
pub use components::*;
pub use material::{DirectionalLight, Fog, Lights, Material, PointLight, Shading, Wire};
pub use math::{vec3, Mat4, Quat, Transform, Vec3, Vec4};
pub use mesh::{models, Mesh, ObjError};
pub use scene::{additive, Scene, Stats};
pub use viewport::{Background, Frame3D, Viewport3D, Viewport3DResponse};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
