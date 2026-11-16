//! The Vercel AI SDK v7 UI message stream (`x-vercel-ai-ui-message-stream: v1`): SSE lines
//! `data: {json}` ending with `data: [DONE]`, as produced by `toUIMessageStreamResponse()`.
//!
//! The part types here are written from the SDK's documented chunk schema; unknown part types
//! are ignored.

use crate::event::{AgentEvent, SourceRef, StopReason};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One UI message stream part, except `data-*` parts (see [`VercelAdapter::push_value`]).
#[allow(missing_docs)] // field names mirror the protocol
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum VercelPart {
    #[serde(rename_all = "camelCase")]
    Start {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message_metadata: Option<Value>,
    },
    #[serde(rename_all = "camelCase")]
    Finish {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        finish_reason: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message_metadata: Option<Value>,
    },
    #[serde(rename_all = "camelCase")]
    Abort {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Error {
        error_text: String,
    },
    StartStep,
    FinishStep,
    #[serde(rename_all = "camelCase")]
    TextStart {
        id: String,
    },
    #[serde(rename_all = "camelCase")]
    TextDelta {
        id: String,
        delta: String,
    },
    #[serde(rename_all = "camelCase")]
    TextEnd {
        id: String,
    },
    #[serde(rename_all = "camelCase")]
    ReasoningStart {
        id: String,
    },
    #[serde(rename_all = "camelCase")]
    ReasoningDelta {
        id: String,
        delta: String,
    },
    #[serde(rename_all = "camelCase")]
    ReasoningEnd {
        id: String,
    },
    #[serde(rename_all = "camelCase")]
    ToolInputStart {
        tool_call_id: String,
        tool_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    ToolInputDelta {
        tool_call_id: String,
        input_text_delta: String,
    },
    #[serde(rename_all = "camelCase")]
    ToolInputAvailable {
        tool_call_id: String,
        tool_name: String,
        #[serde(default)]
        input: Value,
    },
    #[serde(rename_all = "camelCase")]
    ToolInputError {
        tool_call_id: String,
        tool_name: String,
        #[serde(default)]
        input: Value,
        error_text: String,
    },
    #[serde(rename_all = "camelCase")]
    ToolApprovalRequest {
        approval_id: String,
        tool_call_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    ToolApprovalResponse {
        approval_id: String,
        approved: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    ToolOutputAvailable {
        tool_call_id: String,
        #[serde(default)]
        output: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        preliminary: Option<bool>,
    },
    #[serde(rename_all = "camelCase")]
    ToolOutputError {
        tool_call_id: String,
        error_text: String,
    },
    #[serde(rename_all = "camelCase")]
    ToolOutputDenied {
        tool_call_id: String,
    },
    #[serde(rename_all = "camelCase")]
    SourceUrl {
        source_id: String,
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    SourceDocument {
        source_id: String,
        #[serde(default)]
        media_type: String,
        #[serde(default)]
        title: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filename: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    File {
        url: String,
        media_type: String,
    },
    #[serde(rename_all = "camelCase")]
    Custom {
        kind: String,
    },
    #[serde(rename_all = "camelCase")]
    MessageMetadata {
        message_metadata: Value,
    },
    /// A part type this version does not know.
    #[serde(other)]
    Unknown,
}

/// Converts UI message stream parts into [`AgentEvent`]s.
///
/// ```
/// use glowkit_stream::adapters::vercel::VercelAdapter;
/// let mut a = VercelAdapter::new();
/// let ev = a.push_json(r#"{"type":"text-delta","id":"t0","delta":"Hi"}"#).unwrap();
/// assert_eq!(ev, vec![glowkit_stream::AgentEvent::text("t0", "Hi")]);
/// assert!(a.push_json("[DONE]").unwrap().is_empty());
/// ```
#[derive(Clone, Debug, Default)]
pub struct VercelAdapter {
    step: u32,
}

impl VercelAdapter {
    /// A fresh adapter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse one SSE `data:` payload. `[DONE]` yields nothing.
    pub fn push_json(&mut self, data: &str) -> Result<Vec<AgentEvent>, serde_json::Error> {
        if data.trim() == "[DONE]" {
            return Ok(Vec::new());
        }
        let v: Value = serde_json::from_str(data)?;
        Ok(self.push_value(&v))
    }

    /// Convert one part given as JSON: handles `data-*` parts, then the typed ones.
    pub fn push_value(&mut self, v: &Value) -> Vec<AgentEvent> {
        let ty = v.get("type").and_then(Value::as_str).unwrap_or_default();
        if let Some(name) = ty.strip_prefix("data-") {
            return vec![AgentEvent::Custom {
                name: format!("vercel/data-{name}"),
                value: v.get("data").cloned().unwrap_or(Value::Null),
            }];
        }
        if ty == "custom" {
            return vec![AgentEvent::Custom { name: "vercel/custom".into(), value: v.clone() }];
        }
        match serde_json::from_value::<VercelPart>(v.clone()) {
            Ok(p) => self.push(&p),
            Err(_) => Vec::new(),
        }
    }

    /// Convert one typed part.
    pub fn push(&mut self, part: &VercelPart) -> Vec<AgentEvent> {
        use VercelPart as P;
        let one = |e: AgentEvent| vec![e];
        match part {
            P::Start { message_id, .. } => {
                self.step = 0;
                one(AgentEvent::RunStart { run_id: message_id.clone() })
            }
            P::Finish { finish_reason, .. } => {
                one(AgentEvent::RunFinish { stop: StopReason::from_wire(finish_reason.as_deref().unwrap_or("stop")) })
            }
            P::Abort { .. } => one(AgentEvent::RunFinish { stop: StopReason::Cancelled }),
            P::Error { error_text } => {
                vec![AgentEvent::error(error_text.clone()), AgentEvent::RunFinish { stop: StopReason::Error }]
            }
            P::StartStep => {
                self.step += 1;
                one(AgentEvent::StepStart { name: Some(format!("Step {}", self.step)) })
            }
            P::FinishStep => one(AgentEvent::StepEnd { name: None }),
            P::TextStart { id } => one(AgentEvent::TextStart { id: id.clone() }),
            P::TextDelta { id, delta } => one(AgentEvent::text(id, delta)),
            P::TextEnd { id } => one(AgentEvent::TextEnd { id: id.clone() }),
            P::ReasoningStart { .. } => Vec::new(),
            P::ReasoningDelta { id, delta } => one(AgentEvent::reasoning(id, delta)),
            P::ReasoningEnd { id } => one(AgentEvent::ReasoningEnd { id: id.clone() }),
            P::ToolInputStart { tool_call_id, tool_name, .. } => {
                one(AgentEvent::ToolCallStart { id: tool_call_id.clone(), name: tool_name.clone() })
            }
            P::ToolInputDelta { tool_call_id, input_text_delta } => {
                one(AgentEvent::ToolArgsDelta { id: tool_call_id.clone(), delta: input_text_delta.clone() })
            }
            P::ToolInputAvailable { tool_call_id, tool_name, input } => one(AgentEvent::ToolInputReady {
                id: tool_call_id.clone(),
                name: Some(tool_name.clone()),
                input: input.clone(),
            }),
            P::ToolInputError { tool_call_id, tool_name, input, error_text } => vec![
                AgentEvent::ToolInputReady {
                    id: tool_call_id.clone(),
                    name: Some(tool_name.clone()),
                    input: input.clone(),
                },
                AgentEvent::ToolResult {
                    id: tool_call_id.clone(),
                    output: Value::String(error_text.clone()),
                    is_error: true,
                },
            ],
            P::ToolApprovalRequest { approval_id, tool_call_id, reason } => one(AgentEvent::ApprovalRequest {
                id: approval_id.clone(),
                tool_call_id: Some(tool_call_id.clone()),
                title: None,
                description: reason.clone(),
                options: Vec::new(),
            }),
            P::ToolApprovalResponse { approval_id, approved, reason } => one(AgentEvent::ApprovalDecision {
                id: approval_id.clone(),
                approved: *approved,
                reason: reason.clone(),
            }),
            P::ToolOutputAvailable { tool_call_id, output, preliminary } => {
                if *preliminary == Some(true) {
                    one(AgentEvent::ToolRunning { id: tool_call_id.clone() })
                } else {
                    one(AgentEvent::ToolResult { id: tool_call_id.clone(), output: output.clone(), is_error: false })
                }
            }
            P::ToolOutputError { tool_call_id, error_text } => one(AgentEvent::ToolResult {
                id: tool_call_id.clone(),
                output: Value::String(error_text.clone()),
                is_error: true,
            }),
            P::ToolOutputDenied { tool_call_id } => one(AgentEvent::decision(tool_call_id.clone(), false)),
            P::SourceUrl { source_id, url, title } => one(AgentEvent::Source(SourceRef {
                id: source_id.clone(),
                title: title.clone().unwrap_or_default(),
                url: url.clone(),
                snippet: String::new(),
            })),
            P::SourceDocument { source_id, title, filename, .. } => one(AgentEvent::Source(SourceRef {
                id: source_id.clone(),
                title: title.clone(),
                url: String::new(),
                snippet: filename.clone().unwrap_or_default(),
            })),
            P::File { url, media_type } => one(AgentEvent::Custom {
                name: "vercel/file".into(),
                value: serde_json::json!({"url": url, "mediaType": media_type}),
            }),
            P::Custom { kind } => one(AgentEvent::Custom { name: format!("vercel/custom/{kind}"), value: Value::Null }),
            P::MessageMetadata { message_metadata } => {
                one(AgentEvent::Custom { name: "vercel/message-metadata".into(), value: message_metadata.clone() })
            }
            P::Unknown => Vec::new(),
        }
    }
}

/// The `approval` object an AI SDK client puts on the tool part when the user answers
/// (`{"id": approvalId, "approved": bool}`); resend the conversation with it so the backend runs
/// or skips the tool.
pub fn approval_json(approval_id: &str, approved: bool) -> Value {
    serde_json::json!({"id": approval_id, "approved": approved})
}

/// The conversation so far as AI SDK `UIMessage`s (`{"id", "role", "parts"}`), the body your
/// `useChat`-style route expects as `messages`. Tool parts carry their state, and an answered
/// approval is sent as `approval-responded` so the backend runs (or skips) the tool.
pub fn ui_messages(session: &crate::Session) -> Vec<Value> {
    use crate::{Part, Role, ToolState};
    use serde_json::json;
    session
        .messages
        .iter()
        .map(|m| {
            let role = if m.role == Role::User { "user" } else { "assistant" };
            let approvals: Vec<&crate::ApprovalPart> = m
                .parts
                .iter()
                .filter_map(|p| match p {
                    Part::Approval(a) => Some(a),
                    _ => None,
                })
                .collect();
            let parts: Vec<Value> = m
                .parts
                .iter()
                .filter_map(|p| match p {
                    Part::Text(t) => Some(json!({"type": "text", "text": t.text})),
                    Part::Reasoning(r) => Some(json!({"type": "reasoning", "text": r.text})),
                    Part::ToolCall(t) => {
                        let approval = approvals.iter().find(|a| a.tool_call_id.as_deref() == Some(&t.id));
                        let mut part = json!({
                            "type": format!("tool-{}", t.name),
                            "toolCallId": t.id,
                            "input": t.input.clone().unwrap_or(Value::Null),
                        });
                        let state = match (t.state, approval.map(|a| a.decision)) {
                            (ToolState::Done, _) => {
                                part["output"] = t.output.clone().unwrap_or(Value::Null);
                                "output-available"
                            }
                            (ToolState::Error, _) => {
                                part["errorText"] = Value::String(t.error.clone().unwrap_or_default());
                                "output-error"
                            }
                            (ToolState::Denied, _) | (_, Some(Some(_))) => "approval-responded",
                            (_, Some(None)) => "approval-requested",
                            _ => "input-available",
                        };
                        if let Some(a) = approval {
                            part["approval"] = match a.decision {
                                Some(ok) => json!({"id": a.id, "approved": ok}),
                                None => json!({"id": a.id}),
                            };
                        }
                        part["state"] = Value::String(state.into());
                        Some(part)
                    }
                    _ => None,
                })
                .collect();
            json!({"id": m.id, "role": role, "parts": parts})
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{Session, ToolState};
    use serde_json::json;

    fn run(lines: &[&str]) -> Session {
        let mut a = VercelAdapter::new();
        let mut s = Session::new();
        for l in lines {
            s.extend(a.push_json(l).expect("valid part"));
        }
        s
    }

    #[test]
    fn kebab_case_wire_names() {
        let p = VercelPart::ToolInputDelta { tool_call_id: "c".into(), input_text_delta: "{".into() };
        assert_eq!(
            serde_json::to_string(&p).unwrap(),
            r#"{"type":"tool-input-delta","toolCallId":"c","inputTextDelta":"{"}"#
        );
        assert_eq!(serde_json::to_string(&VercelPart::StartStep).unwrap(), r#"{"type":"start-step"}"#);
    }

    #[test]
    fn tool_lifecycle_and_denial() {
        let s = run(&[
            r#"{"type":"start","messageId":"m"}"#,
            r#"{"type":"tool-input-start","toolCallId":"a","toolName":"weather"}"#,
            r#"{"type":"tool-input-delta","toolCallId":"a","inputTextDelta":"{\"city\":"}"#,
            r#"{"type":"tool-input-delta","toolCallId":"a","inputTextDelta":"\"Rome\"}"}"#,
            r#"{"type":"tool-input-available","toolCallId":"a","toolName":"weather","input":{"city":"Rome"}}"#,
            r#"{"type":"tool-output-available","toolCallId":"a","output":{"temp":21},"preliminary":true}"#,
            r#"{"type":"tool-output-available","toolCallId":"a","output":{"temp":22}}"#,
            r#"{"type":"tool-input-start","toolCallId":"b","toolName":"rm"}"#,
            r#"{"type":"tool-approval-request","approvalId":"ap","toolCallId":"b"}"#,
            r#"{"type":"tool-output-denied","toolCallId":"b"}"#,
            r#"{"type":"tool-input-error","toolCallId":"c","toolName":"x","input":"{bad","errorText":"invalid json"}"#,
            r#"{"type":"finish","finishReason":"tool-calls"}"#,
            "[DONE]",
        ]);
        let a = s.tool_call("a").unwrap();
        assert_eq!((a.state, a.output.clone()), (ToolState::Done, Some(json!({"temp": 22}))));
        assert_eq!(s.tool_call("b").unwrap().state, ToolState::Denied);
        assert_eq!(s.approval("ap").unwrap().decision, Some(false));
        assert_eq!(s.tool_call("c").unwrap().state, ToolState::Error);
        assert_eq!(s.status, crate::session::RunStatus::Finished(StopReason::ToolCalls));
    }

    #[test]
    fn ui_messages_carry_approval_answers() {
        let mut s = run(&[
            r#"{"type":"text-delta","id":"t","delta":"Booking."}"#,
            r#"{"type":"tool-input-available","toolCallId":"b","toolName":"book","input":{"id":1}}"#,
            r#"{"type":"tool-approval-request","approvalId":"ap","toolCallId":"b"}"#,
        ]);
        s.push_user("ok");
        let before = ui_messages(&s);
        assert_eq!(before[0]["parts"][1]["state"], "approval-requested");
        s.apply(AgentEvent::decision("ap", true));
        let after = ui_messages(&s);
        assert_eq!(after[0]["parts"][1]["type"], "tool-book");
        assert_eq!(after[0]["parts"][1]["state"], "approval-responded");
        assert_eq!(after[0]["parts"][1]["approval"]["approved"], true);
        assert_eq!(after[1]["role"], "user");
    }

    #[test]
    fn data_custom_unknown_and_sources() {
        let mut a = VercelAdapter::new();
        let ev = a.push_json(r#"{"type":"data-weather","id":"w","data":{"temp":3}}"#).unwrap();
        assert_eq!(ev, vec![AgentEvent::Custom { name: "vercel/data-weather".into(), value: json!({"temp": 3}) }]);
        assert!(a.push_json(r#"{"type":"brand-new-part","x":1}"#).unwrap().is_empty());
        assert!(a.push_json(r#"{"type":"text-delta"}"#).unwrap().is_empty(), "malformed parts are skipped");
        let ev =
            a.push_json(r#"{"type":"source-url","sourceId":"s","url":"https://example.org","title":"Ex"}"#).unwrap();
        assert!(matches!(&ev[0], AgentEvent::Source(s) if s.title == "Ex"));
        let ev = a.push_json(r#"{"type":"tool-approval-response","approvalId":"ap","approved":true}"#).unwrap();
        assert_eq!(ev, vec![AgentEvent::decision("ap", true)]);
    }
}
