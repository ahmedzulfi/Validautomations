//! Design tokens. Every glowkit component reads its colors and radii from the active [`Theme`],
//! so one call to [`Theme::install`] restyles the whole app (including stock egui widgets).

use crate::color::hex;
use egui::{Color32, Context, CornerRadius, Id, Stroke, Ui};

/// Semantic color + shape tokens, modeled on the token set popular web design systems use.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub name: &'static str,
    pub dark: bool,
    pub background: Color32,
    pub foreground: Color32,
    pub card: Color32,
    pub card_foreground: Color32,
    pub popover: Color32,
    pub muted: Color32,
    pub muted_foreground: Color32,
    pub border: Color32,
    pub input: Color32,
    pub ring: Color32,
    pub primary: Color32,
    pub primary_foreground: Color32,
    pub secondary: Color32,
    pub secondary_foreground: Color32,
    pub accent: Color32,
    pub accent_foreground: Color32,
    pub destructive: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub info: Color32,
    /// Brand gradient used by "magic" components (shimmer, beams, glows).
    pub gradient: [Color32; 3],
    /// Chart palette.
    pub chart: [Color32; 6],
    /// Base corner radius in points.
    pub radius: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// Zinc dark: the default.
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
            chart: [hex("#8B5CF6"), hex("#22D3EE"), hex("#F472B6"), hex("#FBBF24"), hex("#34D399"), hex("#60A5FA")],
            radius: 10.0,
        }
    }

    /// Zinc light.
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
            chart: [hex("#7C3AED"), hex("#0891B2"), hex("#DB2777"), hex("#D97706"), hex("#059669"), hex("#2563EB")],
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
            chart: [hex("#22D3EE"), hex("#6366F1"), hex("#A855F7"), hex("#2DD4BF"), hex("#F472B6"), hex("#FBBF24")],
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
            chart: [hex("#F97316"), hex("#EAB308"), hex("#EF4444"), hex("#FB7185"), hex("#A3E635"), hex("#38BDF8")],
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
            chart: [hex("#22C55E"), hex("#A3E635"), hex("#2DD4BF"), hex("#FACC15"), hex("#4ADE80"), hex("#86EFAC")],
            ..Self::dark()
        }
    }

    /// All built-in presets.
    pub fn presets() -> Vec<Theme> {
        vec![Self::dark(), Self::light(), Self::ocean(), Self::ember(), Self::matrix()]
    }

    fn id() -> Id {
        Id::unique("glowkit_theme")
    }

    /// The active theme for this context (defaults to [`Theme::dark`]).
    pub fn current(ctx: &Context) -> Theme {
        ctx.data_mut(|d| d.get_temp_mut_or_insert_with(Self::id(), Theme::dark).clone())
    }

    /// Shortcut for `Theme::current(ui.ctx())`.
    pub fn of(ui: &Ui) -> Theme {
        Self::current(ui.ctx())
    }

    /// Corner radius token.
    pub fn corner(&self) -> CornerRadius {
        CornerRadius::same(self.radius.round() as u8)
    }

    /// Corner radius scaled by `factor` (e.g. 0.5 for small controls).
    pub fn corner_scaled(&self, factor: f32) -> CornerRadius {
        CornerRadius::same((self.radius * factor).round().clamp(0.0, 255.0) as u8)
    }

    /// Hairline border stroke.
    pub fn border_stroke(&self) -> Stroke {
        Stroke::new(1.0, self.border)
    }

    /// Make this theme active and restyle egui's built-in widgets to match.
    pub fn install(&self, ctx: &Context) {
        ctx.data_mut(|d| d.insert_temp(Self::id(), self.clone()));
        let mut v = if self.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
        v.panel_fill = self.background;
        v.window_fill = self.popover;
        v.extreme_bg_color = self.muted;
        v.faint_bg_color = self.card;
        v.override_text_color = Some(self.foreground);
        v.hyperlink_color = self.accent;
        v.selection.bg_fill = crate::color::with_alpha(self.accent, 0.35);
        v.selection.stroke = Stroke::new(1.0, self.accent);
        v.window_stroke = self.border_stroke();
        v.window_corner_radius = self.corner();
        v.menu_corner_radius = self.corner_scaled(0.8);
        let r = self.corner_scaled(0.6);
        for (w, fill) in [
            (&mut v.widgets.noninteractive, self.card),
            (&mut v.widgets.inactive, self.secondary),
            (&mut v.widgets.hovered, crate::color::shade(self.secondary, if self.dark { 0.05 } else { -0.04 })),
            (&mut v.widgets.active, crate::color::shade(self.secondary, if self.dark { 0.09 } else { -0.08 })),
            (&mut v.widgets.open, self.secondary),
        ] {
            w.bg_fill = fill;
            w.weak_bg_fill = fill;
            w.corner_radius = r;
            w.bg_stroke = Stroke::new(1.0, self.border);
            w.fg_stroke.color = self.foreground;
        }
        v.widgets.noninteractive.fg_stroke.color = self.muted_foreground;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, crate::color::shade(self.border, 0.1));
        v.widgets.active.bg_stroke = Stroke::new(1.0, self.ring);
        ctx.set_theme(if self.dark { egui::Theme::Dark } else { egui::Theme::Light });
        ctx.all_styles_mut(|s| {
            s.visuals = v.clone();
            s.spacing.item_spacing = egui::vec2(8.0, 8.0);
            s.spacing.button_padding = egui::vec2(12.0, 6.0);
            s.spacing.interact_size.y = 30.0;
        });
    }
}
