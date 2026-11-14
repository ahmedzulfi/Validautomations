//! Navigation and buttons made of liquid glass: glass tabs, a liquid glass button and a dark
//! morphing navbar that springs open into its own dropdown.

use super::common::{cover_uv, display_font, idle_time, image_rounded, lerp_c, semibold_font, vgrad_rect, wall_dt};
use egui::epaint::{Mesh, Vertex};
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Painter, Pos2, Rect, Response, Sense,
    Shape, Stroke, StrokeKind, TextureId, Ui, Vec2, Widget,
};
use glowkit_core::color::{hex, hsv, with_alpha};
use glowkit_core::{paint, Spring, Theme};
use glowkit_effects::{MicroIcon, PaintedScene};
use std::f32::consts::TAU;

fn cr(r: f32) -> CornerRadius {
    CornerRadius::same(r.round().clamp(0.0, 255.0) as u8)
}

/// Signed distance from `p` to the boundary of a pill (stadium) `rect`, positive inside, plus the
/// outward normal at the nearest boundary point.
fn pill_sd(rect: Rect, p: Pos2) -> (f32, Vec2) {
    let r = rect.height().min(rect.width()) / 2.0;
    let inner = rect.shrink2(vec2(r, r));
    let q = pos2(p.x.clamp(inner.min.x, inner.max.x), p.y.clamp(inner.min.y, inner.max.y));
    let d = p - q;
    let l = d.length();
    let n = if l > 1e-4 { d / l } else { vec2(0.0, if p.y < rect.center().y { -1.0 } else { 1.0 }) };
    (r - l, n)
}

/// Paint glass optics over a pill: a top sheen, a bevel (bright top rim, dark lower rim), an
/// inner shade and a specular highlight aimed at `light` (local direction).
fn glass_body(painter: &Painter, rect: Rect, tint: Color32, light: Vec2, strength: f32, dark_bg: bool) {
    let r = rect.height() / 2.0;
    painter.rect_filled(rect, cr(r), tint);
    // top sheen
    let sheen = Rect::from_min_max(rect.min + vec2(r * 0.45, 2.5), pos2(rect.max.x - r * 0.45, rect.center().y));
    vgrad_rect(painter, sheen, sheen.height() / 2.0, with_alpha(Color32::WHITE, 0.20 * strength.min(1.0)), Color32::TRANSPARENT);
    // bevel rims: draw rings with angle-dependent brightness
    let ring = paint::rounded_rect_points(rect.shrink(1.0), r - 1.0, 10);
    let c = rect.center();
    let lit = |p: Pos2, k: f32| {
        let n = (p - c).normalized();
        let d = n.dot(light.normalized());
        let a = (d.max(0.0).powf(3.0) * 0.95 + 0.18) * k;
        with_alpha(Color32::WHITE, a.min(1.0))
    };
    let pts: Vec<Pos2> = ring.clone();
    let n = pts.len();
    paint::gradient_polyline(painter, &pts, true, 1.6, |s| lit(pts[((s * n as f32) as usize).min(n - 1)], strength));
    let ring2 = paint::rounded_rect_points(rect.shrink(4.0), r - 4.0, 10);
    let n2 = ring2.len();
    paint::gradient_polyline(painter, &ring2, true, 3.0, |s| {
        let p = ring2[((s * n2 as f32) as usize).min(n2 - 1)];
        let nn = (p - c).normalized();
        let d = nn.dot(-light.normalized());
        with_alpha(if dark_bg { Color32::BLACK } else { hex("#5A5A68") }, (d.max(0.0) * 0.22 * strength).min(1.0))
    });
    // specular blob near the lit rim
    let sp = c + vec2(light.x * (rect.width() / 2.0 - r * 0.55), light.y * r * 0.55);
    paint::glow_ellipse(painter, sp, vec2(r * 0.9, r * 0.32), with_alpha(Color32::WHITE, 0.30 * strength));
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct TabsState {
    x: Spring,
    squeeze: Spring,
    timer: f32,
    dragging: bool,
}

/// A tab switch whose selection is a thick lens of liquid glass: it magnifies the track under
/// its rim, darkens the label it covers, throws rainbow glints that orbit its edge, and
/// squeezes in flight, lands and wobbles between tabs. Click a tab, drag the lens, or use the
/// arrow keys. Autoplay cycles the tabs while idle.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut tab = 0usize;
/// ui.add(glowkit_interactions::GlassTabs::new(&mut tab, ["Work", "Studio", "Journal", "Contact"]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct GlassTabs<'a> {
    selected: &'a mut usize,
    tabs: Vec<String>,
    width: Option<f32>,
    height: f32,
    wobble: f32,
    autoplay: f32,
}

impl<'a> GlassTabs<'a> {
    /// Tabs with `selected` holding the active index.
    pub fn new(selected: &'a mut usize, tabs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            selected,
            tabs: tabs.into_iter().map(Into::into).collect(),
            width: None,
            height: 64.0,
            wobble: 1.0,
            autoplay: 0.0,
        }
    }
    /// Total width (default: 150 points per tab, capped by the available width).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w.max(120.0));
        self
    }
    /// Track height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.clamp(32.0, 120.0);
        self
    }
    /// How much the lens squeezes and wobbles, `0..=2`.
    pub fn wobble(mut self, w: f32) -> Self {
        self.wobble = w.clamp(0.0, 2.0);
        self
    }
    /// Step to the next tab every `secs` seconds while idle (0 = off).
    pub fn autoplay(mut self, secs: f32) -> Self {
        self.autoplay = secs.max(0.0);
        self
    }
}

impl Widget for GlassTabs<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.tabs.len().max(1);
        let w = self.width.unwrap_or(n as f32 * 150.0).min(ui.available_width() - 4.0);
        let h = self.height;
        let (rect, response) = ui.allocate_exact_size(vec2(w, h + 24.0), Sense::click_and_drag());
        let id = response.id;
        let track = Rect::from_center_size(rect.center(), vec2(w, h));
        let pad = 8.0;
        let tab_w = (track.width() - pad * 2.0) / n as f32;
        let tab_center = |i: usize| track.min.x + pad + tab_w * (i as f32 + 0.5);
        let dt = glowkit_core::dt(ui);
        let wdt = wall_dt(ui);
        let pointer = response.interact_pointer_pos().or(response.hover_pos());
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        let mut sel = (*self.selected).min(n - 1);
        if response.clicked() {
            if let Some(p) = pointer {
                sel = (((p.x - track.min.x - pad) / tab_w).floor().max(0.0) as usize).min(n - 1);
            }
        }
        if response.hovered() {
            let k = ui.input(|i| {
                i.key_pressed(egui::Key::ArrowRight) as i32 - i.key_pressed(egui::Key::ArrowLeft) as i32
            });
            sel = (sel as i32 + k).clamp(0, n as i32 - 1) as usize;
        }
        let dragging = response.dragged();
        let drag_x = pointer.map(|p| p.x).unwrap_or(0.0);
        let released = response.drag_stopped();
        let idle = idle_time(ui, id, response.hovered() || dragging);
        let auto = self.autoplay;
        let wob = self.wobble;
        let target_x = tab_center(sel);
        let (x, sq, step) = glowkit_core::with_state(
            ui,
            id.with("glass_tabs"),
            || TabsState {
                x: Spring::new(target_x).with(220.0, 16.0),
                squeeze: Spring::new(0.0).with(320.0, 9.0),
                timer: 0.0,
                dragging: false,
            },
            |s| {
                let mut step = 0;
                s.timer += wdt;
                if auto > 0.0 && idle > 1.5 && s.timer > auto {
                    s.timer = 0.0;
                    step = 1;
                }
                if dragging {
                    s.dragging = true;
                    let before = s.x.value;
                    s.x.value = drag_x.clamp(tab_center(0), tab_center(n - 1));
                    s.x.velocity = (s.x.value - before) / dt.max(1e-3);
                    s.x.target = s.x.value;
                } else {
                    s.x.target = target_x;
                    s.x.stiffness = 220.0;
                    s.x.damping = 16.0 + (2.0 - wob) * 6.0;
                    s.x.step(dt);
                }
                // squeeze follows speed, then rings when the lens lands
                s.squeeze.target = (s.x.velocity.abs() / 1400.0).min(1.0);
                s.squeeze.step(dt);
                (s.x.value, s.squeeze.value, step)
            },
        );
        if released {
            sel = (((x - track.min.x - pad) / tab_w).floor().max(0.0) as usize).min(n - 1);
        }
        if step != 0 {
            sel = (sel + 1) % n;
        }
        *self.selected = sel;
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(24.0));
        let time = glowkit_core::time(ui);
        // track: a recessed pill with inner shadow
        let (track_c, track_hi, label_c, label_on) = if t.dark {
            (hex("#1B1B20"), with_alpha(Color32::WHITE, 0.06), hex("#6E6E78"), hex("#F4F4F7"))
        } else {
            (hex("#DEDEE1"), with_alpha(Color32::WHITE, 0.9), hex("#9C9CA3"), hex("#26262B"))
        };
        painter.rect_filled(track, cr(h / 2.0), track_c);
        paint::gradient_rect(
            &painter,
            Rect::from_min_max(track.min, pos2(track.max.x, track.min.y + h * 0.45)),
            h / 2.0,
            90.0,
            &[with_alpha(Color32::BLACK, if t.dark { 0.35 } else { 0.10 }), with_alpha(Color32::BLACK, 0.0)],
        );
        painter.rect_stroke(track, cr(h / 2.0), Stroke::new(1.0, track_hi), StrokeKind::Inside);
        let font = semibold_font(ui, h * 0.36);
        for (i, label) in self.tabs.iter().enumerate() {
            painter.text(pos2(tab_center(i), track.center().y), Align2::CENTER_CENTER, label, font.clone(), label_c);
        }
        // the lens
        let sqz = sq * wob;
        let lw = tab_w * (0.98 + 0.22 * sqz);
        let lh = (h + 12.0) * (1.0 - 0.14 * sqz);
        let lens = Rect::from_center_size(pos2(x, track.center().y), vec2(lw, lh));
        let sh = if t.dark { Color32::from_black_alpha(140) } else { Color32::from_rgba_unmultiplied(40, 40, 60, 46) };
        paint::soft_shadow(&painter, lens.translate(vec2(0.0, 6.0)), lh / 2.0, 14.0, Vec2::ZERO, sh);
        // magnified track under the lens
        let inner = painter.with_clip_rect(lens.shrink(3.0));
        inner.rect_filled(lens, cr(lh / 2.0), lerp_c(track_c, if t.dark { hex("#2A2A31") } else { hex("#ECECEE") }, 0.7));
        let mag = 1.1;
        let big = FontId::new(font.size * mag, font.family.clone());
        for (i, label) in self.tabs.iter().enumerate() {
            let cx = x + (tab_center(i) - x) * mag;
            let on = (1.0 - ((tab_center(i) - x).abs() / tab_w)).clamp(0.0, 1.0);
            inner.text(
                pos2(cx, track.center().y),
                Align2::CENTER_CENTER,
                label,
                big.clone(),
                lerp_c(label_c, label_on, on),
            );
        }
        let tint = if t.dark { with_alpha(Color32::WHITE, 0.04) } else { with_alpha(Color32::WHITE, 0.18) };
        glass_body(&painter, lens, tint, vec2(-0.45, -0.9), 1.0, t.dark);
        // orbiting rainbow glints
        let ring = paint::rounded_rect_points(lens.shrink(1.5), lh / 2.0 - 1.5, 12);
        let head = (time * 0.18).fract();
        paint::gradient_polyline(&painter, &ring, true, 2.0, |s| {
            let d1 = ((s - head + 0.5).rem_euclid(1.0) - 0.5).abs();
            let d2 = ((s - head).rem_euclid(1.0) - 0.5).abs();
            let k = (1.0 - d1 / 0.11).max(0.0).powi(2).max((1.0 - d2 / 0.07).max(0.0).powi(2) * 0.7);
            with_alpha(hsv(s * 900.0 + time * 60.0, 0.55, 1.0), k * 0.85)
        });
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// The look of a [`LiquidGlassButton`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GlassVariant {
    /// Clear smoked glass.
    Clear,
    /// Milky frosted glass with a stronger blur.
    Frosted,
    /// Glass tinted with a color.
    Tinted(Color32),
}

/// Leading / trailing glyph of a [`LiquidGlassButton`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlassGlyph {
    /// No icon.
    None,
    /// A chevron after the label.
    Chevron,
    /// An arrow after the label.
    Arrow,
    /// A plus before the label.
    Plus,
}

/// A pill of liquid glass: it blurs what is behind it and bends it at the bevel like a thick
/// lens, its rim highlights chase the pointer, and it tilts and presses in with a spring.
///
/// Behind it the button paints a refracted copy of its backdrop: either a screen-aligned dot
/// grid (paint the same grid around it with [`LiquidGlassButton::paint_dot_backdrop`]) or an
/// image you place with [`LiquidGlassButton::over_image`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{GlassGlyph, GlassVariant, LiquidGlassButton};
/// if ui.add(LiquidGlassButton::new("Get started").icon(GlassGlyph::Chevron)).clicked() {}
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct LiquidGlassButton {
    label: String,
    icon: GlassGlyph,
    variant: GlassVariant,
    disabled: bool,
    height: f32,
    refraction: f32,
    image: Option<(TextureId, Rect, Rect)>,
}

/// Pitch of the default dot backdrop.
const DOT_PITCH: f32 = 22.0;

impl LiquidGlassButton {
    /// A clear glass button.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: GlassGlyph::None,
            variant: GlassVariant::Clear,
            disabled: false,
            height: 52.0,
            refraction: 1.0,
            image: None,
        }
    }
    /// Icon next to the label.
    pub fn icon(mut self, icon: GlassGlyph) -> Self {
        self.icon = icon;
        self
    }
    /// Glass variant.
    pub fn variant(mut self, v: GlassVariant) -> Self {
        self.variant = v;
        self
    }
    /// Disabled buttons are dimmed and ignore the pointer.
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }
    /// Button height (the width follows the label).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.clamp(28.0, 120.0);
        self
    }
    /// Lens strength at the bevel, `0..=2`.
    pub fn refraction(mut self, k: f32) -> Self {
        self.refraction = k.clamp(0.0, 2.0);
        self
    }
    /// Refract an image instead of the dot grid: `tex` (cropped by `uv`) is drawn at `screen`
    /// behind the button.
    pub fn over_image(mut self, tex: TextureId, screen: Rect, uv: Rect) -> Self {
        self.image = Some((tex, screen, uv));
        self
    }

    /// Paint the screen-aligned dot grid the default backdrop refracts.
    pub fn paint_dot_backdrop(ui: &Ui, rect: Rect) {
        let t = Theme::of(ui);
        let painter = ui.painter_at(rect);
        let c = if t.dark { with_alpha(Color32::WHITE, 0.22) } else { with_alpha(Color32::BLACK, 0.22) };
        let x0 = (rect.min.x / DOT_PITCH).ceil() as i32;
        let x1 = (rect.max.x / DOT_PITCH).floor() as i32;
        let y0 = (rect.min.y / DOT_PITCH).ceil() as i32;
        let y1 = (rect.max.y / DOT_PITCH).floor() as i32;
        let mut mesh = Mesh::default();
        for y in y0..=y1 {
            for x in x0..=x1 {
                super::common::mesh_circle(&mut mesh, pos2(x as f32 * DOT_PITCH, y as f32 * DOT_PITCH), 1.2, 6, c);
            }
        }
        painter.add(Shape::mesh(mesh));
    }
}

impl Widget for LiquidGlassButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let h = self.height;
        let font = semibold_font(ui, h * 0.38);
        let galley = ui.painter().layout_no_wrap(self.label.clone(), font.clone(), Color32::WHITE);
        let icon_w = if self.icon == GlassGlyph::None { 0.0 } else { h * 0.42 };
        let w = galley.size().x + icon_w + h * 1.1;
        let sense = if self.disabled { Sense::hover() } else { Sense::click() };
        let (rect, response) = ui.allocate_exact_size(vec2(w, h) + vec2(16.0, 16.0), sense);
        let id = response.id;
        let body = Rect::from_center_size(rect.center(), vec2(w, h));
        let hover = if self.disabled { None } else { response.hover_pos() };
        if hover.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        let time = glowkit_core::time(ui);
        let idle = idle_time(ui, id, hover.is_some());
        let pressed = response.is_pointer_button_down_on() && !self.disabled;
        let press = glowkit_core::anim::spring(ui, id.with("press"), if pressed { 1.0 } else { 0.0 }, 420.0, 22.0);
        let hov = glowkit_core::anim::spring(ui, id.with("hov"), if hover.is_some() { 1.0 } else { 0.0 }, 220.0, 20.0);
        // light direction: toward the pointer, or slowly orbiting when idle
        let ldir = match hover {
            Some(p) => {
                let d = p - body.center();
                vec2(d.x / (w / 2.0), d.y / (h / 2.0)).clamp(vec2(-1.0, -1.0), vec2(1.0, 1.0))
            }
            None if !self.disabled && idle > 0.5 => {
                let a = time * 0.9 + id.value() as f32 * 1e-3;
                vec2(a.cos(), a.sin() * 0.6 - 0.5)
            }
            None => vec2(-0.4, -0.9),
        };
        let lx = glowkit_core::anim::follow(ui, id.with("lx"), ldir.x, 8.0);
        let ly = glowkit_core::anim::follow(ui, id.with("ly"), ldir.y, 8.0);
        let light = vec2(lx, ly);
        if !self.disabled {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(16.0));
        let scale = 1.0 - 0.04 * press + 0.015 * hov;
        let body = Rect::from_center_size(body.center() + vec2(0.0, 1.5 * press), body.size() * scale);
        let r = body.height() / 2.0;
        let alpha = if self.disabled { 0.45 } else { 1.0 };
        // shadow
        let sh = if t.dark { Color32::from_black_alpha(170) } else { Color32::from_rgba_unmultiplied(30, 30, 50, 60) };
        paint::soft_shadow(&painter, body.translate(vec2(0.0, 5.0 - 2.0 * press)), r, 14.0, Vec2::ZERO, sh.gamma_multiply(alpha));
        // refracted backdrop
        let bevel = r * 0.85;
        let strength = self.refraction;
        let frosted = matches!(self.variant, GlassVariant::Frosted);
        let refract = |p: Pos2| -> Pos2 {
            let (d, n) = pill_sd(body, p);
            let k = (1.0 - (d / bevel).clamp(0.0, 1.0)).powi(2);
            let mag = body.center() + (p - body.center()) * 0.92;
            mag + n * k * bevel * 0.9 * strength
        };
        match self.image {
            Some((tex, screen, uv)) => {
                let blur_taps: &[(f32, f32)] =
                    if frosted { &[(0.0, 0.0), (3.0, 0.0), (-3.0, 0.0), (0.0, 3.0), (0.0, -3.0)] } else { &[(0.0, 0.0), (1.5, 1.5), (-1.5, -1.5)] };
                for (k, (ox, oy)) in blur_taps.iter().enumerate() {
                    let a = if k == 0 { 1.0 } else { 0.5 };
                    lens_image(&painter, body, tex, screen, uv, vec2(*ox, *oy), &refract, with_alpha(Color32::WHITE, a * alpha));
                }
            }
            None => {
                let dot = if t.dark { with_alpha(Color32::WHITE, 0.5) } else { with_alpha(Color32::BLACK, 0.45) };
                let x0 = (body.min.x / DOT_PITCH).floor() as i32 - 2;
                let x1 = (body.max.x / DOT_PITCH).ceil() as i32 + 2;
                let y0 = (body.min.y / DOT_PITCH).floor() as i32 - 2;
                let y1 = (body.max.y / DOT_PITCH).ceil() as i32 + 2;
                let clip = painter.with_clip_rect(body);
                let mut mesh = Mesh::default();
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        let src = pos2(x as f32 * DOT_PITCH, y as f32 * DOT_PITCH);
                        // approximate inverse of `refract` with a few fixed-point iterations
                        let mut p = src;
                        for _ in 0..6 {
                            let q = refract(p);
                            p += (src - q) * 0.8;
                        }
                        if pill_sd(body, p).0 < -1.0 {
                            continue;
                        }
                        let (d, _) = pill_sd(body, p);
                        let edge = 1.0 - (d / bevel).clamp(0.0, 1.0);
                        let rr = if frosted { 2.2 } else { 1.1 + edge * 1.3 };
                        let c = dot.gamma_multiply(if frosted { 0.25 } else { 0.35 + 0.4 * edge } * alpha);
                        super::common::mesh_circle(&mut mesh, p, rr, 8, c);
                    }
                }
                clip.add(Shape::mesh(mesh));
            }
        }
        let tint = match self.variant {
            GlassVariant::Clear => {
                if t.dark {
                    with_alpha(hex("#1A1A20"), 0.55)
                } else {
                    with_alpha(hex("#2A2A33"), 0.62)
                }
            }
            GlassVariant::Frosted => with_alpha(hex("#C9C9D2"), if t.dark { 0.55 } else { 0.62 }),
            GlassVariant::Tinted(c) => with_alpha(c, 0.72),
        };
        glass_body(&painter, body, tint.gamma_multiply(alpha), light, alpha * (0.85 + 0.35 * hov), true);
        // label
        let text_c = with_alpha(Color32::WHITE, 0.94 * alpha);
        let total = galley.size().x + icon_w;
        let mut x = body.center().x - total / 2.0;
        if self.icon == GlassGlyph::Plus {
            MicroIcon::Plus.paint(&painter, pos2(x + icon_w * 0.4, body.center().y), h * 0.34, text_c);
            x += icon_w;
        }
        let gp = pos2(x, body.center().y - galley.size().y / 2.0);
        painter.galley(gp + vec2(0.0, 1.0), galley.clone(), with_alpha(Color32::BLACK, 0.35 * alpha));
        painter.galley(gp, galley.clone(), text_c);
        let ix = gp.x + galley.size().x + icon_w * 0.62 + 2.0 * hov;
        match self.icon {
            GlassGlyph::Chevron => {
                let c = pos2(ix, body.center().y);
                let s = h * 0.1;
                painter.line(vec![c + vec2(-s * 0.5, -s), c + vec2(s * 0.5, 0.0), c + vec2(-s * 0.5, s)], Stroke::new(2.0, text_c));
            }
            GlassGlyph::Arrow => MicroIcon::ArrowRight.paint(&painter, pos2(ix, body.center().y), h * 0.34, text_c),
            _ => {}
        }
        response
    }
}

#[allow(clippy::too_many_arguments)]
fn lens_image(
    painter: &Painter,
    body: Rect,
    tex: TextureId,
    screen: Rect,
    uv: Rect,
    offset: Vec2,
    refract: &impl Fn(Pos2) -> Pos2,
    tint: Color32,
) {
    let (nx, ny) = (28usize, 10usize);
    let mut m = Mesh::with_texture(tex);
    for j in 0..=ny {
        for i in 0..=nx {
            let mut p = pos2(
                body.min.x + i as f32 / nx as f32 * body.width(),
                body.min.y + j as f32 / ny as f32 * body.height(),
            );
            // pull the grid onto the pill outline
            let (d, n) = pill_sd(body, p);
            if d < 0.0 {
                p += n * d;
            }
            let src = refract(p) + offset;
            let u = uv.min.x + (src.x - screen.min.x) / screen.width() * uv.width();
            let v = uv.min.y + (src.y - screen.min.y) / screen.height() * uv.height();
            m.vertices.push(Vertex { pos: p, uv: pos2(u, v), color: tint });
        }
    }
    let w = (nx + 1) as u32;
    for j in 0..ny as u32 {
        for i in 0..nx as u32 {
            let a = j * w + i;
            m.add_triangle(a, a + 1, a + w + 1);
            m.add_triangle(a, a + w + 1, a + w);
        }
    }
    painter.add(Shape::mesh(m));
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct NavState {
    open: Option<usize>,
    shown: usize,
    prev: usize,
    switch_t: f32,
    w: Spring,
    h: Spring,
    hl: Spring,
    ghost_t: f32,
    hover_close: f32,
}

/// A dark pill navigation bar that springs open into its own dropdown: one shell resizes
/// between three panels while their content cross-blurs and drifts. The panels hold a picture
/// reel, an accordion of cards and a link list with a gliding highlight. Hover (or click) an
/// item to open it; a ghost visitor tours the menu while the pointer is away.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::MorphNavbar::new().brand("Fieldhouse"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct MorphNavbar {
    brand: String,
    items: [String; 3],
    cta: String,
    autoplay: bool,
}

impl Default for MorphNavbar {
    fn default() -> Self {
        Self::new()
    }
}

impl MorphNavbar {
    /// A navbar with placeholder content.
    pub fn new() -> Self {
        Self {
            brand: "Fieldhouse".into(),
            items: ["Studio".into(), "Projects".into(), "Journal".into()],
            cta: "Contact".into(),
            autoplay: true,
        }
    }
    /// Brand name on the left.
    pub fn brand(mut self, s: impl Into<String>) -> Self {
        self.brand = s.into();
        self
    }
    /// Labels of the three menu items (reel, accordion, list).
    pub fn items(mut self, a: impl Into<String>, b: impl Into<String>, c: impl Into<String>) -> Self {
        self.items = [a.into(), b.into(), c.into()];
        self
    }
    /// Label of the light call-to-action pill.
    pub fn cta(mut self, s: impl Into<String>) -> Self {
        self.cta = s.into();
        self
    }
    /// Tour the menu with a ghost visitor while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const REEL: [(&str, &str); 3] = [("Method", "How we read the ground"), ("Nursery", "Grown on our own land"), ("Stone", "Quarried close to home")];
const ACCORDION: [(&str, &str); 4] = [
    ("Hillside house", "Terraced garden, 2024"),
    ("Courtyard", "A cloister of grasses"),
    ("Rooftop meadow", "Six storeys up"),
    ("Riverside walk", "Public realm, 1.2 km"),
];
const LINKS: [(&str, &str); 4] = [
    ("Notes from the nursery", "6 min read"),
    ("Why we plant in autumn", "4 min read"),
    ("A field guide to gravel", "9 min read"),
    ("Drawing with water", "5 min read"),
];

impl Widget for MorphNavbar {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w_avail = ui.available_width().min(760.0);
        let (rect, response) = ui.allocate_exact_size(vec2(w_avail, 360.0), Sense::hover());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let wdt = wall_dt(ui);
        let time = glowkit_core::time(ui);
        let bar_h = 52.0;
        let closed_w = 470.0f32.min(w_avail - 8.0);
        let panel_sizes = [vec2(640.0f32.min(w_avail - 8.0), 282.0), vec2(560.0f32.min(w_avail - 8.0), 320.0), vec2(520.0f32.min(w_avail - 8.0), 268.0)];
        let top = rect.min.y + 10.0;
        let pointer = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p));
        let idle = idle_time(ui, id, pointer.is_some());
        // item hit rects in the bar (depend on the current shell width; use last frame's)
        let prev: NavState = glowkit_core::state(ui, id.with("nav"), NavState::default);
        let shell_w = if prev.w.target == 0.0 { closed_w } else { prev.w.value };
        let shell_h = if prev.h.target == 0.0 { bar_h } else { prev.h.value };
        let shell = Rect::from_min_size(pos2(rect.center().x - shell_w / 2.0, top), vec2(shell_w, shell_h));
        let bar = Rect::from_min_size(shell.min, vec2(shell_w, bar_h));
        let item_font = semibold_font(ui, 14.5);
        let brand_font = display_font(ui, 17.0);
        let item_w = 84.0;
        let items_x0 = bar.center().x - item_w * 1.5 + 10.0;
        let item_rect = |i: usize| Rect::from_min_size(pos2(items_x0 + i as f32 * item_w, bar.min.y + 8.0), vec2(item_w, bar_h - 16.0));
        // ghost tour: open 0 -> 1 -> 2 -> closed, each for a few seconds
        let ghost_on = self.autoplay && pointer.is_none() && idle > 1.0;
        let mut hover_item = None;
        if let Some(p) = pointer {
            for i in 0..3 {
                if item_rect(i).expand(4.0).contains(p) {
                    hover_item = Some(i);
                }
            }
        }
        let over_shell = pointer.is_some_and(|p| shell.expand(10.0).contains(p));
        let ghost_phase = (prev.ghost_t / 3.2) as usize % 4;
        let ghost_open = if ghost_on && ghost_phase < 3 { Some(ghost_phase) } else { None };
        // ghost pointer inside the open panel (for highlights)
        let ghost_y = (prev.ghost_t % 3.2) / 3.2;
        let (open, hl_index) = {
            let mut s = prev;
            if ghost_on {
                s.ghost_t += wdt;
                s.open = ghost_open;
            } else {
                s.ghost_t = 0.0;
                if let Some(i) = hover_item {
                    s.open = Some(i);
                    s.hover_close = 0.0;
                } else if !over_shell {
                    s.hover_close += wdt;
                    if s.hover_close > 0.35 {
                        s.open = None;
                    }
                } else {
                    s.hover_close = 0.0;
                }
            }
            if let Some(o) = s.open {
                if o != s.shown {
                    s.prev = s.shown;
                    s.shown = o;
                    s.switch_t = 0.0;
                }
            }
            s.switch_t += wdt;
            let target = match s.open {
                Some(o) => vec2(panel_sizes[o].x, bar_h + panel_sizes[o].y),
                None => vec2(closed_w, bar_h),
            };
            if s.w.target == 0.0 {
                s.w = Spring::new(closed_w).with(260.0, 22.0);
                s.h = Spring::new(bar_h).with(260.0, 22.0);
                s.hl = Spring::new(0.0).with(300.0, 26.0);
            }
            s.w.target = target.x;
            s.h.target = target.y;
            s.w.step(dt);
            s.h.step(dt);
            // highlight row target
            let rows = match s.shown {
                1 => 4.0,
                2 => 4.0,
                _ => 3.0,
            };
            let hl_t = if ghost_on {
                (ghost_y * rows).floor().min(rows - 1.0)
            } else if let Some(p) = pointer {
                let content_top = shell.min.y + bar_h + 12.0;
                let ph = (shell.height() - bar_h - 24.0).max(1.0);
                if s.shown == 0 {
                    ((p.x - shell.min.x - 14.0) / ((shell.width() - 28.0) / 3.0)).floor().clamp(0.0, 2.0)
                } else {
                    ((p.y - content_top) / (ph / rows)).floor().clamp(0.0, rows - 1.0)
                }
            } else {
                s.hl.target
            };
            s.hl.target = hl_t;
            s.hl.step(dt);
            glowkit_core::set_state(ui, id.with("nav"), s);
            (s, s.hl.value)
        };
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let st = open;
        let painter = ui.painter_at(rect.expand(8.0));
        // shell
        let shell = Rect::from_min_size(pos2(rect.center().x - st.w.value / 2.0, top), vec2(st.w.value, st.h.value.max(bar_h)));
        let shell_r = (bar_h / 2.0).min(10.0 + (shell.height() - bar_h).max(0.0)).max(18.0).min(bar_h / 2.0);
        let sh = if t.dark { Color32::from_black_alpha(160) } else { Color32::from_rgba_unmultiplied(20, 20, 30, 70) };
        paint::soft_shadow(&painter, shell.translate(vec2(0.0, 10.0)), shell_r, 24.0, Vec2::ZERO, sh);
        let ink = hex("#151517");
        painter.rect_filled(shell, cr(shell_r), ink);
        painter.rect_stroke(shell, cr(shell_r), Stroke::new(1.0, with_alpha(Color32::WHITE, 0.07)), StrokeKind::Inside);
        // bar content
        let bar = Rect::from_min_size(shell.min, vec2(shell.width(), bar_h));
        // brand mark
        let mark_c = pos2(bar.min.x + 28.0, bar.center().y);
        painter.circle_filled(mark_c, 9.0, hex("#E9E4D8"));
        painter.circle_filled(mark_c + vec2(3.0, -3.0), 4.0, ink);
        painter.text(mark_c + vec2(16.0, 0.0), Align2::LEFT_CENTER, &self.brand, brand_font, hex("#F2EFE8"));
        let items_x0 = bar.center().x - item_w * 1.5 + 10.0;
        let item_rect = |i: usize| Rect::from_min_size(pos2(items_x0 + i as f32 * item_w, bar.min.y + 8.0), vec2(item_w, bar_h - 16.0));
        for (i, label) in self.items.iter().enumerate() {
            let r = item_rect(i);
            let active = st.open == Some(i);
            let a = glowkit_core::anim::spring(ui, id.with(("item", i)), if active { 1.0 } else { 0.0 }, 300.0, 26.0);
            if a > 0.01 {
                painter.rect_filled(r.shrink2(vec2(4.0, 0.0)), cr(r.height() / 2.0), with_alpha(Color32::WHITE, 0.10 * a));
            }
            painter.text(r.center() - vec2(6.0, 0.0), Align2::CENTER_CENTER, label, item_font.clone(), lerp_c(hex("#A9A6A0"), hex("#FFFFFF"), a));
            let cc = pos2(r.center().x + r.width() * 0.3, r.center().y);
            let s = 3.0;
            let flip = 1.0 - 2.0 * a;
            painter.line(vec![cc + vec2(-s, -s * 0.5 * flip), cc + vec2(0.0, s * 0.5 * flip), cc + vec2(s, -s * 0.5 * flip)], Stroke::new(1.3, lerp_c(hex("#7D7A75"), hex("#FFFFFF"), a)));
        }
        let cta_g = painter.layout_no_wrap(self.cta.clone(), item_font.clone(), ink);
        let cta = Rect::from_min_size(pos2(bar.max.x - cta_g.size().x - 40.0, bar.min.y + 8.0), vec2(cta_g.size().x + 30.0, bar_h - 16.0));
        painter.rect_filled(cta, cr(cta.height() / 2.0), hex("#F2EFE8"));
        painter.galley(cta.center() - cta_g.size() / 2.0, cta_g, ink);
        // panel content
        let body = Rect::from_min_max(pos2(shell.min.x + 12.0, bar.max.y + 4.0), pos2(shell.max.x - 12.0, shell.max.y - 12.0));
        if body.height() > 6.0 {
            let clip = painter.with_clip_rect(body.expand2(vec2(2.0, 0.0)));
            let k = ((st.h.value - bar_h) / (panel_sizes[st.shown].y)).clamp(0.0, 1.0);
            let e = glowkit_core::ease(glowkit_core::Easing::OutCubic, (st.switch_t / 0.45).min(1.0));
            let dir = if st.shown >= st.prev { 1.0 } else { -1.0 };
            if e < 1.0 && st.prev != st.shown {
                nav_panel(ui, &clip, st.prev, body, (1.0 - e) * k, -dir * 30.0 * e, 6.0 * e, hl_index, time);
            }
            nav_panel(ui, &clip, st.shown, body, e * k, dir * 30.0 * (1.0 - e), 6.0 * (1.0 - e), hl_index, time);
        }
        response
    }
}

#[allow(clippy::too_many_arguments)]
fn nav_panel(ui: &Ui, painter: &Painter, which: usize, body: Rect, alpha: f32, dx: f32, blur: f32, hl: f32, time: f32) {
    if alpha < 0.01 {
        return;
    }
    let body = body.translate(vec2(dx, 0.0));
    let fg = with_alpha(hex("#F2EFE8"), alpha);
    let mute = with_alpha(hex("#A39F97"), alpha);
    let title_f = semibold_font(ui, 16.0);
    let small = FontId::proportional(12.5);
    let text = |p: Pos2, s: &str, f: &FontId, c: Color32, align: Align2| {
        let g = painter.layout_no_wrap(s.to_owned(), f.clone(), c);
        let pos = align.anchor_size(p, g.size()).min;
        paint::blurred_galley(painter, pos, &g, c, blur);
    };
    let blur_img = |r: Rect, scene: usize, tint: Color32| {
        let tex = PaintedScene::texture(ui.ctx(), scene);
        let uv = cover_uv(PaintedScene::ASPECT, r.width() / r.height());
        if blur > 0.5 {
            for k in 0..4 {
                let a = k as f32 / 4.0 * TAU;
                image_rounded(painter, r.translate(vec2(a.cos(), a.sin()) * blur), 14.0, tex, uv, tint.gamma_multiply(0.3));
            }
        } else {
            image_rounded(painter, r, 14.0, tex, uv, tint);
        }
    };
    match which {
        0 => {
            // picture reel: three tall cards with captions
            let gap = 10.0;
            let cw = (body.width() - gap * 2.0) / 3.0;
            for (i, (title, sub)) in REEL.iter().enumerate() {
                let r = Rect::from_min_size(pos2(body.min.x + i as f32 * (cw + gap), body.min.y + 4.0), vec2(cw, body.height() - 4.0));
                let on = (1.0 - (hl - i as f32).abs()).clamp(0.0, 1.0);
                let drift = (time * 0.3 + i as f32).sin() * 2.0;
                let rr = r.shrink(2.0 * (1.0 - on)).translate(vec2(0.0, drift * 0.3));
                blur_img(rr, [4, 6, 3][i], with_alpha(Color32::WHITE, alpha * (0.75 + 0.25 * on)));
                paint::gradient_rect(
                    painter,
                    Rect::from_min_max(pos2(rr.min.x, rr.max.y - 74.0), rr.max),
                    14.0,
                    90.0,
                    &[with_alpha(Color32::BLACK, 0.0), with_alpha(Color32::BLACK, 0.55 * alpha)],
                );
                text(pos2(rr.min.x + 16.0, rr.max.y - 38.0), title, &title_f, fg, Align2::LEFT_BOTTOM);
                text(pos2(rr.min.x + 16.0, rr.max.y - 16.0), sub, &small, with_alpha(Color32::WHITE, 0.75 * alpha), Align2::LEFT_BOTTOM);
            }
        }
        1 => {
            // accordion of cards: the highlighted one expands to show its picture
            let n = ACCORDION.len();
            let collapsed = 46.0;
            let gap = 6.0;
            let expanded = body.height() - collapsed * (n - 1) as f32 - gap * (n - 1) as f32;
            let mut y = body.min.y + 2.0;
            for (i, (title, sub)) in ACCORDION.iter().enumerate() {
                let on = (1.0 - (hl - i as f32).abs()).clamp(0.0, 1.0);
                let hgt = collapsed + (expanded - collapsed) * on;
                let r = Rect::from_min_size(pos2(body.min.x, y), vec2(body.width(), hgt - 2.0));
                painter.rect_filled(r, cr(14.0), with_alpha(Color32::WHITE, 0.06 * alpha));
                if on > 0.05 {
                    let img = Rect::from_min_max(pos2(r.max.x - r.width() * 0.46, r.min.y + 6.0), r.max - vec2(6.0, 6.0));
                    if img.height() > 8.0 {
                        blur_img(img, [1, 7, 2, 0][i], with_alpha(Color32::WHITE, alpha * on));
                    }
                }
                text(pos2(r.min.x + 16.0, r.min.y + 23.0), title, &title_f, fg, Align2::LEFT_CENTER);
                text(pos2(r.min.x + 16.0, r.min.y + 23.0 + 22.0 * on), sub, &small, with_alpha(hex("#A39F97"), alpha * on), Align2::LEFT_CENTER);
                text(pos2(r.min.x + body.width() * 0.46, r.min.y + 23.0), &format!("0{}", i + 1), &small, with_alpha(hex("#A39F97"), alpha * (1.0 - on)), Align2::RIGHT_CENTER);
                y += hgt + gap - 2.0;
            }
        }
        _ => {
            // link list with a gliding highlight
            let n = LINKS.len();
            let rh = body.height() / n as f32;
            let hr = Rect::from_min_size(pos2(body.min.x, body.min.y + hl * rh + 2.0), vec2(body.width(), rh - 4.0));
            painter.rect_filled(hr, cr(12.0), with_alpha(Color32::WHITE, 0.08 * alpha));
            for (i, (title, meta)) in LINKS.iter().enumerate() {
                let cy = body.min.y + (i as f32 + 0.5) * rh;
                let on = (1.0 - (hl - i as f32).abs()).clamp(0.0, 1.0);
                let thumb = Rect::from_center_size(pos2(body.min.x + 34.0, cy), vec2(44.0, rh - 18.0));
                blur_img(thumb, i + 2, with_alpha(Color32::WHITE, alpha));
                text(pos2(body.min.x + 70.0 + 4.0 * on, cy - 9.0), title, &title_f, fg, Align2::LEFT_CENTER);
                text(pos2(body.min.x + 70.0 + 4.0 * on, cy + 11.0), meta, &small, mute, Align2::LEFT_CENTER);
                MicroIcon::ArrowUpRight.paint(painter, pos2(body.max.x - 24.0 + 3.0 * on, cy), 14.0, with_alpha(hex("#F2EFE8"), alpha * (0.3 + 0.7 * on)));
            }
        }
    }
}

