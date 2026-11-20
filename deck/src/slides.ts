import { uid } from "./lib/ids";
import { LAYOUTS, type Deck, type Layout, type Slide } from "./types";

export function blankSlide(layout: Layout = "bullets", patch: Partial<Slide> = {}): Slide {
  return {
    id: uid("s_"),
    layout,
    title: "",
    subtitle: "",
    body: "",
    bullets: [],
    columns: [
      { heading: "", bullets: [] },
      { heading: "", bullets: [] },
    ],
    stats: [],
    quote: "",
    attribution: "",
    chart: { kind: "bar", labels: [], values: [], unit: "" },
    table: { headers: [], rows: [] },
    timeline: [],
    image: { url: "", prompt: "" },
    notes: "",
    ...patch,
  };
}

/** Sensible starter content so a freshly added slide isn't empty. */
export function starterSlide(layout: Layout): Slide {
  switch (layout) {
    case "title":
      return blankSlide(layout, { title: "Presentation title", subtitle: "A one-line promise" });
    case "section":
      return blankSlide(layout, { title: "Section title", subtitle: "What this part covers" });
    case "bullets":
      return blankSlide(layout, { title: "Key points", bullets: ["First point", "Second point", "Third point"] });
    case "two-column":
      return blankSlide(layout, {
        title: "Before and after",
        columns: [
          { heading: "Before", bullets: ["Slow", "Manual"] },
          { heading: "After", bullets: ["Fast", "Automatic"] },
        ],
      });
    case "stats":
      return blankSlide(layout, {
        title: "By the numbers",
        stats: [
          { value: "3x", label: "Faster" },
          { value: "42%", label: "Cheaper" },
          { value: "10k", label: "Users" },
        ],
      });
    case "quote":
      return blankSlide(layout, { quote: "A memorable line goes here.", attribution: "Someone notable" });
    case "image":
      return blankSlide(layout, { title: "Picture this", body: "A short caption that explains the image." });
    case "chart":
      return blankSlide(layout, {
        title: "Growth",
        chart: { kind: "bar", labels: ["Q1", "Q2", "Q3", "Q4"], values: [12, 19, 27, 41], unit: "" },
      });
    case "table":
      return blankSlide(layout, {
        title: "Comparison",
        table: {
          headers: ["", "Us", "Them"],
          rows: [
            ["Price", "$12", "$30"],
            ["Setup", "Minutes", "Weeks"],
          ],
        },
      });
    case "timeline":
      return blankSlide(layout, {
        title: "Roadmap",
        timeline: [
          { label: "Q1", text: "Launch" },
          { label: "Q2", text: "Grow" },
          { label: "Q3", text: "Scale" },
        ],
      });
    case "closing":
      return blankSlide(layout, { title: "Thank you", subtitle: "Questions?" });
  }
}

/**
 * Switch a slide's layout, carrying content across where it maps
 * (bullets <-> columns, etc.) and seeding starter content otherwise.
 */
export function changeLayout(slide: Slide, layout: Layout): Slide {
  if (slide.layout === layout) return slide;
  const next: Slide = { ...slide, layout };
  const starter = starterSlide(layout);
  const allBullets = [...slide.bullets, ...slide.columns.flatMap((c) => c.bullets)];
  switch (layout) {
    case "bullets":
      if (!next.bullets.length) {
        next.bullets = allBullets.length
          ? allBullets
          : slide.stats.length
            ? slide.stats.map((s) => `${s.value} ${s.label}`)
            : slide.timeline.length
              ? slide.timeline.map((t) => `${t.label}: ${t.text}`)
              : starter.bullets;
      }
      break;
    case "two-column":
      if (!slide.columns.some((c) => c.bullets.length)) {
        if (slide.bullets.length >= 2) {
          const half = Math.ceil(slide.bullets.length / 2);
          next.columns = [
            { heading: "", bullets: slide.bullets.slice(0, half) },
            { heading: "", bullets: slide.bullets.slice(half) },
          ];
          next.bullets = [];
        } else next.columns = starter.columns;
      }
      break;
    case "stats":
      if (!slide.stats.length) next.stats = starter.stats;
      break;
    case "timeline":
      if (!slide.timeline.length) {
        const parsed = slide.bullets.map((b) => /^([^:]{1,16}):\s*(.+)$/.exec(b));
        next.timeline =
          slide.bullets.length >= 2 && parsed.every(Boolean)
            ? parsed.map((m) => ({ label: m![1], text: m![2] }))
            : starter.timeline;
      }
      break;
    case "chart":
      if (!slide.chart.labels.length) next.chart = starter.chart;
      break;
    case "table":
      if (!slide.table.headers.length && !slide.table.rows.length) next.table = starter.table;
      break;
    case "quote":
      if (!slide.quote) next.quote = slide.body || slide.title || starter.quote;
      break;
    case "title":
    case "section":
    case "closing":
      if (!slide.subtitle) next.subtitle = slide.body;
      break;
  }
  if (!next.title && layout !== "quote") next.title = starter.title;
  return next;
}

/**
 * Coerce untrusted input (AI output, imported JSON) into a complete Slide.
 * Missing or malformed fields fall back to blanks rather than throwing.
 */
export function normalizeSlide(raw: unknown): Slide {
  const r = (raw && typeof raw === "object" ? raw : {}) as Record<string, unknown>;
  const str = (v: unknown) => (typeof v === "string" ? v : v == null ? "" : String(v));
  const strArr = (v: unknown) => (Array.isArray(v) ? v.map(str).filter((s) => s.trim() !== "") : []);
  const layout = (LAYOUTS as readonly string[]).includes(str(r.layout)) ? (r.layout as Layout) : "bullets";

  const colsIn = Array.isArray(r.columns) ? r.columns : [];
  const columns = [0, 1].map((i) => {
    const c = (colsIn[i] ?? {}) as Record<string, unknown>;
    return { heading: str(c.heading), bullets: strArr(c.bullets) };
  });

  const chartIn = (r.chart ?? {}) as Record<string, unknown>;
  const kind = ["bar", "line", "donut"].includes(str(chartIn.kind)) ? (chartIn.kind as "bar") : "bar";
  const labels = strArr(chartIn.labels);
  const values = (Array.isArray(chartIn.values) ? chartIn.values : []).map((v) => {
    const n = typeof v === "number" ? v : parseFloat(str(v).replace(/[^0-9.-]/g, ""));
    return Number.isFinite(n) ? n : 0;
  });
  const n = Math.min(labels.length, values.length);

  const tableIn = (r.table ?? {}) as Record<string, unknown>;
  const headers = Array.isArray(tableIn.headers) ? tableIn.headers.map(str) : [];
  const rows = (Array.isArray(tableIn.rows) ? tableIn.rows : [])
    .filter(Array.isArray)
    .map((row) => (row as unknown[]).map(str));

  const imageIn = (r.image ?? {}) as Record<string, unknown>;

  return blankSlide(layout, {
    id: str(r.id) || uid("s_"),
    title: str(r.title),
    subtitle: str(r.subtitle),
    body: str(r.body),
    bullets: strArr(r.bullets),
    columns,
    stats: (Array.isArray(r.stats) ? r.stats : [])
      .map((s) => {
        const o = (s ?? {}) as Record<string, unknown>;
        return { value: str(o.value), label: str(o.label) };
      })
      .filter((s) => s.value || s.label),
    quote: str(r.quote),
    attribution: str(r.attribution),
    chart: { kind, labels: labels.slice(0, n), values: values.slice(0, n), unit: str(chartIn.unit) },
    table: { headers, rows },
    timeline: (Array.isArray(r.timeline) ? r.timeline : [])
      .map((t) => {
        const o = (t ?? {}) as Record<string, unknown>;
        return { label: str(o.label), text: str(o.text) };
      })
      .filter((t) => t.label || t.text),
    image: { url: str(imageIn.url), prompt: str(imageIn.prompt ?? r.imagePrompt) },
    notes: str(r.notes),
  });
}

export function newDeck(title: string, slides: Slide[], themeId = "paper"): Deck {
  const now = Date.now();
  return {
    id: uid("d_"),
    title: title || "Untitled deck",
    themeId,
    themeOverrides: {},
    brand: { logo: "", footer: "" },
    transition: "fade",
    slides: slides.length ? slides : [starterSlide("title")],
    createdAt: now,
    updatedAt: now,
  };
}

/** Accept a deck from imported JSON, repairing anything missing. */
export function normalizeDeck(raw: unknown): Deck {
  const r = (raw && typeof raw === "object" ? raw : {}) as Record<string, unknown>;
  const slides = (Array.isArray(r.slides) ? r.slides : []).map(normalizeSlide);
  const deck = newDeck(typeof r.title === "string" ? r.title : "", slides, typeof r.themeId === "string" ? r.themeId : "paper");
  if (typeof r.id === "string" && r.id) deck.id = r.id;
  if (r.themeOverrides && typeof r.themeOverrides === "object") deck.themeOverrides = r.themeOverrides as Deck["themeOverrides"];
  const brand = (r.brand ?? {}) as Record<string, unknown>;
  deck.brand = { logo: typeof brand.logo === "string" ? brand.logo : "", footer: typeof brand.footer === "string" ? brand.footer : "" };
  if (r.transition === "none" || r.transition === "fade" || r.transition === "slide") deck.transition = r.transition;
  if (typeof r.createdAt === "number") deck.createdAt = r.createdAt;
  return deck;
}
