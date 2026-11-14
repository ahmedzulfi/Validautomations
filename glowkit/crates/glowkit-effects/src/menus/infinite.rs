use super::common::bold_text;
use crate::motion::CardFace;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, InnerResponse, Pos2, Sense, Stroke, Ui, Vec2};
use glowkit_core::anim::{follow, spring};
use glowkit_core::color::{gradient, mix, shade, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::PI;

type M3 = [f32; 9];
type V3 = [f32; 3];

fn mul(a: &M3, b: &M3) -> M3 {
    let mut r = [0.0; 9];
    for i in 0..3 {
        for j in 0..3 {
            r[i * 3 + j] = (0..3).map(|k| a[i * 3 + k] * b[k * 3 + j]).sum();
        }
    }
    r
}

fn apply(m: &M3, v: V3) -> V3 {
    [
        m[0] * v[0] + m[1] * v[1] + m[2] * v[2],
        m[3] * v[0] + m[4] * v[1] + m[5] * v[2],
        m[6] * v[0] + m[7] * v[1] + m[8] * v[2],
    ]
}

/// Rotation of `angle` radians about the unit `axis` (Rodrigues).
fn axis_angle(axis: V3, angle: f32) -> M3 {
    let (s, c) = angle.sin_cos();
    let t = 1.0 - c;
    let [x, y, z] = axis;
    [
        t * x * x + c,
        t * x * y - s * z,
        t * x * z + s * y,
        t * x * y + s * z,
        t * y * y + c,
        t * y * z - s * x,
        t * x * z - s * y,
        t * y * z + s * x,
        t * z * z + c,
    ]
}

/// Re-orthonormalize a rotation matrix so drift never accumulates.
fn orthonormalize(m: &M3) -> M3 {
    let norm = |v: V3| {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
        [v[0] / l, v[1] / l, v[2] / l]
    };
    let cross = |a: V3, b: V3| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let r0 = norm([m[0], m[1], m[2]]);
    let r1 = [m[3], m[4], m[5]];
    let r2 = norm(cross(r0, r1));
    let r1 = cross(r2, r0);
    [r0[0], r0[1], r0[2], r1[0], r1[1], r1[2], r2[0], r2[1], r2[2]]
}

fn fibonacci_sphere(n: usize) -> Vec<V3> {
    let golden = PI * (3.0 - 5f32.sqrt());
    (0..n)
        .map(|i| {
            let y = 1.0 - (i as f32 + 0.5) / n as f32 * 2.0;
            let r = (1.0 - y * y).sqrt();
            let th = golden * i as f32;
            [th.cos() * r, y, th.sin() * r]
        })
        .collect()
}

#[derive(Clone, Copy, Debug)]
struct Sphere {
    rot: M3,
    vel: Vec2,
    last_user: f64,
}

/// A sphere of round tiles you grab and spin. It coasts with inertia, then gently snaps the
/// nearest tile to face you; that tile grows, its title appears on the left, its description on
/// the right, and a round action button pops up underneath.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{CardFace, InfiniteMenu};
/// let items = vec![CardFace::new("🌊", "Ocean").subtitle("Deep blue calm")];
/// let r = InfiniteMenu::new(items).show(ui);
/// if let Some(i) = r.inner { /* action button of item i clicked */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct InfiniteMenu {
    items: Vec<CardFace>,
    height: f32,
    tiles: usize,
    autoplay: bool,
}

impl InfiniteMenu {
    /// A sphere menu showing these items (repeated to cover the sphere).
    pub fn new(items: Vec<CardFace>) -> Self {
        Self { items, height: 380.0, tiles: 30, autoplay: false }
    }
    /// Height of the widget.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(200.0);
        self
    }
    /// Number of tiles on the sphere (items repeat), 8..=96.
    pub fn tiles(mut self, n: usize) -> Self {
        self.tiles = n.clamp(8, 96);
        self
    }
    /// Spin and settle on its own while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the menu. `inner` is the front item when its action button is clicked.
    pub fn show(self, ui: &mut Ui) -> InnerResponse<Option<usize>> {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::click_and_drag());
        let id = resp.id;
        if self.items.is_empty() || !ui.is_rect_visible(rect) {
            return InnerResponse::new(None, resp);
        }
        let dragging_now = resp.dragged();
        let resp = resp.on_hover_cursor(if dragging_now { CursorIcon::Grabbing } else { CursorIcon::Grab });
        let dt = glowkit_core::dt(ui);
        let now = ui.input(|i| i.time);
        let hovered = ui.rect_contains_pointer(rect);
        let dragging = resp.dragged();
        let delta = if dragging { ui.input(|i| i.pointer.delta()) } else { Vec2::ZERO };
        let radius = (self.height * 0.36).min(rect.width() * 0.26);
        let pts = fibonacci_sphere(self.tiles);
        let autoplay = self.autoplay && !hovered;

        let (rot, vel) = glowkit_core::with_state(
            ui,
            id.with("sphere"),
            || Sphere { rot: axis_angle([1.0, 0.0, 0.0], 0.25), vel: Vec2::ZERO, last_user: -100.0 },
            |s| {
                if dragging {
                    let a = delta / radius;
                    let r = mul(&axis_angle([0.0, 1.0, 0.0], a.x), &axis_angle([1.0, 0.0, 0.0], a.y));
                    s.rot = mul(&r, &s.rot);
                    let v = vec2(a.x, a.y) / dt.max(1e-3);
                    s.vel = s.vel * 0.5 + v * 0.5;
                    s.last_user = now;
                } else {
                    let auto = autoplay && now - s.last_user > 3.0;
                    if auto && (now % 4.0) < 1.3 {
                        s.vel = s.vel + (vec2(1.3, 0.35) - s.vel) * (dt * 4.0).min(1.0);
                    } else {
                        s.vel *= (-dt * 3.2).exp();
                    }
                    let r = mul(&axis_angle([0.0, 1.0, 0.0], s.vel.x * dt), &axis_angle([1.0, 0.0, 0.0], s.vel.y * dt));
                    s.rot = mul(&r, &s.rot);
                    if s.vel.length() < 0.9 {
                        // snap the front-most tile to face the viewer
                        let mut best = (f32::MIN, [0.0, 0.0, 1.0]);
                        for p in &pts {
                            let v = apply(&s.rot, *p);
                            if v[2] > best.0 {
                                best = (v[2], v);
                            }
                        }
                        let v = best.1;
                        let axis = [v[1], -v[0], 0.0];
                        let l = (axis[0] * axis[0] + axis[1] * axis[1]).sqrt();
                        if l > 1e-5 {
                            let ang = v[2].clamp(-1.0, 1.0).acos() * (dt * 6.0).min(1.0);
                            s.rot = mul(&axis_angle([axis[0] / l, axis[1] / l, 0.0], ang), &s.rot);
                        }
                    }
                }
                s.rot = orthonormalize(&s.rot);
                (s.rot, s.vel)
            },
        );
        glowkit_core::animate(ui);

        // project tiles
        let center = rect.center() - vec2(0.0, 14.0);
        let persp = 3.2;
        let mut tiles: Vec<(usize, V3)> = pts.iter().enumerate().map(|(i, p)| (i, apply(&rot, *p))).collect();
        tiles.sort_by(|a, b| a.1[2].total_cmp(&b.1[2]));
        let front = tiles.last().map(|x| x.0).unwrap_or(0);
        let front_z = tiles.last().map(|x| x.1[2]).unwrap_or(1.0);
        let settled_raw = if dragging { 0.0 } else { ((front_z - 0.96) / 0.035).clamp(0.0, 1.0) * (1.0 - vel.length() / 1.2).clamp(0.0, 1.0) };
        let settled = follow(ui, id.with("settled"), settled_raw, 9.0);
        let move_k = spring(ui, id.with("move"), if dragging || vel.length() > 0.9 { 1.0 } else { 0.0 }, 220.0, 24.0);
        let painter = ui.painter_at(rect);

        paint::glow(&painter, center, radius * 1.9, with_alpha(t.accent, if t.dark { 0.16 } else { 0.1 }));
        // sphere outline hint
        painter.circle_stroke(center, radius * 1.02, Stroke::new(1.0, with_alpha(t.foreground, 0.05)));

        let n_items = self.items.len();
        let disc_r = radius * (1.0 / (self.tiles as f32).sqrt()) * 1.3;
        for (i, v) in &tiles {
            let z = v[2];
            let f = persp / (persp - z);
            let pos = center + vec2(v[0], -v[1]) * radius * f;
            let is_front = *i == front;
            let grow = if is_front { 1.0 + 0.35 * settled } else { 1.0 };
            let r = disc_r * f * grow * (1.0 - 0.25 * move_k);
            let facing = z.max(0.0);
            let alpha = (0.15 + 0.85 * ((z + 0.35) / 1.0).clamp(0.0, 1.0)).clamp(0.0, 1.0);
            if z < -0.2 {
                // back side: faint dots only
                painter.circle_filled(pos, r * 0.25, with_alpha(t.muted_foreground, 0.12 * (1.0 + z)));
                continue;
            }
            let face = &self.items[i % n_items];
            let [c0, c1] = face.colors.unwrap_or_else(|| {
                let a = t.chart[(i % n_items) % t.chart.len()];
                [a, mix(a, t.chart[(i % n_items + 1) % t.chart.len()], 0.6)]
            });
            // ellipse: squashed along the radial direction by how much the disc faces us
            let dir = vec2(v[0], -v[1]);
            let dir = if dir.length() > 1e-4 { dir.normalized() } else { vec2(1.0, 0.0) };
            let perp = dir.rot90();
            let squash = 0.25 + 0.75 * facing.powf(0.7);
            let outline: Vec<Pos2> = (0..48)
                .map(|k| {
                    let a = k as f32 / 48.0 * std::f32::consts::TAU;
                    pos + dir * a.cos() * r * squash + perp * a.sin() * r
                })
                .collect();
            let bg = t.background;
            let dim = 1.0 - alpha;
            paint::fill_convex_fn(&painter, &outline, 4, |q| {
                let d = (q - pos) / r.max(1.0);
                let u = ((d.x + d.y) * 0.35 + 0.5).clamp(0.0, 1.0);
                let col = gradient(&[shade(c0, 0.12), c1], u);
                let hl = ((-d.y * 0.5 + 0.2).max(0.0)).powi(2) * 0.35;
                mix(mix(col, Color32::WHITE, hl), bg, dim * 0.85)
            });
            let mut rim = outline.clone();
            rim.push(rim[0]);
            painter.add(egui::Shape::line(rim, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.25 * alpha))));
            if !face.icon.is_empty() && facing > 0.2 {
                painter.text(
                    pos,
                    Align2::CENTER_CENTER,
                    &face.icon,
                    FontId::proportional((r * 0.8 * squash.max(0.6)).max(1.0)),
                    with_alpha(Color32::WHITE, alpha * ((facing - 0.2) / 0.4).clamp(0.0, 1.0)),
                );
            }
            if is_front && settled > 0.01 {
                painter.circle_stroke(pos, r + 4.0, Stroke::new(2.0, with_alpha(t.foreground, 0.5 * settled)));
            }
        }

        // title, description, action button
        let face = &self.items[front % n_items];
        let a = settled;
        let slide = (1.0 - a) * 16.0;
        let left_w = (center.x - radius * 1.25 - rect.min.x - 32.0).max(80.0);
        let title_font = FontId::proportional((left_w * 0.16).clamp(18.0, 40.0));
        bold_text(
            &painter,
            pos2(rect.min.x + 32.0 - slide, center.y),
            Align2::LEFT_CENTER,
            &face.title,
            title_font,
            with_alpha(t.foreground, a),
        );
        if !face.subtitle.is_empty() {
            let g = painter.layout(
                face.subtitle.clone(),
                FontId::proportional(15.0),
                with_alpha(t.muted_foreground, a),
                left_w.min(220.0),
            );
            let x = rect.max.x - 32.0 - g.size().x + slide;
            painter.galley(pos2(x, center.y - g.size().y / 2.0), g, Color32::TRANSPARENT);
        }
        let mut clicked = None;
        let btn_c = pos2(center.x, rect.max.y - 28.0);
        let bs = spring(ui, id.with("btn"), if settled > 0.6 { 1.0 } else { 0.0 }, 280.0, 16.0).max(0.0);
        if bs > 0.02 {
            let br = ui.interact(
                egui::Rect::from_center_size(btn_c, Vec2::splat(52.0)),
                id.with("action"),
                Sense::click(),
            );
            let br = br.on_hover_cursor(CursorIcon::PointingHand);
            if br.clicked() {
                clicked = Some(front % n_items);
            }
            let hv = follow(ui, id.with("bh"), if br.hovered() { 1.0 } else { 0.0 }, 14.0);
            let r = 24.0 * bs * (1.0 + 0.08 * hv);
            paint::soft_shadow(
                &painter,
                egui::Rect::from_center_size(btn_c, Vec2::splat(r * 2.0)),
                r,
                16.0,
                vec2(0.0, 4.0),
                with_alpha(t.accent, 0.4 * bs.min(1.0)),
            );
            painter.circle_filled(btn_c, r, t.accent);
            let s = 6.5 * bs;
            let st = Stroke::new(2.2, t.accent_foreground);
            painter.line_segment([btn_c + vec2(-s, s), btn_c + vec2(s, -s)], st);
            painter.line_segment([btn_c + vec2(-s * 0.3, -s), btn_c + vec2(s, -s)], st);
            painter.line_segment([btn_c + vec2(s, -s), btn_c + vec2(s, s * 0.3)], st);
        }
        InnerResponse::new(clicked, resp)
    }
}
