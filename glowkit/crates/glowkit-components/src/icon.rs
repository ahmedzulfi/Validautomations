//! One icon type for every component.
//!
//! glowkit-components ships two painted vector icon sets: [`LineIcon`] (navigation & overlays)
//! and [`UiIcon`] (inputs & data display). Every component that takes an icon accepts
//! `impl Into<Icon>`, so either set works anywhere:
//!
//! ```no_run
//! use glowkit_components::{CommandItem, LineIcon, UiIcon};
//! let a = CommandItem::new("Profile").icon(LineIcon::User);
//! let b = CommandItem::new("Inbox").icon(UiIcon::Mail);
//! ```

use crate::{LineIcon, UiIcon};
use egui::{Color32, Painter, Rect};

/// An icon from either built-in set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    /// From the navigation / overlay set.
    Line(LineIcon),
    /// From the inputs / data-display set.
    Ui(UiIcon),
}

impl Icon {
    /// Paint the icon centered in `rect` with `color`.
    pub fn paint(self, painter: &Painter, rect: Rect, color: Color32) {
        match self {
            Icon::Line(i) => i.paint(painter, rect, color),
            Icon::Ui(i) => i.paint(painter, rect, color),
        }
    }
}

impl From<LineIcon> for Icon {
    fn from(i: LineIcon) -> Self {
        Icon::Line(i)
    }
}

impl From<UiIcon> for Icon {
    fn from(i: UiIcon) -> Self {
        Icon::Ui(i)
    }
}
