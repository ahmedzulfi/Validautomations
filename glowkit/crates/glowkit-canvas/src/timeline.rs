//! [`TimelineEditor`]: video-editor style tracks with clips, a scrubbable playhead and a
//! zoomable ruler.

use crate::common::{self, icon_button, paint_icon, qfont, qmono, Icon};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};
use glowkit_core::{color::with_alpha, noise, Theme};

/// What a clip / track holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClipKind {
    Video,
    Audio,
    Title,
    Effect,
}

/// A clip on a track (times in seconds).
#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    pub id: u64,
    pub name: String,
    pub start: f32,
    pub len: f32,
    pub color: Option<Color32>,
}

impl Clip {
    /// A clip.
    pub fn new(id: u64, name: impl Into<String>, start: f32, len: f32) -> Self {
        Self { id, name: name.into(), start, len, color: None }
    }
    /// Builder: color.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// End time.
    pub fn end(&self) -> f32 {
        self.start + self.len
    }
}

/// A track (lane) of clips.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub name: String,
    pub kind: ClipKind,
    pub clips: Vec<Clip>,
    pub muted: bool,
    pub locked: bool,
}

impl Track {
    /// A track.
    pub fn new(name: impl Into<String>, kind: ClipKind, clips: Vec<Clip>) -> Self {
        Self { name: name.into(), kind, clips, muted: false, locked: false }
    }
    fn height(&self) -> f32 {
        match self.kind {
            ClipKind::Video => 58.0,
            ClipKind::Audio => 48.0,
            _ => 36.0,
        }
    }
}

/// The project: tracks, duration and frame rate. Owned by the caller.
#[derive(Clone, Debug, PartialEq)]
pub struct Timeline {
    pub tracks: Vec<Track>,
    pub duration: f32,
    pub fps: f32,
}

/// Format seconds as `MM:SS:FF` timecode.
pub fn timecode(t: f32, fps: f32) -> String {
    let t = t.max(0.0);
    let frames = (t * fps).round() as i64;
    let f = frames % fps as i64;
    let s = (frames / fps as i64) % 60;
    let m = frames / fps as i64 / 60;
    format!("{m:02}:{s:02}:{f:02}")
}

/// Pick a "nice" ruler step (seconds) so labels are at least `min_px` apart at `px_per_sec`.
pub fn ruler_step(px_per_sec: f32, min_px: f32) -> f32 {
    const STEPS: [f32; 12] = [0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0];
    STEPS.iter().copied().find(|s| s * px_per_sec >= min_px).unwrap_or(600.0)
}

/// Snap `t` to the closest of `targets` within `tol` seconds.
pub fn snap_to(t: f32, targets: &[f32], tol: f32) -> Option<f32> {
    targets.iter().copied().filter(|x| (x - t).abs() <= tol).min_by(|a, b| (a - t).abs().total_cmp(&(b - t).abs()))
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Move,
    TrimL,
    TrimR,
}

#[derive(Clone, Copy, Debug)]
struct ClipDrag {
    id: u64,
    mode: Mode,
    grab_t: f32,
    start: f32,
    len: f32,
    track: usize,
}

/// Caller-owned state of a [`TimelineEditor`].
#[derive(Clone, Debug)]
pub struct TimelineState {
    /// Playhead time in seconds.
    pub playhead: f32,
    pub playing: bool,
    /// Zoom (pixels per second).
    pub px_per_sec: f32,
    /// First visible second.
    pub scroll: f32,
    pub selected: Option<u64>,
    /// Snap clips to edges and the playhead.
    pub snapping: bool,
    drag: Option<ClipDrag>,
    snap_line: Option<f32>,
    scrubbing: bool,
}

impl Default for TimelineState {
    fn default() -> Self {
        Self { playhead: 0.0, playing: false, px_per_sec: 34.0, scroll: 0.0, selected: None, snapping: true, drag: None, snap_line: None, scrubbing: false }
    }
}

/// A non-linear video editor timeline (Premiere / DaVinci / CapCut pattern): stacked video,
/// title and audio tracks with clip thumbnails and waveforms, drag to move clips across tracks,
/// trim from either edge with magnetic snapping, a scrubbable playhead and a zoomable ruler.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_canvas::{Clip, ClipKind, Timeline, TimelineEditor, TimelineState, Track};
/// # let mut state = TimelineState::default();
/// let mut tl = Timeline { fps: 30.0, duration: 60.0, tracks: vec![Track::new("V1", ClipKind::Video, vec![Clip::new(1, "Intro", 0.0, 6.0)])] };
/// TimelineEditor::new(&mut tl, &mut state).height(420.0).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct TimelineEditor<'a> {
    tl: &'a mut Timeline,
    state: &'a mut TimelineState,
    height: f32,
    header: f32,
}

const TOOLBAR: f32 = 44.0;
const RULER: f32 = 30.0;

impl<'a> TimelineEditor<'a> {
    /// An editor for `tl`.
    pub fn new(tl: &'a mut Timeline, state: &'a mut TimelineState) -> Self {
        Self { tl, state, height: 420.0, header: 156.0 }
    }
    /// Height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }

    /// Show the editor. Returns `true` when clips changed.
    pub fn show(self, ui: &mut Ui) -> bool {
        let TimelineEditor { tl, state: st, height, header } = self;
        let t = Theme::of(ui);
        let id = ui.scope_id().with("glowkit_timeline");
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return false;
        }
        let dt = glowkit_core::dt(ui);
        let time = glowkit_core::time(ui);
        let mut changed = false;
        let painter = ui.painter_at(rect);
        let panel_bg = if t.dark { t.card } else { t.muted };
        painter.rect(rect, CornerRadius::same(12), panel_bg, Stroke::new(1.0, t.border), StrokeKind::Inside);
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let lanes_x = rect.left() + header;
        let lanes_w = rect.right() - lanes_x - 10.0;
        let ruler = Rect::from_min_size(pos2(lanes_x, rect.top() + TOOLBAR), vec2(lanes_w, RULER));
        let tracks_top = ruler.bottom() + 6.0;
        let lanes = Rect::from_min_max(pos2(lanes_x, tracks_top), pos2(lanes_x + lanes_w, rect.bottom() - 10.0));

        // ---- playback
        if st.playing {
            st.playhead += dt;
            if st.playhead > tl.duration {
                st.playhead = 0.0;
            }
            // keep the playhead visible
            let vis = lanes_w / st.px_per_sec;
            if st.playhead > st.scroll + vis * 0.85 {
                st.scroll = st.playhead - vis * 0.15;
            }
            ui.ctx().request_repaint();
        }

        // ---- zoom / scroll
        if ui.rect_contains_pointer(rect) {
            let (z, d, p) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta, i.pointer.hover_pos()));
            if (z - 1.0).abs() > 1e-4 {
                let anchor = p.map(|p| p.x).unwrap_or(lanes.center().x);
                let at = st.scroll + (anchor - lanes_x) / st.px_per_sec;
                st.px_per_sec = (st.px_per_sec * z).clamp(4.0, 400.0);
                st.scroll = at - (anchor - lanes_x) / st.px_per_sec;
            }
            if d != Vec2::ZERO {
                st.scroll -= (if d.x.abs() > d.y.abs() { d.x } else { d.y }) / st.px_per_sec;
                ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
            }
        }
        st.scroll = st.scroll.clamp(-1.0, (tl.duration - lanes_w / st.px_per_sec + 2.0).max(-1.0));
        let px = st.px_per_sec;
        let scroll = st.scroll;
        let x_of = move |s: f32| lanes_x + (s - scroll) * px;
        let t_of = move |x: f32| scroll + (x - lanes_x) / px;

        // ---- toolbar
        let play_r = Rect::from_center_size(pos2(rect.left() + 30.0, rect.top() + TOOLBAR / 2.0 + 2.0), Vec2::splat(32.0));
        let pr = ui.interact(play_r, id.with("play"), Sense::click());
        let ph = ui.ctx().animate_bool_with_time(id.with("playh"), pr.hovered(), 0.1);
        painter.circle_filled(play_r.center(), 15.0 + ph, t.accent);
        paint_icon(&painter, play_r.shrink(9.0).translate(vec2(if st.playing { 0.0 } else { 1.0 }, 0.0)), if st.playing { Icon::Pause } else { Icon::Play }, t.accent_foreground);
        if pr.on_hover_cursor(CursorIcon::PointingHand).clicked() {
            st.playing = !st.playing;
        }
        let tc = timecode(st.playhead, tl.fps);
        painter.text(pos2(rect.left() + 56.0, play_r.center().y), Align2::LEFT_CENTER, tc, qmono(16.0), t.foreground);
        painter.text(pos2(rect.left() + 56.0 + 92.0, play_r.center().y + 1.0), Align2::LEFT_CENTER, format!("/ {}", timecode(tl.duration, tl.fps)), qmono(11.0), t.muted_foreground);
        // zoom controls (right)
        let zr = Rect::from_min_size(pos2(rect.right() - 196.0, rect.top() + 9.0), vec2(186.0, 28.0));
        if icon_button(ui, id.with("zo"), Rect::from_min_size(zr.min, Vec2::splat(28.0)), Icon::Minus, false, "Zoom out").clicked() {
            st.px_per_sec = (st.px_per_sec / 1.4).max(4.0);
        }
        if icon_button(ui, id.with("zi"), Rect::from_min_size(pos2(zr.right() - 28.0, zr.top()), Vec2::splat(28.0)), Icon::Plus, false, "Zoom in").clicked() {
            st.px_per_sec = (st.px_per_sec * 1.4).min(400.0);
        }
        let track_r = Rect::from_min_max(pos2(zr.left() + 34.0, zr.center().y - 2.0), pos2(zr.right() - 34.0, zr.center().y + 2.0));
        painter.rect_filled(track_r, CornerRadius::same(2), with_alpha(t.foreground, 0.12));
        let zk = ((st.px_per_sec / 4.0).ln() / (100.0f32).ln()).clamp(0.0, 1.0);
        let knob = pos2(track_r.left() + track_r.width() * zk, track_r.center().y);
        let kresp = ui.interact(Rect::from_center_size(knob, Vec2::splat(16.0)).union(track_r.expand2(vec2(0.0, 6.0))), id.with("zk"), Sense::click_and_drag());
        if let (true, Some(p)) = (kresp.dragged() || kresp.clicked(), kresp.interact_pointer_pos()) {
            let k = ((p.x - track_r.left()) / track_r.width()).clamp(0.0, 1.0);
            let anchor_t = st.playhead;
            let ax = x_of(anchor_t);
            st.px_per_sec = 4.0 * 100f32.powf(k);
            st.scroll = anchor_t - (ax - lanes_x) / st.px_per_sec;
        }
        let kh = ui.ctx().animate_bool_with_time(id.with("kh"), kresp.hovered() || kresp.dragged(), 0.1);
        painter.rect_filled(Rect::from_min_max(track_r.min, pos2(knob.x, track_r.max.y)), CornerRadius::same(2), t.accent);
        painter.circle(knob, 6.0 + kh * 1.5, t.foreground, Stroke::new(2.0, t.accent));
        // snapping toggle
        let snap_r = Rect::from_min_size(pos2(zr.left() - 40.0, zr.top()), Vec2::splat(28.0));
        if icon_button(ui, id.with("snap"), snap_r, Icon::Magnet, st.snapping, "Snapping").clicked() {
            st.snapping = !st.snapping;
        }

        // ---- ruler
        let rp = painter.with_clip_rect(ruler.expand2(vec2(0.0, 2.0)));
        rp.rect_filled(ruler, CornerRadius::same(6), with_alpha(t.foreground, 0.035));
        let step = ruler_step(px, 70.0);
        let minor = step / 5.0;
        let mut s = (scroll / minor).floor() * minor;
        while x_of(s) < ruler.right() + 1.0 {
            let x = x_of(s);
            let major = ((s / step).round() * step - s).abs() < minor * 0.1;
            if s >= -1e-3 && s <= tl.duration + 1e-3 {
                let h = if major { 10.0 } else { 5.0 };
                rp.line_segment([pos2(x, ruler.bottom() - h), pos2(x, ruler.bottom())], Stroke::new(1.0, if major { t.muted_foreground } else { with_alpha(t.muted_foreground, 0.5) }));
                if major {
                    let label = if step < 1.0 { format!("{s:.2}s") } else { let si = s.round() as i64; format!("{:02}:{:02}", si / 60, si % 60) };
                    rp.text(pos2(x + 4.0, ruler.top() + 9.0), Align2::LEFT_CENTER, label, qmono(10.0), t.muted_foreground);
                }
            }
            s += minor;
        }
        let rresp = ui.interact(ruler, id.with("ruler"), Sense::click_and_drag());
        if rresp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
        }
        if rresp.is_pointer_button_down_on() {
            if let Some(p) = rresp.interact_pointer_pos() {
                st.playhead = t_of(p.x).clamp(0.0, tl.duration);
                st.scrubbing = true;
            }
        } else {
            st.scrubbing = false;
        }

        // ---- track headers + lanes
        let lp = painter.with_clip_rect(lanes);
        let mut y = tracks_top;
        let mut lane_rects = Vec::new();
        let mut toggles: Vec<(usize, u8)> = Vec::new();
        for (ti, tr) in tl.tracks.iter().enumerate() {
            let h = tr.height();
            let lane = Rect::from_min_size(pos2(lanes_x, y), vec2(lanes_w, h));
            lane_rects.push(lane);
            lp.rect_filled(lane, CornerRadius::same(6), with_alpha(t.foreground, if t.dark { 0.025 } else { 0.05 }));
            // header
            let hr = Rect::from_min_max(pos2(rect.left() + 10.0, y), pos2(lanes_x - 8.0, y + h));
            painter.rect(hr, CornerRadius::same(7), if t.dark { t.background } else { t.card }, Stroke::new(1.0, t.border), StrokeKind::Inside);
            let kind_col = match tr.kind {
                ClipKind::Video => t.chart[0],
                ClipKind::Audio => t.chart[4],
                ClipKind::Title => t.chart[3],
                ClipKind::Effect => t.chart[2],
            };
            painter.rect_filled(Rect::from_min_size(hr.min + vec2(0.0, 6.0), vec2(3.0, h - 12.0)), CornerRadius::same(2), kind_col);
            painter.text(pos2(hr.left() + 12.0, hr.center().y), Align2::LEFT_CENTER, &tr.name, qfont(12.5), if tr.muted { t.muted_foreground } else { t.foreground });
            for (k, (label, on)) in [("M", tr.muted), ("L", tr.locked)].iter().enumerate() {
                let br = Rect::from_center_size(pos2(hr.right() - 14.0 - k as f32 * 24.0, hr.center().y), vec2(20.0, 18.0));
                let resp = ui.interact(br, id.with(("tg", ti, k)), Sense::click());
                let hv = ui.ctx().animate_bool_with_time(id.with(("tgh", ti, k)), resp.hovered(), 0.1);
                painter.rect(br, CornerRadius::same(4), if *on { with_alpha(if k == 0 { t.warning } else { t.destructive }, 0.25) } else { with_alpha(t.foreground, 0.05 + 0.06 * hv) }, Stroke::NONE, StrokeKind::Inside);
                painter.text(br.center(), Align2::CENTER_CENTER, *label, qfont(10.0), if *on { t.foreground } else { t.muted_foreground });
                if resp.on_hover_text(if k == 0 { "Mute" } else { "Lock" }).on_hover_cursor(CursorIcon::PointingHand).clicked() {
                    toggles.push((ti, k as u8));
                }
            }
            y += h + 6.0;
        }
        for (ti, k) in toggles {
            if k == 0 {
                tl.tracks[ti].muted = !tl.tracks[ti].muted;
            } else {
                tl.tracks[ti].locked = !tl.tracks[ti].locked;
            }
        }

        // ---- clip interaction
        let lane_at = |y: f32| lane_rects.iter().position(|r| y >= r.top() - 3.0 && y <= r.bottom() + 3.0);
        let mut hovered: Option<(u64, Option<Mode>)> = None;
        for (ti, tr) in tl.tracks.iter().enumerate() {
            let lane = lane_rects[ti];
            for c in &tr.clips {
                let r = Rect::from_min_max(pos2(x_of(c.start), lane.top() + 3.0), pos2(x_of(c.end()), lane.bottom() - 3.0));
                if !r.intersects(lanes) || tr.locked {
                    continue;
                }
                let resp = ui.interact(r.intersect(lanes), id.with(("clip", c.id)), Sense::click_and_drag());
                // classify by where the press happened (the drag only starts after a threshold)
                let p = if resp.drag_started() { ui.input(|i| i.pointer.press_origin()).or(pointer).unwrap_or_default() } else { pointer.unwrap_or_default() };
                let edge = if (p.x - r.left()).abs() < 7.0 {
                    Some(Mode::TrimL)
                } else if (p.x - r.right()).abs() < 7.0 {
                    Some(Mode::TrimR)
                } else {
                    None
                };
                if resp.hovered() {
                    hovered = Some((c.id, edge));
                    ui.ctx().set_cursor_icon(if edge.is_some() { CursorIcon::ResizeColumn } else { CursorIcon::Grab });
                }
                if resp.clicked() {
                    st.selected = Some(c.id);
                }
                if resp.drag_started() {
                    st.selected = Some(c.id);
                    st.drag = Some(ClipDrag { id: c.id, mode: edge.unwrap_or(Mode::Move), grab_t: t_of(p.x), start: c.start, len: c.len, track: ti });
                }
            }
        }
        let bg = ui.interact(lanes, id.with("lanes_bg"), Sense::click());
        if bg.clicked() {
            st.selected = None;
        }
        st.snap_line = None;
        if let Some(d) = st.drag {
            if let Some(p) = pointer {
                let dt_ = t_of(p.x) - d.grab_t;
                // snap targets: other clip edges, playhead, zero
                let mut targets = vec![0.0, st.playhead];
                for tr in &tl.tracks {
                    for c in &tr.clips {
                        if c.id != d.id {
                            targets.push(c.start);
                            targets.push(c.end());
                        }
                    }
                }
                let tol = 8.0 / px;
                let snapping = st.snapping && !ui.input(|i| i.modifiers.alt);
                let target_track = lane_at(p.y).filter(|k| tl.tracks[*k].kind == tl.tracks[d.track].kind && !tl.tracks[*k].locked).unwrap_or(d.track);
                let mut new = (d.start, d.len);
                match d.mode {
                    Mode::Move => {
                        let mut s = (d.start + dt_).max(0.0);
                        if snapping {
                            let a = snap_to(s, &targets, tol).map(|x| (x - s, x));
                            let b = snap_to(s + d.len, &targets, tol).map(|x| (x - (s + d.len), x));
                            let best = match (a, b) {
                                (Some(a), Some(b)) => Some(if a.0.abs() <= b.0.abs() { a } else { b }),
                                (a, b) => a.or(b),
                            };
                            if let Some((off, line)) = best {
                                s += off;
                                st.snap_line = Some(line);
                            }
                        }
                        new.0 = s;
                    }
                    Mode::TrimL => {
                        let mut s = (d.start + dt_).clamp(0.0, d.start + d.len - 0.2);
                        if snapping {
                            if let Some(x) = snap_to(s, &targets, tol) {
                                s = x.min(d.start + d.len - 0.2);
                                st.snap_line = Some(x);
                            }
                        }
                        new = (s, d.start + d.len - s);
                    }
                    Mode::TrimR => {
                        let mut e = (d.start + d.len + dt_).max(d.start + 0.2);
                        if snapping {
                            if let Some(x) = snap_to(e, &targets, tol) {
                                e = x.max(d.start + 0.2);
                                st.snap_line = Some(x);
                            }
                        }
                        new.1 = e - d.start;
                    }
                }
                // move between tracks
                let cur_track = tl.tracks.iter().position(|tr| tr.clips.iter().any(|c| c.id == d.id));
                if let Some(ct) = cur_track {
                    if ct != target_track && d.mode == Mode::Move {
                        let k = tl.tracks[ct].clips.iter().position(|c| c.id == d.id).expect("clip");
                        let c = tl.tracks[ct].clips.remove(k);
                        tl.tracks[target_track].clips.push(c);
                        changed = true;
                    }
                }
                for tr in &mut tl.tracks {
                    for c in &mut tr.clips {
                        if c.id == d.id && (c.start, c.len) != new {
                            c.start = new.0;
                            c.len = new.1;
                            changed = true;
                        }
                    }
                }
                ui.ctx().set_cursor_icon(if d.mode == Mode::Move { CursorIcon::Grabbing } else { CursorIcon::ResizeColumn });
            }
            if ui.input(|i| !i.pointer.primary_down()) {
                st.drag = None;
                st.snap_line = None;
            }
        }
        if ui.rect_contains_pointer(rect) && !ui.ctx().text_edit_focused() && common::pressed(ui, egui::Key::Space) {
            st.playing = !st.playing;
        }
        if let Some(sel) = st.selected {
            if ui.rect_contains_pointer(rect) && (common::pressed(ui, egui::Key::Delete) || common::pressed(ui, egui::Key::Backspace)) {
                for tr in &mut tl.tracks {
                    tr.clips.retain(|c| c.id != sel);
                }
                st.selected = None;
                changed = true;
            }
        }

        // ---- paint clips
        for (ti, tr) in tl.tracks.iter().enumerate() {
            let lane = lane_rects[ti];
            let base = match tr.kind {
                ClipKind::Video => t.chart[0],
                ClipKind::Audio => t.chart[4],
                ClipKind::Title => t.chart[3],
                ClipKind::Effect => t.chart[2],
            };
            for c in &tr.clips {
                let r = Rect::from_min_max(pos2(x_of(c.start), lane.top() + 3.0), pos2(x_of(c.end()), lane.bottom() - 3.0));
                if !r.intersects(lanes) {
                    continue;
                }
                let col = c.color.unwrap_or(base);
                let sel = ui.ctx().animate_bool_with_time(id.with(("cs", c.id)), st.selected == Some(c.id), 0.12);
                let hov = ui.ctx().animate_bool_with_time(id.with(("ch", c.id)), hovered.is_some_and(|h| h.0 == c.id) || st.drag.is_some_and(|d| d.id == c.id), 0.12);
                let dim = if tr.muted { 0.45 } else { 1.0 };
                let mut cp = lp.clone();
                cp.set_opacity(dim);
                let rr = CornerRadius::same(6);
                cp.rect_filled(r, rr, col.lerp_to_gamma(Color32::BLACK, if t.dark { 0.35 } else { 0.1 }));
                let inner = r.shrink(1.5);
                match tr.kind {
                    ClipKind::Video => {
                        // thumbnail strip
                        let thumb_w = (inner.height() - 16.0) * 16.0 / 9.0;
                        let strip = Rect::from_min_max(pos2(inner.left(), inner.top() + 16.0), inner.max);
                        let tcp = cp.with_clip_rect(strip.intersect(lanes));
                        let mut x = strip.left();
                        let mut k = 0;
                        while x < strip.right() {
                            let tr_ = Rect::from_min_size(pos2(x, strip.top()), vec2(thumb_w - 1.0, strip.height()));
                            let seed = c.id as f32 * 3.1 + k as f32 * 0.7;
                            let c1 = glowkit_core::color::hsv((noise::noise2(seed, 1.0) * 120.0 + c.id as f32 * 40.0) % 360.0, 0.45, 0.55);
                            let c2 = glowkit_core::color::hsv((noise::noise2(seed, 5.0) * 120.0 + c.id as f32 * 40.0 + 40.0) % 360.0, 0.5, 0.3);
                            glowkit_core::paint::gradient_rect(&tcp, tr_, 2.0, 70.0, &[c1, c2]);
                            // a little "horizon" for depth
                            tcp.line_segment([pos2(tr_.left(), tr_.center().y + 3.0), pos2(tr_.right(), tr_.center().y - 2.0)], Stroke::new(1.0, with_alpha(Color32::WHITE, 0.15)));
                            x += thumb_w;
                            k += 1;
                        }
                        cp.text(inner.left_top() + vec2(6.0, 8.0), Align2::LEFT_CENTER, common::elide(&cp, &c.name, qfont(11.0), inner.width() - 10.0), qfont(11.0), Color32::WHITE);
                    }
                    ClipKind::Audio => {
                        cp.rect_filled(inner, CornerRadius::same(5), col.lerp_to_gamma(Color32::BLACK, if t.dark { 0.55 } else { 0.25 }));
                        let mid = inner.center().y + 5.0;
                        let bar = 3.0;
                        let x0 = inner.left().max(lanes.left());
                        let mut x = inner.left() + ((x0 - inner.left()) / bar).floor() * bar;
                        let wave_col = col.lerp_to_gamma(Color32::WHITE, 0.25);
                        while x < inner.right().min(lanes.right()) {
                            let ts_ = t_of(x) - c.start;
                            let amp = (noise::noise2(ts_ * 3.0, c.id as f32) * 0.5 + 0.5).powf(1.5) * (0.6 + 0.4 * (ts_ * 1.7).sin().abs());
                            let h = (amp * (inner.height() - 18.0) * 0.5).max(1.0);
                            cp.line_segment([pos2(x, mid - h), pos2(x, mid + h)], Stroke::new(1.6, wave_col));
                            x += bar;
                        }
                        cp.text(inner.left_top() + vec2(6.0, 8.0), Align2::LEFT_CENTER, common::elide(&cp, &c.name, qfont(11.0), inner.width() - 10.0), qfont(11.0), Color32::WHITE);
                    }
                    _ => {
                        glowkit_core::paint::gradient_rect(&cp, inner, 5.0, 0.0, &[col.lerp_to_gamma(Color32::BLACK, 0.2), col.lerp_to_gamma(Color32::BLACK, 0.45)]);
                        cp.text(inner.left_center() + vec2(8.0, 0.0), Align2::LEFT_CENTER, common::elide(&cp, &c.name, qfont(11.5), inner.width() - 14.0), qfont(11.5), Color32::WHITE);
                    }
                }
                let border = with_alpha(col.lerp_to_gamma(Color32::WHITE, 0.4), 0.6 + 0.4 * hov).lerp_to_gamma(t.foreground, sel);
                cp.rect_stroke(r, rr, Stroke::new(1.0 + sel * 1.2, border), StrokeKind::Inside);
                if hov > 0.05 && r.width() > 24.0 {
                    // trim brackets
                    for (x, dir) in [(r.left() + 4.0, 1.0), (r.right() - 4.0, -1.0)] {
                        let pts = vec![pos2(x + dir * 3.0, r.top() + 8.0), pos2(x, r.top() + 8.0), pos2(x, r.bottom() - 8.0), pos2(x + dir * 3.0, r.bottom() - 8.0)];
                        cp.add(Shape::line(pts, Stroke::new(2.0, with_alpha(Color32::WHITE, 0.85 * hov))));
                    }
                }
            }
        }
        // snap line
        if let Some(sx) = st.snap_line {
            let x = x_of(sx);
            common::dashed_polyline(&painter, &[pos2(x, ruler.top()), pos2(x, lanes.bottom())], 4.0, 3.0, 0.0, Stroke::new(1.2, t.warning));
        }
        // ---- playhead
        let phx = x_of(st.playhead);
        if phx >= lanes.left() - 1.0 && phx <= lanes.right() + 1.0 {
            let col = t.destructive;
            painter.line_segment([pos2(phx, ruler.top()), pos2(phx, lanes.bottom())], Stroke::new(1.5, col));
            let top = ruler.top();
            let head = vec![pos2(phx - 7.0, top), pos2(phx + 7.0, top), pos2(phx + 7.0, top + 10.0), pos2(phx, top + 17.0), pos2(phx - 7.0, top + 10.0)];
            painter.add(Shape::convex_polygon(head, col, Stroke::NONE));
            if st.playing || st.scrubbing {
                glowkit_core::paint::glow(&painter, pos2(phx, top + 8.0), 14.0 + 3.0 * (time * 6.0).sin(), with_alpha(col, 0.35));
            }
        }
        changed
    }
}

/// Unused helper retained for API symmetry.
#[allow(dead_code)]
fn _p(_: Pos2) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timecodes() {
        assert_eq!(timecode(0.0, 30.0), "00:00:00");
        assert_eq!(timecode(61.5, 30.0), "01:01:15");
        assert_eq!(timecode(-3.0, 24.0), "00:00:00");
    }

    #[test]
    fn ruler_steps() {
        assert_eq!(ruler_step(100.0, 70.0), 1.0);
        assert_eq!(ruler_step(10.0, 70.0), 10.0);
        assert_eq!(ruler_step(1000.0, 70.0), 0.1);
    }

    #[test]
    fn snapping() {
        assert_eq!(snap_to(4.95, &[0.0, 5.0, 10.0], 0.1), Some(5.0));
        assert_eq!(snap_to(4.5, &[0.0, 5.0, 10.0], 0.1), None);
    }
}
