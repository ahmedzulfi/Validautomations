#![allow(clippy::excessive_precision)] // OKLab matrices are published reference constants
//! Color helpers: hex parsing, HSV, OKLab mixing, gradients, CSS output.
//!
//! A straight port of `glowkit_core::color` without the egui dependency.

use std::fmt;

/// An 8-bit sRGB color with straight (unmultiplied) alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);

    /// Opaque color.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    /// Color with alpha.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    /// `[r, g, b, a]`.
    pub const fn to_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
    /// Channels as `0..=1` floats.
    pub fn to_f32(self) -> [f32; 4] {
        [self.r as f32 / 255.0, self.g as f32 / 255.0, self.b as f32 / 255.0, self.a as f32 / 255.0]
    }
    /// CSS color: `#rrggbb` when opaque, otherwise `rgb(r g b / a)`.
    pub fn to_css(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("rgb({} {} {} / {:.3})", self.r, self.g, self.b, self.a as f32 / 255.0)
        }
    }
    /// Same color with alpha `a` in `0..=1`.
    pub fn alpha(self, a: f32) -> Self {
        with_alpha(self, a)
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_css())
    }
}

/// Parse `#rgb`, `#rrggbb` or `#rrggbbaa` at compile time or runtime. Panics on bad input.
pub const fn hex(s: &str) -> Color {
    let b = s.as_bytes();
    let start = if !b.is_empty() && b[0] == b'#' { 1 } else { 0 };
    let n = b.len() - start;
    const fn h(c: u8) -> u8 {
        match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            b'A'..=b'F' => c - b'A' + 10,
            _ => panic!("invalid hex digit"),
        }
    }
    if n == 3 {
        Color::rgb(h(b[start]) * 17, h(b[start + 1]) * 17, h(b[start + 2]) * 17)
    } else if n == 6 || n == 8 {
        let r = h(b[start]) * 16 + h(b[start + 1]);
        let g = h(b[start + 2]) * 16 + h(b[start + 3]);
        let bb = h(b[start + 4]) * 16 + h(b[start + 5]);
        let a = if n == 8 { h(b[start + 6]) * 16 + h(b[start + 7]) } else { 255 };
        Color::rgba(r, g, bb, a)
    } else {
        panic!("hex color must be #rgb, #rrggbb or #rrggbbaa")
    }
}

/// HSV to color. `h` in degrees (any range), `s` and `v` in `0..=1`.
pub fn hsv(h: f32, s: f32, v: f32) -> Color {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    Color::rgb(((r + m) * 255.0).round() as u8, ((g + m) * 255.0).round() as u8, ((b + m) * 255.0).round() as u8)
}

/// Set the alpha of a color, `a` in `0..=1`.
pub fn with_alpha(c: Color, a: f32) -> Color {
    Color { a: (a.clamp(0.0, 1.0) * 255.0).round() as u8, ..c }
}

/// Multiply the existing alpha by `a`.
pub fn fade(c: Color, a: f32) -> Color {
    Color { a: (c.a as f32 * a.clamp(0.0, 1.0)).round() as u8, ..c }
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Convert to OKLab `[L, a, b, alpha]`.
pub fn to_oklab(c: Color) -> [f32; 4] {
    let r = srgb_to_linear(c.r as f32 / 255.0);
    let g = srgb_to_linear(c.g as f32 / 255.0);
    let b = srgb_to_linear(c.b as f32 / 255.0);
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
        c.a as f32 / 255.0,
    ]
}

/// Convert from OKLab `[L, a, b, alpha]`.
pub fn from_oklab(lab: [f32; 4]) -> Color {
    let [ll, aa, bb, alpha] = lab;
    let l = (ll + 0.3963377774 * aa + 0.2158037573 * bb).powi(3);
    let m = (ll - 0.1055613458 * aa - 0.0638541728 * bb).powi(3);
    let s = (ll - 0.0894841775 * aa - 1.2914855480 * bb).powi(3);
    let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let b = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;
    Color::rgba(
        (linear_to_srgb(r) * 255.0).round() as u8,
        (linear_to_srgb(g) * 255.0).round() as u8,
        (linear_to_srgb(b) * 255.0).round() as u8,
        (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

/// Perceptual mix of two colors (OKLab), `t` in `0..=1`.
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let la = to_oklab(a);
    let lb = to_oklab(b);
    from_oklab([
        la[0] + (lb[0] - la[0]) * t,
        la[1] + (lb[1] - la[1]) * t,
        la[2] + (lb[2] - la[2]) * t,
        la[3] + (lb[3] - la[3]) * t,
    ])
}

/// Sample a multi-stop gradient (evenly spaced stops) at `t` in `0..=1`.
pub fn gradient(stops: &[Color], t: f32) -> Color {
    match stops.len() {
        0 => Color::TRANSPARENT,
        1 => stops[0],
        n => {
            let t = t.clamp(0.0, 1.0) * (n - 1) as f32;
            let i = (t.floor() as usize).min(n - 2);
            mix(stops[i], stops[i + 1], t - i as f32)
        }
    }
}

/// Sample a gradient that wraps around (last stop blends back into the first).
pub fn gradient_cyclic(stops: &[Color], t: f32) -> Color {
    match stops.len() {
        0 => Color::TRANSPARENT,
        1 => stops[0],
        n => {
            let t = t.rem_euclid(1.0) * n as f32;
            let i = t.floor() as usize % n;
            mix(stops[i], stops[(i + 1) % n], t.fract())
        }
    }
}

/// Lighten (positive) or darken (negative) in OKLab lightness.
pub fn shade(c: Color, amount: f32) -> Color {
    let mut lab = to_oklab(c);
    lab[0] = (lab[0] + amount).clamp(0.0, 1.0);
    from_oklab(lab)
}

/// Relative luminance (WCAG).
pub fn luminance(c: Color) -> f32 {
    0.2126 * srgb_to_linear(c.r as f32 / 255.0)
        + 0.7152 * srgb_to_linear(c.g as f32 / 255.0)
        + 0.0722 * srgb_to_linear(c.b as f32 / 255.0)
}

/// Black or white, whichever reads best on top of `bg`.
pub fn contrast_text(bg: Color) -> Color {
    if luminance(bg) > 0.4 {
        Color::rgb(10, 10, 12)
    } else {
        Color::WHITE
    }
}

/// A CSS `linear-gradient(...)` through `stops` (OKLab-interpolated, `samples` stops),
/// optionally cyclic so it tiles seamlessly when its background is repeated.
pub fn css_linear_gradient(angle_deg: f32, stops: &[Color], cyclic: bool, samples: usize) -> String {
    let n = samples.max(2);
    let mut s = format!("linear-gradient({angle_deg}deg");
    for i in 0..n {
        let t = i as f32 / (n - 1) as f32;
        let c = if cyclic { gradient_cyclic(stops, t) } else { gradient(stops, t) };
        s.push_str(&format!(", {} {:.1}%", c.to_css(), t * 100.0));
    }
    s.push(')');
    s
}

/// Some ready-made palettes for gradients.
pub mod palettes {
    use super::{hex, Color};
    pub const AURORA: [Color; 3] = [hex("#3A29FF"), hex("#FF94B4"), hex("#FF3232")];
    pub const SUNSET: [Color; 4] = [hex("#FF6B6B"), hex("#FFA94D"), hex("#FFD43B"), hex("#F06595")];
    pub const OCEAN: [Color; 3] = [hex("#0EA5E9"), hex("#6366F1"), hex("#22D3EE")];
    pub const NEON: [Color; 4] = [hex("#40FFAA"), hex("#4079FF"), hex("#FF40D9"), hex("#40FFAA")];
    pub const RAINBOW: [Color; 6] =
        [hex("#FF0080"), hex("#FF8C00"), hex("#FFE600"), hex("#00D26A"), hex("#00B4FF"), hex("#7B2FFF")];
    pub const MONO: [Color; 2] = [hex("#FFFFFF"), hex("#71717A")];
    pub const MAGIC: [Color; 3] = [hex("#9E7AFF"), hex("#FE8BBB"), hex("#FFBD7A")];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parses() {
        assert_eq!(hex("#fff"), Color::WHITE);
        assert_eq!(hex("#000000"), Color::BLACK);
        assert_eq!(hex("ff0000"), Color::rgb(255, 0, 0));
        assert_eq!(hex("#8B5CF6").to_css(), "#8b5cf6");
    }

    #[test]
    fn oklab_roundtrip() {
        for c in [Color::rgb(12, 200, 99), Color::rgb(250, 3, 128), Color::WHITE] {
            let back = from_oklab(to_oklab(c));
            assert!((c.r as i32 - back.r as i32).abs() <= 1);
            assert!((c.g as i32 - back.g as i32).abs() <= 1);
            assert!((c.b as i32 - back.b as i32).abs() <= 1);
        }
    }

    #[test]
    fn hsv_primaries() {
        assert_eq!(hsv(0.0, 1.0, 1.0), Color::rgb(255, 0, 0));
        assert_eq!(hsv(120.0, 1.0, 1.0), Color::rgb(0, 255, 0));
        assert_eq!(hsv(240.0, 1.0, 1.0), Color::rgb(0, 0, 255));
    }
}
