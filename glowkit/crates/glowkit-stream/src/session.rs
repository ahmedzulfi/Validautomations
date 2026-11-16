//! [`Session`]: folds [`AgentEvent`]s into a thread of messages made of parts.

use crate::event::{AgentEvent, ApprovalOption, PlanEntry, SourceRef, StopReason, Usage};
use crate::partial_json::parse_partial_json;
use serde_json::Value;
use std::collections::HashMap;

/// Who wrote a [`Message`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    /// The person using the app.
    User,
    /// The agent / model.
    Assistant,
}

/// Lifecycle of a [`ToolCallPart`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ToolState {
    /// Arguments are streaming in, or the call waits for approval.
    #[default]
    Pending,
    /// Executing.
    Running,
    /// Finished successfully.
    Done,
    /// Failed.
    Error,
    /// The user (or a policy) refused to run it.
    Denied,
}

impl ToolState {
    /// Whether the call reached a final state.
    pub fn is_final(self) -> bool {
        matches!(self, ToolState::Done | ToolState::Error | ToolState::Denied)
    }
}

/// Streamed answer text (Markdown).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextPart {
    /// Source part id.
    pub id: String,
    /// Text received so far.
    pub text: String,
    /// Still receiving.
    pub streaming: bool,
}

/// The model's visible reasoning.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReasoningPart {
    /// Source part id.
    pub id: String,
    /// Text received so far.
    pub text: String,
    /// Still receiving.
    pub streaming: bool,
}

/// A tool / function call.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ToolCallPart {
    /// Call id.
    pub id: String,
    /// Tool name (empty until known).
    pub name: String,
    /// Raw argument JSON text received so far.
    pub args_text: String,
    /// Arguments: final when [`Self::input_final`], else a best-effort parse of `args_text`.
    pub input: Option<Value>,
    /// Whether `input` is the complete, parsed input.
    pub input_final: bool,
    /// Tool output.
    pub output: Option<Value>,
    /// Error text when the call failed.
    pub error: Option<String>,
    /// Lifecycle state.
    pub state: ToolState,
    /// The approval gating this call, if any.
    pub approval_id: Option<String>,
}

impl ToolCallPart {
    /// Arguments pretty-printed (falls back to the raw text while it is not valid JSON yet).
    pub fn input_pretty(&self) -> String {
        match &self.input {
            Some(v) => pretty(v),
            None => self.args_text.clone(),
        }
    }

    /// Output pretty-printed (strings are shown as-is).
    pub fn output_pretty(&self) -> Option<String> {
        self.output.as_ref().map(pretty)
    }
}

/// Pretty-print JSON; strings are returned without quotes.
pub fn pretty(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => serde_json::to_string_pretty(other).unwrap_or_default(),
    }
}

/// A request for the user's approval.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ApprovalPart {
    /// Approval id: send it back with the decision.
    pub id: String,
    /// The gated tool call.
    pub tool_call_id: Option<String>,
    /// Question shown to the user.
    pub title: String,
    /// Longer explanation.
    pub description: Option<String>,
    /// Offered choices; empty means approve / deny.
    pub options: Vec<ApprovalOption>,
    /// `None` while waiting, then whether it was approved.
    pub decision: Option<bool>,
    /// Reason given with the decision.
    pub reason: Option<String>,
}

/// An inline error.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ErrorPart {
    /// Message.
    pub message: String,
    /// Code.
    pub code: Option<String>,
}

/// One piece of a [`Message`].
#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    /// Answer text.
    Text(TextPart),
    /// Reasoning.
    Reasoning(ReasoningPart),
    /// A tool call.
    ToolCall(ToolCallPart),
    /// An approval request.
    Approval(ApprovalPart),
    /// The agent's plan.
    Plan(Vec<PlanEntry>),
    /// A source / citation.
    Source(SourceRef),
    /// An error.
    Error(ErrorPart),
}

/// One message of the thread.
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    /// Stable id (`"m0"`, `"m1"`...).
    pub id: String,
    /// Author.
    pub role: Role,
    /// Content, in arrival order.
    pub parts: Vec<Part>,
}

impl Message {
    /// All text parts joined.
    pub fn text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|p| match p {
                Part::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// The sources cited in this message.
    pub fn sources(&self) -> impl Iterator<Item = &SourceRef> {
        self.parts.iter().filter_map(|p| match p {
            Part::Source(s) => Some(s),
            _ => None,
        })
    }
}

/// Where the current run stands.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum RunStatus {
    /// No run yet.
    #[default]
    Idle,
    /// A run is streaming.
    Running,
    /// The last run ended.
    Finished(StopReason),
    /// The last run failed.
    Failed(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Kind {
    Text,
    Reasoning,
    Tool,
    Approval,
}

/// The state of an agent thread: every [`AgentEvent`] is applied with [`Session::apply`].
///
/// It is a plain value (`Clone + Send + Sync`), has no egui dependency and never panics on odd
/// input: deltas without a start open their part, results for unknown calls create the call,
/// parallel tool calls interleave by id, and argument JSON is previewed while incomplete.
///
/// ```
/// use glowkit_stream::{AgentEvent, Session};
/// let mut s = Session::new();
/// s.push_user("Hi!");
/// s.apply(AgentEvent::text("t1", "Hello "));
/// s.apply(AgentEvent::text("t1", "there"));
/// assert_eq!(s.messages[1].text(), "Hello there");
/// ```
#[derive(Clone, Debug, Default)]
pub struct Session {
    /// The thread.
    pub messages: Vec<Message>,
    /// Current run status.
    pub status: RunStatus,
    /// Name of the step in progress.
    pub step: Option<String>,
    /// Latest usage numbers.
    pub usage: Usage,
    /// Last error message.
    pub last_error: Option<String>,
    /// Custom / source-specific events, most recent last (capped at 64).
    pub custom: Vec<(String, Value)>,
    /// Bumped on every applied event; handy to know when to repaint or save.
    pub version: u64,
    index: HashMap<(Kind, String), (usize, usize)>,
    new_message: bool,
    next_id: u64,
}

impl Session {
    /// An empty thread.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a user message (the prompt you send to the agent).
    pub fn push_user(&mut self, text: impl Into<String>) {
        let id = self.fresh_id();
        self.messages.push(Message {
            id: id.clone(),
            role: Role::User,
            parts: vec![Part::Text(TextPart { id, text: text.into(), streaming: false })],
        });
        self.new_message = true;
        self.version += 1;
    }

    /// Whether a run is streaming.
    pub fn is_running(&self) -> bool {
        self.status == RunStatus::Running
    }

    /// Approvals still waiting for the user.
    pub fn pending_approvals(&self) -> impl Iterator<Item = &ApprovalPart> {
        self.messages.iter().flat_map(|m| &m.parts).filter_map(|p| match p {
            Part::Approval(a) if a.decision.is_none() => Some(a),
            _ => None,
        })
    }

    /// Find a tool call by id.
    pub fn tool_call(&self, id: &str) -> Option<&ToolCallPart> {
        match self.locate(Kind::Tool, id).map(|(m, p)| &self.messages[m].parts[p]) {
            Some(Part::ToolCall(t)) => Some(t),
            _ => None,
        }
    }

    /// Find an approval by id.
    pub fn approval(&self, id: &str) -> Option<&ApprovalPart> {
        match self.locate(Kind::Approval, id).map(|(m, p)| &self.messages[m].parts[p]) {
            Some(Part::Approval(a)) => Some(a),
            _ => None,
        }
    }

    /// Every tool call in the thread.
    pub fn tool_calls(&self) -> impl Iterator<Item = &ToolCallPart> {
        self.messages.iter().flat_map(|m| &m.parts).filter_map(|p| match p {
            Part::ToolCall(t) => Some(t),
            _ => None,
        })
    }

    /// Apply many events.
    pub fn extend(&mut self, events: impl IntoIterator<Item = AgentEvent>) {
        for e in events {
            self.apply(e);
        }
    }

    /// Fold one event into the thread.
    pub fn apply(&mut self, event: AgentEvent) {
        self.version += 1;
        match event {
            AgentEvent::RunStart { .. } => {
                self.status = RunStatus::Running;
                self.new_message = true;
                self.last_error = None;
            }
            AgentEvent::RunFinish { stop } => {
                self.close_streaming();
                self.step = None;
                // An error already reported keeps the run marked as failed.
                if !matches!(self.status, RunStatus::Failed(_)) {
                    self.status = match stop {
                        StopReason::Error => {
                            RunStatus::Failed(self.last_error.clone().unwrap_or_else(|| "Run failed".to_owned()))
                        }
                        s => RunStatus::Finished(s),
                    };
                }
            }
            AgentEvent::StepStart { name } => {
                self.ensure_running();
                self.step = name;
            }
            AgentEvent::StepEnd { name } => {
                if name.is_none() || name == self.step {
                    self.step = None;
                }
            }
            AgentEvent::TextStart { id } => {
                self.text_part(&id);
            }
            AgentEvent::TextDelta { id, delta } => {
                if let Part::Text(t) = self.text_part(&id) {
                    t.text.push_str(&delta);
                    t.streaming = true;
                }
            }
            AgentEvent::TextEnd { id } => {
                if let Some(Part::Text(t)) = self.get_mut(Kind::Text, &id) {
                    t.streaming = false;
                }
            }
            AgentEvent::ReasoningDelta { id, delta } => {
                if let Part::Reasoning(r) = self.reasoning_part(&id) {
                    r.text.push_str(&delta);
                    r.streaming = true;
                }
            }
            AgentEvent::ReasoningEnd { id } => {
                if let Some(Part::Reasoning(r)) = self.get_mut(Kind::Reasoning, &id) {
                    r.streaming = false;
                }
            }
            AgentEvent::ToolCallStart { id, name } => {
                if let Part::ToolCall(t) = self.tool_part(&id) {
                    if !name.is_empty() {
                        t.name = name;
                    }
                }
            }
            AgentEvent::ToolArgsDelta { id, delta } => {
                if let Part::ToolCall(t) = self.tool_part(&id) {
                    if !t.input_final {
                        t.args_text.push_str(&delta);
                        if let Some(v) = parse_partial_json(&t.args_text) {
                            t.input = Some(v);
                        }
                    }
                }
            }
            AgentEvent::ToolInputReady { id, name, input } => {
                if let Part::ToolCall(t) = self.tool_part(&id) {
                    if let Some(n) = name.filter(|n| !n.is_empty()) {
                        t.name = n;
                    }
                    if t.args_text.is_empty() {
                        t.args_text = serde_json::to_string(&input).unwrap_or_default();
                    }
                    t.input = Some(input);
                    t.input_final = true;
                }
            }
            AgentEvent::ToolRunning { id } => {
                if let Part::ToolCall(t) = self.tool_part(&id) {
                    if !t.state.is_final() {
                        t.state = ToolState::Running;
                    }
                }
            }
            AgentEvent::ToolResult { id, output, is_error } => {
                let approval = if let Part::ToolCall(t) = self.tool_part(&id) {
                    if is_error {
                        t.error = Some(pretty(&output));
                        t.state = ToolState::Error;
                    } else {
                        t.state = ToolState::Done;
                    }
                    t.output = Some(output);
                    t.input_final = t.input_final || t.input.is_some();
                    t.approval_id.clone()
                } else {
                    None
                };
                // A result for a gated call means the approval was granted.
                if let Some(aid) = approval {
                    if let Some(Part::Approval(a)) = self.get_mut(Kind::Approval, &aid) {
                        a.decision.get_or_insert(true);
                    }
                }
            }
            AgentEvent::ApprovalRequest { id, tool_call_id, title, description, options } => {
                self.approval_request(id, tool_call_id, title, description, options);
            }
            AgentEvent::ApprovalDecision { id, approved, reason } => self.decide(&id, approved, reason),
            AgentEvent::Plan { entries } => {
                let (m, _) = self.current_message();
                let msg = &mut self.messages[m];
                match msg.parts.iter_mut().rev().find_map(|p| match p {
                    Part::Plan(e) => Some(e),
                    _ => None,
                }) {
                    Some(existing) => *existing = entries,
                    None => msg.parts.push(Part::Plan(entries)),
                }
            }
            AgentEvent::Source(src) => {
                let (m, _) = self.current_message();
                let msg = &mut self.messages[m];
                let dup = msg
                    .sources()
                    .any(|s| (!src.id.is_empty() && s.id == src.id) || (!src.url.is_empty() && s.url == src.url));
                if !dup {
                    msg.parts.push(Part::Source(src));
                }
            }
            AgentEvent::Usage(u) => self.usage.merge(&u),
            AgentEvent::Error { message, code } => {
                let (m, _) = self.current_message();
                self.messages[m].parts.push(Part::Error(ErrorPart { message: message.clone(), code }));
                self.close_streaming();
                self.last_error = Some(message.clone());
                self.status = RunStatus::Failed(message);
            }
            AgentEvent::Custom { name, value } => {
                self.custom.push((name, value));
                if self.custom.len() > 64 {
                    self.custom.remove(0);
                }
            }
        }
    }

    // ---------------------------------------------------------------------------------------

    fn fresh_id(&mut self) -> String {
        let id = format!("m{}", self.next_id);
        self.next_id += 1;
        id
    }

    fn ensure_running(&mut self) {
        if matches!(self.status, RunStatus::Idle | RunStatus::Finished(_)) {
            self.status = RunStatus::Running;
        }
    }

    /// The assistant message new parts go to, creating one when needed.
    fn current_message(&mut self) -> (usize, usize) {
        self.ensure_running();
        let reuse = !self.new_message && self.messages.last().is_some_and(|m| m.role == Role::Assistant);
        if !reuse {
            let id = self.fresh_id();
            self.messages.push(Message { id, role: Role::Assistant, parts: Vec::new() });
            self.new_message = false;
        }
        let m = self.messages.len() - 1;
        (m, self.messages[m].parts.len())
    }

    fn locate(&self, kind: Kind, id: &str) -> Option<(usize, usize)> {
        self.index.get(&(kind, id.to_owned())).copied()
    }

    fn get_mut(&mut self, kind: Kind, id: &str) -> Option<&mut Part> {
        let (m, p) = self.locate(kind, id)?;
        self.messages.get_mut(m)?.parts.get_mut(p)
    }

    fn insert(&mut self, kind: Kind, id: &str, part: Part) -> (usize, usize) {
        let (m, p) = self.current_message();
        self.messages[m].parts.push(part);
        self.index.insert((kind, id.to_owned()), (m, p));
        (m, p)
    }

    fn part_or_insert(&mut self, kind: Kind, id: &str, make: impl FnOnce() -> Part) -> &mut Part {
        let (m, p) = match self.locate(kind, id) {
            Some(at) => at,
            None => self.insert(kind, id, make()),
        };
        &mut self.messages[m].parts[p]
    }

    fn text_part(&mut self, id: &str) -> &mut Part {
        self.part_or_insert(Kind::Text, id, || {
            Part::Text(TextPart { id: id.to_owned(), text: String::new(), streaming: true })
        })
    }

    fn reasoning_part(&mut self, id: &str) -> &mut Part {
        self.part_or_insert(Kind::Reasoning, id, || {
            Part::Reasoning(ReasoningPart { id: id.to_owned(), text: String::new(), streaming: true })
        })
    }

    fn tool_part(&mut self, id: &str) -> &mut Part {
        self.part_or_insert(Kind::Tool, id, || Part::ToolCall(ToolCallPart { id: id.to_owned(), ..Default::default() }))
    }

    fn approval_request(
        &mut self,
        id: String,
        tool_call_id: Option<String>,
        title: Option<String>,
        description: Option<String>,
        options: Vec<ApprovalOption>,
    ) {
        let mut tool_name = None;
        if let Some(tid) = &tool_call_id {
            if let Part::ToolCall(t) = self.tool_part(tid) {
                t.approval_id = Some(id.clone());
                if !t.state.is_final() {
                    t.state = ToolState::Pending;
                }
                tool_name = Some(t.name.clone()).filter(|n| !n.is_empty());
            }
        }
        let title = title.unwrap_or_else(|| match &tool_name {
            Some(n) => format!("Allow {n}?"),
            None => "Approval required".to_owned(),
        });
        if let Some(Part::Approval(a)) = self.get_mut(Kind::Approval, &id) {
            a.title = title;
            a.description = description.or(a.description.take());
            if !options.is_empty() {
                a.options = options;
            }
            return;
        }
        let part = Part::Approval(ApprovalPart {
            id: id.clone(),
            tool_call_id: tool_call_id.clone(),
            title,
            description,
            options,
            decision: None,
            reason: None,
        });
        // Place it right after the gated tool call when we know where that is.
        match tool_call_id.and_then(|t| self.locate(Kind::Tool, &t)) {
            Some((m, p)) => {
                self.messages[m].parts.insert(p + 1, part);
                for v in self.index.values_mut() {
                    if v.0 == m && v.1 > p {
                        v.1 += 1;
                    }
                }
                self.index.insert((Kind::Approval, id), (m, p + 1));
            }
            None => {
                self.insert(Kind::Approval, &id, part);
            }
        }
    }

    fn decide(&mut self, id: &str, approved: bool, reason: Option<String>) {
        // The id may name the approval or the gated tool call.
        let approval_id = if self.locate(Kind::Approval, id).is_some() {
            Some(id.to_owned())
        } else {
            self.tool_call(id).and_then(|t| t.approval_id.clone())
        };
        let mut tool = None;
        if let Some(aid) = approval_id {
            if let Some(Part::Approval(a)) = self.get_mut(Kind::Approval, &aid) {
                a.decision = Some(approved);
                a.reason = reason;
                tool = a.tool_call_id.clone();
            }
        }
        let tool = tool.or_else(|| self.locate(Kind::Tool, id).map(|_| id.to_owned()));
        if let Some(tid) = tool {
            if let Some(Part::ToolCall(t)) = self.get_mut(Kind::Tool, &tid) {
                if !t.state.is_final() {
                    t.state = if approved { ToolState::Running } else { ToolState::Denied };
                }
            }
        }
    }

    fn close_streaming(&mut self) {
        for part in self.messages.iter_mut().flat_map(|m| &mut m.parts) {
            match part {
                Part::Text(t) => t.streaming = false,
                Part::Reasoning(r) => r.streaming = false,
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::PlanStatus;
    use serde_json::json;

    fn tool(s: &Session, id: &str) -> ToolCallPart {
        s.tool_call(id).cloned().expect("tool call exists")
    }

    #[test]
    fn text_streams_into_one_assistant_message() {
        let mut s = Session::new();
        s.push_user("hi");
        s.extend([
            AgentEvent::RunStart { run_id: None },
            AgentEvent::TextStart { id: "t".into() },
            AgentEvent::text("t", "Hel"),
            AgentEvent::text("t", "lo"),
            AgentEvent::TextEnd { id: "t".into() },
            AgentEvent::RunFinish { stop: StopReason::EndTurn },
        ]);
        assert_eq!(s.messages.len(), 2);
        assert_eq!(s.messages[1].role, Role::Assistant);
        assert_eq!(s.messages[1].text(), "Hello");
        assert_eq!(s.status, RunStatus::Finished(StopReason::EndTurn));
        assert!(matches!(&s.messages[1].parts[0], Part::Text(t) if !t.streaming));
    }

    #[test]
    fn missing_start_events_are_tolerated() {
        let mut s = Session::new();
        s.apply(AgentEvent::text("x", "no start"));
        s.apply(AgentEvent::TextEnd { id: "never-seen".into() });
        s.apply(AgentEvent::ReasoningEnd { id: "nope".into() });
        s.apply(AgentEvent::ToolResult { id: "c9".into(), output: json!("ok"), is_error: false });
        assert_eq!(s.messages.len(), 1);
        assert_eq!(s.messages[0].text(), "no start");
        let t = tool(&s, "c9");
        assert_eq!(t.state, ToolState::Done);
        assert!(t.name.is_empty());
        assert!(s.is_running());
    }

    #[test]
    fn args_before_start_then_name_arrives() {
        let mut s = Session::new();
        s.apply(AgentEvent::ToolArgsDelta { id: "c1".into(), delta: "{\"q\":".into() });
        s.apply(AgentEvent::ToolCallStart { id: "c1".into(), name: "search".into() });
        s.apply(AgentEvent::ToolArgsDelta { id: "c1".into(), delta: "\"rust".into() });
        let t = tool(&s, "c1");
        assert_eq!(t.name, "search");
        assert_eq!(t.input, Some(json!({"q": "rust"})));
        assert!(!t.input_final);
    }

    #[test]
    fn parallel_tool_calls_interleave() {
        let mut s = Session::new();
        s.extend([
            AgentEvent::ToolCallStart { id: "a".into(), name: "weather".into() },
            AgentEvent::ToolCallStart { id: "b".into(), name: "time".into() },
            AgentEvent::ToolArgsDelta { id: "b".into(), delta: "{\"tz\":\"UTC\"".into() },
            AgentEvent::ToolArgsDelta { id: "a".into(), delta: "{\"city\":\"Oslo\"}".into() },
            AgentEvent::ToolArgsDelta { id: "b".into(), delta: "}".into() },
            AgentEvent::ToolRunning { id: "a".into() },
            AgentEvent::ToolResult { id: "b".into(), output: json!({"time": "12:00"}), is_error: false },
            AgentEvent::ToolResult { id: "a".into(), output: json!("boom"), is_error: true },
        ]);
        let (a, b) = (tool(&s, "a"), tool(&s, "b"));
        assert_eq!(a.input, Some(json!({"city": "Oslo"})));
        assert_eq!(b.input, Some(json!({"tz": "UTC"})));
        assert_eq!(a.state, ToolState::Error);
        assert_eq!(a.error.as_deref(), Some("boom"));
        assert_eq!(b.state, ToolState::Done);
        assert_eq!(s.messages.len(), 1);
        assert_eq!(s.messages[0].parts.len(), 2);
    }

    #[test]
    fn input_ready_overrides_partial_preview() {
        let mut s = Session::new();
        s.apply(AgentEvent::ToolArgsDelta { id: "c".into(), delta: "{\"a\": [1, 2".into() });
        assert_eq!(tool(&s, "c").input, Some(json!({"a": [1, 2]})));
        s.apply(AgentEvent::ToolInputReady {
            id: "c".into(),
            name: Some("sum".into()),
            input: json!({"a": [1, 2, 3]}),
        });
        s.apply(AgentEvent::ToolArgsDelta { id: "c".into(), delta: "garbage".into() });
        let t = tool(&s, "c");
        assert!(t.input_final);
        assert_eq!(t.name, "sum");
        assert_eq!(t.input, Some(json!({"a": [1, 2, 3]})));
    }

    #[test]
    fn approval_flow_approve_and_deny() {
        let mut s = Session::new();
        s.extend([
            AgentEvent::ToolCallStart { id: "c1".into(), name: "delete_file".into() },
            AgentEvent::TextStart { id: "t".into() },
            AgentEvent::ApprovalRequest {
                id: "ap1".into(),
                tool_call_id: Some("c1".into()),
                title: None,
                description: None,
                options: vec![],
            },
        ]);
        // approval sits right after its tool call, before the later text
        assert!(matches!(&s.messages[0].parts[1], Part::Approval(a) if a.title == "Allow delete_file?"));
        assert!(matches!(&s.messages[0].parts[2], Part::Text(_)));
        assert_eq!(s.pending_approvals().count(), 1);
        // index stayed consistent after the insert
        s.apply(AgentEvent::text("t", "still works"));
        assert_eq!(s.messages[0].text(), "still works");

        s.apply(AgentEvent::decision("ap1", false));
        assert_eq!(tool(&s, "c1").state, ToolState::Denied);
        assert_eq!(s.approval("ap1").unwrap().decision, Some(false));
        assert_eq!(s.pending_approvals().count(), 0);

        let mut s2 = Session::new();
        s2.apply(AgentEvent::ApprovalRequest {
            id: "c2".into(),
            tool_call_id: Some("c2".into()),
            title: Some("Run it?".into()),
            description: None,
            options: vec![],
        });
        s2.apply(AgentEvent::decision("c2", true));
        assert_eq!(tool(&s2, "c2").state, ToolState::Running);
        s2.apply(AgentEvent::ToolResult { id: "c2".into(), output: json!(1), is_error: false });
        assert_eq!(tool(&s2, "c2").state, ToolState::Done);
    }

    #[test]
    fn result_implies_approval() {
        let mut s = Session::new();
        s.apply(AgentEvent::ApprovalRequest {
            id: "a".into(),
            tool_call_id: Some("c".into()),
            title: None,
            description: None,
            options: vec![],
        });
        s.apply(AgentEvent::ToolResult { id: "c".into(), output: json!("done"), is_error: false });
        assert_eq!(s.approval("a").unwrap().decision, Some(true));
    }

    #[test]
    fn new_run_starts_new_message_but_results_find_old_calls() {
        let mut s = Session::new();
        s.extend([
            AgentEvent::RunStart { run_id: Some("r1".into()) },
            AgentEvent::ToolCallStart { id: "c".into(), name: "book".into() },
            AgentEvent::RunFinish { stop: StopReason::Interrupted },
            AgentEvent::RunStart { run_id: Some("r2".into()) },
            AgentEvent::ToolResult { id: "c".into(), output: json!("booked"), is_error: false },
            AgentEvent::text("t2", "Done."),
        ]);
        assert_eq!(s.messages.len(), 2);
        assert_eq!(s.messages[0].parts.len(), 1);
        assert_eq!(tool(&s, "c").state, ToolState::Done);
        assert_eq!(s.messages[1].text(), "Done.");
    }

    #[test]
    fn plan_sources_usage_error_custom() {
        let mut s = Session::new();
        s.apply(AgentEvent::Plan { entries: vec![PlanEntry::new("a", PlanStatus::InProgress)] });
        s.apply(AgentEvent::Plan {
            entries: vec![PlanEntry::new("a", PlanStatus::Completed), PlanEntry::new("b", PlanStatus::Pending)],
        });
        let src =
            SourceRef { id: "s1".into(), title: "Doc".into(), url: "https://example.com".into(), snippet: "".into() };
        s.apply(AgentEvent::Source(src.clone()));
        s.apply(AgentEvent::Source(src));
        s.apply(AgentEvent::Usage(Usage { input_tokens: Some(100), ..Default::default() }));
        s.apply(AgentEvent::Usage(Usage { output_tokens: Some(20), ..Default::default() }));
        s.apply(AgentEvent::Custom { name: "x".into(), value: json!(1) });
        s.apply(AgentEvent::text("t", "partial"));
        s.apply(AgentEvent::Error { message: "rate limited".into(), code: Some("429".into()) });
        s.apply(AgentEvent::RunFinish { stop: StopReason::Error });

        let parts = &s.messages[0].parts;
        assert_eq!(parts.iter().filter(|p| matches!(p, Part::Plan(_))).count(), 1);
        assert!(matches!(&parts[0], Part::Plan(e) if e.len() == 2 && e[0].status == PlanStatus::Completed));
        assert_eq!(s.messages[0].sources().count(), 1);
        assert_eq!(s.usage.total(), Some(120));
        assert_eq!(s.custom.len(), 1);
        assert!(matches!(parts.last(), Some(Part::Error(e)) if e.code.as_deref() == Some("429")));
        assert_eq!(s.status, RunStatus::Failed("rate limited".into()));
        assert!(matches!(&parts[2], Part::Text(t) if !t.streaming));
    }

    #[test]
    fn steps_track_current_name() {
        let mut s = Session::new();
        s.apply(AgentEvent::StepStart { name: Some("plan".into()) });
        assert_eq!(s.step.as_deref(), Some("plan"));
        s.apply(AgentEvent::StepEnd { name: Some("other".into()) });
        assert_eq!(s.step.as_deref(), Some("plan"));
        s.apply(AgentEvent::StepEnd { name: Some("plan".into()) });
        assert_eq!(s.step, None);
    }
}
