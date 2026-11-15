//! Animation math, copied verbatim from `glowkit_core::anim` (31 easings, springs, stagger,
//! exponential follow), plus small adapters that drive them from GPUI:
//!
//! * [`Easing::curve`] turns an easing into the `Fn(f32) -> f32` that
//!   [`gpui::Animation::with_easing`] expects, for declarative `with_animation` tweens;
//! * [`FrameClock`] measures per-frame `dt` for physics (springs) that live in a
//!   `window.use_keyed_state(..)` entity and keep requesting animation frames until settled.

use std::f32::consts::PI;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Every classic easing curve, plus spring-like `Back` and `Elastic` variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Easing {
    Linear,
    InQuad,
    OutQuad,
    InOutQuad,
    InCubic,
    #[default]
    OutCubic,
    InOutCubic,
    InQuart,
    OutQuart,
    InOutQuart,
    InQuint,
    OutQuint,
    InOutQuint,
    InSine,
    OutSine,
    InOutSine,
    InExpo,
    OutExpo,
    InOutExpo,
    InCirc,
    OutCirc,
    InOutCirc,
    InBack,
    OutBack,
    InOutBack,
    InElastic,
    OutElastic,
    InOutElastic,
    InBounce,
    OutBounce,
    InOutBounce,
}

impl Easing {
    /// All easings, handy for pickers and demos.
    pub const ALL: [Easing; 31] = [
        Easing::Linear,
        Easing::InQuad,
        Easing::OutQuad,
        Easing::InOutQuad,
        Easing::InCubic,
        Easing::OutCubic,
        Easing::InOutCubic,
        Easing::InQuart,
        Easing::OutQuart,
        Easing::InOutQuart,
        Easing::InQuint,
        Easing::OutQuint,
        Easing::InOutQuint,
        Easing::InSine,
        Easing::OutSine,
        Easing::InOutSine,
        Easing::InExpo,
        Easing::OutExpo,
        Easing::InOutExpo,
        Easing::InCirc,
        Easing::OutCirc,
        Easing::InOutCirc,
        Easing::InBack,
        Easing::OutBack,
        Easing::InOutBack,
        Easing::InElastic,
        Easing::OutElastic,
        Easing::InOutElastic,
        Easing::InBounce,
        Easing::OutBounce,
        Easing::InOutBounce,
    ];

    /// Apply the curve to `t` (clamped to `0..=1`).
    pub fn apply(self, t: f32) -> f32 {
        ease(self, t)
    }
}

fn out_bounce(t: f32) -> f32 {
    const N: f32 = 7.5625;
    const D: f32 = 2.75;
    if t < 1.0 / D {
        N * t * t
    } else if t < 2.0 / D {
        let t = t - 1.5 / D;
        N * t * t + 0.75
    } else if t < 2.5 / D {
        let t = t - 2.25 / D;
        N * t * t + 0.9375
    } else {
        let t = t - 2.625 / D;
        N * t * t + 0.984375
    }
}

/// Evaluate an easing curve at `t` (clamped to `0..=1`).
pub fn ease(e: Easing, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    const C1: f32 = 1.70158;
    const C2: f32 = C1 * 1.525;
    const C3: f32 = C1 + 1.0;
    const C4: f32 = (2.0 * PI) / 3.0;
    const C5: f32 = (2.0 * PI) / 4.5;
    match e {
        Easing::Linear => t,
        Easing::InQuad => t * t,
        Easing::OutQuad => 1.0 - (1.0 - t) * (1.0 - t),
        Easing::InOutQuad => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }
        Easing::InCubic => t * t * t,
        Easing::OutCubic => 1.0 - (1.0 - t).powi(3),
        Easing::InOutCubic => {
            if t < 0.5 {
                4.0 * t * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
            }
        }
        Easing::InQuart => t.powi(4),
        Easing::OutQuart => 1.0 - (1.0 - t).powi(4),
        Easing::InOutQuart => {
            if t < 0.5 {
                8.0 * t.powi(4)
            } else {
                1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
            }
        }
        Easing::InQuint => t.powi(5),
        Easing::OutQuint => 1.0 - (1.0 - t).powi(5),
        Easing::InOutQuint => {
            if t < 0.5 {
                16.0 * t.powi(5)
            } else {
                1.0 - (-2.0 * t + 2.0).powi(5) / 2.0
            }
        }
        Easing::InSine => 1.0 - (t * PI / 2.0).cos(),
        Easing::OutSine => (t * PI / 2.0).sin(),
        Easing::InOutSine => -((PI * t).cos() - 1.0) / 2.0,
        Easing::InExpo => {
            if t == 0.0 {
                0.0
            } else {
                2f32.powf(10.0 * t - 10.0)
            }
        }
        Easing::OutExpo => {
            if t == 1.0 {
                1.0
            } else {
                1.0 - 2f32.powf(-10.0 * t)
            }
        }
        Easing::InOutExpo => {
            if t == 0.0 || t == 1.0 {
                t
            } else if t < 0.5 {
                2f32.powf(20.0 * t - 10.0) / 2.0
            } else {
                (2.0 - 2f32.powf(-20.0 * t + 10.0)) / 2.0
            }
        }
        Easing::InCirc => 1.0 - (1.0 - t * t).sqrt(),
        Easing::OutCirc => (1.0 - (t - 1.0).powi(2)).sqrt(),
        Easing::InOutCirc => {
            if t < 0.5 {
                (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0
            } else {
                ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0
            }
        }
        Easing::InBack => C3 * t * t * t - C1 * t * t,
        Easing::OutBack => 1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2),
        Easing::InOutBack => {
            if t < 0.5 {
                ((2.0 * t).powi(2) * ((C2 + 1.0) * 2.0 * t - C2)) / 2.0
            } else {
                ((2.0 * t - 2.0).powi(2) * ((C2 + 1.0) * (t * 2.0 - 2.0) + C2) + 2.0) / 2.0
            }
        }
        Easing::InElastic => {
            if t == 0.0 || t == 1.0 {
                t
            } else {
                -(2f32.powf(10.0 * t - 10.0)) * ((t * 10.0 - 10.75) * C4).sin()
            }
        }
        Easing::OutElastic => {
            if t == 0.0 || t == 1.0 {
                t
            } else {
                2f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * C4).sin() + 1.0
            }
        }
        Easing::InOutElastic => {
            if t == 0.0 || t == 1.0 {
                t
            } else if t < 0.5 {
                -(2f32.powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * C5).sin()) / 2.0
            } else {
                (2f32.powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * C5).sin()) / 2.0 + 1.0
            }
        }
        Easing::InBounce => 1.0 - out_bounce(1.0 - t),
        Easing::OutBounce => out_bounce(t),
        Easing::InOutBounce => {
            if t < 0.5 {
                (1.0 - out_bounce(1.0 - 2.0 * t)) / 2.0
            } else {
                (1.0 + out_bounce(2.0 * t - 1.0)) / 2.0
            }
        }
    }
}

/// Linear interpolation.
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Inverse lerp, clamped to `0..=1`.
#[inline]
pub fn unlerp(a: f32, b: f32, v: f32) -> f32 {
    if (b - a).abs() < f32::EPSILON {
        0.0
    } else {
        ((v - a) / (b - a)).clamp(0.0, 1.0)
    }
}

/// Hermite smoothstep.
#[inline]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = unlerp(edge0, edge1, x);
    t * t * (3.0 - 2.0 * t)
}

/// Frame-rate independent exponential approach of `current` toward `target`.
///
/// `speed` is roughly "how many e-foldings per second" (10 feels snappy, 4 feels soft).
#[inline]
pub fn approach(current: f32, target: f32, speed: f32, dt: f32) -> f32 {
    target + (current - target) * (-speed * dt).exp()
}

/// Stagger helper: progress (0..1) of item `index` out of a group animating over `duration`
/// seconds, where each item starts `delay` seconds after the previous one.
pub fn stagger(elapsed: f32, index: usize, delay: f32, duration: f32) -> f32 {
    ((elapsed - index as f32 * delay) / duration.max(1e-4)).clamp(0.0, 1.0)
}

/// A critically-damped (or bouncy) spring, the same model framer-motion style libraries use.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    pub value: f32,
    pub velocity: f32,
    pub target: f32,
    /// Higher = snappier. Typical: 100..400.
    pub stiffness: f32,
    /// Higher = less bounce. Typical: 10..30.
    pub damping: f32,
    pub mass: f32,
}

impl Default for Spring {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl Spring {
    pub fn new(value: f32) -> Self {
        Self { value, velocity: 0.0, target: value, stiffness: 170.0, damping: 26.0, mass: 1.0 }
    }

    /// A playful spring with visible overshoot.
    pub fn bouncy(value: f32) -> Self {
        Self { stiffness: 300.0, damping: 12.0, ..Self::new(value) }
    }

    /// A soft spring with no overshoot.
    pub fn gentle(value: f32) -> Self {
        Self { stiffness: 120.0, damping: 22.0, ..Self::new(value) }
    }

    pub fn with(mut self, stiffness: f32, damping: f32) -> Self {
        self.stiffness = stiffness;
        self.damping = damping;
        self
    }

    /// Advance the simulation by `dt` seconds (sub-stepped for stability).
    pub fn step(&mut self, dt: f32) -> f32 {
        let steps = ((dt / (1.0 / 240.0)).ceil() as usize).clamp(1, 16);
        let h = dt / steps as f32;
        for _ in 0..steps {
            let force = -self.stiffness * (self.value - self.target) - self.damping * self.velocity;
            self.velocity += force / self.mass * h;
            self.value += self.velocity * h;
        }
        self.value
    }

    pub fn is_settled(&self) -> bool {
        (self.value - self.target).abs() < 1e-3 && self.velocity.abs() < 1e-3
    }
}

impl Easing {
    /// This easing as a closure for [`gpui::Animation::with_easing`].
    pub fn curve(self) -> impl Fn(f32) -> f32 + 'static {
        move |t| ease(self, t)
    }
}

/// Seconds since the first call (a process-wide animation clock, like `glowkit_core::time`).
pub fn now() -> f32 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f32()
}

/// `Duration` from seconds (clamped to a sane minimum).
pub fn secs(s: f32) -> Duration {
    Duration::from_secs_f32(s.max(0.016))
}

/// Measures the time between renders so springs advance in real time.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameClock {
    last: Option<Instant>,
}

impl FrameClock {
    /// Seconds since the previous tick (first tick: one 60 Hz frame), clamped to 1/20 s so a
    /// stalled window doesn't make springs explode.
    pub fn tick(&mut self) -> f32 {
        let now = Instant::now();
        let dt = self.last.map(|l| (now - l).as_secs_f32()).unwrap_or(1.0 / 60.0);
        self.last = Some(now);
        dt.clamp(0.0, 1.0 / 20.0)
    }
}

/// Two springs for a 2D offset.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spring2 {
    pub x: Spring,
    pub y: Spring,
}

impl Spring2 {
    pub fn new(stiffness: f32, damping: f32) -> Self {
        Self { x: Spring::new(0.0).with(stiffness, damping), y: Spring::new(0.0).with(stiffness, damping) }
    }
    pub fn set_target(&mut self, x: f32, y: f32) {
        self.x.target = x;
        self.y.target = y;
    }
    pub fn step(&mut self, dt: f32) -> (f32, f32) {
        (self.x.step(dt), self.y.step(dt))
    }
    pub fn is_settled(&self) -> bool {
        self.x.is_settled() && self.y.is_settled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easings_hit_endpoints() {
        for e in Easing::ALL {
            assert!((ease(e, 0.0)).abs() < 1e-3, "{e:?} at 0");
            assert!((ease(e, 1.0) - 1.0).abs() < 1e-3, "{e:?} at 1");
        }
    }

    #[test]
    fn spring_converges() {
        let mut s = Spring::new(0.0);
        s.target = 1.0;
        for _ in 0..600 {
            s.step(1.0 / 60.0);
        }
        assert!(s.is_settled());
        assert!((s.value - 1.0).abs() < 1e-3);
    }

    #[test]
    fn stagger_progress() {
        assert_eq!(stagger(0.0, 0, 0.1, 1.0), 0.0);
        assert_eq!(stagger(2.0, 3, 0.1, 1.0), 1.0);
        assert!((stagger(0.6, 1, 0.1, 1.0) - 0.5).abs() < 1e-5);
    }
}
