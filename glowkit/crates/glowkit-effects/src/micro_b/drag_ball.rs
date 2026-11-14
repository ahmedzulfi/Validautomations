use super::common::{ghost_cursor, idle_for, ink, shadow_color, surface};
use egui::{vec2, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};
use glowkit_core::anim::{smoothstep, Spring};
use glowkit_core::color::{shade, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug)]
struct BallState {
    pos: Vec2,
    vel: Vec2,
    grab: Option<Vec2>,
    squish: Spring,
    held: f32,
    ghost: f32,
}

/// A squishy ball you grab. It swells under the pointer, squeezes and pulses while you hold it,
/// stretches along the direction you drag it, and when you let go it flies back home on a loose
/// spring, wobbling like jelly until it settles.
///
/// While idle a ghost hand picks it up and swings it around.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::DragBall::new().size(56.0).stretch(0.36));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DragBall {
    size: f32,
    height: f32,
    stretch: f32,
    give: f32,
    grip: f32,
    dark: bool,
    autoplay: bool,
}

impl Default for DragBall {
    fn default() -> Self {
        Self::new()
    }
}

impl DragBall {
    /// A draggable ball.
    pub fn new() -> Self {
        Self { size: 56.0, height: 320.0, stretch: 0.36, give: 0.5, grip: 0.5, dark: false, autoplay: true }
    }
    /// Diameter in points.
    pub fn size(mut self, d: f32) -> Self {
        self.size = d.clamp(16.0, 200.0);
        self
    }
    /// Stage height (width is all available).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(80.0);
        self
    }
    /// How much it stretches along its motion (0..1).
    pub fn stretch(mut self, k: f32) -> Self {
        self.stretch = k.clamp(0.0, 1.0);
        self
    }
    /// How loose and wobbly the return is (0..1).
    pub fn give(mut self, k: f32) -> Self {
        self.give = k.clamp(0.0, 1.0);
        self
    }
    /// How tightly it sticks to the pointer while dragged (0..1).
    pub fn grip(mut self, k: f32) -> Self {
        self.grip = k.clamp(0.0, 1.0);
        self
    }
    /// Ink ball instead of the light surface.
    pub fn dark(mut self, on: bool) -> Self {
        self.dark = on;
        self
    }
    /// Let a ghost hand play with it while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for DragBall {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (stage, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let id = response.id;
        let home = stage.center();
        let r = self.size / 2.0;
        let st0: BallState = glowkit_core::state(ui, id, || BallState {
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
            grab: None,
            squish: Spring::new(0.0),
            held: 0.0,
            ghost: -1.0,
        });
        let hit = Rect::from_center_size(home + st0.pos, Vec2::splat(self.size * 1.3));
        let b = ui.interact(hit, id.with("ball"), Sense::drag());
        if b.hovered() || b.dragged() {
            ui.ctx().set_cursor_icon(if b.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        let dt = glowkit_core::dt(ui);
        let time = glowkit_core::time(ui);
        let idle = idle_for(ui, id, b.hovered() || b.dragged());
        let ptr = b.interact_pointer_pos();
        let (pressed, dragging, released) = (
            b.drag_started() || (b.is_pointer_button_down_on() && st0.grab.is_none()),
            b.dragged() || b.is_pointer_button_down_on(),
            b.drag_stopped(),
        );
        let auto = self.autoplay && idle > 2.0;
        let grip = 140.0 + 700.0 * self.grip;
        let give = self.give;
        let half = stage.size() / 2.0 - Vec2::splat(r);
        let st = glowkit_core::with_state(
            ui,
            id,
            || st0,
            |s| {
                let mut target: Option<Pos2> = None;
                if pressed {
                    if let Some(p) = ptr {
                        s.grab = Some(p - (home + s.pos));
                        s.ghost = -1.0;
                        s.squish.velocity -= 4.0;
                    }
                }
                if let (Some(g), true, Some(p)) = (s.grab, dragging, ptr) {
                    target = Some(p - g);
                }
                if released || (!dragging && s.grab.is_some()) {
                    s.grab = None;
                    s.squish.velocity += 6.0;
                }
                // ghost: grab, swing in a loop, let go
                if auto && s.grab.is_none() && s.ghost < 0.0 && s.vel.length() < 5.0 {
                    s.ghost = 0.0;
                }
                if s.ghost >= 0.0 {
                    s.ghost += dt;
                    let g = s.ghost;
                    if (0.6..2.2).contains(&g) {
                        let k = smoothstep(0.6, 1.0, g);
                        let a = (g - 0.6) * 4.2;
                        let orbit = vec2(a.cos() - 1.0, a.sin() * 0.6) * 70.0 * k;
                        target = Some(home + orbit);
                        if g < 0.7 {
                            s.squish.velocity -= 30.0 * dt;
                        }
                    } else if (2.2..2.25).contains(&g) {
                        s.squish.velocity += 6.0 * dt * 20.0;
                    }
                    if g > 3.4 {
                        s.ghost = -1.0;
                    }
                }
                let holding = target.is_some();
                s.held = if holding { s.held + dt } else { 0.0 };
                let (k, damp) = if holding {
                    (grip, 2.0 * grip.sqrt() * 0.9)
                } else {
                    // looser and wobblier with more give
                    let k = 160.0 - 60.0 * give;
                    (k, 2.0 * k.sqrt() * (0.55 - 0.4 * give))
                };
                let goal = target.map_or(Vec2::ZERO, |p| p - home);
                let steps = 4;
                let h = dt / steps as f32;
                for _ in 0..steps {
                    let acc = (goal - s.pos) * k - s.vel * damp;
                    s.vel += acc * h;
                    s.pos += s.vel * h;
                }
                s.pos = vec2(s.pos.x.clamp(-half.x, half.x), s.pos.y.clamp(-half.y, half.y));
                // squeeze while held (a slow pulse), jelly wobble after
                s.squish.target = if holding { 0.08 + 0.03 * (s.held * 7.0).sin() } else { 0.0 };
                s.squish.stiffness = 260.0;
                s.squish.damping = 7.0 + 10.0 * (1.0 - give);
                s.squish.step(dt);
                *s
            },
        );
        let moving = st.vel.length() > 0.5
            || st.pos.length() > 0.5
            || !st.squish.is_settled()
            || st.grab.is_some()
            || st.ghost >= 0.0;
        if moving || auto {
            glowkit_core::animate(ui);
        }
        let swell = glowkit_core::anim::spring(
            ui,
            id.with("swell"),
            if b.hovered() || st.grab.is_some() || (0.6..2.2).contains(&st.ghost) { 1.0 } else { 0.0 },
            300.0,
            16.0,
        );
        if !ui.is_rect_visible(stage) {
            return response;
        }
        let painter = ui.painter_at(stage);
        let c = home + st.pos;
        let rad = r * (1.0 + 0.12 * swell);
        // deformation: stretch along velocity, squash (pinch) from the squish spring
        let speed = st.vel.length();
        let axis = if speed > 1.0 { st.vel / speed } else { Vec2::X };
        let s = (speed / 1400.0).min(1.0) * self.stretch * 1.4;
        let along = (1.0 + s) * (1.0 + st.squish.value);
        let across = 1.0 / (1.0 + s) * (1.0 - st.squish.value * 0.9);
        let wob = (time * 22.0).sin() * st.squish.velocity.abs().min(4.0) * 0.004;
        let n = 56;
        let pts: Vec<Pos2> = (0..n)
            .map(|i| {
                let a = i as f32 / n as f32 * TAU;
                let v = Vec2::angled(a) * rad * (1.0 + wob * (3.0 * a).cos());
                let x = v.dot(axis) * along;
                let y = v.dot(axis.rot90()) * across;
                c + axis * x + axis.rot90() * y
            })
            .collect();
        let (bg, _) = if self.dark { ink(&t) } else { (surface(&t), t.foreground) };
        let lift = 0.6 + 0.4 * swell;
        paint::soft_shadow(
            &painter,
            Rect::from_center_size(c, vec2(rad * 2.0 * along.max(across), rad * 2.0 * across.min(along))),
            rad,
            10.0 + 10.0 * swell,
            vec2(0.0, 4.0 + 6.0 * swell),
            shadow_color(&t, lift),
        );
        let lit = shade(bg, if t.dark { 0.05 } else { 0.0 });
        let dim = shade(bg, -0.06);
        paint::fill_convex_fn(
            &painter,
            &pts,
            6,
            paint::radial_fn(c - vec2(rad * 0.35, rad * 0.45), rad * 1.9, &[lit, dim]),
        );
        let mut rim = pts.clone();
        rim.push(pts[0]);
        painter.add(egui::Shape::line(
            rim,
            Stroke::new(1.0, if t.dark { with_alpha(egui::Color32::WHITE, 0.06) } else { with_alpha(t.border, 0.6) }),
        ));
        if st.ghost >= 0.0 {
            let g = st.ghost;
            let a = smoothstep(0.1, 0.5, g) * (1.0 - smoothstep(2.15, 2.35, g));
            ghost_cursor(&painter, &t, c + vec2(4.0, 6.0), a, (0.6..2.2).contains(&g));
        }
        response.union(b)
    }
}
