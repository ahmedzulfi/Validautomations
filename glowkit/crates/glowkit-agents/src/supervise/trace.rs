//! Where time goes: [`TraceWaterfall`] (spans on a timeline, like a tracing UI) and
//! [`RunTimeline`] (an agent's steps on a rail with their tool calls).

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui};
use glowkit_ai::Icon;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// What a [`TraceSpan`] measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum SpanKind {
    /// An agent or sub-agent (a parent span).
    #[default]
    Agent,
    /// A model call.
    Llm,
    /// A tool call.
    Tool,
    /// A retrieval / vector search.
    Retrieval,
}

impl SpanKind {
    fn color(self, t: &Theme) -> Color32 {
        match self {
            SpanKind::Agent => mix(t.muted_foreground, t.foreground, 0.1),
            SpanKind::Llm => t.accent,
            SpanKind::Tool => t.chart[1],
            SpanKind::Retrieval => t.chart[4],
        }
    }
    fn icon(self) -> Icon {
        match self {
            SpanKind::Agent => Icon::Bot,
            SpanKind::Llm => Icon::Sparkle,
            SpanKind::Tool => Icon::Gear,
            SpanKind::Retrieval => Icon::Search,
        }
    }
    fn label(self) -> &'static str {
        match self {
            SpanKind::Agent => "Agent",
            SpanKind::Llm => "LLM call",
            SpanKind::Tool => "Tool call",
            SpanKind::Retrieval => "Retrieval",
        }
    }
}

/// One span in a [`TraceWaterfall`]. Spans are listed in tree order; `depth` nests a span under
/// the closest previous span with a smaller depth.
#[derive(Clone, Debug, Default)]
pub struct TraceSpan {
    /// Span name, e.g. `"web_search"`.
    pub name: String,
    /// Secondary text, e.g. the model or the query.
    pub detail: String,
    /// What it measures.
    pub kind: SpanKind,
    /// Nesting depth (0 = root).
    pub depth: usize,
    /// Start time in seconds from the start of the trace.
    pub start: f32,
    /// Duration in seconds; `None` while it is still running.
    pub dur: Option<f32>,
    /// Tokens used.
    pub tokens: f32,
    /// Cost in dollars.
    pub cost: f32,
    /// Whether it failed.
    pub error: bool,
}

impl TraceSpan {
    /// A span called `name` of `kind` at `depth`, starting at `start` seconds.
    pub fn new(name: impl Into<String>, kind: SpanKind, depth: usize, start: f32) -> Self {
        Self { name: name.into(), kind, depth, start, ..Default::default() }
    }
    /// Secondary text.
    pub fn detail(mut self, s: impl Into<String>) -> Self {
        self.detail = s.into();
        self
    }
    /// Finished duration in seconds.
    pub fn dur(mut self, d: f32) -> Self {
        self.dur = Some(d);
        self
    }
    /// Tokens and cost.
    pub fn usage(mut self, tokens: f32, cost: f32) -> Self {
        self.tokens = tokens;
        self.cost = cost;
        self
    }
    /// Mark it as failed.
    pub fn error(mut self, e: bool) -> Self {
        self.error = e;
        self
    }
    fn end(&self, now: f32) -> f32 {
        self.dur.map(|d| self.start + d).unwrap_or(now.max(self.start))
    }
}

/// A trace of one agent run as a waterfall, the way tracing tools show it: a summary strip
/// (duration, cost, tokens, LLM and tool calls, errors), a span tree on the left whose parents
/// fold, and a bar per span on a time axis colored by kind (agent, LLM, tool, retrieval) with its
/// duration and cost. The slowest leaf is tagged, running spans grow toward a live "now" line,
/// and hovering a row highlights it and opens a detail card.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{SpanKind, TraceSpan, TraceWaterfall};
/// let spans = vec![
///     TraceSpan::new("Orchestrator", SpanKind::Agent, 0, 0.0).dur(9.0),
///     TraceSpan::new("Plan", SpanKind::Llm, 1, 0.2).dur(3.1).usage(1800.0, 0.04),
///     TraceSpan::new("web_search", SpanKind::Tool, 1, 3.4).dur(2.2),
/// ];
/// ui.add(TraceWaterfall::new(&spans).title("Launch the pricing page").now(9.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TraceWaterfall<'a> {
    spans: &'a [TraceSpan],
    title: String,
    now: Option<f32>,
    width: f32,
    row_h: f32,
}

impl<'a> TraceWaterfall<'a> {
    /// A waterfall over `spans`.
    pub fn new(spans: &'a [TraceSpan]) -> Self {
        Self { spans, title: String::new(), now: None, width: 880.0, row_h: 30.0 }
    }
    /// Run title shown in the header.
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Current trace time in seconds: running spans end here and a live line is drawn.
    pub fn now(mut self, t: f32) -> Self {
        self.now = Some(t);
        self
    }
    /// Maximum width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(420.0);
        self
    }
    /// Row height in points.
    pub fn row_height(mut self, h: f32) -> Self {
        self.row_h = h.clamp(22.0, 44.0);
        self
    }
}

fn nice_step(span: f32) -> f32 {
    for s in [0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0] {
        if span / s <= 6.0 {
            return s;
        }
    }
    600.0
}

impl egui::Widget for TraceWaterfall<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = ui.next_auto_id().with("trace_wf");
        let spans = self.spans;
        let live = self.now.is_some() && spans.iter().any(|s| s.dur.is_none());
        let now = self.now.unwrap_or_else(|| spans.iter().map(|s| s.end(0.0)).fold(0.0, f32::max));
        let collapsed: Vec<usize> = glowkit_core::state(ui, id.with("fold"), Vec::new);
        // visibility
        let mut visible = Vec::new();
        let mut hide_below: Option<usize> = None;
        for (i, s) in spans.iter().enumerate() {
            if let Some(d) = hide_below {
                if s.depth > d {
                    continue;
                }
                hide_below = None;
            }
            visible.push(i);
            if collapsed.contains(&i) {
                hide_below = Some(s.depth);
            }
        }
        let has_kids = |i: usize| spans.get(i + 1).is_some_and(|n| n.depth > spans[i].depth);
        let rh = self.row_h;
        let header = 44.0;
        let stats = 58.0;
        let axis = 30.0;
        let body = visible.len() as f32 * rh;
        let h = header + stats + axis + body + 10.0;
        let (rect, resp) = alloc_centered(ui, self.width, h, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let p = ui.painter_at(rect.expand(14.0));
        panel(&p, &t, rect, 14.0);
        let pad = 16.0;
        // header
        let hy = rect.min.y + header / 2.0;
        icon(&p, Icon::Sparkle, pos2(rect.min.x + pad + 6.0, hy), 13.0, t.accent);
        let tr = text(&p, pos2(rect.min.x + pad + 20.0, hy), Align2::LEFT_CENTER, "Trace", font(13.5), t.foreground, 80.0);
        text(&p, pos2(tr.max.x + 8.0, hy), Align2::LEFT_CENTER, &self.title, font(12.5), dim(&t), rect.width() - 220.0);
        if live {
            state_badge(&p, &t, pos2(rect.max.x - pad, hy), Align2::RIGHT_CENTER, RunState::Running, now);
        } else {
            badge(&p, &t, pos2(rect.max.x - pad, hy), Align2::RIGHT_CENTER, "Complete", t.success, true);
        }
        hline(&p, &t, rect.min.x, rect.max.x, rect.min.y + header);
        // stats
        let total = spans.iter().map(|s| s.end(now)).fold(0.0, f32::max);
        let cost: f32 = spans.iter().map(|s| s.cost).sum();
        let tokens: f32 = spans.iter().map(|s| s.tokens).sum();
        let llm = spans.iter().filter(|s| s.kind == SpanKind::Llm).count();
        let tools = spans.iter().filter(|s| matches!(s.kind, SpanKind::Tool | SpanKind::Retrieval)).count();
        let errs = spans.iter().filter(|s| s.error).count();
        let cells = [
            ("DURATION", fmt_secs(total)),
            ("COST", fmt_usd(cost)),
            ("TOKENS", fmt_tok(tokens)),
            ("LLM CALLS", llm.to_string()),
            ("TOOL CALLS", tools.to_string()),
            ("ERRORS", errs.to_string()),
        ];
        let sy = rect.min.y + header;
        let cw = rect.width() / cells.len() as f32;
        for (i, (k, v)) in cells.iter().enumerate() {
            let x = rect.min.x + cw * i as f32;
            if i > 0 {
                p.line_segment([pos2(x, sy + 12.0), pos2(x, sy + stats - 12.0)], Stroke::new(1.0, hair(&t)));
            }
            text(&p, pos2(x + pad, sy + 18.0), Align2::LEFT_CENTER, k, font(9.5), faint(&t), cw);
            let c = if *k == "ERRORS" && errs > 0 { ink(&t, t.destructive) } else { t.foreground };
            text(&p, pos2(x + pad, sy + 38.0), Align2::LEFT_CENTER, v, mono(13.0), c, cw);
        }
        hline(&p, &t, rect.min.x, rect.max.x, sy + stats);
        // axis
        let split = rect.min.x + (rect.width() * 0.34).clamp(200.0, 300.0);
        let tl = Rect::from_min_max(pos2(split + 10.0, sy + stats), pos2(rect.max.x - 96.0, rect.max.y - 10.0));
        let span_t = (total * 1.04).max(1.0);
        let step = nice_step(span_t);
        let x_of = |s: f32| tl.min.x + s / span_t * tl.width();
        let ay = sy + stats + axis / 2.0;
        text(&p, pos2(rect.min.x + pad, ay), Align2::LEFT_CENTER, "SPAN", font(9.5), faint(&t), 100.0);
        let mut k = 0.0;
        while k <= span_t {
            let x = x_of(k);
            p.line_segment([pos2(x, sy + stats + 4.0), pos2(x, rect.max.y - 10.0)], Stroke::new(1.0, with_alpha(hair(&t), 0.7)));
            text(&p, pos2(x + 4.0, ay), Align2::LEFT_CENTER, &format!("{}s", k), font(10.5), faint(&t), 40.0);
            k += step;
        }
        p.line_segment([pos2(split, sy + stats), pos2(split, rect.max.y)], Stroke::new(1.0, hair(&t)));
        hline(&p, &t, rect.min.x, rect.max.x, sy + stats + axis);
        // slowest leaf
        let slowest = (0..spans.len())
            .filter(|i| !has_kids(*i))
            .max_by(|a, b| (spans[*a].end(now) - spans[*a].start).total_cmp(&(spans[*b].end(now) - spans[*b].start)));
        // rows
        let top = sy + stats + axis;
        let mut hovered: Option<(usize, Rect)> = None;
        let mut toggle: Option<usize> = None;
        for (row, &i) in visible.iter().enumerate() {
            let s = &spans[i];
            let y = top + row as f32 * rh;
            let rr = Rect::from_min_max(pos2(rect.min.x + 1.0, y), pos2(rect.max.x - 1.0, y + rh));
            let rresp = ui.interact(rr, id.with(("row", i)), Sense::click());
            if rresp.hovered() {
                hovered = Some((i, rr));
                p.rect_filled(rr, cr(0.0), with_alpha(t.foreground, if t.dark { 0.04 } else { 0.025 }));
            }
            let cy = rr.center().y;
            let x = rect.min.x + pad + s.depth as f32 * 16.0;
            if has_kids(i) {
                let ic = if collapsed.contains(&i) { Icon::ChevronRight } else { Icon::ChevronDown };
                icon(&p, ic, pos2(x + 5.0, cy), 11.0, dim(&t));
                if rresp.clicked() {
                    toggle = Some(i);
                }
                if rresp.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
            }
            let kc = if s.error { t.destructive } else { s.kind.color(&t) };
            let ib = Rect::from_center_size(pos2(x + 22.0, cy), vec2(18.0, 18.0));
            p.rect_filled(ib, cr(5.0), tint(&t, kc));
            icon(&p, s.kind.icon(), ib.center(), 11.0, ink(&t, kc));
            let name_font = if s.depth == 0 || s.kind == SpanKind::Agent { font(12.5) } else { font(12.0) };
            let nr = text(&p, pos2(x + 36.0, cy), Align2::LEFT_CENTER, &s.name, name_font, t.foreground, split - x - 46.0);
            let room = split - nr.max.x - 14.0;
            if room > 24.0 {
                text(&p, pos2(nr.max.x + 6.0, cy), Align2::LEFT_CENTER, &s.detail, font(11.0), faint(&t), room);
            }
            // bar
            let e = s.end(now);
            let x0 = x_of(s.start);
            let x1 = x_of(e).max(x0 + 3.0);
            let bar = Rect::from_min_max(pos2(x0, cy - 6.0), pos2(x1, cy + 6.0));
            let parent = has_kids(i);
            let base = if parent { mix(track(&t), kc, 0.15) } else { mix(surface(&t), kc, if t.dark { 0.35 } else { 0.22 }) };
            p.rect_filled(bar, cr(4.0), base);
            if !parent {
                p.rect_stroke(bar, cr(4.0), Stroke::new(1.0, with_alpha(kc, 0.45)), StrokeKind::Inside);
            }
            if s.dur.is_none() {
                // running: shimmering leading edge
                let w = (bar.width()).min(40.0);
                let er = Rect::from_min_max(pos2(bar.max.x - w, bar.min.y), bar.max);
                paint::gradient_rect(&p, er, 4.0, 0.0, &[with_alpha(kc, 0.0), with_alpha(kc, 0.55)]);
                spinner(&p, pos2(bar.max.x + 9.0, cy), 4.5, kc, now, 1.5);
            }
            let lx = if s.dur.is_none() { bar.max.x + 20.0 } else { bar.max.x + 6.0 };
            let label = if s.cost > 0.0 { format!("{} · {}", fmt_secs(e - s.start), fmt_usd(s.cost)) } else { fmt_secs(e - s.start) };
            let lr = text(&p, pos2(lx, cy), Align2::LEFT_CENTER, &label, font(10.5), dim(&t), rect.max.x - lx - 8.0);
            if slowest == Some(i) && s.dur.is_some() && lr.max.x + 60.0 < rect.max.x {
                badge(&p, &t, pos2(lr.max.x + 6.0, cy), Align2::LEFT_CENTER, "SLOWEST", t.warning, false);
            }
            if s.error {
                badge(&p, &t, pos2(split - 8.0, cy), Align2::RIGHT_CENTER, "error", t.destructive, false);
            }
        }
        if live {
            let x = x_of(now);
            p.line_segment([pos2(x, sy + stats + 4.0), pos2(x, rect.max.y - 10.0)], Stroke::new(1.2, with_alpha(t.info, 0.8)));
            p.circle_filled(pos2(x, sy + stats + axis), 3.0, t.info);
            glowkit_core::animate(ui);
        }
        // hover card
        if let Some((i, rr)) = hovered {
            let s = &spans[i];
            let w = 220.0;
            let e = s.end(now);
            let lines = [
                ("Kind", s.kind.label().to_string()),
                ("Start", format!("+{}", fmt_secs(s.start))),
                ("Duration", fmt_secs(e - s.start)),
                ("Tokens", if s.tokens > 0.0 { fmt_tok(s.tokens) } else { "—".into() }),
                ("Cost", if s.cost > 0.0 { fmt_usd(s.cost) } else { "—".into() }),
            ];
            let ch = 34.0 + lines.len() as f32 * 19.0;
            let mut cx = x_of(s.start).min(rect.max.x - w - 8.0);
            cx = cx.max(split + 8.0);
            let mut cy = rr.max.y + 4.0;
            if cy + ch > rect.max.y + 40.0 {
                cy = rr.min.y - ch - 4.0;
            }
            let card = Rect::from_min_size(pos2(cx, cy), vec2(w, ch));
            let lp = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, id.with("tip")));
            panel_fill(&lp, &t, card, 10.0, if t.dark { t.popover } else { t.card }, 1.4);
            text(&lp, pos2(card.min.x + 12.0, card.min.y + 16.0), Align2::LEFT_CENTER, &s.name, font(12.5), t.foreground, w - 24.0);
            for (k, (a, b)) in lines.iter().enumerate() {
                let y = card.min.y + 38.0 + k as f32 * 19.0;
                lp.text(pos2(card.min.x + 12.0, y), Align2::LEFT_CENTER, *a, font(11.0), dim(&t));
                lp.text(pos2(card.max.x - 12.0, y), Align2::RIGHT_CENTER, b, mono(11.0), t.foreground);
            }
        }
        if let Some(i) = toggle {
            glowkit_core::with_state(ui, id.with("fold"), Vec::<usize>::new, |v| {
                if let Some(k) = v.iter().position(|x| *x == i) {
                    v.remove(k);
                } else {
                    v.push(i);
                }
            });
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// RunTimeline

/// A tool call inside a [`TimelineStep`].
#[derive(Clone, Debug)]
pub struct StepTool {
    /// Tool name, e.g. `"read_logs"`.
    pub name: String,
    /// Icon shown before the name.
    pub icon: Icon,
    /// State of the call.
    pub state: RunState,
    /// Seconds it took (or has taken so far).
    pub secs: f32,
    /// One line of detail (arguments, result or error).
    pub detail: String,
}

impl StepTool {
    /// A tool call `name` in `state`.
    pub fn new(name: impl Into<String>, icon: Icon, state: RunState) -> Self {
        Self { name: name.into(), icon, state, secs: 0.0, detail: String::new() }
    }
    /// Seconds taken.
    pub fn secs(mut self, s: f32) -> Self {
        self.secs = s;
        self
    }
    /// One line of detail.
    pub fn detail(mut self, s: impl Into<String>) -> Self {
        self.detail = s.into();
        self
    }
}

/// One step of a [`RunTimeline`].
#[derive(Clone, Debug)]
pub struct TimelineStep {
    /// Step title.
    pub title: String,
    /// State of the step.
    pub state: RunState,
    /// Seconds taken.
    pub secs: f32,
    /// Tool calls made in this step.
    pub tools: Vec<StepTool>,
    /// Error or note shown in a box under a failed / waiting step.
    pub note: String,
}

impl TimelineStep {
    /// A step called `title` in `state`.
    pub fn new(title: impl Into<String>, state: RunState) -> Self {
        Self { title: title.into(), state, secs: 0.0, tools: Vec::new(), note: String::new() }
    }
    /// Seconds taken.
    pub fn secs(mut self, s: f32) -> Self {
        self.secs = s;
        self
    }
    /// Add a tool call.
    pub fn tool(mut self, t: StepTool) -> Self {
        self.tools.push(t);
        self
    }
    /// Error or note for a failed / waiting step.
    pub fn note(mut self, s: impl Into<String>) -> Self {
        self.note = s.into();
        self
    }
}

/// What happened in a [`RunTimeline`] this frame.
pub struct RunTimelineOutput {
    /// The widget response.
    pub response: Response,
    /// Index of the step whose "Retry step" button was clicked.
    pub retry: Option<usize>,
}

/// An agent run as a quiet vertical trail: a header with the task, "3 / 6" and a progress bar,
/// then every step on a rail with its state mark and time. The current step opens to show its
/// tool calls with live spinners, finished steps fold to "2 tool calls" (click to open), a failed
/// step shows its error in a red box with a Retry button, and a soft light runs down the rail
/// into the step being worked on.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{RunState, RunTimeline, StepTool, TimelineStep};
/// use glowkit_ai::Icon;
/// let steps = vec![
///     TimelineStep::new("Investigate the failed deploy", RunState::Done).secs(9.0)
///         .tool(StepTool::new("read_logs", Icon::Terminal, RunState::Done).secs(1.2)),
///     TimelineStep::new("Install the missing dependency", RunState::Running)
///         .tool(StepTool::new("shell", Icon::Terminal, RunState::Running).detail("pnpm add zod")),
/// ];
/// let out = RunTimeline::new("Fix the failing deploy", &steps).show(ui);
/// ```
#[must_use = "call `.show(ui)` or add it with `ui.add(...)`"]
pub struct RunTimeline<'a> {
    task: String,
    steps: &'a [TimelineStep],
    width: f32,
    now: Option<f32>,
}

impl<'a> RunTimeline<'a> {
    /// A timeline for `task` over `steps`.
    pub fn new(task: impl Into<String>, steps: &'a [TimelineStep]) -> Self {
        Self { task: task.into(), steps, width: 520.0, now: None }
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(300.0);
        self
    }
    /// Time used for animation (default: the egui clock).
    pub fn time(mut self, t: f32) -> Self {
        self.now = Some(t);
        self
    }

    /// Show it and report retry clicks.
    pub fn show(self, ui: &mut Ui) -> RunTimelineOutput {
        let t = Theme::of(ui);
        let now = self.now.unwrap_or_else(|| glowkit_core::time(ui));
        let id = ui.next_auto_id().with("run_timeline");
        let toggled: Vec<usize> = glowkit_core::state(ui, id.with("open"), Vec::new);
        let steps = self.steps;
        // open amount per step
        let open: Vec<f32> = steps
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let auto = matches!(s.state, RunState::Running | RunState::Failed | RunState::NeedsInput);
                let o = auto ^ toggled.contains(&i);
                glowkit_core::anim::spring(ui, id.with(("o", i)), if o { 1.0 } else { 0.0 }, 300.0, 28.0)
            })
            .collect();
        let tool_h = 30.0;
        let step_h = |i: usize| -> f32 {
            let s = &steps[i];
            let mut h = 30.0;
            if !s.tools.is_empty() {
                let inner = 8.0 + s.tools.len() as f32 * (tool_h + 4.0);
                h += 34.0 * (1.0 - open[i]) + (inner + 6.0) * open[i];
            }
            if !s.note.is_empty() && matches!(s.state, RunState::Failed | RunState::NeedsInput) {
                h += 62.0;
            }
            h + 8.0
        };
        let header = 70.0;
        let body: f32 = (0..steps.len()).map(step_h).sum();
        let h = header + body + 14.0;
        let (rect, response) = alloc_centered(ui, self.width, h, Sense::hover());
        let mut retry = None;
        if !ui.is_rect_visible(rect) {
            return RunTimelineOutput { response, retry };
        }
        let p = ui.painter_at(rect.expand(14.0));
        panel(&p, &t, rect, 14.0);
        let pad = 16.0;
        let done = steps.iter().filter(|s| s.state == RunState::Done).count();
        let cur = steps.iter().position(|s| s.state != RunState::Done).unwrap_or(steps.len());
        text(&p, pos2(rect.min.x + pad, rect.min.y + 22.0), Align2::LEFT_CENTER, &self.task, font(14.0), t.foreground, rect.width() - 120.0);
        text(
            &p,
            pos2(rect.max.x - pad, rect.min.y + 22.0),
            Align2::RIGHT_CENTER,
            &format!("{done} / {}", steps.len()),
            mono(12.0),
            dim(&t),
            80.0,
        );
        let frac = glowkit_core::anim::follow(ui, id.with("frac"), done as f32 / steps.len().max(1) as f32, 6.0);
        let any_fail = steps.iter().any(|s| s.state == RunState::Failed);
        let bar_c = if any_fail { t.destructive } else if done == steps.len() { t.success } else { t.foreground };
        meter(&p, &t, Rect::from_min_size(pos2(rect.min.x + pad, rect.min.y + 42.0), vec2(rect.width() - pad * 2.0, 4.0)), frac, bar_c);
        hline(&p, &t, rect.min.x, rect.max.x, rect.min.y + header - 10.0);
        let rail_x = rect.min.x + pad + 9.0;
        let mut y = rect.min.y + header;
        let mut centers = Vec::new();
        for (i, s) in steps.iter().enumerate() {
            let sh = step_h(i);
            let cy = y + 14.0;
            centers.push(cy);
            // rail segment to next
            if i + 1 < steps.len() {
                let c = if s.state == RunState::Done { mix(hair(&t), t.foreground, 0.35) } else { hair(&t) };
                p.line_segment([pos2(rail_x, cy + 11.0), pos2(rail_x, y + sh + 3.0)], Stroke::new(1.5, c));
            }
            state_icon(&p, &t, pos2(rail_x, cy), 8.5, s.state, now);
            let tx = rail_x + 20.0;
            let title_c = if s.state == RunState::Queued { dim(&t) } else { t.foreground };
            let tr = text(&p, pos2(tx, cy), Align2::LEFT_CENTER, &s.title, font(13.0), title_c, rect.max.x - tx - 110.0);
            if s.state != RunState::Queued {
                text(&p, pos2(tr.max.x + 8.0, cy), Align2::LEFT_CENTER, &fmt_secs(s.secs), font(11.0), faint(&t), 60.0);
            }
            if matches!(s.state, RunState::Running | RunState::NeedsInput | RunState::Failed) {
                state_badge(&p, &t, pos2(rect.max.x - pad, cy), Align2::RIGHT_CENTER, s.state, now);
            }
            let mut yy = y + 32.0;
            if !s.tools.is_empty() {
                let o = open[i];
                let box_w = rect.max.x - pad - tx;
                let inner = 8.0 + s.tools.len() as f32 * (tool_h + 4.0);
                let box_h = 30.0 + (inner - 30.0 + 6.0) * o;
                let br = Rect::from_min_size(pos2(tx, yy), vec2(box_w, box_h.max(28.0)));
                let hr = Rect::from_min_size(br.min, vec2(br.width(), 28.0));
                let hresp = ui.interact(hr, id.with(("hdr", i)), Sense::click());
                if hresp.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                if hresp.clicked() {
                    glowkit_core::with_state(ui, id.with("open"), Vec::<usize>::new, |v| {
                        if let Some(k) = v.iter().position(|x| *x == i) {
                            v.remove(k);
                        } else {
                            v.push(i);
                        }
                    });
                }
                p.rect(br, cr(9.0), if hresp.hovered() { mix(well(&t), t.foreground, 0.02) } else { well(&t) }, Stroke::new(1.0, hair(&t)), StrokeKind::Inside);
                let pc = faded(&p, 1.0 - o);
                let n = s.tools.len();
                pc.text(pos2(br.min.x + 11.0, br.min.y + 14.0), Align2::LEFT_CENTER, format!("{n} tool call{}", if n == 1 { "" } else { "s" }), font(11.5), dim(&t));
                let ic = if o > 0.5 { Icon::ChevronDown } else { Icon::ChevronRight };
                icon(&p, ic, pos2(br.max.x - 14.0, br.min.y + 14.0), 11.0, dim(&t));
                if o > 0.02 {
                    let po = faded(&p.with_clip_rect(br), o);
                    for (k, tool) in s.tools.iter().enumerate() {
                        let ty = br.min.y + 6.0 + k as f32 * (tool_h + 4.0) + (1.0 - o) * 6.0;
                        let trr = Rect::from_min_size(pos2(br.min.x + 6.0, ty), vec2(br.width() - 34.0, tool_h));
                        po.rect(trr, cr(7.0), surface(&t), Stroke::new(1.0, hair(&t)), StrokeKind::Inside);
                        icon(&po, tool.icon, pos2(trr.min.x + 13.0, trr.center().y), 12.0, dim(&t));
                        let nr = text(&po, pos2(trr.min.x + 26.0, trr.center().y), Align2::LEFT_CENTER, &tool.name, mono(11.0), t.foreground, 150.0);
                        let sr = if tool.state == RunState::Done {
                            let label = format!("{}", fmt_secs(tool.secs));
                            let r = text(&po, pos2(trr.max.x - 10.0, trr.center().y), Align2::RIGHT_CENTER, &label, font(10.5), faint(&t), 60.0);
                            icon(&po, Icon::Check, pos2(r.min.x - 9.0, trr.center().y), 11.0, t.success);
                            r.min.x - 18.0
                        } else if tool.state == RunState::Running {
                            spinner(&po, pos2(trr.max.x - 14.0, trr.center().y), 5.0, t.info, now, 1.6);
                            trr.max.x - 26.0
                        } else {
                            badge(&po, &t, pos2(trr.max.x - 8.0, trr.center().y), Align2::RIGHT_CENTER, tool.state.label(), tool.state.color(&t), false).min.x - 6.0
                        };
                        text(&po, pos2(nr.max.x + 10.0, trr.center().y), Align2::LEFT_CENTER, &tool.detail, font(11.0), faint(&t), sr - nr.max.x - 16.0);
                    }
                }
                yy += br.height() + 6.0;
            }
            if !s.note.is_empty() && matches!(s.state, RunState::Failed | RunState::NeedsInput) {
                let c = s.state.color(&t);
                let nr = Rect::from_min_size(pos2(tx, yy), vec2(rect.max.x - pad - tx, 54.0));
                p.rect(nr, cr(9.0), tint(&t, c), Stroke::new(1.0, with_alpha(c, 0.35)), StrokeKind::Inside);
                icon(&p, Icon::AlertCircle, pos2(nr.min.x + 15.0, nr.min.y + 16.0), 13.0, ink(&t, c));
                let bw = 92.0;
                para(&p, pos2(nr.min.x + 28.0, nr.min.y + 8.0), &s.note, font(11.5), ink(&t, c), nr.width() - 44.0 - bw, 2);
                if s.state == RunState::Failed {
                    let b = Rect::from_min_size(pos2(nr.max.x - bw - 10.0, nr.center().y - 13.0), vec2(bw, 26.0));
                    if button(ui, b, id.with(("retry", i)), "Retry step", Some(Icon::Refresh), Btn::Primary).clicked() {
                        retry = Some(i);
                    }
                }
            }
            y += sh;
        }
        // light running down the rail into the current step
        if cur < steps.len() && cur > 0 && steps[cur].state == RunState::Running {
            let y0 = centers[0];
            let y1 = centers[cur];
            let u = (now * 0.6).fract();
            let head = y0 + (y1 - y0) * u;
            let pts = vec![pos2(rail_x, (head - 40.0).max(y0)), pos2(rail_x, head)];
            paint::gradient_polyline(&p, &pts, false, 2.0, |k| with_alpha(t.info, k * 0.9));
        }
        if steps.iter().any(|s| s.state.is_active()) {
            glowkit_core::animate(ui);
        }
        let _ = Id::NULL;
        RunTimelineOutput { response, retry }
    }
}

impl egui::Widget for RunTimeline<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}
