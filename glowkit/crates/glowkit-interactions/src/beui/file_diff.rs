use super::common::{card, cr, hover_fill, icon, idle_for, morph_axes, text_width, BeIcon};
use egui::{pos2, vec2, Align2, FontId, Rect, Sense, Stroke, Ui};
use glowkit_core::anim::spring;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LineKind {
    Context,
    Added,
    Removed,
}

#[derive(Clone, Copy, Debug)]
struct DiffState {
    open: bool,
    since: f64,
}

/// An agent's file change, the way a coding assistant shows it: a one-line file header with
/// "+2 -1" counts that springs open into the diff. Removed lines are struck through in red,
/// added lines type themselves in on green, line by line, as if being written right now; the
/// chevron turns as the panel's height morphs. "Replay" rewinds the writing.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::FileDiff;
/// ui.add(
///     FileDiff::new("src/runner.rs")
///         .context("fn run(job: Job) {")
///         .removed("    job.start();")
///         .added("    job.start_with_retry(3);")
///         .context("}"),
/// );
/// ```
pub struct FileDiff {
    path: String,
    lines: Vec<(LineKind, String)>,
    width: f32,
    speed: f32,
    start_line: usize,
    autoplay: bool,
}

impl FileDiff {
    /// A diff of the file at `path`.
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into(), lines: Vec::new(), width: 520.0, speed: 60.0, start_line: 12, autoplay: true }
    }
    /// An unchanged line.
    pub fn context(mut self, s: impl Into<String>) -> Self {
        self.lines.push((LineKind::Context, s.into()));
        self
    }
    /// An added line (types itself in).
    pub fn added(mut self, s: impl Into<String>) -> Self {
        self.lines.push((LineKind::Added, s.into()));
        self
    }
    /// A removed line (gets struck through).
    pub fn removed(mut self, s: impl Into<String>) -> Self {
        self.lines.push((LineKind::Removed, s.into()));
        self
    }
    /// Width of the panel.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(280.0);
        self
    }
    /// Typing speed in characters per second.
    pub fn speed(mut self, cps: f32) -> Self {
        self.speed = cps.clamp(5.0, 400.0);
        self
    }
    /// Number of the first line shown.
    pub fn start_line(mut self, n: usize) -> Self {
        self.start_line = n;
        self
    }
    /// Open, write, close and replay on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for FileDiff {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let painter = ui.painter().clone();
        let head_h = 44.0;
        let line_h = 22.0;
        let body_h = self.lines.len() as f32 * line_h + 16.0;
        let w = self.width;
        let (outer, resp) = ui.allocate_exact_size(vec2(w, head_h + body_h + 40.0), Sense::hover());
        let id = resp.id;
        let now = ui.input(|i| i.time);
        let head = Rect::from_min_size(outer.min, vec2(w, head_h));
        let full = Rect::from_min_size(outer.min, vec2(w, head_h + body_h));
        let hr = ui.interact(head, id.with("head"), Sense::click());
        let replay = Rect::from_min_size(pos2(outer.min.x + 4.0, outer.max.y - 26.0), vec2(80.0, 22.0));
        let rr = ui.interact(replay, id.with("replay"), Sense::click());
        if hr.hovered() || rr.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let idle = idle_for(ui, id, ui.rect_contains_pointer(outer));
        let auto = self.autoplay && idle > 2.0;
        // total characters to write decides how long the writing takes
        let chars: usize = self.lines.iter().filter(|l| l.0 == LineKind::Added).map(|l| l.1.len()).sum();
        let write_time = 0.5 + chars as f32 / self.speed + 0.25 * self.lines.len() as f32 * 0.3;
        let st = glowkit_core::with_state(
            ui,
            id,
            || DiffState { open: false, since: now - 10.0 },
            |s| {
                if hr.clicked() {
                    s.open = !s.open;
                    s.since = now;
                }
                if rr.clicked() {
                    s.open = true;
                    s.since = now;
                }
                if auto {
                    let held = (now - s.since) as f32;
                    if (s.open && held > write_time + 2.2) || (!s.open && held > 1.2) {
                        s.open = !s.open;
                        s.since = now;
                    }
                }
                *s
            },
        );
        let since = (now - st.since) as f32;
        if auto || (st.open && since < write_time + 0.5) {
            glowkit_core::animate(ui);
        }
        let (rect, radius) =
            morph_axes(ui, id.with("box"), if st.open { full } else { head }, 16.0, (300.0, 30.0), (240.0, 22.0));
        if !ui.is_rect_visible(outer) {
            return resp;
        }
        card(&painter, &t, rect, radius, 0.3 + 0.4 * ((rect.height() - head_h) / body_h).clamp(0.0, 1.0));
        if hr.hovered() {
            painter.rect_filled(head.shrink(3.0), cr(13.0), with_alpha(hover_fill(&t), 0.6));
        }
        // header
        let cy = head.center().y;
        icon(&painter, BeIcon::Code, pos2(head.min.x + 22.0, cy), 14.0, t.muted_foreground, 1.3);
        painter.text(
            pos2(head.min.x + 38.0, cy),
            Align2::LEFT_CENTER,
            &self.path,
            FontId::monospace(12.5),
            t.foreground,
        );
        let adds = self.lines.iter().filter(|l| l.0 == LineKind::Added).count();
        let dels = self.lines.iter().filter(|l| l.0 == LineKind::Removed).count();
        let chev_k = spring(ui, id.with("chev"), if st.open { 1.0 } else { 0.0 }, 300.0, 20.0);
        let cc = pos2(head.max.x - 20.0, cy);
        let ang = chev_k * std::f32::consts::PI;
        let s = 4.0;
        let (sn, cs) = ang.sin_cos();
        let rot = |x: f32, y: f32| cc + vec2(x * cs - y * sn, x * sn + y * cs) * s;
        painter.add(egui::Shape::line(
            vec![rot(-1.0, -0.5), rot(0.0, 0.6), rot(1.0, -0.5)],
            Stroke::new(1.4, t.muted_foreground),
        ));
        let done = st.open && since > write_time;
        let ck = spring(ui, id.with("check"), if done { 1.0 } else { 0.0 }, 400.0, 16.0);
        icon(
            &painter,
            BeIcon::Check,
            pos2(head.max.x - 44.0, cy),
            11.0 * (0.6 + 0.4 * ck),
            mix(t.muted_foreground, t.success, ck),
            1.5,
        );
        let minus = format!("-{dels}");
        let mw = text_width(&painter, &minus, 12.5);
        painter.text(pos2(head.max.x - 60.0, cy), Align2::RIGHT_CENTER, &minus, FontId::monospace(12.5), t.destructive);
        painter.text(
            pos2(head.max.x - 66.0 - mw, cy),
            Align2::RIGHT_CENTER,
            format!("+{adds}"),
            FontId::monospace(12.5),
            t.success,
        );
        // body
        let clip = painter.with_clip_rect(rect.shrink(1.0));
        let body_top = head.max.y + 6.0;
        if rect.height() > head_h + 2.0 {
            clip.line_segment(
                [pos2(rect.min.x, head.max.y), pos2(rect.max.x, head.max.y)],
                Stroke::new(1.0, with_alpha(t.border, 0.8)),
            );
        }
        let mut clock = 0.45f32; // seconds into the writing when each line begins
        let mut ln_old = self.start_line;
        for (i, (kind, text)) in self.lines.iter().enumerate() {
            let y = body_top + i as f32 * line_h + line_h / 2.0;
            let row = Rect::from_min_size(pos2(rect.min.x + 1.0, y - line_h / 2.0), vec2(w - 2.0, line_h));
            let start = clock;
            let dur = match kind {
                LineKind::Added => text.len() as f32 / self.speed,
                LineKind::Removed => 0.35,
                LineKind::Context => 0.0,
            };
            clock += dur + if *kind == LineKind::Context { 0.0 } else { 0.12 };
            let p = if st.open { ((since - start) / dur.max(1e-3)).clamp(0.0, 1.0) } else { 1.0 };
            let started = !st.open || since >= start;
            let mono = FontId::monospace(12.0);
            let num_col = with_alpha(t.muted_foreground, 0.7);
            match kind {
                LineKind::Context => {
                    clip.text(
                        pos2(row.min.x + 30.0, y),
                        Align2::RIGHT_CENTER,
                        ln_old.to_string(),
                        mono.clone(),
                        num_col,
                    );
                    clip.text(
                        pos2(row.min.x + 58.0, y),
                        Align2::LEFT_CENTER,
                        text,
                        mono,
                        mix(t.foreground, t.muted_foreground, 0.35),
                    );
                    ln_old += 1;
                }
                LineKind::Removed => {
                    let a = if started { 1.0 } else { 0.0 };
                    clip.rect_filled(row, cr(0.0), with_alpha(t.destructive, 0.1 * a));
                    clip.text(
                        pos2(row.min.x + 30.0, y),
                        Align2::RIGHT_CENTER,
                        ln_old.to_string(),
                        mono.clone(),
                        num_col,
                    );
                    clip.text(pos2(row.min.x + 42.0, y), Align2::LEFT_CENTER, "-", mono.clone(), t.destructive);
                    let r = clip.text(
                        pos2(row.min.x + 58.0, y),
                        Align2::LEFT_CENTER,
                        text,
                        mono,
                        mix(t.foreground, t.destructive, 0.6 * a),
                    );
                    if started && p > 0.0 {
                        let x1 = r.min.x + (r.width()) * p;
                        clip.line_segment(
                            [pos2(r.min.x, y + 0.5), pos2(x1, y + 0.5)],
                            Stroke::new(1.2, with_alpha(t.destructive, 0.8)),
                        );
                    }
                    ln_old += 1;
                }
                LineKind::Added => {
                    if !started {
                        continue;
                    }
                    let grow = if st.open { ((since - start) / 0.2).clamp(0.0, 1.0) } else { 1.0 };
                    clip.rect_filled(row, cr(0.0), with_alpha(t.success, 0.12 * grow));
                    clip.text(pos2(row.min.x + 42.0, y), Align2::LEFT_CENTER, "+", mono.clone(), t.success);
                    let n = ((text.len() as f32) * p).round() as usize;
                    let shown: String = text.chars().take(n).collect();
                    let r = clip.text(pos2(row.min.x + 58.0, y), Align2::LEFT_CENTER, &shown, mono, t.foreground);
                    if st.open && p < 1.0 && (now * 2.5) as i64 % 2 == 0 {
                        clip.rect_filled(
                            Rect::from_min_size(pos2(r.max.x + 1.0, y - 7.0), vec2(7.0, 14.0)),
                            cr(1.0),
                            with_alpha(t.accent, 0.8),
                        );
                    }
                }
            }
        }
        // replay
        let rc = if rr.hovered() { t.foreground } else { t.muted_foreground };
        icon(&painter, BeIcon::Refresh, pos2(replay.min.x + 10.0, replay.center().y), 11.0, rc, 1.3);
        painter.text(
            pos2(replay.min.x + 22.0, replay.center().y),
            Align2::LEFT_CENTER,
            "Replay",
            FontId::proportional(12.5),
            rc,
        );
        resp
    }
}
