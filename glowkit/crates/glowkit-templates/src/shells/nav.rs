//! The navigation data model the shells render: a caller-owned tree of [`NavSection`]s holding
//! [`NavItem`]s (which may nest), plus workspaces, the signed-in user and notifications.

use super::icon::AppIcon;

/// One navigation entry. Leaf items navigate to their `route`; items with children expand and
/// collapse (the `expanded` flag lives in the item, so the caller owns it).
///
/// ```no_run
/// use glowkit_templates::{AppIcon, NavItem};
/// use glowkit_components::LineIcon;
/// let item = NavItem::new("projects", "Projects")
///     .icon(LineIcon::Folder)
///     .badge("3")
///     .child(NavItem::new("projects/active", "Active"))
///     .child(NavItem::new("projects/archived", "Archived"))
///     .expanded(true);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct NavItem {
    /// Route this item navigates to (also its identity).
    pub route: String,
    /// Visible text.
    pub label: String,
    /// Leading icon (also used by the icon rail).
    pub icon: Option<AppIcon>,
    /// Count or label on the right ("12", "New").
    pub badge: Option<String>,
    /// Nested items.
    pub children: Vec<NavItem>,
    /// Whether the children are shown.
    pub expanded: bool,
    /// Keyboard hint shown in the command palette.
    pub shortcut: Option<String>,
}

impl NavItem {
    /// An item with a route and label.
    pub fn new(route: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            route: route.into(),
            label: label.into(),
            icon: None,
            badge: None,
            children: Vec::new(),
            expanded: false,
            shortcut: None,
        }
    }
    /// Leading icon (accepts [`AppIcon`], `LineIcon` or `UiIcon`).
    pub fn icon(mut self, icon: impl Into<AppIcon>) -> Self {
        self.icon = Some(icon.into());
        self
    }
    /// Badge text on the right.
    pub fn badge(mut self, text: impl Into<String>) -> Self {
        self.badge = Some(text.into());
        self
    }
    /// Add a nested item.
    pub fn child(mut self, item: NavItem) -> Self {
        self.children.push(item);
        self
    }
    /// Start expanded.
    pub fn expanded(mut self, yes: bool) -> Self {
        self.expanded = yes;
        self
    }
    /// Shortcut hint for the command palette.
    pub fn shortcut(mut self, s: impl Into<String>) -> Self {
        self.shortcut = Some(s.into());
        self
    }
    /// Is `route` this item or one of its descendants?
    pub fn contains(&self, route: &str) -> bool {
        self.route == route || self.children.iter().any(|c| c.contains(route))
    }
}

/// A titled group of [`NavItem`]s ("Platform", "Projects"). In the dual-sidebar layout each
/// section becomes one icon in the rail.
#[derive(Clone, Debug, PartialEq)]
pub struct NavSection {
    /// Group label.
    pub title: String,
    /// Icon used by the dual-sidebar rail.
    pub icon: Option<AppIcon>,
    /// Items.
    pub items: Vec<NavItem>,
}

impl NavSection {
    /// A section with a title.
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), icon: None, items: Vec::new() }
    }
    /// Rail icon for the dual layout.
    pub fn icon(mut self, icon: impl Into<AppIcon>) -> Self {
        self.icon = Some(icon.into());
        self
    }
    /// Add an item.
    pub fn item(mut self, item: NavItem) -> Self {
        self.items.push(item);
        self
    }
}

/// Labels from the section down to the item at `route` (for breadcrumbs).
pub fn nav_path(nav: &[NavSection], route: &str) -> Vec<String> {
    fn walk(items: &[NavItem], route: &str, out: &mut Vec<String>) -> bool {
        for it in items {
            out.push(it.label.clone());
            if it.route == route || walk(&it.children, route, out) {
                return true;
            }
            out.pop();
        }
        false
    }
    for s in nav {
        let mut out = Vec::new();
        if walk(&s.items, route, &mut out) {
            return out;
        }
    }
    Vec::new()
}

/// The item at `route`, if any.
pub fn nav_find<'a>(nav: &'a [NavSection], route: &str) -> Option<&'a NavItem> {
    fn walk<'a>(items: &'a [NavItem], route: &str) -> Option<&'a NavItem> {
        for it in items {
            if it.route == route {
                return Some(it);
            }
            if let Some(f) = walk(&it.children, route) {
                return Some(f);
            }
        }
        None
    }
    nav.iter().find_map(|s| walk(&s.items, route))
}

/// Every leaf (navigable) item, depth-first.
pub fn nav_leaves(nav: &[NavSection]) -> Vec<&NavItem> {
    fn walk<'a>(items: &'a [NavItem], out: &mut Vec<&'a NavItem>) {
        for it in items {
            if it.children.is_empty() {
                out.push(it);
            } else {
                walk(&it.children, out);
            }
        }
    }
    let mut out = Vec::new();
    for s in nav {
        walk(&s.items, &mut out);
    }
    out
}

/// A workspace / team in the sidebar switcher.
#[derive(Clone, Debug, PartialEq)]
pub struct Workspace {
    /// Name ("Acme Inc").
    pub name: String,
    /// Sub-line, usually the plan ("Enterprise").
    pub plan: String,
}

impl Workspace {
    /// A workspace with name and plan.
    pub fn new(name: impl Into<String>, plan: impl Into<String>) -> Self {
        Self { name: name.into(), plan: plan.into() }
    }
}

/// The signed-in user shown in the sidebar footer / user menu.
#[derive(Clone, Debug, PartialEq)]
pub struct ShellUser {
    /// Display name.
    pub name: String,
    /// Email.
    pub email: String,
}

impl ShellUser {
    /// A user with a name and email.
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        Self { name: name.into(), email: email.into() }
    }
}

/// One entry in the topbar notifications dropdown.
#[derive(Clone, Debug, PartialEq)]
pub struct ShellNotification {
    /// Bold first line.
    pub title: String,
    /// Detail text (two lines max).
    pub body: String,
    /// Relative time ("2m ago").
    pub time: String,
    /// Leading icon.
    pub icon: AppIcon,
    /// Unread entries get a dot and count towards the bell badge.
    pub unread: bool,
}

impl ShellNotification {
    /// An unread notification.
    pub fn new(
        icon: impl Into<AppIcon>,
        title: impl Into<String>,
        body: impl Into<String>,
        time: impl Into<String>,
    ) -> Self {
        Self { title: title.into(), body: body.into(), time: time.into(), icon: icon.into(), unread: true }
    }
    /// Mark as already read.
    pub fn read(mut self) -> Self {
        self.unread = false;
        self
    }
}
