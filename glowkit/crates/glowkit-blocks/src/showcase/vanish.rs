use super::util::idle_time;
use egui::{pos2, vec2, Color32, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget};
use glowkit_core::{anim::smoothstep, color::with_alpha, paint, text::Glyphs, Rng, Theme};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
struct Particle {
    pos: Vec2,
    vel: Vec2,
    delay: f32,
    size: f32,
}

#[derive(Clone, Debug, Default)]
struct VanishState {
    /// The text being dissolved, with each glyph's offset and wipe delay.
    glyphs: Arc<Vec<(char, Vec2, f32)>>,
    particles: Arc<Vec<Particle>>,
    start: f64,
    /// Autoplay: phrase index, characters typed, time of the last keystroke.
    auto_phrase: usize,
    auto_chars: usize,
    auto_last: f64,
    seed: u64,
}

/// What [`VanishInput::show`] returns.
pub struct VanishOutput {
    /// Response of the text field.
    pub response: Response,
    /// The text the user submitted this frame (Enter or the arrow button), if any.
    pub submitted: Option<String>,
}

/// A pill-shaped input whose placeholder cycles through prompts (sliding up and fading), and
/// which dissolves the submitted text into drifting particles, right to left, when you press
/// Enter. With `autoplay` it types and vanishes sample prompts while unfocused.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut text = String::new();
/// let out = glowkit_blocks::VanishInput::new(&mut text)
///     .placeholders(&["Ask me anything", "What is glowkit?"])
///     .show(ui);
/// if let Some(q) = out.submitted { println!("{q}"); }
/// ```
#[must_use = "call `.show(ui)` or add it with `ui.add(...)`"]
pub struct VanishInput<'a> {
    text: &'a mut String,
    placeholders: Vec<String>,
    width: f32,
    font_size: f32,
    autoplay: bool,
    interval: f32,
}

impl<'a> VanishInput<'a> {
    /// An input editing `text`.
    pub fn new(text: &'a mut String) -> Self {
        Self { text, placeholders: vec!["Type something and press Enter".into()], width: 520.0, font_size: 15.0, autoplay: false, interval: 3.0 }
    }
    /// Placeholder prompts to cycle through.
    pub fn placeholders(mut self, p: &[&str]) -> Self {
        self.placeholders = p.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// Font size.
    pub fn font_size(mut self, s: f32) -> Self {
        self.font_size = s;
        self
    }
    /// Seconds each placeholder stays.
    pub fn interval(mut self, s: f32) -> Self {
        self.interval = s.max(0.5);
        self
    }
    /// Type and vanish the placeholders by itself while unfocused and empty.
    pub fn autoplay(mut self, b: bool) -> Self {
        self.autoplay = b;
        self
    }

    /// Show the input.
    pub fn show(self, ui: &mut Ui) -> VanishOutput {
        let t = Theme::of(ui);
        let h = self.font_size * 3.2;
        let (rect, bg_resp) = ui.allocate_exact_size(vec2(self.width, h), Sense::click());
        let id = bg_resp.id.with("vanish");
        let now = ui.input(|i| i.time);
        let time = now as f32;
        let font = FontId::proportional(self.font_size);
        let pad = h * 0.42;
        let btn_r = h * 0.34;
        let btn_c = pos2(rect.max.x - h * 0.5, rect.center().y);
        let text_rect = Rect::from_min_max(pos2(rect.min.x + pad, rect.min.y + 4.0), pos2(btn_c.x - btn_r - 8.0, rect.max.y - 4.0));
        let row_h = ui.painter().layout_no_wrap("Ag".into(), font.clone(), Color32::WHITE).size().y;
        let text_origin = pos2(text_rect.min.x, rect.center().y - row_h / 2.0);
        let mut st: VanishState = glowkit_core::state(ui, id, VanishState::default);
        let vanishing = (now - st.start) < 1.6 && !st.glyphs.is_empty();

        // frame
        let p = ui.painter().clone();
        let focused_prev = ui.memory(|m| m.has_focus(id.with("edit")));
        let ring = glowkit_core::anim::follow(ui, id.with("ring"), if focused_prev { 1.0 } else { 0.0 }, 12.0);
        paint::soft_shadow(&p, rect, h / 2.0, 18.0, vec2(0.0, 6.0), Color32::from_black_alpha(if t.dark { 90 } else { 22 }));
        p.rect_filled(rect, h / 2.0, if t.dark { t.card } else { Color32::WHITE });
        p.rect_stroke(rect, h / 2.0, Stroke::new(1.0, t.border.lerp_to_gamma(t.ring, ring * 0.8)), StrokeKind::Inside);

        // the real text field
        let edit_color = if vanishing { Color32::TRANSPARENT } else { t.foreground };
        let edit = egui::TextEdit::singleline(self.text)
            .id(id.with("edit"))
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .font(font.clone())
            .text_color(edit_color)
            .align(egui::Align2::LEFT_CENTER)
            .desired_width(text_rect.width());
        let resp = ui.put(text_rect, edit);
        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        // arrow button
        let btn_rect = Rect::from_center_size(btn_c, Vec2::splat(btn_r * 2.0));
        let btn = ui.interact(btn_rect, id.with("btn"), Sense::click());
        if bg_resp.clicked() {
            resp.request_focus();
        }
        let mut submitted = None;
        let mut dissolve: Option<String> = None;
        if (enter || btn.clicked()) && !self.text.trim().is_empty() && !vanishing {
            submitted = Some(self.text.clone());
            dissolve = Some(std::mem::take(self.text));
        }

        // autoplay: type a prompt, then vanish it
        let active = resp.has_focus() || bg_resp.hovered() || !self.text.is_empty();
        let idle = idle_time(ui, id, active);
        let mut auto_text: Option<String> = None;
        if self.autoplay && idle > 1.5 && !vanishing && !self.placeholders.is_empty() {
            let phrase: Vec<char> = self.placeholders[st.auto_phrase % self.placeholders.len()].chars().collect();
            if st.auto_chars < phrase.len() {
                if now - st.auto_last > 0.055 {
                    st.auto_chars += 1;
                    st.auto_last = now;
                }
                auto_text = Some(phrase[..st.auto_chars].iter().collect());
            } else if now - st.auto_last > 0.9 {
                dissolve = Some(phrase.iter().collect());
                st.auto_phrase += 1;
                st.auto_chars = 0;
                st.auto_last = now + 1.4;
            } else {
                auto_text = Some(phrase.iter().collect());
            }
            glowkit_core::animate(ui);
        } else if !self.autoplay || active {
            st.auto_chars = 0;
        }

        if let Some(txt) = dissolve {
            let g = Glyphs::layout(&p, &txt, font.clone(), None);
            let w = g.size.x.max(1.0);
            st.seed = st.seed.wrapping_add(1);
            let mut rng = Rng::new(st.seed.wrapping_mul(7919));
            let mut glyphs = Vec::new();
            let mut parts = Vec::new();
            for gb in &g.glyphs {
                // right-most glyphs go first
                let delay = (1.0 - (gb.offset.x + gb.size.x * 0.5) / w) * 0.55;
                glyphs.push((gb.ch, gb.offset, delay));
                if gb.ch.is_whitespace() {
                    continue;
                }
                let per = ((gb.size.x * gb.size.y) / 14.0).clamp(6.0, 18.0) as usize;
                for _ in 0..per {
                    let o = gb.offset + vec2(rng.range(0.1, 0.9) * gb.size.x, rng.range(0.28, 0.82) * gb.size.y);
                    parts.push(Particle {
                        pos: (text_origin - rect.min) + o,
                        vel: vec2(rng.range(-30.0, 70.0), rng.range(-45.0, 25.0)),
                        delay: delay + rng.range(0.0, 0.08),
                        size: rng.range(0.9, 1.9),
                    });
                }
            }
            st.glyphs = Arc::new(glyphs);
            st.particles = Arc::new(parts);
            st.start = now;
        }

        let el = (now - st.start) as f32;
        let vanishing = el < 1.6 && !st.glyphs.is_empty();
        if vanishing {
            glowkit_core::animate(ui);
        }
        let clip = p.with_clip_rect(rect.shrink(2.0).intersect(ui.clip_rect()));
        // dissolving glyphs + particles
        if vanishing {
            for (ch, off, delay) in st.glyphs.iter() {
                let k = 1.0 - smoothstep(*delay, *delay + 0.08, el);
                if k > 0.0 {
                    glowkit_core::text::paint_char(&clip, *ch, text_origin + *off, &font, with_alpha(t.foreground, k), 1.0, 0.0);
                }
            }
            for pt in st.particles.iter() {
                let age = el - pt.delay;
                if age < 0.0 {
                    continue;
                }
                let life = (1.0 - age / 0.9).clamp(0.0, 1.0);
                if life <= 0.0 {
                    continue;
                }
                let drift = pt.vel * age + vec2(0.0, -12.0 * age * age);
                let pp = rect.min + pt.pos + drift;
                clip.circle_filled(pp, pt.size * (0.5 + 0.5 * life), with_alpha(t.foreground, life * life));
            }
        }
        // autoplay text with caret
        if let Some(at) = &auto_text {
            let r = clip.text(text_origin, egui::Align2::LEFT_TOP, at, font.clone(), t.foreground);
            if (time * 2.0).fract() < 0.6 {
                clip.rect_filled(Rect::from_min_size(pos2(r.max.x + 1.0, text_origin.y + 1.0), vec2(1.5, row_h - 2.0)), 0.0, t.foreground);
            }
        }
        // placeholders
        let show_ph = self.text.is_empty() && !vanishing && auto_text.is_none() && !self.placeholders.is_empty();
        if show_ph {
            glowkit_core::animate(ui);
            let n = self.placeholders.len();
            let slot = time / self.interval;
            let i = slot as usize % n;
            let f = slot.fract() * self.interval; // seconds into this slot
            let enter_k = smoothstep(0.0, 0.35, f);
            let leave_k = smoothstep(self.interval - 0.35, self.interval, f);
            let dy = (1.0 - enter_k) * 10.0 - leave_k * 10.0;
            let a = enter_k * (1.0 - leave_k);
            clip.text(text_origin + vec2(0.0, dy), egui::Align2::LEFT_TOP, &self.placeholders[i], font.clone(), with_alpha(t.muted_foreground, a));
        }
        // button
        let has_text = !self.text.is_empty() || auto_text.as_ref().is_some_and(|s| !s.is_empty());
        let on = glowkit_core::anim::follow(ui, id.with("on"), if has_text { 1.0 } else { 0.0 }, 12.0);
        let bg = t.muted.lerp_to_gamma(t.primary, on);
        let fg = t.muted_foreground.lerp_to_gamma(t.primary_foreground, on);
        p.circle_filled(btn_c, btn_r, bg);
        let len = btn_r * 0.9;
        let ax = btn_c.x - len / 2.0 + (1.0 - on) * 0.0;
        let tail = len * (0.35 + 0.65 * on);
        p.line_segment([pos2(ax + len - tail, btn_c.y), pos2(ax + len, btn_c.y)], Stroke::new(2.0, fg));
        super::util::chevron(&p, pos2(ax + len - btn_r * 0.15, btn_c.y), btn_r * 0.7, vec2(1.0, 0.0), Stroke::new(2.0, fg));
        if btn.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        glowkit_core::set_state(ui, id, st);
        VanishOutput { response: resp, submitted }
    }
}

impl Widget for VanishInput<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}
