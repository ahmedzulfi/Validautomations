//! Small control transitions: a bell badge, icon swaps, a success check, an avatar row, a field
//! that shakes on error, a search box that dissolves its text, a "learn more" chevron, a drawn
//! checkbox and a double-bounce switch.

use super::common::{
    avatar, bell, change, chip, cr, layer, material, more_chip, phase, round_line, soft_disc, span, surface, text_width, trim,
    ui_layer, xf_around, MotionCurve, Tween, AVATAR_TONES,
};
use egui::emath::TSTransform;
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};
use glowkit_core::color::{hex, mix, shade, with_alpha};
use glowkit_core::{paint, Rng, Theme};
use glowkit_effects::MicroIcon;
use std::f32::consts::PI;

// ---------------------------------------------------------------------------------------------

/// A bell button whose count badge pops in: it slides up out of the bell's shoulder, swells past
/// full size on a springy curve and un-blurs; it ducks away quickly when the count drops to zero.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::BellBadge::new(3));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BellBadge {
    count: u32,
    size: f32,
    color: Option<Color32>,
    ring: bool,
    open_dur: f32,
    close_dur: f32,
}

impl BellBadge {
    /// A bell showing `count` (0 hides the badge).
    pub fn new(count: u32) -> Self {
        Self { count, size: 40.0, color: None, ring: false, open_dur: 0.5, close_dur: 0.18 }
    }
    /// Button diameter (default 40).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(20.0);
        self
    }
    /// Badge color (default a bright red).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Also give the bell a little ring when the badge arrives.
    pub fn ring(mut self, on: bool) -> Self {
        self.ring = on;
        self
    }
    /// Pop-in and duck-out durations in seconds (default 0.5 / 0.18).
    pub fn durations(mut self, open: f32, close: f32) -> Self {
        self.open_dur = open.max(0.02);
        self.close_dur = close.max(0.02);
        self
    }
}

impl Widget for BellBadge {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let pad = self.size * 0.45;
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(self.size + pad * 2.0), Sense::click());
        let id = response.id;
        let on = self.count > 0;
        let ph = phase(ui, id, on, self.open_dur, self.close_dur, MotionCurve::LINEAR, MotionCurve::LINEAR);
        let cnt = change(ui, id.with("count"), &self.count, 0.4);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let c = rect.center();
        let btn = Rect::from_center_size(c, Vec2::splat(self.size));
        let hover = response.hovered();
        material(painter, &t, btn, self.size / 2.0, if hover { mix(surface(&t), t.foreground, 0.03) } else { surface(&t) });
        let ring_a = if self.ring && ph.on && ph.age < 0.9 {
            let k = ph.age / 0.9;
            (k * PI * 6.0).sin() * 0.32 * (1.0 - k).powi(2)
        } else {
            0.0
        };
        bell(painter, c, self.size * 0.42, t.foreground, 1.5, ring_a);
        // badge
        let (scale, alpha, blur, slide) = if ph.on {
            let a = ph.age;
            (
                MotionCurve::BOUNCE.timed(a, 0.0, self.open_dur),
                MotionCurve::BOUNCE.timed(a, 0.0, self.open_dur * 0.8).min(1.0),
                2.0 * (1.0 - MotionCurve::BOUNCE.timed(a, 0.0, self.open_dur).min(1.0)),
                1.0 - MotionCurve::SMOOTH_OUT.timed(a, 0.0, self.open_dur * 0.52),
            )
        } else {
            let k = MotionCurve::STANDARD.at(ph.age / self.close_dur);
            (1.0 - k, 1.0 - k, 2.0 * k, 0.0)
        };
        if alpha <= 0.003 {
            return response;
        }
        // a small bump when the number changes while shown
        let bump = if ph.on && ph.age > self.open_dur && cnt.age < 0.4 { 1.0 + 0.18 * (1.0 - MotionCurve::BOUNCE.at(cnt.age / 0.4)) } else { 1.0 };
        let shown_count = if ph.on { self.count } else { cnt.prev.max(1) };
        let label = if shown_count > 99 { "99+".to_string() } else { shown_count.to_string() };
        let font = FontId::proportional(self.size * 0.27);
        let tw = text_width(painter, &label, &font);
        let bh = self.size * 0.4;
        let bw = (tw + bh * 0.55).max(bh);
        let anchor = c + vec2(self.size * 0.3, -self.size * 0.3);
        let off = vec2(-8.2, 12.4) * (self.size / 40.0) * slide;
        let col = self.color.unwrap_or(hex("#F0293E"));
        let ring_col = t.background;
        layer(ui, xf_around(anchor, scale.max(0.0) * bump, off), alpha, blur, |pt| {
            let r = Rect::from_center_size(anchor, vec2(bw, bh));
            pt.rect_filled(r.expand(1.5), cr(bh / 2.0 + 1.5), ring_col);
            pt.rect_filled(r, cr(bh / 2.0), col);
            pt.text(r.center() + vec2(0.0, 0.3), Align2::CENTER_CENTER, &label, font.clone(), Color32::WHITE);
        });
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// The two icons an [`SwapIcon`] swaps between.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SwapIconPair {
    /// Hamburger menu and close cross.
    #[default]
    MenuClose,
    /// Play triangle and pause bars.
    PlayPause,
    /// Copy squares and a check.
    CopyCheck,
    /// Plus and minus.
    PlusMinus,
}

fn paint_swap_icon(p: &egui::Painter, pair: SwapIconPair, second: bool, c: Pos2, size: f32, col: Color32) {
    let s = size / 24.0;
    let q = |x: f32, y: f32| c + vec2((x - 12.0) * s, (y - 12.0) * s);
    let w = 1.75 * s;
    match (pair, second) {
        (SwapIconPair::MenuClose, false) => {
            for y in [7.0, 12.0, 17.0] {
                round_line(p, &[q(4.0, y), q(20.0, y)], w, col);
            }
        }
        (SwapIconPair::MenuClose, true) => {
            round_line(p, &[q(6.0, 6.0), q(18.0, 18.0)], w, col);
            round_line(p, &[q(18.0, 6.0), q(6.0, 18.0)], w, col);
        }
        (SwapIconPair::PlayPause, false) => {
            let pts = vec![q(8.0, 5.5), q(18.5, 12.0), q(8.0, 18.5)];
            p.add(egui::Shape::convex_polygon(pts.clone(), col, Stroke::new(w * 0.9, col)));
        }
        (SwapIconPair::PlayPause, true) => {
            for x in [8.0, 16.0] {
                p.rect_filled(Rect::from_center_size(q(x, 12.0), vec2(3.2 * s, 12.0 * s)), cr(1.2 * s), col);
            }
        }
        (SwapIconPair::CopyCheck, false) => MicroIcon::Copy.paint(p, c, size * 0.8, col),
        (SwapIconPair::CopyCheck, true) => {
            round_line(p, &[q(5.0, 12.5), q(10.0, 17.5), q(19.5, 7.0)], w, col);
        }
        (SwapIconPair::PlusMinus, false) => {
            round_line(p, &[q(5.0, 12.0), q(19.0, 12.0)], w, col);
            round_line(p, &[q(12.0, 5.0), q(12.0, 19.0)], w, col);
        }
        (SwapIconPair::PlusMinus, true) => round_line(p, &[q(5.0, 12.0), q(19.0, 12.0)], w, col),
    }
}

/// An icon button that swaps between two icons: the old one shrinks to a quarter, blurs and
/// fades while the new one grows out of the same spot.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut open = false;
/// ui.add(glowkit_interactions::SwapIcon::new(&mut open, glowkit_interactions::SwapIconPair::MenuClose));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SwapIcon<'a> {
    on: &'a mut bool,
    pair: SwapIconPair,
    size: f32,
    dur: f32,
    start_scale: f32,
    framed: bool,
}

impl<'a> SwapIcon<'a> {
    /// Swap `pair` according to `on` (clicking toggles it).
    pub fn new(on: &'a mut bool, pair: SwapIconPair) -> Self {
        Self { on, pair, size: 24.0, dur: 0.25, start_scale: 0.25, framed: true }
    }
    /// Icon size (default 24).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(8.0);
        self
    }
    /// Swap duration in seconds (default 0.25).
    pub fn duration(mut self, s: f32) -> Self {
        self.dur = s.max(0.02);
        self
    }
    /// Scale the icons shrink to / grow from (default 0.25).
    pub fn start_scale(mut self, s: f32) -> Self {
        self.start_scale = s.clamp(0.0, 1.0);
        self
    }
    /// Draw the round button behind the icon (default on).
    pub fn framed(mut self, on: bool) -> Self {
        self.framed = on;
        self
    }
}

impl Widget for SwapIcon<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let d = self.size * 1.9;
        let (rect, mut response) = ui.allocate_exact_size(Vec2::splat(d + 8.0), Sense::click());
        response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.clicked() {
            *self.on = !*self.on;
            response.mark_changed();
        }
        let ph = phase(ui, response.id, *self.on, self.dur, self.dur, MotionCurve::EASE_IN_OUT, MotionCurve::EASE_IN_OUT);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let c = rect.center();
        if self.framed {
            let press = if response.is_pointer_button_down_on() { 0.95 } else { 1.0 };
            let b = Rect::from_center_size(c, Vec2::splat(d * press));
            let fill = if response.hovered() { mix(surface(&t), t.foreground, 0.03) } else { surface(&t) };
            material(ui.painter(), &t, b, d / 2.0, fill);
        }
        let p = ph.p.clamp(0.0, 1.0);
        let col = t.foreground;
        let ss = self.start_scale;
        // first icon visible at p = 0, second at p = 1
        for (second, k) in [(false, 1.0 - p), (true, p)] {
            if k <= 0.003 {
                continue;
            }
            let s = ss + (1.0 - ss) * k;
            layer(ui, xf_around(c, s, Vec2::ZERO), k, 2.0 * (1.0 - k), |pt| {
                paint_swap_icon(pt, self.pair, second, c, self.size, col);
            });
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A success badge: a green disc rises in on a springy bob, spinning back from an angle while
/// it sharpens out of a heavy blur, and its check draws itself a beat later. Hiding fades and
/// softens it away.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::SuccessCheck::new(true).size(56.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SuccessCheck {
    shown: bool,
    size: f32,
    color: Option<Color32>,
    dur: f32,
    rise: f32,
    turn: f32,
    blur: f32,
}

impl SuccessCheck {
    /// A check shown while `shown` is true.
    pub fn new(shown: bool) -> Self {
        Self { shown, size: 48.0, color: None, dur: 0.5, rise: 40.0, turn: 80.0, blur: 10.0 }
    }
    /// Disc diameter (default 48).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(12.0);
        self
    }
    /// Disc color (default a vivid green).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Duration of each part in seconds (default 0.5).
    pub fn duration(mut self, s: f32) -> Self {
        self.dur = s.max(0.05);
        self
    }
    /// How far below it starts, in points (default 40).
    pub fn rise(mut self, d: f32) -> Self {
        self.rise = d;
        self
    }
    /// Starting angle in degrees (default 80).
    pub fn turn(mut self, deg: f32) -> Self {
        self.turn = deg;
        self
    }
    /// Starting blur (default 10).
    pub fn blur(mut self, b: f32) -> Self {
        self.blur = b.max(0.0);
        self
    }
}

impl Widget for SuccessCheck {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(vec2(self.size + 40.0, self.size + self.rise.abs() + 30.0), Sense::hover());
        let ph = phase(ui, response.id, self.shown, self.dur + 0.1, self.dur, MotionCurve::LINEAR, MotionCurve::LINEAR);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let col = self.color.unwrap_or(hex("#36B517"));
        let a = ph.age;
        let (alpha, ang, blur, dy, draw) = if ph.on {
            (
                MotionCurve::SMOOTH_OUT.timed(a, 0.0, self.dur),
                self.turn.to_radians() * (1.0 - MotionCurve::SMOOTH_OUT.timed(a, 0.0, self.dur)),
                self.blur * (1.0 - MotionCurve::SMOOTH_OUT.timed(a, 0.0, self.dur)),
                self.rise * (1.0 - MotionCurve::new(0.34, 1.35, 0.64, 1.0).timed(a, 0.0, self.dur)),
                MotionCurve::SMOOTH_OUT.timed(a, 0.08, self.dur),
            )
        } else {
            let k = MotionCurve::SMOOTH_OUT.at(a / self.dur);
            (1.0 - k, 0.0, 4.0 * k, 0.0, 1.0)
        };
        if alpha <= 0.003 {
            return response;
        }
        let c = pos2(rect.center().x, rect.min.y + 15.0 + self.size / 2.0) + vec2(0.0, dy);
        let r = self.size / 2.0;
        let painter = ui.painter().clone();
        let mut pt = painter.clone();
        pt.multiply_opacity(alpha);
        paint::soft_shadow(&pt, Rect::from_center_size(c + vec2(0.0, 3.0), Vec2::splat(self.size * 0.8)), r, 14.0, Vec2::ZERO, with_alpha(col, 0.35));
        soft_disc(&pt, c, r, blur, col);
        let s = self.size / 48.0;
        let (sn, cs) = ang.sin_cos();
        let q = |x: f32, y: f32| {
            let v = vec2((x - 24.0) * s, (y - 24.0) * s);
            c + vec2(v.x * cs - v.y * sn, v.x * sn + v.y * cs)
        };
        let mut path = vec![q(17.28, 24.96)];
        path.extend((1..=6).map(|i| super::common::along(q(17.28, 24.96), q(21.76, 29.76), i as f32 / 6.0)));
        path.extend((1..=10).map(|i| super::common::along(q(21.76, 29.76), q(30.72, 20.16), i as f32 / 10.0)));
        let part = trim(&path, draw);
        if part.len() >= 2 {
            layer(ui, TSTransform::IDENTITY, alpha, blur * 0.5, |p| round_line(p, &part, 3.0 * s, Color32::WHITE));
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Lift {
    k: Tween,
    rising: bool,
}

/// A row of overlapping avatars that lifts toward the pointer: the hovered face rises and grows
/// a little, its neighbours rise less and less, and on the way down they drop back with a quick
/// springy bounce.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::AvatarStack::new(6).more(2));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AvatarStack {
    count: usize,
    more: usize,
    radius: f32,
    overlap: f32,
    lift: f32,
    falloff: f32,
    scale: f32,
    dur: f32,
    force: Option<Option<usize>>,
    tones: Vec<[Color32; 2]>,
}

impl AvatarStack {
    /// `count` painted avatars.
    pub fn new(count: usize) -> Self {
        Self {
            count,
            more: 0,
            radius: 16.0,
            overlap: 0.28,
            lift: 4.0,
            falloff: 0.45,
            scale: 1.05,
            dur: 0.32,
            force: None,
            tones: AVATAR_TONES.to_vec(),
        }
    }
    /// Show a trailing "+N" chip.
    pub fn more(mut self, n: usize) -> Self {
        self.more = n;
        self
    }
    /// Avatar radius (default 16).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.max(6.0);
        self
    }
    /// Overlap as a share of the diameter (default 0.28).
    pub fn overlap(mut self, k: f32) -> Self {
        self.overlap = k.clamp(0.0, 0.8);
        self
    }
    /// How far the hovered avatar rises, in points (default 4).
    pub fn lift(mut self, d: f32) -> Self {
        self.lift = d;
        self
    }
    /// How much of the lift each neighbour keeps per step (default 0.45).
    pub fn falloff(mut self, k: f32) -> Self {
        self.falloff = k.clamp(0.0, 1.0);
        self
    }
    /// Scale of the hovered avatar (default 1.05).
    pub fn scale(mut self, s: f32) -> Self {
        self.scale = s.max(0.5);
        self
    }
    /// Transition length in seconds (default 0.32).
    pub fn duration(mut self, s: f32) -> Self {
        self.dur = s.max(0.02);
        self
    }
    /// Drive the hovered avatar yourself (overrides the pointer).
    pub fn hovered(mut self, i: Option<usize>) -> Self {
        self.force = Some(i);
        self
    }
    /// Gradient color pairs for the avatars (cycled).
    pub fn tones(mut self, tones: Vec<[Color32; 2]>) -> Self {
        if !tones.is_empty() {
            self.tones = tones;
        }
        self
    }
}

impl Widget for AvatarStack {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.count + (self.more > 0) as usize;
        let d = self.radius * 2.0;
        let step = d * (1.0 - self.overlap);
        let w = step * (n.max(1) - 1) as f32 + d;
        let (rect, response) = ui.allocate_exact_size(vec2(w + 16.0, d + self.lift.abs() * 2.0 + 16.0), Sense::hover());
        let x0 = rect.min.x + 8.0 + self.radius;
        let cy = rect.center().y + self.lift * 0.5;
        let centers: Vec<Pos2> = (0..n).map(|i| pos2(x0 + i as f32 * step, cy)).collect();
        let hover = ui.input(|i| i.pointer.hover_pos()).filter(|_| response.hovered()).and_then(|p| {
            // last drawn wins (they overlap to the right)
            (0..n).rev().find(|&i| (p - centers[i]).length() <= self.radius + 1.0)
        });
        let active = if hover.is_some() { hover } else { self.force.unwrap_or(None) };
        let now = ui.input(|i| i.time);
        let dur = self.dur;
        let falloff = self.falloff;
        let back = MotionCurve::new(0.34, 3.85, 0.64, 1.0);
        let lifts = glowkit_core::with_state(
            ui,
            response.id.with("lifts"),
            || vec![Lift { k: Tween::new(0.0), rising: true }; n],
            |v| {
                v.resize(n, Lift { k: Tween::new(0.0), rising: true });
                for (i, l) in v.iter_mut().enumerate() {
                    let target = active.map_or(0.0, |a| falloff.powi((i as i32 - a as i32).abs()));
                    if (target - l.k.to).abs() > 1e-4 {
                        let curve = if l.rising { MotionCurve::SMOOTH_OUT } else { back };
                        let cur = l.k.value(now, dur, curve);
                        l.rising = target > cur;
                        l.k = Tween { from: cur, to: target, since: now };
                    }
                }
                v.clone()
            },
        );
        if lifts.iter().any(|l| l.k.moving(now, dur)) {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let ring = t.background;
        for (i, c) in centers.iter().enumerate() {
            let l = lifts[i];
            let k = l.k.value(now, dur, if l.rising { MotionCurve::SMOOTH_OUT } else { back });
            let is_active = active == Some(i) || (active.is_none() && l.k.from > 0.99);
            let s = 1.0 + (self.scale - 1.0) * if is_active { k } else { 0.0 };
            let center = *c + vec2(0.0, -self.lift * k);
            let r = self.radius * s;
            if i < self.count {
                avatar(painter, center, r, self.tones[i % self.tones.len()], Some(ring));
            } else {
                more_chip(painter, &t, center, r, self.more, ring);
            }
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct ShakeState {
    since: f64,
    attempt: u32,
    err: bool,
}

/// A text field that refuses: on a failed attempt it shakes side to side with decaying
/// swings, its outline turns red and a message fades in under it; after a few seconds both ease
/// back to neutral.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut email = String::from("John");
/// let attempts = 1;
/// ui.add(glowkit_interactions::ShakeField::new(&mut email).error("Please enter a valid email.").attempt(attempts));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ShakeField<'a> {
    text: &'a mut String,
    error: Option<String>,
    attempt: u32,
    width: f32,
    distance: f32,
    hold: f32,
    hint: String,
}

impl<'a> ShakeField<'a> {
    /// A field editing `text`.
    pub fn new(text: &'a mut String) -> Self {
        Self { text, error: None, attempt: 0, width: 240.0, distance: 6.0, hold: 3.0, hint: String::new() }
    }
    /// The error to show (shakes when it appears).
    pub fn error(mut self, msg: impl Into<String>) -> Self {
        self.error = Some(msg.into());
        self
    }
    /// Bump this number on every failed submit to shake again.
    pub fn attempt(mut self, n: u32) -> Self {
        self.attempt = n;
        self
    }
    /// Field width (default 240).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(80.0);
        self
    }
    /// Shake distance in points (default 6).
    pub fn distance(mut self, d: f32) -> Self {
        self.distance = d;
        self
    }
    /// Seconds the error stays before easing back (default 3; 0 keeps it).
    pub fn hold(mut self, s: f32) -> Self {
        self.hold = s.max(0.0);
        self
    }
    /// Placeholder text.
    pub fn hint(mut self, s: impl Into<String>) -> Self {
        self.hint = s.into();
        self
    }
}

/// Offset of the shake at `age` seconds (segments of 80, 80, 60 and 60 ms).
fn shake_offset(age: f32, d: f32) -> f32 {
    let segs = [(0.08, 0.0, d), (0.08, d, -d), (0.06, -d, d * 0.66), (0.06, d * 0.66, 0.0)];
    let mut t0 = 0.0;
    for (len, a, b) in segs {
        if age < t0 + len {
            return a + (b - a) * MotionCurve::SMOOTH_OUT.at((age - t0) / len);
        }
        t0 += len;
    }
    0.0
}

impl Widget for ShakeField<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let h = 38.0;
        let (rect, response) = ui.allocate_exact_size(vec2(self.width + 2.0 * self.distance + 4.0, h + 26.0), Sense::hover());
        let now = ui.input(|i| i.time);
        let err = self.error.is_some();
        let attempt = self.attempt;
        let st = glowkit_core::with_state(
            ui,
            response.id.with("shake"),
            || ShakeState { since: -1e6, attempt, err },
            |s| {
                if (err && !s.err) || (err && attempt != s.attempt) {
                    s.since = now;
                }
                s.err = err;
                s.attempt = attempt;
                *s
            },
        );
        let age = (now - st.since) as f32;
        let holding = self.hold <= 0.0 || age < self.hold;
        let red = if !err {
            0.0
        } else if holding {
            span(age, 0.0, 0.15)
        } else {
            1.0 - span(age - self.hold, 0.0, 0.28)
        };
        if age < self.hold + 0.4 {
            ui.ctx().request_repaint();
        }
        let dx = shake_offset(age, self.distance);
        let field = Rect::from_min_size(pos2(rect.center().x - self.width / 2.0 + dx, rect.min.y + 2.0), vec2(self.width, h));
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let bad = t.destructive;
            let border = mix(if t.dark { with_alpha(Color32::WHITE, 0.12) } else { t.border }, bad, red);
            painter.rect(field, cr(10.0), surface(&t), Stroke::new(1.0, border), egui::StrokeKind::Inside);
            if red > 0.0 {
                painter.rect_stroke(field.expand(2.0), cr(12.0), Stroke::new(3.0, with_alpha(bad, 0.12 * red)), egui::StrokeKind::Inside);
            }
            if let Some(msg) = &self.error {
                painter.text(
                    pos2(field.min.x - dx + 2.0, field.max.y + 6.0),
                    Align2::LEFT_TOP,
                    msg,
                    FontId::proportional(12.0),
                    with_alpha(bad, red),
                );
            }
        }
        let edit_rect = field.shrink2(vec2(12.0, 8.0));
        let mut te = egui::TextEdit::singleline(self.text).frame(egui::Frame::NONE).desired_width(edit_rect.width()).font(FontId::proportional(14.0));
        if !self.hint.is_empty() {
            te = te.hint_text(self.hint.clone());
        }
        ui.put(edit_rect, te);
        response
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone)]
struct Dust {
    text: String,
    origin: Pos2,
    since: f64,
    seed: u64,
}

/// A search pill whose clear button dissolves the query: the characters drift down and break
/// into dust from the button outward, a soft haze blooms where they were, and the placeholder
/// fades back in.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut query = String::from("How do transitions work?");
/// ui.add(glowkit_interactions::DissolveSearch::new(&mut query));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DissolveSearch<'a> {
    text: &'a mut String,
    placeholder: String,
    width: f32,
    dur: f32,
    fly: f32,
    clear_now: bool,
}

impl<'a> DissolveSearch<'a> {
    /// A search field editing `text`.
    pub fn new(text: &'a mut String) -> Self {
        Self { text, placeholder: "Search".into(), width: 280.0, dur: 1.0, fly: 12.0, clear_now: false }
    }
    /// Clear (with the dissolve) this frame when `trigger` is true, as if the button was pressed.
    pub fn clear_if(mut self, trigger: bool) -> Self {
        self.clear_now = trigger;
        self
    }
    /// Placeholder (default "Search").
    pub fn placeholder(mut self, s: impl Into<String>) -> Self {
        self.placeholder = s.into();
        self
    }
    /// Width (default 280).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(120.0);
        self
    }
    /// Length of the whole dissolve in seconds (default 1).
    pub fn duration(mut self, s: f32) -> Self {
        self.dur = s.max(0.2);
        self
    }
    /// How far the characters drift, in points (default 12).
    pub fn fly(mut self, d: f32) -> Self {
        self.fly = d;
        self
    }
}

impl Widget for DissolveSearch<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let h = 38.0;
        let (rect, response) = ui.allocate_exact_size(vec2(self.width, h + 24.0), Sense::hover());
        let pill = Rect::from_center_size(rect.center(), vec2(self.width, h));
        let id = response.id;
        let now = ui.input(|i| i.time);
        let has = !self.text.is_empty();
        let font = FontId::proportional(14.0);
        let text_x = pill.min.x + 36.0;
        let clear_c = pos2(pill.max.x - 19.0, pill.center().y);
        let clear_resp = if has {
            Some(ui.interact(Rect::from_center_size(clear_c, Vec2::splat(22.0)), id.with("clear"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand))
        } else {
            None
        };
        let mut dust: Option<Dust> = glowkit_core::state(ui, id.with("dust"), || None);
        if has && (self.clear_now || clear_resp.as_ref().is_some_and(|r| r.clicked())) {
            let galley_h = ui.painter().layout_no_wrap("Ag".into(), font.clone(), Color32::WHITE).size().y;
            dust = Some(Dust { text: self.text.clone(), origin: pos2(text_x, pill.center().y - galley_h / 2.0), since: now, seed: now.to_bits() });
            self.text.clear();
            glowkit_core::set_state(ui, id.with("dust"), dust.clone());
        }
        let fill_k = glowkit_core::anim::follow(ui, id.with("fill"), (!self.text.is_empty()) as u8 as f32, 18.0);
        let painter = ui.painter().clone();
        if ui.is_rect_visible(rect) {
            let filled = chip(&t);
            let bg = mix(surface(&t), filled, fill_k);
            let border = with_alpha(if t.dark { with_alpha(Color32::WHITE, 0.12) } else { t.border }, 1.0 - fill_k);
            painter.rect(pill, cr(h / 2.0), bg, Stroke::new(1.0, border), egui::StrokeKind::Inside);
            // magnifier
            let mc = pos2(pill.min.x + 19.0, pill.center().y - 0.5);
            painter.circle_stroke(mc - vec2(1.0, 1.0), 5.0, Stroke::new(1.4, t.muted_foreground));
            painter.line_segment([mc + vec2(2.6, 2.6), mc + vec2(5.6, 5.6)], Stroke::new(1.4, t.muted_foreground));
        }
        // dissolve
        let mut busy = false;
        if let Some(d) = &dust {
            let age = (now - d.since) as f32;
            if age < self.dur + 0.1 {
                busy = true;
                let g = glowkit_core::text::Glyphs::layout(&painter, &d.text, font.clone(), None);
                let n = g.glyphs.len().max(1);
                let tail = self.dur * 0.35;
                let each = self.dur * 0.4;
                // haze
                let env = if age < self.dur * 0.15 { age / (self.dur * 0.15) } else { 1.0 - span(age, self.dur * 0.15, self.dur) };
                let haze = if t.dark { Color32::WHITE } else { Color32::BLACK };
                let w = g.size.x.min(pill.width() - 70.0);
                let clip = painter.with_clip_rect(rect.expand(12.0));
                for (dx, rw, rh, a) in [(0.5, 0.4, 7.0, 0.22), (0.72, 0.28, 8.0, 0.18), (0.3, 0.32, 6.0, 0.16), (0.58, 0.45, 5.0, 0.14)] {
                    paint::glow_ellipse(
                        &clip,
                        pos2(d.origin.x + w * dx, pill.center().y + 4.0 + age * 6.0),
                        vec2(w * rw * 1.5, rh * 1.6),
                        with_alpha(haze, a * 0.42 * env),
                    );
                }
                let mut rng = Rng::new(d.seed);
                for (i, gb) in g.glyphs.iter().enumerate() {
                    let delay = tail * (n - 1 - i) as f32 / n as f32;
                    let local = age - delay;
                    let k = MotionCurve::SMOOTH_OUT.at(local / each);
                    let (jx, jy) = (rng.range(-1.0, 1.0), rng.range(0.4, 1.0));
                    if gb.ch.is_whitespace() {
                        continue;
                    }
                    let p0 = d.origin + gb.offset;
                    if local < each {
                        let off = vec2(jx * 3.0 * k, self.fly * jy * k);
                        layer(ui, TSTransform::from_translation(off), 1.0 - span(local, each * 0.1, each), 2.5 * k, |pt| {
                            pt.text(p0, Align2::LEFT_TOP, gb.ch, font.clone(), t.foreground);
                        });
                    }
                    // dust motes
                    let dk = span(local, 0.0, each * 1.6);
                    if dk > 0.0 && dk < 1.0 {
                        for m in 0..4 {
                            let mut r2 = Rng::new(d.seed ^ ((i * 7 + m) as u64).wrapping_mul(0x9E37_79B9));
                            let start = p0 + vec2(r2.range(0.0, gb.size.x), r2.range(gb.size.y * 0.25, gb.size.y * 0.85));
                            let vel = vec2(r2.range(-6.0, 10.0), r2.range(6.0, 22.0));
                            let e = MotionCurve::SMOOTH_OUT.at(dk);
                            let pos = start + vel * e;
                            let a = (1.0 - dk) * span(dk, 0.0, 0.1) * 0.7;
                            clip.circle_filled(pos, r2.range(0.5, 1.1), with_alpha(t.foreground, a));
                        }
                    }
                }
            }
        }
        // placeholder
        if self.text.is_empty() && ui.is_rect_visible(rect) {
            let a = match &dust {
                Some(d) => span((now - d.since) as f32, self.dur * 0.25, self.dur * 0.65),
                None => 1.0,
            };
            let a = MotionCurve::SMOOTH_OUT.at(a);
            if a > 0.0 {
                painter.text(
                    pos2(text_x, pill.center().y + 4.0 * (1.0 - a)),
                    Align2::LEFT_CENTER,
                    &self.placeholder,
                    font.clone(),
                    with_alpha(t.muted_foreground, a),
                );
            }
        }
        // the text field itself, fading in when text arrives in one go (paste / reset)
        let arrive = change(ui, id.with("arrive"), &self.text.is_empty(), 0.5);
        let k = if !self.text.is_empty() && arrive.prev { MotionCurve::SMOOTH_OUT.at(arrive.age / 0.4) } else { 1.0 };
        let edit = Rect::from_min_max(pos2(text_x - 2.0, pill.min.y + 8.0), pos2(clear_c.x - 14.0, pill.max.y - 8.0));
        ui_layer(ui, TSTransform::from_translation(vec2(0.0, self.fly * 0.5 * (1.0 - k))), k, 2.0 * (1.0 - k), |ui| {
            ui.put(edit, egui::TextEdit::singleline(self.text).frame(egui::Frame::NONE).font(font.clone()).desired_width(edit.width()));
        });
        // clear button
        let ck = glowkit_core::anim::follow(ui, id.with("clr"), has as u8 as f32, 20.0);
        if ck > 0.01 {
            let hov = clear_resp.as_ref().is_some_and(|r| r.hovered());
            let col = if hov { t.foreground } else { mix(t.muted_foreground, t.foreground, 0.1) };
            let r = 8.0 * (0.6 + 0.4 * ck);
            painter.circle_filled(clear_c, r, with_alpha(col, ck * 0.85));
            let x = r * 0.36;
            let bg = if t.dark { Color32::BLACK } else { Color32::WHITE };
            painter.line_segment([clear_c - vec2(x, x), clear_c + vec2(x, x)], Stroke::new(1.4, with_alpha(bg, ck)));
            painter.line_segment([clear_c + vec2(-x, x), clear_c + vec2(x, -x)], Stroke::new(1.4, with_alpha(bg, ck)));
        }
        if busy {
            glowkit_core::animate(ui);
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A "Learn more" pill whose chevron leans forward on hover: it nudges right while its two arms
/// splay open, and (optionally) a shaft grows in to turn it into an arrow.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// if ui.add(glowkit_interactions::LearnMoreLink::new("Learn more")).clicked() {
///     // open docs
/// }
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct LearnMoreLink {
    label: String,
    shift: f32,
    spread: f32,
    dur: f32,
    arrow: bool,
    force: Option<bool>,
    pill: bool,
}

impl LearnMoreLink {
    /// A link labelled `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), shift: 2.0, spread: 8.0, dur: 0.35, arrow: false, force: None, pill: true }
    }
    /// How far the chevron nudges right (default 2).
    pub fn shift(mut self, d: f32) -> Self {
        self.shift = d;
        self
    }
    /// How far each arm splays, in degrees (default 8).
    pub fn spread(mut self, deg: f32) -> Self {
        self.spread = deg;
        self
    }
    /// Transition length in seconds (default 0.35).
    pub fn duration(mut self, s: f32) -> Self {
        self.dur = s.max(0.02);
        self
    }
    /// Grow a shaft so the chevron becomes an arrow on hover.
    pub fn arrow(mut self, on: bool) -> Self {
        self.arrow = on;
        self
    }
    /// Drive the hover state yourself (overrides the pointer).
    pub fn hovered(mut self, on: bool) -> Self {
        self.force = Some(on);
        self
    }
    /// Draw the pill behind the label (default on).
    pub fn pill(mut self, on: bool) -> Self {
        self.pill = on;
        self
    }
}

impl Widget for LearnMoreLink {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = FontId::proportional(14.0);
        let tw = text_width(ui.painter(), &self.label, &font);
        let (rect, response) = ui.allocate_exact_size(vec2(tw + 52.0, 36.0), Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        let on = response.hovered() || self.force.unwrap_or(false);
        let ph = phase(ui, response.id, on, self.dur, self.dur, MotionCurve::SMOOTH_OUT, MotionCurve::SMOOTH_OUT);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let p = ph.p;
        if self.pill {
            let fill = mix(chip(&t), shade(chip(&t), if t.dark { 0.04 } else { -0.03 }), p.clamp(0.0, 1.0));
            painter.rect_filled(rect, cr(18.0), fill);
        }
        let x0 = rect.min.x + 18.0;
        painter.text(pos2(x0, rect.center().y), Align2::LEFT_CENTER, &self.label, font, t.foreground);
        let s = 1.0;
        let tip = pos2(x0 + tw + 15.0 + self.shift * p, rect.center().y);
        let spread = self.spread.to_radians() * p;
        let arm = 4.0 * 1.414 * s;
        let col = t.foreground;
        for (base, sign) in [(PI * 0.75, -1.0), (-PI * 0.75, 1.0)] {
            let a = base + sign * spread;
            let end = tip + vec2(a.cos(), a.sin()) * arm;
            round_line(painter, &[end, tip], 1.5, col);
        }
        if self.arrow && p > 0.01 {
            let len = 9.0 * p.clamp(0.0, 1.2);
            round_line(painter, &[tip - vec2(len, 0.0), tip], 1.5, with_alpha(col, p.clamp(0.0, 1.0)));
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A checkbox whose tick draws itself: the box fills in quickly, then the check stroke is drawn
/// from end to end on a long smooth curve; unchecking erases it fast.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut notify = false;
/// ui.add(glowkit_interactions::DrawnCheckbox::new(&mut notify, "Notify me"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DrawnCheckbox<'a> {
    on: &'a mut bool,
    label: String,
    size: f32,
    box_dur: f32,
    draw_dur: f32,
    undraw_dur: f32,
    color: Option<Color32>,
}

impl<'a> DrawnCheckbox<'a> {
    /// A checkbox for `on` with a label.
    pub fn new(on: &'a mut bool, label: impl Into<String>) -> Self {
        Self { on, label: label.into(), size: 18.0, box_dur: 0.15, draw_dur: 0.35, undraw_dur: 0.15, color: None }
    }
    /// Box size (default 18).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(10.0);
        self
    }
    /// Box fill, check draw and uncheck durations in seconds (default 0.15 / 0.35 / 0.15).
    pub fn durations(mut self, fill: f32, draw: f32, undraw: f32) -> Self {
        self.box_dur = fill.max(0.01);
        self.draw_dur = draw.max(0.01);
        self.undraw_dur = undraw.max(0.01);
        self
    }
    /// Checked fill color (default: the foreground ink).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

impl Widget for DrawnCheckbox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = FontId::proportional(14.0);
        let tw = if self.label.is_empty() { 0.0 } else { text_width(ui.painter(), &self.label, &font) + 10.0 };
        let (rect, mut response) = ui.allocate_exact_size(vec2(self.size + tw + 4.0, self.size.max(22.0)), Sense::click());
        response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.clicked() {
            *self.on = !*self.on;
            response.mark_changed();
        }
        let on = *self.on;
        let fill = phase(ui, response.id.with("fill"), on, self.box_dur, self.box_dur, MotionCurve::SMOOTH_OUT, MotionCurve::SMOOTH_OUT);
        let draw = phase(ui, response.id.with("draw"), on, self.draw_dur, self.undraw_dur, MotionCurve::SMOOTH_OUT, MotionCurve::SMOOTH_OUT);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let press = if response.is_pointer_button_down_on() { 0.9 } else { 1.0 };
        let b = Rect::from_center_size(pos2(rect.min.x + 2.0 + self.size / 2.0, rect.center().y), Vec2::splat(self.size * press));
        let ink = self.color.unwrap_or(if t.dark { hex("#EDEDED") } else { hex("#0D0D0D") });
        let k = fill.p.clamp(0.0, 1.0);
        let off = surface(&t);
        let r = self.size * 0.28;
        painter.rect_filled(b, cr(r), mix(off, ink, k));
        let ring = if t.dark { with_alpha(Color32::WHITE, 0.18) } else { Color32::from_black_alpha(40) };
        painter.rect_stroke(b, cr(r), Stroke::new(1.0, with_alpha(ring, 1.0 - k)), egui::StrokeKind::Inside);
        if response.hovered() && !on {
            painter.rect_stroke(b, cr(r), Stroke::new(1.0, with_alpha(t.foreground, 0.25)), egui::StrokeKind::Inside);
        }
        let s = b.width() / 18.0;
        let q = |x: f32, y: f32| b.min + vec2(x * s, y * s);
        let mut path = vec![q(5.0, 9.4)];
        path.extend((1..=5).map(|i| super::common::along(q(5.0, 9.4), q(7.8, 12.2), i as f32 / 5.0)));
        path.extend((1..=8).map(|i| super::common::along(q(7.8, 12.2), q(13.2, 6.4), i as f32 / 8.0)));
        let part = trim(&path, draw.p.clamp(0.0, 1.0));
        let check_col = if t.dark { hex("#111111") } else { Color32::WHITE };
        round_line(painter, &part, 2.0 * s, check_col);
        if !self.label.is_empty() {
            painter.text(pos2(b.max.x + 10.0, rect.center().y), Align2::LEFT_CENTER, &self.label, font, t.foreground);
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A switch whose pill thumb double-bounces: it overshoots the far end by a point, swings back a
/// hair and settles, all on a springy curve, while the track flips color instantly.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut on = false;
/// ui.add(glowkit_interactions::BounceSwitch::new(&mut on));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BounceSwitch<'a> {
    on: &'a mut bool,
    scale: f32,
    dur: f32,
    overshoot: f32,
    color: Option<Color32>,
}

impl<'a> BounceSwitch<'a> {
    /// A switch for `on`.
    pub fn new(on: &'a mut bool) -> Self {
        Self { on, scale: 1.0, dur: 0.35, overshoot: 1.0, color: None }
    }
    /// Size multiplier (default 1 = a 44 x 24 track).
    pub fn scale(mut self, s: f32) -> Self {
        self.scale = s.clamp(0.5, 4.0);
        self
    }
    /// Transition length in seconds (default 0.35).
    pub fn duration(mut self, s: f32) -> Self {
        self.dur = s.max(0.02);
        self
    }
    /// Overshoot past the end, in points (default 1).
    pub fn overshoot(mut self, d: f32) -> Self {
        self.overshoot = d;
        self
    }
    /// Track color when on (default: the theme primary).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

impl Widget for BounceSwitch<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let s = self.scale;
        let size = vec2(44.0, 24.0) * s;
        let (rect, mut response) = ui.allocate_exact_size(size + Vec2::splat(4.0), Sense::click());
        response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.clicked() {
            *self.on = !*self.on;
            response.mark_changed();
        }
        let on = *self.on;
        let ph = phase(ui, response.id, on, self.dur, self.dur, MotionCurve::LINEAR, MotionCurve::LINEAR);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let track = Rect::from_center_size(rect.center(), size);
        let on_col = self.color.unwrap_or(t.primary);
        let off_col = if t.dark { with_alpha(Color32::WHITE, 0.15) } else { hex("#D9D9DC") };
        painter.rect_filled(track, cr(size.y / 2.0), if on { on_col } else { off_col });
        let travel = 14.66 * s;
        let ov = self.overshoot * s;
        // keyframes in eased time: 0 -> end + overshoot (55%) -> end (80%) -> end
        let u = MotionCurve::new(0.34, 1.35, 0.64, 1.0).at(ph.t).clamp(0.0, 1.0);
        let x = if ph.age > 1e5 {
            if on { travel } else { 0.0 }
        } else {
            let (a, b, ovs) = if on { (0.0, travel, ov) } else { (travel, 0.0, -ov) };
            if u < 0.55 {
                a + (b + ovs - a) * (u / 0.55)
            } else if u < 0.8 {
                b + ovs - ovs * ((u - 0.55) / 0.25)
            } else {
                b
            }
        };
        // a little stretch while it travels
        let speed = if ph.t < 1.0 { (1.0 - (2.0 * ph.t - 1.0).abs()).max(0.0) } else { 0.0 };
        let thumb_w = 22.0 * s + 3.0 * s * speed;
        let thumb = Rect::from_min_size(pos2(track.min.x + 4.0 * s + x - if on { 3.0 * s * speed } else { 0.0 }, track.center().y - 7.0 * s), vec2(thumb_w, 14.0 * s));
        paint::soft_shadow(painter, thumb, 7.0 * s, 4.0, vec2(0.0, 1.0), Color32::from_black_alpha(40));
        let thumb_col = if on { glowkit_core::color::contrast_text(on_col) } else { Color32::WHITE };
        painter.rect_filled(thumb, cr(7.0 * s), thumb_col);
        response
    }
}
