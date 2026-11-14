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

#[inline]
fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_premultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

#[inline]
fn lerp_vertex(a: &Vertex, b: &Vertex, t: f32) -> Vertex {
    Vertex { pos: a.pos.lerp(b.pos, t), uv: a.uv.lerp(b.uv, t), color: lerp_color(a.color, b.color, t) }
}

fn push_tri(out: &mut Mesh, v: [Vertex; 3]) {
    let i = out.vertices.len() as u32;
    out.vertices.extend_from_slice(&v);
    out.add_triangle(i, i + 1, i + 2);
}

/// Split every triangle (whose bounds touch `region`) until no edge is longer than `max_len`,
/// so the mesh can be bent smoothly. Triangles outside `region` are dropped.
pub(crate) fn subdivide(mesh: &Mesh, max_len: f32, region: Rect) -> Mesh {
    let mut out = Mesh::with_texture(mesh.texture_id);
    let mut stack: Vec<[Vertex; 3]> = Vec::new();
    let max2 = max_len * max_len;
    for tri in mesh.indices.chunks_exact(3) {
        let v = [mesh.vertices[tri[0] as usize], mesh.vertices[tri[1] as usize], mesh.vertices[tri[2] as usize]];
        let bb = Rect::from_points(&[v[0].pos, v[1].pos, v[2].pos]);
        if !bb.intersects(region) {
            continue;
        }
        stack.push(v);
        while let Some([a, b, c]) = stack.pop() {
            let ab = a.pos.distance_sq(b.pos);
            let bc = b.pos.distance_sq(c.pos);
            let ca = c.pos.distance_sq(a.pos);
            let m = ab.max(bc).max(ca);
            if m <= max2 || out.vertices.len() > 240_000 {
                push_tri(&mut out, [a, b, c]);
                continue;
            }
            if m == ab {
                let mid = lerp_vertex(&a, &b, 0.5);
                stack.push([a, mid, c]);
                stack.push([mid, b, c]);
            } else if m == bc {
                let mid = lerp_vertex(&b, &c, 0.5);
                stack.push([a, b, mid]);
                stack.push([a, mid, c]);
            } else {
                let mid = lerp_vertex(&c, &a, 0.5);
                stack.push([a, b, mid]);
                stack.push([mid, b, c]);
            }
        }
    }
    out
}

/// Clip a mesh to a **convex** polygon (any winding), interpolating uv and color on cut edges.
pub(crate) fn clip_mesh(mesh: &Mesh, poly: &[Pos2]) -> Mesh {
    let mut out = Mesh::with_texture(mesh.texture_id);
    if poly.len() < 3 {
        return out;
    }
    let mut area = 0.0;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        area += a.x * b.y - b.x * a.y;
    }
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let edges: Vec<(Pos2, Vec2)> = (0..poly.len()).map(|i| (poly[i], poly[(i + 1) % poly.len()] - poly[i])).collect();
    let side = |p: Pos2, (a, d): (Pos2, Vec2)| (d.x * (p - a).y - d.y * (p - a).x) * sign;
    let bounds = Rect::from_points(poly);
    let mut buf: Vec<Vertex> = Vec::with_capacity(12);
    let mut nxt: Vec<Vertex> = Vec::with_capacity(12);
    for tri in mesh.indices.chunks_exact(3) {
        let v = [mesh.vertices[tri[0] as usize], mesh.vertices[tri[1] as usize], mesh.vertices[tri[2] as usize]];
        let bb = Rect::from_points(&[v[0].pos, v[1].pos, v[2].pos]);
        if !bb.intersects(bounds) {
            continue;
        }
        if v.iter().all(|x| edges.iter().all(|e| side(x.pos, *e) >= 0.0)) {
            push_tri(&mut out, v);
            continue;
        }
        buf.clear();
        buf.extend_from_slice(&v);
        for e in &edges {
            if buf.is_empty() {
                break;
            }
            nxt.clear();
            for j in 0..buf.len() {
                let p = buf[j];
                let q = buf[(j + 1) % buf.len()];
                let sp = side(p.pos, *e);
                let sq = side(q.pos, *e);
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
    out
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
pub(crate) fn paint_avatar(painter: &Painter, rect: Rect, radius: f32, colors: [Color32; 2], seed: u32, backdrop: bool) {
    let [a, b] = colors;
    if backdrop {
        paint::gradient_rect(painter, rect, radius, 120.0, &[shade(a, 0.08), b]);
    }
    let w = rect.width();
    let h = rect.height();
    let cx = rect.center().x + ((seed % 3) as f32 - 1.0) * w * 0.04;
    if backdrop {
        // backlight
        paint::glow(painter, pos2(cx, rect.min.y + h * 0.38), w * 0.55, with_alpha(Color32::WHITE, 0.28));
        // decorative rings
        let ring_c = pos2(rect.min.x + w * (0.2 + 0.6 * ((seed * 7 % 5) as f32 / 4.0)), rect.min.y + h * 0.18);
        painter.circle_stroke(ring_c, w * 0.16, Stroke::new(1.5, with_alpha(Color32::WHITE, 0.22)));
        painter.circle_filled(ring_c + vec2(w * 0.22, h * 0.1), w * 0.025, with_alpha(Color32::WHITE, 0.4));
    }
    let body = mix(shade(b, -0.42), Color32::from_rgb(12, 10, 24), 0.45);
    let skin = mix(body, a, 0.18);
    let head_r = w * 0.16;
    let head = pos2(cx, rect.min.y + h * 0.47);
    // shoulders
    painter.add(Shape::ellipse_filled(pos2(cx, rect.max.y + h * 0.06), vec2(w * 0.4, h * 0.3), body));
    // neck
    painter.rect_filled(
        Rect::from_center_size(pos2(cx, head.y + head_r * 1.2), vec2(head_r * 0.85, head_r * 1.4)),
        egui::CornerRadius::same(4),
        skin,
    );
    painter.circle_filled(head, head_r, skin);
    // hair
    let hair = shade(body, -0.25);
    match seed % 3 {
        0 => {
            painter.add(Shape::ellipse_filled(head - vec2(0.0, head_r * 0.45), vec2(head_r * 1.08, head_r * 0.7), hair));
        }
        1 => {
            painter.add(Shape::ellipse_filled(head - vec2(0.0, head_r * 0.2), vec2(head_r * 1.2, head_r * 1.1), hair));
            painter.circle_filled(head + vec2(0.0, head_r * 0.12), head_r * 0.9, skin);
        }
        _ => {
            painter.add(Shape::ellipse_filled(
                head - vec2(head_r * 0.15, head_r * 0.55),
                vec2(head_r * 1.0, head_r * 0.55),
                hair,
            ));
        }
    }
    // rim light on the shoulder / head
    paint::arc(painter, head, head_r - 0.5, -2.6, -1.2, 1.6, with_alpha(Color32::WHITE, 0.35));
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
    if let Some(p) = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p) && ui.rect_contains_pointer(rect)) {
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
pub(crate) fn soft_ribbon(painter: &Painter, pts: &[Pos2], closed: bool, mut f: impl FnMut(Pos2) -> (f32, f32, Color32)) {
    let n = pts.len();
    if n < 2 {
        return;
    }
    let mut mesh = Mesh::default();
    let count = if closed { n + 1 } else { n };
    for k in 0..count {
        let i = k % n;
        let p = pts[i];
        let prev = if closed { pts[(i + n - 1) % n] } else { pts[i.saturating_sub(1)] };
        let next = if closed { pts[(i + 1) % n] } else { pts[(i + 1).min(n - 1)] };
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
