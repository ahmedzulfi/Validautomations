//! Design tokens. A port of `glowkit_core::Theme`: the same semantic tokens and the same five
//! presets, plus [`Theme::css_vars`] to expose them as CSS custom properties.

use crate::color::{hex, mix, shade, with_alpha, Color};

/// Semantic color + shape tokens, modeled on the token set popular web design systems use.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub name: &'static str,
    pub dark: bool,
    pub background: Color,
    pub foreground: Color,
    pub card: Color,
    pub card_foreground: Color,
    pub popover: Color,
    pub muted: Color,
    pub muted_foreground: Color,
    pub border: Color,
    pub input: Color,
    pub ring: Color,
    pub primary: Color,
    pub primary_foreground: Color,
    pub secondary: Color,
    pub secondary_foreground: Color,
    pub accent: Color,
    pub accent_foreground: Color,
    pub destructive: Color,
    pub success: Color,
    pub warning: Color,
    pub info: Color,
    /// Brand gradient used by "magic" components (shimmer, beams, glows).
    pub gradient: [Color; 3],
    /// Chart palette.
    pub chart: [Color; 6],
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

    /// Look up a preset by (case-insensitive) name: `midnight`/`dark`, `daylight`/`light`,
    /// `ocean`, `ember`, `matrix`.
    pub fn by_name(name: &str) -> Option<Theme> {
        match name.to_ascii_lowercase().as_str() {
            "midnight" | "dark" => Some(Self::dark()),
            "daylight" | "light" => Some(Self::light()),
            "ocean" => Some(Self::ocean()),
            "ember" => Some(Self::ember()),
            "matrix" => Some(Self::matrix()),
            _ => None,
        }
    }

    /// Every token as `(css-variable-name, value)`, e.g. `("--gk-background", "#09090b")`.
    ///
    /// Besides the raw tokens this emits a few derived values the components share:
    /// `--gk-radius` (px), `--gk-radius-sm`, `--gk-gradient` (a CSS gradient through the brand
    /// stops), `--gk-surface-hover`, `--gk-shadow-color`, `--gk-ring-soft` and `--gk-color-scheme`.
    pub fn css_vars(&self) -> Vec<(&'static str, String)> {
        let mut v: Vec<(&'static str, String)> = vec![
            ("--gk-background", self.background.to_css()),
            ("--gk-foreground", self.foreground.to_css()),
            ("--gk-card", self.card.to_css()),
            ("--gk-card-foreground", self.card_foreground.to_css()),
            ("--gk-popover", self.popover.to_css()),
            ("--gk-muted", self.muted.to_css()),
            ("--gk-muted-foreground", self.muted_foreground.to_css()),
            ("--gk-border", self.border.to_css()),
            ("--gk-input", self.input.to_css()),
            ("--gk-ring", self.ring.to_css()),
            ("--gk-primary", self.primary.to_css()),
            ("--gk-primary-foreground", self.primary_foreground.to_css()),
            ("--gk-secondary", self.secondary.to_css()),
            ("--gk-secondary-foreground", self.secondary_foreground.to_css()),
            ("--gk-accent", self.accent.to_css()),
            ("--gk-accent-foreground", self.accent_foreground.to_css()),
            ("--gk-destructive", self.destructive.to_css()),
            ("--gk-success", self.success.to_css()),
            ("--gk-warning", self.warning.to_css()),
            ("--gk-info", self.info.to_css()),
        ];
        const GRAD: [&str; 3] = ["--gk-gradient-1", "--gk-gradient-2", "--gk-gradient-3"];
        for (name, c) in GRAD.iter().zip(self.gradient) {
            v.push((name, c.to_css()));
        }
        const CHART: [&str; 6] = ["--gk-chart-1", "--gk-chart-2", "--gk-chart-3", "--gk-chart-4", "--gk-chart-5", "--gk-chart-6"];
        for (name, c) in CHART.iter().zip(self.chart) {
            v.push((name, c.to_css()));
        }
        v.push(("--gk-radius", format!("{}px", self.radius)));
        v.push(("--gk-radius-sm", format!("{}px", (self.radius * 0.6).round())));
        v.push(("--gk-radius-lg", format!("{}px", (self.radius * 1.4).round())));
        v.push(("--gk-gradient", crate::color::css_linear_gradient(90.0, &self.gradient, false, 7)));
        v.push((
            "--gk-surface-hover",
            shade(self.secondary, if self.dark { 0.05 } else { -0.04 }).to_css(),
        ));
        v.push(("--gk-shadow-color", with_alpha(Color::BLACK, if self.dark { 0.5 } else { 0.12 }).to_css()));
        v.push(("--gk-ring-soft", with_alpha(self.ring, 0.35).to_css()));
        v.push(("--gk-accent-soft", with_alpha(self.accent, if self.dark { 0.16 } else { 0.10 }).to_css()));
        v.push(("--gk-card-tint", mix(self.card, self.accent, if self.dark { 0.06 } else { 0.03 }).to_css()));
        v.push(("--gk-dot", mix(self.background, self.foreground, if self.dark { 0.2 } else { 0.22 }).to_css()));
        v.push(("--gk-shimmer-bg", if self.dark { "#050507".into() } else { self.primary.to_css() }));
        v.push((
            "--gk-spotlight",
            if self.dark { with_alpha(Color::WHITE, 0.16).to_css() } else { with_alpha(self.accent, 0.12).to_css() },
        ));
        v.push(("--gk-spotlight-bg", if self.dark { mix(self.card, self.background, 0.5) } else { self.card }.to_css()));
        v.push(("--gk-color-scheme", if self.dark { "dark".into() } else { "light".into() }));
        v
    }

    /// The tokens as a CSS declaration block body: `--gk-background: #09090b; ...`.
    /// Put it in a `style` attribute or inside a selector.
    pub fn css_declarations(&self) -> String {
        let mut s = String::new();
        for (k, v) in self.css_vars() {
            s.push_str(k);
            s.push_str(": ");
            s.push_str(&v);
            s.push_str("; ");
        }
        s.push_str(if self.dark { "color-scheme: dark;" } else { "color-scheme: light;" });
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_have_vars() {
        for t in Theme::presets() {
            let vars = t.css_vars();
            assert!(vars.iter().any(|(k, _)| *k == "--gk-accent"));
            assert!(t.css_declarations().contains("--gk-background"));
        }
        assert_eq!(Theme::by_name("Ocean").unwrap().name, "Ocean");
    }
}
