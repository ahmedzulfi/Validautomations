//! A one-time-code field that offers an incoming code, types it in and verifies it.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Ui, Vec2};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Default)]
struct OtpState {
    focused: bool,
    kb: f32,
    kb_v: f32,
    pop: [f32; 8],
    pop_v: [f32; 8],
    shake: f32,
    shake_v: f32,
    status: u8,
    status_t: f32,
    typing: Option<(String, f32)>,
    sugg: usize,
    sugg_since: f32,
    key_flash: [f32; 12],
    g: Gesture,
    cycle: u32,
    last_len: usize,
}

/// A security-code field with autofill, like the one phones show after an SMS arrives.
///
/// Tap the slots to focus: a number pad springs up with a suggestion strip on top showing the
/// code that just arrived. Tap the suggestion and the digits type themselves in one by one, each
/// popping into its slot. Once every slot is filled the code is checked against
/// [`OtpAutofill::expected`]: a match turns the slots green and locks them; a mismatch shakes the
/// row with a damped spring, flashes it red and clears it. The keypad and the keyboard both type
/// too (backspace deletes).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut code = String::new();
/// use glowkit_interactions::OtpAutofill;
/// ui.add(OtpAutofill::new(&mut code).expected("482913").suggestion("482913"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct OtpAutofill<'a> {
    value: &'a mut String,
    len: usize,
    expected: Option<String>,
    suggestions: Vec<String>,
    size: Option<Vec2>,
    shake: f32,
    title: String,
    autoplay: bool,
}

impl<'a> OtpAutofill<'a> {
    /// A 6-digit field editing `value`.
    pub fn new(value: &'a mut String) -> Self {
        Self {
            value,
            len: 6,
            expected: None,
            suggestions: Vec::new(),
            size: None,
            shake: 60.0,
            title: "Enter the code".into(),
            autoplay: false,
        }
    }
    /// Number of digits (4..=8).
    pub fn digits(mut self, n: usize) -> Self {
        self.len = n.clamp(4, 8);
        self
    }
    /// The code that verifies (otherwise any complete code is accepted).
    pub fn expected(mut self, code: impl Into<String>) -> Self {
        self.expected = Some(code.into());
        self
    }
    /// A code offered above the keypad (call again to queue more).
    pub fn suggestion(mut self, code: impl Into<String>) -> Self {
        self.suggestions.push(code.into());
        self
    }
    /// Widget size (default: all available space).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Strength of the wrong-code shake, 0..100.
    pub fn shake(mut self, s: f32) -> Self {
        self.shake = s.clamp(0.0, 100.0);
        self
    }
    /// Heading above the slots.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    /// Play a scripted demo (a stale code, then the right one) while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const SCRIPT: [Stroke2; 3] = [tap(0.8, 0.5, 0.33), tap(2.2, 0.5, 0.575), tap(5.4, 0.5, 0.575)];
const KEYS: [&str; 12] = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "", "0", "⌫"];

impl egui::Widget for OtpAutofill<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = self.size.unwrap_or(ui.available_size());
        let (rect, mut response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let touch = touch(ui, &response, rect, self.autoplay, &SCRIPT, 10.0);
        let mut st: OtpState = glowkit_core::state(ui, id.with("otp"), OtpState::default);
        let n = self.len;
        let mut suggestions = self.suggestions.clone();
        let expected = if touch.ghost {
            suggestions = vec!["115620".into(), "482913".into()];
            Some("482913".to_string())
        } else {
            self.expected.clone()
        };
        if suggestions.is_empty() {
            if let Some(e) = &expected {
                suggestions.push(e.clone());
            }
        }
        if touch.ghost && touch.cycle != st.cycle {
            st = OtpState { cycle: touch.cycle, ..Default::default() };
            self.value.clear();
        }

        // geometry
        let slot = vec2(((rect.width() - 60.0) / (n as f32 + 1.2)).min(40.0), 44.0);
        let gap = 5.0;
        let half = n / 2;
        let dash = 18.0;
        let total = slot.x * n as f32 + gap * (n as f32 - 2.0) + dash;
        let row_y = rect.min.y + rect.height() * 0.33;
        let slot_rect = |i: usize| {
            let mut x = rect.center().x - total / 2.0 + i as f32 * (slot.x + gap);
            if i >= half {
                x += dash - gap;
            }
            Rect::from_min_size(pos2(x, row_y - slot.y / 2.0), slot)
        };
        let row = Rect::from_min_max(slot_rect(0).min, slot_rect(n - 1).max).expand(8.0);
        let kb_h = 196.0_f32.min(rect.height() * 0.4);
        let bar_h = 46.0;
        let kb_top = rect.max.y - kb_h * st.kb;
        let bar = Rect::from_min_max(pos2(rect.min.x, kb_top - bar_h), pos2(rect.max.x, kb_top));
        let key_rect = |k: usize| {
            let (c, r) = (k % 3, k / 3);
            let kw = (rect.width() - 24.0) / 3.0;
            let kh = (kb_h - 20.0) / 4.0;
            Rect::from_min_size(
                pos2(rect.min.x + 8.0 + c as f32 * (kw + 4.0), kb_top + 8.0 + r as f32 * kh),
                vec2(kw, kh - 4.0),
            )
        };

        // input
        let g = st.g.update(&touch, now, dt);
        let locked = st.status == 1;
        let sugg_code = suggestions.get(st.sugg.min(suggestions.len().saturating_sub(1))).cloned();
        let sugg_ready = st.focused && self.value.is_empty() && st.typing.is_none() && now - st.sugg_since > 0.5;
        if g.tap {
            let p = st.g.last;
            if row.contains(p) && !locked {
                st.focused = true;
            } else if st.kb > 0.5 && bar.contains(p) && sugg_ready && sugg_code.is_some() {
                st.typing = Some((sugg_code.clone().unwrap_or_default(), now));
            } else if st.kb > 0.5 && p.y > kb_top && !locked {
                if let Some(k) = (0..12).find(|&k| key_rect(k).contains(p)) {
                    st.key_flash[k] = 1.0;
                    match KEYS[k] {
                        "⌫" => {
                            self.value.pop();
                        }
                        "" => {}
                        d if self.value.len() < n => self.value.push_str(d),
                        _ => {}
                    }
                }
            } else if !row.contains(p) && p.y < bar.min.y {
                st.focused = false;
            }
        }
        if st.focused && !locked && !touch.ghost {
            let events = ui.input(|i| i.events.clone());
            for e in events {
                match e {
                    egui::Event::Text(s) => {
                        for ch in s.chars().filter(|c| c.is_ascii_digit()) {
                            if self.value.len() < n {
                                self.value.push(ch);
                            }
                        }
                    }
                    egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } => {
                        self.value.pop();
                    }
                    _ => {}
                }
            }
        }
        if let Some((code, t0)) = st.typing.clone() {
            let k = (((now - t0) / 0.075) as usize + 1).min(code.len()).min(n);
            if self.value.len() < k {
                *self.value = code.chars().take(k).collect();
            }
            if k >= code.len().min(n) {
                st.typing = None;
            }
        }
        self.value.retain(|c| c.is_ascii_digit());
        self.value.truncate(n);
        let len = self.value.len();
        if len > st.last_len {
            for i in st.last_len..len {
                st.pop[i] = 0.0;
                st.pop_v[i] = 9.0;
            }
            response.mark_changed();
        } else if len < st.last_len {
            response.mark_changed();
        }
        st.last_len = len;
        if len == n && st.status == 0 {
            let ok = expected.as_ref().is_none_or(|e| e == self.value.as_str());
            st.status = if ok { 1 } else { 2 };
            st.status_t = now;
            if ok {
                st.focused = false;
                for i in 0..n {
                    st.pop_v[i] = 6.0 + i as f32;
                }
            } else {
                st.shake_v = 300.0 + self.shake * 12.0;
            }
        }
        if st.status == 2 && now - st.status_t > 0.75 {
            self.value.clear();
            st.status = 0;
            st.last_len = 0;
            st.sugg += 1;
            st.sugg_since = now + 0.6;
            response.mark_changed();
        }

        // springs
        let kb_target = if st.focused { 1.0 } else { 0.0 };
        spring_step(&mut st.kb, &mut st.kb_v, kb_target, 260.0, 26.0, dt);
        for i in 0..8 {
            spring_step(&mut st.pop[i], &mut st.pop_v[i], 0.0, 520.0, 16.0, dt);
        }
        spring_step(&mut st.shake, &mut st.shake_v, 0.0, 900.0, 9.0, dt);
        for f in st.key_flash.iter_mut() {
            *f = (*f - dt * 4.0).max(0.0);
        }
        let animating = !settled(st.kb, st.kb_v, kb_target)
            || st.pop_v.iter().any(|v| v.abs() > 0.01)
            || st.shake_v.abs() > 0.5
            || st.typing.is_some()
            || st.status == 2
            || st.key_flash.iter().any(|f| *f > 0.0);
        let snapshot = st.clone();
        glowkit_core::set_state(ui, id.with("otp"), st);
        let st = snapshot;
        if animating || touch.ghost {
            ui.ctx().request_repaint();
        } else if st.focused {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }

        // paint
        let painter = ui.painter_at(rect);
        let fg = t.foreground;
        text_bold(&painter, pos2(rect.center().x, row_y - 92.0), Align2::CENTER_CENTER, &self.title, 21.0, fg);
        text(
            &painter,
            pos2(rect.center().x, row_y - 64.0),
            Align2::CENTER_CENTER,
            format!("We sent a {n}-digit code to +1 555 0100."),
            12.0,
            t.muted_foreground,
        );
        let green = if t.dark { hexa(0x30, 0xD1, 0x58) } else { hexa(0x1F, 0xA8, 0x4A) };
        let red = if t.dark { hexa(0xFF, 0x45, 0x3A) } else { hexa(0xE5, 0x30, 0x2A) };
        let blue = system_blue(&t);
        let caret_on = (now * 2.0).fract() < 0.55;
        let shake_x = st.shake * 0.06;
        let chars: Vec<char> = self.value.chars().collect();
        for i in 0..n {
            let r = slot_rect(i).translate(vec2(shake_x, 0.0));
            let p = st.pop[i];
            let rr = Rect::from_center_size(r.center() - vec2(0.0, p * 2.0), r.size() * (1.0 + p * 0.04));
            let (fill, stroke) = match st.status {
                1 => (mix(recess(&t), green, 0.14), Some(green)),
                2 => (mix(recess(&t), red, 0.12), Some(red)),
                _ if st.focused && i == len.min(n - 1) => (surface(&t), Some(blue)),
                _ => (recess(&t), None),
            };
            painter.rect_filled(rr, cr(9.0), fill);
            if let Some(s) = stroke {
                painter.rect_stroke(rr, cr(9.0), egui::Stroke::new(1.5, s), egui::StrokeKind::Inside);
            }
            if let Some(ch) = chars.get(i) {
                let col = match st.status {
                    1 => green,
                    2 => red,
                    _ => fg,
                };
                painter.text(
                    rr.center() - vec2(0.0, p * 3.0),
                    Align2::CENTER_CENTER,
                    ch,
                    FontId::proportional(22.0 * (1.0 + p * 0.08)),
                    col,
                );
            } else if st.focused && i == len && caret_on && st.status == 0 {
                painter.line_segment(
                    [rr.center() - vec2(0.0, 11.0), rr.center() + vec2(0.0, 11.0)],
                    egui::Stroke::new(1.6, blue),
                );
            }
            if i + 1 == half {
                let dx = slot_rect(i).max.x + dash / 2.0 + shake_x - gap / 2.0 + 2.0;
                painter.line_segment(
                    [pos2(dx - 4.0, row_y), pos2(dx + 4.0, row_y)],
                    egui::Stroke::new(1.5, t.muted_foreground),
                );
            }
        }
        let status_y = row_y + 46.0;
        match st.status {
            1 => {
                let a = ramp(now - st.status_t, 0.0, 0.25);
                glyph(&painter, Glyph::Check, pos2(rect.center().x - 34.0, status_y), 12.0, with_alpha(green, a));
                text(
                    &painter,
                    pos2(rect.center().x - 24.0, status_y),
                    Align2::LEFT_CENTER,
                    "Verified",
                    13.0,
                    with_alpha(green, a),
                );
            }
            2 => {
                text(
                    &painter,
                    pos2(rect.center().x, status_y),
                    Align2::CENTER_CENTER,
                    "That code didn't work",
                    12.5,
                    red,
                );
            }
            _ => {
                text(&painter, pos2(rect.center().x, status_y), Align2::CENTER_CENTER, "Resend code", 12.5, blue);
            }
        }

        // keypad + suggestion strip
        if st.kb > 0.01 {
            let kb_bg = if t.dark { hexa(0x1C, 0x1C, 0x1F) } else { hexa(0xD5, 0xD7, 0xDD) };
            let pane = Rect::from_min_max(bar.min, rect.max);
            painter.rect_filled(pane, cr(0.0), kb_bg);
            let key_fill = if t.dark { hexa(0x46, 0x46, 0x4B) } else { Color32::WHITE };
            for (k, label) in KEYS.iter().enumerate() {
                if label.is_empty() {
                    continue;
                }
                let r = key_rect(k);
                let f = st.key_flash[k];
                let fill = if k == 11 { kb_bg } else { mix(key_fill, kb_bg, f * 0.6) };
                if k != 11 {
                    painter.rect_filled(r.translate(vec2(0.0, 1.0)), cr(6.0), with_alpha(Color32::BLACK, 0.18));
                }
                painter.rect_filled(r, cr(6.0), fill);
                painter.text(r.center(), Align2::CENTER_CENTER, label, FontId::proportional(20.0), fg);
            }
            // suggestion
            let ready_a = if sugg_ready { ramp(now - st.sugg_since, 0.5, 0.75) } else { 0.0 };
            if let (Some(code), true) = (sugg_code, ready_a > 0.01) {
                let c = bar.center() + vec2(0.0, (1.0 - ready_a) * 8.0);
                text(
                    &painter,
                    c - vec2(0.0, 9.0),
                    Align2::CENTER_CENTER,
                    "From Messages",
                    10.5,
                    with_alpha(t.muted_foreground, ready_a),
                );
                let spaced: String = code.chars().flat_map(|c| [c, '\u{2009}']).collect();
                painter.text(
                    c + vec2(0.0, 8.0),
                    Align2::CENTER_CENTER,
                    spaced.trim_end(),
                    FontId::monospace(15.0),
                    with_alpha(fg, ready_a),
                );
            }
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}
