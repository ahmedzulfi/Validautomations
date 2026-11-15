//! Human in the loop: [`ApprovalQueue`] (pending tool calls to approve, deny or edit),
//! [`HumanInTheLoopPrompt`] (an agent asks a question) and [`ErrorRecoveryCard`] (a failed step
//! waiting for retry or skip).

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, Key, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder};
use glowkit_ai::Icon;
use glowkit_core::anim::{ease, Easing};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// How dangerous a requested action is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub enum Risk {
    /// Read-only or easily undone.
    #[default]
    Low,
    /// Changes something, but recoverable.
    Medium,
    /// Destructive, external or expensive.
    High,
}

impl Risk {
    /// Label, e.g. `"High risk"`.
    pub fn label(self) -> &'static str {
        match self {
            Risk::Low => "Low risk",
            Risk::Medium => "Medium",
            Risk::High => "High risk",
        }
    }
    /// Color in theme `t`.
    pub fn color(self, t: &Theme) -> Color32 {
        match self {
            Risk::Low => t.success,
            Risk::Medium => t.warning,
            Risk::High => t.destructive,
        }
    }
}

/// A person's decision on an [`ApprovalRequest`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Decision {
    /// Allowed to run.
    Approved,
    /// Not allowed.
    Denied,
}

/// A tool call an agent wants to make, waiting for a person.
#[derive(Clone, Debug, Default)]
pub struct ApprovalRequest {
    /// Stable id (used for animation state).
    pub id: u64,
    /// Requesting agent.
    pub agent: String,
    /// Tool name, e.g. `"shell"`.
    pub tool: String,
    /// The exact command or arguments (editable before approving).
    pub command: String,
    /// Why the agent wants it.
    pub reason: String,
    /// Risk tier.
    pub risk: Risk,
    /// Seconds it has been waiting.
    pub waiting: f32,
    /// The decision, once made.
    pub decision: Option<Decision>,
    /// Ticked for a batch action.
    pub selected: bool,
    /// True once a person edited the command.
    pub edited: bool,
}

impl ApprovalRequest {
    /// A pending request `id` from `agent` to run `tool` with `command`.
    pub fn new(id: u64, agent: impl Into<String>, tool: impl Into<String>, command: impl Into<String>) -> Self {
        Self { id, agent: agent.into(), tool: tool.into(), command: command.into(), ..Default::default() }
    }
    /// Why the agent wants it.
    pub fn reason(mut self, s: impl Into<String>) -> Self {
        self.reason = s.into();
        self
    }
    /// Risk tier.
    pub fn risk(mut self, r: Risk) -> Self {
        self.risk = r;
        self
    }
    /// Seconds waiting so far.
    pub fn waiting(mut self, s: f32) -> Self {
        self.waiting = s;
        self
    }
}

#[derive(Clone, Default)]
struct QueueState {
    decided_at: Vec<(u64, f32)>,
    editing: Option<(u64, String)>,
}

/// An inbox of pending tool calls. Each row shows a risk badge, the requesting agent, the tool
/// and the exact command, why it is wanted and how long it has waited, with Edit, Deny and
/// Approve. Tick rows for a batch Approve / Deny, or approve every low-risk request at once.
/// Editing opens the command inline; decided rows morph into a resolved line and fold away.
///
/// The queue edits the `Vec` you pass: it sets `decision`, `selected`, `command` and `edited`.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{ApprovalQueue, ApprovalRequest, Risk};
/// let mut queue = vec![
///     ApprovalRequest::new(1, "Deploy Fixer", "shell", "pnpm add zod@3").risk(Risk::Medium),
///     ApprovalRequest::new(2, "Refund Resolver", "stripe.refunds.create", "amount=1240").risk(Risk::High),
/// ];
/// ui.add(ApprovalQueue::new(&mut queue));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ApprovalQueue<'a> {
    items: &'a mut Vec<ApprovalRequest>,
    width: f32,
    title: String,
}

impl<'a> ApprovalQueue<'a> {
    /// A queue over `items`.
    pub fn new(items: &'a mut Vec<ApprovalRequest>) -> Self {
        Self { items, width: 760.0, title: "Approvals".into() }
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(380.0);
        self
    }
    /// Header title.
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
}

const ROW_H: f32 = 64.0;
const EDIT_H: f32 = 46.0;

impl egui::Widget for ApprovalQueue<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let now = glowkit_core::time(ui);
        let id = ui.next_auto_id().with("approval_queue");
        let mut st: QueueState = glowkit_core::state(ui, id.with("st"), QueueState::default);
        // record decision times
        for it in self.items.iter() {
            if it.decision.is_some() && !st.decided_at.iter().any(|(k, _)| *k == it.id) {
                st.decided_at.push((it.id, now));
            }
        }
        let decided_age = |k: u64| st.decided_at.iter().find(|(x, _)| *x == k).map(|(_, at)| now - at);
        let row_h = |it: &ApprovalRequest, editing: bool| -> f32 {
            match decided_age(it.id) {
                Some(a) => {
                    let shrink = ease(Easing::InOutCubic, ((a - 1.1) / 0.45).clamp(0.0, 1.0));
                    let morph = ease(Easing::OutCubic, (a / 0.35).clamp(0.0, 1.0));
                    (ROW_H + (42.0 - ROW_H) * morph) * (1.0 - shrink)
                }
                None => ROW_H + if editing { EDIT_H } else { 0.0 },
            }
        };
        let editing_id = st.editing.as_ref().map(|e| e.0);
        let header = 58.0;
        let footer = 50.0;
        let body: f32 = self.items.iter().map(|it| row_h(it, editing_id == Some(it.id))).sum();
        let pending: Vec<usize> = (0..self.items.len()).filter(|i| self.items[*i].decision.is_none()).collect();
        let empty_h = if pending.is_empty() { 70.0 } else { 0.0 };
        let target_h = header + body + footer + empty_h;
        let h = glowkit_core::anim::spring(ui, id.with("h"), target_h, 260.0, 30.0);
        let (rect, resp) = alloc_centered(ui, self.width, h, Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        panel(&p, &t, rect, 14.0);
        let pad = 16.0;
        let hy = rect.min.y + 28.0;
        text(&p, pos2(rect.min.x + pad, hy - 8.0), Align2::LEFT_CENTER, &self.title, font(14.0), t.foreground, 200.0);
        let high = pending.iter().filter(|i| self.items[**i].risk == Risk::High).count();
        let sub = format!("{} pending · {} high risk", pending.len(), high);
        text(&p, pos2(rect.min.x + pad, hy + 9.0), Align2::LEFT_CENTER, &sub, font(11.5), dim(&t), 260.0);
        // batch actions
        let selected: Vec<usize> = pending.iter().copied().filter(|i| self.items[*i].selected).collect();
        let mut x = rect.max.x - pad;
        let mut batch: Option<(Vec<usize>, Decision)> = None;
        if selected.is_empty() {
            let low: Vec<usize> = pending.iter().copied().filter(|i| self.items[*i].risk == Risk::Low).collect();
            let label = format!("Approve all low risk ({})", low.len());
            let w = button_w(&p, &label, true);
            let b = Rect::from_min_size(pos2(x - w, hy - 14.0), vec2(w, 28.0));
            if !low.is_empty() && button(ui, b, id.with("lowall"), &label, Some(Icon::Check), Btn::Secondary).clicked() {
                batch = Some((low, Decision::Approved));
            }
        } else {
            let label = format!("Approve {}", selected.len());
            let w = button_w(&p, &label, true);
            let b = Rect::from_min_size(pos2(x - w, hy - 14.0), vec2(w, 28.0));
            if button(ui, b, id.with("bappr"), &label, Some(Icon::Check), Btn::Primary).clicked() {
                batch = Some((selected.clone(), Decision::Approved));
            }
            x = b.min.x - 8.0;
            let w = button_w(&p, "Deny", true);
            let b = Rect::from_min_size(pos2(x - w, hy - 14.0), vec2(w, 28.0));
            if button(ui, b, id.with("bdeny"), "Deny", Some(Icon::X), Btn::Secondary).clicked() {
                batch = Some((selected.clone(), Decision::Denied));
            }
            text(&p, pos2(b.min.x - 10.0, hy), Align2::RIGHT_CENTER, &format!("{} selected", selected.len()), font(11.5), dim(&t), 100.0);
        }
        hline(&p, &t, rect.min.x, rect.max.x, rect.min.y + header);
        let clip = Rect::from_min_max(pos2(rect.min.x, rect.min.y + header), pos2(rect.max.x, rect.max.y - footer));
        let pc = p.with_clip_rect(clip.intersect(p.clip_rect()));
        let mut y = rect.min.y + header;
        let mut decide: Vec<(usize, Decision)> = Vec::new();
        let mut start_edit: Option<(u64, String)> = None;
        let mut stop_edit = false;
        let mut saved_cmd: Option<(usize, String)> = None;
        let mut flip_sel: Option<usize> = None;
        let n_items = self.items.len();
        for i in 0..n_items {
            let it = &self.items[i];
            let editing = editing_id == Some(it.id);
            let rh = row_h(it, editing);
            if rh < 0.5 {
                continue;
            }
            let rr = Rect::from_min_size(pos2(rect.min.x, y), vec2(rect.width(), rh));
            y += rh;
            if let Some(age) = decided_age(it.id) {
                // resolved line
                let a = (1.0 - ((age - 1.0) / 0.4).clamp(0.0, 1.0)).max(0.0);
                let pr = faded(&pc, a);
                let ok = it.decision == Some(Decision::Approved);
                let c = if ok { t.success } else { t.destructive };
                let fl = ease(Easing::OutCubic, (age / 0.5).clamp(0.0, 1.0));
                pr.rect_filled(rr.shrink2(vec2(8.0, 3.0)), cr(9.0), with_alpha(tint(&t, c), 1.0 - 0.5 * fl));
                let cy = rr.center().y;
                state_icon(&pr, &t, pos2(rr.min.x + pad + 8.0, cy), 8.0, if ok { RunState::Done } else { RunState::Failed }, now);
                let verb = if ok { if it.edited { "Approved with edits" } else { "Approved" } } else { "Denied" };
                let vr = text(&pr, pos2(rr.min.x + pad + 24.0, cy), Align2::LEFT_CENTER, verb, font(12.5), ink(&t, c), 160.0);
                text(&pr, pos2(vr.max.x + 10.0, cy), Align2::LEFT_CENTER, &it.command, mono(11.5), dim(&t), rr.width() - vr.max.x + rr.min.x - 120.0);
                text(&pr, pos2(rr.max.x - pad, cy), Align2::RIGHT_CENTER, &it.agent, font(11.5), faint(&t), 140.0);
                hline(&pc, &t, rr.min.x, rr.max.x, rr.max.y);
                glowkit_core::animate(ui);
                continue;
            }
            let hov = ui.rect_contains_pointer(rr) && clip.contains(ui.ctx().pointer_hover_pos().unwrap_or_default());
            if hov || it.selected {
                pc.rect_filled(rr, cr(0.0), with_alpha(t.foreground, if it.selected { 0.04 } else { 0.022 }));
            }
            let cy = rr.min.y + ROW_H / 2.0;
            // checkbox
            let cb = Rect::from_center_size(pos2(rr.min.x + pad + 8.0, cy), vec2(16.0, 16.0));
            let cresp = ui.interact(cb.expand(4.0), id.with(("sel", it.id)), Sense::click());
            if cresp.clicked() {
                flip_sel = Some(i);
            }
            if cresp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            let k = glowkit_core::anim::spring(ui, id.with(("selk", it.id)), if it.selected { 1.0 } else { 0.0 }, 400.0, 24.0);
            pc.rect(cb, cr(4.5), mix(surface(&t), t.foreground, k.clamp(0.0, 1.0)), Stroke::new(1.0, mix(hair(&t), t.foreground, 0.3 + 0.7 * k.clamp(0.0, 1.0))), StrokeKind::Inside);
            if k > 0.05 {
                glowkit_ai::paint_icon_width(&pc, Icon::Check, Rect::from_center_size(cb.center(), vec2(12.0, 12.0) * k.clamp(0.0, 1.2)), surface(&t), 2.0);
            }
            // risk + agent + tool
            let x0 = cb.max.x + 14.0;
            let rb = badge(&pc, &t, pos2(x0, cy - 10.0), Align2::LEFT_CENTER, it.risk.label(), it.risk.color(&t), true);
            let ar = text(&pc, pos2(rb.max.x + 8.0, cy - 10.0), Align2::LEFT_CENTER, &it.agent, font(12.5), t.foreground, 160.0);
            let tc = chip(&pc, &t, pos2(ar.max.x + 8.0, cy - 10.0), Align2::LEFT_CENTER, &it.tool, true);
            let actions_w = 210.0;
            let wait = format!("waiting {}", fmt_secs(it.waiting));
            let wc = if it.waiting > 300.0 { ink(&t, t.warning) } else { faint(&t) };
            text(&pc, pos2(tc.max.x + 10.0, cy - 10.0), Align2::LEFT_CENTER, &wait, font(11.0), wc, (rr.max.x - actions_w - tc.max.x - 20.0).max(10.0));
            // command line
            let cmd_w = rr.max.x - actions_w - x0 - 10.0;
            let cr_ = Rect::from_min_size(pos2(x0, cy + 3.0), vec2(cmd_w, 20.0));
            pc.rect_filled(cr_, cr(5.0), well(&t));
            pc.text(pos2(cr_.min.x + 7.0, cr_.center().y), Align2::LEFT_CENTER, "$", mono(11.0), faint(&t));
            text(&pc, pos2(cr_.min.x + 18.0, cr_.center().y), Align2::LEFT_CENTER, &it.command, mono(11.0), t.foreground, cr_.width() - 24.0);
            if !it.reason.is_empty() && hov {
                // reason as a soft tooltip line under the command when hovered
                let lp = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, id.with("why")));
                let g = galley1(&lp, &it.reason, font(11.5), t.foreground, 360.0);
                let tr = Rect::from_min_size(pos2(cr_.min.x, cr_.max.y + 6.0), g.size() + vec2(20.0, 12.0));
                panel_fill(&lp, &t, tr, 8.0, if t.dark { t.popover } else { t.card }, 1.2);
                lp.galley(tr.min + vec2(10.0, 6.0), g, t.foreground);
            }
            // actions
            let bx = rr.max.x - pad;
            let ab = Rect::from_min_size(pos2(bx - 86.0, cy - 14.0), vec2(86.0, 28.0));
            if button(ui, ab, id.with(("ok", it.id)), "Approve", Some(Icon::Check), Btn::Primary).clicked() {
                decide.push((i, Decision::Approved));
            }
            let db = Rect::from_min_size(pos2(ab.min.x - 70.0, cy - 14.0), vec2(64.0, 28.0));
            if button(ui, db, id.with(("no", it.id)), "Deny", None, Btn::Secondary).clicked() {
                decide.push((i, Decision::Denied));
            }
            let eb = Rect::from_min_size(pos2(db.min.x - 34.0, cy - 14.0), vec2(28.0, 28.0));
            if button(ui, eb, id.with(("ed", it.id)), "", Some(Icon::Pencil), Btn::Ghost).clicked() {
                if editing {
                    stop_edit = true;
                } else {
                    start_edit = Some((it.id, it.command.clone()));
                }
            }
            if editing {
                let er = Rect::from_min_size(pos2(x0, rr.min.y + ROW_H - 4.0), vec2(rr.max.x - pad - x0, EDIT_H - 8.0));
                let mut buf = st.editing.as_ref().map(|e| e.1.clone()).unwrap_or_default();
                let save = Rect::from_min_size(pos2(er.max.x - 128.0, er.min.y + 4.0), vec2(128.0, 30.0));
                let field = Rect::from_min_max(pos2(er.min.x, er.min.y + 4.0), pos2(save.min.x - 8.0, er.min.y + 34.0));
                pc.rect(field, cr(7.0), surface(&t), Stroke::new(1.0, t.ring), StrokeKind::Inside);
                let mut child = ui.new_child(UiBuilder::new().max_rect(field.shrink2(vec2(8.0, 6.0))));
                let te = egui::TextEdit::singleline(&mut buf)
                    .frame(egui::Frame::NONE)
                    .font(mono(11.5))
                    .desired_width(field.width() - 16.0)
                    .id(id.with(("te", it.id)));
                let r = child.add(te);
                if !r.has_focus() && !r.lost_focus() {
                    r.request_focus();
                }
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                if button(ui, save, id.with(("save", it.id)), "Save & approve", None, Btn::Primary).clicked() || enter {
                    saved_cmd = Some((i, buf.clone()));
                }
                if let Some(e) = st.editing.as_mut() {
                    e.1 = buf;
                }
            }
            hline(&pc, &t, rr.min.x + pad, rr.max.x - pad, rr.max.y);
        }
        if pending.is_empty() {
            let cy = rect.max.y - footer - empty_h / 2.0;
            let pa = faded(&p, ((h - (header + footer)) / (empty_h + 1.0)).clamp(0.0, 1.0));
            icon(&pa, Icon::Check, pos2(rect.center().x, cy - 10.0), 18.0, t.success);
            pa.text(pos2(rect.center().x, cy + 12.0), Align2::CENTER_CENTER, "All caught up", font(12.5), dim(&t));
        }
        // footer: shortcuts hint
        let fy = rect.max.y - footer / 2.0;
        hline(&p, &t, rect.min.x, rect.max.x, rect.max.y - footer);
        let mut kx = rect.min.x + pad;
        for (key, what) in [("A", "approve top"), ("D", "deny top"), ("E", "edit")] {
            let kr = Rect::from_min_size(pos2(kx, fy - 10.0), vec2(20.0, 20.0));
            p.rect(kr, cr(5.0), well(&t), Stroke::new(1.0, hair(&t)), StrokeKind::Inside);
            p.text(kr.center(), Align2::CENTER_CENTER, key, mono(10.5), dim(&t));
            let r = text(&p, pos2(kr.max.x + 6.0, fy), Align2::LEFT_CENTER, what, font(11.0), faint(&t), 120.0);
            kx = r.max.x + 16.0;
        }
        let approved = self.items.iter().filter(|i| i.decision == Some(Decision::Approved)).count();
        let denied = self.items.iter().filter(|i| i.decision == Some(Decision::Denied)).count();
        text(&p, pos2(rect.max.x - pad, fy), Align2::RIGHT_CENTER, &format!("{approved} approved · {denied} denied today"), font(11.0), faint(&t), 260.0);
        // keyboard
        if st.editing.is_none() && resp.hovered() {
            if let Some(&top) = pending.first() {
                if ui.input(|i| i.key_pressed(Key::A)) {
                    decide.push((top, Decision::Approved));
                }
                if ui.input(|i| i.key_pressed(Key::D)) {
                    decide.push((top, Decision::Denied));
                }
                if ui.input(|i| i.key_pressed(Key::E)) {
                    start_edit = Some((self.items[top].id, self.items[top].command.clone()));
                }
            }
        }
        // apply
        if let Some(i) = flip_sel {
            self.items[i].selected = !self.items[i].selected;
        }
        for (i, d) in decide {
            self.items[i].decision = Some(d);
            self.items[i].selected = false;
        }
        if let Some((list, d)) = batch {
            for i in list {
                self.items[i].decision = Some(d);
                self.items[i].selected = false;
            }
        }
        if let Some((i, cmd)) = saved_cmd {
            if cmd != self.items[i].command {
                self.items[i].edited = true;
                self.items[i].command = cmd;
            }
            self.items[i].decision = Some(Decision::Approved);
            st.editing = None;
        }
        if stop_edit {
            st.editing = None;
        }
        if let Some(e) = start_edit {
            st.editing = Some(e);
        }
        // forget long-gone decisions whose items were removed
        let ids: Vec<u64> = self.items.iter().map(|i| i.id).collect();
        st.decided_at.retain(|(k, _)| ids.contains(k));
        glowkit_core::set_state(ui, id.with("st"), st);
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// HumanInTheLoopPrompt

/// The answer to a [`HumanInTheLoopPrompt`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HitlAnswer {
    /// One of the offered options (index).
    Choice(usize),
    /// A free-text answer.
    Custom(String),
}

/// An agent's question, asked in its own card: who is asking and what it is working on, the
/// question, numbered options with one marked Recommended, and a last row to type your own
/// answer. A click or the 1-9 keys answer; the card then morphs into a one-line receipt with an
/// Undo. Options light up as the pointer passes.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{HitlAnswer, HumanInTheLoopPrompt};
/// let mut answer: Option<HitlAnswer> = None;
/// ui.add(HumanInTheLoopPrompt::new(&mut answer, "Which region should I seed first?")
///     .agent("Data Loader", "Seed staging")
///     .option("us-east-1", "Closest to the API, 2.1 GB")
///     .option("eu-west-1", "GDPR tenants only")
///     .recommended(0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct HumanInTheLoopPrompt<'a> {
    answer: &'a mut Option<HitlAnswer>,
    question: String,
    agent: (String, String),
    options: Vec<(String, String)>,
    recommended: Option<usize>,
    width: f32,
    allow_custom: bool,
    accent: Option<Color32>,
}

impl<'a> HumanInTheLoopPrompt<'a> {
    /// A prompt asking `question`; the chosen answer is written to `answer`.
    pub fn new(answer: &'a mut Option<HitlAnswer>, question: impl Into<String>) -> Self {
        Self {
            answer,
            question: question.into(),
            agent: ("Agent".into(), String::new()),
            options: Vec::new(),
            recommended: None,
            width: 520.0,
            allow_custom: true,
            accent: None,
        }
    }
    /// Asking agent and the task it is on.
    pub fn agent(mut self, name: impl Into<String>, task: impl Into<String>) -> Self {
        self.agent = (name.into(), task.into());
        self
    }
    /// Add an option with a one-line description.
    pub fn option(mut self, label: impl Into<String>, desc: impl Into<String>) -> Self {
        self.options.push((label.into(), desc.into()));
        self
    }
    /// Mark option `i` as the agent's recommendation.
    pub fn recommended(mut self, i: usize) -> Self {
        self.recommended = Some(i);
        self
    }
    /// Offer a free-text row (default on).
    pub fn allow_custom(mut self, on: bool) -> Self {
        self.allow_custom = on;
        self
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(300.0);
        self
    }
    /// Highlight color (default: theme accent).
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = Some(c);
        self
    }
}

impl egui::Widget for HumanInTheLoopPrompt<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let now = glowkit_core::time(ui);
        let accent = self.accent.unwrap_or(t.accent);
        let id = ui.next_auto_id().with("hitl");
        let answered = self.answer.is_some();
        let m = glowkit_core::anim::spring(ui, id.with("m"), if answered { 1.0 } else { 0.0 }, 220.0, 26.0).clamp(0.0, 1.0);
        let opt_h = 46.0;
        let full_h = 96.0 + self.options.len() as f32 * (opt_h + 6.0) + if self.allow_custom { 46.0 } else { 0.0 } + 12.0;
        let small_h = 52.0;
        let h = full_h + (small_h - full_h) * m;
        let (rect, resp) = alloc_centered(ui, self.width, h, Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        panel(&p, &t, rect, 14.0);
        // left accent strip
        p.rect_filled(Rect::from_min_size(rect.min + vec2(0.0, 12.0), vec2(3.0, rect.height() - 24.0)), cr(2.0), with_alpha(if answered { t.success } else { t.warning }, 0.85));
        let pad = 18.0;
        let mut pick: Option<HitlAnswer> = None;
        // receipt
        if m > 0.02 {
            let pr = faded(&p, ((m - 0.4) / 0.6).clamp(0.0, 1.0));
            let cy = rect.min.y + 26.0;
            state_icon(&pr, &t, pos2(rect.min.x + pad + 8.0, cy), 9.0, RunState::Done, now);
            let label = match self.answer.as_ref() {
                Some(HitlAnswer::Choice(i)) => self.options.get(*i).map(|o| o.0.clone()).unwrap_or_default(),
                Some(HitlAnswer::Custom(s)) => format!("\u{201C}{s}\u{201D}"),
                None => String::new(),
            };
            let r = text(&pr, pos2(rect.min.x + pad + 24.0, cy), Align2::LEFT_CENTER, "You answered", font(12.0), dim(&t), 120.0);
            text(&pr, pos2(r.max.x + 6.0, cy), Align2::LEFT_CENTER, &label, font(12.5), t.foreground, rect.width() - 260.0);
            let ub = Rect::from_min_size(pos2(rect.max.x - pad - 64.0, cy - 13.0), vec2(64.0, 26.0));
            if answered && m > 0.8 && button(ui, ub, id.with("undo"), "Undo", Some(Icon::History), Btn::Ghost).clicked() {
                *self.answer = None;
            }
        }
        if m < 0.98 {
            let pa = faded(&p.with_clip_rect(rect), 1.0 - m);
            let top = rect.min.y + 16.0 - m * 30.0;
            avatar(&pa, &t, pos2(rect.min.x + pad + 13.0, top + 13.0), 26.0, &self.agent.0.chars().take(1).collect::<String>(), t.chart[2]);
            let nr = text(&pa, pos2(rect.min.x + pad + 34.0, top + 6.0), Align2::LEFT_CENTER, &self.agent.0, font(12.5), t.foreground, 200.0);
            text(&pa, pos2(rect.min.x + pad + 34.0, top + 21.0), Align2::LEFT_CENTER, &self.agent.1, font(11.0), faint(&t), 260.0);
            let _ = nr;
            badge(&pa, &t, pos2(rect.max.x - pad, top + 13.0), Align2::RIGHT_CENTER, "Question", t.warning, true);
            para(&pa, pos2(rect.min.x + pad, top + 40.0), &self.question, font(15.0), t.foreground, rect.width() - pad * 2.0, 2);
            let mut y = top + 80.0;
            for (i, (label, desc)) in self.options.iter().enumerate() {
                let r = Rect::from_min_size(pos2(rect.min.x + pad, y), vec2(rect.width() - pad * 2.0, opt_h));
                y += opt_h + 6.0;
                let hit = if answered { None } else { Some(ui.interact(r, id.with(("o", i)), Sense::click())) };
                let hov = hit.as_ref().is_some_and(|h| h.hovered());
                let k = glowkit_core::anim::follow(ui, id.with(("oh", i)), if hov { 1.0 } else { 0.0 }, 14.0);
                let rec = self.recommended == Some(i);
                let stroke = mix(hair(&t), accent, if rec { 0.35 + 0.4 * k } else { 0.6 * k });
                pa.rect(r, cr(10.0), mix(surface(&t), tint(&t, accent), k * 0.8), Stroke::new(1.0, stroke), StrokeKind::Inside);
                let kr = Rect::from_center_size(pos2(r.min.x + 20.0, r.center().y), vec2(22.0, 22.0));
                pa.rect(kr, cr(6.0), mix(well(&t), accent, k), Stroke::new(1.0, hair(&t)), StrokeKind::Inside);
                pa.text(kr.center(), Align2::CENTER_CENTER, format!("{}", i + 1), mono(11.0), mix(dim(&t), t.accent_foreground, k));
                let lr = text(&pa, pos2(r.min.x + 42.0, r.min.y + 15.0), Align2::LEFT_CENTER, label, font(13.0), t.foreground, r.width() - 160.0);
                if rec {
                    badge(&pa, &t, pos2(lr.max.x + 8.0, r.min.y + 15.0), Align2::LEFT_CENTER, "Recommended", accent, false);
                }
                text(&pa, pos2(r.min.x + 42.0, r.min.y + 32.0), Align2::LEFT_CENTER, desc, font(11.5), dim(&t), r.width() - 60.0);
                icon(&pa, Icon::ArrowRight, pos2(r.max.x - 18.0 + 3.0 * k, r.center().y), 13.0, with_alpha(t.foreground, 0.2 + 0.6 * k));
                if let Some(h) = hit {
                    if h.hovered() {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }
                    if h.clicked() {
                        pick = Some(HitlAnswer::Choice(i));
                    }
                }
            }
            if self.allow_custom {
                let r = Rect::from_min_size(pos2(rect.min.x + pad, y), vec2(rect.width() - pad * 2.0, 38.0));
                pa.rect(r, cr(10.0), well(&t), Stroke::new(1.0, hair(&t)), StrokeKind::Inside);
                icon(&pa, Icon::Pencil, pos2(r.min.x + 16.0, r.center().y), 12.0, faint(&t));
                if !answered {
                    let mut buf: String = glowkit_core::state(ui, id.with("custom"), String::new);
                    let mut child = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_max(pos2(r.min.x + 30.0, r.min.y + 9.0), pos2(r.max.x - 80.0, r.max.y - 6.0))));
                    let te = egui::TextEdit::singleline(&mut buf)
                        .frame(egui::Frame::NONE)
                        .hint_text("Or tell it what to do instead…")
                        .font(font(12.5))
                        .desired_width(r.width() - 120.0);
                    let tr = child.add(te);
                    let send = Rect::from_min_size(pos2(r.max.x - 70.0, r.min.y + 6.0), vec2(62.0, 26.0));
                    let enter = tr.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    if (button(ui, send, id.with("send"), "Send", None, Btn::Secondary).clicked() || enter) && !buf.trim().is_empty() {
                        pick = Some(HitlAnswer::Custom(buf.trim().to_owned()));
                        buf.clear();
                    }
                    let focused = tr.has_focus();
                    glowkit_core::set_state(ui, id.with("custom"), buf);
                    if !focused && resp.hovered() {
                        for (k, key) in [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9].iter().enumerate() {
                            if k < self.options.len() && ui.input(|i| i.key_pressed(*key)) {
                                pick = Some(HitlAnswer::Choice(k));
                            }
                        }
                    }
                }
            }
        }
        if let Some(a) = pick {
            *self.answer = Some(a);
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// ErrorRecoveryCard

/// What the person picked on an [`ErrorRecoveryCard`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recovery {
    /// Run the step again.
    Retry,
    /// Skip it and continue with the next step.
    Skip,
    /// Stop the whole run.
    Abort,
}

/// A failed step waiting for a decision: a red header with the step and where it sits in the
/// run, the error code and message, attempt pips ("attempt 2 of 3"), the last log lines, the
/// agent's suggested fix, and Retry / Skip / Abort. With `auto_retry` a ring counts down to the
/// next automatic attempt; while `retrying` the card shows a spinner, and `recovered` turns it
/// green.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{ErrorRecoveryCard, Recovery};
/// let out = ErrorRecoveryCard::new("Capture the partial refund", "402 card_declined")
///     .message("The processor declined the partial capture on the original card.")
///     .step(3, 6).attempt(2, 3)
///     .suggestion("Retry with a manual amount, or credit the account instead.")
///     .show(ui);
/// if out == Some(Recovery::Retry) { /* re-run the step */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct ErrorRecoveryCard {
    step_title: String,
    code: String,
    message: String,
    step: (usize, usize),
    attempt: (usize, usize),
    logs: Vec<String>,
    suggestion: String,
    auto_retry: Option<f32>,
    retrying: bool,
    recovered: bool,
    width: f32,
}

impl ErrorRecoveryCard {
    /// A card for the failed step `title` with error `code`.
    pub fn new(title: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            step_title: title.into(),
            code: code.into(),
            message: String::new(),
            step: (1, 1),
            attempt: (1, 3),
            logs: Vec::new(),
            suggestion: String::new(),
            auto_retry: None,
            retrying: false,
            recovered: false,
            width: 540.0,
        }
    }
    /// Human-readable error message.
    pub fn message(mut self, s: impl Into<String>) -> Self {
        self.message = s.into();
        self
    }
    /// Step index (1-based) out of total.
    pub fn step(mut self, i: usize, of: usize) -> Self {
        self.step = (i, of);
        self
    }
    /// Attempt number (1-based) out of the maximum.
    pub fn attempt(mut self, i: usize, of: usize) -> Self {
        self.attempt = (i, of);
        self
    }
    /// Add a log line.
    pub fn log(mut self, s: impl Into<String>) -> Self {
        self.logs.push(s.into());
        self
    }
    /// The agent's suggested fix.
    pub fn suggestion(mut self, s: impl Into<String>) -> Self {
        self.suggestion = s.into();
        self
    }
    /// Seconds until the next automatic retry (shows a countdown ring).
    pub fn auto_retry(mut self, secs: Option<f32>) -> Self {
        self.auto_retry = secs;
        self
    }
    /// The step is being retried right now.
    pub fn retrying(mut self, on: bool) -> Self {
        self.retrying = on;
        self
    }
    /// The retry worked.
    pub fn recovered(mut self, on: bool) -> Self {
        self.recovered = on;
        self
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(320.0);
        self
    }

    /// Show the card; returns the action clicked this frame.
    pub fn show(self, ui: &mut Ui) -> Option<Recovery> {
        let t = Theme::of(ui);
        let now = glowkit_core::time(ui);
        let id = ui.next_auto_id().with("err_recovery");
        let state = if self.recovered {
            RunState::Done
        } else if self.retrying {
            RunState::Running
        } else {
            RunState::Failed
        };
        let c = state.color(&t);
        let logs_h = if self.logs.is_empty() { 0.0 } else { 14.0 + self.logs.len() as f32 * 17.0 };
        let sug_h = if self.suggestion.is_empty() { 0.0 } else { 52.0 };
        let h = 150.0 + logs_h + sug_h + 56.0;
        let (rect, _) = alloc_centered(ui, self.width, h, Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        let cc = glowkit_core::anim::follow(ui, id.with("c"), match state { RunState::Done => 2.0, RunState::Running => 1.0, _ => 0.0 }, 6.0);
        let col = if cc < 1.0 { mix(t.destructive, t.info, cc) } else { mix(t.info, t.success, cc - 1.0) };
        let _ = c;
        panel(&p, &t, rect, 14.0);
        let head = Rect::from_min_size(rect.min, vec2(rect.width(), 64.0));
        p.rect_filled(head.shrink(1.0), egui::CornerRadius { nw: 13, ne: 13, sw: 0, se: 0 }, tint(&t, col));
        hline(&p, &t, rect.min.x, rect.max.x, head.max.y);
        let pad = 16.0;
        state_icon(&p, &t, pos2(rect.min.x + pad + 10.0, head.center().y), 10.0, state, now);
        let title = match state {
            RunState::Done => "Recovered".to_string(),
            RunState::Running => format!("Retrying step {} of {}", self.step.0, self.step.1),
            _ => format!("Step {} of {} failed", self.step.0, self.step.1),
        };
        text(&p, pos2(rect.min.x + pad + 28.0, head.min.y + 23.0), Align2::LEFT_CENTER, &title, font(13.5), ink(&t, col), 300.0);
        text(&p, pos2(rect.min.x + pad + 28.0, head.min.y + 42.0), Align2::LEFT_CENTER, &self.step_title, font(12.0), t.foreground, rect.width() - 200.0);
        // attempt pips
        let (ai, an) = self.attempt;
        let px = rect.max.x - pad;
        let lr = text(&p, pos2(px, head.min.y + 23.0), Align2::RIGHT_CENTER, &format!("attempt {ai} of {an}"), font(11.0), dim(&t), 120.0);
        for k in 0..an {
            let x = px - (an - 1 - k) as f32 * 14.0 - 4.0;
            let cpos = pos2(x, head.min.y + 42.0);
            let used = k < ai;
            let fill = if !used { track(&t) } else if k + 1 == ai { col } else { with_alpha(t.destructive, 0.55) };
            p.circle_filled(cpos, 4.0, fill);
        }
        let _ = lr;
        // error body
        let mut y = head.max.y + 14.0;
        let codew = chip(&p, &t, pos2(rect.min.x + pad, y + 9.0), Align2::LEFT_CENTER, &self.code, true);
        let _ = codew;
        y += 26.0;
        para(&p, pos2(rect.min.x + pad, y), &self.message, font(12.5), t.foreground, rect.width() - pad * 2.0, 2);
        y += 40.0;
        if !self.logs.is_empty() {
            let lrr = Rect::from_min_size(pos2(rect.min.x + pad, y), vec2(rect.width() - pad * 2.0, logs_h - 4.0));
            p.rect_filled(lrr, cr(8.0), well(&t));
            for (k, l) in self.logs.iter().enumerate() {
                let ly = lrr.min.y + 15.0 + k as f32 * 17.0;
                let err = l.contains("ERR") || l.contains("error");
                text(&p, pos2(lrr.min.x + 10.0, ly), Align2::LEFT_CENTER, l, mono(10.5), if err { ink(&t, t.destructive) } else { dim(&t) }, lrr.width() - 20.0);
            }
            y += logs_h + 6.0;
        }
        if !self.suggestion.is_empty() {
            let sr = Rect::from_min_size(pos2(rect.min.x + pad, y), vec2(rect.width() - pad * 2.0, 44.0));
            p.rect(sr, cr(9.0), tint(&t, t.accent), Stroke::new(1.0, with_alpha(t.accent, 0.25)), StrokeKind::Inside);
            icon(&p, Icon::Lightbulb, pos2(sr.min.x + 16.0, sr.min.y + 15.0), 13.0, ink(&t, t.accent));
            text(&p, pos2(sr.min.x + 30.0, sr.min.y + 14.0), Align2::LEFT_CENTER, "Agent suggests", font(11.0), ink(&t, t.accent), 200.0);
            text(&p, pos2(sr.min.x + 30.0, sr.min.y + 30.0), Align2::LEFT_CENTER, &self.suggestion, font(12.0), t.foreground, sr.width() - 40.0);
            y += sug_h;
        }
        let _ = y;
        // actions
        let by = rect.max.y - 30.0;
        hline(&p, &t, rect.min.x, rect.max.x, rect.max.y - 56.0);
        let mut out = None;
        if state == RunState::Failed {
            let rb = Rect::from_min_size(pos2(rect.max.x - pad - 92.0, by - 14.0), vec2(92.0, 28.0));
            if button(ui, rb, id.with("retry"), "Retry", Some(Icon::Refresh), Btn::Primary).clicked() {
                out = Some(Recovery::Retry);
            }
            let sb = Rect::from_min_size(pos2(rb.min.x - 96.0, by - 14.0), vec2(88.0, 28.0));
            if button(ui, sb, id.with("skip"), "Skip step", None, Btn::Secondary).clicked() {
                out = Some(Recovery::Skip);
            }
            let ab = Rect::from_min_size(pos2(rect.min.x + pad, by - 14.0), vec2(80.0, 28.0));
            if button(ui, ab, id.with("abort"), "Abort run", None, Btn::Danger).clicked() {
                out = Some(Recovery::Abort);
            }
            if let Some(s) = self.auto_retry {
                let cx = sb.min.x - 22.0;
                let frac = (s / 10.0).clamp(0.0, 1.0);
                p.circle_stroke(pos2(cx, by), 9.0, Stroke::new(2.0, track(&t)));
                glowkit_core::paint::arc(&p, pos2(cx, by), 9.0, -std::f32::consts::FRAC_PI_2, -std::f32::consts::FRAC_PI_2 + frac * std::f32::consts::TAU, 2.0, t.info);
                text(&p, pos2(cx - 16.0, by), Align2::RIGHT_CENTER, &format!("auto-retry in {}s", s.ceil() as i32), font(11.0), dim(&t), 140.0);
                glowkit_core::animate(ui);
            }
        } else if state == RunState::Running {
            spinner(&p, pos2(rect.min.x + pad + 8.0, by), 7.0, t.info, now, 1.8);
            text(&p, pos2(rect.min.x + pad + 24.0, by), Align2::LEFT_CENTER, "Running the step again with backoff…", font(12.0), dim(&t), 300.0);
            glowkit_core::animate(ui);
        } else {
            icon(&p, Icon::Check, pos2(rect.min.x + pad + 8.0, by), 14.0, t.success);
            text(&p, pos2(rect.min.x + pad + 24.0, by), Align2::LEFT_CENTER, "Step succeeded, the run continues", font(12.0), dim(&t), 300.0);
        }
        if (cc - cc.round()).abs() > 0.01 {
            glowkit_core::animate(ui);
        }
        out
    }
}
