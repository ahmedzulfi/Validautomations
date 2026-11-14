//! Physical sliders: a thumbless bar range with a springy wake, a slider whose fill sloshes
//! like liquid, and a flingable time ruler.

use super::common::{cr, idle_for, soft_card, surface, track, track_fill};
use egui::epaint::Mesh;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Rect, Response, Sense, Shape, Stroke, Ui};
use glowkit_core::anim::{ease, lerp, Easing, Spring};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

fn sweep(now: f32, t0: f32, from: f32, to: f32, dur: f32) -> f32 {
    lerp(from, to, ease(Easing::InOutCubic, ((now - t0) / dur).clamp(0.0, 1.0)))
}

/// A scripted autoplay "hand": glides between a few stops with quick sweeps. Returns 0..1.
fn auto_hand(idle: f32, stops: &[f32], hold: f32, sweep_dur: f32) -> f32 {
    let per = hold + sweep_dur;
    let k = (idle / per) as usize;
    let p = idle - k as f32 * per;
    let a = stops[k % stops.len()];
    let b = stops[(k + 1) % stops.len()];
    if p < hold {
        a
    } else {
        sweep(p, hold, a, b, sweep_dur)
    }
}

// ---------------------------------------------------------------------------------------------
// WakeSlider

#[derive(Clone, Default)]
struct WakeState {
    bars: Vec<Spring>,
    last_x: Option<f32>,
}

/// A range input made of thin vertical bars and no thumb. Bars left of the value are inked,
/// the rest are faint; drag fast and every bar you sweep past springs up in a wake behind the
/// hand, then settles.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut volume = 50.0_f32;
/// use glowkit_effects::WakeSlider;
/// ui.add(WakeSlider::new(&mut volume).bars(36).wake(60.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct WakeSlider<'a> {
    value: &'a mut f32,
    range: (f32, f32),
    bars: usize,
    wake: f32,
    width: f32,
    bar_height: f32,
    show_value: bool,
    color: Option<Color32>,
    autoplay: bool,
}

impl<'a> WakeSlider<'a> {
    /// A slider editing `value` in `0..=100`.
    pub fn new(value: &'a mut f32) -> Self {
        Self { value, range: (0.0, 100.0), bars: 36, wake: 60.0, width: 260.0, bar_height: 20.0, show_value: true, color: None, autoplay: false }
    }
    /// Value range.
    pub fn range(mut self, lo: f32, hi: f32) -> Self {
        self.range = (lo, hi);
        self
    }
    /// Number of bars (4..=120).
    pub fn bars(mut self, n: usize) -> Self {
        self.bars = n.clamp(4, 120);
        self
    }
    /// How strongly a fast sweep kicks the bars up, 0..100.
    pub fn wake(mut self, w: f32) -> Self {
        self.wake = w.clamp(0.0, 100.0);
        self
    }
    /// Width of the bar row in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(40.0);
        self
    }
    /// Resting bar height in points.
    pub fn bar_height(mut self, h: f32) -> Self {
        self.bar_height = h.max(4.0);
        self
    }
    /// Show the numeric value to the right (default on).
    pub fn show_value(mut self, on: bool) -> Self {
        self.show_value = on;
        self
    }
    /// Ink color of the active bars (default: theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Sweep back and forth on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for WakeSlider<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let label_w = if self.show_value { 44.0 } else { 0.0 };
        let h = self.bar_height * 2.4;
        let (outer, mut response) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::click_and_drag());
        let rect = Rect::from_center_size(outer.center() - vec2(label_w / 2.0, 0.0), vec2(self.width, h));
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let n = self.bars;
        let (lo, hi) = self.range;
        let interacting = response.dragged() || response.is_pointer_button_down_on();
        let idle = idle_for(ui, id, interacting || response.hovered());
        let to_frac = |x: f32| ((x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
        let mut hand = None;
        if interacting {
            if let Some(p) = response.interact_pointer_pos() {
                hand = Some(p.x);
            }
        } else if self.autoplay && idle > 1.0 {
            let f = auto_hand(idle - 1.0, &[0.5, 0.92, 0.18, 0.7, 0.35], 0.9, 0.32);
            hand = Some(rect.min.x + f * rect.width());
        }
        if let Some(x) = hand {
            let v = lo + to_frac(x) * (hi - lo);
            if (v - *self.value).abs() > 1e-4 {
                *self.value = v;
                response.mark_changed();
            }
        }
        let hover_x = response.hover_pos().map(|p| p.x);
        let wake = self.wake / 100.0;
        let step = rect.width() / n as f32;
        let mut st: WakeState = glowkit_core::state(ui, id.with("wake"), WakeState::default);
        if st.bars.len() != n {
            st.bars = (0..n).map(|_| Spring::new(0.0).with(260.0, 9.0)).collect();
        }
        if let (Some(x), Some(px)) = (hand, st.last_x) {
            let speed = (x - px).abs() / dt.max(1e-3);
            if speed > 40.0 {
                let (a, b) = if x < px { (x, px) } else { (px, x) };
                for (i, s) in st.bars.iter_mut().enumerate() {
                    let bx = rect.min.x + (i as f32 + 0.5) * step;
                    if bx >= a - step && bx <= b + step {
                        s.velocity += (speed / 1000.0).min(4.0) * wake * 9.0;
                    }
                }
            }
        }
        st.last_x = hand;
        let mut settled = true;
        for s in st.bars.iter_mut() {
            s.step(dt);
            s.value = s.value.clamp(-0.3, 1.6);
            settled &= s.is_settled();
        }
        let heights: Vec<f32> = st.bars.iter().map(|s| s.value).collect();
        glowkit_core::set_state(ui, id.with("wake"), st);
        if !settled || self.autoplay {
            glowkit_core::animate(ui);
        }
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(outer) {
            return response;
        }
        let painter = ui.painter();
        let frac = ((*self.value - lo) / (hi - lo).max(1e-6)).clamp(0.0, 1.0);
        let ink = self.color.unwrap_or(t.foreground);
        let faint = mix(track_fill(&t), t.foreground, 0.12);
        let bw = (step * 0.42).clamp(1.5, 4.0);
        for (i, e) in heights.iter().enumerate() {
            let bx = rect.min.x + (i as f32 + 0.5) * step;
            let near = hover_x.map(|x| (1.0 - (x - bx).abs() / (step * 3.0)).max(0.0) * 0.18).unwrap_or(0.0);
            let bh = self.bar_height * (1.0 + e.max(-0.2) * 0.9 + near);
            let on = (i as f32 + 0.5) / n as f32 <= frac;
            let c = if on { ink } else { faint };
            let r = Rect::from_center_size(pos2(bx, rect.center().y), vec2(bw, bh));
            painter.rect_filled(r, cr(bw / 2.0), c);
        }
        if self.show_value {
            painter.text(
                pos2(rect.max.x + 14.0, rect.center().y),
                Align2::LEFT_CENTER,
                format!("{}", self.value.round() as i64),
                FontId::proportional(14.0),
                mix(t.muted_foreground, t.foreground, 0.5),
            );
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------
// SloshSlider

#[derive(Clone, Copy)]
struct SloshState {
    edge: Spring,
    last_hand: f32,
}

/// A slider whose fill has weight. The fill is a liquid tied to the handle by a spring: drag
/// fast and it lags, leans and then sloshes past the handle before settling level.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut level = 0.6_f32;
/// use glowkit_effects::SloshSlider;
/// ui.add(SloshSlider::new(&mut level).viscosity(15.0).momentum(55.0).tilt(45.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SloshSlider<'a> {
    value: &'a mut f32,
    size: egui::Vec2,
    viscosity: f32,
    momentum: f32,
    corner: f32,
    tilt: f32,
    stroke: bool,
    fill: Option<Color32>,
    autoplay: bool,
}

impl<'a> SloshSlider<'a> {
    /// A slider editing `value` in `0..=1`.
    pub fn new(value: &'a mut f32) -> Self {
        Self {
            value,
            size: vec2(320.0, 40.0),
            viscosity: 15.0,
            momentum: 55.0,
            corner: 20.0,
            tilt: 45.0,
            stroke: false,
            fill: None,
            autoplay: false,
        }
    }
    /// Size in points.
    pub fn size(mut self, s: egui::Vec2) -> Self {
        self.size = s;
        self
    }
    /// How thick the liquid is (more = settles faster), 0..100.
    pub fn viscosity(mut self, v: f32) -> Self {
        self.viscosity = v.clamp(0.0, 100.0);
        self
    }
    /// How far the liquid carries past the handle, 0..100.
    pub fn momentum(mut self, v: f32) -> Self {
        self.momentum = v.clamp(0.0, 100.0);
        self
    }
    /// Corner radius in points.
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// How much the liquid surface leans while moving, 0..100.
    pub fn tilt(mut self, v: f32) -> Self {
        self.tilt = v.clamp(0.0, 100.0);
        self
    }
    /// Draw a hairline border.
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Liquid color (default: the card surface color).
    pub fn fill(mut self, c: Color32) -> Self {
        self.fill = Some(c);
        self
    }
    /// Slosh back and forth on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

/// Horizontal extent of a rounded rect at height `y`.
fn rounded_span(rect: Rect, r: f32, y: f32) -> (f32, f32) {
    let r = r.min(rect.height() / 2.0).min(rect.width() / 2.0);
    let dy = if y < rect.min.y + r {
        rect.min.y + r - y
    } else if y > rect.max.y - r {
        y - (rect.max.y - r)
    } else {
        0.0
    };
    let inset = r - (r * r - dy.min(r) * dy.min(r)).max(0.0).sqrt();
    (rect.min.x + inset, rect.max.x - inset)
}

impl egui::Widget for SloshSlider<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (outer, mut response) = ui.allocate_exact_size(vec2(ui.available_width(), self.size.y + 16.0), Sense::click_and_drag());
        let rect = Rect::from_center_size(outer.center(), self.size);
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let interacting = response.dragged() || response.is_pointer_button_down_on();
        let idle = idle_for(ui, id, interacting || response.hovered());
        let pad = 8.0;
        let x_of = |v: f32| rect.min.x + pad + v.clamp(0.0, 1.0) * (rect.width() - 2.0 * pad);
        if interacting {
            if let Some(p) = response.interact_pointer_pos() {
                let v = ((p.x - rect.min.x - pad) / (rect.width() - 2.0 * pad)).clamp(0.0, 1.0);
                if (v - *self.value).abs() > 1e-5 {
                    *self.value = v;
                    response.mark_changed();
                }
            }
        } else if self.autoplay && idle > 1.0 {
            *self.value = auto_hand(idle - 1.0, &[0.62, 0.08, 0.88, 0.3, 0.95, 0.45], 1.0, 0.28);
        }
        let hand = x_of(*self.value);
        let stiffness = lerp(260.0, 90.0, self.momentum / 100.0);
        let damping = 2.0 * stiffness.sqrt() * lerp(0.12, 0.9, self.viscosity / 100.0);
        let mut st: SloshState = glowkit_core::state(ui, id.with("slosh"), || SloshState { edge: Spring::new(hand), last_hand: hand });
        st.edge.target = hand;
        st.edge.stiffness = stiffness;
        st.edge.damping = damping;
        st.edge.step(dt);
        st.last_hand = hand;
        glowkit_core::set_state(ui, id.with("slosh"), st);
        let edge = st.edge.value;
        let vel = st.edge.velocity;
        if !st.edge.is_settled() || self.autoplay {
            glowkit_core::animate(ui);
        }
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let radius = self.corner.min(rect.height() / 2.0);
        painter.rect_filled(rect, cr(radius), track(&t));
        // liquid: surface leans with velocity and bulges forward
        let lean = (vel * 0.02 * self.tilt / 45.0).clamp(-40.0, 40.0);
        let bulge = (edge - hand).clamp(-60.0, 60.0) * 0.35;
        let fill = self.fill.unwrap_or_else(|| surface(&t));
        let rows = 28;
        let mut m = Mesh::default();
        for k in 0..=rows {
            let y = rect.min.y + rect.height() * k as f32 / rows as f32;
            let yn = (y - rect.center().y) / (rect.height() / 2.0);
            let x = edge - lean * yn + bulge * (1.0 - yn * yn) * 0.6;
            let (l, r) = rounded_span(rect, radius, y);
            let xr = x.clamp(l, r);
            m.colored_vertex(pos2(l, y), fill);
            m.colored_vertex(pos2(xr, y), fill);
            // feather edge for anti-aliasing
            m.colored_vertex(pos2((xr + 1.0).min(r), y), with_alpha(fill, 0.0));
        }
        for k in 0..rows as u32 {
            let a = k * 3;
            let b = a + 3;
            m.add_triangle(a, a + 1, b + 1);
            m.add_triangle(a, b + 1, b);
            m.add_triangle(a + 1, a + 2, b + 2);
            m.add_triangle(a + 1, b + 2, b + 1);
        }
        painter.add(Shape::mesh(m));
        if self.stroke || t.dark {
            painter.rect_stroke(rect, cr(radius), Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        }
        let hr = Rect::from_center_size(pos2(hand, rect.center().y), vec2(3.0, rect.height() * 0.62));
        painter.rect_filled(hr, cr(1.5), t.foreground);
        response
    }
}

// ---------------------------------------------------------------------------------------------
// TimeScrubber

#[derive(Clone)]
struct ScrubState {
    pos: f32,
    vel: f32,
    snap: Spring,
    snapping: bool,
    shown: String,
    prev: String,
    changed_at: f32,
    last_value: u32,
}

/// A time picker made of a horizontal ruler: drag the ruler under the red needle to set a
/// time, or fling it and it coasts with momentum before springing to the nearest step. The
/// digits roll as they change.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut minutes: u32 = 9 * 60 + 30;
/// use glowkit_effects::TimeScrubber;
/// ui.add(TimeScrubber::new(&mut minutes).step(15).twenty_four_hour(false));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TimeScrubber<'a> {
    minutes: &'a mut u32,
    step: u32,
    h24: bool,
    momentum: f32,
    corner: f32,
    stroke: bool,
    size: egui::Vec2,
    needle: Option<Color32>,
    autoplay: bool,
}

impl<'a> TimeScrubber<'a> {
    /// A scrubber editing `minutes` since midnight (`0..1440`).
    pub fn new(minutes: &'a mut u32) -> Self {
        Self {
            minutes,
            step: 15,
            h24: false,
            momentum: 50.0,
            corner: 20.0,
            stroke: false,
            size: vec2(300.0, 140.0),
            needle: None,
            autoplay: false,
        }
    }
    /// Snap step in minutes (e.g. 5, 15, 30).
    pub fn step(mut self, minutes: u32) -> Self {
        self.step = minutes.clamp(1, 120);
        self
    }
    /// Use a 24-hour clock instead of AM / PM.
    pub fn twenty_four_hour(mut self, on: bool) -> Self {
        self.h24 = on;
        self
    }
    /// How far a fling coasts, 0..100.
    pub fn momentum(mut self, m: f32) -> Self {
        self.momentum = m.clamp(0.0, 100.0);
        self
    }
    /// Corner radius in points.
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// Draw a hairline border.
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Card size in points.
    pub fn size(mut self, s: egui::Vec2) -> Self {
        self.size = s;
        self
    }
    /// Needle color (default: the theme's destructive red).
    pub fn needle(mut self, c: Color32) -> Self {
        self.needle = Some(c);
        self
    }
    /// Fling the ruler now and then while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn fmt_time(m: i64, h24: bool) -> (String, &'static str) {
    let m = m.rem_euclid(1440);
    let (h, mm) = (m / 60, m % 60);
    if h24 {
        (format!("{h:02}:{mm:02}"), "")
    } else {
        let h12 = if h % 12 == 0 { 12 } else { h % 12 };
        (format!("{h12}:{mm:02}"), if h < 12 { "AM" } else { "PM" })
    }
}

fn hour_label(h: i64, h24: bool) -> String {
    let h = h.rem_euclid(24);
    if h24 {
        format!("{h:02}:00")
    } else {
        let h12 = if h % 12 == 0 { 12 } else { h % 12 };
        format!("{h12} {}", if h < 12 { "AM" } else { "PM" })
    }
}

impl egui::Widget for TimeScrubber<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (outer, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.size.y + 10.0), Sense::hover());
        let rect = Rect::from_center_size(outer.center(), self.size);
        let id = response.id;
        let ruler = Rect::from_min_max(pos2(rect.min.x + 16.0, rect.min.y + 66.0), pos2(rect.max.x - 16.0, rect.max.y - 14.0));
        let resp = ui.interact(ruler.expand2(vec2(0.0, 12.0)), id.with("ruler"), Sense::drag());
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let px_per_min = 2.0;
        let step = self.step as f32;
        let friction = lerp(9.0, 1.4, self.momentum / 100.0);
        let dragging = resp.dragged();
        let released = resp.drag_stopped();
        let drag_dx = resp.drag_delta().x;
        let pointer_vx = ui.input(|i| i.pointer.velocity().x);
        let idle = idle_for(ui, id, dragging || resp.hovered());
        let ext = *self.minutes;
        let mut st: ScrubState = glowkit_core::state(ui, id.with("scrub"), || ScrubState {
            pos: ext as f32,
            vel: 0.0,
            snap: Spring::new(ext as f32),
            snapping: false,
            shown: String::new(),
            prev: String::new(),
            changed_at: -10.0,
            last_value: ext,
        });
        if ext != st.last_value && !dragging {
            st.pos = ext as f32;
            st.vel = 0.0;
            st.snapping = false;
        }
        // autoplay: a fling every few seconds
        if self.autoplay && !dragging && idle > 1.5 {
            let k = ((idle - 1.5) / 3.2) as i32;
            let key = id.with(("fling", k));
            let fired: bool = ui.data(|d| d.get_temp(key)).unwrap_or(false);
            if !fired {
                ui.data_mut(|d| d.insert_temp(key, true));
                let dir = if k % 3 == 1 { -1.0 } else { 1.0 };
                st.vel = dir * (60.0 + 30.0 * ((k % 4) as f32));
                st.snapping = false;
            }
        }
        if dragging {
            st.pos -= drag_dx / px_per_min;
            st.vel = 0.0;
            st.snapping = false;
        } else {
            if released {
                st.vel = -pointer_vx / px_per_min;
            }
            if !st.snapping {
                st.pos += st.vel * dt;
                st.vel *= (-friction * dt).exp();
                if st.vel.abs() < step * 0.9 {
                    let target = (st.pos / step).round() * step;
                    st.snap = Spring::new(st.pos).with(200.0, 22.0);
                    st.snap.velocity = st.vel;
                    st.snap.target = target;
                    st.snapping = true;
                }
            } else {
                st.snap.step(dt);
                st.pos = st.snap.value;
            }
        }
        let moving = dragging || !st.snapping || !st.snap.is_settled();
        let shown_min = st.pos.round() as i64;
        let (txt, ampm) = fmt_time(shown_min, self.h24);
        if txt != st.shown {
            st.prev = std::mem::replace(&mut st.shown, txt.clone());
            st.changed_at = now;
        }
        if st.snapping && st.snap.is_settled() {
            let v = (st.snap.target as i64).rem_euclid(1440) as u32;
            *self.minutes = v;
        }
        st.last_value = *self.minutes;
        let pos = st.pos;
        let (prev, changed_at) = (st.prev.clone(), st.changed_at);
        glowkit_core::set_state(ui, id.with("scrub"), st);
        let roll = ((now - changed_at) / 0.16).clamp(0.0, 1.0);
        if moving || roll < 1.0 || self.autoplay {
            glowkit_core::animate(ui);
        }
        if resp.hovered() || dragging {
            ui.ctx().set_cursor_icon(if dragging { CursorIcon::Grabbing } else { CursorIcon::Grab });
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter();
        soft_card(painter, &t, rect, self.corner, 1.0, self.stroke);
        // digits: each changed glyph rolls up
        let font = FontId::proportional(32.0);
        let small = FontId::proportional(14.0);
        let chars: Vec<char> = txt.chars().collect();
        let pchars: Vec<char> = prev.chars().collect();
        let widths: Vec<f32> = chars.iter().map(|c| painter.layout_no_wrap(c.to_string(), font.clone(), t.foreground).size().x).collect();
        let total: f32 = widths.iter().sum();
        let ampm_w = if ampm.is_empty() { 0.0 } else { painter.layout_no_wrap(ampm.into(), small.clone(), t.foreground).size().x + 6.0 };
        let mut x = rect.center().x - (total + ampm_w) / 2.0;
        let base_y = rect.min.y + 36.0;
        let e = ease(Easing::OutCubic, roll);
        for (i, c) in chars.iter().enumerate() {
            let cx = x + widths[i] / 2.0;
            let changed = pchars.get(i) != Some(c) && roll < 1.0;
            if changed {
                painter.text(pos2(cx, base_y + (1.0 - e) * 12.0 ), Align2::CENTER_CENTER, c.to_string(), font.clone(), with_alpha(t.foreground, e));
                if let Some(pc) = pchars.get(i) {
                    painter.text(pos2(cx, base_y - e * 12.0 ), Align2::CENTER_CENTER, pc.to_string(), font.clone(), with_alpha(t.foreground, 1.0 - e));
                }
            } else {
                painter.text(pos2(cx, base_y), Align2::CENTER_CENTER, c.to_string(), font.clone(), t.foreground);
            }
            x += widths[i];
        }
        if !ampm.is_empty() {
            painter.text(pos2(x + 6.0, base_y + 5.0), Align2::LEFT_CENTER, ampm, small, t.muted_foreground);
        }
        // ruler
        let cx = ruler.center().x;
        let tick_y = ruler.min.y + 8.0;
        let half_span = ruler.width() / 2.0 / px_per_min;
        let first = ((pos - half_span) / 5.0).floor() as i64 * 5;
        let last = ((pos + half_span) / 5.0).ceil() as i64 * 5;
        let mut m = first;
        while m <= last {
            let x = cx + (m as f32 - pos) * px_per_min;
            let d = ((x - cx).abs() / (ruler.width() / 2.0)).clamp(0.0, 1.0);
            let fade = 1.0 - d.powf(2.5);
            let (h, w, strong) = if m.rem_euclid(60) == 0 {
                (20.0, 1.6, true)
            } else if m.rem_euclid(30) == 0 {
                (14.0, 1.4, true)
            } else {
                (9.0, 1.2, false)
            };
            let c = if strong { t.foreground } else { mix(t.muted_foreground, t.foreground, 0.4) };
            painter.line_segment([pos2(x, tick_y), pos2(x, tick_y + h)], Stroke::new(w, with_alpha(c, fade)));
            if m.rem_euclid(60) == 0 {
                painter.text(
                    pos2(x, tick_y + 34.0),
                    Align2::CENTER_CENTER,
                    hour_label(m.div_euclid(60), self.h24),
                    FontId::proportional(11.0),
                    with_alpha(t.muted_foreground, fade),
                );
            }
            m += 5;
        }
        let needle = self.needle.unwrap_or(t.destructive);
        let nr = Rect::from_center_size(pos2(cx, tick_y + 11.0), vec2(2.4, 30.0));
        painter.rect_filled(nr, cr(1.2), needle);
        resp
    }
}
