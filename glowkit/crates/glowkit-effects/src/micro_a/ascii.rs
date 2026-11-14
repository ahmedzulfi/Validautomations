//! A cube drawn in ink dots that the pointer drags a wake of characters through.

use super::common::idle_for;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Pos2, Rect, Response, Sense, Ui, Vec2};
use glowkit_core::anim::lerp;
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// Characters an [`AsciiWake`] uses for its wake, from light to heavy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GlyphSet {
    /// `. : - = + * # % @`
    #[default]
    Ascii,
    /// Shade blocks `░ ▒ ▓ █`.
    Blocks,
    /// Bigger and bigger dots only.
    Dots,
    /// `0` and `1`.
    Binary,
}

impl GlyphSet {
    fn chars(self) -> &'static [char] {
        match self {
            GlyphSet::Ascii => &['.', ':', '-', '=', '+', '*', '#', '%', '@'],
            GlyphSet::Blocks => &['·', '░', '▒', '▓', '█'],
            GlyphSet::Dots => &[],
            GlyphSet::Binary => &['0', '1'],
        }
    }
}

#[derive(Clone, Default)]
struct WakeField {
    cols: usize,
    rows: usize,
    heat: Vec<f32>,
    disp: Vec<Vec2>,
    last: Option<Pos2>,
    rings: Vec<(Pos2, f32)>,
}

/// A slowly turning cube drawn as a halftone of ink dots (bigger dots on darker faces). Move
/// the pointer through it and the dots under the hand heat up into ASCII characters that
/// thicken where the pointer is and cool back to dots behind it; characters are dragged along
/// with the motion. Pressing sends a ring of characters out from the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{AsciiWake, GlyphSet};
/// ui.add(AsciiWake::new().glyphs(GlyphSet::Ascii).reach(60.0).trail(1.2).cell(10.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AsciiWake {
    size: f32,
    glyphs: GlyphSet,
    reach: f32,
    trail: f32,
    pull: f32,
    cell: f32,
    spin: f32,
    color: Option<Color32>,
    autoplay: bool,
}

impl Default for AsciiWake {
    fn default() -> Self {
        Self::new()
    }
}

impl AsciiWake {
    /// A 280 pt cube.
    pub fn new() -> Self {
        Self { size: 280.0, glyphs: GlyphSet::Ascii, reach: 60.0, trail: 1.2, pull: 40.0, cell: 10.0, spin: 1.0, color: None, autoplay: false }
    }
    /// Size of the square canvas in points.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(80.0);
        self
    }
    /// Characters used for the wake.
    pub fn glyphs(mut self, g: GlyphSet) -> Self {
        self.glyphs = g;
        self
    }
    /// Radius of the pointer's influence, 0..100.
    pub fn reach(mut self, r: f32) -> Self {
        self.reach = r.clamp(0.0, 100.0);
        self
    }
    /// Seconds the wake takes to cool back to dots.
    pub fn trail(mut self, s: f32) -> Self {
        self.trail = s.max(0.05);
        self
    }
    /// How far characters are dragged along with the pointer, 0..100.
    pub fn pull(mut self, p: f32) -> Self {
        self.pull = p.clamp(0.0, 100.0);
        self
    }
    /// Grid cell size in points (smaller = finer halftone).
    pub fn cell(mut self, c: f32) -> Self {
        self.cell = c.clamp(5.0, 24.0);
        self
    }
    /// Rotation speed multiplier (0 stops the cube).
    pub fn spin(mut self, s: f32) -> Self {
        self.spin = s;
        self
    }
    /// Ink color (default: theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Sweep a virtual pointer through the cube while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

type M3 = [[f32; 3]; 3];

fn rot(ax: f32, ay: f32, az: f32) -> M3 {
    let (sx, cx) = ax.sin_cos();
    let (sy, cy) = ay.sin_cos();
    let (sz, cz) = az.sin_cos();
    let rx = [[1.0, 0.0, 0.0], [0.0, cx, -sx], [0.0, sx, cx]];
    let ry = [[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]];
    let rz = [[cz, -sz, 0.0], [sz, cz, 0.0], [0.0, 0.0, 1.0]];
    mul(rx, mul(ry, rz))
}

fn mul(a: M3, b: M3) -> M3 {
    let mut m = [[0.0; 3]; 3];
    for (i, row) in m.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    m
}

fn apply(m: M3, v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

fn transpose(m: M3) -> M3 {
    [[m[0][0], m[1][0], m[2][0]], [m[0][1], m[1][1], m[2][1]], [m[0][2], m[1][2], m[2][2]]]
}

/// Orthographic ray (+z into the screen) against the unit cube; returns the view-space normal
/// of the face hit.
fn hit_cube(inv: M3, fwd: M3, x: f32, y: f32) -> Option<[f32; 3]> {
    let o = apply(inv, [x, y, -5.0]);
    let d = apply(inv, [0.0, 0.0, 1.0]);
    let mut tmin = f32::NEG_INFINITY;
    let mut tmax = f32::INFINITY;
    let mut axis = 0;
    let mut sign = 0.0;
    for k in 0..3 {
        if d[k].abs() < 1e-6 {
            if o[k].abs() > 1.0 {
                return None;
            }
            continue;
        }
        let t1 = (-1.0 - o[k]) / d[k];
        let t2 = (1.0 - o[k]) / d[k];
        let (a, b) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
        if a > tmin {
            tmin = a;
            axis = k;
            sign = if t1 < t2 { -1.0 } else { 1.0 };
        }
        tmax = tmax.min(b);
    }
    if tmin > tmax {
        return None;
    }
    let mut n = [0.0; 3];
    n[axis] = sign;
    Some(apply(fwd, n))
}

impl egui::Widget for AsciiWake {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (outer, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.size), Sense::click());
        let rect = Rect::from_center_size(outer.center(), Vec2::splat(self.size));
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let hover = response.hover_pos();
        let idle = idle_for(ui, id, hover.is_some());
        let cell = self.cell;
        let cols = (rect.width() / cell) as usize;
        let rows = (rect.height() / cell) as usize;
        let origin = rect.center() - vec2(cols as f32, rows as f32) * cell / 2.0;
        let cell_c = |i: usize, j: usize| origin + vec2((i as f32 + 0.5) * cell, (j as f32 + 0.5) * cell);
        // the hand: real pointer or a lissajous sweep
        let mut hand = hover;
        let mut press = if response.clicked() { hover } else { None };
        if hand.is_none() && self.autoplay && idle > 0.6 {
            let u = idle - 0.6;
            hand = Some(rect.center() + vec2((u * 0.8).sin() * 0.34, (u * 0.55 + 0.6).sin() * 0.26) * self.size);
            let k = (u / 2.8) as i32;
            let key: i32 = glowkit_core::state(ui, id.with("auto_ring"), || -1);
            if k != key {
                glowkit_core::set_state(ui, id.with("auto_ring"), k);
                press = hand;
            }
        }
        let reach = lerp(18.0, 110.0, self.reach / 100.0);
        let decay = (-dt / (self.trail * 0.35)).exp();
        let pull = self.pull / 100.0;
        let ring_speed = self.size * 0.9;
        let mut f: WakeField = glowkit_core::state(ui, id.with("field"), WakeField::default);
        if f.cols != cols || f.rows != rows {
            f = WakeField { cols, rows, heat: vec![0.0; cols * rows], disp: vec![Vec2::ZERO; cols * rows], ..Default::default() };
        }
        if let Some(p) = press {
            f.rings.push((p, now));
        }
        f.rings.retain(|(_, t0)| (now - t0) * ring_speed < self.size);
        let prev = f.last.or(hand);
        let vel = match (hand, prev) {
            (Some(a), Some(b)) => (a - b) / dt.max(1e-3),
            _ => Vec2::ZERO,
        };
        for j in 0..rows {
            for i in 0..cols {
                let k = j * cols + i;
                let c = cell_c(i, j);
                let mut h = f.heat[k] * decay;
                f.disp[k] *= decay;
                if let (Some(a), Some(b)) = (hand, prev) {
                    // distance to the segment the hand swept this frame
                    let ab = a - b;
                    let tt = if ab.length_sq() > 0.0 { ((c - b).dot(ab) / ab.length_sq()).clamp(0.0, 1.0) } else { 1.0 };
                    let d = (c - (b + ab * tt)).length();
                    if d < reach {
                        let w = 1.0 - d / reach;
                        let w = w * w * (3.0 - 2.0 * w);
                        h = h.max(w);
                        f.disp[k] += vel * w * pull * 0.004;
                    }
                }
                for (rc, t0) in &f.rings {
                    let r = (now - t0) * ring_speed;
                    let d = ((c - *rc).length() - r).abs();
                    if d < cell * 1.6 {
                        let fade = 1.0 - r / self.size;
                        h = h.max((1.0 - d / (cell * 1.6)) * fade);
                    }
                }
                let len = f.disp[k].length();
                if len > cell * 2.5 {
                    f.disp[k] *= cell * 2.5 / len;
                }
                f.heat[k] = h.min(1.0);
            }
        }
        f.last = hand;
        glowkit_core::set_state(ui, id.with("field"), f.clone());
        glowkit_core::animate(ui);
        if hover.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let ink = self.color.unwrap_or(t.foreground);
        let fwd = rot(-0.5, now * 0.25 * self.spin + 0.7, 0.0);
        let inv = transpose(fwd);
        let scale = self.size * 0.29;
        let light = {
            let l = [-0.45f32, -0.75, -0.5];
            let n = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt();
            [l[0] / n, l[1] / n, l[2] / n]
        };
        let chars = self.glyphs.chars();
        let font = FontId::monospace(cell * 1.3);
        for j in 0..rows {
            for i in 0..cols {
                let c = cell_c(i, j);
                let l = (c - rect.center()) / scale;
                let Some(n) = hit_cube(inv, fwd, l.x, l.y) else { continue };
                // n points toward the camera (-z); lambert against the light
                let lambert = n[0] * light[0] + n[1] * light[1] + n[2] * light[2];
                let shade = lambert.clamp(0.0, 1.0);
                let dark = 1.0 - shade * 0.75;
                let k = j * cols + i;
                let h = f.heat[k];
                let p = c + f.disp[k];
                if h < 0.28 || chars.is_empty() {
                    let r = cell * (0.13 + 0.25 * dark) * (1.0 + h * 0.9);
                    painter.circle_filled(p, r.min(cell * 0.62), with_alpha(ink, 0.92));
                } else {
                    let lvl = (((h - 0.28) / 0.72 * 0.8 + dark * 0.3).min(1.0) * (chars.len() - 1) as f32).round() as usize;
                    let ch = if self.glyphs == GlyphSet::Binary {
                        chars[(i * 7 + j * 13 + (now * 6.0) as usize) % 2]
                    } else {
                        chars[lvl.min(chars.len() - 1)]
                    };
                    let a = (0.45 + 0.55 * h).min(1.0);
                    painter.text(pos2(p.x, p.y), Align2::CENTER_CENTER, ch, font.clone(), with_alpha(ink, a));
                }
            }
        }
        response
    }
}
