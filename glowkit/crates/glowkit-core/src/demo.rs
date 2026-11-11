//! The demo registry. Every glowkit crate exposes `pub fn demos() -> Vec<Demo>` (behind its
//! `demos` feature) and the gallery app / website lists them.

use egui::Ui;

/// Gallery categories, in sidebar order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    TextAnimations,
    Backgrounds,
    Buttons,
    CardsAndBorders,
    CursorAndInteraction,
    Motion,
    MotionEngine,
    ThreeD,
    Loaders,
    Inputs,
    Navigation,
    Overlays,
    DataDisplay,
    Charts,
    Ai,
    Blocks,
}

impl Category {
    pub const ALL: [Category; 16] = [
        Category::TextAnimations,
        Category::Backgrounds,
        Category::Buttons,
        Category::CardsAndBorders,
        Category::CursorAndInteraction,
        Category::Motion,
        Category::MotionEngine,
        Category::ThreeD,
        Category::Loaders,
        Category::Inputs,
        Category::Navigation,
        Category::Overlays,
        Category::DataDisplay,
        Category::Charts,
        Category::Ai,
        Category::Blocks,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::TextAnimations => "Text Animations",
            Category::Backgrounds => "Backgrounds",
            Category::Buttons => "Buttons",
            Category::CardsAndBorders => "Cards & Borders",
            Category::CursorAndInteraction => "Cursor & Interaction",
            Category::Motion => "Motion & Layout",
            Category::MotionEngine => "Motion Engine",
            Category::ThreeD => "3D",
            Category::Loaders => "Loaders & Progress",
            Category::Inputs => "Inputs & Forms",
            Category::Navigation => "Navigation",
            Category::Overlays => "Overlays & Feedback",
            Category::DataDisplay => "Data Display",
            Category::Charts => "Charts",
            Category::Ai => "AI",
            Category::Blocks => "Blocks",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Category::TextAnimations => "Aa",
            Category::Backgrounds => "▦",
            Category::Buttons => "⏺",
            Category::CardsAndBorders => "▢",
            Category::CursorAndInteraction => "➚",
            Category::Motion => "≋",
            Category::MotionEngine => "»",
            Category::ThreeD => "◆",
            Category::Loaders => "◌",
            Category::Inputs => "✎",
            Category::Navigation => "☰",
            Category::Overlays => "❐",
            Category::DataDisplay => "▤",
            Category::Charts => "📈",
            Category::Ai => "✦",
            Category::Blocks => "▣",
        }
    }
}

/// A single gallery entry.
#[derive(Clone, Copy)]
pub struct Demo {
    /// Unique kebab-case id, e.g. `"shiny-text"`.
    pub id: &'static str,
    pub name: &'static str,
    pub category: Category,
    pub description: &'static str,
    /// The web components this one covers, e.g. `&["React Bits: Shiny Text", "Magic UI: Animated Shiny Text"]`.
    pub covers: &'static [&'static str],
    /// Usage snippet shown under the live preview.
    pub code: &'static str,
    /// Renders the live preview. Called every frame. Use `ui.id()`-derived ids for state.
    pub show: fn(&mut Ui),
}

impl std::fmt::Debug for Demo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Demo").field("id", &self.id).field("category", &self.category).finish()
    }
}
