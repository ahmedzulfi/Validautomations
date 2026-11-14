#![warn(missing_docs)]
//! BoardUI-style dashboard cards: calm, finance-grade widgets with real chart drawing (a true
//! sankey, activity rings, a segmented sleep gauge, a contributions heat grid, pill bar charts,
//! grouped bars and a funnel), plus agent UI pieces (limits, progress, thinking indicators, a
//! composer loader, a web-search trail, a questionnaire), a meter, a notification center and an
//! AI profile page. Every card takes caller data and animates its entry and data changes.

mod agent;
mod bars;
pub(crate) mod common;
mod forms;
mod heat;
mod meter;
mod profile;
mod rings;
mod sankey;

pub use agent::{
    BoardAgentLimits, BoardAgentProgress, BoardAgentThinking, BoardBucket, BoardComposerLoader, BoardLimit,
    BoardSearchStep, BoardThinkingStyle, BoardWebSearch,
};
pub use bars::{BoardEarnings, BoardOrders, BoardSeries, BoardStage, BoardStageBars, BoardSteps};
pub use forms::{
    BoardNotice, BoardNoticeIcon, BoardNotifications, BoardQuestion, BoardQuestionnaire, BoardQuestionnaireState,
};
pub use heat::{BoardContributions, BoardStat};
pub use meter::BoardMeter;
pub use profile::BoardAiProfile;
pub use rings::{
    BoardActiveDays, BoardActivityRings, BoardDay, BoardMonth, BoardRing, BoardSleepMetric, BoardSleepScore,
    BoardSleepWeek,
};
pub use sankey::{BoardFlow, BoardSankey};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
