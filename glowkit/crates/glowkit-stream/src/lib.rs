//! # glowkit-stream
//!
//! Real LLM and agent streams, rendered natively. Adapters turn what your agent speaks (AG-UI,
//! the Vercel AI SDK UI message stream, ACP, async-openai, rig) into one normalized
//! [`AgentEvent`] model; a [`Session`] folds those events into a thread of messages; and with the
//! `egui` feature, [`AgentThread`] draws that thread with the glowkit-ai components.
//!
//! ```text
//! agent / LLM ──► adapter ──► AgentEvent ──► bridge (any thread) ──► Session ──► AgentThread (egui)
//!                                                                      ▲                 │
//!                                                                      └── approvals ◄───┘
//! ```
//!
//! The core (events, [`Session`], [`bridge`], [`sse`], [`replay`]) depends only on serde and has
//! no async runtime and no egui dependency.
//!
//! ```
//! use glowkit_stream::{bridge, AgentEvent, Session};
//! let (tx, mut rx) = bridge::channel();
//! // background task:
//! tx.send(AgentEvent::RunStart { run_id: None }).unwrap();
//! tx.send(AgentEvent::text("t1", "Hello **world**")).unwrap();
//! // UI thread, every frame:
//! let mut session = Session::new();
//! rx.drain_into(&mut session);
//! assert_eq!(session.messages[0].text(), "Hello **world**");
//! ```
//!
//! ## Features
//!
//! | Feature | Adds |
//! |---|---|
//! | `egui` | [`AgentThread`], `EventSender::with_repaint` |
//! | `ag-ui` | AG-UI 1.x adapter (serde only) |
//! | `vercel` | Vercel AI SDK v7 UI message stream adapter (serde only) |
//! | `acp-wire` | ACP JSON-RPC adapter (serde only) |
//! | `acp` | typed ACP adapter for `agent-client-protocol` 5.x |
//! | `async-openai` | Chat Completions + Responses adapters for `async-openai` 0.43 |
//! | `rig` | adapters for `rig-core` / `rig-agent` 0.46 streams |
//! | `http` | `http::stream_sse`: POST to an AG-UI / Vercel endpoint with reqwest |

#![warn(missing_docs)]

pub mod adapters;
pub mod bridge;
mod event;
pub mod fixtures;
mod partial_json;
pub mod replay;
mod session;
pub mod sse;

#[cfg(feature = "http")]
pub mod http;
#[cfg(feature = "egui")]
mod view;

pub use event::{AgentEvent, ApprovalOption, PlanEntry, PlanStatus, SourceRef, StopReason, Usage};
pub use partial_json::parse_partial_json;
pub use session::{
    pretty, ApprovalPart, ErrorPart, Message, Part, ReasoningPart, Role, RunStatus, Session, TextPart, ToolCallPart,
    ToolState,
};
#[cfg(feature = "egui")]
pub use view::{AgentThread, ApprovalResponse, ThreadOutput};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
