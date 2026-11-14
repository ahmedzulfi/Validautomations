//! Per-glyph text layout, so text can be animated letter by letter (or word by word).

use egui::epaint::TextShape;
use egui::{pos2, vec2, Color32, FontId, Galley, Painter, Pos2, Rect, Shape, Vec2};
use std::sync::Arc;

/// One laid-out character.
#[derive(Clone, Copy, Debug)]
pub struct GlyphBox {
    pub ch: char,
    /// Index of the character in the original string (by `char`, not byte).
    pub index: usize,
    /// Index of the word this character belongs to.
    pub word: usize,
    /// Line number.
    pub line: usize,
    /// Top-left of the glyph's logical box, relative to the text block's top-left.
    pub offset: Vec2,
    pub size: Vec2,
}

/// A block of text broken into glyph boxes.
#[derive(Clone, Debug)]
pub struct Glyphs {
    pub glyphs: Vec<GlyphBox>,
    pub size: Vec2,
    pub font: FontId,
    pub galley: Arc<Galley>,
}

impl Glyphs {
    /// Lay out `text` (optionally wrapped at `wrap_width`) and record each glyph's box.
    pub fn layout(painter: &Painter, text: &str, font: FontId, wrap_width: Option<f32>) -> Self {
        let galley = match wrap_width {
            Some(w) => painter.layout(text.to_owned(), font.clone(), Color32::WHITE, w),
            None => painter.layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE),
        };
        let mut glyphs = Vec::new();
        let mut index = 0;
        let mut word = 0;
        let mut prev_space = true;
        for (line, row) in galley.rows.iter().enumerate() {
            for g in &row.row.glyphs {
                let is_space = g.chr.is_whitespace();
                if !is_space && prev_space && index > 0 {
                    word += 1;
                }
                prev_space = is_space;
                let r = g.logical_rect();
                glyphs.push(GlyphBox {
                    ch: g.chr,
                    index,
                    word,
                    line,
                    offset: row.pos.to_vec2() + r.min.to_vec2(),
                    size: vec2(g.advance_width, g.font_height),
                });
                index += 1;
            }
            if row.ends_with_newline {
                prev_space = true;
            }
        }
        split_ligatures(painter, &mut glyphs, &font);
        let size = galley.size();
        Self { glyphs, size, font, galley }
    }

    /// Number of words.
    pub fn word_count(&self) -> usize {
        self.glyphs.last().map(|g| g.word + 1).unwrap_or(0)
    }

    /// Number of non-whitespace characters.
    pub fn visible_count(&self) -> usize {
        self.glyphs.iter().filter(|g| !g.ch.is_whitespace()).count()
    }
}

/// egui shapes ligatures ("ff", "fi", ...) into one glyph followed by zero-width glyphs. Effects
/// that paint letter by letter need a box per character, so split the ligature's advance across
/// its characters in proportion to their standalone widths.
fn split_ligatures(painter: &Painter, glyphs: &mut [GlyphBox], font: &FontId) {
    let n = glyphs.len();
    let mut i = 0;
    while i < n {
        let mut j = i + 1;
        while j < n && glyphs[j].size.x <= 1e-3 && glyphs[j].line == glyphs[i].line && !glyphs[j].ch.is_whitespace() {
            j += 1;
        }
        if j > i + 1 {
            let widths: Vec<f32> = (i..j)
                .map(|k| painter.layout_no_wrap(glyphs[k].ch.to_string(), font.clone(), Color32::WHITE).size().x)
                .collect();
            let sum = widths.iter().sum::<f32>().max(1e-3);
            let total = glyphs[i].size.x;
            let mut x = glyphs[i].offset.x;
            for (k, w) in (i..j).zip(widths) {
                let w = w * total / sum;
                glyphs[k].offset.x = x;
                glyphs[k].size.x = w;
                x += w;
            }
        }
        i = j;
    }
}

/// Paint a single character with its top-left at `pos`, optional rotation (radians, around the
/// glyph center), uniform scale and opacity.
pub fn paint_char(painter: &Painter, ch: char, pos: Pos2, font: &FontId, color: Color32, scale: f32, angle: f32) {
    if ch.is_whitespace() || color.a() == 0 {
        return;
    }
    let f = FontId::new((font.size * scale).max(0.5), font.family.clone());
    let galley = painter.layout_no_wrap(ch.to_string(), f, color);
    paint_galley_transformed(painter, galley, pos, scale, angle, color, font.size);
}

fn paint_galley_transformed(
    painter: &Painter,
    galley: Arc<Galley>,
    pos: Pos2,
    scale: f32,
    angle: f32,
    color: Color32,
    base: f32,
) {
    let sz = galley.size();
    // keep the glyph centered on where the unscaled glyph would be
    let base_w = sz.x / scale.max(1e-3);
    let center = pos + vec2(base_w / 2.0, base * 0.6);
    if angle.abs() < 1e-4 {
        let p = center - sz / 2.0;
        painter.galley(pos2(p.x, p.y), galley, color);
    } else {
        let (s, c) = angle.sin_cos();
        let half = sz / 2.0;
        let rotated = vec2(half.x * c - half.y * s, half.x * s + half.y * c);
        let origin = center - rotated;
        painter.add(Shape::Text(TextShape::new(origin, galley, color).with_angle(angle)));
    }
}

/// Paint a string with its top-left at `pos` with a rotation (radians, around the text center).
pub fn paint_rotated(painter: &Painter, text: &str, center: Pos2, font: FontId, color: Color32, angle: f32) -> Rect {
    let galley = painter.layout_no_wrap(text.to_owned(), font, color);
    let sz = galley.size();
    let (s, c) = angle.sin_cos();
    let half = sz / 2.0;
    let rotated = vec2(half.x * c - half.y * s, half.x * s + half.y * c);
    let origin = center - rotated;
    painter.add(Shape::Text(TextShape::new(origin, galley, color).with_angle(angle)));
    Rect::from_center_size(center, sz)
}

/// Characters used by scramble / decrypt effects.
pub const SCRAMBLE_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=[]{}<>?";

/// Pick a pseudo-random scramble character for (seed, index, tick).
pub fn scramble_char(seed: u64, index: usize, tick: u64, charset: &str) -> char {
    let chars: Vec<char> = charset.chars().collect();
    if chars.is_empty() {
        return '?';
    }
    let mut r = crate::Rng::new(seed ^ (index as u64).wrapping_mul(0x9E37) ^ tick.wrapping_mul(0x85EB_CA6B));
    chars[r.below(chars.len())]
}
