//! Navigation components: tabs, sidebars, navbars, steppers, a command palette, animated links
//! and a table of contents.

mod command;
mod link;
mod navbar;
mod sidebar;
mod stepper;
mod tabs;

pub use command::{CommandItem, CommandPalette};
pub use link::{LinkStyle, TableOfContents, UnderlineLink};
pub use navbar::{auto_hide_on_scroll, FloatingNavbar, Navbar, NavbarResponse, ResizableNavbar};
pub use sidebar::{Sidebar, SidebarItem};
pub use stepper::Stepper;
pub use tabs::{Tabs, TabsStyle};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
