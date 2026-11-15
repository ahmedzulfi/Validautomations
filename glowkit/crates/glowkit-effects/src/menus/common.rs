//! Shared plumbing for the menus module: shape capture and mesh warping (used to fake
//! refraction, turbulence and peeling), a small perspective projector, the generated
//! avatar art and an autoplay pointer.

use egui::epaint::{Mesh, Tessellator, Vertex};
use egui::layers::ShapeIdx;
use egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Shape, Stroke, Ui, Vec2};
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::paint;

// ---------------------------------------------------------------------------------------------
// Capture + mesh warping

/// Run `paint` with a painter, then pull every shape it added back out of the paint list and
/// tessellate them into one mesh (in points). Nothing painted by `paint` is shown directly.
pub(crate) fn capture(ui: &Ui, paint: impl FnOnce(&Painter)) -> Mesh {
    let painter = ui.painter().clone();
    let layer = painter.layer_id();
    let ctx = ui.ctx().clone();
    let start = ctx.graphics_mut(|g| g.entry(layer).next_idx());
    paint(&painter);
    let end = ctx.graphics_mut(|g| g.entry(layer).next_idx());
    let shapes: Vec<Shape> = ctx.graphics_mut(|g| {
        let list = g.entry(layer);
        (start.0..end.0)
            .map(|i| {
                let mut s = Shape::Noop;
                list.mutate_shape(ShapeIdx(i), |cs| s = std::mem::replace(&mut cs.shape, Shape::Noop));
                s
            })
            .collect()
    });
    let ppp = ctx.pixels_per_point();
    let opts = ctx.tessellation_options(|o| *o);
    let tex = ctx.fonts(|f| f.font_image_size());
    let mut tess = Tessellator::new(ppp, opts, tex, Vec::new());
    let mut mesh = Mesh::default();
    for s in shapes {
        push_shape(&mut tess, s, &mut mesh);
    }
    mesh
}

fn push_shape(tess: &mut Tessellator, s: Shape, mesh: &mut Mesh) {
    match s {
        Shape::Noop | Shape::Callback(_) => {}
        Shape::Vec(v) => {
            for s in v {
                push_shape(tess, s, mesh);
            }
        }
        Shape::Mesh(m) if m.texture_id != egui::TextureId::default() => {}
        s => {
            let mut tmp = Mesh::default();
            tess.tessellate_shape(s, &mut tmp);
            if tmp.texture_id == mesh.texture_id || mesh.is_empty() {
                mesh.append(tmp);
            }
        }
    }
}

/// A hash of everything that defines a mesh's look (positions, texture coordinates, colors,
/// indices and texture). Used to cache meshes derived from a capture.
pub(crate) fn mesh_key(mesh: &Mesh) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut acc: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |x: u64| {
        acc = (acc ^ x).wrapping_mul(0x0000_0100_0000_01b3).rotate_left(17);
    };
    for v in &mesh.vertices {
        feed(((v.pos.x.to_bits() as u64) << 32) | v.pos.y.to_bits() as u64);
        feed(((v.uv.x.to_bits() as u64) << 32) | v.uv.y.to_bits() as u64);
        feed(u32::from_le_bytes(v.color.to_array()) as u64);
    }
    for i in &mesh.indices {
        feed(*i as u64);
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (acc, mesh.vertices.len(), mesh.indices.len(), mesh.texture_id).hash(&mut h);
    h.finish()
}

/// A key for text drawn into a cached mesh: changes when the glyphs' places in the font atlas
/// change (the atlas is rebuilt when it fills up), so cached texture coordinates never go stale.
pub(crate) fn text_key(ui: &Ui, texts: &[(&str, f32)]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    ui.ctx().pixels_per_point().to_bits().hash(&mut h);
    ui.ctx().fonts(|f| f.font_image_size()).hash(&mut h);
    for (text, size) in texts {
        text.hash(&mut h);
        size.to_bits().hash(&mut h);
        let g = ui.painter().layout_no_wrap((*text).to_owned(), egui::FontId::proportional(*size), Color32::WHITE);
        for row in &g.rows {
            for glyph in &row.row.glyphs {
                glyph.uv_rect.min.hash(&mut h);
                glyph.uv_rect.max.hash(&mut h);
            }
        }
    }
    h.finish()
}

/// `build()` cached across frames under `id`, rebuilt only when `key` changes.
pub(crate) fn cached_mesh(ui: &Ui, id: egui::Id, key: u64, build: impl FnOnce() -> Mesh) -> std::sync::Arc<Mesh> {
    type Slot = Option<(u64, std::sync::Arc<Mesh>)>;
    if let Some((k, m)) = glowkit_core::state::<Slot>(ui, id, || None) {
        if k == key {
            return m;
        }
    }
    let m = std::sync::Arc::new(build());
    glowkit_core::set_state::<Slot>(ui, id, Some((key, m.clone())));
    m
}

#[inline]
fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_premultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

#[inline]
fn lerp_vertex(a: &Vertex, b: &Vertex, t: f32) -> Vertex {
    Vertex { pos: a.pos.lerp(b.pos, t), uv: a.uv.lerp(b.uv, t), color: lerp_color(a.color, b.color, t) }
}

/// Split every triangle (whose bounds touch `region`) until no edge is longer than `max_len`,
/// so the mesh can be bent smoothly. Triangles outside `region` are dropped.
pub(crate) fn subdivide(mesh: &Mesh, max_len: f32, region: Rect) -> Mesh {
    subdivide_except(mesh, max_len, region, Rect::NOTHING)
}

/// Like [`subdivide`], but triangles that lie entirely inside `keep` are kept as they are
/// (used where a warp only bends geometry near an edge, so the interior needs no extra detail).
///
/// The output is indexed: vertices shared in the input stay shared, and an edge split by two
/// neighboring triangles gets one shared midpoint, so the vertex count stays close to the
/// triangle count / 2 instead of 3 per triangle.
pub(crate) fn subdivide_except(mesh: &Mesh, max_len: f32, region: Rect, keep: Rect) -> Mesh {
    let mut out = Mesh::with_texture(mesh.texture_id);
    let mut remap = vec![u32::MAX; mesh.vertices.len()];
    let mut pts: EdgePoints = Default::default();
    let mut stack: Vec<[u32; 3]> = Vec::new();
    let max2 = max_len * max_len;
    let keep_on = keep.is_positive();
    for tri in mesh.indices.chunks_exact(3) {
        let p = [
            mesh.vertices[tri[0] as usize].pos,
            mesh.vertices[tri[1] as usize].pos,
            mesh.vertices[tri[2] as usize].pos,
        ];
        let bb = Rect::from_points(&p);
        if !bb.intersects(region) {
            continue;
        }
        let mut idx = [0u32; 3];
        for k in 0..3 {
            let i = tri[k] as usize;
            if remap[i] == u32::MAX {
                remap[i] = out.vertices.len() as u32;
                out.vertices.push(mesh.vertices[i]);
            }
            idx[k] = remap[i];
        }
        if keep_on && keep.contains_rect(bb) {
            out.indices.extend_from_slice(&idx);
            continue;
        }
        stack.push(idx);
        while let Some([a, b, c]) = stack.pop() {
            let (pa, pb, pc) =
                (out.vertices[a as usize].pos, out.vertices[b as usize].pos, out.vertices[c as usize].pos);
            let ab = pa.distance_sq(pb);
            let bc = pb.distance_sq(pc);
            let ca = pc.distance_sq(pa);
            let m = ab.max(bc).max(ca);
            if m <= max2 || out.vertices.len() > 60_000 {
                out.indices.extend_from_slice(&[a, b, c]);
                continue;
            }
            // Slivers (line cores, anti-aliasing feathers): bisecting the longest edge would fan
            // out into hundreds of needles, so cut them into a strip along their length instead.
            // The cuts are made by halving both long edges with the same shared midpoints that a
            // neighbor's bisection uses, so neighbors meet without T-junctions.
            let s = ab.min(bc).min(ca);
            let area2 = cross(pb - pa, pc - pa).abs();
            if s <= max2 && area2 / m.sqrt() < max_len * 0.5 {
                // rotate so the short edge is (y, z) and x is the far apex
                let [x, y, z] = if s == bc {
                    [a, b, c]
                } else if s == ca {
                    [b, c, a]
                } else {
                    [c, a, b]
                };
                let max_len2 = max2;
                strip(&mut out, &mut pts, x, y, z, |o, p0, p1| {
                    o.vertices[p0 as usize].pos.distance_sq(o.vertices[p1 as usize].pos) > max_len2
                });
                continue;
            }
            let (x, y) = if m == ab {
                (a, b)
            } else if m == bc {
                (b, c)
            } else {
                (c, a)
            };
            let mid = pts.at(&mut out, x, y, 1, 2);
            if m == ab {
                stack.push([a, mid, c]);
                stack.push([mid, b, c]);
            } else if m == bc {
                stack.push([a, b, mid]);
                stack.push([a, mid, c]);
            } else {
                stack.push([a, b, mid]);
                stack.push([mid, b, c]);
            }
        }
    }
    out
}

/// Subdivide `mesh` (triangles touching `region`) only where `warp` bends it: a triangle is
/// bisected (longest edge first) while one of its edges' warped midpoint strays more than `tol`
/// points from the warped chord, or while an edge is longer than `max_len` outside `flat` (a rect
/// where the warp is known to be affine).
///
/// Edges along a rim, and everything in affine zones, stay coarse. Output is indexed.
///
/// `warp` returns the warped point, its "slack" and any per-point data the caller wants back.
/// The slack is a 1-Lipschitz distance to the zone whose look matters (`<= 0` inside it, e.g. a
/// signed distance minus a margin): edges and triangles that provably stay outside that zone are
/// never split. Returns the mesh and, per output vertex, its warped point and data.
pub(crate) fn subdivide_warp<T: Copy>(
    mesh: &Mesh,
    region: Rect,
    flat: Rect,
    max_len: f32,
    tol: f32,
    warp: impl Fn(Pos2) -> (Pos2, f32, T),
) -> (Mesh, Vec<(Pos2, T)>) {
    const MIN_LEN: f32 = 2.0;
    type W<T> = (Pos2, f32, T);
    let mut out = Mesh::with_texture(mesh.texture_id);
    let mut wpos: Vec<W<T>> = Vec::with_capacity(mesh.vertices.len() * 2);
    let mut remap = vec![u32::MAX; mesh.vertices.len()];
    let mut pts: EdgePoints = Default::default();
    let mut memo: std::collections::HashMap<u64, (bool, Option<W<T>>), std::hash::BuildHasherDefault<EdgeHasher>> =
        Default::default();
    let mut stack: Vec<[u32; 3]> = Vec::new();
    let tol2 = tol * tol;
    let max2 = max_len * max_len;
    let flat_on = flat.is_positive();
    // does the edge a-b need a split? (also returns the warped midpoint when it was computed)
    let need = |a: Pos2, wa: &W<T>, b: Pos2, wb: &W<T>| -> (bool, Option<W<T>>) {
        let l2 = a.distance_sq(b);
        // too short to show a bend (before, or after the warp compresses it), or every point
        // of the edge is provably outside the zone that matters
        if l2 < MIN_LEN * MIN_LEN || wa.0.distance_sq(wb.0) < MIN_LEN * MIN_LEN || (wa.1 + wb.1 - l2.sqrt()) * 0.5 > 0.0
        {
            return (false, None);
        }
        if flat_on && flat.contains(a) && flat.contains(b) {
            return (false, None);
        }
        let mid = warp(a.lerp(b, 0.5));
        let split = l2 > max2 || mid.0.distance_sq(wa.0.lerp(wb.0, 0.5)) > tol2;
        (split, Some(mid))
    };
    let sync = |out: &Mesh, wpos: &mut Vec<W<T>>| {
        while wpos.len() < out.vertices.len() {
            wpos.push(warp(out.vertices[wpos.len()].pos));
        }
    };
    for tri in mesh.indices.chunks_exact(3) {
        let p = [
            mesh.vertices[tri[0] as usize].pos,
            mesh.vertices[tri[1] as usize].pos,
            mesh.vertices[tri[2] as usize].pos,
        ];
        let bb = Rect::from_points(&p);
        if !bb.intersects(region) {
            continue;
        }
        let mut idx = [0u32; 3];
        for k in 0..3 {
            let i = tri[k] as usize;
            if remap[i] == u32::MAX {
                remap[i] = out.vertices.len() as u32;
                out.vertices.push(mesh.vertices[i]);
            }
            idx[k] = remap[i];
        }
        sync(&out, &mut wpos);
        if flat_on && flat.contains_rect(bb) {
            out.indices.extend_from_slice(&idx);
            continue;
        }
        stack.push(idx);
        while let Some([a, b, c]) = stack.pop() {
            if out.vertices.len() > 60_000 {
                out.indices.extend_from_slice(&[a, b, c]);
                continue;
            }
            let (pa, pb, pc) =
                (out.vertices[a as usize].pos, out.vertices[b as usize].pos, out.vertices[c as usize].pos);
            let ab = pa.distance_sq(pb);
            let bc = pb.distance_sq(pc);
            let ca = pc.distance_sq(pa);
            let s = ab.min(bc).min(ca);
            let m = ab.max(bc).max(ca);
            let height = cross(pb - pa, pc - pa).abs() / m.sqrt().max(1e-6);
            if s < 16.0 && height < 3.0 && m > MIN_LEN * MIN_LEN {
                // Sliver (line core, anti-aliasing feather): bisection would fan out into
                // needles, so cut it as a strip along its length.
                let [x, y, z] = if s == bc {
                    [a, b, c]
                } else if s == ca {
                    [b, c, a]
                } else {
                    [c, a, b]
                };
                // halve both long sides in step (shared midpoints, so neighbors stay crack-free)
                // while the warp bends the longer side
                strip(&mut out, &mut pts, x, y, z, |o, s0, s1| {
                    sync(o, &mut wpos);
                    let (s0, s1) = (s0 as usize, s1 as usize);
                    need(o.vertices[s0].pos, &wpos[s0], o.vertices[s1].pos, &wpos[s1]).0
                });
                sync(&out, &mut wpos);
                continue;
            }
            // If any edge needs it, bisect the longest edge: longest-edge bisection keeps the
            // triangles well shaped and terminates, splitting a shorter edge would fan out.
            // cheap triangle-level exits before any per-edge work
            let (sa, sb, sc) = (wpos[a as usize].1, wpos[b as usize].1, wpos[c as usize].1);
            let in_flat = flat_on && flat.contains(pa) && flat.contains(pb) && flat.contains(pc);
            // a long triangle that may reach the zone is split even if its edges stay outside it
            // (it can cover the zone with its interior)
            let reach = sa.min(sb).min(sc) - m.sqrt() <= 0.0;
            if in_flat || !reach || m < MIN_LEN * MIN_LEN {
                out.indices.extend_from_slice(&[a, b, c]);
                continue;
            }
            let long = m > max2;
            let mut test = |x: u32, px: Pos2, y: u32, py: Pos2| -> bool {
                let key = ((x.min(y) as u64) << 32) | x.max(y) as u64;
                if let Some(r) = memo.get(&key) {
                    return r.0;
                }
                let r = need(px, &wpos[x as usize], py, &wpos[y as usize]);
                if r.1.is_some() {
                    memo.insert(key, r);
                }
                r.0
            };
            if !(long || test(a, pa, b, pb) || test(b, pb, c, pc) || test(c, pc, a, pa)) {
                out.indices.extend_from_slice(&[a, b, c]);
                continue;
            }
            let (x, y) = if m == ab {
                (a, b)
            } else if m == bc {
                (b, c)
            } else {
                (c, a)
            };
            let before = out.vertices.len();
            let mid = pts.at(&mut out, x, y, 1, 2);
            if out.vertices.len() > before {
                let key = ((x.min(y) as u64) << 32) | x.max(y) as u64;
                match memo.get(&key) {
                    Some((_, Some(w))) => wpos.push(*w),
                    _ => sync(&out, &mut wpos),
                }
            }
            if (x, y) == (a, b) {
                stack.push([a, mid, c]);
                stack.push([mid, b, c]);
            } else if (x, y) == (b, c) {
                stack.push([a, b, mid]);
                stack.push([a, mid, c]);
            } else {
                stack.push([a, b, mid]);
                stack.push([mid, b, c]);
            }
        }
    }
    sync(&out, &mut wpos);
    let w = wpos.into_iter().map(|(p, _, t)| (p, t)).collect();
    (out, w)
}

/// Cut the sliver `x, y, z` (short edge `y-z`) into a strip. Each long side is halved (with
/// the shared midpoints from `pts`) for as long as `split(out, p0, p1)` holds for a piece, the same
/// rule a neighbor's bisection applies to that edge, so both sides of every edge end up with the
/// same points and no T-junctions; the two chains are then zipped together.
fn strip(
    out: &mut Mesh,
    pts: &mut EdgePoints,
    x: u32,
    y: u32,
    z: u32,
    mut split: impl FnMut(&mut Mesh, u32, u32) -> bool,
) {
    let mut chains: [Vec<(u32, f32)>; 2] = [vec![(x, 0.0)], vec![(x, 0.0)]];
    for (k, end) in [y, z].into_iter().enumerate() {
        // depth-first, left half first, so points come out in order along the side
        let mut stack = vec![(x, end, 0.0f32, 1.0f32, 0u32)];
        while let Some((p0, p1, t0, t1, depth)) = stack.pop() {
            if depth < 12 && split(out, p0, p1) {
                let pm = pts.at(out, p0, p1, 1, 2);
                let tm = (t0 + t1) * 0.5;
                stack.push((pm, p1, tm, t1, depth + 1));
                stack.push((p0, pm, t0, tm, depth + 1));
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
}

/// Points inserted on mesh edges, shared between the triangles on both sides of an edge.
#[derive(Default)]
struct EdgePoints(std::collections::HashMap<u128, u32, std::hash::BuildHasherDefault<EdgeHasher>>);

impl EdgePoints {
    /// Index of the point `i / n` of the way from vertex `x` to vertex `y` (created on first use).
    fn at(&mut self, out: &mut Mesh, x: u32, y: u32, i: u32, n: u32) -> u32 {
        if i == 0 {
            return x;
        }
        if i >= n {
            return y;
        }
        let (a, b, i) = if x < y { (x, y, i) } else { (y, x, n - i) };
        let g = gcd(i, n);
        let (i, n) = (i / g, n / g);
        let key = a as u128 | (b as u128) << 32 | (i as u128) << 64 | (n as u128) << 96;
        *self.0.entry(key).or_insert_with(|| {
            let v = lerp_vertex(&out.vertices[a as usize], &out.vertices[b as usize], i as f32 / n as f32);
            out.vertices.push(v);
            out.vertices.len() as u32 - 1
        })
    }
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.max(1)
}

/// A fast hasher for packed `(u32, u32)` edge keys.
#[derive(Default)]
struct EdgeHasher(u64);

impl std::hash::Hasher for EdgeHasher {
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0 ^ *b as u64).wrapping_mul(0x0100_0000_01b3);
        }
    }
    fn write_u64(&mut self, n: u64) {
        self.0 = (self.0.rotate_left(23) ^ n).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
    fn write_u128(&mut self, n: u128) {
        self.write_u64(n as u64);
        self.write_u64((n >> 64) as u64);
    }
    fn finish(&self) -> u64 {
        self.0 ^ (self.0 >> 29)
    }
}

/// Clip a mesh to a **convex** polygon (any winding), interpolating uv and color on cut edges.
pub(crate) fn clip_mesh(mesh: &Mesh, poly: &[Pos2]) -> Mesh {
    ConvexClip::new(poly).clip(mesh, Vec2::ZERO)
}

/// A convex clip region prepared once and reused for several meshes.
///
/// Collinear outline points are merged, points are tested with an inscribed-rect quick accept and
/// an O(log n) wedge search, and only triangles that actually cross the boundary go through
/// Sutherland–Hodgman clipping.
pub(crate) struct ConvexClip {
    /// Counter-clockwise in the `cross >= 0 is inside` sense, no collinear or duplicate points.
    poly: Vec<Pos2>,
    bounds: Rect,
    /// An axis-aligned rect fully inside the polygon (quick accept).
    inner: Rect,
}

#[inline]
fn cross(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

impl ConvexClip {
    pub fn new(poly: &[Pos2]) -> Self {
        let mut pts: Vec<Pos2> = poly.to_vec();
        let mut area = 0.0;
        for i in 0..pts.len() {
            area += cross(pts[i].to_vec2(), pts[(i + 1) % pts.len()].to_vec2());
        }
        if area < 0.0 {
            pts.reverse();
        }
        // drop duplicates and (nearly) collinear points: straight edges are often pre-subdivided
        let mut changed = true;
        while changed && pts.len() > 3 {
            changed = false;
            let mut i = 0;
            while i < pts.len() && pts.len() > 3 {
                let n = pts.len();
                let a = pts[(i + n - 1) % n];
                let b = pts[i];
                let c = pts[(i + 1) % n];
                let ab = b - a;
                let bc = c - b;
                let len = ab.length() * bc.length();
                if ab.length_sq() < 1e-8 || bc.length_sq() < 1e-8 || cross(ab, bc).abs() <= len * 1e-4 {
                    pts.remove(i);
                    changed = true;
                } else {
                    i += 1;
                }
            }
        }
        let bounds = if pts.is_empty() { Rect::NOTHING } else { Rect::from_points(&pts) };
        let mut clip = Self { poly: pts, bounds, inner: Rect::NOTHING };
        if clip.poly.len() >= 3 {
            // largest uniformly scaled copy of the bounds (about the centroid) that fits inside
            let c = clip.poly.iter().fold(Vec2::ZERO, |acc, p| acc + p.to_vec2()) / clip.poly.len() as f32;
            let c = c.to_pos2();
            let half = bounds.size() / 2.0;
            let fits = |s: f32| {
                let r = Rect::from_center_size(c, half * 2.0 * s);
                [r.left_top(), r.right_top(), r.left_bottom(), r.right_bottom()].iter().all(|&q| clip.contains_slow(q))
            };
            let (mut lo, mut hi) = (0.0f32, 1.0f32);
            for _ in 0..14 {
                let mid = (lo + hi) / 2.0;
                if fits(mid) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            if lo > 0.0 {
                clip.inner = Rect::from_center_size(c, half * 2.0 * lo);
            }
        }
        clip
    }

    #[inline]
    fn side(&self, i: usize, p: Pos2) -> f32 {
        let a = self.poly[i];
        let b = self.poly[(i + 1) % self.poly.len()];
        cross(b - a, p - a)
    }

    fn contains_slow(&self, p: Pos2) -> bool {
        (0..self.poly.len()).all(|i| self.side(i, p) >= 0.0)
    }

    /// Point-in-polygon in O(1) for most points, O(log n) otherwise.
    #[inline]
    pub fn contains(&self, p: Pos2) -> bool {
        if self.inner.contains(p) {
            return true;
        }
        if !self.bounds.contains(p) {
            return false;
        }
        let poly = &self.poly;
        let n = poly.len();
        let p0 = poly[0];
        let d = p - p0;
        if cross(poly[1] - p0, d) < 0.0 || cross(poly[n - 1] - p0, d) > 0.0 {
            return false;
        }
        let (mut lo, mut hi) = (1, n - 1);
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if cross(poly[mid] - p0, d) >= 0.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        cross(poly[lo + 1] - poly[lo], p - poly[lo]) >= 0.0
    }

    /// Clip `mesh` (translated by `offset`) to the polygon.
    pub fn clip(&self, mesh: &Mesh, offset: Vec2) -> Mesh {
        self.clip_hinted(mesh, offset, None)
    }

    /// Like [`ConvexClip::clip`]; `inside[i]` (when given) promises that vertex `i`, translated,
    /// is inside the polygon, which skips its point test.
    pub fn clip_hinted(&self, mesh: &Mesh, offset: Vec2, inside: Option<&[bool]>) -> Mesh {
        let mut out = Mesh::with_texture(mesh.texture_id);
        self.clip_into(mesh, offset, inside, &mut out);
        out
    }

    /// Like [`ConvexClip::clip_hinted`], appending to `out` (which must share `mesh`'s texture).
    pub fn clip_into(&self, mesh: &Mesh, offset: Vec2, inside: Option<&[bool]>, out: &mut Mesh) {
        if self.poly.len() < 3 {
            return;
        }
        out.vertices.reserve(mesh.vertices.len());
        out.indices.reserve(mesh.indices.len());
        // per-vertex: 0 = unknown, 1 = outside, 2 = inside; inside vertices get an output index
        let mut state = match inside {
            Some(h) if h.len() == mesh.vertices.len() => h.iter().map(|&i| if i { 2u8 } else { 0 }).collect(),
            _ => vec![0u8; mesh.vertices.len()],
        };
        let mut remap = vec![u32::MAX; mesh.vertices.len()];
        let mut buf: Vec<Vertex> = Vec::with_capacity(12);
        let mut nxt: Vec<Vertex> = Vec::with_capacity(12);
        let n = self.poly.len();
        for tri in mesh.indices.chunks_exact(3) {
            let mut all_in = true;
            for &i in tri {
                let i = i as usize;
                if state[i] == 0 {
                    state[i] = if self.contains(mesh.vertices[i].pos + offset) { 2 } else { 1 };
                }
                all_in &= state[i] == 2;
            }
            if all_in {
                // convex: all three corners inside means the whole triangle is
                for &i in tri {
                    let i = i as usize;
                    if remap[i] == u32::MAX {
                        remap[i] = out.vertices.len() as u32;
                        let mut v = mesh.vertices[i];
                        v.pos += offset;
                        out.vertices.push(v);
                    }
                    out.indices.push(remap[i]);
                }
                continue;
            }
            let mut v =
                [mesh.vertices[tri[0] as usize], mesh.vertices[tri[1] as usize], mesh.vertices[tri[2] as usize]];
            for x in &mut v {
                x.pos += offset;
            }
            let (a, b, c) = (v[0].pos, v[1].pos, v[2].pos);
            if a.x.max(b.x).max(c.x) < self.bounds.min.x
                || a.x.min(b.x).min(c.x) > self.bounds.max.x
                || a.y.max(b.y).max(c.y) < self.bounds.min.y
                || a.y.min(b.y).min(c.y) > self.bounds.max.y
            {
                continue;
            }
            buf.clear();
            buf.extend_from_slice(&v);
            for e in 0..n {
                if buf.is_empty() {
                    break;
                }
                if buf.iter().all(|p| self.side(e, p.pos) >= 0.0) {
                    continue;
                }
                nxt.clear();
                for j in 0..buf.len() {
                    let p = buf[j];
                    let q = buf[(j + 1) % buf.len()];
                    let sp = self.side(e, p.pos);
                    let sq = self.side(e, q.pos);
                    if sp >= 0.0 {
                        nxt.push(p);
                    }
                    if (sp >= 0.0) != (sq >= 0.0) {
                        let t = sp / (sp - sq);
                        nxt.push(lerp_vertex(&p, &q, t.clamp(0.0, 1.0)));
                    }
                }
                std::mem::swap(&mut buf, &mut nxt);
            }
            if buf.len() >= 3 {
                let base = out.vertices.len() as u32;
                out.vertices.extend_from_slice(&buf);
                for k in 1..buf.len() as u32 - 1 {
                    out.add_triangle(base, base + k, base + k + 1);
                }
            }
        }
    }
}

/// Keep only the parts of `mesh` inside the axis-aligned `rect` (cheap: four edges).
pub(crate) fn clip_rect(mesh: &Mesh, rect: Rect) -> Mesh {
    ConvexClip::new(&[rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()])
        .clip(mesh, Vec2::ZERO)
}

/// Desaturate (and optionally darken) a premultiplied color: `amount` 1 = full grayscale.
#[inline]
pub(crate) fn desaturate(c: Color32, amount: f32, darken: f32) -> Color32 {
    let l = 0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32;
    let k = 1.0 - darken;
    let ch = |x: u8| ((x as f32 + (l - x as f32) * amount) * k).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_premultiplied(ch(c.r()), ch(c.g()), ch(c.b()), c.a())
}

// ---------------------------------------------------------------------------------------------
// Perspective

/// A rotation about the card center followed by a perspective divide.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Persp {
    pub center: Pos2,
    /// Rotation about the x axis (radians, positive brings the top edge closer).
    pub rx: f32,
    /// Rotation about the y axis (radians, positive pushes the right edge away).
    pub ry: f32,
    pub scale: f32,
    pub distance: f32,
}

impl Persp {
    pub fn new(center: Pos2) -> Self {
        Self { center, rx: 0.0, ry: 0.0, scale: 1.0, distance: 900.0 }
    }
    /// Project a point given relative to the center (unscaled).
    pub fn apply(&self, v: Vec2) -> Pos2 {
        let (sy, cy) = self.ry.sin_cos();
        let (sx, cx) = self.rx.sin_cos();
        let x1 = v.x * cy;
        let z1 = v.x * sy;
        let y2 = v.y * cx - z1 * sx;
        let z2 = v.y * sx + z1 * cx;
        let f = self.distance / (self.distance + z2);
        self.center + vec2(x1, y2) * f * self.scale
    }
    /// Project every vertex of a mesh whose positions are in screen space around `center`.
    pub fn warp(&self, mesh: &mut Mesh) {
        for v in &mut mesh.vertices {
            v.pos = self.apply(v.pos - self.center);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Avatar art

/// A generated "portrait": gradient backdrop, soft light, and a stylized head-and-shoulders
/// silhouette. Painted with ordinary shapes so it can be captured and recolored.
pub(crate) fn paint_avatar(
    painter: &Painter,
    rect: Rect,
    radius: f32,
    colors: [Color32; 2],
    seed: u32,
    backdrop: bool,
) {
    let [a, b] = colors;
    if backdrop {
        paint::gradient_rect(painter, rect, radius, 120.0, &[shade(a, 0.08), b]);
    }
    let w = rect.width();
    let h = rect.height();
    let cx = rect.center().x + ((seed % 3) as f32 - 1.0) * w * 0.04;
    if backdrop {
        // backlight
        paint::glow(painter, pos2(cx, rect.min.y + h * 0.38), w * 0.6, with_alpha(Color32::WHITE, 0.3));
    }
    const SKIN: [&str; 5] = ["#F1C27D", "#E0AC69", "#C68642", "#8D5524", "#FFDBAC"];
    const HAIR: [&str; 5] = ["#2B1B17", "#4A2C1D", "#1C1C28", "#A5682A", "#5A3218"];
    let skin = glowkit_core::hex(SKIN[(seed as usize * 2 + 1) % SKIN.len()]);
    let hair = glowkit_core::hex(HAIR[(seed as usize * 3) % HAIR.len()]);
    let shirt = mix(b, Color32::from_rgb(18, 16, 32), 0.5);
    let style = seed % 3;
    let r = w.min(h) * 0.16;
    let head = pos2(cx, rect.min.y + h * 0.44);
    if style == 1 {
        // long hair falls behind the shoulders
        painter.add(Shape::ellipse_filled(head + vec2(0.0, r * 0.6), vec2(r * 1.28, r * 1.7), hair));
    }
    // shoulders
    painter.add(Shape::ellipse_filled(pos2(cx, head.y + r * 1.55 + h * 0.3), vec2(w * 0.42, h * 0.32), shirt));
    // collar
    painter.add(Shape::convex_polygon(
        vec![pos2(cx - r * 0.55, head.y + r * 1.5), pos2(cx + r * 0.55, head.y + r * 1.5), pos2(cx, head.y + r * 2.2)],
        mix(shirt, a, 0.25),
        Stroke::NONE,
    ));
    // neck
    painter.rect_filled(
        Rect::from_min_max(pos2(cx - r * 0.42, head.y + r * 0.5), pos2(cx + r * 0.42, head.y + r * 1.75)),
        egui::CornerRadius::same((r * 0.3) as u8),
        shade(skin, -0.12),
    );
    if style != 1 {
        painter.circle_filled(head - vec2(0.0, r * 0.12), r * 1.1, hair);
    }
    painter.circle_filled(head, r, skin);
    // fringe
    match style {
        0 => {
            painter.add(Shape::ellipse_filled(head - vec2(r * 0.18, r * 0.6), vec2(r * 0.98, r * 0.48), hair));
        }
        1 => {
            painter.add(Shape::ellipse_filled(head - vec2(0.0, r * 0.68), vec2(r * 1.02, r * 0.42), hair));
        }
        _ => {
            painter.circle_filled(head - vec2(0.0, r * 1.3), r * 0.5, hair);
            painter.add(Shape::ellipse_filled(head - vec2(-r * 0.2, r * 0.66), vec2(r * 0.9, r * 0.4), hair));
        }
    }
    // soft rim light
    paint::arc(painter, head, r - 0.6, -2.7, -1.6, 1.4, with_alpha(Color32::WHITE, 0.3));
}

/// Pleasant gradient pairs for default item colors.
pub(crate) const VIVID: [[Color32; 2]; 8] = [
    [glowkit_core::hex("#4F46E5"), glowkit_core::hex("#06B6D4")],
    [glowkit_core::hex("#10B981"), glowkit_core::hex("#0F766E")],
    [glowkit_core::hex("#F59E0B"), glowkit_core::hex("#EF4444")],
    [glowkit_core::hex("#EC4899"), glowkit_core::hex("#8B5CF6")],
    [glowkit_core::hex("#8B5CF6"), glowkit_core::hex("#3B82F6")],
    [glowkit_core::hex("#06B6D4"), glowkit_core::hex("#10B981")],
    [glowkit_core::hex("#F43F5E"), glowkit_core::hex("#F97316")],
    [glowkit_core::hex("#6366F1"), glowkit_core::hex("#A855F7")],
];

// ---------------------------------------------------------------------------------------------
// Pointer

/// The real pointer if it is over `rect`, otherwise (when `autoplay`) a slow Lissajous path.
/// Returns `(point, real)`.
pub(crate) fn pointer_or_auto(ui: &Ui, rect: Rect, autoplay: bool, seed: f32) -> Option<(Pos2, bool)> {
    if let Some(p) = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p) && ui.rect_contains_pointer(rect))
    {
        return Some((p, true));
    }
    if autoplay {
        glowkit_core::animate(ui);
        let t = glowkit_core::time(ui);
        return Some((crate::cards::shell::ambient_point(rect, t, seed), false));
    }
    None
}

/// A closed, softly blurred ribbon: `width` is the solid core, `blur` the feather on each side.
pub(crate) fn soft_ribbon(
    painter: &Painter,
    pts: &[Pos2],
    closed: bool,
    mut f: impl FnMut(Pos2) -> (f32, f32, Color32),
) {
    let n = pts.len();
    if n < 2 {
        return;
    }
    let mut mesh = Mesh::default();
    let count = if closed { n + 1 } else { n };
    for k in 0..count {
        let i = k % n;
        let p = pts[i];
        let w = 3.min(n / 4).max(1);
        let prev = if closed { pts[(i + n - w) % n] } else { pts[i.saturating_sub(w)] };
        let next = if closed { pts[(i + w) % n] } else { pts[(i + w).min(n - 1)] };
        let mut d = next - prev;
        if d.length_sq() < 1e-6 {
            d = vec2(1.0, 0.0);
        }
        let nrm = d.normalized().rot90();
        let (w, blur, col) = f(p);
        let hw = w / 2.0;
        let feather = blur + 0.8;
        mesh.colored_vertex(p + nrm * (hw + feather), Color32::TRANSPARENT);
        mesh.colored_vertex(p + nrm * hw, col);
        mesh.colored_vertex(p - nrm * hw, col);
        mesh.colored_vertex(p - nrm * (hw + feather), Color32::TRANSPARENT);
    }
    for k in 0..count as u32 - 1 {
        let a = k * 4;
        let b = (k + 1) * 4;
        for s in 0..3 {
            mesh.add_triangle(a + s, b + s, b + s + 1);
            mesh.add_triangle(a + s, b + s + 1, a + s + 1);
        }
    }
    painter.add(Shape::mesh(mesh));
}

// ---------------------------------------------------------------------------------------------
// Open / closed toggles with autoplay

#[derive(Clone, Copy, Debug)]
struct ToggleState {
    born: f64,
    open: bool,
    at: f64,
    last_user: f64,
}

/// A persistent open/closed flag for menus. `clicked` flips it. With `autoplay`, while the
/// pointer is away (and the user has not touched it for a few seconds) it opens for
/// `open_for` seconds and closes for `closed_for`. Returns `(open, seconds since last change)`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn toggle(
    ui: &Ui,
    id: egui::Id,
    clicked: bool,
    force: Option<bool>,
    autoplay: bool,
    hovered: bool,
    open_for: f64,
    closed_for: f64,
) -> (bool, f32) {
    let now = ui.input(|i| i.time);
    let (open, since) = glowkit_core::with_state(
        ui,
        id.with("glowkit_toggle"),
        || ToggleState { born: now, open: false, at: -10.0, last_user: -100.0 },
        |s| {
            if clicked {
                s.open = !s.open;
                s.at = now;
                s.last_user = now;
            } else if let Some(f) = force {
                if f != s.open {
                    s.open = f;
                    s.at = now;
                    s.last_user = now;
                }
            } else if autoplay && !hovered && now - s.last_user > 4.0 {
                let want = (now - s.born + 0.3).rem_euclid(open_for + closed_for) < open_for;
                if want != s.open {
                    s.open = want;
                    s.at = now;
                }
            }
            (s.open, (now - s.at) as f32)
        },
    );
    if autoplay || since < 2.5 {
        glowkit_core::animate(ui);
    }
    (open, since)
}

/// Text with a faux-bold weight (two passes offset by a fraction of a point).
pub(crate) fn bold_text(
    painter: &Painter,
    pos: Pos2,
    align: egui::Align2,
    text: &str,
    font: egui::FontId,
    color: Color32,
) -> Rect {
    let w = (font.size / 28.0).clamp(0.35, 1.0);
    painter.text(pos + vec2(w, 0.0), align, text, font.clone(), color);
    painter.text(pos, align, text, font, color)
}

/// Hamburger icon that morphs into an "x" as `open` goes 0 → 1. `lines` is 2 or 3.
pub(crate) fn burger(painter: &Painter, center: Pos2, width: f32, open: f32, lines: usize, color: Color32) {
    let half = width / 2.0;
    let gap = width * 0.3;
    let stroke = Stroke::new((width * 0.1).max(1.6), color);
    let o = open.clamp(0.0, 1.2);
    let ang = std::f32::consts::FRAC_PI_4 * o;
    let draw = |y: f32, a: f32, scale: f32| {
        let c = center + vec2(0.0, y);
        let d = vec2(a.cos(), a.sin()) * half * scale;
        painter.line_segment([c - d, c + d], stroke);
    };
    if lines >= 3 {
        draw(-gap * (1.0 - o.min(1.0)), ang, 1.0);
        if o < 0.98 {
            painter.line_segment(
                [center - vec2(half * (1.0 - o), 0.0), center + vec2(half * (1.0 - o), 0.0)],
                Stroke::new(stroke.width, with_alpha(color, 1.0 - o.min(1.0))),
            );
        }
        draw(gap * (1.0 - o.min(1.0)), -ang, 1.0);
    } else {
        draw(-gap * 0.5 * (1.0 - o.min(1.0)), ang, 1.0);
        draw(gap * 0.5 * (1.0 - o.min(1.0)), -ang, 1.0);
    }
}
