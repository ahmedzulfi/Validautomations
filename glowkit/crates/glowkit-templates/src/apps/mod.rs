//! Complete app templates: interactive screens inside an [`AppShell`](crate::AppShell), with
//! realistic data, loading skeletons, toasts and keyboard shortcuts. Copy one as the starting
//! point of a real desktop app.

mod analytics;
mod assistant;
mod auth;
mod calendar;
mod chat;
mod commerce;
mod common;
mod crm;
mod devconsole;
mod docs;
mod files;
mod finance;
mod mail;
mod music;
mod projects;
mod settings;

pub use analytics::AnalyticsApp;
pub use assistant::{AssistantApp, AssistantChat};
pub use auth::AuthGateApp;
pub use calendar::{CalEvent, CalendarApp};
pub use chat::{ChatApp, ChatPost};
pub use commerce::{CommerceApp, ShopOrder, ShopProduct};
pub use crm::{CrmApp, CrmCustomer, CrmDeal};
pub use devconsole::{ApiKey, DevConsoleApp};
pub use docs::DocsApp;
pub use files::{FileManagerApp, FsEntry};
pub use finance::{FinanceAccount, FinanceApp, FinanceTxn};
pub use mail::{MailApp, MailMessage};
pub use music::{MusicApp, MusicTrack};
pub use projects::{ProjectTask, ProjectsApp};
pub use settings::{SettingsApp, TeamMember};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
