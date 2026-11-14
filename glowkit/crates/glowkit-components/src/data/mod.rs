//! Data display: badges, avatars, cards, alerts, tables, timelines, stats, trees, code blocks,
//! accordions and more.

mod aspect_ratio;
mod avatar;
mod badge;
mod disclosure;
mod display;
mod item;
mod resizable;
mod scroll_area;
mod table;
mod timeline;

pub use aspect_ratio::AspectRatio;
pub use avatar::{Avatar, AvatarCircles, AvatarGroup, AvatarStatus, AvatarStyle};
pub use badge::{Badge, BadgeVariant, Chip, Kbd, Separator};
pub use disclosure::{Accordion, Collapsible, TreeNode, TreeView};
pub use display::{Alert, AlertVariant, Card, DescriptionList, EmptyState, EmptyStateResponse, KeyValue, StatCard};
pub use item::{Item, ItemMedia, ItemSize, ItemVariant};
pub use resizable::{ResizablePanels, SplitDirection};
pub use scroll_area::{ScrollbarMode, StyledScrollArea};
pub use table::{DataTable, TableCell, TableColumn};
pub use timeline::{CodeBlock, Timeline, TimelineItem};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
