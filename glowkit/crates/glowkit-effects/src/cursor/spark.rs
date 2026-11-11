use super::area_input;
use egui::{vec2, Color32, Id, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};
use glowkit_core::anim::{ease, Easing};
use glowkit_core::color::with_alpha;
use glowkit_core::{paint, Theme};
use std::f32::consts::TAU;

#[derive(Clone, Copy)]
struct Burst {
    x: f32,
    y: f32,
    t0: f32,
    rot: f32,
}

fn push_bursts(ui: &Ui, id: Id, clicks: &[egui::Pos2], now: f32, max_age: f32) -> Vec<Burst> {
    glowkit_core::with_state(ui, id.with("bursts"), Vec::<Burst>::new, |v| {
        v.retain(|b| now - b.t0 < max_age);
        for (k, c) in clicks.iter().enumerate() {
            v.push(Burst { x: c.x, y: c.y, t0: now, rot: (now * 7.13 + k as f32).sin() * TAU });
        }
        if v.len() > 64 {
            let extra = v.len() - 64;
            v.drain(0..extra);
        }
        v.clone()
    })
}

/// Lines of light that burst outward from every click, like a tiny firework.
///
/// Paint it over any area; it reads clicks without stealing them from widgets inside.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let rect = ui.max_rect();
/// glowkit_effects::ClickSpark::new().count(10).radius(28.0).show(ui, rect);
/// // or let it allocate an area:
/// ui.add(glowkit_effects::ClickSpark::new().autoplay(true));
/// ```
#[must_use = "call `.show(ui, rect)` or add it with `ui.add(...)`"]
pub struct ClickSpark {
    color: Option<Color32>,
    count: usize,
    radius: f32,
    length: f32,
    width: f32,
    duration: f32,
    easing: Easing,
    autoplay: bool,
    area: Option<Vec2>,
    salt: u64,
}

impl Default for ClickSpark {
    fn default() -> Self {
        Self::new()
    }
}

impl ClickSpark {
    /// Default sparks: 8 lines, 24 pt radius, accent colored.
    pub fn new() -> Self {
        Self { color: None, count: 8, radius: 24.0, length: 12.0, width: 2.0, duration: 0.45, easing: Easing::OutCubic, autoplay: false, area: None, salt: 0 }
    }
    /// Spark color. Defaults to the theme accent.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Number of spark lines per click.
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.clamp(1, 64);
        self
    }
    /// How far the sparks travel, in points.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.max(1.0);
        self
    }
    /// Length of each spark line.
    pub fn length(mut self, l: f32) -> Self {
        self.length = l.max(1.0);
        self
    }
    /// Stroke width of each spark.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(0.5);
        self
    }
    /// Burst duration in seconds.
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s.max(0.05);
        self
    }
    /// Easing of the outward motion.
    pub fn easing(mut self, e: Easing) -> Self {
        self.easing = e;
        self
    }
    /// Simulate a pointer that clicks periodically while the real pointer is elsewhere.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Size of the area allocated by `ui.add(...)` (defaults to full width x 300).
    pub fn desired_size(mut self, size: Vec2) -> Self {
        self.area = Some(size);
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }

    /// Paint the effect over `rect`.
    pub fn show(self, ui: &mut Ui, rect: Rect) {
        let id = ui.id().with(("click_spark", self.salt));
        let input = area_input(ui, rect, id, self.autoplay, 0.7);
        let now = glowkit_core::time(ui);
        let bursts = push_bursts(ui, id, &input.clicks, now, self.duration);
        if bursts.is_empty() || !ui.is_rect_visible(rect) {
            return;
        }
        let color = self.color.unwrap_or_else(|| Theme::of(ui).accent);
        let painter = ui.painter_at(rect);
        for b in &bursts {
            let p = ((now - b.t0) / self.duration).clamp(0.0, 1.0);
            let e = ease(self.easing, p);
            let dist = e * self.radius;
            let len = self.length * (1.0 - e);
            let c = egui::pos2(b.x, b.y);
            for i in 0..self.count {
                let a = b.rot + i as f32 / self.count as f32 * TAU;
                let dir = vec2(a.cos(), a.sin());
                let p0 = c + dir * dist;
                let p1 = c + dir * (dist + len);
                painter.line_segment([p0, p1], Stroke::new(self.width, color));
            }
        }
        glowkit_core::animate(ui);
    }
}

impl Widget for ClickSpark {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = glowkit_core::desired_size(ui, self.area, 300.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        self.show(ui, rect);
        resp
    }
}

/// Concentric rings that ripple out from every click, like a drop hitting water.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let rect = ui.max_rect();
/// glowkit_effects::ClickRipple::new().rings(3).max_radius(140.0).show(ui, rect);
/// ```
#[must_use = "call `.show(ui, rect)` or add it with `ui.add(...)`"]
pub struct ClickRipple {
    color: Option<Color32>,
    rings: usize,
    max_radius: f32,
    duration: f32,
    fill: bool,
    autoplay: bool,
    area: Option<Vec2>,
    salt: u64,
}

impl Default for ClickRipple {
    fn default() -> Self {
        Self::new()
    }
}

impl ClickRipple {
    /// Three rings expanding to 120 pt over one second.
    pub fn new() -> Self {
        Self { color: None, rings: 3, max_radius: 120.0, duration: 1.1, fill: true, autoplay: false, area: None, salt: 0 }
    }
    /// Ring color. Defaults to the theme accent.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Number of rings per click.
    pub fn rings(mut self, n: usize) -> Self {
        self.rings = n.clamp(1, 8);
        self
    }
    /// Final radius of the outer ring.
    pub fn max_radius(mut self, r: f32) -> Self {
        self.max_radius = r.max(4.0);
        self
    }
    /// Ripple duration in seconds.
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s.max(0.1);
        self
    }
    /// Also paint a soft filled disc under the rings.
    pub fn fill(mut self, on: bool) -> Self {
        self.fill = on;
        self
    }
    /// Simulate a pointer that clicks periodically while the real pointer is elsewhere.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Size of the area allocated by `ui.add(...)` (defaults to full width x 300).
    pub fn desired_size(mut self, size: Vec2) -> Self {
        self.area = Some(size);
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }

    /// Paint the effect over `rect`.
    pub fn show(self, ui: &mut Ui, rect: Rect) {
        let id = ui.id().with(("click_ripple", self.salt));
        let input = area_input(ui, rect, id, self.autoplay, 0.9);
        let now = glowkit_core::time(ui);
        let total = self.duration * (1.0 + 0.25 * (self.rings as f32 - 1.0));
        let bursts = push_bursts(ui, id, &input.clicks, now, total);
        if bursts.is_empty() || !ui.is_rect_visible(rect) {
            return;
        }
        let color = self.color.unwrap_or_else(|| Theme::of(ui).accent);
        let painter = ui.painter_at(rect);
        for b in &bursts {
            let c = egui::pos2(b.x, b.y);
            let age = now - b.t0;
            if self.fill {
                let p = (age / self.duration).clamp(0.0, 1.0);
                let r = ease(Easing::OutCubic, p) * self.max_radius * 0.8;
                paint::glow(&painter, c, r.max(1.0), with_alpha(color, 0.22 * (1.0 - p)));
            }
            for k in 0..self.rings {
                let p = ((age - k as f32 * self.duration * 0.25) / self.duration).clamp(0.0, 1.0);
                if p <= 0.0 || p >= 1.0 {
                    continue;
                }
                let r = ease(Easing::OutQuart, p) * self.max_radius * (1.0 - k as f32 * 0.12);
                let a = (1.0 - p).powf(1.5);
                painter.circle_stroke(c, r, Stroke::new(2.0 * (1.0 - p) + 0.5, with_alpha(color, a)));
            }
        }
        glowkit_core::animate(ui);
    }
}

impl Widget for ClickRipple {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = glowkit_core::desired_size(ui, self.area, 300.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        self.show(ui, rect);
        resp
    }
}
