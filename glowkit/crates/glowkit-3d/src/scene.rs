//! The software renderer: transforms, lights, clips, culls and depth-sorts triangles, lines,
//! points and text, then paints them as a few batched egui meshes.

use crate::camera::{Camera, Projector};
use crate::material::{Fog, Lights, Material, Rgb, Shading};
use crate::math::{Mat4, Transform, Vec3};
use crate::mesh::Mesh;
use egui::epaint::{Mesh as EMesh, Vertex, WHITE_UV};
use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Shape};
use std::f32::consts::TAU;

impl From<Transform> for Mat4 {
    fn from(t: Transform) -> Mat4 {
        t.matrix()
    }
}

impl From<&Transform> for Mat4 {
    fn from(t: &Transform) -> Mat4 {
        t.matrix()
    }
}

impl From<Vec3> for Mat4 {
    /// A pure translation.
    fn from(v: Vec3) -> Mat4 {
        Mat4::translation(v)
    }
}

/// Make a color blend additively (premultiplied with zero alpha). Additive light only shows on
/// dark backgrounds; on light themes prefer normal blending.
pub fn additive(c: Color32) -> Color32 {
    let [r, g, b, a] = c.to_array();
    let _ = a;
    Color32::from_rgba_premultiplied(r, g, b, 0)
}

/// Render statistics for the last frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Triangles submitted.
    pub triangles_in: usize,
    /// Triangles that survived culling and clipping.
    pub triangles_drawn: usize,
    pub lines: usize,
    pub points: usize,
    /// egui meshes produced.
    pub batches: usize,
}

enum Prim {
    Tri([Pos2; 3], [Color32; 3]),
    Line { a: Pos2, b: Pos2, wa: f32, wb: f32, ca: Color32, cb: Color32 },
    Disc { c: Pos2, r: f32, color: Color32, soft: bool },
    Ball { c: Pos2, r: f32, color: Color32, light: [f32; 3], spec: f32, ambient: f32 },
    Text(Box<EMesh>),
}

#[derive(Clone, Copy)]
struct Key {
    depth: f32,
    idx: u32,
}

#[derive(Clone, Copy)]
struct CVert {
    v: Vec3,
    c: [f32; 4],
}

/// A frame's worth of 3D content. Obtained from [`crate::Viewport3D::show`] (or built directly
/// with [`Scene::new`] and painted with [`Scene::paint`]).
///
/// Drawing is immediate: each call transforms and shades geometry with the current
/// [`Scene::camera`] and [`Scene::lights`], so set those first. Everything is depth sorted
/// together at the end (painter's algorithm), so draw order does not matter, except inside a
/// [`Scene::group`] where items keep their submission order.
///
/// ```no_run
/// # use glowkit_3d::*;
/// # fn f(scene: &mut Scene) {
/// let cube = Mesh::cube(1.0);
/// scene.draw(&cube, Transform::IDENTITY.with_euler(0.4, 0.8, 0.0), &Material::smooth(egui::Color32::LIGHT_BLUE));
/// scene.point(vec3(0.0, 1.5, 0.0), 0.05, egui::Color32::WHITE);
/// scene.line(vec3(-1.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0), 2.0, egui::Color32::RED);
/// # }
/// ```
pub struct Scene {
    camera: Camera,
    /// Lights used by subsequent draws.
    pub lights: Lights,
    /// Fog used by subsequent draws.
    pub fog: Option<Fog>,
    rect: Rect,
    painter: Painter,
    view: Mat4,
    proj: Projector,
    prims: Vec<Prim>,
    keys: Vec<Key>,
    group: Option<f32>,
    stats: Stats,
    s_view: Vec<Vec3>,
    s_world: Vec<Vec3>,
    s_nrm: Vec<Vec3>,
    s_col: Vec<[f32; 4]>,
}

impl Scene {
    /// A scene drawing into `rect` with `painter`.
    pub fn new(painter: Painter, rect: Rect, camera: Camera) -> Self {
        Self {
            view: camera.view(),
            proj: Projector::new(&camera, rect),
            camera,
            lights: Lights::default(),
            fog: None,
            rect,
            painter,
            prims: Vec::new(),
            keys: Vec::new(),
            group: None,
            stats: Stats::default(),
            s_view: Vec::new(),
            s_world: Vec::new(),
            s_nrm: Vec::new(),
            s_col: Vec::new(),
        }
    }

    /// The current camera.
    pub fn camera(&self) -> &Camera {
        &self.camera
    }
    /// Replace the camera (affects subsequent draws).
    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
        self.view = camera.view();
        self.proj = Projector::new(&camera, self.rect);
    }
    /// The screen rectangle being drawn into.
    pub fn rect(&self) -> Rect {
        self.rect
    }
    /// The painter (already clipped to the viewport).
    pub fn painter(&self) -> &Painter {
        &self.painter
    }
    /// Statistics so far.
    pub fn stats(&self) -> Stats {
        self.stats
    }
    /// The view matrix.
    pub fn view_matrix(&self) -> Mat4 {
        self.view
    }

    /// Project a world point to the screen. Returns `(position, view distance)`.
    pub fn project(&self, p: Vec3) -> Option<(Pos2, f32)> {
        self.proj.project(self.view.transform_point(p))
    }
    /// Screen size (points) of one world unit at world point `p`.
    pub fn pixels_per_unit(&self, p: Vec3) -> f32 {
        self.proj.scale_at(-self.view.transform_point(p).z)
    }
    /// World ray `(origin, direction)` through a screen point.
    pub fn ray(&self, screen: Pos2) -> (Vec3, Vec3) {
        self.camera.ray(self.rect, screen)
    }
    /// View distance of a world point (positive in front of the camera).
    pub fn depth_of(&self, p: Vec3) -> f32 {
        -self.view.transform_point(p).z
    }

    fn push(&mut self, depth: f32, p: Prim) {
        let depth = self.group.unwrap_or(depth);
        self.keys.push(Key { depth, idx: self.prims.len() as u32 });
        self.prims.push(p);
    }

    /// Draw everything submitted inside `f` as one unit sorted at the depth of `anchor`,
    /// keeping submission order inside the group (e.g. a card and the text printed on it).
    pub fn group<R>(&mut self, anchor: Vec3, f: impl FnOnce(&mut Scene) -> R) -> R {
        let prev = self.group;
        self.group = Some(self.depth_of(anchor));
        let r = f(self);
        self.group = prev;
        r
    }

    /// Draw everything submitted inside `f` on top of all other 3D content.
    pub fn overlay<R>(&mut self, f: impl FnOnce(&mut Scene) -> R) -> R {
        let prev = self.group;
        self.group = Some(f32::NEG_INFINITY);
        let r = f(self);
        self.group = prev;
        r
    }

    /// Draw everything submitted inside `f` behind all other 3D content.
    pub fn underlay<R>(&mut self, f: impl FnOnce(&mut Scene) -> R) -> R {
        let prev = self.group;
        self.group = Some(f32::INFINITY);
        let r = f(self);
        self.group = prev;
        r
    }

    #[inline]
    fn fog_apply(&self, fog_on: bool, d: f32, c: &mut [f32; 4]) {
        if let (true, Some(f)) = (fog_on, self.fog) {
            let t = f.amount(d);
            if t > 0.0 {
                if f.fade {
                    c[3] *= 1.0 - t;
                } else {
                    let fc = Rgb::from(f.color);
                    c[0] += (fc.0 - c[0]) * t;
                    c[1] += (fc.1 - c[1]) * t;
                    c[2] += (fc.2 - c[2]) * t;
                }
            }
        }
    }

    fn shade(&self, mat: &Material, grad: &[Rgb; 3], base: Rgb, p: Vec3, n: Vec3) -> Rgb {
        let vdir = (self.camera.position - p).normalized();
        let rim_col = mat.rim_color.map(Rgb::from).unwrap_or(Rgb(1.0, 1.0, 1.0));
        let ndv = n.dot(vdir).max(0.0);
        let mut out = match mat.shading {
            Shading::Unlit | Shading::Wireframe => base,
            Shading::Normal => Rgb(n.x * 0.5 + 0.5, n.y * 0.5 + 0.5, n.z * 0.5 + 0.5),
            Shading::Matcap => {
                let vn = self.view.transform_vector(n);
                let t = (0.5 + 0.5 * (vn.y * 0.8 - vn.x * 0.35)).clamp(0.0, 1.0);
                let g = if t < 0.5 { grad[0].lerp(grad[1], t * 2.0) } else { grad[1].lerp(grad[2], t * 2.0 - 1.0) };
                let h = Vec3::new(-0.35, 0.55, 0.76);
                let s = vn.dot(h).max(0.0).powf(mat.shininess.max(4.0)) * mat.specular;
                g.add(Rgb(s, s, s))
            }
            Shading::Flat | Shading::Smooth | Shading::Toon { .. } => {
                let l = &self.lights;
                let amb = Rgb::from(l.ambient).scale(l.ambient_intensity);
                let sky = Rgb::from(l.sky).scale(l.sky_intensity * (n.y * 0.5 + 0.5));
                let mut diff = amb.add(sky);
                let mut spec = Rgb::default();
                let bands = if let Shading::Toon { bands } = mat.shading { bands.max(2) as f32 } else { 0.0 };
                let mut add_light = |dir: Vec3, col: Rgb, k: f32| {
                    let mut ndl = n.dot(dir).max(0.0);
                    if bands > 0.0 {
                        ndl = ((ndl * bands).floor() / (bands - 1.0)).min(1.0);
                    }
                    diff = diff.add(col.scale(ndl * k));
                    if mat.specular > 0.0 {
                        let h = (dir + vdir).normalized();
                        let mut s = n.dot(h).max(0.0).powf(mat.shininess);
                        if bands > 0.0 {
                            s = if s > 0.5 { 1.0 } else { 0.0 };
                        }
                        spec = spec.add(col.scale(s * mat.specular * k));
                    }
                };
                for d in &l.directional {
                    add_light(d.direction, Rgb::from(d.color), d.intensity);
                }
                for pl in &l.point {
                    let dv = pl.position - p;
                    let dist = dv.length();
                    let att = pl.intensity / (1.0 + 3.0 * (dist / pl.range).powi(2));
                    add_light(dv / dist.max(1e-6), Rgb::from(pl.color), att);
                }
                base.mul(diff).add(spec)
            }
        };
        if mat.rim > 0.0 && mat.shading != Shading::Unlit {
            let f = (1.0 - ndv).powi(3) * mat.rim;
            out = out.add(rim_col.scale(f));
        } else if mat.rim > 0.0 {
            let f = (1.0 - ndv).powi(2) * mat.rim;
            out = out.add(rim_col.scale(f));
        }
        if mat.emissive > 0.0 && !matches!(mat.shading, Shading::Unlit | Shading::Wireframe) {
            out = out.add(base.scale(mat.emissive));
        }
        out
    }

    #[inline]
    fn to_c32(c: [f32; 4], additive: bool) -> Color32 {
        Rgb(c[0], c[1], c[2]).to_color(c[3], additive)
    }

    /// Draw a mesh with a transform (a [`Transform`], a [`Mat4`] or a translation [`Vec3`]).
    pub fn draw(&mut self, mesh: &Mesh, transform: impl Into<Mat4>, mat: &Material) {
        let model: Mat4 = transform.into();
        let nm = model.normal_matrix();
        let n = mesh.positions.len();
        if n == 0 {
            return;
        }
        let mut wp = std::mem::take(&mut self.s_world);
        let mut vp = std::mem::take(&mut self.s_view);
        let mut wn = std::mem::take(&mut self.s_nrm);
        let mut col = std::mem::take(&mut self.s_col);
        wp.clear();
        vp.clear();
        wn.clear();
        col.clear();
        let (mut dmin, mut dmax) = (f32::INFINITY, f32::NEG_INFINITY);
        for i in 0..n {
            let w = model.transform_point(mesh.positions[i]);
            let v = self.view.transform_point(w);
            dmin = dmin.min(-v.z);
            dmax = dmax.max(-v.z);
            wp.push(w);
            vp.push(v);
            wn.push(nm.transform_vector(mesh.normals.get(i).copied().unwrap_or(Vec3::Y)).normalized());
        }
        if dmax < self.camera.near {
            self.restore(wp, vp, wn, col);
            return;
        }
        let base = Rgb::from(mat.color);
        let grad = mat.gradient.map(Rgb::from);
        let has_colors = mesh.colors.len() == n;
        let vbase = |i: usize| -> ([f32; 3], f32) {
            if has_colors {
                let c = mesh.colors[i];
                let r = Rgb::from(c).mul(base);
                ([r.0, r.1, r.2], c.a() as f32 / 255.0)
            } else {
                ([base.0, base.1, base.2], 1.0)
            }
        };
        let per_vertex = !matches!(mat.shading, Shading::Flat | Shading::Wireframe);
        if per_vertex {
            for i in 0..n {
                let (b, a) = vbase(i);
                let mut nn = wn[i];
                if mat.double_sided && nn.dot(self.camera.position - wp[i]) < 0.0 {
                    nn = -nn;
                }
                let c = self.shade(mat, &grad, Rgb(b[0], b[1], b[2]), wp[i], nn);
                let mut c4 = [c.0, c.1, c.2, a * mat.opacity];
                self.fog_apply(mat.fog, -vp[i].z, &mut c4);
                col.push(c4);
            }
        }
        if mat.shading != Shading::Wireframe {
            self.stats.triangles_in += mesh.indices.len();
            let clip = self.rect.expand(4.0);
            for t in &mesh.indices {
                let [ia, ib, ic] = t.map(|i| i as usize);
                let (va, vb, vc) = (vp[ia], vp[ib], vp[ic]);
                let near = self.camera.near;
                let behind = [-va.z < near, -vb.z < near, -vc.z < near];
                if behind[0] && behind[1] && behind[2] {
                    continue;
                }
                // back-face test in view space
                let fnv = (vb - va).cross(vc - va);
                let facing = if self.proj.ortho { fnv.z } else { -fnv.dot(va) };
                if facing <= 0.0 && !mat.double_sided {
                    continue;
                }
                let cols: [[f32; 4]; 3] = if per_vertex {
                    [col[ia], col[ib], col[ic]]
                } else {
                    let mut fnw = (wp[ib] - wp[ia]).cross(wp[ic] - wp[ia]).normalized();
                    if facing <= 0.0 {
                        fnw = -fnw;
                    }
                    let (b0, a0) = vbase(ia);
                    let (b1, _) = vbase(ib);
                    let (b2, _) = vbase(ic);
                    let bb = Rgb((b0[0] + b1[0] + b2[0]) / 3.0, (b0[1] + b1[1] + b2[1]) / 3.0, (b0[2] + b1[2] + b2[2]) / 3.0);
                    let cen = (wp[ia] + wp[ib] + wp[ic]) / 3.0;
                    let c = self.shade(mat, &grad, bb, cen, fnw);
                    let mut c4 = [c.0, c.1, c.2, a0 * mat.opacity];
                    self.fog_apply(mat.fog, -(va.z + vb.z + vc.z) / 3.0, &mut c4);
                    [c4; 3]
                };
                let verts = [CVert { v: va, c: cols[0] }, CVert { v: vb, c: cols[1] }, CVert { v: vc, c: cols[2] }];
                if behind.iter().any(|b| *b) {
                    let poly = clip_near(&verts, near);
                    for k in 1..poly.len().saturating_sub(1) {
                        self.emit_tri([poly[0], poly[k], poly[k + 1]], mat.additive, clip);
                    }
                } else {
                    self.emit_tri(verts, mat.additive, clip);
                }
            }
        }
        if let Some(w) = mat.wire.or(if mat.shading == Shading::Wireframe { Some(crate::material::Wire::new(mat.color)) } else { None }) {
            let overlay = mat.shading != Shading::Wireframe;
            let span = (dmax - dmin).max(1e-3);
            let bias = span * 0.02 + 0.01;
            let wc = Rgb::from(w.color);
            let wa = w.color.a() as f32 / 255.0 * mat.opacity;
            let additive = mat.additive || w.glow > 0.0 && !overlay;
            for e in &mesh.edges {
                let (ia, ib) = (e[0] as usize, e[1] as usize);
                if overlay {
                    let nrm = wn[ia] + wn[ib];
                    let mid = (wp[ia] + wp[ib]) * 0.5;
                    if nrm.dot(self.camera.position - mid) < 0.0 {
                        continue;
                    }
                }
                let (va, vb) = (vp[ia], vp[ib]);
                let Some((va, vb)) = clip_segment(va, vb, self.camera.near) else { continue };
                let fade = |v: Vec3| 1.0 - w.depth_fade * ((-v.z - dmin) / span).clamp(0.0, 1.0);
                let mut ca = [wc.0, wc.1, wc.2, wa * fade(va)];
                let mut cb = [wc.0, wc.1, wc.2, wa * fade(vb)];
                self.fog_apply(mat.fog, -va.z, &mut ca);
                self.fog_apply(mat.fog, -vb.z, &mut cb);
                let (pa, pb) = (self.proj.project_unchecked(va), self.proj.project_unchecked(vb));
                let depth = -(va.z + vb.z) * 0.5 - bias;
                if w.glow > 0.0 {
                    let g = |c: [f32; 4]| additive_c(Self::to_c32([c[0], c[1], c[2], c[3] * 0.22 * w.glow], false));
                    self.push(depth + bias * 0.01, Prim::Line { a: pa, b: pb, wa: w.width * 5.0, wb: w.width * 5.0, ca: g(ca), cb: g(cb) });
                }
                self.stats.lines += 1;
                self.push(depth, Prim::Line { a: pa, b: pb, wa: w.width, wb: w.width, ca: Self::to_c32(ca, additive), cb: Self::to_c32(cb, additive) });
            }
        }
        self.restore(wp, vp, wn, col);
    }

    fn restore(&mut self, wp: Vec<Vec3>, vp: Vec<Vec3>, wn: Vec<Vec3>, col: Vec<[f32; 4]>) {
        self.s_world = wp;
        self.s_view = vp;
        self.s_nrm = wn;
        self.s_col = col;
    }

    fn emit_tri(&mut self, v: [CVert; 3], additive: bool, clip: Rect) {
        let p = v.map(|c| self.proj.project_unchecked(c.v));
        let (minx, maxx) = (p[0].x.min(p[1].x).min(p[2].x), p[0].x.max(p[1].x).max(p[2].x));
        let (miny, maxy) = (p[0].y.min(p[1].y).min(p[2].y), p[0].y.max(p[1].y).max(p[2].y));
        if maxx < clip.min.x || minx > clip.max.x || maxy < clip.min.y || miny > clip.max.y {
            return;
        }
        let depth = -(v[0].v.z + v[1].v.z + v[2].v.z) / 3.0;
        self.stats.triangles_drawn += 1;
        self.push(depth, Prim::Tri(p, v.map(|c| Self::to_c32(c.c, additive))));
    }

    /// Draw a raw world-space triangle with per-vertex colors (no lighting, no culling).
    pub fn triangle(&mut self, p: [Vec3; 3], c: [Color32; 3]) {
        let v = p.map(|x| self.view.transform_point(x));
        let near = self.camera.near;
        let to4 = |c: Color32| {
            let [r, g, b, a] = c.to_array();
            // already premultiplied; store as straight-ish values for interpolation
            let k = if a > 0 { 1.0 / a as f32 } else { 1.0 / 255.0 };
            [r as f32 * k, g as f32 * k, b as f32 * k, a as f32 / 255.0]
        };
        let additive = c.iter().all(|c| c.a() == 0);
        let cv = if additive {
            [0, 1, 2].map(|i| {
                let [r, g, b, _] = c[i].to_array();
                CVert { v: v[i], c: [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0] }
            })
        } else {
            [0, 1, 2].map(|i| CVert { v: v[i], c: to4(c[i]) })
        };
        let clip = self.rect.expand(4.0);
        if v.iter().any(|x| -x.z < near) {
            let poly = clip_near(&cv, near);
            for k in 1..poly.len().saturating_sub(1) {
                self.emit_tri([poly[0], poly[k], poly[k + 1]], additive, clip);
            }
        } else {
            self.emit_tri(cv, additive, clip);
        }
    }

    /// A flat quad (two triangles) with one color, drawn double-sided.
    pub fn quad(&mut self, p: [Vec3; 4], color: Color32) {
        self.group((p[0] + p[1] + p[2] + p[3]) * 0.25, |s| {
            s.triangle([p[0], p[1], p[2]], [color; 3]);
            s.triangle([p[0], p[2], p[3]], [color; 3]);
        });
    }

    /// A round dot of world-space `radius` (scales with perspective).
    pub fn point(&mut self, p: Vec3, radius: f32, color: Color32) {
        self.dot(p, radius, color, false, true);
    }
    /// A soft glowing dot of world-space `radius` (use [`additive`] colors on dark backgrounds).
    pub fn glow_point(&mut self, p: Vec3, radius: f32, color: Color32) {
        self.dot(p, radius, color, true, true);
    }
    /// A dot with a fixed screen radius in points.
    pub fn point_px(&mut self, p: Vec3, radius_px: f32, color: Color32) {
        self.dot(p, radius_px, color, false, false);
    }

    fn dot(&mut self, p: Vec3, r: f32, color: Color32, soft: bool, world: bool) {
        let v = self.view.transform_point(p);
        let Some((s, d)) = self.proj.project(v) else { return };
        let rp = if world { r * self.proj.scale_at(d) } else { r };
        if rp < 0.05 || !self.rect.expand(rp).contains(s) {
            return;
        }
        let color = self.fog_color(color, d);
        self.stats.points += 1;
        self.push(d, Prim::Disc { c: s, r: rp, color, soft });
    }

    /// A lit sphere impostor of world `radius`: a cheap, smooth, perfectly round shaded ball
    /// (lit by the scene's first directional light). Great for molecules, planets, particles.
    pub fn ball(&mut self, p: Vec3, radius: f32, color: Color32) {
        let dir = self.lights.directional.first().map(|d| d.direction).unwrap_or(Vec3::new(0.4, 0.7, 0.6));
        let amb = (self.lights.ambient_intensity + self.lights.sky_intensity * 0.5).clamp(0.0, 1.0);
        self.ball_lit(p, radius, color, dir, amb, 0.45);
    }

    /// A sphere impostor lit from world direction `light` (towards the light) with `ambient`
    /// light and `specular` highlight strength.
    pub fn ball_lit(&mut self, p: Vec3, radius: f32, color: Color32, light: Vec3, ambient: f32, specular: f32) {
        let v = self.view.transform_point(p);
        let Some((s, d)) = self.proj.project(v) else { return };
        let rp = radius * self.proj.scale_at(d);
        if rp < 0.05 || !self.rect.expand(rp).contains(s) {
            return;
        }
        let l = self.view.transform_vector(light).normalized();
        let color = self.fog_color(color, d);
        self.stats.points += 1;
        self.push(d, Prim::Ball { c: s, r: rp, color, light: [l.x, l.y, l.z], spec: specular, ambient });
    }

    fn fog_color(&self, color: Color32, d: f32) -> Color32 {
        match self.fog {
            Some(f) => {
                let t = f.amount(d);
                if t <= 0.0 {
                    color
                } else if f.fade || color.a() == 0 {
                    color.gamma_multiply(1.0 - t)
                } else {
                    color.lerp_to_gamma(f.color, t)
                }
            }
            None => color,
        }
    }

    /// A line segment of constant screen `width` (points).
    pub fn line(&mut self, a: Vec3, b: Vec3, width: f32, color: Color32) {
        self.line_gradient(a, b, width, color, color);
    }

    /// A line segment with a color at each end.
    pub fn line_gradient(&mut self, a: Vec3, b: Vec3, width: f32, ca: Color32, cb: Color32) {
        self.segment(a, b, width, width, ca, cb, false);
    }

    /// A line whose width is in world units (thicker when close to the camera).
    pub fn line_world(&mut self, a: Vec3, b: Vec3, width: f32, ca: Color32, cb: Color32) {
        self.segment(a, b, width, width, ca, cb, true);
    }

    #[allow(clippy::too_many_arguments)]
    fn segment(&mut self, a: Vec3, b: Vec3, wa: f32, wb: f32, ca: Color32, cb: Color32, world: bool) {
        let (va, vb) = (self.view.transform_point(a), self.view.transform_point(b));
        let Some((va2, vb2)) = clip_segment(va, vb, self.camera.near) else { return };
        let (da, db) = (-va2.z, -vb2.z);
        let (pa, pb) = (self.proj.project_unchecked(va2), self.proj.project_unchecked(vb2));
        let (wa, wb) = if world { (wa * self.proj.scale_at(da), wb * self.proj.scale_at(db)) } else { (wa, wb) };
        let (ca, cb) = (self.fog_color(ca, da), self.fog_color(cb, db));
        self.stats.lines += 1;
        self.push((da + db) * 0.5, Prim::Line { a: pa, b: pb, wa, wb, ca, cb });
    }

    /// A polyline through world points with color `f(t)` (`t` 0..1 along the points).
    pub fn polyline(&mut self, pts: &[Vec3], width: f32, closed: bool, f: impl Fn(f32) -> Color32) {
        let n = pts.len();
        if n < 2 {
            return;
        }
        let segs = if closed { n } else { n - 1 };
        for i in 0..segs {
            let j = (i + 1) % n;
            let (ta, tb) = (i as f32 / segs as f32, (i + 1) as f32 / segs as f32);
            self.segment(pts[i], pts[j], width, width, f(ta), f(tb), false);
        }
    }

    /// A circle in 3D (e.g. an orbit ring) around `center` in the plane spanned by `u` and `v`.
    #[allow(clippy::too_many_arguments)]
    pub fn circle(&mut self, center: Vec3, radius: f32, u: Vec3, v: Vec3, segments: usize, width: f32, color: Color32) {
        let n = segments.max(8);
        let pts: Vec<Vec3> = (0..n)
            .map(|i| {
                let a = i as f32 / n as f32 * TAU;
                center + (u * a.cos() + v * a.sin()) * radius
            })
            .collect();
        self.polyline(&pts, width, true, |_| color);
    }

    /// A soft blob shadow on a horizontal ground plane under `center`.
    pub fn shadow(&mut self, center: Vec3, radius: f32, color: Color32) {
        let n = 28;
        let ring = |r: f32| -> Vec<Vec3> {
            (0..n)
                .map(|i| {
                    let a = i as f32 / n as f32 * TAU;
                    center + Vec3::new(a.cos() * r, 0.0, a.sin() * r)
                })
                .collect()
        };
        let (r1, r2) = (ring(radius * 0.45), ring(radius));
        let mid = color.gamma_multiply(0.55);
        let clear = Color32::TRANSPARENT;
        let anchor_depth = self.depth_of(center) + radius * 0.6;
        let prev = self.group;
        self.group = Some(prev.unwrap_or(anchor_depth));
        for i in 0..n {
            let j = (i + 1) % n;
            self.triangle([center, r1[i], r1[j]], [color, mid, mid]);
            self.triangle([r1[i], r2[i], r2[j]], [mid, clear, clear]);
            self.triangle([r1[i], r2[j], r1[j]], [mid, clear, mid]);
        }
        self.group = prev;
    }

    /// A grid of lines on the XZ plane at height `y`, `size` wide with `divisions` cells, fading
    /// out towards the edges.
    pub fn grid(&mut self, y: f32, size: f32, divisions: usize, color: Color32) {
        let h = size / 2.0;
        let d = divisions.max(1);
        let pieces = 8;
        for i in 0..=d {
            let k = -h + size * i as f32 / d as f32;
            for axis in 0..2 {
                for s in 0..pieces {
                    let (t0, t1) = (-h + size * s as f32 / pieces as f32, -h + size * (s + 1) as f32 / pieces as f32);
                    let (a, b) = if axis == 0 { (Vec3::new(k, y, t0), Vec3::new(k, y, t1)) } else { (Vec3::new(t0, y, k), Vec3::new(t1, y, k)) };
                    let fa = |p: Vec3| {
                        let r = (p.x * p.x + p.z * p.z).sqrt() / h;
                        color.gamma_multiply((1.0 - r * r).clamp(0.0, 1.0))
                    };
                    self.line_gradient(a, b, 1.0, fa(a), fa(b));
                }
            }
        }
    }

    /// Text lying on a plane in 3D. `right` and `up` span the plane (unit length); `height` is the
    /// font size in world units; `anchor` picks which point of the text sits at `origin`.
    /// One-sided: invisible from behind.
    #[allow(clippy::too_many_arguments)]
    pub fn text(&mut self, text: &str, origin: Vec3, right: Vec3, up: Vec3, height: f32, color: Color32, anchor: Align2) {
        self.text_fitted(text, origin, right, up, height, f32::INFINITY, color, anchor);
    }

    /// Like [`Scene::text`], but shrinks the text so it is at most `max_width` world units wide.
    #[allow(clippy::too_many_arguments)]
    pub fn text_fitted(&mut self, text: &str, origin: Vec3, right: Vec3, up: Vec3, height: f32, max_width: f32, color: Color32, anchor: Align2) {
        let normal = right.cross(up);
        let to_cam = if self.proj.ortho { -self.camera.forward() } else { self.camera.position - origin };
        if normal.dot(to_cam) <= 0.0 || text.is_empty() {
            return;
        }
        let d = self.depth_of(origin);
        if d < self.camera.near {
            return;
        }
        let px = height * self.proj.scale_at(d);
        if px < 3.0 {
            return;
        }
        let fs = ((px / 2.0).round() * 2.0).clamp(6.0, 160.0);
        let galley = self.painter.layout_no_wrap(text.to_owned(), FontId::proportional(fs), color);
        let size = galley.size();
        let mut k = height / fs;
        if size.x * k > max_width {
            k = max_width / size.x.max(1e-3);
        }
        let off = vec2(-size.x * anchor.x().to_factor(), -size.y * anchor.y().to_factor());
        let color = self.fog_color(color, d);
        let [tw, th] = self.painter.fonts(|f| f.font_image_size());
        let norm = vec2(1.0 / tw.max(1) as f32, 1.0 / th.max(1) as f32);
        let mut out = EMesh::default();
        for row in &galley.rows {
            let m = &row.visuals.mesh;
            if m.is_empty() {
                continue;
            }
            out.texture_id = m.texture_id;
            let base = out.vertices.len() as u32;
            for vx in &m.vertices {
                let g = vx.pos + row.pos.to_vec2() + off;
                let w = origin + right * (g.x * k) - up * (g.y * k);
                let v = self.view.transform_point(w);
                if -v.z < self.camera.near {
                    return;
                }
                let a = vx.color.a() as f32 / 255.0;
                out.vertices.push(Vertex { pos: self.proj.project_unchecked(v), uv: pos2(vx.uv.x * norm.x, vx.uv.y * norm.y), color: color.gamma_multiply(a) });
            }
            out.indices.extend(m.indices.iter().map(|i| i + base));
        }
        if !out.is_empty() {
            self.push(d, Prim::Text(Box::new(out)));
        }
    }

    /// A screen-aligned text label at a world point, with a fixed font size in points.
    pub fn label(&mut self, text: &str, at: Vec3, size: f32, color: Color32, anchor: Align2) {
        let Some((s, d)) = self.project(at) else { return };
        let galley = self.painter.layout_no_wrap(text.to_owned(), FontId::proportional(size), color);
        let sz = galley.size();
        let off = vec2(-sz.x * anchor.x().to_factor(), -sz.y * anchor.y().to_factor());
        let ppp = self.painter.pixels_per_point();
        let origin = s + off;
        let origin = pos2((origin.x * ppp).round() / ppp, (origin.y * ppp).round() / ppp);
        let color = self.fog_color(color, d);
        let [tw, th] = self.painter.fonts(|f| f.font_image_size());
        let norm = vec2(1.0 / tw.max(1) as f32, 1.0 / th.max(1) as f32);
        let mut out = EMesh::default();
        for row in &galley.rows {
            let m = &row.visuals.mesh;
            if m.is_empty() {
                continue;
            }
            out.texture_id = m.texture_id;
            let base = out.vertices.len() as u32;
            for vx in &m.vertices {
                let a = vx.color.a() as f32 / 255.0;
                out.vertices.push(Vertex { pos: origin + row.pos.to_vec2() + vx.pos.to_vec2(), uv: pos2(vx.uv.x * norm.x, vx.uv.y * norm.y), color: color.gamma_multiply(a) });
            }
            out.indices.extend(m.indices.iter().map(|i| i + base));
        }
        if !out.is_empty() {
            self.push(d, Prim::Text(Box::new(out)));
        }
    }

    /// Sort and paint everything. Called automatically by [`crate::Viewport3D`].
    pub fn paint(mut self) -> Stats {
        let mut keys = std::mem::take(&mut self.keys);
        keys.sort_unstable_by(|a, b| b.depth.total_cmp(&a.depth).then(a.idx.cmp(&b.idx)));
        let mut shapes: Vec<Shape> = Vec::new();
        let mut m = EMesh::default();
        let flush = |m: &mut EMesh, shapes: &mut Vec<Shape>| {
            if !m.is_empty() {
                shapes.push(Shape::mesh(std::mem::take(m)));
            }
        };
        for k in &keys {
            if m.vertices.len() > 60_000 {
                flush(&mut m, &mut shapes);
            }
            match &self.prims[k.idx as usize] {
                Prim::Tri(p, c) => {
                    let b = m.vertices.len() as u32;
                    for i in 0..3 {
                        m.vertices.push(Vertex { pos: p[i], uv: WHITE_UV, color: c[i] });
                    }
                    m.indices.extend([b, b + 1, b + 2]);
                }
                Prim::Line { a, b, wa, wb, ca, cb } => emit_line(&mut m, *a, *b, *wa, *wb, *ca, *cb),
                Prim::Disc { c, r, color, soft } => emit_disc(&mut m, *c, *r, *color, *soft),
                Prim::Ball { c, r, color, light, spec, ambient } => emit_ball(&mut m, *c, *r, *color, *light, *spec, *ambient),
                Prim::Text(tm) => {
                    flush(&mut m, &mut shapes);
                    shapes.push(Shape::mesh((**tm).clone()));
                }
            }
        }
        flush(&mut m, &mut shapes);
        self.stats.batches = shapes.len();
        self.painter.extend(shapes);
        self.stats
    }
}

fn additive_c(c: Color32) -> Color32 {
    additive(c)
}

fn clip_near(v: &[CVert; 3], near: f32) -> Vec<CVert> {
    let mut out = Vec::with_capacity(4);
    for i in 0..3 {
        let (a, b) = (v[i], v[(i + 1) % 3]);
        let (da, db) = (-a.v.z - near, -b.v.z - near);
        if da >= 0.0 {
            out.push(a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            let t = da / (da - db);
            let c: [f32; 4] = std::array::from_fn(|k| a.c[k] + (b.c[k] - a.c[k]) * t);
            out.push(CVert { v: a.v.lerp(b.v, t), c });
        }
    }
    out
}

fn clip_segment(a: Vec3, b: Vec3, near: f32) -> Option<(Vec3, Vec3)> {
    let (da, db) = (-a.z - near, -b.z - near);
    match (da >= 0.0, db >= 0.0) {
        (true, true) => Some((a, b)),
        (false, false) => None,
        (true, false) => Some((a, a.lerp(b, da / (da - db)))),
        (false, true) => Some((a.lerp(b, da / (da - db)), b)),
    }
}

fn emit_line(m: &mut EMesh, a: Pos2, b: Pos2, wa: f32, wb: f32, ca: Color32, cb: Color32) {
    let d = b - a;
    let len = d.length();
    if len < 1e-3 {
        return;
    }
    let dir = d / len;
    let n = dir.rot90();
    let thin = |w: f32, c: Color32| if w < 1.0 { (0.5, c.gamma_multiply(w.max(0.0))) } else { (w * 0.5, c) };
    let (ha, ca) = thin(wa, ca);
    let (hb, cb) = thin(wb, cb);
    let f = 0.7;
    let base = m.vertices.len() as u32;
    for (p, h, c) in [(a, ha, ca), (b, hb, cb)] {
        m.vertices.push(Vertex { pos: p + n * (h + f), uv: WHITE_UV, color: Color32::TRANSPARENT });
        m.vertices.push(Vertex { pos: p + n * h, uv: WHITE_UV, color: c });
        m.vertices.push(Vertex { pos: p - n * h, uv: WHITE_UV, color: c });
        m.vertices.push(Vertex { pos: p - n * (h + f), uv: WHITE_UV, color: Color32::TRANSPARENT });
    }
    for s in 0..3 {
        let (i, j) = (base + s, base + 4 + s);
        m.indices.extend([i, j, j + 1, i, j + 1, i + 1]);
    }
}

fn emit_disc(m: &mut EMesh, c: Pos2, r: f32, color: Color32, soft: bool) {
    let n: u32 = if r < 1.5 {
        4
    } else if r < 4.0 {
        8
    } else if r < 12.0 {
        14
    } else {
        24
    };
    let base = m.vertices.len() as u32;
    m.vertices.push(Vertex { pos: c, uv: WHITE_UV, color });
    let (r1, c1, r2) = if soft { (r * 0.4, color.gamma_multiply(0.45), r) } else { ((r - 0.35).max(0.3), color, r + 0.7) };
    let small = !soft && r < 1.0;
    let c1 = if small { color.gamma_multiply(r.max(0.15)) } else { c1 };
    for i in 0..n {
        let a = i as f32 / n as f32 * TAU;
        let d = vec2(a.cos(), a.sin());
        m.vertices.push(Vertex { pos: c + d * r1, uv: WHITE_UV, color: c1 });
        m.vertices.push(Vertex { pos: c + d * r2, uv: WHITE_UV, color: Color32::TRANSPARENT });
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let (ai, ao, bi, bo) = (base + 1 + i * 2, base + 2 + i * 2, base + 1 + j * 2, base + 2 + j * 2);
        m.indices.extend([base, ai, bi, ai, ao, bo, ai, bo, bi]);
    }
}

fn emit_ball(m: &mut EMesh, c: Pos2, r: f32, color: Color32, l: [f32; 3], spec: f32, ambient: f32) {
    let segs: u32 = if r < 3.0 { 8 } else if r < 10.0 { 16 } else { 28 };
    let rings: u32 = if r < 3.0 { 2 } else if r < 10.0 { 4 } else { 6 };
    let base_c = Rgb::from(color);
    let alpha = color.a() as f32 / 255.0;
    let additive = color.a() == 0;
    let alpha = if additive { 1.0 } else { alpha };
    let h = {
        let (x, y, z) = (l[0], l[1], l[2] + 1.0);
        let n = (x * x + y * y + z * z).sqrt().max(1e-6);
        [x / n, y / n, z / n]
    };
    let shade = |nx: f32, ny: f32, nz: f32| -> Color32 {
        let ndl = (nx * l[0] + ny * l[1] + nz * l[2]).max(0.0);
        let s = (nx * h[0] + ny * h[1] + nz * h[2]).max(0.0).powf(28.0) * spec;
        let rim = (1.0 - nz).powi(3) * 0.25;
        let k = ambient + (1.0 - ambient) * ndl;
        base_c.scale(k).add(Rgb(s + rim * base_c.0, s + rim * base_c.1, s + rim * base_c.2)).to_color(alpha, additive)
    };
    let base = m.vertices.len() as u32;
    m.vertices.push(Vertex { pos: c, uv: WHITE_UV, color: shade(0.0, 0.0, 1.0) });
    for k in 1..=rings {
        let s = (k as f32 / rings as f32).powf(0.75);
        let rr = if k == rings { (r - 0.3).max(0.2) } else { r * s };
        let s = rr / r;
        let nz = (1.0 - s * s).max(0.0).sqrt();
        for i in 0..segs {
            let a = i as f32 / segs as f32 * TAU;
            let (ca, sa) = (a.cos(), a.sin());
            // screen y is down; view-space y is up
            m.vertices.push(Vertex { pos: c + vec2(ca, sa) * rr, uv: WHITE_UV, color: shade(ca * s, -sa * s, nz) });
        }
    }
    // feather ring
    for i in 0..segs {
        let a = i as f32 / segs as f32 * TAU;
        m.vertices.push(Vertex { pos: c + vec2(a.cos(), a.sin()) * (r + 0.6), uv: WHITE_UV, color: Color32::TRANSPARENT });
    }
    for i in 0..segs {
        let j = (i + 1) % segs;
        m.indices.extend([base, base + 1 + i, base + 1 + j]);
    }
    for k in 1..=rings {
        let a0 = base + 1 + (k - 1) * segs;
        let b0 = base + 1 + k * segs;
        for i in 0..segs {
            let j = (i + 1) % segs;
            m.indices.extend([a0 + i, b0 + i, b0 + j, a0 + i, b0 + j, a0 + j]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::vec3;

    #[test]
    fn near_clip_splits_triangle() {
        let mk = |z: f32| CVert { v: vec3(0.0, 0.0, z), c: [1.0; 4] };
        let tri = [mk(-1.0), CVert { v: vec3(1.0, 0.0, 1.0), ..mk(0.0) }, CVert { v: vec3(0.0, 1.0, -2.0), ..mk(0.0) }];
        let poly = clip_near(&tri, 0.1);
        assert_eq!(poly.len(), 4);
        assert!(poly.iter().all(|p| -p.v.z >= 0.1 - 1e-5));
        assert!(clip_segment(vec3(0.0, 0.0, 1.0), vec3(0.0, 0.0, 2.0), 0.1).is_none());
        let (a, _) = clip_segment(vec3(0.0, 0.0, 1.0), vec3(0.0, 0.0, -1.0), 0.1).unwrap();
        assert!((a.z + 0.1).abs() < 1e-5);
    }

    /// `cargo test -p glowkit-3d --release -- --ignored --nocapture bench` prints the frame cost.
    #[test]
    #[ignore]
    fn bench_20k_triangles() {
        let ctx = egui::Context::default();
        let knot = Mesh::torus_knot(2, 3, 1.0, 0.3, 1000, 20); // 40k triangles, ~20k front-facing
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let rect = Rect::from_min_size(Pos2::ZERO, vec2(900.0, 400.0));
            let n = 30;
            let t0 = std::time::Instant::now();
            let mut drawn = 0;
            for i in 0..n {
                let mut s = Scene::new(ui.painter().clone(), rect, Camera::looking_at(vec3(0.0, 1.0, 4.0), Vec3::ZERO));
                s.draw(&knot, Transform::IDENTITY.with_euler(0.0, i as f32 * 0.1, 0.0), &Material::smooth(Color32::WHITE).wire(crate::Wire::new(Color32::WHITE)));
                drawn = s.stats().triangles_drawn;
                s.paint();
            }
            println!("{} tris in, {drawn} drawn: {:.2} ms/frame", knot.triangle_count(), t0.elapsed().as_secs_f64() * 1000.0 / n as f64);
        });
        out.textures_delta.clear();
    }

    #[test]
    fn renders_cube_sorted() {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let rect = Rect::from_min_size(Pos2::ZERO, vec2(200.0, 200.0));
            let mut s = Scene::new(ui.painter().clone(), rect, Camera::looking_at(vec3(2.0, 2.0, 3.0), Vec3::ZERO));
            s.draw(&Mesh::cube(1.0), Transform::IDENTITY, &Material::smooth(Color32::WHITE));
            let st = s.stats();
            assert_eq!(st.triangles_in, 12);
            // a cube shows at most three faces
            assert!(st.triangles_drawn <= 6 && st.triangles_drawn >= 4);
            let st = s.paint();
            assert_eq!(st.batches, 1);
        });
        out.textures_delta.clear();
    }
}
