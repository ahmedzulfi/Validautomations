import type { Deck, Theme } from "./types";

export const THEMES: Theme[] = [
  {
    id: "paper",
    name: "Paper",
    dark: false,
    bg: "#fbf8f3",
    surface: "#f1ece3",
    text: "#1d1b18",
    muted: "#6b645a",
    primary: "#c2410c",
    accent: "#0f766e",
    headingFont: "Fraunces",
    bodyFont: "Inter",
    heroBg: "#1d1b18",
    heroText: "#fbf8f3",
  },
  {
    id: "midnight",
    name: "Midnight",
    dark: true,
    bg: "#0d1321",
    surface: "#18213a",
    text: "#eef1f8",
    muted: "#9aa5bf",
    primary: "#f4b740",
    accent: "#5eead4",
    headingFont: "Space Grotesk",
    bodyFont: "Inter",
    heroBg: "#f4b740",
    heroText: "#0d1321",
  },
  {
    id: "studio",
    name: "Studio",
    dark: false,
    bg: "#ffffff",
    surface: "#f3f4f6",
    text: "#111827",
    muted: "#6b7280",
    primary: "#4f46e5",
    accent: "#db2777",
    headingFont: "Manrope",
    bodyFont: "Manrope",
    heroBg: "#4f46e5",
    heroText: "#ffffff",
  },
  {
    id: "forest",
    name: "Forest",
    dark: true,
    bg: "#0f1f1a",
    surface: "#18302a",
    text: "#e8f1ec",
    muted: "#94b3a6",
    primary: "#a3e635",
    accent: "#fbbf24",
    headingFont: "DM Serif Display",
    bodyFont: "Inter",
    heroBg: "#18302a",
    heroText: "#e8f1ec",
  },
  {
    id: "terminal",
    name: "Terminal",
    dark: true,
    bg: "#111111",
    surface: "#1c1c1c",
    text: "#e5e5e5",
    muted: "#8a8a8a",
    primary: "#22c55e",
    accent: "#38bdf8",
    headingFont: "IBM Plex Mono",
    bodyFont: "IBM Plex Mono",
    heroBg: "#22c55e",
    heroText: "#111111",
  },
  {
    id: "blush",
    name: "Blush",
    dark: false,
    bg: "#fff5f5",
    surface: "#fde2e4",
    text: "#3b1f2b",
    muted: "#8c5f6f",
    primary: "#be185d",
    accent: "#7c3aed",
    headingFont: "DM Serif Display",
    bodyFont: "Manrope",
    heroBg: "#3b1f2b",
    heroText: "#fff5f5",
  },
];

export const FONT_CHOICES = [
  "Inter",
  "Manrope",
  "Space Grotesk",
  "Fraunces",
  "DM Serif Display",
  "IBM Plex Mono",
];

export function themeById(id: string): Theme {
  return THEMES.find((t) => t.id === id) ?? THEMES[0];
}

export function resolveTheme(deck: Pick<Deck, "themeId" | "themeOverrides">): Theme {
  return { ...themeById(deck.themeId), ...deck.themeOverrides };
}

/** CSS custom properties for a theme, applied on the slide root. */
export function themeVars(theme: Theme): Record<string, string> {
  return {
    "--s-bg": theme.bg,
    "--s-surface": theme.surface,
    "--s-text": theme.text,
    "--s-muted": theme.muted,
    "--s-primary": theme.primary,
    "--s-accent": theme.accent,
    "--s-hero-bg": theme.heroBg,
    "--s-hero-text": theme.heroText,
    "--s-heading": `"${theme.headingFont}", Georgia, serif`,
    "--s-body": `"${theme.bodyFont}", system-ui, sans-serif`,
  };
}
