//! Selection controls: a magnetic cluster of options, a liquid toggle and a checklist.

use super::common::{cr, idle_for, soft_card, surface, track, track_fill};
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};
use glowkit_core::anim::{ease, lerp, Easing, Spring};
use glowkit_core::color::{gradient_cyclic, hex, mix, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::TAU;

fn bounce_damping(bounce: f32, stiffness: f32) -> f32 {
    2.0 * stiffness.sqrt() * lerp(1.0, 0.15, (bounce / 100.0).clamp(0.0, 1.0))
}

// ---------------------------------------------------------------------------------------------
// MagneticSelect

#[derive(Clone, Copy)]
struct Node {
    pos: Vec2,
    vel: Vec2,
    radius: Spring,
    glow: f32,
}

#[derive(Clone, Default)]
struct MagnetState {
    nodes: Vec<Node>,
    auto_step: i32,
}

/// A cluster of round options. Press one and it swells into a glossy, iridescent orb while
/// the others are shoved out of its way, then drift back toward their places as if held by
/// magnets. Collisions, pull and bounce are all simulated.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut choice = 0_usize;
/// use glowkit_effects::MagneticSelect;
/// ui.add(MagneticSelect::new(&mut choice).options(7).pull(55.0).bounce(55.0).give(50.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct MagneticSelect<'a> {
    selected: &'a mut usize,
    options: usize,
    radius: f32,
    pull: f32,
    bounce: f32,
    give: f32,
    stroke: bool,
    colors: Vec<Color32>,
    autoplay: bool,
}

impl<'a> MagneticSelect<'a> {
    /// A cluster editing the index of the selected option.
    pub fn new(selected: &'a mut usize) -> Self {
        Self {
            selected,
            options: 7,
            radius: 22.0,
            pull: 55.0,
            bounce: 55.0,
            give: 50.0,
            stroke: false,
            colors: vec![hex("#F472B6"), hex("#C084FC"), hex("#38BDF8"), hex("#34D399"), hex("#FACC15")],
            autoplay: false,
        }
    }
    /// Number of options (2..=13).
    pub fn options(mut self, n: usize) -> Self {
        self.options = n.clamp(2, 13);
        self
    }
    /// Resting radius of each option in points.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.max(6.0);
        self
    }
    /// How strongly options are pulled back home, 0..100.
    pub fn pull(mut self, v: f32) -> Self {
        self.pull = v.clamp(0.0, 100.0);
        self
    }
    /// Overshoot of the swell, 0..100.
    pub fn bounce(mut self, v: f32) -> Self {
        self.bounce = v.clamp(0.0, 100.0);
        self
    }
    /// How softly options yield when pushed (higher = squishier), 0..100.
    pub fn give(mut self, v: f32) -> Self {
        self.give = v.clamp(0.0, 100.0);
        self
    }
    /// Draw hairline borders.
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Colors swirling in the selected orb.
    pub fn colors(mut self, c: Vec<Color32>) -> Self {
        if !c.is_empty() {
            self.colors = c;
        }
        self
    }
    /// Pick options on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn home(i: usize, n: usize, r: f32) -> Vec2 {
    if i == 0 {
        return Vec2::ZERO;
    }
    let ring = n - 1;
    let (k, per, rr) = if ring <= 6 { (i - 1, ring, 2.25 * r) } else if i <= 6 { (i - 1, 6, 2.25 * r) } else { (i - 7, ring - 6, 4.4 * r) };
    let off = if i > 6 { 0.5 } else { 0.0 };
    let a = TAU * (k as f32 + off) / per as f32 - TAU / 4.0 + TAU / 12.0;
    vec2(a.cos(), a.sin()) * rr
}

fn paint_orb(painter: &egui::Painter, c: Pos2, r: f32, colors: &[Color32], time: f32, alpha: f32) {
    let pts = paint::circle_points(c, r, 48);
    paint::fill_convex_fn(painter, &pts, 6, |p| {
        let d = p - c;
        let a = d.y.atan2(d.x) / TAU + time * 0.08;
        let rn = (d.length() / r).min(1.0);
        let swirl = a + rn * 0.35 + (rn * 5.0 - time * 1.3).sin() * 0.05;
        let base = gradient_cyclic(colors, swirl);
        // glossy highlight top-left, soft darkening at the rim
        let hl = (1.0 - (d - vec2(-0.35, -0.4) * r).length() / (r * 0.9)).clamp(0.0, 1.0).powi(2) * 0.55;
        let rim = rn.powi(4) * 0.18;
        let col = mix(mix(base, Color32::WHITE, hl), Color32::BLACK, rim);
        with_alpha(col, alpha)
    });
}

impl egui::Widget for MagneticSelect<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.options;
        let r = self.radius;
        let extent = if n > 7 { 5.6 * r } else { 3.6 * r };
        let (rect, mut response) = ui.allocate_exact_size(vec2(ui.available_width(), extent * 2.0 + 20.0), Sense::click());
        let id = response.id;
        let c = rect.center();
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let pressing = response.is_pointer_button_down_on();
        let ptr = response.interact_pointer_pos().or(response.hover_pos());
        let idle = idle_for(ui, id, response.hovered());
        let mut st: MagnetState = glowkit_core::state(ui, id.with("mag"), MagnetState::default);
        if st.nodes.len() != n {
            st.nodes = (0..n)
                .map(|i| Node { pos: home(i, n, r), vel: Vec2::ZERO, radius: Spring::new(r), glow: 0.0 })
                .collect();
        }
        let hit = ptr.and_then(|p| {
            (0..n).rev().find(|&i| (p - (c + st.nodes[i].pos)).length() <= st.nodes[i].radius.value)
        });
        if pressing && response.is_pointer_button_down_on() {
            if let Some(h) = hit {
                if *self.selected != h {
                    *self.selected = h;
                    response.mark_changed();
                }
            }
        }
        if self.autoplay && !response.hovered() && idle > 1.0 {
            let k = ((idle - 1.0) / 1.7) as i32;
            if k != st.auto_step {
                st.auto_step = k;
                let seq = [2usize, 5, 0, 3, 6, 1, 4];
                *self.selected = seq[k.rem_euclid(seq.len() as i32) as usize] % n;
            }
        }
        let sel = (*self.selected).min(n - 1);
        let pull = lerp(20.0, 200.0, self.pull / 100.0);
        let pull_damp = 2.0 * pull.sqrt() * 0.75;
        let swell_stiff = 260.0;
        let swell_damp = bounce_damping(self.bounce, swell_stiff);
        let give = self.give / 100.0;
        let mut settled = true;
        let sub = 4;
        let h = dt / sub as f32;
        for i in 0..n {
            let target = if i == sel {
                r * if pressing && hit == Some(i) { 1.38 } else { 1.5 }
            } else if hit == Some(i) && !pressing {
                r * 1.06
            } else {
                r
            };
            let nd = &mut st.nodes[i];
            nd.radius.target = target;
            nd.radius.stiffness = swell_stiff;
            nd.radius.damping = swell_damp;
            nd.radius.step(dt);
            nd.glow = glowkit_core::anim::approach(nd.glow, if i == sel { 1.0 } else { 0.0 }, 9.0, dt);
            settled &= nd.radius.is_settled() && (nd.glow - if i == sel { 1.0 } else { 0.0 }).abs() < 0.002;
        }
        for _ in 0..sub {
            for i in 0..n {
                let hm = home(i, n, r);
                let nd = &mut st.nodes[i];
                let f = (hm - nd.pos) * pull - nd.vel * pull_damp;
                nd.vel += f * h;
            }
            // collisions: soft position correction + velocity exchange
            for i in 0..n {
                for j in (i + 1)..n {
                    let (a, b) = (st.nodes[i], st.nodes[j]);
                    let d = b.pos - a.pos;
                    let dist = d.length().max(0.01);
                    let min = a.radius.value + b.radius.value + 4.0;
                    if dist < min {
                        let nrm = d / dist;
                        let overlap = min - dist;
                        let k = lerp(0.9, 0.25, give);
                        // the selected (heavier) node yields less
                        let (wa, wb) = if i == sel { (0.15, 0.85) } else if j == sel { (0.85, 0.15) } else { (0.5, 0.5) };
                        st.nodes[i].pos -= nrm * overlap * k * wa;
                        st.nodes[j].pos += nrm * overlap * k * wb;
                        let push = overlap * lerp(40.0, 14.0, give);
                        st.nodes[i].vel -= nrm * push * wa * h * 60.0;
                        st.nodes[j].vel += nrm * push * wb * h * 60.0;
                    }
                }
            }
            for nd in st.nodes.iter_mut() {
                nd.pos += nd.vel * h;
            }
        }
        for (i, nd) in st.nodes.iter().enumerate() {
            settled &= nd.vel.length() < 0.3 && (nd.pos - home(i, n, r)).length() < 30.0;
        }
        let nodes = st.nodes.clone();
        glowkit_core::set_state(ui, id.with("mag"), st);
        if !settled || self.autoplay || nodes.iter().any(|n| n.vel.length() > 0.05) {
            glowkit_core::animate(ui);
        }
        if hit.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|a, b| nodes[*a].glow.partial_cmp(&nodes[*b].glow).unwrap_or(std::cmp::Ordering::Equal));
        let base = surface(&t);
        for i in order {
            let nd = nodes[i];
            let p = c + nd.pos;
            let rr = nd.radius.value.max(1.0);
            let sc = if t.dark { Color32::from_black_alpha(90) } else { Color32::from_rgba_unmultiplied(30, 30, 60, 16) };
            paint::soft_shadow(painter, Rect::from_center_size(p, Vec2::splat(rr * 2.0)), rr, 10.0, vec2(0.0, 3.0), sc);
            painter.circle_filled(p, rr, base);
            if nd.glow > 0.003 {
                paint_orb(painter, p, rr, &self.colors, now, nd.glow);
            }
            if self.stroke || t.dark {
                painter.circle_stroke(p, rr, Stroke::new(1.0, with_alpha(t.border, 1.0 - nd.glow * 0.7)));
            }
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------
// LiquidToggle

#[derive(Clone, Copy)]
struct DropState {
    head: Spring,
    tail: Spring,
    auto_step: i32,
}

/// An on/off switch whose knob is a droplet: the leading edge springs across quickly while the
/// trailing edge lags behind, so the drop stretches mid-slide and snaps round at the end. The
/// track floods from light to ink as it turns on.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut on = false;
/// use glowkit_effects::LiquidToggle;
/// ui.add(LiquidToggle::new(&mut on).speed(50.0).stretch(36.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct LiquidToggle<'a> {
    on: &'a mut bool,
    size: Vec2,
    speed: f32,
    stretch: f32,
    stroke: bool,
    on_color: Option<Color32>,
    autoplay: bool,
}

impl<'a> LiquidToggle<'a> {
    /// A toggle editing `on`.
    pub fn new(on: &'a mut bool) -> Self {
        Self { on, size: vec2(92.0, 46.0), speed: 50.0, stretch: 36.0, stroke: false, on_color: None, autoplay: false }
    }
    /// Track size in points.
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = s;
        self
    }
    /// How fast the droplet travels, 0..100.
    pub fn speed(mut self, v: f32) -> Self {
        self.speed = v.clamp(0.0, 100.0);
        self
    }
    /// How far the droplet stretches mid-slide, 0..100.
    pub fn stretch(mut self, v: f32) -> Self {
        self.stretch = v.clamp(0.0, 100.0);
        self
    }
    /// Draw a hairline border on the track.
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Track color when on (default: theme foreground).
    pub fn on_color(mut self, c: Color32) -> Self {
        self.on_color = Some(c);
        self
    }
    /// Flip on and off by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for LiquidToggle<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (outer, outer_resp) = ui.allocate_exact_size(vec2(ui.available_width(), self.size.y + 8.0), Sense::hover());
        let rect = Rect::from_center_size(outer.center(), self.size);
        let mut response = ui.interact(rect, outer_resp.id.with("toggle"), Sense::click());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let idle = idle_for(ui, id, response.hovered());
        let mut st: DropState = glowkit_core::state(ui, id.with("drop"), || {
            let v = if *self.on { 1.0 } else { 0.0 };
            DropState { head: Spring::new(v), tail: Spring::new(v), auto_step: 0 }
        });
        if response.clicked() {
            *self.on = !*self.on;
            response.mark_changed();
        }
        if self.autoplay && !response.hovered() && idle > 0.8 {
            let k = ((idle - 0.8) / 1.5) as i32;
            if k != st.auto_step {
                st.auto_step = k;
                *self.on = !*self.on;
            }
        }
        let target = if *self.on { 1.0 } else { 0.0 };
        let stiff = lerp(90.0, 520.0, self.speed / 100.0);
        let lag = lerp(1.0, 0.18, self.stretch / 100.0);
        st.head.target = target;
        st.head.stiffness = stiff;
        st.head.damping = 2.0 * stiff.sqrt() * 0.62;
        st.tail.target = target;
        st.tail.stiffness = stiff * lag;
        st.tail.damping = 2.0 * (stiff * lag).sqrt() * 0.95;
        st.head.step(dt);
        st.tail.step(dt);
        glowkit_core::set_state(ui, id.with("drop"), st);
        if !st.head.is_settled() || !st.tail.is_settled() || self.autoplay {
            glowkit_core::animate(ui);
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let h = st.head.value;
        let tl = st.tail.value;
        let fillk = ((h + tl) / 2.0).clamp(0.0, 1.0);
        let on_col = self.on_color.unwrap_or(t.foreground);
        let off_col = surface(&t);
        let track_col = mix(off_col, on_col, ease(Easing::InOutSine, fillk));
        let radius = rect.height() / 2.0;
        if !t.dark {
            paint::soft_shadow(painter, rect, radius, 10.0, vec2(0.0, 2.0), Color32::from_rgba_unmultiplied(30, 30, 60, 12));
        }
        painter.rect_filled(rect, cr(radius), track_col);
        if self.stroke || t.dark {
            painter.rect_stroke(rect, cr(radius), Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        }
        let pad = rect.height() * 0.1;
        let kr = radius - pad;
        let x_of = |v: f32| lerp(rect.min.x + pad + kr, rect.max.x - pad - kr, v);
        let (x0, x1) = (x_of(h.min(tl)), x_of(h.max(tl)));
        let stretch = ((x1 - x0) / (rect.width() - 2.0 * radius)).clamp(0.0, 1.0);
        let ry = kr * (1.0 - 0.16 * stretch);
        let knob = Rect::from_min_max(pos2(x0 - kr, rect.center().y - ry), pos2(x1 + kr, rect.center().y + ry));
        let knob_col = mix(track_fill(&t), if t.dark { Color32::from_gray(245) } else { Color32::WHITE }, ease(Easing::InOutSine, fillk));
        painter.rect_filled(knob, cr(ry), knob_col);
        response
    }
}

// ---------------------------------------------------------------------------------------------
// TaskChecklist

/// One row of a [`TaskChecklist`].
#[derive(Clone, Debug, PartialEq)]
pub struct TodoItem {
    /// The task text.
    pub label: String,
    /// Whether it is ticked.
    pub done: bool,
}

impl TodoItem {
    /// An unticked task.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), done: false }
    }
    /// Start ticked.
    pub fn done(mut self, d: bool) -> Self {
        self.done = d;
        self
    }
}

/// A to-do card. Tick a task and the box fills with a little pop, the check mark draws itself
/// in and a line strikes through the words while they fade. An optional "add" row appends tasks.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{TaskChecklist, TodoItem};
/// let mut tasks = vec![TodoItem::new("Water the ferns"), TodoItem::new("Reply to Ana")];
/// ui.add(TaskChecklist::new(&mut tasks).bounce(50.0).corner(20.0).box_size(18.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TaskChecklist<'a> {
    items: &'a mut Vec<TodoItem>,
    width: f32,
    bounce: f32,
    corner: f32,
    box_size: f32,
    stroke: bool,
    add_label: Option<String>,
    max_items: usize,
    check_color: Option<Color32>,
    autoplay: bool,
}

impl<'a> TaskChecklist<'a> {
    /// A checklist over `items`.
    pub fn new(items: &'a mut Vec<TodoItem>) -> Self {
        Self {
            items,
            width: 300.0,
            bounce: 50.0,
            corner: 20.0,
            box_size: 18.0,
            stroke: false,
            add_label: Some("Add new task".into()),
            max_items: 6,
            check_color: None,
            autoplay: false,
        }
    }
    /// Card width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(120.0);
        self
    }
    /// Pop of the box when ticked, 0..100.
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 100.0);
        self
    }
    /// Card corner radius in points.
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// Checkbox size in points.
    pub fn box_size(mut self, s: f32) -> Self {
        self.box_size = s.clamp(10.0, 32.0);
        self
    }
    /// Draw a hairline border around the card.
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Label of the trailing "add" row (`None` hides it).
    pub fn add_label(mut self, s: Option<String>) -> Self {
        self.add_label = s;
        self
    }
    /// Maximum number of tasks the add row will create.
    pub fn max_items(mut self, n: usize) -> Self {
        self.max_items = n;
        self
    }
    /// Fill color of ticked boxes (default: theme foreground).
    pub fn check_color(mut self, c: Color32) -> Self {
        self.check_color = Some(c);
        self
    }
    /// Tick and untick tasks by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for TaskChecklist<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let row_h = (self.box_size + 22.0).max(38.0);
        let pad = 12.0;
        let rows = self.items.len() + usize::from(self.add_label.is_some());
        let h = rows as f32 * row_h + pad * 2.0 - 8.0;
        let (outer, response) = ui.allocate_exact_size(vec2(ui.available_width(), h + 10.0), Sense::hover());
        let rect = Rect::from_center_size(outer.center(), vec2(self.width.min(outer.width()), h));
        let id = response.id;
        let card = ui.interact(rect, id.with("card"), Sense::hover());
        let idle = idle_for(ui, id, card.hovered());
        if self.autoplay && !card.hovered() && idle > 0.8 && !self.items.is_empty() {
            let k = ((idle - 0.8) / 1.1) as i32;
            let key = id.with("auto_step");
            let last: i32 = ui.data(|d| d.get_temp(key)).unwrap_or(-1);
            if k != last {
                ui.data_mut(|d| d.insert_temp(key, k));
                let n = self.items.len();
                let i = (k as usize) % n;
                // tick down the list, then untick in the same order
                let phase = (k as usize / n) % 2;
                self.items[i].done = phase == 0;
            }
        }
        let font = FontId::proportional(14.5);
        let ink = self.check_color.unwrap_or(t.foreground);
        let mut add_clicked = false;
        let mut rows_geo = Vec::new();
        for i in 0..rows {
            let r = Rect::from_min_size(pos2(rect.min.x + 6.0, rect.min.y + pad - 4.0 + i as f32 * row_h), vec2(rect.width() - 12.0, row_h));
            let resp = ui.interact(r, id.with(("row", i)), Sense::click());
            if resp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if resp.clicked() {
                if i < self.items.len() {
                    self.items[i].done = !self.items[i].done;
                } else {
                    add_clicked = true;
                }
            }
            rows_geo.push((r, resp.hovered()));
        }
        if add_clicked && self.items.len() < self.max_items {
            let n = self.items.len() + 1;
            self.items.push(TodoItem::new(format!("New task {n}")));
        }
        let stiff = 320.0;
        let damp = bounce_damping(self.bounce, stiff);
        let anims: Vec<(f32, f32, f32)> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, it)| {
                let target = if it.done { 1.0 } else { 0.0 };
                let pop = glowkit_core::anim::spring(ui, id.with(("pop", i)), target, stiff, damp);
                let tick = glowkit_core::anim::follow(ui, id.with(("tick", i)), target, 7.0);
                let strike = glowkit_core::anim::follow(ui, id.with(("strike", i)), target, 5.0);
                (pop, tick, strike)
            })
            .collect();
        if self.autoplay {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        soft_card(painter, &t, rect, self.corner, 1.0, self.stroke);
        let bs = self.box_size;
        for (i, (r, hovered)) in rows_geo.iter().enumerate() {
            if *hovered {
                painter.rect_filled(r.shrink2(vec2(0.0, 2.0)), cr((self.corner - 6.0).max(4.0)), with_alpha(track(&t), 0.6));
            }
            let bc = pos2(r.min.x + 10.0 + bs / 2.0, r.center().y);
            let text_x = bc.x + bs / 2.0 + 12.0;
            if i >= self.items.len() {
                let br = Rect::from_center_size(bc, Vec2::splat(bs));
                painter.rect_stroke(br, cr(bs * 0.28), Stroke::new(1.3, with_alpha(t.muted_foreground, 0.35)), egui::StrokeKind::Inside);
                painter.text(pos2(text_x, r.center().y), Align2::LEFT_CENTER, self.add_label.as_deref().unwrap_or(""), font.clone(), with_alpha(t.muted_foreground, 0.7));
                continue;
            }
            let (pop, tick, strike) = anims[i];
            let fill = pop.clamp(0.0, 1.0);
            let over = (pop - 1.0).abs().min(0.4) * if pop > 0.02 { 1.0 } else { 0.0 };
            let scale = 1.0 + (pop - pop.clamp(0.0, 1.0)).abs() * 0.8 + over * 0.15 * (1.0 - fill).max(0.0);
            let br = Rect::from_center_size(bc, Vec2::splat(bs * scale));
            painter.rect_stroke(br, cr(bs * 0.28), Stroke::new(1.3, mix(with_alpha(t.muted_foreground, 0.6), ink, fill)), egui::StrokeKind::Inside);
            if fill > 0.01 {
                let fr = Rect::from_center_size(bc, Vec2::splat(bs * scale * (0.4 + 0.6 * fill)));
                painter.rect_filled(fr, cr(bs * 0.28 * scale), with_alpha(ink, fill));
            }
            let tk = ((tick - 0.15) / 0.85).clamp(0.0, 1.0);
            if tk > 0.0 {
                let p = |u: f32, v: f32| pos2(br.min.x + u * br.width(), br.min.y + v * br.height());
                let pts = [p(0.27, 0.52), p(0.43, 0.68), p(0.74, 0.34)];
                let l1 = pts[0].distance(pts[1]);
                let l2 = pts[1].distance(pts[2]);
                let len = (l1 + l2) * ease(Easing::OutCubic, tk);
                let mut line = vec![pts[0]];
                if len <= l1 {
                    line.push(pts[0] + (pts[1] - pts[0]) * (len / l1));
                } else {
                    line.push(pts[1]);
                    line.push(pts[1] + (pts[2] - pts[1]) * ((len - l1) / l2));
                }
                let tick_col = glowkit_core::color::contrast_text(ink);
                painter.add(Shape::line(line, Stroke::new((bs * 0.1).max(1.6), tick_col)));
            }
            let label = &self.items[i].label;
            let text_col = mix(t.foreground, t.muted_foreground, strike * 0.85);
            let g = painter.layout_no_wrap(label.clone(), font.clone(), text_col);
            let tw = g.size().x;
            painter.galley(pos2(text_x, r.center().y - g.size().y / 2.0), g, text_col);
            let sk = ease(Easing::InOutCubic, strike);
            if sk > 0.01 {
                let y = r.center().y + 0.5;
                painter.line_segment([pos2(text_x - 1.0, y), pos2(text_x - 1.0 + (tw + 2.0) * sk, y)], Stroke::new(1.3, text_col));
            }
        }
        response
    }
}
