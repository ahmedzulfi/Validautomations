//! Laser Flow: a vertical laser beam striking a surface, wrapped in drifting volumetric fog.

use crate::{background, light, pointer, smoothstep, soft_ribbon, Base, Rgb};
use egui::{pos2, vec2, Color32, Pos2, Rect, Ui};
use glowkit_core::{
    noise::fbm3,
    paint::{glow, glow_ellipse, shader_grid},
};

/// A laser beam falling from the top edge and splashing across a surface, with luminous fog
/// curling around it. The beam flickers, the fog flows outward and upward, and the impact point
/// leans gently toward the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::LaserFlow;
/// ui.add(LaserFlow::new().color(glowkit_core::hex("#FF79C6")));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct LaserFlow {
    base: Base,
    color: Option<Color32>,
    impact: f32,
    fog: f32,
    speed: f32,
    width: f32,
    follow: bool,
}

impl Default for LaserFlow {
    fn default() -> Self {
        Self::new()
    }
}

impl LaserFlow {
    /// A laser in the theme accent color.
    pub fn new() -> Self {
        Self { base: Base::default(), color: None, impact: 0.66, fog: 1.0, speed: 1.0, width: 1.0, follow: true }
    }
    /// Beam color (default: the theme accent).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Height of the impact surface as a fraction of the rect (default 0.66).
    pub fn impact(mut self, fraction: f32) -> Self {
        self.impact = fraction.clamp(0.2, 0.95);
        self
    }
    /// Fog density, 0 = clear air (default 1).
    pub fn fog(mut self, k: f32) -> Self {
        self.fog = k.max(0.0);
        self
    }
    /// Fog flow speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Beam thickness multiplier (default 1).
    pub fn width(mut self, k: f32) -> Self {
        self.width = k.clamp(0.2, 4.0);
        self
    }
    /// Let the beam lean toward the pointer (default true).
    pub fn follow_pointer(mut self, on: bool) -> Self {
        self.follow = on;
        self
    }

    /// Paint the laser into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_laser_flow") else { return };
        let t = &f.theme;
        let dark = f.dark();
        let color = self.color.unwrap_or(t.accent);
        let time = f.time * self.speed;
        let target_x = match (self.follow, pointer(ui, rect)) {
            (true, Some(p)) => (p.x - rect.center().x) * 0.25,
            _ => rect.width() * 0.04 * (f.time * 0.4).sin(),
        };
        let lean = glowkit_core::anim::follow(ui, f.id.with("lean"), target_x, 3.0);
        let hit = pos2(rect.center().x + lean, rect.min.y + rect.height() * self.impact);
        let top = pos2(rect.center().x + lean * 0.3, rect.min.y - 4.0);
        let flick = 0.88 + 0.12 * ((time * 31.0).sin() * (time * 17.3).cos()).abs();
        // volumetric fog: fbm noise flowing outward and upward, lit by the beam and the splash
        let col = Rgb::of(color);
        let white = Rgb(1.0, 1.0, 1.0);
        let bg = Rgb::of(f.bg);
        let (w, h) = (rect.width(), rect.height());
        let fog_k = self.fog;
        let beam_w = self.width;
        shader_grid(&f.painter, rect, 72, 44, |uv| {
            let p = pos2(rect.min.x + uv.x * w, rect.min.y + uv.y * h);
            let dx = p.x - (top.x + (hit.x - top.x) * ((p.y - top.y) / (hit.y - top.y)).clamp(0.0, 1.0));
            let dy = p.y - hit.y;
            // beam halo above the surface, splash halo spreading along it
            let above = smoothstep(8.0, -30.0, dy);
            let halo = (-(dx.abs()) / (26.0 * beam_w + uv.y * 70.0)).exp() * above;
            let splash = (-(dx.abs()) / (w * 0.22)).exp() * (-(dy.abs()) / (h * 0.09)).exp();
            let floor = (-(dx.abs()) / (w * 0.35)).exp() * smoothstep(0.0, h * 0.25, dy) * (-(dy) / (h * 0.25)).exp();
            let side = if dx < 0.0 { -1.0 } else { 1.0 };
            let n = fbm3(uv.x * 4.0 - side * time * 0.18, uv.y * 3.0 + time * 0.25, time * 0.1, 3) * 0.5 + 0.5;
            let wisps = smoothstep(0.35, 0.85, n);
            let lit = (halo * 0.55 + splash * 0.9 + floor * 0.35) * (0.35 + 0.9 * wisps) * fog_k;
            let lit = lit.min(1.4);
            if dark {
                bg.add(col.scale(lit * 0.9)).lerp(white, (lit - 0.9).max(0.0) * 0.4).to_color()
            } else {
                bg.lerp(col, (lit * 0.6).min(0.85)).to_color()
            }
        });
        let p = &f.painter;
        // the beam: wide soft glow, colored body and a white-hot core
        let beam: Vec<Pos2> = (0..=24).map(|i| top.lerp(hit, i as f32 / 24.0)).collect();
        let along = |u: f32| smoothstep(0.0, 0.25, u) * 0.6 + 0.4;
        soft_ribbon(p, &beam, 6.0 * beam_w, 26.0 * beam_w, |u| light(color, 0.22 * along(u) * flick, dark));
        soft_ribbon(p, &beam, 2.5 * beam_w, 6.0 * beam_w, |u| light(color, 0.75 * along(u) * flick, dark));
        let core = if dark { Color32::WHITE } else { glowkit_core::color::shade(color, -0.05) };
        soft_ribbon(p, &beam, 1.2 * beam_w, 1.5, |u| light(core, 0.95 * along(u), dark));
        // impact: a flat flare along the surface plus a hot spot
        let flare_w = rect.width() * 0.48;
        let line: Vec<Pos2> =
            (0..=40).map(|i| pos2(hit.x - flare_w + flare_w * 2.0 * i as f32 / 40.0, hit.y)).collect();
        soft_ribbon(p, &line, 1.6, 7.0, |u| {
            let k = (1.0 - (u - 0.5).abs() * 2.0).powf(2.2);
            light(if dark { glowkit_core::mix(color, Color32::WHITE, 0.5) } else { color }, k * flick, dark)
        });
        glow_ellipse(p, hit, vec2(flare_w * 0.6, 26.0), light(color, 0.45 * flick, dark));
        glow(p, hit, 60.0, light(color, 0.6 * flick, dark));
        glow(p, hit, 16.0, light(core, 0.9, dark));
    }
}

background!(LaserFlow);
