//! Number transitions: per-digit pop-in, a subtle cross-fade, and a rolling drum counter.

use super::common::{change, layer, lerp, span, spring_at, xf_around, MotionCurve};
use egui::emath::TSTransform;
use egui::{pos2, vec2, Align2, Color32, FontId, Response, Sense, Ui, Widget};
use glowkit_core::text::Glyphs;
use glowkit_core::Theme;

/// Group the integer part of a decimal string with commas.
pub(crate) fn group_thousands(s: &str) -> String {
    let (sign, rest) = if let Some(r) = s.strip_prefix('-') { ("-", r) } else { ("", s) };
    let (int, frac) = match rest.find('.') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let mut out = String::new();
    for (i, ch) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    format!("{sign}{out}{frac}")
}

fn char_glyphs(ui: &Ui, text: &str, font: &FontId) -> Glyphs {
    Glyphs::layout(ui.painter(), text, font.clone(), None)
}

// ---------------------------------------------------------------------------------------------

/// A number (or any short string) whose characters pop when it changes: the old characters
/// sink away upward, shrinking and blurring one after another, then the new ones rise from below,
/// small and soft, overshoot a touch above the baseline and settle with a spring, staggered from
/// left to right.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let price = 65.78_f32;
/// ui.add(glowkit_interactions::PopNumber::new(format!("{price:.2}")).size(40.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PopNumber {
    text: String,
    size: f32,
    color: Option<Color32>,
    dur: f32,
    exit_dur: f32,
    stagger: f32,
    enter_delay: f32,
    distance: f32,
    start_scale: f32,
    peak: f32,
}

impl PopNumber {
    /// Show `text`; changing it plays the transition.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 36.0,
            color: None,
            dur: 0.6,
            exit_dur: 0.155,
            stagger: 0.04,
            enter_delay: 0.05,
            distance: 0.52,
            start_scale: 0.7,
            peak: 1.03,
        }
    }
    /// Font size in points (default 36).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(6.0);
        self
    }
    /// Text color (default: the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Seconds each new character takes to rise and settle (default 0.6).
    pub fn duration(mut self, s: f32) -> Self {
        self.dur = s.max(0.05);
        self
    }
    /// Seconds each old character takes to leave (default 0.155).
    pub fn exit_duration(mut self, s: f32) -> Self {
        self.exit_dur = s.max(0.02);
        self
    }
    /// Delay between neighbouring characters in seconds (default 0.04).
    pub fn stagger(mut self, s: f32) -> Self {
        self.stagger = s.max(0.0);
        self
    }
    /// Travel distance in ems (default 0.52).
    pub fn distance(mut self, em: f32) -> Self {
        self.distance = em;
        self
    }
    /// Scale a character starts from (default 0.7).
    pub fn start_scale(mut self, s: f32) -> Self {
        self.start_scale = s.clamp(0.0, 1.0);
        self
    }
    /// Peak scale of the settle overshoot (default 1.03).
    pub fn peak(mut self, s: f32) -> Self {
        self.peak = s.max(1.0);
        self
    }
}

impl Widget for PopNumber {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = FontId::proportional(self.size);
        let em = self.size;
        let new_g = char_glyphs(ui, &self.text, &font);
        let size = vec2(new_g.size.x.max(em) + em * 0.4, em * 1.6);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let n = self.text.chars().count();
        let horizon = self.enter_delay + self.stagger * n as f32 + self.dur + 0.05;
        let ch = change(ui, response.id, &self.text, horizon);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let color = self.color.unwrap_or(t.foreground);
        let blur = (0.171 * em - 2.1).max(0.067 * em);
        let center_line = rect.center().y;
        let paint_text = |g: &Glyphs, ui: &Ui, f: &dyn Fn(usize) -> (f32, f32, f32, f32)| {
            let x0 = rect.center().x - g.size.x / 2.0;
            let y0 = center_line - g.size.y / 2.0;
            for (i, gb) in g.glyphs.iter().enumerate() {
                if gb.ch.is_whitespace() {
                    continue;
                }
                let (dy, scale, alpha, bl) = f(i);
                let p = pos2(x0 + gb.offset.x, y0 + gb.offset.y);
                let c = p + gb.size / 2.0;
                layer(ui, xf_around(c, scale, vec2(0.0, dy)), alpha, bl, |pt| {
                    pt.text(p, Align2::LEFT_TOP, gb.ch, font.clone(), color);
                });
            }
        };
        let age = ch.age;
        if age >= horizon {
            paint_text(&new_g, ui, &|_| (0.0, 1.0, 1.0, 0.0));
            return response;
        }
        // leaving characters
        if ch.prev != self.text {
            let old_g = char_glyphs(ui, &ch.prev, &font);
            paint_text(&old_g, ui, &|i| {
                let k = ((age - self.stagger * i as f32) / self.exit_dur).clamp(0.0, 1.0);
                let e = glowkit_core::ease(glowkit_core::Easing::InOutCubic, k);
                let a = 1.0 - glowkit_core::ease(glowkit_core::Easing::InOutQuad, span(k, 0.4, 0.85));
                (-self.distance * em * e, 1.0 + (self.start_scale - 1.0) * e, a, blur * e)
            });
        }
        // arriving characters
        let omega = 22.0 * 0.6 / self.dur;
        paint_text(&new_g, ui, &|i| {
            let local = age - self.enter_delay - self.stagger * i as f32;
            let k = (local / self.dur).clamp(0.0, 1.0);
            if local <= 0.0 {
                return (0.0, 1.0, 0.0, 0.0);
            }
            let rise = 1.0 - glowkit_core::ease(glowkit_core::Easing::InOutSine, k / 0.3);
            let (lift, g) = if k < 0.15 {
                let x = k / 0.15;
                (x * x * (3.0 - 2.0 * x), 1.0)
            } else {
                let g = 1.0 - spring_at(local - 0.15 * self.dur, omega, 0.38);
                (g, g)
            };
            let scale = if g >= 0.0 {
                1.0 + (self.start_scale - 1.0) * g
            } else {
                1.0 + (self.peak - 1.0) * (-g / 0.27).min(1.3)
            };
            let dy = (self.distance * rise + 0.125 * lift) * em;
            let a = span(k, 0.0, 0.3).powf(0.8);
            let bl = blur * (1.0 - span(k, 0.15, 0.6));
            (dy, scale, a, bl)
        });
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A number that changes quietly: the old value drifts a few points away and blurs out while the
/// new one drifts in from the other side, upward for increases and downward for decreases.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::SubtleNumber::new(1284.0).prefix("$").size(40.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SubtleNumber {
    value: f64,
    decimals: usize,
    prefix: String,
    suffix: String,
    size: f32,
    color: Option<Color32>,
    dur: f32,
    distance: f32,
    curve: MotionCurve,
    per_digit: bool,
}

impl SubtleNumber {
    /// Show `value` (grouped with commas).
    pub fn new(value: f64) -> Self {
        Self {
            value,
            decimals: 0,
            prefix: String::new(),
            suffix: String::new(),
            size: 36.0,
            color: None,
            dur: 0.455,
            distance: 0.14,
            curve: MotionCurve::SMOOTH_OUT,
            per_digit: false,
        }
    }
    /// Decimal places (default 0).
    pub fn decimals(mut self, n: usize) -> Self {
        self.decimals = n;
        self
    }
    /// Text before the number, e.g. `"$"`.
    pub fn prefix(mut self, s: impl Into<String>) -> Self {
        self.prefix = s.into();
        self
    }
    /// Text after the number, e.g. `" pts"`.
    pub fn suffix(mut self, s: impl Into<String>) -> Self {
        self.suffix = s.into();
        self
    }
    /// Font size in points (default 36).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(6.0);
        self
    }
    /// Text color (default: the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Transition length in seconds (default 0.455).
    pub fn duration(mut self, s: f32) -> Self {
        self.dur = s.max(0.02);
        self
    }
    /// Drift distance in ems (default 0.14).
    pub fn distance(mut self, em: f32) -> Self {
        self.distance = em;
        self
    }
    /// Timing curve (default [`MotionCurve::SMOOTH_OUT`]).
    pub fn curve(mut self, c: MotionCurve) -> Self {
        self.curve = c;
        self
    }
    /// Only animate the characters that actually changed (default: the whole number moves).
    pub fn per_digit(mut self, on: bool) -> Self {
        self.per_digit = on;
        self
    }
    fn format(&self, v: f64) -> String {
        format!("{}{}{}", self.prefix, group_thousands(&format!("{v:.*}", self.decimals)), self.suffix)
    }
}

impl Widget for SubtleNumber {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = FontId::proportional(self.size);
        let em = self.size;
        let text = self.format(self.value);
        let g = char_glyphs(ui, &text, &font);
        let (rect, response) = ui.allocate_exact_size(vec2(g.size.x + em * 0.6, em * 1.5), Sense::hover());
        let ch = change(ui, response.id, &self.value.to_bits(), self.dur + 0.05);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let color = self.color.unwrap_or(t.foreground);
        let k = self.curve.at(ch.age / self.dur);
        let prev = f64::from_bits(ch.prev);
        let dir = if self.value >= prev { 1.0 } else { -1.0 };
        let d = self.distance * em * dir;
        let blur = 0.042 * em;
        let old_text = self.format(prev);
        let og = char_glyphs(ui, &old_text, &font);
        let place = |g: &Glyphs| pos2(rect.center().x - g.size.x / 2.0, rect.center().y - g.size.y / 2.0);
        if k >= 1.0 || old_text == text {
            ui.painter().galley(place(&g), g.galley.clone(), color);
            return response;
        }
        if self.per_digit && og.glyphs.len() == g.glyphs.len() {
            let (pn, po) = (place(&g), place(&og));
            for (a, b) in g.glyphs.iter().zip(og.glyphs.iter()) {
                if a.ch == b.ch {
                    ui.painter().text(pn + a.offset, Align2::LEFT_TOP, a.ch, font.clone(), color);
                    continue;
                }
                layer(ui, TSTransform::from_translation(vec2(0.0, -d * k)), 1.0 - k, blur * k, |p| {
                    p.text(po + b.offset, Align2::LEFT_TOP, b.ch, font.clone(), color);
                });
                layer(ui, TSTransform::from_translation(vec2(0.0, d * (1.0 - k))), k, blur * (1.0 - k), |p| {
                    p.text(pn + a.offset, Align2::LEFT_TOP, a.ch, font.clone(), color);
                });
            }
            return response;
        }
        layer(ui, TSTransform::from_translation(vec2(0.0, -d * k)), 1.0 - k, blur * k, |p| {
            p.galley(place(&og), og.galley.clone(), color);
        });
        layer(ui, TSTransform::from_translation(vec2(0.0, d * (1.0 - k))), k, blur * (1.0 - k), |p| {
            p.galley(place(&g), g.galley.clone(), color);
        });
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A counter whose digits sit on little drums. On change each column rolls through the numbers
/// to its new digit (the neighbours above and below ghosting past, small and blurred), the
/// columns starting one after another; the arriving digit pulls back a hair, then a spring snaps
/// it into place with a small bounce past full size.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::DrumCounter::new(178).size(48.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DrumCounter {
    value: i64,
    size: f32,
    color: Option<Color32>,
    step: f32,
    stagger: f32,
    spins: u32,
    land: f32,
    group: bool,
}

impl DrumCounter {
    /// Show `value`.
    pub fn new(value: i64) -> Self {
        Self { value, size: 44.0, color: None, step: 0.13, stagger: 0.09, spins: 0, land: 0.74, group: true }
    }
    /// Font size in points (default 44).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(6.0);
        self
    }
    /// Text color (default: the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Seconds per number rolled past (default 0.13).
    pub fn step(mut self, s: f32) -> Self {
        self.step = s.max(0.01);
        self
    }
    /// Delay between columns in seconds (default 0.09).
    pub fn stagger(mut self, s: f32) -> Self {
        self.stagger = s.max(0.0);
        self
    }
    /// Extra full turns each changing column makes (default 0).
    pub fn spins(mut self, n: u32) -> Self {
        self.spins = n;
        self
    }
    /// Length of the landing spring in seconds (default 0.74).
    pub fn land(mut self, s: f32) -> Self {
        self.land = s.max(0.1);
        self
    }
    /// Group thousands with commas (default on).
    pub fn group(mut self, on: bool) -> Self {
        self.group = on;
        self
    }
    fn format(&self, v: i64) -> String {
        if self.group {
            group_thousands(&v.to_string())
        } else {
            v.to_string()
        }
    }
}

const ENTER: f32 = 0.12;
const ANTIC: f32 = 0.1;

impl Widget for DrumCounter {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = FontId::proportional(self.size);
        let em = self.size;
        let text = self.format(self.value);
        let digit_w = (0..10).map(|d| super::common::text_width(ui.painter(), &d.to_string(), &font)).fold(0.0, f32::max);
        let sep_w = super::common::text_width(ui.painter(), ",", &font);
        let cells: Vec<char> = text.chars().collect();
        let width_of = |cs: &[char]| cs.iter().map(|c| if c.is_ascii_digit() { digit_w } else { sep_w }).sum::<f32>();
        let (rect, response) = ui.allocate_exact_size(vec2(width_of(&cells) + em * 0.8, em * 2.4), Sense::hover());
        let ch = change(ui, response.id, &self.value, 0.0);
        let old: Vec<char> = self.format(ch.prev).chars().collect();
        // align old to new from the right
        let n = cells.len();
        let old_at = |i: usize| -> Option<char> {
            let off = n as isize - old.len() as isize;
            let j = i as isize - off;
            if j >= 0 && (j as usize) < old.len() {
                Some(old[j as usize])
            } else {
                None
            }
        };
        let spins = self.spins as f32 * 10.0;
        let roll_ease = MotionCurve::new(0.35, -0.02, 0.43, 1.0);
        let mut longest = 0.0f32;
        let mut col = 0usize;
        let mut plans = Vec::with_capacity(n);
        for (i, c) in cells.iter().enumerate() {
            if let Some(d) = c.to_digit(10) {
                let from = old_at(i).and_then(|o| o.to_digit(10)).unwrap_or(0);
                let steps = ((d + 10 - from) % 10) as f32 + if d != from || old_at(i).is_none() { spins } else { 0.0 };
                let delay = self.stagger * col as f32;
                col += 1;
                let total = delay + ENTER + steps * self.step + ANTIC + self.land;
                if steps > 0.0 {
                    longest = longest.max(total);
                }
                plans.push(Some((from as f32, steps, delay, old_at(i).is_none())));
            } else {
                plans.push(None);
            }
        }
        if ch.age < longest + 0.05 {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let color = self.color.unwrap_or(t.foreground);
        let painter = ui.painter();
        let mut x = rect.center().x - width_of(&cells) / 2.0;
        let cy = rect.center().y;
        let side_off = 0.52 * em;
        let drum_step = 50f32.to_radians();
        let radius = side_off / drum_step.sin();
        for (i, c) in cells.iter().enumerate() {
            let w = if c.is_ascii_digit() { digit_w } else { sep_w };
            let cx = x + w / 2.0;
            x += w;
            let Some((from, steps, delay, fresh)) = plans[i] else {
                painter.text(pos2(cx, cy), Align2::CENTER_CENTER, *c, font.clone(), color);
                continue;
            };
            let target = c.to_digit(10).unwrap_or(0) as f32;
            let local = ch.age - delay;
            if steps <= 0.0 || local >= ENTER + steps * self.step + ANTIC + self.land {
                painter.text(pos2(cx, cy), Align2::CENTER_CENTER, *c, font.clone(), color);
                continue;
            }
            let roll_t = steps * self.step;
            // rolling look (0 at rest, 1 while spinning)
            let (pos, look, col_scale, col_dy) = if local < ENTER {
                (from, span(local, 0.0, ENTER), 1.0, 0.0)
            } else if local < ENTER + roll_t {
                let u = (local - ENTER) / roll_t;
                (from + steps * roll_ease.at(u), 1.0, 1.0, 0.0)
            } else if local < ENTER + roll_t + ANTIC {
                let u = glowkit_core::ease(glowkit_core::Easing::OutCubic, (local - ENTER - roll_t) / ANTIC);
                (from + steps, 1.0, lerp(1.0, 0.94, u), 0.125 * em * u)
            } else {
                let tau = local - ENTER - roll_t - ANTIC;
                let omega = 27.0 * 0.74 / self.land;
                let w = 1.0 - spring_at(tau, omega, 0.22);
                let look = 1.0 - span(tau, 0.0, 0.2);
                (from + steps, look, 1.0 + (0.94 - 1.0) * w, 0.125 * em * w)
            };
            let center_scale = lerp(1.0, 0.9, look);
            let fade_in = if fresh { span(local, 0.0, ENTER + roll_t * 0.5) } else { 1.0 };
            for k in 0..10 {
                let mut dd = (k as f32 - pos).rem_euclid(10.0);
                if dd > 5.0 {
                    dd -= 10.0;
                }
                let ad = dd.abs();
                if ad > 1.6 {
                    continue;
                }
                if (k as f32 - target).abs() > 0.5 && look <= 0.001 {
                    continue;
                }
                let ang = (dd * drum_step).clamp(-1.5, 1.5);
                let y = radius * ang.sin();
                let near = ad.min(1.0);
                let s = lerp(center_scale, 0.7, near) * col_scale;
                let a = lerp(1.0, 0.17, near) * if ad > 0.5 { look } else { 1.0 } * (1.0 - span(ad, 1.0, 1.6)) * fade_in;
                let bl = lerp(0.042, 0.083, near) * em * look;
                let center = pos2(cx, cy + y + col_dy);
                layer(ui, xf_around(center, s, vec2(0.0, 0.0)), a, bl, |p| {
                    p.text(center, Align2::CENTER_CENTER, char::from(b'0' + k as u8), font.clone(), color);
                });
            }
        }
        response
    }
}
