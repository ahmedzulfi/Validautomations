use crate::raster::{self, Coverage};
use crate::util;
use egui::{pos2, vec2, Color32, FontId, Response, Sense, Ui, Widget};
use glowkit_core::{color::gradient_cyclic, paint::galley_with, Theme};
use std::sync::Arc;

#[derive(Clone)]
struct AsciiCache {
    key: u64,
    cov: Arc<Coverage>,
}

/// Default ramp of characters from empty to full ink.
const RAMP: &str = " .:-=+*#%@";

/// Big text drawn as animated ASCII art: the letters are sampled onto a grid of monospace
/// characters (denser symbols where there is more ink), rippled by a flowing wave distortion and
/// tinted with a drifting gradient. The pointer pushes a lens of extra distortion around.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::AsciiText::new("HELLO").size(160.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AsciiText {
    text: String,
    size: f32,
    cell: f32,
    ramp: String,
    colors: Vec<Color32>,
    wave: f32,
    speed: f32,
    glow: bool,
    interactive: bool,
}

impl AsciiText {
    /// Render `text` (keep it short: a word or two) as ASCII art.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 150.0,
            cell: 9.0,
            ramp: RAMP.into(),
            colors: Vec::new(),
            wave: 1.0,
            speed: 1.0,
            glow: true,
            interactive: true,
        }
    }
    /// Height of the source letters in points (default 150).
    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(20.0);
        self
    }
    /// Size of one ASCII character cell in points (default 9; smaller is finer and costlier).
    pub fn cell(mut self, pts: f32) -> Self {
        self.cell = pts.clamp(5.0, 32.0);
        self
    }
    /// Characters from empty to full ink (default `" .:-=+*#%@"`).
    pub fn ramp(mut self, chars: impl Into<String>) -> Self {
        let s: String = chars.into();
        if s.chars().count() >= 2 {
            self.ramp = s;
        }
        self
    }
    /// Gradient colors flowing across the art (default: the theme gradient).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    /// Strength of the wave distortion, 0 = still (default 1).
    pub fn wave(mut self, amount: f32) -> Self {
        self.wave = amount.max(0.0);
        self
    }
    /// Animation speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Paint a faint glow of the letters behind the characters (default true).
    pub fn glow(mut self, on: bool) -> Self {
        self.glow = on;
        self
    }
    /// Whether the pointer bends the art (default true).
    pub fn interactive(mut self, on: bool) -> Self {
        self.interactive = on;
        self
    }
}

impl Widget for AsciiText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let big = FontId::proportional(self.size);
        let galley = util::layout(ui.painter(), self.text.as_str(), &big);
        let thick = self.size * 0.035;
        let mono = FontId::monospace(self.cell * 1.18);
        let char_w = util::layout_char(ui.painter(), 'M', &mono).size().x.max(1.0);
        let cell = vec2(char_w, self.cell * 1.18 * 1.05);
        let pad = vec2(cell.x * 3.0, cell.y * 1.5);
        let src = galley.size();
        let (rect, resp) = ui.allocate_exact_size(src + pad * 2.0, Sense::hover());
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        // sample the ink once per text/size (thickened by drawing the galley a little wider)
        let key = raster::key(&self.text, &[self.size, cell.x, cell.y, thick]);
        let id = resp.id.with("ascii");
        let cached: Option<AsciiCache> = ui.data(|d| d.get_temp(id));
        let cov = match cached {
            Some(c) if c.key == key => c.cov,
            _ => {
                let fine = 3.0;
                let raw = raster::coverage(ui.painter(), &galley, src, vec2(fine, fine));
                // dilate with a small disk so the art reads as heavy display type
                let rr = (thick / fine).round().max(1.0) as i32;
                let mut data = raw.data.clone();
                for y in 0..raw.rows as i32 {
                    for x in 0..raw.cols as i32 {
                        let mut m = 0.0f32;
                        for dy in -rr..=rr {
                            for dx in -rr..=rr {
                                if dx * dx + dy * dy <= rr * rr {
                                    m = m.max(raw.at(x + dx, y + dy));
                                }
                            }
                        }
                        data[y as usize * raw.cols + x as usize] = m;
                    }
                }
                let cov = Arc::new(Coverage { data, ..raw });
                ui.data_mut(|d| d.insert_temp(id, AsciiCache { key, cov: cov.clone() }));
                cov
            }
        };
        let time = glowkit_core::time(ui) * self.speed;
        let pointer = if self.interactive { resp.hover_pos() } else { None };
        let painter = ui.painter_at(rect);
        let origin = rect.min + pad;
        let colors: Vec<Color32> = if self.colors.is_empty() { t.gradient.to_vec() } else { self.colors.clone() };
        if self.glow {
            let gc = if t.dark { 0.05 } else { 0.022 };
            for i in 0..8 {
                let a = i as f32 / 8.0 * std::f32::consts::TAU;
                galley_with(&painter, origin + vec2(a.cos(), a.sin()) * thick, &galley, |p| {
                    let u = (p.x - rect.min.x) / rect.width();
                    gradient_cyclic(&colors, u * 0.8 - time * 0.06).gamma_multiply(gc)
                });
            }
        }
        let ramp: Vec<char> = self.ramp.chars().collect();
        let cols = (rect.width() / cell.x).floor() as usize;
        let rows = (rect.height() / cell.y).floor() as usize;
        let x0 = rect.min.x + (rect.width() - cols as f32 * cell.x) / 2.0;
        let y0 = rect.min.y + (rect.height() - rows as f32 * cell.y) / 2.0;
        let amp = self.wave * self.size * 0.06;
        let mut line = String::with_capacity(cols * 4);
        for r in 0..rows {
            line.clear();
            let cy = y0 + (r as f32 + 0.5) * cell.y;
            let mut any = false;
            for c in 0..cols {
                let cx = x0 + (c as f32 + 0.5) * cell.x;
                // flowing wave displacement + a second, finer ripple
                let mut d = vec2(
                    amp * (cy * 0.045 + time * 2.2).sin() + amp * 0.35 * (cy * 0.13 - time * 3.1).sin(),
                    amp * 0.45 * (cx * 0.03 + time * 1.7).sin(),
                );
                if let Some(p) = pointer {
                    let v = pos2(cx, cy) - p;
                    let fall = (-(v.length_sq()) / (self.size * self.size * 0.35)).exp();
                    d += v.normalized() * fall * self.size * 0.18 * (0.6 + 0.4 * (time * 6.0).sin());
                }
                let local = pos2(cx, cy) + d - origin;
                let v = cov.sample(local);
                // subtle flicker so the texture feels alive
                let flick = 0.06 * ((c as f32 * 12.9898 + r as f32 * 78.233 + (time * 8.0).floor()).sin() * 43758.5).fract();
                let v = (v + flick * (v > 0.05) as u8 as f32).clamp(0.0, 1.0);
                let idx = ((v * (ramp.len() - 1) as f32).round() as usize).min(ramp.len() - 1);
                if idx > 0 {
                    any = true;
                }
                line.push(ramp[idx]);
            }
            if !any {
                continue;
            }
            let g = painter.layout_no_wrap(line.clone(), mono.clone(), Color32::WHITE);
            let pos = pos2(x0, cy - g.size().y / 2.0);
            galley_with(&painter, pos, &g, |p| {
                let u = (p.x - rect.min.x) / rect.width();
                let w = (p.y - rect.min.y) / rect.height();
                gradient_cyclic(&colors, u * 0.8 + w * 0.25 - time * 0.06)
            });
        }
        resp
    }
}
