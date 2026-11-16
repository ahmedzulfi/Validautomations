//! Recorded agent runs bundled with the crate, for demos, offline previews and tests.
//!
//! Each one is a realistic exchange with a fictional "Nova 2" model: reasoning, streamed
//! Markdown, two tool calls (the second needs approval) and an error at the end.

#[allow(unused_imports)]
use crate::event::AgentEvent;

/// An AG-UI 1.0 SSE recording: two runs, the second resumes after the booking approval and
/// ends with `RUN_ERROR`.
pub const AG_UI_RUN: &str = include_str!("../fixtures/ag-ui-run.sse");

/// A Vercel AI SDK v7 UI message stream recording: two responses, the second continues after
/// the approval and ends with an `error` part.
pub const VERCEL_RUN: &str = include_str!("../fixtures/vercel-run.sse");

/// An ACP recording: JSON-RPC messages, one per line, of a coding agent fixing a test (plan,
/// file read, an edit behind `session/request_permission`, a failing test run).
pub const ACP_RUN: &str = include_str!("../fixtures/acp-run.jsonl");

/// [`AG_UI_RUN`] decoded into events.
#[cfg(feature = "ag-ui")]
pub fn ag_ui_events() -> Vec<AgentEvent> {
    crate::adapters::SseDecoder::decode_all(crate::adapters::SseProtocol::AgUi, AG_UI_RUN.as_bytes())
}

/// [`VERCEL_RUN`] decoded into events.
#[cfg(feature = "vercel")]
pub fn vercel_events() -> Vec<AgentEvent> {
    crate::adapters::SseDecoder::decode_all(crate::adapters::SseProtocol::Vercel, VERCEL_RUN.as_bytes())
}

/// [`ACP_RUN`] decoded into events.
#[cfg(feature = "acp-wire")]
pub fn acp_events() -> Vec<AgentEvent> {
    let mut a = crate::adapters::acp::AcpAdapter::new();
    ACP_RUN
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .flat_map(|m| a.push_message(&m))
        .collect()
}

/// The prompt the recorded runs answer.
pub const TRAVEL_PROMPT: &str = "Find the cheapest flight from Oslo to Lisbon next Friday and book it.";

/// The prompt of the ACP recording.
pub const CODING_PROMPT: &str = "The tokenizer test fails on empty input. Can you fix it?";
