//! Quality and memory over time: [`EvalScoreboard`], [`MemoryViewer`] and [`ScheduleList`].

use super::common::*;
use egui::{pos2, vec2, Align2, CursorIcon, Rect, Response, Sense, Stroke, StrokeKind, Ui};
use glowkit_ai::Icon;
use glowkit_core::anim::{ease, Easing};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;
use std::f32::consts::{FRAC_PI_2, TAU};

/// One eval suite in an [`EvalScoreboard`].
#[derive(Clone, Debug, Default)]
pub struct EvalSuite {
    /// Suite name.
    pub name: String,
    /// Passed cases.
    pub passed: u32,
    /// Failed cases.
    pub failed: u32,
    /// Flaky cases.
    pub flaky: u32,
    /// Total cases (cases not yet run fill the rest of the bar).
    pub total: u32,
    /// Pass-rate change vs. the baseline, in percentage points.
    pub delta: f32,
}

impl EvalSuite {
    /// A suite with `total` cases.
    pub fn new(name: impl Into<String>, total: u32) -> Self {
        Self { name: name.into(), total: total.max(1), ..Default::default() }
    }
    /// Passed, failed and flaky counts.
    pub fn results(mut self, passed: u32, failed: u32, flaky: u32) -> Self {
        self.passed = passed;
        self.failed = failed;
        self.flaky = flaky;
        self
    }
    /// Change vs. baseline in points.
    pub fn delta(mut self, d: f32) -> Self {
        self.delta = d;
        self
    }
    fn done(&self) -> u32 {
        self.passed + self.failed + self.flaky
    }
}

/// An eval report: the overall pass rate with its change against the baseline and a trend line,
/// then one row per suite with a stacked pass / flaky / fail bar that grows as cases finish,
/// "46 / 48" counts and a colored delta. A suite still running shows a spinner and the cases
/// left; regressions are tagged.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{EvalScoreboard, EvalSuite};
/// let suites = vec![
///     EvalSuite::new("Refund flows", 48).results(46, 1, 1).delta(2.1),
///     EvalSuite::new("Outreach tone", 42).results(30, 8, 0).delta(-4.8),
/// ];
/// ui.add(EvalScoreboard::new(&suites).baseline("v2.0.3").trend(vec![91.0, 93.0, 94.5, 92.0, 94.6]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct EvalScoreboard<'a> {
    suites: &'a [EvalSuite],
    baseline: String,
    trend: Vec<f32>,
    width: f32,
}

impl<'a> EvalScoreboard<'a> {
    /// A scoreboard over `suites`.
    pub fn new(suites: &'a [EvalSuite]) -> Self {
        Self { suites, baseline: "baseline".into(), trend: Vec::new(), width: 720.0 }
    }
    /// Baseline label, e.g. a version.
    pub fn baseline(mut self, s: impl Into<String>) -> Self {
        self.baseline = s.into();
        self
    }
    /// Pass-rate history in percent (oldest first).
    pub fn trend(mut self, v: Vec<f32>) -> Self {
        self.trend = v;
        self
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(420.0);
        self
    }
}

impl egui::Widget for EvalScoreboard<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let now = glowkit_core::time(ui);
        let id = ui.next_auto_id().with("evals");
        let row_h = 46.0;
        let header = 108.0;
        let h = header + 30.0 + self.suites.len() as f32 * row_h + 10.0;
        let (rect, resp) = alloc_centered(ui, self.width, h, Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        panel(&p, &t, rect, 14.0);
        let pad = 18.0;
        let passed: u32 = self.suites.iter().map(|s| s.passed).sum();
        let done: u32 = self.suites.iter().map(|s| s.done()).sum();
        let total: u32 = self.suites.iter().map(|s| s.total).sum();
        let rate = if done > 0 { passed as f32 / done as f32 * 100.0 } else { 0.0 };
        let rate_v = glowkit_core::anim::follow(ui, id.with("rate"), rate, 4.0);
        text(&p, pos2(rect.min.x + pad, rect.min.y + 22.0), Align2::LEFT_CENTER, "Pass rate", font(12.0), dim(&t), 120.0);
        let big = text(&p, pos2(rect.min.x + pad, rect.min.y + 66.0), Align2::LEFT_BOTTOM, &format!("{rate_v:.1}%"), mono(28.0), t.foreground, 160.0);
        let base = self.trend.first().copied().unwrap_or(rate);
        let d = rate - self.trend.iter().rev().nth(1).copied().unwrap_or(base);
        let dc = if d >= 0.0 { t.success } else { t.destructive };
        badge(&p, &t, pos2(big.max.x + 10.0, big.center().y), Align2::LEFT_CENTER, &format!("{}{:.1} pts", if d >= 0.0 { "+" } else { "" }, d), dc, false);
        text(&p, pos2(rect.min.x + pad, rect.min.y + 84.0), Align2::LEFT_CENTER, &format!("{done} of {total} cases · against {}", self.baseline), font(11.5), faint(&t), 300.0);
        // trend
        let tr = Rect::from_min_max(pos2(rect.center().x + 20.0, rect.min.y + 22.0), pos2(rect.max.x - pad, rect.min.y + 86.0));
        if self.trend.len() > 1 {
            let mut v = self.trend.clone();
            if done > 0 {
                if let Some(l) = v.last_mut() {
                    *l = rate_v;
                }
            }
            sparkline(&p, tr, &v, t.accent);
        }
        hline(&p, &t, rect.min.x, rect.max.x, rect.min.y + header);
        let cy = rect.min.y + header + 15.0;
        let bar_x = rect.min.x + rect.width() * 0.32;
        let bar_w = rect.width() * 0.4;
        for (label, x) in [("SUITE", rect.min.x + pad), ("RESULTS", bar_x), ("CASES", bar_x + bar_w + 16.0), ("DELTA", rect.max.x - pad - 50.0)] {
            text(&p, pos2(x, cy), Align2::LEFT_CENTER, label, font(9.5), faint(&t), 100.0);
        }
        hline(&p, &t, rect.min.x, rect.max.x, rect.min.y + header + 30.0);
        for (i, s) in self.suites.iter().enumerate() {
            let y = rect.min.y + header + 30.0 + i as f32 * row_h + row_h / 2.0;
            let running = s.done() < s.total;
            if running {
                spinner(&p, pos2(rect.min.x + pad + 5.0, y), 5.0, t.info, now, 1.5);
            } else {
                state_icon(&p, &t, pos2(rect.min.x + pad + 5.0, y), 6.0, if s.failed > 0 { RunState::Failed } else { RunState::Done }, now);
            }
            let nr = text(&p, pos2(rect.min.x + pad + 18.0, y), Align2::LEFT_CENTER, &s.name, font(12.5), t.foreground, bar_x - rect.min.x - pad - 30.0 - if s.delta < -2.0 { 70.0 } else { 0.0 });
            if s.delta < -2.0 && !running {
                badge(&p, &t, pos2(nr.max.x + 8.0, y), Align2::LEFT_CENTER, "Regressed", t.destructive, false);
            }
            // stacked bar
            let br = Rect::from_min_size(pos2(bar_x, y - 4.0), vec2(bar_w, 8.0));
            p.rect_filled(br, cr(4.0), track(&t));
            let mut x = br.min.x;
            for (k, (n, c)) in [(s.passed, t.success), (s.flaky, t.warning), (s.failed, t.destructive)].iter().enumerate() {
                let f = glowkit_core::anim::follow(ui, id.with(("seg", i, k)), *n as f32 / s.total as f32, 5.0);
                let w = br.width() * f;
                if w > 0.5 {
                    p.rect_filled(Rect::from_min_size(pos2(x, br.min.y), vec2(w, br.height())), cr(4.0), *c);
                    if x > br.min.x {
                        p.line_segment([pos2(x, br.min.y), pos2(x, br.max.y)], Stroke::new(1.5, surface(&t)));
                    }
                }
                x += w;
            }
            if running {
                let u = (now * 0.8).fract();
                let gx = x + (br.max.x - x) * u;
                p.circle_filled(pos2(gx, y), 2.0, with_alpha(t.info, 0.7 * (1.0 - u)));
                glowkit_core::animate(ui);
            }
            text(&p, pos2(bar_x + bar_w + 16.0, y), Align2::LEFT_CENTER, &format!("{}/{}", s.passed, s.total), mono(11.5), t.foreground, 70.0);
            let dc = if s.delta >= 0.0 { t.success } else { t.destructive };
            let label = if running { format!("{} left", s.total - s.done()) } else { format!("{}{:.1}", if s.delta >= 0.0 { "+" } else { "" }, s.delta) };
            text(&p, pos2(rect.max.x - pad, y), Align2::RIGHT_CENTER, &label, mono(11.5), if running { dim(&t) } else { ink(&t, dc) }, 80.0);
            if i + 1 < self.suites.len() {
                hline(&p, &t, rect.min.x + pad, rect.max.x - pad, y + row_h / 2.0);
            }
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// MemoryViewer

/// Kind of an agent memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum MemoryKind {
    /// A user preference.
    #[default]
    Preference,
    /// A fact about the world or the user.
    Fact,
    /// A how-to learned from experience.
    Procedure,
    /// A rule that must be followed.
    Policy,
}

impl MemoryKind {
    fn label(self) -> &'static str {
        match self {
            MemoryKind::Preference => "Preference",
            MemoryKind::Fact => "Fact",
            MemoryKind::Procedure => "Procedure",
            MemoryKind::Policy => "Policy",
        }
    }
    fn color(self, t: &Theme) -> egui::Color32 {
        match self {
            MemoryKind::Preference => t.info,
            MemoryKind::Fact => t.chart[1],
            MemoryKind::Procedure => t.success,
            MemoryKind::Policy => t.chart[2],
        }
    }
}

/// One entry in a [`MemoryViewer`].
#[derive(Clone, Debug, Default)]
pub struct MemoryEntry {
    /// Stable id.
    pub id: u64,
    /// What the agent remembers.
    pub text: String,
    /// Kind.
    pub kind: MemoryKind,
    /// Where it came from, e.g. `"Chat · Refund Resolver"`.
    pub source: String,
    /// Seconds since it was last written or used.
    pub age: f32,
    /// Retrieval confidence 0..1.
    pub confidence: f32,
    /// Pinned memories are always loaded.
    pub pinned: bool,
}

impl MemoryEntry {
    /// An entry `id` saying `text`.
    pub fn new(id: u64, kind: MemoryKind, text: impl Into<String>) -> Self {
        Self { id, kind, text: text.into(), confidence: 0.9, ..Default::default() }
    }
    /// Source, age in seconds and confidence.
    pub fn meta(mut self, source: impl Into<String>, age: f32, confidence: f32) -> Self {
        self.source = source.into();
        self.age = age;
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
    /// Pinned.
    pub fn pinned(mut self, on: bool) -> Self {
        self.pinned = on;
        self
    }
}

/// What an agent remembers. Filter chips by kind sit above rows that show a pin toggle, a kind
/// badge, the memory itself, where it came from, a recency dot that cools from bright to grey as
/// the memory ages, and a confidence bar. Pinned memories float to the top; new memories slide
/// in with a soft highlight; stale ones (old and low confidence) are tagged for review.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{MemoryEntry, MemoryKind, MemoryViewer};
/// let mut mem = vec![
///     MemoryEntry::new(1, MemoryKind::Preference, "Acme prefers email over phone").meta("Chat · Support", 3600.0, 0.96),
///     MemoryEntry::new(2, MemoryKind::Policy, "Refunds above $500 need finance approval").pinned(true),
/// ];
/// ui.add(MemoryViewer::new(&mut mem));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct MemoryViewer<'a> {
    entries: &'a mut Vec<MemoryEntry>,
    width: f32,
    max_rows: usize,
}

impl<'a> MemoryViewer<'a> {
    /// A viewer over `entries` (the pin toggles write `pinned`).
    pub fn new(entries: &'a mut Vec<MemoryEntry>) -> Self {
        Self { entries, width: 760.0, max_rows: 7 }
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(420.0);
        self
    }
    /// Maximum rows shown.
    pub fn max_rows(mut self, n: usize) -> Self {
        self.max_rows = n.max(1);
        self
    }
}

impl egui::Widget for MemoryViewer<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = ui.next_auto_id().with("memory");
        let filter: Option<MemoryKind> = glowkit_core::state(ui, id.with("f"), || None);
        let mut order: Vec<usize> = (0..self.entries.len()).filter(|i| filter.is_none_or(|k| self.entries[*i].kind == k)).collect();
        order.sort_by(|a, b| {
            let (x, y) = (&self.entries[*a], &self.entries[*b]);
            y.pinned.cmp(&x.pinned).then(x.age.total_cmp(&y.age))
        });
        order.truncate(self.max_rows);
        let row_h = 52.0;
        let header = 92.0;
        let target = header + order.len() as f32 * row_h + 6.0;
        let h = glowkit_core::anim::spring(ui, id.with("h"), target, 260.0, 30.0);
        let (rect, resp) = alloc_centered(ui, self.width, h, Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        panel(&p, &t, rect, 14.0);
        let pad = 16.0;
        icon(&p, Icon::Brain, pos2(rect.min.x + pad + 8.0, rect.min.y + 24.0), 16.0, t.foreground);
        text(&p, pos2(rect.min.x + pad + 24.0, rect.min.y + 24.0), Align2::LEFT_CENTER, "Memory", font(14.0), t.foreground, 120.0);
        let pinned = self.entries.iter().filter(|e| e.pinned).count();
        let stale = self.entries.iter().filter(|e| e.age > 86400.0 * 20.0 && e.confidence < 0.8).count();
        text(&p, pos2(rect.max.x - pad, rect.min.y + 24.0), Align2::RIGHT_CENTER, &format!("{} entries · {pinned} pinned · {stale} to review", self.entries.len()), font(11.5), dim(&t), 300.0);
        // chips
        let mut x = rect.min.x + pad;
        let cy = rect.min.y + 62.0;
        let mut new_filter = filter;
        let kinds = [None, Some(MemoryKind::Preference), Some(MemoryKind::Fact), Some(MemoryKind::Procedure), Some(MemoryKind::Policy)];
        for (k, kind) in kinds.iter().enumerate() {
            let label = kind.map(|k| k.label()).unwrap_or("All");
            let n = self.entries.iter().filter(|e| kind.is_none_or(|k| e.kind == k)).count();
            let s = format!("{label}  {n}");
            let w = text_w(&p, &s, font(11.5)) + 20.0;
            let r = Rect::from_min_size(pos2(x, cy - 12.0), vec2(w, 24.0));
            let cr_ = ui.interact(r, id.with(("chip", k)), Sense::click());
            if cr_.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if cr_.clicked() {
                new_filter = *kind;
            }
            let on = filter == *kind;
            let c = kind.map(|k| k.color(&t)).unwrap_or(t.foreground);
            p.rect(r, cr(12.0), if on { tint(&t, c) } else { surface(&t) }, Stroke::new(1.0, if on { with_alpha(c, 0.35) } else { hair(&t) }), StrokeKind::Inside);
            p.text(r.center(), Align2::CENTER_CENTER, s, font(11.5), if on { ink(&t, c) } else { dim(&t) });
            x = r.max.x + 6.0;
        }
        if new_filter != filter {
            glowkit_core::set_state(ui, id.with("f"), new_filter);
        }
        hline(&p, &t, rect.min.x, rect.max.x, rect.min.y + header - 8.0);
        let top = rect.min.y + header - 8.0;
        let pc = p.with_clip_rect(Rect::from_min_max(pos2(rect.min.x, top), rect.max).intersect(p.clip_rect()));
        let mut flip = None;
        for (slot, &i) in order.iter().enumerate() {
            let e = &self.entries[i];
            let rid = id.with(("row", e.id));
            let y = glowkit_core::anim::spring(ui, rid.with("y"), top + slot as f32 * row_h, 220.0, 26.0);
            let appear = glowkit_core::anim::follow(ui, rid.with("ap"), 1.0, 3.0);
            let rr = Rect::from_min_size(pos2(rect.min.x, y), vec2(rect.width(), row_h));
            let pa = faded(&pc, ease(Easing::OutCubic, appear));
            if appear < 0.98 {
                pa.rect_filled(rr, cr(0.0), with_alpha(t.accent, 0.08 * (1.0 - appear)));
            }
            let my = rr.center().y;
            // pin
            let pr = Rect::from_center_size(pos2(rr.min.x + pad + 8.0, my), vec2(22.0, 22.0));
            let presp = ui.interact(pr, rid.with("pin"), Sense::click());
            if presp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                pa.rect_filled(pr, cr(6.0), well(&t));
            }
            if presp.clicked() {
                flip = Some(i);
            }
            let pk = glowkit_core::anim::spring(ui, rid.with("pk"), if e.pinned { 1.0 } else { 0.0 }, 400.0, 18.0);
            let pin_c = mix(faint(&t), t.foreground, pk.clamp(0.0, 1.0));
            icon(&pa, Icon::Bookmark, pr.center(), 13.0 * (1.0 + 0.15 * (pk - pk.clamp(0.0, 1.0)).abs() * 4.0), pin_c);
            if e.pinned {
                pa.rect_filled(Rect::from_center_size(pr.center() - vec2(0.0, 1.5), vec2(5.0, 6.0)), cr(1.0), with_alpha(t.foreground, 0.8));
            }
            let kb = badge(&pa, &t, pos2(rr.min.x + pad + 26.0, my - 9.0), Align2::LEFT_CENTER, e.kind.label(), e.kind.color(&t), false);
            let right_w = 200.0;
            let tw = rr.max.x - right_w - kb.max.x - 18.0;
            text(&pa, pos2(kb.max.x + 8.0, my - 9.0), Align2::LEFT_CENTER, &e.text, font(12.5), t.foreground, tw);
            // recency dot
            let fresh = (1.0 - (e.age / (86400.0 * 14.0)).min(1.0)).powf(0.6);
            let dc = mix(mix(t.muted_foreground, surface(&t), 0.3), t.success, fresh);
            pa.circle_filled(pos2(rr.min.x + pad + 30.0, my + 10.0), 3.0, dc);
            text(&pa, pos2(rr.min.x + pad + 38.0, my + 10.0), Align2::LEFT_CENTER, &format!("{} · {}", e.source, fmt_ago(e.age)), font(11.0), faint(&t), tw + 60.0);
            // confidence
            let bx = rr.max.x - pad - right_w + 30.0;
            let bar = Rect::from_min_size(pos2(bx, my - 2.0), vec2(90.0, 4.0));
            let cc = if e.confidence < 0.8 { t.warning } else { t.success };
            meter(&pa, &t, bar, e.confidence, cc);
            text(&pa, pos2(bar.max.x + 8.0, my), Align2::LEFT_CENTER, &format!("{:.0}%", e.confidence * 100.0), mono(10.5), dim(&t), 40.0);
            if e.age > 86400.0 * 20.0 && e.confidence < 0.8 {
                badge(&pa, &t, pos2(rr.max.x - pad, my), Align2::RIGHT_CENTER, "Stale", t.warning, false);
            }
            if slot + 1 < order.len() {
                hline(&pa, &t, rr.min.x + pad, rr.max.x - pad, rr.max.y);
            }
        }
        if let Some(i) = flip {
            self.entries[i].pinned = !self.entries[i].pinned;
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// ScheduleList

/// A scheduled agent job in a [`ScheduleList`].
#[derive(Clone, Debug, Default)]
pub struct ScheduledJob {
    /// Job name.
    pub name: String,
    /// Agent that runs it.
    pub agent: String,
    /// Human-readable schedule, e.g. `"Weekdays at 08:30"`.
    pub schedule: String,
    /// Seconds until the next run.
    pub next_in: f32,
    /// Seconds between runs (draws the countdown ring).
    pub period: f32,
    /// Outcomes of recent runs (oldest first).
    pub history: Vec<RunState>,
    /// Whether the job is on.
    pub enabled: bool,
    /// Running right now.
    pub running: bool,
}

impl ScheduledJob {
    /// An enabled job.
    pub fn new(name: impl Into<String>, agent: impl Into<String>, schedule: impl Into<String>) -> Self {
        Self { name: name.into(), agent: agent.into(), schedule: schedule.into(), enabled: true, period: 3600.0, ..Default::default() }
    }
    /// Seconds to the next run and the period.
    pub fn next(mut self, next_in: f32, period: f32) -> Self {
        self.next_in = next_in;
        self.period = period.max(1.0);
        self
    }
    /// Recent run outcomes.
    pub fn history(mut self, h: Vec<RunState>) -> Self {
        self.history = h;
        self
    }
}

/// What a [`ScheduleList`] reports.
pub struct ScheduleOutput {
    /// The widget response.
    pub response: Response,
    /// Index of a job whose "Run now" was clicked.
    pub run_now: Option<usize>,
}

/// Scheduled agent jobs. Each row has a countdown ring that empties toward the next run, the job
/// name, its agent and schedule, a ticking "in 2h 14m", pips for the last runs (green, red,
/// amber), Run now and an on / off switch. A job that is running shows a spinner and "Running
/// now"; switched-off jobs dim.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{ScheduleList, ScheduledJob};
/// let mut jobs = vec![ScheduledJob::new("Morning digest", "Digest Writer", "Weekdays at 08:30").next(5400.0, 86400.0)];
/// let out = ScheduleList::new(&mut jobs).show(ui);
/// ```
#[must_use = "call `.show(ui)` or add it with `ui.add(...)`"]
pub struct ScheduleList<'a> {
    jobs: &'a mut Vec<ScheduledJob>,
    width: f32,
}

impl<'a> ScheduleList<'a> {
    /// A list over `jobs` (the switches write `enabled`).
    pub fn new(jobs: &'a mut Vec<ScheduledJob>) -> Self {
        Self { jobs, width: 720.0 }
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(420.0);
        self
    }

    /// Show it and report "Run now" clicks.
    pub fn show(self, ui: &mut Ui) -> ScheduleOutput {
        let t = Theme::of(ui);
        let now = glowkit_core::time(ui);
        let id = ui.next_auto_id().with("schedule");
        let row_h = 62.0;
        let header = 52.0;
        let h = header + self.jobs.len() as f32 * row_h + 4.0;
        let (rect, response) = alloc_centered(ui, self.width, h, Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        panel(&p, &t, rect, 14.0);
        let pad = 16.0;
        icon(&p, Icon::Clock, pos2(rect.min.x + pad + 7.0, rect.min.y + header / 2.0), 15.0, t.foreground);
        text(&p, pos2(rect.min.x + pad + 22.0, rect.min.y + header / 2.0), Align2::LEFT_CENTER, "Scheduled", font(14.0), t.foreground, 160.0);
        let next = self.jobs.iter().filter(|j| j.enabled).map(|j| j.next_in).fold(f32::MAX, f32::min);
        if next < f32::MAX {
            text(&p, pos2(rect.max.x - pad, rect.min.y + header / 2.0), Align2::RIGHT_CENTER, &format!("next run in {}", fmt_secs(next)), font(11.5), dim(&t), 220.0);
        }
        hline(&p, &t, rect.min.x, rect.max.x, rect.min.y + header);
        let mut run_now = None;
        let mut toggles = Vec::new();
        for (i, j) in self.jobs.iter().enumerate() {
            let rr = Rect::from_min_size(pos2(rect.min.x, rect.min.y + header + i as f32 * row_h), vec2(rect.width(), row_h));
            let cy = rr.center().y;
            let on = glowkit_core::anim::follow(ui, id.with(("on", i)), if j.enabled { 1.0 } else { 0.0 }, 8.0);
            let pa = faded(&p, 0.45 + 0.55 * on);
            let rc = pos2(rr.min.x + pad + 16.0, cy);
            if j.running {
                spinner(&pa, rc, 14.0, t.info, now, 2.2);
                icon(&pa, Icon::Play, rc, 11.0, ink(&t, t.info));
            } else {
                pa.circle_stroke(rc, 14.0, Stroke::new(2.4, track(&t)));
                let f = 1.0 - (j.next_in / j.period).clamp(0.0, 1.0);
                if f > 0.0 {
                    glowkit_core::paint::arc(&pa, rc, 14.0, -FRAC_PI_2, -FRAC_PI_2 + TAU * f, 2.4, if j.enabled { t.foreground } else { t.muted_foreground });
                }
                icon(&pa, Icon::Clock, rc, 12.0, dim(&t));
            }
            let tx = rc.x + 26.0;
            text(&pa, pos2(tx, cy - 9.0), Align2::LEFT_CENTER, &j.name, font(13.0), t.foreground, rect.width() * 0.32);
            text(&pa, pos2(tx, cy + 10.0), Align2::LEFT_CENTER, &format!("{} · {}", j.agent, j.schedule), font(11.0), faint(&t), rect.width() * 0.36);
            let nx = rect.min.x + rect.width() * 0.5;
            if j.running {
                text(&pa, pos2(nx, cy - 9.0), Align2::LEFT_CENTER, "Running now", font(12.0), ink(&t, t.info), 120.0);
            } else if j.enabled {
                text(&pa, pos2(nx, cy - 9.0), Align2::LEFT_CENTER, &format!("in {}", fmt_secs(j.next_in)), mono(12.0), t.foreground, 120.0);
            } else {
                text(&pa, pos2(nx, cy - 9.0), Align2::LEFT_CENTER, "Paused", font(12.0), dim(&t), 120.0);
            }
            for (k, s) in j.history.iter().rev().take(8).collect::<Vec<_>>().into_iter().rev().enumerate() {
                let r = Rect::from_min_size(pos2(nx + k as f32 * 9.0, cy + 6.0), vec2(6.0, 10.0));
                pa.rect_filled(r, cr(2.0), s.color(&t));
            }
            let sw = Rect::from_center_size(pos2(rr.max.x - pad - 18.0, cy), vec2(36.0, 20.0));
            let mut en = j.enabled;
            if switch(ui, sw, id.with(("sw", i)), &mut en, t.success).clicked() {
                toggles.push(i);
            }
            let rb = Rect::from_min_size(pos2(sw.min.x - 14.0 - 84.0, cy - 14.0), vec2(84.0, 28.0));
            if !j.running && button(ui, rb, id.with(("run", i)), "Run now", Some(Icon::Play), Btn::Secondary).clicked() {
                run_now = Some(i);
            }
            if i + 1 < self.jobs.len() {
                hline(&p, &t, rr.min.x + pad, rr.max.x - pad, rr.max.y);
            }
        }
        for i in toggles {
            self.jobs[i].enabled = !self.jobs[i].enabled;
        }
        if self.jobs.iter().any(|j| j.running) {
            glowkit_core::animate(ui);
        }
        ScheduleOutput { response, run_now }
    }
}

impl egui::Widget for ScheduleList<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}
