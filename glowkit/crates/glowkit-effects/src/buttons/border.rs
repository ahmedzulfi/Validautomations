use super::shimmer::{label_size, pressed_rect};
use super::util::{self, cr};
use egui::{pos2, vec2, Color32, Pos2, Response, Ui, Widget};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// A button whose thin border carries a glowing blob of light that keeps orbiting it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::MovingBorderButton::new("Borders are cool"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct MovingBorderButton {
    label: String,
    size: f32,
    color: Option<Color32>,
    background: Option<Color32>,
    text_color: Option<Color32>,
    radius: Option<f32>,
    duration: f32,
    min_width: Option<f32>,
}

impl MovingBorderButton {
    /// A moving-border button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), size: 15.0, color: None, background: None, text_color: None, radius: None, duration: 3.0, min_width: None }
    }
    setter!(/// Font size in points.
        size: f32);
    opt_setter!(/// Color of the orbiting light. Defaults to the theme's second gradient color.
        color: Color32);
    opt_setter!(/// Fill color. Defaults to the theme's card color.
        background: Color32);
    opt_setter!(/// Label color. Defaults to the foreground.
        text_color: Color32);
    opt_setter!(/// Corner radius. Defaults to 1.6x the theme radius.
        radius: f32);
    setter!(/// Seconds per lap.
        duration: f32);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for MovingBorderButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = label_size(ui, &self.label, self.size, 16.0, self.min_width) + vec2(0.0, 8.0);
        let (rect, resp) = util::alloc_button(ui, size, &self.label);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let hover = util::hover_t(ui, resp.id, resp.hovered(), 10.0);
        let press = util::press_t(ui, resp.id, resp.is_pointer_button_down_on());
        let rect = pressed_rect(rect, press);
        let radius = self.radius.unwrap_or(t.radius * 1.6);
        let color = self.color.unwrap_or(if t.dark { glowkit_core::hex("#38BDF8") } else { t.gradient[1] });
        let bg = self.background.unwrap_or(if t.dark { mix(t.card, t.background, 0.4) } else { t.card });
        let text = self.text_color.unwrap_or(t.foreground);
        let p = ui.painter();
        let head = (glowkit_core::time(ui) / self.duration.max(0.3)).fract();
        let hp = util::perimeter_point(rect, radius, head);
        paint::soft_shadow(p, rect, radius, 16.0, vec2(0.0, 4.0), with_alpha(Color32::BLACK, if t.dark { 0.4 } else { 0.08 }));
        paint::glow(p, hp, 34.0 + 8.0 * hover, with_alpha(color, 0.35 + 0.15 * hover));
        p.rect_filled(rect, cr(radius), bg);
        let base = if t.dark { mix(t.border, t.background, 0.2) } else { t.border };
        util::border_fn(p, rect, radius, 1.5, |q, _| {
            let d = q.distance(hp) / 46.0;
            let k = (-d * d).exp();
            base.lerp_to_gamma(color, k)
        });
        // a whisper of the light bleeding inside
        paint::glow(p, hp, 22.0, with_alpha(color, 0.10));
        let g = util::text_galley(ui, &self.label, self.size, text);
        util::galley_centered(p, &g, rect.center(), text);
        glowkit_core::animate(ui);
        resp
    }
}

/// A pill whose border highlight circles around it; on hover the light spreads over the whole
/// border.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::HoverBorderGradient::new("Aceternity UI"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct HoverBorderGradient {
    label: String,
    size: f32,
    highlight: Option<Color32>,
    hover_color: Option<Color32>,
    background: Option<Color32>,
    text_color: Option<Color32>,
    duration: f32,
    clockwise: bool,
    force_hover: bool,
    min_width: Option<f32>,
}

impl HoverBorderGradient {
    /// A hover-border-gradient pill with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: 15.0,
            highlight: None,
            hover_color: None,
            background: None,
            text_color: None,
            duration: 4.0,
            clockwise: true,
            force_hover: false,
            min_width: None,
        }
    }
    setter!(/// Font size in points.
        size: f32);
    opt_setter!(/// Color of the circling highlight. Defaults to the foreground.
        highlight: Color32);
    opt_setter!(/// Color the border floods with on hover. Defaults to a bright blue (the theme's info color).
        hover_color: Color32);
    opt_setter!(/// Fill color. Defaults to the theme background.
        background: Color32);
    opt_setter!(/// Label color. Defaults to the foreground.
        text_color: Color32);
    setter!(/// Seconds per revolution.
        duration: f32);
    setter!(/// Direction of travel.
        clockwise: bool);
    setter!(/// Render as if hovered (for showcases, or to mirror an external hover state).
        force_hover: bool);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for HoverBorderGradient {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = label_size(ui, &self.label, self.size, 8.0, self.min_width);
        let (rect, resp) = util::alloc_button(ui, size, &self.label);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let hover = util::hover_t(ui, resp.id, resp.hovered() || self.force_hover, 5.0);
        let press = util::press_t(ui, resp.id, resp.is_pointer_button_down_on());
        let rect = pressed_rect(rect, press);
        let radius = rect.height() / 2.0;
        let hl = self.highlight.unwrap_or(t.foreground);
        let hc = self.hover_color.unwrap_or(glowkit_core::hex("#3275F8"));
        let bg = self.background.unwrap_or(t.background);
        let text = self.text_color.unwrap_or(t.foreground);
        let p = ui.painter();
        let dir = if self.clockwise { 1.0 } else { -1.0 };
        let a = glowkit_core::time(ui) / self.duration.max(0.3) * std::f32::consts::TAU * dir - std::f32::consts::FRAC_PI_2;
        let c = rect.center();
        let half = rect.size() / 2.0;
        let spot = c + vec2(a.cos() * half.x, a.sin() * half.y);
        let sigma = egui::lerp(rect.width() * 0.22..=rect.width() * 1.4, hover);
        let col = hl.lerp_to_gamma(hc, hover);
        let intensity = |q: Pos2| {
            let d = q.distance(spot) / sigma;
            (-d * d).exp()
        };
        // halo around the spot / whole pill when hovered
        paint::glow_ellipse(p, spot, vec2(rect.height() * 1.4, rect.height() * 0.9), with_alpha(col, if t.dark { 0.18 } else { 0.0 } * (1.0 - hover)));
        if hover > 0.01 {
            paint::soft_shadow(p, rect, radius, 22.0, egui::Vec2::ZERO, with_alpha(hc, 0.45 * hover));
        }
        p.rect_filled(rect, cr(radius), bg);
        util::border_fn(p, rect, radius, 1.2, |q, _| with_alpha(t.border, 1.0).lerp_to_gamma(col, intensity(q)));
        let g = util::text_galley(ui, &self.label, self.size, text);
        util::galley_centered(p, &g, c, text);
        glowkit_core::animate(ui);
        resp
    }
}

/// A button with two "stars" of light gliding along its top and bottom edges in opposite
/// directions.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::StarBorderButton::new("Star Border"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct StarBorderButton {
    label: String,
    size: f32,
    color: Option<Color32>,
    background: Option<Color32>,
    text_color: Option<Color32>,
    radius: Option<f32>,
    duration: f32,
    min_width: Option<f32>,
}

impl StarBorderButton {
    /// A star-border button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), size: 15.0, color: None, background: None, text_color: None, radius: None, duration: 5.0, min_width: None }
    }
    setter!(/// Font size in points.
        size: f32);
    opt_setter!(/// Star color. Defaults to white in dark themes and the accent in light ones.
        color: Color32);
    opt_setter!(/// Fill color. Defaults to a deep gradient from the background.
        background: Color32);
    opt_setter!(/// Label color. Defaults to the foreground.
        text_color: Color32);
    opt_setter!(/// Corner radius. Defaults to 2x the theme radius.
        radius: f32);
    setter!(/// Seconds for one star to cross.
        duration: f32);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for StarBorderButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = label_size(ui, &self.label, self.size, 24.0, self.min_width) + vec2(0.0, 10.0);
        let (rect, resp) = util::alloc_button(ui, size, &self.label);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let hover = util::hover_t(ui, resp.id, resp.hovered(), 8.0);
        let press = util::press_t(ui, resp.id, resp.is_pointer_button_down_on());
        let rect = pressed_rect(rect, press);
        let radius = self.radius.unwrap_or(t.radius * 2.0);
        let color = self.color.unwrap_or(if t.dark { Color32::WHITE } else { t.accent });
        let text = self.text_color.unwrap_or(t.foreground);
        let p = ui.painter();
        let ph = (glowkit_core::time(ui) / self.duration.max(0.5)).fract();
        // eased travel: from beyond one end to beyond the other
        let u = |s: f32| -0.25 + 1.5 * s;
        let top = pos2(rect.min.x + rect.width() * u(ph), rect.min.y);
        let bottom = pos2(rect.max.x - rect.width() * u((ph + 0.5).fract()), rect.max.y);
        let spread = rect.width() * 0.22;
        // stars fade in/out near the ends of their run
        let fade = |s: Pos2| {
            let u = (s.x - rect.min.x) / rect.width();
            glowkit_core::anim::smoothstep(-0.2, 0.15, u) * glowkit_core::anim::smoothstep(1.2, 0.85, u)
        };
        let (ft, fb) = (fade(top), fade(bottom));
        let star = |q: Pos2| {
            let a = q.distance(top) / spread;
            let b = q.distance(bottom) / spread;
            ((-a * a).exp() * ft + (-b * b).exp() * fb).min(1.0)
        };
        for (s, f) in [(top, ft), (bottom, fb)] {
            paint::glow_ellipse(p, s, vec2(spread * 1.1, 10.0), with_alpha(color, (0.30 + 0.2 * hover) * f));
        }
        let bg_top = self.background.unwrap_or(if t.dark { mix(t.background, t.accent, 0.08) } else { t.card });
        let bg_bot = self.background.unwrap_or(if t.dark { t.background } else { mix(t.card, t.muted, 0.6) });
        paint::fill_convex_fn(p, &paint::rounded_rect_points(rect, radius, 10), 5, paint::linear_fn(rect, 90.0, &[bg_top, bg_bot]));
        util::border_fn(p, rect, radius, 1.3, |q, _| t.border.lerp_to_gamma(color, star(q) * 0.95));
        let g = util::text_galley(ui, &self.label, self.size, text);
        util::galley_centered(p, &g, rect.center(), text);
        glowkit_core::animate(ui);
        resp
    }
}
