use super::shimmer::{label_size, pressed_rect};
use super::util::{self, cr};
use egui::{pos2, vec2, Color32, FontId, Pos2, Rect, Response, Stroke, StrokeKind, Ui, Vec2, Widget};
use glowkit_core::color::{contrast_text, mix, shade, with_alpha};
use glowkit_core::{paint, Theme};

// ---------------------------------------------------------------------------------------------
// Ripple

#[derive(Clone, Copy)]
struct Ripple {
    at: Vec2,
    start: f32,
}

/// A button that emits Material-style ink ripples from wherever it is clicked.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::RippleButton::new("Click me"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct RippleButton {
    label: String,
    size: f32,
    background: Option<Color32>,
    ripple_color: Option<Color32>,
    text_color: Option<Color32>,
    radius: Option<f32>,
    duration: f32,
    auto_ripple: Option<f32>,
    min_width: Option<f32>,
}

impl RippleButton {
    /// A ripple button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: 15.0,
            background: None,
            ripple_color: None,
            text_color: None,
            radius: None,
            duration: 0.75,
            auto_ripple: None,
            min_width: None,
        }
    }
    setter!(/// Font size in points.
        size: f32);
    opt_setter!(/// Fill color. Defaults to the theme's card color with a border.
        background: Color32);
    opt_setter!(/// Ripple color. Defaults to the accent.
        ripple_color: Color32);
    opt_setter!(/// Label color. Defaults to the foreground (or a contrasting color for custom fills).
        text_color: Color32);
    opt_setter!(/// Corner radius. Defaults to the theme radius.
        radius: f32);
    setter!(/// Seconds a ripple takes to spread and fade.
        duration: f32);
    opt_setter!(/// Also ripple on its own every this many seconds (an attention-grabbing call to action).
        auto_ripple: f32);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for RippleButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = label_size(ui, &self.label, self.size, 8.0, self.min_width);
        let (rect, resp) = util::alloc_button(ui, size, &self.label);
        let id = resp.id.with("ripples");
        let time = glowkit_core::time(ui);
        let dur = self.duration.max(0.1);
        let mut ripples: Vec<Ripple> = glowkit_core::state(ui, id, Vec::new);
        if resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.any_pressed()) {
            if let Some(pos) = resp.interact_pointer_pos() {
                ripples.push(Ripple { at: pos - rect.min, start: time });
            }
        }
        if let Some(every) = self.auto_ripple {
            let last: f32 = glowkit_core::state(ui, id.with("auto"), || time - every);
            if time - last >= every.max(0.2) {
                let k = (time * 7.31).sin() * 0.5 + 0.5;
                ripples.push(Ripple { at: vec2(rect.width() * (0.2 + 0.6 * k), rect.height() * 0.5), start: time });
                glowkit_core::set_state(ui, id.with("auto"), time);
            }
            ui.ctx().request_repaint();
        }
        ripples.retain(|r| time - r.start < dur);
        if !ripples.is_empty() {
            glowkit_core::animate(ui);
        }
        glowkit_core::set_state(ui, id, ripples.clone());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let hover = util::hover_t(ui, resp.id, resp.hovered(), 10.0);
        let radius = self.radius.unwrap_or(t.radius);
        let custom = self.background.is_some();
        let bg = self.background.unwrap_or(t.card);
        let rc = self.ripple_color.unwrap_or(if custom { contrast_text(bg) } else { t.accent });
        let text = self.text_color.unwrap_or(if custom { contrast_text(bg) } else { t.foreground });
        let p = ui.painter();
        paint::soft_shadow(
            p,
            rect,
            radius,
            10.0,
            vec2(0.0, 2.0),
            with_alpha(Color32::BLACK, if t.dark { 0.3 } else { 0.07 }),
        );
        p.rect_filled(rect, cr(radius), mix(bg, rc, 0.05 * hover));
        let max_r = [rect.left_top(), rect.right_top(), rect.left_bottom(), rect.right_bottom()]
            .iter()
            .map(|c| c.distance(rect.center()))
            .fold(0.0, f32::max)
            * 2.0;
        for r in &ripples {
            let k = ((time - r.start) / dur).clamp(0.0, 1.0);
            let grow = 1.0 - (1.0 - k).powi(3);
            let alpha = (1.0 - k).powf(1.3) * if custom { 0.35 } else { 0.32 };
            util::circle_in_rounded(p, rect, radius, rect.min + r.at, max_r * grow, with_alpha(rc, alpha));
        }
        if !custom {
            p.rect_stroke(
                rect,
                cr(radius),
                Stroke::new(1.0, t.border.lerp_to_gamma(rc, 0.4 * hover)),
                StrokeKind::Inside,
            );
        }
        let g = util::text_galley(ui, &self.label, self.size, text);
        util::galley_centered(p, &g, rect.center(), text);
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Interactive hover

/// A pill with a small dot beside its label. On hover the dot swells to fill the whole button,
/// the label slides away and a new label with an arrow slides in.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::InteractiveHoverButton::new("Book a demo"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct InteractiveHoverButton {
    label: String,
    hover_label: Option<String>,
    size: f32,
    color: Option<Color32>,
    background: Option<Color32>,
    force_hover: bool,
    min_width: Option<f32>,
}

impl InteractiveHoverButton {
    /// An interactive-hover button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            hover_label: None,
            size: 15.0,
            color: None,
            background: None,
            force_hover: false,
            min_width: None,
        }
    }
    /// Label shown while hovered. Defaults to the same label.
    pub fn hover_label(mut self, label: impl Into<String>) -> Self {
        self.hover_label = Some(label.into());
        self
    }
    setter!(/// Font size in points.
        size: f32);
    opt_setter!(/// Dot / fill color. Defaults to the theme's primary color.
        color: Color32);
    opt_setter!(/// Resting fill. Defaults to the theme background.
        background: Color32);
    setter!(/// Render as if hovered (for showcases, or to mirror an external hover state).
        force_hover: bool);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for InteractiveHoverButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let hover_label = self.hover_label.clone().unwrap_or_else(|| self.label.clone());
        let longest = if hover_label.len() > self.label.len() { &hover_label } else { &self.label };
        let size = label_size(ui, longest, self.size, 36.0, self.min_width);
        let (rect, resp) = util::alloc_button(ui, size, &self.label);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let hovered = resp.hovered() || self.force_hover;
        let h = glowkit_core::anim::spring(ui, resp.id.with("h"), if hovered { 1.0 } else { 0.0 }, 160.0, 24.0)
            .clamp(0.0, 1.05);
        let press = util::press_t(ui, resp.id, resp.is_pointer_button_down_on());
        let rect = pressed_rect(rect, press);
        let radius = rect.height() / 2.0;
        let color = self.color.unwrap_or(t.primary);
        let fg = contrast_text(color);
        let bg = self.background.unwrap_or(t.background);
        let p = ui.painter();
        p.rect_filled(rect, cr(radius), bg);
        let g1 = util::text_galley(ui, &self.label, self.size, t.foreground);
        let g2 = util::text_galley(ui, &hover_label, self.size, fg);
        let dot_r = 4.0;
        let content_w = g1.size().x + dot_r * 2.0 + 10.0;
        let dot0 = pos2(rect.center().x - content_w / 2.0 + dot_r, rect.center().y);
        // the dot grows to cover the farthest corner
        let dot_c = dot0.lerp(pos2(rect.min.x + radius * 0.6, rect.center().y), h.clamp(0.0, 1.0) * 0.6);
        let far = rect.right_bottom().distance(dot_c).max(rect.right_top().distance(dot_c)) + 2.0;
        let r = egui::lerp(dot_r..=far, h.clamp(0.0, 1.0).powf(1.4));
        util::circle_in_rounded(p, rect, radius, dot_c, r, color);
        p.rect_stroke(
            rect,
            cr(radius),
            Stroke::new(1.0, t.border.lerp_to_gamma(color, h.clamp(0.0, 1.0))),
            StrokeKind::Inside,
        );
        let clip = p.with_clip_rect(rect.shrink(1.0));
        let shift = rect.width() * 0.6 * h;
        // resting label slides right & fades
        let l1 = pos2(dot0.x + dot_r + 10.0, rect.center().y - g1.size().y / 2.0) + vec2(shift, 0.0);
        clip.galley_with_override_text_color(
            l1,
            g1.clone(),
            t.foreground.gamma_multiply((1.0 - h * 1.6).clamp(0.0, 1.0)),
        );
        // hover label + arrow slide in from the right
        let total2 = g2.size().x + 22.0;
        let x2 = rect.center().x - total2 / 2.0 + (1.0 - h) * rect.width() * 0.5;
        let a2 = h.clamp(0.0, 1.0);
        clip.galley_with_override_text_color(
            pos2(x2, rect.center().y - g2.size().y / 2.0),
            g2.clone(),
            fg.gamma_multiply(a2),
        );
        util::arrow_right(&clip, pos2(x2 + g2.size().x + 14.0, rect.center().y), 12.0, 1.6, fg.gamma_multiply(a2), 1.0);
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Magnetic

/// A button that is pulled toward the pointer when it comes close, springing back when it
/// leaves. The label moves a little further than the body for a parallax feel.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::MagneticButton::new("Hover me").strength(0.4));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct MagneticButton {
    label: String,
    size: f32,
    strength: f32,
    range: f32,
    background: Option<Color32>,
    text_color: Option<Color32>,
    radius: Option<f32>,
    ambient: bool,
    min_width: Option<f32>,
}

impl MagneticButton {
    /// A magnetic button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: 15.0,
            strength: 0.35,
            range: 60.0,
            background: None,
            text_color: None,
            radius: None,
            ambient: false,
            min_width: None,
        }
    }
    setter!(/// Font size in points.
        size: f32);
    setter!(/// Fraction of the pointer offset the button follows (0..1).
        strength: f32);
    setter!(/// How far outside the button (in points) the pull starts.
        range: f32);
    opt_setter!(/// Fill color. Defaults to the theme's primary color.
        background: Color32);
    opt_setter!(/// Label color. Defaults to the primary foreground.
        text_color: Color32);
    opt_setter!(/// Corner radius. Defaults to a full pill.
        radius: f32);
    setter!(/// When the pointer is away, drift gently as if following an invisible cursor.
        ambient: bool);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for MagneticButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = label_size(ui, &self.label, self.size, 16.0, self.min_width);
        let (rect, resp) = util::alloc_button(ui, size, &self.label);
        let zone = rect.expand(self.range);
        let mut target = Vec2::ZERO;
        let pointer = ui.input(|i| i.pointer.hover_pos()).filter(|p| zone.contains(*p) && ui.clip_rect().contains(*p));
        if let Some(p) = pointer {
            target = (p - rect.center()) * self.strength;
        } else if self.ambient {
            let tm = glowkit_core::time(ui);
            let v = vec2((tm * 1.3).sin() * 0.9 + (tm * 0.7).cos() * 0.4, (tm * 1.9).cos() * 0.7)
                * vec2(rect.width() * 0.5, rect.height() * 0.8);
            target = v * self.strength;
            glowkit_core::animate(ui);
        }
        let off = glowkit_core::anim::spring2(ui, resp.id.with("mag"), target, 170.0, 13.0);
        if !ui.is_rect_visible(zone) {
            return resp;
        }
        let hover = util::hover_t(ui, resp.id, resp.hovered(), 10.0);
        let press = util::press_t(ui, resp.id, resp.is_pointer_button_down_on());
        let body = pressed_rect(rect, press).translate(off);
        let radius = self.radius.unwrap_or(rect.height() / 2.0);
        let bg = self.background.unwrap_or(t.primary);
        let text = self.text_color.unwrap_or(contrast_text(bg));
        let p = ui.painter();
        paint::soft_shadow(
            p,
            body,
            radius,
            16.0 + 8.0 * hover,
            off * 0.25 + vec2(0.0, 6.0),
            with_alpha(Color32::BLACK, if t.dark { 0.45 } else { 0.18 }),
        );
        p.rect_filled(body, cr(radius), shade(bg, if t.dark { -0.04 } else { 0.06 } * hover));
        let g = util::text_galley(ui, &self.label, self.size, text);
        util::galley_centered(p, &g, body.center() + off * 0.35, text);
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Fill hover

/// An outlined button that fills with liquid on hover: a wavy surface rises from the bottom and
/// the label flips color as the liquid passes it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::FillHoverButton::new("Dive in"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct FillHoverButton {
    label: String,
    size: f32,
    color: Option<Color32>,
    radius: Option<f32>,
    wave: f32,
    force_hover: bool,
    min_width: Option<f32>,
}

impl FillHoverButton {
    /// A liquid-fill button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: 15.0,
            color: None,
            radius: None,
            wave: 1.0,
            force_hover: false,
            min_width: None,
        }
    }
    setter!(/// Font size in points.
        size: f32);
    opt_setter!(/// Liquid / outline color. Defaults to the theme accent.
        color: Color32);
    opt_setter!(/// Corner radius. Defaults to the theme radius.
        radius: f32);
    setter!(/// Wave height multiplier (0 = flat fill).
        wave: f32);
    setter!(/// Render as if hovered (for showcases, or to mirror an external hover state).
        force_hover: bool);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for FillHoverButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = label_size(ui, &self.label, self.size, 16.0, self.min_width);
        let (rect, resp) = util::alloc_button(ui, size, &self.label);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let hovered = resp.hovered() || self.force_hover;
        let level = glowkit_core::anim::follow(ui, resp.id.with("lvl"), if hovered { 1.0 } else { 0.0 }, 3.2);
        let press = util::press_t(ui, resp.id, resp.is_pointer_button_down_on());
        let rect = pressed_rect(rect, press);
        let radius = self.radius.unwrap_or(t.radius);
        let color = self.color.unwrap_or(t.accent);
        let fg = contrast_text(color);
        let time = glowkit_core::time(ui);
        let amp = 4.0 * self.wave * (1.0 - (level * 2.0 - 1.0).powi(4)).max(0.0);
        let surface = rect.max.y + 6.0 - level * (rect.height() + 14.0);
        let wave = |x: f32| {
            let u = (x - rect.min.x) / rect.width();
            surface + amp * ((u * 9.0 + time * 4.0).sin() * 0.6 + (u * 5.0 - time * 2.6).sin() * 0.4)
        };
        let p = ui.painter();
        p.rect_filled(rect, cr(radius), if t.dark { t.background } else { t.card });
        if level > 0.001 {
            // a lighter back wave for depth
            let back = |x: f32| {
                let u = (x - rect.min.x) / rect.width();
                surface - 2.0 + amp * (u * 7.0 - time * 3.3 + 1.7).sin()
            };
            util::fill_below(p, rect, radius, 48, back, |_| with_alpha(color, 0.35));
            util::fill_below(p, rect, radius, 48, wave, |v| shade(color, 0.05 * (1.0 - v)));
            glowkit_core::animate(ui);
        }
        p.rect_stroke(rect, cr(radius), Stroke::new(1.5, color), StrokeKind::Inside);
        let g = util::text_galley(ui, &self.label, self.size, color);
        let pos = rect.center() - g.size() / 2.0;
        paint::galley_with(p, pos, &g, |q| if q.y > wave(q.x) { fg } else { color });
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Arrow slide

/// A button with a trailing arrow chip: on hover the label nudges left, the chip widens and its
/// arrow shoots out to the right while a fresh one slides in.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::ArrowSlideButton::new("Continue"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ArrowSlideButton {
    label: String,
    size: f32,
    background: Option<Color32>,
    text_color: Option<Color32>,
    chip_color: Option<Color32>,
    force_hover: bool,
    min_width: Option<f32>,
}

impl ArrowSlideButton {
    /// An arrow-slide button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: 15.0,
            background: None,
            text_color: None,
            chip_color: None,
            force_hover: false,
            min_width: None,
        }
    }
    setter!(/// Font size in points.
        size: f32);
    opt_setter!(/// Fill color. Defaults to the theme's primary color.
        background: Color32);
    opt_setter!(/// Label color. Defaults to the primary foreground.
        text_color: Color32);
    opt_setter!(/// Arrow chip color. Defaults to the accent.
        chip_color: Color32);
    setter!(/// Render as if hovered (for showcases, or to mirror an external hover state).
        force_hover: bool);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for ArrowSlideButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = label_size(ui, &self.label, self.size, 34.0, self.min_width);
        let (rect, resp) = util::alloc_button(ui, size, &self.label);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let hovered = resp.hovered() || self.force_hover;
        let h = glowkit_core::anim::spring(ui, resp.id.with("h"), if hovered { 1.0 } else { 0.0 }, 220.0, 22.0);
        let press = util::press_t(ui, resp.id, resp.is_pointer_button_down_on());
        let rect = pressed_rect(rect, press);
        let radius = rect.height() / 2.0;
        let bg = self.background.unwrap_or(t.primary);
        let text = self.text_color.unwrap_or(contrast_text(bg));
        let chip = self.chip_color.unwrap_or(t.accent);
        let p = ui.painter();
        p.rect_filled(rect, cr(radius), bg);
        let pad = 5.0;
        let chip_d = rect.height() - pad * 2.0;
        let chip_w = chip_d + h.max(0.0) * 10.0;
        let chip_rect = Rect::from_min_size(pos2(rect.max.x - pad - chip_w, rect.min.y + pad), vec2(chip_w, chip_d));
        p.rect_filled(chip_rect, cr(chip_d / 2.0), chip);
        let g = util::text_galley(ui, &self.label, self.size, text);
        let label_c = pos2((rect.min.x + chip_rect.min.x) / 2.0 + 4.0 - 6.0 * h, rect.center().y);
        util::galley_centered(p, &g, label_c, text);
        let clip = p.with_clip_rect(chip_rect.shrink(2.0));
        let ac = contrast_text(chip);
        let span = chip_w * 0.9;
        let k = h.clamp(0.0, 1.0);
        util::arrow_right(&clip, chip_rect.center() + vec2(k * span, 0.0), 12.0, 1.8, ac, 1.0);
        util::arrow_right(&clip, chip_rect.center() + vec2((k - 1.0) * span, 0.0), 12.0, 1.8, ac, 1.0);
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// 3D push

/// Visual style of a [`PushButton3D`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PushStyle {
    /// A raised key with a darker edge and soft shadow that pops up on hover and sinks on press.
    #[default]
    Soft,
    /// Neobrutalism: thick outline and a hard offset shadow the face slides onto when pressed.
    Brutal,
}

/// A chunky 3D button you physically push: the face pops up on hover and sinks on press, with
/// a bouncy spring.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{PushButton3D, PushStyle};
/// ui.add(PushButton3D::new("Push me"));
/// ui.add(PushButton3D::new("Brutal").style(PushStyle::Brutal));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PushButton3D {
    label: String,
    size: f32,
    color: Option<Color32>,
    text_color: Option<Color32>,
    style: PushStyle,
    depth: f32,
    radius: Option<f32>,
    min_width: Option<f32>,
}

impl PushButton3D {
    /// A 3D push button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: 15.0,
            color: None,
            text_color: None,
            style: PushStyle::Soft,
            depth: 6.0,
            radius: None,
            min_width: None,
        }
    }
    setter!(/// Font size in points.
        size: f32);
    opt_setter!(/// Face color. Defaults to the accent (soft) or the theme's warning yellow (brutal).
        color: Color32);
    opt_setter!(/// Label color. Defaults to a contrasting color.
        text_color: Color32);
    setter!(/// Visual style.
        style: PushStyle);
    setter!(/// How far the face sits above its base, in points.
        depth: f32);
    opt_setter!(/// Corner radius. Defaults to the theme radius.
        radius: f32);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for PushButton3D {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = label_size(ui, &self.label, self.size, 12.0, self.min_width) + vec2(0.0, self.depth + 2.0);
        let (rect, resp) = util::alloc_button(ui, size, &self.label);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let d = self.depth;
        let down = resp.is_pointer_button_down_on();
        let target = if down {
            d * 0.25
        } else if resp.hovered() {
            -2.0
        } else {
            0.0
        };
        let (k, damp) = if down { (900.0, 30.0) } else { (420.0, 14.0) };
        let lift = glowkit_core::anim::spring(ui, resp.id.with("push"), target, k, damp);
        let p = ui.painter();
        match self.style {
            PushStyle::Soft => {
                let radius = self.radius.unwrap_or(t.radius);
                let color = self.color.unwrap_or(t.accent);
                let text = self.text_color.unwrap_or(contrast_text(color));
                let base = Rect::from_min_max(pos2(rect.min.x, rect.min.y + d), rect.max);
                let face = base.translate(vec2(0.0, -d + lift + 2.0));
                let face = Rect::from_min_size(face.min, vec2(base.width(), base.height() - 2.0));
                paint::soft_shadow(
                    p,
                    base,
                    radius,
                    10.0 + 6.0 * (1.0 - lift / d).clamp(0.0, 2.0),
                    vec2(0.0, 3.0 + (2.0 - lift).max(0.0)),
                    with_alpha(Color32::BLACK, if t.dark { 0.5 } else { 0.22 }),
                );
                // edge: gradient from dark to darker for a curved side
                let edge = Rect::from_min_max(pos2(base.min.x, face.center().y), base.max);
                paint::gradient_rect(
                    p,
                    edge,
                    radius,
                    0.0,
                    &[shade(color, -0.22), shade(color, -0.12), shade(color, -0.12), shade(color, -0.22)],
                );
                paint::gradient_rect(p, face, radius, 90.0, &[shade(color, 0.05), color]);
                p.rect_stroke(face, cr(radius), Stroke::new(1.0, with_alpha(Color32::WHITE, 0.18)), StrokeKind::Inside);
                let g = util::text_galley(ui, &self.label, self.size, text);
                util::galley_centered(p, &g, face.center(), text);
            }
            PushStyle::Brutal => {
                let radius = self.radius.unwrap_or(t.radius * 0.5);
                let color = self.color.unwrap_or(glowkit_core::hex("#FDE047"));
                let text = self.text_color.unwrap_or(Color32::BLACK);
                let ink = if t.dark { Color32::WHITE } else { Color32::BLACK };
                let off = d * 0.8;
                let full = Rect::from_min_max(rect.min, rect.max - vec2(off, off));
                let face_shift = (lift / (d * 0.25)).clamp(0.0, 1.0); // 0 = up, 1 = pressed
                let shadow = full.translate(vec2(off, off));
                let hover_out = (-lift).max(0.0) * 0.5;
                let face = full.translate(vec2(off, off) * face_shift - vec2(hover_out, hover_out));
                p.rect_filled(shadow, cr(radius), ink);
                p.rect_filled(face, cr(radius), color);
                p.rect_stroke(face, cr(radius), Stroke::new(2.0, Color32::BLACK), StrokeKind::Inside);
                let g = util::text_galley(ui, &self.label, self.size, text);
                util::galley_centered(p, &g, face.center(), text);
            }
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Pixel

/// A retro 8-bit button with stair-stepped pixel corners, a chunky bevel and a monospace label.
/// Pressing it drops the face onto its pixel shadow.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::PixelButton::new("PRESS START"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PixelButton {
    label: String,
    size: f32,
    color: Option<Color32>,
    text_color: Option<Color32>,
    border_color: Option<Color32>,
    pixel: f32,
    blink: bool,
    min_width: Option<f32>,
}

impl PixelButton {
    /// A pixel-art button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: 14.0,
            color: None,
            text_color: None,
            border_color: None,
            pixel: 3.0,
            blink: false,
            min_width: None,
        }
    }
    setter!(/// Font size in points.
        size: f32);
    opt_setter!(/// Face color. Defaults to the accent.
        color: Color32);
    opt_setter!(/// Label color. Defaults to a contrasting color.
        text_color: Color32);
    opt_setter!(/// Outline color. Defaults to the foreground.
        border_color: Color32);
    setter!(/// Size of one "pixel" in points.
        pixel: f32);
    setter!(/// Show a blinking arcade cursor before the label.
        blink: bool);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for PixelButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let u = self.pixel.max(1.0);
        let font = FontId::monospace(self.size);
        let galley_probe = ui.painter().layout_no_wrap(self.label.clone(), font.clone(), Color32::WHITE);
        let extra = if self.blink { self.size } else { 0.0 };
        let w = (galley_probe.size().x + self.size * 2.6 + extra + u * 4.0).max(self.min_width.unwrap_or(0.0));
        let h = (self.size * 2.6 + u * 3.0).round();
        let (rect, resp) = util::alloc_button(ui, vec2(w.round(), h), &self.label);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let snap = |v: f32| (v / u).round() * u;
        let down = resp.is_pointer_button_down_on();
        let hovered = resp.hovered();
        let color = self.color.unwrap_or(t.accent);
        let face_col = if hovered { shade(color, 0.05) } else { color };
        let text = self.text_color.unwrap_or(contrast_text(color));
        let ink = self.border_color.unwrap_or(t.foreground);
        let p = ui.painter();
        let x0 = rect.min.x;
        let y0 = rect.min.y;
        let bw = snap(rect.width() - u);
        let bh = snap(rect.height() - u);
        let off = if down { u } else { 0.0 };
        // pixel rectangle helper: notched outline of size (w, h) at origin o
        let block = |o: Pos2, w: f32, h: f32, fill: Color32, outline: Color32| {
            // outline: rows of pixels with 1-pixel notched corners
            p.rect_filled(Rect::from_min_size(o + vec2(u, 0.0), vec2(w - 2.0 * u, h)), 0.0, outline);
            p.rect_filled(Rect::from_min_size(o + vec2(0.0, u), vec2(w, h - 2.0 * u)), 0.0, outline);
            p.rect_filled(Rect::from_min_size(o + vec2(u, u), vec2(w - 2.0 * u, h - 2.0 * u)), 0.0, fill);
        };
        let origin = pos2(snap(x0), snap(y0));
        if !down {
            // drop shadow
            block(origin + vec2(u, u), bw, bh, with_alpha(ink, 0.35), with_alpha(ink, 0.35));
        }
        let o = origin + vec2(off, off);
        block(o, bw, bh, face_col, ink);
        // bevel: highlight on top/left, shade bottom/right (inside the outline)
        let inner = Rect::from_min_size(o + vec2(u, u), vec2(bw - 2.0 * u, bh - 2.0 * u));
        let hi = with_alpha(Color32::WHITE, if down { 0.1 } else { 0.38 });
        let lo = with_alpha(Color32::BLACK, if down { 0.1 } else { 0.28 });
        p.rect_filled(Rect::from_min_size(inner.min, vec2(inner.width() - u, u)), 0.0, hi);
        p.rect_filled(Rect::from_min_size(inner.min, vec2(u, inner.height() - u)), 0.0, hi);
        p.rect_filled(Rect::from_min_size(pos2(inner.min.x + u, inner.max.y - u), vec2(inner.width() - u, u)), 0.0, lo);
        p.rect_filled(
            Rect::from_min_size(pos2(inner.max.x - u, inner.min.y + u), vec2(u, inner.height() - u)),
            0.0,
            lo,
        );
        let g = ui.painter().layout_no_wrap(self.label.clone(), font, text);
        let c = inner.center() + vec2(extra / 2.0, 0.0);
        util::galley_centered(p, &g, c, text);
        if self.blink {
            let time = glowkit_core::time(ui);
            if (time * 2.0).fract() < 0.55 || hovered {
                // a little pixel triangle cursor: columns of 5, 3 and 1 pixels
                let px = (u * 0.75).max(1.0);
                let cx = (c.x - g.size().x / 2.0 - self.size).round();
                for i in 0..3 {
                    let hh = (5 - 2 * i) as f32 * px;
                    p.rect_filled(
                        Rect::from_min_size(pos2(cx + i as f32 * px, (c.y - hh / 2.0).round()), vec2(px, hh)),
                        0.0,
                        text,
                    );
                }
            }
            glowkit_core::animate(ui);
        }
        resp
    }
}
