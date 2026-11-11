use crate::util;
use egui::{pos2, vec2, Color32, Pos2, Rect, Response, Sense, Ui, Widget};
use glowkit_core::{anim::{approach, spring2}, paint, Theme};

/// A sentence where one word at a time is sharp and framed by animated corner brackets, while the
/// other words stay blurred. The focus walks through the words (or follows the pointer).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::TrueFocus::new("True Focus").size(56.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TrueFocus {
    text: String,
    size: f32,
    color: Option<Color32>,
    frame_color: Option<Color32>,
    bold: bool,
    blur: f32,
    interval: f32,
    manual: bool,
}

impl TrueFocus {
    /// Create the effect for `text` (split on whitespace).
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), size: 56.0, color: None, frame_color: None, bold: true, blur: 5.0, interval: 1.4, manual: false }
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Text color (defaults to the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Corner bracket color (defaults to the theme accent).
    pub fn frame_color(mut self, c: Color32) -> Self {
        self.frame_color = Some(c);
        self
    }
    /// Fake-bold the text (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Blur radius of unfocused words in points.
    pub fn blur(mut self, points: f32) -> Self {
        self.blur = points.max(0.0);
        self
    }
    /// Seconds each word stays in focus in automatic mode.
    pub fn interval(mut self, seconds: f32) -> Self {
        self.interval = seconds.max(0.2);
        self
    }
    /// Focus the hovered word instead of cycling automatically.
    pub fn manual(mut self, manual: bool) -> Self {
        self.manual = manual;
        self
    }
}

fn corners(painter: &egui::Painter, r: Rect, arm: f32, width: f32, color: Color32) {
    let s = egui::Stroke::new(width, color);
    let h = width / 2.0;
    let pts: [(Pos2, f32, f32); 4] = [(r.left_top(), 1.0, 1.0), (r.right_top(), -1.0, 1.0), (r.right_bottom(), -1.0, -1.0), (r.left_bottom(), 1.0, -1.0)];
    for (c, sx, sy) in pts {
        let c = c + vec2(-sx * h, -sy * h);
        painter.line(vec![c + vec2(0.0, sy * arm), c, c + vec2(sx * arm, 0.0)], s);
    }
}

impl Widget for TrueFocus {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let painter = ui.painter().clone();
        let words: Vec<_> = self.text.split_whitespace().map(|w| util::layout(&painter, w, &font)).collect();
        let gap = self.size * 0.45;
        let pad = vec2(self.size * 0.16, self.size * 0.04);
        let w: f32 = words.iter().map(|g| g.size().x + bold).sum::<f32>() + gap * words.len().saturating_sub(1) as f32;
        let h = words.iter().map(|g| g.size().y).fold(self.size, f32::max);
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h) + pad * 2.0 + vec2(0.0, 6.0), Sense::hover());
        let inner = rect.shrink2(pad + vec2(0.0, 3.0));
        let mut x = inner.min.x;
        let rects: Vec<Rect> = words
            .iter()
            .map(|g| {
                let r = Rect::from_min_size(pos2(x, inner.min.y), vec2(g.size().x + bold, g.size().y));
                x += g.size().x + bold + gap;
                r
            })
            .collect();
        let n = words.len().max(1);
        let id = resp.id;
        let time = glowkit_core::time(ui);
        let active = if self.manual {
            let hovered = resp.hover_pos().and_then(|p| rects.iter().position(|r| r.expand2(vec2(gap / 2.0, pad.y * 2.0)).contains(p)));
            let last = glowkit_core::state(ui, id.with("focus_last"), || 0usize);
            let a = hovered.unwrap_or(last).min(n - 1);
            glowkit_core::set_state(ui, id.with("focus_last"), a);
            a
        } else {
            glowkit_core::animate(ui);
            ((time / self.interval) as usize) % n
        };
        let dt = glowkit_core::dt(ui);
        let sharp = glowkit_core::with_state(ui, id.with("sharp"), || vec![0.0f32; n], |v| {
            v.resize(n, 0.0);
            for (i, s) in v.iter_mut().enumerate() {
                *s = approach(*s, if i == active { 1.0 } else { 0.0 }, 9.0, dt);
            }
            v.clone()
        });
        if sharp.iter().enumerate().any(|(i, s)| (s - if i == active { 1.0 } else { 0.0 }).abs() > 1e-3) {
            ui.ctx().request_repaint();
        }
        let Some(target) = rects.get(active).copied() else { return resp };
        let target = target.expand2(pad + vec2(0.0, 3.0));
        let a = spring2(ui, id.with("fmin"), target.min.to_vec2(), 210.0, 24.0);
        let b = spring2(ui, id.with("fmax"), target.max.to_vec2(), 210.0, 24.0);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let color = self.color.unwrap_or(t.foreground);
        for ((g, r), s) in words.iter().zip(&rects).zip(&sharp) {
            let c = color.gamma_multiply(0.55 + 0.45 * s);
            util::paint_blurred(&painter, r.min, g, c, self.blur * (1.0 - s), bold);
        }
        let fc = self.frame_color.unwrap_or(t.accent);
        let frame = Rect::from_min_max(a.to_pos2(), b.to_pos2());
        let arm = (self.size * 0.28).min(frame.height() * 0.45);
        let width = (self.size * 0.055).clamp(2.0, 4.0);
        for (i, k) in [(6.0, 0.10), (3.0, 0.18)] {
            corners(&painter, frame, arm + i * 0.3, width + i, fc.gamma_multiply(k));
        }
        corners(&painter, frame, arm, width, fc);
        paint::glow(&painter, frame.left_top(), width * 4.0, fc.gamma_multiply(0.15));
        paint::glow(&painter, frame.right_bottom(), width * 4.0, fc.gamma_multiply(0.15));
        resp
    }
}
