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
//! * Voice & audio: [`VoiceOrb`], [`LiveWaveform`], [`AudioBars`], [`VoiceButton`], [`SpeechInput`],
//!   [`MicSelector`], [`AudioPlayer`], [`ScrubBar`], [`TranscriptViewer`], [`DotMatrix`].
//! * Agent controls: [`Confirmation`], [`OpenInChat`], [`AiToolbar`], [`JsxPreview`].
//!
//! Every component reads its colors from the active [`glowkit_core::Theme`].
//!
//! ```no_run
//! # let ui: &mut egui::Ui = todo!();
//! use glowkit_ai::{ChatMessage, Role};
//! ui.add(ChatMessage::new(Role::User, "What is ownership?"));
//! ui.add(ChatMessage::new(Role::Assistant, "Ownership is Rust's **memory model**..."));
//! ```

#![warn(missing_docs)]

mod artifact;
mod audio_player;
mod code;
mod common;
mod confirmation;
mod context;
mod conversation;
mod dot_matrix;
mod icons;
mod jsx_preview;
mod loader;
pub mod markdown;
mod media;
mod message;
mod mic_selector;
mod misc;
mod open_in_chat;
mod prompt;
mod reasoning;
mod scrub_bar;
mod sources;
mod speech_input;
mod streaming;
mod task;
mod tool;
mod toolbar;
mod transcript;
mod voice;
mod workflow;

pub use artifact::{Artifact, ArtifactOutput, ImageGen};
pub use audio_player::{AudioPlayer, AudioPlayerState};
pub use code::{highlight, CodeBlock, TokenKind};
pub use confirmation::{ApprovalState, Confirmation};
pub use context::{format_tokens, ContextMeter, MeterStyle};
pub use conversation::Conversation;
pub use dot_matrix::{DotMatrix, MatrixContent, MatrixPattern};
pub use icons::{paint_icon, paint_icon_width, Icon};
pub use jsx_preview::{parse_markup, JsxPreview, MarkupNode};
pub use loader::{Loader, LoaderKind, ShimmerText, TypingIndicator};
pub use markdown::Markdown;
pub use message::{Avatar, BranchSwitcher, ChatMessage, MessageAction, MessageActions, MessageOutput, Role};
pub use mic_selector::MicSelector;
pub use misc::{Checkpoint, ModelInfo, ModelSelector, Suggestions, WebPreview, WebPreviewOutput};
pub use open_in_chat::{ChatProvider, OpenInChat};
pub use prompt::{Attachment, AttachmentKind, PromptInput, PromptOutput};
pub use reasoning::{ChainOfThought, Reasoning, StepStatus, ThoughtStep};
pub use scrub_bar::{ScrubBar, ScrubMarker};
pub use sources::{InlineCitation, Source, Sources};
pub use speech_input::{SpeechInput, SpeechPhase};
pub use streaming::{RevealStyle, StreamingText};
pub use task::{TaskItem, TaskList, TaskStatus};
pub use tool::{ToolCall, ToolStatus};
pub use toolbar::{AiToolbar, ToolbarItem};
pub use transcript::{TranscriptViewer, TranscriptWord};
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
