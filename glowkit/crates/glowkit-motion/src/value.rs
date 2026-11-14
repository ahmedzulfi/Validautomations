//! Animated values: the [`Animatable`] trait, [`Transition`]s, the [`MotionValue`] simulation and
//! the immediate-mode [`animate`] entry point.

use egui::{Color32, Id, Pos2, Rect, Ui, Vec2};
use glowkit_core::anim::{ease, Easing, Spring};
use std::marker::PhantomData;

/// Maximum number of scalar components an [`Animatable`] value can have.
pub const MAX_DIM: usize = 8;

/// The scalar components of an [`Animatable`] value (unused trailing slots are zero).
pub type Components = [f32; MAX_DIM];

/// A value that can be animated: anything that can be split into (at most [`MAX_DIM`]) `f32`
/// components and rebuilt from them.
///
/// Implemented for `f32`, [`Vec2`], [`Pos2`], [`Rect`], [`Color32`] and [`crate::Props`].
/// Springs and inertia run independently per component, which is what lets a retargeted
/// spring keep its velocity in every dimension.
///
/// ```no_run
/// use glowkit_motion::Animatable;
/// #[derive(Clone, Copy)]
/// struct Hsl { h: f32, s: f32, l: f32 }
/// impl Animatable for Hsl {
///     const DIM: usize = 3;
///     fn to_components(self) -> glowkit_motion::Components {
///         let mut c = [0.0; 8];
///         c[..3].copy_from_slice(&[self.h, self.s, self.l]);
///         c
///     }
///     fn from_components(c: &glowkit_motion::Components) -> Self { Hsl { h: c[0], s: c[1], l: c[2] } }
/// }
/// ```
pub trait Animatable: Copy + Send + Sync + 'static {
    /// How many components are meaningful.
    const DIM: usize;
    /// Split into components.
    fn to_components(self) -> Components;
    /// Rebuild from components (clamp here if the type has a valid range, e.g. colors).
    fn from_components(c: &Components) -> Self;
    /// Linear interpolation between `a` and `b` (`t` may exceed `0..=1` for overshooting easings).
    fn lerp(a: Self, b: Self, t: f32) -> Self {
        let (ca, cb) = (a.to_components(), b.to_components());
        let mut out = [0.0; MAX_DIM];
        for i in 0..Self::DIM {
            out[i] = ca[i] + (cb[i] - ca[i]) * t;
        }
        Self::from_components(&out)
    }
}

fn comps(v: &[f32]) -> Components {
    let mut c = [0.0; MAX_DIM];
    c[..v.len()].copy_from_slice(v);
    c
}

impl Animatable for f32 {
    const DIM: usize = 1;
    fn to_components(self) -> Components {
        comps(&[self])
    }
    fn from_components(c: &Components) -> Self {
        c[0]
    }
}

impl Animatable for Vec2 {
    const DIM: usize = 2;
    fn to_components(self) -> Components {
        comps(&[self.x, self.y])
    }
    fn from_components(c: &Components) -> Self {
        Vec2::new(c[0], c[1])
    }
}

impl Animatable for Pos2 {
    const DIM: usize = 2;
    fn to_components(self) -> Components {
        comps(&[self.x, self.y])
    }
    fn from_components(c: &Components) -> Self {
        Pos2::new(c[0], c[1])
    }
}

impl Animatable for Rect {
    const DIM: usize = 4;
    fn to_components(self) -> Components {
        comps(&[self.min.x, self.min.y, self.max.x, self.max.y])
    }
    fn from_components(c: &Components) -> Self {
        Rect::from_min_max(Pos2::new(c[0], c[1]), Pos2::new(c[2].max(c[0]), c[3].max(c[1])))
    }
}

/// Colors animate in unmultiplied sRGBA space (0..255 per channel), clamped on output.
impl Animatable for Color32 {
    const DIM: usize = 4;
    fn to_components(self) -> Components {
        let [r, g, b, a] = self.to_srgba_unmultiplied();
        comps(&[r as f32, g as f32, b as f32, a as f32])
    }
    fn from_components(c: &Components) -> Self {
        let q = |v: f32| v.round().clamp(0.0, 255.0) as u8;
        Color32::from_rgba_unmultiplied(q(c[0]), q(c[1]), q(c[2]), q(c[3]))
    }
}

/// How a value travels to a new target, mirroring the transition types of web motion libraries.
///
/// ```no_run
/// use glowkit_motion::Transition;
/// use glowkit_core::Easing;
/// let a = Transition::tween(0.4).ease(Easing::OutBack).delay(0.1);
/// let b = Transition::spring(300.0, 18.0);
/// let c = Transition::inertia(0.0).decay(0.35).bounds(0.0, 200.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Transition {
    /// Time-based interpolation with an easing curve.
    Tween {
        /// Seconds from start to end.
        duration: f32,
        /// Easing curve applied to the progress.
        easing: Easing,
        /// Seconds to wait before starting.
        delay: f32,
    },
    /// Physical spring. Velocity is preserved when the target changes mid-flight.
    Spring {
        /// Higher = snappier (typical 100..500).
        stiffness: f32,
        /// Higher = less bounce (typical 10..40).
        damping: f32,
        /// Heavier = slower and more momentum.
        mass: f32,
        /// Seconds to wait before starting.
        delay: f32,
    },
    /// Momentum glide that decelerates exponentially, like a flicked scroll view.
    ///
    /// The value is launched with its current velocity plus `velocity` units/s toward the
    /// target, and comes to rest wherever momentum takes it. With `velocity == 0` (and no carried
    /// velocity) the launch speed is chosen so it lands exactly on the target. When it leaves
    /// `bounds` it springs back to the nearest bound.
    Inertia {
        /// Extra launch speed (units/s) toward the target.
        velocity: f32,
        /// Deceleration time constant in seconds (0.2 = short glide, 0.8 = long glide).
        decay: f32,
        /// Optional `(min, max)` limits applied to every component.
        bounds: Option<(f32, f32)>,
        /// Seconds to wait before starting.
        delay: f32,
    },
    /// Jump to the target immediately.
    Instant,
}

impl Default for Transition {
    fn default() -> Self {
        Self::SPRING
    }
}

impl Transition {
    /// The default, a responsive spring with a hint of overshoot.
    pub const SPRING: Self = Self::spring(170.0, 22.0);
    /// A snappy, critically damped spring for UI feedback (hover, tap).
    pub const SNAPPY: Self = Self::spring(420.0, 34.0);
    /// A playful spring with visible bounce.
    pub const BOUNCY: Self = Self::spring(280.0, 11.0);
    /// A soft, slow spring with no overshoot.
    pub const GENTLE: Self = Self::spring(90.0, 19.0);
    /// A 0.3 s ease-out tween.
    pub const SMOOTH: Self = Self::tween(0.3);

    /// A tween of `duration` seconds with an ease-out-cubic curve.
    pub const fn tween(duration: f32) -> Self {
        Self::Tween { duration, easing: Easing::OutCubic, delay: 0.0 }
    }

    /// A spring with mass 1.
    pub const fn spring(stiffness: f32, damping: f32) -> Self {
        Self::Spring { stiffness, damping, mass: 1.0, delay: 0.0 }
    }

    /// An inertia glide with extra launch `velocity` and a 0.35 s decay.
    pub const fn inertia(velocity: f32) -> Self {
        Self::Inertia { velocity, decay: 0.35, bounds: None, delay: 0.0 }
    }

    /// Set the easing (tweens only).
    pub fn ease(mut self, e: Easing) -> Self {
        if let Self::Tween { easing, .. } = &mut self {
            *easing = e;
        }
        self
    }

    /// Set the duration (tweens only).
    pub fn duration(mut self, seconds: f32) -> Self {
        if let Self::Tween { duration, .. } = &mut self {
            *duration = seconds.max(0.0);
        }
        self
    }

    /// Set the mass (springs only).
    pub fn mass(mut self, m: f32) -> Self {
        if let Self::Spring { mass, .. } = &mut self {
            *mass = m.max(0.01);
        }
        self
    }

    /// Set the deceleration time constant (inertia only).
    pub fn decay(mut self, seconds: f32) -> Self {
        if let Self::Inertia { decay, .. } = &mut self {
            *decay = seconds.max(0.01);
        }
        self
    }

    /// Set the `(min, max)` bounds (inertia only).
    pub fn bounds(mut self, min: f32, max: f32) -> Self {
        if let Self::Inertia { bounds, .. } = &mut self {
            *bounds = Some((min.min(max), max.max(min)));
        }
        self
    }

    /// Wait `seconds` before starting.
    pub fn delay(mut self, seconds: f32) -> Self {
        match &mut self {
            Self::Tween { delay, .. } | Self::Spring { delay, .. } | Self::Inertia { delay, .. } => {
                *delay = seconds.max(0.0)
            }
            Self::Instant => {}
        }
        self
    }

    /// Add `seconds` to the existing delay.
    pub fn add_delay(self, seconds: f32) -> Self {
        let d = self.get_delay();
        self.delay(d + seconds)
    }

    /// The configured delay in seconds.
    pub fn get_delay(&self) -> f32 {
        match self {
            Self::Tween { delay, .. } | Self::Spring { delay, .. } | Self::Inertia { delay, .. } => *delay,
            Self::Instant => 0.0,
        }
    }
}

/// Spring used to pull an inertia glide back inside its bounds.
const BOUNCE_STIFFNESS: f32 = 500.0;
const BOUNCE_DAMPING: f32 = 40.0;

/// A value plus the physics state needed to animate it (current value, velocity, target).
///
/// This is the pure, UI-independent core of [`animate`]: drive it yourself with
/// [`MotionValue::update`] when you own the state, e.g. in a struct of your app.
///
/// ```no_run
/// use glowkit_motion::{MotionValue, Transition};
/// let mut x = MotionValue::new(0.0_f32);
/// x.animate_to(100.0, Transition::spring(200.0, 20.0));
/// let v = x.update(1.0 / 60.0); // advance one frame
/// ```
#[derive(Clone, Copy, Debug)]
pub struct MotionValue<T: Animatable> {
    current: Components,
    velocity: Components,
    from: Components,
    target: Components,
    lo: Components,
    hi: Components,
    transition: Transition,
    elapsed: f32,
    animating: bool,
    _t: PhantomData<T>,
}

impl<T: Animatable> MotionValue<T> {
    /// A value at rest.
    pub fn new(value: T) -> Self {
        let c = value.to_components();
        Self {
            current: c,
            velocity: [0.0; MAX_DIM],
            from: c,
            target: c,
            lo: [f32::NEG_INFINITY; MAX_DIM],
            hi: [f32::INFINITY; MAX_DIM],
            transition: Transition::default(),
            elapsed: 0.0,
            animating: false,
            _t: PhantomData,
        }
    }

    /// The current value.
    pub fn get(&self) -> T {
        T::from_components(&self.current)
    }

    /// The value it is heading to (for inertia: where it was aimed).
    pub fn target(&self) -> T {
        T::from_components(&self.target)
    }

    /// Current velocity in units per second.
    pub fn velocity(&self) -> T {
        T::from_components(&self.velocity)
    }

    /// Magnitude of the velocity across all components.
    pub fn speed(&self) -> f32 {
        self.velocity[..T::DIM].iter().map(|v| v * v).sum::<f32>().sqrt()
    }

    /// Is it still moving (or waiting out a delay)?
    pub fn is_animating(&self) -> bool {
        self.animating
    }

    /// Jump to `value` and stop.
    pub fn set(&mut self, value: T) {
        let c = value.to_components();
        self.current = c;
        self.from = c;
        self.target = c;
        self.velocity = [0.0; MAX_DIM];
        self.animating = false;
    }

    /// Jump to `value` but keep tracking `velocity` (useful while dragging, so a release can
    /// hand the velocity to a spring or inertia animation).
    pub fn set_with_velocity(&mut self, value: T, velocity: T) {
        self.set(value);
        self.velocity = velocity.to_components();
    }

    /// Start animating to `target`. Calling it again with the same target is a no-op, so it is
    /// safe to call every frame. Retargeting mid-flight starts from the current value and (for
    /// springs and inertia) keeps the current velocity.
    pub fn animate_to(&mut self, target: T, transition: Transition) {
        let t = target.to_components();
        let same = (0..T::DIM).all(|i| (t[i] - self.target[i]).abs() <= 1e-5);
        if same && (self.animating || self.at_target()) {
            self.transition = transition;
            return;
        }
        self.retarget(t, transition);
    }

    /// Like [`Self::animate_to`] but always restarts, even if the target is unchanged.
    pub fn restart_to(&mut self, target: T, transition: Transition) {
        self.retarget(target.to_components(), transition);
    }

    /// Throw the value with an explicit `velocity` (units/s), decelerating with time constant
    /// `decay`, optionally kept inside `min..=max` per component (with a springy bounce).
    pub fn fling(&mut self, velocity: T, decay: f32, min: Option<T>, max: Option<T>) {
        self.velocity = velocity.to_components();
        self.lo = min.map_or([f32::NEG_INFINITY; MAX_DIM], |m| m.to_components());
        self.hi = max.map_or([f32::INFINITY; MAX_DIM], |m| m.to_components());
        self.from = self.current;
        self.transition = Transition::Inertia { velocity: 0.0, decay: decay.max(0.01), bounds: None, delay: 0.0 };
        for i in 0..T::DIM {
            self.target[i] = (self.current[i] + self.velocity[i] * decay).clamp(self.lo[i], self.hi[i]);
        }
        self.elapsed = 0.0;
        self.animating = true;
    }

    fn at_target(&self) -> bool {
        (0..T::DIM).all(|i| (self.current[i] - self.target[i]).abs() <= 1e-4)
    }

    fn retarget(&mut self, t: Components, transition: Transition) {
        self.from = self.current;
        self.target = t;
        self.transition = transition;
        self.elapsed = 0.0;
        self.animating = true;
        self.lo = [f32::NEG_INFINITY; MAX_DIM];
        self.hi = [f32::INFINITY; MAX_DIM];
        match transition {
            Transition::Tween { .. } | Transition::Instant => {}
            Transition::Spring { .. } => {}
            Transition::Inertia { velocity, decay, bounds, .. } => {
                let n = T::DIM;
                let dist: f32 = (0..n).map(|i| (t[i] - self.current[i]).powi(2)).sum::<f32>().sqrt();
                let carried = (0..n).any(|i| self.velocity[i].abs() > 1e-3);
                for i in 0..n {
                    let dir = if dist > 1e-6 { (t[i] - self.current[i]) / dist } else { 0.0 };
                    if velocity == 0.0 && !carried {
                        self.velocity[i] = (t[i] - self.current[i]) / decay;
                    } else {
                        self.velocity[i] += dir * velocity;
                    }
                    if let Some((lo, hi)) = bounds {
                        self.lo[i] = lo;
                        self.hi[i] = hi;
                    }
                    // remember the natural resting point as the "target"
                    self.target[i] = (self.current[i] + self.velocity[i] * decay).clamp(self.lo[i], self.hi[i]);
                }
            }
        }
    }

    /// Advance the simulation by `dt` seconds and return the new value.
    pub fn update(&mut self, dt: f32) -> T {
        if !self.animating {
            return self.get();
        }
        let delay = self.transition.get_delay();
        let before = self.elapsed;
        self.elapsed += dt;
        if self.elapsed < delay {
            return self.get();
        }
        // only simulate the part of this frame that is past the delay
        let dt = (self.elapsed - before.max(delay)).max(0.0);
        let n = T::DIM;
        match self.transition {
            Transition::Instant => {
                self.current = self.target;
                self.velocity = [0.0; MAX_DIM];
                self.animating = false;
            }
            Transition::Tween { duration, easing, .. } => {
                let p = if duration <= 0.0 { 1.0 } else { ((self.elapsed - delay) / duration).min(1.0) };
                let k = ease(easing, p);
                let prev = self.current;
                for i in 0..n {
                    self.current[i] = self.from[i] + (self.target[i] - self.from[i]) * k;
                    self.velocity[i] = if dt > 0.0 { (self.current[i] - prev[i]) / dt } else { 0.0 };
                }
                if p >= 1.0 {
                    self.current = self.target;
                    self.velocity = [0.0; MAX_DIM];
                    self.animating = false;
                }
            }
            Transition::Spring { stiffness, damping, mass, .. } => {
                let mut settled = true;
                for i in 0..n {
                    let mut s = Spring {
                        value: self.current[i],
                        velocity: self.velocity[i],
                        target: self.target[i],
                        stiffness,
                        damping,
                        mass,
                    };
                    s.step(dt);
                    self.current[i] = s.value;
                    self.velocity[i] = s.velocity;
                    let scale = (self.target[i] - self.from[i]).abs().max(1.0);
                    if (s.value - s.target).abs() > 1e-3 * scale || s.velocity.abs() > 1e-2 * scale {
                        settled = false;
                    }
                }
                if settled {
                    self.current = self.target;
                    self.velocity = [0.0; MAX_DIM];
                    self.animating = false;
                }
            }
            Transition::Inertia { decay, .. } => {
                let mut settled = true;
                let steps = ((dt / (1.0 / 240.0)).ceil() as usize).clamp(1, 16);
                let h = dt / steps as f32;
                for i in 0..n {
                    let (mut x, mut v) = (self.current[i], self.velocity[i]);
                    let (lo, hi) = (self.lo[i], self.hi[i]);
                    for _ in 0..steps {
                        if x < lo || x > hi {
                            let edge = x.clamp(lo, hi);
                            let f = -BOUNCE_STIFFNESS * (x - edge) - BOUNCE_DAMPING * v;
                            v += f * h;
                        } else {
                            v *= (-h / decay).exp();
                        }
                        x += v * h;
                    }
                    let inside = x >= lo - 0.01 && x <= hi + 0.01;
                    if v.abs() > 0.5 || !inside {
                        settled = false;
                    }
                    self.current[i] = x;
                    self.velocity[i] = v;
                }
                if settled {
                    for i in 0..n {
                        self.current[i] = self.current[i].clamp(self.lo[i], self.hi[i]);
                    }
                    self.target = self.current;
                    self.velocity = [0.0; MAX_DIM];
                    self.animating = false;
                }
            }
        }
        self.get()
    }
}

fn key(id: Id) -> Id {
    id.with("glowkit_motion_value")
}

/// Animate a value toward `target` with `transition`, storing the simulation under `id`.
///
/// Call it every frame with the value you want; it returns the value to draw this frame. When
/// `target` changes mid-flight the animation retargets smoothly (springs keep their velocity).
/// Repaints are requested only while moving. On the first call the value starts at `target`;
/// use [`animate_from`] to enter from somewhere else.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_motion::{animate, Transition};
/// let open = true;
/// let width: f32 = animate(ui, ui.scope_id().with("panel"), if open { 240.0 } else { 64.0 }, Transition::BOUNCY);
/// let color = animate(ui, ui.scope_id().with("tint"), if open { egui::Color32::RED } else { egui::Color32::GRAY }, Transition::tween(0.4));
/// ```
pub fn animate<T: Animatable>(ui: &Ui, id: Id, target: T, transition: Transition) -> T {
    animate_from(ui, id, target, target, transition)
}

/// Like [`animate`], but the very first frame starts at `initial` (an enter animation).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let y: f32 = glowkit_motion::animate_from(ui, ui.scope_id().with("y"), 40.0, 0.0, glowkit_motion::Transition::SPRING);
/// ```
pub fn animate_from<T: Animatable>(ui: &Ui, id: Id, initial: T, target: T, transition: Transition) -> T {
    let dt = glowkit_core::dt(ui);
    let (v, moving) = glowkit_core::with_state(
        ui,
        key(id),
        || MotionValue::new(initial),
        |mv: &mut MotionValue<T>| {
            mv.animate_to(target, transition);
            let v = mv.update(dt);
            (v, mv.is_animating())
        },
    );
    if moving {
        ui.ctx().request_repaint();
    }
    v
}

/// Read the [`MotionValue`] stored under `id` by [`animate`] (e.g. to inspect its velocity).
pub fn motion_value<T: Animatable>(ui: &Ui, id: Id) -> Option<MotionValue<T>> {
    ui.data(|d| d.get_temp::<MotionValue<T>>(key(id)))
}

/// Mutate (or create) the [`MotionValue`] stored under `id`, e.g. to [`MotionValue::set`] it
/// while dragging or [`MotionValue::fling`] it on release. Requests a repaint.
pub fn with_motion_value<T: Animatable, R>(ui: &Ui, id: Id, init: T, f: impl FnOnce(&mut MotionValue<T>) -> R) -> R {
    ui.ctx().request_repaint();
    glowkit_core::with_state(ui, key(id), || MotionValue::new(init), f)
}

/// Advance the [`MotionValue`] under `id` by one frame without retargeting, returning its value.
pub fn tick<T: Animatable>(ui: &Ui, id: Id, init: T) -> T {
    let dt = glowkit_core::dt(ui);
    let (v, moving) = glowkit_core::with_state(
        ui,
        key(id),
        || MotionValue::new(init),
        |mv: &mut MotionValue<T>| {
            let v = mv.update(dt);
            (v, mv.is_animating())
        },
    );
    if moving {
        ui.ctx().request_repaint();
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run<T: Animatable>(mv: &mut MotionValue<T>, secs: f32) {
        let n = (secs * 60.0) as usize;
        for _ in 0..n {
            mv.update(1.0 / 60.0);
        }
    }

    #[test]
    fn tween_reaches_target_and_stops() {
        let mut v = MotionValue::new(0.0f32);
        v.animate_to(10.0, Transition::tween(0.5));
        run(&mut v, 0.25);
        assert!(v.get() > 0.0 && v.get() < 10.0);
        run(&mut v, 0.4);
        assert_eq!(v.get(), 10.0);
        assert!(!v.is_animating());
    }

    #[test]
    fn delay_holds_value() {
        let mut v = MotionValue::new(0.0f32);
        v.animate_to(1.0, Transition::tween(0.2).delay(0.5));
        run(&mut v, 0.4);
        assert_eq!(v.get(), 0.0);
        run(&mut v, 0.5);
        assert_eq!(v.get(), 1.0);
    }

    #[test]
    fn spring_settles_and_keeps_velocity_on_retarget() {
        let mut v = MotionValue::new(Vec2::ZERO);
        v.animate_to(Vec2::new(100.0, 0.0), Transition::spring(200.0, 20.0));
        run(&mut v, 0.1);
        let vel = v.velocity();
        assert!(vel.x > 0.0);
        v.animate_to(Vec2::new(-100.0, 0.0), Transition::spring(200.0, 20.0));
        assert_eq!(v.velocity(), vel, "velocity preserved across retarget");
        run(&mut v, 5.0);
        assert!(!v.is_animating());
        assert!((v.get().x + 100.0).abs() < 1e-3);
    }

    #[test]
    fn same_target_is_noop() {
        let mut v = MotionValue::new(0.0f32);
        v.animate_to(1.0, Transition::tween(1.0));
        run(&mut v, 0.5);
        let mid = v.get();
        v.animate_to(1.0, Transition::tween(1.0));
        assert_eq!(v.get(), mid);
        run(&mut v, 0.1);
        assert!(v.get() > mid);
    }

    #[test]
    fn inertia_lands_on_target_without_velocity() {
        let mut v = MotionValue::new(0.0f32);
        v.animate_to(50.0, Transition::inertia(0.0).decay(0.2));
        run(&mut v, 3.0);
        assert!(!v.is_animating());
        assert!((v.get() - 50.0).abs() < 1.0, "{}", v.get());
    }

    #[test]
    fn fling_respects_bounds() {
        let mut v = MotionValue::new(0.0f32);
        v.fling(3000.0, 0.4, Some(-10.0), Some(100.0));
        run(&mut v, 4.0);
        assert!(!v.is_animating());
        assert!((v.get() - 100.0).abs() < 0.5, "{}", v.get());
    }

    #[test]
    fn color_roundtrip_and_lerp() {
        let a = Color32::from_rgb(0, 0, 0);
        let b = Color32::from_rgb(200, 100, 50);
        assert_eq!(<Color32 as Animatable>::lerp(a, b, 1.0), b);
        assert_eq!(<Color32 as Animatable>::lerp(a, b, 0.5), Color32::from_rgb(100, 50, 25));
        let r = Rect::from_min_size(Pos2::ZERO, Vec2::splat(10.0));
        assert_eq!(Rect::from_components(&r.to_components()), r);
    }
}
