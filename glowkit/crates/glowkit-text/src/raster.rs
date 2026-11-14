//! Glyph rasterization helpers shared by the "big text" effects: sample how much ink a galley
//! puts in each cell of a grid (read back from the font atlas), and paint galleys with finely
//! subdivided meshes so a color field can vary *inside* each glyph.

use egui::epaint::{Mesh, Vertex};
use egui::{pos2, vec2, Color32, Galley, Painter, Pos2, Rect, Vec2};
use std::sync::Arc;

/// Ink coverage of a galley sampled on a regular grid (galley-local points).
#[derive(Clone, Debug)]
pub(crate) struct Coverage {
    pub cols: usize,
    pub rows: usize,
    /// Size of one cell in points.
    pub cell: Vec2,
    /// Row-major `0..1` coverage, `cols * rows` values.
    pub data: Vec<f32>,
}

impl Coverage {
    /// Coverage of cell `(cx, cy)`; 0 outside the grid.
    pub fn at(&self, cx: i32, cy: i32) -> f32 {
        if cx < 0 || cy < 0 || cx >= self.cols as i32 || cy >= self.rows as i32 {
            0.0
        } else {
            self.data[cy as usize * self.cols + cx as usize]
        }
    }

    /// Bilinear coverage at a galley-local point.
    pub fn sample(&self, p: Vec2) -> f32 {
        let x = p.x / self.cell.x - 0.5;
        let y = p.y / self.cell.y - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (x0, y0) = (x0 as i32, y0 as i32);
        let a = self.at(x0, y0) + (self.at(x0 + 1, y0) - self.at(x0, y0)) * fx;
        let b = self.at(x0, y0 + 1) + (self.at(x0 + 1, y0 + 1) - self.at(x0, y0 + 1)) * fx;
        a + (b - a) * fy
    }
}

/// One textured glyph quad of a galley: local position rect and atlas texel rect.
struct Quad {
    pos: Rect,
    uv: Rect,
}

fn quads(galley: &Galley) -> Vec<Quad> {
    let mut out = Vec::new();
    for row in &galley.rows {
        let off = row.pos.to_vec2();
        for chunk in row.visuals.mesh.vertices.chunks_exact(4) {
            let mut pos = Rect::NOTHING;
            let mut uv = Rect::NOTHING;
            for v in chunk {
                pos.extend_with(v.pos + off);
                uv.extend_with(v.uv);
            }
            if pos.width() > 0.0 && pos.height() > 0.0 {
                out.push(Quad { pos, uv });
            }
        }
    }
    out
}

/// Sample the ink coverage of `galley` on a grid of `cell`-sized cells covering `size` (usually
/// the galley size). Reads the font atlas once; cache the result, it is not cheap.
pub(crate) fn coverage(painter: &Painter, galley: &Arc<Galley>, size: Vec2, cell: Vec2) -> Coverage {
    let cell = vec2(cell.x.max(0.5), cell.y.max(0.5));
    let cols = (size.x / cell.x).ceil().max(1.0) as usize;
    let rows = (size.y / cell.y).ceil().max(1.0) as usize;
    let quads = quads(galley);
    let mut data = vec![0.0f32; cols * rows];
    if quads.is_empty() {
        return Coverage { cols, rows, cell, data };
    }
    painter.fonts(|f| {
        let img = f.image();
        let [w, h] = img.size;
        let alpha = |u: f32, v: f32| -> f32 {
            let x = (u.floor() as i64).clamp(0, w as i64 - 1) as usize;
            let y = (v.floor() as i64).clamp(0, h as i64 - 1) as usize;
            img.pixels[y * w + x].a() as f32 / 255.0
        };
        const SS: usize = 3;
        for cy in 0..rows {
            for cx in 0..cols {
                let mut sum = 0.0;
                for sy in 0..SS {
                    for sx in 0..SS {
                        let p = pos2(
                            (cx as f32 + (sx as f32 + 0.5) / SS as f32) * cell.x,
                            (cy as f32 + (sy as f32 + 0.5) / SS as f32) * cell.y,
                        );
                        for q in &quads {
                            if q.pos.contains(p) {
                                let t = (p - q.pos.min) / q.pos.size();
                                let a = alpha(q.uv.min.x + t.x * q.uv.width(), q.uv.min.y + t.y * q.uv.height());
                                sum += a;
                                break;
                            }
                        }
                    }
                }
                data[cy * cols + cx] = (sum / (SS * SS) as f32).min(1.0);
            }
        }
    });
    Coverage { cols, rows, cell, data }
}

/// Paint `galley` with every glyph quad subdivided into cells of about `step` points, mapping
/// galley-local vertex positions through `map` and coloring each vertex with `color(screen)`.
/// This lets smooth color fields (plasma, aurora, halftone shading) vary inside the letters.
/// The mesh is built once and repeated at each of `copies` (screen offsets, e.g. for fake bold).
pub(crate) fn paint_galley_field(
    painter: &Painter,
    galley: &Arc<Galley>,
    step: f32,
    map: impl Fn(Vec2) -> Pos2,
    color: impl Fn(Pos2) -> Color32,
    copies: &[Vec2],
) {
    let [tw, th] = painter.fonts(|f| f.font_image_size());
    let norm = vec2(1.0 / tw.max(1) as f32, 1.0 / th.max(1) as f32);
    let step = step.max(1.0);
    let mut mesh = Mesh::default();
    for q in quads(galley) {
        let nx = (q.pos.width() / step).ceil().clamp(1.0, 64.0) as u32;
        let ny = (q.pos.height() / step).ceil().clamp(1.0, 64.0) as u32;
        let base = mesh.vertices.len() as u32;
        for j in 0..=ny {
            for i in 0..=nx {
                let t = vec2(i as f32 / nx as f32, j as f32 / ny as f32);
                let local = q.pos.min.to_vec2() + q.pos.size() * t;
                let uv = q.uv.min + q.uv.size() * t;
                let pos = map(local);
                mesh.vertices.push(Vertex { pos, uv: pos2(uv.x * norm.x, uv.y * norm.y), color: color(pos) });
            }
        }
        let w = nx + 1;
        for j in 0..ny {
            for i in 0..nx {
                let a = base + j * w + i;
                mesh.add_triangle(a, a + 1, a + w + 1);
                mesh.add_triangle(a, a + w + 1, a + w);
            }
        }
    }
    if mesh.is_empty() {
        return;
    }
    for (i, off) in copies.iter().enumerate() {
        if i + 1 == copies.len() {
            mesh.translate(*off);
            painter.add(egui::Shape::mesh(mesh));
            break;
        }
        let mut m = mesh.clone();
        m.translate(*off);
        painter.add(egui::Shape::mesh(m));
    }
}

/// Hash a string and a few floats into a cache key.
pub(crate) fn key(text: &str, nums: &[f32]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    for n in nums {
        n.to_bits().hash(&mut h);
    }
    h.finish()
}
