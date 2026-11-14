use super::common::{arrow, ghost_cursor, idle_for, ink, shadow_color};
use egui::{vec2, Pos2, Response, Sense, Stroke, Ui, Vec2, Widget};
use glowkit_core::anim::smoothstep;
use glowkit_core::color::with_alpha;
use glowkit_core::{paint, Theme};
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Rest,
    Pull,
    Fly,
    Home,
}

#[derive(Clone, Copy, Debug)]
struct SlingState {
    mode: Mode,
    t: f32,
    pos: Vec2,
    vel: Vec2,
    grab: Vec2,
    burst: f32,
    hit: f32,
    hit_dir: Vec2,
    ghost: f32,
    ghost_aim: Vec2,
}

/// A ball on a rubber band. Pull it back and the band stretches thinner, clicks into place and
/// trembles at its limit; let go and the ball flies off in the opposite direction, bounces off
/// the walls of the stage (squashing on impact), slows down and springs back home. Its arrow
/// always points where it is heading.
///
/// While idle it fires itself every few seconds.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::Slingshot::new().power(1.0).bounce(0.3));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Slingshot {
    height: f32,
    ball: f32,
    max_pull: f32,
    power: f32,
    bounce: f32,
    dark: bool,
    autoplay: bool,
}

impl Default for Slingshot {
    fn default() -> Self {
        Self::new()
    }
}

impl Slingshot {
    /// A slingshot stage.
    pub fn new() -> Self {
        Self { height: 340.0, ball: 24.0, max_pull: 110.0, power: 1.0, bounce: 0.3, dark: true, autoplay: true }
    }
    /// Stage height (width is all available).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(120.0);
        self
    }
    /// Ball radius.
    pub fn ball_radius(mut self, r: f32) -> Self {
        self.ball = r.clamp(6.0, 60.0);
        self
    }
    /// How far the band stretches before it locks.
    pub fn max_pull(mut self, px: f32) -> Self {
        self.max_pull = px.max(20.0);
        self
    }
    /// Launch power multiplier (1 = default).
    pub fn power(mut self, k: f32) -> Self {
        self.power = k.clamp(0.1, 3.0);
        self
    }
    /// Bounciness off the walls (0..1).
    pub fn bounce(mut self, k: f32) -> Self {
        self.bounce = k.clamp(0.0, 1.0);
        self
    }
    /// Ink ball (default) or a light one.
    pub fn dark(mut self, on: bool) -> Self {
        self.dark = on;
        self
    }
    /// Fire itself every few seconds while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for Slingshot {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (stage, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let id = response.id;
        let home = stage.center();
        let r = self.ball;
        let st0: SlingState = glowkit_core::state(ui, id, || SlingState {
            mode: Mode::Rest,
            t: 0.0,
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
            grab: Vec2::ZERO,
            burst: 9.0,
            hit: 9.0,
            hit_dir: Vec2::X,
            ghost: -1.0,
            ghost_aim: vec2(-1.0, 0.8),
        });
        let ball_rect = egui::Rect::from_center_size(home + st0.pos, Vec2::splat(r * 2.6));
        let b_resp = ui.interact(ball_rect, id.with("ball"), Sense::drag());
        if b_resp.hovered() || b_resp.dragged() {
            ui.ctx().set_cursor_icon(if b_resp.dragged() {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::Grab
            });
        }
        let dt = glowkit_core::dt(ui);
        let time = glowkit_core::time(ui);
        let idle = idle_for(ui, id, b_resp.dragged() || b_resp.hovered());
        let ptr = b_resp.interact_pointer_pos();
        let (started, dragging, released) = (b_resp.drag_started(), b_resp.dragged(), b_resp.drag_stopped());
        let auto = self.autoplay && idle > 2.0;
        let (max_pull, power, rest) = (self.max_pull, self.power, 0.35 + 0.6 * self.bounce);
        let half = stage.size() / 2.0 - Vec2::splat(r);
        let st = glowkit_core::with_state(
            ui,
            id,
            || st0,
            |s| {
                s.t += dt;
                s.burst += dt;
                s.hit += dt;
                let mut launch = false;
                if started && matches!(s.mode, Mode::Rest | Mode::Home) {
                    s.mode = Mode::Pull;
                    s.t = 0.0;
                    s.ghost = -1.0;
                    s.grab = ptr.map(|p| p - (home + s.pos)).unwrap_or_default();
                }
                if s.mode == Mode::Pull && dragging {
                    if let Some(p) = ptr {
                        let raw = p - s.grab - home;
                        let len = raw.length();
                        // resist more as the band stretches, lock at the limit
                        let eased = max_pull * (1.0 - (-len / (max_pull * 0.8)).exp()) / (1.0 - (-1.25f32).exp());
                        s.pos = if len > 0.1 { raw / len * eased.min(max_pull) } else { Vec2::ZERO };
                    }
                }
                if s.mode == Mode::Pull && released {
                    launch = true;
                }
                // ghost: pull back along an aim, tremble, let go
                if auto && s.mode == Mode::Rest && s.ghost < 0.0 && s.t > 0.6 {
                    s.ghost = 0.0;
                    let a = (time * 1.7).sin() * 1.2 + 2.3;
                    s.ghost_aim = vec2(a.cos(), a.sin());
                }
                if s.ghost >= 0.0 {
                    s.ghost += dt;
                    let g = s.ghost;
                    if (0.5..1.5).contains(&g) {
                        s.mode = Mode::Pull;
                        s.pos = s.ghost_aim * max_pull * smoothstep(0.5, 1.4, g);
                    } else if g >= 1.5 && s.mode == Mode::Pull {
                        launch = true;
                    }
                    if g > 1.9 {
                        s.ghost = -1.0;
                    }
                }
                if launch {
                    let stretch = s.pos.length() / max_pull;
                    s.vel = -s.pos * (9.0 + 5.0 * stretch) * power;
                    s.mode = Mode::Fly;
                    s.t = 0.0;
                    s.burst = 0.0;
                }
                match s.mode {
                    Mode::Fly => {
                        let steps = 4;
                        let h = dt / steps as f32;
                        for _ in 0..steps {
                            s.pos += s.vel * h;
                            s.vel *= (-1.1 * h).exp();
                            for (p, v, lim, axis) in [
                                (&mut s.pos.x, &mut s.vel.x, half.x, Vec2::X),
                                (&mut s.pos.y, &mut s.vel.y, half.y, Vec2::Y),
                            ] {
                                if p.abs() > lim {
                                    *p = p.signum() * lim;
                                    if v.abs() > 60.0 {
                                        s.hit = 0.0;
                                        s.hit_dir = axis;
                                    }
                                    *v = -*v * rest;
                                }
                            }
                        }
                        if s.t > 0.4 && (s.vel.length() < 140.0 || s.t > 2.4) {
                            s.mode = Mode::Home;
                            s.t = 0.0;
                        }
                    }
                    Mode::Home => {
                        // critically damped pull back to the socket
                        let k = 90.0;
                        let acc = -s.pos * k - s.vel * 2.0 * k.sqrt() * 0.8;
                        s.vel += acc * dt;
                        s.pos += s.vel * dt;
                        if s.pos.length() < 0.3 && s.vel.length() < 2.0 {
                            s.pos = Vec2::ZERO;
                            s.vel = Vec2::ZERO;
                            s.mode = Mode::Rest;
                            s.t = 0.0;
                        }
                    }
                    _ => {}
                }
                *s
            },
        );
        if st.mode != Mode::Rest || st.ghost >= 0.0 || auto || st.burst < 0.6 {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(stage) {
            return response;
        }
        let painter = ui.painter_at(stage);
        let (ink_bg, ink_fg) = ink(&t);
        let (ball_c, ball_fg) = if self.dark { (ink_bg, ink_fg) } else { (super::common::surface(&t), t.foreground) };
        let well = with_alpha(t.muted_foreground, if t.dark { 0.22 } else { 0.16 });
        // socket
        painter.circle_filled(home, r + 5.0, well);
        let stretch = st.pos.length() / self.max_pull;
        let mut ball = home + st.pos;
        if st.mode == Mode::Pull && stretch > 0.97 {
            // tremble at the limit
            ball += vec2((time * 83.0).sin(), (time * 71.0).cos()) * 0.9;
        }
        // band
        if st.mode == Mode::Pull && stretch > 0.01 {
            let w = (r + 5.0) * 2.0 * (1.0 - 0.45 * stretch);
            painter.line_segment([home, ball], Stroke::new(w, well));
            painter.circle_filled(ball, w / 2.0, well);
        }
        // release burst
        if st.burst < 0.5 {
            let k = st.burst / 0.5;
            let dir = -st.pos.normalized();
            for i in 0..7 {
                let a = dir.angle() + std::f32::consts::PI + (i as f32 - 3.0) * 0.32;
                let d = Vec2::angled(a);
                let r0 = r + 6.0 + k * 18.0;
                let len = 7.0 * (1.0 - k);
                painter.line_segment(
                    [home + d * r0, home + d * (r0 + len)],
                    Stroke::new(1.4, with_alpha(t.muted_foreground, 0.8 * (1.0 - k))),
                );
            }
        }
        // ball squash: along the travel direction while flying, flattened against walls on impact
        let heading = match st.mode {
            Mode::Pull => -st.pos,
            Mode::Rest => vec2(1.0, -1.0),
            _ => st.vel,
        };
        let speed = st.vel.length();
        let mut sx = 1.0 + (speed / 2500.0).min(0.18);
        let mut sy = 1.0 / sx;
        let mut axis = if speed > 1.0 { st.vel / speed } else { Vec2::X };
        if st.hit < 0.18 {
            let k = (1.0 - st.hit / 0.18) * 0.28;
            axis = st.hit_dir;
            sx = 1.0 - k;
            sy = 1.0 + k * 0.7;
        }
        if st.mode == Mode::Pull {
            sx = 1.0 + stretch * 0.08;
            sy = 1.0 - stretch * 0.06;
            axis = if stretch > 0.0 { st.pos.normalized() } else { Vec2::X };
        }
        let n = 48;
        let pts: Vec<Pos2> = (0..n)
            .map(|i| {
                let v = Vec2::angled(i as f32 / n as f32 * TAU) * r;
                let along = v.dot(axis) * sx;
                let across = v.dot(axis.rot90()) * sy;
                ball + axis * along + axis.rot90() * across
            })
            .collect();
        paint::soft_shadow(
            &painter,
            egui::Rect::from_center_size(ball, Vec2::splat(r * 2.0)),
            r,
            12.0,
            vec2(0.0, 5.0),
            shadow_color(&t, 0.9),
        );
        painter.add(egui::Shape::convex_polygon(pts, ball_c, Stroke::NONE));
        let ang = if heading.length() > 0.01 { heading.angle() } else { -std::f32::consts::FRAC_PI_4 };
        let a_s = glowkit_core::anim::follow(ui, id.with("ang_s"), ang.sin(), 18.0);
        let a_c = glowkit_core::anim::follow(ui, id.with("ang_c"), ang.cos(), 18.0);
        arrow(&painter, ball, r * 0.75, a_s.atan2(a_c), Stroke::new(1.8, ball_fg));
        if st.ghost >= 0.0 {
            let g = st.ghost;
            let a = smoothstep(0.1, 0.4, g) * (1.0 - smoothstep(1.5, 1.85, g));
            let tip = if g < 1.5 { ball } else { home + st0.ghost_aim * self.max_pull };
            ghost_cursor(&painter, &t, tip + vec2(3.0, 4.0), a, (0.45..1.5).contains(&g));
        }
        response.union(b_resp)
    }
}
