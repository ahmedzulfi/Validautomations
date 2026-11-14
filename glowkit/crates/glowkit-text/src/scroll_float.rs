use crate::util;
use egui::{pos2, vec2, Color32, Response, Sense, Ui, Widget};
use glowkit_core::{ease, Clock, Easing, Theme, Trigger};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Source {
    Time,
    Fixed(f32),
    Scroll,
}

/// Letters float up into place one after another, starting tall, squeezed and transparent below
/// the line and settling to their natural shape as the progress grows. Drive it by scroll
/// position, by your own progress value, or let it play on a timer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let progress = 0.5;
/// ui.add(glowkit_text::ScrollFloat::new("Floating letters").progress(progress));
/// ui.add(glowkit_text::ScrollFloat::new("Tied to the scroll area").scroll_linked());
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ScrollFloat {
    text: String,
    size: f32,
    color: Option<Color32>,
    bold: bool,
    stagger: f32,
    stretch: f32,
    rise: f32,
    duration: f32,
    trigger: Trigger,
    source: Source,
}

impl ScrollFloat {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 56.0,
            color: None,
            bold: true,
            stagger: 0.035,
            stretch: 2.3,
            rise: 1.2,
            duration: 1.4,
            trigger: Trigger::Loop { pause_ms: 2400 },
            source: Source::Time,
        }
    }
    /// Font size in points (default 56).
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
    /// Share of the total progress between consecutive letters starting (default 0.035).
    pub fn stagger(mut self, s: f32) -> Self {
        self.stagger = s.clamp(0.0, 0.5);
        self
    }
    /// Vertical stretch of a letter before it settles (default 2.3).
    pub fn stretch(mut self, s: f32) -> Self {
        self.stretch = s.max(1.0);
        self
    }
    /// How far below the line letters start, in line heights (default 1.2).
    pub fn rise(mut self, lines: f32) -> Self {
        self.rise = lines.max(0.0);
        self
    }
    /// Seconds for a full time-driven animation (default 1.4).
    pub fn duration(mut self, seconds: f32) -> Self {
        self.duration = seconds.max(0.05);
        self
    }
    /// When the time-driven animation plays (default: loop with a pause).
    pub fn trigger(mut self, t: Trigger) -> Self {
        self.trigger = t;
        self
    }
    /// Drive the animation yourself, `0..=1`.
    pub fn progress(mut self, p: f32) -> Self {
        self.source = Source::Fixed(p.clamp(0.0, 1.0));
        self
    }
    /// Drive the animation from the widget's position in the enclosing scroll area: letters
    /// start floating as it enters at the bottom and settle by the time it reaches the middle.
    pub fn scroll_linked(mut self) -> Self {
        self.source = Source::Scroll;
        self
    }
}

impl Widget for ScrollFloat {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let glyphs = util::glyphs(ui.painter(), &self.text, font.clone(), None);
        let pad = self.size * 0.2;
        let (rect, resp) = ui.allocate_exact_size(glyphs.size + vec2(bold + pad * 2.0, pad * 2.0), Sense::click());
        let progress = match self.source {
            Source::Fixed(p) => p,
            Source::Time => {
                let e = Clock::triggered(ui, resp.id, self.trigger, self.duration, resp.hovered(), resp.clicked());
                (e / self.duration).clamp(0.0, 1.0)
            }
            Source::Scroll => {
                let clip = ui.clip_rect();
                let start = clip.max.y;
                let end = clip.min.y + clip.height() * 0.5;
                ((start - rect.max.y) / (start - end).max(1.0)).clamp(0.0, 1.0)
            }
        };
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let color = self.color.unwrap_or(t.foreground);
        let n = glyphs.glyphs.len().max(1);
        let span = 1.0 - self.stagger * (n - 1) as f32;
        let span = span.max(0.15);
        // when the stagger is too large for this many letters, compress it
        let stagger = if n > 1 { (1.0 - span) / (n - 1) as f32 } else { 0.0 };
        let origin = rect.min + vec2(pad, pad);
        let line_h = glyphs.size.y / (glyphs.glyphs.last().map(|g| g.line + 1).unwrap_or(1)) as f32;
        // letters rise from behind the line's edge, like text revealed from a mask
        let clip = ui.clip_rect().intersect(rect.expand2(vec2(self.size, 0.0)));
        let painter = ui.painter().with_clip_rect(clip);
        for (i, g) in glyphs.glyphs.iter().enumerate() {
            if g.ch.is_whitespace() {
                continue;
            }
            let local = ((progress - i as f32 * stagger) / span).clamp(0.0, 1.0);
            if local <= 0.0 {
                continue;
            }
            let k = ease(Easing::InOutBack, local);
            let alpha = ease(Easing::OutCubic, (local * 1.6).min(1.0));
            let sy = self.stretch + (1.0 - self.stretch) * k;
            let sx = 0.7 + 0.3 * k;
            let dy = (1.0 - k) * self.rise * line_h;
            let galley = util::layout_char(&painter, g.ch, &font);
            let half = galley.size() / 2.0;
            // stretch downward from the glyph's top while it rises out of the masked line
            let anchor = origin + g.offset + vec2(g.size.x / 2.0, 0.0);
            let c = color.gamma_multiply(alpha);
            let steps = (bold / 0.7).ceil() as usize;
            for s in 0..=steps {
                let dx = if steps == 0 { 0.0 } else { bold * s as f32 / steps as f32 };
                util::paint_galley_mapped(
                    &painter,
                    &galley,
                    |v| {
                        let rel = v - vec2(half.x, 0.0);
                        pos2(anchor.x + rel.x * sx + dx, anchor.y + dy + rel.y * sy)
                    },
                    |_| if s == 0 { c } else { c.gamma_multiply(0.75) },
                );
            }
        }
        resp
    }
}
