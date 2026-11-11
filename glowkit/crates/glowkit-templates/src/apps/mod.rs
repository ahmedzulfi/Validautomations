//! Complete app templates: interactive screens inside an [`AppShell`](crate::AppShell), with
//! realistic data, loading skeletons, toasts and keyboard shortcuts. Copy one as the starting
//! point of a real desktop app.

mod analytics;
mod common;

pub use analytics::AnalyticsApp;

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
