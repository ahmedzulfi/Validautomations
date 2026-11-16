//! [AG-UI](https://docs.ag-ui.com) 1.x events (LangGraph, CrewAI, Mastra, Pydantic AI, ADK and
//! other agent frameworks speak it).
//!
//! The wire types here are written from the AG-UI 1.0 JSON schema; they do not depend on any
//! AG-UI crate. Unknown event types deserialize to [`AgUiEvent::Unknown`] and are ignored, and
//! the deprecated `THINKING_*` events are accepted as aliases of the `REASONING_*` ones.

use crate::event::{AgentEvent, StopReason, Usage};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Token usage for one provider and model.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    /// Provider label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Model label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Input tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    /// Output tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    /// Total tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
}

/// Something a paused run waits for (an approval, a missing value...).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Interrupt {
    /// Interrupt id; answer it with a resume entry carrying this id.
    pub id: String,
    /// Why the run stopped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Prompt for the person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// The tool call it concerns, for tool approvals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// JSON Schema of the expected answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<Value>,
}

/// Why a run ended (`RUN_FINISHED.outcome`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunOutcome {
    /// Completed.
    Success,
    /// Paused, waiting for the interrupts to be answered.
    Interrupt {
        /// What the run waits for.
        #[serde(default)]
        interrupts: Vec<Interrupt>,
    },
    /// Stopped before completing.
    Cancelled,
    /// A newer outcome; read as success.
    #[serde(other)]
    Unknown,
}

/// One AG-UI event, as sent on the wire (`{"type": "TEXT_MESSAGE_CONTENT", ...}`).
#[allow(missing_docs)] // field names mirror the protocol
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgUiEvent {
    #[serde(rename_all = "camelCase")]
    RunStarted {
        #[serde(default)]
        thread_id: String,
        #[serde(default)]
        run_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent_run_id: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    RunFinished {
        #[serde(default)]
        thread_id: String,
        #[serde(default)]
        run_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        result: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        outcome: Option<RunOutcome>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        usage: Vec<TokenUsage>,
    },
    #[serde(rename_all = "camelCase")]
    RunError {
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        code: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        usage: Vec<TokenUsage>,
    },
    #[serde(rename_all = "camelCase")]
    StepStarted {
        #[serde(default)]
        step_name: String,
    },
    #[serde(rename_all = "camelCase")]
    StepFinished {
        #[serde(default)]
        step_name: String,
    },
    #[serde(rename_all = "camelCase")]
    TextMessageStart {
        message_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        role: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    TextMessageContent { message_id: String, delta: String },
    #[serde(rename_all = "camelCase")]
    TextMessageEnd { message_id: String },
    #[serde(rename_all = "camelCase")]
    TextMessageChunk {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        role: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        delta: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    ToolCallStart {
        tool_call_id: String,
        tool_call_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent_message_id: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    ToolCallArgs { tool_call_id: String, delta: String },
    #[serde(rename_all = "camelCase")]
    ToolCallEnd { tool_call_id: String },
    #[serde(rename_all = "camelCase")]
    ToolCallChunk {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_call_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent_message_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        delta: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    ToolCallResult {
        #[serde(default)]
        message_id: String,
        tool_call_id: String,
        /// A string, or a list of content parts.
        content: Value,
    },
    #[serde(rename_all = "camelCase", alias = "THINKING_START")]
    ReasoningStart {
        #[serde(default)]
        message_id: String,
    },
    #[serde(rename_all = "camelCase", alias = "THINKING_TEXT_MESSAGE_START")]
    ReasoningMessageStart {
        #[serde(default)]
        message_id: String,
    },
    #[serde(rename_all = "camelCase", alias = "THINKING_TEXT_MESSAGE_CONTENT")]
    ReasoningMessageContent {
        #[serde(default)]
        message_id: String,
        delta: String,
    },
    #[serde(rename_all = "camelCase", alias = "THINKING_TEXT_MESSAGE_END")]
    ReasoningMessageEnd {
        #[serde(default)]
        message_id: String,
    },
    #[serde(rename_all = "camelCase")]
    ReasoningMessageChunk {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        delta: Option<String>,
    },
    #[serde(rename_all = "camelCase", alias = "THINKING_END")]
    ReasoningEnd {
        #[serde(default)]
        message_id: String,
    },
    #[serde(rename_all = "camelCase")]
    ReasoningEncryptedValue {
        #[serde(default)]
        subtype: String,
        #[serde(default)]
        entity_id: String,
    },
    #[serde(rename_all = "camelCase")]
    StateSnapshot { snapshot: Value },
    #[serde(rename_all = "camelCase")]
    StateDelta { delta: Value },
    #[serde(rename_all = "camelCase")]
    MessagesSnapshot { messages: Vec<Value> },
    #[serde(rename_all = "camelCase")]
    ActivitySnapshot {
        message_id: String,
        activity_type: String,
        content: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        replace: Option<bool>,
    },
    #[serde(rename_all = "camelCase")]
    ActivityDelta { message_id: String, activity_type: String, patch: Value },
    #[serde(rename_all = "camelCase")]
    SubagentStarted {
        subagent_run_id: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent_tool_call_id: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    SubagentFinished {
        subagent_run_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        result: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        outcome: Option<Value>,
    },
    #[serde(rename_all = "camelCase")]
    SubagentError {
        subagent_run_id: String,
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        code: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Raw {
        event: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Custom { name: String, value: Value },
    /// Any event type this version does not know.
    #[serde(other)]
    Unknown,
}

/// Converts AG-UI events into [`AgentEvent`]s. Keeps the little state AG-UI needs: the id of
/// the current chunked message / tool call, and each call's argument text.
///
/// ```
/// use glowkit_stream::adapters::ag_ui::AgUiAdapter;
/// let mut a = AgUiAdapter::new();
/// let events = a.push_json(r#"{"type":"TEXT_MESSAGE_CONTENT","messageId":"m1","delta":"Hi"}"#).unwrap();
/// assert_eq!(events, vec![glowkit_stream::AgentEvent::text("m1", "Hi")]);
/// // unknown types are ignored, not errors
/// assert!(a.push_json(r#"{"type":"FROM_THE_FUTURE","x":1}"#).unwrap().is_empty());
/// ```
#[derive(Clone, Debug, Default)]
pub struct AgUiAdapter {
    chunk_message: Option<String>,
    chunk_reasoning: Option<String>,
    chunk_tool: Option<String>,
    args: HashMap<String, String>,
    names: HashMap<String, String>,
}

impl AgUiAdapter {
    /// A fresh adapter (one per thread).
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse one JSON event (an SSE `data:` payload) and convert it.
    pub fn push_json(&mut self, json: &str) -> Result<Vec<AgentEvent>, serde_json::Error> {
        let ev: AgUiEvent = serde_json::from_str(json)?;
        Ok(self.push(&ev))
    }

    /// Convert one event.
    pub fn push(&mut self, ev: &AgUiEvent) -> Vec<AgentEvent> {
        use AgUiEvent as E;
        let mut out = Vec::new();
        match ev {
            E::RunStarted { run_id, .. } => {
                out.push(AgentEvent::RunStart { run_id: Some(run_id.clone()).filter(|r| !r.is_empty()) })
            }
            E::RunFinished { outcome, usage, .. } => {
                self.close_chunks(&mut out);
                push_usage(usage, &mut out);
                let stop = match outcome {
                    Some(RunOutcome::Interrupt { interrupts }) => {
                        for i in interrupts {
                            out.push(AgentEvent::ApprovalRequest {
                                id: i.id.clone(),
                                tool_call_id: i.tool_call_id.clone(),
                                title: i.message.clone(),
                                // reasons are often machine words ("tool_approval"); keep prose only
                                description: i.reason.clone().filter(|r| r.contains(' ')),
                                options: Vec::new(),
                            });
                        }
                        StopReason::Interrupted
                    }
                    Some(RunOutcome::Cancelled) => StopReason::Cancelled,
                    _ => StopReason::EndTurn,
                };
                out.push(AgentEvent::RunFinish { stop });
            }
            E::RunError { message, code, usage } => {
                self.close_chunks(&mut out);
                push_usage(usage, &mut out);
                out.push(AgentEvent::Error { message: message.clone(), code: code.clone() });
                out.push(AgentEvent::RunFinish { stop: StopReason::Error });
            }
            E::StepStarted { step_name } => out.push(AgentEvent::StepStart { name: Some(step_name.clone()) }),
            E::StepFinished { step_name } => out.push(AgentEvent::StepEnd { name: Some(step_name.clone()) }),
            E::TextMessageStart { message_id, role } => {
                if role.as_deref().is_none_or(|r| r == "assistant") {
                    out.push(AgentEvent::TextStart { id: message_id.clone() });
                }
            }
            E::TextMessageContent { message_id, delta } => out.push(AgentEvent::text(message_id, delta)),
            E::TextMessageEnd { message_id } => out.push(AgentEvent::TextEnd { id: message_id.clone() }),
            E::TextMessageChunk { message_id, delta, .. } => {
                if let Some(id) = message_id {
                    if self.chunk_message.as_ref() != Some(id) {
                        if let Some(prev) = self.chunk_message.replace(id.clone()) {
                            out.push(AgentEvent::TextEnd { id: prev });
                        }
                    }
                }
                let id = self.chunk_message.clone().unwrap_or_else(|| "text".to_owned());
                if let Some(d) = delta.as_ref().filter(|d| !d.is_empty()) {
                    out.push(AgentEvent::text(id, d));
                }
            }
            E::ToolCallStart { tool_call_id, tool_call_name, .. } => {
                self.close_message_chunk(&mut out);
                self.names.insert(tool_call_id.clone(), tool_call_name.clone());
                out.push(AgentEvent::ToolCallStart { id: tool_call_id.clone(), name: tool_call_name.clone() });
            }
            E::ToolCallArgs { tool_call_id, delta } => {
                self.args.entry(tool_call_id.clone()).or_default().push_str(delta);
                out.push(AgentEvent::ToolArgsDelta { id: tool_call_id.clone(), delta: delta.clone() });
            }
            E::ToolCallEnd { tool_call_id } => self.end_tool(tool_call_id, &mut out),
            E::ToolCallChunk { tool_call_id, tool_call_name, delta, .. } => {
                if let Some(id) = tool_call_id {
                    if self.chunk_tool.as_ref() != Some(id) {
                        if let Some(prev) = self.chunk_tool.replace(id.clone()) {
                            self.end_tool(&prev, &mut out);
                        }
                        self.close_message_chunk(&mut out);
                        let name = tool_call_name.clone().unwrap_or_default();
                        self.names.insert(id.clone(), name.clone());
                        out.push(AgentEvent::ToolCallStart { id: id.clone(), name });
                    }
                }
                if let (Some(id), Some(d)) = (self.chunk_tool.clone(), delta.as_ref()) {
                    self.args.entry(id.clone()).or_default().push_str(d);
                    out.push(AgentEvent::ToolArgsDelta { id, delta: d.clone() });
                }
            }
            E::ToolCallResult { tool_call_id, content, .. } => {
                if self.chunk_tool.as_ref() == Some(tool_call_id) {
                    self.chunk_tool = None;
                    self.end_tool(tool_call_id, &mut out);
                }
                out.push(AgentEvent::ToolResult {
                    id: tool_call_id.clone(),
                    output: content_value(content),
                    is_error: false,
                });
            }
            E::ReasoningStart { .. } | E::ReasoningMessageStart { .. } | E::ReasoningEncryptedValue { .. } => {}
            E::ReasoningMessageContent { message_id, delta } => {
                out.push(AgentEvent::reasoning(reasoning_id(message_id), delta))
            }
            E::ReasoningMessageEnd { message_id } | E::ReasoningEnd { message_id } => {
                out.push(AgentEvent::ReasoningEnd { id: reasoning_id(message_id) })
            }
            E::ReasoningMessageChunk { message_id, delta } => {
                if let Some(id) = message_id {
                    if self.chunk_reasoning.as_ref() != Some(id) {
                        if let Some(prev) = self.chunk_reasoning.replace(id.clone()) {
                            out.push(AgentEvent::ReasoningEnd { id: prev });
                        }
                    }
                }
                let id = self.chunk_reasoning.clone().unwrap_or_else(|| "reasoning".to_owned());
                if let Some(d) = delta.as_ref().filter(|d| !d.is_empty()) {
                    out.push(AgentEvent::reasoning(id, d));
                }
            }
            E::StateSnapshot { snapshot } => out.push(custom("state_snapshot", snapshot.clone())),
            E::StateDelta { delta } => out.push(custom("state_delta", delta.clone())),
            E::MessagesSnapshot { messages } => out.push(custom("messages_snapshot", Value::Array(messages.clone()))),
            E::ActivitySnapshot { message_id, activity_type, content, .. } => out.push(custom(
                "activity_snapshot",
                json!({"messageId": message_id, "activityType": activity_type, "content": content}),
            )),
            E::ActivityDelta { message_id, activity_type, patch } => out.push(custom(
                "activity_delta",
                json!({"messageId": message_id, "activityType": activity_type, "patch": patch}),
            )),
            E::SubagentStarted { subagent_run_id, name, description, parent_tool_call_id } => {
                if let Some(tc) = parent_tool_call_id {
                    out.push(AgentEvent::ToolRunning { id: tc.clone() });
                }
                out.push(custom(
                    "subagent_started",
                    json!({"subagentRunId": subagent_run_id, "name": name, "description": description}),
                ))
            }
            E::SubagentFinished { subagent_run_id, result, .. } => {
                out.push(custom("subagent_finished", json!({"subagentRunId": subagent_run_id, "result": result})))
            }
            E::SubagentError { subagent_run_id, message, code } => out.push(custom(
                "subagent_error",
                json!({"subagentRunId": subagent_run_id, "message": message, "code": code}),
            )),
            E::Raw { event, source } => out.push(custom("raw", json!({"event": event, "source": source}))),
            E::Custom { name, value } => out.push(AgentEvent::Custom { name: name.clone(), value: value.clone() }),
            E::Unknown => {}
        }
        out
    }

    fn end_tool(&mut self, id: &str, out: &mut Vec<AgentEvent>) {
        let args = self.args.remove(id).unwrap_or_default();
        let input = if args.trim().is_empty() { Some(json!({})) } else { serde_json::from_str(&args).ok() };
        if let Some(input) = input {
            out.push(AgentEvent::ToolInputReady { id: id.to_owned(), name: self.names.get(id).cloned(), input });
        }
    }

    fn close_message_chunk(&mut self, out: &mut Vec<AgentEvent>) {
        if let Some(prev) = self.chunk_message.take() {
            out.push(AgentEvent::TextEnd { id: prev });
        }
    }

    fn close_chunks(&mut self, out: &mut Vec<AgentEvent>) {
        self.close_message_chunk(out);
        if let Some(prev) = self.chunk_reasoning.take() {
            out.push(AgentEvent::ReasoningEnd { id: prev });
        }
        if let Some(prev) = self.chunk_tool.take() {
            self.end_tool(&prev, out);
        }
    }
}

fn reasoning_id(message_id: &str) -> String {
    if message_id.is_empty() {
        "reasoning".to_owned()
    } else {
        message_id.to_owned()
    }
}

fn custom(name: &str, value: Value) -> AgentEvent {
    AgentEvent::Custom { name: format!("ag-ui/{name}"), value }
}

/// Tool results are text (often JSON text) or a list of content parts.
fn content_value(content: &Value) -> Value {
    match content {
        Value::String(s) => serde_json::from_str::<Value>(s)
            .ok()
            .filter(|v| v.is_object() || v.is_array())
            .unwrap_or_else(|| Value::String(s.clone())),
        Value::Array(parts) => {
            let texts: Vec<&str> = parts.iter().filter_map(|p| p.get("text").and_then(Value::as_str)).collect();
            if texts.len() == parts.len() {
                content_value(&Value::String(texts.join("\n")))
            } else {
                content.clone()
            }
        }
        other => other.clone(),
    }
}

fn push_usage(usage: &[TokenUsage], out: &mut Vec<AgentEvent>) {
    if usage.is_empty() {
        return;
    }
    let sum = |f: fn(&TokenUsage) -> Option<u64>| {
        let vals: Vec<u64> = usage.iter().filter_map(f).collect();
        (!vals.is_empty()).then(|| vals.iter().sum())
    };
    out.push(AgentEvent::Usage(Usage {
        input_tokens: sum(|u| u.input_tokens),
        output_tokens: sum(|u| u.output_tokens),
        total_tokens: sum(|u| u.total_tokens),
        ..Default::default()
    }));
}

/// Build the JSON body of an AG-UI run request (`RunAgentInput`).
///
/// `messages` are AG-UI messages (`{"id", "role", "content"}`); `resume` answers the interrupts
/// of the previous run (see [`resume_entry`]).
pub fn run_input(thread_id: &str, run_id: &str, messages: Vec<Value>, resume: Vec<Value>) -> Value {
    let mut v = json!({
        "threadId": thread_id,
        "runId": run_id,
        "protocolVersion": "1.0",
        "messages": messages,
        "tools": [],
        "context": [],
        "state": {},
        "forwardedProps": {},
    });
    if !resume.is_empty() {
        v["resume"] = Value::Array(resume);
    }
    v
}

/// A resume entry answering interrupt `interrupt_id` with `{"approved": bool}`.
pub fn resume_entry(interrupt_id: &str, approved: bool) -> Value {
    json!({"interruptId": interrupt_id, "status": "resolved", "payload": {"approved": approved}})
}

/// The conversation so far as AG-UI messages, ready for [`run_input`]: user and assistant text,
/// assistant tool calls (`toolCalls`) and tool results (`role: "tool"`).
pub fn messages(session: &crate::Session) -> Vec<Value> {
    use crate::{Part, Role};
    let mut out = Vec::new();
    for m in &session.messages {
        match m.role {
            Role::User => out.push(json!({"id": m.id, "role": "user", "content": m.text()})),
            Role::Assistant => {
                let calls: Vec<Value> = m
                    .parts
                    .iter()
                    .filter_map(|p| match p {
                        Part::ToolCall(t) => Some(json!({"id": t.id, "type": "function", "function": {
                            "name": t.name,
                            "arguments": t.input.as_ref().map(Value::to_string).unwrap_or_else(|| t.args_text.clone()),
                        }})),
                        _ => None,
                    })
                    .collect();
                let mut msg = json!({"id": m.id, "role": "assistant", "content": m.text()});
                if !calls.is_empty() {
                    msg["toolCalls"] = Value::Array(calls);
                }
                out.push(msg);
                for p in &m.parts {
                    if let Part::ToolCall(t) = p {
                        if let Some(o) = &t.output {
                            let content = match o {
                                Value::String(s) => s.clone(),
                                other => other.to_string(),
                            };
                            out.push(json!({"id": format!("{}-result", t.id), "role": "tool", "toolCallId": t.id, "content": content}));
                        }
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{Session, ToolState};

    fn run(lines: &[&str]) -> Vec<AgentEvent> {
        let mut a = AgUiAdapter::new();
        lines.iter().flat_map(|l| a.push_json(l).expect("valid event")).collect()
    }

    #[test]
    fn wire_casing_is_screaming_snake() {
        let ev = AgUiEvent::TextMessageContent { message_id: "m".into(), delta: "x".into() };
        let s = serde_json::to_string(&ev).unwrap();
        assert_eq!(s, r#"{"type":"TEXT_MESSAGE_CONTENT","messageId":"m","delta":"x"}"#);
        let ev = AgUiEvent::RunStarted { thread_id: "t".into(), run_id: "r".into(), parent_run_id: None };
        assert!(serde_json::to_string(&ev).unwrap().starts_with(r#"{"type":"RUN_STARTED","threadId""#));
    }

    #[test]
    fn thinking_aliases_map_to_reasoning() {
        let evs = run(&[
            r#"{"type":"THINKING_START"}"#,
            r#"{"type":"THINKING_TEXT_MESSAGE_CONTENT","delta":"hmm"}"#,
            r#"{"type":"THINKING_END"}"#,
        ]);
        assert_eq!(
            evs,
            vec![AgentEvent::reasoning("reasoning", "hmm"), AgentEvent::ReasoningEnd { id: "reasoning".into() }]
        );
    }

    #[test]
    fn chunks_open_and_close_parts() {
        let evs = run(&[
            r#"{"type":"TEXT_MESSAGE_CHUNK","messageId":"m1","role":"assistant","delta":"Hel"}"#,
            r#"{"type":"TEXT_MESSAGE_CHUNK","delta":"lo"}"#,
            r#"{"type":"TOOL_CALL_CHUNK","toolCallId":"c1","toolCallName":"search","delta":"{\"q\":"}"#,
            r#"{"type":"TOOL_CALL_CHUNK","delta":"\"egui\"}"}"#,
            r#"{"type":"TOOL_CALL_RESULT","messageId":"r1","toolCallId":"c1","content":"{\"hits\":3}"}"#,
        ]);
        let mut s = Session::new();
        s.extend(evs.clone());
        assert_eq!(s.messages[0].text(), "Hello");
        let t = s.tool_call("c1").unwrap();
        assert_eq!(t.name, "search");
        assert!(t.input_final);
        assert_eq!(t.input, Some(json!({"q": "egui"})));
        assert_eq!(t.output, Some(json!({"hits": 3})));
        assert!(evs.contains(&AgentEvent::TextEnd { id: "m1".into() }));
    }

    #[test]
    fn interrupt_outcome_becomes_approval() {
        let evs = run(&[
            r#"{"type":"RUN_STARTED","threadId":"t","runId":"r"}"#,
            r#"{"type":"TOOL_CALL_START","toolCallId":"c","toolCallName":"pay"}"#,
            r#"{"type":"TOOL_CALL_END","toolCallId":"c"}"#,
            r#"{"type":"RUN_FINISHED","threadId":"t","runId":"r","outcome":{"type":"interrupt","interrupts":[{"id":"i1","reason":"tool_approval","message":"Pay 5 EUR?","toolCallId":"c"}]},"usage":[{"inputTokens":10,"outputTokens":2},{"inputTokens":5}]}"#,
        ]);
        let mut s = Session::new();
        s.extend(evs);
        let a = s.pending_approvals().next().unwrap();
        assert_eq!(a.title, "Pay 5 EUR?");
        assert_eq!(a.tool_call_id.as_deref(), Some("c"));
        assert_eq!(s.tool_call("c").unwrap().state, ToolState::Pending);
        assert_eq!(s.usage.input_tokens, Some(15));
        assert_eq!(s.status, crate::session::RunStatus::Finished(StopReason::Interrupted));
    }

    #[test]
    fn unknown_and_custom_events() {
        let evs = run(&[
            r#"{"type":"SOMETHING_NEW","foo":[1,2]}"#,
            r#"{"type":"CUSTOM","name":"progress","value":{"pct":40}}"#,
            r#"{"type":"STATE_DELTA","delta":[{"op":"add","path":"/a","value":1}]}"#,
            r#"{"type":"RUN_ERROR","message":"boom","code":"E1"}"#,
        ]);
        assert_eq!(evs.len(), 4);
        assert!(matches!(&evs[1], AgentEvent::Custom { name, .. } if name == "ag-ui/state_delta"));
        assert_eq!(evs[3], AgentEvent::RunFinish { stop: StopReason::Error });
    }

    #[test]
    fn history_messages() {
        let mut s = Session::new();
        s.push_user("hi");
        s.extend(run(&[
            r#"{"type":"TEXT_MESSAGE_CONTENT","messageId":"m","delta":"Sure."}"#,
            r#"{"type":"TOOL_CALL_START","toolCallId":"c","toolCallName":"f"}"#,
            r#"{"type":"TOOL_CALL_ARGS","toolCallId":"c","delta":"{}"}"#,
            r#"{"type":"TOOL_CALL_END","toolCallId":"c"}"#,
            r#"{"type":"TOOL_CALL_RESULT","messageId":"r","toolCallId":"c","content":"ok"}"#,
        ]));
        let m = messages(&s);
        assert_eq!(m.len(), 3);
        assert_eq!(m[0]["role"], "user");
        assert_eq!(m[1]["toolCalls"][0]["function"]["name"], "f");
        assert_eq!(m[2]["toolCallId"], "c");
    }

    #[test]
    fn run_input_shape() {
        let v = run_input("t", "r2", vec![], vec![resume_entry("i1", true)]);
        assert_eq!(v["resume"][0]["interruptId"], "i1");
        assert_eq!(v["resume"][0]["payload"]["approved"], true);
    }
}
