use super::shell::{base_setters, card_surface, shade_rounded, show_shell, tracked_pointer, Base, Content};
use crate::buttons::util;
use crate::buttons::{opt_setter, setter};
use egui::epaint::Mesh;
use egui::{pos2, vec2, Color32, FontId, InnerResponse, Pos2, Shape, Ui};
use glowkit_core::anim::smoothstep;
use glowkit_core::color::{gradient, gradient_cyclic, mix, with_alpha};
use glowkit_core::{paint, Rng, Theme};
use std::f32::consts::TAU;

fn falloff(d: f32, radius: f32) -> f32 {
    let k = (1.0 - d / radius.max(1.0)).clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

// ---------------------------------------------------------------------------------------------

/// A card with a soft spotlight that follows the pointer and a border that lights up in a
/// gradient around it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::MagicCard;
/// MagicCard::new().width(320.0).show(ui, |ui| { ui.heading("Magic"); });
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct MagicCard {
    base: Base,
    gradient_size: f32,
    spot_color: Option<Color32>,
    from: Option<Color32>,
    to: Option<Color32>,
    ambient: bool,
}

impl Default for MagicCard {
    fn default() -> Self {
        Self::new()
    }
}

impl MagicCard {
    /// A magic card with theme colors.
    pub fn new() -> Self {
        Self { base: Base::default(), gradient_size: 220.0, spot_color: None, from: None, to: None, ambient: false }
    }
    base_setters!();
    setter!(/// Spotlight radius in points.
        gradient_size: f32);
    opt_setter!(/// Spotlight color inside the card. Defaults to a soft foreground tint.
        spot_color: Color32);
    opt_setter!(/// Border gradient start (at the pointer). Defaults to the theme's first gradient color.
        from: Color32);
    opt_setter!(/// Border gradient end. Defaults to the theme's second gradient color.
        to: Color32);
    setter!(/// When the pointer is away, let the spotlight wander on its own.
        ambient: bool);

    /// Show the card with `add` inside.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let base = self.base.clone();
        show_shell(
            ui,
            &base,
            |ui, ctx| {
                let t = Theme::of(ui);
                let rect = ctx.rect;
                let radius = base.radius(&t);
                let (ptr, k) = tracked_pointer(ui, ctx, self.ambient, 0.0);
                let p = ui.painter();
                card_surface(p, &t, rect, radius, base.background(&t), None);
                let spot = self.spot_color.unwrap_or(if t.dark {
                    with_alpha(Color32::WHITE, 0.09)
                } else {
                    with_alpha(t.accent, 0.08)
                });
                let size = self.gradient_size;
                if k > 0.01 {
                    shade_rounded(p, rect, radius, 10.0, |q| spot.gamma_multiply(falloff(q.distance(ptr), size) * k));
                }
                let from = self.from.unwrap_or(t.gradient[0]);
                let to = self.to.unwrap_or(t.gradient[1]);
                util::border_fn(p, rect, radius, 1.2, |q, _| {
                    let d = q.distance(ptr);
                    let w = falloff(d, size) * k;
                    t.border.lerp_to_gamma(mix(from, to, (d / size).clamp(0.0, 1.0)), w)
                });
                Content::default()
            },
            add,
            |_, _| {},
        )
    }
}

// ---------------------------------------------------------------------------------------------

/// A dark card lit by a spotlight that follows the pointer, optionally revealing a dot grid
/// inside the light.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::SpotlightCard;
/// SpotlightCard::new().width(320.0).show(ui, |ui| { ui.label("Content"); });
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct SpotlightCard {
    base: Base,
    color: Option<Color32>,
    radius_light: f32,
    dots: bool,
    ambient: bool,
}

impl Default for SpotlightCard {
    fn default() -> Self {
        Self::new()
    }
}

impl SpotlightCard {
    /// A spotlight card.
    pub fn new() -> Self {
        Self { base: Base::default(), color: None, radius_light: 200.0, dots: false, ambient: false }
    }
    base_setters!();
    opt_setter!(/// Spotlight color (alpha controls strength). Defaults to soft white / accent.
        color: Color32);
    setter!(/// Spotlight radius in points.
        radius_light: f32);
    setter!(/// Reveal a grid of dots inside the light.
        dots: bool);
    setter!(/// When the pointer is away, let the spotlight wander on its own.
        ambient: bool);

    /// Show the card with `add` inside.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let base = self.base.clone();
        show_shell(
            ui,
            &base,
            |ui, ctx| {
                let t = Theme::of(ui);
                let rect = ctx.rect;
                let radius = base.radius(&t);
                let (ptr, k) = tracked_pointer(ui, ctx, self.ambient, 1.7);
                let p = ui.painter();
                let bg = base.background.unwrap_or(if t.dark { mix(t.card, t.background, 0.5) } else { t.card });
                card_surface(p, &t, rect, radius, bg, Some(t.border));
                let color = self.color.unwrap_or(if t.dark {
                    with_alpha(Color32::WHITE, 0.16)
                } else {
                    with_alpha(t.accent, 0.12)
                });
                let r = self.radius_light;
                if k > 0.01 {
                    shade_rounded(p, rect.shrink(1.0), radius - 1.0, 10.0, |q| {
                        color.gamma_multiply(falloff(q.distance(ptr), r).powf(1.4) * k)
                    });
                    if self.dots {
                        let step = 16.0;
                        let dot = if t.dark { t.gradient[0] } else { t.accent };
                        let mut y = rect.min.y + step / 2.0;
                        while y < rect.max.y {
                            let mut x = rect.min.x + step / 2.0;
                            while x < rect.max.x {
                                let q = pos2(x, y);
                                let f = falloff(q.distance(ptr), r * 0.55) * k;
                                if f > 0.02 {
                                    p.circle_filled(q, 1.3, with_alpha(dot, f * 0.9));
                                }
                                x += step;
                            }
                            y += step;
                        }
                    }
                    // brighter rim where the light touches the border
                    util::border_fn(p, rect, radius, 1.0, |q, _| {
                        color.gamma_multiply(falloff(q.distance(ptr), r * 0.8) * k * 2.5)
                    });
                }
                Content::default()
            },
            add,
            |_, _| {},
        )
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct AngleState {
    angle: f32,
    vel: f32,
}

/// A card whose border lights up in a multi-color arc on the side facing the pointer. The arc
/// glides around the border as the pointer moves, even from outside the card.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::GlowingBorderCard;
/// GlowingBorderCard::new().width(300.0).show(ui, |ui| { ui.label("Glow"); });
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct GlowingBorderCard {
    base: Base,
    colors: Vec<Color32>,
    spread: f32,
    proximity: f32,
    thickness: f32,
    ambient: bool,
}

impl Default for GlowingBorderCard {
    fn default() -> Self {
        Self::new()
    }
}

impl GlowingBorderCard {
    /// A glowing-border card with a warm multi-color arc.
    pub fn new() -> Self {
        Self {
            base: Base::default(),
            colors: vec![
                glowkit_core::hex("#DD7BBB"),
                glowkit_core::hex("#D79F1E"),
                glowkit_core::hex("#5A922C"),
                glowkit_core::hex("#4C7894"),
            ],
            spread: 60.0,
            proximity: 80.0,
            thickness: 2.0,
            ambient: false,
        }
    }
    base_setters!();
    /// Arc colors (repeating along the arc).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        if !colors.is_empty() {
            self.colors = colors.to_vec();
        }
        self
    }
    setter!(/// Half-width of the lit arc in degrees.
        spread: f32);
    setter!(/// How far outside the card (points) the pointer still lights it.
        proximity: f32);
    setter!(/// Lit border thickness in points.
        thickness: f32);
    setter!(/// When the pointer is away, let the light circle the border on its own.
        ambient: bool);

    /// Show the card with `add` inside.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let base = self.base.clone();
        show_shell(
            ui,
            &base,
            |ui, ctx| {
                let t = Theme::of(ui);
                let rect = ctx.rect;
                let radius = base.radius(&t);
                let c = rect.center();
                let time = glowkit_core::time(ui);
                let dt = glowkit_core::dt(ui);
                let near = ui
                    .input(|i| i.pointer.hover_pos())
                    .filter(|p| rect.expand(self.proximity).contains(*p) && ui.clip_rect().contains(*p));
                let (target, active) = match near {
                    Some(p) => {
                        let d = p - c;
                        ((d.y / rect.height()).atan2(d.x / rect.width()), true)
                    }
                    None if self.ambient => (time * 1.1, true),
                    None => (f32::NAN, false),
                };
                let key = ctx.id.with("angle");
                let ang = glowkit_core::with_state(
                    ui,
                    key,
                    || AngleState { angle: if target.is_nan() { 0.0 } else { target }, vel: 0.0 },
                    |s| {
                        if !target.is_nan() {
                            let mut d = (target - s.angle).rem_euclid(TAU);
                            if d > TAU / 2.0 {
                                d -= TAU;
                            }
                            let steps = 4;
                            let h = dt / steps as f32;
                            for _ in 0..steps {
                                let f = 120.0 * d - 20.0 * s.vel;
                                s.vel += f * h;
                                s.angle += s.vel * h;
                                d -= s.vel * h;
                            }
                        }
                        s.angle
                    },
                );
                let k = glowkit_core::anim::follow(ui, ctx.id.with("on"), if active { 1.0 } else { 0.0 }, 6.0);
                if active || k > 0.001 {
                    glowkit_core::animate(ui);
                }
                let p = ui.painter();
                card_surface(p, &t, rect, radius, base.background(&t), Some(t.border));
                if k > 0.01 {
                    let spread = self.spread.to_radians();
                    let colors = &self.colors;
                    let lit = |q: Pos2| -> (f32, f32) {
                        let d = q - c;
                        let a = (d.y / rect.height()).atan2(d.x / rect.width());
                        let mut diff = (a - ang).rem_euclid(TAU);
                        if diff > TAU / 2.0 {
                            diff -= TAU;
                        }
                        (smoothstep(spread, spread * 0.25, diff.abs()) * k, diff / spread)
                    };
                    let color = |q: Pos2| {
                        let (w, u) = lit(q);
                        (gradient_cyclic(colors, u * 0.5 + 0.5), w)
                    };
                    let outer = rect.expand(3.0);
                    util::border_fn(p, outer, radius + 3.0, 8.0, |q, _| {
                        let (c, w) = color(q);
                        with_alpha(c, 0.18 * w)
                    });
                    util::border_fn(p, rect, radius, self.thickness, |q, _| {
                        let (c, w) = color(q);
                        t.border.lerp_to_gamma(c, w)
                    });
                }
                Content::default()
            },
            add,
            |_, _| {},
        )
    }
}

// ---------------------------------------------------------------------------------------------

/// A card covered in scrambled characters that are only revealed (over a glowing gradient) in a
/// circle around the pointer, like encrypted data being decoded.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::EncryptedCard;
/// EncryptedCard::new().badge("hover").size(egui::vec2(300.0, 300.0)).show(ui, |_| {});
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct EncryptedCard {
    base: Base,
    colors: Vec<Color32>,
    reveal_radius: f32,
    badge: Option<String>,
    ambient: bool,
}

impl Default for EncryptedCard {
    fn default() -> Self {
        Self::new()
    }
}

impl EncryptedCard {
    /// An encrypted card with theme colors.
    pub fn new() -> Self {
        Self { base: Base::default(), colors: Vec::new(), reveal_radius: 150.0, badge: None, ambient: false }
    }
    base_setters!();
    /// Colors of the glow behind revealed characters. Defaults to the theme gradient.
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    setter!(/// Radius of the reveal circle in points.
        reveal_radius: f32);
    /// Text in a round badge at the center of the card.
    pub fn badge(mut self, text: impl Into<String>) -> Self {
        self.badge = Some(text.into());
        self
    }
    setter!(/// When the pointer is away, let the reveal wander on its own.
        ambient: bool);

    /// Show the card with `add` inside.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let base = self.base.clone();
        show_shell(
            ui,
            &base,
            |ui, ctx| {
                let t = Theme::of(ui);
                let rect = ctx.rect;
                let radius = base.radius(&t);
                let (ptr, k) = tracked_pointer(ui, ctx, self.ambient, 0.6);
                let colors = if self.colors.is_empty() { t.gradient[..2].to_vec() } else { self.colors.clone() };
                let p = ui.painter();
                card_surface(p, &t, rect, radius, base.background(&t), Some(t.border));
                let rr = self.reveal_radius;
                let mask = |q: Pos2| falloff(q.distance(ptr), rr) * k;
                if k > 0.01 {
                    let inner = rect.shrink(1.0);
                    shade_rounded(p, inner, radius - 1.0, 10.0, |q| {
                        let u = ((q.x - rect.min.x) / rect.width() + (q.y - rect.min.y) / rect.height()) / 2.0;
                        with_alpha(gradient(&colors, u), mask(q).powf(0.8) * if t.dark { 0.85 } else { 0.75 })
                    });
                    // characters: one galley per row, colored per glyph by the mask
                    let time = glowkit_core::time(ui);
                    let tick = if ctx.pointer.is_some() || self.ambient { (time * 14.0) as u64 } else { 0 };
                    let font = FontId::monospace(11.0);
                    let probe = p.layout_no_wrap("M".into(), font.clone(), Color32::WHITE);
                    let cw = probe.size().x.max(1.0);
                    let lh = probe.size().y.max(1.0);
                    let cols = ((rect.width() - 8.0) / cw) as usize;
                    let rows = ((rect.height() - 8.0) / lh) as usize;
                    let chars: Vec<char> = glowkit_core::text::SCRAMBLE_CHARS.chars().collect();
                    let ink = Color32::WHITE;
                    let clip = p.with_clip_rect(rect.shrink(2.0));
                    for row in 0..rows {
                        let y = rect.min.y + 4.0 + row as f32 * lh;
                        if (y + lh / 2.0 - ptr.y).abs() > rr {
                            continue;
                        }
                        let mut rng = Rng::new(
                            ctx.id.value() ^ (row as u64).wrapping_mul(0x9E37_79B9) ^ tick.wrapping_mul(0x85EB_CA6B),
                        );
                        let line: String = (0..cols).map(|_| chars[rng.below(chars.len())]).collect();
                        let g = clip.layout_no_wrap(line, font.clone(), ink);
                        paint::galley_with(&clip, pos2(rect.min.x + 4.0, y), &g, |q| {
                            with_alpha(ink, (mask(q) * 1.3).min(1.0) * 0.9)
                        });
                    }
                }
                if let Some(b) = &self.badge {
                    let c = rect.center();
                    let g = p.layout_no_wrap(b.clone(), FontId::proportional(22.0), t.foreground);
                    let r = (g.size().x / 2.0 + 22.0).max(44.0);
                    paint::glow(p, c, r * 1.6, with_alpha(if t.dark { Color32::BLACK } else { Color32::WHITE }, 0.6));
                    p.circle_filled(
                        c,
                        r,
                        if t.dark { with_alpha(t.background, 0.85) } else { with_alpha(Color32::WHITE, 0.9) },
                    );
                    p.circle_stroke(c, r, egui::Stroke::new(1.0, t.border));
                    util::galley_centered(p, &g, c, t.foreground);
                }
                Content::default()
            },
            add,
            |_, _| {},
        )
    }
}

// ---------------------------------------------------------------------------------------------

/// A card that fills with shimmering pixels on hover, spreading out from the center.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::PixelCard;
/// PixelCard::new().size(egui::vec2(260.0, 320.0)).show(ui, |ui| { ui.label("Pixels"); });
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct PixelCard {
    base: Base,
    colors: Vec<Color32>,
    gap: f32,
    speed: f32,
    ambient: bool,
}

impl Default for PixelCard {
    fn default() -> Self {
        Self::new()
    }
}

impl PixelCard {
    /// A pixel card in cool blues.
    pub fn new() -> Self {
        Self { base: Base::default(), colors: Vec::new(), gap: 7.0, speed: 1.0, ambient: false }
    }
    base_setters!();
    /// Pixel colors. Defaults to tints of the accent.
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    setter!(/// Pixel spacing in points.
        gap: f32);
    setter!(/// Shimmer speed multiplier.
        speed: f32);
    setter!(/// When the pointer is away, keep fading the pixels in and out on their own.
        ambient: bool);

    /// Show the card with `add` inside.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let base = self.base.clone();
        show_shell(
            ui,
            &base,
            |ui, ctx| {
                let t = Theme::of(ui);
                let rect = ctx.rect;
                let radius = base.radius(&t);
                let time = glowkit_core::time(ui);
                let on = ctx.pointer.is_some() || (self.ambient && (time / 3.2).fract() < 0.6);
                if self.ambient {
                    glowkit_core::animate(ui);
                }
                let h = glowkit_core::anim::follow(
                    ui,
                    ctx.id.with("h"),
                    if on { 1.0 } else { 0.0 },
                    if on { 1.8 } else { 3.0 },
                );
                let p = ui.painter();
                let bg = base.background.unwrap_or(if t.dark { mix(t.card, t.background, 0.5) } else { t.card });
                card_surface(p, &t, rect, radius, bg, Some(t.border));
                if h < 0.003 {
                    return Content::default();
                }
                glowkit_core::animate(ui);
                let colors = if self.colors.is_empty() {
                    vec![mix(t.accent, Color32::WHITE, 0.75), mix(t.accent, Color32::WHITE, 0.35), t.accent]
                } else {
                    self.colors.clone()
                };
                let gap = self.gap.max(3.0);
                let max_s = gap * 0.3;
                let c = rect.center();
                let maxd = (rect.size() / 2.0).length();
                let mut mesh = Mesh::default();
                let inner = rect.shrink(gap * 0.5);
                let cols = (inner.width() / gap) as usize;
                let rows = (inner.height() / gap) as usize;
                let ox = inner.min.x + (inner.width() - cols as f32 * gap) / 2.0 + gap / 2.0;
                let oy = inner.min.y + (inner.height() - rows as f32 * gap) / 2.0 + gap / 2.0;
                for j in 0..rows {
                    for i in 0..cols {
                        let q = pos2(ox + i as f32 * gap, oy + j as f32 * gap);
                        if super::shell::project_into(rect.shrink(2.0), radius, q).distance(q) > 0.1 {
                            continue;
                        }
                        let seed = (j * 7919 + i) as u64;
                        let r = glowkit_core::rng::hash01(seed);
                        let dn = q.distance(c) / maxd;
                        let k = ((h - dn * 0.6 - r * 0.15) / 0.25).clamp(0.0, 1.0);
                        if k <= 0.0 {
                            continue;
                        }
                        let tw = 0.55 + 0.45 * (time * self.speed * (2.0 + 3.0 * r) + r * TAU).sin();
                        let s = max_s * k * tw;
                        if s < 0.3 {
                            continue;
                        }
                        let col = colors
                            [(glowkit_core::rng::hash01(seed ^ 0xABCD) * colors.len() as f32) as usize % colors.len()];
                        let col = col.gamma_multiply((0.3 + 0.5 * k) * if t.dark { 1.0 } else { 0.85 });
                        let idx = mesh.vertices.len() as u32;
                        mesh.colored_vertex(q + vec2(-s, -s), col);
                        mesh.colored_vertex(q + vec2(s, -s), col);
                        mesh.colored_vertex(q + vec2(s, s), col);
                        mesh.colored_vertex(q + vec2(-s, s), col);
                        mesh.add_triangle(idx, idx + 1, idx + 2);
                        mesh.add_triangle(idx, idx + 2, idx + 3);
                    }
                }
                p.add(Shape::mesh(mesh));
                Content::default()
            },
            add,
            |_, _| {},
        )
    }
}

// ---------------------------------------------------------------------------------------------

/// A card that catches a diagonal streak of glare sweeping across it on hover, plus a soft
/// sheen that follows the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::GlareCard;
/// GlareCard::new().size(egui::vec2(280.0, 360.0)).show(ui, |ui| { ui.heading("Glare"); });
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct GlareCard {
    base: Base,
    color: Option<Color32>,
    opacity: f32,
    angle: f32,
    duration: f32,
    band: f32,
    auto_every: Option<f32>,
}

impl Default for GlareCard {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
struct GlareState {
    start: f32,
    was: bool,
}

impl GlareCard {
    /// A glare card.
    pub fn new() -> Self {
        Self {
            base: Base::default(),
            color: None,
            opacity: 0.45,
            angle: -45.0,
            duration: 0.8,
            band: 0.18,
            auto_every: None,
        }
    }
    base_setters!();
    opt_setter!(/// Glare color. Defaults to white.
        color: Color32);
    setter!(/// Peak glare opacity (0..1).
        opacity: f32);
    setter!(/// Angle of the glare streak in degrees.
        angle: f32);
    setter!(/// Seconds for one sweep.
        duration: f32);
    setter!(/// Width of the streak as a fraction of the card diagonal.
        band: f32);
    opt_setter!(/// Also sweep on its own every this many seconds.
        auto_every: f32);

    /// Show the card with `add` inside.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let base = self.base.clone();
        let t = Theme::of(ui);
        show_shell(
            ui,
            &base,
            |ui, ctx| {
                let radius = base.radius(&t);
                card_surface(ui.painter(), &t, ctx.rect, radius, base.background(&t), Some(t.border));
                Content::default()
            },
            add,
            |ui, ctx| {
                let rect = ctx.rect;
                let radius = base.radius(&t);
                let time = glowkit_core::time(ui);
                let hovered = ctx.pointer.is_some();
                let key = ctx.id.with("glare");
                let st = glowkit_core::with_state(
                    ui,
                    key,
                    || GlareState { start: -100.0, was: false },
                    |s| {
                        if hovered && !s.was {
                            s.start = time;
                        }
                        if let Some(every) = self.auto_every {
                            if time - s.start > every.max(self.duration + 0.2) {
                                s.start = time;
                            }
                        }
                        s.was = hovered;
                        *s
                    },
                );
                if self.auto_every.is_some() {
                    glowkit_core::animate(ui);
                }
                let ph = (time - st.start) / self.duration.max(0.1);
                let col = self.color.unwrap_or(Color32::WHITE);
                let a = self.angle.to_radians();
                let dir = vec2(a.cos(), a.sin()).rot90();
                let half = (rect.width() * dir.x.abs() + rect.height() * dir.y.abs()) / 2.0;
                let p = ui.painter();
                let sheen = glowkit_core::anim::follow(ui, ctx.id.with("sheen"), if hovered { 1.0 } else { 0.0 }, 8.0);
                if (0.0..1.0).contains(&ph) {
                    glowkit_core::animate(ui);
                    let e = glowkit_core::ease(glowkit_core::Easing::InOutCubic, ph);
                    let center = -1.0 - self.band * 2.0 + e * (2.0 + self.band * 4.0);
                    let op = self.opacity;
                    let band = self.band * 2.0;
                    shade_rounded(p, rect, radius, 8.0, |q| {
                        let u = (q - rect.center()).dot(dir) / half;
                        let d = ((u - center) / band).abs();
                        let k = (1.0 - d).clamp(0.0, 1.0);
                        with_alpha(col, k * k * op)
                    });
                }
                if sheen > 0.01 {
                    if let Some(ptr) = ctx.pointer.or(Some(rect.center())) {
                        let r = rect.width().max(rect.height()) * 0.7;
                        shade_rounded(p, rect, radius, 12.0, |q| {
                            with_alpha(col, falloff(q.distance(ptr), r) * 0.12 * sheen)
                        });
                    }
                }
            },
        )
    }
}
