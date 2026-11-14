//! 3D picture carousels: a DNA helix, a giant arc, a wave ribbon, an endless image tunnel and a
//! spiral slider. All of them use the built-in painted photographs unless given textures.

use super::common::{
    cover_uv, display_font, drive_loop, drive_snap, edge_alpha, galley_mapped, idle_time, lerp_c, lighten, poly_shadow,
    semibold_font, surface_grid, v3, wall_dt, Cam, V3,
};
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Pos2, Rect, Response, Sense, Stroke, TextureId, Ui, Vec2, Widget};
use glowkit_core::color::{hex, with_alpha};
use glowkit_core::{paint, Theme};
use glowkit_effects::PaintedScene;
use std::f32::consts::{PI, TAU};

fn scene_tex(ui: &Ui, own: &[TextureId], i: usize) -> (TextureId, bool) {
    if own.is_empty() {
        (PaintedScene::texture(ui.ctx(), i % PaintedScene::COUNT), false)
    } else {
        (own[i % own.len()], true)
    }
}

fn full_uv() -> Rect {
    Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0))
}

/// Paint a planar card whose local corners are mapped through `map3` (local -> 3D) and `cam`.
#[allow(clippy::too_many_arguments)]
fn card3d(
    painter: &egui::Painter,
    cam: &Cam,
    tex: TextureId,
    uv: Rect,
    size: Vec2,
    radius: f32,
    n: (usize, usize),
    map3: impl Fn(Vec2) -> V3,
    tint: impl Fn(f32, f32) -> Color32,
) {
    surface_grid(painter, Some(tex), uv, n.0, n.1, size, radius, |u, v| cam.pt(map3(vec2(u * size.x, v * size.y))), tint);
}

// ---------------------------------------------------------------------------------------------

/// Pictures wound onto two opposing strands of a helix that drifts sideways: cards climb and
/// lean against each other as they pass, and the far ones fall back blurred and colour-fringed
/// like the edge of a lens. Drag or scroll to spin it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::DnaCarousel::new().speed(0.25));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DnaCarousel {
    textures: Vec<TextureId>,
    height: f32,
    card: Vec2,
    twist: f32,
    speed: f32,
    fringe: f32,
}

impl Default for DnaCarousel {
    fn default() -> Self {
        Self::new()
    }
}

impl DnaCarousel {
    /// A helix of the built-in painted photographs.
    pub fn new() -> Self {
        Self { textures: Vec::new(), height: 380.0, card: vec2(178.0, 240.0), twist: 1.0, speed: 0.25, fringe: 1.0 }
    }
    /// Your own pictures (cropped to fill each card).
    pub fn textures(mut self, t: impl Into<Vec<TextureId>>) -> Self {
        self.textures = t.into();
        self
    }
    /// Widget height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(160.0);
        self
    }
    /// Card size.
    pub fn card_size(mut self, s: Vec2) -> Self {
        self.card = s.max(vec2(40.0, 40.0));
        self
    }
    /// Turns of twist per screen width, `0.2..=3`.
    pub fn twist(mut self, k: f32) -> Self {
        self.twist = k.clamp(0.2, 3.0);
        self
    }
    /// Idle drift in cards per second.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Strength of the colour fringe on far cards, `0..=2`.
    pub fn fringe(mut self, k: f32) -> Self {
        self.fringe = k.clamp(0.0, 2.0);
        self
    }
}

impl Widget for DnaCarousel {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::drag());
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Grab);
        }
        let spacing = self.card.x * 0.86;
        let off = drive_loop(ui, response.id, &response, true, self.speed, 1.0 / spacing);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let cam = Cam { center: rect.center(), focal: 900.0 };
        let radius = self.card.y * 0.55;
        let per = (rect.width() / spacing / 2.0).ceil() as i32 + 2;
        let k = TAU * self.twist / (rect.width() / spacing);
        let mut cards = Vec::new();
        let base = off.floor() as i32;
        for strand in 0..2 {
            for j in -per..=per {
                let i = base + j;
                let s = i as f32 - off;
                let th = s * k + strand as f32 * PI;
                let x = s * spacing + if strand == 1 { spacing * 0.5 } else { 0.0 };
                let y = th.sin() * radius * 0.42;
                let z = th.cos() * radius;
                let idx = (i * 2 + strand) as usize;
                cards.push((z, x, y, th, idx));
            }
        }
        cards.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let size = self.card;
        let bg = t.muted;
        for (z, x, y, th, idx) in cards {
            let (tex, own) = scene_tex(ui, &self.textures, idx.wrapping_mul(3) % 64);
            let uv = if own { full_uv() } else { cover_uv(PaintedScene::ASPECT, size.x / size.y) };
            let depth = ((z + radius) / (2.0 * radius)).clamp(0.0, 1.0); // 0 near, 1 far
            let lean = th.sin() * 0.16;
            let yaw = -th.sin() * 0.35;
            let map3 = |l: Vec2| v3(l.x - size.x / 2.0, l.y - size.y / 2.0, 0.0).rot_z(lean).rot_y(yaw).add(v3(x, y, z));
            let fog = depth * 0.55;
            let blur = depth * depth * 5.0;
            if (cam.pt(v3(x, y, z)).x - rect.center().x).abs() > rect.width() / 2.0 + size.x {
                continue;
            }
            // shadow
            let outline: Vec<Pos2> = [vec2(0.0, 0.0), vec2(size.x, 0.0), vec2(size.x, size.y), vec2(0.0, size.y)]
                .iter()
                .map(|l| cam.pt(map3(*l)))
                .collect();
            let sc = if t.dark { Color32::from_black_alpha(120) } else { Color32::from_rgba_unmultiplied(20, 20, 40, 40) };
            let ea = edge_alpha(rect, cam.pt(v3(x, y, z)).x, 110.0);
            poly_shadow(&painter, &outline, vec2(0.0, 14.0), 16.0, sc.gamma_multiply((1.0 - depth * 0.7) * ea));
            let edge = |u: f32, v: f32| edge_alpha(rect, cam.pt(map3(vec2(u * size.x, v * size.y))).x, 90.0);
            if blur > 0.8 {
                let f = self.fringe * depth * 7.0;
                for (dx, c) in [(-f, hex("#FF3B30")), (f, hex("#2F7BFF"))] {
                    card3d(&painter, &cam, tex, uv, size, 12.0, (3, 3), |l| map3(l).add(v3(dx, 0.0, 0.0)), |u, v| {
                        with_alpha(c, 0.28 * depth * edge(u, v))
                    });
                }
                for tap in 0..4 {
                    let a = tap as f32 / 4.0 * TAU + 0.4;
                    let o = v3(a.cos() * blur, a.sin() * blur, 0.0);
                    card3d(&painter, &cam, tex, uv, size, 12.0, (3, 3), |l| map3(l).add(o), |u, v| {
                        lerp_c(Color32::WHITE, bg, fog).gamma_multiply(0.33 * edge(u, v))
                    });
                }
            } else {
                card3d(&painter, &cam, tex, uv, size, 12.0, (6, 6), map3, |u, v| lerp_c(Color32::WHITE, bg, fog).gamma_multiply(edge(u, v)));
            }
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// One entry of an [`ArcCarousel`].
#[derive(Clone, Debug, PartialEq)]
pub struct ArcItem {
    /// Title shown under the arc when focused.
    pub title: String,
    /// One-line description.
    pub body: String,
    /// Picture; `None` uses a painted scene.
    pub texture: Option<TextureId>,
}

impl ArcItem {
    /// An item with a title and description.
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self { title: title.into(), body: body.into(), texture: None }
    }
    /// Picture texture.
    pub fn texture(mut self, t: TextureId) -> Self {
        self.texture = Some(t);
        self
    }
}

/// Cards riding a very large circle: the focused card stands upright at the apex and every
/// other card tilts tangent to the curve. The focused item's title, a line of copy and a
/// "Details" pill sit under the arc with arrows. Drag, scroll, click the arrows or use the
/// arrow keys; autoplay steps through while idle.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{ArcCarousel, ArcItem};
/// let items = vec![ArcItem::new("Old Quarter", "Lanes that bend toward the river.")];
/// ui.add(ArcCarousel::new(&items));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ArcCarousel<'a> {
    items: &'a [ArcItem],
    height: f32,
    card: Vec2,
    curve: f32,
    autoplay: f32,
}

impl<'a> ArcCarousel<'a> {
    /// An arc of `items`.
    pub fn new(items: &'a [ArcItem]) -> Self {
        Self { items, height: 400.0, card: vec2(118.0, 150.0), curve: 1.0, autoplay: 2.8 }
    }
    /// Widget height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(240.0);
        self
    }
    /// Card size.
    pub fn card_size(mut self, s: Vec2) -> Self {
        self.card = s.max(vec2(40.0, 40.0));
        self
    }
    /// How tightly the arc bends, `0.3..=2`.
    pub fn curve(mut self, k: f32) -> Self {
        self.curve = k.clamp(0.3, 2.0);
        self
    }
    /// Seconds between autoplay steps (0 = off).
    pub fn autoplay(mut self, secs: f32) -> Self {
        self.autoplay = secs.max(0.0);
        self
    }
}

fn chevron(painter: &egui::Painter, c: Pos2, s: f32, dir: f32, color: Color32) {
    painter.line(vec![c + vec2(-s * 0.4 * dir, -s), c + vec2(s * 0.4 * dir, 0.0), c + vec2(-s * 0.4 * dir, s)], Stroke::new(1.8, color));
}

impl Widget for ArcCarousel<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::click_and_drag());
        let id = response.id;
        let n = self.items.len().max(1);
        // arrow buttons (hit-tested from last frame's layout positions)
        let title_y = rect.min.y + self.height * 0.62;
        let left_btn = Rect::from_center_size(pos2(rect.center().x - 150.0, title_y), vec2(34.0, 34.0));
        let right_btn = Rect::from_center_size(pos2(rect.center().x + 150.0, title_y), vec2(34.0, 34.0));
        let mut nudge = 0.0;
        if response.clicked() {
            if let Some(p) = response.interact_pointer_pos() {
                if left_btn.contains(p) {
                    nudge = -1.0;
                } else if right_btn.contains(p) {
                    nudge = 1.0;
                }
            }
        }
        if response.hover_pos().is_some_and(|p| left_btn.contains(p) || right_btn.contains(p)) {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        } else if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Grab);
        }
        let pos = drive_snap(ui, id, &response, 90.0, self.autoplay, nudge);
        if !ui.is_rect_visible(rect) || self.items.is_empty() {
            return response;
        }
        let painter = ui.painter_at(rect);
        let big_r = 900.0 / self.curve;
        let center = pos2(rect.center().x, rect.min.y + 34.0 + self.card.y * 0.5 + big_r);
        let step = (self.card.x + 22.0) / big_r;
        let half = (rect.width() / 2.0 / big_r).asin().min(PI / 2.0) + step;
        let lo = (pos - half / step).floor() as i32 - 1;
        let hi = (pos + half / step).ceil() as i32 + 1;
        let size = self.card;
        for i in lo..=hi {
            let a = (i as f32 - pos) * step;
            if a.abs() > half + step {
                continue;
            }
            let item = &self.items[i.rem_euclid(n as i32) as usize];
            let focus = (1.0 - (i as f32 - pos).abs()).clamp(0.0, 1.0);
            let scale = 1.0 + 0.22 * focus;
            let c = center + vec2(a.sin(), -a.cos()) * big_r + vec2(0.0, -12.0 * focus);
            let (s, co) = a.sin_cos();
            let map = |l: Vec2| {
                let d = (l - size / 2.0) * scale;
                c + vec2(d.x * co - d.y * s, d.x * s + d.y * co)
            };
            let tex = item.texture.unwrap_or_else(|| PaintedScene::texture(ui.ctx(), i.rem_euclid(n as i32) as usize));
            let uv = if item.texture.is_some() { full_uv() } else { cover_uv(PaintedScene::ASPECT, size.x / size.y) };
            let outline: Vec<Pos2> =
                [vec2(0.0, 0.0), vec2(size.x, 0.0), vec2(size.x, size.y), vec2(0.0, size.y)].iter().map(|l| map(*l)).collect();
            let sc = if t.dark { Color32::from_black_alpha(110) } else { Color32::from_rgba_unmultiplied(30, 25, 20, 40) };
            poly_shadow(&painter, &outline, vec2(0.0, 10.0), 14.0, sc);
            let dim = 0.72 + 0.28 * focus;
            surface_grid(&painter, Some(tex), uv, 3, 3, size, 14.0, |u, v| map(vec2(u * size.x, v * size.y)), |_, v| {
                let k = dim * (1.0 - 0.18 * v * (1.0 - focus));
                Color32::from_gray((255.0 * k) as u8)
            });
        }
        // caption
        let idx = pos.round().rem_euclid(n as f32) as usize;
        let frac = (pos - pos.round()).abs() * 2.0;
        let a = (1.0 - frac).clamp(0.0, 1.0);
        let dy = 8.0 * (1.0 - a);
        let item = &self.items[idx];
        painter.text(pos2(rect.center().x, title_y + dy), Align2::CENTER_CENTER, &item.title, display_font(ui, 28.0), with_alpha(t.foreground, a));
        painter.text(
            pos2(rect.center().x, title_y + 34.0 + dy),
            Align2::CENTER_CENTER,
            &item.body,
            FontId::proportional(13.5),
            with_alpha(t.muted_foreground, a),
        );
        let det = Rect::from_center_size(pos2(rect.center().x, title_y + 72.0), vec2(78.0, 28.0));
        painter.rect_filled(det, egui::CornerRadius::same(14), t.primary);
        painter.text(det.center(), Align2::CENTER_CENTER, "Details", semibold_font(ui, 12.5), t.primary_foreground);
        let hov = response.hover_pos();
        for (b, d) in [(left_btn, -1.0), (right_btn, 1.0)] {
            let on = hov.is_some_and(|p| b.contains(p));
            if on {
                painter.circle_filled(b.center(), 17.0, with_alpha(t.foreground, 0.08));
            }
            chevron(&painter, b.center(), 6.0, d, if on { t.foreground } else { t.muted_foreground });
        }
        // progress dots
        let dots_y = det.max.y + 26.0;
        for k in 0..n {
            let on = if k == idx { 1.0 } else { 0.0 };
            let w = 6.0 + 12.0 * on;
            let x = rect.center().x + (k as f32 - (n as f32 - 1.0) / 2.0) * 14.0;
            painter.rect_filled(
                Rect::from_center_size(pos2(x, dots_y), vec2(w, 4.0)),
                egui::CornerRadius::same(2),
                with_alpha(t.foreground, 0.2 + 0.6 * on),
            );
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// An endless ribbon of glossy picture cards bent into a wave in depth: cards ride up over the
/// crest toward you and sink into fog beyond it, with their titles printed into the surface so
/// the type bends with them. Drag or scroll to move it; it drifts on its own when idle.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::WaveGallery::new().amplitude(1.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct WaveGallery {
    titles: Vec<String>,
    textures: Vec<TextureId>,
    height: f32,
    card: Vec2,
    amplitude: f32,
    speed: f32,
}

impl Default for WaveGallery {
    fn default() -> Self {
        Self::new()
    }
}

impl WaveGallery {
    /// A ribbon of the painted scenes with their names as titles.
    pub fn new() -> Self {
        Self {
            titles: (0..PaintedScene::COUNT).map(|i| PaintedScene::name(i).to_owned()).collect(),
            textures: Vec::new(),
            height: 360.0,
            card: vec2(330.0, 230.0),
            amplitude: 1.0,
            speed: 40.0,
        }
    }
    /// Card titles (cycled).
    pub fn titles(mut self, t: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let v: Vec<String> = t.into_iter().map(Into::into).collect();
        if !v.is_empty() {
            self.titles = v;
        }
        self
    }
    /// Your own pictures (cycled).
    pub fn textures(mut self, t: impl Into<Vec<TextureId>>) -> Self {
        self.textures = t.into();
        self
    }
    /// Widget height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(160.0);
        self
    }
    /// Card size.
    pub fn card_size(mut self, s: Vec2) -> Self {
        self.card = s.max(vec2(60.0, 40.0));
        self
    }
    /// Depth of the wave, `0..=2`.
    pub fn amplitude(mut self, a: f32) -> Self {
        self.amplitude = a.clamp(0.0, 2.0);
        self
    }
    /// Idle drift in points per second.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
}

impl Widget for WaveGallery {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::drag());
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Grab);
        }
        let off = drive_loop(ui, response.id, &response, true, self.speed, 1.0);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let time = glowkit_core::time(ui);
        let painter = ui.painter_at(rect);
        let cam = Cam { center: rect.center() + vec2(0.0, -6.0), focal: 760.0 };
        let gap = 18.0;
        let pitch = self.card.x + gap;
        let amp = 150.0 * self.amplitude;
        let wl = rect.width() * 1.15;
        let wave = |x: f32| -> (f32, f32, f32) {
            // depth (z), vertical lift (y), slope dz/dx
            let ph = x / wl * TAU + time * 0.25;
            let z = amp * (0.15 - ph.cos() * 0.85) - 30.0;
            let y = -ph.sin() * 18.0 * self.amplitude;
            let dz = amp * 0.85 * ph.sin() * TAU / wl;
            (z, y, dz)
        };
        let base = (off / pitch).floor() as i32;
        let n = (rect.width() / pitch / 1.4).ceil() as i32 + 2;
        let size = self.card;
        let bg = t.muted;
        let mut order: Vec<(f32, i32)> = (-n..=n + 1)
            .map(|j| {
                let i = base + j;
                let x0 = i as f32 * pitch - off;
                (wave(x0 + size.x / 2.0).0, i)
            })
            .collect();
        order.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let font = semibold_font(ui, 18.0);
        let fg_on_img = Color32::WHITE;
        for (_, i) in order {
            let x0 = i as f32 * pitch - off;
            let k = i.rem_euclid(1 << 16) as usize;
            let (tex, own) = scene_tex(ui, &self.textures, k);
            let uv = if own { full_uv() } else { cover_uv(PaintedScene::ASPECT, size.x / size.y) };
            let map3 = |l: Vec2| {
                let x = x0 + l.x;
                let (z, y, _) = wave(x);
                v3(x, l.y - size.y / 2.0 + y, z)
            };
            let p_mid = cam.pt(map3(size / 2.0));
            if (p_mid.x - rect.center().x).abs() > rect.width() * 0.75 + size.x {
                continue;
            }
            card3d(&painter, &cam, tex, uv, size, 16.0, (16, 3), map3, |u, _| {
                let x = x0 + u * size.x;
                let (z, _, dz) = wave(x);
                let fog = ((z + 40.0) / (amp * 1.4 + 1.0)).clamp(0.0, 1.0);
                let gloss = (-((dz - 0.55) / 0.22).powi(2)).exp() * 0.45;
                let c = lighten(Color32::WHITE, 0.0);
                let c = lerp_c(c, bg, fog * 0.85);
                let ea = edge_alpha(rect, cam.pt(map3(vec2(u * size.x, 0.5 * size.y))).x, 80.0);
                lighten(c, gloss * (1.0 - fog)).gamma_multiply(ea)
            });
            // title printed into the surface
            let title = &self.titles[k % self.titles.len()];
            let g = painter.layout_no_wrap(title.clone(), font.clone(), fg_on_img);
            let (zt, _, _) = wave(x0 + 20.0);
            let fog = ((zt + 40.0) / (amp * 1.4 + 1.0)).clamp(0.0, 1.0);
            galley_mapped(&painter, &g, with_alpha(fg_on_img, 0.95 * (1.0 - fog)), |l| {
                cam.pt(map3(vec2(20.0 + l.x, size.y - 22.0 - g.size().y + l.y)))
            });
            let ag = painter.layout_no_wrap("→".to_owned(), FontId::proportional(18.0), fg_on_img);
            galley_mapped(&painter, &ag, with_alpha(fg_on_img, 0.9 * (1.0 - fog)), |l| {
                cam.pt(map3(vec2(size.x - 36.0 + l.x, size.y - 22.0 - ag.size().y + l.y)))
            });
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct TunnelState {
    z: f32,
    speed: f32,
    paused: bool,
    hold: f32,
}

/// An endless corridor with pictures hung on all four walls and a fine wireframe, gliding
/// toward you forever. Click to pause it, hold to rush ahead; the view turns slightly toward
/// the pointer. A title floats at the vanishing point.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::ImageTunnel::new().title("Into the archive"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ImageTunnel {
    textures: Vec<TextureId>,
    height: f32,
    speed: f32,
    density: f32,
    title: String,
    subtitle: String,
}

impl Default for ImageTunnel {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageTunnel {
    /// A tunnel of the painted scenes.
    pub fn new() -> Self {
        Self {
            textures: Vec::new(),
            height: 380.0,
            speed: 1.0,
            density: 1.4,
            title: "Into the archive".into(),
            subtitle: "Click to pause · hold to rush ahead".into(),
        }
    }
    /// Your own pictures (cycled).
    pub fn textures(mut self, t: impl Into<Vec<TextureId>>) -> Self {
        self.textures = t.into();
        self
    }
    /// Widget height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(160.0);
        self
    }
    /// Cruise speed multiplier.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s.clamp(0.0, 5.0);
        self
    }
    /// How many pictures hang on the walls, `0.3..=2`.
    pub fn density(mut self, d: f32) -> Self {
        self.density = d.clamp(0.3, 2.0);
        self
    }
    /// Title at the vanishing point (empty hides it).
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Small line under the title.
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = s.into();
        self
    }
}

impl Widget for ImageTunnel {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::click_and_drag());
        let id = response.id;
        let dt = wall_dt(ui);
        let clicked = response.clicked();
        let down = response.is_pointer_button_down_on();
        let cruise = 140.0 * self.speed;
        let st = glowkit_core::with_state(ui, id.with("tunnel"), TunnelState::default, |s| {
            if down {
                s.hold += dt;
            } else {
                s.hold = 0.0;
            }
            if clicked {
                s.paused = !s.paused;
            }
            let target = if s.hold > 0.18 { cruise * 7.0 } else if s.paused { 0.0 } else { cruise };
            s.speed = glowkit_core::anim::approach(s.speed, target, if s.hold > 0.18 { 3.0 } else { 2.2 }, dt);
            s.z += s.speed * dt;
            *s
        });
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        let time = glowkit_core::time(ui);
        let hover = response.hover_pos();
        let idle = idle_time(ui, id, hover.is_some());
        let aim = match hover {
            Some(p) => (p - rect.center()) / (rect.size() / 2.0),
            None if idle > 1.0 => vec2((time * 0.3).sin() * 0.6, (time * 0.23).cos() * 0.3),
            None => Vec2::ZERO,
        };
        let yaw = glowkit_core::anim::follow(ui, id.with("yaw"), aim.x * 0.12, 3.0);
        let pitch = glowkit_core::anim::follow(ui, id.with("pitch"), -aim.y * 0.08, 3.0);
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let (bg, line_c) = if t.dark { (hex("#060608"), with_alpha(Color32::WHITE, 0.10)) } else { (hex("#F4F3EF"), with_alpha(Color32::BLACK, 0.12)) };
        painter.rect_filled(rect, egui::CornerRadius::same((t.radius * 1.6) as u8), bg);
        let f = rect.height() * 0.9;
        let cam = Cam { center: rect.center(), focal: f };
        let hw = rect.width() / rect.height() * 120.0;
        let hh = 120.0;
        let near = 12.0;
        let far = 2400.0;
        let view = |p: V3| p.rot_y(yaw).rot_x(pitch);
        let proj = |p: V3| -> Option<Pos2> {
            let q = view(p);
            if q.z < near {
                return None;
            }
            Some(cam.center + vec2(q.x, q.y) * (f / q.z))
        };
        let fog = |z: f32| (1.0 - (z / far)).clamp(0.0, 1.0).powf(1.6);
        // wireframe rings
        let ring_gap = 120.0;
        let z0 = st.z % ring_gap;
        let mut z = ring_gap - z0;
        while z < far {
            let a = fog(z);
            let pts: Vec<Option<Pos2>> = [v3(-hw, -hh, z), v3(hw, -hh, z), v3(hw, hh, z), v3(-hw, hh, z)].iter().map(|p| proj(*p)).collect();
            if pts.iter().all(|p| p.is_some()) {
                let p: Vec<Pos2> = pts.into_iter().flatten().collect();
                painter.add(egui::Shape::closed_line(p, Stroke::new(1.0, line_c.gamma_multiply(a))));
            }
            z += ring_gap;
        }
        // longitudinal lines
        for k in 0..=6 {
            let s = k as f32 / 6.0 * 2.0 - 1.0;
            for (a, b) in [
                (v3(s * hw, -hh, near + 1.0), v3(s * hw, -hh, far)),
                (v3(s * hw, hh, near + 1.0), v3(s * hw, hh, far)),
                (v3(-hw, s * hh, near + 1.0), v3(-hw, s * hh, far)),
                (v3(hw, s * hh, near + 1.0), v3(hw, s * hh, far)),
            ] {
                let pts: Vec<Pos2> = (0..12)
                    .filter_map(|i| {
                        let tt = (i as f32 / 11.0).powf(2.5);
                        proj(a.add(b.sub(a).mul(tt)))
                    })
                    .collect();
                paint::gradient_polyline(&painter, &pts, false, 1.0, |u| line_c.gamma_multiply(1.0 - u));
            }
        }
        // pictures on the walls, far to near
        let slot = 260.0 / self.density;
        let first = (st.z / slot).floor() as i64;
        let count = (far / slot) as i64 + 2;
        let mut quads = Vec::new();
        for k in (0..count).rev() {
            let idx = first + k;
            let zc = idx as f32 * slot - st.z;
            if zc < near - 160.0 || zc > far {
                continue;
            }
            let h = |s: i64| super::common::hash3(idx as i32, s as i32, 11);
            let wall = (idx.rem_euclid(4)) as usize;
            let len = 150.0 + 90.0 * h(1);
            let span = 0.35 + 0.35 * h(2);
            let along = (h(3) - 0.5) * (1.0 - span) * 1.6;
            quads.push((zc, wall, len, span, along, idx));
        }
        for (zc, wall, len, span, along, idx) in quads {
            let (tex, own) = scene_tex(ui, &self.textures, idx.rem_euclid(97) as usize);
            let uv = if own { full_uv() } else { cover_uv(PaintedScene::ASPECT, 1.4) };
            let map = |u: f32, v: f32| -> Option<Pos2> {
                let zz = zc + (u - 0.5) * len;
                let side = along + (v - 0.5) * span * 2.0;
                let p = match wall {
                    0 => v3(-hw + 0.5, side * hh, zz),
                    1 => v3(hw - 0.5, side * hh, zz),
                    2 => v3(side * hw, -hh + 0.5, zz),
                    _ => v3(side * hw, hh - 0.5, zz),
                };
                proj(p)
            };
            // skip if any corner is behind the camera
            if [map(0.0, 0.0), map(1.0, 0.0), map(0.0, 1.0), map(1.0, 1.0)].iter().any(|p| p.is_none()) {
                continue;
            }
            let near_fade = ((zc - near) / 140.0).clamp(0.0, 1.0);
            surface_grid(&painter, Some(tex), uv, 8, 2, vec2(len, 100.0), 0.0, |u, v| map(u, v).unwrap_or(rect.center()), |u, _| {
                let zz = zc + (u - 0.5) * len;
                let k = fog(zz.max(near)) * near_fade.max(0.0);
                if t.dark {
                    Color32::from_gray((255.0 * k) as u8)
                } else {
                    lerp_c(bg, Color32::WHITE, k)
                }
            });
        }
        // vanishing-point glow and title
        let vp = proj(v3(0.0, 0.0, far)).unwrap_or(rect.center());
        paint::glow(&painter, vp, 120.0, with_alpha(if t.dark { hex("#2B2B3A") } else { Color32::WHITE }, 0.5));
        if !self.title.is_empty() {
            let c = rect.center();
            painter.text(c + vec2(0.0, -8.0), Align2::CENTER_CENTER, &self.title, display_font(ui, 30.0), t.foreground);
            painter.text(c + vec2(0.0, 22.0), Align2::CENTER_CENTER, &self.subtitle, FontId::proportional(12.5), t.muted_foreground);
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// Curved picture panels wound onto a helix that screws along its own track. The far side shows
/// through the gaps, frosted, so the spiral reads as depth instead of a second row. Drag or
/// scroll to wind it; it turns slowly on its own.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::SpiralSlider::new().turns(1.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SpiralSlider {
    textures: Vec<TextureId>,
    height: f32,
    radius: f32,
    per_turn: usize,
    pitch: f32,
    speed: f32,
    frost: f32,
}

impl Default for SpiralSlider {
    fn default() -> Self {
        Self::new()
    }
}

impl SpiralSlider {
    /// A spiral of the painted scenes.
    pub fn new() -> Self {
        Self { textures: Vec::new(), height: 380.0, radius: 165.0, per_turn: 6, pitch: 1.0, speed: 0.22, frost: 1.0 }
    }
    /// Your own pictures (cycled).
    pub fn textures(mut self, t: impl Into<Vec<TextureId>>) -> Self {
        self.textures = t.into();
        self
    }
    /// Widget height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(160.0);
        self
    }
    /// Helix radius in points.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.clamp(40.0, 400.0);
        self
    }
    /// Panels per turn.
    pub fn panels_per_turn(mut self, n: usize) -> Self {
        self.per_turn = n.clamp(3, 16);
        self
    }
    /// Track advance per turn relative to the panel width (`turns` of screw), `0.5..=2`.
    pub fn turns(mut self, k: f32) -> Self {
        self.pitch = k.clamp(0.5, 2.0);
        self
    }
    /// Idle spin in panels per second.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Frost on the far side, `0..=1`.
    pub fn frost(mut self, f: f32) -> Self {
        self.frost = f.clamp(0.0, 1.0);
        self
    }
}

impl Widget for SpiralSlider {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::drag());
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Grab);
        }
        let off = drive_loop(ui, response.id, &response, true, self.speed, 1.0 / 120.0);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let cam = Cam { center: rect.center(), focal: 900.0 };
        let n = self.per_turn as f32;
        let dth = TAU / n;
        let r = self.radius;
        let panel_w = r * dth * 0.92 * 1.25; // along the track
        let advance = panel_w * 0.42 * self.pitch; // track advance per panel
        let arc = dth * 0.88; // angular size of a panel
        let tilt = -0.08;
        let bg = t.muted;
        let frost_c = if t.dark { hex("#C8CCDA") } else { hex("#FFFFFF") };
        let base = off.floor() as i32;
        let span = (rect.width() / advance / 2.0).ceil() as i32 + 3;
        let mut panels: Vec<(f32, i32)> = Vec::new();
        for j in -span..=span {
            let i = base + j;
            let s = i as f32 - off;
            let th = s * dth;
            panels.push((th.cos(), i));
        }
        // far side first (cos > 0 is away)
        panels.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        for (_, i) in panels {
            let s = i as f32 - off;
            let th0 = s * dth;
            let xc = s * advance;
            let (tex, own) = scene_tex(ui, &self.textures, i.rem_euclid(64) as usize);
            let uv = if own { full_uv() } else { cover_uv(PaintedScene::ASPECT, panel_w / (r * arc)) };
            let pt3 = |u: f32, v: f32| -> V3 {
                let a = th0 + (v - 0.5) * arc;
                let x = xc + (u - 0.5) * panel_w + (a - th0) / TAU * advance * n;
                v3(x, a.sin() * r, a.cos() * r).rot_x(0.38).rot_z(tilt)
            };
            let c3 = pt3(0.5, 0.5);
            if (cam.pt(c3).x - rect.center().x).abs() > rect.width() / 2.0 + panel_w {
                continue;
            }
            let back = |v: f32| ((th0 + (v - 0.5) * arc).cos()).max(0.0);
            surface_grid(&painter, Some(tex), uv, 6, 8, vec2(panel_w, r * arc), 8.0, |u, v| cam.pt(pt3(u, v)), |u, v| {
                let b = back(v);
                let k = b * self.frost;
                let c = lerp_c(Color32::WHITE, lerp_c(frost_c, bg, 0.3), k * 0.7);
                c.gamma_multiply((1.0 - 0.3 * k) * edge_alpha(rect, cam.pt(pt3(u, v)).x, 80.0))
            });
        }
        response
    }
}

