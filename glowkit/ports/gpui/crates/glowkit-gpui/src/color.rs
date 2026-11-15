#![allow(clippy::excessive_precision)] // OKLab matrices are published reference constants
//! Color helpers: hex parsing, OKLab mixing, gradients. Ported from `glowkit_core::color`, but
//! working on GPUI's [`Hsla`].

use gpui::{Hsla, Rgba};

/// Parse `#rgb`, `#rrggbb` or `#rrggbbaa`. Panics on malformed input (these are literals).
pub fn hex(s: &str) -> Hsla {
    let s = s.trim_start_matches('#');
    let h = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).expect("invalid hex color");
    let (r, g, b, a) = match s.len() {
        3 => {
            let d = |i: usize| u8::from_str_radix(&s[i..i + 1], 16).expect("invalid hex color") * 17;
            (d(0), d(1), d(2), 255)
        }
        6 => (h(0), h(2), h(4), 255),
        8 => (h(0), h(2), h(4), h(6)),
        _ => panic!("hex color must be #rgb, #rrggbb or #rrggbbaa"),
    };
    Rgba { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a: a as f32 / 255.0 }.into()
}

/// Set the alpha of a color, `a` in `0..=1`.
pub fn with_alpha(c: Hsla, a: f32) -> Hsla {
    Hsla { a: a.clamp(0.0, 1.0), ..c }
}

/// Multiply the existing alpha by `a`.
pub fn fade(c: Hsla, a: f32) -> Hsla {
    Hsla { a: c.a * a.clamp(0.0, 1.0), ..c }
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
pub fn to_oklab(c: Hsla) -> [f32; 4] {
    let Rgba { r, g, b, a } = c.to_rgb();
    let (r, g, b) = (srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b));
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
        a,
    ]
}

/// Convert from OKLab `[L, a, b, alpha]`.
pub fn from_oklab(lab: [f32; 4]) -> Hsla {
    let [ll, aa, bb, alpha] = lab;
    let l = (ll + 0.3963377774 * aa + 0.2158037573 * bb).powi(3);
    let m = (ll - 0.1055613458 * aa - 0.0638541728 * bb).powi(3);
    let s = (ll - 0.0894841775 * aa - 1.2914855480 * bb).powi(3);
    let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let b = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;
    Rgba { r: linear_to_srgb(r), g: linear_to_srgb(g), b: linear_to_srgb(b), a: alpha.clamp(0.0, 1.0) }.into()
}

/// Perceptual mix of two colors (OKLab), `t` in `0..=1`.
pub fn mix(a: Hsla, b: Hsla, t: f32) -> Hsla {
    let t = t.clamp(0.0, 1.0);
    let (la, lb) = (to_oklab(a), to_oklab(b));
    from_oklab([
        la[0] + (lb[0] - la[0]) * t,
        la[1] + (lb[1] - la[1]) * t,
        la[2] + (lb[2] - la[2]) * t,
        la[3] + (lb[3] - la[3]) * t,
    ])
}

/// Cheap sRGB-space lerp (egui's `lerp_to_gamma`), for per-glyph effects evaluated every frame.
pub fn lerp_srgb(a: Hsla, b: Hsla, t: f32) -> Hsla {
    let t = t.clamp(0.0, 1.0);
    let (x, y) = (a.to_rgb(), b.to_rgb());
    Rgba {
        r: x.r + (y.r - x.r) * t,
        g: x.g + (y.g - x.g) * t,
        b: x.b + (y.b - x.b) * t,
        a: x.a + (y.a - x.a) * t,
    }
    .into()
}

/// Sample a multi-stop gradient (evenly spaced stops) at `t` in `0..=1`.
pub fn gradient(stops: &[Hsla], t: f32) -> Hsla {
    match stops.len() {
        0 => gpui::transparent_black(),
        1 => stops[0],
        n => {
            let t = t.clamp(0.0, 1.0) * (n - 1) as f32;
            let i = (t.floor() as usize).min(n - 2);
            mix(stops[i], stops[i + 1], t - i as f32)
        }
    }
}

/// Sample a gradient that wraps around (last stop blends back into the first).
pub fn gradient_cyclic(stops: &[Hsla], t: f32) -> Hsla {
    match stops.len() {
        0 => gpui::transparent_black(),
        1 => stops[0],
        n => {
            let t = t.rem_euclid(1.0) * n as f32;
            let i = t.floor() as usize % n;
            mix(stops[i], stops[(i + 1) % n], t.fract())
        }
    }
}

/// Lighten (positive) or darken (negative) in OKLab lightness.
pub fn shade(c: Hsla, amount: f32) -> Hsla {
    let mut lab = to_oklab(c);
    lab[0] = (lab[0] + amount).clamp(0.0, 1.0);
    from_oklab(lab)
}

/// Relative luminance (WCAG).
pub fn luminance(c: Hsla) -> f32 {
    let Rgba { r, g, b, .. } = c.to_rgb();
    0.2126 * srgb_to_linear(r) + 0.7152 * srgb_to_linear(g) + 0.0722 * srgb_to_linear(b)
}

/// Black or white, whichever reads best on top of `bg`.
pub fn contrast_text(bg: Hsla) -> Hsla {
    if luminance(bg) > 0.4 {
        hex("#0A0A0C")
    } else {
        gpui::white()
    }
}

/// Ready-made gradient palettes (same stops as `glowkit_core::color::palettes`).
pub mod palettes {
    use super::hex;
    use gpui::Hsla;
    pub fn aurora() -> Vec<Hsla> {
        ["#3A29FF", "#FF94B4", "#FF3232"].map(hex).to_vec()
    }
    pub fn sunset() -> Vec<Hsla> {
        ["#FF6B6B", "#FFA94D", "#FFD43B", "#F06595"].map(hex).to_vec()
    }
    pub fn ocean() -> Vec<Hsla> {
        ["#0EA5E9", "#6366F1", "#22D3EE"].map(hex).to_vec()
    }
    pub fn neon() -> Vec<Hsla> {
        ["#40FFAA", "#4079FF", "#FF40D9", "#40FFAA"].map(hex).to_vec()
    }
    pub fn rainbow() -> Vec<Hsla> {
        ["#FF0080", "#FF8C00", "#FFE600", "#00D26A", "#00B4FF", "#7B2FFF"].map(hex).to_vec()
    }
    pub fn magic() -> Vec<Hsla> {
        ["#9E7AFF", "#FE8BBB", "#FFBD7A"].map(hex).to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parses() {
        let w = hex("#fff").to_rgb();
        assert!((w.r - 1.0).abs() < 1e-4 && (w.g - 1.0).abs() < 1e-4 && (w.b - 1.0).abs() < 1e-4);
        let r = hex("ff0000").to_rgb();
        assert!((r.r - 1.0).abs() < 1e-4 && r.g.abs() < 1e-4);
    }

    #[test]
    fn oklab_roundtrip() {
        for s in ["#0CC863", "#FA0380", "#FFFFFF", "#8B5CF6"] {
            let c = hex(s);
            let back = from_oklab(to_oklab(c)).to_rgb();
            let c = c.to_rgb();
            assert!((c.r - back.r).abs() < 0.01 && (c.g - back.g).abs() < 0.01 && (c.b - back.b).abs() < 0.01);
        }
    }
}
