//! Record button that expands into a live waveform and transcript.

use crate::common;
use crate::icons::{paint_icon, Icon};
use crate::media::fmt_clock;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// Phase of a [`SpeechInput`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpeechPhase {
    /// Collapsed record button.
    #[default]
    Idle,
    /// Listening: live waveform, timer and interim transcript.
    Recording,
    /// Recording stopped, waiting for the final transcript.
    Processing,
    /// Final transcript shown.
    Done,
}

#[derive(Clone, Default)]
struct SpeechState {
    levels: Vec<f32>,
    last_push: f64,
    started: f64,
    stopped: f64,
    last_phase: SpeechPhase,
}

/// A voice-dictation control. Idle it is a compact "Dictate" pill with a mic; clicking it
/// springs open into a card with a pulsing stop button, a scrolling live waveform, a timer and the
/// interim transcript streaming in. Stopping shows a "Transcribing…" shimmer, then the final text
/// with a button to record again. The caller owns the [`SpeechPhase`] (clicks advance it) and feeds
/// the mic level and transcript text each frame.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::{SpeechInput, SpeechPhase};
/// let mut phase = SpeechPhase::Idle;
/// ui.add(SpeechInput::new(&mut phase).level(0.4).transcript("Schedule a meeting with"));
/// // when your recognizer finishes: phase = SpeechPhase::Done
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SpeechInput<'a> {
    phase: &'a mut SpeechPhase,
    level: f32,
    transcript: &'a str,
    label: &'a str,
    width: Option<f32>,
    color: Option<Color32>,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> SpeechInput<'a> {
    /// A control bound to `phase`.
    pub fn new(phase: &'a mut SpeechPhase) -> Self {
        Self { phase, level: 0.0, transcript: "", label: "Dictate", width: None, color: None, id_salt: None }
    }
    /// Current mic level `0..=1`.
    pub fn level(mut self, l: f32) -> Self {
        self.level = l.clamp(0.0, 1.0);
        self
    }
    /// Interim (while recording) or final (when done) transcript.
    pub fn transcript(mut self, t: &'a str) -> Self {
        self.transcript = t;
        self
    }
    /// Label of the idle button (default "Dictate").
    pub fn label(mut self, l: &'a str) -> Self {
        self.label = l;
        self
    }
    /// Expanded width (default: available width).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Recording color (default: theme destructive red).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Stable id.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }
}

const BARS: usize = 64;

impl Widget for SpeechInput<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = common::widget_id(ui, self.id_salt).with("speech_input");
        let rec_c = self.color.unwrap_or(t.destructive);
        let now = ui.input(|i| i.time);
        let phase = *self.phase;
        let level = self.level;
        let st = glowkit_core::with_state(ui, id, SpeechState::default, |s| {
            if s.last_phase != phase {
                if phase == SpeechPhase::Recording {
                    s.started = now;
                    s.levels.clear();
                }
                if s.last_phase == SpeechPhase::Recording {
                    s.stopped = now;
                }
                s.last_phase = phase;
            }
            if phase == SpeechPhase::Recording && now - s.last_push > 1.0 / 24.0 {
                s.last_push = now;
                s.levels.push(level);
                if s.levels.len() > BARS {
                    let extra = s.levels.len() - BARS;
                    s.levels.drain(..extra);
                }
            }
            s.clone()
        });
        let full_w = self.width.unwrap_or_else(|| ui.available_width());
        let idle_w = common::galley(ui, self.label, 13.5, t.foreground).size().x + 62.0;
        let pad = 14.0;
        let text_w = full_w - pad * 2.0;
        let final_g = (phase == SpeechPhase::Done && !self.transcript.is_empty()).then(|| {
            ui.painter().layout(self.transcript.to_owned(), FontId::proportional(14.0), t.foreground, text_w - 40.0)
        });
        let (tw, th) = match phase {
            SpeechPhase::Idle => (idle_w, 44.0),
            SpeechPhase::Recording | SpeechPhase::Processing => (full_w, 112.0),
            SpeechPhase::Done => (full_w, final_g.as_ref().map(|g| g.size().y).unwrap_or(18.0) + 64.0),
        };
        let w = glowkit_core::anim::spring(ui, id.with("w"), tw, 260.0, 24.0).clamp(44.0, full_w);
        let h = glowkit_core::anim::spring(ui, id.with("h"), th, 260.0, 26.0).max(44.0);
        let (outer, resp) = ui.allocate_exact_size(vec2(full_w, h), Sense::hover());
        let rect = Rect::from_min_size(outer.min, vec2(w, h));
        let expanded = ((w - idle_w) / (full_w - idle_w).max(1.0)).clamp(0.0, 1.0);
        let cr = CornerRadius::same((22.0 - 8.0 * expanded) as u8);
        let p = ui.painter();
        let recording = phase == SpeechPhase::Recording;
        let border = if recording { with_alpha(rec_c, 0.5) } else { common::border_strong(&t) };
        glowkit_core::paint::soft_shadow(
            p,
            rect,
            cr.nw as f32,
            10.0,
            vec2(0.0, 3.0),
            Color32::from_black_alpha(if t.dark { 60 } else { 14 }),
        );
        p.rect(rect, cr, t.card, Stroke::new(1.0, border), StrokeKind::Inside);
        let clip = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        let mut new_phase = None;

        // main round button (mic / stop)
        let bc = pos2(rect.min.x + 22.0, rect.min.y + 22.0);
        let br = Rect::from_center_size(bc, vec2(34.0, 34.0));
        let bresp = ui.interact(if phase == SpeechPhase::Idle { rect } else { br }, id.with("btn"), Sense::click());
        let bresp = bresp.on_hover_cursor(egui::CursorIcon::PointingHand);
        let bh = common::anim_bool(ui, id.with("bh"), bresp.hovered(), 0.1);
        match phase {
            SpeechPhase::Recording => {
                let time = glowkit_core::time(ui);
                for k in 0..2 {
                    let ph = ((time * 0.9 + k as f32 * 0.5) % 1.0) * (0.6 + level * 0.6);
                    clip.circle_stroke(bc, 17.0 + ph * 12.0, Stroke::new(1.5, with_alpha(rec_c, 0.4 * (1.0 - ph))));
                }
                clip.circle_filled(bc, 17.0, rec_c.lerp_to_gamma(Color32::WHITE, 0.1 * bh));
                clip.rect_filled(Rect::from_center_size(bc, vec2(11.0, 11.0)), CornerRadius::same(2), Color32::WHITE);
                glowkit_core::animate(ui);
            }
            SpeechPhase::Processing => {
                clip.circle_filled(bc, 17.0, common::raised(&t));
                common::spinner(&clip, bc, 8.0, 2.0, t.accent, glowkit_core::time(ui));
                glowkit_core::animate(ui);
            }
            SpeechPhase::Done => {
                clip.circle_filled(bc, 17.0, with_alpha(t.success, if t.dark { 0.16 } else { 0.12 }));
                paint_icon(&clip, Icon::Check, Rect::from_center_size(bc, vec2(15.0, 15.0)), t.success);
            }
            SpeechPhase::Idle => {
                clip.circle_filled(bc, 15.0, t.primary.lerp_to_gamma(t.primary_foreground, 0.1 * bh));
                paint_icon(&clip, Icon::Mic, Rect::from_center_size(bc, vec2(15.0, 15.0)), t.primary_foreground);
                clip.text(
                    pos2(bc.x + 26.0, bc.y),
                    Align2::LEFT_CENTER,
                    self.label,
                    FontId::proportional(13.5),
                    t.foreground,
                );
            }
        }
        if bresp.clicked() {
            new_phase = Some(match phase {
                SpeechPhase::Idle | SpeechPhase::Done => SpeechPhase::Recording,
                SpeechPhase::Recording => SpeechPhase::Processing,
                SpeechPhase::Processing => SpeechPhase::Processing,
            });
        }

        if phase != SpeechPhase::Idle && expanded > 0.05 {
            let fade = ((expanded - 0.3) / 0.7).clamp(0.0, 1.0);
            let header_x = bc.x + 30.0;
            match phase {
                SpeechPhase::Recording | SpeechPhase::Processing => {
                    // live waveform between the button and the timer
                    let timer_x = rect.max.x - pad;
                    let elapsed = if recording { now - st.started } else { st.stopped - st.started };
                    let tr = common::mono(
                        &clip,
                        pos2(timer_x, bc.y),
                        Align2::RIGHT_CENTER,
                        fmt_clock(elapsed as f32),
                        12.0,
                        if recording { rec_c.gamma_multiply(fade) } else { t.muted_foreground.gamma_multiply(fade) },
                    );
                    if recording {
                        clip.circle_filled(
                            pos2(tr.min.x - 9.0, bc.y),
                            3.5,
                            rec_c.gamma_multiply(fade * (0.5 + 0.5 * (glowkit_core::time(ui) * 4.0).sin().abs())),
                        );
                    }
                    let wr = Rect::from_min_max(pos2(header_x, bc.y - 14.0), pos2(tr.min.x - 22.0, bc.y + 14.0));
                    let n = BARS;
                    let gap = 2.0;
                    let bw = ((wr.width() + gap) / n as f32 - gap).max(1.0);
                    let col = if recording { t.foreground } else { t.muted_foreground };
                    for i in 0..n {
                        let li = (i + st.levels.len()).checked_sub(n);
                        let v = li.and_then(|k| st.levels.get(k)).copied().unwrap_or(0.0);
                        let x = wr.min.x + i as f32 * (bw + gap);
                        let bh = (v.powf(0.7) * wr.height()).max(2.5);
                        let age = i as f32 / n as f32;
                        let c = with_alpha(col, (0.25 + 0.75 * age) * fade * if recording { 1.0 } else { 0.6 });
                        clip.rect_filled(
                            Rect::from_center_size(pos2(x + bw / 2.0, wr.center().y), vec2(bw, bh)),
                            CornerRadius::same(1),
                            c,
                        );
                    }
                    // transcript area
                    let ta = Rect::from_min_max(
                        pos2(rect.min.x + pad, rect.min.y + 52.0),
                        pos2(rect.max.x - pad, rect.max.y - 10.0),
                    );
                    clip.line_segment(
                        [pos2(rect.min.x + 1.0, ta.min.y - 6.0), pos2(rect.max.x - 1.0, ta.min.y - 6.0)],
                        Stroke::new(1.0, with_alpha(t.border, fade)),
                    );
                    if recording {
                        let shown = if self.transcript.is_empty() { "Listening…" } else { self.transcript };
                        let g =
                            ui.painter().layout(shown.to_owned(), FontId::proportional(14.0), t.foreground, ta.width());
                        // keep the newest line visible
                        let y = ta.min.y + 6.0 - (g.size().y - (ta.height() - 6.0)).max(0.0);
                        let c = if self.transcript.is_empty() { t.muted_foreground } else { t.foreground };
                        let tclip = clip.with_clip_rect(ta);
                        tclip.galley_with_override_text_color(pos2(ta.min.x, y), g.clone(), c.gamma_multiply(fade));
                        // caret
                        let end = common::galley_end(&g);
                        let blink = ((glowkit_core::time(ui) * 2.0) % 1.0 < 0.55) as u8 as f32;
                        let cp = pos2(ta.min.x + end.min.x + 2.0, y + end.min.y);
                        tclip.rect_filled(
                            Rect::from_min_size(cp, vec2(2.0, end.height().max(16.0))),
                            CornerRadius::ZERO,
                            with_alpha(t.accent, blink * fade),
                        );
                    } else {
                        let mut child = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(Rect::from_min_size(pos2(ta.min.x, ta.min.y + 6.0), vec2(ta.width(), 22.0))),
                        );
                        child.set_clip_rect(ta);
                        child.add(crate::ShimmerText::new("Transcribing your recording…").size(14.0));
                    }
                }
                SpeechPhase::Done => {
                    common::text(
                        &clip,
                        pos2(header_x, bc.y),
                        Align2::LEFT_CENTER,
                        "Transcript ready",
                        13.0,
                        t.muted_foreground.gamma_multiply(fade),
                    );
                    // record again
                    let rr = Rect::from_min_size(pos2(rect.max.x - pad - 92.0, bc.y - 14.0), vec2(92.0, 28.0));
                    let rresp = ui
                        .interact(rr, id.with("again"), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    let rh = common::anim_bool(ui, id.with("rh"), rresp.hovered(), 0.1);
                    clip.rect(
                        rr,
                        CornerRadius::same(14),
                        common::hover_fill(&t).gamma_multiply(0.6 + rh),
                        Stroke::new(1.0, with_alpha(t.border, fade)),
                        StrokeKind::Inside,
                    );
                    paint_icon(
                        &clip,
                        Icon::Mic,
                        Rect::from_center_size(pos2(rr.min.x + 16.0, rr.center().y), vec2(13.0, 13.0)),
                        t.foreground.gamma_multiply(fade),
                    );
                    clip.text(
                        pos2(rr.min.x + 28.0, rr.center().y),
                        Align2::LEFT_CENTER,
                        "Re-record",
                        FontId::proportional(12.0),
                        t.foreground.gamma_multiply(fade),
                    );
                    if rresp.clicked() {
                        new_phase = Some(SpeechPhase::Recording);
                    }
                    if let Some(g) = final_g {
                        let y = rect.min.y + 50.0;
                        clip.line_segment(
                            [pos2(rect.min.x + 1.0, y - 4.0), pos2(rect.max.x - 1.0, y - 4.0)],
                            Stroke::new(1.0, with_alpha(t.border, fade)),
                        );
                        clip.galley_with_override_text_color(
                            pos2(rect.min.x + pad, y + 4.0),
                            g,
                            t.foreground.gamma_multiply(fade),
                        );
                    }
                }
                SpeechPhase::Idle => {}
            }
        }
        let mut resp = resp;
        if let Some(np) = new_phase {
            if np != phase {
                *self.phase = np;
                resp.mark_changed();
            }
        }
        resp
    }
}
