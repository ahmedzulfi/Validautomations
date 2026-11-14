//! Tactile 3D-ish buttons and characters: a goo button, a web-slinging button, a velvet cushion,
//! a glass keycap and a glass capsule sprite.

use super::common::{h01, idle_secs, lerp_rgb, lum, strip, Cam, V3, v3};
use egui::epaint::{Mesh, PathShape, PathStroke};
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
use glowkit_core::anim::Spring;
use glowkit_core::color::{hex, with_alpha};
use glowkit_core::noise::{noise2, noise3};
use glowkit_core::{paint, Theme};
use std::f32::consts::{PI, TAU};

/// Paint a cartoon glove cursor whose index fingertip sits at `tip`.
pub(crate) fn glove(painter: &Painter, tip: Pos2, scale: f32, press: f32, ink: Color32) {
    let s = scale;
    let p = |x: f32, y: f32| tip + vec2(x, y + press * 2.0) * s;
    let fill = Color32::WHITE;
    let o = 1.6 * s;
    let capsule = |a: Pos2, b: Pos2, w: f32, c: Color32| {
        painter.line_segment([a, b], Stroke::new(w, c));
        painter.circle_filled(a, w * 0.5, c);
        painter.circle_filled(b, w * 0.5, c);
    };
    let rrect = |r: Rect, c: Color32| painter.rect_filled(r, 4.0 * s, c);
    for pass in 0..2 {
        let (c, e) = if pass == 0 { (ink, o) } else { (fill, 0.0) };
        capsule(p(0.0, 1.0), p(0.0, 13.0 - press * 4.0), 7.0 * s + e * 2.0, c);
        capsule(p(-5.0, 18.0), p(-8.5, 13.0), 6.0 * s + e * 2.0, c);
        rrect(Rect::from_min_max(p(-5.0, 10.0), p(13.5, 27.0)).expand(e), c);
        painter.circle_filled(p(6.5, 12.0), 3.6 * s + e, c);
        painter.circle_filled(p(10.5, 13.5), 3.4 * s + e, c);
        rrect(Rect::from_min_max(p(-3.0, 26.0), p(12.0, 31.0)).expand(e), c);
    }
    // knuckle creases + cuff line
    let st = Stroke::new(1.0 * s, ink);
    painter.line_segment([p(4.0, 15.0), p(4.0, 18.5)], st);
    painter.line_segment([p(8.5, 16.0), p(8.5, 19.0)], st);
    painter.line_segment([p(-3.0, 26.0), p(12.0, 26.0)], st);
}

/// Points on a pill outline (rounded rect with radius h/2).
fn pill_points(r: Rect, n_cap: usize) -> Vec<Pos2> {
    paint::rounded_rect_points(r, r.height() * 0.5, n_cap)
}

/// Closest point on the pill boundary to `p`, with outward normal.
fn pill_closest(r: Rect, p: Pos2) -> (Pos2, Vec2) {
    let rad = r.height() * 0.5;
    let a = pos2(r.min.x + rad, r.center().y);
    let b = pos2(r.max.x - rad, r.center().y);
    let x = p.x.clamp(a.x, b.x);
    let c = pos2(x, r.center().y);
    let d = p - c;
    let n = if d.length_sq() > 1e-6 { d.normalized() } else { vec2(0.0, -1.0) };
    (c + n * rad, n)
}

fn ghost_path(pill: Rect, rect: Rect, time: f32) -> Pos2 {
    // wanders below the button, now and then swinging far enough away to break contact
    let a = time * 0.5;
    let swing = a.sin() * (0.22 + 0.2 * (a * 0.5).sin().abs());
    pos2(pill.center().x + swing * rect.width(), pill.max.y + 46.0 + (a * 2.0).cos() * 30.0)
}

// ---------------------------------------------------------------------------------------------
// Venom Button
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct ReachState {
    reach: Spring,
    anchor: Pos2,
    snap: f32,
}

/// A glossy black goo button: when the pointer comes close, liquid strands stretch out of the
/// pill, wrap a cartoon glove cursor, and snap back with a wobble when you pull away.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// if ui.add(glowkit_interactions::VenomButton::new("We are Venom")).clicked() { /* ... */ }
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct VenomButton {
    label: String,
    size: Option<Vec2>,
    strands: usize,
    reach: f32,
    color: Option<Color32>,
    autoplay: bool,
}

impl VenomButton {
    /// A goo button with `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), size: None, strands: 6, reach: 150.0, color: None, autoplay: true }
    }
    /// Widget area size (the pill sits in the middle; default 380 x 240).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Number of goo strands (1..12).
    pub fn strands(mut self, n: usize) -> Self {
        self.strands = n.clamp(1, 12);
        self
    }
    /// How far the goo reaches before snapping, in points.
    pub fn reach(mut self, r: f32) -> Self {
        self.reach = r.max(30.0);
        self
    }
    /// Goo color (default glossy black).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Play a ghost glove while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for VenomButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = self.size.unwrap_or(vec2(ui.available_width().min(520.0), 240.0));
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let font = FontId::proportional(22.0);
        let tw = ui.painter().layout_no_wrap(self.label.clone(), font.clone(), Color32::WHITE).size().x;
        let pill = Rect::from_center_size(rect.center() - vec2(0.0, 16.0), vec2(tw + 72.0, 62.0));
        let id = response.id;
        let time = glowkit_core::time(ui);
        let dt = glowkit_core::dt(ui);
        let hover = response.hover_pos();
        let idle = idle_secs(ui, id, hover.is_some());
        let ghost = self.autoplay && hover.is_none() && idle > 1.5;
        let pointer = hover.or(if ghost { Some(ghost_path(pill, rect, time)) } else { None });
        if hover.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::None);
        }
        let press = glowkit_core::anim::spring(ui, id.with("press"), if response.is_pointer_button_down_on() { 1.0 } else { 0.0 }, 500.0, 25.0);
        let (near, dist) = match pointer {
            Some(p) => {
                let (c, _) = pill_closest(pill, p);
                let inside = pill.contains(p);
                (true, if inside { 0.0 } else { c.distance(p) })
            }
            None => (false, f32::MAX),
        };
        let reach = self.reach;
        let st = glowkit_core::with_state(
            ui,
            id.with("venom"),
            || ReachState { reach: Spring::new(0.0).with(160.0, 9.0), anchor: pill.center(), snap: 0.0 },
            |s| {
                let attached = s.reach.target > 0.5;
                let want = near && if attached { dist < reach * 1.25 } else { dist < reach * 0.8 };
                if attached && !want {
                    s.snap = 1.0;
                }
                s.reach.target = if want { 1.0 } else { 0.0 };
                s.reach.step(dt);
                s.snap = (s.snap - dt * 2.5).max(0.0);
                if let Some(p) = pointer {
                    if want {
                        s.anchor = p;
                    }
                }
                *s
            },
        );
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let t = Theme::of(ui);
        let painter = ui.painter_at(rect.expand(4.0));
        let goo = self.color.unwrap_or(hex("#0B0B0E"));
        let hi = lerp_rgb(goo, Color32::WHITE, 0.55);
        let k = st.reach.value.clamp(-0.2, 1.3);
        // shadow
        paint::soft_shadow(&painter, pill.translate(vec2(0.0, 8.0)), pill.height() * 0.5, 18.0, Vec2::ZERO, with_alpha(Color32::BLACK, if t.dark { 0.5 } else { 0.22 }));
        // strands
        let target = st.anchor;
        let (base, nrm) = pill_closest(pill, target);
        let mut mesh = Mesh::default();
        let mut gloss = Mesh::default();
        if k > 0.01 {
            let n = self.strands;
            for i in 0..n {
                let f = if n > 1 { i as f32 / (n - 1) as f32 - 0.5 } else { 0.0 };
                let tangent = nrm.rot90();
                let b0 = base + tangent * f * 46.0 - nrm * 6.0;
                // wrap around the glove: endpoints fan around the hand
                let ang = f * 2.4 + (time * 2.0 + i as f32).sin() * 0.1;
                let wrap = target + vec2(6.0, 14.0) + vec2(ang.sin(), -ang.cos()) * 13.0;
                let end = b0.lerp(wrap, k.min(1.0));
                let mid = b0.lerp(end, 0.5) + nrm * 12.0 * k + tangent * f * 14.0 + vec2(0.0, 10.0 * (1.0 - k));
                let wob = (time * 6.0 + i as f32 * 1.7).sin() * 3.0 * (1.0 - k).abs();
                let pts = paint::quad_bezier(b0, mid + tangent * wob, end, 22);
                let len = pts.len();
                let w = |j: usize| {
                    let s = j as f32 / (len - 1) as f32;
                    (13.0 * (1.0 - s).powf(1.6) + 3.0) * (0.6 + 0.4 * k.min(1.0))
                };
                strip(&mut mesh, &pts, w, |_| goo, |_| Color32::TRANSPARENT);
                let gl: Vec<Pos2> = pts.iter().map(|p| *p - nrm.rot90() * 1.2 - vec2(0.0, 1.2)).collect();
                strip(&mut gloss, &gl[2..len - 3], |_| 1.1, |j| lum(hi, 0.5 * (1.0 - j as f32 / len as f32), false), |_| Color32::TRANSPARENT);
                // goo blob where the strand grips
                if k > 0.6 {
                    painter.circle_filled(end, 3.2 * k.min(1.0), goo);
                }
            }
            // bulge at the root
            let bulge = base + nrm * 4.0 * k;
            painter.circle_filled(bulge, 18.0 * k.min(1.0).max(0.0), goo);
        }
        painter.add(Shape::mesh(mesh));
        // the pill
        let pr = pill.translate(vec2(0.0, press * 2.0)).shrink(press * 1.5);
        let r = pr.height() * 0.5;
        painter.rect_filled(pr, r, goo);
        // gloss: top sheen, rim light
        paint::gradient_rect(&painter, Rect::from_min_max(pr.min + vec2(r * 0.5, 4.0), pos2(pr.max.x - r * 0.5, pr.center().y)), r * 0.6, 90.0, &[with_alpha(Color32::WHITE, 0.32), with_alpha(Color32::WHITE, 0.0)]);
        painter.rect_stroke(pr, r, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.22)), egui::StrokeKind::Inside);
        painter.circle_filled(pos2(pr.min.x + r * 0.75, pr.min.y + r * 0.45), 3.0, with_alpha(Color32::WHITE, 0.85));
        painter.add(Shape::mesh(gloss));
        painter.text(pr.center() + vec2(0.0, 1.0), Align2::CENTER_CENTER, &self.label, font, Color32::WHITE);
        // snap droplets
        if st.snap > 0.0 {
            for i in 0..7 {
                let a = h01(i as u64 * 31) * TAU;
                let d = (1.0 - st.snap) * 40.0;
                let p = base + nrm * 10.0 + vec2(a.cos(), a.sin()) * d;
                painter.circle_filled(p, 3.0 * st.snap, goo);
            }
        }
        if let Some(p) = pointer {
            glove(&painter, p, 1.15, press, hex("#111114"));
        }
        response.on_hover_cursor(CursorIcon::None)
    }
}

// ---------------------------------------------------------------------------------------------
// Spider Button
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
struct WebState {
    shot: Spring,
    attached: bool,
    anchor: Pos2,
    splats: Vec<(Pos2, f32, f32)>, // pos, age, seed
}

fn cobweb(painter: &Painter, center: Pos2, r: f32, a0: f32, a1: f32, col: Color32, seed: u64) {
    let spokes = 6;
    let st = Stroke::new(0.8, col);
    let mut ends = Vec::new();
    for i in 0..=spokes {
        let a = a0 + (a1 - a0) * i as f32 / spokes as f32;
        let l = r * (0.75 + 0.25 * h01(seed + i as u64));
        let e = center + vec2(a.cos(), a.sin()) * l;
        painter.line_segment([center, e], st);
        ends.push((a, l));
    }
    for ring in 1..=4 {
        let f = ring as f32 / 4.5;
        let mut pts = Vec::new();
        for (a, l) in &ends {
            pts.push(center + vec2(a.cos(), a.sin()) * *l * f);
        }
        for w in pts.windows(2) {
            let mid = w[0].lerp(w[1], 0.5).lerp(center, 0.08);
            painter.add(Shape::line(paint::quad_bezier(w[0], mid, w[1], 6), st));
        }
    }
}

fn splat(painter: &Painter, c: Pos2, r: f32, col: Color32, seed: f32) {
    let st = Stroke::new(1.0, col);
    let n = 8;
    let mut rim = Vec::new();
    for i in 0..n {
        let a = i as f32 / n as f32 * TAU + seed;
        let l = r * (0.75 + 0.3 * h01((seed * 1000.0) as u64 + i as u64));
        painter.line_segment([c, c + vec2(a.cos(), a.sin()) * l], st);
        rim.push(c + vec2(a.cos(), a.sin()) * l * 0.6);
    }
    rim.push(rim[0]);
    painter.add(Shape::line(rim, st));
}

/// A glossy red pill button that shoots sagging web lines at the pointer when it comes close,
/// with corner cobwebs, a web splat on click and a snap-back when you pull away.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::SpiderButton::new("Spider Button"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SpiderButton {
    label: String,
    size: Option<Vec2>,
    lines: usize,
    reach: f32,
    color: Option<Color32>,
    autoplay: bool,
}

impl SpiderButton {
    /// A web button with `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), size: None, lines: 4, reach: 170.0, color: None, autoplay: true }
    }
    /// Widget area size (default 380 x 240).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Number of web lines (1..8).
    pub fn lines(mut self, n: usize) -> Self {
        self.lines = n.clamp(1, 8);
        self
    }
    /// Web reach in points.
    pub fn reach(mut self, r: f32) -> Self {
        self.reach = r.max(30.0);
        self
    }
    /// Pill color (default a glossy red).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Play a ghost glove while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for SpiderButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = self.size.unwrap_or(vec2(ui.available_width().min(520.0), 240.0));
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let font = FontId::proportional(22.0);
        let tw = ui.painter().layout_no_wrap(self.label.clone(), font.clone(), Color32::WHITE).size().x;
        let pill = Rect::from_center_size(rect.center() - vec2(0.0, 16.0), vec2(tw + 72.0, 62.0));
        let id = response.id;
        let time = glowkit_core::time(ui);
        let dt = glowkit_core::dt(ui);
        let hover = response.hover_pos();
        let idle = idle_secs(ui, id, hover.is_some());
        let ghost = self.autoplay && hover.is_none() && idle > 1.5;
        let pointer = hover.or(if ghost { Some(ghost_path(pill, rect, time * 0.9 + 1.0)) } else { None });
        let ghost_click = ghost && (time * 0.45).fract() < dt * 0.45;
        let clicked = response.clicked() || ghost_click;
        let press = glowkit_core::anim::spring(ui, id.with("press"), if response.is_pointer_button_down_on() { 1.0 } else { 0.0 }, 500.0, 25.0);
        let dist = pointer.map(|p| if pill.contains(p) { 0.0 } else { pill_closest(pill, p).0.distance(p) }).unwrap_or(f32::MAX);
        let reach = self.reach;
        let st = glowkit_core::with_state(ui, id.with("spider"), WebState::default, |s| {
            s.shot.stiffness = 260.0;
            s.shot.damping = 18.0;
            let want = pointer.is_some() && dist > 12.0 && if s.attached { dist < reach * 1.2 } else { dist < reach };
            s.attached = want;
            s.shot.target = if want { 1.0 } else { 0.0 };
            if want {
                s.anchor = pointer.unwrap_or(s.anchor);
            }
            s.shot.step(dt);
            for sp in s.splats.iter_mut() {
                sp.1 += dt;
            }
            s.splats.retain(|sp| sp.1 < 2.2);
            if clicked {
                if let Some(p) = pointer {
                    s.splats.push((p, 0.0, h01((time * 1000.0) as u64)));
                }
            }
            s.clone()
        });
        if hover.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::None);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let t = Theme::of(ui);
        let painter = ui.painter_at(rect.expand(4.0));
        let red = self.color.unwrap_or(hex("#E3132B"));
        let web = if t.dark { with_alpha(Color32::WHITE, 0.85) } else { with_alpha(hex("#3F3F46"), 0.85) };
        paint::soft_shadow(&painter, pill.translate(vec2(0.0, 8.0)), pill.height() * 0.5, 18.0, Vec2::ZERO, with_alpha(red, if t.dark { 0.35 } else { 0.25 }));
        // splats behind the glove
        for (p, age, seed) in &st.splats {
            let grow = (age / 0.18).min(1.0);
            let fade = (1.0 - (age - 1.4).max(0.0) / 0.8).clamp(0.0, 1.0);
            splat(&painter, *p, 18.0 * grow, with_alpha(web, fade), *seed);
        }
        // the pill
        let pr = pill.translate(vec2(0.0, press * 2.0)).shrink(press * 1.5);
        let r = pr.height() * 0.5;
        paint::gradient_rect(&painter, pr, r, 90.0, &[lerp_rgb(red, Color32::WHITE, 0.12), red, lerp_rgb(red, Color32::BLACK, 0.35)]);
        paint::gradient_rect(&painter, Rect::from_min_max(pr.min + vec2(r * 0.6, 4.0), pos2(pr.max.x - r * 0.6, pr.center().y - 2.0)), r * 0.5, 90.0, &[with_alpha(Color32::WHITE, 0.55), with_alpha(Color32::WHITE, 0.0)]);
        painter.rect_stroke(pr, r, Stroke::new(1.0, lerp_rgb(red, Color32::BLACK, 0.35)), egui::StrokeKind::Inside);
        painter.text(pr.center() + vec2(0.0, 2.0), Align2::CENTER_CENTER, &self.label, font.clone(), with_alpha(Color32::BLACK, 0.3));
        painter.text(pr.center(), Align2::CENTER_CENTER, &self.label, font, Color32::WHITE);
        // corner cobwebs
        let cw = with_alpha(web, 0.55);
        cobweb(&painter, pos2(pr.max.x - r * 0.55, pr.min.y + 1.0), 20.0, PI * 0.05, PI * 0.65, cw, 11);
        cobweb(&painter, pos2(pr.min.x + r * 0.55, pr.max.y - 1.0), 18.0, PI * 1.05, PI * 1.62, cw, 29);
        // web lines
        let k = st.shot.value;
        if k > 0.02 {
            let target = st.anchor;
            let (base, nrm) = pill_closest(pill, target);
            let tangent = nrm.rot90();
            let n = self.lines;
            for i in 0..n {
                let f = if n > 1 { i as f32 / (n - 1) as f32 - 0.5 } else { 0.0 };
                let b0 = base + tangent * f * 26.0;
                let end_t = target + vec2(4.0, 12.0) + vec2(f * 10.0, f.abs() * 6.0);
                let end = b0.lerp(end_t, k.min(1.0));
                let len = b0.distance(end);
                let sag = (len / 10.0) * (1.0 - k.min(1.0) * 0.6) + (time * 3.0 + i as f32).sin() * 1.5;
                let mid = b0.lerp(end, 0.5) + vec2(0.0, sag);
                painter.add(Shape::line(paint::quad_bezier(b0, mid, end, 16), Stroke::new(1.1, web)));
            }
            if k > 0.85 {
                splat(&painter, target + vec2(4.0, 12.0), 9.0, with_alpha(web, 0.8), 0.3);
            }
        }
        if let Some(p) = pointer {
            glove(&painter, p, 1.1, press, hex("#111114"));
        }
        response.on_hover_cursor(CursorIcon::None)
    }
}

// ---------------------------------------------------------------------------------------------
// Velour Bolster
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct Cushion {
    inflate: Spring,
    jx: Spring,
    jy: Spring,
    dent: Spring,
    last: Pos2,
}

/// A velvet pill button that inflates into a soft cushion under the pointer: velvet sheen
/// brightens at the grazing edges, the pile ripples, it jiggles as you move, dents when pressed
/// and settles flat when you leave.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::VelourBolster::new("HOVER ME"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct VelourBolster {
    label: String,
    color: Option<Color32>,
    size: Vec2,
    puff: f32,
    autoplay: bool,
}

impl VelourBolster {
    /// A velvet button with `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), color: None, size: vec2(230.0, 80.0), puff: 1.0, autoplay: true }
    }
    /// Velvet color (default moss green).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Button size.
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = s;
        self
    }
    /// How much it inflates (0..2).
    pub fn puff(mut self, k: f32) -> Self {
        self.puff = k.clamp(0.0, 2.0);
        self
    }
    /// Inflate with a ghost pointer while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for VelourBolster {
    fn ui(self, ui: &mut Ui) -> Response {
        let pad = vec2(40.0, 40.0);
        let (outer, response) = ui.allocate_exact_size(self.size + pad * 2.0, Sense::click());
        let response = response.on_hover_cursor(CursorIcon::PointingHand);
        let rect = Rect::from_center_size(outer.center(), self.size);
        let id = response.id;
        let time = glowkit_core::time(ui);
        let dt = glowkit_core::dt(ui);
        let hover = response.hover_pos().filter(|p| rect.expand(10.0).contains(*p));
        let idle = idle_secs(ui, id, response.hovered());
        let ghost = self.autoplay && hover.is_none() && idle > 1.5 && (time % 5.0) < 3.2;
        let pointer = hover.or(if ghost {
            let a = time * 1.3;
            Some(rect.center() + vec2(a.sin() * rect.width() * 0.32, (a * 1.7).cos() * rect.height() * 0.22))
        } else {
            None
        });
        let pressed = response.is_pointer_button_down_on() || (ghost && (time % 5.0) > 2.2 && (time % 5.0) < 2.6);
        let st = glowkit_core::with_state(
            ui,
            id.with("velour"),
            || Cushion {
                inflate: Spring::new(0.0).with(220.0, 11.0),
                jx: Spring::new(0.0).with(260.0, 7.0),
                jy: Spring::new(0.0).with(260.0, 7.0),
                dent: Spring::new(0.0).with(500.0, 20.0),
                last: rect.center(),
            },
            |s| {
                s.inflate.target = if pointer.is_some() { 1.0 } else { 0.0 };
                s.dent.target = if pressed { 1.0 } else { 0.0 };
                if let Some(p) = pointer {
                    let d = p - s.last;
                    s.jx.velocity += d.x * 0.6;
                    s.jy.velocity += d.y * 0.6;
                    s.last = p;
                }
                for sp in [&mut s.inflate, &mut s.jx, &mut s.jy, &mut s.dent] {
                    sp.step(dt);
                }
                *s
            },
        );
        let moving = !(st.inflate.is_settled() && st.jx.is_settled() && st.jy.is_settled() && st.dent.is_settled());
        if moving || self.autoplay {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(outer) {
            return response;
        }
        let t = Theme::of(ui);
        let painter = ui.painter();
        let base = self.color.unwrap_or(hex("#4D8C1F"));
        let inf = st.inflate.value * self.puff;
        let dent = st.dent.value;
        let jig = vec2(st.jx.value, st.jy.value) * 0.02;
        let grow = vec2(10.0, 8.0) * inf - vec2(4.0, 3.0) * dent;
        let r = Rect::from_center_size(rect.center() + jig * 0.5, self.size + grow + vec2(jig.x.abs(), -jig.x.abs() * 0.5));
        let pr = r.height() * 0.5;
        paint::soft_shadow(painter, r.translate(vec2(0.0, 6.0 + inf * 6.0)), pr, 14.0 + inf * 10.0, Vec2::ZERO, with_alpha(Color32::BLACK, if t.dark { 0.55 } else { 0.25 }));
        let ptr_local = pointer.unwrap_or(r.center());
        let light = v3(-0.45, -0.6, 0.66).norm();
        let height = |p: Pos2| -> f32 {
            // cushion profile: rounded toward the edge, bulge at the pointer, dent when pressed
            let dx = ((p.x - r.center().x).abs() - (r.width() * 0.5 - pr)).max(0.0);
            let dy = p.y - r.center().y;
            let e = 1.0 - ((dx * dx + dy * dy).sqrt() / pr).min(1.0);
            let edge = e.max(0.0).sqrt() * (0.85 + inf * 0.6);
            let d2 = (p - ptr_local).length_sq() / (pr * pr * 1.4);
            let bump = (-d2).exp() * inf * 0.45 - (-d2 * 2.0).exp() * dent * 0.5;
            (edge + bump * e.min(1.0).sqrt()) * pr
        };
        let col = |p: Pos2| -> Color32 {
            let e = 1.5;
            let hx = height(p + vec2(e, 0.0)) - height(p - vec2(e, 0.0));
            let hy = height(p + vec2(0.0, e)) - height(p - vec2(0.0, e));
            let n = v3(-hx / (2.0 * e), -hy / (2.0 * e), 1.0).norm();
            let diff = n.dot(light).max(0.0);
            let sheen = (1.0 - n.z).powf(1.6); // velvet glows at grazing angles
            let pile = noise2(p.x * 0.25 + time * 0.2, p.y * 0.9) * 0.5 + noise3(p.x * 0.06, p.y * 0.06, time * 0.3) * 0.5;
            let dark = lerp_rgb(base, Color32::BLACK, 0.55);
            let mut c = lerp_rgb(dark, base, (0.35 + diff * 0.75).min(1.0));
            c = lerp_rgb(c, lerp_rgb(base, hex("#E8FFD0"), 0.55), (sheen * 1.3 + pile * 0.08).clamp(0.0, 1.0));
            lerp_rgb(c, Color32::BLACK, (-pile * 0.18).max(0.0))
        };
        let pts = paint::rounded_rect_points(r, pr, 10);
        paint::fill_convex_fn(painter, &pts, 14, col);
        // embossed label
        let font = FontId::proportional((r.height() * 0.3).clamp(12.0, 26.0));
        let lc = r.center() + vec2(0.0, dent * 1.5);
        let ink = lerp_rgb(base, hex("#F0FFE0"), 0.8);
        painter.text(lc + vec2(0.0, 1.5), Align2::CENTER_CENTER, &self.label, font.clone(), with_alpha(Color32::BLACK, 0.35));
        painter.text(lc, Align2::CENTER_CENTER, &self.label, font, ink);
        response
    }
}

// ---------------------------------------------------------------------------------------------
// Glass Keycap
// ---------------------------------------------------------------------------------------------

/// A single glass keycap that refracts colored studio light into rainbow fringes: it floats,
/// tilts to follow the pointer and presses down when clicked. The legend is printed on top.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::GlassKeycap::new("esc."));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct GlassKeycap {
    legend: String,
    size: f32,
    colors: Option<[Color32; 3]>,
    float: f32,
}

impl GlassKeycap {
    /// A keycap printed with `legend`.
    pub fn new(legend: impl Into<String>) -> Self {
        Self { legend: legend.into(), size: 120.0, colors: None, float: 1.0 }
    }
    /// Keycap width in points.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.clamp(40.0, 260.0);
        self
    }
    /// The three studio light colors refracted through the glass.
    pub fn colors(mut self, c: [Color32; 3]) -> Self {
        self.colors = Some(c);
        self
    }
    /// Float (bobbing) amount, 0..2.
    pub fn float(mut self, k: f32) -> Self {
        self.float = k.clamp(0.0, 2.0);
        self
    }
}

impl Widget for GlassKeycap {
    fn ui(self, ui: &mut Ui) -> Response {
        let side = self.size * 2.3;
        let (rect, response) = ui.allocate_exact_size(vec2(side * 1.3, side), Sense::click());
        let response = response.on_hover_cursor(CursorIcon::PointingHand);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let id = response.id;
        let t = Theme::of(ui);
        let time = glowkit_core::time(ui);
        let target = match response.hover_pos() {
            Some(p) => ((p - rect.center()) / (rect.size() * 0.5)).clamp(Vec2::splat(-1.0), Vec2::splat(1.0)),
            None => vec2((time * 0.5).sin() * 0.6, (time * 0.37).cos() * 0.4),
        };
        let lean = glowkit_core::anim::spring2(ui, id.with("lean"), target, 70.0, 11.0);
        let auto_press = response.hover_pos().is_none() && (time % 3.6) > 3.25;
        let press = glowkit_core::anim::spring(ui, id.with("press"), if response.is_pointer_button_down_on() || auto_press { 1.0 } else { 0.0 }, 600.0, 18.0);
        let painter = ui.painter_at(rect);
        let [c1, c2, c3] = self.colors.unwrap_or([hex("#FF7A1A"), hex("#FF2E93"), hex("#2E6BFF")]);
        let bob = (time * 1.4).sin() * 6.0 * self.float;
        let cam = Cam { center: rect.center() + vec2(0.0, bob + press * 8.0), focal: self.size * 4.6, dist: 6.0 };
        let yaw = 0.32 + lean.x * 0.45;
        let pitch = 0.5 - lean.y * 0.3;
        let hgt = 0.62 - press * 0.12;
        let tf = |p: V3| p.rot_y(yaw).rot_x(pitch);
        // keycap: bottom square ±1, top ±0.7 dished slightly back
        let bot = [v3(-1.0, -0.35, -1.0), v3(1.0, -0.35, -1.0), v3(1.0, -0.35, 1.0), v3(-1.0, -0.35, 1.0)];
        let top = [v3(-0.7, hgt - 0.35, -0.75), v3(0.7, hgt - 0.35, -0.75), v3(0.7, hgt - 0.35, 0.65), v3(-0.7, hgt - 0.35, 0.65)];
        let pb: Vec<(Pos2, V3)> = bot.iter().map(|p| (cam.project(tf(*p)).map(|x| x.0).unwrap_or(rect.center()), tf(*p))).collect();
        let pt: Vec<(Pos2, V3)> = top.iter().map(|p| (cam.project(tf(*p)).map(|x| x.0).unwrap_or(rect.center()), tf(*p))).collect();
        // shadow + under-glow
        let floor = rect.center() + vec2(0.0, self.size * 0.62);
        paint::glow_ellipse(&painter, floor, vec2(self.size * 0.9 * (1.0 - bob / 60.0), self.size * 0.16), with_alpha(Color32::BLACK, if t.dark { 0.6 } else { 0.18 }));
        if t.dark {
            paint::glow(&painter, cam.center, self.size * 1.1, lum(c2, 0.08, true));
        }
        // faces: sides (4) + top; draw back-to-front by average depth
        let mut faces: Vec<(f32, [Pos2; 4], usize)> = Vec::new();
        for i in 0..4 {
            let j = (i + 1) % 4;
            let quad = [pb[i].0, pb[j].0, pt[j].0, pt[i].0];
            let z = (pb[i].1.z + pb[j].1.z + pt[i].1.z + pt[j].1.z) * 0.25;
            faces.push((z, quad, i));
        }
        faces.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let glass_tint = if t.dark { hex("#0A0A12") } else { hex("#F4F4FA") };
        let face_fill = |quad: &[Pos2; 4], k: usize, front: bool| {
            let center = quad.iter().fold(Vec2::ZERO, |a, p| a + p.to_vec2()) / 4.0;
            let span = (quad[0] - quad[1]).length().max(1.0);
            let phase = time * 0.4 + k as f32 * 1.7 + lean.x * 1.5;
            let f = move |p: Pos2| -> Color32 {
                let u = (p.to_vec2() - center).dot((quad[1] - quad[0]).normalized()) / span + 0.5;
                let band = |off: f32| (-((u - off - 0.5 * (phase).sin() * 0.3).powi(2)) / 0.006).exp();
                let mut c = glass_tint;
                let a = if front { 0.75 } else { 0.4 };
                c = lerp_rgb(c, c1, band(0.2) * a);
                c = lerp_rgb(c, c2, band(0.5) * a);
                c = lerp_rgb(c, c3, band(0.8) * a);
                with_alpha(c, if t.dark { 0.78 } else { 0.62 })
            };
            paint::fill_convex_fn(&painter, quad, 4, f);
        };
        for (z, quad, k) in &faces {
            face_fill(quad, *k, *z > 0.0);
        }
        // top face
        let tq = [pt[0].0, pt[1].0, pt[2].0, pt[3].0];
        let tc = tq.iter().fold(Vec2::ZERO, |a, p| a + p.to_vec2()).to_pos2() / 4.0;
        paint::fill_convex_fn(&painter, &tq, 6, |p| {
            let d = (p - tc).length() / (self.size * 0.6);
            let ang = (p - tc).angle() + time * 0.3;
            let cc = glowkit_core::color::gradient_cyclic(&[c1, c2, c3], ang / TAU + 0.5);
            with_alpha(lerp_rgb(glass_tint, cc, (d * d * 1.3).min(0.85)), if t.dark { 0.9 } else { 0.8 })
        });
        // edges with chromatic fringes
        let edge = |a: Pos2, b: Pos2, w: f32| {
            let n = (b - a).normalized().rot90() * 1.2;
            painter.line_segment([a + n, b + n], Stroke::new(w, lum(c1, 0.55, t.dark)));
            painter.line_segment([a - n, b - n], Stroke::new(w, lum(c3, 0.55, t.dark)));
            let core = if t.dark { with_alpha(Color32::WHITE, 0.85) } else { with_alpha(hex("#27272A"), 0.75) };
            painter.line_segment([a, b], Stroke::new(w * 0.8, core));
        };
        for i in 0..4 {
            let j = (i + 1) % 4;
            edge(pt[i].0, pt[j].0, 1.6);
            edge(pb[i].0, pt[i].0, 1.2);
        }
        for i in 0..4 {
            let j = (i + 1) % 4;
            if pb[i].1.z + pb[j].1.z > -0.4 {
                edge(pb[i].0, pb[j].0, 1.4);
            }
        }
        // legend on the top face (affine-mapped center)
        let font = FontId::proportional(self.size * 0.2);
        let ink = if t.dark { Color32::WHITE } else { hex("#18181B") };
        painter.text(tc, Align2::CENTER_CENTER, &self.legend, font, ink);
        // specular glints
        paint::glow(&painter, pt[0].0.lerp(pt[1].0, 0.3), self.size * 0.08, lum(Color32::WHITE, 0.7, true));
        response
    }
}

// ---------------------------------------------------------------------------------------------
// Cobalt Sprite
// ---------------------------------------------------------------------------------------------

/// A blue sprite with two pill eyes inside a thick glass capsule (clear above, frosted below).
/// It looks toward the pointer, drifts inside the glass and blinks now and then.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::CobaltSprite::new().radius(110.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CobaltSprite {
    radius: f32,
    color: Option<Color32>,
    frost: f32,
}

impl Default for CobaltSprite {
    fn default() -> Self {
        Self::new()
    }
}

impl CobaltSprite {
    /// A sprite with default settings.
    pub fn new() -> Self {
        Self { radius: 110.0, color: None, frost: 0.6 }
    }
    /// Capsule radius in points.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.clamp(30.0, 240.0);
        self
    }
    /// Sprite color (default cobalt blue).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Frosting of the lower half (0..1).
    pub fn frost(mut self, k: f32) -> Self {
        self.frost = k.clamp(0.0, 1.0);
        self
    }
}

impl Widget for CobaltSprite {
    fn ui(self, ui: &mut Ui) -> Response {
        let side = self.radius * 2.0 + 60.0;
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width().max(side), side), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let id = response.id;
        let t = Theme::of(ui);
        let time = glowkit_core::time(ui);
        let painter = ui.painter_at(rect);
        let c = rect.center();
        let r = self.radius;
        let look_t = match response.hover_pos() {
            Some(p) => {
                let d = p - c;
                d / (d.length() + r * 1.5) * 1.6
            }
            None => vec2((time * 0.7).sin() * 0.7, (time * 0.45).sin() * 0.35),
        };
        let look = glowkit_core::anim::spring2(ui, id.with("look"), look_t, 90.0, 12.0);
        let blue = self.color.unwrap_or(hex("#2F3BFF"));
        // blink every ~3.3s (and a double blink sometimes)
        let ph = time % 3.3;
        let tri = |x: f32| if (0.0..0.14).contains(&x) { 1.0 - (x / 0.07 - 1.0).abs() } else { 0.0 };
        let double = h01((time / 3.3) as u64 + 7) > 0.55;
        let blink = tri(ph).max(if double { tri(ph - 0.24) } else { 0.0 });
        let eye_open = 1.0 - blink * 0.9;
        // outer glow / shadow
        if t.dark {
            paint::glow(&painter, c, r * 1.5, lum(blue, 0.12, true));
        } else {
            paint::glow(&painter, c + vec2(0.0, r * 0.9), r * 0.9, with_alpha(hex("#1E1B4B"), 0.12));
        }
        // glass body (faint tint)
        let glass = if t.dark { with_alpha(Color32::WHITE, 0.04) } else { with_alpha(hex("#E0E7FF"), 0.35) };
        painter.circle_filled(c, r, glass);
        // sprite
        let bob = vec2(0.0, (time * 1.6).sin() * r * 0.03);
        let sc = c + look * r * 0.12 + bob + vec2(0.0, -r * 0.05);
        let sr = r * 0.48;
        paint::glow(&painter, sc, sr * 1.6, lum(blue, if t.dark { 0.35 } else { 0.18 }, t.dark));
        let hi = lerp_rgb(blue, hex("#8EA2FF"), 0.8);
        let deep = lerp_rgb(blue, hex("#0A0F5C"), 0.6);
        let light_c = sc + vec2(-sr * 0.35, -sr * 0.4);
        paint::fill_convex_fn(&painter, &paint::circle_points(sc, sr, 64), 10, |p| {
            let d = p.distance(light_c) / (sr * 1.6);
            lerp_rgb(lerp_rgb(hi, blue, (d * 1.6).min(1.0)), deep, ((d - 0.6) * 2.0).clamp(0.0, 1.0))
        });
        // pill eyes
        let eye_c = sc + look * sr * 0.32 + vec2(0.0, -sr * 0.05);
        let eh = sr * 0.36 * eye_open;
        let ew = sr * 0.13;
        for s in [-1.0, 1.0] {
            let e = eye_c + vec2(s * sr * 0.27, 0.0);
            painter.rect_filled(Rect::from_center_size(e, vec2(ew, eh.max(ew * 0.6))), ew * 0.5, hex("#E8ECFF"));
        }
        // frosted lower half with an inner rim
        let frost = self.frost;
        let eq = c.y + r * 0.08;
        let rim_rx = r * 0.98;
        let rim_ry = r * 0.2;
        // the frosted region: circle below the rim ellipse
        let fcol = if t.dark { with_alpha(hex("#C7D2FE"), 0.22 * frost + 0.05) } else { with_alpha(Color32::WHITE, 0.55 * frost + 0.1) };
        let mut region = Vec::new();
        for i in 0..=48 {
            let a = PI - i as f32 / 48.0 * PI;
            region.push(pos2(c.x + a.cos() * rim_rx, eq - a.sin() * rim_ry));
        }
        let start = (eq - c.y) / r;
        let a0 = start.clamp(-1.0, 1.0).asin();
        for i in 0..=48 {
            let a = a0 + (PI - 2.0 * a0) * i as f32 / 48.0;
            region.push(pos2(c.x + a.cos() * r, c.y + a.sin() * r));
        }
        painter.add(Shape::Path(PathShape::convex_polygon(region, fcol, PathStroke::NONE)));
        let ring_col = if t.dark { with_alpha(Color32::WHITE, 0.5) } else { with_alpha(hex("#64748B"), 0.55) };
        let mut ell = Vec::new();
        for i in 0..=64 {
            let a = i as f32 / 64.0 * TAU;
            ell.push(pos2(c.x + a.cos() * rim_rx, eq + a.sin() * rim_ry));
        }
        painter.add(Shape::line(ell, Stroke::new(1.2, ring_col)));
        let mut ell2 = Vec::new();
        for i in 0..=64 {
            let a = i as f32 / 64.0 * TAU;
            ell2.push(pos2(c.x + a.cos() * rim_rx * 0.9, eq + r * 0.05 + a.sin() * rim_ry * 0.85));
        }
        painter.add(Shape::line(ell2, Stroke::new(1.0, with_alpha(ring_col, 0.5))));
        // glass rim: thick bright edge + inner shadow ring
        let edge = if t.dark { with_alpha(Color32::WHITE, 0.85) } else { with_alpha(hex("#475569"), 0.7) };
        for k in 0..5 {
            let f = k as f32 / 5.0;
            painter.circle_stroke(c, r - 1.0 - f * r * 0.07, Stroke::new(2.0, with_alpha(if t.dark { Color32::WHITE } else { hex("#94A3B8") }, 0.12 * (1.0 - f))));
        }
        painter.circle_stroke(c, r, Stroke::new(1.6, edge));
        // specular dashes top-right
        let sp = if t.dark { with_alpha(Color32::WHITE, 0.9) } else { with_alpha(Color32::WHITE, 0.95) };
        for (i, (a, l)) in [(-1.05f32, 0.16f32), (-0.78, 0.1), (-0.58, 0.06)].iter().enumerate() {
            let rr = r * 0.86;
            let a1 = *a + l;
            let pts: Vec<Pos2> = (0..=8).map(|k| {
                let aa = a + (a1 - a) * k as f32 / 8.0;
                c + vec2(aa.cos(), aa.sin()) * rr
            }).collect();
            painter.add(Shape::line(pts, Stroke::new(4.0 - i as f32 * 0.6, sp)));
        }
        response
    }
}
