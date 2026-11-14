use super::common::{
    cached_ink, ghost_cursor, idle_time, lerp_c, sstep, wall_dt, Cam, Ink, Sheet, Soup, PAPER_BACK, V3,
};
use super::crumple::{desk_color, paint_desk};
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::{hex, paint, Theme};
use std::f32::consts::PI;

const HOLES: usize = 11;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Attached,
    Flying,
    Reprint,
}

#[derive(Clone, Copy, Debug)]
struct TearState {
    phase: Phase,
    /// 0..1 how much of the perforation has given (top to bottom).
    tear: f32,
    shown_tear: f32,
    angle: f32,
    ang_vel: f32,
    lift: f32,
    grab: Option<Pos2>,
    t: f32,
    pos: Vec2,
    vel: Vec2,
    spin: f32,
    ghost: f32,
    torn_count: u32,
}

impl Default for TearState {
    fn default() -> Self {
        Self {
            phase: Phase::Attached,
            tear: 0.0,
            shown_tear: 0.0,
            angle: 0.0,
            ang_vel: 0.0,
            lift: 0.0,
            grab: None,
            t: 0.0,
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
            spin: 0.0,
            ghost: -1.0,
            torn_count: 0,
        }
    }
}

/// A train ticket whose stub tears off like real paper. Grab the stub and pull: the perforation
/// gives one hole at a time, the loose corner peels and curls as a bent paper mesh, and once
/// the last hole lets go the stub flutters away, tumbling in 3D, before a fresh one is printed.
///
/// The response reports `changed()` on the frame a stub comes free.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::TearTicket;
/// let r = ui.add(TearTicket::new("Porto", "Lyon").berth("42").accent(egui::Color32::from_rgb(170, 30, 40)));
/// if r.changed() { /* ticket used */ }
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TearTicket {
    from: String,
    to: String,
    berth: String,
    operator: String,
    accent: Option<Color32>,
    size: Option<Vec2>,
    scale: f32,
    resistance: f32,
    autoplay: bool,
}

impl TearTicket {
    /// A ticket from `from` to `to`.
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            berth: "42".into(),
            operator: "NIGHT LINE  ·  SLEEPER".into(),
            accent: None,
            size: None,
            scale: 1.0,
            resistance: 1.0,
            autoplay: true,
        }
    }
    /// Berth number printed big on the stub.
    pub fn berth(mut self, b: impl Into<String>) -> Self {
        self.berth = b.into();
        self
    }
    /// Small header line (operator / service).
    pub fn operator(mut self, s: impl Into<String>) -> Self {
        self.operator = s.into();
        self
    }
    /// Ink color for the stripe, header band and serial (default: a deep train red).
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = Some(c);
        self
    }
    /// Widget size (default: available width x 300).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Ticket scale (default 1 = 440 x 156 points).
    pub fn scale(mut self, k: f32) -> Self {
        self.scale = k.clamp(0.5, 2.0);
        self
    }
    /// How hard you must pull per hole (default 1).
    pub fn resistance(mut self, k: f32) -> Self {
        self.resistance = k.clamp(0.3, 3.0);
        self
    }
    /// Demonstrate the tear with a ghost cursor while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn print_stub(ink: &mut Ink, size: Vec2, accent: Color32, berth: &str) {
    let s = size.x / 110.0;
    ink.rect(Rect::from_min_size(Pos2::ZERO, vec2(size.x, 24.0 * s)), 0.0, accent);
    ink.text(
        pos2(size.x * 0.5, 12.0 * s),
        Align2::CENTER_CENTER,
        "ADMIT ONE",
        FontId::monospace(8.0 * s),
        hex("#FBEFE6"),
    );
    ink.text(pos2(size.x * 0.5, 34.0 * s), Align2::CENTER_TOP, "BERTH", FontId::monospace(6.5 * s), hex("#6B6460"));
    ink.text(pos2(size.x * 0.5, 44.0 * s), Align2::CENTER_TOP, berth, FontId::proportional(30.0 * s), hex("#2A2624"));
    ink.text(
        pos2(size.x * 0.5, 80.0 * s),
        Align2::CENTER_TOP,
        "Coach 07 · Lower",
        FontId::proportional(6.5 * s),
        hex("#6B6460"),
    );
    // barcode
    let mut x = 14.0 * s;
    let mut k = 7u64;
    while x < size.x - 14.0 * s {
        k = k.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let w = (1 + (k >> 60) % 3) as f32 * 0.9 * s;
        ink.rect(Rect::from_min_size(pos2(x, 100.0 * s), vec2(w, 22.0 * s)), 0.0, hex("#2A2624"));
        x += w + (1 + (k >> 50) % 3) as f32 * 0.9 * s;
    }
    ink.text(
        pos2(size.x * 0.5, 126.0 * s),
        Align2::CENTER_TOP,
        "Nº 004821",
        FontId::monospace(5.5 * s),
        hex("#6B6460"),
    );
}

impl Widget for TearTicket {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = glowkit_core::desired_size(ui, self.size, 300.0);
        let (rect, mut response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let id = response.id;
        let k = self.scale;
        let tsize = vec2(440.0, 156.0) * k;
        let stub_w = 112.0 * k;
        let ticket = Rect::from_center_size(rect.center() - vec2(0.0, 4.0), tsize);
        let body = Rect::from_min_max(ticket.min, pos2(ticket.max.x - stub_w, ticket.max.y));
        let stub = Rect::from_min_max(pos2(body.max.x, ticket.min.y), ticket.max);
        let accent = self.accent.unwrap_or(hex("#A3202A"));
        let paper = hex("#F3EEE2");
        let dt = glowkit_core::dt(ui);
        let wdt = wall_dt(ui);
        let idle = idle_time(ui, id, rect);
        let st0: TearState = glowkit_core::state(ui, id, TearState::default);
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let on_stub = pointer.is_some_and(|p| stub.expand(6.0).contains(p)) && st0.phase == Phase::Attached;
        if on_stub || response.dragged() && st0.grab.is_some() {
            ui.ctx().set_cursor_icon(if response.dragged() {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::Grab
            });
        }
        let started = response.drag_started() && on_stub;
        let dragging = response.dragged();
        let stopped = response.drag_stopped();
        let resist = self.resistance;
        let autoplay = self.autoplay && idle > 2.0;
        let pivot_of = |tear: f32| pos2(stub.min.x, stub.min.y + tear * stub.height());
        let grab_home = stub.min + vec2(stub_w * 0.7, stub.height() * 0.18);
        let mut freed = false;
        let mut ghost_draw: Option<(Pos2, bool, f32)> = None;
        let st = glowkit_core::with_state(ui, id, TearState::default, |s| {
            let mut target: Option<Pos2> = None;
            match s.phase {
                Phase::Attached => {
                    if started {
                        s.grab = pointer;
                        s.ghost = -1.0;
                    }
                    if dragging && s.grab.is_some() {
                        target = pointer;
                    } else if s.grab.is_none() && (s.ghost >= 0.0 || autoplay) {
                        if s.ghost < 0.0 {
                            s.ghost = 0.0;
                        }
                        s.ghost += wdt;
                        let g = s.ghost;
                        // approach, press, pull up and to the right in jerks
                        let start = grab_home + vec2(120.0, 60.0);
                        let tip = if g < 0.9 {
                            start.lerp(grab_home, sstep(0.0, 0.9, g))
                        } else {
                            let pull = sstep(1.1, 3.2, g) + 0.04 * ((g - 1.1) * 9.0).sin().max(0.0);
                            grab_home + vec2(100.0, -115.0) * k * pull * 1.25
                        };
                        let pressed = g > 0.95;
                        ghost_draw = Some((tip, pressed, sstep(0.0, 0.3, g)));
                        if pressed {
                            target = Some(tip);
                        }
                    }
                    if let Some(p) = target {
                        let g0 = s.grab.unwrap_or(grab_home);
                        let pull = (p - g0).length();
                        let need = (stub.height() * 1.1) * resist;
                        s.tear = s.tear.max(((pull - 8.0) / need).clamp(0.0, 1.0));
                        let piv = pivot_of(s.shown_tear);
                        let a0 = (g0 - piv).angle();
                        let a1 = (p - piv).angle();
                        let mut da = a1 - a0;
                        while da > PI {
                            da -= 2.0 * PI;
                        }
                        while da < -PI {
                            da += 2.0 * PI;
                        }
                        let goal = da.clamp(-1.3, 0.6) * (0.35 + 0.65 * s.shown_tear);
                        s.ang_vel = (goal - s.angle) / dt.max(1e-3) * 0.4;
                        s.angle += (goal - s.angle) * (1.0 - (-14.0 * dt).exp());
                        s.lift = s.lift + (1.0 - s.lift) * (1.0 - (-8.0 * dt).exp());
                    } else {
                        // let go: spring back flat (the torn holes stay torn)
                        let acc = -160.0 * s.angle - 14.0 * s.ang_vel;
                        s.ang_vel += acc * dt;
                        s.angle += s.ang_vel * dt;
                        s.lift *= (-6.0 * dt).exp();
                        if stopped {
                            s.grab = None;
                        }
                        if s.ghost > 4.0 {
                            s.ghost = -1.0;
                        }
                    }
                    // the perforation gives hole by hole
                    let holes = (s.tear * HOLES as f32).floor() / HOLES as f32;
                    s.shown_tear += (holes - s.shown_tear) * (1.0 - (-18.0 * dt).exp());
                    if s.tear >= 0.999 && s.shown_tear > 0.97 {
                        s.phase = Phase::Flying;
                        s.t = 0.0;
                        s.pos = Vec2::ZERO;
                        s.vel = vec2(120.0, -260.0) * k;
                        s.spin = s.angle;
                        s.grab = None;
                        s.ghost = -1.0;
                        s.torn_count += 1;
                        freed = true;
                    }
                }
                Phase::Flying => {
                    s.t += dt;
                    s.vel.y += 520.0 * dt;
                    s.vel *= (-1.2 * dt).exp();
                    s.vel.x += (s.t * 5.0).sin() * 260.0 * dt;
                    s.pos += s.vel * dt;
                    s.spin += (1.6 + (s.t * 3.0).cos()) * dt;
                    if s.t > 1.6 {
                        s.phase = Phase::Reprint;
                        s.t = 0.0;
                    }
                }
                Phase::Reprint => {
                    s.t += dt;
                    if s.t > 1.0 {
                        *s = TearState { torn_count: s.torn_count, ..TearState::default() };
                    }
                }
            }
            *s
        });
        if freed {
            response.mark_changed();
        }
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let desk = desk_color(&t);
        paint_desk(&painter, rect, desk, t.radius * 1.4);

        // ---- ticket body (flat paper)
        let body_r = 6.0 * k;
        paint::soft_shadow(
            &painter,
            body.translate(vec2(0.0, 6.0)),
            body_r,
            16.0,
            Vec2::ZERO,
            Color32::from_black_alpha(40),
        );
        painter.rect_filled(body, egui::CornerRadius { nw: body_r as u8, sw: body_r as u8, ne: 0, se: 0 }, paper);
        painter.rect_filled(
            Rect::from_min_size(body.min, vec2(5.0 * k, body.height())),
            egui::CornerRadius { nw: body_r as u8, sw: body_r as u8, ne: 0, se: 0 },
            accent,
        );
        let x0 = body.min.x + 22.0 * k;
        let ink = hex("#2A2624");
        let soft = hex("#7A726C");
        painter.text(
            pos2(x0, body.min.y + 16.0 * k),
            Align2::LEFT_TOP,
            &self.operator,
            FontId::monospace(7.5 * k),
            soft,
        );
        painter.text(
            pos2(body.max.x - 16.0 * k, body.min.y + 16.0 * k),
            Align2::RIGHT_TOP,
            "Nº 004821",
            FontId::monospace(7.5 * k),
            accent,
        );
        let r = painter.text(
            pos2(x0, body.min.y + 32.0 * k),
            Align2::LEFT_TOP,
            &self.from,
            FontId::proportional(27.0 * k),
            ink,
        );
        let r2 = painter.text(
            pos2(r.max.x + 7.0 * k, r.max.y - 5.0 * k),
            Align2::LEFT_BOTTOM,
            "to",
            FontId::proportional(14.0 * k),
            accent,
        );
        painter.text(
            pos2(r2.max.x + 7.0 * k, r.min.y),
            Align2::LEFT_TOP,
            &self.to,
            FontId::proportional(27.0 * k),
            ink,
        );
        painter.text(
            pos2(x0, body.min.y + 68.0 * k),
            Align2::LEFT_TOP,
            "Departs 21:10  ·  Arrives 08:05",
            FontId::monospace(7.0 * k),
            soft,
        );
        for (i, (lab, val)) in
            [("COACH", "07"), ("BERTH", self.berth.as_str()), ("CLASS", "I"), ("DATE", "14 Nov")].iter().enumerate()
        {
            let x = x0 + i as f32 * 62.0 * k;
            painter.text(pos2(x, body.min.y + 90.0 * k), Align2::LEFT_TOP, *lab, FontId::monospace(6.0 * k), soft);
            painter.text(pos2(x, body.min.y + 100.0 * k), Align2::LEFT_TOP, *val, FontId::proportional(13.0 * k), ink);
        }
        // guilloche waves
        for w in 0..5 {
            let pts: Vec<Pos2> = (0..=60)
                .map(|i| {
                    let u = i as f32 / 60.0;
                    let x = body.min.x + 8.0 * k + u * (body.width() - 10.0 * k);
                    let y = body.max.y - 22.0 * k
                        + ((u * 9.0 + w as f32 * 0.7).sin() * 5.0 + (u * 23.0 + w as f32).sin() * 1.5) * k;
                    pos2(x, y)
                })
                .collect();
            painter.add(Shape::line(pts, Stroke::new(0.8, with_alpha(accent, 0.22))));
        }
        // torn edge (half holes) along the body's right side where the stub has gone
        let holes_y = |i: usize| stub.min.y + (i as f32 + 0.5) / HOLES as f32 * stub.height();
        let hole_r = 2.6 * k;
        for i in 0..HOLES {
            painter.circle_filled(pos2(stub.min.x, holes_y(i)), hole_r, desk);
        }
        // notches top and bottom
        painter.circle_filled(pos2(stub.min.x, stub.min.y), 6.0 * k, desk);
        painter.circle_filled(pos2(stub.min.x, stub.max.y), 6.0 * k, desk);

        // ---- stub (bent paper mesh)
        let stub_size = stub.size();
        let berth = self.berth.clone();
        let ink_mesh =
            cached_ink(ui, id.with(("stub", berth.clone(), (k * 100.0) as i32, accent.to_array())), 6.0, |ink| {
                print_stub(ink, stub_size, accent, &berth)
            });
        let mut soup = Soup::new();
        let mut cam = Cam::new(stub.center());
        cam.focal = 900.0;
        let mut alpha = 1.0;
        let tear = st.shown_tear;
        let angle = st.angle;
        let lift = st.lift;
        let (sw, sh) = (stub_size.x, stub_size.y);
        let attached_deform = move |u: f32, v: f32| -> V3 {
            let p = vec2((u - 0.5) * sw, (v - 0.5) * sh);
            let piv = vec2(-sw * 0.5, (tear - 0.5) * sh);
            // distance from the part of the perforation that still holds
            let dy = ((v - tear) * sh).min(0.0);
            let dist = vec2(u * sw, dy).length();
            let w = (dist / (sw * 1.05)).clamp(0.0, 1.0);
            let w = w * w * (3.0 - 2.0 * w);
            let a = angle * w;
            let (s, c) = a.sin_cos();
            let q = p - piv;
            let r = piv + vec2(q.x * c - q.y * s, q.x * s + q.y * c);
            let curl = (w * PI * 0.5).sin() * (lift * 22.0 + angle.abs() * 18.0);
            V3 { x: r.x, y: r.y, z: curl }
        };
        let spin = st.spin;
        let ft = st.t;
        let fly_deform = move |u: f32, v: f32| -> V3 {
            V3 { x: (u - 0.5) * sw, y: (v - 0.5) * sh, z: ((u - 0.5) * PI).cos() * 10.0 * (ft * 7.0).sin() }
        };
        let paper_c = paper;
        let front = move |_u: f32, _v: f32| paper_c;
        match st.phase {
            Phase::Attached | Phase::Reprint => {
                if st.phase == Phase::Reprint {
                    alpha = sstep(0.2, 0.9, st.t);
                    cam.offset.y = -(1.0 - alpha) * 10.0;
                }
                let sheet = Sheet {
                    size: stub_size,
                    cols: 10,
                    rows: 14,
                    deform: &attached_deform,
                    front: &front,
                    back: lerp_c(PAPER_BACK, paper, 0.3),
                    ambient: 0.55,
                    gloss: 0.1,
                    bias: 0.0,
                    alpha,
                    ink_bias: 400.0,
                };
                if lift < 0.02 && angle.abs() < 0.01 {
                    paint::soft_shadow(
                        &painter,
                        stub.translate(vec2(0.0, 6.0)),
                        body_r,
                        16.0,
                        Vec2::ZERO,
                        Color32::from_black_alpha((40.0 * alpha) as u8),
                    );
                } else {
                    // a lifted corner throws a wider, softer shadow
                    let c = stub.center() + vec2(14.0, 14.0) * (lift + angle.abs());
                    super::common::contact_shadow(&painter, c, stub_size * 0.6, 0.14 * alpha);
                }
                sheet.emit(&mut soup, &cam, Some(&ink_mesh));
                soup.paint(&painter);
                // perforation holes (where the stub is still attached they show the desk through)
                for i in 0..HOLES {
                    let y = holes_y(i);
                    if (y - stub.min.y) / stub.height() > tear {
                        painter.circle_filled(pos2(stub.min.x, y), hole_r, desk);
                    }
                }
                if st.phase == Phase::Attached && lift < 0.02 && angle.abs() < 0.01 {
                    painter.rect_stroke(
                        stub,
                        egui::CornerRadius { ne: body_r as u8, se: body_r as u8, nw: 0, sw: 0 },
                        Stroke::new(0.8, with_alpha(paper, 0.9)),
                        egui::StrokeKind::Inside,
                    );
                }
            }
            Phase::Flying => {
                cam.center = stub.center() + st.pos;
                cam.roll = spin;
                cam.pitch = (ft * 4.0).sin() * 1.1;
                cam.yaw = ft * 3.2;
                alpha = 1.0 - sstep(0.9, 1.6, ft);
                let sheet = Sheet {
                    size: stub_size,
                    cols: 8,
                    rows: 12,
                    deform: &fly_deform,
                    front: &front,
                    back: lerp_c(PAPER_BACK, paper, 0.3),
                    ambient: 0.5,
                    gloss: 0.2,
                    bias: 0.0,
                    alpha,
                    ink_bias: 400.0,
                };
                super::common::contact_shadow(
                    &painter,
                    pos2(cam.center.x + 30.0, rect.max.y - 30.0),
                    vec2(50.0, 8.0),
                    0.12 * alpha,
                );
                sheet.emit(&mut soup, &cam, Some(&ink_mesh));
                soup.paint(&painter);
            }
        }
        if let Some((tip, pressed, a)) = ghost_draw {
            ghost_cursor(&painter, t.dark, tip, a, pressed);
        }
        if st.torn_count > 0 {
            painter.text(
                pos2(rect.center().x, rect.max.y - 18.0),
                Align2::CENTER_CENTER,
                format!("{} stub{} torn", st.torn_count, if st.torn_count == 1 { "" } else { "s" }),
                FontId::proportional(12.0),
                t.muted_foreground,
            );
        }
        response
    }
}
