//! Watching many runs at once: [`RunMonitor`] (live runs table), [`CostMeter`] (spend against a
//! budget, by model) and [`AgentFleetGrid`] (a tile per agent with a status ring).

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, Rect, Response, Sense, Stroke, StrokeKind, Ui};
use glowkit_ai::Icon;
use glowkit_core::anim::{ease, Easing};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::{FRAC_PI_2, TAU};

/// One run in a [`RunMonitor`].
#[derive(Clone, Debug, Default)]
pub struct RunInfo {
    /// Run id, e.g. `"RUN-4822"`.
    pub id: String,
    /// Agent name.
    pub agent: String,
    /// Run state.
    pub state: RunState,
    /// Current step (1-based).
    pub step: usize,
    /// Total steps.
    pub steps: usize,
    /// What the current step does.
    pub step_label: String,
    /// Seconds since start.
    pub elapsed: f32,
    /// Tokens used.
    pub tokens: f32,
    /// Spend in dollars.
    pub cost: f32,
    /// Recent spend samples for the sparkline (oldest first).
    pub cost_history: Vec<f32>,
}

impl RunInfo {
    /// A run `id` of `agent` in `state`.
    pub fn new(id: impl Into<String>, agent: impl Into<String>, state: RunState) -> Self {
        Self { id: id.into(), agent: agent.into(), state, ..Default::default() }
    }
    /// Current step, total steps and what it does.
    pub fn step(mut self, i: usize, of: usize, label: impl Into<String>) -> Self {
        self.step = i;
        self.steps = of;
        self.step_label = label.into();
        self
    }
    /// Elapsed seconds, tokens and dollars.
    pub fn usage(mut self, elapsed: f32, tokens: f32, cost: f32) -> Self {
        self.elapsed = elapsed;
        self.tokens = tokens;
        self.cost = cost;
        self
    }
    /// Recent spend samples for the sparkline.
    pub fn history(mut self, h: Vec<f32>) -> Self {
        self.cost_history = h;
        self
    }
}

fn priority(s: RunState) -> u8 {
    match s {
        RunState::NeedsInput => 0,
        RunState::Failed => 1,
        RunState::Running => 2,
        RunState::Queued => 3,
        RunState::Paused => 4,
        RunState::Idle => 5,
        RunState::Done => 6,
    }
}

/// A live table of agent runs. Filter tabs (All, Running, Needs you, Failed) carry rolling
/// counts; each row shows the agent and run id, a status badge, the step it is on with a thin
/// progress bar, a ticking elapsed clock, tokens, and its cost with a sparkline. Rows are sorted
/// by urgency and glide to their new place when a run changes state.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{RunInfo, RunMonitor, RunState};
/// let runs = vec![
///     RunInfo::new("RUN-4822", "Refund Resolver", RunState::Running).step(3, 6, "Capture refund").usage(84.0, 12_400.0, 0.21),
///     RunInfo::new("RUN-4817", "Lead Enricher", RunState::NeedsInput).step(2, 4, "Approve CRM write").usage(310.0, 8_100.0, 0.09),
/// ];
/// ui.add(RunMonitor::new(&runs));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct RunMonitor<'a> {
    runs: &'a [RunInfo],
    width: f32,
    title: String,
    max_rows: usize,
}

impl<'a> RunMonitor<'a> {
    /// A monitor over `runs`.
    pub fn new(runs: &'a [RunInfo]) -> Self {
        Self { runs, width: 860.0, title: "Runs".into(), max_rows: 8 }
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(480.0);
        self
    }
    /// Header title.
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Maximum rows shown.
    pub fn max_rows(mut self, n: usize) -> Self {
        self.max_rows = n.max(1);
        self
    }
}

impl egui::Widget for RunMonitor<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let now = glowkit_core::time(ui);
        let id = ui.next_auto_id().with("run_monitor");
        let tab: usize = glowkit_core::state(ui, id.with("tab"), || 0);
        let filt = |s: RunState, tab: usize| match tab {
            1 => s == RunState::Running,
            2 => s == RunState::NeedsInput,
            3 => s == RunState::Failed,
            _ => true,
        };
        let mut order: Vec<usize> = (0..self.runs.len()).filter(|i| filt(self.runs[*i].state, tab)).collect();
        order.sort_by_key(|i| (priority(self.runs[*i].state), self.runs[*i].id.clone()));
        order.truncate(self.max_rows);
        let row_h = 52.0;
        let header = 56.0;
        let colh = 30.0;
        let target = header + colh + order.len() as f32 * row_h + 8.0;
        let h = glowkit_core::anim::spring(ui, id.with("h"), target, 260.0, 30.0);
        let (rect, resp) = alloc_centered(ui, self.width, h, Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        panel(&p, &t, rect, 14.0);
        let pad = 16.0;
        let hy = rect.min.y + header / 2.0;
        let tr =
            text(&p, pos2(rect.min.x + pad, hy), Align2::LEFT_CENTER, &self.title, font(14.0), t.foreground, 120.0);
        let live = self.runs.iter().any(|r| r.state.is_active());
        if live {
            let c = pos2(tr.max.x + 12.0, hy);
            let k = (now * 1.2).fract();
            p.circle_filled(c, 3.0 + 5.0 * k, with_alpha(t.success, 0.3 * (1.0 - k)));
            p.circle_filled(c, 3.0, t.success);
            text(&p, pos2(c.x + 8.0, hy), Align2::LEFT_CENTER, "Live", font(11.5), dim(&t), 40.0);
        }
        // tabs
        let labels = ["All", "Running", "Needs you", "Failed"];
        let counts = [
            self.runs.len(),
            self.runs.iter().filter(|r| r.state == RunState::Running).count(),
            self.runs.iter().filter(|r| r.state == RunState::NeedsInput).count(),
            self.runs.iter().filter(|r| r.state == RunState::Failed).count(),
        ];
        let mut x = rect.max.x - pad;
        let mut tab_rects = Vec::new();
        for k in (0..labels.len()).rev() {
            let lw = text_w(&p, labels[k], font(12.0)) + 30.0;
            let r = Rect::from_min_size(pos2(x - lw, hy - 14.0), vec2(lw, 28.0));
            tab_rects.push((k, r));
            x = r.min.x - 2.0;
        }
        let seg = Rect::from_min_max(pos2(x - 2.0, hy - 16.0), pos2(rect.max.x - pad + 2.0, hy + 16.0));
        p.rect_filled(seg, cr(10.0), track(&t));
        let sel = tab_rects.iter().find(|(k, _)| *k == tab).map(|(_, r)| *r).unwrap_or(seg);
        let kx = glowkit_core::anim::spring(ui, id.with("kx"), sel.min.x, 380.0, 30.0);
        let kw = glowkit_core::anim::spring(ui, id.with("kw"), sel.width(), 380.0, 30.0);
        p.rect_filled(Rect::from_min_size(pos2(kx, sel.min.y), vec2(kw, sel.height())), cr(8.0), surface(&t));
        let mut new_tab = tab;
        for (k, r) in &tab_rects {
            let resp = ui.interact(*r, id.with(("tab", *k)), Sense::click());
            if resp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if resp.clicked() {
                new_tab = *k;
            }
            let on = *k == tab;
            let lr = text(
                &p,
                pos2(r.min.x + 9.0, r.center().y),
                Align2::LEFT_CENTER,
                labels[*k],
                font(12.0),
                if on { t.foreground } else { dim(&t) },
                90.0,
            );
            let cnt = glowkit_core::anim::follow(ui, id.with(("cnt", *k)), counts[*k] as f32, 8.0);
            let alert = (*k == 2 || *k == 3) && counts[*k] > 0;
            let cc = if alert { ink(&t, if *k == 2 { t.warning } else { t.destructive }) } else { faint(&t) };
            p.text(
                pos2(lr.max.x + 5.0, r.center().y),
                Align2::LEFT_CENTER,
                format!("{}", cnt.round() as i32),
                mono(10.5),
                cc,
            );
        }
        if new_tab != tab {
            glowkit_core::set_state(ui, id.with("tab"), new_tab);
        }
        hline(&p, &t, rect.min.x, rect.max.x, rect.min.y + header);
        // columns
        let w = rect.width() - pad * 2.0;
        let cols = [0.0, 0.27, 0.40, 0.66, 0.76, 0.85];
        let cx = |k: usize| rect.min.x + pad + w * cols[k];
        let cy = rect.min.y + header + colh / 2.0;
        for (k, name) in ["AGENT / RUN", "STATUS", "STEP", "ELAPSED", "TOKENS", "COST"].iter().enumerate() {
            text(&p, pos2(cx(k), cy), Align2::LEFT_CENTER, name, font(9.5), faint(&t), 100.0);
        }
        hline(&p, &t, rect.min.x, rect.max.x, rect.min.y + header + colh);
        let top = rect.min.y + header + colh;
        let clip = Rect::from_min_max(pos2(rect.min.x, top), rect.max);
        let pc = p.with_clip_rect(clip.intersect(p.clip_rect()));
        for (slot, &i) in order.iter().enumerate() {
            let r = &self.runs[i];
            let rid = id.with(("row", &r.id));
            let target_y = top + slot as f32 * row_h;
            let y = glowkit_core::anim::spring(ui, rid.with("y"), target_y, 200.0, 24.0);
            let appear = glowkit_core::anim::follow(ui, rid.with("a"), 1.0, 6.0);
            let pr = faded(&pc, appear);
            let rr = Rect::from_min_size(pos2(rect.min.x, y), vec2(rect.width(), row_h));
            if ui.rect_contains_pointer(rr) {
                pr.rect_filled(rr, cr(0.0), with_alpha(t.foreground, 0.022));
            }
            let my = rr.center().y;
            text(&pr, pos2(cx(0), my - 8.0), Align2::LEFT_CENTER, &r.agent, font(12.5), t.foreground, w * 0.25);
            text(&pr, pos2(cx(0), my + 9.0), Align2::LEFT_CENTER, &r.id, mono(10.5), faint(&t), w * 0.25);
            state_badge(&pr, &t, pos2(cx(1), my), Align2::LEFT_CENTER, r.state, now);
            let sw = w * 0.24;
            let sl = if r.steps > 0 {
                format!("{}/{}  {}", r.step.min(r.steps), r.steps, r.step_label)
            } else {
                r.step_label.clone()
            };
            text(&pr, pos2(cx(2), my - 6.0), Align2::LEFT_CENTER, &sl, font(11.5), dim(&t), sw);
            let frac = if r.steps > 0 {
                let base = (r.step.saturating_sub(1)) as f32 / r.steps as f32;
                if r.state == RunState::Done {
                    1.0
                } else {
                    base + if r.state == RunState::Running { 0.5 / r.steps as f32 } else { 0.0 }
                }
            } else {
                0.0
            };
            let fr = glowkit_core::anim::follow(ui, rid.with("fr"), frac, 5.0);
            meter(&pr, &t, Rect::from_min_size(pos2(cx(2), my + 8.0), vec2(sw * 0.8, 3.0)), fr, r.state.color(&t));
            text(&pr, pos2(cx(3), my), Align2::LEFT_CENTER, &fmt_secs(r.elapsed), mono(11.5), t.foreground, 80.0);
            text(&pr, pos2(cx(4), my), Align2::LEFT_CENTER, &fmt_tok(r.tokens), mono(11.5), dim(&t), 80.0);
            let cr_ = text(&pr, pos2(cx(5), my), Align2::LEFT_CENTER, &fmt_usd(r.cost), mono(11.5), t.foreground, 70.0);
            let spark = Rect::from_min_max(pos2(cr_.max.x + 10.0, my - 9.0), pos2(rect.max.x - pad, my + 9.0));
            if spark.width() > 24.0 {
                sparkline(
                    &pr,
                    spark,
                    &r.cost_history,
                    if r.state == RunState::Failed { t.destructive } else { t.accent },
                );
            }
            if slot + 1 < order.len() {
                hline(&pr, &t, rr.min.x + pad, rr.max.x - pad, rr.max.y);
            }
        }
        if live {
            glowkit_core::animate(ui);
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// CostMeter

/// Spend against a budget, by model. A big spend figure and "of $10.00", a budget bar that turns
/// amber past the warning level and red (with a pulsing "over limit") past the budget, tokens in
/// and out, then one stacked bar of spend by model with a legend row per model (tokens and
/// dollars). Every number glides when it changes.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::CostMeter;
/// ui.add(CostMeter::new(10.0)
///     .model("Nova Ultra", 42_000.0, 6_100.0, 2.41)
///     .model("Nova 2", 120_000.0, 18_000.0, 1.62)
///     .model("Nova Flash", 300_000.0, 22_000.0, 0.38));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CostMeter {
    budget: f32,
    models: Vec<(String, f32, f32, f32)>,
    warn_at: f32,
    width: f32,
    title: String,
    period: String,
}

impl CostMeter {
    /// A meter with a budget of `budget` dollars.
    pub fn new(budget: f32) -> Self {
        Self {
            budget: budget.max(0.01),
            models: Vec::new(),
            warn_at: 0.8,
            width: 380.0,
            title: "Spend".into(),
            period: "This session".into(),
        }
    }
    /// Add a model's input tokens, output tokens and dollars.
    pub fn model(mut self, name: impl Into<String>, tokens_in: f32, tokens_out: f32, cost: f32) -> Self {
        self.models.push((name.into(), tokens_in, tokens_out, cost));
        self
    }
    /// Fraction of the budget at which the bar turns amber (default 0.8).
    pub fn warn_at(mut self, f: f32) -> Self {
        self.warn_at = f.clamp(0.1, 1.0);
        self
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(260.0);
        self
    }
    /// Title and period label.
    pub fn title(mut self, title: impl Into<String>, period: impl Into<String>) -> Self {
        self.title = title.into();
        self.period = period.into();
        self
    }
}

impl egui::Widget for CostMeter {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let now = glowkit_core::time(ui);
        let id = ui.next_auto_id().with("cost_meter");
        let spent_t: f32 = self.models.iter().map(|m| m.3).sum();
        let tin: f32 = self.models.iter().map(|m| m.1).sum();
        let tout: f32 = self.models.iter().map(|m| m.2).sum();
        let spent = glowkit_core::anim::follow(ui, id.with("s"), spent_t, 5.0);
        let h = 226.0 + self.models.len() as f32 * 26.0;
        let (rect, resp) = alloc_centered(ui, self.width, h, Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        panel(&p, &t, rect, 14.0);
        let pad = 18.0;
        text(
            &p,
            pos2(rect.min.x + pad, rect.min.y + 22.0),
            Align2::LEFT_CENTER,
            &self.title,
            font(13.0),
            t.foreground,
            160.0,
        );
        text(
            &p,
            pos2(rect.max.x - pad, rect.min.y + 22.0),
            Align2::RIGHT_CENTER,
            &self.period,
            font(11.5),
            faint(&t),
            160.0,
        );
        let frac = spent / self.budget;
        let over = frac > 1.0;
        let warn = frac >= self.warn_at;
        let c = if over {
            t.destructive
        } else if warn {
            t.warning
        } else {
            t.foreground
        };
        let big = text(
            &p,
            pos2(rect.min.x + pad, rect.min.y + 76.0),
            Align2::LEFT_BOTTOM,
            &fmt_usd(spent),
            mono(28.0),
            if over { ink(&t, t.destructive) } else { t.foreground },
            200.0,
        );
        text(
            &p,
            pos2(big.max.x + 8.0, big.max.y - 5.0),
            Align2::LEFT_BOTTOM,
            &format!("of {} budget", fmt_usd(self.budget)),
            font(12.0),
            dim(&t),
            180.0,
        );
        if over {
            let k = 0.6 + 0.4 * (now * 3.0).sin().abs();
            let r = badge(
                &p,
                &t,
                pos2(rect.max.x - pad, big.center().y),
                Align2::RIGHT_CENTER,
                "over limit",
                t.destructive,
                true,
            );
            p.rect_stroke(
                r.expand(2.0 * k),
                cr(8.0),
                Stroke::new(1.0, with_alpha(t.destructive, 0.3 * k)),
                StrokeKind::Outside,
            );
            glowkit_core::animate(ui);
        } else {
            text(
                &p,
                pos2(rect.max.x - pad, big.center().y),
                Align2::RIGHT_CENTER,
                &format!("{:.0}%", frac * 100.0),
                mono(12.0),
                if warn { ink(&t, t.warning) } else { dim(&t) },
                60.0,
            );
        }
        let bar = Rect::from_min_size(pos2(rect.min.x + pad, rect.min.y + 88.0), vec2(rect.width() - pad * 2.0, 8.0));
        p.rect_filled(bar, cr(4.0), track(&t));
        let fw = bar.width() * frac.min(1.0);
        if fw > 1.0 {
            p.rect_filled(Rect::from_min_size(bar.min, vec2(fw.max(8.0), bar.height())), cr(4.0), c);
        }
        let wx = bar.min.x + bar.width() * self.warn_at;
        p.line_segment(
            [pos2(wx, bar.min.y - 3.0), pos2(wx, bar.max.y + 3.0)],
            Stroke::new(1.0, with_alpha(t.foreground, 0.25)),
        );
        // tokens in/out
        let y = rect.min.y + 124.0;
        let half = (rect.width() - pad * 2.0) / 2.0;
        for (k, (label, v, ic)) in
            [("Tokens in", tin, Icon::ArrowDown), ("Tokens out", tout, Icon::ArrowUp)].iter().enumerate()
        {
            let x = rect.min.x + pad + k as f32 * half;
            let ib = Rect::from_center_size(pos2(x + 11.0, y + 4.0), vec2(22.0, 22.0));
            p.rect_filled(ib, cr(6.0), well(&t));
            icon(&p, *ic, ib.center(), 12.0, dim(&t));
            text(&p, pos2(x + 30.0, y - 3.0), Align2::LEFT_CENTER, label, font(10.5), faint(&t), 100.0);
            let vv = glowkit_core::anim::follow(ui, id.with(("tok", k)), *v, 5.0);
            text(&p, pos2(x + 30.0, y + 12.0), Align2::LEFT_CENTER, &fmt_tok(vv), mono(13.0), t.foreground, 100.0);
        }
        // by model
        let sy = rect.min.y + 160.0;
        hline(&p, &t, rect.min.x + pad, rect.max.x - pad, sy - 6.0);
        text(&p, pos2(rect.min.x + pad, sy + 8.0), Align2::LEFT_CENTER, "BY MODEL", font(9.5), faint(&t), 100.0);
        let sb = Rect::from_min_size(pos2(rect.min.x + pad, sy + 22.0), vec2(rect.width() - pad * 2.0, 10.0));
        p.rect_filled(sb, cr(5.0), track(&t));
        let tot = spent_t.max(1e-4);
        let mut x = sb.min.x;
        let n = self.models.len();
        for (k, m) in self.models.iter().enumerate() {
            let f = glowkit_core::anim::follow(ui, id.with(("mf", k)), m.3 / tot, 5.0);
            let w = sb.width() * f;
            let r = Rect::from_min_size(pos2(x, sb.min.y), vec2(w, sb.height()));
            let corner = egui::CornerRadius {
                nw: if k == 0 { 5 } else { 0 },
                sw: if k == 0 { 5 } else { 0 },
                ne: if k + 1 == n { 5 } else { 0 },
                se: if k + 1 == n { 5 } else { 0 },
            };
            p.rect_filled(r, corner, series(&t, k));
            if k + 1 < n && w > 2.0 {
                p.line_segment([pos2(r.max.x, r.min.y), pos2(r.max.x, r.max.y)], Stroke::new(2.0, surface(&t)));
            }
            x += w;
        }
        for (k, m) in self.models.iter().enumerate() {
            let ly = sy + 50.0 + k as f32 * 26.0;
            p.rect_filled(
                Rect::from_center_size(pos2(rect.min.x + pad + 4.0, ly), vec2(8.0, 8.0)),
                cr(2.0),
                series(&t, k),
            );
            text(&p, pos2(rect.min.x + pad + 16.0, ly), Align2::LEFT_CENTER, &m.0, font(12.0), t.foreground, 140.0);
            let cv = glowkit_core::anim::follow(ui, id.with(("mc", k)), m.3, 5.0);
            let r = text(
                &p,
                pos2(rect.max.x - pad, ly),
                Align2::RIGHT_CENTER,
                &fmt_usd(cv),
                mono(11.5),
                t.foreground,
                80.0,
            );
            text(
                &p,
                pos2(r.min.x - 14.0, ly),
                Align2::RIGHT_CENTER,
                &format!("{} tok", fmt_tok(m.1 + m.2)),
                font(11.0),
                faint(&t),
                100.0,
            );
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// AgentFleetGrid

/// One agent tile in an [`AgentFleetGrid`].
#[derive(Clone, Debug, Default)]
pub struct FleetAgent {
    /// Agent name.
    pub name: String,
    /// Role / short description.
    pub role: String,
    /// Current state.
    pub state: RunState,
    /// Progress of the current job 0..1 (drawn as the ring).
    pub progress: f32,
    /// What it is doing.
    pub activity: String,
    /// Runs today.
    pub runs: u32,
    /// Success rate 0..1.
    pub success: f32,
}

impl FleetAgent {
    /// An agent `name` with `role` in `state`.
    pub fn new(name: impl Into<String>, role: impl Into<String>, state: RunState) -> Self {
        Self { name: name.into(), role: role.into(), state, success: 1.0, ..Default::default() }
    }
    /// Current job progress and activity.
    pub fn activity(mut self, progress: f32, s: impl Into<String>) -> Self {
        self.progress = progress.clamp(0.0, 1.0);
        self.activity = s.into();
        self
    }
    /// Runs today and success rate.
    pub fn stats(mut self, runs: u32, success: f32) -> Self {
        self.runs = runs;
        self.success = success.clamp(0.0, 1.0);
        self
    }
}

/// A fleet of agents as a grid of tiles. Each tile has a status ring around the agent's mark
/// (progress for working agents with a bright head, a pulsing amber ring for one that needs you,
/// a red ring for a failure, a dashed one when idle), the agent's name and role, what it is
/// doing, and its runs today and success rate. A summary strip on top counts each state. Tiles
/// lift on hover and the grid reflows with the width.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{AgentFleetGrid, FleetAgent, RunState};
/// let fleet = vec![
///     FleetAgent::new("Deploy Fixer", "DevOps", RunState::Running).activity(0.4, "Reading build logs"),
///     FleetAgent::new("Data Loader", "Data", RunState::NeedsInput).activity(0.7, "Which region first?"),
/// ];
/// ui.add(AgentFleetGrid::new(&fleet));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AgentFleetGrid<'a> {
    agents: &'a [FleetAgent],
    width: f32,
    tile_w: f32,
}

impl<'a> AgentFleetGrid<'a> {
    /// A grid over `agents`.
    pub fn new(agents: &'a [FleetAgent]) -> Self {
        Self { agents, width: 900.0, tile_w: 200.0 }
    }
    /// Maximum width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(240.0);
        self
    }
    /// Preferred tile width in points.
    pub fn tile_width(mut self, w: f32) -> Self {
        self.tile_w = w.clamp(150.0, 320.0);
        self
    }
}

fn ring(p: &egui::Painter, t: &Theme, c: egui::Pos2, r: f32, a: &FleetAgent, now: f32, prog: f32) {
    let col = a.state.color(t);
    let w = 3.0;
    match a.state {
        RunState::Running => {
            p.circle_stroke(c, r, Stroke::new(w, track(t)));
            let a0 = -FRAC_PI_2;
            let a1 = a0 + TAU * prog.max(0.02);
            let c0 = with_alpha(col, 0.35);
            paint::arc_gradient(p, c, r, a0, a1, w, |u| mix(c0, col, u));
            let head = c + egui::Vec2::angled(a1) * r;
            paint::glow(p, head, 9.0, with_alpha(col, 0.45));
            // orbiting shimmer
            let s = now * 2.0;
            p.circle_filled(c + egui::Vec2::angled(s) * (r + 5.0), 1.3, with_alpha(col, 0.5));
        }
        RunState::NeedsInput => {
            let k = 0.5 + 0.5 * (now * 3.2).sin();
            p.circle_stroke(c, r + 3.0 * k, Stroke::new(6.0, with_alpha(col, 0.12 * k)));
            p.circle_stroke(c, r, Stroke::new(w, col));
        }
        RunState::Failed => {
            p.circle_stroke(c, r, Stroke::new(w, col));
        }
        RunState::Done => {
            p.circle_stroke(c, r, Stroke::new(w, with_alpha(col, 0.85)));
        }
        _ => dashed_circle(p, c, r, Stroke::new(1.5, mix(track(t), t.muted_foreground, 0.4)), 18),
    }
}

impl egui::Widget for AgentFleetGrid<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let now = glowkit_core::time(ui);
        let id = ui.next_auto_id().with("fleet_grid");
        let avail = ui.available_width().min(self.width);
        let gap = 12.0;
        let cols = (((avail + gap) / (self.tile_w + gap)).floor() as usize).max(1);
        let tw = (avail - gap * (cols as f32 - 1.0)) / cols as f32;
        let th = 138.0;
        let rows = self.agents.len().div_ceil(cols);
        let header = 48.0;
        let h = header + rows as f32 * (th + gap);
        let (rect, resp) = alloc_centered(ui, avail, h, Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        // summary strip
        let states = [RunState::Running, RunState::NeedsInput, RunState::Failed, RunState::Done, RunState::Idle];
        let hy = rect.min.y + 18.0;
        let tr = text(&p, pos2(rect.min.x + 2.0, hy), Align2::LEFT_CENTER, "Fleet", font(15.0), t.foreground, 100.0);
        text(
            &p,
            pos2(tr.max.x + 8.0, hy),
            Align2::LEFT_CENTER,
            &format!("{} agents", self.agents.len()),
            font(12.0),
            dim(&t),
            100.0,
        );
        let mut x = rect.max.x;
        for s in states.iter().rev() {
            let n = self
                .agents
                .iter()
                .filter(|a| a.state == *s || (*s == RunState::Idle && a.state == RunState::Queued))
                .count();
            let nv = glowkit_core::anim::follow(ui, id.with(("n", s.label())), n as f32, 8.0);
            let r = text(
                &p,
                pos2(x, hy),
                Align2::RIGHT_CENTER,
                &format!("{} {}", nv.round() as i32, s.label().to_lowercase()),
                font(12.0),
                if n > 0 && matches!(s, RunState::NeedsInput | RunState::Failed) {
                    ink(&t, s.color(&t))
                } else {
                    dim(&t)
                },
                140.0,
            );
            p.circle_filled(pos2(r.min.x - 8.0, hy), 3.5, s.color(&t));
            x = r.min.x - 22.0;
        }
        // tiles
        for (i, a) in self.agents.iter().enumerate() {
            let (cx, cy) = (i % cols, i / cols);
            let r0 = Rect::from_min_size(
                pos2(rect.min.x + cx as f32 * (tw + gap), rect.min.y + header + cy as f32 * (th + gap)),
                vec2(tw, th),
            );
            let tid = id.with(("tile", i));
            let hov = ui.rect_contains_pointer(r0);
            let resp_t = ui.interact(r0, tid, Sense::click());
            if resp_t.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            let k = glowkit_core::anim::follow(ui, tid.with("h"), if hov { 1.0 } else { 0.0 }, 12.0);
            let appear =
                ease(Easing::OutCubic, glowkit_core::anim::follow(ui, tid.with("ap"), 1.0, 4.0 + (i % 7) as f32 * 0.4));
            let r = r0.translate(vec2(0.0, -2.0 * k + (1.0 - appear) * 10.0));
            let pa = faded(&p, appear);
            panel_fill(&pa, &t, r, 14.0, surface(&t), 0.5 + k);
            let ac = pos2(r.min.x + 34.0, r.min.y + 36.0);
            let prog = glowkit_core::anim::follow(ui, tid.with("p"), a.progress, 4.0);
            ring(&pa, &t, ac, 20.0, a, now, prog);
            let init: String = a.name.split_whitespace().filter_map(|w| w.chars().next()).take(2).collect();
            pa.text(ac, Align2::CENTER_CENTER, init, font(13.0), t.foreground);
            let tx = ac.x + 30.0;
            text(
                &pa,
                pos2(tx, ac.y - 8.0),
                Align2::LEFT_CENTER,
                &a.name,
                font(13.0),
                t.foreground,
                r.max.x - tx - 10.0,
            );
            text(&pa, pos2(tx, ac.y + 9.0), Align2::LEFT_CENTER, &a.role, font(11.0), faint(&t), r.max.x - tx - 10.0);
            let col = a.state.color(&t);
            let line_c =
                if matches!(a.state, RunState::NeedsInput | RunState::Failed) { ink(&t, col) } else { dim(&t) };
            let ay = r.min.y + 76.0;
            if a.state == RunState::Running {
                spinner(&pa, pos2(r.min.x + 19.0, ay), 4.5, col, now, 1.4);
            } else {
                pa.circle_filled(pos2(r.min.x + 19.0, ay), 3.0, col);
            }
            let act = if a.activity.is_empty() { a.state.label().to_string() } else { a.activity.clone() };
            text(&pa, pos2(r.min.x + 30.0, ay), Align2::LEFT_CENTER, &act, font(11.5), line_c, r.width() - 42.0);
            hline(&pa, &t, r.min.x + 14.0, r.max.x - 14.0, r.min.y + 98.0);
            let sy = r.min.y + 118.0;
            let s1 = text(
                &pa,
                pos2(r.min.x + 14.0, sy),
                Align2::LEFT_CENTER,
                &format!("{}", a.runs),
                mono(12.0),
                t.foreground,
                40.0,
            );
            text(&pa, pos2(s1.max.x + 4.0, sy), Align2::LEFT_CENTER, "runs", font(11.0), faint(&t), 40.0);
            let sc = if a.success < 0.85 { ink(&t, t.warning) } else { t.foreground };
            let s2 = text(&pa, pos2(r.max.x - 14.0, sy), Align2::RIGHT_CENTER, "ok", font(11.0), faint(&t), 40.0);
            text(
                &pa,
                pos2(s2.min.x - 4.0, sy),
                Align2::RIGHT_CENTER,
                &format!("{:.0}%", a.success * 100.0),
                mono(12.0),
                sc,
                50.0,
            );
            // mini success bar between
            let mb = Rect::from_min_max(pos2(s1.max.x + 40.0, sy - 2.0), pos2(s2.min.x - 44.0, sy + 2.0));
            if mb.width() > 16.0 {
                meter(&pa, &t, mb, a.success, mix(t.success, t.muted_foreground, 0.2));
            }
        }
        if self.agents.iter().any(|a| matches!(a.state, RunState::Running | RunState::NeedsInput)) {
            glowkit_core::animate(ui);
        }
        let _: Color32 = t.accent;
        resp
    }
}
