use crate::util;
use egui::{pos2, vec2, Color32, Rect, Response, Sense, Ui, Widget};
use glowkit_core::{anim::stagger, ease, text::Glyphs, Clock, Easing, Rng, Theme, Trigger};

/// Direction the letter reels travel in [`ShuffleText`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ShuffleDirection {
    /// Reels roll upward (new letters come from below).
    #[default]
    Up,
    /// Reels roll downward.
    Down,
    /// Reels slide to the left.
    Left,
    /// Reels slide to the right.
    Right,
}

/// A random letter of the same kind as `ch` (upper, lower, digit), or `ch` itself.
fn similar(ch: char, r: &mut Rng) -> char {
    const UP: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const LO: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    const DI: &[u8] = b"0123456789";
    let set = if ch.is_ascii_uppercase() {
        UP
    } else if ch.is_ascii_lowercase() {
        LO
    } else if ch.is_ascii_digit() {
        DI
    } else {
        return ch;
    };
    set[r.below(set.len())] as char
}

/// Every letter is a little slot-machine reel of random letters that spins and lands on the real
/// letter, staggered across the word.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::ShuffleText::new("SHUFFLE").size(64.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ShuffleText {
    text: String,
    size: f32,
    color: Option<Color32>,
    bold: bool,
    direction: ShuffleDirection,
    reel: usize,
    duration: f32,
    stagger: f32,
    trigger: Trigger,
}

impl ShuffleText {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 56.0,
            color: None,
            bold: true,
            direction: ShuffleDirection::Up,
            reel: 6,
            duration: 0.9,
            stagger: 0.05,
            trigger: Trigger::OnAppear,
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
    /// Fake-bold the text (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Reel direction.
    pub fn direction(mut self, d: ShuffleDirection) -> Self {
        self.direction = d;
        self
    }
    /// Random letters per reel before the real one (default 6).
    pub fn reel_length(mut self, n: usize) -> Self {
        self.reel = n.clamp(1, 30);
        self
    }
    /// Seconds each reel spins.
    pub fn duration(mut self, seconds: f32) -> Self {
        self.duration = seconds.max(0.05);
        self
    }
    /// Seconds between letters.
    pub fn stagger(mut self, seconds: f32) -> Self {
        self.stagger = seconds.max(0.0);
        self
    }
    /// When the shuffle plays.
    pub fn trigger(mut self, t: Trigger) -> Self {
        self.trigger = t;
        self
    }
}

impl Widget for ShuffleText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let glyphs = Glyphs::layout(ui.painter(), &self.text, font.clone(), None);
        let (rect, resp) = ui.allocate_exact_size(glyphs.size + vec2(bold, 0.0), Sense::click());
        let n = glyphs.glyphs.len();
        let total = self.duration + self.stagger * n.saturating_sub(1) as f32;
        let e = Clock::triggered(ui, resp.id, self.trigger, total, resp.hovered(), resp.clicked());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let color = self.color.unwrap_or(t.foreground);
        let base_painter = ui.painter();
        // a new random reel every time the animation restarts
        let run = glowkit_core::state(ui, resp.id.with("clock").with("glowkit_clock"), || Clock { start: 0.0 }).start;
        for g in &glyphs.glyphs {
            if g.ch.is_whitespace() {
                continue;
            }
            let mut r = Rng::new(resp.id.value() ^ (g.index as u64).wrapping_mul(0x9E37) ^ run.to_bits());
            let len = self.reel + r.below(3);
            let p = ease(Easing::OutCubic, stagger(e, g.index, self.stagger, self.duration));
            let pos = p * len as f32; // 0 → first random letter, len → real letter
            let cell = Rect::from_min_size(rect.min + g.offset, g.size);
            let clip = cell.expand2(vec2(bold + 1.0, 0.0)).intersect(ui.clip_rect());
            let painter = base_painter.with_clip_rect(clip);
            let reel: Vec<char> = (0..len).map(|_| similar(g.ch, &mut r)).chain(std::iter::once(g.ch)).collect();
            let base = pos.floor() as usize;
            for j in base..=(base + 1).min(len) {
                let d = j as f32 - pos; // 0 = centered
                let off = match self.direction {
                    ShuffleDirection::Up => vec2(0.0, d * cell.height()),
                    ShuffleDirection::Down => vec2(0.0, -d * cell.height()),
                    ShuffleDirection::Left => vec2(d * cell.width(), 0.0),
                    ShuffleDirection::Right => vec2(-d * cell.width(), 0.0),
                };
                let a = (1.0 - d.abs()).clamp(0.0, 1.0);
                let c = if j == len { color } else { color.gamma_multiply(0.75) };
                let tl = pos2(cell.min.x, cell.min.y) + off;
                util::paint_char_in(&painter, reel[j], &font, tl, cell.width(), c.gamma_multiply(0.3 + 0.7 * a), bold);
            }
        }
        resp
    }
}
