use crate::util;
use egui::{pos2, vec2, Align, Color32, CornerRadius, Rect, Response, Sense, Ui, Widget};
use glowkit_core::{Clock, Theme};
use std::f32::consts::TAU;

/// Shape of the typing cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CursorStyle {
    /// A thin vertical bar.
    #[default]
    Bar,
    /// A solid block, like a terminal.
    Block,
    /// An underscore.
    Underscore,
    /// No cursor.
    None,
}

/// Where the typewriter is in its timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TypeState {
    /// Which string is showing.
    pub index: usize,
    /// How many characters of it are visible.
    pub chars: usize,
    /// True while characters are being typed or deleted (cursor stays solid).
    pub busy: bool,
}

/// Timing for [`type_state`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct TypeTiming {
    pub type_cps: f32,
    pub delete_cps: f32,
    pub hold: f32,
    pub gap: f32,
    pub looping: bool,
}

/// Pure typewriter timeline: for each string, type it, hold, delete it, wait, then the next one.
/// Without looping the last string stays typed.
pub(crate) fn type_state(t: f32, lens: &[usize], tm: TypeTiming) -> TypeState {
    if lens.is_empty() {
        return TypeState { index: 0, chars: 0, busy: false };
    }
    let ty = |n: usize| n as f32 / tm.type_cps.max(0.1);
    let de = |n: usize| n as f32 / tm.delete_cps.max(0.1);
    let cycle: f32 = lens.iter().map(|&n| ty(n) + tm.hold + de(n) + tm.gap).sum();
    let mut t = t.max(0.0);
    if tm.looping {
        t %= cycle.max(1e-3);
    }
    for (i, &n) in lens.iter().enumerate() {
        let last = i + 1 == lens.len();
        if t < ty(n) {
            return TypeState { index: i, chars: ((t * tm.type_cps) as usize).min(n), busy: true };
        }
        t -= ty(n);
        if !tm.looping && last {
            return TypeState { index: i, chars: n, busy: false };
        }
        if t < tm.hold {
            return TypeState { index: i, chars: n, busy: false };
        }
        t -= tm.hold;
        if t < de(n) {
            let gone = (t * tm.delete_cps) as usize;
            return TypeState { index: i, chars: n.saturating_sub(gone), busy: true };
        }
        t -= de(n);
        if t < tm.gap {
            return TypeState { index: i, chars: 0, busy: false };
        }
        t -= tm.gap;
    }
    let i = lens.len() - 1;
    TypeState { index: i, chars: 0, busy: false }
}

/// Types out one or more strings with a blinking cursor, deleting and retyping in a loop.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::TypewriterText::new(["Build fast.", "Ship often.", "Sleep well."])
///     .prefix("We help you ")
///     .size(36.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TypewriterText {
    strings: Vec<String>,
    prefix: String,
    size: f32,
    color: Option<Color32>,
    prefix_color: Option<Color32>,
    colors: Vec<Color32>,
    cursor_color: Option<Color32>,
    cursor: CursorStyle,
    bold: bool,
    mono: bool,
    timing: TypeTiming,
    blink: f32,
    align: Align,
}

impl TypewriterText {
    /// Type the given strings in order.
    pub fn new<S: Into<String>>(strings: impl IntoIterator<Item = S>) -> Self {
        Self {
            strings: strings.into_iter().map(Into::into).collect(),
            prefix: String::new(),
            size: 32.0,
            color: None,
            prefix_color: None,
            colors: Vec::new(),
            cursor_color: None,
            cursor: CursorStyle::Bar,
            bold: true,
            mono: false,
            timing: TypeTiming { type_cps: 14.0, delete_cps: 30.0, hold: 1.6, gap: 0.35, looping: true },
            blink: 1.0,
            align: Align::Center,
        }
    }
    /// Static text shown before the typed part (e.g. `"I build "`).
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Typed text color (defaults to the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Prefix color (defaults to the theme muted foreground).
    pub fn prefix_color(mut self, c: Color32) -> Self {
        self.prefix_color = Some(c);
        self
    }
    /// One color per string (cycled); overrides [`Self::color`].
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    /// Cursor color (defaults to the theme accent).
    pub fn cursor_color(mut self, c: Color32) -> Self {
        self.cursor_color = Some(c);
        self
    }
    /// Cursor shape.
    pub fn cursor(mut self, style: CursorStyle) -> Self {
        self.cursor = style;
        self
    }
    /// Fake-bold the typed text (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Use the monospace font.
    pub fn monospace(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }
    /// Characters typed per second.
    pub fn typing_speed(mut self, cps: f32) -> Self {
        self.timing.type_cps = cps.max(0.1);
        self
    }
    /// Characters deleted per second.
    pub fn deleting_speed(mut self, cps: f32) -> Self {
        self.timing.delete_cps = cps.max(0.1);
        self
    }
    /// Seconds a fully typed string stays before being deleted.
    pub fn hold(mut self, seconds: f32) -> Self {
        self.timing.hold = seconds.max(0.0);
        self
    }
    /// Seconds of empty line before the next string starts.
    pub fn gap(mut self, seconds: f32) -> Self {
        self.timing.gap = seconds.max(0.0);
        self
    }
    /// Loop forever (default) or stop after typing the last string.
    pub fn looping(mut self, looping: bool) -> Self {
        self.timing.looping = looping;
        self
    }
    /// Seconds per cursor blink (0 = no blink).
    pub fn blink(mut self, seconds: f32) -> Self {
        self.blink = seconds.max(0.0);
        self
    }
    /// Horizontal alignment inside the widest-string box (default: centered).
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }
}

impl Widget for TypewriterText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, self.mono);
        let bold = util::weight(self.bold, self.size);
        let painter = ui.painter().clone();
        let prefix = util::layout(&painter, self.prefix.clone(), &font);
        let widest = self.strings.iter().map(|s| util::layout(&painter, s.clone(), &font).size().x).fold(0.0, f32::max);
        let line_h = prefix.size().y.max(self.size * 1.2);
        let cursor_w = match self.cursor {
            CursorStyle::Bar => (self.size * 0.07).max(2.0),
            CursorStyle::Block => self.size * 0.55,
            CursorStyle::Underscore => self.size * 0.55,
            CursorStyle::None => 0.0,
        };
        let gap = if self.cursor == CursorStyle::None { 0.0 } else { self.size * 0.08 };
        let full_w = prefix.size().x + widest + bold + gap + cursor_w;
        let (rect, resp) = ui.allocate_exact_size(vec2(full_w, line_h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            glowkit_core::animate(ui);
            return resp;
        }
        let elapsed = Clock::elapsed(ui, resp.id);
        let lens: Vec<usize> = self.strings.iter().map(|s| s.chars().count()).collect();
        let st = type_state(elapsed, &lens, self.timing);
        let text: String = self.strings.get(st.index).map(|s| s.chars().take(st.chars).collect()).unwrap_or_default();
        let typed = util::layout(&painter, text, &font);
        let content_w = prefix.size().x + typed.size().x + bold + gap + cursor_w;
        let x0 = match self.align {
            Align::Min => rect.min.x,
            Align::Center => rect.center().x - content_w / 2.0,
            Align::Max => rect.max.x - content_w,
        };
        let y0 = rect.min.y + (line_h - prefix.size().y) / 2.0;
        let pcol = self.prefix_color.unwrap_or(t.muted_foreground);
        util::paint_galley(&painter, pos2(x0, y0), &prefix, pcol, bold);
        let col = if self.colors.is_empty() { self.color.unwrap_or(t.foreground) } else { self.colors[st.index % self.colors.len()] };
        let tx = x0 + prefix.size().x;
        util::paint_galley(&painter, pos2(tx, y0), &typed, col, bold);

        // cursor
        let ccol = self.cursor_color.unwrap_or(t.accent);
        let blink = if st.busy || self.blink <= 0.0 {
            1.0
        } else {
            let p = (elapsed / self.blink).fract();
            // mostly on, quick smooth fade off
            (0.5 + 0.5 * (p * TAU).cos()).powf(0.35)
        };
        let cx = tx + typed.size().x + bold + gap;
        let fh = self.size;
        let cy = y0 + (prefix.size().y - fh) / 2.0;
        let c = ccol.gamma_multiply(blink);
        match self.cursor {
            CursorStyle::Bar => {
                let r = Rect::from_min_size(pos2(cx, cy + fh * 0.04), vec2(cursor_w, fh * 1.0));
                painter.rect_filled(r, CornerRadius::same((cursor_w / 2.0) as u8), c);
            }
            CursorStyle::Block => {
                let r = Rect::from_min_size(pos2(cx, cy + fh * 0.08), vec2(cursor_w, fh * 0.95));
                painter.rect_filled(r, CornerRadius::same(2), c.gamma_multiply(0.9));
            }
            CursorStyle::Underscore => {
                let r = Rect::from_min_size(pos2(cx, cy + fh * 0.98), vec2(cursor_w, (fh * 0.08).max(2.0)));
                painter.rect_filled(r, CornerRadius::same(1), c);
            }
            CursorStyle::None => {}
        }
        glowkit_core::animate(ui);
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TM: TypeTiming = TypeTiming { type_cps: 10.0, delete_cps: 20.0, hold: 1.0, gap: 0.5, looping: true };

    #[test]
    fn types_holds_deletes() {
        let lens = [5, 3];
        assert_eq!(type_state(0.0, &lens, TM), TypeState { index: 0, chars: 0, busy: true });
        assert_eq!(type_state(0.25, &lens, TM).chars, 2);
        // typed after 0.5s, holding
        assert_eq!(type_state(0.9, &lens, TM), TypeState { index: 0, chars: 5, busy: false });
        // deleting (1.5 .. 1.75)
        let d = type_state(1.6, &lens, TM);
        assert!(d.busy && d.chars < 5);
        // gap
        assert_eq!(type_state(1.8, &lens, TM).chars, 0);
        // second string starts at 2.25
        assert_eq!(type_state(2.3, &lens, TM).index, 1);
    }

    #[test]
    fn loops_and_stops() {
        let lens = [4];
        let cycle = 0.4 + 1.0 + 0.2 + 0.5;
        assert_eq!(type_state(cycle + 0.05, &lens, TM).chars, 0);
        let once = TypeTiming { looping: false, ..TM };
        assert_eq!(type_state(100.0, &lens, once), TypeState { index: 0, chars: 4, busy: false });
        assert_eq!(type_state(1.0, &[], TM).chars, 0);
    }
}
