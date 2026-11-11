//! GSAP-style timelines: sequence many tweens with relative positions, labels and overlaps,
//! then query any track's value at any time. [`Playback`] adds play / pause / reverse / seek /
//! speed, and [`Scrubber`] draws an interactive timeline view.

use crate::value::{Animatable, Components, MAX_DIM};
use egui::{pos2, vec2, Color32, CornerRadius, FontId, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::anim::{ease, Easing};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// Where a tween is placed on a [`Timeline`], mirroring GSAP's position parameter.
///
/// Converts from strings: `"<"` (start of previous), `">"` (end of previous), `"<0.2"`,
/// `">-0.1"`, `"+=0.5"` (gap after the timeline end), `"-=0.3"` (overlap), `"intro"` /
/// `"intro+=0.2"` (labels) and `"1.5"` (absolute). Converts from `f32` as an absolute time.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum At {
    /// After everything added so far (the default).
    #[default]
    End,
    /// Absolute time in seconds.
    Time(f32),
    /// Relative to the end of the timeline (`"+=0.2"` / `"-=0.2"`).
    EndOffset(f32),
    /// Relative to the start of the previously added tween (`"<"`, `"<0.2"`).
    PrevStart(f32),
    /// Relative to the end of the previously added tween (`">"`, `">0.2"`).
    PrevEnd(f32),
    /// Relative to a label.
    Label(String, f32),
}

impl From<f32> for At {
    fn from(t: f32) -> Self {
        At::Time(t)
    }
}

impl From<&str> for At {
    fn from(s: &str) -> Self {
        let s = s.trim();
        let num = |x: &str| -> f32 {
            let x = x.trim();
            if x.is_empty() {
                0.0
            } else if let Some(r) = x.strip_prefix("+=") {
                r.trim().parse().unwrap_or(0.0)
            } else if let Some(r) = x.strip_prefix("-=") {
                -r.trim().parse::<f32>().unwrap_or(0.0)
            } else {
                x.parse().unwrap_or(0.0)
            }
        };
        if s.is_empty() {
            return At::End;
        }
        if let Some(r) = s.strip_prefix('<') {
            return At::PrevStart(num(r));
        }
        if let Some(r) = s.strip_prefix('>') {
            return At::PrevEnd(num(r));
        }
        if s.starts_with("+=") || s.starts_with("-=") {
            return At::EndOffset(num(s));
        }
        if let Ok(t) = s.parse::<f32>() {
            return At::Time(t);
        }
        // label with optional "+=x" / "-=x"
        let split = s.find("+=").or_else(|| s.find("-="));
        match split {
            Some(i) => At::Label(s[..i].to_owned(), num(&s[i..])),
            None => At::Label(s.to_owned(), 0.0),
        }
    }
}

impl From<String> for At {
    fn from(s: String) -> Self {
        At::from(s.as_str())
    }
}

/// One tween to add to a [`Timeline`]: animates `track` from `from` to `to`.
///
/// ```no_run
/// use glowkit_motion::Tween;
/// use glowkit_core::Easing;
/// let t = Tween::new("box.x", 0.0_f32, 280.0).duration(0.8).ease(Easing::OutBack).name("slide in");
/// ```
#[derive(Clone, Debug)]
pub struct Tween<T: Animatable> {
    track: String,
    name: Option<String>,
    from: T,
    to: T,
    duration: f32,
    easing: Easing,
}

impl<T: Animatable> Tween<T> {
    /// Animate `track` from `from` to `to` (0.5 s, ease-out cubic by default).
    pub fn new(track: impl Into<String>, from: T, to: T) -> Self {
        Self { track: track.into(), name: None, from, to, duration: 0.5, easing: Easing::OutCubic }
    }
    /// Seconds.
    pub fn duration(mut self, seconds: f32) -> Self {
        self.duration = seconds.max(0.0);
        self
    }
    /// Easing curve.
    pub fn ease(mut self, e: Easing) -> Self {
        self.easing = e;
        self
    }
    /// Display name (shown by [`Scrubber`]); defaults to the track name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// A placed, type-erased tween inside a [`Timeline`].
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineEntry {
    /// Track this tween writes to.
    pub track: String,
    /// Display name.
    pub name: String,
    /// Start time in seconds.
    pub start: f32,
    /// Duration in seconds.
    pub duration: f32,
    /// Easing curve.
    pub easing: Easing,
    from: Components,
    to: Components,
    dim: usize,
}

impl TimelineEntry {
    /// End time in seconds.
    pub fn end(&self) -> f32 {
        self.start + self.duration
    }
    /// Eased progress (0..1) of this tween at `time`.
    pub fn progress(&self, time: f32) -> f32 {
        if self.duration <= 0.0 {
            return if time >= self.start { 1.0 } else { 0.0 };
        }
        ease(self.easing, ((time - self.start) / self.duration).clamp(0.0, 1.0))
    }
}

/// A sequence of tweens on named tracks, positioned like a GSAP timeline.
///
/// Values are queried at any time with [`Timeline::value`], so the same timeline can be played,
/// reversed, scrubbed or driven by scroll position.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_motion::{Timeline, Tween, Playback};
/// use glowkit_core::Easing;
/// let tl = Timeline::new()
///     .add(Tween::new("x", 0.0_f32, 300.0).duration(0.8).ease(Easing::OutBack), ())
///     .label("spin")
///     .add(Tween::new("rot", 0.0_f32, 360.0).duration(0.6), "<0.2")   // 0.2 s after x starts
///     .add(Tween::new("fade", 1.0_f32, 0.3), "-=0.3");                // overlap the end by 0.3 s
/// let pb = Playback::update(ui, ui.scope_id().with("tl"), tl.duration());
/// let x: f32 = tl.value("x", pb.time).unwrap_or(0.0);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Timeline {
    entries: Vec<TimelineEntry>,
    labels: Vec<(String, f32)>,
}

/// Allows `()` as "default position" in [`Timeline::add`].
impl From<()> for At {
    fn from(_: ()) -> Self {
        At::End
    }
}

impl Timeline {
    /// An empty timeline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Total length in seconds.
    pub fn duration(&self) -> f32 {
        self.entries.iter().map(|e| e.end()).fold(0.0, f32::max)
    }

    /// Resolve a position to an absolute start time.
    pub fn resolve(&self, at: &At) -> f32 {
        let end = self.duration();
        let prev = self.entries.last();
        let t = match at {
            At::End => end,
            At::Time(t) => *t,
            At::EndOffset(d) => end + d,
            At::PrevStart(d) => prev.map_or(0.0, |p| p.start) + d,
            At::PrevEnd(d) => prev.map_or(0.0, |p| p.end()) + d,
            At::Label(name, d) => self.label_time(name).unwrap_or(end) + d,
        };
        t.max(0.0)
    }

    /// Add a tween at a position (`()` = after the end, or anything convertible into [`At`]).
    pub fn add<T: Animatable>(mut self, tween: Tween<T>, at: impl Into<At>) -> Self {
        let start = self.resolve(&at.into());
        self.entries.push(TimelineEntry {
            name: tween.name.unwrap_or_else(|| tween.track.clone()),
            track: tween.track,
            start,
            duration: tween.duration,
            easing: tween.easing,
            from: tween.from.to_components(),
            to: tween.to.to_components(),
            dim: T::DIM,
        });
        self
    }

    /// Add a label at the current end of the timeline.
    pub fn label(self, name: impl Into<String>) -> Self {
        self.label_at(name, At::End)
    }

    /// Add a label at a position.
    pub fn label_at(mut self, name: impl Into<String>, at: impl Into<At>) -> Self {
        let t = self.resolve(&at.into());
        self.labels.push((name.into(), t));
        self
    }

    /// Time of a label.
    pub fn label_time(&self, name: &str) -> Option<f32> {
        self.labels.iter().rev().find(|(n, _)| n == name).map(|(_, t)| *t)
    }

    /// All labels with their times.
    pub fn labels(&self) -> &[(String, f32)] {
        &self.labels
    }

    /// All placed tweens in insertion order.
    pub fn entries(&self) -> &[TimelineEntry] {
        &self.entries
    }

    /// Distinct track names in order of first appearance.
    pub fn tracks(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for e in &self.entries {
            if !out.contains(&e.track.as_str()) {
                out.push(&e.track);
            }
        }
        out
    }

    /// Value of `track` at `time`. Before its first tween the track holds that tween's `from`
    /// value; between tweens it holds the last finished tween's `to` value; overlapping tweens on
    /// one track resolve to the one that started most recently. `None` if the track is unknown.
    pub fn value<T: Animatable>(&self, track: &str, time: f32) -> Option<T> {
        let mut first: Option<&TimelineEntry> = None;
        let mut active: Option<&TimelineEntry> = None;
        for e in self.entries.iter().filter(|e| e.track == track) {
            if first.is_none_or(|f| e.start < f.start) {
                first = Some(e);
            }
            if e.start <= time && active.is_none_or(|a| e.start >= a.start) {
                active = Some(e);
            }
        }
        let e = active.or(first)?;
        let k = if active.is_some() { e.progress(time) } else { 0.0 };
        let mut c = [0.0; MAX_DIM];
        for i in 0..e.dim.min(T::DIM) {
            c[i] = e.from[i] + (e.to[i] - e.from[i]) * k;
        }
        Some(T::from_components(&c))
    }

    /// [`Self::value`] with a fallback.
    pub fn value_or<T: Animatable>(&self, track: &str, time: f32, default: T) -> T {
        self.value(track, time).unwrap_or(default)
    }
}

/// Play state of a timeline: play, pause, reverse, seek, speed and looping.
///
/// Store it yourself, or use [`Playback::update`] / [`Playback::modify`] to keep it in egui
/// memory under an id.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Playback {
    /// Playhead in seconds.
    pub time: f32,
    /// Is time advancing?
    pub playing: bool,
    /// Playback rate (1 = real time).
    pub speed: f32,
    /// Play backward.
    pub reversed: bool,
    /// Start over (or bounce, with `yoyo`) when reaching an end.
    pub looping: bool,
    /// When looping, flip direction at each end instead of jumping.
    pub yoyo: bool,
    /// Seconds to hold at each end before looping.
    pub loop_delay: f32,
    hold: f32,
}

impl Default for Playback {
    fn default() -> Self {
        Self { time: 0.0, playing: true, speed: 1.0, reversed: false, looping: false, yoyo: false, loop_delay: 0.0, hold: 0.0 }
    }
}

impl Playback {
    /// Playing forward from 0, once.
    pub fn new() -> Self {
        Self::default()
    }
    /// Loop forever.
    pub fn looping(mut self, yoyo: bool, delay: f32) -> Self {
        self.looping = true;
        self.yoyo = yoyo;
        self.loop_delay = delay;
        self
    }
    /// Resume playing.
    pub fn play(&mut self) {
        self.playing = true;
    }
    /// Pause.
    pub fn pause(&mut self) {
        self.playing = false;
    }
    /// Toggle play / pause.
    pub fn toggle(&mut self) {
        self.playing = !self.playing;
    }
    /// Flip direction and keep playing.
    pub fn reverse(&mut self) {
        self.reversed = !self.reversed;
        self.playing = true;
    }
    /// Jump to `time` seconds.
    pub fn seek(&mut self, time: f32) {
        self.time = time.max(0.0);
        self.hold = 0.0;
    }
    /// Jump to the start and play forward.
    pub fn restart(&mut self) {
        self.time = 0.0;
        self.reversed = false;
        self.playing = true;
        self.hold = 0.0;
    }
    /// Progress `0..=1` through a timeline of `duration` seconds.
    pub fn progress(&self, duration: f32) -> f32 {
        if duration <= 0.0 {
            1.0
        } else {
            (self.time / duration).clamp(0.0, 1.0)
        }
    }

    /// Advance by `dt` seconds for a timeline of `duration` seconds.
    pub fn advance(&mut self, dt: f32, duration: f32) {
        if !self.playing {
            return;
        }
        if self.hold > 0.0 {
            self.hold -= dt;
            if self.hold > 0.0 {
                return;
            }
            if self.yoyo {
                self.reversed = !self.reversed;
            } else {
                self.time = if self.reversed { duration } else { 0.0 };
            }
        }
        let step = dt * self.speed * if self.reversed { -1.0 } else { 1.0 };
        self.time += step;
        let hit_end = (!self.reversed && self.time >= duration) || (self.reversed && self.time <= 0.0);
        self.time = self.time.clamp(0.0, duration.max(0.0));
        if hit_end {
            if self.looping {
                self.hold = self.loop_delay.max(1e-4);
            } else {
                self.playing = false;
            }
        }
    }

    fn key(id: Id) -> Id {
        id.with("glowkit_playback")
    }

    /// Advance the playback stored under `id` by this frame's delta and return it. Requests
    /// repaints while playing. The first call creates a [`Playback::new`].
    pub fn update(ui: &Ui, id: Id, duration: f32) -> Playback {
        Self::update_with(ui, id, duration, Playback::new)
    }

    /// Like [`Self::update`] with a custom initial state (e.g. `Playback::new().looping(..)`).
    pub fn update_with(ui: &Ui, id: Id, duration: f32, init: impl FnOnce() -> Playback) -> Playback {
        let dt = glowkit_core::dt(ui);
        let pb = glowkit_core::with_state(ui, Self::key(id), init, |pb| {
            pb.advance(dt, duration);
            *pb
        });
        if pb.playing {
            ui.ctx().request_repaint();
        }
        pb
    }

    /// Mutate the playback stored under `id` (e.g. from a button).
    pub fn modify(ui: &Ui, id: Id, f: impl FnOnce(&mut Playback)) {
        glowkit_core::with_state(ui, Self::key(id), Playback::new, f);
        ui.ctx().request_repaint();
    }
}

/// An interactive timeline view: one row per track with a bar per tween, labels, a time ruler
/// and a draggable playhead. Dragging seeks (and pauses while held).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let tl = glowkit_motion::Timeline::new();
/// let mut pb = glowkit_motion::Playback::new();
/// ui.add(glowkit_motion::Scrubber::new(&tl, &mut pb));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Scrubber<'a> {
    timeline: &'a Timeline,
    playback: &'a mut Playback,
    width: Option<f32>,
    row_height: f32,
    colors: Option<Vec<Color32>>,
}

impl<'a> Scrubber<'a> {
    /// View of `timeline` controlling `playback`.
    pub fn new(timeline: &'a Timeline, playback: &'a mut Playback) -> Self {
        Self { timeline, playback, width: None, row_height: 22.0, colors: None }
    }
    /// Total width (defaults to the available width).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Height of one track row.
    pub fn row_height(mut self, h: f32) -> Self {
        self.row_height = h.max(10.0);
        self
    }
    /// Bar colors per track (defaults to the theme's chart palette).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }
}

impl Widget for Scrubber<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let tracks = self.timeline.tracks();
        let label_w = 70.0;
        let ruler = 20.0;
        let w = self.width.unwrap_or_else(|| ui.available_width());
        let h = ruler + tracks.len() as f32 * self.row_height + 6.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click_and_drag());
        let duration = self.timeline.duration().max(1e-3);
        let lane = Rect::from_min_max(pos2(rect.min.x + label_w, rect.min.y), pos2(rect.max.x - 8.0, rect.max.y));
        let to_x = |time: f32| lane.min.x + time / duration * lane.width();

        if let Some(p) = resp.interact_pointer_pos() {
            if resp.dragged() || resp.clicked() || resp.is_pointer_button_down_on() {
                self.playback.seek(((p.x - lane.min.x) / lane.width()).clamp(0.0, 1.0) * duration);
                self.playback.pause();
            }
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter_at(rect.expand(4.0));
        let palette = self.colors.clone().unwrap_or_else(|| t.chart.to_vec());
        painter.rect_filled(rect, t.corner_scaled(0.8), t.muted);
        // ruler ticks
        let step = if duration > 4.0 { 1.0 } else if duration > 1.5 { 0.5 } else { 0.25 };
        let mut s = 0.0;
        while s <= duration + 1e-3 {
            let x = to_x(s);
            painter.line_segment([pos2(x, rect.min.y + 13.0), pos2(x, rect.max.y - 3.0)], Stroke::new(1.0, with_alpha(t.border, 0.6)));
            painter.text(pos2(x, rect.min.y + 3.0), egui::Align2::CENTER_TOP, format!("{s:.1}"), FontId::monospace(9.5), t.muted_foreground);
            s += step;
        }
        for (name, time) in self.timeline.labels() {
            let x = to_x(*time);
            painter.text(pos2(x + 3.0, rect.min.y + 3.0), egui::Align2::LEFT_TOP, name, FontId::proportional(9.5), t.accent);
        }
        for (row, track) in tracks.iter().enumerate() {
            let y0 = rect.min.y + ruler + row as f32 * self.row_height;
            let color = palette[row % palette.len().max(1)];
            painter.text(pos2(rect.min.x + 10.0, y0 + self.row_height / 2.0), egui::Align2::LEFT_CENTER, *track, FontId::monospace(11.0), t.muted_foreground);
            for e in self.timeline.entries().iter().filter(|e| e.track == *track) {
                let bar = Rect::from_min_max(pos2(to_x(e.start), y0 + 3.0), pos2(to_x(e.end()).max(to_x(e.start) + 3.0), y0 + self.row_height - 3.0));
                let active = self.playback.time >= e.start && self.playback.time <= e.end();
                painter.rect_filled(bar, CornerRadius::same(5), with_alpha(color, if active { 0.55 } else { 0.25 }));
                // progress fill
                let k = ((self.playback.time - e.start) / e.duration.max(1e-4)).clamp(0.0, 1.0);
                if k > 0.0 {
                    let fill = Rect::from_min_max(bar.min, pos2(bar.min.x + bar.width() * k, bar.max.y));
                    painter.rect_filled(fill, CornerRadius::same(5), with_alpha(color, 0.85));
                }
                painter.rect_stroke(bar, CornerRadius::same(5), Stroke::new(1.0, with_alpha(color, 0.9)), StrokeKind::Inside);
                if bar.width() > 46.0 {
                    painter.text(bar.left_center() + vec2(6.0, 0.0), egui::Align2::LEFT_CENTER, &e.name, FontId::proportional(10.5), t.foreground);
                }
            }
        }
        // playhead
        let x = to_x(self.playback.time.clamp(0.0, duration));
        painter.line_segment([pos2(x, rect.min.y + 2.0), pos2(x, rect.max.y - 2.0)], Stroke::new(2.0, t.foreground));
        painter.circle_filled(pos2(x, rect.min.y + 6.0), 5.0, t.foreground);
        painter.circle_filled(pos2(x, rect.min.y + 6.0), 2.5, t.accent);
        resp.on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_resolve() {
        assert_eq!(At::from("<"), At::PrevStart(0.0));
        assert_eq!(At::from(">0.25"), At::PrevEnd(0.25));
        assert_eq!(At::from("+=0.2"), At::EndOffset(0.2));
        assert_eq!(At::from("-=0.3"), At::EndOffset(-0.3));
        assert_eq!(At::from("1.5"), At::Time(1.5));
        assert_eq!(At::from("intro+=0.5"), At::Label("intro".into(), 0.5));
        let tl = Timeline::new()
            .add(Tween::new("a", 0.0_f32, 1.0).duration(1.0), ())
            .label("mid")
            .add(Tween::new("b", 0.0_f32, 1.0).duration(1.0), "<0.5")
            .add(Tween::new("c", 0.0_f32, 1.0).duration(1.0), "-=0.5")
            .add(Tween::new("d", 0.0_f32, 1.0).duration(1.0), "mid+=1");
        let s: Vec<f32> = tl.entries().iter().map(|e| e.start).collect();
        assert_eq!(s, vec![0.0, 0.5, 1.0, 2.0]);
        assert_eq!(tl.duration(), 3.0);
    }

    #[test]
    fn values_hold_between_tweens() {
        let tl = Timeline::new()
            .add(Tween::new("x", 0.0_f32, 10.0).duration(1.0).ease(Easing::Linear), 1.0)
            .add(Tween::new("x", 10.0_f32, 20.0).duration(1.0).ease(Easing::Linear), 3.0);
        assert_eq!(tl.value::<f32>("x", 0.0), Some(0.0));
        assert_eq!(tl.value::<f32>("x", 1.5), Some(5.0));
        assert_eq!(tl.value::<f32>("x", 2.5), Some(10.0));
        assert_eq!(tl.value::<f32>("x", 3.5), Some(15.0));
        assert_eq!(tl.value::<f32>("x", 9.0), Some(20.0));
        assert_eq!(tl.value::<f32>("nope", 1.0), None);
    }

    #[test]
    fn playback_loops_and_reverses() {
        let mut pb = Playback::new();
        pb.advance(0.6, 1.0);
        pb.advance(0.6, 1.0);
        assert_eq!(pb.time, 1.0);
        assert!(!pb.playing);
        pb.reverse();
        pb.advance(0.25, 1.0);
        assert!((pb.time - 0.75).abs() < 1e-5);
        let mut lp = Playback::new().looping(true, 0.0);
        lp.advance(1.2, 1.0);
        lp.advance(0.1, 1.0);
        assert!(lp.reversed && lp.time < 1.0);
    }
}
