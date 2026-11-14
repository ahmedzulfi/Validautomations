//! Audio player card with a scrubbable waveform.

use crate::common;
use crate::media::{self, fmt_clock};
use crate::scrub_bar::ScrubBar;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// Caller-owned playback state for an [`AudioPlayer`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioPlayerState {
    /// Current position in seconds.
    pub position: f32,
    /// Clip length in seconds.
    pub duration: f32,
    /// Whether playback is running.
    pub playing: bool,
    /// Playback rate (1.0 = normal).
    pub speed: f32,
}

impl AudioPlayerState {
    /// A paused clip of `duration` seconds at the start.
    pub fn new(duration: f32) -> Self {
        Self { position: 0.0, duration: duration.max(0.001), playing: false, speed: 1.0 }
    }
    /// Progress in `0..=1`.
    pub fn progress(&self) -> f32 {
        (self.position / self.duration).clamp(0.0, 1.0)
    }
}

/// The playback speeds the speed pill cycles through.
const SPEEDS: [f32; 5] = [0.75, 1.0, 1.25, 1.5, 2.0];

/// An audio player card: a round play / pause button, title and subtitle, a waveform you can
/// click or drag to scrub (played bars in the accent color, a hover preview), elapsed / total time
/// and a playback-speed pill. There is no audio backend: with [`AudioPlayer::simulate`] (the
/// default) the player advances `position` itself, so it can drive a demo, or you can turn that
/// off and feed it the position of a real player.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::{AudioPlayer, AudioPlayerState};
/// let mut state = AudioPlayerState::new(184.0);
/// ui.add(AudioPlayer::new(&mut state).title("Welcome message").subtitle("Aria · Narration"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AudioPlayer<'a> {
    state: &'a mut AudioPlayerState,
    title: Option<&'a str>,
    subtitle: Option<&'a str>,
    seed: u32,
    bars: usize,
    waveform: bool,
    simulate: bool,
    color: Option<Color32>,
    width: Option<f32>,
    looping: bool,
}

impl<'a> AudioPlayer<'a> {
    /// A player bound to `state`.
    pub fn new(state: &'a mut AudioPlayerState) -> Self {
        Self {
            state,
            title: None,
            subtitle: None,
            seed: 1,
            bars: 72,
            waveform: true,
            simulate: true,
            color: None,
            width: None,
            looping: false,
        }
    }
    /// Track title.
    pub fn title(mut self, t: &'a str) -> Self {
        self.title = Some(t);
        self
    }
    /// Muted line under the title (voice, artist, date).
    pub fn subtitle(mut self, s: &'a str) -> Self {
        self.subtitle = Some(s);
        self
    }
    /// Seed of the generated waveform shape.
    pub fn seed(mut self, s: u32) -> Self {
        self.seed = s;
        self
    }
    /// Number of waveform bars (default 72).
    pub fn bars(mut self, n: usize) -> Self {
        self.bars = n.max(8);
        self
    }
    /// Waveform scrubber (default) or a slim [`ScrubBar`].
    pub fn waveform(mut self, w: bool) -> Self {
        self.waveform = w;
        self
    }
    /// Advance the position while playing (default on). Turn off when a real player drives it.
    pub fn simulate(mut self, s: bool) -> Self {
        self.simulate = s;
        self
    }
    /// Restart from the beginning when the end is reached (simulation only).
    pub fn looping(mut self, l: bool) -> Self {
        self.looping = l;
        self
    }
    /// Accent color (default: theme accent).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Card width (default: available width).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
}

impl Widget for AudioPlayer<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let col = self.color.unwrap_or(t.accent);
        let st = self.state;
        if self.simulate && st.playing {
            st.position += glowkit_core::dt(ui) * st.speed;
            if st.position >= st.duration {
                if self.looping {
                    st.position = 0.0;
                } else {
                    st.position = st.duration;
                    st.playing = false;
                }
            }
            glowkit_core::animate(ui);
        }
        let w = self.width.unwrap_or_else(|| ui.available_width());
        let pad = 16.0;
        let wave_h = if self.waveform { 44.0 } else { 22.0 };
        let h = pad * 2.0 + 44.0 + 12.0 + wave_h + 20.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let cr = t.corner_scaled(1.2);
        {
            let p = ui.painter();
            if !t.dark {
                glowkit_core::paint::soft_shadow(p, rect, cr.nw as f32, 10.0, vec2(0.0, 4.0), Color32::from_black_alpha(14));
            }
            p.rect(rect, cr, t.card, Stroke::new(1.0, t.border), StrokeKind::Inside);
        }
        let inner = rect.shrink(pad);
        // header: play button, titles, speed pill
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::left_to_right(egui::Align::Min)));
        let pb = media::play_button(&mut child, st.playing, 44.0, col);
        if pb.clicked() {
            if !st.playing && st.position >= st.duration - 0.01 {
                st.position = 0.0;
            }
            st.playing = !st.playing;
        }
        let tx = inner.min.x + 56.0;
        let p = ui.painter();
        if let Some(ti) = self.title {
            p.text(pos2(tx, inner.min.y + 13.0), Align2::LEFT_CENTER, ti, FontId::proportional(14.5), t.foreground);
        }
        if let Some(s) = self.subtitle {
            p.text(pos2(tx, inner.min.y + 32.0), Align2::LEFT_CENTER, s, FontId::proportional(12.5), t.muted_foreground);
        }
        // speed pill
        let label = format!("{}x", trim(st.speed));
        let g = common::galley(ui, label.clone(), 12.0, t.foreground);
        let sr = Rect::from_min_size(pos2(inner.max.x - g.size().x - 20.0, inner.min.y + 10.0), vec2(g.size().x + 20.0, 24.0));
        let sresp = ui.interact(sr, resp.id.with("speed"), Sense::click()).on_hover_text("Playback speed");
        let sh = common::anim_bool(ui, sresp.id.with("h"), sresp.hovered(), 0.12);
        if sresp.clicked() {
            let i = SPEEDS.iter().position(|s| (s - st.speed).abs() < 1e-3).unwrap_or(1);
            st.speed = SPEEDS[(i + 1) % SPEEDS.len()];
        }
        let p = ui.painter();
        p.rect(
            sr,
            CornerRadius::same(12),
            common::raised(&t).lerp_to_gamma(common::hover_fill(&t), sh),
            Stroke::new(1.0, t.border),
            StrokeKind::Inside,
        );
        common::mono(p, sr.center(), Align2::CENTER_CENTER, label, 11.5, t.foreground);
        // waveform / scrub
        let wr = Rect::from_min_size(pos2(inner.min.x, inner.min.y + 56.0), vec2(inner.width(), wave_h));
        if self.waveform {
            let wresp = ui.interact(wr, resp.id.with("wave"), Sense::click_and_drag());
            let wresp = wresp.on_hover_cursor(egui::CursorIcon::PointingHand);
            if wresp.is_pointer_button_down_on() || wresp.dragged() {
                if let Some(pp) = wresp.interact_pointer_pos() {
                    st.position = ((pp.x - wr.min.x) / wr.width()).clamp(0.0, 1.0) * st.duration;
                }
            }
            let peaks = media::waveform_peaks(self.seed, self.bars);
            let n = peaks.len();
            let gap = 2.0;
            let bw = ((wr.width() + gap) / n as f32 - gap).max(1.0);
            let prog = st.progress();
            let hover = wresp.hover_pos().map(|hp| ((hp.x - wr.min.x) / wr.width()).clamp(0.0, 1.0));
            let p = ui.painter();
            let base = with_alpha(t.muted_foreground, if t.dark { 0.35 } else { 0.32 });
            for (i, v) in peaks.iter().enumerate() {
                let x0 = wr.min.x + i as f32 * (bw + gap);
                let f0 = i as f32 / n as f32;
                let f1 = (i + 1) as f32 / n as f32;
                let bh = (v * wr.height()).max(3.0);
                let br = Rect::from_center_size(pos2(x0 + bw / 2.0, wr.center().y), vec2(bw, bh));
                let c = if f1 <= prog {
                    col
                } else if f0 < prog {
                    // partially played bar: blend
                    base.lerp_to_gamma(col, (prog - f0) / (f1 - f0))
                } else if hover.is_some_and(|hx| f0 < hx) {
                    with_alpha(col, 0.45)
                } else {
                    base
                };
                p.rect_filled(br, CornerRadius::same((bw / 2.0).min(3.0) as u8), c);
            }
            // playhead
            let px = wr.min.x + prog * wr.width();
            p.line_segment([pos2(px, wr.min.y - 2.0), pos2(px, wr.max.y + 2.0)], Stroke::new(1.5, with_alpha(t.foreground, 0.7)));
        } else {
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(wr));
            let mut pos = st.position;
            child.add(ScrubBar::new(&mut pos, st.duration).buffered((st.position + 40.0).min(st.duration)).color(col));
            st.position = pos;
        }
        // times
        let ty = wr.max.y + 13.0;
        let p = ui.painter();
        common::mono(p, pos2(inner.min.x, ty), Align2::LEFT_CENTER, fmt_clock(st.position), 11.5, t.foreground);
        common::mono(p, pos2(inner.max.x, ty), Align2::RIGHT_CENTER, fmt_clock(st.duration), 11.5, t.muted_foreground);
        resp
    }
}

fn trim(v: f32) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}
