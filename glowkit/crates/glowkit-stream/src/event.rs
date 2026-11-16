//! The normalized event model every adapter produces.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Why a run ended.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "reason", content = "detail")]
pub enum StopReason {
    /// The agent finished its turn normally.
    #[default]
    EndTurn,
    /// The model hit its output token limit.
    MaxTokens,
    /// The agent hit its turn or request budget.
    MaxTurns,
    /// The model stopped to call tools (one turn of a longer run).
    ToolCalls,
    /// The run is paused, waiting for outside input such as an approval.
    Interrupted,
    /// The run was cancelled by the client or the user.
    Cancelled,
    /// The model refused to continue.
    Refusal,
    /// The run failed (an [`AgentEvent::Error`] usually precedes it).
    Error,
    /// Anything else, with the source's own wording.
    Other(String),
}

impl StopReason {
    /// A short human-readable label.
    pub fn label(&self) -> &str {
        match self {
            StopReason::EndTurn => "Finished",
            StopReason::MaxTokens => "Stopped: token limit",
            StopReason::MaxTurns => "Stopped: turn limit",
            StopReason::ToolCalls => "Waiting for tools",
            StopReason::Interrupted => "Waiting for input",
            StopReason::Cancelled => "Cancelled",
            StopReason::Refusal => "Refused",
            StopReason::Error => "Failed",
            StopReason::Other(s) => s,
        }
    }

    /// Map a common wire spelling (`"stop"`, `"end_turn"`, `"tool-calls"`, `"length"`...) to a
    /// reason. Unknown spellings become [`StopReason::Other`].
    pub fn from_wire(s: &str) -> Self {
        match s.to_ascii_lowercase().replace('-', "_").as_str() {
            "stop" | "end_turn" | "endturn" | "completed" | "success" | "end" => StopReason::EndTurn,
            "length" | "max_tokens" | "max_output_tokens" => StopReason::MaxTokens,
            "max_turn_requests" | "max_turns" | "max_steps" => StopReason::MaxTurns,
            "tool_calls" | "tool_use" | "function_call" => StopReason::ToolCalls,
            "interrupt" | "interrupted" | "paused" | "suspended" => StopReason::Interrupted,
            "cancelled" | "canceled" | "abort" | "aborted" => StopReason::Cancelled,
            "refusal" | "content_filter" => StopReason::Refusal,
            "error" | "failed" => StopReason::Error,
            other => StopReason::Other(other.to_owned()),
        }
    }
}

/// Status of one plan entry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    /// Not started.
    #[default]
    Pending,
    /// Being worked on.
    InProgress,
    /// Done.
    Completed,
    /// Gave up or failed.
    Failed,
}

/// One entry of an agent's plan / todo list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanEntry {
    /// What the step is.
    pub content: String,
    /// Its progress.
    #[serde(default)]
    pub status: PlanStatus,
}

impl PlanEntry {
    /// An entry with `content` and `status`.
    pub fn new(content: impl Into<String>, status: PlanStatus) -> Self {
        Self { content: content.into(), status }
    }
}

/// A source or citation the agent used.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRef {
    /// Stable id, used to de-duplicate repeated citations.
    pub id: String,
    /// Display title.
    #[serde(default)]
    pub title: String,
    /// Link (empty for documents without one).
    #[serde(default)]
    pub url: String,
    /// Optional excerpt.
    #[serde(default)]
    pub snippet: String,
}

/// Token usage. Every field is optional because sources report different subsets; the
/// [`Session`](crate::Session) keeps the latest non-empty value of each field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    /// Prompt / input tokens.
    #[serde(default)]
    pub input_tokens: Option<u64>,
    /// Completion / output tokens.
    #[serde(default)]
    pub output_tokens: Option<u64>,
    /// Total tokens.
    #[serde(default)]
    pub total_tokens: Option<u64>,
    /// Tokens currently in the context window.
    #[serde(default)]
    pub context_used: Option<u64>,
    /// Size of the context window.
    #[serde(default)]
    pub context_size: Option<u64>,
}

impl Usage {
    /// `input + output` when both are known, else `total_tokens`.
    pub fn total(&self) -> Option<u64> {
        self.total_tokens.or(match (self.input_tokens, self.output_tokens) {
            (Some(i), Some(o)) => Some(i + o),
            _ => None,
        })
    }

    /// Overwrite each field that `other` reports.
    pub fn merge(&mut self, other: &Usage) {
        let pairs = [
            (&mut self.input_tokens, other.input_tokens),
            (&mut self.output_tokens, other.output_tokens),
            (&mut self.total_tokens, other.total_tokens),
            (&mut self.context_used, other.context_used),
            (&mut self.context_size, other.context_size),
        ];
        for (dst, src) in pairs {
            if src.is_some() {
                *dst = src;
            }
        }
    }
}

/// One option offered by an approval request (ACP offers "allow once", "always allow"...).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalOption {
    /// The id to send back when the user picks it.
    pub id: String,
    /// Button label.
    pub label: String,
    /// Whether picking it approves (true) or rejects (false) the request.
    pub approves: bool,
}

/// A normalized agent / LLM stream event. Every adapter converts its source's stream into these,
/// and a [`Session`](crate::Session) folds them into messages.
///
/// Ids are whatever the source uses. Events may arrive out of order or without their start
/// event; the session copes (a delta for an unknown text id simply opens it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// A run (one request to the agent) started.
    RunStart {
        /// Source run id, if any.
        #[serde(default)]
        run_id: Option<String>,
    },
    /// The run ended.
    RunFinish {
        /// Why it ended.
        #[serde(default)]
        stop: StopReason,
    },
    /// A named step (graph node, model turn...) started.
    StepStart {
        /// Step name.
        #[serde(default)]
        name: Option<String>,
    },
    /// A step ended.
    StepEnd {
        /// Step name.
        #[serde(default)]
        name: Option<String>,
    },
    /// A text part opened.
    TextStart {
        /// Part id.
        id: String,
    },
    /// Text was appended to a part.
    TextDelta {
        /// Part id.
        id: String,
        /// The new text.
        delta: String,
    },
    /// A text part is complete.
    TextEnd {
        /// Part id.
        id: String,
    },
    /// Reasoning text was appended (opens the part when needed).
    ReasoningDelta {
        /// Part id.
        id: String,
        /// The new text.
        delta: String,
    },
    /// A reasoning part is complete.
    ReasoningEnd {
        /// Part id.
        id: String,
    },
    /// The model started a tool call.
    ToolCallStart {
        /// Call id.
        id: String,
        /// Tool name.
        name: String,
    },
    /// A fragment of the call's JSON arguments.
    ToolArgsDelta {
        /// Call id.
        id: String,
        /// Raw JSON text fragment.
        delta: String,
    },
    /// The call's arguments are complete and parsed.
    ToolInputReady {
        /// Call id.
        id: String,
        /// Tool name, when the source repeats it here.
        #[serde(default)]
        name: Option<String>,
        /// Parsed arguments.
        input: Value,
    },
    /// The tool started executing (approved, or no approval needed).
    ToolRunning {
        /// Call id.
        id: String,
    },
    /// The tool finished.
    ToolResult {
        /// Call id.
        id: String,
        /// Output (a string, or any JSON).
        output: Value,
        /// Whether the tool failed.
        #[serde(default)]
        is_error: bool,
    },
    /// The agent asks the user before doing something.
    ApprovalRequest {
        /// Approval id (often the tool call id).
        id: String,
        /// The tool call this approval gates, if any.
        #[serde(default)]
        tool_call_id: Option<String>,
        /// Short question, e.g. "Book this flight?".
        #[serde(default)]
        title: Option<String>,
        /// Longer explanation.
        #[serde(default)]
        description: Option<String>,
        /// The choices offered. Empty means plain approve / deny.
        #[serde(default)]
        options: Vec<ApprovalOption>,
    },
    /// An approval was answered (by the user here, or reported by the source).
    ApprovalDecision {
        /// Approval id.
        id: String,
        /// Approved or denied.
        approved: bool,
        /// Optional reason.
        #[serde(default)]
        reason: Option<String>,
    },
    /// The agent's current plan (replaces the previous one).
    Plan {
        /// Plan entries in order.
        entries: Vec<PlanEntry>,
    },
    /// A source / citation.
    Source(SourceRef),
    /// Token usage update.
    Usage(Usage),
    /// Something went wrong.
    Error {
        /// Human-readable message.
        message: String,
        /// Machine-readable code.
        #[serde(default)]
        code: Option<String>,
    },
    /// Anything source-specific: state snapshots, activity, sub-agents, data parts...
    Custom {
        /// Event name, e.g. `"ag-ui/state_snapshot"`.
        name: String,
        /// Payload.
        value: Value,
    },
}

impl AgentEvent {
    /// Shorthand for [`AgentEvent::TextDelta`].
    pub fn text(id: impl Into<String>, delta: impl Into<String>) -> Self {
        AgentEvent::TextDelta { id: id.into(), delta: delta.into() }
    }

    /// Shorthand for [`AgentEvent::ReasoningDelta`].
    pub fn reasoning(id: impl Into<String>, delta: impl Into<String>) -> Self {
        AgentEvent::ReasoningDelta { id: id.into(), delta: delta.into() }
    }

    /// Shorthand for [`AgentEvent::Error`] without a code.
    pub fn error(message: impl Into<String>) -> Self {
        AgentEvent::Error { message: message.into(), code: None }
    }

    /// Shorthand for an approval decision.
    pub fn decision(id: impl Into<String>, approved: bool) -> Self {
        AgentEvent::ApprovalDecision { id: id.into(), approved, reason: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_reason_from_wire() {
        assert_eq!(StopReason::from_wire("stop"), StopReason::EndTurn);
        assert_eq!(StopReason::from_wire("tool-calls"), StopReason::ToolCalls);
        assert_eq!(StopReason::from_wire("max_tokens"), StopReason::MaxTokens);
        assert_eq!(StopReason::from_wire("weird"), StopReason::Other("weird".into()));
    }

    #[test]
    fn events_roundtrip_through_json() {
        let ev = AgentEvent::ToolInputReady { id: "c1".into(), name: None, input: serde_json::json!({"a": 1}) };
        let s = serde_json::to_string(&ev).unwrap();
        assert!(s.contains("\"type\":\"tool_input_ready\""));
        assert_eq!(serde_json::from_str::<AgentEvent>(&s).unwrap(), ev);
    }

    #[test]
    fn usage_merge_keeps_known_fields() {
        let mut u = Usage { input_tokens: Some(10), ..Default::default() };
        u.merge(&Usage { output_tokens: Some(5), ..Default::default() });
        assert_eq!(u.total(), Some(15));
    }
}
