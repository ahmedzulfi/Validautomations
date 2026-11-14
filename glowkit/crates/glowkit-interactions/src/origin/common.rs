//! Shared helpers for the origin module: tiny 3D math, batched dot / line meshes, ghost cursors.

use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{pos2, vec2, Color32, Id, Painter, Pos2, Rect, Response, Shape, TextureId, Ui, Vec2};
use glowkit_core::color::with_alpha;

/// Minimal 3D vector.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub(crate) const fn v3(x: f32, y: f32, z: f32) -> V3 {
    V3 { x, y, z }
}

impl V3 {
    pub fn rot_x(self, a: f32) -> V3 {
        let (s, c) = a.sin_cos();
        v3(self.x, self.y * c - self.z * s, self.y * s + self.z * c)
    }
    pub fn rot_y(self, a: f32) -> V3 {
        let (s, c) = a.sin_cos();
        v3(self.x * c + self.z * s, self.y, -self.x * s + self.z * c)
    }
    pub fn rot_z(self, a: f32) -> V3 {
        let (s, c) = a.sin_cos();
        v3(self.x * c - self.y * s, self.x * s + self.y * c, self.z)
    }
    pub fn len(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
    pub fn norm(self) -> V3 {
        let l = self.len().max(1e-6);
        v3(self.x / l, self.y / l, self.z / l)
    }
    pub fn dot(self, o: V3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
}

impl std::ops::Add for V3 {
    type Output = V3;
    fn add(self, o: V3) -> V3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl std::ops::Sub for V3 {
    type Output = V3;
    fn sub(self, o: V3) -> V3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl std::ops::Mul<f32> for V3 {
    type Output = V3;
    fn mul(self, k: f32) -> V3 {
        v3(self.x * k, self.y * k, self.z * k)
    }
}

/// A pinhole camera looking down -z from `dist` units away; `focal` in points per unit at depth 1.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cam {
    pub center: Pos2,
    pub focal: f32,
    pub dist: f32,
}

impl Cam {
    /// Project a point; returns screen position, perspective scale and view depth (bigger = farther).
    pub fn project(&self, p: V3) -> Option<(Pos2, f32, f32)> {
        let d = self.dist - p.z;
        if d < 0.05 {
            return None;
        }
        let s = self.focal / d;
        Some((pos2(self.center.x + p.x * s, self.center.y - p.y * s), s, d))
    }
}

/// A color that blends additively when painted (premultiplied, zero alpha) on dark themes, and as a
/// plain translucent color on light themes (where adding light would vanish into white).
pub(crate) fn lum(c: Color32, a: f32, additive: bool) -> Color32 {
    let a = a.clamp(0.0, 1.0);
    if additive {
        Color32::from_rgba_premultiplied(
            (c.r() as f32 * a) as u8,
            (c.g() as f32 * a) as u8,
            (c.b() as f32 * a) as u8,
            0,
        )
    } else {
        with_alpha(c, a)
    }
}

/// One mesh collecting many small dots (cheap: 4-7 vertices per dot).
pub(crate) struct Dots {
    pub mesh: Mesh,
}

impl Dots {
    pub fn new(cap: usize) -> Self {
        let mut mesh = Mesh::default();
        mesh.reserve_vertices(cap * 5);
        mesh.reserve_triangles(cap * 4);
        Self { mesh }
    }
    /// Add a dot of radius `r` (a quad for tiny dots, a hexagon otherwise).
    pub fn add(&mut self, p: Pos2, r: f32, c: Color32) {
        if c.a() == 0 && c.r() == 0 && c.g() == 0 && c.b() == 0 {
            return;
        }
        let m = &mut self.mesh;
        let i = m.vertices.len() as u32;
        if r < 1.4 {
            let r = r.max(0.45);
            for d in [vec2(0.0, -r), vec2(r, 0.0), vec2(0.0, r), vec2(-r, 0.0)] {
                m.vertices.push(Vertex { pos: p + d * 1.25, uv: WHITE_UV, color: c });
            }
            m.add_triangle(i, i + 1, i + 2);
            m.add_triangle(i, i + 2, i + 3);
        } else {
            m.vertices.push(Vertex { pos: p, uv: WHITE_UV, color: c });
            for k in 0..6 {
                let a = k as f32 * std::f32::consts::TAU / 6.0;
                m.vertices.push(Vertex { pos: p + vec2(a.cos(), a.sin()) * r, uv: WHITE_UV, color: c });
            }
            for k in 0..6u32 {
                m.add_triangle(i, i + 1 + k, i + 1 + (k + 1) % 6);
            }
        }
    }
    /// Add a soft dot: solid core plus a transparent rim (looks round and anti-aliased).
    pub fn soft(&mut self, p: Pos2, r: f32, c: Color32, transparent: Color32) {
        let m = &mut self.mesh;
        let i = m.vertices.len() as u32;
        m.vertices.push(Vertex { pos: p, uv: WHITE_UV, color: c });
        let n = 8u32;
        for k in 0..n {
            let a = k as f32 * std::f32::consts::TAU / n as f32;
            m.vertices.push(Vertex { pos: p + vec2(a.cos(), a.sin()) * r, uv: WHITE_UV, color: transparent });
        }
        for k in 0..n {
            m.add_triangle(i, i + 1 + k, i + 1 + (k + 1) % n);
        }
    }
    pub fn paint(self, painter: &Painter) {
        if !self.mesh.vertices.is_empty() {
            painter.add(Shape::mesh(self.mesh));
        }
    }
}

/// Append a feathered polyline (anti-aliased by a transparent fringe) to `mesh`.
/// `col(i)` gives the color at point `i`, `w(i)` the core width.
pub(crate) fn strip(
    mesh: &mut Mesh,
    pts: &[Pos2],
    w: impl Fn(usize) -> f32,
    col: impl Fn(usize) -> Color32,
    clear: impl Fn(Color32) -> Color32,
) {
    let n = pts.len();
    if n < 2 {
        return;
    }
    let base = mesh.vertices.len() as u32;
    for i in 0..n {
        let a = pts[i.saturating_sub(1)];
        let b = pts[(i + 1).min(n - 1)];
        let d = b - a;
        let nrm = if d.length_sq() > 1e-8 { d.normalized().rot90() } else { Vec2::X };
        let c = col(i);
        let hw = (w(i) * 0.5).max(0.05);
        let f = 0.9;
        // thin lines: fade the core instead of making it thinner than a pixel
        let (hw, c) = if hw < 0.5 { (0.5, scale_color(c, hw / 0.5)) } else { (hw, c) };
        let cl = clear(c);
        let p = pts[i];
        mesh.vertices.push(Vertex { pos: p - nrm * (hw + f), uv: WHITE_UV, color: cl });
        mesh.vertices.push(Vertex { pos: p - nrm * hw, uv: WHITE_UV, color: c });
        mesh.vertices.push(Vertex { pos: p + nrm * hw, uv: WHITE_UV, color: c });
        mesh.vertices.push(Vertex { pos: p + nrm * (hw + f), uv: WHITE_UV, color: cl });
    }
    for i in 0..(n - 1) as u32 {
        let a = base + i * 4;
        let b = a + 4;
        for k in 0..3 {
            mesh.add_triangle(a + k, b + k, b + k + 1);
            mesh.add_triangle(a + k, b + k + 1, a + k + 1);
        }
    }
}

/// Scale every channel (premultiplied color) by `k`.
pub(crate) fn scale_color(c: Color32, k: f32) -> Color32 {
    let k = k.clamp(0.0, 1.0);
    Color32::from_rgba_premultiplied(
        (c.r() as f32 * k) as u8,
        (c.g() as f32 * k) as u8,
        (c.b() as f32 * k) as u8,
        (c.a() as f32 * k) as u8,
    )
}

/// The transparent fringe color for a premultiplied color: fully transparent.
pub(crate) fn clear(_c: Color32) -> Color32 {
    Color32::TRANSPARENT
}

/// Pointer position inside `rect` if hovering, else a smooth ghost path (for idle showcases).
/// Returns `(position, is_real_pointer)`.
pub(crate) fn pointer_or_ghost(ui: &Ui, rect: Rect, response: &Response, time: f32, ghost: bool) -> (Option<Pos2>, bool) {
    if let Some(p) = response.hover_pos() {
        return (Some(p), true);
    }
    if !ghost {
        return (None, false);
    }
    let _ = ui;
    let t = time * 0.55;
    let u = 0.5 + 0.34 * (t * 1.0).sin() * (0.8 + 0.2 * (t * 0.37).cos());
    let v = 0.5 + 0.28 * (t * 1.7 + 0.6).sin();
    (Some(pos2(rect.min.x + u * rect.width(), rect.min.y + v * rect.height())), false)
}

/// Seconds since the pointer last touched widget `id` (large when never).
pub(crate) fn idle_secs(ui: &Ui, id: Id, active: bool) -> f32 {
    let now = ui.input(|i| i.time);
    glowkit_core::with_state(
        ui,
        id.with("origin_idle"),
        || now - 1000.0,
        |last| {
            if active {
                *last = now;
            }
            (now - *last) as f32
        },
    )
}

/// Append a textured quad with arbitrary corners (tl, tr, br, bl) subdivided `n x n` for less
/// affine distortion. `uv` corners follow the same order; `tint(u, v)` colors each vertex.
pub(crate) fn textured_quad(
    painter: &Painter,
    tex: TextureId,
    corners: [Pos2; 4],
    n: usize,
    tint: impl Fn(f32, f32) -> Color32,
) {
    let n = n.max(1);
    let mut m = Mesh::with_texture(tex);
    let [tl, tr, br, bl] = corners;
    for j in 0..=n {
        let v = j as f32 / n as f32;
        let l = tl.lerp(bl, v);
        let r = tr.lerp(br, v);
        for i in 0..=n {
            let u = i as f32 / n as f32;
            m.vertices.push(Vertex { pos: l.lerp(r, u), uv: pos2(u, v), color: tint(u, v) });
        }
    }
    let w = (n + 1) as u32;
    for j in 0..n as u32 {
        for i in 0..n as u32 {
            let a = j * w + i;
            m.add_triangle(a, a + 1, a + w + 1);
            m.add_triangle(a, a + w + 1, a + w);
        }
    }
    painter.add(Shape::mesh(m));
}

/// Textured mesh from a grid of projected points (`cols+1` x `rows+1`, row-major), with tint.
pub(crate) fn textured_grid(painter: &Painter, tex: TextureId, pts: &[(Pos2, Color32)], cols: usize, rows: usize) {
    let mut m = Mesh::with_texture(tex);
    for j in 0..=rows {
        for i in 0..=cols {
            let (p, c) = pts[j * (cols + 1) + i];
            m.vertices.push(Vertex { pos: p, uv: pos2(i as f32 / cols as f32, j as f32 / rows as f32), color: c });
        }
    }
    let w = (cols + 1) as u32;
    for j in 0..rows as u32 {
        for i in 0..cols as u32 {
            let a = j * w + i;
            m.add_triangle(a, a + 1, a + w + 1);
            m.add_triangle(a, a + w + 1, a + w);
        }
    }
    painter.add(Shape::mesh(m));
}

/// Hash an integer to `0..1`.
pub(crate) fn h01(i: u64) -> f32 {
    glowkit_core::rng::hash01(i)
}

/// Mix two colors in plain RGB (fast, for per-vertex work).
pub(crate) fn lerp_rgb(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgba_premultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

/// Multi-stop RGB gradient (fast).
pub(crate) fn ramp(stops: &[Color32], t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let i = (t.floor() as usize).min(stops.len().saturating_sub(2));
    lerp_rgb(stops[i], stops[(i + 1).min(stops.len() - 1)], t - i as f32)
}
