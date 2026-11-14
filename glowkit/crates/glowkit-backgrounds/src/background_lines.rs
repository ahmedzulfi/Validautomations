//! Background Lines: flowing curves that draw themselves in and out across the area.

use crate::{background, h01, Base};
use egui::{pos2, Color32, Pos2, Rect, Ui};
use glowkit_core::{
    ease,
    paint::{cubic_bezier, gradient_polyline},
    Easing,
};

/// Long, flowing curves sweeping across the area that draw themselves in, linger and erase
/// again, each on its own rhythm and in its own color, like animated SVG strokes.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::BackgroundLines;
/// ui.add(BackgroundLines::new().count(24));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct BackgroundLines {
    base: Base,
    count: usize,
    colors: Vec<Color32>,
    width: f32,
    speed: f32,
    ghost_lines: bool,
}

impl Default for BackgroundLines {
    fn default() -> Self {
        Self::new()
    }
}

impl BackgroundLines {
    /// Twenty lines in the theme chart colors.
    pub fn new() -> Self {
        Self { base: Base::default(), count: 20, colors: Vec::new(), width: 1.6, speed: 1.0, ghost_lines: true }
    }
    /// Number of lines (default 20).
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.clamp(1, 80);
        self
    }
    /// Line colors, cycled (default: the theme chart palette).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    /// Stroke width in points (default 1.6).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.clamp(0.3, 8.0);
        self
    }
    /// Animation speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Show each path faintly while it is not being drawn (default true).
    pub fn ghost_lines(mut self, on: bool) -> Self {
        self.ghost_lines = on;
        self
    }

    /// Paint the lines into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_background_lines") else { return };
        let t = &f.theme;
        let colors: Vec<Color32> = if self.colors.is_empty() { t.chart.to_vec() } else { self.colors.clone() };
        let (w, h) = (rect.width(), rect.height());
        let at = |u: f32, v: f32| pos2(rect.min.x + u * w, rect.min.y + v * h);
        let p = &f.painter;
        let time = f.time * self.speed;
        for i in 0..self.count {
            let s = i as u32;
            let r = |k: u32| h01(s * 13 + 7, k);
            // a sweeping S-curve entering low on the left and leaving high on the right
            let y0 = 0.55 + r(1) * 0.65;
            let y1 = -0.15 + r(2) * 0.6;
            let a = at(-0.1, y0);
            let b = at(1.1, y1);
            let c1 = at(0.25 + r(3) * 0.2, y0 - 0.3 - r(4) * 0.9);
            let c2 = at(0.55 + r(5) * 0.2, y1 + 0.3 + r(6) * 0.9);
            let pts: Vec<Pos2> = cubic_bezier(a, c1, c2, b, 72);
            let color = colors[i % colors.len()];
            if self.ghost_lines {
                p.add(egui::Shape::line(pts.clone(), egui::Stroke::new(1.0, color.gamma_multiply(if t.dark { 0.06 } else { 0.08 }))));
            }
            // draw in, linger, erase (head and tail both travel left to right)
            let period = 5.0 + r(7) * 4.0;
            let ph = ((time + r(8) * period) / period).rem_euclid(1.0);
            let head = ease(Easing::InOutCubic, (ph / 0.55).min(1.0));
            let tail = ease(Easing::InOutCubic, ((ph - 0.5) / 0.5).clamp(0.0, 1.0));
            if head - tail < 0.002 {
                continue;
            }
            let n = pts.len() - 1;
            let i0 = (tail * n as f32).floor() as usize;
            let i1 = ((head * n as f32).ceil() as usize).min(n);
            if i1 <= i0 {
                continue;
            }
            let seg = &pts[i0..=i1];
            let width = self.width * (0.7 + r(9) * 0.6);
            gradient_polyline(p, seg, false, width, |u| {
                // brighter toward the drawing head, soft at the tail
                let k = 0.25 + 0.75 * u.powf(0.6);
                color.gamma_multiply(k)
            });
        }
    }
}

background!(BackgroundLines);
