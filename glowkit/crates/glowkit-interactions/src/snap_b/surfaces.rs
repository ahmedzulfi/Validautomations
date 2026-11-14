//! Image and surface effects: a dither lens over a photograph, a halftone logo and a
//! holographic foil card.

use super::common::{
    bayer4, cover_uv, display_font, galley_mapped, ghost, hash3, idle_time, image_rounded, lerp_c, lighten,
    mesh_circle, mesh_rect, poly_shadow, seed_photo, surface_grid, text_mask, v3, Cam,
};
use egui::epaint::Mesh;
use egui::{pos2, vec2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Ui, Vec2, Widget};
use glowkit_core::color::{hex, hsv, with_alpha};
use glowkit_core::noise::noise3;
use glowkit_core::Theme;
use std::f32::consts::TAU;

// ---------------------------------------------------------------------------------------------

/// A photograph that turns into an ordered-dither print inside a ragged, flickering zone that
/// follows the pointer. Everywhere else the photo stays a smooth image. When the pointer is
/// away a ghost lens drifts over the picture.
///
/// The built-in photo is a procedural backlit seed head (its pixels are needed on the CPU to
/// dither them).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::DitherLens::new().radius(90.0).cell(5.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DitherLens {
    size: Option<Vec2>,
    radius: f32,
    cell: f32,
    ragged: f32,
    inks: Option<[Color32; 2]>,
    autoplay: bool,
}

impl Default for DitherLens {
    fn default() -> Self {
        Self::new()
    }
}

impl DitherLens {
    /// A lens over the built-in photograph.
    pub fn new() -> Self {
        Self { size: None, radius: 92.0, cell: 5.0, ragged: 1.0, inks: None, autoplay: true }
    }
    /// Widget size (default: available width x 340).
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }
    /// Lens radius in points.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.clamp(20.0, 400.0);
        self
    }
    /// Dither cell size in points.
    pub fn cell(mut self, c: f32) -> Self {
        self.cell = c.clamp(2.0, 16.0);
        self
    }
    /// How torn and flickery the lens edge is, `0..=2`.
    pub fn ragged(mut self, r: f32) -> Self {
        self.ragged = r.clamp(0.0, 2.0);
        self
    }
    /// Dark and light ink of the print (default black paper, white ink).
    pub fn inks(mut self, dark: Color32, light: Color32) -> Self {
        self.inks = Some([dark, light]);
        self
    }
    /// Drift a ghost lens over the photo while the pointer is away (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for DitherLens {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = self.size.unwrap_or(vec2(ui.available_width().min(760.0), 340.0));
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let id = response.id;
        let photo = seed_photo(ui.ctx());
        let time = glowkit_core::time(ui);
        let hover = response.hover_pos();
        let idle = idle_time(ui, id, hover.is_some());
        let target = match hover {
            Some(p) => p,
            None if self.autoplay => ghost(rect.shrink(40.0), time * 0.9),
            None => rect.center(),
        };
        let (fx, fy) = (
            glowkit_core::anim::follow(ui, id.with("lx"), target.x, if idle > 0.0 { 5.0 } else { 14.0 }),
            glowkit_core::anim::follow(ui, id.with("ly"), target.y, if idle > 0.0 { 5.0 } else { 14.0 }),
        );
        let lens = pos2(fx, fy);
        let show = if hover.is_some() || self.autoplay { 1.0 } else { 0.0 };
        let grow = glowkit_core::anim::spring(ui, id.with("grow"), show, 160.0, 18.0);
        glowkit_core::animate(ui);
        let painter = ui.painter_at(rect);
        let radius = t.radius * 1.6;
        let uv = cover_uv(photo.w as f32 / photo.h as f32, size.x / size.y);
        image_rounded(&painter, rect, radius, photo.tex, uv, Color32::WHITE);
        let [dark, light] = self.inks.unwrap_or([hex("#050505"), hex("#F4F1EA")]);
        let cell = self.cell;
        let r = self.radius * grow.max(0.0);
        if r > 2.0 {
            let mut mesh = Mesh::default();
            let tick = (time * 14.0) as i32;
            let x0 = ((lens.x - r * 1.5 - rect.min.x) / cell).floor() as i32;
            let x1 = ((lens.x + r * 1.5 - rect.min.x) / cell).ceil() as i32;
            let y0 = ((lens.y - r * 1.5 - rect.min.y) / cell).floor() as i32;
            let y1 = ((lens.y + r * 1.5 - rect.min.y) / cell).ceil() as i32;
            for cy in y0.max(0)..y1 {
                for cx in x0.max(0)..x1 {
                    let c = rect.min + vec2((cx as f32 + 0.5) * cell, (cy as f32 + 0.5) * cell);
                    if !rect.shrink(1.0).contains(c) {
                        continue;
                    }
                    let d = c - lens;
                    let ang = d.y.atan2(d.x);
                    let edge = r
                        * (0.82
                            + 0.22 * self.ragged * noise3(ang.cos() * 1.6, ang.sin() * 1.6, time * 1.3)
                            + 0.08 * self.ragged * noise3(ang * 3.0, 0.0, time * 4.0));
                    let flick = (hash3(cx, cy, tick) - 0.5) * cell * 4.0 * self.ragged;
                    if d.length() > edge + flick {
                        continue;
                    }
                    let u = uv.min.x + (c.x - rect.min.x) / size.x * uv.width();
                    let v = uv.min.y + (c.y - rect.min.y) / size.y * uv.height();
                    let l = photo.luma_at(u, v);
                    // lift contrast so the print reads
                    let l = ((l - 0.06) * 2.1).clamp(0.0, 1.0);
                    let cr = Rect::from_center_size(c, Vec2::splat(cell));
                    mesh_rect(&mut mesh, cr, dark);
                    if l > bayer4(cx as usize, cy as usize) {
                        mesh_rect(&mut mesh, cr.shrink(cell * 0.12), light);
                    } else if l > bayer4(cx as usize, cy as usize) * 0.5 {
                        // faint box outline for mid-tones
                        let o = cr.shrink(cell * 0.18);
                        mesh_rect(&mut mesh, Rect::from_min_max(o.min, pos2(o.max.x, o.min.y + 0.8)), with_alpha(light, 0.55));
                        mesh_rect(&mut mesh, Rect::from_min_max(o.min, pos2(o.min.x + 0.8, o.max.y)), with_alpha(light, 0.55));
                    }
                }
            }
            painter.add(Shape::mesh(mesh));
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A logo built from a rotated halftone dot screen: dots swell where the mark is solid and
/// thin out at its edges, and a wave of ink rolls diagonally through the screen, fattening and
/// brightening the dots as it passes.
///
/// The mark is an eclipse emblem plus a wordmark (any text).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::HalftoneLogo::new("lumen").pitch(7.0).angle(20.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct HalftoneLogo {
    word: String,
    emblem: bool,
    size: f32,
    pitch: f32,
    angle: f32,
    colors: Option<[Color32; 2]>,
    wave: f32,
}

impl HalftoneLogo {
    /// A halftone logo with `word` as the wordmark (empty for emblem only).
    pub fn new(word: impl Into<String>) -> Self {
        Self { word: word.into(), emblem: true, size: 96.0, pitch: 7.0, angle: 20.0, colors: None, wave: 1.0 }
    }
    /// Show the eclipse emblem left of the wordmark (default on).
    pub fn emblem(mut self, on: bool) -> Self {
        self.emblem = on;
        self
    }
    /// Wordmark font size (the emblem scales with it).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.clamp(24.0, 300.0);
        self
    }
    /// Dot screen pitch in points.
    pub fn pitch(mut self, p: f32) -> Self {
        self.pitch = p.clamp(3.0, 24.0);
        self
    }
    /// Screen angle in degrees.
    pub fn angle(mut self, deg: f32) -> Self {
        self.angle = deg;
        self
    }
    /// Light and deep ink (a gradient runs across the mark).
    pub fn colors(mut self, light: Color32, deep: Color32) -> Self {
        self.colors = Some([light, deep]);
        self
    }
    /// Strength of the rolling ink wave, `0..=2`.
    pub fn wave(mut self, w: f32) -> Self {
        self.wave = w.clamp(0.0, 2.0);
        self
    }
}

fn eclipse_coverage(p: Vec2, aa: f32) -> f32 {
    // a ring whose inner hole is pushed off-center, plus a small moon dot
    let disc = |c: Vec2, r: f32| ((r - (p - c).length()) / aa + 0.5).clamp(0.0, 1.0);
    let ring = (disc(Vec2::ZERO, 0.5) - disc(vec2(0.1, -0.08), 0.31)).max(0.0);
    ring.max(disc(vec2(0.1, -0.08), 0.1))
}

impl Widget for HalftoneLogo {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = display_font(ui, self.size);
        let mask = if self.word.is_empty() { None } else { Some(text_mask(ui.ctx(), &self.word, &font, false)) };
        let em = if self.emblem { self.size * 1.25 } else { 0.0 };
        let gap = if self.emblem && mask.is_some() { self.size * 0.18 } else { 0.0 };
        let mw = mask.as_ref().map(|m| m.size.x).unwrap_or(0.0);
        let mh = mask.as_ref().map(|m| m.size.y).unwrap_or(0.0);
        let total = vec2(em + gap + mw, em.max(mh));
        let (rect, response) = ui.allocate_exact_size(total + vec2(30.0, 30.0), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let time = glowkit_core::time(ui);
        let painter = ui.painter_at(rect.expand(10.0));
        let origin = rect.center() - total / 2.0;
        let em_c = origin + vec2(em / 2.0, total.y / 2.0);
        let word_o = origin + vec2(em + gap, (total.y - mh) / 2.0);
        let [light, deep] = self.colors.unwrap_or(if t.dark {
            [hex("#FFD3B0"), hex("#F2621C")]
        } else {
            [hex("#FF9A57"), hex("#C2410C")]
        });
        let cov = |p: Pos2| -> f32 {
            let mut c = 0.0f32;
            if em > 0.0 {
                let l = (p - em_c) / em;
                if l.x.abs() < 0.55 && l.y.abs() < 0.55 {
                    c = c.max(eclipse_coverage(l, 1.2 / em));
                }
            }
            if let Some(m) = &mask {
                let l = p - word_o;
                if l.x >= 0.0 && l.y >= 0.0 && l.x < m.size.x && l.y < m.size.y {
                    c = c.max(m.coverage(l.x / m.size.x, l.y / m.size.y));
                }
            }
            c
        };
        let a = self.angle.to_radians();
        let (s, c) = a.sin_cos();
        let pitch = self.pitch;
        let half = total.length() / 2.0 + pitch * 2.0;
        let n = (half / pitch).ceil() as i32;
        let center = origin + total / 2.0;
        let mut mesh = Mesh::default();
        let segs = if pitch > 8.0 { 10 } else { 7 };
        for j in -n..=n {
            for i in -n..=n {
                let l = vec2(i as f32 * pitch, j as f32 * pitch);
                let p = center + vec2(l.x * c - l.y * s, l.x * s + l.y * c);
                if (p.x - center.x).abs() > total.x / 2.0 + pitch || (p.y - center.y).abs() > total.y / 2.0 + pitch {
                    continue;
                }
                let k = cov(p);
                if k < 0.03 {
                    continue;
                }
                let u = (p.x - origin.x) / total.x.max(1.0);
                let phase = (p.x - origin.x) * 0.9 + (p.y - origin.y) * 0.5;
                let w = 0.5 + 0.5 * (phase / (pitch * 9.0) - time * 2.2).sin();
                let w = w * w * self.wave;
                let jitter = noise3(p.x * 0.05, p.y * 0.05, time * 0.4) * 0.12;
                let rr = pitch * 0.5 * k.sqrt() * (0.62 + 0.3 * w + jitter).min(1.05);
                let col = lerp_c(light, deep, (u * 1.1 - 0.1 + 0.25 * (1.0 - k)).clamp(0.0, 1.0));
                let col = lighten(col, w * 0.25);
                mesh_circle(&mut mesh, p, rr.max(0.4), segs, col);
            }
        }
        painter.add(Shape::mesh(mesh));
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A holographic foil card that tilts toward the pointer in 3D: a thin-film rainbow sheen whose
/// hues slide with the viewing angle, a soft glare that tracks the light, fine foil texture,
/// an iridescent rim and type printed on the tilted surface. A ghost hand tilts it when idle.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::HoloCard::new().title("Lifetime").subtitle("Every component, forever."));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct HoloCard {
    size: Vec2,
    eyebrow: String,
    title: String,
    subtitle: String,
    tilt: f32,
    foil: f32,
    base: Option<Color32>,
    autoplay: bool,
}

impl Default for HoloCard {
    fn default() -> Self {
        Self::new()
    }
}

impl HoloCard {
    /// A member card with placeholder copy.
    pub fn new() -> Self {
        Self {
            size: vec2(290.0, 400.0),
            eyebrow: "MEMBER · 0042".into(),
            title: "Lifetime".into(),
            subtitle: "Every component, forever.".into(),
            tilt: 16.0,
            foil: 1.0,
            base: None,
            autoplay: true,
        }
    }
    /// Card size.
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = s.max(vec2(120.0, 120.0));
        self
    }
    /// Small monospace line at the top.
    pub fn eyebrow(mut self, s: impl Into<String>) -> Self {
        self.eyebrow = s.into();
        self
    }
    /// Large title near the bottom.
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Line under the title.
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = s.into();
        self
    }
    /// Maximum tilt in degrees.
    pub fn tilt(mut self, deg: f32) -> Self {
        self.tilt = deg.clamp(0.0, 40.0);
        self
    }
    /// Foil intensity, `0..=2`.
    pub fn foil(mut self, k: f32) -> Self {
        self.foil = k.clamp(0.0, 2.0);
        self
    }
    /// Card base color (default near-black ink).
    pub fn base(mut self, c: Color32) -> Self {
        self.base = Some(c);
        self
    }
    /// Tilt with a ghost hand when the pointer is away (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for HoloCard {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(self.size + vec2(80.0, 50.0), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let id = response.id;
        let time = glowkit_core::time(ui);
        let card = Rect::from_center_size(rect.center(), self.size);
        let hover = response.hover_pos();
        let _ = idle_time(ui, id, hover.is_some());
        let (nx, ny) = match hover {
            Some(p) => (
                ((p.x - card.center().x) / (card.width() / 2.0)).clamp(-1.2, 1.2),
                ((p.y - card.center().y) / (card.height() / 2.0)).clamp(-1.2, 1.2),
            ),
            None if self.autoplay => ((time * 0.7).sin() * 0.8, (time * 0.53 + 0.8).sin() * 0.6),
            None => (0.0, 0.0),
        };
        let max = self.tilt.to_radians();
        let ry = glowkit_core::anim::spring(ui, id.with("ry"), -nx * max, 120.0, 14.0);
        let rx = glowkit_core::anim::spring(ui, id.with("rx"), ny * max, 120.0, 14.0);
        let lift = glowkit_core::anim::spring(ui, id.with("lift"), if hover.is_some() { 1.0 } else { 0.0 }, 200.0, 20.0);
        glowkit_core::animate(ui);
        let cam = Cam { center: card.center(), focal: 900.0 };
        let (w, h) = (self.size.x, self.size.y);
        let scale = 1.0 + 0.03 * lift;
        let map = |l: Vec2| -> Pos2 {
            let p = v3((l.x - w / 2.0) * scale, (l.y - h / 2.0) * scale, 0.0).rot_x(rx).rot_y(ry);
            cam.pt(p)
        };
        let painter = ui.painter_at(rect.expand(30.0));
        let radius = 22.0;
        let outline: Vec<Pos2> = glowkit_core::paint::rounded_rect_points(Rect::from_min_size(Pos2::ZERO, self.size), radius, 6)
            .into_iter()
            .map(|p| map(p.to_vec2()))
            .collect();
        let sh = if t.dark { Color32::from_black_alpha(120) } else { Color32::from_rgba_unmultiplied(30, 20, 60, 34) };
        let shadow: Vec<Pos2> = outline.iter().map(|p| card.center() + (*p - card.center()) * 0.9).collect();
        poly_shadow(&painter, &shadow, vec2(-ry * 50.0, 26.0 + rx * 24.0 + 8.0 * lift), 34.0, sh);
        let base = self.base.unwrap_or(hex("#121219"));
        let foil = self.foil;
        let glare = vec2(0.5 - ry * 2.4, 0.42 + rx * 2.4);
        surface_grid(&painter, None, Rect::ZERO, 18, 24, self.size, radius, |u, v| map(vec2(u * w, v * h)), |u, v| {
            let n = noise3(u * 2.2, v * 2.6, time * 0.15);
            let hue = 360.0 * (0.55 * u + 0.75 * v + 1.9 * ry - 1.6 * rx + 0.35 * n) + time * 8.0;
            let film = hsv(hue, 0.62, 1.0);
            let band = 0.5 + 0.5 * ((u * 0.8 + v * 1.4 + ry * 2.0 - rx * 1.5) * TAU * 0.9 + n * 2.0).sin();
            let k = (0.10 + 0.24 * band * band) * foil;
            let mut c = lerp_c(base, film, k.min(0.9));
            let d = (vec2(u, v) - glare).length();
            let g = (-(d * d) / 0.07).exp();
            c = lighten(c, g * 0.22 * foil.min(1.2));
            // foil grain
            let gr = hash3((u * 400.0) as i32, (v * 520.0) as i32, 3) - 0.5;
            lighten(c, (gr * 0.06).max(0.0))
        });
        // iridescent rim
        let rim: Vec<Pos2> = outline.clone();
        glowkit_core::paint::gradient_polyline(&painter, &rim, true, 1.2, |s| {
            with_alpha(hsv(s * 720.0 + ry * 400.0 + time * 30.0, 0.35, 1.0), 0.28)
        });
        // type on the surface
        let fg = Color32::from_rgb(246, 246, 250);
        let mono = FontId::monospace(11.0);
        let g1 = painter.layout_no_wrap(spaced(&self.eyebrow), mono, fg);
        galley_mapped(&painter, &g1, with_alpha(fg, 0.6), |l| map(vec2(30.0, 32.0) + l));
        let g2 = painter.layout_no_wrap(self.title.clone(), display_font(ui, 38.0), fg);
        let ty = h - 92.0;
        galley_mapped(&painter, &g2, fg, |l| map(vec2(30.0, ty) + l));
        let g3 = painter.layout_no_wrap(self.subtitle.clone(), FontId::proportional(14.0), fg);
        galley_mapped(&painter, &g3, with_alpha(fg, 0.62), |l| map(vec2(30.0, ty + 50.0) + l));
        response
    }
}

fn spaced(s: &str) -> String {
    let mut o = String::with_capacity(s.len() * 2);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 {
            o.push(' ');
        }
        o.push(ch);
    }
    o
}
