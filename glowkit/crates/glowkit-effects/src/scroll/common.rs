//! Shared machinery for the scroll module: shape capture (so arbitrary egui content can be
//! faded, blurred, transformed or clipped to any convex outline), textured card meshes,
//! procedural placeholder "photos", scroll drivers and view-progress helpers.

use egui::emath::TSTransform;
use egui::epaint::{ClippedShape, Mesh, Tessellator, Vertex, WHITE_UV};
use egui::layers::ShapeIdx;
use egui::{pos2, vec2, Color32, ColorImage, Context, Id, Pos2, Rect, Response, Shape, TextureHandle, TextureId, Ui, Vec2};
use glowkit_core::noise::noise2;

// ---------------------------------------------------------------------------------------------
// Shape capture

/// Index of the next shape that will be painted on `ui`'s layer.
pub(crate) fn shape_mark(ui: &Ui) -> ShapeIdx {
    let layer = ui.layer_id();
    ui.ctx().graphics(|g| g.get(layer).map_or(ShapeIdx(0), |l| l.next_idx()))
}

/// A range of shapes painted after a [`shape_mark`], lifted out of the paint list so they can
/// be post-processed and put back with [`Captured::restore`].
pub(crate) struct Captured {
    layer: egui::LayerId,
    start: usize,
    pub shapes: Vec<ClippedShape>,
}

/// Take every shape painted on `ui`'s layer since `start` (they are replaced by no-ops).
pub(crate) fn capture(ui: &Ui, start: ShapeIdx) -> Captured {
    let layer = ui.layer_id();
    let shapes = ui.ctx().graphics_mut(|g| {
        let list = g.entry(layer);
        let end = list.next_idx().0;
        let mut out = Vec::with_capacity(end.saturating_sub(start.0));
        for i in start.0..end {
            let mut taken = None;
            list.mutate_shape(ShapeIdx(i), |cs| {
                taken = Some(ClippedShape {
                    clip_rect: cs.clip_rect,
                    shape: std::mem::replace(&mut cs.shape, Shape::Noop),
                    transform_after_tessellation: cs.transform_after_tessellation,
                });
            });
            if let Some(t) = taken {
                out.push(t);
            }
        }
        out
    });
    Captured { layer, start: start.0, shapes }
}

impl Captured {
    /// Put shapes back in their original slots. `f(index, clipped)` returns the replacement.
    pub fn restore(self, ctx: &Context, mut f: impl FnMut(usize, ClippedShape) -> (Rect, Shape)) {
        let start = self.start;
        let mut replaced: Vec<(usize, Rect, Shape)> = Vec::with_capacity(self.shapes.len());
        for (i, cs) in self.shapes.into_iter().enumerate() {
            let (clip, s) = f(i, cs);
            replaced.push((start + i, clip, s));
        }
        ctx.graphics_mut(|g| {
            let list = g.entry(self.layer);
            for (idx, clip, s) in replaced {
                list.set(ShapeIdx(idx), clip, s);
            }
        });
    }
}

/// Build a tessellator that matches the current frame.
pub(crate) fn tessellator(ctx: &Context) -> Tessellator {
    let ppp = ctx.pixels_per_point();
    let opts = ctx.tessellation_options(|o| *o);
    let font_size = ctx.fonts(|f| f.font_image_size());
    Tessellator::new(ppp, opts, font_size, Vec::new())
}

/// Turn a shape into meshes (one per texture), recursing into `Shape::Vec`.
pub(crate) fn to_meshes(tess: &mut Tessellator, shape: Shape, out: &mut Vec<Mesh>) {
    match shape {
        Shape::Noop | Shape::Callback(_) => {}
        Shape::Vec(v) => {
            for s in v {
                to_meshes(tess, s, out);
            }
        }
        Shape::Mesh(m) => out.push((*m).clone()),
        s => {
            let tex = s.texture_id();
            let mut m = Mesh::with_texture(tex);
            tess.tessellate_shape(s, &mut m);
            if !m.is_empty() {
                out.push(m);
            }
        }
    }
}

/// Multiply all vertex colors of a mesh by `a`.
pub(crate) fn mesh_alpha(m: &mut Mesh, a: f32) {
    if a >= 0.999 {
        return;
    }
    for v in &mut m.vertices {
        v.color = v.color.gamma_multiply(a.max(0.0));
    }
}

#[derive(Clone, Copy)]
struct CV {
    p: Pos2,
    uv: Pos2,
    c: [f32; 4],
}

fn cv(v: &Vertex) -> CV {
    CV { p: v.pos, uv: v.uv, c: [v.color.r() as f32, v.color.g() as f32, v.color.b() as f32, v.color.a() as f32] }
}

fn cv_lerp(a: CV, b: CV, t: f32) -> CV {
    let c = std::array::from_fn(|k| a.c[k] + (b.c[k] - a.c[k]) * t);
    CV { p: a.p.lerp(b.p, t), uv: a.uv.lerp(b.uv, t), c }
}

/// Clip a mesh to a **convex** polygon (any winding). Triangles fully inside are kept as-is,
/// straddling ones are cut with Sutherland-Hodgman, and outside ones are dropped.
pub(crate) fn clip_mesh(mesh: &Mesh, poly: &[Pos2], safe: Rect) -> Mesh {
    let mut out = Mesh::with_texture(mesh.texture_id);
    if poly.len() < 3 {
        return out;
    }
    let bb = Rect::from_points(poly);
    let area: f32 = (0..poly.len())
        .map(|i| {
            let a = poly[i];
            let b = poly[(i + 1) % poly.len()];
            a.x * b.y - b.x * a.y
        })
        .sum();
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let edge_d = |i: usize, p: Pos2| {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        ((b - a).x * (p - a).y - (b - a).y * (p - a).x) * sign
    };
    let inside = |p: Pos2| (0..poly.len()).all(|i| edge_d(i, p) >= 0.0);
    let mut buf: Vec<CV> = Vec::with_capacity(12);
    let mut nxt: Vec<CV> = Vec::with_capacity(12);
    for tri in mesh.indices.chunks_exact(3) {
        let vs = [&mesh.vertices[tri[0] as usize], &mesh.vertices[tri[1] as usize], &mesh.vertices[tri[2] as usize]];
        let tb = Rect::from_points(&[vs[0].pos, vs[1].pos, vs[2].pos]);
        if !bb.intersects(tb) {
            continue;
        }
        if safe.contains_rect(tb) || vs.iter().all(|v| inside(v.pos)) {
            let base = out.vertices.len() as u32;
            out.vertices.extend(vs.iter().map(|v| **v));
            out.add_triangle(base, base + 1, base + 2);
            continue;
        }
        buf.clear();
        buf.extend(vs.iter().map(|v| cv(v)));
        for e in 0..poly.len() {
            if buf.is_empty() {
                break;
            }
            nxt.clear();
            for k in 0..buf.len() {
                let a = buf[k];
                let b = buf[(k + 1) % buf.len()];
                let da = edge_d(e, a.p);
                let db = edge_d(e, b.p);
                if da >= 0.0 {
                    nxt.push(a);
                }
                if (da >= 0.0) != (db >= 0.0) {
                    nxt.push(cv_lerp(a, b, da / (da - db)));
                }
            }
            std::mem::swap(&mut buf, &mut nxt);
        }
        if buf.len() < 3 {
            continue;
        }
        let base = out.vertices.len() as u32;
        for v in &buf {
            let col = Color32::from_rgba_premultiplied(
                v.c[0].round() as u8,
                v.c[1].round() as u8,
                v.c[2].round() as u8,
                v.c[3].round() as u8,
            );
            out.vertices.push(Vertex { pos: v.p, uv: v.uv, color: col });
        }
        for k in 1..buf.len() as u32 - 1 {
            out.add_triangle(base, base + k, base + k + 1);
        }
    }
    out
}

/// Run `content` and clip everything it paints to the convex `outline` (e.g. a circle or a
/// rounded rect). `transform` is applied first (use it to zoom the content). Triangles inside
/// `safe` (a rect known to be inside the outline) skip the clipping test.
pub(crate) fn paint_clipped(
    ui: &mut Ui,
    outline: &[Pos2],
    safe: Rect,
    transform: TSTransform,
    alpha: f32,
    content: impl FnOnce(&mut Ui),
) {
    let start = shape_mark(ui);
    content(ui);
    let cap = capture(ui, start);
    let ctx = ui.ctx().clone();
    let mut tess = tessellator(&ctx);
    let clip = ui.clip_rect();
    cap.restore(&ctx, |_, cs| {
        let mut meshes = Vec::new();
        to_meshes(&mut tess, cs.shape, &mut meshes);
        let shapes: Vec<Shape> = meshes
            .into_iter()
            .map(|mut m| {
                m.transform(transform);
                let mut c = clip_mesh(&m, outline, safe);
                mesh_alpha(&mut c, alpha);
                Shape::mesh(c)
            })
            .collect();
        (clip, Shape::Vec(shapes))
    });
}

// ---------------------------------------------------------------------------------------------
// Textured / warped card meshes

/// Paint a rounded "card" whose local coordinates (`0..size`) are mapped to the screen by `map`
/// (identity, perspective, anything smooth). It is filled with `tex` (sampled over `uv`) tinted
/// by `tint(local)`, or a flat color when `tex` is `None`. Edges are feathered.
#[allow(clippy::too_many_arguments)]
pub(crate) fn warp_card(
    painter: &egui::Painter,
    size: Vec2,
    radius: f32,
    rings: usize,
    tex: Option<TextureId>,
    uv: Rect,
    map: impl Fn(Vec2) -> Pos2,
    tint: impl Fn(Vec2) -> Color32,
) {
    if size.x <= 0.5 || size.y <= 0.5 {
        return;
    }
    let local = Rect::from_min_size(Pos2::ZERO, size);
    let outline: Vec<Pos2> = glowkit_core::paint::subdivide_closed(
        &glowkit_core::paint::rounded_rect_points(local, radius, 5),
        (size.max_elem() / 10.0).max(4.0),
    );
    let n = outline.len();
    let c = local.center();
    let rings = rings.max(1);
    let mut m = Mesh::with_texture(tex.unwrap_or_default());
    let uv_of = |l: Vec2| {
        if tex.is_some() {
            pos2(uv.min.x + l.x / size.x * uv.width(), uv.min.y + l.y / size.y * uv.height())
        } else {
            WHITE_UV
        }
    };
    let center_l = c.to_vec2();
    m.vertices.push(Vertex { pos: map(center_l), uv: uv_of(center_l), color: tint(center_l) });
    for k in 1..=rings {
        let s = k as f32 / rings as f32;
        for o in &outline {
            let l = center_l + (o.to_vec2() - center_l) * s;
            m.vertices.push(Vertex { pos: map(l), uv: uv_of(l), color: tint(l) });
        }
    }
    // feather ring in screen space
    let sc = map(center_l);
    let outer0 = 1 + (rings - 1) * n;
    for i in 0..n {
        let v = m.vertices[outer0 + i];
        let dir = (v.pos - sc).normalized();
        m.vertices.push(Vertex { pos: v.pos + dir * 0.8, uv: v.uv, color: Color32::TRANSPARENT });
    }
    for i in 0..n {
        let j = (i + 1) % n;
        m.add_triangle(0, 1 + i as u32, 1 + j as u32);
    }
    for k in 1..=rings {
        let a0 = 1 + (k - 1) * n;
        let b0 = 1 + k * n;
        for i in 0..n {
            let j = (i + 1) % n;
            m.add_triangle((a0 + i) as u32, (b0 + i) as u32, (b0 + j) as u32);
            m.add_triangle((a0 + i) as u32, (b0 + j) as u32, (a0 + j) as u32);
        }
    }
    painter.add(Shape::mesh(m));
}

/// Flat (un-warped) textured rounded rect.
pub(crate) fn image_rect(painter: &egui::Painter, rect: Rect, radius: f32, tex: TextureId, uv: Rect, tint: Color32) {
    warp_card(painter, rect.size(), radius, 1, Some(tex), uv, |l| rect.min + l, |_| tint);
}

/// A UV rect that crops a `tex_aspect` texture to fill a `target_aspect` box (CSS `object-fit: cover`).
pub(crate) fn cover_uv(tex_aspect: f32, target_aspect: f32) -> Rect {
    if target_aspect > tex_aspect {
        let h = tex_aspect / target_aspect;
        Rect::from_min_size(pos2(0.0, (1.0 - h) / 2.0), vec2(1.0, h))
    } else {
        let w = target_aspect / tex_aspect;
        Rect::from_min_size(pos2((1.0 - w) / 2.0, 0.0), vec2(w, 1.0))
    }
}

// ---------------------------------------------------------------------------------------------
// Procedural placeholder photos

/// Procedural placeholder "photos": eight painted landscapes (alpenglow peaks, a lagoon, an
/// aurora night, desert dunes, a misty forest, a neon city, a glacier lake and lavender hills),
/// generated once on the CPU and uploaded as textures.
///
/// Every scroll component that shows imagery accepts an optional `egui::TextureId`; when none is
/// given it falls back to one of these, so demos look like real photo galleries without shipping
/// image files.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let tex = glowkit_effects::PaintedScene::texture(ui.ctx(), 2); // aurora night
/// ui.image((tex, egui::vec2(320.0, 200.0)));
/// ```
pub struct PaintedScene;

impl PaintedScene {
    /// Number of built-in scenes.
    pub const COUNT: usize = 8;
    /// Width / height of the generated textures.
    pub const ASPECT: f32 = 1.6;

    /// Display name of scene `index` (wraps around).
    pub fn name(index: usize) -> &'static str {
        ["Alpenglow", "Lagoon", "Aurora", "Dunes", "Evergreen", "Neon City", "Glacier", "Lavender"][index % Self::COUNT]
    }

    /// The dominant accent color of scene `index`, handy for matching UI.
    pub fn accent(index: usize) -> Color32 {
        use glowkit_core::hex;
        [
            hex("#F472B6"),
            hex("#22D3EE"),
            hex("#34D399"),
            hex("#FB923C"),
            hex("#10B981"),
            hex("#E879F9"),
            hex("#38BDF8"),
            hex("#A78BFA"),
        ][index % Self::COUNT]
    }

    /// Texture of scene `index` (wraps around), generated and cached on first use.
    pub fn texture(ctx: &Context, index: usize) -> TextureId {
        let index = index % Self::COUNT;
        let id = Id::unique(("glowkit_painted_scene", index));
        if let Some(h) = ctx.data(|d| d.get_temp::<TextureHandle>(id)) {
            return h.id();
        }
        let img = scene_image(index, 320, 200);
        let h = ctx.load_texture(format!("glowkit-scene-{index}"), img, egui::TextureOptions::LINEAR);
        let tid = h.id();
        ctx.data_mut(|d| d.insert_temp(id, h));
        tid
    }
}

fn hexf(s: &str) -> [f32; 3] {
    let c = glowkit_core::hex(s);
    [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0]
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn grad3(stops: &[[f32; 3]], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let i = (t.floor() as usize).min(stops.len() - 2);
    lerp3(stops[i], stops[i + 1], t - i as f32)
}

fn hash2(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6b343) ^ (y as u32).wrapping_mul(0xd8163841);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1e995);
    h ^= h >> 15;
    (h & 0xffff) as f32 / 65535.0
}

fn ridge(u: f32, seed: f32, freq: f32, sharp: bool) -> f32 {
    let mut s = 0.0;
    let mut amp = 1.0;
    let mut f = freq;
    for _ in 0..4 {
        let n = noise2(u * f, seed);
        s += amp * if sharp { 1.0 - n.abs() * 2.0 } else { n };
        amp *= 0.5;
        f *= 2.1;
    }
    s / 1.875
}

struct SceneSpec {
    sky: Vec<[f32; 3]>,
    sun: Option<(f32, f32, f32, [f32; 3])>,
    layers: Vec<(f32, f32, f32, [f32; 3], bool)>, // base, amp, freq, color, sharp
    water: Option<(f32, [f32; 3])>,
    stars: bool,
    aurora: bool,
    city: bool,
    haze: [f32; 3],
}

fn spec(index: usize) -> SceneSpec {
    match index {
        0 => SceneSpec {
            sky: vec![hexf("#1E1B4B"), hexf("#6D28D9"), hexf("#F472B6"), hexf("#FDBA74")],
            sun: Some((0.7, 0.56, 0.06, hexf("#FFEDD5"))),
            layers: vec![
                (0.52, 0.16, 2.0, hexf("#9D4EDD"), true),
                (0.62, 0.14, 2.6, hexf("#5B21B6"), true),
                (0.74, 0.12, 3.2, hexf("#2E1065"), true),
            ],
            water: None,
            stars: false,
            aurora: false,
            city: false,
            haze: hexf("#F9A8D4"),
        },
        1 => SceneSpec {
            sky: vec![hexf("#0369A1"), hexf("#38BDF8"), hexf("#E0F2FE")],
            sun: Some((0.24, 0.18, 0.05, hexf("#FFFFFF"))),
            layers: vec![(0.54, 0.08, 1.6, hexf("#5EB8C9"), false), (0.6, 0.06, 2.4, hexf("#0F766E"), false)],
            water: Some((0.63, hexf("#06B6D4"))),
            stars: false,
            aurora: false,
            city: false,
            haze: hexf("#E0F2FE"),
        },
        2 => SceneSpec {
            sky: vec![hexf("#020617"), hexf("#0B1A3A"), hexf("#134E4A")],
            sun: Some((0.8, 0.2, 0.025, hexf("#F1F5F9"))),
            layers: vec![(0.6, 0.15, 2.2, hexf("#1E293B"), true), (0.72, 0.1, 3.0, hexf("#0B1220"), true)],
            water: Some((0.8, hexf("#0F2A3A"))),
            stars: true,
            aurora: true,
            city: false,
            haze: hexf("#134E4A"),
        },
        3 => SceneSpec {
            sky: vec![hexf("#F97316"), hexf("#FDBA74"), hexf("#FEF3C7")],
            sun: Some((0.5, 0.44, 0.09, hexf("#FFFBEB"))),
            layers: vec![
                (0.58, 0.06, 1.2, hexf("#FB923C"), false),
                (0.68, 0.08, 1.0, hexf("#EA580C"), false),
                (0.8, 0.07, 0.9, hexf("#9A3412"), false),
            ],
            water: None,
            stars: false,
            aurora: false,
            city: false,
            haze: hexf("#FED7AA"),
        },
        4 => SceneSpec {
            sky: vec![hexf("#ECFDF5"), hexf("#A7F3D0"), hexf("#6EE7B7")],
            sun: Some((0.3, 0.3, 0.07, hexf("#FFFFFF"))),
            layers: vec![
                (0.48, 0.1, 6.0, hexf("#34D399"), true),
                (0.6, 0.1, 9.0, hexf("#047857"), true),
                (0.76, 0.1, 12.0, hexf("#022C22"), true),
            ],
            water: None,
            stars: false,
            aurora: false,
            city: false,
            haze: hexf("#D1FAE5"),
        },
        5 => SceneSpec {
            sky: vec![hexf("#0F0326"), hexf("#4C1D95"), hexf("#DB2777")],
            sun: Some((0.5, 0.52, 0.12, hexf("#FDE68A"))),
            layers: vec![],
            water: Some((0.78, hexf("#1E0B3A"))),
            stars: true,
            aurora: false,
            city: true,
            haze: hexf("#DB2777"),
        },
        6 => SceneSpec {
            sky: vec![hexf("#7DD3FC"), hexf("#BAE6FD"), hexf("#F0F9FF")],
            sun: Some((0.78, 0.22, 0.05, hexf("#FFFFFF"))),
            layers: vec![(0.42, 0.2, 2.0, hexf("#E0F2FE"), true), (0.58, 0.12, 2.8, hexf("#7DD3FC"), true)],
            water: Some((0.68, hexf("#0284C7"))),
            stars: false,
            aurora: false,
            city: false,
            haze: hexf("#F0F9FF"),
        },
        _ => SceneSpec {
            sky: vec![hexf("#312E81"), hexf("#818CF8"), hexf("#F9A8D4"), hexf("#FDE68A")],
            sun: Some((0.35, 0.6, 0.07, hexf("#FFF7ED"))),
            layers: vec![
                (0.62, 0.05, 1.4, hexf("#A78BFA"), false),
                (0.7, 0.06, 1.1, hexf("#7C3AED"), false),
                (0.8, 0.07, 0.9, hexf("#4C1D95"), false),
            ],
            water: None,
            stars: false,
            aurora: false,
            city: false,
            haze: hexf("#FBCFE8"),
        },
    }
}

fn sky_at(s: &SceneSpec, u: f32, v: f32, aspect: f32) -> [f32; 3] {
    let mut c = grad3(&s.sky, v / 0.8);
    if s.aurora {
        for (k, col) in [(0.0, hexf("#34D399")), (1.7, hexf("#22D3EE")), (3.1, hexf("#A78BFA"))] {
            let center = 0.26 + 0.07 * (u * 5.0 + k).sin() + 0.04 * noise2(u * 3.0, k);
            let band = (-((v - center) / 0.07).powi(2)).exp();
            let curtain = 0.55 + 0.45 * noise2(u * 18.0, k + 5.0);
            let a = band * curtain * if k == 0.0 { 0.75 } else { 0.4 };
            c = lerp3(c, col, a);
        }
    }
    if s.stars && v < 0.6 {
        let gx = (u * 320.0) as i32;
        let gy = (v * 200.0) as i32;
        let h = hash2(gx, gy);
        if h > 0.985 {
            let b = (h - 0.985) / 0.015 * (1.0 - v / 0.6);
            c = lerp3(c, [1.0, 1.0, 1.0], b);
        }
    }
    if let Some((sx, sy, r, col)) = s.sun {
        let d = vec2((u - sx) * aspect, v - sy).length();
        let glow = (-(d / (r * 3.5)).powi(2)).exp() * 0.55 + (-(d / (r * 9.0)).powi(2)).exp() * 0.25;
        c = lerp3(c, col, glow);
        if d < r {
            c = lerp3(c, col, ((r - d) / 0.004).clamp(0.0, 1.0));
        }
    }
    c
}

fn land_at(s: &SceneSpec, heights: &[Vec<f32>], u: f32, v: f32) -> Option<[f32; 3]> {
    let mut hit = None;
    let n = s.layers.len();
    for (k, (_, _, _, col, sharp)) in s.layers.iter().enumerate() {
        let hs = &heights[k];
        let h = hs[((u.clamp(0.0, 0.9999)) * hs.len() as f32) as usize];
        if v >= h {
            let depth = 1.0 - k as f32 / n.max(1) as f32;
            let shade = ((v - h) * 3.0).clamp(0.0, 0.35);
            let mut c = lerp3(*col, [0.0, 0.0, 0.0], shade);
            if *sharp && (v - h) < 0.02 && k == 0 {
                c = lerp3(c, [1.0, 1.0, 1.0], 0.15);
            }
            c = lerp3(c, s.haze, depth * 0.25);
            hit = Some(c);
        }
    }
    if s.city {
        let col_w = 0.035;
        let bi = (u / col_w).floor() as i32;
        let hb = hash2(bi, 7);
        let top = 0.48 + hb * 0.26 - if hash2(bi, 9) > 0.8 { 0.12 } else { 0.0 };
        let front_bi = (u / 0.05).floor() as i32;
        let ftop = 0.62 + hash2(front_bi, 3) * 0.14;
        if v >= ftop {
            let mut c = hexf("#12071F");
            let wx = ((u / 0.05).fract() * 6.0) as i32;
            let wy = ((v - ftop) * 90.0) as i32;
            if wx % 2 == 1 && wy % 2 == 1 && hash2(front_bi * 31 + wx, wy) > 0.55 {
                c = grad3(&[hexf("#F0ABFC"), hexf("#FDE68A"), hexf("#67E8F9")], hash2(front_bi, wy));
            }
            hit = Some(c);
        } else if v >= top {
            let mut c = hexf("#2E1065");
            let wx = ((u / col_w).fract() * 5.0) as i32;
            let wy = ((v - top) * 110.0) as i32;
            if wx % 2 == 1 && wy % 2 == 1 && hash2(bi * 17 + wx, wy) > 0.6 {
                c = lerp3(c, hexf("#F472B6"), 0.7);
            }
            hit = Some(lerp3(c, s.haze, 0.18));
        }
    }
    hit
}

fn scene_image(index: usize, w: usize, h: usize) -> ColorImage {
    let s = spec(index);
    let aspect = w as f32 / h as f32;
    let mut px = Vec::with_capacity(w * h);
    let heights: Vec<Vec<f32>> = s
        .layers
        .iter()
        .enumerate()
        .map(|(k, (base, amp, freq, _, sharp))| {
            (0..w).map(|x| base - amp * ridge((x as f32 + 0.5) / w as f32, 3.7 * k as f32 + 1.3, *freq, *sharp)).collect()
        })
        .collect();
    for y in 0..h {
        for x in 0..w {
            let u = (x as f32 + 0.5) / w as f32;
            let v = (y as f32 + 0.5) / h as f32;
            let mut c = match land_at(&s, &heights, u, v) {
                Some(c) => c,
                None => sky_at(&s, u, v, aspect),
            };
            if let Some((wl, wc)) = s.water {
                if v > wl {
                    let ripple = 0.006 * noise2(u * 4.0, v * 60.0) + 0.004 * (v * 220.0).sin();
                    let rv = wl - (v - wl) * 1.2 + ripple;
                    let refl = match land_at(&s, &heights, u + ripple, rv) {
                        Some(c) => c,
                        None => sky_at(&s, u + ripple, rv.max(0.0), aspect),
                    };
                    let depth = ((v - wl) / (1.0 - wl)).clamp(0.0, 1.0);
                    c = lerp3(lerp3(refl, wc, 0.35 + 0.4 * depth), [0.0, 0.0, 0.0], depth * 0.25);
                }
            }
            // gentle vignette + grain
            let dx = u - 0.5;
            let dy = v - 0.5;
            let vig = 1.0 - (dx * dx + dy * dy) * 0.45;
            let grain = (hash2(x as i32, y as i32 + 1000) - 0.5) * 0.025;
            let to8 = |f: f32| ((f * vig + grain).clamp(0.0, 1.0) * 255.0).round() as u8;
            px.push(Color32::from_rgb(to8(c[0]), to8(c[1]), to8(c[2])));
        }
    }
    ColorImage::new([w, h], px)
}

// ---------------------------------------------------------------------------------------------
// Scroll helpers

/// How far `rect` has travelled through the visible viewport (`ui.clip_rect()`):
/// 0 when its top edge sits at the viewport's bottom, 1 when its top reaches `end` (a fraction of
/// the viewport height measured from the top).
pub(crate) fn view_progress(ui: &Ui, rect: Rect, end: f32) -> f32 {
    let clip = ui.clip_rect();
    let start_y = clip.max.y;
    let end_y = clip.min.y + clip.height() * end;
    ((start_y - rect.min.y) / (start_y - end_y).max(1.0)).clamp(0.0, 1.0)
}

#[derive(Clone, Copy, Default)]
pub(crate) struct DriveState {
    pub offset: f32,
    vel: f32,
    dir: f32,
    wait: f32,
    idle: f32,
}

/// A self-contained scroll position for components that own their viewport: wheel and drag
/// (with inertia) while hovered, and a gentle ping-pong autoplay (`speed` points / second) when
/// the pointer has been away for a moment. Returns the offset in `0..=max`.
pub(crate) fn drive_scroll(ui: &Ui, id: Id, response: &Response, max: f32, horizontal: bool, speed: f32) -> f32 {
    let dt = glowkit_core::dt(ui);
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
    let drag = if response.dragged() {
        let d = response.drag_delta();
        if horizontal {
            d.x
        } else {
            d.y
        }
    } else {
        0.0
    };
    let dragging = response.dragged();
    let max = max.max(0.0);
    let (offset, moving) = glowkit_core::with_state(ui, id.with("drive"), DriveState::default, |s| {
        if s.dir == 0.0 {
            s.dir = 1.0;
        }
        if hovered {
            s.idle = 0.0;
        } else {
            s.idle += dt;
        }
        if dragging {
            s.offset -= drag;
            s.vel = if dt > 0.0 { -drag / dt } else { 0.0 };
        } else {
            s.offset -= wheel;
            s.vel *= (-4.0 * dt).exp();
            s.offset += s.vel * dt;
            if speed > 0.0 && s.idle > 1.2 && max > 0.0 {
                if s.wait > 0.0 {
                    s.wait -= dt;
                } else {
                    // ease in and out near the ends
                    let edge = (s.offset.min(max - s.offset) / 60.0).clamp(0.15, 1.0);
                    s.offset += s.dir * speed * edge * dt;
                    if s.offset >= max {
                        s.dir = -1.0;
                        s.wait = 1.4;
                    } else if s.offset <= 0.0 {
                        s.dir = 1.0;
                        s.wait = 1.4;
                    }
                }
            }
        }
        s.offset = s.offset.clamp(0.0, max);
        (s.offset, s.vel.abs() > 0.5 || wheel != 0.0 || (speed > 0.0 && s.idle <= 1.3))
    });
    if speed > 0.0 || moving {
        glowkit_core::animate(ui);
    }
    offset
}

/// Paint a small rounded progress rail (for viewports that own their scrolling).
pub(crate) fn scroll_rail(painter: &egui::Painter, t: &glowkit_core::Theme, track: Rect, frac_visible: f32, pos: f32) {
    let r = egui::CornerRadius::same(99);
    painter.rect_filled(track, r, glowkit_core::color::with_alpha(t.muted_foreground, 0.12));
    let h = (track.height() * frac_visible.clamp(0.08, 1.0)).max(18.0);
    let y = track.min.y + (track.height() - h) * pos.clamp(0.0, 1.0);
    painter.rect_filled(
        Rect::from_min_size(pos2(track.min.x, y), vec2(track.width(), h)),
        r,
        glowkit_core::color::with_alpha(t.muted_foreground, 0.45),
    );
}

/// Same as [`drive_scroll`] but unbounded: the offset grows forever (wrap it yourself), with a
/// constant drift of `speed` points / second while idle.
pub(crate) fn drive_loop(ui: &Ui, id: Id, response: &Response, horizontal: bool, speed: f32) -> f32 {
    let dt = glowkit_core::dt(ui);
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
    let off = glowkit_core::with_state(ui, id.with("drive_loop"), DriveState::default, |s| {
        if hovered {
            s.idle = 0.0;
        } else {
            s.idle += dt;
        }
        if dragging {
            s.offset -= drag;
            s.vel = if dt > 0.0 { -drag / dt } else { 0.0 };
        } else {
            s.offset -= wheel;
            s.vel *= (-3.0 * dt).exp();
            let drift = speed * (s.idle / 1.0).clamp(0.0, 1.0);
            s.offset += (s.vel + drift) * dt;
        }
        s.offset
    });
    glowkit_core::animate(ui);
    off
}

/// A cheap perspective camera for "tilted card" effects: rotate flat screen-space content about
/// the X axis (top edge tipping away) and the Z axis around `pivot`, then project.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Persp {
    pub pivot: Pos2,
    /// Rotation about the horizontal axis, radians (positive tips the top away).
    pub rx: f32,
    /// Rotation in the screen plane, radians.
    pub rz: f32,
    /// Perspective distance in points.
    pub focal: f32,
    pub translate: Vec2,
    pub scale: f32,
}

impl Persp {
    pub fn new(pivot: Pos2) -> Self {
        Self { pivot, rx: 0.0, rz: 0.0, focal: 1000.0, translate: Vec2::ZERO, scale: 1.0 }
    }
    pub fn apply(&self, p: Pos2) -> Pos2 {
        let d = (p - self.pivot) * self.scale;
        let (sz, cz) = self.rz.sin_cos();
        let (x, y) = (d.x * cz - d.y * sz, d.x * sz + d.y * cz);
        let (sx, cx) = self.rx.sin_cos();
        let y2 = y * cx;
        let z = -y * sx;
        let s = self.focal / (self.focal + z).max(1.0);
        self.pivot + self.translate + vec2(x * s, y2 * s)
    }
}

/// Run `content`, then push every vertex it painted through `map` (exact for flat-colored
/// geometry under projective maps; gradients and textures are interpolated affinely per triangle).
pub(crate) fn paint_warped(ui: &mut Ui, map: impl Fn(Pos2) -> Pos2, content: impl FnOnce(&mut Ui)) {
    let start = shape_mark(ui);
    content(ui);
    let cap = capture(ui, start);
    let ctx = ui.ctx().clone();
    let mut tess = tessellator(&ctx);
    let clip = ui.clip_rect();
    cap.restore(&ctx, |_, cs| {
        let mut meshes = Vec::new();
        to_meshes(&mut tess, cs.shape, &mut meshes);
        let shapes = meshes
            .into_iter()
            .map(|mut m| {
                for v in &mut m.vertices {
                    v.pos = map(v.pos);
                }
                Shape::mesh(m)
            })
            .collect();
        (clip, Shape::Vec(shapes))
    });
}

/// Is `p` inside the convex polygon `poly` (any winding)?
pub(crate) fn in_convex(poly: &[Pos2], p: Pos2) -> bool {
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut sign = 0.0f32;
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        let c = (b - a).x * (p - a).y - (b - a).y * (p - a).x;
        if c.abs() < 1e-6 {
            continue;
        }
        if sign == 0.0 {
            sign = c.signum();
        } else if c.signum() != sign {
            return false;
        }
    }
    true
}
