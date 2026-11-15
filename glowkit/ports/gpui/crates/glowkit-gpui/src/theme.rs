//! Design tokens as a GPUI [`Global`]. The values are copied 1:1 from `glowkit_core::theme`, so a
//! GPUI app and an egui app using glowkit share one design language.
//!
//! ```no_run
//! # use gpui::App;
//! # fn init(cx: &mut App) {
//! use glowkit_gpui::{ActiveTheme, Theme};
//! Theme::dark().install(cx);
//! let accent = cx.theme().accent;
//! # }
//! ```

use crate::color::{hex, shade};
use gpui::{px, App, BoxShadow, Global, Hsla, Pixels};
use std::sync::OnceLock;

/// Semantic color + shape tokens (shadcn-style names, glowkit values).
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub name: &'static str,
    pub dark: bool,
    pub background: Hsla,
    pub foreground: Hsla,
    pub card: Hsla,
    pub card_foreground: Hsla,
    pub popover: Hsla,
    pub muted: Hsla,
    pub muted_foreground: Hsla,
    pub border: Hsla,
    pub input: Hsla,
    pub ring: Hsla,
    pub primary: Hsla,
    pub primary_foreground: Hsla,
    pub secondary: Hsla,
    pub secondary_foreground: Hsla,
    pub accent: Hsla,
    pub accent_foreground: Hsla,
    pub destructive: Hsla,
    pub success: Hsla,
    pub warning: Hsla,
    pub info: Hsla,
    /// Brand gradient used by "magic" components (shimmer, beams, glows).
    pub gradient: [Hsla; 3],
    /// Chart palette.
    pub chart: [Hsla; 6],
    /// Base corner radius in pixels.
    pub radius: f32,
}

impl Global for Theme {}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// Zinc dark ("Midnight"): the default.
    pub fn dark() -> Self {
        Self {
            name: "Midnight",
            dark: true,
            background: hex("#09090B"),
            foreground: hex("#FAFAFA"),
            card: hex("#111113"),
            card_foreground: hex("#FAFAFA"),
            popover: hex("#18181B"),
            muted: hex("#1C1C20"),
            muted_foreground: hex("#A1A1AA"),
            border: hex("#27272A"),
            input: hex("#27272A"),
            ring: hex("#8B5CF6"),
            primary: hex("#FAFAFA"),
            primary_foreground: hex("#18181B"),
            secondary: hex("#27272A"),
            secondary_foreground: hex("#FAFAFA"),
            accent: hex("#8B5CF6"),
            accent_foreground: hex("#FFFFFF"),
            destructive: hex("#EF4444"),
            success: hex("#22C55E"),
            warning: hex("#F59E0B"),
            info: hex("#3B82F6"),
            gradient: [hex("#8B5CF6"), hex("#EC4899"), hex("#F59E0B")],
            chart: ["#8B5CF6", "#22D3EE", "#F472B6", "#FBBF24", "#34D399", "#60A5FA"].map(hex),
            radius: 10.0,
        }
    }

    /// Zinc light ("Daylight").
    pub fn light() -> Self {
        Self {
            name: "Daylight",
            dark: false,
            background: hex("#FFFFFF"),
            foreground: hex("#09090B"),
            card: hex("#FFFFFF"),
            card_foreground: hex("#09090B"),
            popover: hex("#FFFFFF"),
            muted: hex("#F4F4F5"),
            muted_foreground: hex("#71717A"),
            border: hex("#E4E4E7"),
            input: hex("#E4E4E7"),
            ring: hex("#7C3AED"),
            primary: hex("#18181B"),
            primary_foreground: hex("#FAFAFA"),
            secondary: hex("#F4F4F5"),
            secondary_foreground: hex("#18181B"),
            accent: hex("#7C3AED"),
            accent_foreground: hex("#FFFFFF"),
            destructive: hex("#DC2626"),
            success: hex("#16A34A"),
            warning: hex("#D97706"),
            info: hex("#2563EB"),
            gradient: [hex("#7C3AED"), hex("#DB2777"), hex("#D97706")],
            chart: ["#7C3AED", "#0891B2", "#DB2777", "#D97706", "#059669", "#2563EB"].map(hex),
            radius: 10.0,
        }
    }

    /// Deep-blue, cyan-accented dark theme.
    pub fn ocean() -> Self {
        Self {
            name: "Ocean",
            background: hex("#020617"),
            card: hex("#0B1224"),
            popover: hex("#0F172A"),
            muted: hex("#111C33"),
            muted_foreground: hex("#94A3B8"),
            border: hex("#1E293B"),
            input: hex("#1E293B"),
            ring: hex("#22D3EE"),
            accent: hex("#22D3EE"),
            accent_foreground: hex("#04131A"),
            gradient: [hex("#22D3EE"), hex("#6366F1"), hex("#A855F7")],
            chart: ["#22D3EE", "#6366F1", "#A855F7", "#2DD4BF", "#F472B6", "#FBBF24"].map(hex),
            ..Self::dark()
        }
    }

    /// Warm, ember-orange dark theme.
    pub fn ember() -> Self {
        Self {
            name: "Ember",
            background: hex("#0C0A09"),
            card: hex("#141110"),
            popover: hex("#1C1917"),
            muted: hex("#1F1B19"),
            muted_foreground: hex("#A8A29E"),
            border: hex("#292524"),
            input: hex("#292524"),
            ring: hex("#F97316"),
            accent: hex("#F97316"),
            accent_foreground: hex("#1C0D02"),
            gradient: [hex("#F97316"), hex("#EF4444"), hex("#EAB308")],
            chart: ["#F97316", "#EAB308", "#EF4444", "#FB7185", "#A3E635", "#38BDF8"].map(hex),
            ..Self::dark()
        }
    }

    /// Green-on-black hacker theme.
    pub fn matrix() -> Self {
        Self {
            name: "Matrix",
            background: hex("#030A05"),
            foreground: hex("#D1FAE5"),
            card: hex("#06130A"),
            card_foreground: hex("#D1FAE5"),
            popover: hex("#081A0E"),
            muted: hex("#0A1F11"),
            muted_foreground: hex("#6EE7B7"),
            border: hex("#14532D"),
            input: hex("#14532D"),
            ring: hex("#22C55E"),
            primary: hex("#22C55E"),
            primary_foreground: hex("#03140A"),
            accent: hex("#4ADE80"),
            accent_foreground: hex("#03140A"),
            gradient: [hex("#22C55E"), hex("#A3E635"), hex("#2DD4BF")],
            chart: ["#22C55E", "#A3E635", "#2DD4BF", "#FACC15", "#4ADE80", "#86EFAC"].map(hex),
            ..Self::dark()
        }
    }

    /// All built-in presets.
    pub fn presets() -> Vec<Theme> {
        vec![Self::dark(), Self::light(), Self::ocean(), Self::ember(), Self::matrix()]
    }

    /// A preset by (case-insensitive) name, e.g. `"daylight"`, `"light"`, `"ocean"`.
    pub fn by_name(name: &str) -> Option<Theme> {
        let n = name.to_ascii_lowercase();
        match n.as_str() {
            "dark" => Some(Self::dark()),
            "light" => Some(Self::light()),
            _ => Self::presets().into_iter().find(|t| t.name.eq_ignore_ascii_case(&n)),
        }
    }

    /// Make this theme the app-wide [`Global`] and redraw every window.
    pub fn install(self, cx: &mut App) {
        cx.set_global(self);
        cx.refresh_windows();
    }

    /// The installed theme, or [`Theme::dark`] when none was installed.
    pub fn global(cx: &App) -> &Theme {
        static FALLBACK: OnceLock<Theme> = OnceLock::new();
        cx.try_global::<Theme>().unwrap_or_else(|| FALLBACK.get_or_init(Theme::dark))
    }

    /// Corner radius token.
    pub fn corner(&self) -> Pixels {
        px(self.radius)
    }

    /// Corner radius scaled by `factor` (e.g. 0.6 for small controls).
    pub fn corner_scaled(&self, factor: f32) -> Pixels {
        px(self.radius * factor)
    }

    /// Slightly raised card surface (glowkit lifts cards a hair in dark themes).
    pub fn surface(&self) -> Hsla {
        if self.dark {
            shade(self.card, 0.015)
        } else {
            self.card
        }
    }

    /// The standard soft drop shadow under cards. `lift` in `0..=1` deepens it (hover).
    pub fn card_shadow(&self, lift: f32) -> Vec<BoxShadow> {
        let a = if self.dark { 0.35 } else { 0.07 };
        vec![BoxShadow {
            color: crate::color::with_alpha(gpui::black(), a + a * 0.6 * lift),
            offset: gpui::point(px(0.), px(2.0 + 4.0 * lift)),
            blur_radius: px(10.0 + 8.0 * lift),
            spread_radius: px(0.),
        }]
    }
}

/// `cx.theme()` on anything that derefs to [`App`] (including `Context<V>`).
pub trait ActiveTheme {
    fn theme(&self) -> &Theme;
}

impl ActiveTheme for App {
    fn theme(&self) -> &Theme {
        Theme::global(self)
    }
}
