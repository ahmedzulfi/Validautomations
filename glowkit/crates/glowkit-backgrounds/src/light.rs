//! Light effects: lightning, glowing orb, ripples, spotlight rays and the lamp.

use crate::{background, h01, light, pointer, smoothstep, Base, Lut, Rgb};
use egui::epaint::{Mesh, Shape};
use egui::{pos2, vec2, Color32, Pos2, Rect, Ui, Vec2};
use glowkit_core::noise::{fbm3, noise3};
use glowkit_core::paint::{circle_points, fill_convex_fn, glow, glow_ellipse, gradient_polyline, shader_grid};
use std::f32::consts::{PI, TAU};

// ---------------------------------------------------------------------------------------------
// Lightning
// ---------------------------------------------------------------------------------------------

/// A crackling bolt of lightning from top to bottom: a jagged, constantly shifting channel with
/// a white-hot core, a wide glow and branches that flicker in and out.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::Lightning;
/// ui.add(Lightning::new().color(glowkit_core::hex("#60A5FA")).intensity(1.2));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct Lightning {
    base: Base,
    color: Option<Color32>,
    x: f32,
    speed: f32,
    intensity: f32,
    branches: bool,
}

impl Default for Lightning {
    fn default() -> Self {
        Self::new()
    }
}

impl Lightning {
    /// A bolt down the middle.
    pub fn new() -> Self {
        Self { base: Base::default(), color: None, x: 0.5, speed: 1.0, intensity: 1.0, branches: true }
    }
    /// Bolt color (default: a blue from the theme).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Horizontal position as a fraction of the width (default 0.5).
    pub fn x(mut self, fraction: f32) -> Self {
        self.x = fraction;
        self
    }
    /// Animation speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Brightness multiplier (default 1).
    pub fn intensity(mut self, i: f32) -> Self {
        self.intensity = i.clamp(0.0, 2.0);
        self
    }
    /// Draw flickering side branches (default true).
    pub fn branches(mut self, on: bool) -> Self {
        self.branches = on;
        self
    }

    fn stroke(p: &egui::Painter, pts: &[Pos2], color: Color32, a: f32, dark: bool, scale: f32) {
        let fade = |u: f32| smoothstep(0.0, 0.08, u) * (1.0 - smoothstep(0.85, 1.0, u) * 0.6);
        gradient_polyline(p, pts, false, 22.0 * scale, |u| light(color, a * 0.07 * fade(u), dark));
        gradient_polyline(p, pts, false, 9.0 * scale, |u| light(color, a * 0.2 * fade(u), dark));
        gradient_polyline(p, pts, false, 3.6 * scale, |u| light(color, a * 0.6 * fade(u), dark));
        let core = if dark { glowkit_core::mix(color, Color32::WHITE, 0.85) } else { glowkit_core::color::shade(color, -0.1) };
        gradient_polyline(p, pts, false, 1.5 * scale, |u| core.gamma_multiply((a * fade(u)).min(1.0)));
    }

    /// Paint the lightning into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_lightning") else { return };
        let t = &f.theme;
        let dark = f.dark();
        let color = self.color.unwrap_or(t.chart[5]);
        let time = f.time * self.speed;
        let flicker = (0.72 + 0.28 * noise3(time * 9.0, 0.5, 2.0).abs() * 2.0).min(1.0) * self.intensity;
        let cx = rect.min.x + rect.width() * self.x;
        let h = rect.height();
        let n = (h / 5.0) as usize + 2;
        let main: Vec<Pos2> = (0..=n)
            .map(|k| {
                let v = k as f32 / n as f32;
                let y = rect.min.y - 4.0 + v * (h + 8.0);
                let slow = fbm3(v * 2.2, time * 0.6, 1.0, 3) * rect.width() * 0.09;
                let jag = noise3(v * 14.0, time * 5.0, 3.0) * 16.0 + noise3(v * 40.0, time * 9.0, 7.0) * 5.0;
                pos2(cx + slow + jag * (0.4 + v), y)
            })
            .collect();
        let p = &f.painter;
        glow_ellipse(p, pos2(cx, rect.center().y), vec2(rect.width() * 0.22, h * 0.75), light(color, 0.28 * flicker, dark));
        glow_ellipse(p, pos2(cx, rect.max.y), vec2(rect.width() * 0.25, 60.0), light(color, 0.35 * flicker, dark));
        if self.branches {
            let epoch = (time * 3.0).floor() as u32;
            let life = (time * 3.0).fract();
            for b in 0..4u32 {
                let seed = epoch.wrapping_mul(13).wrapping_add(b * 7);
                if h01(seed, 1) < 0.35 {
                    continue;
                }
                let start = 0.15 + 0.6 * h01(seed, 2);
                let si = ((start * n as f32) as usize).min(n);
                let side = if h01(seed, 3) < 0.5 { -1.0 } else { 1.0 };
                let len = 50.0 + 110.0 * h01(seed, 4);
                let m = 18;
                let o = main[si];
                let pts: Vec<Pos2> = (0..=m)
                    .map(|k| {
                        let u = k as f32 / m as f32;
                        let jag = noise3(u * 8.0, seed as f32 * 0.37, time * 6.0) * 10.0 * u;
                        o + vec2(side * u * len * 0.75 + jag, u * len * 0.85 + jag * 0.5)
                    })
                    .collect();
                let a = flicker * 0.75 * (1.0 - life).powf(0.5);
                Self::stroke(p, &pts, color, a, dark, 0.5);
            }
        }
        Self::stroke(p, &main, color, flicker, dark, 1.0);
    }
}

background!(Lightning);

// ---------------------------------------------------------------------------------------------
// Orb
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct OrbState {
    hover: f32,
    spin: f32,
}

/// A glowing ring of colored light with a soft, breathing edge. Hover it and the edge ripples
/// and the colors spin faster.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::Orb;
/// ui.add(Orb::new().hue_shift(40.0));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct Orb {
    base: Base,
    colors: Option<Vec<Color32>>,
    radius: Option<f32>,
    hue_shift: f32,
    speed: f32,
    hover: Option<bool>,
}

impl Default for Orb {
    fn default() -> Self {
        Self::new()
    }
}

impl Orb {
    /// An orb in the theme's colors, centered in the rect.
    pub fn new() -> Self {
        Self { base: Base::default(), colors: None, radius: None, hue_shift: 0.0, speed: 1.0, hover: None }
    }
    /// Colors around the ring (default: accent, cyan, pink from the theme).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }
    /// Orb radius in points (default: 30% of the smaller side).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = Some(r.max(4.0));
        self
    }
    /// Rotate the colors around the ring, in degrees.
    pub fn hue_shift(mut self, degrees: f32) -> Self {
        self.hue_shift = degrees;
        self
    }
    /// Animation speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Force the hovered look on or off instead of following the pointer.
    pub fn force_hover(mut self, on: bool) -> Self {
        self.hover = Some(on);
        self
    }

    /// Paint the orb into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_orb") else { return };
        let t = &f.theme;
        let dark = f.dark();
        let colors = self.colors.clone().unwrap_or_else(|| vec![t.accent, t.chart[1], t.gradient[1]]);
        let lut = Lut::new(&colors, true);
        let c = rect.center();
        let radius = self.radius.unwrap_or(rect.width().min(rect.height()) * 0.3);
        let hovered = self.hover.unwrap_or_else(|| pointer(ui, rect).is_some_and(|p| p.distance(c) < radius * 1.2));
        let st = glowkit_core::with_state(ui, f.id, OrbState::default, |s| {
            let target = if hovered { 1.0 } else { 0.0 };
            s.hover += (target - s.hover) * (1.0 - (-f.dt * 4.0).exp());
            s.spin += f.dt * self.speed * (0.08 + 0.25 * s.hover);
            *s
        });
        let time = f.time * self.speed;
        let p = &f.painter;
        glow(p, c, radius * 1.9, light(lut.sample(st.spin + 0.3).to_color(), 0.2, dark));
        // ring mesh: (radius fraction, alpha)
        let rings: [(f32, f32); 7] = [(0.35, 0.0), (0.7, 0.18), (0.88, 0.55), (0.98, 1.0), (1.02, 1.0), (1.1, 0.35), (1.45, 0.0)];
        let seg = 160;
        let mut m = Mesh::default();
        let shift = self.hue_shift / 360.0;
        for (fr, al) in rings {
            for k in 0..seg {
                let a = k as f32 / seg as f32 * TAU;
                let dir = Vec2::angled(a);
                let wob = noise3(dir.x * 1.3 + 3.0, dir.y * 1.3, time * 0.5) * (0.05 + 0.12 * st.hover)
                    + noise3(dir.x * 4.0, dir.y * 4.0, time * 1.4) * 0.05 * st.hover;
                let r = radius * (fr + wob * fr.min(1.0));
                let col = lut.sample(k as f32 / seg as f32 + st.spin + shift + (1.0 - fr) * 0.12);
                let mut col = col.to_color();
                if dark && (fr - 1.0).abs() < 0.05 {
                    col = glowkit_core::mix(col, Color32::WHITE, 0.25);
                }
                m.colored_vertex(c + dir * r, light(col, al, dark));
            }
        }
        for ri in 0..rings.len() - 1 {
            let (a0, b0) = ((ri * seg) as u32, ((ri + 1) * seg) as u32);
            for k in 0..seg as u32 {
                let k1 = (k + 1) % seg as u32;
                m.add_triangle(a0 + k, b0 + k, b0 + k1);
                m.add_triangle(a0 + k, b0 + k1, a0 + k1);
            }
        }
        p.add(Shape::mesh(m));
        let _ = PI;
    }
}

background!(Orb);

// ---------------------------------------------------------------------------------------------
// Ripple
// ---------------------------------------------------------------------------------------------

/// Concentric circles radiating from the center, breathing in a staggered wave and fading
/// toward the bottom. Switch to `expanding` for rings that grow outward and dissolve.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::Ripple;
/// ui.add(Ripple::new().circles(8));
/// ui.add(Ripple::new().expanding(true));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct Ripple {
    base: Base,
    circles: usize,
    first: f32,
    step: f32,
    opacity: f32,
    color: Option<Color32>,
    expanding: bool,
    center: Option<Vec2>,
}

impl Default for Ripple {
    fn default() -> Self {
        Self::new()
    }
}

impl Ripple {
    /// Eight circles, the first 105 points in radius.
    pub fn new() -> Self {
        Self { base: Base::default(), circles: 8, first: 105.0, step: 35.0, opacity: 0.24, color: None, expanding: false, center: None }
    }
    /// Number of circles (default 8).
    pub fn circles(mut self, n: usize) -> Self {
        self.circles = n.clamp(1, 40);
        self
    }
    /// Radius of the innermost circle in points (default 105).
    pub fn first_radius(mut self, r: f32) -> Self {
        self.first = r.max(1.0);
        self
    }
    /// Radius added per circle in points (default 35).
    pub fn step(mut self, s: f32) -> Self {
        self.step = s.max(1.0);
        self
    }
    /// Opacity of the innermost circle (default 0.24).
    pub fn opacity(mut self, a: f32) -> Self {
        self.opacity = a.clamp(0.0, 1.0);
        self
    }
    /// Circle color (default: theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Rings continuously expand outward and fade instead of breathing (default false).
    pub fn expanding(mut self, on: bool) -> Self {
        self.expanding = on;
        self
    }
    /// Center as a fraction of the rect (default: the middle).
    pub fn center(mut self, fraction: Vec2) -> Self {
        self.center = Some(fraction);
        self
    }

    /// Paint the ripples into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_ripple") else { return };
        let t = &f.theme;
        let color = self.color.unwrap_or(t.foreground);
        let cf = self.center.unwrap_or(vec2(0.5, 0.5));
        let c = rect.min + rect.size() * cf;
        let (top, h) = (rect.min.y, rect.height());
        let mask = move |y: f32| 1.0 - smoothstep(0.0, 1.0, (y - top) / h) * 0.95;
        let p = &f.painter;
        let n = self.circles;
        for i in (0..n).rev() {
            let fi = i as f32;
            let (r, a) = if self.expanding {
                let period = 4.0;
                let k = ((f.time / period) + fi / n as f32).fract();
                let max_r = self.first + self.step * n as f32;
                (k * max_r, self.opacity * 1.6 * (1.0 - k).powf(1.5) * smoothstep(0.0, 0.08, k))
            } else {
                // each circle breathes (scale 1 -> 0.9 -> 1) with a stagger
                let ph = ((f.time - fi * 0.06) / 2.0).rem_euclid(1.0);
                let s = 1.0 - 0.1 * (ph * PI).sin().powi(2);
                ((self.first + self.step * fi) * s, (self.opacity - fi * 0.03).max(0.02))
            };
            if r < 1.0 {
                continue;
            }
            let fill = color.gamma_multiply(a * 0.5);
            fill_convex_fn(p, &circle_points(c, r, 128), 2, |q| fill.gamma_multiply(mask(q.y)));
            let border = color.gamma_multiply((a * 1.2 + 0.03).min(1.0) * if i == n - 1 { 0.6 } else { 1.0 });
            let pts = circle_points(c, r, 128);
            gradient_polyline(p, &pts, true, 1.0, |u| border.gamma_multiply(mask(c.y + (u * TAU).sin() * r)));
        }
    }
}

background!(Ripple);

// ---------------------------------------------------------------------------------------------
// Spotlight
// ---------------------------------------------------------------------------------------------

/// Where [`Spotlight`] rays come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SpotlightOrigin {
    /// Top-left corner, shining down and right.
    TopLeft,
    /// Middle of the top edge, shining straight down.
    #[default]
    TopCenter,
    /// Top-right corner, shining down and left.
    TopRight,
}

#[derive(Clone, Copy, Default)]
struct Aim {
    dir: f32,
    appear: f32,
}

/// Volumetric light rays fanning down from above, each ray gently swaying and shimmering, over a
/// soft cone of light. The rays can lean toward the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::{Spotlight, SpotlightOrigin};
/// ui.add(Spotlight::new().origin(SpotlightOrigin::TopLeft).rays(12));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct Spotlight {
    base: Base,
    origin: SpotlightOrigin,
    color: Option<Color32>,
    rays: usize,
    spread: f32,
    speed: f32,
    follow: bool,
    length: f32,
}

impl Default for Spotlight {
    fn default() -> Self {
        Self::new()
    }
}

impl Spotlight {
    /// Light rays from the top center.
    pub fn new() -> Self {
        Self { base: Base::default(), origin: SpotlightOrigin::TopCenter, color: None, rays: 14, spread: 1.0, speed: 1.0, follow: true, length: 1.0 }
    }
    /// Where the light comes from (default top center).
    pub fn origin(mut self, o: SpotlightOrigin) -> Self {
        self.origin = o;
        self
    }
    /// Light color (default: theme foreground on dark themes, accent on light).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Number of rays (default 14).
    pub fn rays(mut self, n: usize) -> Self {
        self.rays = n.min(64);
        self
    }
    /// Fan angle multiplier (default 1).
    pub fn spread(mut self, s: f32) -> Self {
        self.spread = s.clamp(0.1, 3.0);
        self
    }
    /// Shimmer speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Lean the rays toward the pointer (default true).
    pub fn follow_pointer(mut self, on: bool) -> Self {
        self.follow = on;
        self
    }
    /// Ray length multiplier (default 1).
    pub fn length(mut self, l: f32) -> Self {
        self.length = l.clamp(0.1, 3.0);
        self
    }

    /// A ray as a mesh: bright along its axis and near the source, fading to the sides and end.
    fn ray(m: &mut Mesh, o: Pos2, angle: f32, half_width: f32, len: f32, color: Color32, a: f32, dark: bool) {
        let along = [0.0, 0.12, 0.35, 0.65, 1.0];
        let fall = [1.0, 0.8, 0.45, 0.18, 0.0];
        let base = m.vertices.len() as u32;
        for (s, k) in along.iter().zip(fall) {
            for side in [-1.0, 0.0, 1.0] {
                let ang = angle + side * half_width;
                let p = o + Vec2::angled(ang) * len * *s;
                let al = if side == 0.0 { k * a } else { 0.0 };
                m.colored_vertex(p, light(color, al, dark));
            }
        }
        for r in 0..4u32 {
            for c in 0..2u32 {
                let i = base + r * 3 + c;
                m.add_triangle(i, i + 3, i + 4);
                m.add_triangle(i, i + 4, i + 1);
            }
        }
    }

    /// Paint the rays into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_spotlight") else { return };
        let t = &f.theme;
        let dark = f.dark();
        let color = self.color.unwrap_or(if dark { t.foreground } else { t.accent });
        let (o, base_dir) = match self.origin {
            SpotlightOrigin::TopLeft => (rect.left_top() + vec2(-rect.width() * 0.05, -rect.height() * 0.25), 0.62f32),
            SpotlightOrigin::TopCenter => (pos2(rect.center().x, rect.min.y - rect.height() * 0.2), PI / 2.0),
            SpotlightOrigin::TopRight => (rect.right_top() + vec2(rect.width() * 0.05, -rect.height() * 0.25), PI - 0.62),
        };
        let target = if self.follow { pointer(ui, rect).map(|p| (p - o).angle()) } else { None };
        let aim = glowkit_core::with_state(ui, f.id, Aim::default, |s| {
            if s.appear == 0.0 {
                s.dir = base_dir;
            }
            let goal = target.map_or(base_dir, |a| base_dir + (a - base_dir).clamp(-0.5, 0.5) * 0.6);
            s.dir += (goal - s.dir) * (1.0 - (-f.dt * 3.0).exp());
            s.appear = (s.appear + f.dt / 1.6).min(1.0);
            *s
        });
        let appear = glowkit_core::ease(glowkit_core::Easing::OutCubic, aim.appear);
        let len = (rect.size().length() * 1.15) * self.length;
        let time = f.time * self.speed;
        let p = &f.painter;
        let mut m = Mesh::default();
        // broad soft cone
        let spread = 0.42 * self.spread;
        Self::ray(&mut m, o, aim.dir, spread * 0.9, len * 0.95 * appear, color, 0.16 * appear, dark);
        Self::ray(&mut m, o, aim.dir, spread * 0.45, len * 0.8 * appear, color, 0.14 * appear, dark);
        for i in 0..self.rays as u32 {
            let s = if self.rays > 1 { i as f32 / (self.rays - 1) as f32 - 0.5 } else { 0.0 };
            let sway = (time * (0.3 + 0.4 * h01(i, 1)) + h01(i, 2) * TAU).sin() * 0.04;
            let ang = aim.dir + s * spread * 2.0 + sway;
            let hw = 0.012 + 0.03 * h01(i, 3);
            let shimmer = 0.5 + 0.5 * noise3(i as f32 * 3.1, time * 0.5, 0.0).abs() * 2.0;
            let center_bias = 1.0 - s.abs() * 1.2;
            let a = (0.1 + 0.22 * shimmer) * center_bias.max(0.15) * appear;
            let l = len * (0.55 + 0.4 * h01(i, 4)) * appear;
            Self::ray(&mut m, o, ang, hw, l, color, a, dark);
        }
        p.add(Shape::mesh(m));
        let src = o + Vec2::angled(aim.dir) * rect.height() * 0.25;
        glow_ellipse(p, src, vec2(rect.width() * 0.25, rect.height() * 0.3), light(color, 0.22 * appear, dark));
    }
}

background!(Spotlight);

// ---------------------------------------------------------------------------------------------
// Lamp
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct Appear {
    start: Option<f32>,
}

/// The "lamp" hero: a bright horizontal bar of light that widens into view, casting a fan of
/// colored light up into the dark above it. Put your headline just below the bar.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::Lamp;
/// ui.add(Lamp::new().line_y(0.45));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct Lamp {
    base: Base,
    color: Option<Color32>,
    line_y: f32,
    width: f32,
    replay: bool,
}

impl Default for Lamp {
    fn default() -> Self {
        Self::new()
    }
}

impl Lamp {
    /// A lamp in the theme's cyan.
    pub fn new() -> Self {
        Self { base: Base::default(), color: None, line_y: 0.45, width: 0.5, replay: false }
    }
    /// Light color (default: theme chart cyan).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Vertical position of the bar as a fraction of the height (default 0.45).
    pub fn line_y(mut self, fraction: f32) -> Self {
        self.line_y = fraction.clamp(0.05, 0.95);
        self
    }
    /// Bar width as a fraction of the rect width (default 0.5).
    pub fn width(mut self, fraction: f32) -> Self {
        self.width = fraction.clamp(0.05, 1.0);
        self
    }
    /// Replay the widening animation every few seconds (default false: once).
    pub fn replay(mut self, on: bool) -> Self {
        self.replay = on;
        self
    }

    /// Paint the lamp into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_lamp") else { return };
        let t = &f.theme;
        let dark = f.dark();
        let color = self.color.unwrap_or(t.chart[1]);
        let start = glowkit_core::with_state(ui, f.id, Appear::default, |s| *s.start.get_or_insert(f.time));
        let mut el = f.time - start;
        if self.replay {
            el = el.rem_euclid(6.0);
        }
        let k = glowkit_core::ease(glowkit_core::Easing::InOutCubic, ((el - 0.2) / 1.0).clamp(0.0, 1.0));
        let appear = 0.5 + 0.5 * k;
        let ly = rect.min.y + rect.height() * self.line_y;
        let cx = rect.center().x;
        let hw = rect.width() * self.width * 0.5 * appear;
        let above = Rect::from_min_max(rect.min, pos2(rect.max.x, ly));
        let bg = Rgb::of(f.bg);
        let col = Rgb::of(color);
        let lum = 0.55 + 0.45 * k;
        let p = &f.painter;
        if above.height() > 1.0 {
            shader_grid(p, above, 64, 28, |uv| {
                let x = above.min.x + uv.x * above.width();
                let up = (1.0 - uv.y) * above.height(); // points above the bar
                let dx = (x - cx).abs();
                // the fan widens as it rises; light hugs the bar and the fan's edges
                let edge = hw + up * 1.1;
                let inside = 1.0 - smoothstep(edge * 0.55, edge, dx);
                let vfall = (-up / (above.height() * 0.55 + 1.0)).exp();
                let rim = (-((dx - edge * 0.62) / (edge * 0.22 + 1.0)).powi(2)).exp() * 0.5;
                let i = ((inside * 0.75 + rim) * vfall * lum).min(1.0);
                if dark {
                    bg.add(col.scale(i * 0.85)).to_color()
                } else {
                    bg.lerp(col, i * 0.7).to_color()
                }
            });
        }
        glow_ellipse(p, pos2(cx, ly - 8.0), vec2(hw * 0.75, 46.0 * lum), light(color, 0.55 * lum, dark));
        glow_ellipse(p, pos2(cx, ly), vec2(hw * 1.1, 14.0), light(color, 0.6 * lum, dark));
        // the bar itself
        let bar: Vec<Pos2> = (0..=24).map(|i| pos2(cx - hw + 2.0 * hw * i as f32 / 24.0, ly)).collect();
        let core = if dark { glowkit_core::mix(color, Color32::WHITE, 0.55) } else { color };
        gradient_polyline(p, &bar, false, 5.0, |u| light(color, (1.0 - (u * 2.0 - 1.0).abs().powi(3)) * 0.6, dark));
        gradient_polyline(p, &bar, false, 2.0, |u| core.gamma_multiply(1.0 - (u * 2.0 - 1.0).abs().powi(4)));
    }
}

background!(Lamp);
