//! Pointer- and scroll-driven surfaces: a warp grid, a marbling scroll story, a mosaic trail
//! and a step path that draws itself in.

use super::common::{
    cover_uv, dashed, display_font, ghost, hash3, idle_time, lerp_c, mesh_rect, round_corners, semibold_font,
    view_progress, wall_dt,
};
use egui::epaint::{Mesh, Vertex};
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, TextureId, Ui, Vec2, Widget};
use glowkit_core::color::{hex, with_alpha};
use glowkit_core::noise::{fbm3, noise3};
use glowkit_core::{paint, Theme};
use glowkit_effects::PaintedScene;

// ---------------------------------------------------------------------------------------------

/// A fine grid that bends into a gravity well under the pointer: lines are pushed out around a
/// dark sphere, crowding into rings at its rim, and glow brightest where they are pulled
/// hardest. The well follows the pointer with a soft lag and wanders on its own when idle.
///
/// Use it as a widget or paint it behind other content with [`WarpGrid::paint`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::WarpGrid::new().cell(28.0).strength(1.0));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
pub struct WarpGrid {
    size: Option<Vec2>,
    cell: f32,
    strength: f32,
    well: f32,
    color: Option<Color32>,
    autoplay: bool,
}

impl Default for WarpGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl WarpGrid {
    /// A warp grid in the theme accent.
    pub fn new() -> Self {
        Self { size: None, cell: 28.0, strength: 1.0, well: 80.0, color: None, autoplay: true }
    }
    /// Widget size (default: available width x 360).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Grid spacing in points.
    pub fn cell(mut self, c: f32) -> Self {
        self.cell = c.clamp(8.0, 80.0);
        self
    }
    /// Pull strength, `0..=2`.
    pub fn strength(mut self, s: f32) -> Self {
        self.strength = s.clamp(0.0, 2.0);
        self
    }
    /// Radius of the sphere at the bottom of the well, in points.
    pub fn well(mut self, r: f32) -> Self {
        self.well = r.clamp(10.0, 240.0);
        self
    }
    /// Line color (defaults to the theme accent).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Let the well wander while the pointer is away (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Paint the grid into `rect` (reads the pointer, never blocks clicks).
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        if !ui.is_rect_visible(rect) {
            return;
        }
        let t = Theme::of(ui);
        let id = ui.scope_id().with(("snap_b_warp", rect.min.x as i32, rect.min.y as i32));
        let time = glowkit_core::time(ui);
        let hover = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p));
        let _ = idle_time(ui, id, hover.is_some());
        let target = match hover {
            Some(p) => p,
            None if self.autoplay => ghost(rect.shrink2(rect.size() * 0.18), time * 0.55),
            None => rect.center(),
        };
        let c = pos2(
            glowkit_core::anim::follow(ui, id.with("x"), target.x, 4.0),
            glowkit_core::anim::follow(ui, id.with("y"), target.y, 4.0),
        );
        let s = glowkit_core::anim::spring(ui, id.with("s"), self.strength, 120.0, 14.0);
        glowkit_core::animate(ui);
        let painter = ui.painter_at(rect);
        let accent = self.color.unwrap_or(t.accent);
        let b = self.well * s;
        // the sphere
        if b > 2.0 {
            let (core, rim) = if t.dark {
                (lerp_c(t.background, accent, 0.10), lerp_c(t.background, accent, 0.35))
            } else {
                (lerp_c(t.background, accent, 0.18), lerp_c(t.background, accent, 0.45))
            };
            paint::glow(&painter, c, b * 2.2, with_alpha(accent, if t.dark { 0.18 } else { 0.12 }));
            let mut disc = Mesh::default();
            disc.colored_vertex(c + vec2(-b * 0.2, -b * 0.25), core);
            for k in 0..=72 {
                let a = k as f32 / 72.0 * std::f32::consts::TAU;
                disc.colored_vertex(c + vec2(a.cos(), a.sin()) * b * 0.98, rim);
            }
            for k in 0..72u32 {
                disc.add_triangle(0, 1 + k, 2 + k);
            }
            painter.add(Shape::mesh(disc));
            paint::glow(&painter, c + vec2(-b * 0.15, -b * 0.25), b * 0.5, with_alpha(Color32::WHITE, if t.dark { 0.10 } else { 0.35 }));
        }
        let warp = |p: Pos2| -> (Pos2, f32) {
            let d = p - c;
            let r = d.length();
            if r < 1e-3 || b < 1.0 {
                return (p, 0.0);
            }
            let f = (r * r + b * b).sqrt();
            // fade the push out far from the well so the edges stay square
            let k = (-(r / (b * 4.0 + 1.0)).powi(2)).exp();
            let nr = r + (f - r) * (0.35 + 0.65 * k);
            let pull = ((nr - r) / b).clamp(0.0, 1.0);
            (c + d / r * nr, pull)
        };
        let base_a = if t.dark { 0.22 } else { 0.28 };
        let step = 6.0;
        let cell = self.cell;
        let mut mesh = Mesh::default();
        let mut line = |pts: Vec<(Pos2, f32)>| {
            for w in pts.windows(2) {
                let ((a, ka), (bb, kb)) = (w[0], w[1]);
                if a.distance(bb) > step * 4.0 {
                    continue; // a line crossing the well splits around the sphere
                }
                let k = (ka + kb) * 0.5;
                let col = with_alpha(lerp_c(accent, if t.dark { Color32::WHITE } else { accent }, k * 0.4), (base_a + k * 0.75).min(1.0));
                let width = 1.0 + k * 0.6;
                let dir = (bb - a).normalized().rot90() * (width * 0.5);
                let i = mesh.vertices.len() as u32;
                mesh.colored_vertex(a + dir, col);
                mesh.colored_vertex(bb + dir, col);
                mesh.colored_vertex(bb - dir, col);
                mesh.colored_vertex(a - dir, col);
                mesh.add_triangle(i, i + 1, i + 2);
                mesh.add_triangle(i, i + 2, i + 3);
            }
        };
        let x0 = rect.min.x - cell;
        let y0 = rect.min.y - cell;
        let nx = (rect.width() / cell) as i32 + 3;
        let ny = (rect.height() / cell) as i32 + 3;
        for i in 0..nx {
            let x = x0 + i as f32 * cell;
            let n = ((rect.height() + 2.0 * cell) / step) as i32;
            line((0..=n).map(|k| warp(pos2(x, y0 + k as f32 * step))).collect());
        }
        for j in 0..ny {
            let y = y0 + j as f32 * cell;
            let n = ((rect.width() + 2.0 * cell) / step) as i32;
            line((0..=n).map(|k| warp(pos2(x0 + k as f32 * step, y))).collect());
        }
        painter.add(Shape::mesh(mesh));
        paint::vignette(&painter, rect, t.background, 0.55);
    }
}

impl Widget for WarpGrid {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = self.size.unwrap_or(vec2(ui.available_width(), 360.0));
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        self.paint(ui, rect);
        response
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct StoryState {
    pos: f32,
    vel: f32,
    target: f32,
    idle: f32,
    timer: f32,
    last: f32,
    speed: f32,
}

/// A scroll story of full-bleed pictures whose seams tear and melt into marbling as they cross
/// the view: the boundary between two pictures rips along a ragged edge while both images swirl
/// like marbled paper around it, strongest when you scroll fast, then settle clean as each
/// picture arrives.
///
/// Progress comes from (in order): [`Self::progress`], the widget's position in an enclosing
/// `ScrollArea` when [`Self::linked`] is set, or its own viewport (wheel / drag, with an
/// autoplay that steps through the pictures while idle).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::MarbleScroll::new().titles(["Bloom", "Tide", "Ember"]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct MarbleScroll {
    titles: Vec<String>,
    textures: Vec<TextureId>,
    height: f32,
    progress: Option<f32>,
    linked: bool,
    marble: f32,
    autoplay: f32,
}

impl Default for MarbleScroll {
    fn default() -> Self {
        Self::new()
    }
}

impl MarbleScroll {
    /// A story of five painted scenes.
    pub fn new() -> Self {
        Self {
            titles: vec!["Bloom".into(), "Drift".into(), "Thaw".into(), "Glow".into(), "Still".into()],
            textures: Vec::new(),
            height: 380.0,
            progress: None,
            linked: false,
            marble: 1.0,
            autoplay: 3.2,
        }
    }
    /// One title per picture (the number of titles sets the number of pages).
    pub fn titles(mut self, t: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let v: Vec<String> = t.into_iter().map(Into::into).collect();
        if !v.is_empty() {
            self.titles = v;
        }
        self
    }
    /// Your own pictures (cycled over the pages).
    pub fn textures(mut self, t: impl Into<Vec<TextureId>>) -> Self {
        self.textures = t.into();
        self
    }
    /// Viewport height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(120.0);
        self
    }
    /// Drive the story yourself: 0 = first picture, 1 = last.
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = Some(p.clamp(0.0, 1.0));
        self
    }
    /// Follow the enclosing `ScrollArea`: the story advances as the widget travels up through
    /// the visible viewport.
    pub fn linked(mut self, on: bool) -> Self {
        self.linked = on;
        self
    }
    /// Marbling strength, `0..=2`.
    pub fn marble(mut self, k: f32) -> Self {
        self.marble = k.clamp(0.0, 2.0);
        self
    }
    /// Seconds per picture in the idle autoplay (0 = off).
    pub fn autoplay(mut self, secs: f32) -> Self {
        self.autoplay = secs.max(0.0);
        self
    }
}

impl Widget for MarbleScroll {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::drag());
        let id = response.id;
        let n = self.titles.len().max(1);
        let h = self.height;
        let max = (n - 1) as f32 * h;
        let wdt = wall_dt(ui);
        let dt = glowkit_core::dt(ui);
        let external = self.progress.or_else(|| if self.linked { Some(view_progress(ui, rect, 0.0)) } else { None });
        let hovered = response.hovered() || response.dragged();
        let wheel = if response.hovered() { ui.input(|i| i.smooth_scroll_delta().y) } else { 0.0 };
        let drag = if response.dragged() { response.drag_delta().y } else { 0.0 };
        let released = response.drag_stopped();
        let auto = self.autoplay;
        let st = glowkit_core::with_state(ui, id.with("marble"), StoryState::default, |s| {
            if let Some(p) = external {
                s.pos = p * max;
            } else {
                if hovered {
                    s.idle = 0.0;
                } else {
                    s.idle += wdt;
                }
                if drag != 0.0 || wheel != 0.0 {
                    s.pos = (s.pos - drag - wheel).clamp(0.0, max);
                    s.target = s.pos;
                    s.vel = 0.0;
                    s.timer = 0.0;
                } else {
                    if released {
                        s.target = (s.pos / h).round() * h;
                    }
                    s.timer += wdt;
                    if auto > 0.0 && s.idle > 1.2 && s.timer > auto {
                        s.timer = 0.0;
                        let next = (s.target / h).round() + 1.0;
                        s.target = if next > (n - 1) as f32 { 0.0 } else { next * h };
                    }
                    let mut sp = glowkit_core::Spring { value: s.pos, velocity: s.vel, target: s.target, ..Default::default() }.with(60.0, 13.0);
                    sp.step(dt);
                    s.pos = sp.value.clamp(-20.0, max + 20.0);
                    s.vel = sp.velocity;
                }
            }
            let v = if wdt > 0.0 { (s.pos - s.last) / wdt } else { 0.0 };
            s.speed = glowkit_core::anim::approach(s.speed, v.abs(), 6.0, wdt);
            s.last = s.pos;
            *s
        });
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let time = glowkit_core::time(ui);
        let painter = ui.painter_at(rect);
        let radius = t.radius * 1.6;
        painter.rect_filled(rect, egui::CornerRadius::same(radius as u8), Color32::BLACK);
        let s = st.pos;
        let speed_k = (st.speed / 900.0).clamp(0.0, 1.5);
        let first = (s / h).floor().max(0.0) as usize;
        let tex_of = |i: usize| -> (TextureId, Rect) {
            if self.textures.is_empty() {
                let idx = [0usize, 1, 6, 5, 7, 2, 3, 4][i % 8];
                (PaintedScene::texture(ui.ctx(), idx), cover_uv(PaintedScene::ASPECT, rect.width() / h))
            } else {
                (self.textures[i % self.textures.len()], Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)))
            }
        };
        let marble = self.marble;
        // a page drawn as a distorted grid; `edge` = Some(seam_y) for the lower page (torn top)
        let draw_page = |i: usize, top: f32, seam: Option<f32>, amp: f32| {
            let (tex, uv) = tex_of(i);
            let (nx, ny) = (48usize, 44usize);
            let mut m = Mesh::with_texture(tex);
            // overlap neighbours so torn edges never show the backdrop
            let y_lo = rect.min.y.max(top - 80.0);
            let y_hi = rect.max.y.min(top + h + 80.0);
            if y_hi <= y_lo {
                return;
            }
            for j in 0..=ny {
                for k in 0..=nx {
                    let x = rect.min.x + k as f32 / nx as f32 * rect.width();
                    let y = y_lo + j as f32 / ny as f32 * (y_hi - y_lo);
                    // distance to the nearest seam on screen (above or below this page)
                    let d_top = (y - top).abs();
                    let d_bot = (top + h - y).abs();
                    let near = (-d_top.min(d_bot) / 110.0).exp();
                    let kx = (x - rect.min.x) / rect.width();
                    let ky = (y - top) / h;
                    let sw = amp * near * marble;
                    let wx = fbm3(kx * 3.0, ky * 3.0 + time * 0.2, time * 0.15 + i as f32, 3);
                    let wy = fbm3(kx * 3.0 + 7.3, ky * 3.0, time * 0.15 + 3.0, 3);
                    let swirl = noise3(kx * 9.0 + wx * 2.5, ky * 9.0 + wy * 2.5, time * 0.3);
                    let vein = (swirl * 7.0 + wx * 4.0).sin();
                    let du = (wx * 0.16 + vein * 0.035) * sw;
                    let dv = (wy * 0.22 + vein * 0.05) * sw;
                    let u = uv.min.x + (kx + du).clamp(0.0, 1.0) * uv.width();
                    let v = uv.min.y + (ky + dv).clamp(0.0, 1.0) * uv.height();
                    let mut a = 1.0;
                    if let Some(sy) = seam {
                        let edge = sy
                            + (fbm3(kx * 7.0, time * 0.4, i as f32, 4) * 40.0 + noise3(kx * 40.0, time, 2.0) * 9.0) * (0.5 + amp * 1.5);
                        a = ((y - edge) / 3.0 + 0.5).clamp(0.0, 1.0);
                    }
                    let shade = 1.0 - 0.25 * near * amp.min(1.0) * marble.min(1.0) * (0.5 + 0.5 * swirl);
                    let c = Color32::from_gray((255.0 * shade) as u8).gamma_multiply(a);
                    m.vertices.push(Vertex { pos: pos2(x, y), uv: pos2(u, v), color: c });
                }
            }
            let w = (nx + 1) as u32;
            for j in 0..ny as u32 {
                for k in 0..nx as u32 {
                    let a = j * w + k;
                    m.add_triangle(a, a + 1, a + w + 1);
                    m.add_triangle(a, a + w + 1, a + w);
                }
            }
            painter.add(Shape::mesh(m));
        };
        let i0 = first.min(n - 1);
        let top0 = rect.min.y + i0 as f32 * h - s;
        let seam = top0 + h; // screen y of the boundary between page i0 and i0+1
        let in_view = 1.0 - ((seam - rect.center().y).abs() / (h * 0.55)).clamp(0.0, 1.0);
        let amp = (0.25 + in_view * 0.75 + speed_k) * if i0 + 1 < n && seam < rect.max.y { 1.0 } else { 0.0 };
        draw_page(i0, top0, None, amp);
        if i0 + 1 < n && seam < rect.max.y {
            draw_page(i0 + 1, seam, Some(seam), amp);
            // a dark torn lip along the seam
            let pts: Vec<Pos2> = (0..=80)
                .map(|k| {
                    let kx = k as f32 / 80.0;
                    let edge = seam + (fbm3(kx * 7.0, time * 0.4, (i0 + 1) as f32, 4) * 40.0 + noise3(kx * 40.0, time, 2.0) * 9.0) * (0.5 + amp * 1.5);
                    pos2(rect.min.x + kx * rect.width(), edge)
                })
                .collect();
            painter.add(Shape::line(pts, Stroke::new(2.0, with_alpha(Color32::BLACK, 0.35 * (0.3 + amp).min(1.0)))));
        }
        // title of the page that owns the top of the view
        let cur = ((s + h * 0.35) / h).floor().clamp(0.0, (n - 1) as f32) as usize;
        let frac = ((s + h * 0.35) / h).fract();
        let a = (1.0 - ((frac - 0.35).abs() / 0.65)).clamp(0.0, 1.0).powf(0.5);
        let title = self.titles[cur].to_uppercase();
        let f = display_font(ui, 54.0);
        let tp = pos2(rect.center().x, rect.min.y + 46.0 + 12.0 * (1.0 - a));
        painter.text(tp + vec2(0.0, 2.0), Align2::CENTER_TOP, &title, f.clone(), with_alpha(Color32::BLACK, 0.35 * a));
        painter.text(tp, Align2::CENTER_TOP, &title, f, with_alpha(Color32::WHITE, a));
        painter.text(
            pos2(rect.min.x + 22.0, rect.max.y - 18.0),
            Align2::LEFT_BOTTOM,
            format!("{:02} / {:02}", cur + 1, n),
            FontId::monospace(12.0),
            with_alpha(Color32::WHITE, 0.8),
        );
        // progress rail
        let rail = Rect::from_min_size(pos2(rect.max.x - 14.0, rect.min.y + 20.0), vec2(3.0, rect.height() - 40.0));
        painter.rect_filled(rail, egui::CornerRadius::same(2), with_alpha(Color32::WHITE, 0.2));
        let p = if max > 0.0 { (s / max).clamp(0.0, 1.0) } else { 0.0 };
        let th = rail.height() / n as f32;
        painter.rect_filled(
            Rect::from_min_size(pos2(rail.min.x, rail.min.y + (rail.height() - th) * p), vec2(3.0, th)),
            egui::CornerRadius::same(2),
            with_alpha(Color32::WHITE, 0.85),
        );
        round_corners(&painter, rect, radius, t.background);
        response
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct TrailState {
    heat: Vec<f32>,
    last: Option<Pos2>,
    cols: usize,
    rows: usize,
}

/// A field of tiny greyscale thumbnails: along the pointer's path they swell and bloom into
/// full-colour pictures, then shrink back to dots behind it. A ghost pointer draws trails while
/// the real one is away.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::MosaicTrail::new().cell(26.0).reach(70.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct MosaicTrail {
    size: Option<Vec2>,
    cell: f32,
    reach: f32,
    fade: f32,
    textures: Vec<TextureId>,
    autoplay: bool,
}

impl Default for MosaicTrail {
    fn default() -> Self {
        Self::new()
    }
}

impl MosaicTrail {
    /// A mosaic of the painted scenes.
    pub fn new() -> Self {
        Self { size: None, cell: 26.0, reach: 70.0, fade: 1.1, textures: Vec::new(), autoplay: true }
    }
    /// Widget size (default: available width x 360).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Grid pitch in points.
    pub fn cell(mut self, c: f32) -> Self {
        self.cell = c.clamp(10.0, 80.0);
        self
    }
    /// Radius of the bloom around the pointer path.
    pub fn reach(mut self, r: f32) -> Self {
        self.reach = r.clamp(10.0, 300.0);
        self
    }
    /// Seconds for a bloomed tile to shrink back.
    pub fn fade(mut self, secs: f32) -> Self {
        self.fade = secs.clamp(0.1, 6.0);
        self
    }
    /// Your own thumbnails (cycled).
    pub fn textures(mut self, t: impl Into<Vec<TextureId>>) -> Self {
        self.textures = t.into();
        self
    }
    /// Draw trails with a ghost pointer while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn seg_dist(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    (a + ab * t).distance(p)
}

impl Widget for MosaicTrail {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = self.size.unwrap_or(vec2(ui.available_width(), 360.0));
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let id = response.id;
        let time = glowkit_core::time(ui);
        let dt = wall_dt(ui);
        let hover = response.hover_pos();
        let idle = idle_time(ui, id, hover.is_some());
        let pointer = match hover {
            Some(p) => Some(p),
            None if self.autoplay && idle > 0.6 => Some(ghost(rect.shrink(30.0), time * 1.1)),
            None => None,
        };
        let cell = self.cell;
        let cols = (rect.width() / cell).floor().max(1.0) as usize;
        let rows = (rect.height() / cell).floor().max(1.0) as usize;
        let off = rect.min + (rect.size() - vec2(cols as f32, rows as f32) * cell) / 2.0;
        let reach = self.reach;
        let decay = (-dt / self.fade * 2.2).exp();
        let heat = glowkit_core::with_state(ui, id.with("mosaic"), TrailState::default, |s| {
            if s.cols != cols || s.rows != rows {
                *s = TrailState { heat: vec![0.0; cols * rows], last: None, cols, rows };
            }
            for h in &mut s.heat {
                *h *= decay;
            }
            if let Some(p) = pointer {
                let a = s.last.unwrap_or(p);
                let lo = a.min(p) - Vec2::splat(reach);
                let hi = a.max(p) + Vec2::splat(reach);
                let cx0 = (((lo.x - off.x) / cell).floor().max(0.0) as usize).min(cols);
                let cx1 = (((hi.x - off.x) / cell).ceil().max(0.0) as usize).min(cols);
                let cy0 = (((lo.y - off.y) / cell).floor().max(0.0) as usize).min(rows);
                let cy1 = (((hi.y - off.y) / cell).ceil().max(0.0) as usize).min(rows);
                for y in cy0..cy1 {
                    for x in cx0..cx1 {
                        let c = off + vec2((x as f32 + 0.5) * cell, (y as f32 + 0.5) * cell);
                        let d = seg_dist(c, a, p);
                        let k = (1.0 - d / reach).clamp(0.0, 1.0);
                        let hh = &mut s.heat[y * cols + x];
                        *hh = hh.max(k * k * (3.0 - 2.0 * k));
                    }
                }
                s.last = Some(p);
            } else {
                s.last = None;
            }
            s.heat.clone()
        });
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let dot = if t.dark { hex("#E8E8EC") } else { hex("#141416") };
        let mut dots = Mesh::default();
        let mut tiles: Vec<(usize, usize, f32)> = Vec::new();
        for y in 0..rows {
            for x in 0..cols {
                let hh = heat[y * cols + x];
                let c = off + vec2((x as f32 + 0.5) * cell, (y as f32 + 0.5) * cell);
                if hh < 0.04 {
                    let s = cell * 0.30;
                    mesh_rect(&mut dots, Rect::from_center_size(c, Vec2::splat(s)), dot);
                    // a whisper of the thumbnail inside the dot
                    let hv = hash3(x as i32, y as i32, 5);
                    if hv > 0.55 {
                        let g = if t.dark { 0.35 } else { 0.62 };
                        mesh_rect(&mut dots, Rect::from_center_size(c + vec2(0.0, s * 0.12), vec2(s * 0.5, s * 0.22)), lerp_c(dot, Color32::from_gray((255.0 * g) as u8), 0.5));
                    }
                } else {
                    tiles.push((x, y, hh));
                }
            }
        }
        painter.add(Shape::mesh(dots));
        // bloomed tiles, coolest first so the hottest sit on top
        tiles.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        let uv = cover_uv(PaintedScene::ASPECT, 1.0);
        let mut by_tex: std::collections::BTreeMap<u64, Mesh> = Default::default();
        let mut grey = Mesh::default();
        for (x, y, hh) in tiles {
            let c = off + vec2((x as f32 + 0.5) * cell, (y as f32 + 0.5) * cell);
            let e = glowkit_core::ease(glowkit_core::Easing::OutBack, hh.min(1.0));
            let s = cell * (0.30 + 0.66 * e);
            let r = Rect::from_center_size(c, Vec2::splat(s));
            let k = (hash3(x as i32, y as i32, 9) * 997.0) as usize;
            let tex = if self.textures.is_empty() {
                PaintedScene::texture(ui.ctx(), k % PaintedScene::COUNT)
            } else {
                self.textures[k % self.textures.len()]
            };
            let key = match tex {
                TextureId::Managed(v) => v,
                TextureId::User(v) => v ^ (1 << 63),
            };
            let m = by_tex.entry(key).or_insert_with(|| Mesh::with_texture(tex));
            let i = m.vertices.len() as u32;
            let col = (hh * 1.6).clamp(0.0, 1.0);
            let tint = Color32::WHITE;
            for (p, q) in [(r.left_top(), uv.left_top()), (r.right_top(), uv.right_top()), (r.right_bottom(), uv.right_bottom()), (r.left_bottom(), uv.left_bottom())] {
                m.vertices.push(Vertex { pos: p, uv: q, color: tint });
            }
            m.add_triangle(i, i + 1, i + 2);
            m.add_triangle(i, i + 2, i + 3);
            // wash the colour out toward grey while the tile is still small
            let wash = 1.0 - col;
            if wash > 0.02 {
                let g = if t.dark { Color32::from_gray(60) } else { Color32::from_gray(150) };
                mesh_rect(&mut grey, r, with_alpha(g, wash * 0.75));
            }
        }
        for (_, m) in by_tex {
            painter.add(Shape::mesh(m));
        }
        painter.add(Shape::mesh(grey));
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// One step of a [`StepPath`].
#[derive(Clone, Debug, PartialEq)]
pub struct PathStep {
    /// Short title.
    pub title: String,
    /// A sentence or two.
    pub body: String,
}

impl PathStep {
    /// A step with a title and body.
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self { title: title.into(), body: body.into() }
    }
}

/// Process steps joined by one dashed line that hops from dot to dot. It draws itself in as it
/// scrolls into view (inside a `ScrollArea`), each step fading up as the line arrives, then a
/// darker stretch keeps travelling along it.
///
/// Drive it from outside with [`Self::progress`], or let it loop with [`Self::autoplay`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{PathStep, StepPath};
/// let steps = vec![PathStep::new("Listen", "We start with the people who will use it.")];
/// ui.add(StepPath::new(&steps));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct StepPath<'a> {
    steps: &'a [PathStep],
    height: f32,
    progress: Option<f32>,
    autoplay: bool,
    hop: f32,
    color: Option<Color32>,
}

impl<'a> StepPath<'a> {
    /// A path through `steps`.
    pub fn new(steps: &'a [PathStep]) -> Self {
        Self { steps, height: 340.0, progress: None, autoplay: false, hop: 1.0, color: None }
    }
    /// Widget height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(160.0);
        self
    }
    /// Drive the draw-in yourself (0 = nothing drawn, 1 = complete).
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = Some(p.clamp(0.0, 1.0));
        self
    }
    /// Replay the draw-in on a loop instead of following the scroll position.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Height of the hops between dots, `0..=2`.
    pub fn hop(mut self, k: f32) -> Self {
        self.hop = k.clamp(0.0, 2.0);
        self
    }
    /// Color of the dots and the travelling stretch (defaults to the foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

impl Widget for StepPath<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let id = response.id;
        let time = glowkit_core::time(ui);
        let n = self.steps.len();
        let p = match self.progress {
            Some(p) => p,
            None if self.autoplay => {
                let cyc = glowkit_core::anim::Clock::elapsed(ui, id) % 9.0;
                (cyc / 3.6).min(1.0) * if cyc > 8.4 { (9.0 - cyc) / 0.6 } else { 1.0 }
            }
            None => view_progress(ui, rect, 0.55),
        };
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) || n == 0 {
            return response;
        }
        let painter = ui.painter_at(rect);
        let ink = self.color.unwrap_or(t.foreground);
        let col_w = rect.width() / n as f32;
        let drop = ((self.height - 200.0) / (n.max(2) - 1) as f32).clamp(20.0, 70.0);
        let dots: Vec<Pos2> = (0..n)
            .map(|i| pos2(rect.min.x + col_w * (i as f32 + 0.5) - col_w * 0.18, rect.min.y + 40.0 + i as f32 * drop))
            .collect();
        // the hopping path
        let mut path: Vec<Pos2> = Vec::new();
        let mut marks = vec![0.0f32];
        let mut len = 0.0;
        for w in dots.windows(2) {
            let (a, b) = (w[0], w[1]);
            let ctrl = pos2((a.x + b.x) / 2.0, a.y.min(b.y) - 70.0 * self.hop);
            let seg = paint::quad_bezier(a, ctrl, b, 40);
            for (k, q) in seg.iter().enumerate() {
                if let Some(last) = path.last() {
                    if k > 0 {
                        len += last.distance(*q);
                    }
                }
                if k > 0 || path.is_empty() {
                    path.push(*q);
                }
            }
            marks.push(len);
        }
        let drawn = p * len.max(1.0);
        // cut the path at `drawn`
        let cut = |upto: f32| -> Vec<Pos2> {
            let mut out = Vec::new();
            let mut acc = 0.0;
            for w in path.windows(2) {
                let l = w[0].distance(w[1]);
                if out.is_empty() {
                    out.push(w[0]);
                }
                if acc + l >= upto {
                    let f = ((upto - acc) / l.max(1e-3)).clamp(0.0, 1.0);
                    out.push(w[0].lerp(w[1], f));
                    return out;
                }
                out.push(w[1]);
                acc += l;
            }
            out
        };
        let light = with_alpha(t.muted_foreground, 0.45);
        dashed(&painter, &cut(drawn), 6.0, 6.0, 0.0, 1.4, light);
        // travelling darker stretch once complete
        if p > 0.98 && len > 0.0 {
            let head = (time * 140.0) % (len + 160.0);
            let tail = (head - 110.0).max(0.0);
            let seg: Vec<Pos2> = {
                let full = cut(head.min(len));
                let mut acc = 0.0;
                let mut out = Vec::new();
                for w in full.windows(2) {
                    let l = w[0].distance(w[1]);
                    if acc + l >= tail {
                        if out.is_empty() {
                            out.push(w[0].lerp(w[1], ((tail - acc) / l.max(1e-3)).clamp(0.0, 1.0)));
                        }
                        out.push(w[1]);
                    }
                    acc += l;
                }
                out
            };
            dashed(&painter, &seg, 6.0, 6.0, 0.0, 1.8, with_alpha(ink, 0.85));
        }
        // dots and copy
        let title_f = semibold_font(ui, 17.0);
        let num_f = semibold_font(ui, 17.0);
        let body_f = FontId::proportional(13.0);
        for (i, step) in self.steps.iter().enumerate() {
            let at = marks[i];
            let k = ((drawn - at + 30.0) / 70.0).clamp(0.0, 1.0);
            let e = glowkit_core::ease(glowkit_core::Easing::OutCubic, k);
            if e <= 0.0 {
                continue;
            }
            let d = dots[i];
            painter.circle_filled(d, 4.5 * glowkit_core::ease(glowkit_core::Easing::OutBack, k), with_alpha(ink, e));
            let y = d.y + 22.0 + 10.0 * (1.0 - e);
            let num = format!("{:02})", i + 1);
            let gn = painter.layout_no_wrap(num, num_f.clone(), with_alpha(t.muted_foreground, e));
            let gt = painter.layout_no_wrap(step.title.clone(), title_f.clone(), with_alpha(t.foreground, e));
            let tw = gn.size().x + 5.0 + gt.size().x;
            let cx = d.x + col_w * 0.18;
            let x0 = cx - tw / 2.0;
            painter.galley(pos2(x0, y), gn.clone(), with_alpha(t.muted_foreground, e));
            painter.galley(pos2(x0 + gn.size().x + 5.0, y), gt, with_alpha(t.foreground, e));
            let mut job = egui::text::LayoutJob::simple(step.body.clone(), body_f.clone(), with_alpha(t.muted_foreground, e), (col_w - 24.0).max(60.0));
            job.halign = egui::Align::Center;
            let gb = painter.layout_job(job);
            painter.galley(pos2(cx, y + 28.0), gb, with_alpha(t.muted_foreground, e));
        }
        response
    }
}
