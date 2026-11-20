/**
 * Markdown outline <-> deck.
 *
 * This is the no-AI path: anyone can paste an outline and get a real deck,
 * and every deck can round-trip back to markdown.
 *
 * Format (all optional, heuristics fill the gaps):
 *   # Deck title            first H1 -> title slide (next plain line = subtitle)
 *   # Section               later H1 -> section break
 *   ## Slide title          starts a content slide
 *   ---                     also starts a new slide
 *   <!-- layout: stats -->  force a layout
 *   - bullet                bullets
 *   - **42%** Growth        all bold-led bullets -> big numbers
 *   - Q1 2026: Launch       3+ "label: text" bullets with digits in the label -> timeline
 *   ### Left / ### Right    two headed lists -> two columns
 *   > Quote text            quote; a following "— Name" line is the attribution
 *   ![prompt](url)          image slide
 *   | a | b |               table
 *   ```chart bar            chart block, lines of "Label: 12"; optional "unit: %"
 *   Notes:                  everything after is speaker notes
 */
import { blankSlide } from "./slides";
import { LAYOUTS, type ChartKind, type Deck, type Layout, type Slide } from "./types";

interface Chunk {
  heading: 1 | 2 | 0;
  title: string;
  lines: string[];
}

function splitChunks(md: string): Chunk[] {
  const chunks: Chunk[] = [];
  let cur: Chunk | null = null;
  let inFence = false;
  for (const raw of md.replace(/\r\n?/g, "\n").split("\n")) {
    const line = raw.replace(/\s+$/, "");
    if (/^```/.test(line.trim())) inFence = !inFence;
    if (!inFence) {
      const h = /^(#{1,2})\s+(.*)$/.exec(line);
      if (h) {
        cur = { heading: h[1].length as 1 | 2, title: h[2].trim(), lines: [] };
        chunks.push(cur);
        continue;
      }
      if (/^(-{3,}|\*{3,})$/.test(line.trim())) {
        cur = null;
        continue;
      }
    }
    if (!cur) {
      if (line.trim() === "") continue;
      cur = { heading: 0, title: "", lines: [] };
      chunks.push(cur);
    }
    cur.lines.push(line);
  }
  return chunks.filter((c) => c.title || c.lines.some((l) => l.trim()));
}

const stripMd = (s: string) =>
  s
    .replace(/\*\*(.+?)\*\*/g, "$1")
    .replace(/__(.+?)__/g, "$1")
    .replace(/(^|\W)[*_](.+?)[*_](?=\W|$)/g, "$1$2")
    .replace(/`([^`]+)`/g, "$1")
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .trim();

function parseChunk(chunk: Chunk, index: number): Slide {
  const slide = blankSlide("bullets", { title: stripMd(chunk.title) });
  let forced: Layout | null = null;
  const bullets: string[] = [];
  const columns: { heading: string; bullets: string[] }[] = [];
  const paragraphs: string[] = [];
  const quoteLines: string[] = [];
  const tableLines: string[] = [];
  const notes: string[] = [];
  let chart: Slide["chart"] | null = null;
  let inNotes = false;
  let inChart = false;

  for (const line of chunk.lines) {
    const t = line.trim();
    if (inNotes) {
      notes.push(line);
      continue;
    }
    if (inChart) {
      if (/^```/.test(t)) {
        inChart = false;
        continue;
      }
      const unit = /^unit:\s*(.*)$/i.exec(t);
      if (unit && chart) {
        chart.unit = unit[1].trim();
        continue;
      }
      const kv = /^(.+?):\s*(-?[\d.,]+)\s*%?$/.exec(t);
      if (kv && chart) {
        chart.labels.push(kv[1].replace(/^[-*]\s*/, "").trim());
        chart.values.push(parseFloat(kv[2].replace(/,/g, "")));
      }
      continue;
    }
    const layoutDirective = /^<!--\s*layout:\s*([\w-]+)\s*-->$/i.exec(t);
    if (layoutDirective) {
      const l = layoutDirective[1].toLowerCase();
      if ((LAYOUTS as readonly string[]).includes(l)) forced = l as Layout;
      continue;
    }
    if (/^notes?:/i.test(t)) {
      inNotes = true;
      const rest = t.replace(/^notes?:\s*/i, "");
      if (rest) notes.push(rest);
      continue;
    }
    const fence = /^```\s*chart\s*(bar|line|donut|pie)?/i.exec(t);
    if (fence) {
      const kind = (fence[1]?.toLowerCase() === "pie" ? "donut" : fence[1]?.toLowerCase() || "bar") as ChartKind;
      chart = { kind, labels: [], values: [], unit: "" };
      inChart = true;
      continue;
    }
    if (t === "") continue;
    const sub = /^###\s+(.*)$/.exec(t);
    if (sub) {
      columns.push({ heading: stripMd(sub[1]), bullets: [] });
      continue;
    }
    const img = /^!\[([^\]]*)\]\(([^)]*)\)/.exec(t);
    if (img) {
      slide.image = { prompt: img[1].trim(), url: img[2].trim() };
      continue;
    }
    if (t.startsWith("|")) {
      tableLines.push(t);
      continue;
    }
    if (t.startsWith(">")) {
      quoteLines.push(t.replace(/^>\s?/, ""));
      continue;
    }
    const attrib = /^(—|–|--)\s*(.+)$/.exec(t);
    if (attrib && quoteLines.length) {
      slide.attribution = stripMd(attrib[2]);
      continue;
    }
    const bullet = /^\s*(?:[-*+]|\d+[.)])\s+(.*)$/.exec(line);
    if (bullet) {
      if (columns.length) columns[columns.length - 1].bullets.push(bullet[1]);
      else bullets.push(bullet[1]);
      continue;
    }
    paragraphs.push(t);
  }

  slide.notes = notes.join("\n").trim();
  if (chunk.heading === 0 && !slide.title && paragraphs.length) slide.title = stripMd(paragraphs.shift()!);

  // Quote
  if (quoteLines.length) {
    const q = quoteLines.join(" ");
    const inline = /^(.*?)\s+(?:—|–|--)\s*(.+)$/.exec(q);
    if (inline && !slide.attribution) {
      slide.quote = stripMd(inline[1]).replace(/^["“]|["”]$/g, "");
      slide.attribution = stripMd(inline[2]);
    } else slide.quote = stripMd(q).replace(/^["“]|["”]$/g, "");
  }

  // Table
  if (tableLines.length) {
    const cells = (l: string) =>
      l
        .replace(/^\|/, "")
        .replace(/\|$/, "")
        .split("|")
        .map((c) => stripMd(c));
    const rows = tableLines.filter((l) => !(/^[\s|:-]+$/.test(l) && l.includes("-")));
    if (rows.length) {
      slide.table = { headers: cells(rows[0]), rows: rows.slice(1).map(cells) };
    }
  }

  if (chart && chart.labels.length) slide.chart = chart;

  // Stats: every bullet starts with a bold value
  const statMatches = bullets.map((b) => /^\*\*(.+?)\*\*\s*[:—–-]?\s*(.*)$/.exec(b.trim()));
  const isStats = bullets.length >= 2 && bullets.length <= 4 && statMatches.every(Boolean);

  // Timeline: 3+ "label: text" bullets whose labels contain a digit
  const tlMatches = bullets.map((b) => /^([^:]{1,16}):\s+(.+)$/.exec(stripMd(b)));
  const isTimeline = bullets.length >= 3 && tlMatches.every((m) => m && /\d/.test(m[1]));

  if (columns.length >= 2) {
    slide.columns = columns.slice(0, 2).map((c) => ({ heading: c.heading, bullets: c.bullets.map(stripMd) }));
  }
  if (isStats) slide.stats = statMatches.map((m) => ({ value: stripMd(m![1]), label: stripMd(m![2]) }));
  if (isTimeline) slide.timeline = tlMatches.map((m) => ({ label: m![1].trim(), text: m![2].trim() }));
  slide.bullets = bullets.map(stripMd);
  slide.body = stripMd(paragraphs.join(" "));

  let layout: Layout;
  if (forced) layout = forced;
  else if (chunk.heading === 1) layout = index === 0 ? "title" : "section";
  else if (slide.quote) layout = "quote";
  else if (chart && chart.labels.length) layout = "chart";
  else if (tableLines.length) layout = "table";
  else if (slide.image.url || slide.image.prompt) layout = "image";
  else if (columns.length >= 2) layout = "two-column";
  else if (isStats) layout = "stats";
  else if (isTimeline) layout = "timeline";
  else if (!bullets.length && /\b(thank|thanks|questions|contact|let'?s talk)\b/i.test(slide.title)) layout = "closing";
  else layout = "bullets";

  if ((layout === "title" || layout === "section" || layout === "closing") && !slide.subtitle && slide.body) {
    slide.subtitle = slide.body;
    slide.body = "";
  }
  if (layout === "stats" || layout === "timeline") slide.bullets = [];
  slide.layout = layout;
  return slide;
}

export function outlineToSlides(md: string): { title: string; slides: Slide[] } {
  const chunks = splitChunks(md);
  const slides = chunks.map((c, i) => parseChunk(c, i));
  const first = slides[0];
  const title = first?.title || "Untitled deck";
  return { title, slides };
}

// ---------- deck -> markdown ----------

export function deckToMarkdown(deck: Deck): string {
  const out: string[] = [];
  deck.slides.forEach((s, i) => {
    const block: string[] = [];
    const heading = s.layout === "title" || s.layout === "section" ? "#" : "##";
    if (i > 0) block.push("---", "");
    block.push(`${heading} ${s.title || LAYOUT_FALLBACK[s.layout]}`);
    const inferred = parseChunk({ heading: 0, title: "", lines: [] }, 1).layout; // "bullets"
    const needsDirective = !(
      (s.layout === "title" && i === 0) ||
      (s.layout === "section" && i > 0) ||
      s.layout === inferred
    );
    if (needsDirective) block.push(`<!-- layout: ${s.layout} -->`);
    if (s.subtitle) block.push(s.subtitle);
    if (s.body) block.push("", s.body);
    switch (s.layout) {
      case "stats":
        block.push("", ...s.stats.map((st) => `- **${st.value}** ${st.label}`));
        break;
      case "timeline":
        block.push("", ...s.timeline.map((t) => `- ${t.label}: ${t.text}`));
        break;
      case "two-column":
        s.columns.forEach((c) => block.push("", `### ${c.heading || "Column"}`, ...c.bullets.map((b) => `- ${b}`)));
        break;
      case "quote":
        block.push("", `> ${s.quote}`);
        if (s.attribution) block.push(`— ${s.attribution}`);
        break;
      case "chart":
        block.push("", "```chart " + s.chart.kind);
        if (s.chart.unit) block.push(`unit: ${s.chart.unit}`);
        s.chart.labels.forEach((l, j) => block.push(`${l}: ${s.chart.values[j] ?? 0}`));
        block.push("```");
        break;
      case "table":
        if (s.table.headers.length) {
          block.push("", `| ${s.table.headers.join(" | ")} |`, `| ${s.table.headers.map(() => "---").join(" | ")} |`);
          s.table.rows.forEach((r) => block.push(`| ${r.join(" | ")} |`));
        }
        break;
      case "image":
        block.push("", `![${s.image.prompt}](${s.image.url.startsWith("data:") ? "" : s.image.url})`);
        break;
    }
    if (s.bullets.length && s.layout !== "stats" && s.layout !== "timeline") block.push("", ...s.bullets.map((b) => `- ${b}`));
    if (s.notes) block.push("", "Notes:", s.notes);
    out.push(block.join("\n"));
  });
  return out.join("\n\n") + "\n";
}

const LAYOUT_FALLBACK: Record<Layout, string> = {
  title: "Untitled",
  section: "Section",
  bullets: "Slide",
  "two-column": "Comparison",
  stats: "Numbers",
  quote: "Quote",
  image: "Image",
  chart: "Chart",
  table: "Table",
  timeline: "Timeline",
  closing: "Thank you",
};
