use crate::util;
use egui::{pos2, vec2, Color32, Response, Sense, Ui, Widget};
use glowkit_core::{anim::approach, mix, Theme};

/// How emphasis falls off with distance from the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Falloff {
    /// Straight line down to zero at the radius.
    Linear,
    /// Smooth bell curve (default).
    #[default]
    Gaussian,
    /// Sharp peak right under the pointer.
    Exponential,
}

impl Falloff {
    fn apply(self, d: f32) -> f32 {
        let d = d.max(0.0);
        match self {
            Falloff::Linear => (1.0 - d).max(0.0),
            Falloff::Gaussian => (-(d * d) * 4.0).exp() * (d < 1.6) as u8 as f32,
            Falloff::Exponential => (-d * 4.0).exp() * (d < 1.6) as u8 as f32,
        }
    }
}

/// Letters near the pointer swell, thicken and change color, pushing their neighbors aside,
/// like variable-font "proximity" and "pressure" text.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::ProximityText::new("Hover me!").size(64.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ProximityText {
    text: String,
    size: f32,
    color: Option<Color32>,
    highlight: Option<Color32>,
    radius: Option<f32>,
    max_scale: f32,
    max_weight: f32,
    falloff: Falloff,
    auto: bool,
}

impl ProximityText {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 56.0,
            color: None,
            highlight: None,
            radius: None,
            max_scale: 1.3,
            max_weight: 1.0,
            falloff: Falloff::Gaussian,
            auto: false,
        }
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Resting text color (defaults to the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Color letters blend toward when emphasized (default: no color change).
    pub fn highlight(mut self, c: Color32) -> Self {
        self.highlight = Some(c);
        self
    }
    /// Radius of influence in points (default 2 x font size).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = Some(r);
        self
    }
    /// Scale of a fully emphasized letter (default 1.3).
    pub fn max_scale(mut self, s: f32) -> Self {
        self.max_scale = s.max(1.0);
        self
    }
    /// Extra weight of a fully emphasized letter, 0..2 (default 1).
    pub fn max_weight(mut self, w: f32) -> Self {
        self.max_weight = w.clamp(0.0, 2.0);
        self
    }
    /// Falloff curve.
    pub fn falloff(mut self, f: Falloff) -> Self {
        self.falloff = f;
        self
    }
    /// When the pointer is elsewhere, let a virtual pointer drift across the text.
    pub fn auto(mut self, auto: bool) -> Self {
        self.auto = auto;
        self
    }
}

impl Widget for ProximityText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let glyphs = crate::util::glyphs(ui.painter(), &self.text, font.clone(), None);
        let base_w = util::bold_px(self.size) * 0.5;
        let extra_room = glyphs.size.x * (self.max_scale - 1.0) * 0.35;
        let (rect, resp) = ui.allocate_exact_size(glyphs.size + vec2(extra_room, self.size * 0.3), Sense::hover());
        let radius = self.radius.unwrap_or(self.size * 2.0);
        let time = glowkit_core::time(ui);
        let text_rect = egui::Rect::from_center_size(rect.center(), glyphs.size);
        let pointer = resp.hover_pos().or_else(|| {
            self.auto.then(|| {
                pos2(
                    text_rect.center().x + text_rect.width() * 0.55 * (time * 0.8).sin(),
                    text_rect.center().y + self.size * 0.2 * (time * 1.7).cos(),
                )
            })
        });
        if self.auto {
            glowkit_core::animate(ui);
        }
        let dt = glowkit_core::dt(ui);
        let n = glyphs.glyphs.len();
        let falloff = self.falloff;
        let emph = glowkit_core::with_state(
            ui,
            resp.id.with("prox"),
            || vec![0.0f32; n],
            |e| {
                e.resize(n, 0.0);
                for (v, g) in e.iter_mut().zip(&glyphs.glyphs) {
                    let c = text_rect.min + g.offset + g.size / 2.0;
                    let target = pointer.map(|p| falloff.apply(p.distance(c) / radius)).unwrap_or(0.0);
                    *v = approach(*v, target, 14.0, dt);
                }
                e.clone()
            },
        );
        if emph.iter().any(|v| *v > 1e-3) || resp.hovered() {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter();
        let color = self.color.unwrap_or(t.foreground);
        // letters push each other apart as they grow
        let grow: Vec<f32> =
            glyphs.glyphs.iter().zip(&emph).map(|(g, e)| g.size.x * (self.max_scale - 1.0) * e).collect();
        let total: f32 = grow.iter().sum();
        let mut acc = -total / 2.0;
        for (g, (e, gr)) in glyphs.glyphs.iter().zip(emph.iter().zip(&grow)) {
            let shift = acc + gr / 2.0;
            acc += gr;
            if g.ch.is_whitespace() {
                continue;
            }
            let scale = 1.0 + (self.max_scale - 1.0) * e;
            let c = match self.highlight {
                Some(h) => mix(color, h, *e),
                None => color,
            };
            let w = base_w + e * self.max_weight * self.size * 0.05;
            let center = text_rect.min + g.offset + g.size / 2.0 + vec2(shift - w / 2.0, -e * self.size * 0.04);
            util::paint_char_xf(painter, g.ch, &font, center, scale, 0.0, c, w);
            if *e > 0.25 {
                util::paint_char_xf(
                    painter,
                    g.ch,
                    &font,
                    center + vec2(0.0, w * 0.4),
                    scale,
                    0.0,
                    c.gamma_multiply(0.8),
                    w,
                );
            }
        }
        resp
    }
}
