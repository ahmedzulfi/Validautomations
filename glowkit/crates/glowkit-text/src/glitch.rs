use crate::util;
use egui::{pos2, vec2, Color32, Rect, Response, Sense, Ui, Widget};
use glowkit_core::{color::hex, Rng, Theme};

/// Digital glitch: the text is cut into horizontal slices that jump sideways, with red and cyan
/// channel copies splitting off. Calm most of the time, with short violent bursts.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::GlitchText::new("GLITCH").size(96.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct GlitchText {
    text: String,
    size: f32,
    color: Option<Color32>,
    left: Color32,
    right: Color32,
    bold: bool,
    speed: f32,
    intensity: f32,
    hover_only: bool,
    mono: bool,
}

impl GlitchText {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 72.0,
            color: None,
            left: hex("#FF2D6F"),
            right: hex("#00E5FF"),
            bold: true,
            speed: 1.0,
            intensity: 1.0,
            hover_only: false,
            mono: false,
        }
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Main text color (defaults to the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Colors of the two split channels (default hot pink and cyan).
    pub fn channels(mut self, left: Color32, right: Color32) -> Self {
        self.left = left;
        self.right = right;
        self
    }
    /// Fake-bold the text (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Animation speed multiplier.
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed.max(0.05);
        self
    }
    /// How far slices jump (1 = default).
    pub fn intensity(mut self, k: f32) -> Self {
        self.intensity = k.max(0.0);
        self
    }
    /// Only glitch while hovered (static, slightly split text otherwise).
    pub fn hover_only(mut self, on: bool) -> Self {
        self.hover_only = on;
        self
    }
    /// Use the monospace font.
    pub fn monospace(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }
}

impl Widget for GlitchText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, self.mono);
        let bold = util::weight(self.bold, self.size);
        let painter = ui.painter().clone();
        let galley = util::layout(&painter, self.text.clone(), &font);
        let pad = self.size * 0.15;
        let (rect, resp) = ui.allocate_exact_size(galley.size() + vec2(bold + pad * 2.0, 0.0), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        let active = !self.hover_only || resp.hovered();
        let on = crate::util::ramp(ui, id.with("on"), active, 0.15);
        if on > 0.0 {
            glowkit_core::animate(ui);
        }
        let time = glowkit_core::time(ui) * self.speed;
        let tick = (time * 14.0) as u64;
        // bursts: every ~0.6 s window may or may not glitch hard
        let window = (time / 0.6) as u64;
        let mut wr = Rng::new(window.wrapping_mul(7919) ^ id.value());
        let burst_start = wr.range(0.0, 0.4);
        let burst_len = wr.range(0.15, 0.4);
        let in_window = (time / 0.6).fract() * 0.6;
        let bursting = wr.bool(0.75) && in_window > burst_start && in_window < burst_start + burst_len;
        let level = on * self.intensity * if bursting { 1.0 } else { 0.2 };

        let color = self.color.unwrap_or(t.foreground);
        let origin = pos2(rect.min.x + pad, rect.min.y);
        let h = galley.size().y;
        let mut r = Rng::new(tick.wrapping_mul(0x9E37_79B9) ^ id.value());
        let jitter = if bursting { vec2(r.range(-1.0, 1.0), r.range(-1.0, 1.0)) * self.size * 0.025 * level } else { vec2(0.0, 0.0) };
        // slice boundaries
        let cuts = if bursting { 7 } else { 3 };
        let mut ys: Vec<f32> = (0..cuts).map(|_| r.f32()).collect();
        ys.push(0.0);
        ys.push(1.0);
        ys.sort_by(f32::total_cmp);
        let split = self.size * 0.035 * (0.4 + level * 1.6) * on.max(0.35);
        let blend = if t.dark { 0.85 } else { 0.7 };
        for w in ys.windows(2) {
            let (a, b) = (w[0], w[1]);
            if b - a < 1e-3 {
                continue;
            }
            let shifted = r.bool(if bursting { 0.6 } else { 0.25 });
            let sx = if shifted { r.range(-1.0, 1.0) * self.size * 0.14 * level } else { 0.0 };
            let band = Rect::from_min_max(pos2(rect.min.x - self.size, rect.min.y + a * h), pos2(rect.max.x + self.size, rect.min.y + b * h));
            let p = painter.with_clip_rect(band.intersect(ui.clip_rect()));
            let pos = origin + jitter + vec2(sx, 0.0);
            let ch_split = split * if shifted { 1.6 } else { 1.0 };
            util::paint_galley(&p, pos - vec2(ch_split, 0.0), &galley, self.left.gamma_multiply(blend), bold);
            util::paint_galley(&p, pos + vec2(ch_split, 0.0), &galley, self.right.gamma_multiply(blend), bold);
            util::paint_galley(&p, pos, &galley, color, bold);
        }
        // a thin scan line during bursts
        if bursting && r.bool(0.5) {
            let y = rect.min.y + r.f32() * h;
            let c = if r.bool(0.5) { self.left } else { self.right };
            painter.rect_filled(Rect::from_min_size(pos2(rect.min.x + r.range(0.0, rect.width() * 0.4), y), vec2(rect.width() * r.range(0.2, 0.6), 2.0)), 0.0, c.gamma_multiply(0.6));
        }
        resp
    }
}
