//! [`MotionBox`]: a container that animates its children's opacity / position / scale / rotation,
//! with enter, hover and tap states, like `motion.div` on the web.

use crate::transform::{layered, LayerSpec, Props, Space};
use crate::value::{animate_from, with_motion_value, Transition};
use egui::{Align2, Id, Response, Sense, Ui};

/// What a motion container returns: the closure's value, the container's response (hover / tap
/// state, its layout `rect`) and the props applied this frame.
#[derive(Debug)]
pub struct MotionResponse<R> {
    /// Whatever the content closure returned.
    pub inner: R,
    /// Response of the whole container (sensing hover, or clicks when tap states are used).
    pub response: Response,
    /// The props that were applied this frame.
    pub props: Props,
}

#[derive(Clone, Copy)]
struct Gestures {
    hovered: bool,
    pressed: bool,
    last_animate: Props,
    /// Was the latest target change caused by hover / tap (vs. a new `animate`)?
    gesture_cause: bool,
}

/// A container that animates its contents between visual states.
///
/// * `initial`: props on the first frame (the enter animation starts here).
/// * `animate`: the resting props; change them any time and the box animates there.
/// * `while_hover` / `while_tap`: layered on top of `animate` while hovered / pressed (offsets
///   add, scale and opacity multiply).
///
/// Children stay fully interactive: translation and scale are applied with a layer transform
/// that egui also uses for hit-testing. Rotation is visual only.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_motion::{MotionBox, Props, Transition};
/// MotionBox::new(ui.scope_id().with("card"))
///     .initial(Props::new().opacity(0.0).y(20.0).scale(0.95))
///     .animate(Props::IDENTITY)
///     .while_hover(Props::new().scale(1.04).y(-3.0))
///     .while_tap(Props::new().scale(0.96))
///     .transition(Transition::BOUNCY)
///     .show(ui, |ui| {
///         ui.label("Hover me");
///     });
/// ```
#[must_use = "call `.show(ui, ...)`"]
#[derive(Clone, Debug)]
pub struct MotionBox {
    id: Id,
    initial: Option<Props>,
    animate: Props,
    hover: Option<Props>,
    tap: Option<Props>,
    transition: Transition,
    gesture_transition: Option<Transition>,
    origin: Align2,
    sense: Option<Sense>,
}

impl MotionBox {
    /// A motion container whose state is stored under `id`.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            initial: None,
            animate: Props::IDENTITY,
            hover: None,
            tap: None,
            transition: Transition::SPRING,
            gesture_transition: None,
            origin: Align2::CENTER_CENTER,
            sense: None,
        }
    }
    /// Props on the very first frame (defaults to `animate`, i.e. no enter animation).
    pub fn initial(mut self, p: Props) -> Self {
        self.initial = Some(p);
        self
    }
    /// Target props.
    pub fn animate(mut self, p: Props) -> Self {
        self.animate = p;
        self
    }
    /// Props layered on top while the pointer hovers the box.
    pub fn while_hover(mut self, p: Props) -> Self {
        self.hover = Some(p);
        self
    }
    /// Props layered on top while the box is pressed.
    pub fn while_tap(mut self, p: Props) -> Self {
        self.tap = Some(p);
        self
    }
    /// Transition used for `initial` → `animate` and for changes of `animate`.
    pub fn transition(mut self, t: Transition) -> Self {
        self.transition = t;
        self
    }
    /// Transition used when hover / tap states change (defaults to [`Transition::SNAPPY`]).
    pub fn gesture_transition(mut self, t: Transition) -> Self {
        self.gesture_transition = Some(t);
        self
    }
    /// Transform origin for scale and rotation (default: center).
    pub fn origin(mut self, origin: Align2) -> Self {
        self.origin = origin;
        self
    }
    /// What the container itself senses (defaults to hover, or click when `while_tap` is set).
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = Some(sense);
        self
    }

    /// Jump the box stored under `id` back to `props`, e.g. to replay an enter animation.
    pub fn reset(ui: &Ui, id: Id, props: Props) {
        with_motion_value(ui, id.with("glowkit_box"), props, |mv| mv.set(props));
    }

    /// Show the contents.
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> MotionResponse<R> {
        let gkey = self.id.with("glowkit_box_gestures");
        let mut g: Gestures = ui.data(|d| d.get_temp(gkey)).unwrap_or(Gestures {
            hovered: false,
            pressed: false,
            last_animate: self.animate,
            gesture_cause: false,
        });
        if g.last_animate != self.animate {
            g.last_animate = self.animate;
            g.gesture_cause = false;
        }
        let mut target = self.animate;
        if let (true, Some(h)) = (g.hovered, self.hover) {
            target = target.then(h);
        }
        if let (true, Some(t)) = (g.pressed, self.tap) {
            target = target.then(t);
        }
        let transition =
            if g.gesture_cause { self.gesture_transition.unwrap_or(Transition::SNAPPY) } else { self.transition };
        let props =
            animate_from(ui, self.id.with("glowkit_box"), self.initial.unwrap_or(self.animate), target, transition);
        let sense = self.sense.unwrap_or(if self.tap.is_some() { Sense::click() } else { Sense::hover() });
        let ir =
            layered(ui, self.id, LayerSpec { props, origin: self.origin, sense, space: Space::Normal }, add_contents);
        let resp = ir.response;
        let (hovered, pressed) = (resp.contains_pointer(), resp.is_pointer_button_down_on());
        if (hovered != g.hovered && self.hover.is_some()) || (pressed != g.pressed && self.tap.is_some()) {
            g.gesture_cause = true;
            ui.ctx().request_repaint();
        }
        g.hovered = hovered;
        g.pressed = pressed;
        ui.data_mut(|d| d.insert_temp(gkey, g));
        MotionResponse { inner: ir.inner, response: resp, props }
    }
}
