//! Visual props (opacity / translate / scale / rotate) and the transformed-layer machinery that
//! applies them to arbitrary egui content.
//!
//! # How it works
//!
//! Content is placed in its own [`egui::LayerId`] (a *sublayer* drawn directly above the parent
//! layer) and that layer gets a [`TSTransform`] via [`egui::Context::set_transform_layer`]. egui
//! applies layer transforms both when painting **and** when hit-testing, so buttons inside a
//! scaled / translated box stay clickable exactly where they are drawn.
//!
//! Rotation is not part of `TSTransform`; it is applied by rotating the painted shapes of the
//! layer around the transform origin. Painting is exact, but input hit-testing ignores the
//! rotation (it uses the un-rotated rect), and nested transformed layers are not rotated by their
//! parent. Keep rotation for decorative motion.

use crate::value::{Animatable, Components, MAX_DIM};
use egui::emath::{Rot2, TSTransform};
use egui::{Align2, Id, InnerResponse, LayerId, Pos2, Rect, Sense, Shape, Ui, UiBuilder, Vec2};
use std::sync::Arc;

/// Animatable visual properties of a motion element.
///
/// Mirrors the most common style props of web motion libraries. The [`Default`] is the identity
/// (fully opaque, no offset, scale 1, no rotation).
///
/// ```no_run
/// use glowkit_motion::Props;
/// let hidden = Props::new().opacity(0.0).y(24.0).scale(0.92);
/// let shown = Props::IDENTITY;
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Props {
    /// Opacity multiplier, `0..=1`.
    pub opacity: f32,
    /// Horizontal offset in points.
    pub x: f32,
    /// Vertical offset in points.
    pub y: f32,
    /// Uniform scale around the transform origin.
    pub scale: f32,
    /// Rotation in degrees (clockwise) around the transform origin. Visual only, see the module
    /// docs.
    pub rotate: f32,
}

impl Default for Props {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Props {
    /// No change at all.
    pub const IDENTITY: Props = Props { opacity: 1.0, x: 0.0, y: 0.0, scale: 1.0, rotate: 0.0 };
    /// Fully transparent, otherwise identity.
    pub const HIDDEN: Props = Props { opacity: 0.0, ..Self::IDENTITY };

    /// The identity props, ready for chaining.
    pub const fn new() -> Self {
        Self::IDENTITY
    }
    /// Set the opacity.
    pub const fn opacity(mut self, v: f32) -> Self {
        self.opacity = v;
        self
    }
    /// Set the horizontal offset.
    pub const fn x(mut self, v: f32) -> Self {
        self.x = v;
        self
    }
    /// Set the vertical offset.
    pub const fn y(mut self, v: f32) -> Self {
        self.y = v;
        self
    }
    /// Set both offsets.
    pub const fn offset(mut self, v: Vec2) -> Self {
        self.x = v.x;
        self.y = v.y;
        self
    }
    /// Set the scale.
    pub const fn scale(mut self, v: f32) -> Self {
        self.scale = v;
        self
    }
    /// Set the rotation in degrees.
    pub const fn rotate(mut self, degrees: f32) -> Self {
        self.rotate = degrees;
        self
    }
    /// The offset as a vector.
    pub fn translation(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// Layer `other` on top of `self`: offsets and rotations add, scale and opacity multiply.
    /// This is how gesture states (`while_hover`, ...) combine with the base state.
    pub fn then(self, other: Props) -> Props {
        Props {
            opacity: self.opacity * other.opacity,
            x: self.x + other.x,
            y: self.y + other.y,
            scale: self.scale * other.scale,
            rotate: self.rotate + other.rotate,
        }
    }

    /// The translate + scale part as a transform around `pivot` (rotation excluded).
    pub fn ts_transform(&self, pivot: Pos2) -> TSTransform {
        let s = self.scale.max(0.0);
        TSTransform::new(pivot.to_vec2() * (1.0 - s) + self.translation(), s)
    }

    /// Does this change the geometry at all?
    pub fn is_identity_transform(&self) -> bool {
        self.x.abs() < 1e-3 && self.y.abs() < 1e-3 && (self.scale - 1.0).abs() < 1e-4 && self.rotate.abs() < 1e-3
    }
}

impl Animatable for Props {
    const DIM: usize = 5;
    fn to_components(self) -> Components {
        let mut c = [0.0; MAX_DIM];
        c[..5].copy_from_slice(&[self.opacity, self.x, self.y, self.scale, self.rotate]);
        c
    }
    fn from_components(c: &Components) -> Self {
        Props { opacity: c[0].clamp(0.0, 1.0), x: c[1], y: c[2], scale: c[3], rotate: c[4] }
    }
}

/// Rotate one shape by `rot` around `origin` (positions rotate, rects / text / ellipses also turn).
pub fn rotate_shape(shape: &mut Shape, rot: Rot2, origin: Pos2) {
    let angle = rot.angle();
    let r = |p: Pos2| origin + rot * (p - origin);
    match shape {
        Shape::Noop | Shape::Callback(_) | Shape::Band(_) => {}
        Shape::Vec(v) => v.iter_mut().for_each(|s| rotate_shape(s, rot, origin)),
        Shape::Circle(c) => c.center = r(c.center),
        Shape::Ellipse(e) => {
            e.center = r(e.center);
            e.angle += angle;
        }
        Shape::LineSegment { points, .. } => points.iter_mut().for_each(|p| *p = r(*p)),
        Shape::Path(p) => p.points.iter_mut().for_each(|p| *p = r(*p)),
        Shape::Rect(rs) => {
            rs.rect = Rect::from_center_size(r(rs.rect.center()), rs.rect.size());
            rs.angle += angle;
        }
        Shape::Text(t) => {
            t.pos = r(t.pos);
            t.angle += angle;
        }
        Shape::Mesh(m) => Arc::make_mut(m).rotate(rot, origin),
        Shape::QuadraticBezier(b) => b.points.iter_mut().for_each(|p| *p = r(*p)),
        Shape::CubicBezier(b) => b.points.iter_mut().for_each(|p| *p = r(*p)),
    }
}

/// Rotate every shape painted so far on `layer` by `degrees` around `origin` (layer-local
/// coordinates). Used by the motion containers for the `rotate` prop.
pub fn rotate_layer(ctx: &egui::Context, layer: LayerId, degrees: f32, origin: Pos2) {
    if degrees.abs() < 1e-3 {
        return;
    }
    let rot = Rot2::from_angle(degrees.to_radians());
    ctx.graphics_mut(|g| {
        let list = g.entry(layer);
        let end = list.next_idx().0;
        for i in 0..end {
            list.mutate_shape(egui::layers::ShapeIdx(i), |cs| rotate_shape(&mut cs.shape, rot, origin));
        }
    });
}

/// How a transformed container takes up space in its parent layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Space {
    /// Allocate the content's size (normal flow).
    Normal,
    /// Allocate `factor` of the content's size along the main axis (collapsing presence items).
    Collapse(f32),
}

/// Everything [`transformed`] needs besides the content.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LayerSpec {
    pub props: Props,
    pub origin: Align2,
    pub sense: Sense,
    pub space: Space,
}

/// The rect content occupied in layout space last frame (used as the pivot before layout).
fn rect_key(id: Id) -> Id {
    id.with("glowkit_motion_rect")
}

/// Run `add_contents` inside a transformed layer. Returns the content's response (sensing
/// `spec.sense`, registered beneath the content's own widgets); its `rect` is this frame's
/// untransformed layout rect.
pub(crate) fn layered<R>(ui: &mut Ui, id: Id, spec: LayerSpec, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    let ctx = ui.ctx().clone();
    let parent_layer = ui.layer_id();
    let layer = LayerId::new(parent_layer.order, id.with("glowkit_motion_layer"));
    let root = ctx.memory(|m| m.areas().parent_layer(parent_layer)).unwrap_or(parent_layer);
    if root != layer {
        ctx.set_sublayer(root, layer);
    }
    let parent_tf = ctx.layer_transform_to_global(parent_layer).unwrap_or_default();

    let start = ui.available_rect_before_wrap();
    let prev: Option<Rect> = ui.data(|d| d.get_temp(rect_key(id)));
    let pivot_rect = prev.map_or(start, |r| Rect::from_min_size(start.min, r.size()));
    let est = spec.props.ts_transform(spec.origin.pos_in_rect(&pivot_rect));
    let parent_clip = ui.clip_rect();

    let mut child = ui.new_child(UiBuilder::new().layer_id(layer).id_salt(id).sense(spec.sense).max_rect(start));
    child.set_clip_rect(est.inverse() * parent_clip);
    child.multiply_opacity(spec.props.opacity.clamp(0.0, 1.0));
    let inner = add_contents(&mut child);
    let mut response = child.response();
    let rect = child.min_rect();
    drop(child); // registers the final rect for interaction
    ui.data_mut(|d| d.insert_temp(rect_key(id), rect));

    // allocate space in the parent
    match spec.space {
        Space::Normal => {
            ui.advance_cursor_after_rect(rect);
        }
        Space::Collapse(k) => {
            let k = k.clamp(0.0, 1.0);
            let sp = ui.spacing().item_spacing;
            let alloc = if ui.layout().main_dir().is_horizontal() {
                Rect::from_min_size(rect.min, Vec2::new((rect.width() + sp.x) * k - sp.x, rect.height()))
            } else {
                Rect::from_min_size(rect.min, Vec2::new(rect.width(), (rect.height() + sp.y) * k - sp.y))
            };
            if k > 1e-3 {
                ui.advance_cursor_after_rect(alloc);
            }
        }
    }

    let pivot = spec.origin.pos_in_rect(&rect);
    let own = spec.props.ts_transform(pivot);
    ctx.set_transform_layer(layer, parent_tf * own);
    rotate_layer(&ctx, layer, spec.props.rotate, pivot);
    response.rect = rect;
    InnerResponse::new(inner, response)
}

/// Show `add_contents` with `props` applied (opacity, translation, scale, rotation) around
/// `origin`. Unlike [`Ui::with_visual_transform`], input follows the translated / scaled
/// content too. This is the primitive every motion container is built on.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_motion::{transformed, Props};
/// transformed(ui, ui.scope_id().with("card"), Props::new().y(-4.0).scale(1.02), egui::Align2::CENTER_CENTER, |ui| {
///     ui.button("I am lifted and still clickable");
/// });
/// ```
pub fn transformed<R>(ui: &mut Ui, id: Id, props: Props, origin: Align2, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    layered(ui, id, LayerSpec { props, origin, sense: Sense::hover(), space: Space::Normal }, add_contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn props_compose_and_transform() {
        let a = Props::new().x(10.0).scale(2.0).opacity(0.5);
        let b = Props::new().y(5.0).scale(1.5).opacity(0.5);
        let c = a.then(b);
        assert_eq!(c.translation(), Vec2::new(10.0, 5.0));
        assert_eq!(c.scale, 3.0);
        assert_eq!(c.opacity, 0.25);
        let t = Props::new().scale(2.0).ts_transform(Pos2::new(10.0, 10.0));
        assert_eq!(t * Pos2::new(10.0, 10.0), Pos2::new(10.0, 10.0), "pivot is fixed");
        assert_eq!(t * Pos2::new(11.0, 10.0), Pos2::new(12.0, 10.0));
    }

    #[test]
    fn rotating_a_line() {
        let mut s = Shape::line_segment([Pos2::new(1.0, 0.0), Pos2::new(2.0, 0.0)], (1.0, egui::Color32::WHITE));
        rotate_shape(&mut s, Rot2::from_angle(std::f32::consts::FRAC_PI_2), Pos2::ZERO);
        if let Shape::LineSegment { points, .. } = s {
            assert!((points[0] - Pos2::new(0.0, 1.0)).length() < 1e-5);
        } else {
            unreachable!()
        }
    }
}
