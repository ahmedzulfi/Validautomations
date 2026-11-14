//! Named variants and [`MotionGroup`]: orchestrate many children with one variant switch,
//! staggering them one after another.

use crate::motion_box::{MotionBox, MotionResponse};
use crate::transform::Props;
use crate::value::Transition;
use egui::{Id, Ui};

/// A set of named [`Props`] states, e.g. `"hidden"` and `"visible"`.
///
/// ```no_run
/// use glowkit_motion::{Props, Variants};
/// let v = Variants::new()
///     .with("hidden", Props::new().opacity(0.0).x(-24.0))
///     .with("visible", Props::IDENTITY);
/// assert_eq!(v.get("visible"), Props::IDENTITY);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Variants {
    entries: Vec<(String, Props)>,
}

impl Variants {
    /// No variants.
    pub fn new() -> Self {
        Self::default()
    }
    /// Add (or replace) a named state.
    pub fn with(mut self, name: impl Into<String>, props: Props) -> Self {
        let name = name.into();
        self.entries.retain(|(n, _)| *n != name);
        self.entries.push((name, props));
        self
    }
    /// `"hidden"` (transparent, offset by `offset`, slightly smaller) and `"visible"` (identity).
    pub fn fade(offset: egui::Vec2) -> Self {
        Self::new()
            .with("hidden", Props::new().opacity(0.0).offset(offset).scale(0.96))
            .with("visible", Props::IDENTITY)
    }
    /// Props of a variant (identity if unknown).
    pub fn get(&self, name: &str) -> Props {
        self.entries.iter().find(|(n, _)| n == name).map_or(Props::IDENTITY, |(_, p)| *p)
    }
    /// Variant names in insertion order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(n, _)| n.as_str())
    }
}

/// Which child starts first in a staggered group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StaggerOrder {
    /// First child first.
    #[default]
    First,
    /// Last child first.
    Last,
    /// From the middle outward.
    Center,
}

/// Orchestrates children through named [`Variants`]: switching the group's variant animates every
/// child, each one `stagger` seconds after the previous, after an initial `delay_children`.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_motion::{MotionGroup, Variants};
/// let show = true;
/// MotionGroup::new(ui.scope_id().with("list"), Variants::fade(egui::vec2(0.0, 16.0)))
///     .initial("hidden")
///     .animate(if show { "visible" } else { "hidden" })
///     .stagger_children(0.06)
///     .delay_children(0.1)
///     .show(ui, |group, ui| {
///         for item in ["One", "Two", "Three"] {
///             group.child(ui, |ui| { ui.label(item); });
///         }
///     });
/// ```
#[must_use = "call `.show(ui, ...)`"]
#[derive(Clone, Debug)]
pub struct MotionGroup {
    id: Id,
    variants: Variants,
    initial: Option<String>,
    animate: String,
    stagger: f32,
    delay_children: f32,
    order: StaggerOrder,
    exit_order: Option<StaggerOrder>,
    transition: Transition,
}

impl MotionGroup {
    /// A group with these variants (animating to `"visible"` by default).
    pub fn new(id: Id, variants: Variants) -> Self {
        Self {
            id,
            variants,
            initial: None,
            animate: "visible".into(),
            stagger: 0.05,
            delay_children: 0.0,
            order: StaggerOrder::First,
            exit_order: None,
            transition: Transition::SPRING,
        }
    }
    /// Variant used on the first frame.
    pub fn initial(mut self, name: impl Into<String>) -> Self {
        self.initial = Some(name.into());
        self
    }
    /// Variant to animate to.
    pub fn animate(mut self, name: impl Into<String>) -> Self {
        self.animate = name.into();
        self
    }
    /// Seconds between consecutive children.
    pub fn stagger_children(mut self, seconds: f32) -> Self {
        self.stagger = seconds.max(0.0);
        self
    }
    /// Seconds before the first child starts.
    pub fn delay_children(mut self, seconds: f32) -> Self {
        self.delay_children = seconds.max(0.0);
        self
    }
    /// Which child goes first.
    pub fn stagger_order(mut self, order: StaggerOrder) -> Self {
        self.order = order;
        self
    }
    /// Stagger order used when animating to the `initial` variant (e.g. `Last` to exit in
    /// reverse). Defaults to the normal order.
    pub fn exit_order(mut self, order: StaggerOrder) -> Self {
        self.exit_order = Some(order);
        self
    }
    /// Transition for each child (the stagger delay is added on top).
    pub fn transition(mut self, t: Transition) -> Self {
        self.transition = t;
        self
    }

    /// Show the group; add children with [`GroupScope::child`].
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut GroupScope, &mut Ui) -> R) -> R {
        let count_key = self.id.with("glowkit_group_count");
        let last_count: usize = ui.data(|d| d.get_temp(count_key)).unwrap_or(0);
        let exiting = self.initial.as_deref() == Some(self.animate.as_str());
        let order = if exiting { self.exit_order.unwrap_or(self.order) } else { self.order };
        let mut scope = GroupScope { group: self, index: 0, count_hint: last_count, order };
        let r = add(&mut scope, ui);
        let n = scope.index;
        ui.data_mut(|d| d.insert_temp(count_key, n));
        r
    }
}

/// Handed to the [`MotionGroup::show`] closure to add children.
pub struct GroupScope {
    group: MotionGroup,
    index: usize,
    count_hint: usize,
    order: StaggerOrder,
}

impl GroupScope {
    /// Index the next child will get.
    pub fn next_index(&self) -> usize {
        self.index
    }

    /// The variant the group is animating to.
    pub fn variant(&self) -> &str {
        &self.group.animate
    }

    /// Stagger delay (seconds) of child `index`.
    pub fn delay_of(&self, index: usize) -> f32 {
        let n = self.count_hint.max(index + 1);
        let rank = match self.order {
            StaggerOrder::First => index as f32,
            StaggerOrder::Last => (n - 1 - index) as f32,
            StaggerOrder::Center => (index as f32 - (n as f32 - 1.0) / 2.0).abs(),
        };
        self.group.delay_children + rank * self.group.stagger
    }

    /// A pre-configured [`MotionBox`] for the next child (customize it, then `.show`). Prefer
    /// [`Self::child`] unless you need hover / tap states.
    pub fn motion_box(&mut self) -> MotionBox {
        let i = self.index;
        self.index += 1;
        let g = &self.group;
        let target = g.variants.get(&g.animate);
        let initial = g.initial.as_deref().map_or(target, |n| g.variants.get(n));
        MotionBox::new(g.id.with(("child", i)))
            .initial(initial)
            .animate(target)
            .transition(g.transition.add_delay(self.delay_of(i)))
    }

    /// Add the next child.
    pub fn child<R>(&mut self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> MotionResponse<R> {
        self.motion_box().show(ui, add)
    }
}
