import { describe, expect, it } from "vitest";
import { deckToMarkdown, outlineToSlides } from "../src/outline";
import { changeLayout, newDeck, normalizeDeck, normalizeSlide, starterSlide } from "../src/slides";
import { TEMPLATES } from "../src/templates";
import { LAYOUTS } from "../src/types";

describe("outlineToSlides", () => {
  it("makes a title slide from the first H1 with its subtitle", () => {
    const { title, slides } = outlineToSlides("# My Talk\nA subtitle here\n\n## Point one\n- a\n- b");
    expect(title).toBe("My Talk");
    expect(slides[0]).toMatchObject({ layout: "title", title: "My Talk", subtitle: "A subtitle here" });
    expect(slides[1]).toMatchObject({ layout: "bullets", title: "Point one", bullets: ["a", "b"] });
  });

  it("treats later H1s as section breaks", () => {
    const { slides } = outlineToSlides("# Deck\n\n# Part two\nWhat comes next");
    expect(slides[1]).toMatchObject({ layout: "section", title: "Part two", subtitle: "What comes next" });
  });

  it("detects big numbers from bold-led bullets", () => {
    const { slides } = outlineToSlides("## Traction\n- **4,200** customers\n- **$1.9M** ARR\n- **3%** churn");
    expect(slides[0].layout).toBe("stats");
    expect(slides[0].stats).toEqual([
      { value: "4,200", label: "customers" },
      { value: "$1.9M", label: "ARR" },
      { value: "3%", label: "churn" },
    ]);
  });

  it("detects a timeline from dated label: text bullets", () => {
    const { slides } = outlineToSlides("## Roadmap\n- Q1 2027: Payroll\n- Q2 2027: Multi-entity\n- Q3 2027: Tax");
    expect(slides[0].layout).toBe("timeline");
    expect(slides[0].timeline[0]).toEqual({ label: "Q1 2027", text: "Payroll" });
  });

  it("does not mistake ordinary colon bullets for a timeline", () => {
    const { slides } = outlineToSlides("## Notes\n- Pros: fast\n- Cons: pricey\n- Verdict: buy");
    expect(slides[0].layout).toBe("bullets");
  });

  it("parses two headed lists into columns", () => {
    const { slides } = outlineToSlides("## Compare\n### Old\n- slow\n### New\n- fast\n- cheap");
    expect(slides[0].layout).toBe("two-column");
    expect(slides[0].columns).toEqual([
      { heading: "Old", bullets: ["slow"] },
      { heading: "New", bullets: ["fast", "cheap"] },
    ]);
  });

  it("parses quotes with a dash attribution on the next line", () => {
    const { slides } = outlineToSlides('> "Great product."\n— Jane, CEO');
    expect(slides[0]).toMatchObject({ layout: "quote", quote: "Great product.", attribution: "Jane, CEO" });
  });

  it("parses chart blocks with units", () => {
    const { slides } = outlineToSlides("## Revenue\n```chart line\nunit: $k\nJan: 40\nFeb: 1,250\n```");
    expect(slides[0].layout).toBe("chart");
    expect(slides[0].chart).toEqual({ kind: "line", labels: ["Jan", "Feb"], values: [40, 1250], unit: "$k" });
  });

  it("parses tables and drops the separator row", () => {
    const { slides } = outlineToSlides("## Plans\n| Plan | Price |\n| --- | :---: |\n| Free | $0 |\n| Pro | $12 |");
    expect(slides[0].layout).toBe("table");
    expect(slides[0].table).toEqual({ headers: ["Plan", "Price"], rows: [["Free", "$0"], ["Pro", "$12"]] });
  });

  it("parses images, speaker notes, layout directives and --- separators", () => {
    const md = "## Picture\n![a lighthouse at dusk](https://x.test/a.jpg)\nNotes: say hello\nand wave\n---\nLoose slide\n<!-- layout: closing -->";
    const { slides } = outlineToSlides(md);
    expect(slides[0]).toMatchObject({ layout: "image", image: { prompt: "a lighthouse at dusk", url: "https://x.test/a.jpg" } });
    expect(slides[0].notes).toBe("say hello\nand wave");
    expect(slides[1]).toMatchObject({ layout: "closing", title: "Loose slide" });
  });

  it("strips inline markdown from text", () => {
    const { slides } = outlineToSlides("## **Bold** and `code`\n- see [docs](https://x.test) *now*");
    expect(slides[0].title).toBe("Bold and code");
    expect(slides[0].bullets).toEqual(["see docs now"]);
  });

  it("keeps every slide of the pitch template, including the comparison and the quote", () => {
    const pitch = TEMPLATES.find((t) => t.id === "pitch")!;
    const layouts = outlineToSlides(pitch.outline).slides.map((s) => s.layout);
    expect(layouts).toEqual(["title", "bullets", "image", "stats", "chart", "two-column", "quote", "timeline", "closing"]);
  });

  it("builds every template without empty slides", () => {
    for (const t of TEMPLATES) {
      const { slides } = outlineToSlides(t.outline);
      expect(slides.length).toBeGreaterThan(4);
      for (const s of slides) expect(s.title || s.quote).toBeTruthy();
      expect(slides[0].layout).toBe("title");
    }
  });
});

describe("deckToMarkdown", () => {
  it("round-trips every layout through markdown", () => {
    const slides = LAYOUTS.map((l) => starterSlide(l));
    const deck = newDeck("Round trip", slides);
    const back = outlineToSlides(deckToMarkdown(deck)).slides;
    expect(back.map((s) => s.layout)).toEqual(slides.map((s) => s.layout));
    const strip = (s: (typeof slides)[number]) => ({ ...s, id: "", image: { ...s.image, url: "" } });
    back.forEach((s, i) => {
      const a = strip(s);
      const b = strip(slides[i]);
      expect(a.bullets).toEqual(b.bullets);
      expect(a.stats).toEqual(b.stats);
      expect(a.timeline).toEqual(b.timeline);
      expect(a.chart).toEqual(b.chart);
      expect(a.table).toEqual(b.table);
      if (s.layout === "two-column") expect(a.columns).toEqual(b.columns);
      if (s.layout === "quote") expect([a.quote, a.attribution]).toEqual([b.quote, b.attribution]);
    });
  });

  it("keeps speaker notes", () => {
    const s = starterSlide("bullets");
    s.notes = "Remember the demo";
    const md = deckToMarkdown(newDeck("x", [starterSlide("title"), s]));
    expect(outlineToSlides(md).slides[1].notes).toBe("Remember the demo");
  });
});

describe("normalize", () => {
  it("repairs junk AI/imported slides instead of throwing", () => {
    const s = normalizeSlide({ layout: "nope", title: 5, bullets: ["a", "", null, "b"], chart: { kind: "pie", labels: ["x", "y", "z"], values: ["1", "2.5%"] } });
    expect(s.layout).toBe("bullets");
    expect(s.title).toBe("5");
    expect(s.bullets).toEqual(["a", "b"]);
    expect(s.chart).toEqual({ kind: "bar", labels: ["x", "y"], values: [1, 2.5], unit: "" });
    expect(s.columns).toHaveLength(2);
  });

  it("normalizes decks from JSON", () => {
    const d = normalizeDeck({ title: "T", slides: [{ layout: "quote", quote: "q" }], transition: "slide", brand: { footer: "F" } });
    expect(d.title).toBe("T");
    expect(d.slides[0].layout).toBe("quote");
    expect(d.transition).toBe("slide");
    expect(d.brand).toEqual({ logo: "", footer: "F" });
    expect(normalizeDeck(null).slides.length).toBe(1);
  });
});

describe("changeLayout", () => {
  it("splits bullets into two columns", () => {
    const s = { ...starterSlide("bullets"), bullets: ["a", "b", "c", "d"] };
    const n = changeLayout(s, "two-column");
    expect(n.columns.map((c) => c.bullets)).toEqual([
      ["a", "b"],
      ["c", "d"],
    ]);
    expect(changeLayout(n, "bullets").bullets).toEqual(["a", "b", "c", "d"]);
  });

  it("turns label: text bullets into a timeline and seeds empty layouts", () => {
    const s = { ...starterSlide("bullets"), bullets: ["2024: start", "2025: grow"] };
    expect(changeLayout(s, "timeline").timeline).toEqual([
      { label: "2024", text: "start" },
      { label: "2025", text: "grow" },
    ]);
    expect(changeLayout(s, "chart").chart.labels.length).toBeGreaterThan(0);
  });
});
