use crate::util;
use egui::{vec2, Color32, Pos2, Rect, Response, Sense, Ui, Vec2, Widget};
use glowkit_core::{ease, Easing, Rng, Theme, Trigger};

#[derive(Clone, Copy, Debug)]
struct Body {
    pos: Pos2, // center, relative to the widget's top-left
    vel: Vec2,
    angle: f32,
    spin: f32,
    half: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Resting,
    Falling,
    Returning { from_t: f64 },
}

#[derive(Clone, Debug)]
struct Sim {
    phase: Phase,
    bodies: Vec<Body>,
    snapshot: Vec<Body>,
    phase_t: f64,
    key: u64,
    appeared: bool,
}

/// Inner margin of the box the words fall into.
const PAD: f32 = 8.0;

/// One fixed physics step: gravity, walls, floor and simple box-box pushing.
fn step(bodies: &mut [Body], size: Vec2, gravity: f32, h: f32) {
    for b in bodies.iter_mut() {
        b.vel.y += gravity * h;
        b.vel *= 0.999;
        b.spin *= 1.0 - 0.6 * h;
        b.pos += b.vel * h;
        b.angle += b.spin * h;
        let floor = size.y - b.half.y - PAD;
        if b.pos.y > floor {
            b.pos.y = floor;
            if b.vel.y > 60.0 {
                // impact: bounce and lose most of the tumble
                b.vel.y = -b.vel.y * 0.32;
                b.angle *= 0.6;
                b.spin *= 0.4;
            } else if b.vel.y > 0.0 {
                b.vel.y = 0.0;
            }
            b.vel.x *= 0.92;
            b.spin *= 0.85;
        }
        if b.pos.x < b.half.x + PAD {
            b.pos.x = b.half.x + PAD;
            b.vel.x = b.vel.x.abs() * 0.5;
        }
        if b.pos.x > size.x - b.half.x - PAD {
            b.pos.x = size.x - b.half.x - PAD;
            b.vel.x = -b.vel.x.abs() * 0.5;
        }
    }
    for _ in 0..3 {
        for i in 0..bodies.len() {
            for j in i + 1..bodies.len() {
                let (a, b) = (bodies[i], bodies[j]);
                let d = b.pos - a.pos;
                let ox = a.half.x + b.half.x - d.x.abs();
                let oy = a.half.y + b.half.y - d.y.abs();
                if ox <= 0.0 || oy <= 0.0 {
                    continue;
                }
                if oy < ox {
                    // stack vertically: the upper one rests on the lower one
                    let s = d.y.signum();
                    let (upper, lower) = if s > 0.0 { (i, j) } else { (j, i) };
                    let lower_grounded = bodies[lower].pos.y >= size.y - bodies[lower].half.y - PAD - 0.5;
                    let (ku, kl) = if lower_grounded { (1.0, 0.0) } else { (0.5, 0.5) };
                    bodies[upper].pos.y -= oy * ku;
                    bodies[lower].pos.y += oy * kl;
                    let rel = bodies[upper].vel.y - bodies[lower].vel.y;
                    if rel > 0.0 {
                        bodies[upper].vel.y -= rel * if lower_grounded { 1.0 } else { 0.5 };
                        bodies[lower].vel.y += rel * if lower_grounded { 0.0 } else { 0.5 };
                    }
                    bodies[upper].vel.x *= 0.96;
                    bodies[upper].spin *= 0.9;
                } else {
                    let s = d.x.signum();
                    bodies[i].pos.x -= s * ox * 0.5;
                    bodies[j].pos.x += s * ox * 0.5;
                    let rel = (bodies[j].vel.x - bodies[i].vel.x) * s;
                    if rel < 0.0 {
                        bodies[i].vel.x += rel * s * 0.5;
                        bodies[j].vel.x -= rel * s * 0.5;
                    }
                }
            }
        }
    }
}

/// A paragraph whose words drop under gravity when clicked, tumble, bounce and pile up on the
/// floor. Drag through the pile to shove words around; click again to send them home.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::FallingText::new("Click me and watch the words fall").highlight(&["words", "fall"]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct FallingText {
    text: String,
    size: f32,
    color: Option<Color32>,
    highlight_color: Option<Color32>,
    highlight: Vec<String>,
    bold: bool,
    height: f32,
    width: Option<f32>,
    gravity: f32,
    trigger: Trigger,
    background: bool,
}

impl FallingText {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 30.0,
            color: None,
            highlight_color: None,
            highlight: Vec::new(),
            bold: true,
            height: 280.0,
            width: None,
            gravity: 1.0,
            trigger: Trigger::OnClick,
            background: true,
        }
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Text color (defaults to the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Words to draw in the highlight color (matched ignoring punctuation and case).
    pub fn highlight(mut self, words: &[&str]) -> Self {
        self.highlight = words.iter().map(|w| w.to_lowercase()).collect();
        self
    }
    /// Highlight color (defaults to the theme accent).
    pub fn highlight_color(mut self, c: Color32) -> Self {
        self.highlight_color = Some(c);
        self
    }
    /// Fake-bold the text (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Height of the box the words fall into.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(40.0);
        self
    }
    /// Width of the box (defaults to the available width).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Gravity multiplier.
    pub fn gravity(mut self, g: f32) -> Self {
        self.gravity = g.max(0.0);
        self
    }
    /// Draw a subtle rounded container behind the words (default on).
    pub fn background(mut self, on: bool) -> Self {
        self.background = on;
        self
    }
    /// `OnClick` (default) toggles fall / reset; `OnHover` drops on hover; `OnAppear` drops once;
    /// `Loop` drops, rests and resets on its own.
    pub fn trigger(mut self, t: Trigger) -> Self {
        self.trigger = t;
        self
    }
}

impl Widget for FallingText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let w = self.width.unwrap_or(ui.available_width());
        let (rect, resp) = ui.allocate_exact_size(vec2(w, self.height), Sense::click_and_drag());
        let painter = ui.painter().clone();
        let wrap = (w * 0.86).max(100.0);
        let glyphs = crate::util::glyphs(&painter, &self.text, font.clone(), Some(wrap));
        // words: text + home rect
        let mut words: Vec<(String, Rect)> = Vec::new();
        for g in &glyphs.glyphs {
            if g.ch.is_whitespace() {
                continue;
            }
            let r = Rect::from_min_size(g.offset.to_pos2(), g.size);
            match words.get_mut(g.word) {
                Some((s, wr)) => {
                    s.push(g.ch);
                    *wr = wr.union(r);
                }
                None => {
                    while words.len() < g.word {
                        words.push((String::new(), Rect::NOTHING));
                    }
                    words.push((g.ch.to_string(), r));
                }
            }
        }
        words.retain(|(s, _)| !s.is_empty());
        let top = (self.height * 0.18).min((self.height - glyphs.size.y) / 2.0).max(0.0);
        let home_off = vec2((w - glyphs.size.x) / 2.0, top);
        let homes: Vec<Body> = words
            .iter()
            .map(|(_, r)| Body { pos: r.center() + home_off, vel: Vec2::ZERO, angle: 0.0, spin: 0.0, half: vec2(r.width() + bold + 6.0, r.height() * 0.8) / 2.0 })
            .collect();
        let key = words.len() as u64 ^ ((w * 10.0) as u64) << 16 ^ (self.size.to_bits() as u64) << 32;
        let now = ui.input(|i| i.time);
        let id = resp.id.with("falling");
        let mut sim = glowkit_core::state(ui, id, || Sim { phase: Phase::Resting, bodies: homes.clone(), snapshot: Vec::new(), phase_t: now, key, appeared: false });
        if sim.key != key || sim.bodies.len() != homes.len() {
            sim = Sim { phase: Phase::Resting, bodies: homes.clone(), snapshot: Vec::new(), phase_t: now, key, appeared: false };
        }
        let since = (now - sim.phase_t) as f32;
        let drop = |sim: &mut Sim| {
            let mut r = Rng::new(now.to_bits());
            sim.bodies = homes.clone();
            for b in &mut sim.bodies {
                b.vel = vec2(r.range(-260.0, 260.0), r.range(-420.0, 40.0));
                b.spin = r.range(-4.0, 4.0);
            }
            sim.phase = Phase::Falling;
            sim.phase_t = now;
            sim.appeared = true;
        };
        let reset = |sim: &mut Sim| {
            sim.snapshot = sim.bodies.clone();
            sim.phase = Phase::Returning { from_t: now };
            sim.phase_t = now;
        };
        match (self.trigger, sim.phase) {
            (Trigger::OnHover, Phase::Resting) if resp.hovered() => drop(&mut sim),
            (_, Phase::Resting) if resp.clicked() => drop(&mut sim),
            (_, Phase::Falling) if resp.clicked() => reset(&mut sim),
            (Trigger::OnAppear, Phase::Resting) if !sim.appeared && since > 0.6 => drop(&mut sim),
            (Trigger::Loop { pause_ms }, Phase::Resting) if since > 0.4 + pause_ms as f32 / 1000.0 => drop(&mut sim),
            (Trigger::Loop { pause_ms }, Phase::Falling) if since > 3.2 + pause_ms as f32 / 1000.0 => reset(&mut sim),
            _ => {}
        }
        let size = rect.size();
        let ret_dur = 0.9;
        match sim.phase {
            Phase::Falling => {
                let dt = glowkit_core::dt(ui);
                // shove words with the pointer
                if resp.dragged() {
                    if let Some(p) = resp.interact_pointer_pos() {
                        let pv = ui.input(|i| i.pointer.velocity());
                        let local = p - rect.min.to_vec2();
                        for b in &mut sim.bodies {
                            if Rect::from_center_size(b.pos, b.half * 2.0).expand(8.0).contains(local) {
                                b.vel += pv * 0.25;
                                b.spin += pv.x * 0.002;
                            }
                        }
                    }
                }
                let n = ((dt / (1.0 / 240.0)).ceil() as usize).clamp(1, 12);
                for _ in 0..n {
                    step(&mut sim.bodies, size, 2200.0 * self.gravity, dt / n as f32);
                }
                ui.ctx().request_repaint();
            }
            Phase::Returning { from_t } => {
                let k = ease(Easing::InOutCubic, ((now - from_t) as f32 / ret_dur).min(1.0));
                for ((b, s), h) in sim.bodies.iter_mut().zip(&sim.snapshot).zip(&homes) {
                    b.pos = s.pos.lerp(h.pos, k);
                    b.angle = s.angle * (1.0 - k);
                }
                if k >= 1.0 {
                    sim.phase = Phase::Resting;
                    sim.phase_t = now;
                    sim.bodies = homes.clone();
                }
                ui.ctx().request_repaint();
            }
            Phase::Resting => {
                if matches!(self.trigger, Trigger::Loop { .. } | Trigger::OnAppear) {
                    ui.ctx().request_repaint();
                }
            }
        }
        glowkit_core::set_state(ui, id, sim.clone());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        if self.background {
            let r = egui::CornerRadius::same((t.radius * 1.2) as u8);
            painter.rect_filled(rect, r, glowkit_core::with_alpha(t.muted, 0.55));
            painter.rect_stroke(rect, r, t.border_stroke(), egui::StrokeKind::Inside);
        }
        let color = self.color.unwrap_or(t.foreground);
        let hcol = self.highlight_color.unwrap_or(t.accent);
        let p = painter.with_clip_rect(rect.intersect(ui.clip_rect()));
        for ((s, _), b) in words.iter().zip(&sim.bodies) {
            let clean: String = s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
            let c = if self.highlight.contains(&clean) { hcol } else { color };
            let g = util::layout(&p, s.clone(), &font);
            util::paint_galley_xf(&p, &g, rect.min + b.pos.to_vec2(), 1.0, b.angle, c, bold);
        }
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    #[test]
    fn bodies_settle_on_floor() {
        let mut bodies = vec![
            Body { pos: pos2(50.0, 20.0), vel: Vec2::ZERO, angle: 0.3, spin: 1.0, half: vec2(20.0, 10.0) },
            Body { pos: pos2(55.0, 60.0), vel: Vec2::ZERO, angle: 0.0, spin: 0.0, half: vec2(20.0, 10.0) },
        ];
        for _ in 0..2400 {
            step(&mut bodies, vec2(200.0, 200.0), 2000.0, 1.0 / 240.0);
        }
        for b in &bodies {
            assert!(b.pos.y <= 200.0 - b.half.y + 0.01);
            assert!(b.vel.length() < 5.0, "{:?}", b.vel);
        }
        // they stack rather than overlap
        let d = bodies[0].pos - bodies[1].pos;
        assert!(d.x.abs() >= 39.0 || d.y.abs() >= 19.0, "{d:?}");
    }
}
