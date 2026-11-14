use super::common::{idle_time, wall_dt};
use egui::epaint::{Mesh, Vertex};
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, TextureId, Ui, Vec2, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::{hex, Theme};
use glowkit_effects::PaintedScene;

/// How pictures are fixed to a [`ClothesLine`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineClip {
    /// Wooden clothes pegs.
    #[default]
    Peg,
    /// Black binder clips.
    Binder,
    /// A strip of masking tape over the rope.
    Tape,
}

#[derive(Clone, Debug, Default)]
struct LineState {
    offset: f32,
    vel: f32,
    angle: Vec<f32>,
    omega: Vec<f32>,
    drive_t: f32,
    moving: bool,
    hover: Vec<f32>,
}

/// Pictures pegged to a sagging rope that drifts past. Every card is a damped pendulum hanging
/// from its peg, so when the line speeds up or stops the cards swing and settle, and the card
/// itself bends a little like heavy paper as it swings. Drag the line to pull it along (with
/// inertia), hover a card to lift it and read its caption.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{ClothesLine, LineClip};
/// ui.add(ClothesLine::new().clip(LineClip::Binder).speed(50.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ClothesLine {
    pictures: Vec<(TextureId, String, String)>,
    clip: LineClip,
    size: Option<Vec2>,
    card: Vec2,
    gap: f32,
    sag: f32,
    speed: f32,
    swing: f32,
}

impl Default for ClothesLine {
    fn default() -> Self {
        Self::new()
    }
}

impl ClothesLine {
    /// A line of the built-in painted pictures.
    pub fn new() -> Self {
        Self {
            pictures: Vec::new(),
            clip: LineClip::Peg,
            size: None,
            card: vec2(132.0, 168.0),
            gap: 22.0,
            sag: 26.0,
            speed: 46.0,
            swing: 1.0,
        }
    }
    /// Add a picture with a caption and a small subtitle.
    pub fn picture(mut self, tex: TextureId, title: impl Into<String>, subtitle: impl Into<String>) -> Self {
        self.pictures.push((tex, title.into(), subtitle.into()));
        self
    }
    /// Clip style (default pegs).
    pub fn clip(mut self, c: LineClip) -> Self {
        self.clip = c;
        self
    }
    /// Widget size (default: available width x 270).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Card size in points (default 132 x 168).
    pub fn card_size(mut self, s: Vec2) -> Self {
        self.card = s.max(vec2(40.0, 40.0));
        self
    }
    /// How deep the rope sags across the widget (default 26).
    pub fn sag(mut self, px: f32) -> Self {
        self.sag = px.clamp(0.0, 120.0);
        self
    }
    /// Drift speed in points per second (default 46; 0 stays still until dragged).
    pub fn speed(mut self, px: f32) -> Self {
        self.speed = px.clamp(0.0, 400.0);
        self
    }
    /// How freely the cards swing (default 1).
    pub fn swing(mut self, k: f32) -> Self {
        self.swing = k.clamp(0.0, 3.0);
        self
    }
}

fn card_mesh(tex: TextureId, corners: impl Fn(f32, f32) -> Pos2, tint: impl Fn(f32, f32) -> Color32, uv: Rect) -> Mesh {
    let (cols, rows) = (4, 6);
    let mut m = Mesh::with_texture(tex);
    for j in 0..=rows {
        for i in 0..=cols {
            let (u, v) = (i as f32 / cols as f32, j as f32 / rows as f32);
            m.vertices.push(Vertex {
                pos: corners(u, v),
                uv: pos2(uv.min.x + u * uv.width(), uv.min.y + v * uv.height()),
                color: tint(u, v),
            });
        }
    }
    let w = cols + 1;
    for j in 0..rows {
        for i in 0..cols {
            let a = (j * w + i) as u32;
            m.add_triangle(a, a + 1, a + w as u32 + 1);
            m.add_triangle(a, a + w as u32 + 1, a + w as u32);
        }
    }
    m
}

impl Widget for ClothesLine {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = glowkit_core::desired_size(ui, self.size, 270.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let wdt = wall_dt(ui);
        let idle = idle_time(ui, id, rect);
        let pics: Vec<(TextureId, String, String)> = if self.pictures.is_empty() {
            let subs = ["Late June", "Market haul", "Shot on film", "Saturday", "Day off", "After rain", "Early swim", "Picnic"];
            (0..PaintedScene::COUNT)
                .map(|i| (PaintedScene::texture(ui.ctx(), i), PaintedScene::name(i).to_string(), subs[i].to_string()))
                .collect()
        } else {
            self.pictures.clone()
        };
        let n = pics.len().max(1);
        let pitch = self.card.x + self.gap;
        let loop_len = pitch * n as f32;
        let drag = if response.dragged() { response.drag_delta().x } else { 0.0 };
        let dragging = response.dragged();
        if response.hovered() {
            ui.ctx().set_cursor_icon(if dragging { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        let pointer = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p));
        let rope_y0 = rect.min.y + 34.0;
        let sag = self.sag;
        let rope_y = |x: f32| {
            let u = ((x - rect.min.x) / rect.width()).clamp(0.0, 1.0) * 2.0 - 1.0;
            rope_y0 + sag * (1.0 - u * u)
        };
        let speed = self.speed;
        let swing = self.swing;
        let card = self.card;
        // which card is hovered (by current geometry)
        let st0: LineState = glowkit_core::state(ui, id, LineState::default);
        let peg_x = |k: i32, off: f32| rect.min.x + k as f32 * pitch - off.rem_euclid(pitch) + pitch * 0.5;
        let first = -1;
        let last = (rect.width() / pitch).ceil() as i32 + 1;
        let base_k = (st0.offset / pitch).floor() as i32;
        let mut hovered: Option<usize> = None;
        if let Some(p) = pointer {
            for k in first..=last {
                let x = peg_x(k, st0.offset);
                let r = Rect::from_min_size(pos2(x - card.x * 0.5, rope_y(x) + 6.0), card);
                if r.contains(p) {
                    hovered = Some((k + base_k).rem_euclid(n as i32) as usize);
                }
            }
        }
        let st = glowkit_core::with_state(ui, id, LineState::default, |s| {
            s.angle.resize(n, 0.0);
            s.omega.resize(n, 0.0);
            s.hover.resize(n, 0.0);
            let prev_vel = s.vel;
            if dragging {
                s.vel = -drag / dt.max(1e-3);
                s.offset -= drag;
            } else {
                // drift with pauses so the pendulums have something to react to
                s.drive_t += wdt;
                if s.drive_t > if s.moving { 5.0 } else { 1.8 } {
                    s.drive_t = 0.0;
                    s.moving = !s.moving;
                }
                let target = if s.moving && idle > 0.6 { speed } else { 0.0 };
                let k = if target > s.vel.abs() { 1.6 } else { 3.2 };
                s.vel += (target - s.vel) * (1.0 - (-k * dt).exp());
                s.offset += s.vel * dt;
            }
            let acc = ((s.vel - prev_vel) / dt.max(1e-3)).clamp(-3000.0, 3000.0);
            for i in 0..n {
                // pendulum: theta'' = -(g/L) sin(theta) - c theta' - a cos(theta) / L
                let len = card.y * (0.55 + 0.1 * glowkit_core::rng::hash01(i as u64 * 7));
                let g = 900.0;
                let c = 1.6 + 0.6 * glowkit_core::rng::hash01(i as u64 * 3 + 1);
                let alpha = -(g / len) * s.angle[i].sin() - c * s.omega[i] - swing * acc * s.angle[i].cos() / len * 0.9;
                s.omega[i] += alpha * dt;
                s.angle[i] = (s.angle[i] + s.omega[i] * dt).clamp(-0.9, 0.9);
                let goal = if hovered == Some(i) { 1.0 } else { 0.0 };
                s.hover[i] += (goal - s.hover[i]) * (1.0 - (-10.0 * dt).exp());
            }
            if s.offset.abs() > loop_len * 8.0 {
                s.offset = s.offset.rem_euclid(loop_len);
            }
            s.clone()
        });
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let base_k = (st.offset / pitch).floor() as i32;
        // rope (behind the cards' clips): slight dip at every peg
        let pegs: Vec<f32> = (first..=last).map(|k| peg_x(k, st.offset)).collect();
        let rope: Vec<Pos2> = (0..=120)
            .map(|i| {
                let x = rect.min.x + i as f32 / 120.0 * rect.width();
                let local = pegs
                    .windows(2)
                    .find(|w| x >= w[0] && x <= w[1])
                    .map(|w| {
                        let u = (x - w[0]) / (w[1] - w[0]);
                        -4.0 * u * (1.0 - u) * 3.0
                    })
                    .unwrap_or(0.0);
                pos2(x, rope_y(x) + local + 3.0)
            })
            .collect();
        let rope_c = if t.dark { hex("#B89A72") } else { hex("#A88560") };
        painter.add(Shape::line(rope.clone(), Stroke::new(2.4, rope_c)));
        // twist
        for (i, w) in rope.windows(2).enumerate() {
            if i % 2 == 0 {
                painter.line_segment([w[0] + vec2(0.0, -0.8), w[1] + vec2(0.0, 0.8)], Stroke::new(0.8, with_alpha(Color32::BLACK, 0.18)));
            }
        }
        let paper_edge = if t.dark { hex("#2A2A2E") } else { Color32::WHITE };
        for k in first..=last {
            let i = (k + base_k).rem_euclid(n as i32) as usize;
            let px = peg_x(k, st.offset);
            let peg = pos2(px, rope_y(px) + 3.0);
            let th = st.angle[i] + (rope_y(px + 1.0) - rope_y(px - 1.0)) * 0.25;
            let lift = st.hover[i];
            let scale = 1.0 + 0.04 * lift;
            let bend = st.omega[i] * 10.0;
            let (sn, cs) = th.sin_cos();
            let w = card.x * scale;
            let h = card.y * scale;
            let pos_of = |u: f32, v: f32| {
                // card hangs from the middle of its top edge; the lower part trails like heavy paper
                let lx = (u - 0.5) * w - bend * v * v * 0.6;
                let ly = 4.0 + v * h;
                peg + vec2(lx * cs - ly * sn, lx * sn + ly * cs)
            };
            // shadow
            let sh_off = vec2(6.0, 10.0) + vec2(4.0, 6.0) * lift;
            let quad = [pos_of(0.0, 0.0), pos_of(1.0, 0.0), pos_of(1.0, 1.0), pos_of(0.0, 1.0)].map(|p| p + sh_off);
            super::common::soft_quad(&painter, quad, 14.0 + 6.0 * lift, Color32::from_black_alpha(if t.dark { 90 } else { 40 }));
            // paper border then the picture
            let border = card_mesh(TextureId::default(), pos_of, |_, _| paper_edge, Rect::from_min_max(egui::epaint::WHITE_UV, egui::epaint::WHITE_UV));
            painter.add(Shape::mesh(border));
            let inset = 5.0 / w;
            let inset_y = 5.0 / h;
            let tex_aspect = PaintedScene::ASPECT;
            let target = (w - 10.0) / (h - 10.0);
            let uw = (target / tex_aspect).min(1.0);
            let uv = Rect::from_min_size(pos2((1.0 - uw) * 0.5, 0.0), vec2(uw, 1.0));
            let shade_k = |u: f32, _v: f32| {
                let k = 1.0 - (bend * (u - 0.5)).abs().min(0.3) * 0.5;
                let c = (255.0 * k) as u8;
                Color32::from_rgb(c, c, c)
            };
            let pic = card_mesh(pics[i].0, |u, v| pos_of(inset + u * (1.0 - 2.0 * inset), inset_y + v * (1.0 - 2.0 * inset_y)), shade_k, uv);
            painter.add(Shape::mesh(pic));
            if lift > 0.05 {
                let wuv = Rect::from_min_max(egui::epaint::WHITE_UV, egui::epaint::WHITE_UV);
                let fade = card_mesh(
                    TextureId::default(),
                    |u, v| pos_of(inset + u * (1.0 - 2.0 * inset), 0.55 + v * (0.45 - inset_y)),
                    |_, v| Color32::from_black_alpha((v * v * 150.0 * lift) as u8),
                    wuv,
                );
                painter.add(Shape::mesh(fade));
                let a = pos_of(0.5, 0.86);
                painter.text(a, Align2::CENTER_CENTER, &pics[i].1, FontId::proportional(13.0), with_alpha(Color32::WHITE, lift));
                painter.text(a + vec2(0.0, 14.0), Align2::CENTER_CENTER, &pics[i].2, FontId::proportional(9.5), with_alpha(Color32::WHITE, 0.8 * lift));
            }
            // clip on top of the rope
            match self.clip {
                LineClip::Peg => {
                    let wood = hex("#C9A06B");
                    let pts = |dx: f32| {
                        let a = peg + vec2(dx - 3.0, -12.0);
                        let r = Rect::from_min_size(a, vec2(6.0, 26.0));
                        [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()].map(|p| peg + (p - peg).rot(th * 0.6)).to_vec()
                    };
                    painter.add(Shape::convex_polygon(pts(-2.0), glowkit_core::color::shade(wood, -0.06), Stroke::NONE));
                    painter.add(Shape::convex_polygon(pts(2.0), wood, Stroke::new(0.6, with_alpha(Color32::BLACK, 0.25))));
                    painter.line_segment([peg + vec2(-4.0, 0.0), peg + vec2(4.0, 0.0)], Stroke::new(1.6, hex("#8E9198")));
                }
                LineClip::Binder => {
                    let body = [peg + vec2(-9.0, -2.0), peg + vec2(9.0, -2.0), peg + vec2(11.0, 12.0), peg + vec2(-11.0, 12.0)];
                    painter.add(Shape::convex_polygon(body.to_vec(), hex("#1D1D20"), Stroke::NONE));
                    painter.add(Shape::line(vec![peg + vec2(-6.0, 0.0), peg + vec2(-4.0, -14.0), peg + vec2(4.0, -14.0), peg + vec2(6.0, 0.0)], Stroke::new(1.4, hex("#B9BCC2"))));
                }
                LineClip::Tape => {
                    let r = Rect::from_center_size(Pos2::ZERO, vec2(44.0, 16.0));
                    let rot = -0.12 + 0.24 * glowkit_core::rng::hash01(i as u64 + 2);
                    let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()].map(|p| peg + vec2(0.0, 4.0) + p.to_vec2().rot(rot + th)).to_vec();
                    painter.add(Shape::convex_polygon(pts, with_alpha(hex("#EFE4C8"), 0.82), Stroke::NONE));
                }
            }
        }
        response
    }
}

trait Rot {
    fn rot(self, a: f32) -> Self;
}

impl Rot for Vec2 {
    fn rot(self, a: f32) -> Vec2 {
        let (s, c) = a.sin_cos();
        vec2(self.x * c - self.y * s, self.x * s + self.y * c)
    }
}
