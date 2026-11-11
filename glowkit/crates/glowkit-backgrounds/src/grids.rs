//! Grid-based backgrounds: moving squares, dot fields, grid patterns, retro floor, flicker,
//! skewed boxes and hexagons.

use crate::{background, ghost, h01, light, pointer, radial_mask, smoothstep, Base, Frame};
use egui::epaint::{Mesh, PathShape, PathStroke, Shape};
use egui::{pos2, vec2, Color32, Pos2, Rect, Stroke, Ui, Vec2};
use glowkit_core::color::{mix, with_alpha};
use std::f32::consts::{PI, TAU};

/// Cells that were hovered recently, fading out.
#[derive(Clone, Default)]
struct Trail {
    cells: Vec<(i32, i32, f32, u8)>,
    seq: u32,
}

impl Trail {
    /// Light up `cell` (full alpha) and fade the others by `fade` per second.
    fn update(&mut self, cell: Option<(i32, i32)>, dt: f32, fade: f32, ncolors: usize) {
        for c in &mut self.cells {
            c.2 -= dt * fade;
        }
        if let Some((x, y)) = cell {
            if let Some(c) = self.cells.iter_mut().find(|c| c.0 == x && c.1 == y) {
                c.2 = 1.0;
            } else {
                self.seq = self.seq.wrapping_add(1);
                let ci = (h01(self.seq, 7) * ncolors.max(1) as f32) as u8;
                self.cells.push((x, y, 1.0, ci));
            }
        }
        self.cells.retain(|c| c.2 > 0.0);
    }
}

/// Pointer, or a wandering phantom pointer when `auto` is on.
fn hover_point(ui: &Ui, f: &Frame, auto: bool) -> Option<Pos2> {
    pointer(ui, f.rect).or_else(|| auto.then(|| ghost(f.rect, f.time * 0.9)))
}

fn line_color(f: &Frame) -> Color32 {
    let t = &f.theme;
    if t.dark {
        mix(t.border, t.foreground, 0.12)
    } else {
        t.border
    }
}

// ---------------------------------------------------------------------------------------------
// Squares grid
// ---------------------------------------------------------------------------------------------

/// Direction a moving grid travels in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Direction {
    /// Moves right.
    Right,
    /// Moves left.
    Left,
    /// Moves up.
    Up,
    /// Moves down.
    Down,
    /// Moves down and to the right.
    #[default]
    Diagonal,
}

impl Direction {
    fn vec(self) -> Vec2 {
        match self {
            Direction::Right => vec2(1.0, 0.0),
            Direction::Left => vec2(-1.0, 0.0),
            Direction::Up => vec2(0.0, -1.0),
            Direction::Down => vec2(0.0, 1.0),
            Direction::Diagonal => vec2(1.0, 1.0),
        }
    }
}

/// An endlessly scrolling grid of squares. The square under the pointer fills in and fades out
/// as you move on, and the edges dissolve into a soft vignette.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::{Direction, SquaresGrid};
/// ui.add(SquaresGrid::new().direction(Direction::Diagonal).cell_size(40.0).speed(0.5));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct SquaresGrid {
    base: Base,
    cell: f32,
    direction: Direction,
    speed: f32,
    line: Option<Color32>,
    hover: Option<Color32>,
    vignette: bool,
    auto_hover: bool,
}

impl Default for SquaresGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl SquaresGrid {
    /// A diagonally moving 40 pt grid.
    pub fn new() -> Self {
        Self { base: Base::default(), cell: 40.0, direction: Direction::Diagonal, speed: 1.0, line: None, hover: None, vignette: true, auto_hover: false }
    }
    /// Cell size in points (default 40).
    pub fn cell_size(mut self, size: f32) -> Self {
        self.cell = size.max(6.0);
        self
    }
    /// Direction the grid travels (default diagonal).
    pub fn direction(mut self, d: Direction) -> Self {
        self.direction = d;
        self
    }
    /// Speed multiplier (default 1 = 20 points per second).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Grid line color (default: from the theme border).
    pub fn line_color(mut self, c: Color32) -> Self {
        self.line = Some(c);
        self
    }
    /// Fill of the hovered square (default: theme muted / accent tint).
    pub fn hover_color(mut self, c: Color32) -> Self {
        self.hover = Some(c);
        self
    }
    /// Fade the edges into the background (default true).
    pub fn vignette(mut self, on: bool) -> Self {
        self.vignette = on;
        self
    }
    /// When nobody hovers, a phantom pointer wanders around lighting squares (default false).
    pub fn auto_hover(mut self, on: bool) -> Self {
        self.auto_hover = on;
        self
    }

    /// Paint the grid into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_squares_grid") else { return };
        let t = &f.theme;
        let cell = self.cell;
        let off = (self.direction.vec() * f.time * 20.0 * self.speed).to_pos2();
        let off = vec2(off.x.rem_euclid(cell), off.y.rem_euclid(cell));
        let origin = rect.min + off - vec2(cell, cell);
        let hp = hover_point(ui, &f, self.auto_hover);
        let cell_of = |p: Pos2| (((p.x - origin.x) / cell).floor() as i32, ((p.y - origin.y) / cell).floor() as i32);
        // trail is stored in world cells (grid index + scroll) so it moves with the grid
        let scroll = self.direction.vec() * f.time * 20.0 * self.speed;
        let base_ix = ((scroll.x) / cell).floor() as i32;
        let base_iy = ((scroll.y) / cell).floor() as i32;
        let trail = glowkit_core::with_state(ui, f.id, Trail::default, |s| {
            s.update(hp.map(|p| {
                let (x, y) = cell_of(p);
                (x - base_ix, y - base_iy)
            }), f.dt, 1.8, 1);
            s.cells.clone()
        });
        let fill = self.hover.unwrap_or_else(|| if t.dark { mix(t.muted, t.accent, 0.45) } else { mix(t.muted, t.accent, 0.25) });
        for (x, y, a, _) in trail {
            let r = Rect::from_min_size(origin + vec2((x + base_ix) as f32 * cell, (y + base_iy) as f32 * cell), vec2(cell, cell));
            f.painter.rect_filled(r, 0.0, fill.gamma_multiply(a.min(1.0)));
        }
        let lc = self.line.unwrap_or_else(|| line_color(&f));
        let stroke = Stroke::new(1.0, lc);
        let mut x = origin.x;
        while x <= rect.max.x + 1.0 {
            f.painter.vline(x, rect.y_range(), stroke);
            x += cell;
        }
        let mut y = origin.y;
        while y <= rect.max.y + 1.0 {
            f.painter.hline(rect.x_range(), y, stroke);
            y += cell;
        }
        if self.vignette {
            radial_mask(&f.painter, rect, f.bg, 0.1, 1.25);
        }
    }
}

background!(SquaresGrid);

// ---------------------------------------------------------------------------------------------
// Dot grid
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct DotField {
    off: Vec<Vec2>,
    vel: Vec<Vec2>,
    dims: (usize, usize),
    last: Option<Pos2>,
}

/// A field of dots that bulges away from the pointer and springs back with a little wobble.
/// Dots near the pointer light up in the active color; quick swipes fling them.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::DotGrid;
/// ui.add(DotGrid::new().gap(28.0).dot_size(4.0).proximity(140.0));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct DotGrid {
    base: Base,
    gap: f32,
    dot: f32,
    color: Option<Color32>,
    active: Option<Color32>,
    proximity: f32,
    strength: f32,
    auto_hover: bool,
}

impl Default for DotGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl DotGrid {
    /// Dots every 28 points.
    pub fn new() -> Self {
        Self { base: Base::default(), gap: 28.0, dot: 3.5, color: None, active: None, proximity: 140.0, strength: 1.0, auto_hover: false }
    }
    /// Distance between dots in points (default 28).
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap.max(6.0);
        self
    }
    /// Dot diameter in points (default 3.5).
    pub fn dot_size(mut self, size: f32) -> Self {
        self.dot = size.max(0.5);
        self
    }
    /// Resting dot color (default: faint theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Color of dots near the pointer (default: theme accent).
    pub fn active_color(mut self, c: Color32) -> Self {
        self.active = Some(c);
        self
    }
    /// Radius of the pointer's influence in points (default 140).
    pub fn proximity(mut self, r: f32) -> Self {
        self.proximity = r.max(10.0);
        self
    }
    /// How hard dots are pushed (default 1).
    pub fn strength(mut self, s: f32) -> Self {
        self.strength = s;
        self
    }
    /// When nobody hovers, a phantom pointer wanders through the dots (default false).
    pub fn auto_hover(mut self, on: bool) -> Self {
        self.auto_hover = on;
        self
    }

    /// Paint the dots into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_dot_grid") else { return };
        let t = &f.theme;
        let gap = self.gap;
        let cols = (rect.width() / gap).floor() as usize + 1;
        let rows = (rect.height() / gap).floor() as usize + 1;
        let origin = rect.center() - vec2((cols - 1) as f32 * gap, (rows - 1) as f32 * gap) * 0.5;
        let hp = hover_point(ui, &f, self.auto_hover);
        let clicked = pointer(ui, rect).is_some() && ui.input(|i| i.pointer.primary_pressed());
        let (prox, strength, dt) = (self.proximity, self.strength, f.dt);
        let offs = glowkit_core::with_state(ui, f.id, DotField::default, |s| {
            if s.dims != (cols, rows) {
                s.off = vec![Vec2::ZERO; cols * rows];
                s.vel = vec![Vec2::ZERO; cols * rows];
                s.dims = (cols, rows);
            }
            let pv = match (hp, s.last) {
                (Some(p), Some(l)) if dt > 0.0 => (p - l) / dt,
                _ => Vec2::ZERO,
            };
            s.last = hp;
            // sub-step for a stable spring
            let steps = 2;
            let h = dt / steps as f32;
            for _ in 0..steps {
                for j in 0..rows {
                    for i in 0..cols {
                        let k = j * cols + i;
                        let home = origin + vec2(i as f32 * gap, j as f32 * gap);
                        let mut target = Vec2::ZERO;
                        if let Some(p) = hp {
                            let d = home - p;
                            let dist = d.length();
                            if dist < prox && dist > 0.01 {
                                let fall = 1.0 - dist / prox;
                                target = d / dist * fall * fall * gap * 0.75 * strength;
                                s.vel[k] += pv * fall * fall * 0.9 * h * strength;
                                if clicked {
                                    s.vel[k] += d / dist * fall * 900.0 * strength;
                                }
                            }
                        }
                        let acc = (target - s.off[k]) * 140.0 - s.vel[k] * 9.0;
                        s.vel[k] += acc * h;
                        s.off[k] += s.vel[k] * h;
                    }
                }
            }
            s.off.clone()
        });
        let base = self.color.unwrap_or_else(|| if t.dark { mix(f.bg, t.foreground, 0.2) } else { mix(f.bg, t.foreground, 0.22) });
        let active = self.active.unwrap_or(t.accent);
        let dark = f.dark();
        for j in 0..rows {
            for i in 0..cols {
                let home = origin + vec2(i as f32 * gap, j as f32 * gap);
                let pos = home + offs[j * cols + i];
                let k = hp.map_or(0.0, |p| (1.0 - home.distance(p) / prox).clamp(0.0, 1.0));
                let k = k * k * (3.0 - 2.0 * k);
                let c = base.lerp_to_gamma(active, k);
                let r = self.dot * 0.5 * (1.0 + 0.35 * k);
                if k > 0.3 {
                    f.painter.circle_filled(pos, r * 3.0, light(active, (k - 0.3) * 0.25, dark));
                }
                f.painter.circle_filled(pos, r, c);
            }
        }
    }
}

background!(DotGrid);

// ---------------------------------------------------------------------------------------------
// Grid pattern
// ---------------------------------------------------------------------------------------------

/// Thin grid lines with highlighted squares that softly fade in and out at random, optional
/// pointer highlight and skew, masked into a radial vignette.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::GridPattern;
/// ui.add(GridPattern::new().cell_size(40.0).squares(30).skew(-12.0));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct GridPattern {
    base: Base,
    cell: f32,
    squares: usize,
    max_opacity: f32,
    duration: f32,
    line: Option<Color32>,
    fill: Option<Color32>,
    skew: f32,
    interactive: bool,
    mask: bool,
    dashed: bool,
    auto_hover: bool,
}

impl Default for GridPattern {
    fn default() -> Self {
        Self::new()
    }
}

impl GridPattern {
    /// A 40 pt grid with 30 animated squares.
    pub fn new() -> Self {
        Self {
            base: Base::default(),
            cell: 40.0,
            squares: 30,
            max_opacity: 0.5,
            duration: 4.0,
            line: None,
            fill: None,
            skew: 0.0,
            interactive: true,
            mask: true,
            dashed: false,
            auto_hover: false,
        }
    }
    /// Cell size in points (default 40).
    pub fn cell_size(mut self, size: f32) -> Self {
        self.cell = size.max(6.0);
        self
    }
    /// Number of squares fading in and out at any time; 0 for a static grid (default 30).
    pub fn squares(mut self, n: usize) -> Self {
        self.squares = n.min(400);
        self
    }
    /// Peak opacity of a highlighted square (default 0.5).
    pub fn max_opacity(mut self, a: f32) -> Self {
        self.max_opacity = a.clamp(0.0, 1.0);
        self
    }
    /// Seconds for one fade in + out (default 4).
    pub fn duration(mut self, seconds: f32) -> Self {
        self.duration = seconds.max(0.2);
        self
    }
    /// Grid line color (default: from the theme border).
    pub fn line_color(mut self, c: Color32) -> Self {
        self.line = Some(c);
        self
    }
    /// Highlight fill color (default: theme muted foreground).
    pub fn fill_color(mut self, c: Color32) -> Self {
        self.fill = Some(c);
        self
    }
    /// Vertical skew in degrees, for a tilted look (default 0).
    pub fn skew(mut self, degrees: f32) -> Self {
        self.skew = degrees.clamp(-60.0, 60.0);
        self
    }
    /// Highlight the square under the pointer (default true).
    pub fn interactive(mut self, on: bool) -> Self {
        self.interactive = on;
        self
    }
    /// Fade the edges with a radial mask (default true).
    pub fn mask(mut self, on: bool) -> Self {
        self.mask = on;
        self
    }
    /// Dashed grid lines (default false).
    pub fn dashed(mut self, on: bool) -> Self {
        self.dashed = on;
        self
    }
    /// When nobody hovers, a phantom pointer wanders around lighting squares (default false).
    pub fn auto_hover(mut self, on: bool) -> Self {
        self.auto_hover = on;
        self
    }

    /// Paint the pattern into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_grid_pattern") else { return };
        let t = &f.theme;
        let cell = self.cell;
        let k = self.skew.to_radians().tan();
        let cx = rect.center().x;
        let to_screen = |x: f32, y: f32| pos2(x, y + k * (x - cx));
        let extra = (k * rect.width() * 0.5).abs();
        let x0 = rect.min.x + ((rect.width() % cell) * 0.5) - cell;
        let y0 = rect.min.y - extra - ((extra / cell).ceil() * cell - extra) + ((rect.height() % cell) * 0.5) - cell;
        let cols = ((rect.width()) / cell).ceil() as i32 + 2;
        let rows = ((rect.height() + 2.0 * extra) / cell).ceil() as i32 + 3;
        let fill = self.fill.unwrap_or_else(|| if t.dark { mix(t.muted_foreground, t.foreground, 0.2).gamma_multiply(0.4) } else { t.muted_foreground.gamma_multiply(0.35) });
        let quad = |i: i32, j: i32, c: Color32| {
            let (x, y) = (x0 + i as f32 * cell, y0 + j as f32 * cell);
            let pts = vec![to_screen(x, y), to_screen(x + cell, y), to_screen(x + cell, y + cell), to_screen(x, y + cell)];
            f.painter.add(Shape::convex_polygon(pts, c, Stroke::NONE));
        };
        // animated squares
        let time = f.time;
        for s in 0..self.squares as u32 {
            let phase = h01(s, 1) * self.duration;
            let cyc = ((time + phase) / self.duration).floor() as u32;
            let fr = ((time + phase) / self.duration).fract();
            let i = (h01(s.wrapping_mul(31).wrapping_add(cyc), 2) * cols as f32) as i32;
            let j = (h01(s.wrapping_mul(17).wrapping_add(cyc), 3) * rows as f32) as i32;
            let a = (fr * PI).sin().powi(2) * self.max_opacity;
            quad(i, j, fill.gamma_multiply(a));
        }
        if self.interactive {
            let hp = hover_point(ui, &f, self.auto_hover);
            let trail = glowkit_core::with_state(ui, f.id, Trail::default, |s| {
                s.update(
                    hp.map(|p| {
                        let ly = p.y - k * (p.x - cx);
                        (((p.x - x0) / cell).floor() as i32, ((ly - y0) / cell).floor() as i32)
                    }),
                    f.dt,
                    2.0,
                    1,
                );
                s.cells.clone()
            });
            for (i, j, a, _) in trail {
                quad(i, j, with_alpha(t.accent, 0.35 * a.min(1.0)));
            }
        }
        let lc = self.line.unwrap_or_else(|| line_color(&f));
        let stroke = Stroke::new(1.0, lc);
        let dash = |a: Pos2, b: Pos2| {
            if self.dashed {
                f.painter.extend(Shape::dashed_line(&[a, b], stroke, 4.0, 4.0));
            } else {
                f.painter.line_segment([a, b], stroke);
            }
        };
        for i in 0..=cols {
            let x = x0 + i as f32 * cell;
            dash(pos2(x, rect.min.y - 1.0), pos2(x, rect.max.y + 1.0));
        }
        for j in 0..=rows {
            let y = y0 + j as f32 * cell;
            dash(to_screen(rect.min.x - 1.0, y), to_screen(rect.max.x + 1.0, y));
        }
        if self.mask {
            radial_mask(&f.painter, rect, f.bg, 0.25, 1.05);
        }
    }
}

background!(GridPattern);

// ---------------------------------------------------------------------------------------------
// Retro grid
// ---------------------------------------------------------------------------------------------

/// A synthwave perspective grid: a floor of lines stretching to the horizon, scrolling toward
/// the viewer and fading into the distance.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::RetroGrid;
/// ui.add(RetroGrid::new().horizon(0.4).cell_size(60.0));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct RetroGrid {
    base: Base,
    color: Option<Color32>,
    horizon: f32,
    cell: f32,
    speed: f32,
    glow: bool,
}

impl Default for RetroGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl RetroGrid {
    /// A neutral retro grid with its horizon at 40% height.
    pub fn new() -> Self {
        Self { base: Base::default(), color: None, horizon: 0.42, cell: 60.0, speed: 1.0, glow: false }
    }
    /// Line color (default: muted theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Horizon height as a fraction of the rect (default 0.42).
    pub fn horizon(mut self, fraction: f32) -> Self {
        self.horizon = fraction.clamp(0.0, 0.9);
        self
    }
    /// Cell size at the front of the floor, in points (default 60).
    pub fn cell_size(mut self, size: f32) -> Self {
        self.cell = size.max(10.0);
        self
    }
    /// Scroll speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Add a glowing horizon and sun haze in the line color (default false).
    pub fn glow(mut self, on: bool) -> Self {
        self.glow = on;
        self
    }

    /// Paint the floor into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_retro_grid") else { return };
        let t = &f.theme;
        let color = self.color.unwrap_or_else(|| if t.dark { mix(t.muted_foreground, t.foreground, 0.1).gamma_multiply(0.55) } else { t.muted_foreground.gamma_multiply(0.45) });
        let hy = rect.min.y + rect.height() * self.horizon;
        let cx = rect.center().x;
        let focal = rect.width() * 0.6;
        let cam_h = (rect.max.y - hy) * 1.0; // so z=focal hits the bottom edge
        let cell_world = self.cell; // world cell size == screen size at the bottom edge (z = focal)
        let proj = |x: f32, z: f32| pos2(cx + x * focal / z, hy + cam_h * focal / z);
        let near = focal * 0.6;
        let far = focal * 40.0;
        let p = &f.painter;
        let dark = f.dark();
        if self.glow {
            glowkit_core::paint::glow_ellipse(p, pos2(cx, hy), vec2(rect.width() * 0.45, rect.height() * 0.35), light(color, 0.35, dark));
        }
        let depth_alpha = |z: f32| (1.0 - ((z - near) / (focal * 9.0)).clamp(0.0, 1.0)).powf(1.8);
        // horizontal lines (scroll toward the viewer)
        // depth spacing chosen so cells at the bottom edge look roughly square
        let dz = cell_world * focal / cam_h.max(1.0) * 0.8;
        let scroll = (f.time * self.speed * dz * 1.2).rem_euclid(dz);
        let mut z = near - scroll + dz;
        let mut n = 0;
        while z < far && n < 400 {
            let a = depth_alpha(z);
            if a > 0.01 {
                let y = proj(0.0, z).y;
                let w = (1.3 * focal / z).clamp(0.5, 1.6);
                p.hline(rect.x_range(), y, Stroke::new(w, color.gamma_multiply(a)));
            }
            z += dz;
            n += 1;
        }
        // depth lines converging at the vanishing point
        let count = ((rect.width() * 0.5 * near / focal) / cell_world).ceil() as i32 + 2;
        for i in -count..=count {
            let x = i as f32 * cell_world;
            let a = proj(x, near);
            let b = proj(x, focal * 12.0);
            glowkit_core::paint::gradient_polyline(p, &[a, a.lerp(b, 0.5), b], false, 1.0, |u| color.gamma_multiply((1.0 - u).powf(2.0)));
        }
        if self.glow {
            glowkit_core::paint::gradient_polyline(p, &[pos2(rect.min.x, hy), pos2(rect.max.x, hy)], false, 1.5, |u| light(color, (1.0 - (u - 0.5).abs() * 2.0).powf(0.8), dark));
        }
        // fade the sky and the horizon into the background
        let clear = with_alpha(f.bg, 0.0);
        let y_fade = hy + rect.height() * if self.glow { 0.06 } else { 0.14 };
        if !self.glow {
            p.rect_filled(Rect::from_min_max(rect.min, pos2(rect.max.x, hy)), 0.0, f.bg);
        }
        if !self.glow {
            glowkit_core::paint::quad(p, [pos2(rect.min.x, hy), pos2(rect.max.x, hy), pos2(rect.max.x, y_fade), pos2(rect.min.x, y_fade)], [f.bg, f.bg, clear, clear]);
        }
    }
}

background!(RetroGrid);

// ---------------------------------------------------------------------------------------------
// Flickering grid
// ---------------------------------------------------------------------------------------------

/// A grid of tiny squares, each flickering to a random opacity now and then, like a quiet
/// LED panel.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::FlickeringGrid;
/// ui.add(FlickeringGrid::new().square_size(4.0).gap(6.0).flicker_chance(0.3).max_opacity(0.3));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct FlickeringGrid {
    base: Base,
    square: f32,
    gap: f32,
    chance: f32,
    color: Option<Color32>,
    max_opacity: f32,
    mask: bool,
}

impl Default for FlickeringGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl FlickeringGrid {
    /// 4 pt squares every 6 points.
    pub fn new() -> Self {
        Self { base: Base::default(), square: 4.0, gap: 6.0, chance: 0.3, color: None, max_opacity: 0.3, mask: false }
    }
    /// Square size in points (default 4).
    pub fn square_size(mut self, size: f32) -> Self {
        self.square = size.max(1.0);
        self
    }
    /// Gap between squares in points (default 6).
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap.max(0.0);
        self
    }
    /// Average flickers per square per second (default 0.3).
    pub fn flicker_chance(mut self, chance: f32) -> Self {
        self.chance = chance.max(0.0);
        self
    }
    /// Square color (default: theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Highest opacity a square reaches (default 0.3).
    pub fn max_opacity(mut self, a: f32) -> Self {
        self.max_opacity = a.clamp(0.0, 1.0);
        self
    }
    /// Fade the edges with a radial mask (default false).
    pub fn mask(mut self, on: bool) -> Self {
        self.mask = on;
        self
    }

    /// Paint the grid into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_flickering_grid") else { return };
        let color = self.color.unwrap_or(f.theme.foreground);
        let step = self.square + self.gap;
        let cols = (rect.width() / step).ceil() as u32;
        let rows = (rect.height() / step).ceil() as u32;
        let mut m = Mesh::default();
        m.reserve_vertices((cols * rows * 4) as usize);
        let time = f.time;
        let (cxr, cyr) = (rect.width() * 0.5, rect.height() * 0.5);
        for j in 0..rows {
            for i in 0..cols {
                let id = j * 4099 + i;
                let rate = self.chance * (0.5 + h01(id, 1));
                let x = time * rate + h01(id, 2) * 10.0;
                let k = x.floor() as u32;
                let fr = x.fract();
                let a0 = h01(id, k);
                let a1 = h01(id, k + 1);
                let blend = smoothstep(0.0, (0.25 * rate).min(0.5), fr);
                let mut a = (a0 + (a1 - a0) * blend) * self.max_opacity;
                let r = Rect::from_min_size(rect.min + vec2(i as f32 * step, j as f32 * step), Vec2::splat(self.square));
                if self.mask {
                    let d = vec2((r.center().x - rect.min.x - cxr) / cxr, (r.center().y - rect.min.y - cyr) / cyr).length();
                    a *= 1.0 - smoothstep(0.3, 1.05, d);
                }
                if a > 0.004 {
                    m.add_colored_rect(r, color.gamma_multiply(a));
                }
            }
        }
        f.painter.add(Shape::mesh(m));
    }
}

background!(FlickeringGrid);

// ---------------------------------------------------------------------------------------------
// Background boxes
// ---------------------------------------------------------------------------------------------

/// A big skewed, isometric-looking grid of boxes. Boxes light up in random colors as the
/// pointer passes over them and slowly fade back, leaving a colorful trail.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::BackgroundBoxes;
/// ui.add(BackgroundBoxes::new());
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct BackgroundBoxes {
    base: Base,
    colors: Option<Vec<Color32>>,
    line: Option<Color32>,
    box_size: Vec2,
    plus: bool,
    auto_hover: bool,
}

impl Default for BackgroundBoxes {
    fn default() -> Self {
        Self::new()
    }
}

impl BackgroundBoxes {
    /// Boxes of 64 x 32 points.
    pub fn new() -> Self {
        Self { base: Base::default(), colors: None, line: None, box_size: vec2(64.0, 32.0), plus: true, auto_hover: false }
    }
    /// Colors boxes light up with, picked at random (default: theme chart + gradient).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }
    /// Grid line color (default: from the theme border).
    pub fn line_color(mut self, c: Color32) -> Self {
        self.line = Some(c);
        self
    }
    /// Unskewed box size in points (default 64 x 32).
    pub fn box_size(mut self, size: Vec2) -> Self {
        self.box_size = size.max(Vec2::splat(6.0));
        self
    }
    /// Draw small "+" marks at grid intersections (default true).
    pub fn plus_marks(mut self, on: bool) -> Self {
        self.plus = on;
        self
    }
    /// When nobody hovers, a phantom pointer wanders around lighting boxes (default false).
    pub fn auto_hover(mut self, on: bool) -> Self {
        self.auto_hover = on;
        self
    }

    /// Paint the boxes into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_background_boxes") else { return };
        let t = &f.theme;
        let colors = self.colors.clone().unwrap_or_else(|| {
            let mut v = t.chart.to_vec();
            v.extend_from_slice(&t.gradient);
            if t.dark {
                v.iter().map(|c| glowkit_core::color::shade(*c, 0.08)).collect()
            } else {
                v
            }
        });
        // skewX(-48deg) * skewY(14deg) * scale(0.675)
        let (a, b) = ((-48f32).to_radians().tan(), 14f32.to_radians().tan());
        let s = 0.675;
        let m = [[(1.0 + a * b) * s, a * s], [b * s, s]];
        let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];
        let inv = [[m[1][1] / det, -m[0][1] / det], [-m[1][0] / det, m[0][0] / det]];
        let c = rect.center();
        let fwd = |g: Vec2| c + vec2(m[0][0] * g.x + m[0][1] * g.y, m[1][0] * g.x + m[1][1] * g.y);
        let back = |p: Pos2| {
            let d = p - c;
            vec2(inv[0][0] * d.x + inv[0][1] * d.y, inv[1][0] * d.x + inv[1][1] * d.y)
        };
        let bs = self.box_size;
        // grid-space bounds of the rect
        let corners = [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()].map(back);
        let (mut lo, mut hi) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
        for q in corners {
            lo = lo.min(q);
            hi = hi.max(q);
        }
        let (i0, i1) = ((lo.x / bs.x).floor() as i32 - 1, (hi.x / bs.x).ceil() as i32 + 1);
        let (j0, j1) = ((lo.y / bs.y).floor() as i32 - 1, (hi.y / bs.y).ceil() as i32 + 1);
        let hp = hover_point(ui, &f, self.auto_hover);
        let trail = glowkit_core::with_state(ui, f.id, Trail::default, |st| {
            st.update(hp.map(|p| {
                let g = back(p);
                ((g.x / bs.x).floor() as i32, (g.y / bs.y).floor() as i32)
            }), f.dt, 0.55, colors.len());
            st.cells.clone()
        });
        let p = &f.painter;
        for (i, j, al, ci) in trail {
            let (x, y) = (i as f32 * bs.x, j as f32 * bs.y);
            let pts = vec![fwd(vec2(x, y)), fwd(vec2(x + bs.x, y)), fwd(vec2(x + bs.x, y + bs.y)), fwd(vec2(x, y + bs.y))];
            let col = colors[ci as usize % colors.len()];
            let a = al.clamp(0.0, 1.0);
            p.add(Shape::convex_polygon(pts, col.gamma_multiply(a.powf(0.7) * if t.dark { 0.9 } else { 0.75 }), Stroke::NONE));
        }
        let lc = self.line.unwrap_or_else(|| line_color(&f));
        let stroke = Stroke::new(1.0, lc);
        for i in i0..=i1 {
            let x = i as f32 * bs.x;
            p.line_segment([fwd(vec2(x, j0 as f32 * bs.y)), fwd(vec2(x, j1 as f32 * bs.y))], stroke);
        }
        for j in j0..=j1 {
            let y = j as f32 * bs.y;
            p.line_segment([fwd(vec2(i0 as f32 * bs.x, y)), fwd(vec2(i1 as f32 * bs.x, y))], stroke);
        }
        if self.plus {
            let pc = Stroke::new(1.2, if t.dark { mix(lc, t.foreground, 0.25) } else { mix(lc, t.foreground, 0.3) });
            for i in i0..=i1 {
                for j in j0..=j1 {
                    if i.rem_euclid(2) == 0 && j.rem_euclid(2) == 0 {
                        let q = fwd(vec2(i as f32 * bs.x, j as f32 * bs.y));
                        if rect.expand(8.0).contains(q) {
                            p.line_segment([q - vec2(5.0, 0.0), q + vec2(5.0, 0.0)], pc);
                            p.line_segment([q - vec2(0.0, 5.0), q + vec2(0.0, 5.0)], pc);
                        }
                    }
                }
            }
        }
        radial_mask(p, rect, f.bg, 0.35, 1.15);
    }
}

background!(BackgroundBoxes);

// ---------------------------------------------------------------------------------------------
// Hex grid
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct Glow {
    pos: Option<Pos2>,
    strength: f32,
}

/// A honeycomb of hexagon outlines. Random cells pulse with light, a soft wave of brightness
/// rolls across, and cells near the pointer glow.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::HexGrid;
/// ui.add(HexGrid::new().radius(24.0));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct HexGrid {
    base: Base,
    radius: f32,
    line: Option<Color32>,
    color: Option<Color32>,
    pulses: bool,
    auto_hover: bool,
    mask: bool,
}

impl Default for HexGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl HexGrid {
    /// Hexagons with a 24 pt radius.
    pub fn new() -> Self {
        Self { base: Base::default(), radius: 24.0, line: None, color: None, pulses: true, auto_hover: false, mask: true }
    }
    /// Hexagon radius (center to corner) in points (default 24).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.max(4.0);
        self
    }
    /// Outline color (default: from the theme border).
    pub fn line_color(mut self, c: Color32) -> Self {
        self.line = Some(c);
        self
    }
    /// Glow color of lit cells (default: theme accent).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Random cells pulse with light (default true).
    pub fn pulses(mut self, on: bool) -> Self {
        self.pulses = on;
        self
    }
    /// When nobody hovers, a phantom pointer wanders around (default false).
    pub fn auto_hover(mut self, on: bool) -> Self {
        self.auto_hover = on;
        self
    }
    /// Fade the edges with a radial mask (default true).
    pub fn mask(mut self, on: bool) -> Self {
        self.mask = on;
        self
    }

    /// Paint the honeycomb into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_hex_grid") else { return };
        let t = &f.theme;
        let r = self.radius;
        let w = 3f32.sqrt() * r;
        let vh = 1.5 * r;
        let hp = hover_point(ui, &f, self.auto_hover);
        let g = glowkit_core::with_state(ui, f.id, Glow::default, |s| {
            let target = if hp.is_some() { 1.0 } else { 0.0 };
            s.strength += (target - s.strength) * (1.0 - (-f.dt * 5.0).exp());
            if let Some(p) = hp {
                s.pos = Some(s.pos.map_or(p, |q| q + (p - q) * (1.0 - (-f.dt * 12.0).exp())));
            }
            *s
        });
        let lc = self.line.unwrap_or_else(|| line_color(&f));
        let gc = self.color.unwrap_or(t.accent);
        let dark = f.dark();
        let cols = (rect.width() / w).ceil() as i32 + 2;
        let rows = (rect.height() / vh).ceil() as i32 + 2;
        let time = f.time;
        let p = &f.painter;
        let corners: Vec<Vec2> = (0..6).map(|k| Vec2::angled(TAU * (k as f32 + 0.5) / 6.0) * (r - 1.5)).collect();
        for j in -1..rows {
            for i in -1..cols {
                let cx = rect.min.x + i as f32 * w + if j.rem_euclid(2) == 1 { w * 0.5 } else { 0.0 };
                let cy = rect.min.y + j as f32 * vh;
                let c = pos2(cx, cy);
                let id = (j + 100) as u32 * 1031 + (i + 100) as u32;
                let mut k = 0.0;
                if self.pulses {
                    let ph = h01(id, 1) * TAU;
                    let rate = 0.25 + 0.35 * h01(id, 2);
                    k += (0.5 + 0.5 * (time * rate + ph).sin()).powi(24);
                    // a slow diagonal wave
                    let wave = ((cx - rect.min.x) * 0.006 + (cy - rect.min.y) * 0.004 - time * 0.6).sin();
                    k += smoothstep(0.85, 1.0, wave) * 0.25;
                }
                if let Some(m) = g.pos {
                    let d = c.distance(m);
                    k += g.strength * (-(d * d) / (r * r * 9.0)).exp();
                }
                let k = k.min(1.0);
                let pts: Vec<Pos2> = corners.iter().map(|v| c + *v).collect();
                if k > 0.02 {
                    p.add(Shape::convex_polygon(pts.clone(), light(gc, k * 0.35, dark), Stroke::NONE));
                }
                let stroke_c = lc.lerp_to_gamma(gc, k * 0.9);
                p.add(PathShape::closed_line(pts, PathStroke::new(1.0, stroke_c)));
            }
        }
        if self.mask {
            radial_mask(p, rect, f.bg, 0.3, 1.1);
        }
    }
}

background!(HexGrid);
