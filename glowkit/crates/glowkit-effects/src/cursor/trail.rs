use super::area_input;
use crate::motion::common::{paint_card_face, sample_faces, tapered_ribbon, CardFace, CardPose};
use egui::{vec2, Color32, CornerRadius, Pos2, Rect, Response, Sense, Ui, Vec2, Widget};
use glowkit_core::anim::{ease, Easing};
use glowkit_core::color::{gradient, hsv, with_alpha};
use glowkit_core::{paint, Theme};

macro_rules! area_setters {
    () => {
        /// Simulate a pointer moving along a smooth path while the real pointer is elsewhere.
        pub fn autoplay(mut self, on: bool) -> Self {
            self.autoplay = on;
            self
        }
        /// Size of the area allocated by `ui.add(...)` (defaults to full width x 300).
        pub fn desired_size(mut self, size: Vec2) -> Self {
            self.area = Some(size);
            self
        }
        /// Distinguish several instances inside the same parent `Ui`.
        pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
            self.salt = egui::epaint::util::hash(salt);
            self
        }
    };
}
pub(crate) use area_setters;

macro_rules! area_widget {
    ($t:ty) => {
        impl Widget for $t {
            fn ui(self, ui: &mut Ui) -> Response {
                let size = glowkit_core::desired_size(ui, self.area, 300.0);
                let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                self.show(ui, rect);
                resp
            }
        }
    };
}
pub(crate) use area_widget;

// ---------------------------------------------------------------------------------------------

/// A glowing comet tail that follows the pointer and shrinks away when it stops.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let rect = ui.max_rect();
/// glowkit_effects::PointerTrail::new().width(10.0).lifetime(0.5).show(ui, rect);
/// ```
#[must_use = "call `.show(ui, rect)` or add it with `ui.add(...)`"]
pub struct PointerTrail {
    colors: Option<Vec<Color32>>,
    width: f32,
    lifetime: f32,
    glow: bool,
    autoplay: bool,
    area: Option<Vec2>,
    salt: u64,
}

impl Default for PointerTrail {
    fn default() -> Self {
        Self::new()
    }
}

impl PointerTrail {
    /// A 12 pt wide trail in the theme gradient.
    pub fn new() -> Self {
        Self { colors: None, width: 12.0, lifetime: 0.45, glow: true, autoplay: false, area: None, salt: 0 }
    }
    /// Gradient from head to tail. Defaults to the theme gradient.
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }
    /// Width of the trail at the head.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(1.0);
        self
    }
    /// How long (seconds) a point of the trail lives.
    pub fn lifetime(mut self, s: f32) -> Self {
        self.lifetime = s.clamp(0.05, 5.0);
        self
    }
    /// Paint a soft glow at the head.
    pub fn glow(mut self, on: bool) -> Self {
        self.glow = on;
        self
    }
    area_setters!();

    /// Paint the effect over `rect`.
    pub fn show(self, ui: &mut Ui, rect: Rect) {
        let id = ui.scope_id().with(("pointer_trail", self.salt));
        let input = area_input(ui, rect, id, self.autoplay, 0.0);
        let now = glowkit_core::time(ui);
        let pts: Vec<(Pos2, f32)> = glowkit_core::with_state(ui, id.with("pts"), Vec::<(Pos2, f32)>::new, |v| {
            if let Some(p) = input.pos {
                if v.last().map(|l| l.0.distance(p) > 0.5).unwrap_or(true) {
                    v.push((p, now));
                } else if let Some(l) = v.last_mut() {
                    l.1 = now;
                }
            }
            v.retain(|(_, t)| now - *t < self.lifetime);
            if v.len() > 256 {
                let e = v.len() - 256;
                v.drain(0..e);
            }
            v.clone()
        });
        if pts.len() < 2 || !ui.is_rect_visible(rect) {
            if !pts.is_empty() {
                glowkit_core::animate(ui);
            }
            return;
        }
        let t = Theme::of(ui);
        let colors = self.colors.clone().unwrap_or_else(|| t.gradient.to_vec());
        let painter = ui.painter_at(rect);
        let head_first: Vec<Pos2> = pts.iter().rev().map(|p| p.0).collect();
        let smooth = paint::catmull_rom(&head_first, 4);
        let ages: Vec<f32> = pts.iter().rev().map(|p| ((now - p.1) / self.lifetime).clamp(0.0, 1.0)).collect();
        let age_at = |u: f32| {
            let f = u * (ages.len() - 1) as f32;
            let i = f.floor() as usize;
            let j = (i + 1).min(ages.len() - 1);
            ages[i] + (ages[j] - ages[i]) * f.fract()
        };
        let w = self.width;
        tapered_ribbon(
            &painter,
            &smooth,
            |u| w * (1.0 - age_at(u)).powf(0.7) * (1.0 - u * 0.6),
            |u| {
                let a = (1.0 - age_at(u)).powf(1.3);
                with_alpha(gradient(&colors, u), a)
            },
        );
        if self.glow {
            let head = head_first[0];
            let fresh = 1.0 - ages[0];
            paint::glow(&painter, head, w * 2.6, with_alpha(colors[0], 0.45 * fresh));
            painter.circle_filled(head, w * 0.42 * fresh, with_alpha(Color32::WHITE, 0.9 * fresh));
        }
        glowkit_core::animate(ui);
    }
}
area_widget!(PointerTrail);

// ---------------------------------------------------------------------------------------------

#[derive(Clone)]
struct Ribbon {
    pts: Vec<Pos2>,
    vel: Vec2,
}

/// Several silky ribbons that chase the pointer with springy lag, each with its own stiffness,
/// weaving around one another.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let rect = ui.max_rect();
/// glowkit_effects::RibbonTrail::new().thickness(18.0).show(ui, rect);
/// ```
#[must_use = "call `.show(ui, rect)` or add it with `ui.add(...)`"]
pub struct RibbonTrail {
    colors: Option<Vec<Color32>>,
    thickness: f32,
    length: usize,
    friction: f32,
    spring: f32,
    offset: f32,
    autoplay: bool,
    area: Option<Vec2>,
    salt: u64,
}

impl Default for RibbonTrail {
    fn default() -> Self {
        Self::new()
    }
}

impl RibbonTrail {
    /// Three ribbons in the theme gradient.
    pub fn new() -> Self {
        Self {
            colors: None,
            thickness: 16.0,
            length: 40,
            friction: 0.5,
            spring: 0.9,
            offset: 10.0,
            autoplay: false,
            area: None,
            salt: 0,
        }
    }
    /// One ribbon per color. Defaults to the theme gradient (3 ribbons).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }
    /// Ribbon thickness at the head.
    pub fn thickness(mut self, t: f32) -> Self {
        self.thickness = t.max(1.0);
        self
    }
    /// Number of points per ribbon (longer = longer tails).
    pub fn length(mut self, n: usize) -> Self {
        self.length = n.clamp(4, 200);
        self
    }
    /// How tightly each point follows the previous one (0..1).
    pub fn friction(mut self, f: f32) -> Self {
        self.friction = f.clamp(0.05, 0.95);
        self
    }
    /// Spring strength of the ribbon heads (0.2 = lazy, 2 = snappy).
    pub fn spring(mut self, s: f32) -> Self {
        self.spring = s.clamp(0.05, 5.0);
        self
    }
    /// Spread the ribbon heads apart by this many points.
    pub fn offset(mut self, o: f32) -> Self {
        self.offset = o;
        self
    }
    area_setters!();

    /// Paint the effect over `rect`.
    pub fn show(self, ui: &mut Ui, rect: Rect) {
        let id = ui.scope_id().with(("ribbon_trail", self.salt));
        let input = area_input(ui, rect, id, self.autoplay, 0.0);
        let t = Theme::of(ui);
        let colors = self.colors.clone().unwrap_or_else(|| t.gradient.to_vec());
        let n = colors.len().max(1);
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let target = input.pos;
        let len = self.length;
        let start = rect.center();
        let (ribbons, moving) = glowkit_core::with_state(
            ui,
            id.with("ribbons"),
            || (Vec::<Ribbon>::new(), None::<Pos2>),
            |(rs, last)| {
                if rs.len() != n || rs.first().map(|r| r.pts.len() != len).unwrap_or(true) {
                    let p0 = target.unwrap_or(start);
                    *rs = (0..n).map(|_| Ribbon { pts: vec![p0; len], vel: Vec2::ZERO }).collect();
                }
                if target.is_some() {
                    *last = target;
                }
                let goal = last.unwrap_or(start);
                let mut moving = false;
                for (k, r) in rs.iter_mut().enumerate() {
                    let fk = k as f32 / n as f32;
                    let ang = now * 1.3 + fk * std::f32::consts::TAU;
                    let off = vec2(ang.cos(), ang.sin()) * self.offset;
                    // each ribbon gets its own stiffness so they separate while moving
                    let stiff = 160.0 * self.spring * (1.0 - fk * 0.55);
                    let damp = 2.0 * stiff.sqrt() * 0.75;
                    let to = goal + off - r.pts[0];
                    r.vel += (to * stiff - r.vel * damp) * dt;
                    r.pts[0] += r.vel * dt;
                    let f = 1.0 - (1.0 - self.friction).powf(dt * 60.0);
                    for i in 1..r.pts.len() {
                        let prev = r.pts[i - 1];
                        r.pts[i] = r.pts[i].lerp(prev, f);
                    }
                    let spread = r.pts[0].distance(*r.pts.last().unwrap_or(&r.pts[0]));
                    moving |= spread > 0.5 || r.vel.length() > 0.5;
                }
                (rs.clone(), moving)
            },
        );
        if moving || target.is_some() {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return;
        }
        let painter = ui.painter_at(rect);
        for (k, r) in ribbons.iter().enumerate().rev() {
            let c = colors[k % colors.len()];
            let smooth = paint::catmull_rom(&r.pts, 3);
            let th = self.thickness * (1.0 - k as f32 * 0.12);
            tapered_ribbon(&painter, &smooth, |u| th * (1.0 - u).powf(0.6), |u| with_alpha(c, 0.95 * (1.0 - u * 0.7)));
            painter.circle_filled(r.pts[0], th * 0.5, c);
        }
    }
}
area_widget!(RibbonTrail);

// ---------------------------------------------------------------------------------------------

/// The area turns into an invisible grid; cells light up under the pointer and fade out,
/// leaving a pixelated trail.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let rect = ui.max_rect();
/// glowkit_effects::PixelTrail::new().cell(20.0).fade(0.6).show(ui, rect);
/// ```
#[must_use = "call `.show(ui, rect)` or add it with `ui.add(...)`"]
pub struct PixelTrail {
    color: Option<Color32>,
    rainbow: bool,
    cell: f32,
    fade: f32,
    rounding: f32,
    brush: i32,
    autoplay: bool,
    area: Option<Vec2>,
    salt: u64,
}

impl Default for PixelTrail {
    fn default() -> Self {
        Self::new()
    }
}

impl PixelTrail {
    /// 24 pt accent-colored pixels that fade in half a second.
    pub fn new() -> Self {
        Self {
            color: None,
            rainbow: false,
            cell: 24.0,
            fade: 0.55,
            rounding: 0.0,
            brush: 0,
            autoplay: false,
            area: None,
            salt: 0,
        }
    }
    /// Pixel color. Defaults to the theme accent.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Cycle hues over time instead of using one color.
    pub fn rainbow(mut self, on: bool) -> Self {
        self.rainbow = on;
        self
    }
    /// Size of a grid cell in points.
    pub fn cell(mut self, size: f32) -> Self {
        self.cell = size.max(4.0);
        self
    }
    /// Seconds for a lit cell to fade out.
    pub fn fade(mut self, s: f32) -> Self {
        self.fade = s.clamp(0.05, 5.0);
        self
    }
    /// Corner rounding of each pixel as a fraction of its size (0.5 = dots).
    pub fn rounding(mut self, r: f32) -> Self {
        self.rounding = r.clamp(0.0, 0.5);
        self
    }
    /// Brush radius in cells (0 = a single cell).
    pub fn brush(mut self, r: i32) -> Self {
        self.brush = r.clamp(0, 4);
        self
    }
    area_setters!();

    /// Paint the effect over `rect`.
    pub fn show(self, ui: &mut Ui, rect: Rect) {
        let id = ui.scope_id().with(("pixel_trail", self.salt));
        let input = area_input(ui, rect, id, self.autoplay, 0.0);
        let now = glowkit_core::time(ui);
        let cell = self.cell;
        let brush = self.brush;
        let cells: Vec<(i32, i32, f32)> = glowkit_core::with_state(
            ui,
            id.with("cells"),
            || (Vec::<(i32, i32, f32)>::new(), None::<Pos2>),
            |(cells, last)| {
                if let Some(p) = input.pos {
                    let from = last.unwrap_or(p);
                    let steps = ((from.distance(p) / (cell * 0.4)).ceil() as usize).clamp(1, 64);
                    for s in 0..=steps {
                        let q = from.lerp(p, s as f32 / steps as f32) - rect.min;
                        let (ci, cj) = ((q.x / cell).floor() as i32, (q.y / cell).floor() as i32);
                        for di in -brush..=brush {
                            for dj in -brush..=brush {
                                if di * di + dj * dj > brush * brush {
                                    continue;
                                }
                                let (i, j) = (ci + di, cj + dj);
                                if let Some(c) = cells.iter_mut().find(|c| c.0 == i && c.1 == j) {
                                    c.2 = now;
                                } else {
                                    cells.push((i, j, now));
                                }
                            }
                        }
                    }
                }
                *last = input.pos;
                cells.retain(|c| now - c.2 < self.fade);
                cells.clone()
            },
        );
        if cells.is_empty() {
            return;
        }
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return;
        }
        let base = self.color.unwrap_or_else(|| Theme::of(ui).accent);
        let painter = ui.painter_at(rect);
        for (i, j, t0) in cells {
            let age = ((now - t0) / self.fade).clamp(0.0, 1.0);
            let a = 1.0 - ease(Easing::InQuad, age);
            let col =
                if self.rainbow { hsv((t0 * 120.0 + (i + j) as f32 * 6.0).rem_euclid(360.0), 0.75, 1.0) } else { base };
            let r = Rect::from_min_size(rect.min + vec2(i as f32 * cell, j as f32 * cell), Vec2::splat(cell));
            let shrink = cell * 0.5 * ease(Easing::InCubic, age) * 0.6;
            let r = r.shrink(shrink);
            let cr = CornerRadius::same((r.width() * self.rounding).round().clamp(0.0, 255.0) as u8);
            painter.rect_filled(r, cr, with_alpha(col, a));
        }
    }
}
area_widget!(PixelTrail);

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct TrailCard {
    pos: Pos2,
    t0: f32,
    face: usize,
    angle: f32,
    drift: Vec2,
}

/// Moving the pointer drops a trail of cards that pop in, tilt, and then shrink away: the
/// "image trail" effect, with painted cards instead of photos.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let rect = ui.max_rect();
/// glowkit_effects::CardTrail::new().spacing(70.0).show(ui, rect);
/// ```
#[must_use = "call `.show(ui, rect)` or add it with `ui.add(...)`"]
pub struct CardTrail {
    faces: Vec<CardFace>,
    card_size: Vec2,
    spacing: f32,
    lifetime: f32,
    autoplay: bool,
    area: Option<Vec2>,
    salt: u64,
}

impl Default for CardTrail {
    fn default() -> Self {
        Self::new()
    }
}

impl CardTrail {
    /// Cards from a built-in colorful set.
    pub fn new() -> Self {
        Self {
            faces: sample_faces(),
            card_size: vec2(96.0, 124.0),
            spacing: 64.0,
            lifetime: 1.1,
            autoplay: false,
            area: None,
            salt: 0,
        }
    }
    /// The faces to cycle through.
    pub fn faces(mut self, faces: Vec<CardFace>) -> Self {
        if !faces.is_empty() {
            self.faces = faces;
        }
        self
    }
    /// Card size in points.
    pub fn card_size(mut self, size: Vec2) -> Self {
        self.card_size = size;
        self
    }
    /// Pointer travel (points) between two dropped cards.
    pub fn spacing(mut self, s: f32) -> Self {
        self.spacing = s.max(8.0);
        self
    }
    /// Seconds each card stays before it is gone.
    pub fn lifetime(mut self, s: f32) -> Self {
        self.lifetime = s.clamp(0.3, 5.0);
        self
    }
    area_setters!();

    /// Paint the effect over `rect`.
    pub fn show(self, ui: &mut Ui, rect: Rect) {
        let id = ui.scope_id().with(("card_trail", self.salt));
        let input = area_input(ui, rect, id, self.autoplay, 0.0);
        let now = glowkit_core::time(ui);
        let nfaces = self.faces.len();
        let cards: Vec<TrailCard> = glowkit_core::with_state(
            ui,
            id.with("cards"),
            || (Vec::<TrailCard>::new(), None::<Pos2>, 0usize),
            |(cards, last, counter)| {
                if let Some(p) = input.pos {
                    match *last {
                        None => *last = Some(p),
                        Some(l) if l.distance(p) >= self.spacing => {
                            *counter += 1;
                            let mut rng = glowkit_core::Rng::new(*counter as u64 * 7919);
                            let dir = (p - l).normalized();
                            cards.push(TrailCard {
                                pos: p,
                                t0: now,
                                face: *counter % nfaces,
                                angle: rng.range(-0.28, 0.28),
                                drift: dir * 18.0 + vec2(0.0, 26.0),
                            });
                            *last = Some(p);
                        }
                        _ => {}
                    }
                } else {
                    *last = None;
                }
                cards.retain(|c| now - c.t0 < self.lifetime);
                if cards.len() > 24 {
                    let e = cards.len() - 24;
                    cards.drain(0..e);
                }
                cards.clone()
            },
        );
        if cards.is_empty() {
            return;
        }
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return;
        }
        let t = Theme::of(ui);
        let painter = ui.painter_at(rect);
        for c in &cards {
            let age = (now - c.t0) / self.lifetime;
            let pop = ease(Easing::OutBack, (age / 0.22).min(1.0));
            let out = ease(Easing::InCubic, ((age - 0.45) / 0.55).clamp(0.0, 1.0));
            let mut pose = CardPose::new(c.pos + c.drift * ease(Easing::OutCubic, age), self.card_size);
            pose.scale = (0.4 + 0.6 * pop) * (1.0 - 0.55 * out);
            pose.alpha = (age / 0.08).min(1.0) * (1.0 - out);
            pose.angle = c.angle * (0.4 + 0.6 * pop);
            pose.shadow = 0.8;
            paint_card_face(&painter, &t, &self.faces[c.face % nfaces], c.face, pose);
        }
    }
}
area_widget!(CardTrail);
