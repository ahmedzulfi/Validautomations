//! Synced transcript with word-level highlighting.

use crate::common;
use crate::media::fmt_clock;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Rect, Sense, Shape, Ui};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// One timed word of a transcript.
#[derive(Clone, Debug, PartialEq)]
pub struct TranscriptWord {
    /// The word (punctuation included).
    pub text: String,
    /// Start time in seconds.
    pub start: f32,
    /// End time in seconds.
    pub end: f32,
    /// Speaker of this word; a change of speaker starts a new paragraph.
    pub speaker: Option<String>,
}

impl TranscriptWord {
    /// A word spoken from `start` to `end` seconds.
    pub fn new(text: impl Into<String>, start: f32, end: f32) -> Self {
        Self { text: text.into(), start, end, speaker: None }
    }
    /// Set the speaker.
    pub fn speaker(mut self, s: impl Into<String>) -> Self {
        self.speaker = Some(s.into());
        self
    }
    /// Build evenly-timed words from `(speaker, text)` segments, starting at `start` seconds and
    /// speaking `words_per_second`, with a short pause between segments. Handy for demos and tests.
    pub fn timeline(segments: &[(&str, &str)], start: f32, words_per_second: f32) -> Vec<Self> {
        let step = 1.0 / words_per_second.max(0.1);
        let mut t = start;
        let mut out = Vec::new();
        for (speaker, text) in segments {
            for w in text.split_whitespace() {
                // longer words take a little longer
                let d = step * (0.7 + w.chars().count() as f32 * 0.06).min(1.6);
                let pause = if w.ends_with(['.', '?', '!']) { step * 0.6 } else if w.ends_with(',') { step * 0.3 } else { 0.0 };
                out.push(Self::new(w, t, t + d).speaker(*speaker));
                t += d + pause;
            }
            t += step * 1.2;
        }
        out
    }
}

/// A synced transcript: words are grouped into speaker paragraphs (with avatar dot, name and
/// timestamp), already-spoken words read in the foreground color, upcoming words are muted and
/// the current word sits on an accent pill that glides from word to word. Click any word to seek;
/// the view scrolls to keep the current word in sight.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::{TranscriptViewer, TranscriptWord};
/// let words = TranscriptWord::timeline(&[("Agent", "Hi! How can I help you today?")], 0.0, 2.5);
/// let mut position = 1.2;
/// if let Some(seek) = TranscriptViewer::new(&words, position).height(220.0).show(ui) {
///     position = seek;
/// }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct TranscriptViewer<'a> {
    words: &'a [TranscriptWord],
    position: f32,
    height: f32,
    size: f32,
    color: Option<Color32>,
    auto_scroll: bool,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> TranscriptViewer<'a> {
    /// A transcript of `words` at playback `position` seconds.
    pub fn new(words: &'a [TranscriptWord], position: f32) -> Self {
        Self { words, position, height: 240.0, size: 15.0, color: None, auto_scroll: true, id_salt: None }
    }
    /// Viewport height (scrolls beyond it; default 240).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Font size (default 15).
    pub fn text_size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    /// Highlight color (default: theme accent).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Keep the current word scrolled into view (default on).
    pub fn auto_scroll(mut self, a: bool) -> Self {
        self.auto_scroll = a;
        self
    }
    /// Stable id (needed when several viewers share a parent).
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Index of the word being spoken at `position` (the last started word during pauses).
    pub fn current_index(words: &[TranscriptWord], position: f32) -> Option<usize> {
        if words.first().is_none_or(|w| position < w.start) {
            return None;
        }
        let i = words.partition_point(|w| w.start <= position);
        Some(i.saturating_sub(1))
    }

    /// Show the transcript. Returns the time to seek to when a word was clicked.
    pub fn show(self, ui: &mut Ui) -> Option<f32> {
        let t = Theme::of(ui);
        let col = self.color.unwrap_or(t.accent);
        let id = common::widget_id(ui, self.id_salt).with("transcript");
        let cur = Self::current_index(self.words, self.position);
        let speaking = cur.is_some_and(|i| self.position <= self.words[i].end + 0.05);
        let mut seek = None;
        let last_cur: Option<usize> = glowkit_core::state(ui, id.with("last"), || None);
        let changed = last_cur != cur;
        glowkit_core::set_state(ui, id.with("last"), cur);
        let font = FontId::proportional(self.size);
        let line_h = self.size * 1.65;
        egui::ScrollArea::vertical().id_salt(id.with("scroll")).max_height(self.height).auto_shrink([false, true]).show(
            ui,
            |ui| {
                let w = ui.available_width() - 8.0;
                let origin = ui.cursor().min;
                let pill_slot = ui.painter().add(Shape::Noop);
                let space_w = ui.painter().layout_no_wrap(" ".into(), font.clone(), t.foreground).size().x;
                let mut x = 0.0f32;
                let mut y = 0.0f32;
                let mut speaker: Option<&str> = None;
                let mut cur_rect = None;
                let mut placed: Vec<(Rect, usize)> = Vec::with_capacity(self.words.len());
                for (i, word) in self.words.iter().enumerate() {
                    if i == 0 || word.speaker.as_deref() != speaker {
                        if i > 0 {
                            y += line_h + 12.0;
                        }
                        speaker = word.speaker.as_deref();
                        if let Some(s) = speaker {
                            let p = ui.painter();
                            let hue = s.bytes().fold(11u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32)) % 360;
                            let sc = glowkit_core::hsv(hue as f32, 0.55, if t.dark { 0.9 } else { 0.6 });
                            let c = origin + vec2(8.0, y + 9.0);
                            p.circle_filled(c, 8.0, with_alpha(sc, 0.2));
                            p.text(c, Align2::CENTER_CENTER, s.chars().next().unwrap_or('?').to_string(), FontId::proportional(9.5), sc);
                            let r = common::text(p, c + vec2(14.0, 0.0), Align2::LEFT_CENTER, s, 12.5, t.foreground);
                            common::mono(p, pos2(r.max.x + 8.0, c.y), Align2::LEFT_CENTER, fmt_clock(word.start), 11.0, t.muted_foreground);
                            y += 26.0;
                        }
                        x = 0.0;
                    }
                    let g = ui.painter().layout_no_wrap(word.text.clone(), font.clone(), t.foreground);
                    let ww = g.size().x;
                    if x > 0.0 && x + ww > w {
                        x = 0.0;
                        y += line_h;
                    }
                    let r = Rect::from_min_size(origin + vec2(x, y), vec2(ww, line_h));
                    placed.push((r, i));
                    let past = cur.is_some_and(|c| i < c);
                    let is_cur = Some(i) == cur;
                    let color = if is_cur && speaking {
                        glowkit_core::color::contrast_text(col)
                    } else if past || is_cur {
                        t.foreground
                    } else {
                        with_alpha(t.muted_foreground, if t.dark { 0.75 } else { 0.85 })
                    };
                    if is_cur {
                        cur_rect = Some(r);
                    }
                    ui.painter().galley_with_override_text_color(pos2(r.min.x, r.center().y - g.size().y / 2.0), g, color);
                    x += ww + space_w + 1.5;
                }
                let total = vec2(w, y + line_h + 4.0);
                let (area, resp) = ui.allocate_exact_size(total, Sense::click());
                // hover + click on words
                if let Some(hp) = resp.hover_pos() {
                    if let Some((r, i)) = placed.iter().find(|(r, _)| r.expand2(vec2(space_w / 2.0, 0.0)).contains(hp)) {
                        ui.painter().rect_filled(r.expand2(vec2(3.0, -2.0)), CornerRadius::same(5), common::hover_fill(&t));
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        if resp.clicked() {
                            seek = Some(self.words[*i].start);
                        }
                    }
                }
                // gliding highlight pill
                if let Some(r) = cur_rect {
                    let target = r.expand2(vec2(3.0, -3.0));
                    let ax = glowkit_core::anim::follow(ui, id.with("px"), target.min.x - area.min.x, 40.0);
                    let ay = glowkit_core::anim::follow(ui, id.with("py"), target.min.y - area.min.y, 40.0);
                    let aw = glowkit_core::anim::follow(ui, id.with("pw"), target.width(), 40.0);
                    let pr = Rect::from_min_size(area.min + vec2(ax, ay), vec2(aw, target.height()));
                    let a = if speaking { 1.0 } else { 0.0 };
                    let fade = glowkit_core::anim::follow(ui, id.with("pa"), a, 12.0);
                    ui.painter().set(
                        pill_slot,
                        Shape::Vec(vec![
                            Shape::rect_filled(pr.expand(2.0), CornerRadius::same(8), with_alpha(col, 0.18 * fade)),
                            Shape::rect_filled(pr, CornerRadius::same(6), with_alpha(col, fade)),
                        ]),
                    );
                    if self.auto_scroll && changed {
                        ui.scroll_to_rect(r.expand(line_h), None);
                    }
                }
            },
        );
        seek
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_and_current_word() {
        let w = TranscriptWord::timeline(&[("A", "Hello there, friend."), ("B", "Hi!")], 1.0, 2.0);
        assert_eq!(w.len(), 4);
        assert_eq!(w[0].speaker.as_deref(), Some("A"));
        assert_eq!(w[3].speaker.as_deref(), Some("B"));
        assert!(w.windows(2).all(|p| p[0].end <= p[1].start + 1e-4));
        assert_eq!(TranscriptViewer::current_index(&w, 0.5), None);
        assert_eq!(TranscriptViewer::current_index(&w, 1.0), Some(0));
        assert_eq!(TranscriptViewer::current_index(&w, w[2].start + 0.01), Some(2));
        assert_eq!(TranscriptViewer::current_index(&w, 999.0), Some(3));
    }
}
