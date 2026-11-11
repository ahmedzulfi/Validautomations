//! [`Viewport3D`]: the widget that hosts a [`Scene`] with orbit controls.

use crate::camera::{Camera, OrbitControls, Projection};
use crate::material::{Fog, Lights};
use crate::scene::{Scene, Stats};
use egui::{Color32, Id, IdSalt, Pos2, Rect, Response, Sense, Ui, Vec2};
use glowkit_core::{paint, Theme};
use std::sync::Arc;

/// What a [`Viewport3D`] paints behind the 3D content.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Background {
    /// Nothing: the 3D content sits on whatever is behind the widget.
    #[default]
    None,
    /// A solid rounded rectangle.
    Solid(Color32),
    /// A vertical gradient (top, bottom).
    Vertical(Color32, Color32),
    /// A radial gradient (center, edge).
    Radial(Color32, Color32),
}

/// Per-frame context handed to [`Viewport3D::show`]'s closure.
pub struct Frame3D<'a> {
    /// The hosting ui (read-only; use it for state and input).
    pub ui: &'a Ui,
    /// The widget id: derive state ids from it.
    pub id: Id,
    pub rect: Rect,
    /// Seconds since app start.
    pub time: f32,
    /// Frame delta in seconds.
    pub dt: f32,
    /// Pointer position if it is over the viewport.
    pub pointer: Option<Pos2>,
    /// Pointer in `-1..1` (x right, y up) if it is over the viewport.
    pub pointer_ndc: Option<Vec2>,
    /// The viewport's response (clicks, drags, hover).
    pub response: &'a Response,
    /// Active theme.
    pub theme: Theme,
    /// The orbit controls state (after this frame's input).
    pub controls: OrbitControls,
}

impl Frame3D<'_> {
    /// A value cached for the lifetime of the widget (built once with `init`). Wrap heavy things
    /// such as meshes in an [`Arc`].
    pub fn memo<T: Clone + Send + Sync + 'static>(&self, key: impl std::hash::Hash + std::fmt::Debug, init: impl FnOnce() -> T) -> T {
        glowkit_core::state(self.ui, self.id.with(("memo", key)), init)
    }
    /// A cached mesh (built once).
    pub fn mesh(&self, key: impl std::hash::Hash + std::fmt::Debug, init: impl FnOnce() -> crate::Mesh) -> Arc<crate::Mesh> {
        self.memo(key, || Arc::new(init()))
    }
    /// Mutable per-widget state.
    pub fn state<T: Clone + Send + Sync + 'static, R>(&self, key: impl std::hash::Hash + std::fmt::Debug, init: impl FnOnce() -> T, f: impl FnOnce(&mut T) -> R) -> R {
        glowkit_core::with_state(self.ui, self.id.with(("state", key)), init, f)
    }
    /// Whether the pointer is over the viewport.
    pub fn hovered(&self) -> bool {
        self.pointer.is_some()
    }
    /// Whether the user clicked the viewport this frame (and where).
    pub fn click(&self) -> Option<Pos2> {
        if self.response.clicked() {
            self.response.interact_pointer_pos()
        } else {
            None
        }
    }
}

/// What [`Viewport3D::show`] returns.
pub struct Viewport3DResponse<R> {
    pub response: Response,
    pub inner: R,
    /// Render statistics.
    pub stats: Stats,
}

/// A widget that renders a 3D [`Scene`] with drag-to-orbit camera controls.
///
/// All geometry is projected, lit and depth-sorted on the CPU and painted as batched egui
/// meshes, so it works anywhere egui does (native and WebAssembly) with no extra GPU setup.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_3d::*;
/// Viewport3D::new("my-scene")
///     .height(320.0)
///     .controls(OrbitControls::new().distance(5.0).auto_rotate(0.5))
///     .show(ui, |scene, ctx| {
///         let knot = ctx.mesh("knot", || Mesh::torus_knot(2, 3, 1.2, 0.35, 160, 14));
///         let mat = Material::smooth(ctx.theme.accent).specular(0.6).rim(0.4);
///         scene.draw(&knot, Transform::IDENTITY.with_euler(0.0, ctx.time * 0.3, 0.0), &mat);
///     });
/// ```
#[must_use = "call `.show(ui, |scene, ctx| ...)`"]
pub struct Viewport3D {
    salt: IdSalt,
    size: Option<Vec2>,
    height: f32,
    controls: Option<OrbitControls>,
    camera: Option<Camera>,
    projection: Projection,
    background: Background,
    lights: Option<Lights>,
    fog: Option<Fog>,
    animate: bool,
    sense: Sense,
}

impl Viewport3D {
    /// A new viewport. `id_salt` must be unique among siblings.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            salt: IdSalt::new(id_salt),
            size: None,
            height: 320.0,
            controls: Some(OrbitControls::new()),
            camera: None,
            projection: Projection::Perspective { fov_y: 45f32.to_radians() },
            background: Background::None,
            lights: None,
            fog: None,
            animate: true,
            sense: Sense::click_and_drag(),
        }
    }
    /// Height (width fills the available space).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Exact size.
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Orbit controls (initial state and settings). Enabled by default.
    pub fn controls(mut self, c: OrbitControls) -> Self {
        self.controls = Some(c);
        self
    }
    /// Use a fixed camera instead of orbit controls.
    pub fn camera(mut self, c: Camera) -> Self {
        self.camera = Some(c);
        self.controls = None;
        self
    }
    /// Vertical field of view in degrees.
    pub fn fov(mut self, degrees: f32) -> Self {
        self.projection = Projection::Perspective { fov_y: degrees.clamp(1.0, 170.0).to_radians() };
        self
    }
    /// Orthographic projection showing `height` world units.
    pub fn orthographic(mut self, height: f32) -> Self {
        self.projection = Projection::Orthographic { height };
        self
    }
    /// Background fill.
    pub fn background(mut self, b: Background) -> Self {
        self.background = b;
        self
    }
    /// Lights (defaults to a soft studio rig).
    pub fn lights(mut self, l: Lights) -> Self {
        self.lights = Some(l);
        self
    }
    /// Distance fog.
    pub fn fog(mut self, f: Fog) -> Self {
        self.fog = Some(f);
        self
    }
    /// Keep repainting every frame (default true). Turn off for static scenes.
    pub fn animate(mut self, a: bool) -> Self {
        self.animate = a;
        self
    }
    /// Interaction sense (default click and drag).
    pub fn sense(mut self, s: Sense) -> Self {
        self.sense = s;
        self
    }

    /// Allocate the viewport and draw into it.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Scene, &Frame3D<'_>) -> R) -> Viewport3DResponse<R> {
        let size = self.size.unwrap_or_else(|| egui::vec2(ui.available_width(), self.height));
        let id = ui.scope_id().with_salt(self.salt);
        let (rect, response) = ui.allocate_exact_size(size, self.sense);
        let theme = Theme::of(ui);
        let dt = glowkit_core::dt(ui);
        let mut controls = self.controls.unwrap_or_default();
        if let Some(init) = self.controls {
            let cid = id.with("orbit");
            controls = glowkit_core::with_state(ui, cid, || init, |c| {
                // settings come from the builder every frame; motion state persists
                let (yaw, pitch, dist, target_dist, yv, pv, idle) = (c.yaw, c.pitch, c.distance, c.target_distance, c.yaw_velocity, c.pitch_velocity, c.idle);
                *c = OrbitControls { yaw, pitch, distance: dist, target_distance: target_dist, yaw_velocity: yv, pitch_velocity: pv, idle, ..init };
                c.update(ui, &response, dt);
                *c
            });
            if controls.is_moving() {
                ui.ctx().request_repaint();
            }
            if response.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            } else if response.hovered() && controls.rotate {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
        }
        let camera = self.camera.unwrap_or_else(|| controls.camera(self.projection));
        let painter = ui.painter_at(rect);
        let r = theme.radius;
        match self.background {
            Background::None => {}
            Background::Solid(c) => {
                painter.rect_filled(rect, r, c);
            }
            Background::Vertical(a, b) => paint::gradient_rect(&painter, rect, r, 90.0, &[a, b]),
            Background::Radial(a, b) => {
                let pts = paint::rounded_rect_points(rect, r, 8);
                let stops = [a, b];
                paint::fill_convex_fn(&painter, &pts, 10, paint::radial_fn(rect.center(), rect.size().length() * 0.55, &stops));
            }
        }
        let pointer = response.hover_pos().filter(|p| rect.contains(*p));
        let pointer_ndc = pointer.map(|p| egui::vec2((p.x - rect.center().x) / (rect.width() / 2.0), -(p.y - rect.center().y) / (rect.height() / 2.0)));
        let mut scene = Scene::new(painter, rect, camera);
        if let Some(l) = self.lights {
            scene.lights = l;
        }
        scene.fog = self.fog;
        let frame = Frame3D { ui, id, rect, time: glowkit_core::time(ui), dt, pointer, pointer_ndc, response: &response, theme, controls };
        let visible = ui.is_rect_visible(rect);
        let inner = add(&mut scene, &frame);
        let stats = if visible { scene.paint() } else { Stats::default() };
        if self.animate && visible {
            glowkit_core::animate(ui);
        }
        Viewport3DResponse { response, inner, stats }
    }
}
