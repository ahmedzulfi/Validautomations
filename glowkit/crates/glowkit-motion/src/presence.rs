//! Enter / exit animations: [`Presence`] keeps content alive while it animates out, and
//! [`PresenceList`] does the same for keyed lists (with smooth layout shifts).

use crate::transform::{layered, LayerSpec, Props, Space};
use crate::value::{with_motion_value, MotionValue, Transition};
use egui::{Align2, Id, InnerResponse, Sense, Ui, Vec2};
use std::hash::Hash;

#[derive(Clone, Copy, Debug)]
struct PresenceState {
    mounted: bool,
    /// Last layout position relative to the parent's origin (for FLIP-style shifts).
    last_pos: Option<Vec2>,
}

/// Shows content with an enter animation and keeps rendering it while it animates out.
///
/// When `visible` turns false, the content keeps being drawn (and laid out) while animating to
/// the `exit` props; afterwards it is no longer shown. With [`Presence::collapse`] the space it
/// takes also grows / shrinks smoothly so siblings slide instead of jumping.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_motion::{Presence, Props};
/// let open = true;
/// Presence::new(ui.scope_id().with("toast"), open)
///     .initial(Props::new().opacity(0.0).y(-12.0).scale(0.95))
///     .exit(Props::new().opacity(0.0).x(40.0))
///     .show(ui, |ui| ui.label("Saved!"));
/// ```
#[must_use = "call `.show(ui, ...)`"]
#[derive(Clone, Debug)]
pub struct Presence {
    id: Id,
    visible: bool,
    initial: Props,
    exit: Option<Props>,
    style: Props,
    transition: Transition,
    exit_transition: Option<Transition>,
    collapse: bool,
    animate_first: bool,
    origin: Align2,
    layout_shift: Option<Transition>,
}

impl Presence {
    /// Content stored under `id`, currently wanted (`visible`) or not.
    pub fn new(id: Id, visible: bool) -> Self {
        Self {
            id,
            visible,
            initial: Props::new().opacity(0.0).scale(0.96),
            exit: None,
            style: Props::IDENTITY,
            transition: Transition::SPRING,
            exit_transition: None,
            collapse: false,
            animate_first: true,
            origin: Align2::CENTER_CENTER,
            layout_shift: None,
        }
    }
    /// Props the content enters from.
    pub fn initial(mut self, p: Props) -> Self {
        self.initial = p;
        self
    }
    /// Alias of [`Self::initial`].
    pub fn enter(self, p: Props) -> Self {
        self.initial(p)
    }
    /// Props the content exits to (defaults to `initial`).
    pub fn exit(mut self, p: Props) -> Self {
        self.exit = Some(p);
        self
    }
    /// Extra props layered on top every frame (e.g. a hover lift).
    pub fn style(mut self, p: Props) -> Self {
        self.style = p;
        self
    }
    /// Transition for entering.
    pub fn transition(mut self, t: Transition) -> Self {
        self.transition = t;
        self
    }
    /// Transition for exiting (defaults to a quick 0.22 s tween).
    pub fn exit_transition(mut self, t: Transition) -> Self {
        self.exit_transition = Some(t);
        self
    }
    /// Animate the space the content occupies along the layout's main axis.
    pub fn collapse(mut self, on: bool) -> Self {
        self.collapse = on;
        self
    }
    /// Animate in on the very first frame too (default `true`).
    pub fn animate_first(mut self, on: bool) -> Self {
        self.animate_first = on;
        self
    }
    /// Transform origin for scale / rotation.
    pub fn origin(mut self, o: Align2) -> Self {
        self.origin = o;
        self
    }
    /// When the content's layout position jumps (siblings were inserted / removed / reordered),
    /// glide from the old position with this transition (FLIP).
    pub fn layout_shift(mut self, t: Transition) -> Self {
        self.layout_shift = Some(t);
        self
    }

    /// Is the content currently mounted (visible or still exiting)?
    pub fn is_mounted(ui: &Ui, id: Id) -> bool {
        ui.data(|d| d.get_temp::<PresenceState>(id.with("glowkit_presence"))).is_some_and(|s| s.mounted)
    }

    /// Show the content. Returns `None` while the content is not mounted.
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> Option<InnerResponse<R>> {
        let skey = self.id.with("glowkit_presence");
        let pkey = self.id.with("glowkit_presence_props");
        let ckey = self.id.with("glowkit_presence_size");
        let okey = self.id.with("glowkit_presence_shift");
        let first = ui.data(|d| d.get_temp::<PresenceState>(skey)).is_none();
        let mut st = ui.data(|d| d.get_temp::<PresenceState>(skey)).unwrap_or(PresenceState { mounted: false, last_pos: None });
        let dt = glowkit_core::dt(ui);
        let exit = self.exit.unwrap_or(self.initial);
        let exit_tr = self.exit_transition.unwrap_or(Transition::tween(0.22));

        if self.visible && !st.mounted {
            st.mounted = true;
            let start = if first && !self.animate_first { Props::IDENTITY } else { self.initial };
            let k0 = if start == Props::IDENTITY { 1.0 } else { 0.0 };
            with_motion_value(ui, pkey, start, |mv: &mut MotionValue<Props>| mv.set(start));
            with_motion_value(ui, ckey, k0, |mv: &mut MotionValue<f32>| mv.set(k0));
        }
        if !st.mounted {
            ui.data_mut(|d| d.insert_temp(skey, st));
            return None;
        }
        let (target, tr, k_target) = if self.visible { (Props::IDENTITY, self.transition, 1.0) } else { (exit, exit_tr, 0.0) };
        let (props, moving) = with_motion_value(ui, pkey, target, |mv: &mut MotionValue<Props>| {
            mv.animate_to(target, tr);
            (mv.update(dt), mv.is_animating())
        });
        let collapse_tr = if self.visible { Transition::tween(0.32).ease(glowkit_core::Easing::OutCubic) } else { exit_tr };
        let (k, k_moving) = with_motion_value(ui, ckey, k_target, |mv: &mut MotionValue<f32>| {
            mv.animate_to(k_target, collapse_tr);
            (mv.update(dt), mv.is_animating())
        });

        // FLIP layout shift
        let mut shift = Vec2::ZERO;
        if let Some(shift_tr) = self.layout_shift {
            let origin = ui.max_rect().min;
            let pos = ui.cursor().min - origin;
            let jump = st.last_pos.map_or(Vec2::ZERO, |p| p - pos);
            st.last_pos = Some(pos);
            shift = with_motion_value(ui, okey, Vec2::ZERO, |mv: &mut MotionValue<Vec2>| {
                // small per-frame movement comes from siblings collapsing smoothly; only real jumps
                // (reorders, instant insertions) are animated
                if jump.length() > 16.0 && jump.length() < 4000.0 {
                    let cur = mv.get();
                    let vel = mv.velocity();
                    mv.set_with_velocity(cur + jump, vel);
                }
                mv.animate_to(Vec2::ZERO, shift_tr);
                mv.update(dt)
            });
        }

        let space = if self.collapse { Space::Collapse(k) } else { Space::Normal };
        let all = props.then(self.style).then(Props::new().offset(shift));
        let ir = layered(ui, self.id, LayerSpec { props: all, origin: self.origin, sense: Sense::hover(), space }, add_contents);
        if !self.visible && !moving && !k_moving {
            st.mounted = false;
            st.last_pos = None;
        }
        ui.data_mut(|d| d.insert_temp(skey, st));
        Some(ir)
    }
}

#[derive(Clone)]
struct ListEntry<T> {
    key: Id,
    item: T,
    removing: bool,
}

/// Merge the previously shown entries with the new items: new order wins, removed entries stay
/// next to the neighbour they followed so they can animate out in place.
fn merge<T: Clone>(old: &[ListEntry<T>], new: Vec<(Id, T)>) -> Vec<ListEntry<T>> {
    let mut out: Vec<ListEntry<T>> = new.into_iter().map(|(key, item)| ListEntry { key, item, removing: false }).collect();
    let mut prev_kept: Option<Id> = None;
    let mut pending_front = 0;
    for e in old {
        if out.iter().any(|o| o.key == e.key && !o.removing) {
            prev_kept = Some(e.key);
            continue;
        }
        let removed = ListEntry { key: e.key, item: e.item.clone(), removing: true };
        match prev_kept.and_then(|k| out.iter().position(|o| o.key == k)) {
            Some(i) => {
                // after the kept neighbour and any removed entries already placed after it
                let mut j = i + 1;
                while j < out.len() && out[j].removing {
                    j += 1;
                }
                out.insert(j, removed);
            }
            None => {
                out.insert(pending_front, removed);
                pending_front += 1;
            }
        }
        prev_kept = Some(e.key);
    }
    out
}

/// A keyed list whose items animate in when added and out when removed, while the remaining
/// items slide smoothly into their new places (also on reorder).
///
/// Items are cloned into egui memory so removed ones can still be drawn while they exit.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_motion::{PresenceList, Props};
/// let todos = vec![(1, "Buy milk"), (2, "Walk the dog")];
/// PresenceList::new(ui.scope_id().with("todos"))
///     .initial(Props::new().opacity(0.0).x(-30.0))
///     .exit(Props::new().opacity(0.0).x(30.0))
///     .show(ui, &todos, |t| t.0, |ui, t| { ui.label(t.1); });
/// ```
#[must_use = "call `.show(ui, ...)`"]
#[derive(Clone, Debug)]
pub struct PresenceList {
    id: Id,
    initial: Props,
    exit: Option<Props>,
    transition: Transition,
    exit_transition: Option<Transition>,
    shift: Transition,
}

impl PresenceList {
    /// A list stored under `id`.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            initial: Props::new().opacity(0.0).scale(0.9),
            exit: None,
            transition: Transition::SPRING,
            exit_transition: None,
            shift: Transition::spring(260.0, 28.0),
        }
    }
    /// Props items enter from.
    pub fn initial(mut self, p: Props) -> Self {
        self.initial = p;
        self
    }
    /// Props items exit to (defaults to `initial`).
    pub fn exit(mut self, p: Props) -> Self {
        self.exit = Some(p);
        self
    }
    /// Enter transition.
    pub fn transition(mut self, t: Transition) -> Self {
        self.transition = t;
        self
    }
    /// Exit transition.
    pub fn exit_transition(mut self, t: Transition) -> Self {
        self.exit_transition = Some(t);
        self
    }
    /// Transition for items sliding to new positions.
    pub fn layout_transition(mut self, t: Transition) -> Self {
        self.shift = t;
        self
    }

    /// Show `items` (identified by `key`), drawing each with `add`. Returns how many entries are
    /// currently drawn (including exiting ones).
    pub fn show<T, K>(self, ui: &mut Ui, items: &[T], key: impl Fn(&T) -> K, mut add: impl FnMut(&mut Ui, &T)) -> usize
    where
        T: Clone + Send + Sync + 'static,
        K: Hash + std::fmt::Debug,
    {
        let lkey = self.id.with("glowkit_presence_list");
        let first = ui.data(|d| d.get_temp::<Vec<ListEntry<T>>>(lkey)).is_none();
        let old: Vec<ListEntry<T>> = ui.data(|d| d.get_temp(lkey)).unwrap_or_default();
        let new: Vec<(Id, T)> = items.iter().map(|t| (self.id.with(key(t)), t.clone())).collect();
        let mut entries = merge(&old, new);
        let mut gone = Vec::new();
        for e in &entries {
            let mut p = Presence::new(e.key, !e.removing)
                .initial(self.initial)
                .transition(self.transition)
                .collapse(true)
                .animate_first(!first)
                .layout_shift(self.shift);
            if let Some(x) = self.exit {
                p = p.exit(x);
            }
            if let Some(x) = self.exit_transition {
                p = p.exit_transition(x);
            }
            if p.show(ui, |ui| add(ui, &e.item)).is_none() && e.removing {
                gone.push(e.key);
            }
        }
        entries.retain(|e| !gone.contains(&e.key));
        let n = entries.len();
        ui.data_mut(|d| d.insert_temp(lkey, entries));
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(v: &[ListEntry<u32>]) -> Vec<(u32, bool)> {
        v.iter().map(|e| (e.item, e.removing)).collect()
    }

    #[test]
    fn merge_keeps_removed_in_place() {
        let id = |i: u32| Id::NULL.with(i);
        let old: Vec<ListEntry<u32>> = [1, 2, 3, 4].iter().map(|&i| ListEntry { key: id(i), item: i, removing: false }).collect();
        let new = vec![(id(1), 1), (id(3), 3), (id(5), 5)];
        let m = merge(&old, new);
        assert_eq!(keys(&m), vec![(1, false), (2, true), (3, false), (4, true), (5, false)]);
        let new2 = vec![(id(3), 3)];
        let m2 = merge(&old, new2);
        assert_eq!(keys(&m2), vec![(1, true), (2, true), (3, false), (4, true)]);
    }
}
