use super::common::{bold_text, capture, clip_mesh, paint_avatar, pointer_or_auto, subdivide, Persp, VIVID};
use crate::buttons::util::{self, clip_convex, outline};
use crate::motion::common::{polygon_shadow, sd_round_rect};
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, InnerResponse, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
use glowkit_core::anim::{follow, smoothstep, spring, spring2};
use glowkit_core::color::{hsv, mix, shade, with_alpha};
use glowkit_core::{noise, paint, Theme};

/// Additive blend of `add` (weighted by `a`) onto a premultiplied color, keeping its alpha.
fn screen(c: Color32, add: Color32, a: f32) -> Color32 {
    let f = |x: u8, y: u8| {
        let x = x as f32 / 255.0;
        let y = y as f32 / 255.0 * a;
        ((1.0 - (1.0 - x) * (1.0 - y)) * 255.0).round() as u8
    };
    let al = c.a() as f32 / 255.0;
    let r = Color32::from_rgb(f(c.r(), add.r()), f(c.g(), add.g()), f(c.b(), add.b()));
    if c.a() == 255 {
        r
    } else {
        r.gamma_multiply(al)
    }
}

/// Smoothed pointer over `rect` in -1..1 coordinates plus an "active" fade.
fn tracked(ui: &Ui, id: egui::Id, rect: Rect, autoplay: bool, seed: f32) -> (Vec2, f32, Option<(Pos2, bool)>) {
    let ptr = pointer_or_auto(ui, rect, autoplay, seed);
    let target = ptr.map(|(p, _)| ((p - rect.center()) / (rect.size() / 2.0)).clamp(Vec2::splat(-1.0), Vec2::splat(1.0)));
    let rel = spring2(ui, id.with("rel"), target.unwrap_or(Vec2::ZERO), 150.0, 18.0);
    let k = follow(ui, id.with("k"), if ptr.is_some() { 1.0 } else { 0.0 }, 6.0);
    (rel, k, ptr)
}

// ---------------------------------------------------------------------------------------------

/// A tall, holographic trading-card style profile: a portrait over a deep gradient, name and
/// title on top, and a frosted contact bar at the bottom. It tilts in 3D toward the pointer
/// while rainbow foil bands and a glare slide across its surface.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let r = glowkit_effects::ProfileCard::new("Ada Park", "Software Engineer").handle("adapark").show(ui);
/// if r.inner { /* contact clicked */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct ProfileCard {
    name: String,
    title: String,
    handle: String,
    status: String,
    contact: String,
    colors: Option<[Color32; 2]>,
    size: Vec2,
    max_tilt: f32,
    seed: u32,
    autoplay: bool,
}

impl ProfileCard {
    /// A profile card for a person.
    pub fn new(name: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            title: title.into(),
            handle: "glowkit".into(),
            status: "Online".into(),
            contact: "Contact Me".into(),
            colors: None,
            size: vec2(290.0, 400.0),
            max_tilt: 14.0,
            seed: 1,
            autoplay: false,
        }
    }
    /// Handle shown in the bottom bar (without the @).
    pub fn handle(mut self, handle: impl Into<String>) -> Self {
        self.handle = handle.into();
        self
    }
    /// Status line under the handle.
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self
    }
    /// Label of the contact button (empty hides it).
    pub fn contact_label(mut self, label: impl Into<String>) -> Self {
        self.contact = label.into();
        self
    }
    /// Card gradient colors (deep → bright). Default: derived from the theme gradient.
    pub fn colors(mut self, a: Color32, b: Color32) -> Self {
        self.colors = Some([a, b]);
        self
    }
    /// Card size in points.
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size.max(vec2(160.0, 220.0));
        self
    }
    /// Maximum tilt in degrees.
    pub fn max_tilt(mut self, deg: f32) -> Self {
        self.max_tilt = deg;
        self
    }
    /// Picks one of a few generated portrait styles.
    pub fn portrait(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }
    /// Sway and shimmer on its own while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the card. `inner` is true when the contact button was clicked.
    pub fn show(self, ui: &mut Ui) -> InnerResponse<bool> {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(self.size + vec2(40.0, 40.0), Sense::hover());
        let id = resp.id;
        if !ui.is_rect_visible(rect) {
            return InnerResponse::new(false, resp);
        }
        let card = Rect::from_center_size(rect.center(), self.size);
        let (rel, k, _) = tracked(ui, id, card.expand(20.0), self.autoplay, 0.4);
        let time = glowkit_core::time(ui);
        let [c0, c1] = self.colors.unwrap_or([shade(t.gradient[0], -0.55), t.gradient[1]]);
        let max = self.max_tilt.to_radians();
        let mut pr = Persp::new(card.center());
        pr.rx = rel.y * max;
        pr.ry = rel.x * max;
        pr.scale = 1.0 + 0.03 * k;
        pr.distance = 800.0;
        let radius = 26.0;
        let painter = ui.painter();

        // halo + shadow
        let halo_c = card.center() + vec2(rel.x, rel.y) * -10.0;
        paint::glow_ellipse(painter, halo_c, card.size() * 0.75, with_alpha(c1, if t.dark { 0.35 } else { 0.22 }));
        let local_outline = outline(Rect::from_center_size(Pos2::ZERO, card.size()), radius, 10.0);
        let proj: Vec<Pos2> = local_outline.iter().map(|p| pr.apply(p.to_vec2())).collect();
        polygon_shadow(painter, &proj, card.center(), vec2(-rel.x * 8.0, 14.0 - rel.y * 6.0), 18.0, Color32::BLACK.gamma_multiply(if t.dark { 0.8 } else { 0.35 }));

        // the card face, captured so it can be foil-shaded and projected as one surface
        let name = self.name.clone();
        let title = self.title.clone();
        let handle = format!("@{}", self.handle);
        let status = self.status.clone();
        let contact = self.contact.clone();
        let seed = self.seed;
        let bar = Rect::from_min_max(pos2(card.min.x + 14.0, card.max.y - 78.0), pos2(card.max.x - 14.0, card.max.y - 14.0));
        let btn_g = ui.painter().layout_no_wrap(contact.clone(), FontId::proportional(13.0), Color32::WHITE);
        let btn = Rect::from_min_size(
            pos2(bar.max.x - 12.0 - btn_g.size().x - 26.0, bar.center().y - 16.0),
            vec2(btn_g.size().x + 26.0, 32.0),
        );
        let mesh = capture(ui, |p| {
            paint::fill_convex_fn(p, &outline(card, radius, 10.0), 6, |q| {
                let u = ((q.y - card.min.y) / card.height()).clamp(0.0, 1.0);
                let v = ((q.x - card.min.x) / card.width()).clamp(0.0, 1.0);
                let base = mix(c0, mix(c1, c0, 0.35), (1.0 - u) * 0.9);
                mix(base, shade(c0, -0.5), (v * 0.3 + u * 0.4).min(1.0) * 0.5)
            });
            // radial light behind the portrait
            paint::glow(p, pos2(card.center().x, card.min.y + card.height() * 0.55), card.width() * 0.62, with_alpha(c1, 0.55));
            let art = Rect::from_min_max(pos2(card.min.x, card.min.y + card.height() * 0.22), card.max);
            paint_avatar(p, art, 0.0, [mix(c1, Color32::WHITE, 0.2), c0], seed, false);
            bold_text(
                p,
                pos2(card.center().x, card.min.y + 38.0),
                Align2::CENTER_TOP,
                &name,
                FontId::proportional((card.width() * 0.105).clamp(16.0, 34.0)),
                Color32::WHITE,
            );
            p.text(
                pos2(card.center().x, card.min.y + 38.0 + (card.width() * 0.105).clamp(16.0, 34.0) * 1.35),
                Align2::CENTER_TOP,
                &title,
                FontId::proportional(14.5),
                with_alpha(Color32::WHITE, 0.88),
            );
            // frosted bar
            p.rect(bar, util::cr(18.0), with_alpha(Color32::WHITE, 0.12), Stroke::new(1.0, with_alpha(Color32::WHITE, 0.22)), egui::StrokeKind::Inside);
            let av = pos2(bar.min.x + 30.0, bar.center().y);
            paint::gradient_circle(p, av, 17.0, &[mix(c1, Color32::WHITE, 0.3), c0]);
            p.circle_stroke(av, 17.0, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.4)));
            p.text(av, Align2::CENTER_CENTER, name.chars().next().unwrap_or('?').to_string(), FontId::proportional(14.0), Color32::WHITE);
            p.text(pos2(av.x + 26.0, bar.center().y - 9.0), Align2::LEFT_CENTER, &handle, FontId::proportional(13.5), Color32::WHITE);
            p.circle_filled(pos2(av.x + 30.0, bar.center().y + 10.0), 3.0, glowkit_core::hex("#4ADE80"));
            p.text(
                pos2(av.x + 37.0, bar.center().y + 10.0),
                Align2::LEFT_CENTER,
                &status,
                FontId::proportional(12.0),
                with_alpha(Color32::WHITE, 0.7),
            );
            if !contact.is_empty() {
                p.rect(btn, util::cr(16.0), with_alpha(Color32::WHITE, 0.1), Stroke::new(1.0, with_alpha(Color32::WHITE, 0.35)), egui::StrokeKind::Inside);
                p.text(btn.center(), Align2::CENTER_CENTER, &contact, FontId::proportional(13.0), Color32::WHITE);
            }
        });
        let mesh = clip_mesh(&subdivide(&mesh, 12.0, card.expand(1.0)), &outline(card, radius, 6.0));
        let mut mesh = mesh;
        // holographic foil + glare, evaluated in card space before projection
        let glare = card.center() + vec2(rel.x, rel.y) * card.size() * 0.5;
        let shift = rel.x * 0.35 + rel.y * 0.2 + time * 0.03;
        let foil = 0.18 + 0.32 * k;
        for v in &mut mesh.vertices {
            let u = (v.pos - card.min) / card.size();
            let band = ((u.x * 1.4 + u.y * 2.2 + shift * 3.0) * std::f32::consts::PI).sin() * 0.5 + 0.5;
            let hue = (u.x * 0.5 + u.y * 0.9 + shift).rem_euclid(1.0);
            let rainbow = hsv(hue, 0.55, 1.0);
            let mut c = screen(v.color, rainbow, foil * band.powf(2.5));
            let d = v.pos.distance(glare) / (card.width() * 0.8);
            let g = (1.0 - d).clamp(0.0, 1.0);
            c = screen(c, Color32::WHITE, g * g * 0.45 * k);
            v.color = c;
        }
        pr.warp(&mut mesh);
        painter.add(Shape::mesh(mesh));
        // foil rim
        util::ribbon(painter, &proj, 1.6, |q, s| {
            let hue = (s + shift).rem_euclid(1.0);
            let d = q.distance(pr.apply(glare - card.center()));
            let g = (1.0 - d / card.width()).clamp(0.0, 1.0);
            with_alpha(mix(hsv(hue, 0.5, 1.0), Color32::WHITE, 0.4), 0.35 + 0.5 * g * k)
        });
        // contact button
        let mut clicked = false;
        if !self.contact.is_empty() {
            let bp = Rect::from_center_size(pr.apply(btn.center() - card.center()), btn.size());
            let br = ui.interact(bp, id.with("contact"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            clicked = br.clicked();
            if br.hovered() {
                painter.rect_filled(bp, util::cr(16.0), with_alpha(Color32::WHITE, 0.12));
            }
        }
        if k > 0.001 || self.autoplay {
            glowkit_core::animate(ui);
        }
        InnerResponse::new(clicked, resp)
    }
}

// ---------------------------------------------------------------------------------------------

/// One tile of [`GlassIcons`].
#[derive(Clone, Debug, PartialEq)]
pub struct GlassIcon {
    /// Glyph on the glass (emoji or a short string).
    pub icon: String,
    /// Label that appears under the tile on hover.
    pub label: String,
    /// Backplate gradient; `None` picks from a vivid palette.
    pub colors: Option<[Color32; 2]>,
}

impl GlassIcon {
    /// A glass icon with a glyph and a label.
    pub fn new(icon: impl Into<String>, label: impl Into<String>) -> Self {
        Self { icon: icon.into(), label: label.into(), colors: None }
    }
    /// Backplate gradient colors.
    pub fn colors(mut self, a: Color32, b: Color32) -> Self {
        self.colors = Some([a, b]);
        self
    }
}

/// A row of glossy icon tiles: a frosted glass square floats in front of a tilted, vividly
/// colored backplate whose blurred color shows through the glass. On hover the backplate swings
/// further out, the glass lifts toward you and the label fades in underneath.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{GlassIcon, GlassIcons};
/// let r = GlassIcons::new(vec![GlassIcon::new("📁", "Files"), GlassIcon::new("✉", "Mail")]).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct GlassIcons {
    items: Vec<GlassIcon>,
    size: f32,
    autoplay: bool,
}

impl GlassIcons {
    /// A row of glass icons.
    pub fn new(items: Vec<GlassIcon>) -> Self {
        Self { items, size: 72.0, autoplay: false }
    }
    /// Tile size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(24.0);
        self
    }
    /// Hover the tiles one after another while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the row. `inner` is the tile clicked this frame.
    pub fn show(self, ui: &mut Ui) -> InnerResponse<Option<usize>> {
        let t = Theme::of(ui);
        let s = self.size;
        let cell = vec2(s * 1.75, s * 1.95);
        let n = self.items.len();
        let (rect, resp) = ui.allocate_exact_size(vec2(cell.x * n as f32, cell.y), Sense::hover());
        let id = resp.id;
        if !ui.is_rect_visible(rect) {
            return InnerResponse::new(None, resp);
        }
        let painter = ui.painter();
        let time = glowkit_core::time(ui);
        let away = !ui.rect_contains_pointer(rect);
        let auto = if self.autoplay && away && n > 0 {
            glowkit_core::animate(ui);
            let k = (time / 1.3) as usize % (n + 1);
            (k < n).then_some(k)
        } else {
            None
        };
        let mut clicked = None;
        let radius = s * 0.26;
        for (i, item) in self.items.iter().enumerate() {
            let c = pos2(rect.min.x + cell.x * (i as f32 + 0.5), rect.min.y + s * 0.15 + s * 0.6);
            let tile = Rect::from_center_size(c, Vec2::splat(s));
            let r = ui.interact(tile.expand(8.0), id.with(("t", i)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            if r.clicked() {
                clicked = Some(i);
            }
            let h = spring(ui, id.with(("h", i)), if r.hovered() || auto == Some(i) { 1.0 } else { 0.0 }, 220.0, 20.0);
            let [a, b] = item.colors.unwrap_or(VIVID[(i * 3) % VIVID.len()]);
            // backplate
            let ang = (15.0 + 10.0 * h).to_radians();
            let bc = c + vec2(-0.08, -0.1) * s * h;
            let (sn, cs) = ang.sin_cos();
            let rot = |p: Vec2| bc + vec2(p.x * cs - p.y * sn, p.x * sn + p.y * cs);
            let back: Vec<Pos2> = outline(Rect::from_center_size(Pos2::ZERO, Vec2::splat(s)), radius, 6.0).into_iter().map(|p| rot(p.to_vec2())).collect();
            polygon_shadow(painter, &back, bc, vec2(0.0, 6.0), 8.0, with_alpha(b, if t.dark { 0.5 } else { 0.35 }));
            paint::fill_convex_fn(painter, &back, 3, paint::linear_fn(Rect::from_center_size(bc, Vec2::splat(s)), 45.0, &[a, b]));
            // glass, lifted toward the viewer on hover
            let lift = 1.0 + 0.06 * h;
            let gc = c + vec2(0.06, 0.04) * s - vec2(0.0, 4.0) * h;
            let glass = Rect::from_center_size(gc, Vec2::splat(s * lift));
            let back_rect = Rect::from_center_size(Pos2::ZERO, Vec2::splat(s));
            let blur = s * 0.16;
            crate::cards::shell::shade_rounded(painter, glass, radius * lift, s / 14.0, |q| {
                // blurred coverage of the backplate behind the glass
                let d = q - bc;
                let l = pos2(d.x * cs + d.y * sn, -d.x * sn + d.y * cs);
                let sd = sd_round_rect(l, back_rect, radius);
                let cov = smoothstep(blur, -blur, sd);
                let u = ((l.x + l.y) / (2.0 * s) + 0.5).clamp(0.0, 1.0);
                let col = mix(a, b, u);
                let tint = mix(Color32::WHITE, mix(col, Color32::WHITE, 0.25), cov);
                let gq = (q - glass.min) / glass.size();
                let sheen = (1.0 - (gq.x + gq.y) * 0.8).clamp(0.0, 1.0) * 0.25;
                let base_a = if t.dark { 0.1 } else { 0.35 };
                with_alpha(mix(tint, Color32::WHITE, sheen), base_a + cov * 0.6 + sheen * 0.3)
            });
            util::inner_border_fn(painter, glass, radius * lift, 1.2, |q, _| {
                let gq = (q - glass.min) / glass.size();
                with_alpha(Color32::WHITE, 0.25 + 0.5 * (1.0 - (gq.x + gq.y) * 0.5).clamp(0.0, 1.0))
            });
            if !t.dark {
                painter.rect_stroke(glass, util::cr(radius * lift), Stroke::new(1.0, with_alpha(b, 0.18)), egui::StrokeKind::Outside);
            }
            painter.text(
                gc + vec2(0.0, 1.0),
                Align2::CENTER_CENTER,
                &item.icon,
                FontId::proportional(s * 0.42 * lift),
                with_alpha(Color32::BLACK, 0.12),
            );
            painter.text(gc, Align2::CENTER_CENTER, &item.icon, FontId::proportional(s * 0.42 * lift), Color32::WHITE);
            // label
            let la = h.clamp(0.0, 1.0);
            if la > 0.01 {
                painter.text(
                    pos2(c.x, tile.max.y + s * 0.2 + (1.0 - la) * 8.0),
                    Align2::CENTER_TOP,
                    &item.label,
                    FontId::proportional((s * 0.2).clamp(11.0, 16.0)),
                    with_alpha(t.foreground, la),
                );
            }
        }
        InnerResponse::new(clicked, resp)
    }
}

// ---------------------------------------------------------------------------------------------

/// A picture card that decays on hover: its content ripples and tears with fractal turbulence
/// proportional to how fast the pointer moves, then slowly heals. The card also drifts and
/// leans after the pointer. You supply the picture as a painter closure.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_effects::DecayCard::new().title("Decay").show(ui, |painter, rect| {
///     glowkit_core::paint::gradient_rect(painter, rect, 0.0, 45.0, &[glowkit_core::hex("#F97316"), glowkit_core::hex("#7C3AED")]);
/// });
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct DecayCard {
    size: Vec2,
    title: String,
    intensity: f32,
    radius: f32,
    autoplay: bool,
}

impl Default for DecayCard {
    fn default() -> Self {
        Self::new()
    }
}

impl DecayCard {
    /// A decay card.
    pub fn new() -> Self {
        Self { size: vec2(260.0, 330.0), title: String::new(), intensity: 1.0, radius: 6.0, autoplay: false }
    }
    /// Card size.
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size.max(vec2(60.0, 60.0));
        self
    }
    /// Big caption over the bottom of the card (empty hides it).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
    /// Strength of the turbulence (1 = default).
    pub fn intensity(mut self, k: f32) -> Self {
        self.intensity = k.max(0.0);
        self
    }
    /// Corner radius.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.max(0.0);
        self
    }
    /// Stir itself up every few seconds while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the card; `art` paints the picture into the given rect.
    pub fn show(self, ui: &mut Ui, art: impl FnOnce(&egui::Painter, Rect)) -> Response {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(self.size + vec2(30.0, 30.0), Sense::hover());
        let id = resp.id;
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let card0 = Rect::from_center_size(rect.center(), self.size);
        let dt = glowkit_core::dt(ui);
        let time = glowkit_core::time(ui);
        let (rel, k, ptr) = tracked(ui, id, rect, false, 0.0);
        let real = ptr.is_some_and(|p| p.1);
        let speed = if real { ui.input(|i| i.pointer.velocity().length()) } else { 0.0 };
        let auto = self.autoplay && !real;
        let energy = glowkit_core::with_state(ui, id.with("energy"), || 0.0f32, |e| {
            *e += speed * dt * 0.0035;
            if auto {
                let ph = (time / 3.6).fract();
                if ph < 0.35 {
                    *e += dt * 2.6;
                }
            }
            *e = (*e * (-dt * 1.4).exp()).clamp(0.0, 1.2);
            *e
        });
        if energy > 0.002 || auto || k > 0.001 {
            glowkit_core::animate(ui);
        }
        let center = card0.center() + vec2(rel.x, rel.y) * 10.0 * k;
        let angle = rel.x * 0.035 * k;
        let card = Rect::from_center_size(center, self.size);
        let painter = ui.painter();
        paint::soft_shadow(
            painter,
            card,
            self.radius,
            26.0,
            vec2(0.0, 10.0),
            Color32::BLACK.gamma_multiply(if t.dark { 0.6 } else { 0.22 }),
        );
        let mesh = capture(ui, |p| art(p, card));
        let mut mesh = subdivide(&mesh, 7.0, card.expand(2.0));
        let amp = 46.0 * self.intensity * energy;
        let z = time * 0.7;
        let (sn, cs) = angle.sin_cos();
        for v in &mut mesh.vertices {
            let q = v.pos;
            let mut d = Vec2::ZERO;
            if amp > 0.05 {
                let nx = noise::fbm3(q.x * 0.018, q.y * 0.018, z, 3);
                let ny = noise::fbm3(q.x * 0.018 + 31.7, q.y * 0.018 - 12.3, z, 3);
                d = vec2(nx, ny) * amp;
            }
            let l = q + d - center;
            v.pos = center + vec2(l.x * cs - l.y * sn, l.x * sn + l.y * cs);
        }
        let rotated: Vec<Pos2> = outline(card, self.radius, 8.0)
            .into_iter()
            .map(|p| {
                let l = p - center;
                center + vec2(l.x * cs - l.y * sn, l.x * sn + l.y * cs)
            })
            .collect();
        let mesh = clip_mesh(&mesh, &rotated);
        painter.add(Shape::mesh(mesh));
        if !self.title.is_empty() {
            let size = (self.size.x * 0.2).clamp(18.0, 64.0);
            let font = FontId::proportional(size);
            let w = painter.layout_no_wrap(self.title.clone(), font.clone(), Color32::WHITE).size().x;
            let local = vec2(-self.size.x / 2.0 + 18.0 + w / 2.0, self.size.y / 2.0 - 14.0 - size * 0.6);
            let at = center + vec2(local.x * cs - local.y * sn, local.x * sn + local.y * cs);
            let tp = |dx: f32, dy: f32, c: Color32| {
                glowkit_core::text::paint_rotated(painter, &self.title, at + vec2(dx, dy), font.clone(), c, angle);
            };
            tp(1.5, 2.5, with_alpha(Color32::BLACK, 0.35));
            tp(0.0, 0.0, Color32::WHITE);
            tp(size / 30.0, 0.0, Color32::WHITE);
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------

/// A die-cut sticker you can peel: drag it and the corner lifts along a fold line, showing the
/// paper-white back of the sticker with a soft curl shadow. Let go and it sticks back down.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::StickerPeel::new("🦀", "Rust"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct StickerPeel {
    icon: String,
    text: String,
    size: Vec2,
    colors: Option<[Color32; 2]>,
    back: Option<Color32>,
    radius: f32,
    rest_peel: f32,
    autoplay: bool,
}

impl StickerPeel {
    /// A sticker with a glyph and a short caption.
    pub fn new(icon: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            text: text.into(),
            size: vec2(180.0, 180.0),
            colors: None,
            back: None,
            radius: 36.0,
            rest_peel: 16.0,
            autoplay: false,
        }
    }
    /// Sticker size.
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size.max(vec2(48.0, 48.0));
        self
    }
    /// Sticker gradient colors. Default: the theme gradient.
    pub fn colors(mut self, a: Color32, b: Color32) -> Self {
        self.colors = Some([a, b]);
        self
    }
    /// Color of the sticker's back side.
    pub fn back_color(mut self, c: Color32) -> Self {
        self.back = Some(c);
        self
    }
    /// Corner radius.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.max(0.0);
        self
    }
    /// How far (points) the corner stays lifted at rest, as a hint.
    pub fn rest_peel(mut self, p: f32) -> Self {
        self.rest_peel = p.max(0.0);
        self
    }
    /// Peel and re-stick on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for StickerPeel {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(self.size + vec2(60.0, 60.0), Sense::drag());
        let id = resp.id;
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let dragging_now = resp.dragged();
        let resp = resp.on_hover_cursor(if dragging_now { CursorIcon::Grabbing } else { CursorIcon::Grab });
        let s = Rect::from_center_size(rect.center(), self.size);
        let rr = self.radius.min(self.size.min_elem() / 2.0);
        let corner = s.max - Vec2::splat(rr) + vec2(1.0, 1.0).normalized() * rr;
        let diag = (s.min - s.max).normalized();
        let rest = corner + diag * self.rest_peel;
        let now = glowkit_core::time(ui);
        let drag_origin: Option<Pos2> = glowkit_core::state(ui, id.with("origin"), || None);
        if resp.drag_started() {
            glowkit_core::set_state(ui, id.with("origin"), resp.interact_pointer_pos());
            glowkit_core::set_state(ui, id.with("last_user"), now);
        }
        let last_user: f32 = glowkit_core::state(ui, id.with("last_user"), || -100.0);
        let target = if resp.dragged() {
            let p = resp.interact_pointer_pos().unwrap_or(corner);
            let o = drag_origin.unwrap_or(corner);
            let v = (p - o) + (rest - corner);
            let max = self.size.length() * 0.9;
            corner + if v.length() > max { v.normalized() * max } else { v }
        } else if self.autoplay && now - last_user > 3.0 {
            glowkit_core::animate(ui);
            let ph = (now / 4.2).fract();
            let e = smoothstep(0.0, 0.45, ph) * (1.0 - smoothstep(0.62, 0.95, ph));
            let swing = vec2(0.0, 0.25) * (now * 0.9).sin();
            corner + (diag + swing).normalized() * (self.rest_peel + e * self.size.min_elem() * 0.72)
        } else {
            rest
        };
        let p = corner + spring2(ui, id.with("peel"), target - corner, if resp.dragged() { 400.0 } else { 160.0 }, if resp.dragged() { 32.0 } else { 14.0 });

        let painter = ui.painter();
        let [a, b] = self.colors.unwrap_or([t.gradient[0], t.gradient[1]]);
        let back = self.back.unwrap_or(if t.dark { hsv(0.0, 0.0, 0.92) } else { hsv(0.0, 0.0, 0.97) });
        let shape = outline(s, self.radius, 6.0);
        let v = corner - p;
        let len = v.length();
        let (stay, flap, n, m) = if len > 0.5 {
            let n = v / len;
            let m = p + v / 2.0;
            let u = n.rot90();
            let big = 4000.0;
            let keep = [m + u * big, m - u * big, m - u * big - n * big, m + u * big - n * big];
            let lift = [m + u * big, m + u * big + n * big, m - u * big + n * big, m - u * big];
            (clip_convex(&shape, &keep), clip_convex(&shape, &lift), n, m)
        } else {
            (shape.clone(), Vec::new(), vec2(1.0, 0.0), corner)
        };
        // shadow under the stuck part
        polygon_shadow(painter, &stay, s.center(), vec2(0.0, 3.0), 3.0, Color32::BLACK.gamma_multiply(if t.dark { 0.6 } else { 0.25 }));
        // front artwork, clipped to what is still stuck down
        let icon = self.icon.clone();
        let text = self.text.clone();
        let radius = self.radius;
        let mesh = capture(ui, |pp| {
            pp.rect_filled(s, util::cr(radius), Color32::WHITE);
            let inner = s.shrink(s.width().min(s.height()) * 0.06);
            let ir = (radius - s.width() * 0.06).max(2.0);
            paint::gradient_rect(pp, inner, ir, 135.0, &[a, b]);
            paint::glow(pp, inner.left_top() + inner.size() * vec2(0.3, 0.25), inner.width() * 0.6, with_alpha(Color32::WHITE, 0.25));
            let has_text = !text.is_empty();
            let ic = inner.center() - vec2(0.0, if has_text { inner.height() * 0.1 } else { 0.0 });
            pp.text(ic, Align2::CENTER_CENTER, &icon, FontId::proportional(inner.height() * 0.4), Color32::WHITE);
            if has_text {
                bold_text(
                    pp,
                    pos2(inner.center().x, inner.max.y - inner.height() * 0.2),
                    Align2::CENTER_CENTER,
                    &text,
                    FontId::proportional((inner.height() * 0.13).clamp(10.0, 30.0)),
                    Color32::WHITE,
                );
            }
        });
        if stay.len() >= 3 {
            painter.add(Shape::mesh(clip_mesh(&mesh, &stay)));
        }
        if flap.len() >= 3 {
            let reflect = |q: Pos2| q - 2.0 * (q - m).dot(n) * n;
            let flipped: Vec<Pos2> = flap.iter().map(|q| reflect(*q)).collect();
            // curl shadow cast on the sticker beside the fold
            let reach = (len * 0.5).clamp(6.0, 30.0);
            if stay.len() >= 3 {
                let strip = [m + n.rot90() * 4000.0, m - n.rot90() * 4000.0, m - n.rot90() * 4000.0 - n * reach, m + n.rot90() * 4000.0 - n * reach];
                let sp = clip_convex(&stay, &strip);
                if sp.len() >= 3 {
                    paint::fill_convex_fn(painter, &sp, 3, |q| {
                        let d = (m - q).dot(n) / reach;
                        Color32::BLACK.gamma_multiply(0.28 * (1.0 - d).clamp(0.0, 1.0).powi(2))
                    });
                }
            }
            // flap shadow
            let fc = flipped.iter().fold(Vec2::ZERO, |acc, q| acc + q.to_vec2()) / flipped.len() as f32;
            polygon_shadow(painter, &flipped, fc.to_pos2(), -n * 5.0 + vec2(0.0, 3.0), 6.0, Color32::BLACK.gamma_multiply(0.45));
            let half = (len / 2.0).max(1.0);
            paint::fill_convex_fn(painter, &flipped, 4, |q| {
                let d = ((q - m).dot(-n) / half).clamp(0.0, 1.0);
                let curl = smoothstep(0.0, 0.35, d);
                mix(shade(back, -0.22), mix(back, Color32::WHITE, 0.6), curl * 0.85 + (1.0 - d) * 0.0)
            });
            // crease highlight
            let mut along: Vec<Pos2> = flipped.iter().copied().filter(|q| (*q - m).dot(n).abs() < 0.8).collect();
            if along.len() >= 2 {
                along.sort_by(|x, y| (*x - m).dot(n.rot90()).total_cmp(&(*y - m).dot(n.rot90())));
                painter.line_segment([along[0], *along.last().unwrap()], Stroke::new(1.2, with_alpha(Color32::WHITE, 0.8)));
            }
            let mut rim = flipped.clone();
            rim.push(rim[0]);
            painter.add(Shape::line(rim, Stroke::new(0.8, with_alpha(Color32::BLACK, 0.12))));
        }
        resp
    }
}
