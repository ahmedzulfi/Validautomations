//! A banknote-shaped sheet simulated as cloth: grab it, drag it and fold it over itself.

use super::common::idle_for;
use egui::epaint::{Mesh, Vertex};
use egui::{
    pos2, vec2, Color32, ColorImage, Context, CursorIcon, Id, Pos2, Response, Sense, Shape, Stroke, TextureId, Ui, Vec2,
};
use glowkit_core::anim::{ease, lerp, Easing};
use glowkit_core::noise::noise2;
use glowkit_core::Theme;

const NX: usize = 19;
const NY: usize = 9;

/// The face printed on a [`FoldingBill`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BillPrint {
    /// Plain silver-gray paper.
    #[default]
    Plain,
    /// A green banknote engraving: frame, guilloche waves and medallions.
    Print,
}

/// A sheet of paper the size of a banknote, simulated as cloth (verlet particles with
/// stretch, shear and bend constraints) and drawn in perspective with lighting. Grab any point
/// and drag: it lifts off the table, the rest of the sheet follows and folds over itself, and
/// when you let go it drops and settles.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{BillPrint, FoldingBill};
/// ui.add(FoldingBill::new().print(BillPrint::Plain).stiffness(40.0).settle(0.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct FoldingBill {
    print: BillPrint,
    size: Vec2,
    stiffness: f32,
    settle: f32,
    texture: Option<TextureId>,
    autoplay: bool,
}

impl Default for FoldingBill {
    fn default() -> Self {
        Self::new()
    }
}

impl FoldingBill {
    /// A plain 300 x 130 sheet.
    pub fn new() -> Self {
        Self {
            print: BillPrint::Plain,
            size: vec2(300.0, 130.0),
            stiffness: 40.0,
            settle: 0.0,
            texture: None,
            autoplay: false,
        }
    }
    /// Face print.
    pub fn print(mut self, p: BillPrint) -> Self {
        self.print = p;
        self
    }
    /// Sheet size in points (at rest).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = s;
        self
    }
    /// Resistance to bending, 0..100 (low = cloth, high = card).
    pub fn stiffness(mut self, s: f32) -> Self {
        self.stiffness = s.clamp(0.0, 100.0);
        self
    }
    /// How strongly the sheet un-creases back to flat when left alone, 0..100.
    pub fn settle(mut self, s: f32) -> Self {
        self.settle = s.clamp(0.0, 100.0);
        self
    }
    /// Your own face texture (overrides [`FoldingBill::print`]).
    pub fn texture(mut self, tex: TextureId) -> Self {
        self.texture = Some(tex);
        self
    }
    /// Grab and fold corners by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

#[derive(Clone)]
struct Cloth {
    p: Vec<[f32; 3]>,
    prev: Vec<[f32; 3]>,
    rest: Vec<[f32; 3]>,
    grab: Option<usize>,
    grab_off: Vec2,
    size: Vec2,
    auto_cycle: i32,
    auto_from: Pos2,
}

impl Cloth {
    fn new(size: Vec2) -> Self {
        let mut rest = Vec::with_capacity(NX * NY);
        for j in 0..NY {
            for i in 0..NX {
                rest.push([
                    (i as f32 / (NX - 1) as f32 - 0.5) * size.x,
                    (j as f32 / (NY - 1) as f32 - 0.5) * size.y,
                    0.0,
                ]);
            }
        }
        // start slightly rotated, like the reference sheet tossed on a table
        let p: Vec<[f32; 3]> = rest
            .iter()
            .map(|r| {
                let (s, c) = (-0.05f32).sin_cos();
                [r[0] * c - r[1] * s, r[0] * s + r[1] * c, 0.0]
            })
            .collect();
        Self { prev: p.clone(), p, rest, grab: None, grab_off: Vec2::ZERO, size, auto_cycle: -1, auto_from: Pos2::ZERO }
    }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn len(a: [f32; 3]) -> f32 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norm(a: [f32; 3]) -> [f32; 3] {
    let l = len(a).max(1e-6);
    [a[0] / l, a[1] / l, a[2] / l]
}

const TILT: f32 = 0.52;
const FOCAL: f32 = 900.0;

fn project(c: Pos2, p: [f32; 3]) -> (Pos2, f32) {
    let (s, co) = TILT.sin_cos();
    let y = p[1] * co - p[2] * s;
    let depth = p[1] * s + p[2] * co;
    let k = FOCAL / (FOCAL - depth);
    (c + vec2(p[0], y) * k, depth)
}

fn unproject(c: Pos2, screen: Pos2, z: f32) -> [f32; 3] {
    let (s, co) = TILT.sin_cos();
    // iterate: perspective depends on y
    let mut x = screen.x - c.x;
    let mut y = (screen.y - c.y + z * s) / co;
    for _ in 0..4 {
        let depth = y * s + z * co;
        let k = FOCAL / (FOCAL - depth);
        x = (screen.x - c.x) / k;
        y = ((screen.y - c.y) / k + z * s) / co;
    }
    [x, y, z]
}

fn bill_texture(ctx: &Context, print: BillPrint) -> TextureId {
    let key = Id::unique(("glowkit_bill_tex", print as u8));
    if let Some(h) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(key)) {
        return h.id();
    }
    let (w, h) = (320usize, 140usize);
    let mut px = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let u = x as f32 / w as f32;
            let v = y as f32 / h as f32;
            let fiber = noise2(u * 60.0, v * 12.0) * 0.5 + noise2(u * 140.0, v * 140.0) * 0.5;
            let c: [f32; 3] = match print {
                BillPrint::Plain => {
                    let g = 0.9 + fiber * 0.03;
                    [g, g + 0.004, g + 0.012]
                }
                BillPrint::Print => {
                    let paper = [0.86 + fiber * 0.03, 0.9 + fiber * 0.03, 0.82 + fiber * 0.03];
                    let ink = [0.17, 0.36, 0.26];
                    let mut k: f32 = 0.0;
                    // double frame
                    let dx = (u - 0.5).abs() * 2.0;
                    let dy = (v - 0.5).abs() * 2.0;
                    let edge = (1.0 - dx) * w as f32 / 2.0;
                    let edgy = (1.0 - dy) * h as f32 / 2.0;
                    let m = edge.min(edgy);
                    if (5.0..7.0).contains(&m) || (11.0..12.0).contains(&m) {
                        k = 0.9;
                    }
                    // guilloche band between frames
                    if (7.0..11.0).contains(&m) {
                        let g = ((u * 90.0 + (v * 40.0).sin() * 2.0).sin() * (v * 70.0).cos()).abs();
                        k = k.max(if g > 0.7 { 0.6 } else { 0.1 });
                    }
                    // central medallion
                    let mc = vec2((u - 0.5) * w as f32 / h as f32, v - 0.5);
                    let r = mc.length();
                    if r < 0.3 {
                        let ring = ((r * 120.0).sin() * 0.5 + 0.5) * (1.0 - r / 0.3);
                        let a = mc.y.atan2(mc.x);
                        let rose = ((a * 12.0 + r * 40.0).sin() * 0.5 + 0.5) * 0.5;
                        k = k.max((ring * 0.6 + rose * 0.4) * 0.9);
                    }
                    if (0.29..0.31).contains(&r) {
                        k = 0.9;
                    }
                    // corner seals
                    for (cx, cy) in [(0.12, 0.27), (0.88, 0.73)] {
                        let d = vec2((u - cx) * w as f32 / h as f32, v - cy).length();
                        if d < 0.11 {
                            k = k.max(if d > 0.09 { 0.95 } else { ((d * 160.0).sin() * 0.5 + 0.5) * 0.55 });
                        }
                    }
                    // waves across the field
                    let wave = ((u * 26.0 + (v * 9.0).sin() * 1.4).sin() * 0.5 + 0.5).powi(12);
                    if m > 12.0 && r > 0.31 {
                        k = k.max(wave * 0.35);
                    }
                    [
                        paper[0] + (ink[0] - paper[0]) * k,
                        paper[1] + (ink[1] - paper[1]) * k,
                        paper[2] + (ink[2] - paper[2]) * k,
                    ]
                }
            };
            px.extend_from_slice(&[
                (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                255,
            ]);
        }
    }
    let img = ColorImage::from_rgba_unmultiplied([w, h], &px);
    let handle = ctx.load_texture(format!("glowkit-bill-{}", print as u8), img, egui::TextureOptions::LINEAR);
    let id = handle.id();
    ctx.data_mut(|d| d.insert_temp(key, handle));
    id
}

fn satisfy(p: &mut [[f32; 3]], a: usize, b: usize, rest: f32, k: f32, pinned: Option<usize>) {
    let d = sub(p[b], p[a]);
    let l = len(d).max(1e-5);
    let diff = (l - rest) / l * k;
    let (wa, wb) = match pinned {
        Some(g) if g == a => (0.0, 1.0),
        Some(g) if g == b => (1.0, 0.0),
        _ => (0.5, 0.5),
    };
    for (i, di) in d.iter().enumerate() {
        p[a][i] += di * diff * wa;
        p[b][i] -= di * diff * wb;
    }
}

impl egui::Widget for FoldingBill {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (outer, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), self.size.y * 1.9 + 40.0), Sense::click_and_drag());
        let id = response.id;
        let c = outer.center() + vec2(0.0, 10.0);
        let dt = glowkit_core::dt(ui).min(1.0 / 30.0);
        let mut cl: Cloth = glowkit_core::state(ui, id.with("cloth"), || Cloth::new(self.size));
        if cl.size != self.size {
            cl = Cloth::new(self.size);
        }
        let interacting = response.dragged() || response.is_pointer_button_down_on();
        let idle = idle_for(ui, id, interacting || response.hovered());
        let mut lift = 46.0;
        // grabbing
        let mut target: Option<Pos2> = None;
        if response.drag_started() || (response.is_pointer_button_down_on() && cl.grab.is_none()) {
            if let Some(p) = response.interact_pointer_pos() {
                let best = (0..cl.p.len())
                    .map(|i| (i, (project(c, cl.p[i]).0 - p).length()))
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                if let Some((i, d)) = best {
                    if d < 40.0 {
                        cl.grab = Some(i);
                        cl.grab_off = project(c, cl.p[i]).0 - p;
                    }
                }
            }
        }
        if interacting {
            target = response.interact_pointer_pos();
        } else if self.autoplay && idle > 1.0 {
            // scripted fold: grab a corner, sweep it over, drop
            let cycle = 4.2;
            let u = (idle - 1.0) % cycle;
            let k = ((idle - 1.0) / cycle) as i32;
            let corners = [NX - 1, (NY - 1) * NX, NX * NY - 1, 0];
            let corner = corners[k.rem_euclid(4) as usize];
            if k != cl.auto_cycle {
                cl.auto_cycle = k;
                cl.auto_from = project(c, cl.p[corner]).0;
            }
            if u < 2.2 {
                cl.grab = Some(corner);
                let from = cl.auto_from;
                let e = ease(Easing::InOutCubic, (u / 1.8).clamp(0.0, 1.0));
                let to = c + vec2(-cl.rest[corner][0] * 0.35, -cl.rest[corner][1] * 0.2);
                target = Some(pos2(lerp(from.x, to.x, e), lerp(from.y, to.y, e)));
                // lift the corner in an arc and lay it down on top of the sheet
                lift = 6.0 + 44.0 * (e * std::f32::consts::PI).sin();
            } else {
                cl.grab = None;
            }
        }
        if !interacting && target.is_none() {
            cl.grab = None;
        }
        // simulate
        let substeps = 3;
        let h = dt / substeps as f32;
        let gravity = 900.0;
        let bend_k = lerp(0.02, 0.6, self.stiffness / 100.0);
        // while autoplaying, the sheet relaxes between folds so it never ends up crumpled
        let auto_relax = self.autoplay && !interacting && cl.grab.is_none();
        let settle = if auto_relax { (self.settle / 100.0).max(0.45) } else { self.settle / 100.0 };
        let sx = self.size.x / (NX - 1) as f32;
        let sy = self.size.y / (NY - 1) as f32;
        let diag = (sx * sx + sy * sy).sqrt();
        let idx = |i: usize, j: usize| j * NX + i;
        for _ in 0..substeps {
            for k in 0..cl.p.len() {
                let cur = cl.p[k];
                let on_table = cur[2] <= 0.01;
                let fr = if on_table { 0.82 } else { 0.99 };
                let mut nx = [0.0; 3];
                for a in 0..3 {
                    nx[a] = cur[a] + (cur[a] - cl.prev[k][a]) * fr;
                }
                nx[2] -= gravity * h * h;
                cl.prev[k] = cur;
                cl.p[k] = nx;
            }
            if let (Some(g), Some(tp)) = (cl.grab, target) {
                let w = unproject(c, tp, lift);
                let gp = &mut cl.p[g];
                let d = sub(w, *gp);
                let l = len(d);
                let step = (l * 0.6).min(14.0) / l.max(1e-4);
                for a in 0..3 {
                    gp[a] += d[a] * step;
                }
            }
            for _ in 0..4 {
                for j in 0..NY {
                    for i in 0..NX {
                        if i + 1 < NX {
                            satisfy(&mut cl.p, idx(i, j), idx(i + 1, j), sx, 1.0, cl.grab);
                        }
                        if j + 1 < NY {
                            satisfy(&mut cl.p, idx(i, j), idx(i, j + 1), sy, 1.0, cl.grab);
                        }
                        if i + 1 < NX && j + 1 < NY {
                            satisfy(&mut cl.p, idx(i, j), idx(i + 1, j + 1), diag, 0.6, cl.grab);
                            satisfy(&mut cl.p, idx(i + 1, j), idx(i, j + 1), diag, 0.6, cl.grab);
                        }
                        if i + 2 < NX {
                            satisfy(&mut cl.p, idx(i, j), idx(i + 2, j), sx * 2.0, bend_k, cl.grab);
                        }
                        if j + 2 < NY {
                            satisfy(&mut cl.p, idx(i, j), idx(i, j + 2), sy * 2.0, bend_k, cl.grab);
                        }
                    }
                }
                for q in cl.p.iter_mut() {
                    if q[2] < 0.0 {
                        q[2] = 0.0;
                    }
                }
            }
            // table friction: points resting on the table resist sliding, so pulling a corner
            // folds the sheet instead of dragging it along
            for (k, q) in cl.p.iter_mut().enumerate() {
                if q[2] <= 0.5 && Some(k) != cl.grab {
                    let pv = cl.prev[k];
                    q[0] = pv[0] + (q[0] - pv[0]) * 0.3;
                    q[1] = pv[1] + (q[1] - pv[1]) * 0.3;
                }
            }
            if settle > 0.0 && cl.grab.is_none() {
                // pull toward the rest shape, aligned to the sheet's current center and heading
                let n = cl.p.len() as f32;
                let mut cen = [0.0; 3];
                for q in &cl.p {
                    for a in 0..2 {
                        cen[a] += q[a] / n;
                    }
                }
                let l = cl.p[idx(0, NY / 2)];
                let r = cl.p[idx(NX - 1, NY / 2)];
                let ang = (r[1] - l[1]).atan2(r[0] - l[0]);
                let (s, co) = ang.sin_cos();
                let k = settle * 6.0 * h;
                for (q, rr) in cl.p.iter_mut().zip(cl.rest.iter()) {
                    let tx = cen[0] + rr[0] * co - rr[1] * s;
                    let ty = cen[1] + rr[0] * s + rr[1] * co;
                    q[0] += (tx - q[0]) * k;
                    q[1] += (ty - q[1]) * k;
                    q[2] += (0.0 - q[2]) * k * 0.5;
                }
            }
        }
        // the table gently re-centres and straightens the sheet (a rigid move, folds survive)
        if cl.grab.is_none() {
            let n = cl.p.len() as f32;
            let mut cen = [0.0f32; 2];
            for q in &cl.p {
                cen[0] += q[0] / n;
                cen[1] += q[1] / n;
            }
            let l = cl.p[idx(0, NY / 2)];
            let r = cl.p[idx(NX - 1, NY / 2)];
            let mut ang = (r[1] - l[1]).atan2(r[0] - l[0]) + 0.05;
            if ang.abs() > std::f32::consts::FRAC_PI_2 {
                ang -= std::f32::consts::PI * ang.signum();
            }
            let k = (1.6 * dt).min(1.0);
            let (s, co) = (-ang * k).sin_cos();
            for (q, pv) in cl.p.iter_mut().zip(cl.prev.iter_mut()) {
                for v in [q, pv] {
                    let x = v[0] - cen[0];
                    let y = v[1] - cen[1];
                    v[0] = cen[0] * (1.0 - k) + x * co - y * s;
                    v[1] = cen[1] * (1.0 - k) + x * s + y * co;
                }
            }
        }
        let pts = cl.p.clone();
        glowkit_core::set_state(ui, id.with("cloth"), cl);
        glowkit_core::animate(ui);
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(if response.dragged() { CursorIcon::Grabbing } else { CursorIcon::Grab });
        }
        if !ui.is_rect_visible(outer) {
            return response;
        }
        let painter = ui.painter();
        // contact shadow: the sheet's outline flattened onto the table
        {
            let mut outline = Vec::new();
            for i in 0..NX {
                outline.push(idx(i, 0));
            }
            for j in 1..NY {
                outline.push(idx(NX - 1, j));
            }
            for i in (0..NX - 1).rev() {
                outline.push(idx(i, NY - 1));
            }
            for j in (1..NY - 1).rev() {
                outline.push(idx(0, j));
            }
            let flat: Vec<Pos2> =
                outline.iter().map(|&k| project(c, [pts[k][0], pts[k][1], 0.0]).0 + vec2(0.0, 5.0)).collect();
            let hull = convex_hull(flat);
            let sc =
                if t.dark { Color32::from_black_alpha(26) } else { Color32::from_rgba_unmultiplied(30, 30, 50, 9) };
            for g in [10.0f32, 6.0, 3.0, 0.0] {
                let cen = hull.iter().fold(Vec2::ZERO, |a, p| a + p.to_vec2()) / hull.len().max(1) as f32;
                let grown: Vec<Pos2> = hull.iter().map(|p| *p + (p.to_vec2() - cen).normalized() * g).collect();
                painter.add(Shape::convex_polygon(grown, sc, Stroke::NONE));
            }
        }
        let tex = self.texture.unwrap_or_else(|| bill_texture(ui.ctx(), self.print));
        let proj: Vec<(Pos2, f32)> = pts.iter().map(|q| project(c, *q)).collect();
        // per-vertex normals
        let mut normals = vec![[0.0f32, 0.0, 1.0]; pts.len()];
        for j in 0..NY {
            for i in 0..NX {
                let l = pts[idx(i.saturating_sub(1), j)];
                let r = pts[idx((i + 1).min(NX - 1), j)];
                let u = pts[idx(i, j.saturating_sub(1))];
                let d = pts[idx(i, (j + 1).min(NY - 1))];
                normals[idx(i, j)] = norm(cross(sub(r, l), sub(d, u)));
            }
        }
        let light = norm([-0.35, -0.55, 0.75]);
        let view = norm([0.0, TILT.sin(), TILT.cos()]);
        let half = norm([light[0] + view[0], light[1] + view[1], light[2] + view[2]]);
        let mut mesh = Mesh::with_texture(tex);
        for (k, q) in proj.iter().enumerate() {
            let n = normals[k];
            // facing: which side we see (front faces +z)
            let facing = n[0] * view[0] + n[1] * view[1] + n[2] * view[2];
            let nn = if facing < 0.0 { [-n[0], -n[1], -n[2]] } else { n };
            let diff = (nn[0] * light[0] + nn[1] * light[1] + nn[2] * light[2]).max(0.0);
            let spec = (nn[0] * half[0] + nn[1] * half[1] + nn[2] * half[2]).max(0.0).powf(24.0);
            let mut l = 0.38 + 0.62 * diff;
            if facing < 0.0 {
                l *= 0.86;
            }
            let shade = (l * 255.0).min(255.0) as u8;
            let col = Color32::from_rgb(shade, shade, (shade as f32 * 1.01).min(255.0) as u8);
            let col = glowkit_core::color::mix(col, Color32::WHITE, spec * 0.75);
            let i = k % NX;
            let j = k / NX;
            mesh.vertices.push(Vertex {
                pos: q.0,
                uv: pos2(i as f32 / (NX - 1) as f32, j as f32 / (NY - 1) as f32),
                color: col,
            });
        }
        // painter's algorithm: far quads first
        let mut quads: Vec<(f32, usize, usize)> = Vec::with_capacity((NX - 1) * (NY - 1));
        for j in 0..NY - 1 {
            for i in 0..NX - 1 {
                let d = proj[idx(i, j)].1 + proj[idx(i + 1, j)].1 + proj[idx(i, j + 1)].1 + proj[idx(i + 1, j + 1)].1;
                quads.push((d, i, j));
            }
        }
        quads.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        for (_, i, j) in quads {
            let (a, b, cc, d) =
                (idx(i, j) as u32, idx(i + 1, j) as u32, idx(i + 1, j + 1) as u32, idx(i, j + 1) as u32);
            mesh.add_triangle(a, b, cc);
            mesh.add_triangle(a, cc, d);
        }
        painter.add(Shape::mesh(mesh));
        response
    }
}

fn convex_hull(mut pts: Vec<Pos2>) -> Vec<Pos2> {
    pts.sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });
    if pts.len() < 3 {
        return pts;
    }
    let cross = |o: Pos2, a: Pos2, b: Pos2| (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
    let mut lower: Vec<Pos2> = Vec::new();
    for p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], *p) <= 0.0 {
            lower.pop();
        }
        lower.push(*p);
    }
    let mut upper: Vec<Pos2> = Vec::new();
    for p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], *p) <= 0.0 {
            upper.pop();
        }
        upper.push(*p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}
