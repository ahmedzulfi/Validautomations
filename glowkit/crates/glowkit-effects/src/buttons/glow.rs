use super::shimmer::{label_size, pressed_rect};
use super::util::{self, cr};
use egui::{pos2, vec2, Color32, Response, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::{contrast_text, gradient, shade, with_alpha};
use glowkit_core::{paint, Theme};

/// A solid button that sends rings of color pulsing outward, like a heartbeat.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::PulsatingButton::new("Join the waitlist"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PulsatingButton {
    label: String,
    size: f32,
    background: Option<Color32>,
    pulse_color: Option<Color32>,
    text_color: Option<Color32>,
    radius: Option<f32>,
    duration: f32,
    spread: f32,
    min_width: Option<f32>,
}

impl PulsatingButton {
    /// A pulsating button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), size: 15.0, background: None, pulse_color: None, text_color: None, radius: None, duration: 1.6, spread: 12.0, min_width: None }
    }
    setter!(/// Font size in points.
        size: f32);
    opt_setter!(/// Fill color. Defaults to the theme accent.
        background: Color32);
    opt_setter!(/// Color of the pulse rings. Defaults to the fill color.
        pulse_color: Color32);
    opt_setter!(/// Label color. Defaults to a color that contrasts with the fill.
        text_color: Color32);
    opt_setter!(/// Corner radius. Defaults to the theme radius.
        radius: f32);
    setter!(/// Seconds per pulse.
        duration: f32);
    setter!(/// How far the rings travel, in points.
        spread: f32);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for PulsatingButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = label_size(ui, &self.label, self.size, 0.0, self.min_width);
        let (rect, resp) = util::alloc_button(ui, size, &self.label);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let hover = util::hover_t(ui, resp.id, resp.hovered(), 10.0);
        let press = util::press_t(ui, resp.id, resp.is_pointer_button_down_on());
        let rect = pressed_rect(rect, press);
        let radius = self.radius.unwrap_or(t.radius);
        let bg = self.background.unwrap_or(t.accent);
        let pulse = self.pulse_color.unwrap_or(bg);
        let text = self.text_color.unwrap_or(contrast_text(bg));
        let p = ui.painter();
        let time = glowkit_core::time(ui) / self.duration.max(0.2);
        for k in 0..2 {
            let ph = (time + k as f32 * 0.5).fract();
            let e = 1.0 - (1.0 - ph).powi(3);
            let grow = e * self.spread;
            let a = (1.0 - ph).powf(1.8) * 0.5;
            paint::soft_shadow(p, rect.expand(grow * 0.7), radius + grow * 0.7, 2.0 + grow * 0.8, egui::Vec2::ZERO, with_alpha(pulse, a));
        }
        let fill = shade(bg, 0.04 * hover);
        p.rect_filled(rect, cr(radius), fill);
        // top sheen
        paint::fill_convex_fn(p, &paint::rounded_rect_points(rect, radius, 8), 4, |q| {
            let v = (q.y - rect.min.y) / rect.height();
            with_alpha(Color32::WHITE, (0.5 - v).max(0.0) * 0.22)
        });
        let g = util::text_galley(ui, &self.label, self.size, text);
        util::galley_centered(p, &g, rect.center(), text);
        glowkit_core::animate(ui);
        resp
    }
}

/// A gradient pill floating on a soft, breathing pool of its own colored light. The glow grows
/// and the button lifts on hover.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::GlowButton::new("Start building"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct GlowButton {
    label: String,
    size: f32,
    colors: Vec<Color32>,
    text_color: Option<Color32>,
    radius: Option<f32>,
    intensity: f32,
    min_width: Option<f32>,
}

impl GlowButton {
    /// A glowing gradient button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), size: 15.0, colors: Vec::new(), text_color: None, radius: None, intensity: 1.0, min_width: None }
    }
    setter!(/// Font size in points.
        size: f32);
    /// Gradient stops for the fill and the glow. Defaults to the theme gradient.
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    opt_setter!(/// Label color. Defaults to white.
        text_color: Color32);
    opt_setter!(/// Corner radius. Defaults to a full pill.
        radius: f32);
    setter!(/// Glow strength multiplier (1 = default).
        intensity: f32);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for GlowButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = label_size(ui, &self.label, self.size, 8.0, self.min_width);
        let (rect, resp) = util::alloc_button(ui, size, &self.label);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let hover = util::hover_t(ui, resp.id, resp.hovered(), 9.0);
        let press = util::press_t(ui, resp.id, resp.is_pointer_button_down_on());
        let lift = glowkit_core::anim::spring(ui, resp.id.with("lift"), if resp.hovered() { -2.0 } else { 0.0 } + press * 2.0, 300.0, 18.0);
        let rect = pressed_rect(rect, press).translate(vec2(0.0, lift));
        let colors = if self.colors.is_empty() { t.gradient[..2].to_vec() } else { self.colors.clone() };
        let radius = self.radius.unwrap_or(rect.height() / 2.0);
        let text = self.text_color.unwrap_or(Color32::WHITE);
        let p = ui.painter();
        let time = glowkit_core::time(ui);
        let breathe = 0.85 + 0.15 * (time * 2.0).sin();
        let a = (if t.dark { 0.75 } else { 0.5 }) * self.intensity * (breathe + 0.4 * hover);
        let n = 5;
        for i in 0..n {
            let u = (i as f32 + 0.5) / n as f32;
            let c = gradient(&colors, u);
            let center = pos2(rect.min.x + rect.width() * u, rect.center().y + rect.height() * 0.3);
            paint::glow_ellipse(p, center, vec2(rect.width() * (0.3 + 0.06 * hover), rect.height() * (0.9 + 0.3 * hover)), with_alpha(c, a * 0.55));
        }
        paint::gradient_rect(p, rect, radius, 0.0, &colors);
        // glossy top highlight and hairline
        paint::fill_convex_fn(p, &paint::rounded_rect_points(rect, radius, 10), 4, |q| {
            let v = (q.y - rect.min.y) / rect.height();
            with_alpha(Color32::WHITE, (0.55 - v).max(0.0) * (0.28 + 0.1 * hover))
        });
        util::border_fn(p, rect, radius, 1.0, |q, _| {
            let v = (q.y - rect.min.y) / rect.height();
            with_alpha(Color32::WHITE, 0.35 * (1.0 - v))
        });
        let g = util::text_galley(ui, &self.label, self.size, text);
        util::galley_centered(p, &g, rect.center(), text);
        glowkit_core::animate(ui);
        resp
    }
}

/// An outlined neon tube: glowing border and text that flicker to life, and fill with light on
/// hover.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_core::hex;
/// ui.add(glowkit_effects::NeonButton::new("NEON").color(hex("#22D3EE")));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct NeonButton {
    label: String,
    size: f32,
    color: Option<Color32>,
    radius: Option<f32>,
    flicker: bool,
    force_hover: bool,
    min_width: Option<f32>,
}

impl NeonButton {
    /// A neon button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), size: 15.0, color: None, radius: None, flicker: true, force_hover: false, min_width: None }
    }
    setter!(/// Font size in points.
        size: f32);
    opt_setter!(/// Neon color. Defaults to the theme accent.
        color: Color32);
    opt_setter!(/// Corner radius. Defaults to half the theme radius.
        radius: f32);
    setter!(/// Occasional flicker, like a real tube.
        flicker: bool);
    setter!(/// Render as if hovered (for showcases, or to mirror an external hover state).
        force_hover: bool);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for NeonButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = label_size(ui, &self.label, self.size, 12.0, self.min_width);
        let (rect, resp) = util::alloc_button(ui, size, &self.label);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let hover = util::hover_t(ui, resp.id, resp.hovered() || self.force_hover, 8.0);
        let press = util::press_t(ui, resp.id, resp.is_pointer_button_down_on());
        let rect = pressed_rect(rect, press);
        let radius = self.radius.unwrap_or(t.radius * 0.5);
        let color = self.color.unwrap_or(t.accent);
        let time = glowkit_core::time(ui);
        // flicker: brief dips at pseudo-random moments
        let mut on = 1.0;
        if self.flicker && hover < 0.5 {
            let slot = (time * 6.0).floor() as u64 ^ (resp.id.value() & 0xffff);
            let r = glowkit_core::rng::hash01(slot);
            if r > 0.94 {
                on = 0.35 + 0.3 * glowkit_core::rng::hash01(slot.wrapping_mul(31));
            }
        }
        let p = ui.painter();
        let glow_a = (if t.dark { 0.55 } else { 0.4 }) * on;
        paint::soft_shadow(p, rect, radius, 26.0 + 12.0 * hover, egui::Vec2::ZERO, with_alpha(color, glow_a * (0.6 + 0.6 * hover)));
        // inner glow (inset)
        p.rect_filled(rect, cr(radius), if t.dark { t.background } else { t.card });
        paint::fill_convex_fn(p, &paint::rounded_rect_points(rect.shrink(1.0), radius, 8), 5, |q| {
            let d = ((q - rect.center()) / (rect.size() / 2.0)).length();
            with_alpha(color, (d * d * 0.22 * on).min(0.4))
        });
        if hover > 0.01 {
            p.rect_filled(rect, cr(radius), with_alpha(color, hover));
        }
        p.rect_stroke(rect.shrink(1.0), cr(radius), Stroke::new(2.0, color.gamma_multiply(0.5 + 0.5 * on)), StrokeKind::Middle);
        p.rect_stroke(rect.shrink(1.0), cr(radius), Stroke::new(0.8, with_alpha(Color32::WHITE, 0.5 * on * (1.0 - hover))), StrokeKind::Middle);
        let ink = if t.dark { color } else { shade(color, -0.12) };
        let text_col = ink.lerp_to_gamma(contrast_text(color), hover);
        let g = util::text_galley(ui, &self.label, self.size, text_col);
        // text glow
        if hover < 0.99 {
            let gc = with_alpha(color, 0.18 * on * (1.0 - hover));
            for (dx, dy) in [(-1.5, 0.0), (1.5, 0.0), (0.0, -1.5), (0.0, 1.5)] {
                util::galley_centered(p, &g, rect.center() + vec2(dx, dy), gc);
            }
        }
        util::galley_centered(p, &g, rect.center(), text_col.gamma_multiply(0.55 + 0.45 * on.max(hover)));
        glowkit_core::animate(ui);
        resp
    }
}
