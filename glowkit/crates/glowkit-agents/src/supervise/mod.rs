#![warn(missing_docs)]
//! Blocks for supervising AI agents: agent trees and fleets, traces and run timelines,
//! approval queues and diff review, cost and context meters, live logs and sandboxes, session
//! managers, MCP servers and skills, evals, memory, schedules, plans and human-in-the-loop cards.
//!
//! Every block is a plain view over data you own, so you can drive it from a real agent runtime
//! or from a simulation; the gallery demos simulate agents progressing over time.

mod common;
mod approval;
mod diff;
mod evals;
mod plan;
mod logs;
mod sessions;
mod tools;
mod runs;
mod trace;
mod tree;

pub use common::{fmt_ago, fmt_secs, fmt_tok, fmt_usd, RunState};
pub use approval::{ApprovalQueue, ApprovalRequest, Decision, ErrorRecoveryCard, HitlAnswer, HumanInTheLoopPrompt, Recovery, Risk};
pub use diff::{DiffFile, DiffHunk, DiffLineKind, DiffReview};
pub use evals::{EvalScoreboard, EvalSuite, MemoryEntry, MemoryKind, MemoryViewer, ScheduleList, ScheduleOutput, ScheduledJob};
pub use plan::{ContextPart, ContextWindowViz, PlanBoard, PlanCard};
pub use logs::{LogLevel, LogLine, LogStream, SandboxStatus};
pub use sessions::{AgentSession, AgentSessionList, Device, SessionAction, SessionDetailHeader};
pub use tools::{McpServer, McpServerList, Skill, SkillsBrowser};
pub use runs::{AgentFleetGrid, CostMeter, FleetAgent, RunInfo, RunMonitor};
pub use trace::{RunTimeline, RunTimelineOutput, SpanKind, StepTool, TimelineStep, TraceSpan, TraceWaterfall};
pub use tree::{AgentNode, AgentTree, HandoffCard, HandoffItem};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
