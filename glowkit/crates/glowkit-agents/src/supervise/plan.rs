//! Planning and context: [`PlanBoard`] (plan steps on a small board) and [`ContextWindowViz`]
//! (what fills the model's context window).

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Rect, Response, Sense, Stroke, StrokeKind, Ui};
use glowkit_ai::Icon;
use glowkit_core::anim::{ease, Easing};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// One step of a [`PlanBoard`].
#[derive(Clone, Debug)]
pub struct PlanCard {
    /// Stable id (cards glide between columns by id).
    pub id: u64,
    /// What the step does.
    pub title: String,
    /// `Queued` = planned, `Running`, `NeedsInput` / `Failed` = review, `Done`.
    pub state: RunState,
    /// Agent assigned to it.
    pub agent: String,
    /// Main tool icon.
    pub icon: Icon,
    /// Progress 0..1 while running.
    pub progress: f32,
    /// Estimate or actual time in seconds.
    pub secs: f32,
}

impl PlanCard {
    /// A step `id` titled `title` in `state`.
    pub fn new(id: u64, title: impl Into<String>, state: RunState) -> Self {
        Self { id, title: title.into(), state, agent: String::new(), icon: Icon::Sparkle, progress: 0.0, secs: 0.0 }
    }
    /// Assigned agent and tool icon.
    pub fn agent(mut self, name: impl Into<String>, icon: Icon) -> Self {
        self.agent = name.into();
        self.icon = icon;
        self
    }
    /// Progress 0..1 and seconds.
    pub fn progress(mut self, p: f32, secs: f32) -> Self {
        self.progress = p.clamp(0.0, 1.0);
        self.secs = secs;
        self
    }
}

fn column(s: RunState) -> usize {
    match s {
        RunState::Queued | RunState::Idle | RunState::Paused => 0,
        RunState::Running => 1,
        RunState::NeedsInput | RunState::Failed => 2,
        RunState::Done => 3,
    }
}

/// An agent's plan as a small board with four columns: Planned, Running, Review and Done. Step
/// cards carry their number, title, agent and tool, with a live progress bar while running, an
/// amber or red edge when they need review, and a check once done. When a step changes state its
/// card lifts and glides to the next column with a spring; column counts roll.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{PlanBoard, PlanCard, RunState};
/// use glowkit_ai::Icon;
/// let steps = vec![
///     PlanCard::new(1, "Read the failing build log", RunState::Done).agent("Deploy Fixer", Icon::Terminal),
///     PlanCard::new(2, "Add the missing dependency", RunState::Running).agent("Deploy Fixer", Icon::Code).progress(0.4, 12.0),
///     PlanCard::new(3, "Re-run the test suite", RunState::Queued),
/// ];
/// ui.add(PlanBoard::new("Fix the failing deploy", &steps));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PlanBoard<'a> {
    goal: String,
    steps: &'a [PlanCard],
    width: f32,
}

impl<'a> PlanBoard<'a> {
    /// A board for `goal` over `steps` (in plan order).
    pub fn new(goal: impl Into<String>, steps: &'a [PlanCard]) -> Self {
        Self { goal: goal.into(), steps, width: 900.0 }
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(480.0);
        self
    }
}

impl egui::Widget for PlanBoard<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let now = glowkit_core::time(ui);
        let id = ui.next_auto_id().with("plan_board");
        let names = ["Planned", "Running", "Review", "Done"];
        let card_h = 78.0;
        let gap = 8.0;
        let mut cols: [Vec<usize>; 4] = Default::default();
        for (i, s) in self.steps.iter().enumerate() {
            cols[column(s.state)].push(i);
        }
        let most = cols.iter().map(|c| c.len()).max().unwrap_or(0).max(2);
        let header = 54.0;
        let col_head = 34.0;
        let h = header + col_head + most as f32 * (card_h + gap) + 14.0;
        let (rect, resp) = alloc_centered(ui, self.width, h, Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        panel(&p, &t, rect, 16.0);
        let pad = 14.0;
        let done = cols[3].len();
        text(
            &p,
            pos2(rect.min.x + pad + 2.0, rect.min.y + 20.0),
            Align2::LEFT_CENTER,
            "Plan",
            font(11.5),
            faint(&t),
            60.0,
        );
        text(
            &p,
            pos2(rect.min.x + pad + 2.0, rect.min.y + 38.0),
            Align2::LEFT_CENTER,
            &self.goal,
            font(14.0),
            t.foreground,
            rect.width() - 220.0,
        );
        let frac = glowkit_core::anim::follow(ui, id.with("f"), done as f32 / self.steps.len().max(1) as f32, 5.0);
        let bar = Rect::from_min_size(pos2(rect.max.x - pad - 140.0, rect.min.y + 36.0), vec2(140.0, 4.0));
        meter(&p, &t, bar, frac, t.success);
        text(
            &p,
            pos2(bar.max.x, rect.min.y + 22.0),
            Align2::RIGHT_CENTER,
            &format!("{done} of {} steps", self.steps.len()),
            font(11.5),
            dim(&t),
            140.0,
        );
        let cw = (rect.width() - pad * 2.0 - gap * 3.0) / 4.0;
        let top = rect.min.y + header;
        for (c, name) in names.iter().enumerate() {
            let cr_ = Rect::from_min_max(
                pos2(rect.min.x + pad + c as f32 * (cw + gap), top),
                pos2(rect.min.x + pad + c as f32 * (cw + gap) + cw, rect.max.y - pad),
            );
            p.rect_filled(cr_, cr(12.0), well(&t));
            let dot = [t.muted_foreground, t.info, t.warning, t.success][c];
            p.circle_filled(pos2(cr_.min.x + 14.0, top + col_head / 2.0), 3.5, dot);
            text(
                &p,
                pos2(cr_.min.x + 24.0, top + col_head / 2.0),
                Align2::LEFT_CENTER,
                name,
                font(12.0),
                t.foreground,
                cw - 50.0,
            );
            let n = glowkit_core::anim::follow(ui, id.with(("n", c)), cols[c].len() as f32, 8.0);
            text(
                &p,
                pos2(cr_.max.x - 12.0, top + col_head / 2.0),
                Align2::RIGHT_CENTER,
                &format!("{}", n.round() as i32),
                mono(11.0),
                faint(&t),
                30.0,
            );
        }
        // cards
        let mut drawn: Vec<(f32, usize, Rect)> = Vec::new();
        for (c, list) in cols.iter().enumerate() {
            for (k, &i) in list.iter().enumerate() {
                let s = &self.steps[i];
                let target =
                    pos2(rect.min.x + pad + c as f32 * (cw + gap) + 6.0, top + col_head + k as f32 * (card_h + gap));
                let cid = id.with(("card", s.id));
                let pos = glowkit_core::anim::spring2(ui, cid.with("p"), target.to_vec2(), 170.0, 20.0);
                let moving = (pos - target.to_vec2()).length();
                let r = Rect::from_min_size(pos.to_pos2(), vec2(cw - 12.0, card_h));
                drawn.push((moving, i, r));
            }
        }
        drawn.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (moving, i, r) in drawn {
            let s = &self.steps[i];
            let lift = (moving / 40.0).clamp(0.0, 1.0);
            panel_fill(&p, &t, r, 10.0, surface(&t), 0.5 + lift * 2.0);
            let sc = s.state.color(&t);
            if matches!(s.state, RunState::NeedsInput | RunState::Failed) {
                p.rect_stroke(r, cr(10.0), Stroke::new(1.3, with_alpha(sc, 0.7)), StrokeKind::Inside);
            }
            let num = format!("{:02}", i + 1);
            text(&p, pos2(r.min.x + 11.0, r.min.y + 15.0), Align2::LEFT_CENTER, &num, mono(10.5), faint(&t), 30.0);
            if s.state == RunState::Done {
                state_icon(&p, &t, pos2(r.max.x - 14.0, r.min.y + 15.0), 6.5, RunState::Done, now);
            } else if s.state != RunState::Queued {
                state_icon(&p, &t, pos2(r.max.x - 14.0, r.min.y + 15.0), 6.5, s.state, now);
            }
            let tc = if s.state == RunState::Done { dim(&t) } else { t.foreground };
            para(&p, pos2(r.min.x + 11.0, r.min.y + 25.0), &s.title, font(12.0), tc, r.width() - 22.0, 2);
            let by = r.max.y - 13.0;
            if !s.agent.is_empty() {
                icon(&p, s.icon, pos2(r.min.x + 17.0, by), 11.0, faint(&t));
                text(
                    &p,
                    pos2(r.min.x + 27.0, by),
                    Align2::LEFT_CENTER,
                    &s.agent,
                    font(10.5),
                    faint(&t),
                    r.width() - 80.0,
                );
            }
            if s.secs > 0.0 {
                text(
                    &p,
                    pos2(r.max.x - 11.0, by),
                    Align2::RIGHT_CENTER,
                    &fmt_secs(s.secs),
                    mono(10.0),
                    faint(&t),
                    50.0,
                );
            }
            if s.state == RunState::Running {
                let pr = glowkit_core::anim::follow(ui, id.with(("pr", s.id)), s.progress, 5.0);
                let br = Rect::from_min_size(pos2(r.min.x + 1.0, r.max.y - 3.0), vec2((r.width() - 2.0) * pr, 2.0));
                p.rect_filled(br, cr(1.0), sc);
                let u = (now * 0.7).fract();
                let gx = br.min.x + br.width() * u;
                paint::glow(&p, pos2(gx, br.center().y), 6.0, with_alpha(sc, 0.4));
            }
        }
        if self.steps.iter().any(|s| s.state.is_active()) {
            glowkit_core::animate(ui);
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// ContextWindowViz

/// A part of the context window in a [`ContextWindowViz`].
#[derive(Clone, Debug)]
pub struct ContextPart {
    /// Label, e.g. `"System prompt"`.
    pub label: String,
    /// Tokens it takes.
    pub tokens: f32,
    /// Color (default: chart palette).
    pub color: Option<Color32>,
}

impl ContextPart {
    /// A part `label` taking `tokens`.
    pub fn new(label: impl Into<String>, tokens: f32) -> Self {
        Self { label: label.into(), tokens: tokens.max(0.0), color: None }
    }
    /// Its color.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

/// What fills a model's context window. A headline "142K / 200K" with the percentage, one
/// stacked bar by source with an auto-compact marker, and a field of cells (each a slice of the
/// window) colored by source with the free space left as dotted cells. Hovering a legend row or
/// a cell lights up that source everywhere; segments glide when the window grows or is
/// compacted.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{ContextPart, ContextWindowViz};
/// let parts = vec![
///     ContextPart::new("System prompt", 3_200.0),
///     ContextPart::new("Tools", 14_800.0),
///     ContextPart::new("Files", 61_000.0),
///     ContextPart::new("Conversation", 48_000.0),
/// ];
/// ui.add(ContextWindowViz::new(&parts, 200_000.0).compact_at(0.9));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ContextWindowViz<'a> {
    parts: &'a [ContextPart],
    limit: f32,
    compact_at: f32,
    width: f32,
    cells: usize,
}

impl<'a> ContextWindowViz<'a> {
    /// A view of `parts` in a window of `limit` tokens.
    pub fn new(parts: &'a [ContextPart], limit: f32) -> Self {
        Self { parts, limit: limit.max(1.0), compact_at: 0.9, width: 640.0, cells: 200 }
    }
    /// Fraction at which the conversation is auto-compacted (draws a marker).
    pub fn compact_at(mut self, f: f32) -> Self {
        self.compact_at = f.clamp(0.1, 1.0);
        self
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(320.0);
        self
    }
    /// Number of cells in the field (default 200).
    pub fn cells(mut self, n: usize) -> Self {
        self.cells = n.clamp(40, 600);
        self
    }
}

impl egui::Widget for ContextWindowViz<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = ui.next_auto_id().with("ctx_viz");
        let cols = 40usize;
        let rows = self.cells.div_ceil(cols);
        let legend_rows = self.parts.len().div_ceil(2);
        let h = 132.0 + rows as f32 * 12.0 + legend_rows as f32 * 24.0 + 6.0;
        let (rect, resp) = alloc_centered(ui, self.width, h, Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        panel(&p, &t, rect, 14.0);
        let pad = 18.0;
        let used_t: f32 = self.parts.iter().map(|p| p.tokens).sum();
        let used = glowkit_core::anim::follow(ui, id.with("u"), used_t, 5.0);
        let frac = used / self.limit;
        text(
            &p,
            pos2(rect.min.x + pad, rect.min.y + 22.0),
            Align2::LEFT_CENTER,
            "Context window",
            font(12.5),
            dim(&t),
            200.0,
        );
        let big = text(
            &p,
            pos2(rect.min.x + pad, rect.min.y + 62.0),
            Align2::LEFT_BOTTOM,
            &fmt_tok(used),
            mono(26.0),
            t.foreground,
            140.0,
        );
        text(
            &p,
            pos2(big.max.x + 6.0, big.max.y - 5.0),
            Align2::LEFT_BOTTOM,
            &format!("/ {} tokens", fmt_tok(self.limit)),
            font(12.0),
            dim(&t),
            160.0,
        );
        let pc = if frac >= self.compact_at {
            t.destructive
        } else if frac > self.compact_at - 0.15 {
            t.warning
        } else {
            t.success
        };
        badge(
            &p,
            &t,
            pos2(rect.max.x - pad, rect.min.y + 22.0),
            Align2::RIGHT_CENTER,
            &format!("{:.0}% full", frac * 100.0),
            pc,
            true,
        );
        if frac > self.compact_at - 0.15 {
            text(
                &p,
                pos2(rect.max.x - pad, big.center().y + 4.0),
                Align2::RIGHT_CENTER,
                &format!("auto-compacts at {:.0}%", self.compact_at * 100.0),
                font(11.0),
                faint(&t),
                200.0,
            );
        }
        // hovered part
        let hov_part: Option<usize> = glowkit_core::state(ui, id.with("hp"), || None);
        let mut new_hover: Option<usize> = None;
        // stacked bar
        let bar = Rect::from_min_size(pos2(rect.min.x + pad, rect.min.y + 80.0), vec2(rect.width() - pad * 2.0, 12.0));
        p.rect_filled(bar, cr(6.0), track(&t));
        let mut x = bar.min.x;
        let color_of = |i: usize, c: &ContextPart| c.color.unwrap_or_else(|| series(&t, i));
        let mut part_fr = Vec::new();
        for (i, c) in self.parts.iter().enumerate() {
            let f = glowkit_core::anim::follow(ui, id.with(("pf", i)), c.tokens / self.limit, 5.0);
            part_fr.push(f);
            let w = bar.width() * f;
            let r = Rect::from_min_size(pos2(x, bar.min.y), vec2(w, bar.height()));
            let dim_it = hov_part.is_some_and(|h| h != i);
            let col = color_of(i, c);
            p.with_clip_rect(r).rect_filled(bar, cr(6.0), if dim_it { with_alpha(col, 0.3) } else { col });
            if i > 0 && w > 1.0 {
                p.line_segment([pos2(x, bar.min.y), pos2(x, bar.max.y)], Stroke::new(1.5, surface(&t)));
            }
            if ui.rect_contains_pointer(r) {
                new_hover = Some(i);
            }
            x += w;
        }
        let mx = bar.min.x + bar.width() * self.compact_at;
        p.line_segment(
            [pos2(mx, bar.min.y - 5.0), pos2(mx, bar.max.y + 5.0)],
            Stroke::new(1.5, with_alpha(t.foreground, 0.5)),
        );
        // cell field
        let field_top = bar.max.y + 22.0;
        let cell = (bar.width() / cols as f32).floor();
        let per = self.limit / self.cells as f32;
        let mut bounds = Vec::new();
        let mut acc = 0.0;
        for f in &part_fr {
            acc += f * self.cells as f32;
            bounds.push(acc);
        }
        for k in 0..self.cells {
            let (cx, cy) = (k % cols, k / cols);
            let r = Rect::from_min_size(
                pos2(bar.min.x + cx as f32 * cell, field_top + cy as f32 * 12.0),
                vec2(cell - 3.0, 9.0),
            );
            let kf = k as f32 + 0.5;
            let part = bounds.iter().position(|b| kf < *b);
            match part {
                Some(i) => {
                    let col = color_of(i, &self.parts[i]);
                    let dim_it = hov_part.is_some_and(|h| h != i);
                    let lit = hov_part == Some(i);
                    let c = if dim_it { mix(track(&t), col, 0.25) } else { col };
                    p.rect_filled(r, cr(2.0), c);
                    if lit {
                        p.rect_stroke(
                            r.expand(1.0),
                            cr(3.0),
                            Stroke::new(1.0, with_alpha(col, 0.6)),
                            StrokeKind::Outside,
                        );
                    }
                    if ui.rect_contains_pointer(r) {
                        new_hover = Some(i);
                    }
                }
                None => {
                    p.circle_filled(r.center(), 1.2, mix(track(&t), t.muted_foreground, 0.3));
                }
            }
            let at_mark = k == (self.compact_at * self.cells as f32).round() as usize;
            let _ = (kf, per);
            if at_mark {
                p.line_segment(
                    [pos2(r.min.x - 1.5, r.min.y - 2.0), pos2(r.min.x - 1.5, r.max.y + 2.0)],
                    Stroke::new(1.0, with_alpha(t.foreground, 0.4)),
                );
            }
        }
        // legend
        let ly0 = field_top + rows as f32 * 12.0 + 18.0;
        let lw = (rect.width() - pad * 2.0) / 2.0;
        for (i, c) in self.parts.iter().enumerate() {
            let r = Rect::from_min_size(
                pos2(rect.min.x + pad + (i % 2) as f32 * lw, ly0 + (i / 2) as f32 * 24.0),
                vec2(lw - 12.0, 22.0),
            );
            let hov = ui.rect_contains_pointer(r);
            if hov {
                new_hover = Some(i);
            }
            let k =
                glowkit_core::anim::follow(ui, id.with(("lh", i)), if hov_part == Some(i) { 1.0 } else { 0.0 }, 14.0);
            if k > 0.01 {
                p.rect_filled(r.expand2(vec2(4.0, 0.0)), cr(6.0), with_alpha(t.foreground, 0.04 * k));
            }
            let col = color_of(i, c);
            p.rect_filled(Rect::from_center_size(pos2(r.min.x + 5.0, r.center().y), vec2(9.0, 9.0)), cr(2.5), col);
            text(
                &p,
                pos2(r.min.x + 16.0, r.center().y),
                Align2::LEFT_CENTER,
                &c.label,
                font(12.0),
                t.foreground,
                r.width() - 110.0,
            );
            let pr = text(
                &p,
                pos2(r.max.x, r.center().y),
                Align2::RIGHT_CENTER,
                &format!("{:.0}%", c.tokens / self.limit * 100.0),
                mono(11.0),
                faint(&t),
                40.0,
            );
            text(
                &p,
                pos2(pr.min.x - 8.0, r.center().y),
                Align2::RIGHT_CENTER,
                &fmt_tok(c.tokens),
                mono(11.0),
                dim(&t),
                60.0,
            );
        }
        if new_hover != hov_part {
            glowkit_core::set_state(ui, id.with("hp"), new_hover);
        }
        let _ = (ease(Easing::Linear, 0.0), Icon::Brain);
        resp
    }
}
