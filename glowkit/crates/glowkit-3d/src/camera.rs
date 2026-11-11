//! Cameras and orbit controls.

use crate::math::{vec3, Mat4, Vec3};
use egui::{Pos2, Rect, Response, Ui};

/// Perspective or orthographic projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Projection {
    /// Perspective with a vertical field of view in radians.
    Perspective { fov_y: f32 },
    /// Orthographic showing `height` world units vertically.
    Orthographic { height: f32 },
}

/// A camera looking from `position` at `target`.
///
/// ```no_run
/// use glowkit_3d::{Camera, vec3};
/// let cam = Camera::looking_at(vec3(0.0, 2.0, 6.0), vec3(0.0, 0.0, 0.0)).fov_degrees(40.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub projection: Projection,
    /// Near clip distance (geometry closer than this is clipped).
    pub near: f32,
    /// Far distance (used for orthographic depth and as a culling hint).
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self::looking_at(vec3(0.0, 1.5, 6.0), Vec3::ZERO)
    }
}

impl Camera {
    /// Perspective camera (45° fov) at `position` looking at `target`.
    pub fn looking_at(position: Vec3, target: Vec3) -> Self {
        Self { position, target, up: Vec3::Y, projection: Projection::Perspective { fov_y: 45f32.to_radians() }, near: 0.05, far: 500.0 }
    }
    /// Vertical field of view in degrees (switches to perspective).
    pub fn fov_degrees(mut self, deg: f32) -> Self {
        self.projection = Projection::Perspective { fov_y: deg.clamp(1.0, 170.0).to_radians() };
        self
    }
    /// Orthographic projection showing `height` world units vertically.
    pub fn orthographic(mut self, height: f32) -> Self {
        self.projection = Projection::Orthographic { height };
        self
    }
    /// The view matrix (world → camera space).
    pub fn view(&self) -> Mat4 {
        Mat4::look_at(self.position, self.target, self.up)
    }
    /// The projection matrix for a viewport aspect ratio (width / height).
    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        match self.projection {
            Projection::Perspective { fov_y } => Mat4::perspective(fov_y, aspect, self.near, self.far),
            Projection::Orthographic { height } => {
                let h = height / 2.0;
                Mat4::orthographic(-h * aspect, h * aspect, -h, h, self.near, self.far)
            }
        }
    }
    /// Unit vector the camera looks along.
    pub fn forward(&self) -> Vec3 {
        (self.target - self.position).normalized()
    }
    /// Project a world point into `rect`. Returns the screen position and view distance, or
    /// `None` if the point is behind the camera.
    pub fn project(&self, rect: Rect, p: Vec3) -> Option<(Pos2, f32)> {
        let v = self.view().transform_point(p);
        let proj = Projector::new(self, rect);
        proj.project(v)
    }
    /// A world-space ray `(origin, direction)` through a screen position in `rect`.
    pub fn ray(&self, rect: Rect, screen: Pos2) -> (Vec3, Vec3) {
        let nx = (screen.x - rect.center().x) / (rect.height() / 2.0);
        let ny = -(screen.y - rect.center().y) / (rect.height() / 2.0);
        let f = self.forward();
        let mut r = f.cross(self.up).normalized();
        if r.length_sq() < 1e-8 {
            r = f.any_orthogonal();
        }
        let u = r.cross(f);
        match self.projection {
            Projection::Perspective { fov_y } => {
                let t = (fov_y / 2.0).tan();
                (self.position, (f + r * nx * t + u * ny * t).normalized())
            }
            Projection::Orthographic { height } => (self.position + (r * nx + u * ny) * (height / 2.0), f),
        }
    }
}

/// Fast camera-space → screen projection used by the renderer.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Projector {
    pub center: Pos2,
    /// Pixels per unit at distance 1 (perspective) or per unit (ortho).
    pub focal: f32,
    pub ortho: bool,
    pub near: f32,
}

impl Projector {
    pub fn new(cam: &Camera, rect: Rect) -> Self {
        let half_h = rect.height() / 2.0;
        match cam.projection {
            Projection::Perspective { fov_y } => Self { center: rect.center(), focal: half_h / (fov_y / 2.0).tan(), ortho: false, near: cam.near },
            Projection::Orthographic { height } => Self { center: rect.center(), focal: half_h / (height / 2.0).max(1e-4), ortho: true, near: cam.near },
        }
    }
    /// Camera-space point → (screen, distance). `None` when in front of the near plane.
    #[inline]
    pub fn project(&self, v: Vec3) -> Option<(Pos2, f32)> {
        let d = -v.z;
        if d < self.near {
            return None;
        }
        Some((self.project_unchecked(v), d))
    }
    #[inline]
    pub fn project_unchecked(&self, v: Vec3) -> Pos2 {
        if self.ortho {
            Pos2::new(self.center.x + v.x * self.focal, self.center.y - v.y * self.focal)
        } else {
            let k = self.focal / (-v.z).max(1e-6);
            Pos2::new(self.center.x + v.x * k, self.center.y - v.y * k)
        }
    }
    /// Screen size (points) of a world length at distance `d`.
    #[inline]
    pub fn scale_at(&self, d: f32) -> f32 {
        if self.ortho {
            self.focal
        } else {
            self.focal / d.max(1e-6)
        }
    }
}

/// Drag-to-orbit camera controls with inertia, pinch / ctrl+scroll zoom, damping and
/// auto-rotation. Stored per widget by [`crate::Viewport3D`].
///
/// ```no_run
/// use glowkit_3d::OrbitControls;
/// let controls = OrbitControls::new().distance(6.0).pitch(0.35).auto_rotate(0.4);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitControls {
    pub target: Vec3,
    /// Rotation around +Y (radians).
    pub yaw: f32,
    /// Elevation (radians, positive looks down from above).
    pub pitch: f32,
    pub distance: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub min_pitch: f32,
    pub max_pitch: f32,
    /// Auto-rotation speed in radians per second (0 = off). Pauses while dragging.
    pub auto_rotate: f32,
    /// How quickly inertia dies out (per second, higher = stops sooner).
    pub damping: f32,
    /// Radians per point of drag.
    pub sensitivity: f32,
    /// Allow zooming with pinch / ctrl+scroll.
    pub zoom: bool,
    /// Also zoom with the plain scroll wheel while hovered.
    pub scroll_zoom: bool,
    /// Allow rotating by dragging.
    pub rotate: bool,
    // runtime
    pub yaw_velocity: f32,
    pub pitch_velocity: f32,
    pub target_distance: f32,
    pub idle: f32,
}

impl Default for OrbitControls {
    fn default() -> Self {
        Self::new()
    }
}

impl OrbitControls {
    /// Default controls: 6 units away, slightly above, gentle damping, no auto-rotation.
    pub fn new() -> Self {
        Self {
            target: Vec3::ZERO,
            yaw: 0.6,
            pitch: 0.35,
            distance: 6.0,
            min_distance: 1.0,
            max_distance: 50.0,
            min_pitch: -1.45,
            max_pitch: 1.45,
            auto_rotate: 0.0,
            damping: 4.0,
            sensitivity: 0.008,
            zoom: true,
            scroll_zoom: false,
            rotate: true,
            yaw_velocity: 0.0,
            pitch_velocity: 0.0,
            target_distance: 6.0,
            idle: 10.0,
        }
    }
    /// Point the camera orbits around.
    pub fn target(mut self, t: Vec3) -> Self {
        self.target = t;
        self
    }
    /// Initial yaw (radians).
    pub fn yaw(mut self, y: f32) -> Self {
        self.yaw = y;
        self
    }
    /// Initial pitch (radians).
    pub fn pitch(mut self, p: f32) -> Self {
        self.pitch = p;
        self
    }
    /// Initial distance from the target.
    pub fn distance(mut self, d: f32) -> Self {
        self.distance = d;
        self.target_distance = d;
        self
    }
    /// Zoom limits.
    pub fn distance_range(mut self, min: f32, max: f32) -> Self {
        self.min_distance = min;
        self.max_distance = max;
        self
    }
    /// Pitch limits (radians).
    pub fn pitch_range(mut self, min: f32, max: f32) -> Self {
        self.min_pitch = min;
        self.max_pitch = max;
        self
    }
    /// Auto-rotation speed in radians per second.
    pub fn auto_rotate(mut self, speed: f32) -> Self {
        self.auto_rotate = speed;
        self
    }
    /// Inertia damping per second.
    pub fn damping(mut self, d: f32) -> Self {
        self.damping = d.max(0.0);
        self
    }
    /// Enable/disable zoom.
    pub fn zoom(mut self, z: bool) -> Self {
        self.zoom = z;
        self
    }
    /// Zoom with the plain scroll wheel too (off by default so pages still scroll).
    pub fn scroll_zoom(mut self, z: bool) -> Self {
        self.scroll_zoom = z;
        self
    }
    /// Enable/disable drag rotation.
    pub fn rotate(mut self, r: bool) -> Self {
        self.rotate = r;
        self
    }

    /// Feed one frame of input. Call once per frame with the viewport's response.
    pub fn update(&mut self, ui: &Ui, response: &Response, dt: f32) {
        let dt = dt.max(1e-4);
        self.idle += dt;
        if self.rotate && response.dragged() {
            let d = response.drag_delta();
            let dy = d.x * self.sensitivity;
            let dp = d.y * self.sensitivity;
            self.yaw -= dy;
            self.pitch += dp;
            self.yaw_velocity = -dy / dt;
            self.pitch_velocity = dp / dt;
            self.idle = 0.0;
        } else {
            let k = (-self.damping * dt).exp();
            self.yaw += self.yaw_velocity * dt;
            self.pitch += self.pitch_velocity * dt;
            self.yaw_velocity *= k;
            self.pitch_velocity *= k;
            if self.yaw_velocity.abs() < 1e-3 {
                self.yaw_velocity = 0.0;
            }
            if self.pitch_velocity.abs() < 1e-3 {
                self.pitch_velocity = 0.0;
            }
            // ease auto-rotation back in after the user lets go
            let ramp = ((self.idle - 0.6) / 1.2).clamp(0.0, 1.0);
            self.yaw += self.auto_rotate * ramp * dt;
        }
        self.pitch = self.pitch.clamp(self.min_pitch, self.max_pitch);
        if self.zoom && response.hovered() {
            let (zoom, scroll) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta.y));
            if (zoom - 1.0).abs() > 1e-4 {
                self.target_distance /= zoom;
                self.idle = 0.0;
            }
            if self.scroll_zoom && scroll.abs() > 0.0 {
                self.target_distance *= (-scroll * 0.002).exp();
            }
        }
        self.target_distance = self.target_distance.clamp(self.min_distance, self.max_distance);
        self.distance = glowkit_core::anim::approach(self.distance, self.target_distance, 12.0, dt);
    }

    /// Whether the controls are still moving (inertia or zoom easing).
    pub fn is_moving(&self) -> bool {
        self.yaw_velocity != 0.0 || self.pitch_velocity != 0.0 || (self.distance - self.target_distance).abs() > 1e-3 || self.auto_rotate != 0.0
    }

    /// Camera position for the current yaw/pitch/distance.
    pub fn eye(&self) -> Vec3 {
        self.target + Vec3::from_spherical(self.distance, self.yaw, self.pitch)
    }

    /// A camera at the orbit position with the given projection.
    pub fn camera(&self, projection: Projection) -> Camera {
        Camera { projection, ..Camera::looking_at(self.eye(), self.target) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_center_and_behind() {
        let cam = Camera::looking_at(vec3(0.0, 0.0, 5.0), Vec3::ZERO);
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(200.0, 100.0));
        let (p, d) = cam.project(rect, Vec3::ZERO).unwrap();
        assert!((p - rect.center()).length() < 1e-3);
        assert!((d - 5.0).abs() < 1e-4);
        assert!(cam.project(rect, vec3(0.0, 0.0, 10.0)).is_none());
        // a point at the top edge of the frustum maps to the top of the rect
        let t = (22.5f32).to_radians().tan() * 5.0;
        let (top, _) = cam.project(rect, vec3(0.0, t, 0.0)).unwrap();
        assert!((top.y - rect.top()).abs() < 1e-2);
    }

    #[test]
    fn ray_hits_projected_point() {
        let cam = Camera::looking_at(vec3(1.0, 2.0, 5.0), Vec3::ZERO);
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(300.0, 200.0));
        let target = vec3(0.4, -0.3, 0.2);
        let (s, _) = cam.project(rect, target).unwrap();
        let (o, d) = cam.ray(rect, s);
        let along = (target - o).dot(d);
        assert!(((o + d * along) - target).length() < 1e-3);
    }

    #[test]
    fn orbit_eye_distance() {
        let c = OrbitControls::new().distance(4.0).yaw(1.0).pitch(0.5);
        assert!((c.eye().length() - 4.0).abs() < 1e-4);
    }
}
