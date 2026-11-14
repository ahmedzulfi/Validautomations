//! Voice UI: animated orb, audio bars, live waveform and the mic button.

use crate::common;
use crate::icons::{paint_icon, Icon};
use egui::{pos2, vec2, Color32, CornerRadius, Id, Pos2, Rect, Response, Sense, Stroke, Ui, Widget};
use glowkit_core::color::{gradient, gradient_cyclic, mix, with_alpha};
use glowkit_core::noise::noise3;
use glowkit_core::{paint, Theme};
use std::f32::consts::TAU;

/// What a voice agent is doing; changes how the [`VoiceOrb`] moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VoiceState {
    /// Resting: slow breathing.
    #[default]
    Idle,
    /// Hearing the user: ripples with input `amplitude`.
    Listening,
    /// Working: swirls faster, slightly contracted.
    Thinking,
    /// Talking: pulses strongly with output `amplitude`.
    Speaking,
}

/// An organic, glowing orb for voice agents. Its outline wobbles with smooth noise, colors swirl
/// inside, and it swells with the `amplitude` you feed it (0..1, e.g. mic or TTS level).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let level = 0.3;
/// use glowkit_ai::{VoiceOrb, VoiceState};
/// ui.add(VoiceOrb::new().state(VoiceState::Speaking).amplitude(level).size(200.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct VoiceOrb {
    state: VoiceState,
    amplitude: f32,
    size: f32,
    colors: Option<[Color32; 3]>,
    id_salt: Option<egui::IdSalt>,
}

impl Default for VoiceOrb {
    fn default() -> Self {
        Self::new()
    }
}

impl VoiceOrb {
    /// An idle orb.
    pub fn new() -> Self {
        Self { state: VoiceState::Idle, amplitude: 0.0, size: 180.0, colors: None, id_salt: None }
    }
    /// Agent state.
    pub fn state(mut self, s: VoiceState) -> Self {
        self.state = s;
        self
    }
    /// Current audio level, 0..1 (smoothed internally).
    pub fn amplitude(mut self, a: f32) -> Self {
        self.amplitude = a.clamp(0.0, 1.0);
        self
    }
    /// Diameter in points.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    /// Three colors swirling inside. Defaults to the theme gradient.
    pub fn colors(mut self, c: [Color32; 3]) -> Self {
        self.colors = Some(c);
        self
    }
    /// Stable id for the smoothed state.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }
}

fn blob(center: Pos2, r: f32, wobble: f32, time: f32, seed: f32, n: usize) -> Vec<Pos2> {
    (0..n)
        .map(|i| {
            let a = i as f32 / n as f32 * TAU;
            let (s, c) = a.sin_cos();
            let d = noise3(c * 1.3 + seed, s * 1.3 - seed, time) * 0.7
                + noise3(c * 2.6 + seed * 2.0, s * 2.6, time * 1.7) * 0.3;
            center + vec2(c, s) * r * (1.0 + d * wobble)
        })
        .collect()
}

impl Widget for VoiceOrb {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let pad = self.size * 0.25;
        let (rect, resp) = ui.allocate_exact_size(vec2(self.size + pad * 2.0, self.size + pad * 2.0), Sense::hover());
        let id = common::widget_id(ui, self.id_salt);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let amp = glowkit_core::anim::follow(ui, id.with("amp"), self.amplitude, 14.0);
        let (speed_t, wobble_t, scale_t, energy_t) = match self.state {
            VoiceState::Idle => (0.25, 0.05, 0.94, 0.0),
            VoiceState::Listening => (0.45, 0.07 + amp * 0.12, 0.96 + amp * 0.06, 0.4 + amp * 0.6),
            VoiceState::Thinking => (1.1, 0.06, 0.9, 0.5),
            VoiceState::Speaking => (0.6, 0.06 + amp * 0.16, 0.95 + amp * 0.12, 0.6 + amp * 0.4),
        };
        let speed = glowkit_core::anim::follow(ui, id.with("spd"), speed_t, 3.0);
        let wobble = glowkit_core::anim::follow(ui, id.with("wob"), wobble_t, 6.0);
        let scale = glowkit_core::anim::follow(ui, id.with("scl"), scale_t, 8.0);
        let energy = glowkit_core::anim::follow(ui, id.with("eng"), energy_t, 4.0);
        // advance our own phase so speed changes don't jump
        let dt = glowkit_core::dt(ui);
        let phase = glowkit_core::with_state(
            ui,
            id.with("ph"),
            || 0.0f32,
            |p| {
                *p += dt * speed;
                *p
            },
        );
        let time = glowkit_core::time(ui);
        let breathe = (time * 1.4).sin() * 0.012;
        let colors = self.colors.unwrap_or(t.gradient);
        let c = rect.center();
        let r = self.size / 2.0 * (scale + breathe);
        let p = ui.painter();
        // halo
        paint::glow(
            p,
            c,
            r * (1.45 + energy * 0.25),
            with_alpha(colors[0], if t.dark { 0.20 + energy * 0.18 } else { 0.14 + energy * 0.12 }),
        );
        paint::glow(
            p,
            c + vec2(r * 0.2, r * 0.15),
            r * 1.25,
            with_alpha(colors[1], if t.dark { 0.10 + energy * 0.1 } else { 0.08 }),
        );
        // body: back layer (offset blob) + front layer with swirling conic colors
        let back = blob(c + vec2(r * 0.015, r * 0.02), r * 1.0, wobble * 1.4, phase * 0.8 + 3.0, 4.2, 72);
        let back_cols = [colors[1], colors[2]];
        paint::fill_convex_fn(p, &back, 4, |q| {
            with_alpha(gradient(&back_cols, ((q.y - c.y) / r * 0.5 + 0.5).clamp(0.0, 1.0)), 0.35)
        });
        let front = blob(c, r, wobble, phase, 0.0, 96);
        let rot = phase * 0.35;
        let hl = c + vec2((phase * 0.9).cos(), (phase * 0.7).sin()) * r * 0.35 - vec2(r * 0.15, r * 0.25);
        let swirl = [colors[0], colors[1], colors[2], colors[1]];
        let core = if t.dark { mix(colors[0], Color32::BLACK, 0.35) } else { mix(colors[0], Color32::WHITE, 0.15) };
        paint::fill_convex_fn(p, &front, 10, |q| {
            let d = q - c;
            let ang = d.y.atan2(d.x) / TAU + rot;
            let dist = (d.length() / r).clamp(0.0, 1.0);
            let warp = noise3(d.x / r * 1.5, d.y / r * 1.5, phase * 0.7) * 0.25;
            let base = gradient_cyclic(&swirl, ang + warp);
            // darker toward the center gives depth, bright rim
            let col = mix(core, base, 0.35 + 0.65 * dist.powf(0.8));
            let h = (1.0 - (q.distance(hl) / (r * 0.9))).clamp(0.0, 1.0);
            mix(col, Color32::WHITE, h * h * (0.35 + energy * 0.15))
        });
        // glassy rim
        let rim: Vec<Pos2> = front.clone();
        paint::gradient_polyline(p, &rim, true, 1.2, |u| {
            with_alpha(Color32::WHITE, 0.12 + 0.18 * ((u * TAU + phase).sin() * 0.5 + 0.5))
        });
        glowkit_core::animate(ui);
        resp
    }
}

/// Mirrored rounded bars from a slice of levels (0..1), e.g. an FFT or a short level history.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_ai::AudioBars::new(&[0.2, 0.5, 0.9, 0.4, 0.1]).height(48.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AudioBars<'a> {
    levels: &'a [f32],
    height: f32,
    bar_width: f32,
    gap: f32,
    color: Option<Color32>,
    gradient: bool,
}

impl<'a> AudioBars<'a> {
    /// Bars for `levels` (each 0..1).
    pub fn new(levels: &'a [f32]) -> Self {
        Self { levels, height: 56.0, bar_width: 4.0, gap: 3.0, color: None, gradient: true }
    }
    /// Total height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Bar width and gap.
    pub fn bar(mut self, width: f32, gap: f32) -> Self {
        self.bar_width = width;
        self.gap = gap;
        self
    }
    /// Solid bar color (turns the gradient off).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self.gradient = false;
        self
    }
}

impl Widget for AudioBars<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.levels.len();
        let w = n as f32 * (self.bar_width + self.gap) - self.gap;
        let (rect, resp) = ui.allocate_exact_size(vec2(w.max(1.0), self.height), Sense::hover());
        let p = ui.painter();
        for (i, l) in self.levels.iter().enumerate() {
            let h = (self.bar_width).max(l.clamp(0.0, 1.0) * self.height);
            let x = rect.min.x + i as f32 * (self.bar_width + self.gap) + self.bar_width / 2.0;
            let col = match (self.gradient, self.color) {
                (true, _) => gradient(&t.gradient, i as f32 / n.max(1) as f32),
                (false, Some(c)) => c,
                _ => t.foreground,
            };
            p.rect_filled(
                Rect::from_center_size(pos2(x, rect.center().y), vec2(self.bar_width, h)),
                CornerRadius::same((self.bar_width / 2.0) as u8),
                col,
            );
        }
        resp
    }
}

#[derive(Clone)]
struct WaveState {
    samples: Vec<f32>,
    acc: f32,
}

/// A scrolling recording waveform: every few milliseconds the current `amplitude` becomes a new
/// bar entering from the right, older bars scroll left and fade out. When not `active` it
/// settles into a flat dotted line.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mic_level = 0.5;
/// ui.add(glowkit_ai::LiveWaveform::new(mic_level).active(true).height(64.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct LiveWaveform {
    amplitude: f32,
    active: bool,
    height: f32,
    bar_width: f32,
    gap: f32,
    rate: f32,
    color: Option<Color32>,
    id_salt: Option<egui::IdSalt>,
}

impl LiveWaveform {
    /// A waveform fed with the current `amplitude` (0..1).
    pub fn new(amplitude: f32) -> Self {
        Self {
            amplitude: amplitude.clamp(0.0, 1.0),
            active: true,
            height: 64.0,
            bar_width: 3.0,
            gap: 2.0,
            rate: 24.0,
            color: None,
            id_salt: None,
        }
    }
    /// Recording (scrolling) or idle.
    pub fn active(mut self, on: bool) -> Self {
        self.active = on;
        self
    }
    /// Total height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Bar width and gap.
    pub fn bar(mut self, width: f32, gap: f32) -> Self {
        self.bar_width = width;
        self.gap = gap;
        self
    }
    /// New bars per second.
    pub fn rate(mut self, per_sec: f32) -> Self {
        self.rate = per_sec.max(1.0);
        self
    }
    /// Bar color. Defaults to the foreground.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Stable id for the sample history.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }
}

impl Widget for LiveWaveform {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(vec2(w, self.height), Sense::hover());
        let id: Id = common::widget_id(ui, self.id_salt);
        let step = self.bar_width + self.gap;
        let cap = (w / step).ceil() as usize + 2;
        let dt = glowkit_core::dt(ui);
        let (samples, frac) = glowkit_core::with_state(
            ui,
            id.with("wave"),
            || WaveState {
                // start with a quiet room-noise history so the strip is never empty
                samples: if self.active {
                    (0..cap).map(|i| 0.04 + 0.08 * glowkit_core::rng::hash01(i as u64 * 31 + 7)).collect()
                } else {
                    Vec::new()
                },
                acc: 0.0,
            },
            |s| {
                if self.active {
                    s.acc += dt * self.rate;
                    while s.acc >= 1.0 {
                        s.acc -= 1.0;
                        // slight randomness so constant input still looks alive
                        let jitter = 0.75
                            + 0.5 * glowkit_core::rng::hash01(s.samples.len() as u64 * 7919 + (s.acc * 1000.0) as u64);
                        s.samples.push((self.amplitude * jitter).clamp(0.0, 1.0));
                    }
                    let excess = s.samples.len().saturating_sub(cap);
                    if excess > 0 {
                        s.samples.drain(..excess);
                    }
                } else {
                    for v in &mut s.samples {
                        *v *= (1.0 - dt * 6.0).max(0.0);
                    }
                }
                (s.samples.clone(), s.acc)
            },
        );
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let col = self.color.unwrap_or(t.foreground);
        let p = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        let cy = rect.center().y;
        // idle dotted baseline
        let mut x = rect.min.x + step / 2.0;
        while x < rect.max.x {
            p.circle_filled(pos2(x, cy), 1.0, with_alpha(t.muted_foreground, 0.35));
            x += step;
        }
        let offset = if self.active { frac * step } else { 0.0 };
        for (k, v) in samples.iter().rev().enumerate() {
            let x = rect.max.x - step / 2.0 - k as f32 * step - offset;
            if x < rect.min.x - step {
                break;
            }
            let age = (x - rect.min.x) / rect.width();
            let fade = (age * 3.0).clamp(0.0, 1.0);
            let h = (v * self.height * 0.92).max(2.0);
            p.rect_filled(
                Rect::from_center_size(pos2(x, cy), vec2(self.bar_width, h)),
                CornerRadius::same((self.bar_width / 2.0) as u8),
                with_alpha(col, fade * if k == 0 { 1.0 } else { 0.85 }),
            );
        }
        if self.active || samples.iter().any(|v| *v > 0.01) {
            glowkit_core::animate(ui);
        }
        resp
    }
}

/// A round microphone button. While `listening` it turns accent-colored and emits soft pulsing
/// rings that grow with `amplitude`. Returns the click response (toggle listening yourself).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut listening = false;
/// if ui.add(glowkit_ai::VoiceButton::new(listening).amplitude(0.4)).clicked() {
///     listening = !listening;
/// }
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct VoiceButton {
    listening: bool,
    amplitude: f32,
    size: f32,
}

impl VoiceButton {
    /// A mic button; `listening` shows the active state.
    pub fn new(listening: bool) -> Self {
        Self { listening, amplitude: 0.0, size: 56.0 }
    }
    /// Input level 0..1 (ring size).
    pub fn amplitude(mut self, a: f32) -> Self {
        self.amplitude = a.clamp(0.0, 1.0);
        self
    }
    /// Diameter.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
}

impl Widget for VoiceButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let pad = self.size * 0.6;
        let (rect, resp) = ui.allocate_exact_size(vec2(self.size + pad * 2.0, self.size + pad * 2.0), Sense::hover());
        let c = rect.center();
        let r = self.size / 2.0;
        let btn = ui
            .interact(Rect::from_center_size(c, vec2(self.size, self.size)), resp.id.with("btn"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let on = common::anim_bool(ui, resp.id.with("on"), self.listening, 0.25);
        let h = common::anim_bool(ui, resp.id.with("h"), btn.hovered(), 0.12);
        let amp = glowkit_core::anim::follow(
            ui,
            resp.id.with("amp"),
            if self.listening { self.amplitude } else { 0.0 },
            12.0,
        );
        let time = glowkit_core::time(ui);
        let p = ui.painter();
        let accent = t.destructive;
        if on > 0.0 {
            for k in 0..3 {
                let ph = (time * 0.7 + k as f32 / 3.0).fract();
                let rr = r + ph * pad * (0.6 + amp * 0.5);
                p.circle_stroke(c, rr, Stroke::new(1.5, with_alpha(accent, (1.0 - ph) * 0.45 * on)));
            }
            paint::glow(p, c, r * (1.5 + amp * 0.6), with_alpha(accent, 0.25 * on));
            glowkit_core::animate(ui);
        }
        let press = if btn.is_pointer_button_down_on() { 0.94 } else { 1.0 };
        let base = mix(if t.dark { t.secondary } else { t.muted }, common::raised(&t), h * 0.5);
        let fill = mix(base, accent, on);
        let rr = r * press * (1.0 + amp * 0.06 * on);
        p.circle_filled(c, rr, fill);
        p.circle_stroke(c, rr, Stroke::new(1.0, mix(t.border, accent, on)));
        let icon_col = mix(t.foreground, Color32::WHITE, on);
        let ic = Rect::from_center_size(c, vec2(self.size * 0.42, self.size * 0.42));
        if self.listening {
            paint_icon(p, Icon::Stop, ic.shrink(self.size * 0.04), icon_col);
        } else {
            paint_icon(p, Icon::Mic, ic, icon_col);
        }
        btn
    }
}
