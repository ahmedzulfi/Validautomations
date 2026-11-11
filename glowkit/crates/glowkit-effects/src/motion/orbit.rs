use egui::{vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};
use glowkit_core::color::{gradient, mix, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::TAU;

#[derive(Clone, Debug)]
struct Ring {
    radius: f32,
    icons: Vec<String>,
    period: f32,
    reverse: bool,
    icon_size: f32,
}

/// Icons circling a center on concentric rings, each ring at its own speed and direction.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::OrbitingCircles::new()
///     .center("✦")
///     .ring(80.0, &["🦀", "⚡"], 12.0)
///     .reverse_ring(150.0, &["🌐", "🎨", "📦"], 20.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct OrbitingCircles {
    center: Option<String>,
    rings: Vec<Ring>,
    path: bool,
    paused_on_hover: bool,
}

impl Default for OrbitingCircles {
    fn default() -> Self {
        Self::new()
    }
}

impl OrbitingCircles {
    /// An empty orbit system; add rings with [`Self::ring`].
    pub fn new() -> Self {
        Self { center: None, rings: Vec::new(), path: true, paused_on_hover: false }
    }
    /// Glyph or short text shown in the center circle.
    pub fn center(mut self, s: impl Into<String>) -> Self {
        self.center = Some(s.into());
        self
    }
    /// Add a clockwise ring of `icons` at `radius`, completing a turn every `period` seconds.
    pub fn ring<S: AsRef<str>>(mut self, radius: f32, icons: &[S], period: f32) -> Self {
        self.rings.push(Ring { radius, icons: icons.iter().map(|s| s.as_ref().to_owned()).collect(), period: period.max(0.5), reverse: false, icon_size: 40.0 });
        self
    }
    /// Add a counter-clockwise ring.
    pub fn reverse_ring<S: AsRef<str>>(mut self, radius: f32, icons: &[S], period: f32) -> Self {
        self = self.ring(radius, icons, period);
        if let Some(r) = self.rings.last_mut() {
            r.reverse = true;
        }
        self
    }
    /// Size of the icons on the most recently added ring.
    pub fn icon_size(mut self, s: f32) -> Self {
        if let Some(r) = self.rings.last_mut() {
            r.icon_size = s.max(8.0);
        }
        self
    }
    /// Draw the ring paths (default on).
    pub fn path(mut self, on: bool) -> Self {
        self.path = on;
        self
    }
    /// Freeze the orbits while hovered.
    pub fn pause_on_hover(mut self, on: bool) -> Self {
        self.paused_on_hover = on;
        self
    }
}

fn icon_bubble(painter: &egui::Painter, t: &Theme, c: Pos2, size: f32, icon: &str) {
    paint::soft_shadow(painter, Rect::from_center_size(c, Vec2::splat(size)), size / 2.0, 10.0, vec2(0.0, 3.0), with_alpha(Color32::BLACK, if t.dark { 0.45 } else { 0.12 }));
    let fill = if t.dark { mix(t.card, t.secondary, 0.5) } else { t.card };
    painter.circle(c, size / 2.0, fill, Stroke::new(1.0, t.border));
    painter.text(c, Align2::CENTER_CENTER, icon, FontId::proportional(size * 0.46), t.foreground);
}

impl Widget for OrbitingCircles {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let max_r = self.rings.iter().map(|r| r.radius + r.icon_size / 2.0).fold(40.0, f32::max);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), max_r * 2.0 + 16.0), Sense::hover());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let paused = self.paused_on_hover && response.hovered();
        let clock = glowkit_core::with_state(ui, id.with("clock"), || 0.0f32, |c| {
            if !paused {
                *c += dt;
            }
            *c
        });
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(10.0));
        let c = rect.center();
        if self.path {
            for r in &self.rings {
                painter.circle_stroke(c, r.radius, Stroke::new(1.0, with_alpha(t.muted_foreground, if t.dark { 0.22 } else { 0.28 })));
            }
        }
        if let Some(label) = &self.center {
            let size = 76.0;
            paint::glow(&painter, c, size * 1.4, with_alpha(t.gradient[0], if t.dark { 0.35 } else { 0.18 }));
            let stops = [t.gradient[0], t.gradient[1]];
            paint::gradient_circle(&painter, c, size / 2.0, &[gradient(&stops, 0.0), gradient(&stops, 1.0)]);
            painter.circle_stroke(c, size / 2.0, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.25)));
            painter.text(c, Align2::CENTER_CENTER, label, FontId::proportional(28.0), Color32::WHITE);
        }
        for r in &self.rings {
            let n = r.icons.len().max(1);
            let base = clock / r.period * TAU * if r.reverse { -1.0 } else { 1.0 };
            for (k, icon) in r.icons.iter().enumerate() {
                let a = base + k as f32 / n as f32 * TAU;
                let p = c + vec2(a.cos(), a.sin()) * r.radius;
                icon_bubble(&painter, &t, p, r.icon_size, icon);
            }
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A glowing gradient pulse that travels along a curved path between two points, over a faint
/// track: for diagrams that show data flowing between services.
///
/// Paint the beams first, then your nodes on top.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let (a, b) = (egui::pos2(0.0, 0.0), egui::pos2(200.0, 0.0));
/// glowkit_effects::AnimatedBeam::new(a, b).curvature(-60.0).duration(3.0).paint(ui);
/// ```
#[must_use = "call `.paint(ui)`"]
pub struct AnimatedBeam {
    from: Pos2,
    to: Pos2,
    curvature: f32,
    reverse: bool,
    duration: f32,
    delay: f32,
    width: f32,
    length: f32,
    colors: Option<[Color32; 2]>,
    track: Option<Color32>,
}

impl AnimatedBeam {
    /// A beam from `from` to `to` (e.g. the centers of two node rects).
    pub fn new(from: Pos2, to: Pos2) -> Self {
        Self { from, to, curvature: 0.0, reverse: false, duration: 3.0, delay: 0.0, width: 2.0, length: 0.35, colors: None, track: None }
    }
    /// Bend the path: offset of the curve's midpoint, perpendicular-ish (negative = up).
    pub fn curvature(mut self, c: f32) -> Self {
        self.curvature = c;
        self
    }
    /// Travel from `to` back to `from`.
    pub fn reverse(mut self, on: bool) -> Self {
        self.reverse = on;
        self
    }
    /// Seconds for one pass.
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s.max(0.2);
        self
    }
    /// Time offset in seconds (stagger several beams).
    pub fn delay(mut self, s: f32) -> Self {
        self.delay = s;
        self
    }
    /// Stroke width.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(0.5);
        self
    }
    /// Length of the pulse as a fraction of the path.
    pub fn length(mut self, l: f32) -> Self {
        self.length = l.clamp(0.02, 1.0);
        self
    }
    /// Gradient of the pulse (tail to head). Defaults to the theme gradient.
    pub fn colors(mut self, start: Color32, stop: Color32) -> Self {
        self.colors = Some([start, stop]);
        self
    }
    /// Color of the static track.
    pub fn track_color(mut self, c: Color32) -> Self {
        self.track = Some(c);
        self
    }

    /// Paint the track and the moving pulse.
    pub fn paint(self, ui: &Ui) {
        let t = Theme::of(ui);
        let ctrl = self.from.lerp(self.to, 0.5) + vec2(0.0, self.curvature);
        let pts = paint::quad_bezier(self.from, ctrl, self.to, 64);
        let painter = ui.painter();
        let track = self.track.unwrap_or(with_alpha(t.muted_foreground, if t.dark { 0.2 } else { 0.25 }));
        painter.add(egui::Shape::line(pts.clone(), Stroke::new(self.width, track)));
        let [c0, c1] = self.colors.unwrap_or([t.gradient[0], t.gradient[1]]);
        let time = glowkit_core::time(ui) - self.delay;
        let len = self.length;
        let phase = (time / self.duration).rem_euclid(1.0);
        // head runs from 0 to 1 + len so the tail fully exits
        let head = phase * (1.0 + len);
        let rev = self.reverse;
        paint::gradient_polyline(painter, &pts, false, self.width + 0.6, |u| {
            let v = if rev { 1.0 - u } else { u };
            let d = head - v;
            if !(0.0..=len).contains(&d) {
                return Color32::TRANSPARENT;
            }
            let k = 1.0 - d / len; // 1 at the head
            with_alpha(mix(c0, c1, k), (k * 2.5).min(1.0))
        });
        if head <= 1.0 {
            let u = if rev { 1.0 - head } else { head };
            let i = ((u * (pts.len() - 1) as f32).round() as usize).min(pts.len() - 1);
            paint::glow(painter, pts[i], self.width * 7.0, with_alpha(c1, 0.4));
        }
        glowkit_core::animate(ui);
    }
}
