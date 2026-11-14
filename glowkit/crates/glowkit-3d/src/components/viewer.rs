use crate::{models, vec3};
use crate::{Background, Material, Mesh, OrbitControls, Shading, Transform, Viewport3D, Wire};
use egui::{Color32, Response, Ui, Widget};
use glowkit_core::{color, Theme};
use std::sync::Arc;

/// A model shown by [`ModelViewer`].
#[derive(Clone, Debug)]
pub enum Model {
    Cube,
    Sphere,
    Torus,
    TorusKnot,
    Icosahedron,
    Dodecahedron,
    Octahedron,
    Capsule,
    Cone,
    Rocket,
    Gem,
    Mug,
    Tree,
    /// Your own mesh (e.g. from [`Mesh::from_obj`]); it is re-centered and scaled to fit.
    Custom(Arc<Mesh>),
}

impl Model {
    /// The built-in models, for pickers.
    pub const BUILT_IN: [Model; 13] = [
        Model::TorusKnot,
        Model::Rocket,
        Model::Gem,
        Model::Mug,
        Model::Tree,
        Model::Icosahedron,
        Model::Dodecahedron,
        Model::Octahedron,
        Model::Torus,
        Model::Sphere,
        Model::Cube,
        Model::Capsule,
        Model::Cone,
    ];

    /// Display name.
    pub fn name(&self) -> &'static str {
        match self {
            Model::Cube => "Cube",
            Model::Sphere => "Sphere",
            Model::Torus => "Torus",
            Model::TorusKnot => "Torus knot",
            Model::Icosahedron => "Icosahedron",
            Model::Dodecahedron => "Dodecahedron",
            Model::Octahedron => "Octahedron",
            Model::Capsule => "Capsule",
            Model::Cone => "Cone",
            Model::Rocket => "Rocket",
            Model::Gem => "Gem",
            Model::Mug => "Mug",
            Model::Tree => "Tree",
            Model::Custom(_) => "Custom",
        }
    }

    /// Faceted models look best with flat shading.
    pub fn faceted(&self) -> bool {
        matches!(
            self,
            Model::Gem | Model::Icosahedron | Model::Dodecahedron | Model::Octahedron | Model::Tree | Model::Cube
        )
    }

    /// Build the mesh, fitted to a bounding sphere of radius `r`.
    pub fn mesh(&self, r: f32) -> Mesh {
        let m = match self {
            Model::Cube => Mesh::cube(1.0),
            Model::Sphere => Mesh::uv_sphere(1.0, 40, 20),
            Model::Torus => Mesh::torus(1.0, 0.4, 48, 20),
            Model::TorusKnot => Mesh::torus_knot(2, 3, 1.0, 0.3, 180, 14),
            Model::Icosahedron => Mesh::icosahedron(1.0),
            Model::Dodecahedron => Mesh::dodecahedron(1.0),
            Model::Octahedron => Mesh::octahedron(1.0),
            Model::Capsule => Mesh::capsule(0.5, 1.0, 32, 8),
            Model::Cone => Mesh::cone(0.8, 1.6, 40),
            Model::Rocket => models::rocket(),
            Model::Gem => models::gem(),
            Model::Mug => models::mug(),
            Model::Tree => models::tree(),
            Model::Custom(m) => (**m).clone(),
        };
        m.fit(r)
    }
}

/// How [`ModelViewer`] shades its model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Look {
    /// Soft studio lighting with a highlight and rim light.
    #[default]
    Studio,
    /// Glossy gradient (matcap) in the theme's brand colors.
    Gradient,
    /// Cel shaded.
    Toon,
    /// Low-poly facets.
    Flat,
    /// Normal-colored rainbow.
    Normals,
    /// Glowing wireframe only.
    Wireframe,
}

impl Look {
    /// All looks, for pickers.
    pub const ALL: [Look; 6] = [Look::Studio, Look::Gradient, Look::Toon, Look::Flat, Look::Normals, Look::Wireframe];
    /// Display name.
    pub fn name(self) -> &'static str {
        match self {
            Look::Studio => "Studio",
            Look::Gradient => "Gradient",
            Look::Toon => "Toon",
            Look::Flat => "Flat",
            Look::Normals => "Normals",
            Look::Wireframe => "Wireframe",
        }
    }
}

/// A 3D model viewer: a lit, slowly spinning model floating above a soft shadow. Drag to
/// orbit (with inertia), pinch or ctrl+scroll to zoom.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_3d::{ModelViewer, Model, Look};
/// ui.add(ModelViewer::new(Model::Rocket).look(Look::Studio).wireframe(false).height(360.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ModelViewer {
    model: Model,
    look: Look,
    color: Option<Color32>,
    wireframe: bool,
    height: f32,
    auto_rotate: f32,
    shadow: bool,
    grid: bool,
    float: bool,
    id_salt: Option<String>,
}

impl ModelViewer {
    /// A viewer for `model`.
    pub fn new(model: Model) -> Self {
        Self {
            model,
            look: Look::Studio,
            color: None,
            wireframe: false,
            height: 360.0,
            auto_rotate: 0.5,
            shadow: true,
            grid: false,
            float: true,
            id_salt: None,
        }
    }
    /// Shading look.
    pub fn look(mut self, look: Look) -> Self {
        self.look = look;
        self
    }
    /// Base color (defaults to the theme accent; models with their own colors keep them).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Overlay the mesh's edges.
    pub fn wireframe(mut self, on: bool) -> Self {
        self.wireframe = on;
        self
    }
    /// Height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Auto-rotation speed (radians/second, 0 = off).
    pub fn auto_rotate(mut self, speed: f32) -> Self {
        self.auto_rotate = speed;
        self
    }
    /// Soft contact shadow under the model.
    pub fn shadow(mut self, on: bool) -> Self {
        self.shadow = on;
        self
    }
    /// Show a floor grid.
    pub fn grid(mut self, on: bool) -> Self {
        self.grid = on;
        self
    }
    /// Gentle floating (bobbing) motion.
    pub fn float(mut self, on: bool) -> Self {
        self.float = on;
        self
    }
    /// Id salt, if you show several viewers side by side.
    pub fn id_salt(mut self, salt: impl Into<String>) -> Self {
        self.id_salt = Some(salt.into());
        self
    }
}

impl Widget for ModelViewer {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let accent = self.color.unwrap_or(t.accent);
        let salt = self.id_salt.clone().unwrap_or_else(|| "model-viewer".to_owned());
        let bg = if t.dark {
            Background::Radial(color::mix(t.card, accent, 0.14), t.card)
        } else {
            Background::Radial(color::mix(t.background, accent, 0.07), color::mix(t.background, t.muted, 0.6))
        };
        Viewport3D::new(salt)
            .height(self.height)
            .background(bg)
            .fov(38.0)
            .controls(
                OrbitControls::new()
                    .distance(4.5)
                    .pitch(0.24)
                    .yaw(0.5)
                    .auto_rotate(self.auto_rotate)
                    .distance_range(2.5, 12.0),
            )
            .show(ui, |scene, ctx| {
                let model = self.model.clone();
                let key = model.name();
                let mesh = match &model {
                    Model::Custom(m) => Arc::new((**m).clone().fit(1.35)),
                    _ => ctx.mesh(("model", key), || model.mesh(1.35)),
                };
                let bob = if self.float { (ctx.time * 1.3).sin() * 0.08 } else { 0.0 };
                let tf = Transform::from_translation(vec3(0.0, 0.1 + bob, 0.0)).with_euler(
                    0.12 * (ctx.time * 0.7).sin(),
                    0.0,
                    0.0,
                );
                let faceted = model.faceted();
                // models with their own vertex colors keep them unless a color was set explicitly
                let accent = if !mesh.colors.is_empty()
                    && self.color.is_none()
                    && matches!(self.look, Look::Studio | Look::Flat | Look::Toon)
                {
                    Color32::WHITE
                } else {
                    accent
                };
                let mut mat = match self.look {
                    Look::Studio => Material::smooth(accent)
                        .specular(0.55)
                        .shininess(40.0)
                        .rim(0.35)
                        .rim_color(color::mix(accent, Color32::WHITE, 0.6)),
                    Look::Gradient => Material::matcap([
                        color::shade(t.gradient[0], -0.25),
                        t.gradient[1],
                        color::mix(t.gradient[2], Color32::WHITE, 0.35),
                    ])
                    .shininess(24.0),
                    Look::Toon => Material::toon(accent, 3).rim(0.0),
                    Look::Flat => Material::flat(accent).specular(0.2),
                    Look::Normals => Material::normal(),
                    Look::Wireframe => Material::wireframe(accent)
                        .wire(Wire::new(accent).width(1.2).depth_fade(0.8).glow(if t.dark { 1.0 } else { 0.0 })),
                };
                if faceted && self.look == Look::Studio {
                    mat.shading = Shading::Flat;
                }
                if self.wireframe && self.look != Look::Wireframe {
                    let wc = if t.dark {
                        color::with_alpha(Color32::WHITE, 0.55)
                    } else {
                        color::with_alpha(t.foreground, 0.45)
                    };
                    mat = mat.wire(Wire::new(wc).width(1.0).depth_fade(0.3));
                }
                if self.grid {
                    scene.grid(-1.45, 8.0, 16, color::with_alpha(t.muted_foreground, if t.dark { 0.35 } else { 0.5 }));
                }
                if self.shadow {
                    let s = 1.0 - bob * 1.5;
                    scene.shadow(vec3(0.0, -1.45, 0.0), 1.35 * s, crate::components::shadow_c(&t, 1.0));
                }
                scene.draw(&mesh, tf, &mat);
            })
            .response
    }
}
