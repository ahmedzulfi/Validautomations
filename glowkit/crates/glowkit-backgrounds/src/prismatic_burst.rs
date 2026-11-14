//! Prismatic Burst: rainbow light rays exploding from a point.

use crate::{background, h01, light, pointer, Base, Lut};
use egui::epaint::{Mesh, Shape};
use egui::{vec2, Color32, Pos2, Rect, Ui};
use glowkit_core::{color::palettes, paint::glow};
use std::f32::consts::TAU;

/// Rays of prismatic light bursting outward from the center: dozens of soft wedges in shifting
/// rainbow hues that rotate, flicker and breathe, with shock rings rippling out now and then.
/// The burst's origin drifts toward the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::PrismaticBurst;
/// ui.add(PrismaticBurst::new().rays(48).speed(0.8));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct PrismaticBurst {
    base: Base,
    colors: Vec<Color32>,
    rays: usize,
    speed: f32,
    intensity: f32,
    rings: bool,
    follow: bool,
}

impl Default for PrismaticBurst {
    fn default() -> Self {
        Self::new()
    }
}

impl PrismaticBurst {
    /// A full-spectrum burst.
    pub fn new() -> Self {
        Self {
            base: Base::default(),
            colors: Vec::new(),
            rays: 44,
            speed: 1.0,
            intensity: 1.0,
            rings: true,
            follow: true,
        }
    }
    /// Ray colors, cycled around the burst (default: a rainbow).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    /// Number of rays (default 44).
    pub fn rays(mut self, n: usize) -> Self {
        self.rays = n.clamp(4, 160);
        self
    }
    /// Animation speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Brightness multiplier (default 1).
    pub fn intensity(mut self, k: f32) -> Self {
        self.intensity = k.max(0.0);
        self
    }
    /// Expanding shock rings (default true).
    pub fn rings(mut self, on: bool) -> Self {
        self.rings = on;
        self
    }
    /// Let the burst's origin lean toward the pointer (default true).
    pub fn follow_pointer(mut self, on: bool) -> Self {
        self.follow = on;
        self
    }

    /// Paint the burst into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_prismatic_burst") else { return };
        let dark = f.dark();
        let stops: Vec<Color32> = if self.colors.len() >= 2 { self.colors.clone() } else { palettes::RAINBOW.to_vec() };
        let lut = Lut::new(&stops, true);
        let time = f.time * self.speed;
        let target = if self.follow { pointer(ui, rect) } else { None };
        let aim = target.map(|p| (p - rect.center()) * 0.25).unwrap_or_default();
        let off = glowkit_core::anim::spring2(ui, f.id.with("aim"), aim, 30.0, 11.0);
        let c = rect.center() + off;
        let reach = rect.size().length() * 0.62;
        let k = self.intensity * if dark { 1.0 } else { 0.85 };
        let p = &f.painter;
        let mut mesh = Mesh::default();
        let breathe = 0.85 + 0.15 * (time * 1.3).sin();
        // two layers: broad soft rays and thin sharp ones
        for layer in 0..2u32 {
            let n = if layer == 0 { (self.rays / 3).max(3) } else { self.rays };
            let spin = if layer == 0 { time * 0.05 } else { -time * 0.08 };
            for i in 0..n {
                let seed = i as u32 * 7 + layer * 1013;
                let r1 = h01(seed, 1);
                let r2 = h01(seed, 2);
                let r3 = h01(seed, 3);
                let base_a = (i as f32 + r1 * 0.6) / n as f32 * TAU + spin;
                let wobble = 0.03 * (time * (0.6 + r2) + r3 * TAU).sin();
                let a = base_a + wobble;
                let width =
                    if layer == 0 { (0.10 + r2 * 0.12) * (24.0 / n as f32).min(1.4) } else { 0.012 + r3 * 0.03 };
                let len = reach * (0.55 + 0.45 * r3) * (0.85 + 0.15 * (time * (1.0 + r1) + r2 * 9.0).sin()) * breathe;
                let flick = 0.6 + 0.4 * (time * (2.0 + r2 * 3.0) + r1 * 20.0).sin().abs();
                let hue = a / TAU + time * 0.04;
                let col = lut.sample(hue).to_color();
                let amt = k * flick * if layer == 0 { 0.32 } else { 0.55 };
                let center_c = light(col, amt, dark);
                let tip_c = light(col, amt * 0.12, dark);
                let clear = Color32::TRANSPARENT;
                let start = 6.0;
                let dir = |ang: f32| vec2(ang.cos(), ang.sin());
                let i0 = mesh.vertices.len() as u32;
                mesh.colored_vertex(c + dir(a) * start, center_c);
                mesh.colored_vertex(c + dir(a - width) * len, clear);
                mesh.colored_vertex(c + dir(a) * len, tip_c);
                mesh.colored_vertex(c + dir(a + width) * len, clear);
                mesh.add_triangle(i0, i0 + 1, i0 + 2);
                mesh.add_triangle(i0, i0 + 2, i0 + 3);
            }
        }
        p.add(Shape::mesh(mesh));
        // shock rings: thin rainbow circles expanding and fading
        if self.rings {
            let period = 2.6;
            for j in 0..3 {
                let ph = ((time + j as f32 * period / 3.0) / period).rem_euclid(1.0);
                let r = 20.0 + ph * reach * 0.8;
                let a = (1.0 - ph).powf(1.6) * 0.5 * k;
                let segs = 96;
                let pts: Vec<Pos2> = (0..=segs)
                    .map(|s| c + vec2((s as f32 / segs as f32 * TAU).cos(), (s as f32 / segs as f32 * TAU).sin()) * r)
                    .collect();
                glowkit_core::paint::gradient_polyline(p, &pts, true, 1.5 + 4.0 * (1.0 - ph), |u| {
                    light(lut.sample(u + time * 0.1).to_color(), a, dark)
                });
            }
        }
        // hot core
        let core = if dark { Color32::WHITE } else { lut.sample(time * 0.04).to_color() };
        glow(p, c, 140.0 * breathe, light(lut.sample(time * 0.05).to_color(), 0.45 * k, dark));
        glow(p, c, 42.0, light(core, 0.9 * k, dark));
    }
}

background!(PrismaticBurst);
