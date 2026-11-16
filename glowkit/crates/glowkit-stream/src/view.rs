//! [`AgentThread`]: renders a [`Session`] with the glowkit-ai components.

use crate::event::{AgentEvent, PlanStatus};
use crate::session::{ApprovalPart, ErrorPart, Message, Part, Role, RunStatus, Session, ToolCallPart, ToolState};
use egui::{vec2, CornerRadius, Frame, Margin, RichText, Stroke, Ui};
use glowkit_ai::{
    ApprovalState, ChatMessage, Confirmation, ContextMeter, Conversation, Markdown, MeterStyle, Reasoning, ShimmerText,
    Source, Sources, TaskItem, TaskList, TaskStatus, ToolCall, ToolStatus,
};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// The user's answer to an approval request, returned by [`AgentThread::show`]. Send it back to
/// your agent (AG-UI resume, Vercel approval response, ACP permission response...).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovalResponse {
    /// The approval id from the request.
    pub approval_id: String,
    /// The gated tool call, if any.
    pub tool_call_id: Option<String>,
    /// Approved or denied.
    pub approved: bool,
    /// For sources that offer named options (ACP), the option matching the decision.
    pub option_id: Option<String>,
}

impl ApprovalResponse {
    /// The matching [`AgentEvent::ApprovalDecision`] (already applied to the session by the view).
    pub fn to_event(&self) -> AgentEvent {
        AgentEvent::ApprovalDecision { id: self.approval_id.clone(), approved: self.approved, reason: None }
    }
}

/// What happened in an [`AgentThread`] this frame.
#[derive(Clone, Debug, Default)]
pub struct ThreadOutput {
    /// Approvals the user answered this frame.
    pub decisions: Vec<ApprovalResponse>,
}

/// Renders a whole agent thread from a [`Session`]: user bubbles, assistant messages with their
/// reasoning, tool calls (with live argument previews and status badges), approval cards, plans,
/// sources and errors, in the order they streamed, plus a shimmering status line while the
/// agent works.
///
/// Approval decisions are applied to the session and returned so you can send them back.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut session = glowkit_stream::Session::new();
/// use glowkit_stream::AgentThread;
/// let out = AgentThread::new(&mut session).show(ui);
/// for d in out.decisions {
///     // forward `d` to the agent
/// }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct AgentThread<'a> {
    session: &'a mut Session,
    scroll: bool,
    height: Option<f32>,
    max_width: Option<f32>,
    usage: bool,
    model: Option<&'a str>,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> AgentThread<'a> {
    /// A thread view over `session`.
    pub fn new(session: &'a mut Session) -> Self {
        Self { session, scroll: true, height: None, max_width: None, usage: true, model: None, id_salt: None }
    }
    /// Wrap the messages in a sticky-bottom [`Conversation`] scroll area (default on).
    pub fn scroll(mut self, on: bool) -> Self {
        self.scroll = on;
        self
    }
    /// Height of the scroll area (defaults to the available height).
    pub fn height(mut self, h: f32) -> Self {
        self.height = Some(h);
        self
    }
    /// Center the messages in a column at most this wide.
    pub fn max_width(mut self, w: f32) -> Self {
        self.max_width = Some(w);
        self
    }
    /// Show the context meter when the source reports a context window (default on).
    pub fn usage(mut self, on: bool) -> Self {
        self.usage = on;
        self
    }
    /// Model name shown in the context meter tooltip.
    pub fn model(mut self, name: &'a str) -> Self {
        self.model = Some(name);
        self
    }
    /// Stable id when several threads share a parent.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Draw the thread and return the decisions made this frame.
    pub fn show(self, ui: &mut Ui) -> ThreadOutput {
        let AgentThread { session, scroll, height, max_width, usage, model, id_salt } = self;
        let salt = id_salt.map(|s| ui.scope_id().with_salt(s)).unwrap_or_else(|| ui.scope_id().with("agent-thread"));
        let mut decisions = Vec::new();
        if usage {
            usage_row(ui, session, model);
        }
        let mut draw = |ui: &mut Ui| {
            ui.spacing_mut().item_spacing.y = 14.0;
            body(ui, session, salt, &mut decisions);
        };
        if scroll {
            let mut c = Conversation::new().id_salt(salt.with("conv"));
            if let Some(h) = height {
                c = c.height(h);
            }
            if let Some(w) = max_width {
                c = c.max_width(w);
            }
            c.show(ui, draw);
        } else {
            draw(ui);
        }
        for d in &decisions {
            session.apply(d.to_event());
        }
        ThreadOutput { decisions }
    }
}

fn usage_row(ui: &mut Ui, session: &Session, model: Option<&str>) {
    let u = session.usage;
    let (Some(size), used) = (u.context_size, u.context_used.or(u.total())) else { return };
    let used = used.unwrap_or(0);
    ui.horizontal(|ui| {
        let mut m = ContextMeter::new(used, size).style(MeterStyle::Bar);
        if let Some(name) = model {
            m = m.model(name);
        }
        if let Some(i) = u.input_tokens {
            m = m.segment("Input", i);
        }
        if let Some(o) = u.output_tokens {
            m = m.segment("Output", o);
        }
        ui.add(m);
    });
}

fn body(ui: &mut Ui, session: &Session, salt: egui::Id, decisions: &mut Vec<ApprovalResponse>) {
    let t = Theme::of(ui);
    let last = session.messages.len().saturating_sub(1);
    for (i, msg) in session.messages.iter().enumerate() {
        match msg.role {
            Role::User => {
                ChatMessage::new(glowkit_ai::Role::User, &msg.text()).id_salt(salt.with(&msg.id)).show(ui, |_| {});
            }
            Role::Assistant => {
                let busy = session.is_running() && i == last;
                ChatMessage::new(glowkit_ai::Role::Assistant, "").id_salt(salt.with(&msg.id)).show(ui, |ui| {
                    assistant_parts(ui, msg, salt, decisions);
                    if busy && !msg.parts.iter().any(streaming) {
                        status_line(ui, session);
                    }
                });
            }
        }
    }
    let waiting = session.is_running() && session.messages.last().is_none_or(|m| m.role == Role::User);
    if waiting {
        ChatMessage::new(glowkit_ai::Role::Assistant, "").id_salt(salt.with("pending")).show(ui, |ui| {
            status_line(ui, session);
        });
    }
    if let RunStatus::Finished(stop) = &session.status {
        if !matches!(stop, crate::StopReason::EndTurn | crate::StopReason::ToolCalls) {
            ui.label(RichText::new(stop.label()).size(11.5).color(t.muted_foreground));
        }
    }
}

fn streaming(p: &Part) -> bool {
    match p {
        Part::Text(t) => t.streaming,
        Part::Reasoning(r) => r.streaming,
        Part::ToolCall(c) => !c.state.is_final() && c.approval_id.is_none(),
        _ => false,
    }
}

fn status_line(ui: &mut Ui, session: &Session) {
    let label = match &session.step {
        Some(step) => format!("Working: {step}"),
        None if session.pending_approvals().next().is_some() => "Waiting for your approval".to_owned(),
        None => "Thinking".to_owned(),
    };
    ui.add(ShimmerText::new(&label).size(13.5));
}

fn assistant_parts(ui: &mut Ui, msg: &Message, salt: egui::Id, decisions: &mut Vec<ApprovalResponse>) {
    let mid = salt.with(&msg.id);
    let tools = |id: &str| {
        msg.parts.iter().find_map(|p| match p {
            Part::ToolCall(t) if t.id == id => Some(t),
            _ => None,
        })
    };
    for part in &msg.parts {
        match part {
            Part::Text(t) => {
                if !t.text.is_empty() || t.streaming {
                    ui.add(Markdown::new(&t.text).caret(t.streaming).fade_tail(if t.streaming { 28 } else { 0 }));
                }
            }
            Part::Reasoning(r) => {
                ui.add(Reasoning::new(&r.text).streaming(r.streaming).id_salt(mid.with(("reasoning", &r.id))));
            }
            Part::ToolCall(c) => tool_call(ui, c, mid),
            Part::Approval(a) => approval(ui, a, a.tool_call_id.as_deref().and_then(tools), mid, decisions),
            Part::Plan(entries) => {
                let items: Vec<TaskItem<'_>> = entries
                    .iter()
                    .map(|e| {
                        let s = match e.status {
                            PlanStatus::Pending => TaskStatus::Pending,
                            PlanStatus::InProgress => TaskStatus::Running,
                            PlanStatus::Completed => TaskStatus::Done,
                            PlanStatus::Failed => TaskStatus::Failed,
                        };
                        TaskItem::new(&e.content, s)
                    })
                    .collect();
                ui.add(TaskList::new("Plan", &items).id_salt(mid.with("plan")));
            }
            Part::Source(_) => {} // grouped below
            Part::Error(e) => error_card(ui, e),
        }
    }
    let sources: Vec<Source<'_>> = msg
        .sources()
        .map(|s| Source::new(if s.title.is_empty() { &s.url } else { &s.title }, &s.url).snippet(&s.snippet))
        .collect();
    if !sources.is_empty() {
        ui.add(Sources::new(&sources).id_salt(mid.with("sources")));
    }
}

fn tool_call(ui: &mut Ui, c: &ToolCallPart, mid: egui::Id) {
    let status = match c.state {
        ToolState::Pending => ToolStatus::Pending,
        ToolState::Running => ToolStatus::Running,
        ToolState::Done => ToolStatus::Completed,
        ToolState::Error | ToolState::Denied => ToolStatus::Error,
    };
    let name = if c.name.is_empty() { "tool" } else { c.name.as_str() };
    let input = c.input_pretty();
    let output = c.output_pretty();
    let mut w = ToolCall::new(name).status(status).id_salt(mid.with(("tool", &c.id)));
    if !input.is_empty() {
        w = w.input(&input);
    }
    let denied = "Denied by the user";
    match (c.state, &c.error, &output) {
        (ToolState::Denied, _, _) => w = w.error(denied),
        (_, Some(err), _) => w = w.error(err),
        (_, None, Some(out)) => w = w.output(out),
        _ => {}
    }
    if c.approval_id.is_some() && c.state == ToolState::Pending {
        w = w.description("Waiting for approval");
    }
    ui.add(w);
}

fn approval(
    ui: &mut Ui,
    a: &ApprovalPart,
    tool: Option<&ToolCallPart>,
    mid: egui::Id,
    decisions: &mut Vec<ApprovalResponse>,
) {
    let mut state = match a.decision {
        None => ApprovalState::Pending,
        Some(true) => ApprovalState::Approved,
        Some(false) => ApprovalState::Denied,
    };
    let before = state;
    let label = |approves: bool, fallback: &'static str| {
        a.options.iter().find(|o| o.approves == approves).map(|o| o.label.as_str()).unwrap_or(fallback)
    };
    let input = tool.map(|t| t.input_pretty()).unwrap_or_default();
    ui.push_id(mid.with(("approval", &a.id)), |ui| {
        let mut w = Confirmation::new(&mut state, &a.title).labels(label(true, "Approve"), label(false, "Deny"));
        if let Some(t) = tool.filter(|t| !t.name.is_empty()) {
            w = w.tool(&t.name);
        }
        if let Some(d) = &a.description {
            w = w.description(d);
        }
        if !input.is_empty() {
            w = w.code_lang(&input, "json");
        }
        ui.add(w);
    });
    if before == ApprovalState::Pending && state != ApprovalState::Pending {
        let approved = state == ApprovalState::Approved;
        decisions.push(ApprovalResponse {
            approval_id: a.id.clone(),
            tool_call_id: a.tool_call_id.clone(),
            approved,
            option_id: a.options.iter().find(|o| o.approves == approved).map(|o| o.id.clone()),
        });
    }
}

fn error_card(ui: &mut Ui, e: &ErrorPart) {
    let t = Theme::of(ui);
    Frame::new()
        .fill(with_alpha(t.destructive, 0.08))
        .stroke(Stroke::new(1.0, with_alpha(t.destructive, 0.45)))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = vec2(8.0, 4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Error").strong().size(13.0).color(t.destructive));
                if let Some(code) = &e.code {
                    ui.label(RichText::new(code).monospace().size(11.5).color(t.muted_foreground));
                }
            });
            ui.label(RichText::new(&e.message).size(13.0).color(t.foreground));
        });
}
