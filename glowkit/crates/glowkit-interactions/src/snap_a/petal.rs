use super::common::{h01, idle_time, sstep, wall_dt, Ink};
use egui::epaint::{Mesh, Vertex};
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::{hex, Theme};
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, Default)]
struct PetalState {
    /// Scene time: 0 = whole word; grows while dissolving, shrinks while rewinding.
    t: f32,
    forward: bool,
    hold: f32,
}

const FLIGHT: f32 = 3.6;

/// A word that erodes into tumbling red and white petals, then time runs backwards and every
/// petal flies home to rebuild it. Each petal is a real fragment of the glyph (cut from the
/// font atlas) that turns into a petal as it leaves, and its path is a pure function of time,
/// so the rewind is exact.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::PetalDissolve::new("Ember").size(72.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PetalDissolve {
    word: String,
    size: f32,
    colors: Option<[Color32; 2]>,
    wind: f32,
    density: usize,
    progress: Option<f32>,
    height: f32,
    autoplay: bool,
}

impl PetalDissolve {
    /// Dissolve `word`.
    pub fn new(word: impl Into<String>) -> Self {
        Self {
            word: word.into(),
            size: 72.0,
            colors: None,
            wind: 1.0,
            density: 7,
            progress: None,
            height: 300.0,
            autoplay: true,
        }
    }
    /// Font size (default 72).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.clamp(16.0, 200.0);
        self
    }
    /// The two petal colors (default crimson and white; crimson and blush on light themes).
    pub fn colors(mut self, c: [Color32; 2]) -> Self {
        self.colors = Some(c);
        self
    }
    /// Wind strength (default 1).
    pub fn wind(mut self, k: f32) -> Self {
        self.wind = k.clamp(0.0, 3.0);
        self
    }
    /// Petals per glyph edge (default 7; total ≈ density² per letter).
    pub fn density(mut self, n: usize) -> Self {
        self.density = n.clamp(2, 14);
        self
    }
    /// Drive it directly: `0` the whole word, `1` every petal gone.
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = Some(p.clamp(0.0, 1.0));
        self
    }
    /// Widget height (default 300).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(self.size * 2.0);
        self
    }
    /// Dissolve and rebuild on a loop while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn petal_shape(c: Pos2, len: f32, angle: f32, flip: f32) -> Vec<Pos2> {
    // a soft teardrop, squashed by `flip` to fake a 3D tumble
    let (s, co) = angle.sin_cos();
    (0..14)
        .map(|i| {
            let a = i as f32 / 14.0 * PI * 2.0;
            let r = 1.0 - 0.35 * (a * 0.5).sin().powi(2) * 0.0;
            let x = a.cos() * len * 0.5 * r;
            let y = a.sin() * len * 0.28 * (0.6 + 0.4 * (1.0 + a.cos()) * 0.5) * flip;
            c + vec2(x * co - y * s, x * s + y * co)
        })
        .collect()
}

impl Widget for PetalDissolve {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let wdt = wall_dt(ui);
        let idle = idle_time(ui, id, rect);
        let word_c = rect.center() - vec2(0.0, 24.0);
        let btn = Rect::from_center_size(pos2(rect.center().x, rect.max.y - 40.0), vec2(88.0, 26.0));
        let btn_r = ui.interact(btn, id.with("btn"), Sense::click());
        if btn_r.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let clicked = btn_r.clicked();
        let span = FLIGHT + 1.2;
        let autoplay = self.autoplay && idle > 1.5;
        let progress = self.progress;
        let st = glowkit_core::with_state(ui, id, PetalState::default, |s| {
            if let Some(p) = progress {
                s.t = p * span;
                return *s;
            }
            if clicked {
                s.forward = !s.forward;
                s.hold = 0.0;
            }
            if s.forward {
                s.t = (s.t + dt).min(span);
            } else {
                // the rewind runs a little faster than the fall
                s.t = (s.t - dt * 1.35).max(0.0);
            }
            if autoplay && !clicked {
                let parked = (s.forward && s.t >= span) || (!s.forward && s.t <= 0.0);
                if parked {
                    s.hold += wdt;
                    if s.hold > if s.forward { 0.8 } else { 1.4 } {
                        s.forward = !s.forward;
                        s.hold = 0.0;
                    }
                }
            }
            *s
        });
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let ink_c = if t.dark { hex("#E8E2D6") } else { hex("#1C1917") };
        let colors = self.colors.unwrap_or(if t.dark {
            [hex("#C8102E"), hex("#F4F1EA")]
        } else {
            [hex("#C8102E"), hex("#F6B3BF")]
        });
        // tessellate the word once into glyph quads
        let font = FontId::proportional(self.size);
        let mut ink = Ink::new(&painter);
        ink.text(word_c, Align2::CENTER_CENTER, &self.word, font, ink_c);
        let mesh = ink.mesh(ui.ctx(), 1e6);
        let g = self.density;
        let mut letters = Mesh::default();
        let mut petals = Mesh::default();
        let word_w = rect.width() * 0.5;
        let wind = self.wind;
        let mut idx = 0u64;
        // each glyph quad is two triangles; rebuild quads from consecutive triangle pairs
        for tri in mesh.indices.chunks_exact(6) {
            let mut vs: Vec<Vertex> = tri.iter().map(|i| mesh.vertices[*i as usize]).collect();
            vs.dedup_by(|a, b| a.pos == b.pos);
            let mut uniq: Vec<Vertex> = Vec::new();
            for v in &vs {
                if !uniq.iter().any(|u| u.pos == v.pos) {
                    uniq.push(*v);
                }
            }
            if uniq.len() != 4 {
                continue;
            }
            let min = uniq.iter().fold(pos2(f32::MAX, f32::MAX), |m, v| pos2(m.x.min(v.pos.x), m.y.min(v.pos.y)));
            let max = uniq.iter().fold(pos2(f32::MIN, f32::MIN), |m, v| pos2(m.x.max(v.pos.x), m.y.max(v.pos.y)));
            let uv_at = |p: Pos2| -> Pos2 {
                // bilinear from the quad's corners (axis-aligned glyph quads)
                let tl = uniq.iter().find(|v| v.pos == min).map(|v| v.uv);
                let br = uniq.iter().find(|v| v.pos == max).map(|v| v.uv);
                match (tl, br) {
                    (Some(a), Some(b)) => {
                        let fx = (p.x - min.x) / (max.x - min.x).max(1e-3);
                        let fy = (p.y - min.y) / (max.y - min.y).max(1e-3);
                        pos2(a.x + (b.x - a.x) * fx, a.y + (b.y - a.y) * fy)
                    }
                    _ => uniq[0].uv,
                }
            };
            let cw = (max.x - min.x) / g as f32;
            let ch = (max.y - min.y) / (g as f32 * 1.3).round();
            let gy = ((max.y - min.y) / ch).round() as usize;
            for j in 0..gy {
                for i in 0..g {
                    idx += 1;
                    let r0 = pos2(min.x + i as f32 * cw, min.y + j as f32 * ch);
                    let r1 = r0 + vec2(cw, ch);
                    let home = r0.lerp(r1, 0.5);
                    // erosion sweeps from left to right with some scatter
                    let delay =
                        ((home.x - (word_c.x - word_w * 0.5)) / word_w).clamp(0.0, 1.0) * 1.4 + h01(idx * 7) * 0.5;
                    let tau = st.t - delay;
                    let piece = 1.0 - sstep(0.0, 0.18, tau);
                    if piece > 0.0 {
                        let off = if tau > 0.0 { path(tau, idx, wind) * 0.3 } else { Vec2::ZERO };
                        let base = letters.vertices.len() as u32;
                        for p in [r0, pos2(r1.x, r0.y), r1, pos2(r0.x, r1.y)] {
                            letters.vertices.push(Vertex {
                                pos: p + off,
                                uv: uv_at(p),
                                color: with_alpha(ink_c, piece),
                            });
                        }
                        letters.add_triangle(base, base + 1, base + 2);
                        letters.add_triangle(base, base + 2, base + 3);
                    }
                    if tau > 0.0 {
                        let a = sstep(0.0, 0.2, tau) * (1.0 - sstep(FLIGHT - 1.0, FLIGHT, tau));
                        if a > 0.01 {
                            let pos = home + path(tau, idx, wind);
                            let spin = tau * (2.0 + h01(idx * 3) * 4.0) * if h01(idx * 5) > 0.5 { 1.0 } else { -1.0 };
                            let flip = (tau * (3.0 + h01(idx * 11) * 5.0)).cos().abs().max(0.15);
                            let col = if h01(idx * 13) > 0.45 { colors[0] } else { colors[1] };
                            let len = (cw * 1.9).clamp(4.0, 14.0) * (0.8 + 0.5 * h01(idx * 17));
                            let pts = petal_shape(pos, len, spin, flip);
                            let shade = 0.75 + 0.25 * flip;
                            let c = with_alpha(super::common::lit(col, shade), a);
                            let b = petals.vertices.len() as u32;
                            petals.colored_vertex(pos, c);
                            for p in &pts {
                                petals.colored_vertex(*p, c);
                            }
                            let m = pts.len() as u32;
                            for k in 0..m {
                                petals.add_triangle(b, b + 1 + k, b + 1 + (k + 1) % m);
                            }
                        }
                    }
                }
            }
        }
        painter.add(Shape::mesh(letters));
        painter.add(Shape::mesh(petals));
        // button
        let label = if st.forward { "RETURN" } else { "BEGIN" };
        let bc = with_alpha(ink_c, 0.6);
        let pts = vec![
            btn.left_center(),
            btn.left_top() + vec2(8.0, 0.0),
            btn.right_top() - vec2(8.0, 0.0),
            btn.right_center(),
            btn.right_bottom() - vec2(8.0, 0.0),
            btn.left_bottom() + vec2(8.0, 0.0),
        ];
        painter.add(Shape::closed_line(pts, Stroke::new(1.0, bc)));
        painter.text(btn.center(), Align2::CENTER_CENTER, label, FontId::proportional(10.0), bc);
        response
    }
}

/// Offset of petal `idx` after `tau` seconds of flight: blown downwind, lifted, looping.
fn path(tau: f32, idx: u64, wind: f32) -> Vec2 {
    let r = |k: u64| h01(idx * 31 + k);
    let vx = (60.0 + r(1) * 140.0) * wind;
    let vy = -30.0 - r(2) * 50.0;
    let loop_r = 10.0 + r(3) * 26.0;
    let w = 2.0 + r(4) * 3.0;
    let ph = r(5) * PI * 2.0;
    vec2(
        vx * tau + (tau * w + ph).sin() * loop_r - ph.sin() * loop_r + (r(6) - 0.5) * 120.0 * tau,
        vy * tau + 22.0 * tau * tau + (tau * w * 0.7 + ph).cos() * loop_r * 0.6 - ph.cos() * loop_r * 0.6,
    )
}
