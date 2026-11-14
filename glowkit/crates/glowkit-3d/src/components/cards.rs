use super::{rounded_outline, salt_or};
use crate::{vec3, Background, Camera, Scene, Vec3, Viewport3D};
use egui::{Align2, Color32, Response, Ui, Vec2, Widget};
use glowkit_core::{color, ease, Easing, Theme};
use std::f32::consts::{PI, TAU};

/// A card shown by the 3D card components: a gradient face with a title and subtitle.
#[derive(Clone, Debug, PartialEq)]
pub struct Card3D {
    pub title: String,
    pub subtitle: String,
    /// Gradient from the top-left to the bottom-right corner.
    pub colors: [Color32; 2],
}

impl Card3D {
    /// A card with a title, subtitle and two-stop gradient.
    pub fn new(title: impl Into<String>, subtitle: impl Into<String>, colors: [Color32; 2]) -> Self {
        Self { title: title.into(), subtitle: subtitle.into(), colors }
    }

    /// A set of pleasant sample cards.
    pub fn samples() -> Vec<Card3D> {
        let h = color::hex;
        vec![
            Card3D::new("Aurora", "Northern lights", [h("#8B5CF6"), h("#22D3EE")]),
            Card3D::new("Ember", "Warm gradients", [h("#F97316"), h("#EC4899")]),
            Card3D::new("Lagoon", "Deep blue sea", [h("#0EA5E9"), h("#2DD4BF")]),
            Card3D::new("Orchid", "Soft petals", [h("#D946EF"), h("#F472B6")]),
            Card3D::new("Citrus", "Fresh & bright", [h("#FACC15"), h("#84CC16")]),
            Card3D::new("Nebula", "Far, far away", [h("#6366F1"), h("#A855F7")]),
            Card3D::new("Coral", "Summer reef", [h("#FB7185"), h("#FDBA74")]),
            Card3D::new("Forest", "Quiet green", [h("#10B981"), h("#065F46")]),
        ]
    }
}

pub(crate) struct CardStyle {
    pub radius: f32,
    pub opacity: f32,
    pub text: Color32,
    pub border: Color32,
    pub sheen: f32,
    pub text_scale: f32,
    /// Fade alpha out towards the card's local top (used for floor reflections).
    pub reflect: bool,
}

/// Draw a card face (both sides; the back side shows `back` if given, else a darker face).
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_card(
    scene: &mut Scene,
    center: Vec3,
    right: Vec3,
    up: Vec3,
    size: Vec2,
    card: &Card3D,
    back: Option<&Card3D>,
    style: &CardStyle,
) {
    let normal = right.cross(up);
    let cam = scene.camera().position;
    let front = normal.dot(cam - center) > 0.0;
    let (right, card, dim) = match (front, back) {
        (true, _) => (right, card, 1.0),
        (false, Some(b)) => (-right, b, 1.0),
        (false, None) => (-right, card, 0.55),
    };
    let show_text = front || back.is_some();
    let n = right.cross(up);
    let outline = rounded_outline(size.x, size.y, style.radius, 5);
    let world = |x: f32, y: f32| center + right * x + up * y;
    // sheen: brighter when the card faces the light / viewer
    let to_cam = (cam - center).normalized();
    let light = vec3(-0.4, 0.6, 0.7).normalized();
    let h = (to_cam + light).normalized();
    let sheen = n.dot(h).max(0.0).powf(16.0) * style.sheen;
    let a = style.opacity;
    let col_at = |x: f32, y: f32| -> Color32 {
        let t = ((x / size.x - y / size.y) * 0.5 + 0.5).clamp(0.0, 1.0);
        let c = color::mix(card.colors[0], card.colors[1], t);
        let c = if dim < 1.0 { color::mix(c, Color32::BLACK, 1.0 - dim) } else { c };
        let c = color::mix(c, Color32::WHITE, sheen * (1.0 - t * 0.6));
        let k = if style.reflect { (0.5 - y / size.y).clamp(0.0, 1.0).powi(2) } else { 1.0 };
        color::with_alpha(c, a * k)
    };
    scene.group(center, |s| {
        let cc = col_at(0.0, 0.0);
        let m = outline.len();
        for i in 0..m {
            let (p, q) = (outline[i], outline[(i + 1) % m]);
            s.triangle([world(0.0, 0.0), world(p.0, p.1), world(q.0, q.1)], [cc, col_at(p.0, p.1), col_at(q.0, q.1)]);
        }
        // inner decoration: a soft circle top-right
        let deco_c = color::with_alpha(Color32::WHITE, 0.16 * a);
        let (dx, dy, dr) = (size.x * 0.22, size.y * 0.2, size.x.min(size.y) * 0.22);
        let k = 20;
        for i in 0..k {
            let (a0, a1) = (i as f32 / k as f32 * TAU, (i + 1) as f32 / k as f32 * TAU);
            s.triangle(
                [
                    world(dx, dy),
                    world(dx + a0.cos() * dr, dy + a0.sin() * dr),
                    world(dx + a1.cos() * dr, dy + a1.sin() * dr),
                ],
                [deco_c, color::with_alpha(Color32::WHITE, 0.04 * a), color::with_alpha(Color32::WHITE, 0.04 * a)],
            );
        }
        let pts: Vec<Vec3> = outline.iter().map(|p| world(p.0, p.1)).collect();
        if !style.reflect {
            let bc = style.border.gamma_multiply(a);
            s.polyline(&pts, 1.0, true, |_| bc);
        }
        if !show_text || style.reflect {
            return;
        }
        let ts = style.text_scale * size.y.min(size.x * 1.4);
        let tc = style.text.gamma_multiply(a);
        let pad = size.x * 0.09;
        let maxw = size.x - pad * 2.0;
        s.text_fitted(
            &card.title,
            world(-size.x / 2.0 + pad, -size.y / 2.0 + pad + ts * 0.42),
            right,
            up,
            ts * 0.19,
            maxw,
            tc,
            Align2::LEFT_BOTTOM,
        );
        s.text_fitted(
            &card.subtitle,
            world(-size.x / 2.0 + pad, -size.y / 2.0 + pad),
            right,
            up,
            ts * 0.1,
            maxw,
            tc.gamma_multiply(0.8),
            Align2::LEFT_BOTTOM,
        );
    });
}

fn text_on(card: &Card3D) -> Color32 {
    let c = color::mix(card.colors[0], card.colors[1], 0.5);
    if color::luminance(c) > 0.5 {
        color::hex("#0A0A0C")
    } else {
        Color32::WHITE
    }
}

// ---------------------------------------------------------------------------------------------
// CardCarousel3D
// ---------------------------------------------------------------------------------------------

/// A ring of gradient cards spinning in 3D, with a glossy floor reflection. Drag sideways to
/// spin it (with inertia); it auto-rotates when left alone.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_3d::{CardCarousel3D, Card3D};
/// ui.add(CardCarousel3D::new(Card3D::samples()).speed(0.25).height(360.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CardCarousel3D {
    cards: Vec<Card3D>,
    card_size: Vec2,
    speed: f32,
    reflection: bool,
    tilt: f32,
    height: f32,
    id_salt: Option<String>,
}

#[derive(Clone, Copy, Default)]
struct Spin {
    angle: f32,
    vel: f32,
    idle: f32,
}

impl CardCarousel3D {
    /// A carousel of `cards`.
    pub fn new(cards: Vec<Card3D>) -> Self {
        Self {
            cards,
            card_size: egui::vec2(1.4, 1.9),
            speed: 0.3,
            reflection: true,
            tilt: 0.12,
            height: 360.0,
            id_salt: None,
        }
    }
    /// Card size in world units.
    pub fn card_size(mut self, s: Vec2) -> Self {
        self.card_size = s;
        self
    }
    /// Auto-rotation speed (radians/second).
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Floor reflection.
    pub fn reflection(mut self, on: bool) -> Self {
        self.reflection = on;
        self
    }
    /// Camera elevation (radians).
    pub fn tilt(mut self, t: f32) -> Self {
        self.tilt = t;
        self
    }
    /// Height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Id salt for several instances.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.id_salt = Some(s.into());
        self
    }
}

impl Widget for CardCarousel3D {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.cards.len().max(1);
        let size = self.card_size;
        let radius = ((size.x + 0.55) * n as f32 / TAU).max(1.2);
        Viewport3D::new(salt_or(&self.id_salt, "card-carousel"))
            .height(self.height)
            .camera(Camera::default())
            .show(ui, |scene, ctx| {
                let dist = radius + 4.1;
                let eye = vec3(0.0, dist * self.tilt.sin() + 0.2, dist * self.tilt.cos());
                scene.set_camera(Camera::looking_at(eye, vec3(0.0, -0.3, 0.0)).fov_degrees(38.0));
                let drag = if ctx.response.dragged() { Some(ctx.response.drag_delta().x) } else { None };
                let dt = ctx.dt.max(1e-4);
                let spin = ctx.state("spin", Spin::default, |s| {
                    if let Some(dx) = drag {
                        let d = dx * 0.006;
                        s.angle += d;
                        s.vel = d / dt;
                        s.idle = 0.0;
                    } else {
                        s.idle += dt;
                        s.vel *= (-3.0 * dt).exp();
                        let auto = self.speed * ((s.idle - 0.5) / 1.0).clamp(0.0, 1.0);
                        s.angle += (s.vel - auto) * dt;
                    }
                    *s
                });
                if ctx.response.hovered() {
                    ctx.ui.ctx().set_cursor_icon(if ctx.response.dragged() {
                        egui::CursorIcon::Grabbing
                    } else {
                        egui::CursorIcon::Grab
                    });
                }
                let floor = -size.y / 2.0 - 0.04;
                let border = color::with_alpha(Color32::WHITE, 0.35);
                for (i, card) in self.cards.iter().enumerate() {
                    let a = spin.angle + i as f32 / n as f32 * TAU;
                    let c = vec3(a.sin() * radius, 0.0, a.cos() * radius);
                    let right = vec3(a.cos(), 0.0, -a.sin());
                    let facing = (a.cos() * 0.5 + 0.5).clamp(0.0, 1.0);
                    let opacity = 0.35 + 0.65 * facing;
                    let style = CardStyle {
                        radius: 0.12,
                        opacity,
                        text: text_on(card),
                        border,
                        sheen: 0.55,
                        text_scale: 1.0,
                        reflect: false,
                    };
                    draw_card(scene, c, right, Vec3::Y, size, card, None, &style);
                    if self.reflection && facing > 0.35 {
                        let rc = vec3(c.x, floor * 2.0 - c.y, c.z);
                        let fade = if t.dark { 0.3 } else { 0.22 };
                        let style = CardStyle { opacity: opacity * fade, reflect: true, ..style };
                        scene.underlay(|s| draw_card(s, rc, right, -Vec3::Y, size, card, None, &style));
                    }
                }
                // floor glow
                scene.underlay(|s| {
                    s.shadow(
                        vec3(0.0, floor, 0.0),
                        radius * 1.3,
                        super::glow_c(&t, t.accent, if t.dark { 0.12 } else { 0.08 }),
                    )
                });
            })
            .response
    }
}

// ---------------------------------------------------------------------------------------------
// FlipCard3D
// ---------------------------------------------------------------------------------------------

/// A card that flips over in 3D when clicked (with a springy overshoot) and tilts towards the
/// pointer while hovered. Optionally flips on its own on a timer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_3d::{FlipCard3D, Card3D};
/// let front = Card3D::new("Front", "Click to flip", [egui::Color32::BLUE, egui::Color32::LIGHT_BLUE]);
/// let back = Card3D::new("Back", "Hello!", [egui::Color32::RED, egui::Color32::YELLOW]);
/// ui.add(FlipCard3D::new(front, back).size(egui::vec2(220.0, 300.0)));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct FlipCard3D {
    front: Card3D,
    back: Card3D,
    size: Vec2,
    auto_flip: Option<f32>,
    offset: f32,
    id_salt: Option<String>,
}

#[derive(Clone, Copy, Default)]
struct FlipState {
    flips: u32,
    spring: glowkit_core::Spring,
    tilt: (f32, f32),
}

impl FlipCard3D {
    /// A flip card with two faces.
    pub fn new(front: Card3D, back: Card3D) -> Self {
        Self { front, back, size: egui::vec2(220.0, 300.0), auto_flip: None, offset: 0.0, id_salt: None }
    }
    /// Widget size in points (the card fills most of it).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = s;
        self
    }
    /// Flip automatically every `seconds` (plus `offset` seconds), in addition to clicks.
    pub fn auto_flip(mut self, seconds: f32, offset: f32) -> Self {
        self.auto_flip = Some(seconds.max(0.5));
        self.offset = offset;
        self
    }
    /// Id salt for several instances.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.id_salt = Some(s.into());
        self
    }
}

impl Widget for FlipCard3D {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = self.size;
        let ch = 2.0f32;
        let cw = ch * (size.x / size.y) * 0.84;
        Viewport3D::new(salt_or(&self.id_salt, "flip-card"))
            .size(size)
            .camera(Camera::looking_at(vec3(0.0, 0.0, 5.0), Vec3::ZERO).fov_degrees(30.0))
            .show(ui, |scene, ctx| {
                let clicked = ctx.response.clicked();
                let target_tilt = ctx.pointer_ndc.map(|p| (p.y * 0.35, p.x * 0.45)).unwrap_or((0.0, 0.0));
                let auto_index =
                    self.auto_flip.map(|s| (((ctx.time + self.offset) / s).floor().max(0.0)) as u32).unwrap_or(0);
                let dt = ctx.dt;
                let st = ctx.state(
                    "flip",
                    || FlipState {
                        flips: 0,
                        spring: glowkit_core::Spring::new(0.0).with(120.0, 14.0),
                        tilt: (0.0, 0.0),
                    },
                    |s| {
                        if clicked {
                            s.flips += 1;
                        }
                        s.spring.target = (s.flips + auto_index) as f32 * PI;
                        s.spring.step(dt);
                        let k = 1.0 - (-8.0 * dt).exp();
                        s.tilt.0 += (target_tilt.0 - s.tilt.0) * k;
                        s.tilt.1 += (target_tilt.1 - s.tilt.1) * k;
                        *s
                    },
                );
                if ctx.response.hovered() {
                    ctx.ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                let yaw = st.spring.value + st.tilt.1;
                let pitch = -st.tilt.0;
                let q = crate::Quat::from_axis_angle(Vec3::X, pitch) * crate::Quat::from_axis_angle(Vec3::Y, yaw);
                let right = q.rotate(Vec3::X);
                let up = q.rotate(Vec3::Y);
                let t = Theme::of(ctx.ui);
                let style = CardStyle {
                    radius: 0.14,
                    opacity: 1.0,
                    text: text_on(&self.front),
                    border: color::with_alpha(Color32::WHITE, 0.35),
                    sheen: 0.7,
                    text_scale: 1.0,
                    reflect: false,
                };
                let back_style = CardStyle { text: text_on(&self.back), ..style };
                let lift = (st.spring.value % PI / PI * PI).sin().abs();
                scene.shadow(vec3(0.0, -ch / 2.0 - 0.25, 0.0), cw * (0.7 - 0.15 * lift), super::shadow_c(&t, 0.8));
                let normal = right.cross(up);
                let front_visible = normal.dot(scene.camera().position) > 0.0;
                let st2 = if front_visible { &style } else { &back_style };
                draw_card(scene, Vec3::ZERO, right, up, egui::vec2(cw, ch), &self.front, Some(&self.back), st2);
            })
            .response
    }
}

// ---------------------------------------------------------------------------------------------
// Marquee3D
// ---------------------------------------------------------------------------------------------

/// A tilted 3D wall of cards whose columns scroll in alternating directions, fading out at the
/// edges: a perspective "3D marquee".
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_3d::{Marquee3D, Card3D};
/// ui.add(Marquee3D::new(Card3D::samples()).columns(5).speed(0.4).height(360.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Marquee3D {
    cards: Vec<Card3D>,
    columns: usize,
    speed: f32,
    card_size: Vec2,
    height: f32,
    id_salt: Option<String>,
}

impl Marquee3D {
    /// A marquee of `cards` (repeated as needed).
    pub fn new(cards: Vec<Card3D>) -> Self {
        Self { cards, columns: 5, speed: 0.35, card_size: egui::vec2(1.6, 1.05), height: 360.0, id_salt: None }
    }
    /// Number of columns.
    pub fn columns(mut self, n: usize) -> Self {
        self.columns = n.clamp(1, 12);
        self
    }
    /// Scroll speed (world units per second).
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Card size in world units.
    pub fn card_size(mut self, s: Vec2) -> Self {
        self.card_size = s;
        self
    }
    /// Height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Id salt for several instances.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.id_salt = Some(s.into());
        self
    }
}

impl Widget for Marquee3D {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let bg = if t.dark { Background::Radial(color::mix(t.card, t.accent, 0.08), t.card) } else { Background::None };
        Viewport3D::new(salt_or(&self.id_salt, "marquee-3d"))
            .height(self.height)
            .background(bg)
            .camera(Camera::looking_at(vec3(0.0, 0.0, 9.0), Vec3::ZERO).fov_degrees(40.0))
            .show(ui, |scene, ctx| {
                if self.cards.is_empty() {
                    return;
                }
                let cols = self.columns;
                let (cw, ch) = (self.card_size.x, self.card_size.y);
                let gap = 0.22;
                let pitch_y = ch + gap;
                let per_col = 9usize;
                let col_h = per_col as f32 * pitch_y;
                // the wall: tilted back and rotated, like an isometric board
                let hover = ctx.pointer_ndc.unwrap_or_default();
                let q = crate::Quat::from_axis_angle(Vec3::Z, -0.62 + hover.x * 0.05)
                    * crate::Quat::from_axis_angle(Vec3::X, -0.9 - hover.y * 0.05);
                let q = crate::Quat::from_axis_angle(Vec3::X, 0.0) * q;
                let right = q.rotate(Vec3::X);
                let up = q.rotate(Vec3::Y);
                let total_w = cols as f32 * (cw + gap);
                let reach = total_w.max(col_h) * 0.5;
                let border = color::with_alpha(Color32::WHITE, 0.3);
                for c in 0..cols {
                    let dir = if c % 2 == 0 { 1.0 } else { -1.0 };
                    let x = (c as f32 + 0.5) * (cw + gap) - total_w / 2.0;
                    let off = (ctx.time * self.speed * dir * (1.0 + (c % 3) as f32 * 0.15)).rem_euclid(pitch_y);
                    for r in 0..per_col {
                        let y = r as f32 * pitch_y - col_h / 2.0 + off;
                        let local = egui::vec2(x, y);
                        let d = (local.length() / reach).clamp(0.0, 1.0);
                        let fade = 1.0 - ease(Easing::InCubic, d);
                        if fade <= 0.02 {
                            continue;
                        }
                        let card = &self.cards[(c * 3 + r) % self.cards.len()];
                        let center = right * x + up * y;
                        let style = CardStyle {
                            radius: 0.1,
                            opacity: fade,
                            text: text_on(card),
                            border,
                            sheen: 0.4,
                            text_scale: 1.0,
                            reflect: false,
                        };
                        draw_card(scene, center, right, up, egui::vec2(cw, ch), card, None, &style);
                    }
                }
            })
            .response
    }
}
