use super::common::{pointer_or_auto, soft_ribbon};
use crate::buttons::util::{self, cr, outline};
use egui::{
    pos2, vec2, Align, Color32, CursorIcon, Layout, Pos2, Rect, Response, Sense, Stroke, Ui, UiBuilder, Vec2, Widget,
};
use glowkit_core::anim::{follow, smoothstep, spring2};
use glowkit_core::color::{gradient, mix, with_alpha};
use glowkit_core::rng::hash01;
use glowkit_core::{paint, Theme};

/// Smoothed pointer position inside `rect` (real or autoplay) plus an "active" 0..1 fade.
fn smooth_pointer(ui: &Ui, id: egui::Id, rect: Rect, autoplay: bool, seed: f32, stiffness: f32) -> (Pos2, f32, bool) {
    let ptr = pointer_or_auto(ui, rect, autoplay, seed);
    let last: Vec2 = glowkit_core::state(ui, id.with("last"), || {
        rect.center() - rect.min + vec2(rect.width() * 0.25, -rect.height() * 0.2)
    });
    let target = ptr.map(|p| p.0 - rect.min).unwrap_or(last);
    glowkit_core::set_state(ui, id.with("last"), target);
    let p = rect.min + spring2(ui, id.with("ptr"), target, stiffness, stiffness.sqrt() * 2.0);
    let k = follow(ui, id.with("k"), if ptr.is_some() { 1.0 } else { 0.0 }, 5.0);
    (p, k, ptr.is_some_and(|p| p.1))
}

// ---------------------------------------------------------------------------------------------

/// A grid of short rounded lines that all swivel to point at the pointer, like iron filings
/// around a magnet. Lines near the pointer can light up in the accent color.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::MagnetLines::new(9, 9).size(egui::vec2(320.0, 320.0)));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct MagnetLines {
    rows: usize,
    cols: usize,
    size: Option<Vec2>,
    line_length: f32,
    line_width: f32,
    color: Option<Color32>,
    near_color: Option<Color32>,
    base_angle: f32,
    autoplay: bool,
}

impl MagnetLines {
    /// A `rows` x `cols` grid of lines.
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows: rows.max(1),
            cols: cols.max(1),
            size: None,
            line_length: 0.62,
            line_width: 4.0,
            color: None,
            near_color: None,
            base_angle: -10.0,
            autoplay: false,
        }
    }
    /// Size of the grid (default: 320 x 320).
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }
    /// Line length as a fraction of the cell size.
    pub fn line_length(mut self, k: f32) -> Self {
        self.line_length = k.clamp(0.05, 1.5);
        self
    }
    /// Line thickness in points.
    pub fn line_width(mut self, w: f32) -> Self {
        self.line_width = w.max(0.5);
        self
    }
    /// Line color. Default: muted foreground.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Color lines take on near the pointer (default: theme accent). Use the line color to disable.
    pub fn near_color(mut self, c: Color32) -> Self {
        self.near_color = Some(c);
        self
    }
    /// Resting angle (degrees) before the pointer arrives.
    pub fn base_angle(mut self, deg: f32) -> Self {
        self.base_angle = deg;
        self
    }
    /// Sweep an invisible magnet around while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for MagnetLines {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = self.size.unwrap_or(vec2(320.0, 320.0));
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let (p, k, _) = smooth_pointer(ui, resp.id, rect, self.autoplay, 1.1, 120.0);
        let painter = ui.painter_at(rect.expand(self.line_width));
        let cell = vec2(rect.width() / self.cols as f32, rect.height() / self.rows as f32);
        let len = cell.min_elem() * self.line_length;
        let base = self.color.unwrap_or(mix(t.muted_foreground, t.foreground, 0.2));
        let near = self.near_color.unwrap_or(t.accent);
        let rest = self.base_angle.to_radians();
        let reach = rect.size().length() * 0.45;
        for r in 0..self.rows {
            for c in 0..self.cols {
                let center = pos2(rect.min.x + (c as f32 + 0.5) * cell.x, rect.min.y + (r as f32 + 0.5) * cell.y);
                let d = p - center;
                let aim = d.y.atan2(d.x);
                // blend from the resting angle toward the pointer as it becomes active
                let delta =
                    (aim - rest + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                let ang = rest + delta * k;
                let dir = vec2(ang.cos(), ang.sin()) * len / 2.0;
                let prox = (1.0 - d.length() / reach).clamp(0.0, 1.0) * k;
                let col = mix(base, near, prox * prox);
                let w = self.line_width * (1.0 + 0.25 * prox);
                painter.line_segment([center - dir, center + dir], Stroke::new(w, col));
                painter.circle_filled(center - dir, w / 2.0, col);
                painter.circle_filled(center + dir, w / 2.0, col);
            }
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct PixelState {
    /// Which content is currently shown (false = first).
    shown: bool,
    /// Which content is wanted.
    target: bool,
    /// Start of the running transition, if any.
    start: Option<f64>,
    /// What the running transition switches to.
    to: bool,
    last_user: f64,
}

/// A card that swaps between two contents through a pixel dissolve: a grid of solid squares
/// pops in in random order until the card is covered, the content switches underneath, and the
/// squares pop away again. Hover (or click) shows the second content.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_effects::PixelTransition::new().show(
///     ui,
///     |ui| { ui.heading("Front"); },
///     |ui| { ui.heading("Back"); },
/// );
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct PixelTransition {
    size: Vec2,
    grid: usize,
    color: Option<Color32>,
    duration: f32,
    radius: Option<f32>,
    background: Option<Color32>,
    autoplay: bool,
}

impl Default for PixelTransition {
    fn default() -> Self {
        Self::new()
    }
}

impl PixelTransition {
    /// A pixel-transition card.
    pub fn new() -> Self {
        Self {
            size: vec2(260.0, 260.0),
            grid: 9,
            color: None,
            duration: 0.6,
            radius: None,
            background: None,
            autoplay: false,
        }
    }
    /// Card size.
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size.max(vec2(40.0, 40.0));
        self
    }
    /// Number of pixels along the longer side.
    pub fn grid(mut self, n: usize) -> Self {
        self.grid = n.clamp(2, 64);
        self
    }
    /// Pixel color. Default: theme foreground.
    pub fn pixel_color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Total transition time in seconds (cover + reveal).
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s.max(0.05);
        self
    }
    /// Corner radius.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = Some(r.max(0.0));
        self
    }
    /// Card fill. Default: the theme card color.
    pub fn background(mut self, c: Color32) -> Self {
        self.background = Some(c);
        self
    }
    /// Flip back and forth on its own while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the card with `first` normally and `second` while hovered.
    pub fn show(self, ui: &mut Ui, first: impl FnOnce(&mut Ui), second: impl FnOnce(&mut Ui)) -> Response {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(self.size, Sense::click());
        let id = resp.id;
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
        let now = ui.input(|i| i.time);
        let hovered = resp.hovered();
        let clicked = resp.clicked();
        let auto = self.autoplay;
        let dur = self.duration as f64;
        let (shown, progress) = glowkit_core::with_state(
            ui,
            id.with("pixel"),
            || PixelState { shown: false, target: false, start: None, to: false, last_user: -100.0 },
            |s| {
                if hovered || clicked {
                    s.last_user = now;
                }
                if clicked {
                    s.target = !s.shown;
                } else if hovered {
                    s.target = true;
                } else if auto && now - s.last_user > 2.0 {
                    s.target = (now % 4.4) > 2.2;
                } else if now - s.last_user > 0.05 {
                    s.target = false;
                }
                let mut progress = None;
                if let Some(st) = s.start {
                    let e = (now - st) / dur;
                    if e >= 0.5 {
                        s.shown = s.to;
                    }
                    if e >= 1.0 {
                        s.start = None;
                    } else {
                        progress = Some(e as f32);
                    }
                }
                if s.start.is_none() && s.target != s.shown {
                    s.start = Some(now);
                    s.to = s.target;
                    progress = Some(0.0);
                }
                (s.shown, progress)
            },
        );
        if progress.is_some() || auto {
            glowkit_core::animate(ui);
        }
        let radius = self.radius.unwrap_or(t.radius * 1.4);
        let painter = ui.painter().clone();
        let painter = &painter;
        paint::soft_shadow(
            painter,
            rect,
            radius,
            20.0,
            vec2(0.0, 6.0),
            Color32::BLACK.gamma_multiply(if t.dark { 0.4 } else { 0.08 }),
        );
        painter.rect_filled(rect, cr(radius), self.background.unwrap_or(t.card));
        let mut child = ui.new_child(
            UiBuilder::new().id_salt(id.with("content")).max_rect(rect).layout(Layout::top_down(Align::Min)),
        );
        child.set_clip_rect(rect.intersect(ui.clip_rect()));
        if shown {
            second(&mut child);
        } else {
            first(&mut child);
        }
        painter.rect_stroke(rect, cr(radius), Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        if let Some(e) = progress {
            let n = self.grid;
            let (cols, rows) = if rect.width() >= rect.height() {
                (n, ((n as f32 * rect.height() / rect.width()).round() as usize).max(1))
            } else {
                (((n as f32 * rect.width() / rect.height()).round() as usize).max(1), n)
            };
            let cw = rect.width() / cols as f32;
            let ch = rect.height() / rows as f32;
            let color = self.color.unwrap_or(t.foreground);
            let clip = painter.with_clip_rect(rect);
            let (phase, frac) = if e < 0.5 { (0u64, e * 2.0) } else { (1, (e - 0.5) * 2.0) };
            let shape = outline(rect, radius, 6.0);
            for y in 0..rows {
                for x in 0..cols {
                    let i = (y * cols + x) as u64;
                    let r = hash01(i * 7919 + phase * 104_729 + 17);
                    let on = if phase == 0 { r < frac } else { r >= frac };
                    if !on {
                        continue;
                    }
                    let cell =
                        Rect::from_min_size(rect.min + vec2(x as f32 * cw, y as f32 * ch), vec2(cw, ch)).expand(0.4);
                    let poly = util::clip_convex(&outline(cell, 0.0, 1000.0), &shape);
                    if poly.len() >= 3 {
                        util::fill_poly(&clip, &poly, color);
                    }
                }
            }
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------

/// The outline drawn by [`ShapeBlur`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ShapeBlurKind {
    /// A rounded rectangle.
    #[default]
    RoundedRect,
    /// A circle (or ellipse for non-square sizes).
    Circle,
    /// A diamond.
    Diamond,
    /// A five-pointed star.
    Star,
}

/// A soft, out-of-focus outline of a shape that snaps into crisp focus around the pointer, as if
/// a lens followed it. Several nested copies can be drawn for a ripple-like look.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{ShapeBlur, ShapeBlurKind};
/// ui.add(ShapeBlur::new(ShapeBlurKind::Circle).size(egui::vec2(240.0, 240.0)));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ShapeBlur {
    kind: ShapeBlurKind,
    size: Vec2,
    blur: f32,
    focus_radius: f32,
    stroke: f32,
    rings: usize,
    colors: Option<[Color32; 2]>,
    autoplay: bool,
}

impl ShapeBlur {
    /// A blurred outline of `kind`.
    pub fn new(kind: ShapeBlurKind) -> Self {
        Self {
            kind,
            size: vec2(240.0, 240.0),
            blur: 16.0,
            focus_radius: 110.0,
            stroke: 2.0,
            rings: 1,
            colors: None,
            autoplay: false,
        }
    }
    /// Widget size (the shape is inset a little to leave room for the blur).
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size.max(vec2(40.0, 40.0));
        self
    }
    /// Maximum blur in points, far from the pointer.
    pub fn blur(mut self, px: f32) -> Self {
        self.blur = px.max(0.0);
        self
    }
    /// Radius around the pointer within which the outline comes into focus.
    pub fn focus_radius(mut self, r: f32) -> Self {
        self.focus_radius = r.max(1.0);
        self
    }
    /// Sharp line width in points.
    pub fn stroke(mut self, w: f32) -> Self {
        self.stroke = w.max(0.5);
        self
    }
    /// Number of nested outlines (1..=6).
    pub fn rings(mut self, n: usize) -> Self {
        self.rings = n.clamp(1, 6);
        self
    }
    /// Gradient along the outline. Default: theme foreground.
    pub fn colors(mut self, a: Color32, b: Color32) -> Self {
        self.colors = Some([a, b]);
        self
    }
    /// Move the focus around on its own while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    fn points(&self, rect: Rect) -> Vec<Pos2> {
        let pts = match self.kind {
            ShapeBlurKind::RoundedRect => outline(rect, rect.size().min_elem() * 0.18, 4.0),
            ShapeBlurKind::Circle => (0..160)
                .map(|i| {
                    let a = i as f32 / 160.0 * std::f32::consts::TAU;
                    rect.center() + vec2(a.cos() * rect.width() / 2.0, a.sin() * rect.height() / 2.0)
                })
                .collect(),
            ShapeBlurKind::Diamond => {
                let c = rect.center();
                let h = rect.size() / 2.0;
                vec![c - vec2(0.0, h.y), c + vec2(h.x, 0.0), c + vec2(0.0, h.y), c - vec2(h.x, 0.0)]
            }
            ShapeBlurKind::Star => {
                let c = rect.center() + vec2(0.0, rect.height() * 0.05);
                let h = rect.size() / 2.0;
                (0..10)
                    .map(|i| {
                        let a = -std::f32::consts::FRAC_PI_2 + i as f32 / 10.0 * std::f32::consts::TAU;
                        let r = if i % 2 == 0 { 1.0 } else { 0.45 };
                        c + vec2(a.cos() * h.x, a.sin() * h.y) * r
                    })
                    .collect()
            }
        };
        let pts = match self.kind {
            ShapeBlurKind::Diamond | ShapeBlurKind::Star => chaikin(pts, 4),
            _ => pts,
        };
        paint::subdivide_closed(&pts, 3.0)
    }
}

/// Round off the corners of a closed polygon (Chaikin corner cutting).
fn chaikin(mut pts: Vec<Pos2>, iterations: usize) -> Vec<Pos2> {
    for _ in 0..iterations {
        let n = pts.len();
        let mut out = Vec::with_capacity(n * 2);
        for i in 0..n {
            let a = pts[i];
            let b = pts[(i + 1) % n];
            out.push(a.lerp(b, 0.25));
            out.push(a.lerp(b, 0.75));
        }
        pts = out;
    }
    pts
}

impl Widget for ShapeBlur {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(self.size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let (p, k, _) = smooth_pointer(ui, resp.id, rect, self.autoplay, 3.3, 140.0);
        let painter = ui.painter();
        let colors = self.colors.unwrap_or([t.foreground, t.foreground]);
        let inset = self.blur * 0.8 + 6.0;
        let base = rect.shrink(inset);
        for ring in 0..self.rings {
            let shrink = ring as f32 * (base.size().min_elem() * 0.12).min(20.0);
            let r = base.shrink(shrink);
            if r.width() < 8.0 || r.height() < 8.0 {
                break;
            }
            let pts = self.points(r);
            let n = pts.len() as f32;
            let fade = 1.0 - ring as f32 * 0.12;
            let col_at = |i: usize| gradient(&colors, (i as f32 / n * 2.0 - 1.0).abs());
            let focus = |q: Pos2| (1.0 - smoothstep(self.focus_radius * 0.15, self.focus_radius, q.distance(p))) * k;
            let mut i = 0usize;
            // wide halo (the blur's soft tail)
            soft_ribbon(painter, &pts, true, |q| {
                let f = focus(q);
                let b = self.blur * (1.0 - f);
                i += 1;
                let c = col_at(i);
                (0.0, b * 2.2, with_alpha(c, 0.22 * fade * (b / self.blur.max(1.0))))
            });
            let mut j = 0usize;
            soft_ribbon(painter, &pts, true, |q| {
                let f = focus(q);
                let b = self.blur * (1.0 - f);
                j += 1;
                let c = col_at(j);
                let a = (self.stroke + 1.0) / (self.stroke + 1.0 + b * 0.8);
                (self.stroke * (0.8 + 0.2 * f), b, with_alpha(c, a.clamp(0.0, 1.0) * fade))
            });
        }
        resp
    }
}
