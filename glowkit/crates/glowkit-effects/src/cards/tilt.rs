use super::shell::{base_setters, show_shell, tracked_pointer, Base, Content};
use crate::buttons::setter;
use crate::buttons::util::{self, cr};
use egui::{pos2, vec2, Color32, InnerResponse, Pos2, Rect, Ui, Vec2};
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::{paint, Rng, Theme};

// ---------------------------------------------------------------------------------------------

/// A card that tilts in 3D toward the pointer, with real perspective, a moving glare and
/// parallax content.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::TiltCard;
/// TiltCard::new().size(egui::vec2(280.0, 340.0)).show(ui, |ui| { ui.heading("Tilt"); });
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct TiltCard {
    base: Base,
    max_tilt: f32,
    scale: f32,
    glare: bool,
    parallax: f32,
    ambient: bool,
    colors: Option<[Color32; 2]>,
}

impl Default for TiltCard {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
struct Proj {
    c: Pos2,
    ax: f32,
    ay: f32,
    scale: f32,
    persp: f32,
}

impl Proj {
    /// Local point (relative to card center) to screen, plus its depth.
    fn apply(&self, v: Vec2) -> (Pos2, f32) {
        let (sy, cy) = self.ay.sin_cos();
        let (sx, cx) = self.ax.sin_cos();
        let x1 = v.x * cy;
        let z1 = v.x * sy;
        let y2 = v.y * cx - z1 * sx;
        let z2 = v.y * sx + z1 * cx;
        let f = self.persp / (self.persp + z2);
        (self.c + vec2(x1, y2) * f * self.scale, z2)
    }
}

impl TiltCard {
    /// A tilt card.
    pub fn new() -> Self {
        Self { base: Base::default(), max_tilt: 14.0, scale: 1.04, glare: true, parallax: 10.0, ambient: false, colors: None }
    }
    base_setters!();
    setter!(/// Maximum tilt in degrees.
        max_tilt: f32);
    setter!(/// Scale while hovered (1 = none).
        scale: f32);
    setter!(/// Paint a glare that follows the pointer.
        glare: bool);
    setter!(/// How far (points) the content shifts with the tilt.
        parallax: f32);
    setter!(/// When the pointer is away, sway gently on its own.
        ambient: bool);
    /// Fill the card with a diagonal two-color gradient instead of the card color.
    pub fn gradient(mut self, a: Color32, b: Color32) -> Self {
        self.colors = Some([a, b]);
        self
    }

    /// Show the card with `add` inside.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let base = self.base.clone();
        let t = Theme::of(ui);
        let proj_store: std::cell::Cell<Option<(Proj, Pos2, f32)>> = std::cell::Cell::new(None);
        let max = self.max_tilt.to_radians();
        show_shell(
            ui,
            &base,
            |ui, ctx| {
                let rect = ctx.rect;
                let radius = base.radius(&t);
                let (ptr, k) = tracked_pointer(ui, ctx, self.ambient, 0.3);
                let n = (ptr - rect.center()) / (rect.size() / 2.0);
                let n = vec2(n.x.clamp(-1.0, 1.0), n.y.clamp(-1.0, 1.0)) * k;
                let tilt = glowkit_core::anim::spring2(ui, ctx.id.with("tilt"), n, 200.0, 18.0);
                let hk = glowkit_core::anim::spring(ui, ctx.id.with("hk"), if ctx.pointer.is_some() { 1.0 } else { 0.0 }, 200.0, 20.0);
                let proj = Proj { c: rect.center(), ax: tilt.y * max, ay: tilt.x * max, scale: 1.0 + (self.scale - 1.0) * hk, persp: 900.0 };
                let half = rect.size() / 2.0;
                let local = paint::rounded_rect_points(Rect::from_center_size(Pos2::ZERO, rect.size()), radius, 10);
                let local = paint::subdivide_closed(&local, 16.0);
                let pts: Vec<Pos2> = local.iter().map(|p| proj.apply(p.to_vec2()).0).collect();
                let p = ui.painter();
                // shadow follows the tilt
                let shadow_rect = Rect::from_center_size(rect.center() + vec2(-tilt.x * 10.0, -tilt.y * 10.0 + 14.0), rect.size() * proj.scale * 0.92);
                paint::soft_shadow(p, shadow_rect, radius, 30.0 + 10.0 * hk, Vec2::ZERO, with_alpha(Color32::BLACK, if t.dark { 0.55 } else { 0.18 }));
                let fill = base.background(&t);
                let colors = self.colors;
                paint::fill_convex_fn(p, &pts, 6, |q| {
                    let v = q - rect.center();
                    let base_c = match colors {
                        Some([a, b]) => mix(a, b, ((v.x / half.x + v.y / half.y) * 0.25 + 0.5).clamp(0.0, 1.0)),
                        None => fill,
                    };
                    // light the side that comes toward the viewer
                    let lightness = -(tilt.x * v.x / half.x + tilt.y * v.y / half.y) * 0.05;
                    shade(base_c, lightness)
                });
                util::ribbon(p, &pts, 1.0, |q, _| {
                    let v = (q - rect.center()) / half;
                    let up = (-(v.x + v.y) * 0.5 + 0.5).clamp(0.0, 1.0);
                    if t.dark {
                        with_alpha(Color32::WHITE, 0.06 + 0.16 * up)
                    } else {
                        t.border.lerp_to_gamma(Color32::WHITE, up * 0.5)
                    }
                });
                proj_store.set(Some((proj, ptr, k)));
                Content { offset: vec2(tilt.x, tilt.y) * self.parallax, opacity: 1.0 }
            },
            add,
            |ui, ctx| {
                if !self.glare {
                    return;
                }
                let Some((proj, ptr, k)) = proj_store.get() else { return };
                if k < 0.01 {
                    return;
                }
                let rect = ctx.rect;
                let radius = base.radius(&t);
                let local = paint::rounded_rect_points(Rect::from_center_size(Pos2::ZERO, rect.size()), radius, 10);
                let local = paint::subdivide_closed(&local, 16.0);
                let pts: Vec<Pos2> = local.iter().map(|p| proj.apply(p.to_vec2()).0).collect();
                let r = rect.width().max(rect.height()) * 0.8;
                paint::fill_convex_fn(ui.painter(), &pts, 8, |q| {
                    let d = q.distance(ptr) / r;
                    let f = (1.0 - d).clamp(0.0, 1.0);
                    with_alpha(Color32::WHITE, f * f * 0.22 * k)
                });
            },
        )
    }
}

// ---------------------------------------------------------------------------------------------

/// A bold, colorful card that wobbles toward the pointer on a springy rubber band while its
/// content drifts the other way. A fine grain texture gives it a printed feel.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::WobbleCard;
/// WobbleCard::new().width(400.0).show(ui, |ui| { ui.heading("Wobble"); });
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct WobbleCard {
    base: Base,
    colors: Option<[Color32; 2]>,
    strength: f32,
    ambient: bool,
}

impl Default for WobbleCard {
    fn default() -> Self {
        Self::new()
    }
}

impl WobbleCard {
    /// A wobble card in the accent color.
    pub fn new() -> Self {
        Self { base: Base::default(), colors: None, strength: 1.0, ambient: false }
    }
    base_setters!();
    /// Fill gradient (top-left to bottom-right). Defaults to shades of the accent.
    pub fn colors(mut self, a: Color32, b: Color32) -> Self {
        self.colors = Some([a, b]);
        self
    }
    setter!(/// Movement multiplier.
        strength: f32);
    setter!(/// When the pointer is away, wobble gently on its own.
        ambient: bool);

    /// Show the card with `add` inside.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let base = self.base.clone();
        let t = Theme::of(ui);
        show_shell(
            ui,
            &base,
            |ui, ctx| {
                let rect = ctx.rect;
                let radius = base.radius(&t);
                let (ptr, k) = tracked_pointer(ui, ctx, self.ambient, 2.1);
                let target = (ptr - rect.center()) / 18.0 * self.strength * k;
                let off = glowkit_core::anim::spring2(ui, ctx.id.with("wob"), target, 160.0, 9.0);
                let body = rect.translate(off);
                let [a, b] = self.colors.unwrap_or([shade(t.accent, -0.05), shade(t.accent, -0.22)]);
                let p = ui.painter();
                paint::soft_shadow(p, body, radius, 28.0, vec2(0.0, 10.0) + off * 0.3, with_alpha(b, if t.dark { 0.45 } else { 0.35 }));
                paint::gradient_rect(p, body, radius, 35.0, &[a, b]);
                // grain
                let mut rng = Rng::new(ctx.id.value());
                let clip = p.with_clip_rect(body.shrink(radius * 0.3));
                for _ in 0..((body.width() * body.height()) / 70.0) as usize {
                    let q = pos2(rng.range(body.min.x, body.max.x), rng.range(body.min.y, body.max.y));
                    let light = rng.bool(0.5);
                    let c = if light { with_alpha(Color32::WHITE, rng.range(0.03, 0.09)) } else { with_alpha(Color32::BLACK, rng.range(0.03, 0.08)) };
                    clip.rect_filled(Rect::from_center_size(q, vec2(1.0, 1.0)), 0.0, c);
                }
                // top sheen
                paint::fill_convex_fn(p, &paint::rounded_rect_points(body, radius, 10), 5, |q| {
                    let v = (q.y - body.min.y) / body.height();
                    with_alpha(Color32::WHITE, (0.4 - v).max(0.0) * 0.18)
                });
                p.rect_stroke(body, cr(radius), egui::Stroke::new(1.0, with_alpha(Color32::WHITE, 0.12)), egui::StrokeKind::Inside);
                Content { offset: off - off * 1.6, opacity: 1.0 }
            },
            |ui| {
                ui.style_mut().visuals.override_text_color = Some(Color32::WHITE);
                add(ui)
            },
            |_, _| {},
        )
    }
}

// ---------------------------------------------------------------------------------------------

/// A frosted-glass card: translucent fill, a bright edge catching light from the top-left and a
/// soft shadow. Looks best over something colorful.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::GlassCard;
/// GlassCard::new().width(320.0).show(ui, |ui| { ui.label("Frosted"); });
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct GlassCard {
    base: Base,
    tint: Option<Color32>,
    opacity: f32,
    frost: bool,
}

impl Default for GlassCard {
    fn default() -> Self {
        Self::new()
    }
}

impl GlassCard {
    /// A glass card.
    pub fn new() -> Self {
        Self { base: Base::default(), tint: None, opacity: 1.0, frost: true }
    }
    base_setters!();
    /// Tint color mixed into the glass. Defaults to white.
    pub fn tint(mut self, color: Color32) -> Self {
        self.tint = Some(color);
        self
    }
    setter!(/// Opacity multiplier of the glass fill (1 = default).
        opacity: f32);
    setter!(/// Add a faint frosted grain.
        frost: bool);

    /// Show the card with `add` inside.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let base = self.base.clone();
        let t = Theme::of(ui);
        show_shell(
            ui,
            &base,
            |ui, ctx| {
                let rect = ctx.rect;
                let radius = base.radius(&t);
                let tint = self.tint.unwrap_or(Color32::WHITE);
                let p = ui.painter();
                paint::soft_shadow(p, rect, radius, 32.0, vec2(0.0, 12.0), with_alpha(Color32::BLACK, if t.dark { 0.35 } else { 0.10 }));
                let (a0, a1) = if t.dark { (0.20, 0.07) } else { (0.62, 0.38) };
                let op = self.opacity;
                paint::fill_convex_fn(p, &paint::rounded_rect_points(rect, radius, 10), 6, |q| {
                    let u = (((q.x - rect.min.x) / rect.width() + (q.y - rect.min.y) / rect.height()) / 2.0).clamp(0.0, 1.0);
                    with_alpha(tint, (a0 + (a1 - a0) * u) * op)
                });
                if self.frost {
                    let mut rng = Rng::new(ctx.id.value() ^ 77);
                    let clip = p.with_clip_rect(rect.shrink(radius * 0.3));
                    for _ in 0..((rect.width() * rect.height()) / 90.0) as usize {
                        let q = pos2(rng.range(rect.min.x, rect.max.x), rng.range(rect.min.y, rect.max.y));
                        clip.rect_filled(Rect::from_center_size(q, vec2(1.0, 1.0)), 0.0, with_alpha(Color32::WHITE, rng.range(0.02, 0.07)));
                    }
                }
                // light-catching rim
                util::border_fn(p, rect, radius, 1.0, |q, _| {
                    let u = (((q.x - rect.min.x) / rect.width() + (q.y - rect.min.y) / rect.height()) / 2.0).clamp(0.0, 1.0);
                    if t.dark {
                        with_alpha(Color32::WHITE, 0.45 * (1.0 - u) + 0.06)
                    } else {
                        with_alpha(Color32::WHITE, 0.95 * (1.0 - u)).lerp_to_gamma(with_alpha(Color32::BLACK, 0.10), u)
                    }
                });
                // top inner highlight streak
                let hl = Rect::from_min_max(rect.min + vec2(radius, 1.0), pos2(rect.max.x - radius, rect.min.y + 2.0));
                paint::gradient_polyline(p, &[hl.left_top(), hl.center_top(), hl.right_top()], false, 1.0, |s| {
                    with_alpha(Color32::WHITE, (1.0 - (s * 2.0 - 1.0).abs()) * if t.dark { 0.5 } else { 0.9 })
                });
                Content::default()
            },
            add,
            |_, _| {},
        )
    }
}

// ---------------------------------------------------------------------------------------------

/// A card that lifts toward you on hover: it rises, its shadow deepens and its border warms to
/// the accent.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::HoverLiftCard;
/// HoverLiftCard::new().width(260.0).show(ui, |ui| { ui.label("Lift"); });
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct HoverLiftCard {
    base: Base,
    lift: f32,
    accent: Option<Color32>,
    force_hover: bool,
}

impl Default for HoverLiftCard {
    fn default() -> Self {
        Self::new()
    }
}

impl HoverLiftCard {
    /// A hover-lift card.
    pub fn new() -> Self {
        Self { base: Base::default(), lift: 8.0, accent: None, force_hover: false }
    }
    base_setters!();
    setter!(/// How far the card rises, in points.
        lift: f32);
    /// Border / glow color while lifted. Defaults to the accent.
    pub fn accent(mut self, color: Color32) -> Self {
        self.accent = Some(color);
        self
    }
    setter!(/// Render as if hovered (for showcases, or to mirror an external hover state).
        force_hover: bool);

    /// Show the card with `add` inside.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let base = self.base.clone();
        let t = Theme::of(ui);
        show_shell(
            ui,
            &base,
            |ui, ctx| {
                let rect = ctx.rect;
                let radius = base.radius(&t);
                let on = ctx.pointer.is_some() || self.force_hover;
                let k = glowkit_core::anim::spring(ui, ctx.id.with("lift"), if on { 1.0 } else { 0.0 }, 220.0, 20.0);
                let body = rect.translate(vec2(0.0, -self.lift * k));
                let accent = self.accent.unwrap_or(t.accent);
                let p = ui.painter();
                paint::soft_shadow(p, body, radius, 16.0 + 26.0 * k, vec2(0.0, 4.0 + 14.0 * k), with_alpha(Color32::BLACK, if t.dark { 0.35 + 0.2 * k } else { 0.06 + 0.08 * k }));
                if k > 0.01 {
                    paint::soft_shadow(p, body, radius, 30.0, vec2(0.0, 8.0), with_alpha(accent, 0.18 * k.clamp(0.0, 1.0)));
                }
                p.rect_filled(body, cr(radius), base.background(&t));
                p.rect_stroke(body, cr(radius), egui::Stroke::new(1.0, t.border.lerp_to_gamma(accent, 0.6 * k.clamp(0.0, 1.0))), egui::StrokeKind::Inside);
                Content { offset: vec2(0.0, -self.lift * k), opacity: 1.0 }
            },
            add,
            |_, _| {},
        )
    }
}
