use egui::{vec2, Color32, Pos2, Rect, Response, Sense, Ui, Vec2, Widget};
use glowkit_core::color::mix;
use glowkit_core::paint::{circle_points, fill_convex_fn, rounded_rect_points, subdivide_closed};
use glowkit_core::Theme;

/// A placeholder block with a soft shimmer sweeping across it while content loads. All
/// skeletons share one sweep in screen space, so a layout of them shimmers as a single surface.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::Skeleton;
/// ui.add(Skeleton::circle(40.0));
/// ui.add(Skeleton::new(egui::vec2(220.0, 14.0)));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Skeleton {
    size: Vec2,
    radius: Option<f32>,
    circle: bool,
    shimmer: bool,
}

impl Skeleton {
    /// A rounded rectangle placeholder.
    pub fn new(size: Vec2) -> Self {
        Self { size, radius: None, circle: false, shimmer: true }
    }
    /// A circular placeholder (avatars).
    pub fn circle(diameter: f32) -> Self {
        Self { size: vec2(diameter, diameter), radius: None, circle: true, shimmer: true }
    }
    /// A line of text placeholder (`width` × 12 pt).
    pub fn line(width: f32) -> Self {
        Self::new(vec2(width, 12.0))
    }
    /// Corner radius (default: theme radius × 0.6).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = Some(r);
        self
    }
    /// Disable the shimmer (a static pulse is used instead).
    pub fn shimmer(mut self, s: bool) -> Self {
        self.shimmer = s;
        self
    }

    /// Paint into an existing rect.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let t = Theme::of(ui);
        let base = if t.dark { mix(t.muted, t.foreground, 0.04) } else { mix(t.muted, t.input, 0.7) };
        let hi = if t.dark { mix(t.muted, t.foreground, 0.12) } else { mix(t.muted, t.background, 0.9) };
        let time = glowkit_core::time(ui);
        let outline: Vec<Pos2> = if self.circle {
            circle_points(rect.center(), rect.width() / 2.0, 48)
        } else {
            let r = self.radius.unwrap_or(t.radius * 0.6).min(rect.height() / 2.0);
            subdivide_closed(&rounded_rect_points(rect, r, 6), 10.0)
        };
        if self.shimmer {
            let screen = ui.ctx().content_rect();
            let span = screen.width() + 600.0;
            let center = screen.min.x - 300.0 + ((time / 1.8).fract()) * span;
            let band = 160.0;
            fill_convex_fn(ui.painter(), &outline, 4, |p| {
                let d = ((p.x + (p.y - screen.min.y) * 0.25 - center) / band).abs();
                let k = (1.0 - d).clamp(0.0, 1.0);
                base.lerp_to_gamma(hi, k * k * (3.0 - 2.0 * k))
            });
        } else {
            let k = 0.5 + 0.5 * (time * 2.5).sin();
            let c: Color32 = base.lerp_to_gamma(hi, k * 0.6);
            fill_convex_fn(ui.painter(), &outline, 2, |_| c);
        }
        glowkit_core::animate(ui);
    }
}

impl Widget for Skeleton {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, resp) = ui.allocate_exact_size(self.size, Sense::hover());
        if ui.is_rect_visible(rect) {
            self.paint(ui, rect);
        }
        resp
    }
}
