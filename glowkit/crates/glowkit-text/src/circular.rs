use crate::util;
use egui::{vec2, Color32, Response, Sense, Ui, Widget};
use glowkit_core::{anim::approach, Theme};
use std::f32::consts::TAU;

/// What [`CircularText`] does while hovered.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum HoverSpin {
    /// Spin faster (default, 4x).
    #[default]
    SpeedUp,
    /// Spin slower (0.25x).
    SlowDown,
    /// Stop spinning.
    Pause,
    /// Spin the other way, fast.
    Reverse,
    /// Ignore hover.
    Ignore,
}

impl HoverSpin {
    fn factor(self) -> f32 {
        match self {
            HoverSpin::SpeedUp => 4.0,
            HoverSpin::SlowDown => 0.25,
            HoverSpin::Pause => 0.0,
            HoverSpin::Reverse => -3.0,
            HoverSpin::Ignore => 1.0,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Spin {
    angle: f32,
    factor: f32,
}

/// Text laid out around a circle that slowly rotates, spinning faster when hovered.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::CircularText::new("GLOWKIT * MADE IN RUST * ").center_text("✦"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CircularText {
    text: String,
    size: f32,
    radius: Option<f32>,
    color: Option<Color32>,
    center: String,
    center_color: Option<Color32>,
    bold: bool,
    period: f32,
    hover: HoverSpin,
}

impl CircularText {
    /// Create the ring for `text` (it is spread evenly over the full circle).
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 18.0,
            radius: None,
            color: None,
            center: String::new(),
            center_color: None,
            bold: true,
            period: 18.0,
            hover: HoverSpin::SpeedUp,
        }
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Ring radius in points (defaults to whatever spaces the letters naturally).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = Some(r);
        self
    }
    /// Letter color (defaults to the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Text or icon drawn in the middle of the ring.
    pub fn center_text(mut self, s: impl Into<String>) -> Self {
        self.center = s.into();
        self
    }
    /// Color of the center text (defaults to the theme accent).
    pub fn center_color(mut self, c: Color32) -> Self {
        self.center_color = Some(c);
        self
    }
    /// Fake-bold the letters (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Seconds per revolution (negative spins counter-clockwise, 0 = still).
    pub fn period(mut self, seconds: f32) -> Self {
        self.period = seconds;
        self
    }
    /// Hover behaviour.
    pub fn on_hover(mut self, h: HoverSpin) -> Self {
        self.hover = h;
        self
    }
}

impl Widget for CircularText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let glyphs = crate::util::glyphs(ui.painter(), &self.text, font.clone(), None);
        let total = glyphs.size.x.max(1.0);
        let radius = self.radius.unwrap_or(total / TAU * 1.3).max(self.size);
        let side = 2.0 * (radius + self.size * 0.9);
        let (rect, resp) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
        let dt = glowkit_core::dt(ui);
        let base = if self.period.abs() > 1e-3 { TAU / self.period } else { 0.0 };
        let target = if resp.hovered() { self.hover.factor() } else { 1.0 };
        let target_speed = if base == 0.0 && resp.hovered() && self.hover != HoverSpin::Ignore { TAU / 6.0 * target.signum() } else { base * target };
        let spin = glowkit_core::with_state(ui, resp.id.with("spin"), || Spin { angle: 0.0, factor: base }, |s| {
            s.factor = approach(s.factor, target_speed, 5.0, dt);
            s.angle = (s.angle + s.factor * dt) % TAU;
            *s
        });
        if spin.factor.abs() > 1e-4 || (spin.factor - target_speed).abs() > 1e-4 {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter();
        let color = self.color.unwrap_or(t.foreground);
        let c = rect.center();
        for g in &glyphs.glyphs {
            if g.ch.is_whitespace() {
                continue;
            }
            let a = spin.angle + (g.offset.x + g.size.x / 2.0) / total * TAU;
            let pos = c + vec2(a.sin(), -a.cos()) * radius;
            util::paint_char_xf(painter, g.ch, &font, pos, 1.0, a, color, bold);
        }
        if !self.center.is_empty() {
            let cf = util::font(radius * 0.55, false);
            let g = util::layout(painter, self.center.clone(), &cf);
            let ccol = self.center_color.unwrap_or(t.accent);
            glowkit_core::paint::glow(painter, c, radius * 0.6, ccol.gamma_multiply(0.25));
            util::paint_galley(painter, c - g.size() / 2.0, &g, ccol, 0.0);
        }
        resp
    }
}
