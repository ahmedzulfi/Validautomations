//! Triangle meshes: geometry container, procedural primitives, an OBJ loader and a few built-in
//! models.

use crate::math::{vec3, Mat4, Vec3};
use egui::Color32;
use std::f32::consts::{PI, TAU};

/// An indexed triangle mesh with per-vertex normals and optional per-vertex colors.
///
/// Faces are counter-clockwise when seen from outside. `edges` lists the "visual" edges used by
/// wireframe rendering (quad outlines rather than every triangle diagonal where possible).
///
/// ```no_run
/// use glowkit_3d::Mesh;
/// let knot = Mesh::torus_knot(2, 3, 1.0, 0.28, 160, 14);
/// let sphere = Mesh::uv_sphere(1.0, 32, 16);
/// let cube = Mesh::cube(1.0).with_color(egui::Color32::LIGHT_BLUE);
/// ```
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    /// Per-vertex colors, multiplied with the material color. Empty means "all white".
    pub colors: Vec<Color32>,
    pub indices: Vec<[u32; 3]>,
    /// Wireframe edges (vertex index pairs).
    pub edges: Vec<[u32; 2]>,
}

impl Mesh {
    /// An empty mesh.
    pub fn new() -> Self {
        Self::default()
    }
    /// Number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }
    /// Number of triangles.
    pub fn triangle_count(&self) -> usize {
        self.indices.len()
    }
    /// Add a vertex, returning its index.
    pub fn vertex(&mut self, p: Vec3, n: Vec3) -> u32 {
        self.positions.push(p);
        self.normals.push(n);
        if !self.colors.is_empty() {
            self.colors.push(Color32::WHITE);
        }
        (self.positions.len() - 1) as u32
    }
    /// Add a colored vertex, returning its index.
    pub fn colored_vertex(&mut self, p: Vec3, n: Vec3, c: Color32) -> u32 {
        if self.colors.len() < self.positions.len() {
            self.colors.resize(self.positions.len(), Color32::WHITE);
        }
        self.positions.push(p);
        self.normals.push(n);
        self.colors.push(c);
        (self.positions.len() - 1) as u32
    }
    /// Add a triangle (and its three edges).
    pub fn triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.push([a, b, c]);
        self.edges.extend([[a, b], [b, c], [c, a]]);
    }
    /// Add a quad `a b c d` (counter-clockwise) as two triangles and four edges.
    pub fn quad(&mut self, a: u32, b: u32, c: u32, d: u32) {
        self.indices.push([a, b, c]);
        self.indices.push([a, c, d]);
        self.edges.extend([[a, b], [b, c], [c, d], [d, a]]);
    }
    /// Bounding box `(min, max)`.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for p in &self.positions {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        if self.positions.is_empty() {
            (Vec3::ZERO, Vec3::ZERO)
        } else {
            (lo, hi)
        }
    }
    /// Radius of the bounding sphere around the origin.
    pub fn radius(&self) -> f32 {
        self.positions.iter().map(|p| p.length()).fold(0.0, f32::max)
    }
    /// Recenter on the origin and scale so the bounding sphere has radius `r`.
    pub fn fit(mut self, r: f32) -> Self {
        let (lo, hi) = self.bounds();
        let c = (lo + hi) * 0.5;
        for p in &mut self.positions {
            *p -= c;
        }
        let k = r / self.radius().max(1e-6);
        for p in &mut self.positions {
            *p *= k;
        }
        self
    }
    /// Set every vertex color.
    pub fn with_color(mut self, c: Color32) -> Self {
        self.colors = vec![c; self.positions.len()];
        self
    }
    /// Color every vertex with `f(position, normal)`.
    pub fn paint(mut self, f: impl Fn(Vec3, Vec3) -> Color32) -> Self {
        self.colors = self.positions.iter().zip(&self.normals).map(|(p, n)| f(*p, *n)).collect();
        self
    }
    /// Apply a matrix to positions and normals.
    pub fn transformed(mut self, m: &Mat4) -> Self {
        let nm = m.normal_matrix();
        for p in &mut self.positions {
            *p = m.transform_point(*p);
        }
        for n in &mut self.normals {
            *n = nm.transform_vector(*n).normalized();
        }
        self
    }
    /// Append another mesh.
    pub fn merge(&mut self, other: &Mesh) {
        let base = self.positions.len() as u32;
        let had_colors = !self.colors.is_empty() || !other.colors.is_empty();
        if had_colors && self.colors.len() < self.positions.len() {
            self.colors.resize(self.positions.len(), Color32::WHITE);
        }
        self.positions.extend_from_slice(&other.positions);
        self.normals.extend_from_slice(&other.normals);
        if had_colors {
            if other.colors.is_empty() {
                self.colors.extend(std::iter::repeat_n(Color32::WHITE, other.positions.len()));
            } else {
                self.colors.extend_from_slice(&other.colors);
            }
        }
        self.indices.extend(other.indices.iter().map(|t| [t[0] + base, t[1] + base, t[2] + base]));
        self.edges.extend(other.edges.iter().map(|e| [e[0] + base, e[1] + base]));
    }
    /// Recompute smooth normals by averaging (area-weighted) face normals.
    pub fn compute_normals(&mut self) {
        let mut n = vec![Vec3::ZERO; self.positions.len()];
        for t in &self.indices {
            let [a, b, c] = t.map(|i| self.positions[i as usize]);
            let f = (b - a).cross(c - a);
            for i in t {
                n[*i as usize] += f;
            }
        }
        self.normals = n.into_iter().map(|v| v.normalized()).collect();
    }
    /// Fix winding using the vertex normals, drop degenerate triangles and de-duplicate edges.
    /// Every built-in primitive calls this; call it after building your own mesh by hand.
    pub fn finish(mut self) -> Self {
        let pos = &self.positions;
        let nor = &self.normals;
        self.indices.retain_mut(|t| {
            let [a, b, c] = t.map(|i| pos[i as usize]);
            let f = (b - a).cross(c - a);
            let scale = (b - a).length_sq().max((c - a).length_sq()).max(1e-20);
            if f.length_sq() < scale * scale * 1e-10 {
                return false;
            }
            if !nor.is_empty() {
                let avg = nor[t[0] as usize] + nor[t[1] as usize] + nor[t[2] as usize];
                if f.dot(avg) < 0.0 {
                    t.swap(1, 2);
                }
            }
            true
        });
        for e in &mut self.edges {
            if e[0] > e[1] {
                e.swap(0, 1);
            }
        }
        self.edges.retain(|e| e[0] != e[1] && pos[e[0] as usize].distance(pos[e[1] as usize]) > 1e-6);
        self.edges.sort_unstable();
        self.edges.dedup();
        self
    }

    /// Build a parametric surface over a `cols x rows` grid. `f(u, v)` returns position and normal,
    /// with `u, v` in `0..=1`.
    pub fn parametric(cols: usize, rows: usize, f: impl Fn(f32, f32) -> (Vec3, Vec3)) -> Self {
        let (cols, rows) = (cols.max(1), rows.max(1));
        let mut m = Mesh::new();
        for j in 0..=rows {
            for i in 0..=cols {
                let (p, n) = f(i as f32 / cols as f32, j as f32 / rows as f32);
                m.vertex(p, n);
            }
        }
        let w = (cols + 1) as u32;
        for j in 0..rows as u32 {
            for i in 0..cols as u32 {
                let a = j * w + i;
                m.quad(a, a + 1, a + w + 1, a + w);
            }
        }
        m.finish()
    }

    /// Axis-aligned cube centered on the origin with edge length `size`.
    pub fn cube(size: f32) -> Self {
        Self::cuboid(Vec3::splat(size))
    }

    /// Axis-aligned box with the given full dimensions.
    pub fn cuboid(size: Vec3) -> Self {
        let h = size * 0.5;
        let mut m = Mesh::new();
        for axis in 0..3 {
            for sign in [-1.0f32, 1.0] {
                let mut n = Vec3::ZERO;
                let (u, v) = match axis {
                    0 => {
                        n.x = sign;
                        (Vec3::Z, Vec3::Y)
                    }
                    1 => {
                        n.y = sign;
                        (Vec3::X, Vec3::Z)
                    }
                    _ => {
                        n.z = sign;
                        (Vec3::X, Vec3::Y)
                    }
                };
                let c = n * h;
                let (u, v) = (u * h, v * h);
                let (u, v) = (vec3(u.x, u.y, u.z), vec3(v.x, v.y, v.z));
                let a = m.vertex(c - u - v, n);
                let b = m.vertex(c + u - v, n);
                let cc = m.vertex(c + u + v, n);
                let d = m.vertex(c - u + v, n);
                m.quad(a, b, cc, d);
            }
        }
        m.finish()
    }

    /// Latitude/longitude sphere.
    pub fn uv_sphere(radius: f32, segments: usize, rings: usize) -> Self {
        Self::parametric(segments.max(3), rings.max(2), |u, v| {
            let n = Vec3::from_spherical(1.0, u * TAU, PI / 2.0 - v * PI);
            (n * radius, n)
        })
    }

    /// Geodesic sphere: an icosahedron subdivided `subdivisions` times (0..=5).
    pub fn icosphere(radius: f32, subdivisions: u32) -> Self {
        let (mut pos, mut faces) = icosahedron_data();
        for _ in 0..subdivisions.min(5) {
            let mut cache = std::collections::HashMap::new();
            let mut mid = |a: u32, b: u32, pos: &mut Vec<Vec3>| -> u32 {
                let key = (a.min(b), a.max(b));
                *cache.entry(key).or_insert_with(|| {
                    pos.push(((pos[a as usize] + pos[b as usize]) * 0.5).normalized());
                    (pos.len() - 1) as u32
                })
            };
            let mut nf = Vec::with_capacity(faces.len() * 4);
            for [a, b, c] in faces {
                let ab = mid(a, b, &mut pos);
                let bc = mid(b, c, &mut pos);
                let ca = mid(c, a, &mut pos);
                nf.extend([[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
            }
            faces = nf;
        }
        let mut m = Mesh::new();
        for p in &pos {
            m.vertex(*p * radius, *p);
        }
        for [a, b, c] in faces {
            m.triangle(a, b, c);
        }
        m.finish()
    }

    /// Torus lying in the XZ plane: `major` ring radius, `minor` tube radius.
    pub fn torus(major: f32, minor: f32, segments: usize, sides: usize) -> Self {
        Self::parametric(segments.max(3), sides.max(3), |u, v| {
            let (a, b) = (u * TAU, v * TAU);
            let ring = vec3(a.cos(), 0.0, a.sin());
            let n = ring * b.cos() + Vec3::Y * b.sin();
            (ring * major + n * minor, n)
        })
    }

    /// A `(p, q)` torus knot tube (`p` windings around the axis, `q` through the hole).
    pub fn torus_knot(p: u32, q: u32, radius: f32, tube: f32, segments: usize, sides: usize) -> Self {
        let n = segments.max(8);
        let path: Vec<Vec3> = (0..n)
            .map(|i| {
                let t = i as f32 / n as f32 * TAU;
                let (pt, qt) = (p as f32 * t, q as f32 * t);
                let r = 2.0 + qt.cos();
                vec3(r * pt.cos(), -qt.sin(), r * pt.sin()) * (radius / 3.0)
            })
            .collect();
        Self::tube(&path, |_| tube, sides, true)
    }

    /// A tube of radius `radius(t)` (t in 0..1) swept along `path` with parallel-transport frames.
    pub fn tube(path: &[Vec3], radius: impl Fn(f32) -> f32, sides: usize, closed: bool) -> Self {
        let n = path.len();
        if n < 2 {
            return Mesh::new();
        }
        let sides = sides.max(3);
        let tangent = |i: usize| -> Vec3 {
            let (a, b) = if closed {
                (path[(i + n - 1) % n], path[(i + 1) % n])
            } else {
                (path[i.saturating_sub(1)], path[(i + 1).min(n - 1)])
            };
            (b - a).normalized()
        };
        let mut normals = Vec::with_capacity(n + 1);
        let mut nrm = tangent(0).any_orthogonal();
        normals.push(nrm);
        for i in 1..=n {
            let t = tangent(i % n);
            nrm = (nrm - t * nrm.dot(t)).normalized();
            normals.push(nrm);
        }
        // spread the closing twist of a loop over its length
        let twist = if closed {
            let t0 = tangent(0);
            let (a, b) = (normals[0], normals[n]);
            let ang = a.dot(b).clamp(-1.0, 1.0).acos();
            if a.cross(b).dot(t0) < 0.0 {
                -ang
            } else {
                ang
            }
        } else {
            0.0
        };
        let rings = if closed { n + 1 } else { n };
        let mut m = Mesh::new();
        for i in 0..rings {
            let t = tangent(i % n);
            let ang = -twist * i as f32 / n as f32;
            let nn = crate::math::Quat::from_axis_angle(t, ang).rotate(normals[i]);
            let bn = t.cross(nn);
            let r = radius(i as f32 / (rings - 1) as f32);
            for s in 0..=sides {
                let a = s as f32 / sides as f32 * TAU;
                let dir = nn * a.cos() + bn * a.sin();
                m.vertex(path[i % n] + dir * r, dir);
            }
        }
        let w = (sides + 1) as u32;
        for i in 0..(rings - 1) as u32 {
            for s in 0..sides as u32 {
                let a = i * w + s;
                m.quad(a, a + w, a + w + 1, a + 1);
            }
        }
        m.finish()
    }

    /// Surface of revolution around +Y. `profile` is a list of `(radius, y)` points walked so the
    /// outside is on the right (bottom center → outer wall → top for a closed solid). Sharp
    /// corners (more than ~35°) keep hard normals; gentle bends are smoothed.
    pub fn lathe(profile: &[(f32, f32)], segments: usize) -> Self {
        let segs = segments.max(3);
        let k = profile.len();
        let mut m = Mesh::new();
        if k < 2 {
            return m;
        }
        let seg_n: Vec<(f32, f32)> = profile
            .windows(2)
            .map(|w| {
                let (dr, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
                let l = (dr * dr + dy * dy).sqrt().max(1e-9);
                (dy / l, -dr / l)
            })
            .collect();
        for (si, w) in profile.windows(2).enumerate() {
            let n_at = |pi: usize| -> (f32, f32) {
                let own = seg_n[si];
                let other = if pi == si { si.checked_sub(1) } else if si + 1 < seg_n.len() { Some(si + 1) } else { None };
                match other {
                    Some(o) if own.0 * seg_n[o].0 + own.1 * seg_n[o].1 > 0.82 => {
                        let (x, y) = (own.0 + seg_n[o].0, own.1 + seg_n[o].1);
                        let l = (x * x + y * y).sqrt().max(1e-9);
                        (x / l, y / l)
                    }
                    _ => own,
                }
            };
            let (n0, n1) = (n_at(si), n_at(si + 1));
            let base = m.positions.len() as u32;
            for s in 0..=segs {
                let a = s as f32 / segs as f32 * TAU;
                let (c, sn) = (a.cos(), a.sin());
                for ((r, y), (nr, ny)) in [(w[0], n0), (w[1], n1)] {
                    m.vertex(vec3(r * c, y, r * sn), vec3(nr * c, ny, nr * sn).normalized());
                }
            }
            for s in 0..segs as u32 {
                let a = base + s * 2;
                m.quad(a, a + 2, a + 3, a + 1);
            }
        }
        m.finish()
    }

    /// Cylinder (or truncated cone) along Y, centered on the origin, with caps.
    pub fn cylinder(radius_top: f32, radius_bottom: f32, height: f32, segments: usize) -> Self {
        let h = height / 2.0;
        Self::lathe(&[(0.0, -h), (radius_bottom, -h), (radius_top, h), (0.0, h)], segments)
    }

    /// Cone along Y with its tip up.
    pub fn cone(radius: f32, height: f32, segments: usize) -> Self {
        Self::cylinder(0.0, radius, height, segments)
    }

    /// Capsule along Y: a cylinder of `length` with hemispherical ends.
    pub fn capsule(radius: f32, length: f32, segments: usize, rings: usize) -> Self {
        let rings = rings.max(2);
        let h = length / 2.0;
        let mut prof = Vec::new();
        for i in 0..=rings {
            let a = -PI / 2.0 + i as f32 / rings as f32 * PI / 2.0;
            prof.push((radius * a.cos(), -h + radius * a.sin()));
        }
        for i in 0..=rings {
            let a = i as f32 / rings as f32 * PI / 2.0;
            prof.push((radius * a.cos(), h + radius * a.sin()));
        }
        Self::lathe(&prof, segments)
    }

    /// Flat plane in XZ facing +Y, `width x depth`, subdivided into `cols x rows` quads.
    pub fn plane(width: f32, depth: f32, cols: usize, rows: usize) -> Self {
        Self::parametric(cols, rows, |u, v| (vec3((u - 0.5) * width, 0.0, (v - 0.5) * depth), Vec3::Y))
    }

    /// Build a faceted polyhedron from positions and convex polygon faces (any winding).
    /// Every face gets its own vertices so facets stay crisp.
    pub fn polyhedron(positions: &[Vec3], faces: &[Vec<u32>]) -> Self {
        let mut m = Mesh::new();
        for f in faces {
            let pts: Vec<Vec3> = f.iter().map(|&i| positions[i as usize]).collect();
            let c = pts.iter().fold(Vec3::ZERO, |a, p| a + *p) / pts.len() as f32;
            let mut n = (pts[1] - pts[0]).cross(pts[2] - pts[0]).normalized();
            if n.dot(c) < 0.0 {
                n = -n;
            }
            let base = m.positions.len() as u32;
            for p in &pts {
                m.vertex(*p, n);
            }
            for i in 1..pts.len() as u32 - 1 {
                m.indices.push([base, base + i, base + i + 1]);
            }
            for i in 0..pts.len() as u32 {
                m.edges.push([base + i, base + (i + 1) % pts.len() as u32]);
            }
        }
        m.finish()
    }

    /// Regular octahedron with circumradius `r`.
    pub fn octahedron(r: f32) -> Self {
        let p = [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z].map(|v| v * r);
        let faces = [[0, 2, 4], [0, 4, 3], [0, 3, 5], [0, 5, 2], [1, 4, 2], [1, 3, 4], [1, 5, 3], [1, 2, 5]];
        Self::polyhedron(&p, &faces.map(|f| f.to_vec()))
    }

    /// Regular icosahedron with circumradius `r`.
    pub fn icosahedron(r: f32) -> Self {
        let (p, f) = icosahedron_data();
        let p: Vec<Vec3> = p.into_iter().map(|v| v * r).collect();
        Self::polyhedron(&p, &f.iter().map(|t| t.to_vec()).collect::<Vec<_>>())
    }

    /// Regular dodecahedron with circumradius `r` (built as the dual of the icosahedron).
    pub fn dodecahedron(r: f32) -> Self {
        let (p, f) = icosahedron_data();
        let centers: Vec<Vec3> = f.iter().map(|t| ((p[t[0] as usize] + p[t[1] as usize] + p[t[2] as usize]) / 3.0).normalized() * r).collect();
        let mut faces = Vec::new();
        for (vi, v) in p.iter().enumerate() {
            let mut around: Vec<u32> = f.iter().enumerate().filter(|(_, t)| t.contains(&(vi as u32))).map(|(i, _)| i as u32).collect();
            let u = v.any_orthogonal();
            let w = v.cross(u);
            around.sort_by(|&a, &b| {
                let ang = |i: u32| {
                    let d = centers[i as usize];
                    d.dot(w).atan2(d.dot(u))
                };
                ang(a).total_cmp(&ang(b))
            });
            faces.push(around);
        }
        Self::polyhedron(&centers, &faces)
    }

    /// Parse a Wavefront OBJ string (`v`, `vn`, `f`; polygons are triangulated as fans). Vertex
    /// colors written as `v x y z r g b` (0..1) are supported. Missing normals are computed.
    pub fn from_obj(src: &str) -> Result<Mesh, ObjError> {
        let mut pos = Vec::new();
        let mut cols: Vec<Option<Color32>> = Vec::new();
        let mut nor = Vec::new();
        let mut m = Mesh::new();
        let mut map = std::collections::HashMap::new();
        let mut any_color = false;
        let mut need_normals = false;
        let mut face_no = 0usize;
        for (ln, line) in src.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            let mut it = line.split_whitespace();
            let err = |msg: &str| ObjError { line: ln + 1, message: msg.to_owned() };
            let nums = |it: std::str::SplitWhitespace<'_>| -> Result<Vec<f32>, ObjError> {
                it.map(|s| s.parse::<f32>().map_err(|_| err("bad number"))).collect()
            };
            match it.next() {
                Some("v") => {
                    let v = nums(it)?;
                    if v.len() < 3 {
                        return Err(err("vertex needs 3 coordinates"));
                    }
                    pos.push(vec3(v[0], v[1], v[2]));
                    if v.len() >= 6 {
                        any_color = true;
                        let c = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
                        cols.push(Some(Color32::from_rgb(c(v[3]), c(v[4]), c(v[5]))));
                    } else {
                        cols.push(None);
                    }
                }
                Some("vn") => {
                    let v = nums(it)?;
                    if v.len() < 3 {
                        return Err(err("normal needs 3 coordinates"));
                    }
                    nor.push(vec3(v[0], v[1], v[2]).normalized());
                }
                Some("f") => {
                    face_no += 1;
                    let mut idx = Vec::new();
                    for tok in it {
                        let mut parts = tok.split('/');
                        let resolve = |s: Option<&str>, len: usize| -> Result<Option<usize>, ObjError> {
                            match s {
                                None | Some("") => Ok(None),
                                Some(s) => {
                                    let i: i64 = s.parse().map_err(|_| err("bad index"))?;
                                    let r = if i < 0 { len as i64 + i } else { i - 1 };
                                    if r < 0 || r as usize >= len {
                                        Err(err("index out of range"))
                                    } else {
                                        Ok(Some(r as usize))
                                    }
                                }
                            }
                        };
                        let vi = resolve(parts.next(), pos.len())?.ok_or_else(|| err("face without vertex"))?;
                        let _vt = parts.next();
                        let ni = resolve(parts.next(), nor.len())?;
                        if ni.is_none() {
                            need_normals = true;
                        }
                        // without normals, faces get their own vertices so the model stays faceted
                        let key = (vi, ni.unwrap_or(usize::MAX - face_no));
                        let id = *map.entry(key).or_insert_with(|| {
                            m.positions.push(pos[vi]);
                            m.normals.push(ni.map(|n| nor[n]).unwrap_or(Vec3::ZERO));
                            m.colors.push(cols[vi].unwrap_or(Color32::WHITE));
                            (m.positions.len() - 1) as u32
                        });
                        idx.push(id);
                    }
                    if idx.len() < 3 {
                        return Err(err("face needs at least 3 vertices"));
                    }
                    for i in 1..idx.len() - 1 {
                        m.indices.push([idx[0], idx[i], idx[i + 1]]);
                    }
                    for i in 0..idx.len() {
                        m.edges.push([idx[i], idx[(i + 1) % idx.len()]]);
                    }
                }
                _ => {}
            }
        }
        if m.indices.is_empty() {
            return Err(ObjError { line: 0, message: "no faces".to_owned() });
        }
        if !any_color {
            m.colors.clear();
        }
        if need_normals {
            m.compute_normals();
            // keep the file's winding: finish() would only re-derive it from these normals
        }
        for e in &mut m.edges {
            if e[0] > e[1] {
                e.swap(0, 1);
            }
        }
        m.edges.sort_unstable();
        m.edges.dedup();
        Ok(m)
    }
}

/// Error from [`Mesh::from_obj`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjError {
    /// 1-based line number (0 for whole-file errors).
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ObjError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OBJ line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ObjError {}

fn icosahedron_data() -> (Vec<Vec3>, Vec<[u32; 3]>) {
    let t = (1.0 + 5f32.sqrt()) / 2.0;
    let p: Vec<Vec3> = [
        (-1.0, t, 0.0),
        (1.0, t, 0.0),
        (-1.0, -t, 0.0),
        (1.0, -t, 0.0),
        (0.0, -1.0, t),
        (0.0, 1.0, t),
        (0.0, -1.0, -t),
        (0.0, 1.0, -t),
        (t, 0.0, -1.0),
        (t, 0.0, 1.0),
        (-t, 0.0, -1.0),
        (-t, 0.0, 1.0),
    ]
    .iter()
    .map(|&(x, y, z)| vec3(x, y, z).normalized())
    .collect();
    let f = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    (p, f)
}

/// Built-in models, built procedurally (or parsed from tiny embedded OBJ strings), with vertex
/// colors so they look right with any lit material.
pub mod models {
    use super::*;
    use glowkit_core::color::hex;

    /// A low-poly toy rocket (white body, red nose and fins, blue porthole), about 2.6 units tall.
    pub fn rocket() -> Mesh {
        let white = hex("#F4F4F5");
        let red = hex("#EF4444");
        let body = Mesh::lathe(
            &[(0.0, -1.05), (0.30, -1.05), (0.40, -0.75), (0.46, -0.3), (0.47, 0.1), (0.44, 0.45), (0.36, 0.78), (0.24, 1.05), (0.11, 1.25), (0.0, 1.35)],
            24,
        )
        .paint(|p, _| if p.y > 0.72 { red } else { white });
        let mut m = body;
        let nozzle = Mesh::cylinder(0.26, 0.34, 0.3, 20).transformed(&Mat4::translation(vec3(0.0, -1.18, 0.0))).with_color(hex("#52525B"));
        m.merge(&nozzle);
        for i in 0..3 {
            let a = i as f32 / 3.0 * TAU;
            let pts = [vec3(0.3, -1.1, -0.04), vec3(0.85, -1.25, -0.04), vec3(0.42, -0.35, -0.04), vec3(0.3, -1.1, 0.04), vec3(0.85, -1.25, 0.04), vec3(0.42, -0.35, 0.04)];
            let c = pts.iter().fold(Vec3::ZERO, |s, p| s + *p) / 6.0;
            let fin = Mesh::polyhedron(&pts.map(|p| p - c), &[vec![0, 1, 2], vec![3, 4, 5], vec![0, 1, 4, 3], vec![1, 2, 5, 4], vec![2, 0, 3, 5]]);
            m.merge(&fin.transformed(&(Mat4::rotation_y(a) * Mat4::translation(c))).with_color(red));
        }
        let ring = Mesh::torus(0.17, 0.035, 20, 8).transformed(&(Mat4::translation(vec3(0.0, 0.32, 0.45)) * Mat4::rotation_x(PI / 2.0))).with_color(hex("#A1A1AA"));
        m.merge(&ring);
        let glass = Mesh::uv_sphere(0.15, 16, 8).transformed(&(Mat4::translation(vec3(0.0, 0.32, 0.4)) * Mat4::scale(vec3(1.0, 1.0, 0.45)))).with_color(hex("#38BDF8"));
        m.merge(&glass);
        m
    }

    /// A brilliant-cut style gem (faceted; use flat shading).
    pub fn gem() -> Mesh {
        Mesh::lathe(&[(0.0, -0.85), (1.0, 0.08), (1.0, 0.18), (0.62, 0.5), (0.0, 0.5)], 10)
    }

    /// A coffee mug with handle and coffee inside.
    pub fn mug() -> Mesh {
        let body_c = hex("#FAFAFA");
        let mut m = Mesh::lathe(
            &[(0.0, -0.6), (0.52, -0.6), (0.6, -0.52), (0.62, -0.3), (0.64, 0.55), (0.64, 0.6), (0.56, 0.6), (0.56, -0.48), (0.0, -0.48)],
            32,
        )
        .with_color(body_c);
        let coffee = Mesh::lathe(&[(0.56, 0.38), (0.0, 0.38)], 32).with_color(hex("#5B3A1E"));
        m.merge(&coffee);
        let arc: Vec<Vec3> = (0..=16)
            .map(|i| {
                let a = i as f32 / 16.0 * PI;
                vec3(0.58 + 0.36 * a.sin(), 0.02 + 0.33 * a.cos(), 0.0)
            })
            .collect();
        m.merge(&Mesh::tube(&arc, |_| 0.075, 10, false).with_color(body_c));
        m
    }

    /// Source of the built-in low-poly tree OBJ (with vertex colors).
    pub const TREE_OBJ: &str = "# low-poly tree
v -0.12 0.0 -0.12 0.45 0.28 0.16
v 0.12 0.0 -0.12 0.45 0.28 0.16
v 0.12 0.0 0.12 0.45 0.28 0.16
v -0.12 0.0 0.12 0.45 0.28 0.16
v -0.12 0.5 -0.12 0.45 0.28 0.16
v 0.12 0.5 -0.12 0.45 0.28 0.16
v 0.12 0.5 0.12 0.45 0.28 0.16
v -0.12 0.5 0.12 0.45 0.28 0.16
v -0.6 0.4 -0.6 0.13 0.6 0.33
v 0.6 0.4 -0.6 0.13 0.6 0.33
v 0.6 0.4 0.6 0.13 0.6 0.33
v -0.6 0.4 0.6 0.13 0.6 0.33
v 0.0 1.3 0.0 0.3 0.78 0.45
v -0.45 0.9 -0.45 0.2 0.7 0.4
v 0.45 0.9 -0.45 0.2 0.7 0.4
v 0.45 0.9 0.45 0.2 0.7 0.4
v -0.45 0.9 0.45 0.2 0.7 0.4
v 0.0 1.75 0.0 0.45 0.88 0.55
f 4 3 7 8
f 3 2 6 7
f 2 1 5 6
f 1 4 8 5
f 12 11 13
f 11 10 13
f 10 9 13
f 9 12 13
f 9 10 11 12
f 17 16 18
f 16 15 18
f 15 14 18
f 14 17 18
f 14 15 16 17
";

    /// Source of the built-in paper plane OBJ.
    pub const PAPER_PLANE_OBJ: &str = "# paper plane
v 0.0 0.0 -1.0
v -0.75 0.05 0.6
v -0.08 0.0 0.6
v 0.0 -0.28 0.6
v 0.08 0.0 0.6
v 0.75 0.05 0.6
f 1 2 3
f 1 3 4
f 1 4 5
f 1 5 6
";

    /// A low-poly tree parsed from [`TREE_OBJ`].
    pub fn tree() -> Mesh {
        Mesh::from_obj(TREE_OBJ).unwrap_or_default()
    }

    /// A paper plane parsed from [`PAPER_PLANE_OBJ`] (render with a double-sided material).
    pub fn paper_plane() -> Mesh {
        Mesh::from_obj(PAPER_PLANE_OBJ).unwrap_or_default()
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_outward(name: &str, m: &Mesh) {
        assert!(!m.indices.is_empty(), "{name} empty");
        assert_eq!(m.positions.len(), m.normals.len(), "{name}");
        for t in &m.indices {
            let [a, b, c] = t.map(|i| m.positions[i as usize]);
            let f = (b - a).cross(c - a);
            let avg = m.normals[t[0] as usize] + m.normals[t[1] as usize] + m.normals[t[2] as usize];
            assert!(f.dot(avg) >= 0.0, "{name}: triangle winding disagrees with normals");
            for i in t {
                assert!((*i as usize) < m.positions.len());
            }
        }
    }

    #[test]
    fn primitives_are_consistent() {
        let all = [
            ("cube", Mesh::cube(1.0)),
            ("sphere", Mesh::uv_sphere(1.0, 16, 8)),
            ("ico", Mesh::icosphere(1.0, 2)),
            ("torus", Mesh::torus(1.0, 0.3, 24, 12)),
            ("knot", Mesh::torus_knot(2, 3, 1.0, 0.2, 64, 8)),
            ("cyl", Mesh::cylinder(0.5, 0.5, 1.0, 12)),
            ("cone", Mesh::cone(0.5, 1.0, 12)),
            ("capsule", Mesh::capsule(0.4, 1.0, 12, 4)),
            ("plane", Mesh::plane(2.0, 2.0, 4, 4)),
            ("octa", Mesh::octahedron(1.0)),
            ("icosa", Mesh::icosahedron(1.0)),
            ("dodeca", Mesh::dodecahedron(1.0)),
            ("rocket", models::rocket()),
            ("gem", models::gem()),
            ("mug", models::mug()),
        ];
        for (name, m) in &all {
            check_outward(name, m);
        }
    }

    #[test]
    fn closed_solids_point_outward() {
        for m in [Mesh::cube(1.0), Mesh::uv_sphere(1.0, 12, 6), Mesh::dodecahedron(1.0), Mesh::cylinder(0.5, 0.5, 1.0, 8)] {
            for t in &m.indices {
                let [a, b, c] = t.map(|i| m.positions[i as usize]);
                let f = (b - a).cross(c - a);
                assert!(f.dot((a + b + c) / 3.0) > 0.0);
            }
        }
    }

    #[test]
    fn counts() {
        assert_eq!(Mesh::cube(1.0).triangle_count(), 12);
        assert_eq!(Mesh::cube(1.0).edges.len(), 24);
        assert_eq!(Mesh::icosahedron(1.0).triangle_count(), 20);
        assert_eq!(Mesh::dodecahedron(1.0).triangle_count(), 36);
        assert_eq!(Mesh::dodecahedron(1.0).edges.len(), 60);
        assert_eq!(Mesh::icosphere(1.0, 1).triangle_count(), 80);
        assert!((Mesh::icosphere(2.0, 2).radius() - 2.0).abs() < 1e-4);
    }

    #[test]
    fn obj_parsing() {
        let m = Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvn 0 0 1\nf 1//1 2//1 3//1 4//1\n").unwrap();
        assert_eq!(m.triangle_count(), 2);
        assert_eq!(m.edges.len(), 4);
        assert_eq!(m.normals[0], Vec3::Z);
        let neg = Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\n").unwrap();
        assert!((neg.normals[0] - Vec3::Z).length() < 1e-5);
        assert!(Mesh::from_obj("v 0 0 0\nf 1 2 3\n").is_err());
        assert!(Mesh::from_obj("").is_err());
        let tree = models::tree();
        assert_eq!(tree.triangle_count(), 4 * 2 + 4 + 2 + 4 + 2);
        assert!(!tree.colors.is_empty());
        check_outward("plane", &models::paper_plane());
    }
}
