//! HUD widgets: minimap, ammo counter, ability cooldown icons and crosshairs.

use super::common::*;
use egui::epaint::Mesh;
use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};
use glowkit_core::anim::spring;
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::paint;
use std::f32::consts::{PI, TAU};

// ---------------------------------------------------------------------------------------------
// Minimap

/// What a [`GameMapMarker`] shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameMarkerKind {
    /// Hostile: pulsing red dot.
    Enemy,
    /// Friendly: green dot.
    Ally,
    /// Quest objective: bobbing gold diamond (pinned to the rim when out of range).
    Objective,
    /// Loot: purple star.
    Loot,
    /// A ping: rings expanding from the spot for a few seconds after `age` 0.
    Ping,
}

/// A point of interest on a [`GameMinimap`], in world units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GameMapMarker {
    /// World position (x right, y down / south).
    pub pos: Vec2,
    /// What it is.
    pub kind: GameMarkerKind,
    /// Seconds since it was placed (drives ping rings).
    pub age: f32,
}

impl GameMapMarker {
    /// A marker at `pos`.
    pub fn new(kind: GameMarkerKind, pos: Vec2) -> Self {
        Self { pos, kind, age: 0.0 }
    }
    /// Seconds since placement.
    pub fn age(mut self, a: f32) -> Self {
        self.age = a;
        self
    }
}

/// A round or square minimap. The terrain is generated procedurally from `seed` (so it works
/// before your real map texture exists), with fog of war that only clears around the explored
/// breadcrumb trail you pass in. The player arrow sits in the middle with a view cone; the map
/// can rotate with the player or stay north-up. Enemies pulse, pings ripple, and objectives out
/// of range cling to the rim pointing the way.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{GameMapMarker, GameMarkerKind, GameMinimap};
/// let markers = [GameMapMarker::new(GameMarkerKind::Objective, egui::vec2(300.0, -120.0))];
/// ui.add(GameMinimap::new(egui::vec2(0.0, 0.0), 0.4).markers(&markers).round(true).size(180.0));
/// ```
pub struct GameMinimap<'a> {
    player: Vec2,
    heading: f32,
    markers: &'a [GameMapMarker],
    explored: &'a [Vec2],
    reveal: f32,
    round: bool,
    rotate: bool,
    zoom: f32,
    size: f32,
    seed: f32,
    style: GameStyle,
    palette: Option<GamePalette>,
}

impl<'a> GameMinimap<'a> {
    /// A minimap centered on `player` (world units) facing `heading` radians (0 = north, clockwise).
    pub fn new(player: Vec2, heading: f32) -> Self {
        Self {
            player,
            heading,
            markers: &[],
            explored: &[],
            reveal: 70.0,
            round: true,
            rotate: true,
            zoom: 220.0,
            size: 180.0,
            seed: 3.0,
            style: GameStyle::SciFi,
            palette: None,
        }
    }
    /// Markers to show.
    pub fn markers(mut self, m: &'a [GameMapMarker]) -> Self {
        self.markers = m;
        self
    }
    /// Explored breadcrumb positions; fog clears within `reveal` units of them and the player.
    /// Pass an empty slice to disable fog.
    pub fn explored(mut self, e: &'a [Vec2], reveal: f32) -> Self {
        self.explored = e;
        self.reveal = reveal.max(1.0);
        self
    }
    /// Round (true, default) or square.
    pub fn round(mut self, r: bool) -> Self {
        self.round = r;
        self
    }
    /// Rotate the map with the player (default) or keep north up.
    pub fn rotate(mut self, r: bool) -> Self {
        self.rotate = r;
        self
    }
    /// World units from center to edge.
    pub fn zoom(mut self, z: f32) -> Self {
        self.zoom = z.max(10.0);
        self
    }
    /// Diameter in points.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.clamp(80.0, 600.0);
        self
    }
    /// Terrain seed.
    pub fn seed(mut self, s: f32) -> Self {
        self.seed = s;
        self
    }
    /// Visual style.
    pub fn style(mut self, s: GameStyle) -> Self {
        self.style = s;
        self
    }
    /// Override the palette.
    pub fn palette(mut self, p: GamePalette) -> Self {
        self.palette = Some(p);
        self
    }

    fn terrain(&self, w: Vec2) -> Color32 {
        let n = glowkit_core::noise::fbm3(w.x * 0.004, w.y * 0.004, self.seed, 4);
        let detail = glowkit_core::noise::noise2(w.x * 0.03, w.y * 0.03) * 0.05;
        let h = n + detail;
        match self.style {
            GameStyle::SciFi => {
                if h < -0.08 {
                    mix(hex("#03101E"), hex("#06223A"), (h + 0.6).clamp(0.0, 1.0))
                } else {
                    let band = ((h * 14.0).fract() < 0.12) as i32 as f32;
                    mix(mix(hex("#0B3550"), hex("#1C6C8A"), (h * 2.0).clamp(0.0, 1.0)), hex("#5FE6FF"), band * 0.35)
                }
            }
            GameStyle::Fantasy => {
                if h < -0.08 {
                    mix(hex("#6E8A96"), hex("#9DB3B5"), (h + 0.5).clamp(0.0, 1.0))
                } else if h < -0.02 {
                    hex("#E4CF9C")
                } else {
                    let band = ((h * 10.0).fract() < 0.1) as i32 as f32;
                    mix(mix(hex("#C9B27A"), hex("#8D9A5A"), (h * 2.5).clamp(0.0, 1.0)), hex("#6A4B2A"), band * 0.4)
                }
            }
            GameStyle::Minimal => {
                if h < -0.08 {
                    hex("#1A2233")
                } else {
                    mix(hex("#2A2F3B"), hex("#4A5263"), (h * 2.0).clamp(0.0, 1.0))
                }
            }
        }
    }

    fn fog(&self, w: Vec2) -> f32 {
        if self.explored.is_empty() {
            return 0.0;
        }
        let mut d = (w - self.player).length();
        for e in self.explored {
            d = d.min((w - *e).length());
        }
        glowkit_core::anim::smoothstep(self.reveal * 0.6, self.reveal * 1.15, d)
    }
}

impl egui::Widget for GameMinimap<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(self.size + 16.0), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        glowkit_core::animate(ui);
        let time = glowkit_core::time(ui);
        let painter = ui.painter();
        let c = rect.center();
        let r = self.size / 2.0;
        let rot = if self.rotate { -self.heading } else { 0.0 };
        let (sr, cr) = rot.sin_cos();
        // map-space (points from center) -> world
        let to_world = |d: Vec2| {
            // inverse rotation
            let x = d.x * cr + d.y * sr;
            let y = -d.x * sr + d.y * cr;
            self.player + vec2(x, y) * (self.zoom / r)
        };
        let to_map = |w: Vec2| {
            let d = (w - self.player) * (r / self.zoom);
            vec2(d.x * cr - d.y * sr, d.x * sr + d.y * cr)
        };
        let fog_col = match self.style {
            GameStyle::Fantasy => hex("#3B2A18"),
            _ => hex("#02050A"),
        };
        let shade_px = |p: Pos2| {
            let w = to_world(p - c);
            let col = self.terrain(w);
            let f = self.fog(w);
            let fogged = mix(col, fog_col, 0.82);
            mix(col, fogged, f)
        };
        // drop shadow / frame back
        paint::soft_shadow(
            painter,
            Rect::from_center_size(c, Vec2::splat(self.size)),
            if self.round { r } else { 10.0 },
            18.0,
            vec2(0.0, 6.0),
            Color32::from_black_alpha(150),
        );
        if self.round {
            let rings = 16;
            let segs = 56;
            let mut m = Mesh::default();
            m.colored_vertex(c, shade_px(c));
            for k in 1..=rings {
                let rr = r * k as f32 / rings as f32;
                for i in 0..segs {
                    let a = i as f32 / segs as f32 * TAU;
                    let p = c + vec2(a.cos(), a.sin()) * rr;
                    m.colored_vertex(p, shade_px(p));
                }
            }
            let n = segs as u32;
            for i in 0..n {
                m.add_triangle(0, 1 + i, 1 + (i + 1) % n);
            }
            for k in 1..rings as u32 {
                let a0 = 1 + (k - 1) * n;
                let b0 = 1 + k * n;
                for i in 0..n {
                    let j = (i + 1) % n;
                    m.add_triangle(a0 + i, b0 + i, b0 + j);
                    m.add_triangle(a0 + i, b0 + j, a0 + j);
                }
            }
            painter.add(Shape::mesh(m));
        } else {
            let mr = Rect::from_center_size(c, Vec2::splat(self.size));
            paint::shader_grid(painter, mr, 30, 30, |uv| {
                shade_px(pos2(mr.min.x + uv.x * mr.width(), mr.min.y + uv.y * mr.height()))
            });
        }
        let inside =
            |p: Vec2| if self.round { p.length() <= r - 7.0 } else { p.x.abs() <= r - 7.0 && p.y.abs() <= r - 7.0 };
        let clamp_edge = |p: Vec2| {
            if self.round {
                p.normalized() * (r - 8.0)
            } else {
                let k = (r - 8.0) / p.x.abs().max(p.y.abs()).max(1e-3);
                p * k
            }
        };
        // range rings / grid
        let gc = with_alpha(if self.style == GameStyle::Fantasy { hex("#5A3D20") } else { pal.accent }, 0.18);
        if self.round {
            for k in 1..3 {
                painter.circle_stroke(c, r * k as f32 / 3.0, Stroke::new(1.0, gc));
            }
        } else {
            for k in 1..4 {
                let o = -r + self.size * k as f32 / 4.0;
                painter.line_segment([c + vec2(o, -r), c + vec2(o, r)], Stroke::new(1.0, gc));
                painter.line_segment([c + vec2(-r, o), c + vec2(r, o)], Stroke::new(1.0, gc));
            }
        }
        // sci-fi radar sweep
        if self.style == GameStyle::SciFi && self.round {
            let a = time * 1.6;
            let mut m = Mesh::default();
            m.colored_vertex(c, with_alpha(pal.accent, 0.0));
            let n = 20;
            for i in 0..=n {
                let aa = a - 0.9 * i as f32 / n as f32;
                m.colored_vertex(
                    c + vec2(aa.cos(), aa.sin()) * (r - 1.0),
                    with_alpha(pal.accent, 0.28 * (1.0 - i as f32 / n as f32)),
                );
            }
            for i in 0..n as u32 {
                m.add_triangle(0, 1 + i, 2 + i);
            }
            painter.add(Shape::mesh(m));
        }
        // markers
        for (i, mk) in self.markers.iter().enumerate() {
            let d = to_map(mk.pos);
            let vis = inside(d);
            let p = c + if vis { d } else { clamp_edge(d) };
            match mk.kind {
                GameMarkerKind::Enemy => {
                    if vis {
                        let pulse = 0.5 + 0.5 * (time * 5.0 + i as f32).sin();
                        paint::glow(painter, p, 9.0, with_alpha(pal.bad, 0.5 * pulse));
                        painter.circle_filled(p, 3.6, pal.bad);
                        painter.circle_stroke(p, 3.6, Stroke::new(1.0, Color32::from_black_alpha(180)));
                    }
                }
                GameMarkerKind::Ally => {
                    if vis {
                        painter.circle_filled(p, 3.6, pal.good);
                        painter.circle_stroke(p, 3.6, Stroke::new(1.0, Color32::from_black_alpha(180)));
                    }
                }
                GameMarkerKind::Loot => {
                    if vis {
                        paint::star(painter, p, 6.0, 2.6, 5, time * 0.8, GameRarity::Epic.color());
                    }
                }
                GameMarkerKind::Objective => {
                    let bob = (time * 3.0).sin() * 2.0;
                    let q = p + vec2(0.0, if vis { bob } else { 0.0 });
                    let s = if vis { 7.0 } else { 6.0 };
                    paint::glow(painter, q, 14.0, with_alpha(hex("#FFC94A"), 0.45));
                    painter.add(Shape::convex_polygon(
                        vec![q + vec2(0.0, -s), q + vec2(s * 0.75, 0.0), q + vec2(0.0, s), q + vec2(-s * 0.75, 0.0)],
                        hex("#FFC94A"),
                        Stroke::new(1.2, Color32::from_black_alpha(200)),
                    ));
                    if !vis {
                        let dir = d.normalized();
                        let tip = q + dir * 9.0;
                        let perp = vec2(-dir.y, dir.x);
                        painter.add(Shape::convex_polygon(
                            vec![tip, q + dir * 4.0 + perp * 4.0, q + dir * 4.0 - perp * 4.0],
                            hex("#FFC94A"),
                            Stroke::NONE,
                        ));
                    }
                }
                GameMarkerKind::Ping => {
                    if mk.age < 4.0 {
                        for k in 0..3 {
                            let t = (mk.age * 1.2 - k as f32 * 0.3).rem_euclid(1.2);
                            if mk.age * 1.2 >= k as f32 * 0.3 {
                                ring(painter, p, t, 1.2, 2.0, 26.0, pal.warn, 2.5);
                            }
                        }
                        let s = 1.0 + 0.4 * (1.0 - (mk.age * 4.0).min(1.0));
                        painter.circle_filled(p, 6.0 * s, pal.warn);
                        painter.text(p, Align2::CENTER_CENTER, "!", FontId::proportional(10.0 * s), Color32::BLACK);
                    }
                }
            }
        }
        // player: view cone + arrow
        let ah = if self.rotate { 0.0 } else { self.heading };
        let fwd = vec2(ah.sin(), -ah.cos());
        let side = vec2(-fwd.y, fwd.x);
        {
            let mut m = Mesh::default();
            m.colored_vertex(c, with_alpha(Color32::WHITE, 0.28));
            let n = 12;
            for i in 0..=n {
                let a = -0.55 + 1.1 * i as f32 / n as f32;
                let dir = fwd * a.cos() + side * a.sin();
                m.colored_vertex(c + dir * r * 0.55, with_alpha(Color32::WHITE, 0.0));
            }
            for i in 0..n as u32 {
                m.add_triangle(0, 1 + i, 2 + i);
            }
            painter.add(Shape::mesh(m));
        }
        let arrow = vec![c + fwd * 9.0, c - fwd * 6.0 + side * 6.5, c - fwd * 2.5, c - fwd * 6.0 - side * 6.5];
        paint::glow(painter, c, 14.0, with_alpha(pal.accent, 0.5));
        painter.add(Shape::convex_polygon(vec![arrow[0], arrow[1], arrow[2]], Color32::WHITE, Stroke::NONE));
        painter.add(Shape::convex_polygon(vec![arrow[0], arrow[2], arrow[3]], hex("#DDE6F0"), Stroke::NONE));
        let mut ol = arrow.clone();
        ol.push(arrow[0]);
        painter.add(Shape::line(ol, Stroke::new(1.2, Color32::from_black_alpha(220))));
        // frame
        let edge_c = match self.style {
            GameStyle::Fantasy => pal.edge,
            GameStyle::SciFi => pal.accent,
            GameStyle::Minimal => hex("#FFFFFF40"),
        };
        if self.round {
            match self.style {
                GameStyle::Fantasy => {
                    painter.circle_stroke(c, r + 3.0, Stroke::new(6.0, pal.edge));
                    painter.circle_stroke(c, r + 6.0, Stroke::new(1.5, glowkit_core::color::shade(pal.edge, -0.35)));
                    painter.circle_stroke(c, r, Stroke::new(1.5, glowkit_core::color::shade(pal.edge, -0.35)));
                }
                GameStyle::SciFi => {
                    painter.circle_stroke(c, r, Stroke::new(1.5, with_alpha(edge_c, 0.8)));
                    for k in 0..72 {
                        let a = k as f32 / 72.0 * TAU + rot;
                        let l = if k % 6 == 0 { 6.0 } else { 3.0 };
                        let d = vec2(a.cos(), a.sin());
                        painter.line_segment(
                            [c + d * (r + 3.0), c + d * (r + 3.0 + l)],
                            Stroke::new(1.0, with_alpha(edge_c, 0.6)),
                        );
                    }
                }
                GameStyle::Minimal => {
                    painter.circle_stroke(c, r, Stroke::new(2.0, edge_c));
                }
            }
        } else {
            let mr = Rect::from_center_size(c, Vec2::splat(self.size));
            match self.style {
                GameStyle::Fantasy => {
                    painter.rect_stroke(mr.expand(3.0), 4.0, Stroke::new(6.0, pal.edge), egui::StrokeKind::Middle);
                }
                _ => {
                    painter.rect_stroke(mr, 2.0, Stroke::new(1.5, with_alpha(edge_c, 0.8)), egui::StrokeKind::Outside);
                    let b = 14.0;
                    for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                        let p = c + vec2(sx * (r + 4.0), sy * (r + 4.0));
                        painter.line_segment([p, p - vec2(sx * b, 0.0)], Stroke::new(2.0, edge_c));
                        painter.line_segment([p, p - vec2(0.0, sy * b)], Stroke::new(2.0, edge_c));
                    }
                }
            }
        }
        // north marker
        let ndir = {
            let d = vec2(0.0, -1.0);
            vec2(d.x * cr - d.y * sr, d.x * sr + d.y * cr)
        };
        let np = if self.round { c + ndir * (r + 2.0) } else { c + clamp_edge(ndir * 1000.0) + ndir * 10.0 };
        let (bg, fg) = match self.style {
            GameStyle::Fantasy => (hex("#8E2F25"), hex("#FBEFD2")),
            GameStyle::SciFi => (hex("#06101E"), pal.accent),
            GameStyle::Minimal => (hex("#F4F5F7"), hex("#111318")),
        };
        painter.circle_filled(np, 8.5, bg);
        painter.circle_stroke(np, 8.5, Stroke::new(1.0, with_alpha(fg, 0.6)));
        painter.text(np, Align2::CENTER_CENTER, "N", FontId::proportional(10.0), fg);
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Ammo counter

/// An ammo counter: the magazine count in big type over a row of bullets, with the reserve
/// beside it. Each shot kicks the number, ejects a spinning casing from the bullet row and dims a
/// round; low ammo turns the count amber and pulses, an empty magazine flashes RELOAD, and a
/// reload sweeps the rounds back in.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::GameAmmoCounter::new(23, 30, 120).weapon("AR-7 Vanguard"));
/// ```
pub struct GameAmmoCounter<'a> {
    mag: u32,
    mag_size: u32,
    reserve: u32,
    reload: Option<f32>,
    weapon: &'a str,
    icon: GameIcon,
    style: GameStyle,
    palette: Option<GamePalette>,
    cues: Option<&'a mut Vec<GameCue>>,
}

impl<'a> GameAmmoCounter<'a> {
    /// `mag` rounds loaded out of `mag_size`, `reserve` spare rounds.
    pub fn new(mag: u32, mag_size: u32, reserve: u32) -> Self {
        Self {
            mag,
            mag_size: mag_size.max(1),
            reserve,
            reload: None,
            weapon: "",
            icon: GameIcon::Rifle,
            style: GameStyle::SciFi,
            palette: None,
            cues: None,
        }
    }
    /// Reload progress 0..1 while reloading (`None` when not reloading).
    pub fn reloading(mut self, p: Option<f32>) -> Self {
        self.reload = p;
        self
    }
    /// Weapon name.
    pub fn weapon(mut self, name: &'a str) -> Self {
        self.weapon = name;
        self
    }
    /// Weapon icon.
    pub fn icon(mut self, i: GameIcon) -> Self {
        self.icon = i;
        self
    }
    /// Visual style.
    pub fn style(mut self, s: GameStyle) -> Self {
        self.style = s;
        self
    }
    /// Override the palette.
    pub fn palette(mut self, p: GamePalette) -> Self {
        self.palette = Some(p);
        self
    }
    /// Collect feedback cues ([`GameCue::Shot`], [`GameCue::Reload`]).
    pub fn cues(mut self, c: &'a mut Vec<GameCue>) -> Self {
        self.cues = Some(c);
        self
    }
}

impl egui::Widget for GameAmmoCounter<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let big = display_font(ui, 44.0);
        let small = semi_font(ui, 16.0);
        let (rect, resp) = ui.allocate_exact_size(vec2(260.0, 104.0), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let time = glowkit_core::time(ui);
        let (since, prev, _) = changed(ui, resp.id, self.mag as f32);
        let shot = prev > self.mag as f32 && since < 0.6;
        let reloaded = prev < self.mag as f32 && since < 0.6;
        if since < 0.001 {
            if prev > self.mag as f32 {
                push_cue(&mut self.cues, GameCue::Shot);
            } else if prev < self.mag as f32 {
                push_cue(&mut self.cues, GameCue::Reload);
            }
        }
        let frac = self.mag as f32 / self.mag_size as f32;
        let low = frac <= 0.25;
        if since < 0.7 || low || self.reload.is_some() {
            glowkit_core::animate(ui);
        }
        let painter = ui.painter();
        frame(painter, rect, self.style, &pal, 0.0);
        let text_c = if matches!(self.style, GameStyle::Fantasy) { pal.text } else { pal.hud };
        // weapon line
        self.icon.paint(
            painter,
            pos2(rect.min.x + 30.0, rect.min.y + 24.0),
            34.0,
            if self.style == GameStyle::Fantasy { hex("#4A3A2A") } else { self.icon.tint() },
        );
        painter.text(
            pos2(rect.min.x + 54.0, rect.min.y + 24.0),
            Align2::LEFT_CENTER,
            self.weapon.to_uppercase(),
            FontId::proportional(11.0),
            pal.dim,
        );
        // count
        let kick = if shot { (1.0 - since / 0.18).max(0.0) } else { 0.0 };
        let mag_c = if self.mag == 0 {
            mix(pal.bad, Color32::WHITE, 0.5 + 0.5 * (time * 10.0).sin())
        } else if low {
            mix(pal.warn, text_c, 0.3 + 0.3 * (time * 6.0).sin())
        } else {
            mix(text_c, Color32::WHITE, kick)
        };
        let num = format!("{}", self.mag);
        let anchor = pos2(rect.max.x - 92.0, rect.min.y + 62.0 - kick * 5.0);
        let g = painter.layout_no_wrap(num, FontId::new(big.size * (1.0 + 0.22 * kick), big.family.clone()), mag_c);
        let gr = Align2::RIGHT_CENTER.anchor_size(anchor, g.size());
        if low || kick > 0.0 {
            blur(painter, gr.min, &g, with_alpha(if low { pal.warn } else { pal.accent }, 0.5), 5.0);
        }
        painter.galley(gr.min, g, mag_c);
        painter.text(
            pos2(rect.max.x - 84.0, rect.min.y + 70.0),
            Align2::LEFT_CENTER,
            format!("/ {}", self.reserve),
            small,
            with_alpha(text_c, 0.65),
        );
        // bullets row
        let n = self.mag_size.min(40) as usize;
        let shown_loaded = ((self.mag as f32 / self.mag_size as f32) * n as f32).round() as usize;
        let row =
            Rect::from_min_max(pos2(rect.min.x + 16.0, rect.max.y - 24.0), pos2(rect.max.x - 16.0, rect.max.y - 12.0));
        let bw = (row.width() / n as f32).min(9.0);
        let reload_p = self.reload.unwrap_or(0.0).clamp(0.0, 1.0);
        for i in 0..n {
            let x = row.max.x - (i as f32 + 0.5) * bw;
            let loaded = i < shown_loaded;
            let refill = self.reload.is_some() && (i as f32) < reload_p * n as f32;
            let b = Rect::from_center_size(pos2(x, row.center().y), vec2((bw * 0.55).max(2.0), row.height()));
            let col = if loaded {
                if low {
                    pal.warn
                } else {
                    hex("#E9C46A")
                }
            } else if refill {
                with_alpha(pal.accent, 0.8)
            } else {
                with_alpha(text_c, 0.15)
            };
            painter.rect_filled(b, egui::CornerRadius { nw: 3, ne: 3, sw: 0, se: 0 }, col);
            if loaded {
                painter.rect_filled(Rect::from_min_max(pos2(b.min.x, b.max.y - 3.0), b.max), 0.0, hex("#B5852E"));
            }
        }
        // ejected casing
        if shot && since < 0.6 {
            let i = shown_loaded;
            let x0 = row.max.x - (i as f32 + 0.5) * bw;
            let p = pos2(x0 + since * 120.0, row.center().y - since * 160.0 + since * since * 520.0);
            let ang = since * 22.0;
            let d = vec2(ang.cos(), ang.sin()) * 4.0;
            paint::line(painter, p - d, p + d, 3.0, with_alpha(hex("#E9C46A"), 1.0 - since / 0.6));
            paint::glow(
                painter,
                pos2(row.max.x + 4.0, row.center().y),
                16.0 * (1.0 - since / 0.2).max(0.0),
                with_alpha(hex("#FFD27A"), 0.6),
            );
        }
        if reloaded {
            paint::soft_shadow(
                painter,
                rect,
                6.0,
                20.0,
                vec2(0.0, 0.0),
                with_alpha(pal.accent, 0.4 * (1.0 - since / 0.6)),
            );
        }
        // status line
        let status = if let Some(p) = self.reload {
            let w = row.width();
            painter.rect_filled(
                Rect::from_min_size(pos2(row.min.x, row.min.y - 6.0), vec2(w * p, 2.0)),
                0.0,
                pal.accent,
            );
            Some(("RELOADING", pal.accent))
        } else if self.mag == 0 {
            Some(("RELOAD", pal.bad))
        } else if low {
            Some(("LOW AMMO", pal.warn))
        } else {
            None
        };
        if let Some((s, c)) = status {
            let a = if self.reload.is_some() { 1.0 } else { 0.55 + 0.45 * (time * 8.0).sin() };
            painter.text(
                pos2(rect.min.x + 16.0, rect.min.y + 58.0),
                Align2::LEFT_CENTER,
                s,
                FontId::monospace(12.0),
                with_alpha(c, a),
            );
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Cooldown icon

/// An ability slot with a radial cooldown sweep. While cooling down a dark wedge covers the icon
/// and unwinds clockwise with a bright hand on its edge and the seconds left in the middle; when
/// it comes back up the slot flashes white, pops with a spring, throws a ring and sparkles. Also
/// shows a keybind, stacked charges and an "out of resource" tint.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{GameCooldownIcon, GameIcon};
/// ui.add(GameCooldownIcon::new(GameIcon::Fire, 3.2, 8.0).key("Q"));
/// ```
pub struct GameCooldownIcon<'a> {
    icon: GameIcon,
    remaining: f32,
    total: f32,
    key: &'a str,
    charges: Option<u32>,
    disabled: bool,
    size: f32,
    color: Option<Color32>,
    style: GameStyle,
    palette: Option<GamePalette>,
    cues: Option<&'a mut Vec<GameCue>>,
}

impl<'a> GameCooldownIcon<'a> {
    /// An ability with `remaining` of `total` seconds of cooldown left.
    pub fn new(icon: GameIcon, remaining: f32, total: f32) -> Self {
        Self {
            icon,
            remaining,
            total: total.max(0.01),
            key: "",
            charges: None,
            disabled: false,
            size: 64.0,
            color: None,
            style: GameStyle::SciFi,
            palette: None,
            cues: None,
        }
    }
    /// Keybind label.
    pub fn key(mut self, k: &'a str) -> Self {
        self.key = k;
        self
    }
    /// Stacked charges badge.
    pub fn charges(mut self, n: u32) -> Self {
        self.charges = Some(n);
        self
    }
    /// Tint as unusable (not enough mana, silenced).
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }
    /// Slot size in points.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.clamp(28.0, 160.0);
        self
    }
    /// Icon color (default: the icon's tint).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Visual style.
    pub fn style(mut self, s: GameStyle) -> Self {
        self.style = s;
        self
    }
    /// Override the palette.
    pub fn palette(mut self, p: GamePalette) -> Self {
        self.palette = Some(p);
        self
    }
    /// Collect feedback cues ([`GameCue::Ready`]).
    pub fn cues(mut self, c: &'a mut Vec<GameCue>) -> Self {
        self.cues = Some(c);
        self
    }
}

/// Fan from `c` covering angles a0..a1 (clockwise from 12 o'clock, radians) clipped to a square.
fn square_fan(painter: &Painter, c: Pos2, half: f32, a0: f32, a1: f32, color: Color32) {
    if a1 - a0 <= 1e-3 {
        return;
    }
    let mut m = Mesh::default();
    m.colored_vertex(c, color);
    let n = (((a1 - a0) / TAU) * 64.0).ceil().max(2.0) as usize;
    for i in 0..=n {
        let a = a0 + (a1 - a0) * i as f32 / n as f32;
        let d = vec2(a.sin(), -a.cos());
        let k = half / d.x.abs().max(d.y.abs()).max(1e-4);
        m.colored_vertex(c + d * k, color);
    }
    for i in 0..n as u32 {
        m.add_triangle(0, 1 + i, 2 + i);
    }
    painter.add(Shape::mesh(m));
}

impl egui::Widget for GameCooldownIcon<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let s = self.size;
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(s + 8.0), Sense::click());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let ready = self.remaining <= 0.0;
        let (since, prev, _) = changed(ui, resp.id, if ready { 1.0 } else { 0.0 });
        let became_ready = ready && prev < 0.5 && since < 1.0;
        let used = !ready && prev > 0.5 && since < 0.4;
        if since < 0.0005 && ready && prev < 0.5 {
            push_cue(&mut self.cues, GameCue::Ready);
        }
        let hover = resp.hovered();
        let pop_target = if hover { 1.06 } else { 1.0 };
        let mut scale = spring(ui, resp.id.with("pop"), pop_target, 420.0, 16.0);
        if became_ready && since < 0.5 {
            scale += 0.18 * (since / 0.5 * PI).sin() * (1.0 - since / 0.5);
        }
        if used {
            scale -= 0.1 * (1.0 - since / 0.4);
        }
        if !ready || became_ready {
            glowkit_core::animate(ui);
        }
        let painter = ui.painter();
        let c = rect.center();
        let sr = Rect::from_center_size(c, Vec2::splat(s * scale));
        let lit = if hover {
            0.8
        } else if became_ready {
            (1.0 - since).max(0.0)
        } else {
            0.0
        };
        if ready && !self.disabled {
            paint::glow(painter, c, s * 0.75, with_alpha(pal.accent, 0.12 + 0.2 * lit));
        }
        slot(painter, sr, self.style, &pal, None, lit);
        let ic = self.color.unwrap_or_else(|| self.icon.tint());
        let ic = if self.disabled {
            mix(ic, hex("#3A4A80"), 0.6)
        } else if !ready {
            mix(ic, hex("#808080"), 0.45)
        } else {
            ic
        };
        self.icon.paint(painter, c, s * 0.62 * scale, ic);
        if !ready {
            let frac = (self.remaining / self.total).clamp(0.0, 1.0);
            let done = (1.0 - frac) * TAU;
            let half = sr.width() / 2.0 - 2.0;
            square_fan(painter, c, half, done, TAU, Color32::from_black_alpha(170));
            let d = vec2(done.sin(), -done.cos());
            let k = half / d.x.abs().max(d.y.abs()).max(1e-4);
            painter.line_segment([c, c + d * k], Stroke::new(2.0, with_alpha(pal.accent, 0.9)));
            paint::glow(painter, c + d * k, 8.0, with_alpha(pal.accent, 0.6));
            let txt = if self.remaining < 3.0 {
                format!("{:.1}", self.remaining)
            } else {
                format!("{}", self.remaining.ceil() as i64)
            };
            outlined(
                painter,
                c,
                Align2::CENTER_CENTER,
                &txt,
                FontId::proportional(s * 0.32),
                Color32::WHITE,
                Color32::from_black_alpha(230),
            );
        }
        if self.disabled {
            painter.rect_filled(sr.shrink(2.0), 4.0, with_alpha(hex("#1E3A8A"), 0.35));
        }
        if became_ready {
            let k = since / 0.35;
            if k < 1.0 {
                painter.rect_filled(sr.shrink(1.0), 4.0, with_alpha(Color32::WHITE, 0.75 * (1.0 - k)));
            }
            ring(painter, c, since, 0.6, s * 0.45, s * 0.95, pal.accent, 3.0);
            burst(painter, c, since, 10, 41, s * 3.0, 0.0, pal.accent, 2.6);
        }
        if !self.key.is_empty() {
            let kp = pos2(sr.min.x + 2.0, sr.min.y + 2.0);
            let kr = Rect::from_min_size(kp, vec2(16.0f32.max(self.key.len() as f32 * 7.0 + 6.0), 15.0));
            painter.rect_filled(kr, 3.0, hex("#000000C8"));
            painter.rect_stroke(kr, 3.0, Stroke::new(1.0, with_alpha(pal.edge, 0.6)), egui::StrokeKind::Inside);
            painter.text(kr.center(), Align2::CENTER_CENTER, self.key, FontId::monospace(10.0), Color32::WHITE);
        }
        if let Some(n) = self.charges {
            outlined(
                painter,
                sr.max - vec2(4.0, 2.0),
                Align2::RIGHT_BOTTOM,
                &n.to_string(),
                FontId::proportional(s * 0.26),
                Color32::WHITE,
                Color32::BLACK,
            );
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Crosshair

/// The shape of a [`GameCrosshair`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum GameCrosshairShape {
    /// Four lines with a gap that opens with spread.
    #[default]
    Cross,
    /// A single dot.
    Dot,
    /// A ring that grows with spread, with four ticks.
    Circle,
    /// A chevron under a dot.
    Chevron,
    /// Four corner brackets.
    Brackets,
    /// A thin scope reticle with mil dots.
    Scope,
}

impl GameCrosshairShape {
    /// All shapes.
    pub const ALL: [GameCrosshairShape; 6] = [
        GameCrosshairShape::Cross,
        GameCrosshairShape::Dot,
        GameCrosshairShape::Circle,
        GameCrosshairShape::Chevron,
        GameCrosshairShape::Brackets,
        GameCrosshairShape::Scope,
    ];
}

/// A dynamic crosshair. Spread (0 still .. 1 sprinting / spraying) opens it on a spring, a hit
/// pops a white X hit marker, and a kill turns it into a bigger red X. Every line has a dark
/// outline so it reads on any background.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{GameCrosshair, GameCrosshairShape};
/// ui.add(GameCrosshair::new(GameCrosshairShape::Cross).spread(0.4).hit(Some(0.05), false));
/// ```
pub struct GameCrosshair {
    shape: GameCrosshairShape,
    spread: f32,
    hit_age: Option<f32>,
    kill: bool,
    color: Color32,
    size: f32,
    thickness: f32,
}

impl GameCrosshair {
    /// A crosshair of `shape`.
    pub fn new(shape: GameCrosshairShape) -> Self {
        Self { shape, spread: 0.0, hit_age: None, kill: false, color: hex("#7CFFB2"), size: 96.0, thickness: 2.0 }
    }
    /// Spread / bloom 0..1.
    pub fn spread(mut self, s: f32) -> Self {
        self.spread = s.clamp(0.0, 1.5);
        self
    }
    /// Seconds since the last hit (shows a hit marker for ~0.35 s), and whether it was a kill.
    pub fn hit(mut self, age: Option<f32>, kill: bool) -> Self {
        self.hit_age = age;
        self.kill = kill;
        self
    }
    /// Line color.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = c;
        self
    }
    /// Widget size in points.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.clamp(24.0, 400.0);
        self
    }
    /// Line thickness.
    pub fn thickness(mut self, t: f32) -> Self {
        self.thickness = t.clamp(1.0, 6.0);
        self
    }
}

fn ol_line(painter: &Painter, a: Pos2, b: Pos2, w: f32, c: Color32) {
    painter.line_segment([a, b], Stroke::new(w + 2.0, Color32::from_black_alpha((c.a() as f32 * 0.7) as u8)));
    painter.line_segment([a, b], Stroke::new(w, c));
}

impl egui::Widget for GameCrosshair {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(self.size), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let sp = spring(ui, resp.id.with("spread"), self.spread, 260.0, 18.0).max(0.0);
        let painter = ui.painter();
        let c = rect.center();
        let col = self.color;
        let w = self.thickness;
        let unit = self.size / 96.0;
        let gap = (5.0 + sp * 22.0) * unit;
        let len = 9.0 * unit;
        match self.shape {
            GameCrosshairShape::Cross => {
                for d in [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)] {
                    ol_line(painter, c + d * gap, c + d * (gap + len), w, col);
                }
                painter.circle_filled(c, w * 0.75, col);
            }
            GameCrosshairShape::Dot => {
                painter.circle_filled(c, w * 1.6 + 1.0, Color32::from_black_alpha(160));
                painter.circle_filled(c, w * 1.6, col);
                painter.circle_stroke(c, gap + 4.0 * unit, Stroke::new(1.0, with_alpha(col, 0.25 * sp.min(1.0))));
            }
            GameCrosshairShape::Circle => {
                let r = 10.0 * unit + sp * 20.0 * unit;
                painter.circle_stroke(c, r, Stroke::new(w + 2.0, Color32::from_black_alpha(130)));
                painter.circle_stroke(c, r, Stroke::new(w * 0.8, col));
                for d in [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)] {
                    ol_line(painter, c + d * (r - 3.0 * unit), c + d * (r + 5.0 * unit), w, col);
                }
                painter.circle_filled(c, w * 0.7, col);
            }
            GameCrosshairShape::Chevron => {
                let o = gap * 0.6;
                let pts =
                    [c + vec2(-9.0 * unit, 6.0 * unit + o), c + vec2(0.0, o), c + vec2(9.0 * unit, 6.0 * unit + o)];
                ol_line(painter, pts[0], pts[1], w, col);
                ol_line(painter, pts[1], pts[2], w, col);
                painter.circle_filled(c - vec2(0.0, 2.0), w, col);
            }
            GameCrosshairShape::Brackets => {
                let r = gap + 6.0 * unit;
                let b = 7.0 * unit;
                for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                    let p = c + vec2(sx * r, sy * r);
                    ol_line(painter, p, p - vec2(sx * b, 0.0), w, col);
                    ol_line(painter, p, p - vec2(0.0, sy * b), w, col);
                }
                painter.circle_filled(c, w * 0.8, col);
            }
            GameCrosshairShape::Scope => {
                let r = self.size * 0.46;
                painter.circle_stroke(c, r, Stroke::new(4.0, Color32::from_black_alpha(140)));
                painter.circle_stroke(c, r, Stroke::new(1.5, with_alpha(col, 0.8)));
                for (a, b) in [
                    (c - vec2(r, 0.0), c - vec2(gap, 0.0)),
                    (c + vec2(gap, 0.0), c + vec2(r, 0.0)),
                    (c + vec2(0.0, gap), c + vec2(0.0, r)),
                    (c - vec2(0.0, r), c - vec2(0.0, gap)),
                ] {
                    ol_line(painter, a, b, 1.0, col);
                }
                for k in 1..5 {
                    let o = gap + k as f32 * (r - gap) / 5.0;
                    for p in [c + vec2(0.0, o), c + vec2(o, 0.0), c - vec2(o, 0.0)] {
                        painter.circle_filled(p, 2.4, Color32::from_black_alpha(160));
                        painter.circle_filled(p, 1.5, col);
                    }
                }
                painter.circle_filled(c, 1.8, hex("#FF3B3B"));
                paint::glow(painter, c, 6.0, with_alpha(hex("#FF3B3B"), 0.6));
            }
        }
        if let Some(age) = self.hit_age {
            let dur = if self.kill { 0.55 } else { 0.32 };
            if (0.0..dur).contains(&age) {
                ui.ctx().request_repaint();
                let k = age / dur;
                let s = 1.0 + 0.5 * (1.0 - (age / 0.08).min(1.0));
                let hc = if self.kill { hex("#FF3B4A") } else { Color32::WHITE };
                let hc = with_alpha(hc, 1.0 - k * k);
                let r0 = (6.0 + gap * 0.4) * unit.max(0.6) * s;
                let r1 = r0 + if self.kill { 13.0 } else { 9.0 } * unit * s;
                for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                    let d = vec2(sx, sy).normalized();
                    ol_line(painter, c + d * r0, c + d * r1, if self.kill { w + 1.5 } else { w + 0.5 }, hc);
                }
            }
        }
        resp
    }
}
