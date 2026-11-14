//! Metallic Paint: a liquid-metal shimmer poured into a shape or logo.

use crate::{background, ghost, pointer, smoothstep, Base, Lut, Rgb};
use egui::epaint::{Mesh, Shape};
use egui::{vec2, Color32, Pos2, Rect, Ui, Vec2};
use glowkit_core::{
    color::hex,
    noise::fbm3,
    paint::{glow, soft_shadow},
};
use std::f32::consts::{PI, TAU};

/// The silhouette [`MetallicPaint`] fills.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum MetalShape {
    /// A four-pointed sparkle (default).
    #[default]
    Sparkle,
    /// A heart.
    Heart,
    /// A five-pointed star with softened points.
    Star,
    /// An app-icon style squircle.
    Squircle,
    /// A circle.
    Circle,
    /// Your own outline in `-1..1` coordinates (y down). It must be star-shaped around the
    /// origin (every point visible from the center), like most logos' outer silhouettes.
    Custom(Vec<Vec2>),
}

impl MetalShape {
    /// Outline points in `-1..1` space, resampled evenly by arc length.
    fn outline(&self) -> Vec<Vec2> {
        resample(&self.raw_outline(), 160)
    }

    fn raw_outline(&self) -> Vec<Vec2> {
        let n = 1440;
        let ring = |r: &dyn Fn(f32) -> f32| -> Vec<Vec2> {
            (0..n)
                .map(|i| {
                    let a = i as f32 / n as f32 * TAU;
                    vec2(a.cos(), a.sin()) * r(a)
                })
                .collect()
        };
        match self {
            MetalShape::Sparkle => ring(&|a| {
                let p = 0.62;
                1.0 / (a.cos().abs().powf(p) + a.sin().abs().powf(p)).powf(1.0 / p)
            }),
            MetalShape::Squircle => ring(&|a| {
                let p = 4.5;
                0.92 / (a.cos().abs().powf(p) + a.sin().abs().powf(p)).powf(1.0 / p)
            }),
            MetalShape::Circle => ring(&|_| 0.92),
            MetalShape::Star => ring(&|a| {
                // five rounded points, first one pointing up
                let k = ((a + PI / 2.0) * 5.0 / TAU).rem_euclid(1.0);
                let d = (k - 0.5).abs() * 2.0; // 1 at a point, 0 between
                0.46 + 0.54 * d.powf(2.2)
            }),
            MetalShape::Heart => (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32 * TAU;
                    let x = 16.0 * t.sin().powi(3);
                    let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
                    vec2(x / 17.0, -(y + 2.5) / 17.0)
                })
                .collect(),
            MetalShape::Custom(pts) => pts.clone(),
        }
    }
}

/// Resample a closed polyline to `n` points evenly spaced along its length (keeps sharp tips
/// from turning into long sliver triangles).
fn resample(pts: &[Vec2], n: usize) -> Vec<Vec2> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let m = pts.len();
    let mut cum = Vec::with_capacity(m + 1);
    let mut acc = 0.0;
    cum.push(0.0);
    for i in 0..m {
        acc += (pts[(i + 1) % m] - pts[i]).length();
        cum.push(acc);
    }
    let total = acc.max(1e-6);
    let mut out = Vec::with_capacity(n);
    let mut j = 0;
    for k in 0..n {
        let s = k as f32 / n as f32 * total;
        while j + 1 < cum.len() - 1 && cum[j + 1] < s {
            j += 1;
        }
        let seg = (cum[j + 1] - cum[j]).max(1e-6);
        let t = ((s - cum[j]) / seg).clamp(0.0, 1.0);
        out.push(pts[j] + (pts[(j + 1) % m] - pts[j]) * t);
    }
    out
}

/// A shape (sparkle, heart, star, squircle, circle or your own outline) filled with flowing
/// liquid metal: chrome bands that ripple and fold, a bevelled rim and a highlight that slides
/// across as the pointer moves.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::{MetallicPaint, MetalShape};
/// ui.add(MetallicPaint::new().shape(MetalShape::Heart));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct MetallicPaint {
    base: Base,
    shape: MetalShape,
    tint: Option<Color32>,
    scale: f32,
    speed: f32,
    pattern: f32,
}

impl Default for MetallicPaint {
    fn default() -> Self {
        Self::new()
    }
}

impl MetallicPaint {
    /// A chrome sparkle.
    pub fn new() -> Self {
        Self { base: Base::default(), shape: MetalShape::Sparkle, tint: None, scale: 0.8, speed: 1.0, pattern: 1.0 }
    }
    /// The silhouette to fill (default sparkle).
    pub fn shape(mut self, shape: MetalShape) -> Self {
        self.shape = shape;
        self
    }
    /// Tint the metal (default: neutral chrome).
    pub fn tint(mut self, c: Color32) -> Self {
        self.tint = Some(c);
        self
    }
    /// Size of the shape relative to the rect's shorter side (default 0.8).
    pub fn scale(mut self, k: f32) -> Self {
        self.scale = k.clamp(0.1, 1.0);
        self
    }
    /// Flow speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Density of the chrome bands (default 1).
    pub fn pattern(mut self, k: f32) -> Self {
        self.pattern = k.clamp(0.2, 4.0);
        self
    }

    /// Paint the metal shape centered in `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_metallic_paint") else { return };
        let dark = f.dark();
        let time = f.time * self.speed;
        let outline = self.shape.outline();
        if outline.len() < 3 {
            return;
        }
        let r = rect.width().min(rect.height()) * 0.5 * self.scale;
        let center = rect.center();
        // light direction follows the pointer
        let target = pointer(ui, rect).unwrap_or_else(|| ghost(rect, f.time * 0.6));
        let aim = (target - center) / (rect.size() * 0.5);
        let light_dir = glowkit_core::anim::spring2(ui, f.id.with("light"), aim, 30.0, 10.0);
        // backdrop: a soft shadow / glow under the shape
        let p = &f.painter;
        let size = Rect::from_center_size(center, Vec2::splat(r * 1.6));
        if dark {
            let tint = self.tint.unwrap_or(f.theme.accent);
            glow(p, center, r * 1.5, glowkit_core::color::with_alpha(tint, 0.12));
        } else {
            soft_shadow(p, size.translate(vec2(0.0, r * 0.12)), r * 0.6, r * 0.25, Vec2::ZERO, Color32::from_black_alpha(30));
        }
        let stops = [
            hex("#2A2D34"),
            hex("#8C929C"),
            hex("#F4F6FA"),
            hex("#A7ADB8"),
            hex("#4A4F58"),
            hex("#D9DDE4"),
            hex("#FFFFFF"),
            hex("#6B717C"),
        ];
        let lut = Lut::new(&stops, true);
        let tint = self.tint.map(Rgb::of);
        let pattern = self.pattern;
        let shade = |q: Vec2, k: f32, normal: Vec2| -> Color32 {
            // q: shape space (-1..1), k: 0 at center .. 1 at rim
            let warp = fbm3(q.x * 1.3 * pattern, q.y * 1.3 * pattern, time * 0.18, 3);
            let warp2 = fbm3(q.x * 2.6 * pattern + 4.0, q.y * 2.6 * pattern, time * 0.25, 2);
            let v = (q.x * 0.35 + q.y * 0.8) * pattern + warp * 1.4 + warp2 * 0.35 - time * 0.12 + light_dir.x * 0.15;
            let mut c = lut.sample(v);
            // bevel: rim lit from the light side, shadowed opposite
            let bevel = smoothstep(0.78, 1.0, k);
            let lit = normal.dot(vec2(-0.5 - light_dir.x * 0.6, -0.8 - light_dir.y * 0.6).normalized());
            c = if lit > 0.0 { c.lerp(Rgb(1.0, 1.0, 1.0), bevel * lit * 0.8) } else { c.scale(1.0 + bevel * lit * 0.65) };
            // a broad sheen sweeping across
            let sweep = ((q.x - q.y) * 0.7 - (time * 0.35).rem_euclid(4.0) + 2.0).abs();
            c = c.lerp(Rgb(1.0, 1.0, 1.0), (1.0 - sweep / 0.35).max(0.0) * 0.35);
            if let Some(tc) = tint {
                c = Rgb(c.0 * (0.35 + 0.75 * tc.0), c.1 * (0.35 + 0.75 * tc.1), c.2 * (0.35 + 0.75 * tc.2));
            }
            c.to_color()
        };
        // mesh: concentric rings from the center out to the outline, plus a feathered edge
        let n = outline.len();
        let rings = 14;
        let feather = 1.2;
        let mut mesh = Mesh::default();
        let to_screen = |q: Vec2| -> Pos2 { center + q * r };
        mesh.colored_vertex(to_screen(Vec2::ZERO), shade(Vec2::ZERO, 0.0, Vec2::ZERO));
        // normals of the outline (pointing out)
        let normals: Vec<Vec2> = (0..n)
            .map(|i| {
                let a = outline[(i + n - 1) % n];
                let b = outline[(i + 1) % n];
                let d = b - a;
                let nn = vec2(d.y, -d.x).normalized();
                if nn.dot(outline[i]) < 0.0 {
                    -nn
                } else {
                    nn
                }
            })
            .collect();
        for ring in 1..=rings {
            let k = ring as f32 / rings as f32;
            // denser near the rim, where the bevel lives
            let k = 1.0 - (1.0 - k).powf(1.6);
            for i in 0..n {
                let q = outline[i] * k;
                mesh.colored_vertex(to_screen(q), shade(q, k, normals[i] * k));
            }
        }
        // feather ring
        for i in 0..n {
            let q = outline[i];
            let pos = to_screen(q) + (normals[i] + q.normalized()).normalized() * feather;
            mesh.colored_vertex(pos, Color32::TRANSPARENT);
        }
        let n32 = n as u32;
        for i in 0..n32 {
            mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n32);
        }
        for ring in 0..rings as u32 {
            let a0 = 1 + ring * n32;
            let b0 = a0 + n32;
            for i in 0..n32 {
                let j = (i + 1) % n32;
                mesh.add_triangle(a0 + i, b0 + i, b0 + j);
                mesh.add_triangle(a0 + i, b0 + j, a0 + j);
            }
        }
        p.add(Shape::mesh(mesh));
        // fine dark edge so the silhouette reads on light backgrounds
        let edge: Vec<Pos2> = outline.iter().map(|q| to_screen(*q)).collect();
        let ec = if dark { Color32::from_white_alpha(40) } else { Color32::from_black_alpha(50) };
        p.add(Shape::closed_line(edge, egui::Stroke::new(1.0, ec)));
    }
}

background!(MetallicPaint);
