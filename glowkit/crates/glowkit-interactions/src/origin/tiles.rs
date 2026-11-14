//! Material showpieces: a furry app-icon tile and a turning ring of iridescent glass slabs.

use super::common::{h01, idle_secs, lerp_rgb, ramp, strip, Cam, V3, v3};
use egui::epaint::Mesh;
use egui::{pos2, vec2, Color32, CursorIcon, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
use glowkit_core::color::{hex, with_alpha};
use glowkit_core::noise::noise3;
use glowkit_core::{paint, Theme};
use std::f32::consts::{PI, TAU};

/// The mark combed into a [`PlushTile`]'s fur.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PlushMark {
    /// A bold ring with a dot (default).
    #[default]
    Ring,
    /// A heart.
    Heart,
    /// Four rounded petals in four colors.
    Petals,
    /// A lightning bolt.
    Bolt,
}

impl PlushMark {
    /// Is `uv` (0..1 across the tile) inside the mark? Returns a palette index, or `None` for the
    /// tile's base fur.
    fn sample(self, uv: Vec2) -> Option<usize> {
        let p = (uv - vec2(0.5, 0.5)) * 2.0; // -1..1
        match self {
            PlushMark::Ring => {
                let d = p.length();
                if (0.36..0.62).contains(&d) || d < 0.16 {
                    Some(0)
                } else {
                    None
                }
            }
            PlushMark::Heart => {
                let x = p.x * 1.25;
                let y = -p.y * 1.25 + 0.25;
                let a = x * x + y * y - 0.42;
                if a * a * a - x * x * y * y * y < 0.0 {
                    Some(0)
                } else {
                    None
                }
            }
            PlushMark::Petals => {
                let q = p.abs();
                let d = ((q.x - 0.3).powi(2) + (q.y - 0.3).powi(2)).sqrt();
                if d < 0.26 {
                    Some(match (p.x < 0.0, p.y < 0.0) {
                        (true, true) => 0,
                        (false, true) => 1,
                        (false, false) => 2,
                        (true, false) => 3,
                    })
                } else {
                    None
                }
            }
            PlushMark::Bolt => {
                let (x, y) = (p.x, p.y);
                let upper = y < 0.05 && y > -0.7 && x > -0.35 + (y + 0.7) * 0.25 - 0.05 && x < 0.25 + (y + 0.7) * 0.05 && x - y * 0.55 < 0.35;
                let lower = y >= -0.05 && y < 0.72 && x > -0.2 + (y * 0.1) && x < 0.42 - (y + 0.05) * 0.75 && x + 0.1 > (y - 0.05) * 0.2 - 0.25;
                if upper || lower {
                    Some(0)
                } else {
                    None
                }
            }
        }
    }
}

/// An app-icon tile covered in soft fur: thousands of short combed strands carry a mark in the
/// pile, part under the pointer to reveal the darker under-fur, and stir in a gentle breeze.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::PlushTile::new().mark(glowkit_interactions::PlushMark::Heart));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PlushTile {
    size: f32,
    mark: PlushMark,
    base: Option<Color32>,
    colors: Option<[Color32; 4]>,
    density: f32,
    length: f32,
    breeze: f32,
}

impl Default for PlushTile {
    fn default() -> Self {
        Self::new()
    }
}

impl PlushTile {
    /// A plush tile with default settings.
    pub fn new() -> Self {
        Self { size: 220.0, mark: PlushMark::Ring, base: None, colors: None, density: 1.0, length: 1.0, breeze: 1.0 }
    }
    /// Tile edge length in points.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.clamp(60.0, 360.0);
        self
    }
    /// The mark combed into the fur.
    pub fn mark(mut self, m: PlushMark) -> Self {
        self.mark = m;
        self
    }
    /// Base fur color (default cream).
    pub fn base(mut self, c: Color32) -> Self {
        self.base = Some(c);
        self
    }
    /// Mark colors (the ring, heart and bolt use the first).
    pub fn colors(mut self, c: [Color32; 4]) -> Self {
        self.colors = Some(c);
        self
    }
    /// Strand density multiplier (0.4..1.6).
    pub fn density(mut self, k: f32) -> Self {
        self.density = k.clamp(0.4, 1.6);
        self
    }
    /// Strand length multiplier (0.4..2).
    pub fn length(mut self, k: f32) -> Self {
        self.length = k.clamp(0.4, 2.0);
        self
    }
    /// Breeze strength (0..2).
    pub fn breeze(mut self, k: f32) -> Self {
        self.breeze = k.clamp(0.0, 2.0);
        self
    }
}

impl Widget for PlushTile {
    fn ui(self, ui: &mut Ui) -> Response {
        let side = self.size + 60.0;
        let (outer, response) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
        if !ui.is_rect_visible(outer) {
            return response;
        }
        glowkit_core::animate(ui);
        let t = Theme::of(ui);
        let id = response.id;
        let time = glowkit_core::time(ui);
        let rect = Rect::from_center_size(outer.center(), Vec2::splat(self.size));
        let idle = idle_secs(ui, id, response.hovered());
        let pointer = response.hover_pos().or(if idle > 1.0 {
            let a = time * 0.7;
            Some(rect.center() + vec2(a.cos() * self.size * 0.28, (a * 1.6).sin() * self.size * 0.25))
        } else {
            None
        });
        let presence = glowkit_core::anim::follow(ui, id.with("presence"), if pointer.is_some() { 1.0 } else { 0.0 }, 6.0);
        let pf = glowkit_core::anim::spring2(ui, id.with("ptr"), pointer.unwrap_or(rect.center()).to_vec2(), 120.0, 18.0).to_pos2();
        let painter = ui.painter_at(outer);
        let base = self.base.unwrap_or(hex("#F2EBDD"));
        let cols = self.colors.unwrap_or([hex("#E2232E"), hex("#2F6BFF"), hex("#F5B300"), hex("#1FA855")]);
        let radius = self.size * 0.24;
        paint::soft_shadow(&painter, rect.translate(vec2(0.0, 10.0)), radius, 22.0, Vec2::ZERO, with_alpha(Color32::BLACK, if t.dark { 0.6 } else { 0.25 }));
        // skin under the fur
        let skin_of = |uv: Vec2| -> Color32 {
            match self.mark.sample(uv) {
                Some(i) => lerp_rgb(cols[i], Color32::BLACK, 0.35),
                None => lerp_rgb(base, hex("#8C7A60"), 0.45),
            }
        };
        paint::fill_convex_fn(&painter, &paint::rounded_rect_points(rect, radius, 10), 8, |p| {
            let uv = (p - rect.min) / rect.size();
            skin_of(uv)
        });
        // strands
        let spacing = (3.4 / self.density).max(2.0) * (self.size / 220.0).clamp(0.7, 1.4);
        let n = (self.size / spacing) as usize;
        let len = 7.0 * self.length * (self.size / 220.0).clamp(0.7, 1.5);
        let mut mesh = Mesh::default();
        mesh.reserve_vertices(n * n * 12);
        let light = vec2(-0.6, -0.8);
        let part_r = self.size * 0.2;
        for j in 0..n {
            for i in 0..n {
                let seed = (j * 1000 + i) as u64 * 2_654_435_761 % 1_000_003;
                let root = rect.min + vec2((i as f32 + h01(seed)) * spacing, (j as f32 + h01(seed + 1)) * spacing);
                // inside the rounded square (allow a fuzzy edge)
                let q = (root - rect.center()).abs() - (rect.size() * 0.5 - Vec2::splat(radius));
                let outside = q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0) - radius;
                if outside > -1.0 + h01(seed + 2) * 2.0 {
                    continue;
                }
                let uv = (root - rect.min) / rect.size();
                let cidx = self.mark.sample(uv);
                let col = match cidx {
                    Some(k) => cols[k],
                    None => base,
                };
                // combed direction: flowing pile + breeze
                let flow = noise3(uv.x * 2.2, uv.y * 2.2, 0.5) * 1.4 + 1.9;
                let br = noise3(uv.x * 1.5 + time * 0.35, uv.y * 1.5, time * 0.2) * 0.5 * self.breeze;
                let mut dir = vec2((flow + br).cos(), (flow + br).sin());
                let mut l = len * (0.75 + 0.5 * h01(seed + 3));
                let mut lift = 1.0;
                // parting under the pointer
                let d = root - pf;
                let dl = d.length();
                if dl < part_r * 1.6 {
                    let k = (1.0 - dl / (part_r * 1.6)).powi(2) * presence;
                    let out = if dl > 1e-3 { d / dl } else { dir };
                    dir = (dir * (1.0 - k) + out * k * 1.8).normalized();
                    l *= 1.0 + k * 0.6;
                    lift = 1.0 - k * 0.5;
                }
                // edge strands lean outward (rounded pillow)
                if outside > -radius * 0.25 {
                    let out = (root - rect.center()).normalized();
                    dir = (dir + out * 0.8).normalized();
                }
                let shade = (0.8 + 0.35 * dir.dot(-light).max(-0.5)) * lift;
                let pillow = 1.0 - ((uv - vec2(0.35, 0.3)).length() * 0.45).min(0.4);
                let jitter = 0.88 + 0.24 * h01(seed + 4);
                let root_c = lerp_rgb(col, Color32::BLACK, 0.4);
                let tip_c = lerp_rgb(col, Color32::WHITE, (0.12 + (shade - 0.8) * 0.5).clamp(0.0, 0.5));
                let tip_c = lerp_rgb(Color32::BLACK, tip_c, (shade * pillow * jitter).min(1.1).min(1.0));
                let tip = root + dir * l;
                let mid = root.lerp(tip, 0.5) + dir.rot90() * (h01(seed + 5) - 0.5) * 1.6;
                strip(
                    &mut mesh,
                    &[root, mid, tip],
                    |k| [1.5, 1.2, 0.6][k],
                    |k| [root_c, lerp_rgb(root_c, tip_c, 0.7), tip_c][k],
                    |_| Color32::TRANSPARENT,
                );
            }
        }
        painter.add(Shape::mesh(mesh));
        // soft top light
        paint::glow(&painter, rect.min + rect.size() * vec2(0.32, 0.25), self.size * 0.55, with_alpha(Color32::WHITE, 0.08));
        response.on_hover_cursor(CursorIcon::Grab)
    }
}

// ---------------------------------------------------------------------------------------------
// Corolla Gleam
// ---------------------------------------------------------------------------------------------

/// A crown of thick iridescent glass slabs standing in a ring: it turns slowly, breathes, and
/// leans toward the pointer while thin-film colors slide across every slab.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::CorollaGleam::new().slabs(16));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CorollaGleam {
    size: Option<Vec2>,
    slabs: usize,
    speed: f32,
    film: Option<Vec<Color32>>,
    background: Option<Color32>,
}

impl Default for CorollaGleam {
    fn default() -> Self {
        Self::new()
    }
}

impl CorollaGleam {
    /// A ring with default settings.
    pub fn new() -> Self {
        Self { size: None, slabs: 16, speed: 1.0, film: None, background: None }
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Number of slabs in the ring (6..32).
    pub fn slabs(mut self, n: usize) -> Self {
        self.slabs = n.clamp(6, 32);
        self
    }
    /// Turn speed multiplier.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Thin-film colors cycled across the slabs.
    pub fn film(mut self, c: Vec<Color32>) -> Self {
        if c.len() >= 2 {
            self.film = Some(c);
        }
        self
    }
    /// Background fill (default: the theme background).
    pub fn background(mut self, c: Color32) -> Self {
        self.background = Some(c);
        self
    }
}

impl Widget for CorollaGleam {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = glowkit_core::desired_size(ui, self.size, 380.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let t = Theme::of(ui);
        let id = response.id;
        let time = glowkit_core::time(ui);
        let target = match response.hover_pos() {
            Some(p) => ((p - rect.center()) / (rect.size() * 0.5)).clamp(Vec2::splat(-1.0), Vec2::splat(1.0)),
            None => vec2((time * 0.35).sin() * 0.5, (time * 0.27).sin() * 0.3),
        };
        let lean = glowkit_core::anim::spring2(ui, id.with("lean"), target, 40.0, 9.0);
        let painter = ui.painter_at(rect);
        let bg = self.background.unwrap_or(t.background);
        painter.rect_filled(rect, 0.0, bg);
        let film = self.film.clone().unwrap_or_else(|| vec![hex("#FF8AD8"), hex("#8B5CF6"), hex("#4F46E5"), hex("#14B8A6"), hex("#F59E0B"), hex("#F472B6")]);
        let unit = rect.height().min(rect.width() * 0.55);
        let cam = Cam { center: pos2(rect.center().x, rect.max.y - unit * 0.08), focal: unit * 3.4, dist: 7.0 };
        let yaw = time * 0.18 * self.speed + lean.x * 0.4;
        let pitch = 0.18 + lean.y * 0.2;
        let breathe = 1.0 + (time * 0.9).sin() * 0.04;
        let n = self.slabs;
        let (r0, r1, hgt) = (0.55, 1.85, 1.55);
        let thick = 0.09;
        // local slab outline (u radial, v up): a tongue with a round top
        let mut outline: Vec<(f32, f32)> = Vec::new();
        let rw = (r1 - r0) * 0.5;
        let uc = (r0 + r1) * 0.5;
        let top = hgt * breathe - rw;
        outline.push((r0, -0.4));
        outline.push((r1, -0.4));
        for k in 0..=12 {
            let a = k as f32 / 12.0 * PI;
            outline.push((uc + a.cos() * rw, top + a.sin() * rw));
        }
        struct Slab {
            depth: f32,
            front: Vec<Pos2>,
            back: Vec<Pos2>,
            facing: f32,
            k: usize,
        }
        let mut slabs: Vec<Slab> = Vec::with_capacity(n);
        for k in 0..n {
            let th = k as f32 / n as f32 * TAU + yaw;
            let radial = v3(th.cos(), 0.0, th.sin());
            let normal = v3(-th.sin(), 0.0, th.cos());
            let place = |u: f32, v: f32, off: f32| -> V3 { (radial * u + v3(0.0, v, 0.0) + normal * off).rot_x(pitch) };
            let proj = |p: V3| cam.project(p).map(|x| x.0).unwrap_or(cam.center);
            let front: Vec<Pos2> = outline.iter().map(|(u, v)| proj(place(*u, *v, thick))).collect();
            let back: Vec<Pos2> = outline.iter().map(|(u, v)| proj(place(*u, *v, -thick))).collect();
            let c = place(uc, hgt * 0.5, 0.0);
            let facing = normal.rot_x(pitch).z;
            slabs.push(Slab { depth: c.z, front, back, facing, k });
        }
        slabs.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap_or(std::cmp::Ordering::Equal));
        // floor glow
        if t.dark {
            paint::glow_ellipse(&painter, cam.center, vec2(unit * 1.1, unit * 0.18), with_alpha(film[1], 0.25));
        } else {
            paint::glow_ellipse(&painter, cam.center, vec2(unit * 1.1, unit * 0.16), with_alpha(film[2], 0.12));
        }
        for s in &slabs {
            let f = s.facing.abs();
            let hue = s.k as f32 / n as f32 + time * 0.04 + s.facing * 0.25;
            let tint = |p: Pos2, extra: f32| -> Color32 {
                let y = (cam.center.y - p.y) / unit;
                let c = glowkit_core::color::gradient_cyclic(&film, hue + y * 0.35 + extra);
                let sheen = (1.0 - f).powi(3) * 0.5;
                lerp_rgb(c, Color32::WHITE, sheen + (y - 0.8).max(0.0) * 0.25)
            };
            // edge band (thickness) between the two faces
            let edge_c = lerp_rgb(glowkit_core::color::gradient_cyclic(&film, hue + 0.3), Color32::WHITE, 0.45);
            let mut band = Mesh::default();
            let m = s.front.len();
            for i in 0..m {
                let j = (i + 1) % m;
                let b0 = band.vertices.len() as u32;
                for p in [s.front[i], s.front[j], s.back[j], s.back[i]] {
                    band.colored_vertex(p, edge_c);
                }
                band.add_triangle(b0, b0 + 1, b0 + 2);
                band.add_triangle(b0, b0 + 2, b0 + 3);
            }
            painter.add(Shape::mesh(band));
            let face = if s.facing > 0.0 { &s.front } else { &s.back };
            paint::fill_convex_fn(&painter, face, 5, |p| with_alpha(tint(p, 0.0), 0.94));
            // inner glassy highlight along the rounded rim
            let rim: Vec<Pos2> = face[2..].to_vec();
            painter.add(Shape::line(rim, Stroke::new(1.4, with_alpha(Color32::WHITE, 0.35 + 0.4 * (1.0 - f)))));
        }
        // fade into the floor
        let fade_h = unit * 0.12;
        paint::shader_grid(&painter, Rect::from_min_max(pos2(rect.min.x, rect.max.y - fade_h), rect.max), 1, 4, |uv| with_alpha(bg, uv.y));
        let _ = ramp;
        response
    }
}
