//! # glowkit-ai
//!
//! AI interface components for egui: everything needed to build a polished chat app in Rust.
//!
//! * Content: [`Markdown`] (streaming-safe), [`CodeBlock`], [`StreamingText`].
//! * Chat: [`ChatMessage`], [`MessageActions`], [`BranchSwitcher`], [`Conversation`],
//!   [`PromptInput`], [`Suggestions`], [`ModelSelector`], [`Checkpoint`].
//! * Agent state: [`Reasoning`], [`ChainOfThought`], [`ToolCall`], [`TaskList`], [`Sources`],
//!   [`InlineCitation`], [`ContextMeter`], [`Workflow`].
//! * Output: [`Artifact`], [`ImageGen`], [`WebPreview`].
//! * Loading: [`TypingIndicator`], [`Loader`], [`ShimmerText`].
//! * Voice: [`VoiceOrb`], [`LiveWaveform`], [`AudioBars`], [`VoiceButton`].
//!
//! Every component reads its colors from the active [`glowkit_core::Theme`].
//!
//! ```no_run
//! # let ui: &mut egui::Ui = todo!();
//! use glowkit_ai::{ChatMessage, Role};
//! ui.add(ChatMessage::new(Role::User, "What is ownership?"));
//! ui.add(ChatMessage::new(Role::Assistant, "Ownership is Rust's **memory model**..."));
//! ```

mod artifact;
mod code;
mod common;
mod context;
mod conversation;
mod icons;
mod loader;
pub mod markdown;
mod message;
mod misc;
mod prompt;
mod reasoning;
mod sources;
mod streaming;
mod task;
mod tool;
mod voice;
mod workflow;

pub use artifact::{Artifact, ArtifactOutput, ImageGen};
pub use code::{highlight, CodeBlock, TokenKind};
pub use context::{format_tokens, ContextMeter, MeterStyle};
pub use conversation::Conversation;
pub use icons::{paint_icon, paint_icon_width, Icon};
pub use loader::{Loader, LoaderKind, ShimmerText, TypingIndicator};
pub use markdown::Markdown;
pub use message::{Avatar, BranchSwitcher, ChatMessage, MessageAction, MessageActions, MessageOutput, Role};
pub use misc::{Checkpoint, ModelInfo, ModelSelector, Suggestions, WebPreview, WebPreviewOutput};
pub use prompt::{Attachment, AttachmentKind, PromptInput, PromptOutput};
pub use reasoning::{ChainOfThought, Reasoning, StepStatus, ThoughtStep};
pub use sources::{InlineCitation, Source, Sources};
pub use streaming::{RevealStyle, StreamingText};
pub use task::{TaskItem, TaskList, TaskStatus};
pub use tool::{ToolCall, ToolStatus};
pub use voice::{AudioBars, LiveWaveform, VoiceButton, VoiceOrb, VoiceState};
pub use workflow::{NodeKind, NodeStatus, Workflow, WorkflowEdge, WorkflowNode};

#[cfg(feature = "demos")]
mod chat_demo;
#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
