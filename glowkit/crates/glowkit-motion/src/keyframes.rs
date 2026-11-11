//! Keyframe animations: a value passing through several stops, with per-segment easing,
//! repeats, yoyo / mirror and repeat delays.

use crate::value::Animatable;
use egui::{Id, Ui};
use glowkit_core::anim::{ease, Easing};

/// How many times a [`Keyframes`] animation plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Repeat {
    /// Play once.
    #[default]
    Once,
    /// Play `n` extra times after the first (so `Count(1)` plays twice in total).
    Count(u32),
    /// Loop forever.
    Infinite,
}

/// What happens on every repeat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RepeatType {
    /// Jump back to the first keyframe and play forward again.
    #[default]
    Loop,
    /// Play backward in time on every other iteration (easing curves are reversed too).
    Yoyo,
    /// Swap the order of the keyframes on every other iteration but keep easings forward.
    Mirror,
}

/// A value animated through a list of `(offset, value)` stops, offsets in `0..=1`.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_motion::{Keyframes, Repeat};
/// use glowkit_core::Easing;
/// let y = Keyframes::new([(0.0, 0.0_f32), (0.4, -60.0), (1.0, 0.0)])
///     .duration(1.2)
///     .ease(Easing::InOutSine)
///     .repeat(Repeat::Infinite)
///     .repeat_delay(0.3)
///     .play(ui, ui.scope_id().with("bounce"));
/// ```
#[derive(Clone, Debug)]
pub struct Keyframes<T: Animatable> {
    frames: Vec<(f32, T)>,
    easings: Vec<Easing>,
    duration: f32,
    delay: f32,
    repeat: Repeat,
    repeat_type: RepeatType,
    repeat_delay: f32,
}

impl<T: Animatable> Keyframes<T> {
    /// Build from `(offset, value)` pairs; offsets are sorted and clamped to `0..=1`.
    pub fn new(frames: impl IntoIterator<Item = (f32, T)>) -> Self {
        let mut frames: Vec<(f32, T)> = frames.into_iter().map(|(o, v)| (o.clamp(0.0, 1.0), v)).collect();
        frames.sort_by(|a, b| a.0.total_cmp(&b.0));
        let segs = frames.len().saturating_sub(1);
        Self {
            frames,
            easings: vec![Easing::InOutCubic; segs],
            duration: 1.0,
            delay: 0.0,
            repeat: Repeat::Once,
            repeat_type: RepeatType::Loop,
            repeat_delay: 0.0,
        }
    }

    /// Evenly spaced keyframes.
    pub fn evenly(values: impl IntoIterator<Item = T>) -> Self {
        let v: Vec<T> = values.into_iter().collect();
        let n = v.len().max(2) - 1;
        Self::new(v.into_iter().enumerate().map(|(i, x)| (i as f32 / n as f32, x)))
    }

    /// Seconds for one iteration.
    pub fn duration(mut self, seconds: f32) -> Self {
        self.duration = seconds.max(1e-3);
        self
    }
    /// Seconds before the first iteration.
    pub fn delay(mut self, seconds: f32) -> Self {
        self.delay = seconds.max(0.0);
        self
    }
    /// Use the same easing for every segment.
    pub fn ease(mut self, e: Easing) -> Self {
        self.easings.iter_mut().for_each(|x| *x = e);
        self
    }
    /// Per-segment easings (segment `i` runs from keyframe `i` to `i + 1`). Missing entries keep
    /// their current easing.
    pub fn ease_segments(mut self, easings: impl IntoIterator<Item = Easing>) -> Self {
        for (slot, e) in self.easings.iter_mut().zip(easings) {
            *slot = e;
        }
        self
    }
    /// How many times to play.
    pub fn repeat(mut self, r: Repeat) -> Self {
        self.repeat = r;
        self
    }
    /// What each repeat does (loop / yoyo / mirror).
    pub fn repeat_type(mut self, t: RepeatType) -> Self {
        self.repeat_type = t;
        self
    }
    /// Shortcut for `repeat_type(RepeatType::Yoyo)`.
    pub fn yoyo(self) -> Self {
        self.repeat_type(RepeatType::Yoyo)
    }
    /// Shortcut for `repeat_type(RepeatType::Mirror)`.
    pub fn mirror(self) -> Self {
        self.repeat_type(RepeatType::Mirror)
    }
    /// Pause between iterations, in seconds.
    pub fn repeat_delay(mut self, seconds: f32) -> Self {
        self.repeat_delay = seconds.max(0.0);
        self
    }

    /// Total running time in seconds, or `None` for infinite repeats.
    pub fn total_duration(&self) -> Option<f32> {
        let iters = match self.repeat {
            Repeat::Once => 1,
            Repeat::Count(n) => n + 1,
            Repeat::Infinite => return None,
        };
        Some(self.delay + iters as f32 * self.duration + (iters - 1) as f32 * self.repeat_delay)
    }

    /// Has the animation finished at `time` seconds?
    pub fn is_finished(&self, time: f32) -> bool {
        self.total_duration().is_some_and(|d| time >= d)
    }

    /// Value at progress `p` (`0..=1`) through a single forward iteration.
    pub fn sample_progress(&self, p: f32) -> T {
        self.sample_dir(p, false)
    }

    fn sample_dir(&self, p: f32, reversed: bool) -> T {
        let n = self.frames.len();
        if n == 0 {
            return T::from_components(&[0.0; crate::value::MAX_DIM]);
        }
        if n == 1 {
            return self.frames[0].1;
        }
        let p = p.clamp(0.0, 1.0);
        // view the keyframes in (possibly) reversed order without allocating
        let frame = |k: usize| -> (f32, T) {
            if reversed {
                let (o, v) = self.frames[n - 1 - k];
                (1.0 - o, v)
            } else {
                self.frames[k]
            }
        };
        let easing = |k: usize| if reversed { self.easings[n - 2 - k] } else { self.easings[k] };
        if p <= frame(0).0 {
            return frame(0).1;
        }
        for k in 0..n - 1 {
            let (o0, v0) = frame(k);
            let (o1, v1) = frame(k + 1);
            if p <= o1 {
                let local = if o1 - o0 <= 1e-6 { 1.0 } else { (p - o0) / (o1 - o0) };
                return T::lerp(v0, v1, ease(easing(k), local));
            }
        }
        frame(n - 1).1
    }

    /// Value at `time` seconds since the animation started (delay, repeats and direction applied).
    pub fn sample(&self, time: f32) -> T {
        let t = time - self.delay;
        if t <= 0.0 {
            return self.sample_progress(0.0);
        }
        let cycle = self.duration + self.repeat_delay;
        let max_iter = match self.repeat {
            Repeat::Once => Some(0),
            Repeat::Count(n) => Some(n),
            Repeat::Infinite => None,
        };
        let mut iter = (t / cycle).floor() as u32;
        let mut local = t - iter as f32 * cycle;
        if let Some(m) = max_iter {
            if iter > m || (iter == m && local >= self.duration) {
                iter = m;
                local = self.duration;
            }
        }
        let p = (local / self.duration).min(1.0);
        let odd = iter % 2 == 1;
        match self.repeat_type {
            RepeatType::Loop => self.sample_dir(p, false),
            RepeatType::Yoyo if odd => self.sample_dir(1.0 - p, false),
            RepeatType::Yoyo => self.sample_dir(p, false),
            RepeatType::Mirror => self.sample_dir(p, odd),
        }
    }

    /// Play the keyframes with a clock stored under `id` (starts on first call) and return the
    /// current value. Requests repaints until finished.
    pub fn play(&self, ui: &Ui, id: Id) -> T {
        let t = glowkit_core::Clock::elapsed(ui, id.with("glowkit_keyframes"));
        if !self.is_finished(t) {
            ui.ctx().request_repaint();
        }
        self.sample(t)
    }

    /// Restart the clock used by [`Self::play`] for `id`.
    pub fn restart(ui: &Ui, id: Id) {
        glowkit_core::Clock::restart(ui, id.with("glowkit_keyframes"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kf() -> Keyframes<f32> {
        Keyframes::new([(0.0, 0.0), (0.5, 10.0), (1.0, 0.0)]).ease(Easing::Linear).duration(1.0)
    }

    #[test]
    fn samples_segments() {
        let k = kf();
        assert_eq!(k.sample(0.0), 0.0);
        assert!((k.sample(0.25) - 5.0).abs() < 1e-4);
        assert!((k.sample(0.5) - 10.0).abs() < 1e-4);
        assert!((k.sample(0.75) - 5.0).abs() < 1e-4);
        assert_eq!(k.sample(5.0), 0.0);
        assert!(k.is_finished(1.0));
    }

    #[test]
    fn repeat_and_delay() {
        let k = Keyframes::new([(0.0, 0.0_f32), (1.0, 1.0)]).ease(Easing::Linear).repeat(Repeat::Count(1)).repeat_delay(0.5);
        assert_eq!(k.total_duration(), Some(2.5));
        assert!((k.sample(1.2) - 1.0).abs() < 1e-4, "holds the end during the repeat delay");
        assert!((k.sample(2.0) - 0.5).abs() < 1e-4);
        assert!((k.sample(9.0) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn yoyo_and_mirror_reverse() {
        let base = Keyframes::new([(0.0, 0.0_f32), (1.0, 1.0)]).ease(Easing::InQuad).repeat(Repeat::Infinite);
        let yoyo = base.clone().yoyo();
        let mirror = base.mirror();
        // second iteration at 25%: yoyo plays time backwards (value = ease(0.75)),
        // mirror plays 1 -> 0 with the easing applied forwards (value = 1 - ease(0.25)).
        assert!((yoyo.sample(1.25) - 0.5625).abs() < 1e-4);
        assert!((mirror.sample(1.25) - (1.0 - 0.0625)).abs() < 1e-4);
    }
}
