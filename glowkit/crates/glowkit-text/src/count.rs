use crate::util;
use egui::{pos2, vec2, Color32, FontId, Painter, Response, Sense, Ui, Widget};
use glowkit_core::{ease, Clock, Easing, Theme, Trigger};

/// Format `value` with `decimals` fraction digits, a thousands `separator` (may be empty) and a
/// `decimal_point`. Negative zero is printed as zero.
///
/// ```
/// assert_eq!(glowkit_text::format_number(1234567.891, 2, ",", "."), "1,234,567.89");
/// assert_eq!(glowkit_text::format_number(-9876.6, 0, " ", "."), "-9 877");
/// ```
pub fn format_number(value: f64, decimals: usize, separator: &str, decimal_point: &str) -> String {
    let s = format!("{:.*}", decimals, value.abs());
    let (int, frac) = match s.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (s.as_str(), None),
    };
    let is_zero = s.bytes().all(|b| b == b'0' || b == b'.');
    let mut out = String::with_capacity(s.len() + 8);
    if value < 0.0 && !is_zero {
        out.push('-');
    }
    let n = int.len();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (n - i) % 3 == 0 {
            out.push_str(separator);
        }
        out.push(c);
    }
    if let Some(f) = frac {
        out.push_str(decimal_point);
        out.push_str(f);
    }
    out
}

/// Paint `s` with digits in fixed-width slots (tabular figures) so numbers don't wobble.
/// Returns the width. With `paint == None` only measures.
pub(crate) fn tabular(painter: &Painter, s: &str, font: &FontId, slot: f32, at: Option<(egui::Pos2, Color32, f32)>) -> f32 {
    let mut x = 0.0;
    for ch in s.chars() {
        let g = util::layout_char(painter, ch, font);
        let w = if ch.is_ascii_digit() { slot } else { g.size().x };
        if let Some((pos, color, bold)) = at {
            util::paint_galley(painter, pos2(pos.x + x + (w - g.size().x) / 2.0, pos.y), &g, color, bold);
        }
        x += w;
    }
    x
}

#[derive(Clone, Copy)]
struct CountState {
    from: f64,
    to: f64,
    shown: f64,
}

/// A number that counts up (or down) to its target with easing, thousands separators, decimals
/// and an optional prefix/suffix. Changing the target later animates from the current value.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::CountUp::new(12_480.0).prefix("$").decimals(2).size(56.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CountUp {
    to: f64,
    from: f64,
    duration: f32,
    easing: Easing,
    decimals: usize,
    separator: String,
    decimal_point: String,
    prefix: String,
    suffix: String,
    size: f32,
    color: Option<Color32>,
    affix_color: Option<Color32>,
    bold: bool,
    trigger: Trigger,
}

impl CountUp {
    /// Count to `to`.
    pub fn new(to: f64) -> Self {
        Self {
            to,
            from: 0.0,
            duration: 2.2,
            easing: Easing::OutExpo,
            decimals: 0,
            separator: ",".into(),
            decimal_point: ".".into(),
            prefix: String::new(),
            suffix: String::new(),
            size: 48.0,
            color: None,
            affix_color: None,
            bold: true,
            trigger: Trigger::OnAppear,
        }
    }
    /// Starting value (default 0). Count down by making it larger than the target.
    pub fn from(mut self, from: f64) -> Self {
        self.from = from;
        self
    }
    /// Seconds to reach the target.
    pub fn duration(mut self, seconds: f32) -> Self {
        self.duration = seconds.max(0.01);
        self
    }
    /// Easing curve (default `OutExpo`, fast then gently settling).
    pub fn easing(mut self, e: Easing) -> Self {
        self.easing = e;
        self
    }
    /// Digits after the decimal point.
    pub fn decimals(mut self, n: usize) -> Self {
        self.decimals = n.min(10);
        self
    }
    /// Thousands separator (`""` for none).
    pub fn separator(mut self, s: impl Into<String>) -> Self {
        self.separator = s.into();
        self
    }
    /// Decimal point string (e.g. `","` for European formatting).
    pub fn decimal_point(mut self, s: impl Into<String>) -> Self {
        self.decimal_point = s.into();
        self
    }
    /// Text before the number (e.g. `"$"`).
    pub fn prefix(mut self, s: impl Into<String>) -> Self {
        self.prefix = s.into();
        self
    }
    /// Text after the number (e.g. `"%"` or `"+"`).
    pub fn suffix(mut self, s: impl Into<String>) -> Self {
        self.suffix = s.into();
        self
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Number color (defaults to the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Prefix/suffix color (defaults to the number color).
    pub fn affix_color(mut self, c: Color32) -> Self {
        self.affix_color = Some(c);
        self
    }
    /// Fake-bold the digits (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// When the count plays.
    pub fn trigger(mut self, t: Trigger) -> Self {
        self.trigger = t;
        self
    }
}

impl Widget for CountUp {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let painter = ui.painter().clone();
        let slot = util::digit_width(&painter, &font);
        let fmt = |v: f64| format_number(v, self.decimals, &self.separator, &self.decimal_point);
        let id = ui.next_auto_id();
        let st = glowkit_core::state(ui, id.with("count"), || CountState { from: self.from, to: self.to, shown: self.from });
        let mut st = st;
        if st.to != self.to {
            st = CountState { from: st.shown, to: self.to, shown: st.shown };
            Clock::restart(ui, id);
        }
        let pre = util::layout(&painter, self.prefix.clone(), &font);
        let suf = util::layout(&painter, self.suffix.clone(), &font);
        let num_w = tabular(&painter, &fmt(st.from), &font, slot, None).max(tabular(&painter, &fmt(st.to), &font, slot, None));
        let h = pre.size().y.max(self.size * 1.2);
        let (rect, resp) = ui.allocate_exact_size(vec2(pre.size().x + num_w + suf.size().x + bold, h), Sense::click());
        let elapsed = Clock::triggered(ui, id, self.trigger, self.duration, resp.hovered(), resp.clicked());
        let k = ease(self.easing, elapsed / self.duration) as f64;
        st.shown = st.from + (st.to - st.from) * k;
        glowkit_core::set_state(ui, id.with("count"), st);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let color = self.color.unwrap_or(t.foreground);
        let acol = self.affix_color.unwrap_or(color);
        let s = fmt(st.shown);
        let w = pre.size().x + tabular(&painter, &s, &font, slot, None) + suf.size().x;
        let x0 = rect.center().x - (w + bold) / 2.0;
        let y0 = rect.center().y - pre.size().y.max(slot) / 2.0;
        let y0 = y0.max(rect.min.y);
        util::paint_galley(&painter, pos2(x0, y0), &pre, acol, bold);
        let nw = tabular(&painter, &s, &font, slot, Some((pos2(x0 + pre.size().x, y0), color, bold)));
        util::paint_galley(&painter, pos2(x0 + pre.size().x + nw, y0), &suf, acol, bold);
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_numbers() {
        assert_eq!(format_number(0.0, 0, ",", "."), "0");
        assert_eq!(format_number(999.0, 0, ",", "."), "999");
        assert_eq!(format_number(1000.0, 0, ",", "."), "1,000");
        assert_eq!(format_number(1234567.0, 0, ",", "."), "1,234,567");
        assert_eq!(format_number(1234.5678, 2, ",", "."), "1,234.57");
        assert_eq!(format_number(1234.5, 1, ".", ","), "1.234,5");
        assert_eq!(format_number(-1234.0, 0, ",", "."), "-1,234");
        assert_eq!(format_number(-0.001, 2, ",", "."), "0.00");
        assert_eq!(format_number(12345.0, 0, "", "."), "12345");
        assert_eq!(format_number(0.5, 3, ",", "."), "0.500");
    }
}
