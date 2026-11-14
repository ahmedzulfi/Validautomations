use super::util::{self, cr};
use egui::{vec2, Color32, Pos2, Response, Ui, Vec2, Widget};
use glowkit_core::color::{gradient_cyclic, mix, palettes, with_alpha};
use glowkit_core::{paint, Theme};

/// Standard button size for a label of font size `size`.
pub(crate) fn label_size(ui: &Ui, label: &str, size: f32, extra_w: f32, min_width: Option<f32>) -> Vec2 {
    let g = util::text_galley(ui, label, size, Color32::WHITE);
    let h = (size * 2.8).round();
    let w = (g.size().x + size * 3.2 + extra_w).max(min_width.unwrap_or(0.0)).round();
    vec2(w, h)
}

/// Press "squish": the painted rect shrinks slightly while held.
pub(crate) fn pressed_rect(rect: egui::Rect, press: f32) -> egui::Rect {
    rect.shrink2(rect.size() * 0.025 * press)
}

/// A dark pill with a spark of light racing around its border and a soft inner glow that
/// rises on hover.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// if ui.add(glowkit_effects::ShimmerButton::new("Shimmer Button")).clicked() { /* ... */ }
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ShimmerButton {
    label: String,
    size: f32,
    background: Option<Color32>,
    shimmer_color: Option<Color32>,
    text_color: Option<Color32>,
    radius: Option<f32>,
    duration: f32,
    min_width: Option<f32>,
}

impl ShimmerButton {
    /// A shimmer button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: 15.0,
            background: None,
            shimmer_color: None,
            text_color: None,
            radius: None,
            duration: 3.0,
            min_width: None,
        }
    }
    setter!(/// Font size in points (the button scales with it).
        size: f32);
    opt_setter!(/// Fill color. Defaults to near-black (the theme's darkest surface).
        background: Color32);
    opt_setter!(/// Color of the travelling spark. Defaults to white.
        shimmer_color: Color32);
    opt_setter!(/// Label color. Defaults to white.
        text_color: Color32);
    opt_setter!(/// Corner radius. Defaults to a full pill.
        radius: f32);
    setter!(/// Seconds per lap of the spark.
        duration: f32);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for ShimmerButton {
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
        let radius = self.radius.unwrap_or(rect.height() / 2.0);
        let bg = self.background.unwrap_or(if t.dark { glowkit_core::hex("#050507") } else { t.primary });
        let spark = self.shimmer_color.unwrap_or(Color32::WHITE);
        let text = self.text_color.unwrap_or(Color32::WHITE);
        let p = ui.painter();
        let time = glowkit_core::time(ui);
        let head = (time / self.duration.max(0.2)).fract();

        let hp = util::perimeter_point(rect, radius, head);
        paint::soft_shadow(
            p,
            rect,
            radius,
            18.0,
            vec2(0.0, 6.0),
            with_alpha(Color32::BLACK, if t.dark { 0.5 } else { 0.25 }),
        );
        p.rect_filled(rect, cr(radius), bg);
        // light spilling inward from the spark (clipped to the button)
        util::inner_glow(p, rect, radius, hp, rect.height() * 0.9, with_alpha(spark, 0.10 + 0.06 * hover));
        // inner highlight rising from the bottom on hover
        let inner = rect.shrink(1.0);
        paint::fill_convex_fn(p, &paint::rounded_rect_points(inner, radius - 1.0, 10), 6, |q: Pos2| {
            let v = ((q.y - inner.min.y) / inner.height()).clamp(0.0, 1.0);
            let k = v.powf(3.0) * (0.10 + 0.22 * hover);
            with_alpha(spark, k)
        });
        // hairline + spark
        util::inner_border_fn(p, rect, radius, 1.4, |_, s| {
            let k = util::sweep_profile(head, s, 0.035, 0.16);
            with_alpha(spark, 0.12 + 0.88 * k)
        });
        let g = util::text_galley(ui, &self.label, self.size, text);
        util::galley_centered(p, &g, rect.center(), text);
        glowkit_core::animate(ui);
        resp
    }
}

/// A subtle surface button whose label and border catch a band of light that sweeps across.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::ShinyButton::new("Shiny Button"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ShinyButton {
    label: String,
    size: f32,
    background: Option<Color32>,
    text_color: Option<Color32>,
    shine_color: Option<Color32>,
    radius: Option<f32>,
    period: f32,
    min_width: Option<f32>,
}

impl ShinyButton {
    /// A shiny button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: 15.0,
            background: None,
            text_color: None,
            shine_color: None,
            radius: None,
            period: 2.6,
            min_width: None,
        }
    }
    setter!(/// Font size in points.
        size: f32);
    opt_setter!(/// Fill color. Defaults to the theme's card color with an accent tint.
        background: Color32);
    opt_setter!(/// Base label color. Defaults to the muted foreground.
        text_color: Color32);
    opt_setter!(/// Color of the sweeping light. Defaults to the foreground.
        shine_color: Color32);
    opt_setter!(/// Corner radius. Defaults to the theme radius.
        radius: f32);
    setter!(/// Seconds between sweeps.
        period: f32);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for ShinyButton {
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
        let base = self.text_color.unwrap_or(t.muted_foreground.lerp_to_gamma(t.foreground, 0.35 + 0.4 * hover));
        let shine = self.shine_color.unwrap_or(t.foreground);
        let bg = self.background.unwrap_or(mix(t.card, t.accent, if t.dark { 0.06 } else { 0.03 }));
        let p = ui.painter();
        let time = glowkit_core::time(ui);
        // band position: travels from -0.3 to 1.3 then rests
        let cycle = self.period.max(0.5);
        let ph = (time % cycle) / (cycle * 0.75);
        let band_c = -0.3 + ph.min(1.0) * 1.6;
        let band = |q: Pos2| {
            let u = (q.x - rect.min.x + (q.y - rect.center().y) * -0.6) / rect.width();
            let d = ((u - band_c) / 0.16).abs();
            let k = (1.0 - d).clamp(0.0, 1.0);
            k * k * (3.0 - 2.0 * k)
        };
        paint::soft_shadow(
            p,
            rect,
            radius,
            10.0 + 10.0 * hover,
            vec2(0.0, 2.0 + 3.0 * hover),
            with_alpha(Color32::BLACK, if t.dark { 0.35 } else { 0.08 + 0.06 * hover }),
        );
        p.rect_filled(rect, cr(radius), bg);
        // a faint radial accent wash from the top
        paint::fill_convex_fn(p, &paint::rounded_rect_points(rect.shrink(1.0), radius - 1.0, 8), 5, |q: Pos2| {
            let d = ((q - rect.center_top()) / vec2(rect.width() * 0.6, rect.height())).length();
            with_alpha(t.accent, (1.0 - d).clamp(0.0, 1.0) * (0.10 + 0.08 * hover))
        });
        util::border_fn(p, rect, radius, 1.0, |q, _| t.border.lerp_to_gamma(shine, band(q) * 0.85));
        let g = util::text_galley(ui, &self.label, self.size, base);
        let pos = rect.center() - g.size() / 2.0;
        paint::galley_with(p, pos, &g, |q| base.lerp_to_gamma(shine, band(q)));
        glowkit_core::animate(ui);
        resp
    }
}

/// A high-contrast button with a flowing rainbow border and a blurred rainbow glow underneath.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::RainbowButton::new("Get Unlimited Access"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct RainbowButton {
    label: String,
    size: f32,
    colors: Vec<Color32>,
    background: Option<Color32>,
    text_color: Option<Color32>,
    radius: Option<f32>,
    speed: f32,
    min_width: Option<f32>,
}

impl RainbowButton {
    /// A rainbow button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: 15.0,
            colors: palettes::RAINBOW.to_vec(),
            background: None,
            text_color: None,
            radius: None,
            speed: 2.0,
            min_width: None,
        }
    }
    setter!(/// Font size in points.
        size: f32);
    /// Gradient stops for the border and glow (wraps around). Defaults to a rainbow.
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        if !colors.is_empty() {
            self.colors = colors.to_vec();
        }
        self
    }
    opt_setter!(/// Fill color. Defaults to the theme's primary color.
        background: Color32);
    opt_setter!(/// Label color. Defaults to the primary foreground.
        text_color: Color32);
    opt_setter!(/// Corner radius. Defaults to the theme radius.
        radius: f32);
    setter!(/// Seconds per gradient cycle.
        speed: f32);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for RainbowButton {
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
        let radius = self.radius.unwrap_or(t.radius * 1.2);
        let bg = self.background.unwrap_or(t.primary);
        let text = self.text_color.unwrap_or(t.primary_foreground);
        let p = ui.painter();
        let shift = glowkit_core::time(ui) / self.speed.max(0.1);
        let colors = &self.colors;
        // glow pooled under the bottom edge
        let n = 14;
        for i in 0..n {
            let u = (i as f32 + 0.5) / n as f32;
            let x = rect.min.x + rect.width() * (0.1 + 0.8 * u);
            let c = gradient_cyclic(colors, u * 0.8 - shift);
            let a = (if t.dark { 0.55 } else { 0.40 } + 0.2 * hover) * 0.55;
            paint::glow_ellipse(
                p,
                egui::pos2(x, rect.max.y + rect.height() * 0.05),
                vec2((rect.width() * 0.2).max(rect.height() * 1.2), rect.height() * (0.55 + 0.15 * hover)),
                with_alpha(c, a),
            );
        }
        p.rect_filled(rect, cr(radius), bg);
        util::border_fn(p, rect, radius, 2.0, |q, _| {
            let u = (q.x - rect.min.x) / rect.width();
            gradient_cyclic(colors, u * 0.8 - shift)
        });
        let g = util::text_galley(ui, &self.label, self.size, text);
        util::galley_centered(p, &g, rect.center(), text);
        glowkit_core::animate(ui);
        resp
    }
}

/// A transparent button outlined by a rotating gradient; on hover the gradient floods in.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::GradientOutlineButton::new("Explore"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct GradientOutlineButton {
    label: String,
    size: f32,
    colors: Vec<Color32>,
    radius: Option<f32>,
    border_width: f32,
    speed: f32,
    force_hover: bool,
    min_width: Option<f32>,
}

impl GradientOutlineButton {
    /// A gradient-outline button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: 15.0,
            colors: Vec::new(),
            radius: None,
            border_width: 2.0,
            speed: 4.0,
            force_hover: false,
            min_width: None,
        }
    }
    setter!(/// Font size in points.
        size: f32);
    /// Gradient stops. Defaults to the theme gradient.
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    opt_setter!(/// Corner radius. Defaults to a full pill.
        radius: f32);
    setter!(/// Border thickness in points.
        border_width: f32);
    setter!(/// Seconds per rotation of the gradient (0 = static).
        speed: f32);
    setter!(/// Render as if hovered (for showcases, or to mirror an external hover state).
        force_hover: bool);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for GradientOutlineButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = label_size(ui, &self.label, self.size, 0.0, self.min_width);
        let (rect, resp) = util::alloc_button(ui, size, &self.label);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let hover = util::hover_t(ui, resp.id, resp.hovered() || self.force_hover, 8.0);
        let press = util::press_t(ui, resp.id, resp.is_pointer_button_down_on());
        let rect = pressed_rect(rect, press);
        let colors = if self.colors.is_empty() { t.gradient.to_vec() } else { self.colors.clone() };
        let radius = self.radius.unwrap_or(rect.height() / 2.0);
        let rot = if self.speed > 0.0 { glowkit_core::time(ui) / self.speed } else { 0.0 };
        let c = rect.center();
        let aspect = vec2(1.0 / rect.width(), 1.0 / rect.height());
        let grad = |q: Pos2| {
            let d = (q - c) * aspect;
            let a = d.y.atan2(d.x) / std::f32::consts::TAU;
            gradient_cyclic(&colors, a - rot)
        };
        let p = ui.painter();
        if hover > 0.01 {
            for (i, col) in colors.iter().enumerate() {
                let x = rect.min.x + rect.width() * (i as f32 + 0.5) / colors.len() as f32;
                paint::glow_ellipse(
                    p,
                    egui::pos2(x, c.y + 4.0),
                    vec2(rect.width() * 0.35, rect.height() * 0.9),
                    with_alpha(*col, 0.22 * hover),
                );
            }
            paint::fill_convex_fn(p, &paint::rounded_rect_points(rect, radius, 10), 4, |q| {
                grad(q).gamma_multiply(hover)
            });
        }
        util::border_fn(p, rect, radius, self.border_width, |q, _| grad(q));
        let g = util::text_galley(ui, &self.label, self.size, t.foreground);
        let pos = c - g.size() / 2.0;
        let fg = Color32::WHITE;
        paint::galley_with(p, pos, &g, |q| grad(q).lerp_to_gamma(fg, hover));
        if self.speed > 0.0 {
            glowkit_core::animate(ui);
        }
        resp
    }
}
