//! Assistant and tool-UI blocks: a branching thread with edit-and-resend, smart composers with
//! mentions and slash commands, thread lists, generative tool-result cards (tables, charts, forms,
//! plans, maps...), copilot sidebars and popups, inline ghost-text suggestions, selection
//! toolbars, mode and model pickers, usage banners, citations and a voice-mode overlay.

pub(crate) mod common;
#[cfg(feature = "demos")]
mod demos;
#[cfg(feature = "demos")]
mod demos_chrome;
#[cfg(feature = "demos")]
mod demos_tools;
mod banners;
mod composer;
mod copilot;
mod inline;
mod pickers;
mod thread_list;
mod voice_mode;
mod location;
mod thread;
mod tools_code;
mod tools_data;
mod tools_hitl;
mod tools_media;
mod tools_progress;

pub use banners::{Citation, CitationsFooter, CitedText, UsageBannerOutput, UsageLimitBanner};
pub use copilot::{CopilotPopup, CopilotSidebar, WelcomeScreen, WelcomeSuggestion};
pub use inline::{InlineSuggestion, InlineSuggestionOutput, SelectionAction, SelectionAskBar, SelectionOutput};
pub use pickers::{AgentMode, AgentModeSwitcher, ModelPickerRich, ModelSpec};
pub use thread_list::{ThreadEntry, ThreadListOutput, ThreadListSidebar, ThreadListState};
pub use voice_mode::{VoiceModeOutput, VoiceModeOverlay};
pub use composer::{ComposerOutput, ComposerState, ContextChip, MentionItem, SlashCommand, SmartComposer, StagedFile};
pub use location::{LocationResult, Place};
pub use tools_code::CodeResult;
pub use tools_data::{ChartKind, ChartResult, ColumnKind, DataTableResult};
pub use tools_hitl::{FieldKind, FormField, FormResult, FormState, OptionListResult, OptionState, ToolOption};
pub use tools_media::{ImageGalleryResult, LinkPreviewResult, Sky, WeatherResult};
pub use tools_progress::{PlanResult, PlanStatus, PlanStep, ProgressSteps};
pub use thread::{AssistantThread, ThreadEvent, ThreadModel, ThreadOutput, ThreadTurn, TurnVersion};

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
