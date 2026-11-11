//! Small physics helpers: a [`SpringChain`] (follow-the-leader trails), a verlet [`Rope`], and the
//! [`Lanyard`] widget (a card hanging on a strap you can grab and fling).

use crate::gesture::VelocityTracker;
use crate::transform::{layered, LayerSpec, Props, Space};
use egui::{pos2, vec2, Align2, Color32, Id, Pos2, Rect, Response, Sense, Stroke, Ui, UiBuilder, Vec2};
use glowkit_core::{color, paint, Theme};

/// A chain of points where each one springs toward the one in front of it, the first toward a
/// target. Great for trailing cursors, snakes and tails.
///
/// ```no_run
/// use glowkit_motion::SpringChain;
/// let mut chain = SpringChain::new(12, egui::pos2(0.0, 0.0)).stiffness(260.0).damping(18.0);
/// chain.update(egui::pos2(100.0, 50.0), 1.0 / 60.0);
/// for p in chain.points() { /* draw a dot */ }
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct SpringChain {
    points: Vec<Pos2>,
    velocities: Vec<Vec2>,
    stiffness: f32,
    damping: f32,
    falloff: f32,
}

impl SpringChain {
    /// `n` points stacked at `at`.
    pub fn new(n: usize, at: Pos2) -> Self {
        Self { points: vec![at; n.max(1)], velocities: vec![Vec2::ZERO; n.max(1)], stiffness: 300.0, damping: 22.0, falloff: 0.92 }
    }
    /// Spring stiffness of the first link.
    pub fn stiffness(mut self, k: f32) -> Self {
        self.stiffness = k.max(1.0);
        self
    }
    /// Spring damping.
    pub fn damping(mut self, d: f32) -> Self {
        self.damping = d.max(0.0);
        self
    }
    /// Each following link is this much softer (`0..=1`, 1 = all equal) for a whippier tail.
    pub fn falloff(mut self, f: f32) -> Self {
        self.falloff = f.clamp(0.1, 1.0);
        self
    }
    /// The points, head first.
    pub fn points(&self) -> &[Pos2] {
        &self.points
    }
    /// Speed of each point.
    pub fn speeds(&self) -> impl Iterator<Item = f32> + '_ {
        self.velocities.iter().map(|v| v.length())
    }
    /// Is everything at rest?
    pub fn is_settled(&self, head: Pos2) -> bool {
        self.velocities.iter().all(|v| v.length() < 0.5) && self.points.iter().all(|p| p.distance(head) < 0.5)
    }
    /// Advance by `dt` seconds with the head chasing `head`.
    pub fn update(&mut self, head: Pos2, dt: f32) {
        let steps = ((dt / (1.0 / 240.0)).ceil() as usize).clamp(1, 16);
        let h = dt / steps as f32;
        for _ in 0..steps {
            let mut k = self.stiffness;
            for i in 0..self.points.len() {
                let target = if i == 0 { head } else { self.points[i - 1] };
                let f = (target - self.points[i]) * k - self.velocities[i] * self.damping;
                self.velocities[i] += f * h;
                self.points[i] += self.velocities[i] * h;
                k *= self.falloff;
            }
        }
    }
}

/// A verlet-integrated rope: a chain of particles with fixed distances, gravity and damping.
/// Pin particles (e.g. the first to an anchor) and move pinned ones to drag the rope.
///
/// ```no_run
/// use glowkit_motion::Rope;
/// let mut rope = Rope::new(egui::pos2(100.0, 0.0), egui::vec2(0.0, 1.0), 12, 10.0);
/// rope.pin(0, egui::pos2(100.0, 0.0));
/// rope.update(1.0 / 60.0);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Rope {
    points: Vec<Pos2>,
    prev: Vec<Pos2>,
    pinned: Vec<bool>,
    lengths: Vec<f32>,
    /// Acceleration applied to every free particle (points / s²).
    pub gravity: Vec2,
    /// Velocity kept per step (`0..=1`, lower = more air drag).
    pub damping: f32,
    /// Constraint iterations per step (more = stiffer rope).
    pub iterations: usize,
    accumulator: f32,
}

impl Rope {
    /// Fixed simulation step in seconds.
    pub const STEP: f32 = 1.0 / 120.0;

    /// A rope of `segments` links of `segment_len` starting at `start` going along `dir`.
    pub fn new(start: Pos2, dir: Vec2, segments: usize, segment_len: f32) -> Self {
        let d = dir.normalized();
        let points: Vec<Pos2> = (0..=segments.max(1)).map(|i| start + d * segment_len * i as f32).collect();
        Self {
            prev: points.clone(),
            pinned: vec![false; points.len()],
            lengths: vec![segment_len; points.len() - 1],
            points,
            gravity: vec2(0.0, 1400.0),
            damping: 0.985,
            iterations: 24,
            accumulator: 0.0,
        }
    }
    /// Particle positions.
    pub fn points(&self) -> &[Pos2] {
        &self.points
    }
    /// Rest length of link `i` (between particles `i` and `i + 1`).
    pub fn segment_len(&self, i: usize) -> f32 {
        self.lengths.get(i).copied().unwrap_or(0.0)
    }
    /// Change the rest length of link `i` (e.g. a heavier, longer last link for a hanging card).
    pub fn set_segment_len(&mut self, i: usize, len: f32) {
        if let Some(l) = self.lengths.get_mut(i) {
            *l = len.max(0.0);
        }
    }
    /// Pin particle `i` at `pos` (it stops falling and can be moved by calling `pin` again).
    pub fn pin(&mut self, i: usize, pos: Pos2) {
        if i < self.points.len() {
            self.pinned[i] = true;
            self.points[i] = pos;
        }
    }
    /// Release particle `i`; it keeps the velocity it was moved with.
    pub fn unpin(&mut self, i: usize) {
        if i < self.points.len() {
            self.pinned[i] = false;
        }
    }
    /// Move a particle as a positional correction: the move turns into velocity, like a hand
    /// pushing it (use inside [`Self::update_with`] for drag and collision constraints).
    pub fn nudge(&mut self, i: usize, delta: Vec2) {
        if i < self.points.len() {
            self.points[i] += delta;
        }
    }
    /// Move a particle together with its previous position (its velocity is preserved).
    pub fn shift(&mut self, i: usize, delta: Vec2) {
        if i < self.points.len() {
            self.points[i] += delta;
            self.prev[i] += delta;
        }
    }
    /// Set the velocity (points / s) of particle `i`.
    pub fn set_velocity(&mut self, i: usize, v: Vec2) {
        if i < self.points.len() {
            self.prev[i] = self.points[i] - v * Self::STEP;
        }
    }
    /// Velocity (points / s) of particle `i`.
    pub fn velocity(&self, i: usize) -> Vec2 {
        (self.points[i] - self.prev[i]) / Self::STEP
    }
    /// Translate the whole rope (e.g. to follow a scrolled container).
    pub fn translate(&mut self, delta: Vec2) {
        for (p, q) in self.points.iter_mut().zip(self.prev.iter_mut()) {
            *p += delta;
            *q += delta;
        }
    }

    /// Advance by `dt` seconds in fixed steps. `extra(i, &mut point)` can add custom constraints
    /// after the built-in distance constraints of every iteration.
    pub fn update(&mut self, dt: f32) {
        self.update_with(dt, |_| {});
    }

    /// [`Self::update`] with an extra constraint pass run every iteration.
    pub fn update_with(&mut self, dt: f32, mut extra: impl FnMut(&mut Rope)) {
        self.accumulator = (self.accumulator + dt).min(0.1);
        while self.accumulator >= Self::STEP {
            self.accumulator -= Self::STEP;
            for i in 0..self.points.len() {
                if self.pinned[i] {
                    self.prev[i] = self.points[i];
                    continue;
                }
                let v = (self.points[i] - self.prev[i]) * self.damping;
                self.prev[i] = self.points[i];
                self.points[i] += v + self.gravity * Self::STEP * Self::STEP;
            }
            for _ in 0..self.iterations {
                for i in 0..self.points.len() - 1 {
                    let (a, b) = (self.points[i], self.points[i + 1]);
                    let d = b - a;
                    let len = d.length().max(1e-4);
                    let diff = (len - self.lengths[i]) / len;
                    let (wa, wb) = match (self.pinned[i], self.pinned[i + 1]) {
                        (true, true) => (0.0, 0.0),
                        (true, false) => (0.0, 1.0),
                        (false, true) => (1.0, 0.0),
                        (false, false) => (0.5, 0.5),
                    };
                    self.points[i] += d * diff * wa;
                    self.points[i + 1] -= d * diff * wb;
                }
                extra(self);
            }
        }
    }
}

#[derive(Clone, Debug)]
struct LanyardState {
    rope: Rope,
    origin: Pos2,
    grab: Option<(f32, Vec2)>,
    tracker: VelocityTracker,
    idle: f32,
}

/// A card hanging from a strap, simulated with a verlet rope. Grab the card to swing it around,
/// let go to fling it; it sways gently while idle. The card's contents are regular egui widgets
/// (drawn rotated with the card; input follows the card's position but not its rotation).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_motion::Lanyard::new(ui.scope_id().with("badge")).show(ui, |ui| {
///     ui.heading("Ada Lovelace");
///     ui.label("Engineer");
/// });
/// ```
#[must_use = "call `.show(ui, ...)`"]
#[derive(Clone, Debug)]
pub struct Lanyard {
    id: Id,
    height: f32,
    card: Vec2,
    rope_len: f32,
    strap: Option<[Color32; 3]>,
    card_fill: Option<Color32>,
    sway: bool,
}

impl Lanyard {
    /// A lanyard stored under `id`.
    pub fn new(id: Id) -> Self {
        Self { id, height: 380.0, card: vec2(170.0, 220.0), rope_len: 150.0, strap: None, card_fill: None, sway: true }
    }
    /// Height of the area (the width is all available width).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(100.0);
        self
    }
    /// Card size.
    pub fn card_size(mut self, size: Vec2) -> Self {
        self.card = size;
        self
    }
    /// Strap length in points.
    pub fn rope_length(mut self, len: f32) -> Self {
        self.rope_len = len.max(20.0);
        self
    }
    /// Strap gradient (defaults to the theme gradient).
    pub fn strap_colors(mut self, c: [Color32; 3]) -> Self {
        self.strap = Some(c);
        self
    }
    /// Card background (defaults to the theme card color).
    pub fn card_fill(mut self, c: Color32) -> Self {
        self.card_fill = Some(c);
        self
    }
    /// Gentle swaying when nobody is touching it (default on).
    pub fn idle_sway(mut self, on: bool) -> Self {
        self.sway = on;
        self
    }

    /// Show the lanyard with `add_contents` drawn on the card.
    pub fn show(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) -> Response {
        let t = Theme::of(ui);
        let (area, resp) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let anchor = pos2(area.center().x, area.min.y + 6.0);
        let segments = 14;
        let seg = self.rope_len / segments as f32;
        let card_h = self.card.y;
        let skey = self.id.with("glowkit_lanyard");
        let mut st: LanyardState = ui.data(|d| d.get_temp(skey)).unwrap_or_else(|| {
            // start swung out to one side so it is already moving on the first frame
            let dir = vec2(0.8, 0.6);
            let mut rope = Rope::new(anchor, dir, segments + 1, seg);
            // the last link is the card's stiff axis (top clip → bottom edge)
            rope.set_segment_len(segments, card_h);
            rope.pin(0, anchor);
            LanyardState { rope, origin: area.min, grab: None, tracker: VelocityTracker::default(), idle: 0.0 }
        });
        // follow the container when it moves (scrolling, resizing)
        let shift = area.min - st.origin;
        if shift != Vec2::ZERO {
            st.rope.translate(shift);
            st.origin = area.min;
        }
        st.rope.pin(0, anchor);
        let n = st.rope.points().len();
        let (top_i, bot_i) = (n - 2, n - 1);

        let dt = glowkit_core::dt(ui);
        let now = ui.input(|i| i.time);
        let pointer = ui.input(|i| i.pointer.latest_pos());
        let down = ui.input(|i| i.pointer.primary_down());

        // card geometry from the last link
        let geom = |rope: &Rope| {
            let top = rope.points()[top_i];
            let bot = rope.points()[bot_i];
            let axis = (bot - top).normalized();
            (top, axis)
        };
        let (top, axis) = geom(&st.rope);
        let card_center = top + axis * card_h / 2.0;
        // rotation that maps card-local "down" onto the card axis
        let rot = egui::emath::Rot2::from_angle(axis.y.atan2(axis.x) - std::f32::consts::FRAC_PI_2);

        // grab / drag / fling
        let hit = ui.interact(area, self.id.with("glowkit_lanyard_drag"), Sense::drag());
        if hit.drag_started() {
            if let Some(p) = pointer {
                // pointer in card-local coordinates
                let local = rot.inverse() * (p - card_center);
                if local.x.abs() <= self.card.x / 2.0 + 8.0 && local.y.abs() <= card_h / 2.0 + 8.0 {
                    let u = ((local.y + card_h / 2.0) / card_h).clamp(0.0, 1.0);
                    st.grab = Some((u, vec2(local.x, 0.0)));
                    st.tracker.clear();
                }
            }
        }
        let card_l = self.card;
        if let (Some((u, _)), Some(p), true) = (st.grab, pointer, down) {
            st.tracker.push(now, p);
            st.idle = 0.0;
            st.rope.update_with(dt, |rope| {
                let pts = rope.points();
                let (a, b) = (pts[top_i], pts[bot_i]);
                let grip = a + (b - a) * u;
                let d = (p - grip) * 0.5;
                rope.nudge(top_i, d);
                rope.nudge(bot_i, d);
            });
        } else {
            if st.grab.take().is_some() {
                let v = st.tracker.velocity().clamp(Vec2::splat(-3000.0), Vec2::splat(3000.0));
                st.rope.set_velocity(top_i, v * 0.8);
                st.rope.set_velocity(bot_i, v);
            }
            st.idle += dt;
            if self.sway {
                let w = (glowkit_core::time(ui) * 0.9).sin() * 600.0 * (st.idle / 2.0).min(1.0);
                st.rope.gravity = vec2(w, 1400.0);
            }
            let floor = area.max.y - 4.0;
            let (lo, hi) = (area.min.x + card_l.x * 0.5, area.max.x - card_l.x * 0.5);
            st.rope.update_with(dt, |rope| {
                // keep the card inside the area
                for i in [top_i, bot_i] {
                    let p = rope.points()[i];
                    let c = pos2(p.x.clamp(lo, hi), p.y.min(floor));
                    if c != p {
                        rope.nudge(i, c - p);
                    }
                }
            });
        }
        let (top, axis) = geom(&st.rope);
        let card_center = top + axis * card_h / 2.0;
        let angle_deg = (axis.y.atan2(axis.x) - std::f32::consts::FRAC_PI_2).to_degrees();

        // strap
        let painter = ui.painter_at(area);
        let strap = self.strap.unwrap_or(t.gradient);
        let pts: Vec<Pos2> = st.rope.points()[..=top_i].to_vec();
        let smooth = paint::catmull_rom(&pts, 4);
        painter.circle_filled(anchor, 6.0, t.border);
        paint::gradient_polyline(&painter, &smooth, false, 12.0, |k| color::gradient(&strap, k));
        paint::gradient_polyline(&painter, &smooth, false, 3.0, |_| color::with_alpha(Color32::WHITE, 0.18));
        painter.circle_filled(anchor, 4.0, t.muted_foreground);

        // card: a transformed layer (translate + rotate) holding the user's content
        let rest = Rect::from_min_size(area.min, self.card);
        let props = Props::new().offset(card_center - rest.center()).rotate(angle_deg);
        let fill = self.card_fill.unwrap_or(t.card);
        let mut host = ui.new_child(UiBuilder::new().max_rect(rest).id_salt(self.id.with("host")));
        layered(&mut host, self.id.with("card"), LayerSpec { props, origin: Align2::CENTER_CENTER, sense: Sense::hover(), space: Space::Normal }, |ui| {
            let p = ui.painter();
            paint::soft_shadow(p, rest.translate(vec2(0.0, 10.0)), t.radius + 4.0, 26.0, Vec2::ZERO, Color32::from_black_alpha(if t.dark { 150 } else { 60 }));
            p.rect_filled(rest, t.radius + 4.0, fill);
            p.rect_stroke(rest, t.radius + 4.0, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
            // clip slot + metal clip
            let slot = Rect::from_center_size(pos2(rest.center().x, rest.min.y + 16.0), vec2(36.0, 7.0));
            p.rect_filled(slot, 4.0, if t.dark { t.background } else { t.muted });
            let clip = Rect::from_center_size(pos2(rest.center().x, rest.min.y + 2.0), vec2(20.0, 22.0));
            p.rect_filled(clip, 5.0, color::mix(t.muted_foreground, t.foreground, 0.3));
            p.rect_filled(clip.shrink2(vec2(5.0, 6.0)), 2.0, fill);
            let inner = rest.shrink2(vec2(16.0, 0.0)).with_min_y(rest.min.y + 32.0).with_max_y(rest.max.y - 14.0);
            let mut cui = ui.new_child(UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Center)));
            add_contents(&mut cui);
            ui.allocate_rect(rest, Sense::hover());
        });
        if st.grab.is_some() || hit.hovered() {
            ui.ctx().set_cursor_icon(if st.grab.is_some() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        ui.data_mut(|d| d.insert_temp(skey, st));
        ui.ctx().request_repaint();
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_follows_head() {
        let mut c = SpringChain::new(5, Pos2::ZERO).stiffness(200.0).damping(25.0);
        for _ in 0..600 {
            c.update(pos2(50.0, 20.0), 1.0 / 60.0);
        }
        assert!(c.is_settled(pos2(50.0, 20.0)));
    }

    #[test]
    fn rope_keeps_its_length() {
        let mut r = Rope::new(Pos2::ZERO, Vec2::X, 10, 10.0);
        r.pin(0, Pos2::ZERO);
        for _ in 0..240 {
            r.update(1.0 / 60.0);
        }
        let len: f32 = r.points().windows(2).map(|w| w[0].distance(w[1])).sum();
        assert!((len - 100.0).abs() < 3.0, "{len}");
        // it fell: the end hangs below the anchor
        assert!(r.points()[10].y > 80.0);
    }
}
