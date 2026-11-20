// @vitest-environment node
import { describe, expect, it } from "vitest";
import { deckToHtml } from "../src/export/html";
import { buildPptx, safeName } from "../src/export/pptx";
import { outlineToSlides } from "../src/outline";
import { newDeck, starterSlide } from "../src/slides";
import { TEMPLATES } from "../src/templates";
import { LAYOUTS } from "../src/types";

const everyLayout = () => newDeck("All <layouts> & co", LAYOUTS.map(starterSlide), "midnight");

describe("HTML export", () => {
  it("renders every slide into one self-contained page", () => {
    const deck = everyLayout();
    deck.slides[2].notes = "</script><b>notes</b>";
    const html = deckToHtml(deck);
    expect(html.startsWith("<!doctype html>")).toBe(true);
    expect(html.match(/class="frame"/g)).toHaveLength(LAYOUTS.length);
    expect(html).toContain("<title>All &lt;layouts&gt; &amp; co</title>");
    expect(html).toContain(".slide {"); // slide CSS inlined
    expect(html).not.toContain("</script><b>"); // notes can't break out of the script
    expect(html).not.toContain("contenteditable");
  });
});

describe("PowerPoint export", () => {
  it("builds a valid .pptx with one slide per deck slide", async () => {
    const deck = everyLayout();
    deck.slides[1].notes = "Speaker note";
    const pptx = await buildPptx(deck);
    const buf = (await pptx.write({ outputType: "nodebuffer" })) as Buffer;
    expect(buf.subarray(0, 2).toString()).toBe("PK"); // a zip
    const text = buf.toString("latin1");
    for (let i = 1; i <= LAYOUTS.length; i++) expect(text).toContain(`ppt/slides/slide${i}.xml`);
    expect(text).not.toContain(`ppt/slides/slide${LAYOUTS.length + 1}.xml`);
    expect(text).toContain("ppt/charts/chart"); // the chart slide is a native chart
  });

  it("exports every template", async () => {
    for (const t of TEMPLATES) {
      const { title, slides } = outlineToSlides(t.outline);
      const pptx = await buildPptx(newDeck(title, slides, t.themeId));
      const buf = (await pptx.write({ outputType: "nodebuffer" })) as Buffer;
      expect(buf.length).toBeGreaterThan(10_000);
    }
  });

  it("makes safe file names", () => {
    expect(safeName("Q3: Review / Final?")).toBe("Q3-Review-Final");
    expect(safeName("")).toBe("deck");
  });
});
