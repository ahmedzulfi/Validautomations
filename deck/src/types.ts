export const LAYOUTS = [
  "title",
  "section",
  "bullets",
  "two-column",
  "stats",
  "quote",
  "image",
  "chart",
  "table",
  "timeline",
  "closing",
] as const;

export type Layout = (typeof LAYOUTS)[number];

export const LAYOUT_LABELS: Record<Layout, string> = {
  title: "Title",
  section: "Section break",
  bullets: "Bullets",
  "two-column": "Two columns",
  stats: "Big numbers",
  quote: "Quote",
  image: "Image + text",
  chart: "Chart",
  table: "Table",
  timeline: "Timeline",
  closing: "Closing",
};

export type ChartKind = "bar" | "line" | "donut";

export interface Column {
  heading: string;
  bullets: string[];
}

export interface Stat {
  value: string;
  label: string;
}

export interface ChartData {
  kind: ChartKind;
  labels: string[];
  values: number[];
  unit: string;
}

export interface TableData {
  headers: string[];
  rows: string[][];
}

export interface TimelineItem {
  label: string;
  text: string;
}

export interface ImageData {
  url: string;
  prompt: string;
}

export interface Slide {
  id: string;
  layout: Layout;
  title: string;
  subtitle: string;
  body: string;
  bullets: string[];
  columns: Column[];
  stats: Stat[];
  quote: string;
  attribution: string;
  chart: ChartData;
  table: TableData;
  timeline: TimelineItem[];
  image: ImageData;
  notes: string;
}

export interface Theme {
  id: string;
  name: string;
  dark: boolean;
  bg: string;
  surface: string;
  text: string;
  muted: string;
  primary: string;
  accent: string;
  headingFont: string;
  bodyFont: string;
  /** Background used for title, section and closing slides. */
  heroBg: string;
  heroText: string;
}

export interface Brand {
  logo: string; // data URL or ""
  footer: string;
}

export type Transition = "none" | "fade" | "slide";

export interface Deck {
  id: string;
  title: string;
  themeId: string;
  /** Per-deck overrides on top of the theme (brand kit). */
  themeOverrides: Partial<Theme>;
  brand: Brand;
  transition: Transition;
  slides: Slide[];
  createdAt: number;
  updatedAt: number;
}
