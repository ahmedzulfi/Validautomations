//! Frame-driven animation hooks: springs, clocks and tweens.
//!
//! Continuous, non-interactive motion (shimmer sweeps, marquees, beams) is plain CSS keyframes
//! in `glowkit.css`. Interactive motion that has to follow the pointer or a changing value uses
//! these hooks, which step glowkit's spring model once per animation frame
//! (`requestAnimationFrame` on the web, a ~60 Hz timer on desktop) and go idle when settled.

use dioxus::prelude::*;
use futures_util::StreamExt;
use glowkit_tokens::{ease, Easing, Spring};

/// Wait for the next animation frame. Returns a monotonic timestamp in milliseconds.
pub async fn next_frame() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{closure::Closure, JsCast};
        let (tx, rx) = futures_channel::oneshot::channel::<f64>();
        let cb = Closure::once_into_js(move |ts: f64| {
            let _ = tx.send(ts);
        });
        if let Some(w) = web_sys::window() {
            let _ = w.request_animation_frame(cb.unchecked_ref());
        }
        rx.await.unwrap_or_else(|_| now_ms())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
        now_ms()
    }
}

/// Wait `ms` milliseconds (a `setTimeout` on the web, a tokio timer on desktop).
pub async fn sleep_ms(ms: f64) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{closure::Closure, JsCast};
        let (tx, rx) = futures_channel::oneshot::channel::<()>();
        let cb = Closure::once_into_js(move || {
            let _ = tx.send(());
        });
        if let Some(w) = web_sys::window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms.max(0.0) as i32);
        }
        let _ = rx.await;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::sleep(std::time::Duration::from_secs_f64(ms.max(0.0) / 1000.0)).await;
    }
}

/// Monotonic clock in milliseconds.
pub fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window().and_then(|w| w.performance()).map(|p| p.now()).unwrap_or_else(js_sys::Date::now)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::OnceLock;
        static START: OnceLock<std::time::Instant> = OnceLock::new();
        START.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() * 1000.0
    }
}

/// Spring tuning: `stiffness` (higher = snappier, 100..400) and `damping` (higher = less bounce).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringConfig {
    pub stiffness: f32,
    pub damping: f32,
}

impl SpringConfig {
    /// glowkit's default spring (170 / 26).
    pub const DEFAULT: Self = Self { stiffness: 170.0, damping: 26.0 };
    /// Snappy highlight spring (380 / 30), used by tabs.
    pub const SNAPPY: Self = Self { stiffness: 380.0, damping: 30.0 };
    /// Playful spring with overshoot (300 / 12).
    pub const BOUNCY: Self = Self { stiffness: 300.0, damping: 12.0 };
    /// Soft spring without overshoot (120 / 22).
    pub const GENTLE: Self = Self { stiffness: 120.0, damping: 22.0 };

    pub const fn new(stiffness: f32, damping: f32) -> Self {
        Self { stiffness, damping }
    }
}

impl Default for SpringConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A set of springs animated together. `Copy`, so it can be moved into any event handler.
///
/// Read values with [`Springs::get`] (reactive: the component re-renders while they move) and
/// steer them with [`Springs::set`].
#[derive(Clone, Copy, PartialEq)]
pub struct Springs {
    sims: CopyValue<Vec<Spring>>,
    values: Signal<Vec<f32>>,
    wake: Coroutine<()>,
}

impl Springs {
    /// Current value of spring `i` (0 if out of range). Subscribes the caller.
    pub fn get(&self, i: usize) -> f32 {
        self.values.read().get(i).copied().unwrap_or(0.0)
    }
    /// All current values. Subscribes the caller.
    pub fn values(&self) -> Vec<f32> {
        self.values.read().clone()
    }
    /// Current target of spring `i`.
    pub fn target(&self, i: usize) -> f32 {
        self.sims.read().get(i).map(|s| s.target).unwrap_or(0.0)
    }
    /// Animate spring `i` toward `target`.
    pub fn set(&self, i: usize, target: f32) {
        let mut sims = self.sims;
        let changed = {
            let mut w = sims.write();
            match w.get_mut(i) {
                Some(s) if (s.target - target).abs() > f32::EPSILON => {
                    s.target = target;
                    true
                }
                _ => false,
            }
        };
        if changed {
            self.wake.send(());
        }
    }
    /// Animate every spring toward `targets` (extra springs are created or dropped as needed).
    pub fn set_all(&self, targets: &[f32]) {
        let mut sims = self.sims;
        let cfg = sims.read().first().map(|s| (s.stiffness, s.damping)).unwrap_or((170.0, 26.0));
        let resized = sims.read().len() != targets.len();
        if resized {
            sims.write().resize_with(targets.len(), || Spring::new(0.0).with(cfg.0, cfg.1));
            let mut values = self.values;
            values.write().resize(targets.len(), 0.0);
        }
        for (i, t) in targets.iter().enumerate() {
            self.set(i, *t);
        }
    }
    /// Jump spring `i` to `value` without animating.
    pub fn snap(&self, i: usize, value: f32) {
        let mut sims = self.sims;
        if let Some(s) = sims.write().get_mut(i) {
            s.value = value;
            s.target = value;
            s.velocity = 0.0;
        }
        let mut values = self.values;
        let mut w = values.write();
        if let Some(v) = w.get_mut(i) {
            *v = value;
        }
    }
    /// Give spring `i` a kick (points per second), e.g. for a bounce.
    pub fn impulse(&self, i: usize, velocity: f32) {
        let mut sims = self.sims;
        if let Some(s) = sims.write().get_mut(i) {
            s.velocity += velocity;
        }
        self.wake.send(());
    }
}

/// `n` springs starting at `initial`, all with the same tuning. See [`Springs`].
pub fn use_springs(initial: Vec<f32>, config: SpringConfig) -> Springs {
    let sims = use_hook(|| {
        CopyValue::new(
            initial.iter().map(|v| Spring::new(*v).with(config.stiffness, config.damping)).collect::<Vec<_>>(),
        )
    });
    let values = use_signal(|| initial.clone());
    let wake = use_coroutine(move |mut rx: UnboundedReceiver<()>| async move {
        let mut values = values;
        while rx.next().await.is_some() {
            let mut last = now_ms();
            loop {
                let now = next_frame().await;
                // drain extra wake-ups that arrived while animating
                while rx.try_recv().is_ok() {}
                let dt = (((now - last) / 1000.0) as f32).clamp(0.0, 1.0 / 20.0);
                last = now;
                let (out, settled) = {
                    let mut sims = sims;
                    let mut w = sims.write();
                    let mut settled = true;
                    for s in w.iter_mut() {
                        s.step(dt);
                        if s.is_settled() {
                            s.value = s.target;
                            s.velocity = 0.0;
                        } else {
                            settled = false;
                        }
                    }
                    (w.iter().map(|s| s.value).collect::<Vec<_>>(), settled)
                };
                values.set(out);
                if settled {
                    break;
                }
            }
        }
    });
    Springs { sims, values, wake }
}

/// A single spring. Read with `.get()`, steer with `.set(target)`.
#[derive(Clone, Copy, PartialEq)]
pub struct SpringValue(pub Springs);

impl SpringValue {
    pub fn get(&self) -> f32 {
        self.0.get(0)
    }
    pub fn set(&self, target: f32) {
        self.0.set(0, target)
    }
    pub fn target(&self) -> f32 {
        self.0.target(0)
    }
    pub fn snap(&self, value: f32) {
        self.0.snap(0, value)
    }
    pub fn impulse(&self, velocity: f32) {
        self.0.impulse(0, velocity)
    }
}

/// One spring starting at `initial`; the glowkit equivalent of `glowkit_core::anim::spring`.
///
/// ```ignore
/// let x = use_spring(0.0, SpringConfig::BOUNCY);
/// rsx! { div { onclick: move |_| x.set(120.0), style: "transform: translateX({x.get()}px)" } }
/// ```
pub fn use_spring(initial: f32, config: SpringConfig) -> SpringValue {
    SpringValue(use_springs(vec![initial], config))
}

/// Seconds since the component mounted, updated every animation frame while `running`.
/// Use for timeline-driven effects (typewriter, streaming text). Re-renders the caller each frame
/// while running, so keep the subtree small.
pub fn use_clock(running: bool) -> Signal<f64> {
    let mut elapsed = use_signal(|| 0.0f64);
    let mut run = use_signal(|| running);
    if *run.peek() != running {
        run.set(running);
    }
    use_future(move || async move {
        let mut last = now_ms();
        loop {
            let now = next_frame().await;
            let dt = (now - last).clamp(0.0, 100.0) / 1000.0;
            last = now;
            if *run.peek() {
                elapsed += dt;
            }
        }
    });
    elapsed
}

/// Tween a number from its current value to `target` over `duration` seconds with `easing`.
/// When `target` changes the tween restarts from wherever it is. Returns the animated value.
pub fn use_tween(from: f64, target: f64, duration: f32, easing: Easing, delay: f32) -> Signal<f64> {
    let mut shown = use_signal(|| from);
    let mut goal = use_signal(|| target);
    let mut gen = use_signal(|| 0u64);
    if *goal.peek() != target {
        goal.set(target);
        gen += 1;
    }
    let generation = gen();
    use_effect(move || {
        let my = gen();
        let start_val = *shown.peek();
        let end = *goal.peek();
        spawn(async move {
            let t0 = now_ms() + delay as f64 * 1000.0;
            loop {
                let now = next_frame().await;
                if *gen.peek() != my {
                    break;
                }
                let t = ((now - t0) / 1000.0 / duration.max(1e-3) as f64).clamp(0.0, 1.0) as f32;
                let k = ease(easing, t) as f64;
                shown.set(start_val + (end - start_val) * k);
                if t >= 1.0 {
                    break;
                }
            }
        });
    });
    let _ = generation;
    shown
}

/// Replays a CSS-driven animation according to a glowkit [`Trigger`]: render the animated
/// subtree with `key: "{replay.key()}"` so it remounts (and restarts its keyframes) whenever
/// the trigger fires. Wire [`Replay::hover`] to `onmouseenter` and [`Replay::click`] to
/// `onclick`; `Trigger::Loop` replays every `cycle` seconds plus the pause.
#[derive(Clone, Copy, PartialEq)]
pub struct Replay {
    gen: Signal<u64>,
    trigger: Signal<glowkit_tokens::Trigger>,
}

impl Replay {
    /// Changes every time the animation should restart.
    pub fn key(&self) -> u64 {
        (self.gen)()
    }
    /// Call from `onmouseenter`.
    pub fn hover(&self) {
        if *self.trigger.peek() == glowkit_tokens::Trigger::OnHover {
            let mut g = self.gen;
            g += 1;
        }
    }
    /// Call from `onclick`.
    pub fn click(&self) {
        if *self.trigger.peek() == glowkit_tokens::Trigger::OnClick {
            let mut g = self.gen;
            g += 1;
        }
    }
    /// Restart now, regardless of the trigger.
    pub fn restart(&self) {
        let mut g = self.gen;
        g += 1;
    }
}

/// See [`Replay`].
pub fn use_replay(trigger: glowkit_tokens::Trigger, cycle: f32) -> Replay {
    let mut trig = use_signal(|| trigger);
    if *trig.peek() != trigger {
        trig.set(trigger);
    }
    let mut cyc = use_signal(|| cycle);
    if *cyc.peek() != cycle {
        cyc.set(cycle);
    }
    let mut gen = use_signal(|| 0u64);
    use_future(move || async move {
        loop {
            match *trig.peek() {
                glowkit_tokens::Trigger::Loop { pause_ms } => {
                    sleep_ms(*cyc.peek() as f64 * 1000.0 + pause_ms as f64).await;
                    if matches!(*trig.peek(), glowkit_tokens::Trigger::Loop { .. }) {
                        gen += 1;
                    }
                }
                _ => sleep_ms(1000.0).await,
            }
        }
    });
    Replay { gen, trigger: trig }
}
