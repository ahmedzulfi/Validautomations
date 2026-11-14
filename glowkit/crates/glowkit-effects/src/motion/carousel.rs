use super::common::{theme_card, CardFace};
use egui::emath::TSTransform;
use egui::epaint::Mesh;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui,
    UiBuilder, Vec2,
};
use glowkit_core::anim::Spring;
use glowkit_core::color::{gradient, mix, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::TAU;

/// Result of [`Carousel::show`].
pub struct CarouselResponse {
    /// Response of the whole carousel viewport.
    pub response: Response,
    /// Index of the item currently snapped to (or being snapped to).
    pub index: usize,
}

#[derive(Clone, Copy)]
struct CarouselState {
    pos: Spring,
    drag_start: f32,
    dragging: bool,
    last_auto: f32,
}

/// A draggable carousel: swipe or drag to move between items, which snap into place on a
/// spring; neighbors peek in at the sides and dots show the position.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let r = glowkit_effects::Carousel::new(5).autoplay(3.0).show(ui, |ui, i| {
///     ui.heading(format!("Slide {i}"));
/// });
/// ```
#[must_use = "call `.show(ui, |ui, index| ...)`"]
pub struct Carousel {
    count: usize,
    item_size: Vec2,
    gap: f32,
    looping: bool,
    autoplay: Option<f32>,
    pause_on_hover: bool,
    dots: bool,
    card: bool,
    salt: u64,
}

impl Carousel {
    /// A carousel of `count` items.
    pub fn new(count: usize) -> Self {
        Self {
            count,
            item_size: vec2(320.0, 220.0),
            gap: 20.0,
            looping: false,
            autoplay: None,
            pause_on_hover: true,
            dots: true,
            card: true,
            salt: 0,
        }
    }
    /// Size of each item.
    pub fn item_size(mut self, s: Vec2) -> Self {
        self.item_size = s;
        self
    }
    /// Gap between items.
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g.max(0.0);
        self
    }
    /// Wrap around from the last item to the first.
    pub fn looping(mut self, on: bool) -> Self {
        self.looping = on;
        self
    }
    /// Advance automatically every `seconds`.
    pub fn autoplay(mut self, seconds: f32) -> Self {
        self.autoplay = Some(seconds.max(0.5));
        self
    }
    /// Pause autoplay while hovered (default on).
    pub fn pause_on_hover(mut self, on: bool) -> Self {
        self.pause_on_hover = on;
        self
    }
    /// Show position dots under the items (default on).
    pub fn dots(mut self, on: bool) -> Self {
        self.dots = on;
        self
    }
    /// Paint a theme card behind each item (default on).
    pub fn card(mut self, on: bool) -> Self {
        self.card = on;
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }

    /// Show the carousel; `item(ui, index)` fills one item.
    pub fn show(self, ui: &mut Ui, mut item: impl FnMut(&mut Ui, usize)) -> CarouselResponse {
        let n = self.count;
        let dots_h = if self.dots { 34.0 } else { 0.0 };
        let (rect, response) = ui
            .allocate_exact_size(vec2(ui.available_width(), self.item_size.y + 24.0 + dots_h), Sense::click_and_drag());
        let id = response.id.with(self.salt);
        if n == 0 {
            return CarouselResponse { response, index: 0 };
        }
        let step = self.item_size.x + self.gap;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let hovered = response.hovered();
        let nf = n as f32;
        let looping = self.looping;
        let wrap = |x: f32| if looping { x.rem_euclid(nf) } else { x };
        let mut st = glowkit_core::state(ui, id.with("st"), || CarouselState {
            pos: Spring::new(0.0).with(240.0, 26.0),
            drag_start: 0.0,
            dragging: false,
            last_auto: now,
        });
        if response.drag_started() {
            st.dragging = true;
            st.drag_start = st.pos.value;
        }
        if st.dragging {
            let total = response.total_drag_delta().map(|d| d.x).unwrap_or(0.0);
            let mut p = st.drag_start - total / step;
            if !looping {
                // rubber band past the ends
                if p < 0.0 {
                    p = -(-p).powf(0.6) * 0.35;
                } else if p > nf - 1.0 {
                    p = nf - 1.0 + (p - nf + 1.0).powf(0.6) * 0.35;
                }
            }
            st.pos.value = p;
            st.pos.velocity = 0.0;
            if response.drag_stopped() || !response.dragged() {
                st.dragging = false;
                let v = ui.input(|i| i.pointer.velocity().x) / step;
                let mut target = (st.pos.value - v * 0.18).round();
                target = target.clamp(st.drag_start.round() - 1.0, st.drag_start.round() + 1.0);
                if !looping {
                    target = target.clamp(0.0, nf - 1.0);
                }
                st.pos.target = target;
                st.pos.velocity = -v * 0.5;
                st.last_auto = now;
            }
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        } else {
            if let Some(every) = self.autoplay {
                if self.pause_on_hover && hovered {
                    st.last_auto = now;
                } else if now - st.last_auto >= every {
                    st.last_auto = now;
                    let next = st.pos.target + 1.0;
                    st.pos.target = if looping {
                        next
                    } else if next > nf - 1.0 {
                        0.0
                    } else {
                        next
                    };
                }
                glowkit_core::animate(ui);
            }
            st.pos.step(dt);
            if hovered {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
        }
        // keep the spring bounded when looping
        if looping && st.pos.is_settled() && (st.pos.target < 0.0 || st.pos.target >= nf) {
            let w = st.pos.target.rem_euclid(nf);
            st.pos.value += w - st.pos.target;
            st.pos.target = w;
        }
        if !st.pos.is_settled() {
            ui.ctx().request_repaint();
        }
        let pos = st.pos.value;
        let index = (wrap(st.pos.target).round() as usize).min(n - 1);

        let viewport = Rect::from_min_size(rect.min, vec2(rect.width(), self.item_size.y + 24.0));
        let clip = viewport.intersect(ui.clip_rect());
        let c = viewport.center();
        let t = Theme::of(ui);
        let reach = (viewport.width() / 2.0 / step).ceil() + 1.0;
        for i in 0..n {
            let mut rel = i as f32 - pos;
            if looping {
                rel = (rel + nf / 2.0).rem_euclid(nf) - nf / 2.0;
            }
            if rel.abs() > reach {
                continue;
            }
            let center = c + vec2(rel * step, 0.0);
            let k = rel.abs().min(1.0);
            let scale = 1.0 - 0.1 * k;
            let alpha = 1.0 - 0.45 * k;
            let r = Rect::from_center_size(center, self.item_size);
            let tf = TSTransform::from_translation(center.to_vec2())
                * TSTransform::from_scaling(scale)
                * TSTransform::from_translation(-center.to_vec2());
            ui.with_visual_transform(tf, |ui| {
                let mut child = ui.new_child(UiBuilder::new().max_rect(r.shrink(20.0)).id_salt(("carousel_item", i)));
                child.set_clip_rect(clip);
                child.multiply_opacity(alpha);
                if self.card {
                    theme_card(&child.painter().with_clip_rect(clip), &t, r, (t.radius * 1.6).round(), 0.6);
                }
                item(&mut child, i);
            });
        }
        if self.dots {
            let y = rect.max.y - dots_h / 2.0;
            let gap = 8.0;
            let widths: Vec<f32> = (0..n)
                .map(|i| {
                    glowkit_core::anim::spring(
                        ui,
                        id.with(("dot", i)),
                        if i == index { 22.0 } else { 8.0 },
                        300.0,
                        24.0,
                    )
                })
                .collect();
            let total: f32 = widths.iter().sum::<f32>() + gap * (nf - 1.0);
            let mut x = c.x - total / 2.0;
            let painter = ui.painter();
            for (i, w) in widths.iter().enumerate() {
                let d = Rect::from_min_size(pos2(x, y - 4.0), vec2(*w, 8.0));
                let on = ((w - 8.0) / 14.0).clamp(0.0, 1.0);
                painter.rect_filled(
                    d,
                    CornerRadius::same(4),
                    mix(with_alpha(t.muted_foreground, 0.35), t.foreground, on),
                );
                let hit = ui.interact(d.expand(4.0), id.with(("dot_hit", i)), Sense::click());
                if hit.clicked() {
                    let mut target = i as f32;
                    if looping {
                        let cur = st.pos.target;
                        let delta = (target - cur.rem_euclid(nf) + nf / 2.0).rem_euclid(nf) - nf / 2.0;
                        target = cur + delta;
                    }
                    st.pos.target = target;
                    st.last_auto = now;
                    ui.ctx().request_repaint();
                }
                if hit.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                x += w + gap;
            }
        }
        glowkit_core::set_state(ui, id.with("st"), st);
        CarouselResponse { response, index }
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct RingState {
    angle: f32,
    vel: f32,
}

/// Cards arranged on a rotating 3D ring with real perspective: drag to spin it, release to
/// let it coast, and it drifts on its own when idle.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{CardFace, RingCarousel};
/// let faces = [CardFace::new("🌊", "Ocean"), CardFace::new("🌅", "Sunset"), CardFace::new("🌲", "Forest")];
/// ui.add(RingCarousel::new(&faces).speed(12.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct RingCarousel<'a> {
    faces: &'a [CardFace],
    card_size: Vec2,
    radius: Option<f32>,
    speed: f32,
    perspective: f32,
    tilt: f32,
    height: Option<f32>,
}

impl<'a> RingCarousel<'a> {
    /// A ring of `faces`.
    pub fn new(faces: &'a [CardFace]) -> Self {
        Self {
            faces,
            card_size: vec2(150.0, 200.0),
            radius: None,
            speed: 14.0,
            perspective: 3.0,
            tilt: 9.0,
            height: None,
        }
    }
    /// Card size in points.
    pub fn card_size(mut self, s: Vec2) -> Self {
        self.card_size = s;
        self
    }
    /// Ring radius (defaults to fit the cards side by side).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = Some(r.max(10.0));
        self
    }
    /// Idle rotation speed in degrees per second (negative = other way).
    pub fn speed(mut self, deg: f32) -> Self {
        self.speed = deg;
        self
    }
    /// Camera distance as a multiple of the radius (smaller = stronger perspective).
    pub fn perspective(mut self, p: f32) -> Self {
        self.perspective = p.max(1.3);
        self
    }
    /// Camera tilt in degrees (looking down onto the ring).
    pub fn tilt(mut self, deg: f32) -> Self {
        self.tilt = deg.clamp(-45.0, 45.0);
        self
    }
    /// Height of the widget.
    pub fn height(mut self, h: f32) -> Self {
        self.height = Some(h);
        self
    }
}

impl egui::Widget for RingCarousel<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let n = self.faces.len();
        let h = self.height.unwrap_or(self.card_size.y * 1.3 + 80.0);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::drag());
        let id = response.id;
        if n == 0 {
            return response;
        }
        let dt = glowkit_core::dt(ui);
        let auto = self.speed.to_radians();
        let mut st = glowkit_core::state(ui, id.with("ring"), || RingState { angle: 0.0, vel: auto });
        if response.dragged() {
            let dx = response.drag_delta().x;
            let rad = dx * 0.006;
            st.angle += rad;
            st.vel = if dt > 0.0 { rad / dt } else { 0.0 };
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        } else {
            st.vel += (auto - st.vel) * (1.0 - (-1.6 * dt).exp());
            st.angle += st.vel * dt;
            if response.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
        }
        st.angle = st.angle.rem_euclid(TAU);
        glowkit_core::set_state(ui, id.with("ring"), st);
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let t = Theme::of(ui);
        let radius =
            self.radius.unwrap_or_else(|| (self.card_size.x * 1.3 * n as f32 / TAU).max(self.card_size.x * 0.9));
        let f = radius * self.perspective;
        let c = rect.center() - vec2(0.0, 10.0);
        let (tilt_s, tilt_c) = self.tilt.to_radians().sin_cos();
        let project = |p: [f32; 3]| -> (Pos2, f32) {
            // look slightly down onto the ring, then perspective-divide (front card at scale 1)
            let y = p[1] * tilt_c + p[2] * tilt_s;
            let z = p[2] * tilt_c - p[1] * tilt_s;
            let s = (f - radius) / (f - z);
            (c + vec2(p[0], y) * s, s)
        };
        let painter = ui.painter_at(rect);
        // floor shadow
        paint::glow_ellipse(
            &painter,
            c + vec2(0.0, self.card_size.y * 0.62),
            vec2(radius * 1.3, 26.0),
            with_alpha(Color32::BLACK, if t.dark { 0.55 } else { 0.12 }),
        );
        let outline_local = paint::rounded_rect_points(
            Rect::from_center_size(Pos2::ZERO, self.card_size),
            (t.radius * 1.4).min(20.0),
            5,
        );
        let mut order: Vec<(f32, usize, f32)> = (0..n)
            .map(|i| {
                let th = st.angle + i as f32 / n as f32 * TAU;
                (radius * th.cos(), i, th)
            })
            .collect();
        order.sort_by(|a, b| a.0.total_cmp(&b.0));
        let half = self.card_size / 2.0;
        for (z, i, th) in order {
            let (sn, cs) = th.sin_cos();
            let world = |l: Pos2| [radius * sn + l.x * cs, l.y, radius * cs - l.x * sn];
            let facing = cs; // >0 faces the viewer
            let depth = (z / radius + 1.0) / 2.0; // 0 back .. 1 front
            let face = &self.faces[i];
            let [c0, c1] = face.resolved_colors(&t, i);
            let bg = if t.dark { t.background } else { t.muted };
            let fog = 0.55 * (1.0 - depth);
            let (center, s) = project(world(Pos2::ZERO));
            let mut mesh = Mesh::default();
            let col_at = |l: Pos2| {
                let u = ((l.x / half.x) * 0.5 + (l.y / half.y) * 0.5) * 0.5 + 0.5;
                let col = if facing >= 0.0 { gradient(&[c0, c1], u) } else { mix(c1, bg, 0.35) };
                mix(col, bg, fog)
            };
            mesh.colored_vertex(center, col_at(Pos2::ZERO));
            for l in &outline_local {
                let (p, _) = project(world(*l));
                mesh.colored_vertex(p, col_at(*l));
            }
            let m = outline_local.len() as u32;
            for k in 0..m {
                mesh.add_triangle(0, 1 + k, 1 + (k + 1) % m);
            }
            painter.add(Shape::mesh(mesh));
            let rim: Vec<Pos2> = outline_local.iter().map(|l| project(world(*l)).0).collect();
            painter
                .add(Shape::closed_line(rim, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.18 * (0.3 + 0.7 * depth)))));
            if facing > 0.55 {
                let a = ((facing - 0.55) / 0.3).min(1.0) * (1.0 - fog);
                let (ip, _) = project(world(pos2(0.0, -half.y * 0.15)));
                painter.text(
                    ip,
                    Align2::CENTER_CENTER,
                    &face.icon,
                    FontId::proportional(self.card_size.y * 0.24 * s),
                    with_alpha(Color32::WHITE, a),
                );
                let (tp, _) = project(world(pos2(0.0, half.y * 0.5)));
                painter.text(
                    tp,
                    Align2::CENTER_CENTER,
                    &face.title,
                    FontId::proportional((self.card_size.y * 0.085 * s).max(1.0)),
                    with_alpha(Color32::WHITE, a),
                );
            }
        }
        response
    }
}
