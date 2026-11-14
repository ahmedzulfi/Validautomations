//! Shared plumbing for the transition widgets: cubic-bezier curves, open/close phases, value
//! change tracking, transformed / faded / blurred painting layers and the quiet "studio" look
//! (white material cards, skeleton bars, soft layered shadows).

use egui::emath::TSTransform;
use egui::layers::ShapeIdx;
use egui::{pos2, vec2, Color32, CornerRadius, FontId, Id, Painter, Pos2, Rect, Shape, Stroke, Ui, Vec2};
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::TAU;

/// A CSS-style `cubic-bezier(x1, y1, x2, y2)` timing curve. Every transition widget takes one, so
/// you can retune the feel without touching the widget.
///
/// ```no_run
/// use glowkit_interactions::MotionCurve;
/// let eased = MotionCurve::SMOOTH_OUT.at(0.5);
/// let custom = MotionCurve::new(0.34, 1.56, 0.64, 1.0); // a springier overshoot
/// # let _ = (eased, custom);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionCurve {
    /// First control point, x (0..1).
    pub x1: f32,
    /// First control point, y (may leave 0..1 for anticipation / overshoot).
    pub y1: f32,
    /// Second control point, x (0..1).
    pub x2: f32,
    /// Second control point, y (may leave 0..1 for anticipation / overshoot).
    pub y2: f32,
}

impl MotionCurve {
    /// Straight line.
    pub const LINEAR: Self = Self::new(0.0, 0.0, 1.0, 1.0);
    /// The signature long, smooth deceleration (`0.22, 1, 0.36, 1`).
    pub const SMOOTH_OUT: Self = Self::new(0.22, 1.0, 0.36, 1.0);
    /// A small springy overshoot (`0.34, 1.36, 0.64, 1`).
    pub const BOUNCE: Self = Self::new(0.34, 1.36, 0.64, 1.0);
    /// A softer overshoot (`0.34, 1.25, 0.64, 1`).
    pub const SOFT_BOUNCE: Self = Self::new(0.34, 1.25, 0.64, 1.0);
    /// CSS `ease-in-out`.
    pub const EASE_IN_OUT: Self = Self::new(0.42, 0.0, 0.58, 1.0);
    /// CSS `ease-out`.
    pub const EASE_OUT: Self = Self::new(0.0, 0.0, 0.58, 1.0);
    /// CSS `ease`.
    pub const EASE: Self = Self::new(0.25, 0.1, 0.25, 1.0);
    /// Material "standard" curve, used for quick exits (`0.4, 0, 0.2, 1`).
    pub const STANDARD: Self = Self::new(0.4, 0.0, 0.2, 1.0);

    /// A curve from its two control points.
    pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    fn bez(a: f32, b: f32, s: f32) -> f32 {
        let u = 1.0 - s;
        3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s
    }

    fn bez_d(a: f32, b: f32, s: f32) -> f32 {
        let u = 1.0 - s;
        3.0 * u * u * a + 6.0 * u * s * (b - a) + 3.0 * s * s * (1.0 - b)
    }

    /// Evaluate the curve at linear progress `t` (clamped to `0..=1`).
    pub fn at(self, t: f32) -> f32 {
        if t <= 0.0 {
            return 0.0;
        }
        if t >= 1.0 {
            return 1.0;
        }
        // Newton first, bisection as a fallback.
        let mut s = t;
        for _ in 0..8 {
            let x = Self::bez(self.x1, self.x2, s) - t;
            if x.abs() < 1e-5 {
                return Self::bez(self.y1, self.y2, s);
            }
            let d = Self::bez_d(self.x1, self.x2, s);
            if d.abs() < 1e-6 {
                break;
            }
            s = (s - x / d).clamp(0.0, 1.0);
        }
        let (mut lo, mut hi) = (0.0f32, 1.0f32);
        s = t;
        for _ in 0..30 {
            let x = Self::bez(self.x1, self.x2, s);
            if (x - t).abs() < 1e-5 {
                break;
            }
            if x < t {
                lo = s;
            } else {
                hi = s;
            }
            s = (lo + hi) * 0.5;
        }
        Self::bez(self.y1, self.y2, s)
    }

    /// Progress of a `dur`-second transition `age` seconds in, optionally `delay`ed.
    pub fn timed(self, age: f32, delay: f32, dur: f32) -> f32 {
        self.at((age - delay) / dur.max(1e-4))
    }
}

/// An underdamped spring step response (0 -> 1 with overshoot), evaluated analytically at `t`
/// seconds. `omega` is the natural frequency (rad/s), `zeta` the damping ratio (< 1 bounces).
pub(crate) fn spring_at(t: f32, omega: f32, zeta: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    let z = zeta.clamp(0.05, 0.999);
    let wd = omega * (1.0 - z * z).sqrt();
    let e = (-z * omega * t).exp();
    1.0 - e * ((wd * t).cos() + z * omega / wd * (wd * t).sin())
}

/// Linear interpolation.
#[inline]
pub(crate) fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Linear progress through `[a, b]`, clamped.
#[inline]
pub(crate) fn span(x: f32, a: f32, b: f32) -> f32 {
    ((x - a) / (b - a).max(1e-5)).clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------------------------
// Time

#[derive(Clone, Copy, Debug)]
struct FlipState {
    on: bool,
    since: f64,
    from: f32,
    last: f32,
}

/// Where an open/close (on/off) transition is.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Phase {
    /// The current target.
    pub on: bool,
    /// Seconds since the target last changed (huge when it never did).
    pub age: f32,
    /// Linear progress of the running transition (0..1).
    pub t: f32,
    /// Eased value: 0 = fully off, 1 = fully on (may overshoot with bouncy curves). Starts from
    /// wherever the previous transition was when it got interrupted.
    pub p: f32,
}

/// Track a boolean target and ease a 0..1 value toward it with separate on/off durations and
/// curves. Requests repaints while moving.
pub(crate) fn phase(
    ui: &Ui,
    id: Id,
    on: bool,
    dur_on: f32,
    dur_off: f32,
    c_on: MotionCurve,
    c_off: MotionCurve,
) -> Phase {
    let now = ui.input(|i| i.time);
    let ph = glowkit_core::with_state(
        ui,
        id.with("tx_phase"),
        || FlipState { on, since: -1e6, from: on as u8 as f32, last: on as u8 as f32 },
        |f| {
            if f.on != on {
                f.on = on;
                f.since = now;
                f.from = f.last;
            }
            let age = (now - f.since) as f32;
            let dur = if on { dur_on } else { dur_off };
            let t = (age / dur.max(1e-4)).clamp(0.0, 1.0);
            let c = if on { c_on } else { c_off };
            let target = on as u8 as f32;
            let p = f.from + (target - f.from) * c.at(t);
            f.last = p;
            Phase { on, age, t, p }
        },
    );
    if ph.t < 1.0 {
        ui.ctx().request_repaint();
    }
    ph
}

#[derive(Clone, Debug)]
struct ChangeState<T> {
    cur: T,
    prev: T,
    since: f64,
}

/// A value and the one it replaced, with the seconds since the change.
#[derive(Clone, Debug)]
pub(crate) struct Change<T> {
    pub prev: T,
    pub age: f32,
}

/// Track changes of `value`, remembering the previous one and when it changed. Requests repaints
/// for `horizon` seconds after a change.
pub(crate) fn change<T: Clone + PartialEq + Send + Sync + 'static>(ui: &Ui, id: Id, value: &T, horizon: f32) -> Change<T> {
    let now = ui.input(|i| i.time);
    let c = glowkit_core::with_state(
        ui,
        id.with("tx_change"),
        || ChangeState { cur: value.clone(), prev: value.clone(), since: -1e6 },
        |s| {
            if s.cur != *value {
                s.prev = std::mem::replace(&mut s.cur, value.clone());
                s.since = now;
            }
            Change { prev: s.prev.clone(), age: (now - s.since) as f32 }
        },
    );
    if c.age < horizon {
        ui.ctx().request_repaint();
    }
    c
}

/// Seconds since `active` was last true for `id` (starts large).
pub(crate) fn idle_for(ui: &Ui, id: Id, active: bool) -> f32 {
    let now = ui.input(|i| i.time);
    glowkit_core::with_state(
        ui,
        id.with("tx_idle"),
        || now - 1000.0,
        |last| {
            if active {
                *last = now;
            }
            (now - *last) as f32
        },
    )
}

// ---------------------------------------------------------------------------------------------
// Layers: paint anything, then move / scale / fade / blur it as one piece

/// Paint with `f`, then transform everything it painted by `xf`, fade it to `opacity` and
/// apply a fake gaussian blur of roughly `blur` points.
pub(crate) fn layer(ui: &Ui, xf: TSTransform, opacity: f32, blur: f32, f: impl FnOnce(&Painter)) {
    if opacity <= 0.004 {
        return;
    }
    let lid = ui.layer_id();
    let start = ui.ctx().graphics(|g| g.get(lid).map_or(0, |l| l.next_idx().0));
    let mut p = ui.painter().clone();
    p.multiply_opacity(opacity.min(1.0));
    f(&p);
    ui.ctx().graphics_mut(|g| {
        let list = g.entry(lid);
        let end = list.next_idx();
        if xf != TSTransform::IDENTITY {
            list.transform_range(ShapeIdx(start), end, xf);
        }
        if blur > 0.2 {
            let k = (blur / 2.5).clamp(0.0, 1.0);
            let mut copies = Vec::with_capacity(end.0 - start);
            for i in start..end.0 {
                list.mutate_shape(ShapeIdx(i), |cs| {
                    copies.push(cs.clone());
                    fade_shape(&mut cs.shape, 1.0 - 0.62 * k);
                });
            }
            let taps = 8;
            let a = 0.2 * k;
            for i in 0..taps {
                let ang = i as f32 / taps as f32 * TAU + 0.3;
                let r = blur * if i % 2 == 0 { 0.9 } else { 0.45 };
                let d = vec2(ang.cos(), ang.sin()) * r;
                for cs in &copies {
                    let mut s = cs.shape.clone();
                    s.translate(d);
                    fade_shape(&mut s, a);
                    list.add(cs.clip_rect, s);
                }
            }
        }
    });
}

fn fade_shape(s: &mut Shape, a: f32) {
    match s {
        Shape::Text(t) => t.opacity_factor *= a,
        Shape::Vec(v) => v.iter_mut().for_each(|s| fade_shape(s, a)),
        _ => egui::epaint::shape_transform::adjust_colors(s, move |c| {
            if *c != Color32::PLACEHOLDER {
                *c = c.gamma_multiply(a);
            }
        }),
    }
}

/// A transform that scales by `s` around `origin` and then moves by `offset`.
pub(crate) fn xf_around(origin: Pos2, s: f32, offset: Vec2) -> TSTransform {
    TSTransform::from_translation(origin.to_vec2() + offset)
        * TSTransform::from_scaling(s)
        * TSTransform::from_translation(-origin.to_vec2())
}

// ---------------------------------------------------------------------------------------------
// Look

/// Corner radius from points.
#[inline]
pub(crate) fn cr(r: f32) -> CornerRadius {
    CornerRadius::same(r.round().clamp(0.0, 255.0) as u8)
}

/// The raised "material" card color.
pub(crate) fn surface(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.muted, 0.06)
    } else {
        t.card
    }
}

/// The pale bar color used for skeleton / placeholder lines.
pub(crate) fn skeleton(t: &Theme) -> Color32 {
    if t.dark {
        with_alpha(Color32::WHITE, 0.08)
    } else {
        mix(t.muted, t.border, 0.25)
    }
}

/// The quiet pill used for secondary buttons.
pub(crate) fn chip(t: &Theme) -> Color32 {
    if t.dark {
        with_alpha(Color32::WHITE, 0.06)
    } else {
        mix(t.muted, t.border, 0.35)
    }
}

/// Paint the layered "material" shadow, fill and hairline of a floating surface.
pub(crate) fn material(painter: &Painter, t: &Theme, rect: Rect, r: f32, fill: Color32) {
    if t.dark {
        paint::soft_shadow(painter, rect, r, 10.0, vec2(0.0, 3.0), Color32::from_black_alpha(90));
        painter.rect(rect, cr(r), fill, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.07)), egui::StrokeKind::Inside);
    } else {
        paint::soft_shadow(painter, rect, r, 40.0, vec2(0.0, 4.0), Color32::from_black_alpha(14));
        paint::soft_shadow(painter, rect, r, 6.0, vec2(0.0, 2.0), Color32::from_black_alpha(12));
        painter.rect(rect, cr(r), fill, Stroke::new(1.0, Color32::from_black_alpha(15)), egui::StrokeKind::Outside);
    }
}

/// A rounded skeleton bar.
pub(crate) fn bar(painter: &Painter, t: &Theme, rect: Rect) {
    painter.rect_filled(rect, cr(rect.height() / 2.0), skeleton(t));
}

/// Width of a single line of text.
pub(crate) fn text_width(painter: &Painter, s: &str, font: &FontId) -> f32 {
    painter.layout_no_wrap(s.to_owned(), font.clone(), Color32::WHITE).size().x
}

/// The palette used for painted avatars (pairs of gradient stops).
pub(crate) const AVATAR_TONES: [[Color32; 2]; 8] = [
    [Color32::from_rgb(0xF9, 0xA8, 0xD4), Color32::from_rgb(0xA8, 0x55, 0xF7)],
    [Color32::from_rgb(0x93, 0xC5, 0xFD), Color32::from_rgb(0x63, 0x66, 0xF1)],
    [Color32::from_rgb(0xFD, 0xBA, 0x74), Color32::from_rgb(0xEF, 0x44, 0x44)],
    [Color32::from_rgb(0x86, 0xEF, 0xAC), Color32::from_rgb(0x14, 0xB8, 0xA6)],
    [Color32::from_rgb(0xFD, 0xE6, 0x8A), Color32::from_rgb(0xF5, 0x9E, 0x0B)],
    [Color32::from_rgb(0xA5, 0xB4, 0xFC), Color32::from_rgb(0x8B, 0x5C, 0xF6)],
    [Color32::from_rgb(0x67, 0xE8, 0xF9), Color32::from_rgb(0x25, 0x63, 0xEB)],
    [Color32::from_rgb(0xFC, 0xA5, 0xA5), Color32::from_rgb(0xDB, 0x27, 0x77)],
];

/// A painted portrait avatar: a gradient disc with a head-and-shoulders silhouette.
pub(crate) fn avatar(painter: &Painter, center: Pos2, r: f32, tone: [Color32; 2], ring: Option<Color32>) {
    if let Some(ring) = ring {
        painter.circle_filled(center, r + (r * 0.09).max(1.5), ring);
    }
    let rect = Rect::from_center_size(center, Vec2::splat(r * 2.0));
    let pts = paint::circle_points(center, r, 48);
    paint::fill_convex_fn(painter, &pts, 3, paint::linear_fn(rect, 135.0, &tone));
    let ink = with_alpha(Color32::WHITE, 0.82);
    // shoulders: a clipped ellipse sitting on the bottom edge
    let clip = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
    let sh_c = center + vec2(0.0, r * 0.98);
    let sh: Vec<Pos2> = (0..=24)
        .map(|i| {
            let a = std::f32::consts::PI + i as f32 / 24.0 * std::f32::consts::PI;
            sh_c + vec2(a.cos() * r * 0.68, a.sin() * r * 0.52)
        })
        .collect();
    // keep only what is inside the disc
    let inside: Vec<Pos2> = sh
        .into_iter()
        .map(|p| {
            let d = p - center;
            if d.length() > r * 0.97 {
                center + d.normalized() * r * 0.97
            } else {
                p
            }
        })
        .collect();
    clip.add(Shape::convex_polygon(inside, ink, Stroke::NONE));
    clip.circle_filled(center + vec2(0.0, -r * 0.16), r * 0.3, ink);
}

/// A "+N" chip in avatar size.
pub(crate) fn more_chip(painter: &Painter, t: &Theme, center: Pos2, r: f32, n: usize, ring: Color32) {
    painter.circle_filled(center, r + (r * 0.09).max(1.5), ring);
    painter.circle_filled(center, r, if t.dark { shade(t.muted, 0.14) } else { mix(t.muted, t.border, 0.8) });
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        format!("+{n}"),
        FontId::proportional(r * 0.78),
        t.foreground,
    );
}

/// Points of a cubic bezier.
pub(crate) fn cubic(a: Pos2, b: Pos2, c: Pos2, d: Pos2, n: usize) -> Vec<Pos2> {
    paint::cubic_bezier(a, b, c, d, n)
}

/// Partial polyline: the first `k` (0..1) of its length.
pub(crate) fn trim(pts: &[Pos2], k: f32) -> Vec<Pos2> {
    if pts.len() < 2 || k >= 1.0 {
        return pts.to_vec();
    }
    if k <= 0.0 {
        return Vec::new();
    }
    let total: f32 = pts.windows(2).map(|w| (w[1] - w[0]).length()).sum();
    let goal = total * k;
    let mut acc = 0.0;
    let mut out = vec![pts[0]];
    for w in pts.windows(2) {
        let l = (w[1] - w[0]).length();
        if acc + l >= goal {
            let f = (goal - acc) / l.max(1e-5);
            out.push(w[0] + (w[1] - w[0]) * f);
            return out;
        }
        acc += l;
        out.push(w[1]);
    }
    out
}

/// A stroked line with round caps (egui paths have butt caps, so dot the ends).
pub(crate) fn round_line(painter: &Painter, pts: &[Pos2], width: f32, color: Color32) {
    if pts.len() < 2 {
        return;
    }
    painter.add(Shape::line(pts.to_vec(), Stroke::new(width, color)));
    painter.circle_filled(pts[0], width / 2.0, color);
    painter.circle_filled(*pts.last().unwrap_or(&pts[0]), width / 2.0, color);
}

/// Painted bell outline, centered on `c`, fitting a `size` box.
pub(crate) fn bell(painter: &Painter, c: Pos2, size: f32, color: Color32, width: f32, angle: f32) {
    let s = size / 16.0;
    let (sn, cs) = angle.sin_cos();
    let pivot = vec2(0.0, (1.35 - 8.0) * s);
    let p = |x: f32, y: f32| {
        let v = vec2((x - 8.0) * s, (y - 8.0) * s) - pivot;
        c + pivot + vec2(v.x * cs - v.y * sn, v.x * sn + v.y * cs)
    };
    let mut body = Vec::new();
    body.extend(cubic(p(2.6, 11.3), p(3.6, 10.0), p(4.0, 8.2), p(4.0, 5.4), 8));
    body.extend(cubic(p(4.0, 5.4), p(4.0, 3.1), p(5.8, 1.35), p(8.0, 1.35), 8));
    body.extend(cubic(p(8.0, 1.35), p(10.2, 1.35), p(12.0, 3.1), p(12.0, 5.4), 8));
    body.extend(cubic(p(12.0, 5.4), p(12.0, 8.2), p(12.4, 10.0), p(13.4, 11.3), 8));
    body.push(p(2.6, 11.3));
    painter.add(Shape::line(body, Stroke::new(width, color)));
    let clapper = cubic(p(6.3, 13.9), p(6.8, 14.4), p(7.4, 14.6), p(8.0, 14.6), 6)
        .into_iter()
        .chain(cubic(p(8.0, 14.6), p(8.6, 14.6), p(9.2, 14.4), p(9.7, 13.9), 6))
        .collect::<Vec<_>>();
    round_line(painter, &clapper, width, color);
}

/// Linear position of a point on the segment between `a` and `b`.
#[inline]
pub(crate) fn along(a: Pos2, b: Pos2, t: f32) -> Pos2 {
    pos2(lerp(a.x, b.x, t), lerp(a.y, b.y, t))
}

/// A value easing from `from` to `to`, started at `since`; retargeting starts from wherever it is.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Tween {
    pub from: f32,
    pub to: f32,
    pub since: f64,
}

impl Tween {
    /// A tween resting at `v`.
    pub fn new(v: f32) -> Self {
        Self { from: v, to: v, since: -1e6 }
    }
    /// Current value.
    pub fn value(&self, now: f64, dur: f32, curve: MotionCurve) -> f32 {
        let k = curve.at(((now - self.since) as f32) / dur.max(1e-4));
        lerp(self.from, self.to, k)
    }
    /// Move toward `to` (no-op when already heading there).
    pub fn retarget(&mut self, to: f32, now: f64, dur: f32, curve: MotionCurve) {
        if (to - self.to).abs() > 1e-4 {
            self.from = self.value(now, dur, curve);
            self.to = to;
            self.since = now;
        }
    }
    /// Jump straight to `v`.
    pub fn snap(&mut self, v: f32) {
        *self = Self::new(v);
    }
    /// Whether the tween is still moving.
    pub fn moving(&self, now: f64, dur: f32) -> bool {
        ((now - self.since) as f32) < dur
    }
}

/// Paint a title + subtitle pair left-aligned at `x`, vertically centered on `cy`.
pub(crate) fn two_lines(painter: &Painter, t: &Theme, x: f32, cy: f32, title: &str, sub: &str, max_w: f32) {
    let tf = FontId::proportional(13.0);
    let sf = FontId::proportional(12.0);
    let tg = painter.layout(title.to_owned(), tf, t.foreground, max_w);
    let sg = painter.layout(sub.to_owned(), sf, t.muted_foreground, max_w);
    let h = tg.size().y + sg.size().y + 2.0;
    let y0 = cy - h / 2.0;
    painter.galley(pos2(x, y0), tg.clone(), t.foreground);
    painter.galley(pos2(x, y0 + tg.size().y + 2.0), sg, t.muted_foreground);
}

/// A soft rounded "chip" button with a label; returns its response. `press` scales it on press.
pub(crate) fn chip_button(ui: &mut Ui, label: &str) -> egui::Response {
    let t = Theme::of(ui);
    let font = FontId::proportional(13.0);
    let w = text_width(ui.painter(), label, &font) + 28.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 32.0), egui::Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if ui.is_rect_visible(rect) {
        let down = resp.is_pointer_button_down_on();
        let r = if down { rect.shrink(1.0) } else { rect };
        let fill = if resp.hovered() { shade(chip(&t), if t.dark { 0.04 } else { -0.03 }) } else { chip(&t) };
        ui.painter().rect_filled(r, cr(r.height() / 2.0), fill);
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, label, font, t.foreground);
    }
    resp
}

/// Run `add` (which may add real widgets), then move / scale / fade / blur everything it painted.
pub(crate) fn ui_layer<R>(ui: &mut Ui, xf: TSTransform, opacity: f32, blur: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let lid = ui.layer_id();
    let start = ui.ctx().graphics(|g| g.get(lid).map_or(0, |l| l.next_idx().0));
    let r = add(ui);
    ui.ctx().graphics_mut(|g| {
        let list = g.entry(lid);
        let end = list.next_idx();
        if xf != TSTransform::IDENTITY {
            list.transform_range(ShapeIdx(start), end, xf);
        }
        let k = (blur / 2.5).clamp(0.0, 1.0);
        let mut copies = Vec::new();
        for i in start..end.0 {
            list.mutate_shape(ShapeIdx(i), |cs| {
                if blur > 0.2 {
                    copies.push(cs.clone());
                }
                let a = opacity.clamp(0.0, 1.0) * if blur > 0.2 { 1.0 - 0.62 * k } else { 1.0 };
                if a < 0.999 {
                    fade_shape(&mut cs.shape, a);
                }
            });
        }
        if blur > 0.2 {
            for i in 0..8 {
                let ang = i as f32 / 8.0 * TAU + 0.3;
                let d = vec2(ang.cos(), ang.sin()) * blur * if i % 2 == 0 { 0.9 } else { 0.45 };
                for cs in &copies {
                    let mut sh = cs.shape.clone();
                    sh.translate(d);
                    fade_shape(&mut sh, 0.2 * k * opacity.clamp(0.0, 1.0));
                    list.add(cs.clip_rect, sh);
                }
            }
        }
    });
    r
}

/// A disc with a soft (fake-blurred) edge.
pub(crate) fn soft_disc(painter: &Painter, c: Pos2, r: f32, blur: f32, color: Color32) {
    if blur < 0.4 {
        painter.circle_filled(c, r, color);
        return;
    }
    let n = 7;
    for i in 0..n {
        let f = i as f32 / (n - 1) as f32;
        let rr = (r - blur * 0.5 + blur * f).max(0.5);
        painter.circle_filled(c, rr, color.gamma_multiply(if i == 0 { 1.0 } else { 0.22 * (1.0 - f) + 0.06 }));
    }
}
