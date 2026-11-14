//! LED dot-matrix display.

use crate::common;
use egui::{pos2, vec2, Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// A built-in animated pattern for a [`DotMatrix`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MatrixPattern {
    /// A sine wave sweeping across.
    #[default]
    Wave,
    /// Concentric rings rippling out from the center.
    Ripple,
    /// Falling drops.
    Rain,
    /// A comet orbiting the center (loading).
    Loader,
    /// Twinkling random dots.
    Sparkle,
}

/// What a [`DotMatrix`] displays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MatrixContent<'a> {
    /// Text in a 5 x 7 pixel font; scrolls as a marquee when wider than the display.
    Text(&'a str),
    /// Level meter: one value (`0..=1`) per column group, drawn as bars from the bottom.
    Levels(&'a [f32]),
    /// An animated pattern.
    Pattern(MatrixPattern),
    /// Raw brightness per dot (`0..=1`), row-major, `cols * rows` long.
    Pixels(&'a [f32]),
}

/// An LED dot-matrix display: a grid of round "LEDs" that light up with a soft glow and fade
/// out with phosphor-like persistence. Shows scrolling text in a built-in 5 x 7 pixel font, live
/// level bars, animated patterns (wave, ripple, rain, loader, sparkle) or your own pixels.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::{DotMatrix, MatrixContent, MatrixPattern};
/// ui.add(DotMatrix::new(48, 9).content(MatrixContent::Text("HELLO WORLD")));
/// ui.add(DotMatrix::new(16, 16).content(MatrixContent::Pattern(MatrixPattern::Ripple)));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DotMatrix<'a> {
    cols: usize,
    rows: usize,
    content: MatrixContent<'a>,
    pitch: f32,
    color: Option<Color32>,
    gradient: bool,
    speed: f32,
    persistence: f32,
    square: bool,
    bezel: bool,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> DotMatrix<'a> {
    /// A display of `cols` x `rows` dots.
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols: cols.max(1),
            rows: rows.max(1),
            content: MatrixContent::Pattern(MatrixPattern::Wave),
            pitch: 9.0,
            color: None,
            gradient: false,
            speed: 1.0,
            persistence: 0.12,
            square: false,
            bezel: true,
            id_salt: None,
        }
    }
    /// What to display.
    pub fn content(mut self, c: MatrixContent<'a>) -> Self {
        self.content = c;
        self
    }
    /// Distance between dot centers in points (default 9).
    pub fn pitch(mut self, p: f32) -> Self {
        self.pitch = p.max(2.0);
        self
    }
    /// LED color (default: theme accent).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Color the LEDs with the theme gradient across the display.
    pub fn gradient(mut self, g: bool) -> Self {
        self.gradient = g;
        self
    }
    /// Animation / marquee speed multiplier (default 1).
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Seconds a dot takes to fade out (default 0.12; 0 = instant).
    pub fn persistence(mut self, s: f32) -> Self {
        self.persistence = s.max(0.0);
        self
    }
    /// Square pixels instead of round LEDs.
    pub fn square(mut self, s: bool) -> Self {
        self.square = s;
        self
    }
    /// Draw the dark display bezel behind the dots (default on).
    pub fn bezel(mut self, b: bool) -> Self {
        self.bezel = b;
        self
    }
    /// Stable id for the persistence buffer.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }

    fn target(&self, time: f32) -> Vec<f32> {
        let (cols, rows) = (self.cols, self.rows);
        let mut out = vec![0.0f32; cols * rows];
        let tm = time * self.speed;
        match self.content {
            MatrixContent::Text(s) => {
                let glyph_cols = text_columns(s);
                let w = glyph_cols.len();
                let y0 = (rows as i32 - 7) / 2;
                let x_off = if w <= cols {
                    (cols as i32 - w as i32) / 2
                } else {
                    let span = (w + cols) as f32;
                    cols as i32 - ((tm * 14.0) % span) as i32
                };
                for (gx, bits) in glyph_cols.iter().enumerate() {
                    let x = gx as i32 + x_off;
                    if x < 0 || x >= cols as i32 {
                        continue;
                    }
                    for r in 0..7 {
                        let y = y0 + r;
                        if y >= 0 && (y as usize) < rows && bits & (1 << r) != 0 {
                            out[y as usize * cols + x as usize] = 1.0;
                        }
                    }
                }
            }
            MatrixContent::Levels(levels) => {
                if !levels.is_empty() {
                    for c in 0..cols {
                        let li = c * levels.len() / cols;
                        let v = levels[li].clamp(0.0, 1.0);
                        let lit = (v * rows as f32).round() as usize;
                        for r in 0..lit.min(rows) {
                            out[(rows - 1 - r) * cols + c] = 1.0;
                        }
                    }
                }
            }
            MatrixContent::Pixels(px) => {
                for (o, v) in out.iter_mut().zip(px) {
                    *o = v.clamp(0.0, 1.0);
                }
            }
            MatrixContent::Pattern(p) => {
                let cx = (cols as f32 - 1.0) / 2.0;
                let cy = (rows as f32 - 1.0) / 2.0;
                for r in 0..rows {
                    for c in 0..cols {
                        let (x, y) = (c as f32, r as f32);
                        let v = match p {
                            MatrixPattern::Wave => {
                                let wy = cy + (x * 0.35 - tm * 4.0).sin() * cy * 0.85;
                                (1.0 - (y - wy).abs() / 1.2).clamp(0.0, 1.0)
                            }
                            MatrixPattern::Ripple => {
                                let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
                                let k = (d * 0.9 - tm * 5.0).sin();
                                ((k - 0.55) / 0.45).clamp(0.0, 1.0)
                            }
                            MatrixPattern::Rain => {
                                let seed = hash(c as u32);
                                let speed = 6.0 + (seed % 7) as f32;
                                let len = rows as f32 + 6.0;
                                let head = (tm * speed + (seed % 97) as f32) % len;
                                let d = head - y;
                                if (0.0..4.0).contains(&d) {
                                    1.0 - d / 4.0
                                } else {
                                    0.0
                                }
                            }
                            MatrixPattern::Loader => {
                                let a = (y - cy).atan2(x - cx);
                                let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
                                let ring = cx.min(cy) * 0.75;
                                if (d - ring).abs() < 0.9 {
                                    let head = (tm * 5.0).rem_euclid(std::f32::consts::TAU);
                                    let diff = (head - a).rem_euclid(std::f32::consts::TAU);
                                    (1.0 - diff / 3.5).max(0.08)
                                } else {
                                    0.0
                                }
                            }
                            MatrixPattern::Sparkle => {
                                let cell = hash((r * cols + c) as u32);
                                let phase = (cell % 1000) as f32 / 1000.0;
                                let f = ((tm * 0.9 + phase) % 1.0) * 6.0;
                                if f < 1.0 {
                                    1.0 - f
                                } else {
                                    0.0
                                }
                            }
                        };
                        out[r * cols + c] = v;
                    }
                }
            }
        }
        out
    }
}

impl Widget for DotMatrix<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let pad = if self.bezel { self.pitch * 1.2 } else { 0.0 };
        let size = vec2(self.cols as f32 * self.pitch + pad * 2.0, self.rows as f32 * self.pitch + pad * 2.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        glowkit_core::animate(ui);
        let id = common::widget_id(ui, self.id_salt).with("dot_matrix");
        let time = glowkit_core::time(ui);
        let dt = glowkit_core::dt(ui);
        let target = self.target(time);
        let n = target.len();
        let decay = if self.persistence <= 0.0 { 1.0 } else { (dt / self.persistence).min(1.0) };
        let lit = glowkit_core::with_state(
            ui,
            id,
            || vec![0.0f32; n],
            |buf| {
                if buf.len() != n {
                    *buf = vec![0.0; n];
                }
                for (b, tv) in buf.iter_mut().zip(&target) {
                    // rise instantly, fall with persistence
                    *b = if *tv >= *b { *tv } else { *b + (*tv - *b) * decay };
                }
                buf.clone()
            },
        );
        let p = ui.painter();
        let screen =
            if t.dark { mix(t.background, Color32::BLACK, 0.5) } else { mix(t.foreground, Color32::BLACK, 0.4) };
        if self.bezel {
            let cr = CornerRadius::same((self.pitch * 1.1) as u8);
            p.rect(
                rect,
                cr,
                screen,
                Stroke::new(1.0, if t.dark { t.border } else { mix(t.foreground, Color32::WHITE, 0.2) }),
                StrokeKind::Inside,
            );
            // glass sheen
            let sheen = Rect::from_min_max(rect.min + vec2(2.0, 2.0), pos2(rect.max.x - 2.0, rect.center().y));
            glowkit_core::paint::gradient_rect(
                p,
                sheen,
                cr.nw as f32,
                90.0,
                &[Color32::from_white_alpha(10), Color32::TRANSPARENT],
            );
        }
        let on_base = self.color.unwrap_or(t.accent);
        let dark_screen = self.bezel || t.dark;
        let off = if dark_screen { with_alpha(on_base, 0.10) } else { with_alpha(t.muted_foreground, 0.16) };
        let r = self.pitch * 0.36;
        let origin = rect.min + vec2(pad + self.pitch / 2.0, pad + self.pitch / 2.0);
        // glows first, then dots on top
        for row in 0..self.rows {
            for c in 0..self.cols {
                let b = lit[row * self.cols + c];
                if b > 0.35 {
                    let on = self.led_color(&t, on_base, c);
                    let center = origin + vec2(c as f32 * self.pitch, row as f32 * self.pitch);
                    p.circle_filled(center, r * 2.1, with_alpha(on, 0.09 * b));
                }
            }
        }
        for row in 0..self.rows {
            for c in 0..self.cols {
                let b = lit[row * self.cols + c];
                let center = origin + vec2(c as f32 * self.pitch, row as f32 * self.pitch);
                let on = self.led_color(&t, on_base, c);
                let col = if b <= 0.01 { off } else { off.lerp_to_gamma(on, b) };
                if self.square {
                    p.rect_filled(Rect::from_center_size(center, vec2(r * 2.0, r * 2.0)), CornerRadius::same(1), col);
                } else {
                    p.circle_filled(center, r, col);
                }
                if b > 0.6 && dark_screen {
                    p.circle_filled(
                        center - vec2(r * 0.25, r * 0.25),
                        r * 0.35,
                        with_alpha(Color32::WHITE, 0.35 * (b - 0.6) / 0.4),
                    );
                }
            }
        }
        resp
    }
}

impl DotMatrix<'_> {
    fn led_color(&self, t: &Theme, base: Color32, col: usize) -> Color32 {
        if self.gradient {
            glowkit_core::color::gradient(&t.gradient, col as f32 / (self.cols.max(2) - 1) as f32)
        } else {
            base
        }
    }
}

fn hash(x: u32) -> u32 {
    let mut h = x.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    h ^= h >> 15;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^ (h >> 13)
}

/// Rows of the 5 x 7 glyph for `c` (bit 4 = leftmost column), or `None` if unsupported.
pub(crate) fn glyph(c: char) -> Option<[u8; 7]> {
    let g: [u8; 7] = match c.to_ascii_uppercase() {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        ' ' => [0; 7],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100],
        '?' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b00000, 0b00100],
        '.' => [0, 0, 0, 0, 0, 0b01100, 0b01100],
        ',' => [0, 0, 0, 0, 0b01100, 0b00100, 0b01000],
        ':' => [0, 0b01100, 0b01100, 0, 0b01100, 0b01100, 0],
        '\'' => [0b00100, 0b00100, 0b01000, 0, 0, 0, 0],
        '-' => [0, 0, 0, 0b11111, 0, 0, 0],
        '+' => [0, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0],
        '%' => [0b11000, 0b11001, 0b00010, 0b00100, 0b01000, 0b10011, 0b00011],
        '/' => [0b00001, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b10000],
        '>' => [0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000],
        '<' => [0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010],
        '#' => [0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010],
        '*' | '♥' => [0, 0b01010, 0b11111, 0b11111, 0b01110, 0b00100, 0],
        _ => return None,
    };
    Some(g)
}

/// Columns of `text` rendered in the 5 x 7 font with one blank column between glyphs. Each
/// column is a bitmask with bit `r` set when row `r` (top = 0) is lit.
pub(crate) fn text_columns(text: &str) -> Vec<u8> {
    let mut cols = Vec::new();
    for ch in text.chars() {
        let g = glyph(ch).unwrap_or(glyph('?').unwrap_or([0; 7]));
        for x in 0..5 {
            let mut m = 0u8;
            for (r, row) in g.iter().enumerate() {
                if row & (1 << (4 - x)) != 0 {
                    m |= 1 << r;
                }
            }
            cols.push(m);
        }
        cols.push(0);
    }
    cols.pop();
    cols
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_covers_alphanumerics() {
        for c in ('A'..='Z').chain('0'..='9') {
            let g = glyph(c).expect("glyph");
            assert!(g.iter().any(|r| *r != 0), "{c} is blank");
            assert!(g.iter().all(|r| *r < 32));
        }
    }

    #[test]
    fn text_width() {
        assert_eq!(text_columns("AB").len(), 11);
        // 'I' has its stem in the middle column
        let i = text_columns("I");
        assert_eq!(i[2], 0b111_1111);
    }
}
