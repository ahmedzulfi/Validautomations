//! Multi-agent structure: [`AgentTree`] (who started whom) and [`HandoffCard`] (what went along
//! when one agent passed work to the next).

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, Rect, Response, Sense, Stroke, Ui};
use glowkit_ai::Icon;
use glowkit_core::anim::{ease, Easing};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// One agent in an [`AgentTree`]: its name, model, current task, state, own cost and time, and
/// the sub-agents it started.
#[derive(Clone, Debug, Default)]
pub struct AgentNode {
    /// Display name, e.g. `"Researcher"`.
    pub name: String,
    /// Model label, e.g. `"Sonnet 4.5"`.
    pub model: String,
    /// What it is doing right now.
    pub task: String,
    /// Life-cycle state.
    pub state: RunState,
    /// Its own spend in dollars (children are rolled up on top).
    pub cost: f32,
    /// Seconds it has been working.
    pub secs: f32,
    /// Sub-agents it started.
    pub children: Vec<AgentNode>,
}

impl AgentNode {
    /// A queued agent called `name` running on `model`.
    pub fn new(name: impl Into<String>, model: impl Into<String>) -> Self {
        Self { name: name.into(), model: model.into(), ..Default::default() }
    }
    /// What it is doing right now.
    pub fn task(mut self, s: impl Into<String>) -> Self {
        self.task = s.into();
        self
    }
    /// Life-cycle state.
    pub fn state(mut self, s: RunState) -> Self {
        self.state = s;
        self
    }
    /// Own spend in dollars.
    pub fn cost(mut self, c: f32) -> Self {
        self.cost = c;
        self
    }
    /// Seconds spent.
    pub fn secs(mut self, s: f32) -> Self {
        self.secs = s;
        self
    }
    /// Add a sub-agent.
    pub fn child(mut self, c: AgentNode) -> Self {
        self.children.push(c);
        self
    }
    /// Cost of this agent plus every descendant.
    pub fn total_cost(&self) -> f32 {
        self.cost + self.children.iter().map(|c| c.total_cost()).sum::<f32>()
    }
    /// Number of agents in this branch (itself included).
    pub fn count(&self) -> usize {
        1 + self.children.iter().map(|c| c.count()).sum::<usize>()
    }
    fn count_state(&self, s: RunState) -> usize {
        (self.state == s) as usize + self.children.iter().map(|c| c.count_state(s)).sum::<usize>()
    }
}

fn initials(name: &str) -> String {
    let mut it = name.split_whitespace();
    let a = it.next().and_then(|w| w.chars().next()).unwrap_or('A');
    match it.next().and_then(|w| w.chars().next()) {
        Some(b) => format!("{a}{b}"),
        None => a.to_uppercase().collect(),
    }
}

/// An orchestrator and its sub-agents as a live tree of cards. Each card shows the agent's mark,
/// model, state badge, current task and its cost and time; the orchestrator sits on top, its
/// sub-agents in a row below and deeper agents stack under their parent with elbow connectors.
/// Connectors to working agents carry a gliding light. The chevron on a card folds its branch
/// into one line with the branch's rolled-up cost.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{AgentNode, AgentTree, RunState};
/// let root = AgentNode::new("Orchestrator", "Opus 4.1").state(RunState::Running).task("Ship the pricing page")
///     .child(AgentNode::new("Researcher", "Sonnet 4.5").state(RunState::Running).cost(0.06))
///     .child(AgentNode::new("Writer", "Sonnet 4.5").state(RunState::Queued));
/// ui.add(AgentTree::new(&root).title("Agents"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AgentTree<'a> {
    root: &'a AgentNode,
    title: String,
    width: f32,
    card_w: f32,
    flow: bool,
    accent: Option<Color32>,
}

impl<'a> AgentTree<'a> {
    /// A tree for `root` and its descendants.
    pub fn new(root: &'a AgentNode) -> Self {
        Self { root, title: "Agents".into(), width: 860.0, card_w: 210.0, flow: true, accent: None }
    }
    /// Header title.
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Maximum width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(320.0);
        self
    }
    /// Preferred card width in points.
    pub fn card_width(mut self, w: f32) -> Self {
        self.card_w = w.clamp(150.0, 320.0);
        self
    }
    /// Run a light along connectors to working agents.
    pub fn flow(mut self, on: bool) -> Self {
        self.flow = on;
        self
    }
    /// Connector / highlight color (default: the theme's info blue).
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = Some(c);
        self
    }
}

const CARD_H: f32 = 104.0;
const SUB_H: f32 = 80.0;

struct Placed<'n> {
    node: &'n AgentNode,
    rect: Rect,
    key: String,
    depth: usize,
    color: usize,
    alpha: f32,
}

impl egui::Widget for AgentTree<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let now = glowkit_core::time(ui);
        let accent = self.accent.unwrap_or(t.info);
        let base_id = ui.next_auto_id().with("agent_tree");
        let collapsed: Vec<String> = glowkit_core::state(ui, base_id.with("collapsed"), Vec::new);
        let root = self.root;
        let kids = &root.children;
        let avail = ui.available_width().min(self.width);
        let gap = 14.0;
        let n = kids.len().max(1) as f32;
        let cw = ((avail - 28.0 - gap * (n - 1.0)) / n).min(self.card_w);
        // fold amounts (animated)
        let open = |key: &str| -> f32 {
            let target = if collapsed.iter().any(|k| k == key) { 0.0 } else { 1.0 };
            glowkit_core::anim::spring(ui, base_id.with(("open", key)), target, 260.0, 26.0)
        };
        let root_open = open("r");
        // measure: deepest column height
        let mut col_h: Vec<f32> = Vec::new();
        let mut col_open: Vec<f32> = Vec::new();
        for (i, k) in kids.iter().enumerate() {
            let o = open(&format!("r/{i}"));
            col_open.push(o);
            let below = stack_count(k) as f32 * (SUB_H + 10.0);
            col_h.push(CARD_H + below * o);
        }
        let deep = col_h.iter().cloned().fold(0.0, f32::max);
        let header = 46.0;
        let level_gap = 46.0;
        let body_h = CARD_H + (level_gap + deep) * root_open;
        let total_h = header + body_h + 30.0;
        let (rect, resp) = alloc_centered(ui, avail, total_h, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let p = ui.painter_at(rect.expand(12.0));
        // header
        let hx = rect.min.x + 14.0;
        let hy = rect.min.y + 18.0;
        icon(&p, Icon::Sparkle, pos2(hx + 6.0, hy), 14.0, accent);
        text(&p, pos2(hx + 20.0, hy), Align2::LEFT_CENTER, &self.title, font(14.0), t.foreground, 200.0);
        let running = root.count_state(RunState::Running);
        let needs = root.count_state(RunState::NeedsInput);
        let total = fmt_usd(root.total_cost());
        let mut x = rect.max.x - 14.0;
        let r = text(&p, pos2(x, hy), Align2::RIGHT_CENTER, &total, mono(12.5), t.foreground, 120.0);
        x = r.min.x - 14.0;
        if needs > 0 {
            let r = badge(&p, &t, pos2(x, hy), Align2::RIGHT_CENTER, &format!("{needs} need you"), t.warning, true);
            x = r.min.x - 8.0;
        }
        let r = text(&p, pos2(x, hy), Align2::RIGHT_CENTER, &format!("{running} running"), font(12.0), ink(&t, accent), 120.0);
        text(&p, pos2(r.min.x - 10.0, hy), Align2::RIGHT_CENTER, &format!("{} agents", root.count()), font(12.0), dim(&t), 120.0);

        // place cards
        let top = rect.min.y + header;
        let rw = cw.max(200.0).min(avail - 28.0);
        let root_rect = Rect::from_center_size(pos2(rect.center().x, top + CARD_H / 2.0), vec2(rw, CARD_H));
        let mut placed: Vec<Placed> =
            vec![Placed { node: root, rect: root_rect, key: "r".into(), depth: 0, color: 0, alpha: 1.0 }];
        let row_w = cw * n + gap * (n - 1.0);
        let x0 = rect.center().x - row_w / 2.0;
        let y1 = top + CARD_H + level_gap * root_open;
        let mut edges: Vec<(Vec<Pos>, bool, f32)> = Vec::new();
        for (i, k) in kids.iter().enumerate() {
            let a = ease(Easing::OutCubic, ((root_open - 0.15) / 0.85).clamp(0.0, 1.0));
            if a <= 0.01 {
                break;
            }
            let r = Rect::from_min_size(pos2(x0 + i as f32 * (cw + gap), y1), vec2(cw, CARD_H));
            let key = format!("r/{i}");
            // connector from root
            let bus = top + CARD_H + level_gap * root_open * 0.5;
            let pts = rounded_path(
                &[
                    pos2(root_rect.center().x, root_rect.max.y),
                    pos2(root_rect.center().x, bus),
                    pos2(r.center().x, bus),
                    pos2(r.center().x, r.min.y),
                ],
                9.0,
            );
            edges.push((pts, k.state == RunState::Running, a));
            placed.push(Placed { node: k, rect: r, key: key.clone(), depth: 1, color: i + 1, alpha: a });
            // stacked descendants
            let o = col_open[i] * a;
            let mut y = r.max.y + 10.0;
            let mut stack: Vec<(&AgentNode, usize, Rect)> = Vec::new();
            flatten(k, 1, &mut stack);
            for (j, (node, depth, _)) in stack.iter().enumerate() {
                if o <= 0.02 {
                    break;
                }
                let indent = 16.0 * (*depth as f32 - 1.0) + 16.0;
                let sr = Rect::from_min_size(pos2(r.min.x + indent, y), vec2(cw - indent, SUB_H));
                let parent_x = r.min.x + indent - 16.0 + 12.0;
                let from_y = if j == 0 { r.max.y } else { y - 10.0 - SUB_H * 0.5 };
                let pts = rounded_path(
                    &[pos2(parent_x, from_y), pos2(parent_x, sr.center().y), pos2(sr.min.x, sr.center().y)],
                    8.0,
                );
                edges.push((pts, node.state == RunState::Running, o));
                placed.push(Placed {
                    node,
                    rect: sr,
                    key: format!("{key}/s{j}"),
                    depth: *depth + 1,
                    color: i + 1,
                    alpha: o,
                });
                y += (SUB_H + 10.0) * o;
            }
        }
        // edges
        for (i, (pts, live, a)) in edges.iter().enumerate() {
            let c = if *live { mix(hair(&t), accent, 0.55) } else { mix(hair(&t), t.muted_foreground, 0.25) };
            p.add(egui::Shape::line(pts.clone(), Stroke::new(1.4, with_alpha(c, *a))));
            if self.flow && *live && *a > 0.5 {
                let head = ((now * 0.55 + i as f32 * 0.37) % 1.4) / 1.2;
                comet(&p, pts, head, 0.35, accent, 2.0);
                glowkit_core::animate(ui);
            }
            if let Some(end) = pts.last() {
                p.circle_filled(*end, 2.6, with_alpha(c, *a));
            }
        }
        // cards
        let mut toggled: Option<String> = None;
        for pl in &placed {
            let id = base_id.with(("card", &pl.key));
            let n = pl.node;
            let has_kids = !n.children.is_empty() && pl.depth <= 1;
            let folded = collapsed.contains(&pl.key);
            if let Some(k) = card(ui, &p, &t, pl, id, now, accent, has_kids, folded) {
                toggled = Some(k);
            }
        }
        if root.state.is_active() || root.count_state(RunState::Running) > 0 {
            glowkit_core::animate(ui);
        }
        if let Some(k) = toggled {
            glowkit_core::with_state(ui, base_id.with("collapsed"), Vec::<String>::new, |v| {
                if let Some(i) = v.iter().position(|x| *x == k) {
                    v.remove(i);
                } else {
                    v.push(k);
                }
            });
        }
        resp
    }
}

type Pos = egui::Pos2;

fn stack_count(n: &AgentNode) -> usize {
    n.children.iter().map(|c| 1 + stack_count(c)).sum()
}

fn flatten<'n>(n: &'n AgentNode, depth: usize, out: &mut Vec<(&'n AgentNode, usize, Rect)>) {
    for c in &n.children {
        out.push((c, depth, Rect::NOTHING));
        flatten(c, depth + 1, out);
    }
}

#[allow(clippy::too_many_arguments)]
fn card(
    ui: &Ui,
    p: &egui::Painter,
    t: &Theme,
    pl: &Placed,
    id: egui::Id,
    now: f32,
    accent: Color32,
    has_kids: bool,
    folded: bool,
) -> Option<String> {
    let n = pl.node;
    let r = pl.rect;
    let a = pl.alpha.clamp(0.0, 1.0);
    if a < 0.03 {
        return None;
    }
    let pa = faded(p, a);
    let hov = ui.rect_contains_pointer(r);
    let h = glowkit_core::anim::follow(ui, id.with("h"), if hov { 1.0 } else { 0.0 }, 12.0);
    let lift = Rect::from_min_size(r.min - vec2(0.0, h * 1.5), r.size());
    let fill = surface(t);
    panel_fill(&pa, t, lift, 12.0, fill, 0.6 + h * 0.8);
    if n.state == RunState::Running {
        let k = 0.5 + 0.5 * (now * 2.2).sin();
        pa.rect_stroke(lift, cr(12.0), Stroke::new(1.0, with_alpha(accent, 0.18 + 0.2 * k)), egui::StrokeKind::Inside);
    } else if n.state == RunState::NeedsInput {
        pa.rect_stroke(lift, cr(12.0), Stroke::new(1.2, with_alpha(t.warning, 0.6)), egui::StrokeKind::Inside);
    }
    let compact = pl.depth >= 2;
    let pad = 11.0;
    let col = series(t, pl.color);
    let av = if compact { 22.0 } else { 26.0 };
    let ac = pos2(lift.min.x + pad + av / 2.0, lift.min.y + pad + av / 2.0);
    avatar(&pa, t, ac, av, &initials(&n.name), col);
    let tx = ac.x + av / 2.0 + 8.0;
    let badge_r = state_badge(&pa, t, pos2(lift.max.x - pad, ac.y), Align2::RIGHT_CENTER, n.state, now);
    let name_w = badge_r.min.x - tx - 6.0;
    text(&pa, pos2(tx, ac.y - 6.5), Align2::LEFT_CENTER, &n.name, font(13.0), t.foreground, name_w);
    text(&pa, pos2(tx, ac.y + 7.5), Align2::LEFT_CENTER, &n.model, font(10.5), faint(t), name_w);
    let task_y = lift.min.y + pad + av + 7.0;
    let rows = if compact { 1 } else { 2 };
    para(&pa, pos2(lift.min.x + pad, task_y), &n.task, font(11.5), dim(t), lift.width() - pad * 2.0, rows);
    // footer
    let fy = lift.max.y - pad - 5.0;
    let shown_cost = if folded { n.total_cost() } else { n.cost };
    let cost_r = text(&pa, pos2(lift.min.x + pad, fy), Align2::LEFT_CENTER, &fmt_usd(shown_cost), mono(11.0), t.foreground, 90.0);
    text(&pa, pos2(cost_r.max.x + 6.0, fy), Align2::LEFT_CENTER, &format!("· {}", fmt_secs(n.secs)), font(11.0), faint(t), 80.0);
    let mut toggled = None;
    if has_kids {
        let label = format!("{}", n.count() - 1);
        let br = Rect::from_center_size(pos2(lift.max.x - pad - 16.0, fy), vec2(36.0, 20.0));
        let resp = ui.interact(br, id.with("fold"), Sense::click());
        if resp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            pa.rect_filled(br, cr(6.0), well(t));
        }
        pa.text(pos2(br.min.x + 9.0, fy), Align2::LEFT_CENTER, &label, font(11.0), dim(t));
        let ic = if folded { Icon::ChevronRight } else { Icon::ChevronDown };
        icon(&pa, ic, pos2(br.max.x - 10.0, fy), 11.0, dim(t));
        if resp.clicked() {
            toggled = Some(pl.key.clone());
        }
    }
    // progress hairline for running agents
    if n.state == RunState::Running {
        let w = lift.width() - 24.0;
        let x = lift.min.x + 12.0;
        let u = (now * 0.45 + pl.color as f32 * 0.21).fract();
        let seg_w = w * 0.3;
        let s0 = x + (w + seg_w) * u - seg_w;
        let a0 = s0.max(x);
        let a1 = (s0 + seg_w).min(x + w);
        if a1 > a0 {
            pa.line_segment([pos2(a0, lift.max.y - 1.0), pos2(a1, lift.max.y - 1.0)], Stroke::new(1.5, with_alpha(accent, 0.7)));
        }
    }
    toggled
}

// ---------------------------------------------------------------------------------------------
// HandoffCard

/// One piece of context in a [`HandoffCard`].
#[derive(Clone, Debug)]
pub struct HandoffItem {
    /// Label, e.g. `"pricing.csv"` or `"Brand voice notes"`.
    pub label: String,
    /// Size in tokens.
    pub tokens: f32,
    /// False when the item did not make it to the receiving agent.
    pub kept: bool,
    /// Icon shown before the label.
    pub icon: Icon,
}

impl HandoffItem {
    /// A kept item.
    pub fn new(label: impl Into<String>, tokens: f32) -> Self {
        Self { label: label.into(), tokens, kept: true, icon: Icon::File }
    }
    /// Mark it as dropped (shown struck through).
    pub fn dropped(mut self) -> Self {
        self.kept = false;
        self
    }
    /// Icon shown before the label.
    pub fn icon(mut self, i: Icon) -> Self {
        self.icon = i;
        self
    }
}

/// A relay hand-off between two agents. The sender and receiver sit at either end of a curved
/// track; a baton packet travels across as `progress` goes from 0 to 1, the receiver lights up
/// on arrival, and the context that went along drops in below as chips with token sizes, while
/// anything that got lost on the way is struck through.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{HandoffCard, HandoffItem};
/// ui.add(HandoffCard::new(("Researcher", "Sonnet 4.5"), ("Writer", "Haiku 4.5"))
///     .reason("Research done, 5 competitors compared")
///     .item(HandoffItem::new("competitors.md", 2400.0))
///     .item(HandoffItem::new("raw HTML", 18000.0).dropped())
///     .progress(0.6));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct HandoffCard {
    from: (String, String),
    to: (String, String),
    reason: String,
    items: Vec<HandoffItem>,
    progress: Option<f32>,
    width: f32,
    accent: Option<Color32>,
}

impl HandoffCard {
    /// A hand-off from `(name, model)` to `(name, model)`.
    pub fn new(from: (&str, &str), to: (&str, &str)) -> Self {
        Self {
            from: (from.0.into(), from.1.into()),
            to: (to.0.into(), to.1.into()),
            reason: String::new(),
            items: Vec::new(),
            progress: None,
            width: 520.0,
            accent: None,
        }
    }
    /// The note the sender left with the hand-off.
    pub fn reason(mut self, s: impl Into<String>) -> Self {
        self.reason = s.into();
        self
    }
    /// Add a context item.
    pub fn item(mut self, i: HandoffItem) -> Self {
        self.items.push(i);
        self
    }
    /// Transfer progress 0..1 (default: loops by itself).
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = Some(p.clamp(0.0, 1.0));
        self
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(300.0);
        self
    }
    /// Baton / track color (default: theme accent).
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = Some(c);
        self
    }
}

impl egui::Widget for HandoffCard {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let now = glowkit_core::time(ui);
        let accent = self.accent.unwrap_or(t.accent);
        let rows = self.items.len().div_ceil(2) as f32;
        let h = 150.0 + rows * 30.0 + if self.reason.is_empty() { 0.0 } else { 34.0 };
        let (rect, resp) = alloc_centered(ui, self.width, h, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let prog = match self.progress {
            Some(v) => v,
            None => {
                glowkit_core::animate(ui);
                ((now % 6.0) / 3.2).min(1.0)
            }
        };
        let p = ui.painter_at(rect.expand(16.0));
        panel(&p, &t, rect, 14.0);
        let pad = 16.0;
        // header line
        text(&p, pos2(rect.min.x + pad, rect.min.y + 20.0), Align2::LEFT_CENTER, "Hand-off", font(13.0), t.foreground, 200.0);
        let done = prog >= 1.0;
        let st = if done { RunState::Done } else { RunState::Running };
        let label = if done { "Received" } else { "Passing" };
        let c = st.color(&t);
        let br = badge(&p, &t, pos2(rect.max.x - pad, rect.min.y + 20.0), Align2::RIGHT_CENTER, label, c, true);
        let kept_tok: f32 = self.items.iter().filter(|i| i.kept).map(|i| i.tokens).sum();
        text(
            &p,
            pos2(br.min.x - 10.0, rect.min.y + 20.0),
            Align2::RIGHT_CENTER,
            &format!("{} tokens carried", fmt_tok(kept_tok)),
            font(11.5),
            dim(&t),
            200.0,
        );
        // agents and track
        let y = rect.min.y + 74.0;
        let a = pos2(rect.min.x + pad + 20.0, y);
        let b = pos2(rect.max.x - pad - 20.0, y);
        let mid = pos2((a.x + b.x) / 2.0, y - 26.0);
        let track_pts = paint::quad_bezier(pos2(a.x + 26.0, y), mid, pos2(b.x - 26.0, y), 48);
        // dashed base track
        for (i, w) in track_pts.windows(2).enumerate() {
            if (i / 2) % 2 == 0 {
                p.line_segment([w[0], w[1]], Stroke::new(1.3, hair(&t)));
            }
        }
        let e = ease(Easing::InOutCubic, prog);
        let sent = path_slice(&track_pts, 0.0, e);
        if sent.len() > 1 {
            paint::gradient_polyline(&p, &sent, false, 2.0, |u| with_alpha(accent, 0.25 + 0.6 * u));
        }
        if !done {
            let bp = path_at(&track_pts, e);
            paint::glow(&p, bp, 16.0, with_alpha(accent, 0.3));
            let r = Rect::from_center_size(bp, vec2(18.0, 12.0));
            p.rect_filled(r, cr(4.0), accent);
            icon(&p, Icon::File, bp, 9.0, t.accent_foreground);
        }
        let arrive = ((prog - 0.97) / 0.03).clamp(0.0, 1.0);
        for (k, (pos, (name, model))) in [(a, &self.from), (b, &self.to)].into_iter().enumerate() {
            let col = series(&t, k * 2 + 1);
            if k == 1 && arrive > 0.0 {
                let ring = ((now * 1.3).fract()) * arrive;
                p.circle_stroke(pos, 20.0 + ring * 10.0, Stroke::new(1.5, with_alpha(col, 0.4 * (1.0 - ring))));
            }
            avatar(&p, &t, pos, 34.0, &initials(name), col);
            let align = if k == 0 { Align2::LEFT_TOP } else { Align2::RIGHT_TOP };
            let tx = if k == 0 { pos.x - 17.0 } else { pos.x + 17.0 };
            text(&p, pos2(tx, pos.y + 24.0), align, name, font(12.5), t.foreground, 140.0);
            text(&p, pos2(tx, pos.y + 40.0), align, model, font(10.5), faint(&t), 140.0);
        }
        // reason
        let mut yy = rect.min.y + 136.0;
        if !self.reason.is_empty() {
            let qr = Rect::from_min_size(pos2(rect.min.x + pad, yy), vec2(rect.width() - pad * 2.0, 26.0));
            p.rect_filled(qr, cr(8.0), well(&t));
            p.line_segment([pos2(qr.min.x + 1.0, qr.min.y + 5.0), pos2(qr.min.x + 1.0, qr.max.y - 5.0)], Stroke::new(2.0, with_alpha(accent, 0.6)));
            text(&p, pos2(qr.min.x + 12.0, qr.center().y), Align2::LEFT_CENTER, &self.reason, font(11.5), dim(&t), qr.width() - 20.0);
            yy += 34.0;
        }
        // items
        let colw = (rect.width() - pad * 2.0 - 8.0) / 2.0;
        for (i, it) in self.items.iter().enumerate() {
            let appear = ease(Easing::OutBack, ((prog * 1.1 - 0.15 - i as f32 * 0.08) / 0.3).clamp(0.0, 1.0));
            if appear <= 0.0 {
                continue;
            }
            let cx = rect.min.x + pad + (i % 2) as f32 * (colw + 8.0);
            let cy = yy + (i / 2) as f32 * 30.0 + (1.0 - appear) * 8.0;
            let r = Rect::from_min_size(pos2(cx, cy), vec2(colw, 24.0));
            let pa = faded(&p, appear.min(1.0));
            let fg = if it.kept { t.foreground } else { faint(&t) };
            pa.rect(r, cr(7.0), if it.kept { surface(&t) } else { well(&t) }, Stroke::new(1.0, hair(&t)), egui::StrokeKind::Inside);
            icon(&pa, it.icon, pos2(r.min.x + 13.0, r.center().y), 12.0, if it.kept { dim(&t) } else { faint(&t) });
            let tr = text(&pa, pos2(r.min.x + 25.0, r.center().y), Align2::LEFT_CENTER, &it.label, font(11.5), fg, r.width() - 90.0);
            if !it.kept {
                pa.line_segment([pos2(tr.min.x, tr.center().y), pos2(tr.max.x, tr.center().y)], Stroke::new(1.0, faint(&t)));
                text(&pa, pos2(r.max.x - 8.0, r.center().y), Align2::RIGHT_CENTER, "dropped", font(10.5), ink(&t, t.destructive), 60.0);
            } else {
                text(&pa, pos2(r.max.x - 8.0, r.center().y), Align2::RIGHT_CENTER, &fmt_tok(it.tokens), mono(10.5), dim(&t), 60.0);
            }
        }
        if prog < 1.0 {
            glowkit_core::animate(ui);
        }
        resp
    }
}
