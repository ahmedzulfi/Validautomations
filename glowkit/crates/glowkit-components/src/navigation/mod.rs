//! Navigation components: tabs, breadcrumbs, pagination, sidebars, navbars, menus, steppers,
//! a command palette, animated links, a table of contents and a mobile bottom bar.

mod breadcrumb;
mod command;
mod link;
mod navbar;
mod navmenu;
mod sidebar;
mod stepper;
mod tabs;

pub use breadcrumb::{Breadcrumb, BreadcrumbSeparator, Pagination};
pub use command::{CommandItem, CommandPalette};
pub use link::{BottomNav, LinkStyle, TableOfContents, UnderlineLink};
pub use navbar::{auto_hide_on_scroll, FloatingNavbar, Navbar, NavbarResponse, ResizableNavbar};
pub use navmenu::{Menubar, NavMenuLink, NavigationMenu};
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
