//! Shared machinery for the snap_b module: cached text coverage masks (so headlines can be filled
//! with per-pixel materials), mask-driven meshes, a tiny 3D camera, warped textured cards, a
//! procedural photograph with CPU-side pixels, scroll / autoplay drivers and small paint helpers.

use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{
    pos2, vec2, Color32, ColorImage, Context, FontFamily, FontId, Id, Painter, Pos2, Rect, Response, Shape,
    TextureHandle, TextureId, TextureOptions, Ui, Vec2,
};
use std::sync::Arc;

// ---------------------------------------------------------------------------------------------
// Fonts

/// A heavy display font: the gallery's `"display"` family when installed, else proportional.
pub(crate) fn display_font(ui: &Ui, size: f32) -> FontId {
    let fam = FontFamily::Name("display".into());
    let has = ui.ctx().fonts(|f| f.families().contains(&fam));
    FontId::new(size, if has { fam } else { FontFamily::Proportional })
}

/// The gallery's `"semibold"` family when installed, else proportional.
pub(crate) fn semibold_font(ui: &Ui, size: f32) -> FontId {
    let fam = FontFamily::Name("semibold".into());
    let has = ui.ctx().fonts(|f| f.families().contains(&fam));
    FontId::new(size, if has { fam } else { FontFamily::Proportional })
}

// ---------------------------------------------------------------------------------------------
// Text masks

/// A rasterised line of text: a white coverage texture plus the coverage and a softened copy on
/// the CPU (used to derive bevel normals and to sample shapes).
pub(crate) struct TextMask {
    pub tex: TextureId,
    _handle: TextureHandle,
    /// Size in points (including padding).
    pub size: Vec2,
    /// Pixel dimensions.
    pub w: usize,
    pub h: usize,
    /// Coverage 0..1, row-major.
    pub alpha: Vec<f32>,
    /// Blurred coverage 0..1 (a cheap height field for bevels).
    pub soft: Vec<f32>,
    /// Blur radius (pixels) of `soft`.
    pub soft_r: f32,
}

impl TextMask {
    fn idx(&self, x: f32, y: f32) -> (usize, usize, f32, f32) {
        let fx = (x * (self.w as f32) - 0.5).clamp(0.0, self.w as f32 - 1.001);
        let fy = (y * (self.h as f32) - 0.5).clamp(0.0, self.h as f32 - 1.001);
        (fx as usize, fy as usize, fx.fract(), fy.fract())
    }
    fn bilinear(&self, buf: &[f32], u: f32, v: f32) -> f32 {
        let (x, y, fx, fy) = self.idx(u, v);
        let x1 = (x + 1).min(self.w - 1);
        let y1 = (y + 1).min(self.h - 1);
        let a = buf[y * self.w + x];
        let b = buf[y * self.w + x1];
        let c = buf[y1 * self.w + x];
        let d = buf[y1 * self.w + x1];
        let top = a + (b - a) * fx;
        let bot = c + (d - c) * fx;
        top + (bot - top) * fy
    }
    /// Coverage at normalized `uv` (0..1 over the mask).
    pub fn coverage(&self, u: f32, v: f32) -> f32 {
        self.bilinear(&self.alpha, u, v)
    }
    /// Softened coverage at `uv`.
    pub fn height(&self, u: f32, v: f32) -> f32 {
        self.bilinear(&self.soft, u, v)
    }
    /// Surface normal (x, y) of the softened coverage, roughly -1..1 at the letter edges.
    pub fn normal(&self, u: f32, v: f32) -> Vec2 {
        let du = self.soft_r * 0.6 / self.w as f32;
        let dv = self.soft_r * 0.6 / self.h as f32;
        let gx = self.height(u + du, v) - self.height(u - du, v);
        let gy = self.height(u, v + dv) - self.height(u, v - dv);
        let n = vec2(-gx, -gy) * 1.7;
        let l = n.length();
        if l > 1.0 {
            n / l
        } else {
            n
        }
    }
    /// Max coverage inside the normalized rect (coarse occupancy test).
    pub fn any_in(&self, u0: f32, v0: f32, u1: f32, v1: f32) -> bool {
        let x0 = ((u0 * self.w as f32).floor() as isize - 1).clamp(0, self.w as isize - 1) as usize;
        let x1 = ((u1 * self.w as f32).ceil() as isize + 1).clamp(0, self.w as isize - 1) as usize;
        let y0 = ((v0 * self.h as f32).floor() as isize - 1).clamp(0, self.h as isize - 1) as usize;
        let y1 = ((v1 * self.h as f32).ceil() as isize + 1).clamp(0, self.h as isize - 1) as usize;
        for y in y0..=y1 {
            let row = &self.alpha[y * self.w..y * self.w + self.w];
            if row[x0..=x1].iter().any(|a| *a > 0.01) {
                return true;
            }
        }
        false
    }
}

fn hash_u(x: u32, y: u32, s: u32) -> f32 {
    let mut h = x.wrapping_mul(0x8da6_b343) ^ y.wrapping_mul(0xd816_3841) ^ s.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h & 0xffff) as f32 / 65535.0
}

fn box_blur(src: &[f32], w: usize, h: usize, r: usize) -> Vec<f32> {
    let mut tmp = vec![0.0; w * h];
    let mut out = vec![0.0; w * h];
    let norm = 1.0 / (2 * r + 1) as f32;
    for y in 0..h {
        let row = &src[y * w..y * w + w];
        let mut acc = 0.0;
        acc += row.iter().take(r.min(w - 1) + 1).sum::<f32>();
        acc += row[0] * r as f32;
        for x in 0..w {
            tmp[y * w + x] = acc * norm;
            let add = row[(x + r + 1).min(w - 1)];
            let sub = row[x.saturating_sub(r)];
            acc += add - sub;
        }
    }
    for x in 0..w {
        let mut acc = 0.0;
        for k in 0..=r.min(h - 1) {
            acc += tmp[k * w + x];
        }
        acc += tmp[x] * r as f32;
        for y in 0..h {
            out[y * w + x] = acc * norm;
            let add = tmp[(y + r + 1).min(h - 1) * w + x];
            let sub = tmp[y.saturating_sub(r) * w + x];
            acc += add - sub;
        }
    }
    out
}

/// Rasterise `text` in `font` into a cached [`TextMask`]. `grain` bakes film grain into the
/// texture's brightness (multiply it by vertex colors to get a grainy fill).
pub(crate) fn text_mask(ctx: &Context, text: &str, font: &FontId, grain: bool) -> Arc<TextMask> {
    let ppp = ctx.pixels_per_point();
    let key = Id::unique(("snap_b_text_mask", text, font.size.to_bits(), format!("{:?}", font.family), ppp.to_bits(), grain));
    if let Some(m) = ctx.data(|d| d.get_temp::<Arc<TextMask>>(key)) {
        return m;
    }
    let galley = ctx.fonts_mut(|f| f.layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE));
    let atlas = ctx.fonts(|f| f.image());
    let [aw, ah] = atlas.size;
    let pad = (font.size * 0.12).max(4.0);
    let size = galley.size() + Vec2::splat(pad * 2.0);
    let w = ((size.x * ppp).ceil() as usize).max(4);
    let h = ((size.y * ppp).ceil() as usize).max(4);
    let mut alpha = vec![0.0f32; w * h];
    // (raw glyph coverage, emboldened below)
    let sample = |u: f32, v: f32| -> f32 {
        let x = (u - 0.5).clamp(0.0, aw as f32 - 1.001);
        let y = (v - 0.5).clamp(0.0, ah as f32 - 1.001);
        let (xi, yi) = (x as usize, y as usize);
        let (fx, fy) = (x.fract(), y.fract());
        let p = |xx: usize, yy: usize| atlas.pixels[yy.min(ah - 1) * aw + xx.min(aw - 1)].a() as f32 / 255.0;
        let top = p(xi, yi) + (p(xi + 1, yi) - p(xi, yi)) * fx;
        let bot = p(xi, yi + 1) + (p(xi + 1, yi + 1) - p(xi, yi + 1)) * fx;
        top + (bot - top) * fy
    };
    for row in &galley.rows {
        let verts = &row.visuals.mesh.vertices;
        let off = row.pos.to_vec2() + Vec2::splat(pad);
        for q in verts.chunks_exact(4) {
            let (mut p0, mut p1) = (pos2(f32::MAX, f32::MAX), pos2(f32::MIN, f32::MIN));
            let (mut t0, mut t1) = (pos2(f32::MAX, f32::MAX), pos2(f32::MIN, f32::MIN));
            for v in q {
                p0 = p0.min(v.pos);
                p1 = p1.max(v.pos);
                t0 = t0.min(v.uv);
                t1 = t1.max(v.uv);
            }
            if p1.x - p0.x < 0.1 || p1.y - p0.y < 0.1 {
                continue;
            }
            let x0 = ((p0.x + off.x) * ppp).floor().max(0.0) as usize;
            let x1 = (((p1.x + off.x) * ppp).ceil() as usize).min(w);
            let y0 = ((p0.y + off.y) * ppp).floor().max(0.0) as usize;
            let y1 = (((p1.y + off.y) * ppp).ceil() as usize).min(h);
            for py in y0..y1 {
                let ty = ((py as f32 + 0.5) / ppp - off.y - p0.y) / (p1.y - p0.y);
                if !(0.0..=1.0).contains(&ty) {
                    continue;
                }
                let v = t0.y + ty * (t1.y - t0.y);
                for px in x0..x1 {
                    let tx = ((px as f32 + 0.5) / ppp - off.x - p0.x) / (p1.x - p0.x);
                    if !(0.0..=1.0).contains(&tx) {
                        continue;
                    }
                    let u = t0.x + tx * (t1.x - t0.x);
                    let a = sample(u, v);
                    let cell = &mut alpha[py * w + px];
                    *cell = cell.max(a);
                }
            }
        }
    }
    // embolden: blur the coverage and re-threshold below the midpoint (dilates and rounds)
    let br = ((font.size * ppp * 0.04).round() as usize).clamp(1, 16);
    let blurred = box_blur(&box_blur(&alpha, w, h, br), w, h, br);
    let alpha: Vec<f32> = blurred
        .iter()
        .map(|v| {
            let x = ((v - 0.17) / 0.16).clamp(0.0, 1.0);
            x * x * (3.0 - 2.0 * x)
        })
        .collect();
    let r = ((font.size * ppp * 0.05).round() as usize).clamp(2, 14);
    let soft = box_blur(&box_blur(&alpha, w, h, r), w, h, r);
    let pixels: Vec<Color32> = alpha
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let (x, y) = ((i % w) as u32, (i / w) as u32);
            let (g, aa) = if grain {
                let n = hash_u(x, y, 7);
                let n2 = hash_u(x, y, 13);
                (0.5 + 0.5 * n, a * (0.72 + 0.28 * n2))
            } else {
                (1.0, *a)
            };
            let av = (aa * 255.0).round() as u8;
            let cv = (aa * g * 255.0).round() as u8;
            Color32::from_rgba_premultiplied(cv, cv, cv, av)
        })
        .collect();
    let handle = ctx.load_texture(
        format!("snap_b_mask_{}", key.value()),
        ColorImage::new([w, h], pixels),
        TextureOptions::LINEAR,
    );
    let m = Arc::new(TextMask { tex: handle.id(), _handle: handle, size, w, h, alpha, soft, soft_r: r as f32 });
    ctx.data_mut(|d| d.insert_temp(key, m.clone()));
    m
}

/// Paint `mask` with its top-left at `origin` as a grid mesh of `cell`-point quads, each vertex
/// colored by `color(uv, screen_pos)`. Only cells that contain ink are emitted.
pub(crate) fn paint_mask_smooth(
    painter: &Painter,
    mask: &TextMask,
    origin: Pos2,
    cell: f32,
    color: impl Fn(Vec2, Pos2) -> Color32,
) {
    let cols = ((mask.size.x / cell).ceil() as usize).max(1);
    let rows = ((mask.size.y / cell).ceil() as usize).max(1);
    let mut m = Mesh::with_texture(mask.tex);
    let stride = cols + 1;
    let mut index = vec![u32::MAX; stride * (rows + 1)];
    for y in 0..rows {
        for x in 0..cols {
            let u0 = x as f32 / cols as f32;
            let u1 = (x + 1) as f32 / cols as f32;
            let v0 = y as f32 / rows as f32;
            let v1 = (y + 1) as f32 / rows as f32;
            if !mask.any_in(u0, v0, u1, v1) {
                continue;
            }
            let mut ids = [0u32; 4];
            for (k, (dx, dy)) in [(0, 0), (1, 0), (1, 1), (0, 1)].into_iter().enumerate() {
                let gi = (y + dy) * stride + x + dx;
                if index[gi] == u32::MAX {
                    let uv = vec2((x + dx) as f32 / cols as f32, (y + dy) as f32 / rows as f32);
                    let p = origin + vec2(uv.x * mask.size.x, uv.y * mask.size.y);
                    index[gi] = m.vertices.len() as u32;
                    m.vertices.push(Vertex { pos: p, uv: uv.to_pos2(), color: color(uv, p) });
                }
                ids[k] = index[gi];
            }
            m.add_triangle(ids[0], ids[1], ids[2]);
            m.add_triangle(ids[0], ids[2], ids[3]);
        }
    }
    painter.add(Shape::mesh(m));
}

/// Like [`paint_mask_smooth`] but every cell is a flat color (`color(cell_center_uv, cell_index)`),
/// for dither and halftone looks. Return `None` to skip a cell.
pub(crate) fn paint_mask_cells(
    painter: &Painter,
    mask: &TextMask,
    origin: Pos2,
    cell: f32,
    color: impl Fn(Vec2, (usize, usize)) -> Option<Color32>,
) {
    let cols = ((mask.size.x / cell).ceil() as usize).max(1);
    let rows = ((mask.size.y / cell).ceil() as usize).max(1);
    let (cw, ch) = (1.0 / cols as f32, 1.0 / rows as f32);
    let mut m = Mesh::with_texture(mask.tex);
    for y in 0..rows {
        for x in 0..cols {
            let (u0, v0) = (x as f32 * cw, y as f32 * ch);
            if !mask.any_in(u0, v0, u0 + cw, v0 + ch) {
                continue;
            }
            let Some(c) = color(vec2(u0 + cw * 0.5, v0 + ch * 0.5), (x, y)) else { continue };
            let i = m.vertices.len() as u32;
            for (dx, dy) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
                let uv = pos2(u0 + dx * cw, v0 + dy * ch);
                let p = origin + vec2(uv.x * mask.size.x, uv.y * mask.size.y);
                m.vertices.push(Vertex { pos: p, uv, color: c });
            }
            m.add_triangle(i, i + 1, i + 2);
            m.add_triangle(i, i + 2, i + 3);
        }
    }
    painter.add(Shape::mesh(m));
}

/// Paint the mask as a flat tinted texture (e.g. for glows and shadows), scaled by `scale`
/// around its center.
pub(crate) fn paint_mask_flat(painter: &Painter, mask: &TextMask, center: Pos2, scale: f32, color: Color32) {
    let r = Rect::from_center_size(center, mask.size * scale);
    painter.image(mask.tex, r, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), color);
}

/// Soft halo of the mask: several enlarged / offset copies at low alpha.
pub(crate) fn mask_glow(painter: &Painter, mask: &TextMask, center: Pos2, radius: f32, color: Color32) {
    let taps = 12;
    for i in 0..taps {
        let a = i as f32 / taps as f32 * std::f32::consts::TAU;
        let rr = radius * if i % 2 == 0 { 1.0 } else { 0.55 };
        paint_mask_flat(painter, mask, center + vec2(a.cos(), a.sin()) * rr, 1.0, color.gamma_multiply(1.0 / taps as f32 * 2.0));
    }
}

/// The 4x4 Bayer threshold at a cell, in `0..1`.
pub(crate) fn bayer4(x: usize, y: usize) -> f32 {
    const B: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    (B[y & 3][x & 3] as f32 + 0.5) / 16.0
}

// ---------------------------------------------------------------------------------------------
// 3D

/// A 3D point.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub(crate) fn v3(x: f32, y: f32, z: f32) -> V3 {
    V3 { x, y, z }
}

impl V3 {
    pub fn add(self, o: V3) -> V3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
    pub fn sub(self, o: V3) -> V3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
    pub fn mul(self, k: f32) -> V3 {
        v3(self.x * k, self.y * k, self.z * k)
    }
    /// Rotate about the Y axis (yaw).
    pub fn rot_y(self, a: f32) -> V3 {
        let (s, c) = a.sin_cos();
        v3(self.x * c + self.z * s, self.y, -self.x * s + self.z * c)
    }
    /// Rotate about the X axis (pitch).
    pub fn rot_x(self, a: f32) -> V3 {
        let (s, c) = a.sin_cos();
        v3(self.x, self.y * c - self.z * s, self.y * s + self.z * c)
    }
    /// Rotate about the Z axis (roll).
    pub fn rot_z(self, a: f32) -> V3 {
        let (s, c) = a.sin_cos();
        v3(self.x * c - self.y * s, self.x * s + self.y * c, self.z)
    }
}

/// Pinhole camera looking down +z (positive z goes away from the viewer).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cam {
    pub center: Pos2,
    pub focal: f32,
}

impl Cam {
    /// Project; returns screen point and scale (1 at z = 0).
    pub fn project(&self, p: V3) -> (Pos2, f32) {
        let s = self.focal / (self.focal + p.z).max(1.0);
        (self.center + vec2(p.x, p.y) * s, s)
    }
    pub fn pt(&self, p: V3) -> Pos2 {
        self.project(p).0
    }
}

/// Paint a textured (or flat) card mapped by `map(u, v)` (u, v in 0..1) with per-vertex tint.
///
/// With `radius > 0` the card is a rounded rect (corner radius in local points of a card of
/// `size`) meshed as concentric rings, dense enough (`nx`, `ny`) to follow curved maps; the
/// edge gets a sub-pixel feather so it stays smooth. With `radius == 0` it is a plain grid.
#[allow(clippy::too_many_arguments)]
pub(crate) fn surface_grid(
    painter: &Painter,
    tex: Option<TextureId>,
    uv: Rect,
    nx: usize,
    ny: usize,
    size: Vec2,
    radius: f32,
    map: impl Fn(f32, f32) -> Pos2,
    tint: impl Fn(f32, f32) -> Color32,
) {
    let nx = nx.max(1);
    let ny = ny.max(1);
    let mut m = Mesh::with_texture(tex.unwrap_or_default());
    let tuv = |u: f32, v: f32| {
        if tex.is_some() {
            pos2(uv.min.x + u * uv.width(), uv.min.y + v * uv.height())
        } else {
            WHITE_UV
        }
    };
    if size.x <= 0.5 || size.y <= 0.5 {
        return;
    }
    let r = radius.min(size.x / 2.0).min(size.y / 2.0);
    if r <= 0.5 {
        for j in 0..=ny {
            for i in 0..=nx {
                let (u, v) = (i as f32 / nx as f32, j as f32 / ny as f32);
                m.vertices.push(Vertex { pos: map(u, v), uv: tuv(u, v), color: tint(u, v) });
            }
        }
        let w = (nx + 1) as u32;
        for j in 0..ny as u32 {
            for i in 0..nx as u32 {
                let a = j * w + i;
                m.add_triangle(a, a + 1, a + w + 1);
                m.add_triangle(a, a + w + 1, a + w);
            }
        }
        painter.add(Shape::mesh(m));
        return;
    }
    // rounded outline in local space, subdivided so long edges follow the map
    let local = Rect::from_min_size(Pos2::ZERO, size);
    let seg = (size.x / nx as f32).min(size.y / ny as f32).max(2.0);
    let outline = glowkit_core::paint::subdivide_closed(&glowkit_core::paint::rounded_rect_points(local, r, 5), seg);
    let n = outline.len();
    let rings = (nx.max(ny) / 2).max(1);
    let c = local.center();
    let uvl = |p: Pos2| (p.x / size.x, p.y / size.y);
    let (cu, cv) = uvl(c);
    m.vertices.push(Vertex { pos: map(cu, cv), uv: tuv(cu, cv), color: tint(cu, cv) });
    for k in 1..=rings {
        let s = k as f32 / rings as f32;
        for o in &outline {
            let p = c + (*o - c) * s;
            let (u, v) = uvl(p);
            m.vertices.push(Vertex { pos: map(u, v), uv: tuv(u, v), color: tint(u, v) });
        }
    }
    for i in 0..n {
        m.add_triangle(0, 1 + i as u32, 1 + ((i + 1) % n) as u32);
    }
    for k in 1..rings {
        let a0 = 1 + (k - 1) * n;
        let b0 = 1 + k * n;
        for i in 0..n {
            let j = (i + 1) % n;
            m.add_triangle((a0 + i) as u32, (b0 + i) as u32, (b0 + j) as u32);
            m.add_triangle((a0 + i) as u32, (b0 + j) as u32, (a0 + j) as u32);
        }
    }
    // feather: push a transparent copy of the outer ring ~0.8 px outward in screen space
    let outer0 = 1 + (rings - 1) * n;
    let f0 = m.vertices.len();
    for i in 0..n {
        let p = m.vertices[outer0 + i].pos;
        let prev = m.vertices[outer0 + (i + n - 1) % n].pos;
        let next = m.vertices[outer0 + (i + 1) % n].pos;
        let mut d = (next - prev).rot90().normalized();
        let inward = if rings > 1 { m.vertices[1 + (rings - 2) * n + i].pos } else { m.vertices[0].pos };
        if d.dot(p - inward) < 0.0 {
            d = -d;
        }
        let v = m.vertices[outer0 + i];
        m.vertices.push(Vertex { pos: p + d * 0.9, uv: v.uv, color: Color32::TRANSPARENT });
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let (a, an) = ((outer0 + i) as u32, (outer0 + j) as u32);
        let (b, bn) = ((f0 + i) as u32, (f0 + j) as u32);
        m.add_triangle(a, b, bn);
        m.add_triangle(a, bn, an);
    }
    painter.add(Shape::mesh(m));
}

/// A UV rect that crops a `tex_aspect` texture to fill a `target_aspect` box.
pub(crate) fn cover_uv(tex_aspect: f32, target_aspect: f32) -> Rect {
    if target_aspect > tex_aspect {
        let h = tex_aspect / target_aspect;
        Rect::from_min_size(pos2(0.0, (1.0 - h) / 2.0), vec2(1.0, h))
    } else {
        let w = target_aspect / tex_aspect;
        Rect::from_min_size(pos2((1.0 - w) / 2.0, 0.0), vec2(w, 1.0))
    }
}

/// A flat textured rounded rect (via [`surface_grid`]).
pub(crate) fn image_rounded(painter: &Painter, rect: Rect, radius: f32, tex: TextureId, uv: Rect, tint: Color32) {
    surface_grid(
        painter,
        Some(tex),
        uv,
        8,
        8,
        rect.size(),
        radius,
        |u, v| rect.min + vec2(u * rect.width(), v * rect.height()),
        |_, _| tint,
    );
}

/// Convex polygon soft shadow made of stacked translucent copies.
pub(crate) fn poly_shadow(painter: &Painter, pts: &[Pos2], offset: Vec2, spread: f32, color: Color32) {
    if pts.len() < 3 {
        return;
    }
    let c = pts.iter().fold(Vec2::ZERO, |a, p| a + p.to_vec2()) / pts.len() as f32;
    let layers = 6;
    for k in 0..layers {
        let f = (k + 1) as f32 / layers as f32;
        let grow = spread * (1.0 - f);
        let poly: Vec<Pos2> = pts
            .iter()
            .map(|p| {
                let d = *p - c.to_pos2();
                let l = d.length().max(1.0);
                *p + d / l * grow + offset
            })
            .collect();
        painter.add(Shape::convex_polygon(poly, color.gamma_multiply(1.0 / layers as f32 * 1.4 * f), egui::Stroke::NONE));
    }
}

// ---------------------------------------------------------------------------------------------
// Drivers

/// Real time since the last frame (clamped).
pub(crate) fn wall_dt(ui: &Ui) -> f32 {
    ui.input(|i| i.unstable_dt).clamp(0.0, 0.1)
}

/// Seconds since the pointer last interacted with `response` (0 while interacting).
pub(crate) fn idle_time(ui: &Ui, id: Id, active: bool) -> f32 {
    let dt = wall_dt(ui);
    glowkit_core::with_state(ui, id.with("snap_b_idle"), || 10.0f32, |s| {
        if active {
            *s = 0.0;
        } else {
            *s += dt;
        }
        *s
    })
}

/// A ghost pointer that wanders a Lissajous path over `rect`, for idle autoplay.
pub(crate) fn ghost(rect: Rect, t: f32) -> Pos2 {
    let x = 0.5 + 0.36 * (t * 0.53).sin() + 0.06 * (t * 1.7).sin();
    let y = 0.5 + 0.30 * (t * 0.81 + 0.6).sin() + 0.05 * (t * 1.3).cos();
    pos2(rect.min.x + x * rect.width(), rect.min.y + y * rect.height())
}

#[derive(Clone, Copy, Default)]
struct LoopState {
    offset: f32,
    vel: f32,
    idle: f32,
}

/// An unbounded scroll / drag position (wheel + drag with inertia) that drifts at `speed` units
/// per second while idle. `scale` converts points of drag to offset units.
pub(crate) fn drive_loop(ui: &Ui, id: Id, response: &Response, horizontal: bool, speed: f32, scale: f32) -> f32 {
    let dt = wall_dt(ui);
    let hovered = response.hovered() || response.dragged();
    let wheel = if response.hovered() {
        ui.input(|i| {
            let d = i.smooth_scroll_delta();
            if horizontal {
                d.x + d.y
            } else {
                d.y
            }
        })
    } else {
        0.0
    };
    let dragging = response.dragged();
    let drag = if dragging {
        let d = response.drag_delta();
        if horizontal {
            d.x
        } else {
            d.y
        }
    } else {
        0.0
    };
    let off = glowkit_core::with_state(ui, id.with("snap_b_loop"), LoopState::default, |s| {
        if hovered {
            s.idle = 0.0;
        } else {
            s.idle += dt;
        }
        if dragging {
            s.offset -= drag * scale;
            s.vel = if dt > 0.0 { -drag * scale / dt } else { 0.0 };
        } else {
            s.offset -= wheel * scale;
            s.vel *= (-3.0 * dt).exp();
            let drift = speed * ((s.idle - 0.8) / 1.2).clamp(0.0, 1.0);
            s.offset += (s.vel + drift) * dt;
        }
        s.offset
    });
    glowkit_core::animate(ui);
    off
}

#[derive(Clone, Copy, Default)]
struct SnapState {
    pos: f32,
    vel: f32,
    target: f32,
    idle: f32,
    timer: f32,
    drag_acc: f32,
}

/// A snapping index carousel: drag / wheel / arrow keys move between integer slots with a
/// spring, autoplay steps every `interval` seconds when idle (0 = off). Returns the continuous
/// position (unbounded; wrap it yourself). `step_px` is how many points of drag equal one slot.
pub(crate) fn drive_snap(
    ui: &Ui,
    id: Id,
    response: &Response,
    step_px: f32,
    interval: f32,
    nudge: f32,
) -> f32 {
    let dt = glowkit_core::dt(ui);
    let wdt = wall_dt(ui);
    let hovered = response.hovered() || response.dragged();
    let dragging = response.dragged();
    let released = response.drag_stopped();
    let drag = if dragging { response.drag_delta().x + response.drag_delta().y * 0.0 } else { 0.0 };
    let wheel = if response.hovered() {
        ui.input(|i| {
            let d = i.smooth_scroll_delta();
            d.x + d.y
        })
    } else {
        0.0
    };
    let keys = if response.hovered() || response.has_focus() {
        ui.input(|i| {
            (i.key_pressed(egui::Key::ArrowRight) as i32 - i.key_pressed(egui::Key::ArrowLeft) as i32) as f32
        })
    } else {
        0.0
    };
    let pos = glowkit_core::with_state(ui, id.with("snap_b_snap"), SnapState::default, |s| {
        if hovered {
            s.idle = 0.0;
        } else {
            s.idle += wdt;
        }
        if nudge != 0.0 {
            s.target = s.target.round() + nudge;
            s.timer = 0.0;
        }
        if dragging {
            s.pos -= drag / step_px.max(1.0);
            s.drag_acc += drag;
            s.vel = 0.0;
            s.target = s.pos;
        } else {
            if released {
                let fling = -s.drag_acc.signum() * (s.drag_acc.abs() > 8.0) as i32 as f32 * 0.5;
                s.target = (s.pos + fling).round();
                s.drag_acc = 0.0;
            }
            if wheel.abs() > 0.5 && s.timer > 0.25 {
                s.target = s.target.round() - wheel.signum();
                s.timer = 0.0;
            }
            if keys != 0.0 {
                s.target = s.target.round() + keys;
                s.timer = 0.0;
            }
            s.timer += wdt;
            if interval > 0.0 && s.idle > 1.5 && s.timer > interval {
                s.target = s.target.round() + 1.0;
                s.timer = 0.0;
            }
            let mut sp = glowkit_core::Spring { value: s.pos, velocity: s.vel, target: s.target, ..glowkit_core::Spring::new(0.0) }
                .with(140.0, 19.0);
            sp.step(dt);
            s.pos = sp.value;
            s.vel = sp.velocity;
        }
        s.pos
    });
    glowkit_core::animate(ui);
    pos
}

/// How far `rect` has travelled through the visible viewport (`ui.clip_rect()`): 0 when its top
/// edge sits at the viewport's bottom, 1 when its top reaches `end` (fraction from the top).
pub(crate) fn view_progress(ui: &Ui, rect: Rect, end: f32) -> f32 {
    let clip = ui.clip_rect();
    let start_y = clip.max.y;
    let end_y = clip.min.y + clip.height() * end;
    ((start_y - rect.min.y) / (start_y - end_y).max(1.0)).clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------------------------
// A procedural photograph with CPU-side pixels (for effects that sample image brightness).

/// Pixels + texture of a procedural backlit seed-head photograph.
pub(crate) struct Photo {
    pub tex: TextureId,
    _handle: TextureHandle,
    pub w: usize,
    pub h: usize,
    pub luma: Vec<f32>,
}

impl Photo {
    /// Luminance at normalized `uv`.
    pub fn luma_at(&self, u: f32, v: f32) -> f32 {
        let x = ((u * self.w as f32) as isize).clamp(0, self.w as isize - 1) as usize;
        let y = ((v * self.h as f32) as isize).clamp(0, self.h as isize - 1) as usize;
        self.luma[y * self.w + x]
    }
}

/// A procedural "macro photograph": a glowing dandelion-like seed head on a dark ground with a
/// few loose seeds drifting away. Cached per context.
pub(crate) fn seed_photo(ctx: &Context) -> Arc<Photo> {
    let key = Id::unique("snap_b_seed_photo");
    if let Some(p) = ctx.data(|d| d.get_temp::<Arc<Photo>>(key)) {
        return p;
    }
    let (w, h) = (480usize, 300usize);
    let mut buf = vec![[0.0f32; 3]; w * h];
    // background: deep warm-black vignette
    for y in 0..h {
        for x in 0..w {
            let u = x as f32 / w as f32;
            let v = y as f32 / h as f32;
            let d = ((u - 0.35).powi(2) + (v - 0.45).powi(2)).sqrt();
            let g = (0.09 - d * 0.08).max(0.0);
            let n = glowkit_core::noise::noise01(u * 6.0, v * 6.0, 1.3) * 0.02;
            buf[y * w + x] = [g * 0.9 + n, g * 0.75 + n, g * 0.55 + n];
        }
    }
    let mut add = |x: f32, y: f32, c: [f32; 3], a: f32| {
        if x < 0.0 || y < 0.0 {
            return;
        }
        let (xi, yi) = (x as usize, y as usize);
        if xi >= w || yi >= h {
            return;
        }
        let p = &mut buf[yi * w + xi];
        for k in 0..3 {
            p[k] += c[k] * a;
        }
    };
    let mut rng = glowkit_core::Rng::new(42);
    let warm = [1.0, 0.93, 0.8];
    // stem
    for i in 0..400 {
        let t = i as f32 / 400.0;
        let x = 168.0 - 20.0 * t + 6.0 * (t * 3.0).sin();
        let y = 150.0 + t * 160.0;
        for k in -4..=4 {
            let edge = 1.0 - (k as f32 / 4.5).abs();
            add(x + k as f32, y, [0.55, 0.5, 0.25], 0.5 * edge);
        }
    }
    // a seed head: many filaments radiating from a core, each ending in a tiny umbrella
    let mut head = |cx: f32, cy: f32, rad: f32, n: usize, bright: f32, rng: &mut glowkit_core::Rng| {
        for _ in 0..n {
            let a = rng.range(0.0, std::f32::consts::TAU);
            let len = rad * rng.range(0.75, 1.0);
            let steps = (len * 1.2) as usize;
            for s in 0..steps {
                let t = s as f32 / steps as f32;
                let r = 8.0 + t * len;
                add(cx + a.cos() * r, cy + a.sin() * r, warm, 0.10 * bright * (0.4 + t));
            }
            // umbrella: a little fan of fine hairs
            let tip = pos2(cx + a.cos() * (8.0 + len), cy + a.sin() * (8.0 + len));
            for k in 0..9 {
                let b = a + (k as f32 - 4.0) * 0.16;
                for s in 0..10 {
                    let r = s as f32;
                    add(tip.x + b.cos() * r, tip.y + b.sin() * r, warm, 0.08 * bright);
                }
            }
        }
        // dark seed core
        for y in -16..=16 {
            for x in -16..=16 {
                let d = ((x * x + y * y) as f32).sqrt();
                if d < 16.0 {
                    let k = 1.0 - d / 16.0;
                    add(cx + x as f32, cy + y as f32, [-0.5, -0.45, -0.4], k * 0.9);
                }
            }
        }
    };
    head(165.0, 135.0, 115.0, 900, 1.0, &mut rng);
    for (cx, cy, r) in [(345.0, 70.0, 32.0), (420.0, 120.0, 26.0), (300.0, 160.0, 24.0), (450.0, 40.0, 20.0)] {
        head(cx, cy, r, 90, 1.1, &mut rng);
    }
    let mut luma = Vec::with_capacity(w * h);
    let pixels: Vec<Color32> = buf
        .iter()
        .map(|c| {
            let r = c[0].clamp(0.0, 1.0);
            let g = c[1].clamp(0.0, 1.0);
            let b = c[2].clamp(0.0, 1.0);
            // soft tone curve
            let tc = |x: f32| 1.0 - (-2.2 * x).exp();
            let (r, g, b) = (tc(r), tc(g), tc(b));
            luma.push(0.3 * r + 0.59 * g + 0.11 * b);
            Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
        })
        .collect();
    let handle = ctx.load_texture("snap_b_seed_photo", ColorImage::new([w, h], pixels), TextureOptions::LINEAR);
    let p = Arc::new(Photo { tex: handle.id(), _handle: handle, w, h, luma });
    ctx.data_mut(|d| d.insert_temp(key, p.clone()));
    p
}

// ---------------------------------------------------------------------------------------------
// Misc paint helpers

/// Linear interpolation of colors in gamma space (cheap, fine for small steps).
pub(crate) fn lerp_c(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()), f(a.a(), b.a()))
}

/// Multi-stop gradient using [`lerp_c`].
pub(crate) fn ramp(stops: &[Color32], t: f32) -> Color32 {
    if stops.len() == 1 {
        return stops[0];
    }
    let t = t.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let i = (t.floor() as usize).min(stops.len() - 2);
    lerp_c(stops[i], stops[i + 1], t - i as f32)
}

/// Scale RGB (keep alpha) of an opaque color.
pub(crate) fn scale_rgb(c: Color32, k: f32) -> Color32 {
    let f = |x: u8| (x as f32 * k).clamp(0.0, 255.0) as u8;
    Color32::from_rgba_premultiplied(f(c.r()), f(c.g()), f(c.b()), c.a())
}

/// Additive-ish brighten toward white by `k`.
pub(crate) fn lighten(c: Color32, k: f32) -> Color32 {
    lerp_c(c, Color32::from_rgba_premultiplied(255, 255, 255, c.a()), k)
}

/// Dashed polyline.
pub(crate) fn dashed(painter: &Painter, pts: &[Pos2], dash: f32, gap: f32, phase: f32, width: f32, color: Color32) {
    let mut acc = -phase.rem_euclid(dash + gap);
    for w2 in pts.windows(2) {
        let (a, b) = (w2[0], w2[1]);
        let len = a.distance(b);
        if len < 1e-3 {
            continue;
        }
        let dir = (b - a) / len;
        let mut s = 0.0;
        while s < len {
            let period = dash + gap;
            let local = (acc + s).rem_euclid(period);
            if local < dash {
                let e = (s + dash - local).min(len);
                painter.line_segment([a + dir * s, a + dir * e], egui::Stroke::new(width, color));
                s = e + 1e-3;
            } else {
                s += period - local;
            }
        }
        acc += len;
    }
}

/// Paint a galley whose glyph vertices are pushed through `map` (local point relative to the
/// galley's top-left -> screen). Lets text ride tilted / curved surfaces.
pub(crate) fn galley_mapped(
    painter: &Painter,
    galley: &Arc<egui::Galley>,
    color: Color32,
    map: impl Fn(Vec2) -> Pos2,
) {
    let [tw, th] = painter.fonts(|fo| fo.font_image_size());
    let norm = vec2(1.0 / tw.max(1) as f32, 1.0 / th.max(1) as f32);
    for row in &galley.rows {
        let mut mesh = row.visuals.mesh.clone();
        if mesh.is_empty() {
            continue;
        }
        let off = row.pos.to_vec2();
        for v in &mut mesh.vertices {
            let l = v.pos.to_vec2() + off;
            let a = v.color.a() as f32 / 255.0;
            *v = Vertex { pos: map(l), uv: pos2(v.uv.x * norm.x, v.uv.y * norm.y), color: color.gamma_multiply(a) };
        }
        painter.add(Shape::mesh(mesh));
    }
}

/// Append a filled circle (fan of `segs` triangles) to `mesh` (untextured).
pub(crate) fn mesh_circle(mesh: &mut Mesh, c: Pos2, r: f32, segs: usize, color: Color32) {
    let i0 = mesh.vertices.len() as u32;
    mesh.colored_vertex(c, color);
    for k in 0..segs {
        let a = k as f32 / segs as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(c + vec2(a.cos(), a.sin()) * r, color);
    }
    for k in 0..segs as u32 {
        mesh.add_triangle(i0, i0 + 1 + k, i0 + 1 + (k + 1) % segs as u32);
    }
}

/// Append an axis-aligned flat rect to `mesh` (untextured).
pub(crate) fn mesh_rect(mesh: &mut Mesh, r: Rect, color: Color32) {
    let i = mesh.vertices.len() as u32;
    mesh.colored_vertex(r.left_top(), color);
    mesh.colored_vertex(r.right_top(), color);
    mesh.colored_vertex(r.right_bottom(), color);
    mesh.colored_vertex(r.left_bottom(), color);
    mesh.add_triangle(i, i + 1, i + 2);
    mesh.add_triangle(i, i + 2, i + 3);
}

/// Deterministic per-cell hash in `0..1`.
pub(crate) fn hash3(x: i32, y: i32, s: i32) -> f32 {
    hash_u(x as u32, y as u32, s as u32)
}

/// Cover the four corners of `rect` outside a radius-`r` rounding with `bg` (mask square-cut
/// content so it looks rounded).
pub(crate) fn round_corners(painter: &Painter, rect: Rect, r: f32, bg: Color32) {
    let mut m = Mesh::default();
    let segs = 10;
    for (corner, center, a0) in [
        (rect.left_top(), rect.left_top() + vec2(r, r), 0.5f32),
        (rect.right_top(), rect.right_top() + vec2(-r, r), 0.75),
        (rect.right_bottom(), rect.right_bottom() + vec2(-r, -r), 0.0),
        (rect.left_bottom(), rect.left_bottom() + vec2(r, -r), 0.25),
    ] {
        let i0 = m.vertices.len() as u32;
        m.colored_vertex(corner, bg);
        for k in 0..=segs {
            let a = (a0 + 0.25 * k as f32 / segs as f32) * std::f32::consts::TAU;
            m.colored_vertex(center + vec2(a.cos(), a.sin()) * r, bg);
        }
        for k in 0..segs as u32 {
            m.add_triangle(i0, i0 + 1 + k, i0 + 2 + k);
        }
    }
    painter.add(Shape::mesh(m));
}

/// Rounded rect with a vertical gradient (premultiplied lerp, safe for fades to transparent).
pub(crate) fn vgrad_rect(painter: &Painter, rect: Rect, radius: f32, top: Color32, bottom: Color32) {
    let pts = glowkit_core::paint::rounded_rect_points(rect, radius, 8);
    let mut m = Mesh::default();
    let c = rect.center();
    let col = |p: Pos2| lerp_c(top, bottom, (p.y - rect.min.y) / rect.height().max(1.0));
    m.colored_vertex(c, col(c));
    for p in &pts {
        m.colored_vertex(*p, col(*p));
    }
    let n = pts.len() as u32;
    for i in 0..n {
        m.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(Shape::mesh(m));
}

/// 0 at the left / right edges of `rect`, 1 more than `w` points inside (smoothstep).
pub(crate) fn edge_alpha(rect: Rect, x: f32, w: f32) -> f32 {
    let k = ((x - rect.min.x) / w).min((rect.max.x - x) / w).clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}
