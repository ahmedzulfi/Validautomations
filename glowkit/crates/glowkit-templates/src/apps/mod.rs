//! Complete app templates: interactive screens inside an [`AppShell`](crate::AppShell), with
//! realistic data, loading skeletons, toasts and keyboard shortcuts. Copy one as the starting
//! point of a real desktop app.

mod analytics;
mod assistant;
mod chat;
mod commerce;
mod common;
mod devconsole;
mod docs;
mod calendar;
mod music;
mod files;
mod crm;
mod finance;
mod mail;
mod projects;
mod settings;

pub use analytics::AnalyticsApp;
pub use devconsole::{ApiKey, DevConsoleApp};
pub use docs::DocsApp;
pub use calendar::{CalEvent, CalendarApp};
pub use music::{MusicApp, MusicTrack};
pub use files::{FileManagerApp, FsEntry};
pub use commerce::{CommerceApp, ShopOrder, ShopProduct};
pub use finance::{FinanceAccount, FinanceApp, FinanceTxn};
pub use projects::{ProjectTask, ProjectsApp};
pub use mail::{MailApp, MailMessage};
pub use chat::{ChatApp, ChatPost};
pub use assistant::{AssistantApp, AssistantChat};
pub use settings::{SettingsApp, TeamMember};
pub use crm::{CrmApp, CrmCustomer, CrmDeal};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
