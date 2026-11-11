use super::shell::{base_setters, card_surface, show_shell, Base, Content};
use crate::buttons::util::{self, cr};
use crate::buttons::{opt_setter, setter};
use egui::{vec2, Color32, InnerResponse, Pos2, Rect, Shape, Stroke, Ui};
use glowkit_core::color::{gradient, gradient_cyclic, mix, palettes, with_alpha};
use glowkit_core::{noise, paint, Theme};
use std::f32::consts::TAU;

/// Show a plain card surface with `effect` painted on its border, then the content.
fn bordered_card<R>(ui: &mut Ui, base: &Base, border: bool, effect: impl FnOnce(&Ui, Rect), add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    show_shell(
        ui,
        base,
        |ui, ctx| {
            let t = Theme::of(ui);
            let radius = base.radius(&t);
            card_surface(ui.painter(), &t, ctx.rect, radius, base.background(&t), border.then_some(t.border));
            effect(ui, ctx.rect);
            Content::default()
        },
        add,
        |_, _| {},
    )
}

// ---------------------------------------------------------------------------------------------

/// A comet of light that travels around the border of any rect, fading from one color into
/// another along its tail.
///
/// Use [`BorderBeam::show`] for a ready-made card, or [`BorderBeam::paint`] around anything.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::BorderBeam;
/// BorderBeam::new().width(320.0).show(ui, |ui| { ui.heading("Login"); });
/// ```
#[must_use = "call `.show(ui, ...)` or `.paint(ui, rect)`"]
pub struct BorderBeam {
    base: Base,
    from: Option<Color32>,
    to: Option<Color32>,
    duration: f32,
    length: f32,
    thickness: f32,
    delay: f32,
    reverse: bool,
}

impl Default for BorderBeam {
    fn default() -> Self {
        Self::new()
    }
}

impl BorderBeam {
    /// A border beam with theme colors.
    pub fn new() -> Self {
        Self { base: Base::default(), from: None, to: None, duration: 6.0, length: 0.22, thickness: 1.6, delay: 0.0, reverse: false }
    }
    base_setters!();
    opt_setter!(/// Tail color. Defaults to the theme's third gradient color.
        from: Color32);
    opt_setter!(/// Head color. Defaults to the theme's first gradient color.
        to: Color32);
    setter!(/// Seconds per lap.
        duration: f32);
    setter!(/// Beam length as a fraction of the perimeter.
        length: f32);
    setter!(/// Beam thickness in points.
        thickness: f32);
    setter!(/// Start offset in seconds (stagger several beams).
        delay: f32);
    setter!(/// Travel counter-clockwise.
        reverse: bool);

    /// Paint just the beam around `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let t = Theme::of(ui);
        let radius = self.base.radius(&t);
        let from = self.from.unwrap_or(t.gradient[2]);
        let to = self.to.unwrap_or(t.gradient[0]);
        let mut head = ((glowkit_core::time(ui) + self.delay) / self.duration.max(0.2)).fract();
        if self.reverse {
            head = 1.0 - head;
        }
        let len = self.length.clamp(0.02, 0.9);
        let dir = if self.reverse { -1.0 } else { 1.0 };
        let p = ui.painter();
        let hp = util::perimeter_point(rect, radius, head);
        paint::glow(p, hp, 26.0, with_alpha(to, 0.22));
        util::border_fn(p, rect, radius, self.thickness, |_, s| {
            let d = ((head - s) * dir).rem_euclid(1.0);
            if d < len {
                let k = 1.0 - d / len;
                mix(from, to, k).gamma_multiply(k.powf(1.3))
            } else {
                Color32::TRANSPARENT
            }
        });
        glowkit_core::animate(ui);
    }

    /// Show a card with the beam on its border and `add` inside.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        bordered_card(ui, &self.base, true, |ui, rect| self.paint(ui, rect), add)
    }
}

// ---------------------------------------------------------------------------------------------

/// A border that shimmers with a band of multi-colored light drifting diagonally across it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::ShineBorder;
/// ShineBorder::new().width(320.0).show(ui, |ui| { ui.label("Shiny"); });
/// ```
#[must_use = "call `.show(ui, ...)` or `.paint(ui, rect)`"]
pub struct ShineBorder {
    base: Base,
    colors: Vec<Color32>,
    duration: f32,
    thickness: f32,
}

impl Default for ShineBorder {
    fn default() -> Self {
        Self::new()
    }
}

impl ShineBorder {
    /// A shine border with Magic-style colors.
    pub fn new() -> Self {
        Self { base: Base::default(), colors: palettes::MAGIC.to_vec(), duration: 7.0, thickness: 1.6 }
    }
    base_setters!();
    /// Shine colors.
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        if !colors.is_empty() {
            self.colors = colors.to_vec();
        }
        self
    }
    setter!(/// Seconds per sweep.
        duration: f32);
    setter!(/// Border thickness in points.
        thickness: f32);

    /// Paint just the shining border around `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let t = Theme::of(ui);
        let radius = self.base.radius(&t);
        let time = glowkit_core::time(ui) / self.duration.max(0.3);
        let ph = time.fract();
        // the shine band slides along the diagonal, sweeping out and back in
        let c = -0.4 + 1.8 * (0.5 - 0.5 * (ph * TAU).cos());
        let diag = vec2(rect.width(), rect.height());
        let colors = &self.colors;
        util::border_fn(ui.painter(), rect, radius, self.thickness, |q: Pos2, _| {
            let u = (q - rect.min).dot(diag) / diag.length_sq();
            let d = (u - c) / 0.32;
            let k = (-d * d).exp();
            gradient_cyclic(colors, u * 1.2 + time * 0.5).gamma_multiply(k)
        });
        glowkit_core::animate(ui);
    }

    /// Show a card with a shining border and `add` inside.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        bordered_card(ui, &self.base, true, |ui, rect| self.paint(ui, rect), add)
    }
}

// ---------------------------------------------------------------------------------------------

/// A jittery, crackling border of electricity with layered glow.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::ElectricBorder;
/// ElectricBorder::new().width(300.0).show(ui, |ui| { ui.label("Zap"); });
/// ```
#[must_use = "call `.show(ui, ...)` or `.paint(ui, rect)`"]
pub struct ElectricBorder {
    base: Base,
    color: Option<Color32>,
    speed: f32,
    chaos: f32,
    thickness: f32,
}

impl Default for ElectricBorder {
    fn default() -> Self {
        Self::new()
    }
}

impl ElectricBorder {
    /// An electric border in cyan.
    pub fn new() -> Self {
        Self { base: Base::default(), color: None, speed: 1.0, chaos: 1.0, thickness: 1.6 }
    }
    base_setters!();
    opt_setter!(/// Arc color. Defaults to an electric cyan (accent in light themes).
        color: Color32);
    setter!(/// Animation speed multiplier.
        speed: f32);
    setter!(/// Displacement multiplier (0 = calm, 2 = wild).
        chaos: f32);
    setter!(/// Core line thickness in points.
        thickness: f32);

    /// Paint just the electric border around `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let t = Theme::of(ui);
        let radius = self.base.radius(&t);
        let color = self.color.unwrap_or(if t.dark { glowkit_core::hex("#7DF9FF") } else { t.accent });
        let time = glowkit_core::time(ui) * self.speed;
        let base_pts = util::outline(rect, radius, 3.0);
        let n = base_pts.len();
        let perim: f32 = (0..n).map(|i| base_pts[i].distance(base_pts[(i + 1) % n])).sum();
        let center = rect.center();
        let amp = 5.0 * self.chaos;
        let displaced = |seed: f32, scale: f32| -> Vec<Pos2> {
            let mut s = 0.0;
            (0..n)
                .map(|i| {
                    let p = base_pts[i];
                    let next = base_pts[(i + 1) % n];
                    let prev = base_pts[(i + n - 1) % n];
                    let normal = (next - prev).normalized().rot90();
                    let normal = if normal.dot(p - center) < 0.0 { -normal } else { normal };
                    let u = s / perim.max(1.0);
                    s += p.distance(next);
                    // two octaves of looping noise around the loop + fast flicker
                    let ang = u * TAU;
                    let (cx, cy) = (ang.cos() * 6.0, ang.sin() * 6.0);
                    let d = noise::fbm3(cx + seed, cy, time * 1.6 + seed, 3) + 0.35 * noise::noise3(cx * 3.0, cy * 3.0 + seed, time * 7.0);
                    p + normal * d * amp * scale
                })
                .collect()
        };
        let p = ui.painter();
        let outer = displaced(0.0, 1.0);
        util::ribbon(p, &outer, 10.0, |_, _| with_alpha(color, 0.10));
        util::ribbon(p, &outer, 4.5, |_, _| with_alpha(color, 0.28));
        util::ribbon(p, &outer, self.thickness, |_, _| mix(color, Color32::WHITE, 0.35));
        let second = displaced(17.3, 0.7);
        util::ribbon(p, &second, self.thickness * 0.7, |_, _| with_alpha(color, 0.55));
        // soft inner glow
        paint::soft_shadow(p, rect.shrink(4.0), radius, 28.0, egui::Vec2::ZERO, with_alpha(color, 0.10));
        glowkit_core::animate(ui);
    }

    /// Show a card with an electric border and `add` inside.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        bordered_card(ui, &self.base, false, |ui, rect| self.paint(ui, rect), add)
    }
}

// ---------------------------------------------------------------------------------------------

/// A border filled with a conic gradient that keeps rotating, with an optional soft glow.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::AnimatedGradientBorder;
/// AnimatedGradientBorder::new().glow(true).width(320.0).show(ui, |ui| { ui.label("Hi"); });
/// ```
#[must_use = "call `.show(ui, ...)` or `.paint(ui, rect)`"]
pub struct AnimatedGradientBorder {
    base: Base,
    colors: Vec<Color32>,
    duration: f32,
    thickness: f32,
    glow: bool,
}

impl Default for AnimatedGradientBorder {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimatedGradientBorder {
    /// A rotating gradient border with the theme gradient.
    pub fn new() -> Self {
        Self { base: Base::default(), colors: Vec::new(), duration: 4.0, thickness: 2.0, glow: false }
    }
    base_setters!();
    /// Gradient stops (wraps around). Defaults to the theme gradient.
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    setter!(/// Seconds per rotation.
        duration: f32);
    setter!(/// Border thickness in points.
        thickness: f32);
    setter!(/// Paint a blurred copy of the gradient behind the card.
        glow: bool);

    /// Paint the glow (if enabled) and the border around `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        self.paint_glow(ui, rect);
        self.paint_border(ui, rect);
    }

    fn colors_for(&self, t: &Theme) -> Vec<Color32> {
        if self.colors.is_empty() {
            t.gradient.to_vec()
        } else {
            self.colors.clone()
        }
    }

    fn paint_glow(&self, ui: &Ui, rect: Rect) {
        if !self.glow {
            return;
        }
        let t = Theme::of(ui);
        let radius = self.base.radius(&t);
        let colors = self.colors_for(&t);
        let rot = glowkit_core::time(ui) / self.duration.max(0.3);
        let p = ui.painter();
        let n = 36;
        for i in 0..n {
            let s = i as f32 / n as f32;
            let q = util::perimeter_point(rect, radius, s);
            let d = q - rect.center();
            let a = (d.y / rect.height()).atan2(d.x / rect.width()) / TAU + 0.5;
            let c = gradient_cyclic(&colors, a - rot);
            paint::glow(p, q, rect.width().min(rect.height()) * 0.42, with_alpha(c, if t.dark { 0.10 } else { 0.07 }));
        }
    }

    fn paint_border(&self, ui: &Ui, rect: Rect) {
        let t = Theme::of(ui);
        let radius = self.base.radius(&t);
        let colors = self.colors_for(&t);
        let rot = glowkit_core::time(ui) / self.duration.max(0.3);
        let c = rect.center();
        util::border_fn(ui.painter(), rect, radius, self.thickness, |q, _| {
            let d = q - c;
            let a = (d.y / rect.height()).atan2(d.x / rect.width()) / TAU + 0.5;
            gradient_cyclic(&colors, a - rot)
        });
        glowkit_core::animate(ui);
    }

    /// Show a card with the rotating gradient border and `add` inside.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let base = self.base.clone();
        show_shell(
            ui,
            &base,
            |ui, ctx| {
                let t = Theme::of(ui);
                let radius = base.radius(&t);
                self.paint_glow(ui, ctx.rect);
                card_surface(ui.painter(), &t, ctx.rect, radius, base.background(&t), None);
                self.paint_border(ui, ctx.rect);
                Content::default()
            },
            add,
            |_, _| {},
        )
    }
}

// ---------------------------------------------------------------------------------------------

/// A dashed border whose dashes march around the rect, like a selection marquee.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::DashedMarchingBorder;
/// DashedMarchingBorder::new().width(300.0).show(ui, |ui| { ui.label("Drop files here"); });
/// ```
#[must_use = "call `.show(ui, ...)` or `.paint(ui, rect)`"]
pub struct DashedMarchingBorder {
    base: Base,
    color: Option<Color32>,
    dash: f32,
    gap: f32,
    speed: f32,
    thickness: f32,
    gradient: bool,
}

impl Default for DashedMarchingBorder {
    fn default() -> Self {
        Self::new()
    }
}

impl DashedMarchingBorder {
    /// A marching-ants border.
    pub fn new() -> Self {
        Self { base: Base::default(), color: None, dash: 10.0, gap: 7.0, speed: 30.0, thickness: 1.6, gradient: false }
    }
    base_setters!();
    opt_setter!(/// Dash color. Defaults to the muted foreground.
        color: Color32);
    setter!(/// Dash length in points.
        dash: f32);
    setter!(/// Gap length in points.
        gap: f32);
    setter!(/// Marching speed in points per second (negative = reverse).
        speed: f32);
    setter!(/// Line thickness in points.
        thickness: f32);
    setter!(/// Color the dashes with the theme gradient around the loop.
        gradient: bool);

    /// Paint just the marching border around `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let t = Theme::of(ui);
        let radius = self.base.radius(&t);
        let color = self.color.unwrap_or(t.muted_foreground);
        let r = rect.shrink(self.thickness / 2.0);
        let pts = util::outline(r, (radius - self.thickness / 2.0).max(0.0), 2.0);
        let n = pts.len();
        let mut lens = Vec::with_capacity(n + 1);
        let mut total = 0.0;
        lens.push(0.0);
        for i in 0..n {
            total += pts[i].distance(pts[(i + 1) % n]);
            lens.push(total);
        }
        // fit a whole number of dash periods around the loop so the seam is invisible
        let period0 = (self.dash + self.gap).max(2.0);
        let count = (total / period0).round().max(1.0);
        let period = total / count;
        let dash = period * self.dash / period0;
        let offset = (glowkit_core::time(ui) * self.speed).rem_euclid(period);
        let at = |s: f32| -> Pos2 {
            let s = s.rem_euclid(total);
            let i = lens.partition_point(|&l| l <= s).saturating_sub(1).min(n - 1);
            let seg = (lens[i + 1] - lens[i]).max(1e-4);
            pts[i].lerp(pts[(i + 1) % n], (s - lens[i]) / seg)
        };
        let p = ui.painter();
        let gcol = t.gradient;
        for k in 0..count as usize {
            let s0 = k as f32 * period + offset;
            let s1 = s0 + dash;
            let steps = ((dash / 2.0).ceil() as usize).max(1);
            let mut line: Vec<Pos2> = (0..steps).map(|j| at(s0 + dash * j as f32 / steps as f32)).collect();
            line.push(at(s1));
            let c = if self.gradient { gradient_cyclic(&gcol, s0 / total) } else { color };
            p.add(Shape::line(line, Stroke::new(self.thickness, c)));
        }
        glowkit_core::animate(ui);
    }

    /// Show a card with the marching border and `add` inside.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        bordered_card(ui, &self.base, false, |ui, rect| self.paint(ui, rect), add)
    }
}

// ---------------------------------------------------------------------------------------------

/// A card wrapped in a thick, animated two-tone gradient border that bleeds a big neon glow
/// around itself.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::NeonGradientCard;
/// NeonGradientCard::new().width(320.0).show(ui, |ui| { ui.heading("Neon"); });
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct NeonGradientCard {
    base: Base,
    first: Option<Color32>,
    second: Option<Color32>,
    border_size: f32,
    duration: f32,
    glow: f32,
}

impl Default for NeonGradientCard {
    fn default() -> Self {
        Self::new()
    }
}

impl NeonGradientCard {
    /// A neon gradient card with pink/cyan defaults.
    pub fn new() -> Self {
        Self { base: Base::default(), first: None, second: None, border_size: 3.0, duration: 4.0, glow: 1.0 }
    }
    base_setters!();
    opt_setter!(/// First neon color. Defaults to hot pink.
        first: Color32);
    opt_setter!(/// Second neon color. Defaults to cyan.
        second: Color32);
    setter!(/// Border thickness in points.
        border_size: f32);
    setter!(/// Seconds per color rotation.
        duration: f32);
    setter!(/// Glow strength multiplier.
        glow: f32);

    /// Show the neon card with `add` inside.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let base = self.base.clone();
        show_shell(
            ui,
            &base,
            |ui, ctx| {
                let t = Theme::of(ui);
                let rect = ctx.rect;
                let radius = base.radius(&t);
                let a = self.first.unwrap_or(glowkit_core::hex("#FF00AA"));
                let b = self.second.unwrap_or(glowkit_core::hex("#00FFF1"));
                let stops = [a, b];
                let rot = glowkit_core::time(ui) / self.duration.max(0.3);
                let color_at = |q: Pos2| {
                    let d = q - rect.center();
                    let ang = (d.y / rect.height()).atan2(d.x / rect.width()) / TAU + 0.5;
                    let u = (ang - rot).rem_euclid(1.0);
                    gradient(&stops, 1.0 - (u * 2.0 - 1.0).abs())
                };
                let p = ui.painter();
                let n = 40;
                let glow_a = (if t.dark { 0.13 } else { 0.10 }) * self.glow;
                for i in 0..n {
                    let s = i as f32 / n as f32;
                    let q = util::perimeter_point(rect, radius, s);
                    paint::glow(p, q, rect.width().min(rect.height()) * 0.5, with_alpha(color_at(q), glow_a));
                }
                paint::fill_convex_fn(p, &paint::rounded_rect_points(rect, radius, 10), 8, color_at);
                let inner = rect.shrink(self.border_size);
                p.rect_filled(inner, cr((radius - self.border_size).max(0.0)), base.background(&t));
                glowkit_core::animate(ui);
                Content::default()
            },
            add,
            |_, _| {},
        )
    }
}
