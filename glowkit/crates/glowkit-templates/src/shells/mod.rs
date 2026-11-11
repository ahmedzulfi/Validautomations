//! Dashboard shells: [`AppShell`] lays out a sidebar, a topbar and your content in seven
//! layouts ([`ShellLayout`]), driven by a caller-owned [`NavSection`] / [`NavItem`] tree.

mod icon;
pub(crate) mod kit;
mod nav;
mod shell;
mod sidebar;

pub use icon::AppIcon;
pub use nav::{nav_find, nav_leaves, nav_path, NavItem, NavSection, ShellNotification, ShellUser, Workspace};
pub use shell::{AppShell, ShellContext, ShellLayout, ShellResponse};
pub(crate) use sidebar::chevron as sidebar_chevron;

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
