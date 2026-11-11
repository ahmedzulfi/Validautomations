use crate::util;
use egui::{pos2, vec2, Color32, Rect, Response, Sense, Ui, Widget};
use glowkit_core::{Spring, Theme};

/// Split `value` into its digit columns: returns the integer `scaled = round(|v| * 10^decimals)`
/// and how many integer digits it has (at least `min_digits`, at least 1).
pub(crate) fn digits_of(value: f64, decimals: usize, min_digits: usize) -> (u64, usize) {
    let scaled = (value.abs() * 10f64.powi(decimals as i32)).round().min(1e17) as u64;
    let int = scaled / 10u64.pow(decimals as u32);
    let mut n = 1;
    let mut x = int;
    while x >= 10 {
        x /= 10;
        n += 1;
    }
    (scaled, n.max(min_digits))
}

/// Target roll position of the column for place `k` (0 = units, -1 = tenths...), i.e. the number
/// truncated at that place. Its last digit is the digit shown.
pub(crate) fn column_target(scaled: u64, decimals: usize, k: i32) -> u64 {
    let shift = (k + decimals as i32).max(0) as u32;
    scaled / 10u64.saturating_pow(shift)
}

#[derive(Clone, Copy, Debug)]
struct Column {
    k: i32,
    roll: Spring,
    presence: Spring,
}

#[derive(Clone, Debug, Default)]
struct SlideState {
    cols: Vec<Column>,
    minus: f32,
}

/// An odometer: each digit rolls vertically to its new value with a spring whenever the number
/// changes. Carrying past 9 rolls on to 0 like a real counter; new leading digits slide in.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut count = 0.0;
/// ui.add(glowkit_text::SlidingNumber::new(count).size(64.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SlidingNumber {
    value: f64,
    decimals: usize,
    min_digits: usize,
    size: f32,
    color: Option<Color32>,
    bold: bool,
    stiffness: f32,
    damping: f32,
    prefix: String,
    suffix: String,
}

impl SlidingNumber {
    /// Show `value`.
    pub fn new(value: f64) -> Self {
        Self {
            value,
            decimals: 0,
            min_digits: 1,
            size: 48.0,
            color: None,
            bold: true,
            stiffness: 170.0,
            damping: 20.0,
            prefix: String::new(),
            suffix: String::new(),
        }
    }
    /// Digits after the decimal point.
    pub fn decimals(mut self, n: usize) -> Self {
        self.decimals = n.min(6);
        self
    }
    /// Pad with leading zeros to at least this many integer digits.
    pub fn min_digits(mut self, n: usize) -> Self {
        self.min_digits = n.clamp(1, 15);
        self
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Digit color (defaults to the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Fake-bold the digits (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Spring stiffness and damping for the roll (defaults 170 / 20: a hint of bounce).
    pub fn spring(mut self, stiffness: f32, damping: f32) -> Self {
        self.stiffness = stiffness;
        self.damping = damping;
        self
    }
    /// Static text before the number.
    pub fn prefix(mut self, s: impl Into<String>) -> Self {
        self.prefix = s.into();
        self
    }
    /// Static text after the number.
    pub fn suffix(mut self, s: impl Into<String>) -> Self {
        self.suffix = s.into();
        self
    }
}

impl Widget for SlidingNumber {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let painter = ui.painter().clone();
        let slot = util::digit_width(&painter, &font);
        let line_h = util::layout_char(&painter, '0', &font).size().y;
        let dt = glowkit_core::dt(ui);
        let id = ui.next_auto_id().with("sliding");
        let (scaled, int_digits) = digits_of(self.value, self.decimals, self.min_digits);
        let lo = -(self.decimals as i32);
        let hi = int_digits as i32 - 1;
        let negative = self.value < 0.0 && scaled > 0;
        let (stiff, damp) = (self.stiffness, self.damping);
        let st = glowkit_core::with_state(ui, id, SlideState::default, |s| {
            let fresh = s.cols.is_empty();
            for k in lo..=hi.max(s.cols.iter().map(|c| c.k).max().unwrap_or(hi)) {
                let target = column_target(scaled, self.decimals, k) as f32;
                let present = if k <= hi { 1.0 } else { 0.0 };
                if let Some(c) = s.cols.iter_mut().find(|c| c.k == k) {
                    c.roll.target = target;
                    c.presence.target = present;
                    let diff = target - c.roll.value;
                    if diff.abs() > 10.0 {
                        // keep the displayed digit, skip whole turns of the wheel
                        c.roll.value = target - diff.signum() * (10.0 + diff.abs() % 10.0);
                    }
                } else {
                    let start = if fresh { target } else { target - target % 10.0 };
                    let mut roll = Spring::new(start).with(stiff, damp);
                    roll.target = target;
                    let mut presence = Spring::new(if fresh { 1.0 } else { 0.0 }).with(220.0, 26.0);
                    presence.target = present;
                    s.cols.push(Column { k, roll, presence });
                }
            }
            s.cols.retain(|c| c.k >= lo && (c.k <= hi || c.presence.value > 0.01));
            s.cols.sort_by(|a, b| b.k.cmp(&a.k));
            for c in &mut s.cols {
                c.roll.stiffness = stiff;
                c.roll.damping = damp;
                c.roll.step(dt);
                c.presence.step(dt);
            }
            s.minus = glowkit_core::anim::approach(s.minus, if negative { 1.0 } else { 0.0 }, 14.0, dt);
            s.clone()
        });
        let moving = st.cols.iter().any(|c| !c.roll.is_settled() || !c.presence.is_settled()) || (st.minus - if negative { 1.0 } else { 0.0 }).abs() > 1e-3;
        if moving {
            ui.ctx().request_repaint();
        }

        let pre = util::layout(&painter, self.prefix.clone(), &font);
        let suf = util::layout(&painter, self.suffix.clone(), &font);
        let minus = util::layout_char(&painter, '-', &font);
        let dot = util::layout_char(&painter, '.', &font);
        let cols_w: f32 = st.cols.iter().map(|c| slot * c.presence.value.clamp(0.0, 1.2)).sum();
        let dot_w = if self.decimals > 0 { dot.size().x } else { 0.0 };
        let minus_w = minus.size().x * st.minus;
        let w = pre.size().x + minus_w + cols_w + dot_w + suf.size().x + bold;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, line_h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let color = self.color.unwrap_or(t.foreground);
        let clip = rect.expand2(vec2(bold + 2.0, 0.0)).intersect(ui.clip_rect());
        let p = painter.with_clip_rect(clip);
        let mut x = rect.min.x;
        util::paint_galley(&p, pos2(x, rect.min.y), &pre, color, bold);
        x += pre.size().x;
        util::paint_galley(&p, pos2(x, rect.min.y), &minus, color.gamma_multiply(st.minus), bold);
        x += minus_w;
        for c in &st.cols {
            if c.k == -1 {
                util::paint_galley(&p, pos2(x, rect.min.y), &dot, color, bold);
                x += dot_w;
            }
            let pres = c.presence.value.clamp(0.0, 1.2);
            let cw = slot * pres;
            let col_rect = Rect::from_min_size(pos2(x, rect.min.y), vec2(cw, line_h));
            let base = c.roll.value.floor();
            let frac = c.roll.value - base;
            for j in -1..=2 {
                let digit = ((base as i64 + j).rem_euclid(10)) as u8;
                let dy = (j as f32 - frac) * line_h;
                let a = (1.0 - dy.abs() / line_h).clamp(0.0, 1.0);
                if a <= 0.0 {
                    continue;
                }
                let g = util::layout_char(&p, (b'0' + digit) as char, &font);
                let gx = col_rect.center().x - g.size().x / 2.0;
                // higher digits wait below and roll up into place as the value grows
                util::paint_galley(&p, pos2(gx, rect.min.y + dy), &g, color.gamma_multiply(a * a * pres.min(1.0)), bold);
            }
            x += cw;
        }
        util::paint_galley(&p, pos2(x, rect.min.y), &suf, color, bold);
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digit_columns() {
        assert_eq!(digits_of(0.0, 0, 1), (0, 1));
        assert_eq!(digits_of(1234.0, 0, 1), (1234, 4));
        assert_eq!(digits_of(-12.25, 2, 1), (1225, 2));
        assert_eq!(digits_of(5.0, 0, 3), (5, 3));
        assert_eq!(column_target(1234, 0, 0), 1234);
        assert_eq!(column_target(1234, 0, 2), 12);
        assert_eq!(column_target(1235, 2, -2), 1235);
        assert_eq!(column_target(1235, 2, 0), 12);
        assert_eq!(column_target(1235, 2, 1) % 10, 1);
    }
}
