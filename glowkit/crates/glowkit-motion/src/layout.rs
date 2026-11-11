//! Layout animations: when an item's place in the layout changes (reorder, grid ↔ list,
//! expand), it glides from where it was to where it is now. Using the same key in two different
//! places gives a shared-element transition.

use crate::value::{MotionValue, Transition};
use egui::{Id, LayerId, Pos2, Rect, Response, Sense, Ui, UiBuilder, Vec2};
use std::hash::Hash;

/// Animates items between layout positions and sizes, keyed by identity (FLIP-style).
///
/// Each item reserves its *target* space in the layout, but is drawn in its *animated* rect, on
/// a layer above its siblings while moving. Content is laid out inside the animated rect (so a
/// growing card really grows, text re-wraps, buttons stay clickable where they are drawn).
/// Positions are tracked relative to the container's origin, so scrolling never triggers
/// animations.
///
/// Reusing a key in a different place (e.g. a thumbnail in a grid, then a large detail view)
/// makes the item fly from one place to the other, a shared element ("layout id") transition.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_motion::LayoutAnim;
/// let items = vec![3, 1, 2];
/// LayoutAnim::new(ui.scope_id().with("list")).show(ui, |layout, ui| {
///     for i in &items {
///         layout.item(ui, i, egui::vec2(200.0, 36.0), |ui| {
///             ui.label(format!("Item {i}"));
///         });
///     }
/// });
/// ```
#[must_use = "call `.show(ui, ...)`"]
#[derive(Clone, Copy, Debug)]
pub struct LayoutAnim {
    id: Id,
    transition: Transition,
}

impl LayoutAnim {
    /// A layout group stored under `id`.
    pub fn new(id: Id) -> Self {
        Self { id, transition: Transition::spring(320.0, 30.0) }
    }
    /// Transition used for position / size changes.
    pub fn transition(mut self, t: Transition) -> Self {
        self.transition = t;
        self
    }
    /// Run `add`, which places items with the [`LayoutScope`].
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut LayoutScope, &mut Ui) -> R) -> R {
        let parent = ui.layer_id();
        let overlay = LayerId::new(parent.order, self.id.with("glowkit_layout_overlay"));
        let root = ui.ctx().memory(|m| m.areas().parent_layer(parent)).unwrap_or(parent);
        ui.ctx().set_sublayer(root, overlay);
        if let Some(tf) = ui.ctx().layer_transform_to_global(parent) {
            ui.ctx().set_transform_layer(overlay, tf);
        }
        let mut scope = LayoutScope { id: self.id, origin: ui.cursor().min, transition: self.transition, overlay };
        add(&mut scope, ui)
    }
}

/// Places animated items; handed to the [`LayoutAnim::show`] closure.
pub struct LayoutScope {
    id: Id,
    origin: Pos2,
    transition: Transition,
    overlay: LayerId,
}

/// Where an item is drawn this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutFrame {
    /// The animated rect the content is drawn in.
    pub rect: Rect,
    /// The rect it is heading to.
    pub target: Rect,
    /// Still moving?
    pub animating: bool,
}

impl LayoutScope {
    /// The origin positions are measured from (top-left of the container when it started).
    pub fn origin(&self) -> Pos2 {
        self.origin
    }

    /// Animate the rect of `key` toward `target` without drawing anything. Use it to paint
    /// custom shapes.
    pub fn rect(&mut self, ui: &Ui, key: impl Hash + std::fmt::Debug, target: Rect) -> LayoutFrame {
        let key = self.id.with(key).with("glowkit_layout");
        let rel = target.translate(-self.origin.to_vec2());
        let dt = glowkit_core::dt(ui);
        let tr = self.transition;
        let (r, animating) = glowkit_core::with_state(ui, key, || MotionValue::new(rel), |mv: &mut MotionValue<Rect>| {
            mv.animate_to(rel, tr);
            (mv.update(dt), mv.is_animating())
        });
        if animating {
            ui.ctx().request_repaint();
        }
        LayoutFrame { rect: r.translate(self.origin.to_vec2()), target, animating }
    }

    /// Place an item at an explicit `target` rect (no space is allocated) and draw it.
    pub fn item_at(&mut self, ui: &mut Ui, key: impl Hash + std::fmt::Debug + Copy, target: Rect, add: impl FnOnce(&mut Ui)) -> Response {
        let frame = self.rect(ui, key, target);
        let layer = if frame.animating { self.overlay } else { ui.layer_id() };
        let mut child = ui.new_child(
            UiBuilder::new().layer_id(layer).id_salt(self.id.with(key)).max_rect(frame.rect).sense(Sense::click()),
        );
        child.set_clip_rect(ui.clip_rect());
        add(&mut child);
        let mut resp = child.response();
        resp.rect = frame.rect;
        resp
    }

    /// Allocate `size` in the parent layout as the item's target and draw it animated.
    pub fn item(&mut self, ui: &mut Ui, key: impl Hash + std::fmt::Debug + Copy, size: Vec2, add: impl FnOnce(&mut Ui)) -> Response {
        let (target, _) = ui.allocate_exact_size(size, Sense::hover());
        self.item_at(ui, key, target, add)
    }
}
