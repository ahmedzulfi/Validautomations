//! Gestures: pointer velocity tracking, [`Draggable`] (axis lock, constraints, elastic overdrag,
//! momentum, snap points) and [`Reorder`] (drag-to-reorder lists).

use crate::transform::{layered, LayerSpec, Props, Space};
use crate::value::{animate, MotionValue, Transition};
use egui::{pos2, vec2, Align2, Id, LayerId, Pos2, Rect, Response, Sense, Ui, UiBuilder, Vec2};
use std::hash::Hash;

/// Estimates pointer (or any 2D) velocity from recent samples, like a pan gesture recognizer.
///
/// ```
/// use glowkit_motion::VelocityTracker;
/// let mut t = VelocityTracker::default();
/// t.push(0.0, egui::pos2(0.0, 0.0));
/// t.push(0.05, egui::pos2(10.0, 0.0));
/// assert!((t.velocity().x - 200.0).abs() < 1e-3);
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct VelocityTracker {
    samples: [(f64, Pos2); 12],
    len: usize,
    head: usize,
}

impl VelocityTracker {
    /// Samples older than this (seconds) relative to the newest are ignored.
    pub const WINDOW: f64 = 0.1;

    /// Forget all samples.
    pub fn clear(&mut self) {
        self.len = 0;
        self.head = 0;
    }

    /// Record a position at `time` seconds.
    pub fn push(&mut self, time: f64, pos: Pos2) {
        self.samples[self.head] = (time, pos);
        self.head = (self.head + 1) % self.samples.len();
        self.len = (self.len + 1).min(self.samples.len());
    }

    fn get(&self, i: usize) -> (f64, Pos2) {
        // i = 0 is the newest
        let n = self.samples.len();
        self.samples[(self.head + n - 1 - i) % n]
    }

    /// Velocity in units per second over the last [`Self::WINDOW`] seconds.
    pub fn velocity(&self) -> Vec2 {
        if self.len < 2 {
            return Vec2::ZERO;
        }
        let (t1, p1) = self.get(0);
        let mut oldest = self.get(1);
        for i in 1..self.len {
            let s = self.get(i);
            if t1 - s.0 > Self::WINDOW {
                break;
            }
            oldest = s;
        }
        let dt = (t1 - oldest.0) as f32;
        if dt <= 1e-5 {
            return Vec2::ZERO;
        }
        (p1 - oldest.1) / dt
    }
}

/// Which axes a drag may move along.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Axis {
    /// Free movement.
    #[default]
    Both,
    /// Horizontal only.
    X,
    /// Vertical only.
    Y,
}

impl Axis {
    /// Zero out the locked component.
    pub fn mask(self, v: Vec2) -> Vec2 {
        match self {
            Axis::Both => v,
            Axis::X => vec2(v.x, 0.0),
            Axis::Y => vec2(0.0, v.y),
        }
    }
}

/// Rubber-band a value past `min..=max`: inside it is unchanged, outside the overshoot is
/// scaled by `elastic` (0 = hard stop, 1 = no resistance).
pub fn rubber_band(v: f32, min: f32, max: f32, elastic: f32) -> f32 {
    if v < min {
        min - (min - v) * elastic
    } else if v > max {
        max + (v - max) * elastic
    } else {
        v
    }
}

#[derive(Clone, Copy, Debug)]
struct DragState {
    offset: MotionValue<Vec2>,
    dragging: bool,
    grab: Pos2,
    start_offset: Vec2,
    tracker: VelocityTracker,
    rest: Option<Rect>,
}

/// What a drag container returns.
#[derive(Debug)]
pub struct DragResponse<R> {
    /// The content closure's value.
    pub inner: R,
    /// Response of the draggable area.
    pub response: Response,
    /// Current offset from the resting layout position.
    pub offset: Vec2,
    /// Is it being dragged right now?
    pub dragging: bool,
    /// Current velocity (points per second).
    pub velocity: Vec2,
}

/// Makes its contents draggable, with momentum and physics.
///
/// * `axis`: lock to X or Y.
/// * `constraints`: a rect (in the same coordinates as the `Ui`) the content must stay inside;
///   dragging past it rubber-bands by `elastic`.
/// * `momentum`: on release the content keeps gliding with the pointer's velocity and bounces
///   back inside the constraints.
/// * `snap_points` / `snap_to_origin`: on release, spring to the snap point nearest to where
///   momentum would have taken it.
/// * `while_drag` / `while_hover`: props layered on top during those gestures.
///
/// Buttons inside keep working: only drags that start on empty space move the content.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_motion::{Draggable, Axis, Props};
/// let area = ui.max_rect();
/// Draggable::new(ui.scope_id().with("puck"))
///     .constraints(area)
///     .elastic(0.35)
///     .while_drag(Props::new().scale(1.1))
///     .show(ui, |ui| ui.label("Drag me"));
/// ```
#[must_use = "call `.show(ui, ...)`"]
#[derive(Clone, Debug)]
pub struct Draggable {
    id: Id,
    axis: Axis,
    constraints: Option<Rect>,
    elastic: f32,
    momentum: bool,
    decay: f32,
    snap_points: Vec<Vec2>,
    snap_to_origin: bool,
    snap_transition: Transition,
    hover: Option<Props>,
    drag: Option<Props>,
}

impl Draggable {
    /// A draggable whose state is stored under `id`.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            axis: Axis::Both,
            constraints: None,
            elastic: 0.35,
            momentum: true,
            decay: 0.3,
            snap_points: Vec::new(),
            snap_to_origin: false,
            snap_transition: Transition::spring(380.0, 28.0),
            hover: None,
            drag: None,
        }
    }
    /// Restrict movement to one axis.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
    /// Keep the content inside this rect.
    pub fn constraints(mut self, rect: Rect) -> Self {
        self.constraints = Some(rect);
        self
    }
    /// How far past the constraints it can be pulled (0 = hard stop, 1 = free).
    pub fn elastic(mut self, e: f32) -> Self {
        self.elastic = e.clamp(0.0, 1.0);
        self
    }
    /// Keep gliding after release.
    pub fn momentum(mut self, on: bool) -> Self {
        self.momentum = on;
        self
    }
    /// Glide time constant in seconds (longer = slides further).
    pub fn decay(mut self, seconds: f32) -> Self {
        self.decay = seconds.max(0.01);
        self
    }
    /// Offsets (relative to the resting position) to snap to on release.
    pub fn snap_points(mut self, points: impl IntoIterator<Item = Vec2>) -> Self {
        self.snap_points = points.into_iter().collect();
        self
    }
    /// Spring back to the resting position on release.
    pub fn snap_to_origin(mut self, on: bool) -> Self {
        self.snap_to_origin = on;
        self
    }
    /// Spring used for snapping.
    pub fn snap_transition(mut self, t: Transition) -> Self {
        self.snap_transition = t;
        self
    }
    /// Props layered on top while hovered.
    pub fn while_hover(mut self, p: Props) -> Self {
        self.hover = Some(p);
        self
    }
    /// Props layered on top while dragged.
    pub fn while_drag(mut self, p: Props) -> Self {
        self.drag = Some(p);
        self
    }

    fn offset_limits(&self, rest: Rect) -> (Vec2, Vec2) {
        let Some(c) = self.constraints else {
            return (Vec2::splat(f32::NEG_INFINITY), Vec2::splat(f32::INFINITY));
        };
        let mut lo = c.min - rest.min;
        let mut hi = c.max - rest.max;
        if lo.x > hi.x {
            lo.x = (lo.x + hi.x) / 2.0;
            hi.x = lo.x;
        }
        if lo.y > hi.y {
            lo.y = (lo.y + hi.y) / 2.0;
            hi.y = lo.y;
        }
        (self.axis.mask(lo), self.axis.mask(hi))
    }

    /// Programmatically move the draggable stored under `id` to `offset` (relative to its resting
    /// position), like drag controls on the web. Ignored while the user is dragging it.
    pub fn animate_to(ui: &Ui, id: Id, offset: Vec2, transition: Transition) {
        let key = id.with("glowkit_drag");
        if let Some(mut st) = ui.data(|d| d.get_temp::<DragState>(key)) {
            if !st.dragging {
                st.offset.restart_to(offset, transition);
                ui.data_mut(|d| d.insert_temp(key, st));
            }
        }
        ui.ctx().request_repaint();
    }

    /// Current offset and whether it is being dragged, for the draggable stored under `id`.
    pub fn state(ui: &Ui, id: Id) -> (Vec2, bool) {
        ui.data(|d| d.get_temp::<DragState>(id.with("glowkit_drag"))).map_or((Vec2::ZERO, false), |s| (s.offset.get(), s.dragging))
    }

    /// Show the contents.
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> DragResponse<R> {
        let skey = self.id.with("glowkit_drag");
        let mut st: DragState = ui.data(|d| d.get_temp(skey)).unwrap_or(DragState {
            offset: MotionValue::new(Vec2::ZERO),
            dragging: false,
            grab: Pos2::ZERO,
            start_offset: Vec2::ZERO,
            tracker: VelocityTracker::default(),
            rest: None,
        });
        let dt = glowkit_core::dt(ui);
        let now = ui.input(|i| i.time);
        let pointer = ui.input(|i| i.pointer.latest_pos());
        let down = ui.input(|i| i.pointer.primary_down());

        if st.dragging {
            if let (Some(p), true) = (pointer, down) {
                st.tracker.push(now, p);
                let raw = st.start_offset + self.axis.mask(p - st.grab);
                let (lo, hi) = st.rest.map_or((Vec2::splat(f32::NEG_INFINITY), Vec2::splat(f32::INFINITY)), |r| self.offset_limits(r));
                let off = vec2(rubber_band(raw.x, lo.x, hi.x, self.elastic), rubber_band(raw.y, lo.y, hi.y, self.elastic));
                let v = self.axis.mask(st.tracker.velocity());
                st.offset.set_with_velocity(off, v);
            }
        } else {
            st.offset.update(dt);
        }
        let offset = st.offset.get();

        let mut gesture = Props::IDENTITY;
        let hovered_before: bool = ui.data(|d| d.get_temp(self.id.with("glowkit_drag_hover"))).unwrap_or(false);
        if let (true, Some(h)) = (hovered_before || st.dragging, self.hover) {
            gesture = gesture.then(h);
        }
        if let (true, Some(d)) = (st.dragging, self.drag) {
            gesture = gesture.then(d);
        }
        let gesture = animate(ui, self.id.with("glowkit_drag_gesture"), gesture, Transition::SNAPPY);
        let props = gesture.then(Props::new().offset(offset));
        let ir = layered(ui, self.id, LayerSpec { props, origin: Align2::CENTER_CENTER, sense: Sense::drag(), space: Space::Normal }, add_contents);
        let resp = ir.response;
        st.rest = Some(resp.rect);
        ui.data_mut(|d| d.insert_temp(self.id.with("glowkit_drag_hover"), resp.contains_pointer()));

        if resp.drag_started() && !st.dragging {
            if let Some(p) = pointer {
                st.dragging = true;
                st.grab = p;
                st.start_offset = offset;
                st.tracker.clear();
                st.tracker.push(now, p);
            }
        }
        if st.dragging && (!down || resp.drag_stopped()) {
            st.dragging = false;
            let v = self.axis.mask(st.tracker.velocity());
            let (lo, hi) = self.offset_limits(resp.rect);
            let mut snaps = self.snap_points.clone();
            if self.snap_to_origin {
                snaps.push(Vec2::ZERO);
            }
            let cur = st.offset.get();
            st.offset.set_with_velocity(cur, v);
            if !snaps.is_empty() {
                let projected = cur + v * self.decay;
                let target = snaps
                    .iter()
                    .copied()
                    .min_by(|a, b| (*a - projected).length_sq().total_cmp(&(*b - projected).length_sq()))
                    .unwrap_or(Vec2::ZERO);
                st.offset.restart_to(target, self.snap_transition);
            } else if self.momentum {
                st.offset.fling(v, self.decay, Some(lo), Some(hi));
            } else {
                st.offset.fling(Vec2::ZERO, self.decay, Some(lo), Some(hi));
            }
        }
        if st.dragging || st.offset.is_animating() {
            ui.ctx().request_repaint();
        }
        if resp.contains_pointer() || st.dragging {
            ui.ctx().set_cursor_icon(if st.dragging { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        let out = DragResponse { inner: ir.inner, response: resp, offset, dragging: st.dragging, velocity: st.offset.velocity() };
        ui.data_mut(|d| d.insert_temp(skey, st));
        out
    }
}

/// Per-item info handed to the [`Reorder`] content closure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReorderItem {
    /// Current index in the list.
    pub index: usize,
    /// Is this item being dragged?
    pub dragging: bool,
    /// Is this item moving to a new slot?
    pub moving: bool,
    /// The rect the item is drawn in.
    pub rect: Rect,
}

#[derive(Clone, Copy, Debug)]
struct ReorderDrag {
    key: Id,
    grab_dy: f32,
    tracker: VelocityTracker,
}

/// A vertical list you can reorder by dragging items, with the others sliding out of the way.
///
/// The `Vec` is reordered in place while dragging. Drags that start on interactive widgets inside
/// an item are left to those widgets.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut fruits = vec!["🍎 Apple", "🍌 Banana", "🍇 Grapes"];
/// glowkit_motion::Reorder::new(ui.scope_id().with("fruits")).show(ui, &mut fruits, |f| *f, |ui, f, _state| {
///     ui.label(*f);
/// });
/// ```
#[must_use = "call `.show(ui, ...)`"]
#[derive(Clone, Copy, Debug)]
pub struct Reorder {
    id: Id,
    item_height: f32,
    gap: f32,
    transition: Transition,
    lift: Props,
}

impl Reorder {
    /// A reorderable list stored under `id`.
    pub fn new(id: Id) -> Self {
        Self { id, item_height: 44.0, gap: 8.0, transition: Transition::spring(420.0, 34.0), lift: Props::new().scale(1.03) }
    }
    /// Height of every row.
    pub fn item_height(mut self, h: f32) -> Self {
        self.item_height = h.max(4.0);
        self
    }
    /// Gap between rows.
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g.max(0.0);
        self
    }
    /// Transition for items sliding into new slots.
    pub fn transition(mut self, t: Transition) -> Self {
        self.transition = t;
        self
    }
    /// Props applied to the dragged item (default: scale 1.03).
    pub fn lift(mut self, p: Props) -> Self {
        self.lift = p;
        self
    }

    /// Show the list. Returns `true` if the order changed this frame.
    pub fn show<T, K: Hash + std::fmt::Debug>(self, ui: &mut Ui, items: &mut Vec<T>, key: impl Fn(&T) -> K, mut add: impl FnMut(&mut Ui, &T, ReorderItem)) -> bool {
        let dkey = self.id.with("glowkit_reorder_drag");
        let mut drag: Option<ReorderDrag> = ui.data(|d| d.get_temp(dkey));
        let width = ui.available_width();
        let origin = ui.cursor().min;
        let pitch = self.item_height + self.gap;
        let n = items.len();
        let total = Rect::from_min_size(origin, vec2(width, (n as f32 * pitch - self.gap).max(0.0)));
        ui.allocate_rect(total, Sense::hover());
        let pointer = ui.input(|i| i.pointer.latest_pos());
        let down = ui.input(|i| i.pointer.primary_down());
        let now = ui.input(|i| i.time);
        let dt = glowkit_core::dt(ui);
        let keys: Vec<Id> = items.iter().map(|t| self.id.with(key(t))).collect();

        // reorder while dragging
        let mut changed = false;
        if let (Some(dr), Some(p)) = (drag.as_mut(), pointer) {
            dr.tracker.push(now, p);
            if let Some(mut i) = keys.iter().position(|k| *k == dr.key) {
                let center = p.y - dr.grab_dy - origin.y + self.item_height / 2.0;
                let slot = ((center - self.item_height / 2.0) / pitch).round().clamp(0.0, n.saturating_sub(1) as f32) as usize;
                while i < slot {
                    items.swap(i, i + 1);
                    i += 1;
                    changed = true;
                }
                while i > slot {
                    items.swap(i, i - 1);
                    i -= 1;
                    changed = true;
                }
            }
        }
        let keys: Vec<Id> = items.iter().map(|t| self.id.with(key(t))).collect();

        let parent = ui.layer_id();
        let overlay = LayerId::new(parent.order, self.id.with("glowkit_reorder_overlay"));
        let root = ui.ctx().memory(|m| m.areas().parent_layer(parent)).unwrap_or(parent);
        ui.ctx().set_sublayer(root, overlay);
        if let Some(tf) = ui.ctx().layer_transform_to_global(parent) {
            ui.ctx().set_transform_layer(overlay, tf);
        }
        let mut any_moving = false;
        for (i, item) in items.iter().enumerate() {
            let k = keys[i];
            let slot_y = i as f32 * pitch;
            let is_drag = drag.is_some_and(|d| d.key == k);
            let ykey = k.with("glowkit_reorder_y");
            let mut mv: MotionValue<f32> = ui.data(|d| d.get_temp(ykey)).unwrap_or(MotionValue::new(slot_y));
            if is_drag {
                if let (Some(p), Some(dr)) = (pointer, drag) {
                    let y = (p.y - dr.grab_dy - origin.y).clamp(-self.item_height * 0.5, total.height() - self.item_height * 0.5);
                    mv.set_with_velocity(y, dr.tracker.velocity().y);
                }
            } else {
                mv.animate_to(slot_y, self.transition);
                mv.update(dt);
            }
            let y = mv.get();
            let moving = mv.is_animating();
            any_moving |= moving;
            ui.data_mut(|d| d.insert_temp(ykey, mv));
            let rect = Rect::from_min_size(pos2(origin.x, origin.y + y), vec2(width, self.item_height));
            let resp = ui.interact(rect, k.with("glowkit_reorder_handle"), Sense::drag());
            if resp.drag_started() && drag.is_none() {
                if let Some(p) = pointer {
                    let mut tracker = VelocityTracker::default();
                    tracker.push(now, p);
                    drag = Some(ReorderDrag { key: k, grab_dy: p.y - rect.min.y, tracker });
                }
            }
            if resp.hovered() || is_drag {
                ui.ctx().set_cursor_icon(if is_drag { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
            }
            let lift = animate(ui, k.with("glowkit_reorder_lift"), if is_drag { self.lift } else { Props::IDENTITY }, Transition::SNAPPY);
            let state = ReorderItem { index: i, dragging: is_drag, moving, rect };
            let layer_ui_id = k.with("glowkit_reorder_item");
            if is_drag || moving || lift != Props::IDENTITY {
                let mut child = ui.new_child(UiBuilder::new().layer_id(overlay).max_rect(rect).id_salt(layer_ui_id));
                child.set_clip_rect(ui.clip_rect());
                if is_drag || lift != Props::IDENTITY {
                    // the dragged row gets its own transformed layer so it can scale
                    layered(&mut child, layer_ui_id, LayerSpec { props: lift, origin: Align2::CENTER_CENTER, sense: Sense::hover(), space: Space::Normal }, |ui| add(ui, item, state));
                } else {
                    add(&mut child, item, state);
                }
            } else {
                let mut child = ui.new_child(UiBuilder::new().max_rect(rect).id_salt(layer_ui_id));
                add(&mut child, item, state);
            }
        }
        if !down {
            drag = None;
        }
        if drag.is_some() || any_moving {
            ui.ctx().request_repaint();
        }
        ui.data_mut(|d| match drag {
            Some(dr) => {
                d.insert_temp(dkey, dr);
            }
            None => d.remove::<ReorderDrag>(dkey),
        });
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rubber_band_scales_overshoot() {
        assert_eq!(rubber_band(5.0, 0.0, 10.0, 0.5), 5.0);
        assert_eq!(rubber_band(-10.0, 0.0, 10.0, 0.5), -5.0);
        assert_eq!(rubber_band(20.0, 0.0, 10.0, 0.0), 10.0);
    }

    #[test]
    fn tracker_uses_recent_window() {
        let mut t = VelocityTracker::default();
        t.push(0.0, pos2(0.0, 0.0));
        t.push(1.0, pos2(1000.0, 0.0)); // old sample, outside the window from the next ones
        t.push(1.05, pos2(1010.0, 0.0));
        t.push(1.1, pos2(1020.0, 5.0));
        let v = t.velocity();
        assert!((v.x - 200.0).abs() < 1e-2, "{v:?}");
        let mut empty = VelocityTracker::default();
        assert_eq!(empty.velocity(), Vec2::ZERO);
        empty.clear();
    }

    #[test]
    fn axis_mask() {
        assert_eq!(Axis::X.mask(vec2(3.0, 4.0)), vec2(3.0, 0.0));
        assert_eq!(Axis::Y.mask(vec2(3.0, 4.0)), vec2(0.0, 4.0));
    }
}
