//! Glyph backgrounds: glitching letter walls and falling code rain.

use crate::{background, h01, radial_mask, Base};
use egui::{vec2, Color32, FontId, Galley, Painter, Rect, Ui};
use glowkit_core::Rng;
use std::sync::Arc;

const GLITCH_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ!@#$&*()-_+=/[]{};:<>,0123456789";
const RAIN_CHARS: &str = "0123456789ABCDEFZ:.=*+-<>|¦ﾊﾐﾋｰｳｼﾅﾓﾆｻﾜﾂｵﾘｱﾎﾃﾏｹﾒｴｶｷﾑﾕﾗｾﾈｽﾀﾇﾍ";

/// Lay out each character once (colorless) so it can be painted in any color.
fn glyph_set(painter: &Painter, chars: &str, font: &FontId) -> Vec<Arc<Galley>> {
    chars
        .chars()
        .filter(|c| painter.fonts_mut(|f| f.has_glyph(font, *c)))
        .map(|c| painter.layout_no_wrap(c.to_string(), font.clone(), Color32::PLACEHOLDER))
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Letter glitch
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct GlitchState {
    cells: Vec<(u16, Color32, Color32, f32)>,
    dims: (usize, usize),
    timer: f32,
    rng: Option<Rng>,
}

/// A wall of monospace letters that constantly glitch: random characters swap out and fade to
/// new colors, framed by a dark vignette. A cyberpunk terminal backdrop.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::LetterGlitch;
/// use glowkit_core::hex;
/// ui.add(LetterGlitch::new().colors(&[hex("#2b4539"), hex("#61dca3"), hex("#61b3dc")]));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct LetterGlitch {
    base: Base,
    colors: Option<Vec<Color32>>,
    font_size: f32,
    speed: f32,
    amount: f32,
    outer_vignette: bool,
    center_vignette: bool,
}

impl Default for LetterGlitch {
    fn default() -> Self {
        Self::new()
    }
}

impl LetterGlitch {
    /// Glitching letters in the theme's colors.
    pub fn new() -> Self {
        Self {
            base: Base::default(),
            colors: None,
            font_size: 15.0,
            speed: 1.0,
            amount: 0.05,
            outer_vignette: true,
            center_vignette: false,
        }
    }
    /// Letter colors (default: a dim and two bright theme colors).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        if !colors.is_empty() {
            self.colors = Some(colors.to_vec());
        }
        self
    }
    /// Font size in points (default 15).
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size.clamp(6.0, 64.0);
        self
    }
    /// Glitch speed multiplier (default 1 = 20 glitch steps per second).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed.max(0.01);
        self
    }
    /// Fraction of letters changing per step (default 0.05).
    pub fn amount(mut self, fraction: f32) -> Self {
        self.amount = fraction.clamp(0.0, 1.0);
        self
    }
    /// Darken the edges (default true).
    pub fn outer_vignette(mut self, on: bool) -> Self {
        self.outer_vignette = on;
        self
    }
    /// Darken the center, to make room for a headline (default false).
    pub fn center_vignette(mut self, on: bool) -> Self {
        self.center_vignette = on;
        self
    }

    /// Paint the letters into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_letter_glitch") else { return };
        let t = &f.theme;
        let colors = self.colors.clone().unwrap_or_else(|| {
            if t.dark {
                vec![glowkit_core::mix(f.bg, t.accent, 0.35), t.chart[1], t.accent]
            } else {
                vec![glowkit_core::mix(f.bg, t.accent, 0.3), t.chart[1], t.accent]
            }
        });
        let font = FontId::monospace(self.font_size);
        let glyphs = glyph_set(&f.painter, GLITCH_CHARS, &font);
        if glyphs.is_empty() {
            return;
        }
        let cw = self.font_size * 0.62;
        let ch = self.font_size * 1.25;
        let cols = (rect.width() / cw).ceil() as usize;
        let rows = (rect.height() / ch).ceil() as usize;
        let (dt, speed, amount) = (f.dt, self.speed, self.amount);
        let ng = glyphs.len();
        let cells = glowkit_core::with_state(ui, f.id, GlitchState::default, |s| {
            let rng = s.rng.get_or_insert_with(|| Rng::new(1337));
            if s.dims != (cols, rows) {
                s.cells = (0..cols * rows)
                    .map(|_| {
                        let c = *rng.pick(&colors);
                        (rng.below(ng) as u16, c, c, 1.0)
                    })
                    .collect();
                s.dims = (cols, rows);
            }
            s.timer += dt * speed;
            let step = 0.05;
            let mut steps = 0;
            while s.timer >= step && steps < 4 {
                s.timer -= step;
                steps += 1;
                let n = ((s.cells.len() as f32 * amount) as usize).max(1);
                for _ in 0..n {
                    let i = rng.below(s.cells.len());
                    let cell = &mut s.cells[i];
                    let cur = cell.1.lerp_to_gamma(cell.2, cell.3.min(1.0));
                    *cell = (rng.below(ng) as u16, cur, *rng.pick(&colors), 0.0);
                }
            }
            for c in &mut s.cells {
                c.3 = (c.3 + dt * 8.0).min(1.0);
            }
            s.cells.clone()
        });
        for j in 0..rows {
            for i in 0..cols {
                let (g, from, to, k) = cells[j * cols + i];
                let pos = rect.min + vec2(i as f32 * cw, j as f32 * ch);
                f.painter.galley(pos, glyphs[g as usize % ng].clone(), from.lerp_to_gamma(to, k));
            }
        }
        if self.outer_vignette {
            radial_mask(&f.painter, rect, f.bg, 0.2, 1.2);
        }
        if self.center_vignette {
            let c = rect.center();
            glowkit_core::paint::glow_ellipse(&f.painter, c, rect.size() * 0.55, f.bg.gamma_multiply(1.0));
        }
    }
}

background!(LetterGlitch);

// ---------------------------------------------------------------------------------------------
// Matrix rain
// ---------------------------------------------------------------------------------------------

/// Columns of glyphs raining down the screen: each drop has a bright head and a fading trail,
/// with characters flickering as they fall.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::MatrixRain;
/// ui.add(MatrixRain::new().font_size(14.0).speed(1.2));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct MatrixRain {
    base: Base,
    color: Option<Color32>,
    head: Option<Color32>,
    font_size: f32,
    speed: f32,
    density: f32,
}

impl Default for MatrixRain {
    fn default() -> Self {
        Self::new()
    }
}

impl MatrixRain {
    /// Green code rain.
    pub fn new() -> Self {
        Self { base: Base::default(), color: None, head: None, font_size: 14.0, speed: 1.0, density: 0.8 }
    }
    /// Trail color (default: the theme's success green).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Color of the leading glyph (default: near-white tint of the trail color).
    pub fn head_color(mut self, c: Color32) -> Self {
        self.head = Some(c);
        self
    }
    /// Glyph size in points (default 14).
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size.clamp(6.0, 48.0);
        self
    }
    /// Fall speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Fraction of columns with a drop at any time (default 0.8).
    pub fn density(mut self, d: f32) -> Self {
        self.density = d.clamp(0.0, 1.0);
        self
    }

    /// Paint the rain into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_matrix_rain") else { return };
        let t = &f.theme;
        let dark = f.dark();
        let color = self.color.unwrap_or(t.success);
        let head = self.head.unwrap_or_else(|| {
            if dark {
                glowkit_core::mix(color, Color32::WHITE, 0.8)
            } else {
                glowkit_core::color::shade(color, -0.15)
            }
        });
        let font = FontId::monospace(self.font_size);
        let glyphs = glyph_set(&f.painter, RAIN_CHARS, &font);
        if glyphs.is_empty() {
            return;
        }
        let ng = glyphs.len() as f32;
        let cw = self.font_size * 0.75;
        let ch = self.font_size * 1.15;
        let cols = (rect.width() / cw).ceil() as u32;
        let rows = (rect.height() / ch).ceil() as i32;
        let time = f.time * self.speed;
        for c in 0..cols {
            for lane in 0..2u32 {
                let id = c * 2 + lane;
                if h01(id, 1) > self.density * if lane == 0 { 1.0 } else { 0.45 } {
                    continue;
                }
                let speed = 7.0 + 14.0 * h01(id, 2);
                let trail = 6 + (h01(id, 3) * 22.0) as i32;
                let cycle = (rows + trail) as f32 + 4.0 + 20.0 * h01(id, 4);
                let headf = (time * speed + h01(id, 5) * cycle).rem_euclid(cycle);
                let hr = headf.floor() as i32;
                let depth = 0.55 + 0.45 * h01(id, 6);
                for k in 0..trail {
                    let r = hr - k;
                    if r < 0 || r >= rows {
                        continue;
                    }
                    let rate = 2.0 + 6.0 * h01(c * 131 + r as u32, 7);
                    let gi = (h01(c * 977 + r as u32 * 13, (time * rate) as u32) * ng) as usize;
                    let pos = rect.min + vec2(c as f32 * cw, r as f32 * ch);
                    let a = (1.0 - k as f32 / trail as f32).powf(1.4) * depth;
                    let col = if k == 0 { head } else { color.gamma_multiply(a) };
                    f.painter.galley(pos, glyphs[gi.min(glyphs.len() - 1)].clone(), col);
                }
            }
        }
    }
}

background!(MatrixRain);
