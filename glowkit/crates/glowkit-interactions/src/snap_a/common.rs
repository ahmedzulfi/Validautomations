//! Shared machinery for the snap_a module: a tiny 3D sheet renderer (paper / cloth meshes with
//! facet shading and depth sorting), an off-screen "ink" recorder so text and drawings can be
//! printed onto a deformed sheet, a crumple field, scroll-progress helpers, idle timers and a
//! ghost cursor for autoplaying demos.

use egui::epaint::{Mesh, Tessellator, Vertex, WHITE_UV};
use egui::{pos2, vec2, Align2, Color32, Context, FontId, Id, Painter, Pos2, Rect, Shape, Stroke, Ui, Vec2};
use glowkit_core::color::with_alpha;
use std::f32::consts::PI;
use std::sync::Arc;

// ---------------------------------------------------------------------------------------------
// Small math

/// A 3D vector (x right, y down, z toward the viewer).
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
    pub fn dot(self, o: V3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: V3) -> V3 {
        v3(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }
    pub fn len(self) -> f32 {
        self.dot(self).sqrt()
    }
    pub fn norm(self) -> V3 {
        let l = self.len().max(1e-6);
        v3(self.x / l, self.y / l, self.z / l)
    }
    pub fn lerp(self, o: V3, t: f32) -> V3 {
        self + (o - self) * t
    }
}

/// A rotation about the X axis (pitch), then Y (yaw), then Z (roll), with its sines and
/// cosines worked out once, for per-vertex use.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Rot {
    pitch: (f32, f32),
    yaw: (f32, f32),
    roll: (f32, f32),
}

impl Rot {
    pub fn new(pitch: f32, yaw: f32, roll: f32) -> Self {
        Self { pitch: pitch.sin_cos(), yaw: yaw.sin_cos(), roll: roll.sin_cos() }
    }
    #[inline]
    pub fn apply(&self, v: V3) -> V3 {
        let (s, c) = self.pitch;
        let p = v3(v.x, v.y * c - v.z * s, v.y * s + v.z * c);
        let (s, c) = self.yaw;
        let p = v3(p.x * c + p.z * s, p.y, -p.x * s + p.z * c);
        let (s, c) = self.roll;
        v3(p.x * c - p.y * s, p.x * s + p.y * c, p.z)
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

/// Triangle wave in `-0.5..=0.5`: sharp ridges, the building block of paper creases.
#[inline]
pub(crate) fn tri_wave(x: f32) -> f32 {
    (x.rem_euclid(1.0) - 0.5).abs() * 2.0 - 0.5
}

/// Cheap hash to `0..1`.
#[inline]
pub(crate) fn h01(i: u64) -> f32 {
    glowkit_core::rng::hash01(i)
}

/// Linear (non-perceptual) color lerp: cheap enough for per-vertex use.
#[inline]
pub(crate) fn lerp_c(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()), f(a.a(), b.a()))
}

/// Multiply a color's rgb by `k` (lighting), keeping alpha.
#[inline]
pub(crate) fn lit(c: Color32, k: f32) -> Color32 {
    let f = |x: u8| (x as f32 * k).clamp(0.0, 255.0) as u8;
    Color32::from_rgba_premultiplied(f(c.r()).min(c.a()), f(c.g()).min(c.a()), f(c.b()).min(c.a()), c.a())
}

/// Smooth 0..1 ramp.
#[inline]
pub(crate) fn sstep(a: f32, b: f32, x: f32) -> f32 {
    glowkit_core::anim::smoothstep(a, b, x)
}

// ---------------------------------------------------------------------------------------------
// Camera + lighting

/// A simple perspective camera: rotate local geometry, then project around `center`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cam {
    pub center: Pos2,
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
    pub focal: f32,
    /// Offset applied after rotation (camera space).
    pub offset: V3,
}

impl Cam {
    pub fn new(center: Pos2) -> Self {
        Self { center, pitch: 0.0, yaw: 0.0, roll: 0.0, focal: 1400.0, offset: V3::default() }
    }
    /// Local point to camera space, as a function with the rotation worked out once.
    pub fn viewer(&self) -> impl Fn(V3) -> V3 {
        let r = Rot::new(self.pitch, self.yaw, self.roll);
        let o = self.offset;
        move |p| r.apply(p) + o
    }
    /// Camera-space point to screen.
    pub fn project(&self, p: V3) -> Pos2 {
        let s = self.focal / (self.focal - p.z).max(40.0);
        self.center + vec2(p.x * s, p.y * s)
    }
}

/// Light coming from the upper left, slightly toward the viewer.
pub(crate) const LIGHT: V3 = v3(-0.42, -0.62, 0.66);

/// Diffuse + a soft specular sheen for a camera-space normal facing the viewer, normalised so
/// a sheet lying flat toward the camera gets exactly 1.
#[inline]
pub(crate) fn shade(n: V3, ambient: f32, gloss: f32) -> f32 {
    // the light and half vectors are constants: normalise them once
    static LH: std::sync::OnceLock<(V3, V3)> = std::sync::OnceLock::new();
    let &(l, h) = LH.get_or_init(|| {
        let l = LIGHT.norm();
        (l, (l + v3(0.0, 0.0, 1.0)).norm())
    });
    let d = n.dot(l).max(0.0);
    let s = n.dot(h).max(0.0).powi(18) * gloss;
    let flat = ambient + (1.0 - ambient) * l.z + h.z.powi(18) * gloss;
    (ambient + (1.0 - ambient) * d + s) / flat
}

// ---------------------------------------------------------------------------------------------
// Depth-sorted triangle soup

/// Collects triangles from any number of sheets, then paints them back to front as one mesh.
#[derive(Default)]
pub(crate) struct Soup {
    tris: Vec<(f32, [Vertex; 3])>,
}

impl Soup {
    pub fn new() -> Self {
        Self { tris: Vec::with_capacity(4096) }
    }
    pub fn push(&mut self, z: f32, v: [Vertex; 3]) {
        self.tris.push((z, v));
    }
    /// Fade every vertex by its vertical distance from `cy` (full inside `inner`, gone at `outer`).
    pub fn fade_y(&mut self, cy: f32, inner: f32, outer: f32) {
        for (_, v) in &mut self.tris {
            for vx in v.iter_mut() {
                let k = 1.0 - sstep(inner, outer, (vx.pos.y - cy).abs());
                vx.color = vx.color.gamma_multiply(k);
            }
        }
    }
    pub fn paint(self, painter: &Painter) {
        // Sort compact (depth, index) keys instead of the 64-byte triangles; the index keeps
        // the order of equal depths as emitted (a stable sort), and NaN depths sort as equal.
        let ordered = |z: f32| {
            let b = if z.is_nan() { 0.0f32.to_bits() } else { z.to_bits() };
            if b & 0x8000_0000 != 0 {
                !b
            } else {
                b | 0x8000_0000
            }
        };
        let mut keys: Vec<u64> =
            self.tris.iter().enumerate().map(|(i, (z, _))| ((ordered(*z) as u64) << 32) | i as u64).collect();
        keys.sort_unstable();
        let mut m = Mesh::default();
        m.reserve_vertices(self.tris.len().min(60_000) * 3);
        m.reserve_triangles(self.tris.len());
        for key in keys {
            let v = &self.tris[(key & 0xFFFF_FFFF) as usize].1;
            let b = m.vertices.len() as u32;
            m.vertices.extend_from_slice(v);
            m.add_triangle(b, b + 1, b + 2);
            if m.vertices.len() > 60_000 {
                painter.add(Shape::mesh(std::mem::take(&mut m)));
            }
        }
        painter.add(Shape::mesh(m));
    }
}

/// A sheet (paper, cloth) described by a deform function over `u, v in 0..1`.
pub(crate) struct Sheet<'a> {
    pub size: Vec2,
    pub cols: usize,
    pub rows: usize,
    /// Local 3D position of `(u, v)` (before the camera transform).
    pub deform: &'a dyn Fn(f32, f32) -> V3,
    /// Front color at `(u, v)`.
    pub front: &'a dyn Fn(f32, f32) -> Color32,
    /// Color of the reverse side.
    pub back: Color32,
    pub ambient: f32,
    pub gloss: f32,
    /// Depth bias added to every triangle of this sheet (to layer sheets that touch).
    pub bias: f32,
    /// Overall opacity.
    pub alpha: f32,
    /// Extra depth bias for printed ink. Large values keep ink on top of its own sheet (right
    /// for sheets that never overlap themselves); small values let folds hide it.
    pub ink_bias: f32,
}

impl Sheet<'_> {
    /// Emit facet-shaded triangles (and optional ink printed on the front) into `soup`.
    pub fn emit(&self, soup: &mut Soup, cam: &Cam, ink: Option<&Mesh>) {
        let (cols, rows) = (self.cols.max(1), self.rows.max(1));
        let w = cols + 1;
        let mut pts: Vec<V3> = Vec::with_capacity(w * (rows + 1));
        let mut scr: Vec<Pos2> = Vec::with_capacity(w * (rows + 1));
        let mut col: Vec<Color32> = Vec::with_capacity(w * (rows + 1));
        let view = cam.viewer();
        for j in 0..=rows {
            for i in 0..=cols {
                let (u, v) = (i as f32 / cols as f32, j as f32 / rows as f32);
                let p = view((self.deform)(u, v));
                pts.push(p);
                scr.push(cam.project(p));
                col.push((self.front)(u, v));
            }
        }
        let a = self.alpha.clamp(0.0, 1.0);
        let back = self.back;
        let mut tri = |ia: usize, ib: usize, ic: usize| {
            let (pa, pb, pc) = (pts[ia], pts[ib], pts[ic]);
            let n = (pb - pa).cross(pc - pa).norm();
            // grid winding gives +z normals for the front face
            let front = n.z > 0.0;
            let nn = if front { n } else { n * -1.0 };
            let k = shade(nn, self.ambient, self.gloss);
            let z = (pa.z + pb.z + pc.z) / 3.0 + self.bias;
            let vx = |i: usize| {
                let base = if front { col[i] } else { back };
                Vertex { pos: scr[i], uv: WHITE_UV, color: lit(base, k).gamma_multiply(a) }
            };
            soup.push(z, [vx(ia), vx(ib), vx(ic)]);
        };
        for j in 0..rows {
            for i in 0..cols {
                let i0 = j * w + i;
                // alternate the diagonal for a less regular crease pattern
                if (i + j) % 2 == 0 {
                    tri(i0, i0 + 1, i0 + w + 1);
                    tri(i0, i0 + w + 1, i0 + w);
                } else {
                    tri(i0, i0 + 1, i0 + w);
                    tri(i0 + 1, i0 + w + 1, i0 + w);
                }
            }
        }
        if let Some(ink) = ink {
            self.emit_ink(soup, cam, ink);
        }
    }

    /// Projected outline of the sheet (clockwise), for an anti-aliased edge on near-flat sheets.
    pub fn outline(&self, cam: &Cam, per_side: usize) -> Vec<Pos2> {
        let n = per_side.max(1);
        let mut out = Vec::with_capacity(n * 4);
        let view = cam.viewer();
        let mut push = |u: f32, v: f32| out.push(cam.project(view((self.deform)(u, v))));
        for i in 0..n {
            push(i as f32 / n as f32, 0.0);
        }
        for i in 0..n {
            push(1.0, i as f32 / n as f32);
        }
        for i in 0..n {
            push(1.0 - i as f32 / n as f32, 1.0);
        }
        for i in 0..n {
            push(0.0, 1.0 - i as f32 / n as f32);
        }
        out
    }

    /// Map ink (laid out in local points `0..size`) onto the deformed front face.
    fn emit_ink(&self, soup: &mut Soup, cam: &Cam, ink: &Mesh) {
        let a = self.alpha.clamp(0.0, 1.0);
        let view = cam.viewer();
        let map = |p: Pos2| {
            let u = (p.x / self.size.x).clamp(0.0, 1.0);
            let v = (p.y / self.size.y).clamp(0.0, 1.0);
            view((self.deform)(u, v))
        };
        let mut cache: Vec<(V3, Pos2)> = Vec::with_capacity(ink.vertices.len());
        for vtx in &ink.vertices {
            let p = map(vtx.pos);
            cache.push((p, cam.project(p)));
        }
        for t in ink.indices.chunks_exact(3) {
            let (ia, ib, ic) = (t[0] as usize, t[1] as usize, t[2] as usize);
            let (la, lb, lc) = (ink.vertices[ia].pos, ink.vertices[ib].pos, ink.vertices[ic].pos);
            let local_area = (lb - la).x * (lc - la).y - (lb - la).y * (lc - la).x;
            if local_area.abs() < 1e-6 {
                continue;
            }
            let (pa, pb, pc) = (cache[ia].0, cache[ib].0, cache[ic].0);
            let mut n = (pb - pa).cross(pc - pa).norm();
            if local_area < 0.0 {
                n = n * -1.0;
            }
            if n.z <= 0.0 {
                continue; // facing away: this side of the paper is hidden
            }
            let k = shade(n, self.ambient, self.gloss);
            let z = (pa.z + pb.z + pc.z) / 3.0 + self.bias + self.ink_bias;
            let vx = |i: usize| {
                let src = ink.vertices[i];
                Vertex { pos: cache[i].1, uv: src.uv, color: lit(src.color, k).gamma_multiply(a) }
            };
            soup.push(z, [vx(ia), vx(ib), vx(ic)]);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Ink: record shapes off-screen, tessellate, subdivide, then print them onto a sheet

/// Records shapes in local coordinates (not painted) so they can be mapped onto a deformed sheet.
pub(crate) struct Ink {
    pub painter: Painter,
    pub shapes: Vec<Shape>,
}

impl Ink {
    pub fn new(painter: &Painter) -> Self {
        Self { painter: painter.clone(), shapes: Vec::new() }
    }
    pub fn galley(&self, text: &str, font: FontId, color: Color32) -> Arc<egui::Galley> {
        self.painter.layout_no_wrap(text.to_owned(), font, color)
    }
    pub fn text(&mut self, pos: Pos2, anchor: Align2, text: &str, font: FontId, color: Color32) -> Rect {
        let g = self.galley(text, font, color);
        let r = anchor.anchor_size(pos, g.size());
        self.shapes.push(Shape::galley(r.min, g, color));
        r
    }
    pub fn rect(&mut self, r: Rect, radius: f32, c: Color32) {
        self.shapes.push(Shape::rect_filled(r, radius, c));
    }
    pub fn circle(&mut self, c: Pos2, r: f32, col: Color32) {
        self.shapes.push(Shape::circle_filled(c, r, col));
    }
    pub fn ring(&mut self, c: Pos2, r: f32, w: f32, col: Color32) {
        self.shapes.push(Shape::circle_stroke(c, r, Stroke::new(w, col)));
    }
    pub fn line(&mut self, pts: Vec<Pos2>, w: f32, c: Color32) {
        self.shapes.push(Shape::line(pts, Stroke::new(w, c)));
    }
    pub fn poly(&mut self, pts: Vec<Pos2>, c: Color32) {
        self.shapes.push(Shape::convex_polygon(pts, c, Stroke::NONE));
    }
    /// Tessellate everything into one mesh (font texture), subdividing triangles longer than
    /// `max_edge` so they bend with the sheet.
    pub fn mesh(self, ctx: &Context, max_edge: f32) -> Mesh {
        subdivide(&self.tessellate(ctx), max_edge)
    }
    /// Tessellate everything into one mesh, without subdividing.
    pub fn tessellate(self, ctx: &Context) -> Mesh {
        let mut tess = tessellator(ctx);
        let mut out = Mesh::default();
        for s in self.shapes {
            let mut m = Mesh::default();
            tess.tessellate_shape(s, &mut m);
            out.append(m);
        }
        out
    }
}

fn tessellator(ctx: &Context) -> Tessellator {
    let ppp = ctx.pixels_per_point();
    let opts = ctx.tessellation_options(|o| *o);
    let font_size = ctx.fonts(|f| f.font_image_size());
    Tessellator::new(ppp, opts, font_size, Vec::new())
}

fn mid(a: &Vertex, b: &Vertex) -> Vertex {
    Vertex { pos: a.pos.lerp(b.pos, 0.5), uv: a.uv.lerp(b.uv, 0.5), color: lerp_c(a.color, b.color, 0.5) }
}

/// Split triangles until no edge is longer than `max_edge` points.
///
/// The output is indexed: vertices shared in the input stay shared and an edge split from both
/// sides gets a single midpoint, so per-vertex work on the result (mapping onto a deformed sheet)
/// runs once per point instead of once per triangle corner.
pub(crate) fn subdivide(m: &Mesh, max_edge: f32) -> Mesh {
    let mut out = Mesh::with_texture(m.texture_id);
    out.vertices.extend_from_slice(&m.vertices);
    out.indices.reserve(m.indices.len());
    // edge midpoints, shared by the triangles on both sides of an edge
    let mut mids: std::collections::HashMap<u64, u32> = std::collections::HashMap::new();
    let mut at = |out: &mut Mesh, x: u32, y: u32| -> u32 {
        let (a, b) = (x.min(y), x.max(y));
        *mids.entry(((a as u64) << 32) | b as u64).or_insert_with(|| {
            let v = mid(&out.vertices[a as usize], &out.vertices[b as usize]);
            out.vertices.push(v);
            out.vertices.len() as u32 - 1
        })
    };
    let mut stack: Vec<[u32; 3]> = Vec::new();
    let mut strip: Vec<(u32, u32, f32, f32, u32)> = Vec::new();
    let max2 = max_edge * max_edge;
    for t in m.indices.chunks_exact(3) {
        stack.push([t[0], t[1], t[2]]);
        while let Some([a, b, c]) = stack.pop() {
            let (pa, pb, pc) =
                (out.vertices[a as usize].pos, out.vertices[b as usize].pos, out.vertices[c as usize].pos);
            let l = [(pa - pb).length_sq(), (pb - pc).length_sq(), (pc - pa).length_sq()];
            let lm = l[0].max(l[1]).max(l[2]);
            if lm <= max2 || stack.len() > 4000 {
                out.indices.extend_from_slice(&[a, b, c]);
                continue;
            }
            // Slivers (thin strokes, anti-aliasing feathers): bisection would fan out into
            // needles, so cut them into a strip along their length instead.
            let ls = l[0].min(l[1]).min(l[2]);
            let ab = pb - pa;
            let ac = pc - pa;
            let height = (ab.x * ac.y - ab.y * ac.x).abs() / lm.sqrt();
            if ls <= max2 && height < max_edge * 0.5 {
                let [x, y, z] = if ls == l[1] {
                    [a, b, c]
                } else if ls == l[2] {
                    [b, c, a]
                } else {
                    [c, a, b]
                };
                // Halve each long side (shared midpoints) while its pieces are too long: the
                // same rule a neighbor's bisection applies to that edge, so both sides of every
                // edge get the same points (no T-junctions, which crack once deformed). Then zip
                // the two chains together.
                let mut chains: [Vec<(u32, f32)>; 2] = [vec![(x, 0.0)], vec![(x, 0.0)]];
                for (k, end) in [y, z].into_iter().enumerate() {
                    strip.clear();
                    strip.push((x, end, 0.0, 1.0, 0));
                    while let Some((p0, p1, t0, t1, depth)) = strip.pop() {
                        let len2 = out.vertices[p0 as usize].pos.distance_sq(out.vertices[p1 as usize].pos);
                        if depth < 12 && len2 > max2 {
                            let pm = at(&mut out, p0, p1);
                            let tm = (t0 + t1) * 0.5;
                            strip.push((pm, p1, tm, t1, depth + 1));
                            strip.push((p0, pm, t0, tm, depth + 1));
                        } else {
                            chains[k].push((p1, t1));
                        }
                    }
                }
                let [p, q] = &chains;
                let (mut i, mut j) = (0, 0);
                while i + 1 < p.len() || j + 1 < q.len() {
                    let step_p = j + 1 >= q.len() || (i + 1 < p.len() && p[i + 1].1 <= q[j + 1].1);
                    let tri = if step_p {
                        i += 1;
                        [p[i - 1].0, p[i].0, q[j].0]
                    } else {
                        j += 1;
                        [p[i].0, q[j].0, q[j - 1].0]
                    };
                    if tri[0] != tri[1] && tri[1] != tri[2] && tri[0] != tri[2] {
                        out.indices.extend_from_slice(&tri);
                    }
                }
                continue;
            }
            let (x, y) = if lm == l[0] {
                (a, b)
            } else if lm == l[1] {
                (b, c)
            } else {
                (c, a)
            };
            let mi = at(&mut out, x, y);
            if lm == l[0] {
                stack.push([a, mi, c]);
                stack.push([mi, b, c]);
            } else if lm == l[1] {
                stack.push([a, b, mi]);
                stack.push([a, mi, c]);
            } else {
                stack.push([a, b, mi]);
                stack.push([mi, b, c]);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Crumpled paper

/// A deterministic crumple field: a set of random fold lines that give paper its facets.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Crumple {
    folds: [(Vec2, f32, f32, f32); 9],
    tumble: (f32, f32, f32),
}

impl Crumple {
    pub fn new(seed: u64) -> Self {
        let folds = std::array::from_fn(|k| {
            let a = h01(seed * 131 + k as u64 * 7) * PI * 2.0;
            let f = 1.4 + h01(seed * 17 + k as u64 * 3 + 1) * 3.6;
            let ph = h01(seed * 29 + k as u64 * 11 + 2);
            let amp = 1.0 / (1.0 + k as f32 * 0.35);
            (vec2(a.cos(), a.sin()), f, ph, amp)
        });
        let tumble =
            ((h01(seed * 5 + 91) - 0.5) * 2.6, (h01(seed * 7 + 37) - 0.5) * 3.2, (h01(seed * 3 + 13) - 0.5) * 2.0);
        Self { folds, tumble }
    }

    /// Crease height in roughly `-1..1`.
    pub fn crease(&self, u: f32, v: f32, octave: f32) -> f32 {
        let mut h = 0.0;
        for (d, f, ph, amp) in &self.folds {
            h += tri_wave((d.x * u + d.y * v) * f * octave + ph) * amp;
        }
        h * 0.7
    }

    /// The sheet of `size` crumpled by `c` (0 flat with faint creases left over, 1 a tight
    /// ball), with the per-sheet terms (tumble rotation, easing) worked out once. `residual`
    /// scales the creases of the flat state.
    pub fn pose(&self, size: Vec2, c: f32, residual: f32) -> CrumplePose<'_> {
        let c = c.clamp(0.0, 1.0);
        let (a, b, g) = self.tumble;
        CrumplePose {
            crumple: self,
            size,
            c,
            residual,
            dim: (size.x + size.y) * 0.5,
            tumble: Rot::new(a * c, b * c, g * c),
            ease: c * c * (3.0 - 2.0 * c),
            wobble: (PI * c).sin(),
        }
    }
}

/// A [`Crumple`] at a fixed size and crumple amount (see [`Crumple::pose`]).
pub(crate) struct CrumplePose<'a> {
    crumple: &'a Crumple,
    size: Vec2,
    c: f32,
    residual: f32,
    dim: f32,
    tumble: Rot,
    ease: f32,
    wobble: f32,
}

impl CrumplePose<'_> {
    /// Position of `(u, v)`.
    pub fn at(&self, u: f32, v: f32) -> V3 {
        let (size, dim) = (self.size, self.dim);
        let h = self.crumple.crease(u, v, 1.0);
        let flat = v3((u - 0.5) * size.x, (v - 0.5) * size.y, h * dim * 0.006 * self.residual);
        if self.c <= 1e-4 {
            return flat;
        }
        // wrap the sheet round a ball: centre at the front pole, the edges tucked behind
        let d = vec2((u - 0.5) * 2.0, (v - 0.5) * 2.0);
        let r = (d.length() / std::f32::consts::SQRT_2).min(1.0);
        let dir = if d.length() > 1e-4 { d.normalized() } else { vec2(1.0, 0.0) };
        let th = r * PI * 0.94;
        let h2 = self.crumple.crease(u, v, 2.3);
        let radius = dim * 0.2 * (1.0 + 0.3 * h + 0.16 * h2);
        let ball = v3(th.sin() * dir.x, th.sin() * dir.y, th.cos()) * radius;
        let ball = self.tumble.apply(ball);
        // ease the path so the paper bunches up, not just shrinks
        let mut p = flat.lerp(ball, self.ease);
        let wobble = self.wobble;
        p.z += (h * 0.5 + h2 * 0.5) * dim * 0.09 * wobble;
        p.x += h2 * dim * 0.03 * wobble;
        p.y += h * dim * 0.03 * wobble;
        p
    }
}

/// Paper colors that read on both light and dark desks.
pub(crate) const PAPER: Color32 = Color32::from_rgb(246, 243, 236);
pub(crate) const PAPER_BACK: Color32 = Color32::from_rgb(226, 222, 212);

// ---------------------------------------------------------------------------------------------
// Scroll + autoplay helpers

/// For a tall "sticky" track `rect`: progress 0 when its top reaches the viewport top, 1 when
/// its bottom reaches the viewport bottom, plus the pinned stage rect (viewport-sized, clamped
/// inside the track).
pub(crate) fn sticky(ui: &Ui, rect: Rect, stage_h: f32) -> (f32, Rect) {
    let clip = ui.clip_rect();
    let travel = (rect.height() - stage_h).max(1.0);
    let top = (clip.min.y).clamp(rect.min.y, rect.max.y - stage_h);
    let p = ((clip.min.y - rect.min.y) / travel).clamp(0.0, 1.0);
    (p, Rect::from_min_size(pos2(rect.min.x, top), vec2(rect.width(), stage_h)))
}

/// Seconds since the pointer last interacted with `rect` (hover counts).
pub(crate) fn idle_time(ui: &Ui, id: Id, rect: Rect) -> f32 {
    let now = glowkit_core::time(ui);
    let active = ui.rect_contains_pointer(rect) && ui.input(|i| i.pointer.is_moving() || i.pointer.any_down());
    let last: f32 = glowkit_core::state(ui, id.with("snap_idle"), || now - 100.0);
    if active {
        glowkit_core::set_state(ui, id.with("snap_idle"), now);
        0.0
    } else {
        (now - last).max(0.0)
    }
}

/// Real frame time (for autoplay timers).
pub(crate) fn wall_dt(ui: &Ui) -> f32 {
    ui.input(|i| i.unstable_dt).clamp(0.0, 0.1)
}

/// A small arrow cursor used to demonstrate gestures while idle.
pub(crate) fn ghost_cursor(painter: &Painter, dark: bool, tip: Pos2, alpha: f32, pressed: bool) {
    if alpha <= 0.01 {
        return;
    }
    let s = if pressed { 0.88 } else { 1.0 };
    let pts: Vec<Pos2> = [(0.0, 0.0), (0.0, 16.0), (4.2, 12.3), (7.0, 18.2), (9.6, 17.0), (6.9, 11.3), (12.0, 11.3)]
        .iter()
        .map(|(x, y)| tip + vec2(*x, *y) * s)
        .collect();
    let (fill, line) = if dark { (Color32::WHITE, Color32::BLACK) } else { (Color32::BLACK, Color32::WHITE) };
    let c = tip + vec2(3.5, 9.0) * s;
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        painter.add(Shape::convex_polygon(vec![c, a, b], with_alpha(fill, alpha), Stroke::NONE));
    }
    let mut closed = pts.clone();
    closed.push(pts[0]);
    painter.add(Shape::line(closed, Stroke::new(1.2, with_alpha(line, alpha))));
    if pressed {
        painter.circle_stroke(tip, 9.0, Stroke::new(1.5, with_alpha(fill, alpha * 0.35)));
    }
}

/// A soft contact shadow: an ellipse whose darkness falls off smoothly to nothing.
pub(crate) fn contact_shadow(painter: &Painter, center: Pos2, radii: Vec2, strength: f32) {
    let rings = 8;
    let seg = 40;
    let mut m = Mesh::default();
    let a0 = (strength.clamp(0.0, 1.0) * 255.0) as u8;
    m.colored_vertex(center, Color32::from_black_alpha(a0));
    for k in 1..=rings {
        let s = k as f32 / rings as f32;
        let a = (1.0 - s * s).powf(1.6) * strength.clamp(0.0, 1.0);
        let c = Color32::from_black_alpha((a * 255.0) as u8);
        for i in 0..seg {
            let ang = i as f32 / seg as f32 * PI * 2.0;
            m.colored_vertex(center + vec2(ang.cos() * radii.x * s, ang.sin() * radii.y * s), c);
        }
    }
    for i in 0..seg as u32 {
        m.add_triangle(0, 1 + i, 1 + (i + 1) % seg as u32);
    }
    for k in 1..rings as u32 {
        let a0 = 1 + (k - 1) * seg as u32;
        let b0 = 1 + k * seg as u32;
        for i in 0..seg as u32 {
            let j = (i + 1) % seg as u32;
            m.add_triangle(a0 + i, b0 + i, b0 + j);
            m.add_triangle(a0 + i, b0 + j, a0 + j);
        }
    }
    painter.add(Shape::mesh(m));
}

/// Fill a closed polygon (convex or star-shaped around `center`) as a triangle fan.
pub(crate) fn fan(painter: &Painter, center: Pos2, pts: &[Pos2], c: Color32) {
    let mut m = Mesh::default();
    m.colored_vertex(center, c);
    for p in pts {
        m.colored_vertex(*p, c);
    }
    let n = pts.len() as u32;
    for i in 0..n {
        m.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(Shape::mesh(m));
}

/// Marching-squares fill of `field(p) > 0` over `rect` with cell size `cell`; the color of each
/// vertex comes from `color(p)`. Returns the contour segments (for anti-aliased outlines).
pub(crate) fn iso_fill(
    painter: &Painter,
    rect: Rect,
    cell: f32,
    field: impl Fn(Pos2) -> f32,
    color: impl Fn(Pos2) -> Color32,
) -> Vec<(Pos2, Pos2)> {
    let nx = ((rect.width() / cell).ceil() as usize).max(1);
    let ny = ((rect.height() / cell).ceil() as usize).max(1);
    let w = nx + 1;
    let mut vals = Vec::with_capacity(w * (ny + 1));
    for j in 0..=ny {
        for i in 0..=nx {
            vals.push(field(pos2(rect.min.x + i as f32 * cell, rect.min.y + j as f32 * cell)));
        }
    }
    let mut m = Mesh::default();
    let mut segs = Vec::new();
    let mut poly: Vec<Pos2> = Vec::with_capacity(8);
    for j in 0..ny {
        for i in 0..nx {
            let p = [
                pos2(rect.min.x + i as f32 * cell, rect.min.y + j as f32 * cell),
                pos2(rect.min.x + (i + 1) as f32 * cell, rect.min.y + j as f32 * cell),
                pos2(rect.min.x + (i + 1) as f32 * cell, rect.min.y + (j + 1) as f32 * cell),
                pos2(rect.min.x + i as f32 * cell, rect.min.y + (j + 1) as f32 * cell),
            ];
            let v = [vals[j * w + i], vals[j * w + i + 1], vals[(j + 1) * w + i + 1], vals[(j + 1) * w + i]];
            let inside = v.iter().filter(|x| **x > 0.0).count();
            if inside == 0 {
                continue;
            }
            poly.clear();
            let mut cross: Vec<Pos2> = Vec::with_capacity(4);
            for k in 0..4 {
                let k2 = (k + 1) % 4;
                if v[k] > 0.0 {
                    poly.push(p[k]);
                }
                if (v[k] > 0.0) != (v[k2] > 0.0) {
                    let t = v[k] / (v[k] - v[k2]);
                    let q = p[k].lerp(p[k2], t);
                    poly.push(q);
                    cross.push(q);
                }
            }
            if inside < 4 && cross.len() == 2 {
                segs.push((cross[0], cross[1]));
            } else if cross.len() == 4 {
                segs.push((cross[0], cross[1]));
                segs.push((cross[2], cross[3]));
            }
            let base = m.vertices.len() as u32;
            for q in &poly {
                m.colored_vertex(*q, color(*q));
            }
            for k in 1..poly.len().saturating_sub(1) as u32 {
                m.add_triangle(base, base + k, base + k + 1);
            }
        }
    }
    painter.add(Shape::mesh(m));
    segs
}

/// Smooth minimum (polynomial) used to melt shapes together.
#[inline]
pub(crate) fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = (0.5 + 0.5 * (b - a) / k.max(1e-4)).clamp(0.0, 1.0);
    b + (a - b) * h - k * h * (1.0 - h)
}

/// Signed distance to a rounded rectangle (negative inside).
#[inline]
pub(crate) fn sd_round_rect(p: Pos2, r: Rect, radius: f32) -> f32 {
    let c = r.center();
    let half = r.size() * 0.5 - Vec2::splat(radius);
    let d = vec2((p.x - c.x).abs() - half.x, (p.y - c.y).abs() - half.y);
    let outside = vec2(d.x.max(0.0), d.y.max(0.0)).length();
    outside + d.x.max(d.y).min(0.0) - radius
}

/// A procedural painted "photo" for paper sheets, sampled per vertex (`u, v in 0..1`).
/// `kind` picks the scene: 0 sunset sea, 1 mountain lake, 2 meadow, 3 night city.
pub(crate) fn photo(kind: usize, u: f32, v: f32) -> Color32 {
    use glowkit_core::color::gradient;
    use glowkit_core::hex;
    let n = glowkit_core::noise::noise2(u * 6.0, v * 6.0) * 0.5;
    match kind % 4 {
        0 => {
            let horizon = 0.58 + 0.02 * (u * 9.0).sin();
            if v < horizon {
                let sky = gradient(&[hex("#2B2D5C"), hex("#7A4C8F"), hex("#F08A6C"), hex("#FFD29A")], v / horizon);
                let sun = (1.0 - (vec2(u - 0.62, (v - horizon + 0.08) * 1.3).length() / 0.11)).clamp(0.0, 1.0);
                lerp_c(sky, hex("#FFF4D6"), sun * sun * 1.4)
            } else {
                let d = (v - horizon) / (1.0 - horizon);
                let sea = gradient(&[hex("#C9776E"), hex("#4A3F6B"), hex("#1C2440")], d);
                let glint = ((u - 0.62).abs() < 0.05 + d * 0.08) as i32 as f32 * (0.5 + n) * (1.0 - d);
                let rock = if u < 0.32 && v > 0.62 + (u * 14.0).sin() * 0.03 + u * 0.4 { 1.0 } else { 0.0 };
                lerp_c(lerp_c(sea, hex("#FFD7A8"), glint * 0.6), hex("#14121F"), rock * 0.9)
            }
        }
        1 => {
            let ridge = 0.42 + 0.12 * (u * 7.0).sin() * (u * 2.3).cos() + 0.04 * n;
            if v < ridge {
                gradient(&[hex("#9CC9E8"), hex("#DCEEF7")], v / ridge)
            } else if v < 0.66 {
                let snow = ((ridge + 0.06 - v) * 12.0).clamp(0.0, 1.0);
                lerp_c(gradient(&[hex("#4E6B78"), hex("#29404A")], (v - ridge) / (0.66 - ridge)), hex("#F4FAFF"), snow)
            } else {
                let d = (v - 0.66) / 0.34;
                lerp_c(hex("#3E7C8C"), hex("#1A3C47"), d + n * 0.3)
            }
        }
        2 => {
            let hill = 0.55 + 0.08 * (u * 5.0 + 1.0).sin();
            if v < hill {
                let sky = gradient(&[hex("#F6C9A8"), hex("#FBE7C6")], v / hill);
                let sun = (1.0 - (vec2(u - 0.3, v - 0.32).length() / 0.09)).clamp(0.0, 1.0);
                lerp_c(sky, Color32::WHITE, sun)
            } else {
                let d = (v - hill) / (1.0 - hill);
                let grass = gradient(&[hex("#9BBF5A"), hex("#5F8A3A"), hex("#3B5E2B")], d + n * 0.4);
                let flower = (glowkit_core::noise::noise2(u * 40.0, v * 40.0) > 0.55) as i32 as f32;
                lerp_c(grass, hex("#F5D04C"), flower * d)
            }
        }
        _ => {
            let sky = gradient(&[hex("#0B1026"), hex("#2B1E57"), hex("#7A2E6E")], v / 0.7);
            let col = ((u * 22.0).floor() * 7.31).sin() * 0.5 + 0.5;
            let top = 0.38 + col * 0.3;
            if v > top && v < 0.86 {
                let lit_w = glowkit_core::noise::noise2((u * 60.0).floor(), (v * 50.0).floor()) > 0.25;
                if lit_w {
                    hex("#FFC86B")
                } else {
                    hex("#151226")
                }
            } else if v >= 0.86 {
                lerp_c(hex("#2A1E45"), hex("#0B0A18"), (v - 0.86) / 0.14)
            } else {
                sky
            }
        }
    }
}

/// Ink tessellated and subdivided once and reused while it stays the same.
///
/// `draw` runs and is tessellated every frame (cheap: it only records shapes and text layout is
/// cached by egui); the costly subdivision is redone only when the tessellated ink changes in any
/// way, including glyph texture coordinates (egui repacks the font atlas when it fills up).
pub(crate) fn cached_ink(ui: &Ui, key: Id, max_edge: f32, draw: impl FnOnce(&mut Ink)) -> Arc<Mesh> {
    use std::hash::{Hash, Hasher};
    let mut ink = Ink::new(ui.painter());
    draw(&mut ink);
    let raw = ink.tessellate(ui.ctx());
    let mut acc: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |x: u64| acc = (acc ^ x).wrapping_mul(0x0000_0100_0000_01b3).rotate_left(17);
    for v in &raw.vertices {
        feed(((v.pos.x.to_bits() as u64) << 32) | v.pos.y.to_bits() as u64);
        feed(((v.uv.x.to_bits() as u64) << 32) | v.uv.y.to_bits() as u64);
        feed(u32::from_le_bytes(v.color.to_array()) as u64);
    }
    for i in &raw.indices {
        feed(*i as u64);
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (acc, raw.vertices.len(), raw.indices.len(), raw.texture_id, max_edge.to_bits()).hash(&mut h);
    let stamp = h.finish();
    let id = key.with("snap_ink");
    if let Some((k, m)) = ui.data(|d| d.get_temp::<(u64, Arc<Mesh>)>(id)) {
        if k == stamp {
            return m;
        }
    }
    let m = Arc::new(subdivide(&raw, max_edge));
    ui.data_mut(|d| d.insert_temp(id, (stamp, m.clone())));
    m
}

/// Ellipse outline, optionally rotated by `angle` radians.
pub(crate) fn ellipse_pts(c: Pos2, r: Vec2, angle: f32, n: usize) -> Vec<Pos2> {
    let (s, co) = angle.sin_cos();
    (0..n)
        .map(|i| {
            let a = i as f32 / n as f32 * PI * 2.0;
            let p = vec2(a.cos() * r.x, a.sin() * r.y);
            c + vec2(p.x * co - p.y * s, p.x * s + p.y * co)
        })
        .collect()
}

/// A soft "clay" blob: an ellipse shaded as if lit from the upper left.
pub(crate) fn clay(painter: &Painter, c: Pos2, r: Vec2, angle: f32, base: Color32) {
    let pts = ellipse_pts(c, r, angle, 44);
    let hi = glowkit_core::color::shade(base, 0.14);
    let lo = glowkit_core::color::shade(base, -0.16);
    let light = c + vec2(-r.x * 0.35, -r.y * 0.45);
    let stops = [hi, base, lo];
    glowkit_core::paint::fill_convex_fn(
        painter,
        &pts,
        5,
        glowkit_core::paint::radial_fn(light, r.x.max(r.y) * 1.7, &stops),
    );
}

/// A soft shadow under any convex quad: an opaque core fading to nothing over `blur` points.
pub(crate) fn soft_quad(painter: &Painter, q: [Pos2; 4], blur: f32, color: Color32) {
    let c = Pos2::new((q[0].x + q[1].x + q[2].x + q[3].x) / 4.0, (q[0].y + q[1].y + q[2].y + q[3].y) / 4.0);
    let mut m = Mesh::default();
    for p in q {
        let d = (p - c).normalized();
        m.colored_vertex(p - d * blur * 0.5, color);
    }
    for p in q {
        let d = (p - c).normalized();
        m.colored_vertex(p + d * blur, Color32::TRANSPARENT);
    }
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    for i in 0..4u32 {
        let j = (i + 1) % 4;
        m.add_triangle(i, 4 + i, 4 + j);
        m.add_triangle(i, 4 + j, j);
    }
    painter.add(Shape::mesh(m));
}

/// An anti-aliased polyline with a per-point width (a ribbon with feathered edges).
pub(crate) fn ribbon_line(painter: &Painter, pts: &[Pos2], widths: &[f32], color: Color32, closed: bool) {
    let n = pts.len();
    if n < 2 {
        return;
    }
    let mut m = Mesh::default();
    for i in 0..n {
        let prev = if closed { pts[(i + n - 1) % n] } else { pts[i.saturating_sub(1)] };
        let next = if closed { pts[(i + 1) % n] } else { pts[(i + 1).min(n - 1)] };
        let d = next - prev;
        let nrm = if d.length() > 1e-4 { vec2(-d.y, d.x).normalized() } else { vec2(0.0, 1.0) };
        let w = widths[i.min(widths.len() - 1)] * 0.5;
        let p = pts[i];
        m.colored_vertex(p + nrm * (w + 0.8), Color32::TRANSPARENT);
        m.colored_vertex(p + nrm * (w - 0.3).max(0.0), color);
        m.colored_vertex(p - nrm * (w - 0.3).max(0.0), color);
        m.colored_vertex(p - nrm * (w + 0.8), Color32::TRANSPARENT);
    }
    let segs = if closed { n } else { n - 1 };
    for i in 0..segs as u32 {
        let j = (i + 1) % n as u32;
        for l in 0..3u32 {
            let (a, b, c, d) = (i * 4 + l, i * 4 + l + 1, j * 4 + l, j * 4 + l + 1);
            m.add_triangle(a, b, d);
            m.add_triangle(a, d, c);
        }
    }
    painter.add(Shape::mesh(m));
}
