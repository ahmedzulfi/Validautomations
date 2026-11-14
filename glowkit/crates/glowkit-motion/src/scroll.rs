//! Scroll-linked motion: scroll progress, value mapping (`transform`), parallax and in-view
//! detection for content inside an [`egui::ScrollArea`].

use crate::value::{Animatable, MAX_DIM};
use egui::scroll_area::ScrollAreaOutput;
use egui::{Id, Rect, Ui, Vec2};

/// Scroll progress `0..=1` on each axis, from a [`ScrollAreaOutput`] (0 when nothing scrolls).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let out = egui::ScrollArea::vertical().show(ui, |ui| { /* long content */ });
/// let p = glowkit_motion::scroll_progress(&out).y;
/// ```
pub fn scroll_progress<R>(out: &ScrollAreaOutput<R>) -> Vec2 {
    let max = (out.content_size - out.inner_rect.size()).max(Vec2::ZERO);
    let o = out.state.clamped_offset();
    Vec2::new(
        if max.x > 0.0 { (o.x / max.x).clamp(0.0, 1.0) } else { 0.0 },
        if max.y > 0.0 { (o.y / max.y).clamp(0.0, 1.0) } else { 0.0 },
    )
}

/// Scroll progress of the [`egui::ScrollArea`] with id `scroll_id` as of the last frame, usable
/// *before* (or inside) the scroll area. Pair with `ScrollArea::id_salt` and
/// `ScrollAreaOutput::id`.
pub fn scroll_progress_of(ui: &Ui, scroll_id: Id, viewport: Vec2, content: Vec2) -> Vec2 {
    let o = egui::scroll_area::State::load(ui.ctx(), scroll_id).map_or(Vec2::ZERO, |s| s.clamped_offset());
    let max = (content - viewport).max(Vec2::ZERO);
    Vec2::new(
        if max.x > 0.0 { (o.x / max.x).clamp(0.0, 1.0) } else { 0.0 },
        if max.y > 0.0 { (o.y / max.y).clamp(0.0, 1.0) } else { 0.0 },
    )
}

/// How far the content of the enclosing scroll area is scrolled, in points. Call it at the top of
/// the `ScrollArea::show` closure.
pub fn scroll_offset(ui: &Ui) -> Vec2 {
    ui.clip_rect().min - ui.max_rect().min
}

/// Map `value` through piecewise-linear `input` → `output` ranges, clamped at the ends (like
/// `useTransform`). `input` must be ascending and as long as `output`.
///
/// ```
/// use glowkit_motion::transform;
/// assert_eq!(transform(0.5, &[0.0, 1.0], &[0.0_f32, 100.0]), 50.0);
/// let c: egui::Color32 = transform(0.0, &[0.0, 1.0], &[egui::Color32::RED, egui::Color32::BLUE]);
/// assert_eq!(c, egui::Color32::RED);
/// ```
pub fn transform<T: Animatable>(value: f32, input: &[f32], output: &[T]) -> T {
    transform_with(value, input, output, true)
}

/// Like [`transform`] but extrapolates past the ends when `clamp` is false.
pub fn transform_with<T: Animatable>(value: f32, input: &[f32], output: &[T], clamp: bool) -> T {
    let n = input.len().min(output.len());
    match n {
        0 => T::from_components(&[0.0; MAX_DIM]),
        1 => output[0],
        _ => {
            let mut i = 0;
            while i + 2 < n && value > input[i + 1] {
                i += 1;
            }
            let (a, b) = (input[i], input[i + 1]);
            let mut t = if (b - a).abs() < f32::EPSILON { 1.0 } else { (value - a) / (b - a) };
            if clamp {
                t = t.clamp(0.0, 1.0);
            }
            T::lerp(output[i], output[i + 1], t)
        }
    }
}

/// Parallax offset: content that should move at `factor` × the scroll speed gets this extra
/// offset (`factor` 0 = moves with the page, 0.5 = half speed, 1 = fixed in the viewport).
pub fn parallax(scroll: f32, factor: f32) -> f32 {
    scroll * factor
}

/// Detects when an element enters the visible part of its scroll area (`whileInView`).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let rect = egui::Rect::NOTHING;
/// let seen = glowkit_motion::InView::new(ui.scope_id().with("card")).amount(0.4).once(true).check(ui, rect);
/// ```
#[derive(Clone, Copy, Debug)]
#[must_use = "call `.check(ui, rect)`"]
pub struct InView {
    id: Id,
    amount: f32,
    once: bool,
    margin: f32,
}

impl InView {
    /// A detector stored under `id`.
    pub fn new(id: Id) -> Self {
        Self { id, amount: 0.3, once: false, margin: 0.0 }
    }
    /// Fraction of the element (`0..=1`) that must be visible.
    pub fn amount(mut self, a: f32) -> Self {
        self.amount = a.clamp(0.0, 1.0);
        self
    }
    /// Stay "in view" forever after the first time.
    pub fn once(mut self, once: bool) -> Self {
        self.once = once;
        self
    }
    /// Grow (positive) or shrink (negative) the viewport by this many points.
    pub fn margin(mut self, m: f32) -> Self {
        self.margin = m;
        self
    }
    /// Is `rect` in view of the current clip rect?
    pub fn check(self, ui: &Ui, rect: Rect) -> bool {
        let key = self.id.with("glowkit_in_view");
        if self.once && ui.data(|d| d.get_temp::<bool>(key)).unwrap_or(false) {
            return true;
        }
        let vis = visible_fraction(ui.clip_rect().expand(self.margin), rect);
        let inside = vis >= self.amount.max(1e-4);
        if self.once && inside {
            ui.data_mut(|d| d.insert_temp(key, true));
        }
        inside
    }
}

/// Shortcut for `InView::new(id).check(ui, rect)`.
pub fn in_view(ui: &Ui, id: Id, rect: Rect) -> bool {
    InView::new(id).check(ui, rect)
}

/// Fraction of `rect`'s area inside `viewport`.
pub fn visible_fraction(viewport: Rect, rect: Rect) -> f32 {
    let i = viewport.intersect(rect);
    if !i.is_positive() || rect.area() <= 0.0 {
        return 0.0;
    }
    (i.area() / rect.area()).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    #[test]
    fn transform_maps_piecewise() {
        let r = [0.0, 0.5, 1.0];
        let o = [0.0_f32, 10.0, 0.0];
        assert_eq!(transform(0.25, &r, &o), 5.0);
        assert_eq!(transform(0.75, &r, &o), 5.0);
        assert_eq!(transform(-1.0, &r, &o), 0.0);
        assert_eq!(transform_with(2.0, &[0.0, 1.0], &[0.0_f32, 10.0], false), 20.0);
    }

    #[test]
    fn visibility() {
        let vp = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0));
        assert_eq!(visible_fraction(vp, Rect::from_min_max(pos2(0.0, 50.0), pos2(10.0, 150.0))), 0.5);
        assert_eq!(visible_fraction(vp, Rect::from_min_max(pos2(0.0, 200.0), pos2(10.0, 250.0))), 0.0);
    }
}
