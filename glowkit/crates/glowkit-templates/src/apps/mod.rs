//! Complete app templates: interactive screens inside an [`AppShell`](crate::AppShell), with
//! realistic data, loading skeletons, toasts and keyboard shortcuts. Copy one as the starting
//! point of a real desktop app.

mod analytics;
mod chat;
mod commerce;
mod common;
mod crm;
mod finance;
mod mail;
mod projects;

pub use analytics::AnalyticsApp;
pub use commerce::{CommerceApp, ShopOrder, ShopProduct};
pub use finance::{FinanceAccount, FinanceApp, FinanceTxn};
pub use projects::{ProjectTask, ProjectsApp};
pub use mail::{MailApp, MailMessage};
pub use chat::{ChatApp, ChatPost};
pub use crm::{CrmApp, CrmCustomer, CrmDeal};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
