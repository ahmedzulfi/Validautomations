//! Text effects: letters that roll over on hover, and a vertical word reel driven by momentum.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Ui};
use glowkit_core::anim::{approach, ease, Easing};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::text::{paint_char, Glyphs};
use glowkit_core::Theme;
use std::f32::consts::PI;

// ---------------------------------------------------------------------------------------------
// FlipLetters

#[derive(Clone, Default)]
struct FlipState {
    starts: Vec<f32>,
    last_hover: Option<usize>,
}

/// A word whose letters roll over like a split-flap when the pointer passes.
///
/// Each letter sits in its own little window. Hover one and it rolls up and out of the window
/// while a fresh copy rolls in from below with a springy overshoot and a slight tilt; the roll
/// ripples outward to the neighbours, one after another. Click to send the wave across the
/// whole word.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::FlipLetters;
/// ui.add(FlipLetters::new("Gesture").size(64.0).ripple(50.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct FlipLetters {
    text: String,
    size: f32,
    ripple: f32,
    duration: f32,
    color: Option<Color32>,
    accent: Option<Color32>,
    autoplay: bool,
}

impl FlipLetters {
    /// Rolling letters spelling `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), size: 60.0, ripple: 50.0, duration: 0.6, color: None, accent: None, autoplay: false }
    }
    /// Font size.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(8.0);
        self
    }
    /// How far the roll ripples to neighbouring letters, 0..100.
    pub fn ripple(mut self, r: f32) -> Self {
        self.ripple = r.clamp(0.0, 100.0);
        self
    }
    /// Seconds per roll.
    pub fn duration(mut self, d: f32) -> Self {
        self.duration = d.clamp(0.15, 3.0);
        self
    }
    /// Letter color (default: the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Color of the incoming copy (default: same as the letters).
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = Some(c);
        self
    }
    /// Sweep a ghost pointer across the word while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for FlipLetters {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = FontId::proportional(self.size);
        let glyphs = Glyphs::layout(ui.painter(), &self.text, font.clone(), None);
        let size = glyphs.size + vec2(0.0, self.size * 0.2);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let id = response.id;
        let now = glowkit_core::time(ui);
        let mut st: FlipState = glowkit_core::state(ui, id.with("flip"), FlipState::default);
        let n = glyphs.glyphs.len();
        st.starts.resize(n, -10.0);
        let origin = rect.min;
        let idle = idle_for(ui, id, response.hovered());
        let mut pointer = response.hover_pos();
        let mut ghost = None;
        if self.autoplay && !response.hovered() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
        }
        if self.autoplay && !response.hovered() && idle > GHOST_DELAY {
            let e = (idle - GHOST_DELAY) % 4.0;
            let u = ramp(e, 0.4, 1.8);
            let x = rect.min.x - 20.0 + u * (rect.width() + 40.0);
            let p = pos2(x, rect.center().y + (u * PI * 2.0).sin() * 6.0);
            let a = ramp(e, 0.1, 0.4) * (1.0 - ramp(e, 1.9, 2.3));
            if e > 0.3 && e < 2.0 {
                pointer = Some(p);
            }
            ghost = Some((p, a));
            ui.ctx().request_repaint();
        }
        let hit = pointer.and_then(|p| glyphs.glyphs.iter().position(|g| Rect::from_min_size(origin + g.offset, g.size).contains(p)));
        let spread = (self.ripple / 100.0 * 6.0).round() as i32;
        let stagger = 0.045;
        if let Some(h) = hit {
            if st.last_hover != Some(h) {
                for (i, s) in st.starts.iter_mut().enumerate() {
                    let d = (i as i32 - h as i32).abs();
                    if d <= spread && now - *s > self.duration {
                        *s = now + d as f32 * stagger;
                    }
                }
            }
        }
        st.last_hover = hit;
        if response.clicked() {
            for (i, s) in st.starts.iter_mut().enumerate() {
                *s = now + i as f32 * stagger;
            }
        }
        let busy = st.starts.iter().any(|s| now - s < self.duration + 0.05);
        let s = st.clone();
        glowkit_core::set_state(ui, id.with("flip"), st);
        if busy {
            ui.ctx().request_repaint();
        }
        let painter = ui.painter_at(rect.expand(self.size * 0.3));
        let col = self.color.unwrap_or(t.foreground);
        let acc = self.accent.unwrap_or(col);
        let line_h = self.size * 1.15;
        for (i, g) in glyphs.glyphs.iter().enumerate() {
            let p = origin + g.offset;
            let u = ((now - s.starts[i]) / self.duration).clamp(0.0, 1.0);
            let k = ease(Easing::OutBack, u);
            if u <= 0.0 || u >= 1.0 {
                paint_char(&painter, g.ch, p, &font, col, 1.0, 0.0);
                continue;
            }
            let cell = Rect::from_min_size(p - vec2(4.0, 0.0), vec2(g.size.x + 8.0, line_h));
            let clip = painter.with_clip_rect(cell.intersect(painter.clip_rect()));
            let tilt = (u * PI).sin() * 0.22;
            paint_char(&clip, g.ch, p - vec2(0.0, k * line_h), &font, with_alpha(col, 1.0 - u * 0.6), 1.0, -tilt);
            paint_char(&clip, g.ch, p + vec2(0.0, (1.0 - k) * line_h), &font, mix(acc, col, u * u), 1.0, tilt * 0.6);
        }
        if let Some((p, a)) = ghost {
            ghost_touch(&painter, p, 0.0, a);
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------
// TextReel

#[derive(Clone, Copy, Default)]
struct ReelState {
    pos: f32,
    vel: f32,
    dir: f32,
    g: Gesture,
}

/// A vertical reel of words that streams past a fixed label, steered by your scroll.
///
/// The words drift upward at a calm pace under a small spaced-out label ("LET'S Build",
/// "LET'S Ship", ...). Scroll the wheel or drag the reel and you inject momentum: it speeds up,
/// slows, or reverses direction to follow you, then eases back to its cruising speed in the
/// direction you last pushed it. The word passing the middle is bold and dark; words fade and
/// shrink toward the edges like a drum.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::TextReel;
/// ui.add(TextReel::new("LET'S", &["Build", "Ship", "Iterate", "Learn"]).speed(40.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TextReel {
    label: String,
    words: Vec<String>,
    speed: f32,
    size: f32,
    height: f32,
    autoplay: bool,
}

impl TextReel {
    /// A reel of `words` under `label`.
    pub fn new(label: impl Into<String>, words: &[&str]) -> Self {
        Self {
            label: label.into(),
            words: words.iter().map(|s| s.to_string()).collect(),
            speed: 40.0,
            size: 40.0,
            height: 300.0,
            autoplay: false,
        }
    }
    /// Cruising speed in points per second.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s.abs();
        self
    }
    /// Font size of the words.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(10.0);
        self
    }
    /// Widget height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(80.0);
        self
    }
    /// Push the reel around on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const REEL_SCRIPT: [Stroke2; 2] = [drag(1.0, 1.25, (0.5, 0.7), (0.5, 0.25)), drag(4.0, 4.3, (0.5, 0.3), (0.5, 0.78))];

impl egui::Widget for TextReel {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let touch = touch(ui, &response, rect, self.autoplay, &REEL_SCRIPT, 7.5);
        let mut st: ReelState = glowkit_core::state(ui, id.with("reel"), ReelState::default);
        if st.dir == 0.0 {
            st.dir = 1.0;
        }
        let g = st.g.update(&touch, now, dt);
        let spacing = self.size * 1.25;
        if st.g.down {
            st.pos -= g.delta.y / spacing;
            st.vel = -st.g.vel.y / spacing;
        } else {
            if response.hovered() {
                let wheel = ui.input(|i| i.smooth_scroll_delta.y);
                st.vel -= wheel / spacing * 9.0;
            }
            if st.vel.abs() > self.speed / spacing * 1.5 {
                st.dir = st.vel.signum();
            }
            let cruise = st.dir * self.speed / spacing;
            st.vel = approach(st.vel, cruise, 1.4, dt);
            st.pos += st.vel * dt;
        }
        let n = self.words.len().max(1) as f32;
        st.pos = st.pos.rem_euclid(n);
        let s = st;
        glowkit_core::set_state(ui, id.with("reel"), st);
        ui.ctx().request_repaint();
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let cy = rect.center().y - self.size * 0.6;
        let label_y = cy - self.size * 0.85;
        // label, spaced
        let spaced: String = self.label.chars().flat_map(|c| [c, '\u{2009}', '\u{2009}']).collect();
        text(&painter, pos2(rect.center().x, label_y), Align2::CENTER_CENTER, spaced.trim_end(), 12.5, t.muted_foreground);
        let reel = Rect::from_min_max(pos2(rect.min.x, cy - self.size * 0.62), rect.max);
        let clip = painter.with_clip_rect(reel);
        let k0 = s.pos.floor() as i32 - 4;
        let speed_blur = (s.vel.abs() * spacing / 900.0).clamp(0.0, 1.0);
        for k in k0..k0 + 9 {
            let rel = k as f32 - s.pos;
            let y = cy + rel * spacing;
            let d = (rel.max(rel * -3.0) / 3.2).clamp(0.0, 1.0);
            let word = &self.words[k.rem_euclid(self.words.len() as i32) as usize];
            let sc = 1.0 - 0.3 * d;
            let a = (1.0 - d).powf(1.3);
            let col = mix(t.foreground, t.muted_foreground, (d * 1.6).min(1.0));
            let font = FontId::proportional(self.size * sc);
            if d < 0.25 {
                painter_bold(&clip, pos2(rect.center().x, y), word, font.clone(), with_alpha(col, a));
            } else {
                clip.text(pos2(rect.center().x, y), Align2::CENTER_CENTER, word, font.clone(), with_alpha(col, a));
            }
            if speed_blur > 0.05 {
                for j in 1..3 {
                    let off = j as f32 * 3.0 * speed_blur * s.vel.signum();
                    clip.text(pos2(rect.center().x, y + off), Align2::CENTER_CENTER, word, font.clone(), with_alpha(col, a * 0.15));
                }
            }
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}

fn painter_bold(p: &egui::Painter, c: egui::Pos2, s: &str, font: FontId, col: Color32) {
    p.text(c + vec2(0.5, 0.0), Align2::CENTER_CENTER, s, font.clone(), col);
    p.text(c, Align2::CENTER_CENTER, s, font, col);
}
