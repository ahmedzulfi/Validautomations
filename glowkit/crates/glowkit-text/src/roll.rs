use crate::util;
use egui::{pos2, vec2, Color32, Rect, Response, Sense, Ui, Widget};
use glowkit_core::{anim::stagger, ease, Clock, Easing, Theme, Trigger};

/// Each letter rolls up out of view while an identical letter rolls in from below, staggered
/// across the word. With the default [`Trigger::OnHover`] the roll holds while hovered and rolls
/// back when the pointer leaves, great for menus and links.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::TextRoll::new("Hover me").size(48.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TextRoll {
    text: String,
    size: f32,
    color: Option<Color32>,
    roll_color: Option<Color32>,
    bold: bool,
    stagger: f32,
    duration: f32,
    trigger: Trigger,
    center_out: bool,
}

impl TextRoll {
    /// Create the effect for `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 40.0,
            color: None,
            roll_color: None,
            bold: true,
            stagger: 0.03,
            duration: 0.45,
            trigger: Trigger::OnHover,
            center_out: false,
        }
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Resting text color (defaults to the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Color of the letters that roll in (defaults to the theme accent).
    pub fn roll_color(mut self, c: Color32) -> Self {
        self.roll_color = Some(c);
        self
    }
    /// Fake-bold the text (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Seconds between letters.
    pub fn stagger(mut self, seconds: f32) -> Self {
        self.stagger = seconds.max(0.0);
        self
    }
    /// Seconds one letter takes to roll.
    pub fn duration(mut self, seconds: f32) -> Self {
        self.duration = seconds.max(0.01);
        self
    }
    /// `OnHover` (default) holds while hovered; `Loop` rolls there and back forever;
    /// `OnAppear` / `OnClick` roll once.
    pub fn trigger(mut self, t: Trigger) -> Self {
        self.trigger = t;
        self
    }
    /// Start the stagger from the middle letters instead of the first.
    pub fn center_out(mut self, on: bool) -> Self {
        self.center_out = on;
        self
    }
}

impl Widget for TextRoll {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let glyphs = crate::util::glyphs(ui.painter(), &self.text, font.clone(), None);
        let (rect, mut resp) = ui.allocate_exact_size(glyphs.size + vec2(bold, 0.0), Sense::click());
        if self.trigger == Trigger::OnHover {
            resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        let n = glyphs.glyphs.len();
        let span = self.duration + self.stagger * n.saturating_sub(1) as f32;
        let hold = 0.9;
        let tl = match self.trigger {
            Trigger::OnHover => util::ramp(ui, resp.id.with("roll"), resp.hovered(), span) * span,
            Trigger::Loop { pause_ms } => {
                let e = Clock::triggered(ui, resp.id, Trigger::Loop { pause_ms }, span * 2.0 + hold, false, false);
                if e <= span + hold {
                    e.min(span)
                } else {
                    (span - (e - span - hold)).max(0.0)
                }
            }
            tr => Clock::triggered(ui, resp.id, tr, span, resp.hovered(), resp.clicked()),
        };
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let color = self.color.unwrap_or(t.foreground);
        let roll = self.roll_color.unwrap_or(t.accent);
        let line_h = glyphs.size.y / glyphs.galley.rows.len().max(1) as f32;
        let clip = Rect::from_min_max(pos2(rect.min.x - bold, rect.min.y), pos2(rect.max.x + bold, rect.max.y)).intersect(ui.clip_rect());
        let painter = ui.painter().with_clip_rect(clip);
        let mid = (n as f32 - 1.0) / 2.0;
        for g in &glyphs.glyphs {
            if g.ch.is_whitespace() {
                continue;
            }
            let rank = if self.center_out { (g.index as f32 - mid).abs().round() as usize } else { g.index };
            let p = ease(Easing::InOutCubic, stagger(tl, rank, self.stagger, self.duration));
            let gal = util::layout_char(&painter, g.ch, &font);
            let base = rect.min + g.offset;
            if p < 1.0 {
                util::paint_galley(&painter, base - vec2(0.0, p * line_h), &gal, color.gamma_multiply(1.0 - p * 0.6), bold);
            }
            if p > 0.0 {
                util::paint_galley(&painter, base + vec2(0.0, (1.0 - p) * line_h), &gal, roll.gamma_multiply(0.4 + 0.6 * p), bold);
            }
        }
        resp
    }
}
