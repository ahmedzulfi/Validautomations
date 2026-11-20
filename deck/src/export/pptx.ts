/**
 * Native PowerPoint export: real text boxes, tables and charts that stay
 * editable in PowerPoint, Keynote and Google Slides (not screenshots).
 */
import PptxGenJS from "pptxgenjs";
import { resolveTheme } from "../themes";
import { fileName } from "./download";
import type { Deck, Slide, Theme } from "../types";

const IN = (px: number) => px / 96; // slides are designed at 1280x720 px = 13.333x7.5 in
const c = (hex: string) => hex.replace("#", "").slice(0, 6).toUpperCase();

type PSlide = ReturnType<PptxGenJS["addSlide"]>;

/** Rough line count for text in a box, so stacked boxes don't overlap (no layout engine in pptx). */
function estLines(text: string, boxPx: number, pt: number): number {
  const charPx = pt * 1.333 * 0.52; // average glyph width ~0.52em
  const perLine = Math.max(8, Math.floor(boxPx / charPx));
  return text.split("\n").reduce((n, para) => n + Math.max(1, Math.ceil(para.length / perLine)), 0);
}
const linePx = (pt: number, lh = 1.15) => pt * 1.333 * lh;

function heading(s: PSlide, text: string, t: Theme, opts: { y?: number; size?: number; color?: string; h?: number; w?: number; x?: number; align?: "left" | "center" } = {}) {
  if (!text) return;
  s.addText(text, {
    x: IN(opts.x ?? 88),
    y: IN(opts.y ?? 64),
    w: IN(opts.w ?? 1104),
    h: IN(opts.h ?? 120),
    fontFace: t.headingFont,
    fontSize: opts.size ?? 38,
    bold: true,
    color: c(opts.color ?? t.text),
    valign: "top",
    align: opts.align ?? "left",
    fit: "shrink",
    margin: 0,
  });
}

function bulletList(s: PSlide, items: string[], t: Theme, box: { x: number; y: number; w: number; h: number }, size = 22) {
  if (!items.length) return;
  s.addText(
    items.map((b) => ({ text: b, options: { bullet: { code: "25A0" }, paraSpaceAfter: 10 } })),
    {
      x: IN(box.x),
      y: IN(box.y),
      w: IN(box.w),
      h: IN(box.h),
      fontFace: t.bodyFont,
      fontSize: size,
      color: c(t.text),
      valign: "top",
      fit: "shrink",
      margin: 0,
    },
  );
}

async function imageData(url: string): Promise<string | null> {
  if (!url) return null;
  if (url.startsWith("data:")) return url;
  try {
    const res = await fetch(url);
    const blob = await res.blob();
    return await new Promise((resolve) => {
      const r = new FileReader();
      r.onload = () => resolve(String(r.result));
      r.onerror = () => resolve(null);
      r.readAsDataURL(blob);
    });
  } catch {
    return null; // cross-origin images without CORS can't be embedded
  }
}

async function addSlide(pptx: PptxGenJS, slide: Slide, deck: Deck, t: Theme, index: number) {
  const s = pptx.addSlide();
  const hero = slide.layout === "title" || slide.layout === "section" || slide.layout === "closing";
  s.background = { color: c(hero ? t.heroBg : t.bg) };
  if (slide.notes) s.addNotes(slide.notes);
  const fg = hero ? t.heroText : t.text;
  const contentTop = 64 + 110; // default top of the content area under a one-line title

  switch (slide.layout) {
    case "title":
    case "section":
    case "closing": {
      const center = slide.layout === "closing";
      s.addShape(pptx.ShapeType.roundRect, {
        x: IN(center ? 592 : 88),
        y: IN(220),
        w: IN(96),
        h: IN(10),
        fill: { color: c(slide.layout === "title" ? fg : t.primary) },
        line: { color: c(slide.layout === "title" ? fg : t.primary) },
        rectRadius: 0.05,
      });
      const heroH = estLines(slide.title, 1104, 54) * linePx(54, 1.1);
      heading(s, slide.title, t, { y: 260, size: 54, color: fg, h: heroH + 10, align: center ? "center" : "left" });
      if (slide.subtitle)
        s.addText(slide.subtitle, {
          x: IN(88),
          y: IN(260 + heroH + 30),
          w: IN(1104),
          h: IN(110),
          fontFace: t.bodyFont,
          fontSize: 24,
          color: c(fg),
          transparency: 15,
          valign: "top",
          align: center ? "center" : "left",
          margin: 0,
        });
      break;
    }
    case "quote":
      s.addText("“", { x: IN(120), y: IN(90), w: IN(200), h: IN(150), fontFace: t.headingFont, fontSize: 140, color: c(t.primary), margin: 0 });
      s.addText(slide.quote, {
        x: IN(128),
        y: IN(220),
        w: IN(1024),
        h: IN(280),
        fontFace: t.headingFont,
        fontSize: 36,
        color: c(t.text),
        valign: "top",
        fit: "shrink",
        margin: 0,
      });
      if (slide.attribution)
        s.addText(`— ${slide.attribution}`, { x: IN(128), y: IN(Math.min(560, 220 + estLines(slide.quote, 1024, 36) * linePx(36, 1.2) + 30)), w: IN(1024), h: IN(50), fontFace: t.bodyFont, fontSize: 20, color: c(t.muted), margin: 0 });
      break;
    default: {
      const titleH = slide.title ? estLines(slide.title, 1104, 38) * linePx(38) : 0;
      heading(s, slide.title, t, { h: titleH + 8 });
      let y = Math.max(contentTop - 50, 64 + titleH + 28);
      if (slide.body && slide.layout !== "image") {
        const bodyH = estLines(slide.body, 1104, 20) * linePx(20, 1.3);
        s.addText(slide.body, { x: IN(88), y: IN(y), w: IN(1104), h: IN(bodyH + 6), fontFace: t.bodyFont, fontSize: 20, color: c(t.muted), margin: 0, valign: "top" });
        y += bodyH + 20;
      }
      const area = { x: 88, y, w: 1104, h: 636 - y };
      switch (slide.layout) {
        case "bullets":
          bulletList(s, slide.bullets, t, area, 24);
          break;
        case "two-column":
          slide.columns.slice(0, 2).forEach((col, i) => {
            const x = 88 + i * 572;
            s.addShape(pptx.ShapeType.roundRect, {
              x: IN(x),
              y: IN(area.y),
              w: IN(532),
              h: IN(area.h),
              fill: { color: c(t.surface) },
              line: { color: c(t.surface) },
              rectRadius: 0.2,
            });
            s.addShape(pptx.ShapeType.rect, { x: IN(x), y: IN(area.y), w: IN(532), h: IN(8), fill: { color: c(i ? t.primary : t.muted) }, line: { color: c(i ? t.primary : t.muted) } });
            heading(s, col.heading, t, { x: x + 36, y: area.y + 32, w: 460, h: 50, size: 24 });
            bulletList(s, col.bullets, t, { x: x + 36, y: area.y + 96, w: 460, h: area.h - 120 }, 20);
          });
          break;
        case "stats": {
          const n = Math.max(slide.stats.length, 1);
          const w = (1104 - (n - 1) * 32) / n;
          slide.stats.forEach((st, i) => {
            const x = 88 + i * (w + 32);
            s.addShape(pptx.ShapeType.rect, { x: IN(x), y: IN(300), w: IN(w), h: IN(6), fill: { color: c(t.primary) }, line: { color: c(t.primary) } });
            s.addText(st.value, { x: IN(x), y: IN(320), w: IN(w), h: IN(120), fontFace: t.headingFont, fontSize: n > 3 ? 54 : 66, bold: true, color: c(t.primary), margin: 0, fit: "shrink" });
            s.addText(st.label, { x: IN(x), y: IN(450), w: IN(w), h: IN(80), fontFace: t.bodyFont, fontSize: 20, color: c(t.muted), valign: "top", margin: 0 });
          });
          break;
        }
        case "image": {
          s.addText(slide.body, { x: IN(88), y: IN(y), w: IN(480), h: IN(area.h), fontFace: t.bodyFont, fontSize: 22, color: c(t.text), valign: "top", margin: 0, fit: "shrink" });
          const data = await imageData(slide.image.url);
          if (data) s.addImage({ data, x: IN(620), y: IN(y), w: IN(572), h: IN(area.h), sizing: { type: "cover", w: IN(572), h: IN(area.h) } });
          else
            s.addShape(pptx.ShapeType.roundRect, {
              x: IN(620),
              y: IN(y),
              w: IN(572),
              h: IN(area.h),
              fill: { color: c(t.surface) },
              line: { color: c(t.surface) },
              rectRadius: 0.2,
            });
          break;
        }
        case "chart": {
          const n = Math.min(slide.chart.labels.length, slide.chart.values.length);
          if (!n) break;
          const type = slide.chart.kind === "line" ? pptx.ChartType.line : slide.chart.kind === "donut" ? pptx.ChartType.doughnut : pptx.ChartType.bar;
          s.addChart(type, [{ name: slide.title || "Series", labels: slide.chart.labels.slice(0, n), values: slide.chart.values.slice(0, n) }], {
            x: IN(area.x),
            y: IN(area.y),
            w: IN(area.w),
            h: IN(area.h),
            chartColors: slide.chart.kind === "donut" ? [c(t.primary), c(t.accent), c(t.muted), c(t.text)] : [c(t.primary)],
            showValue: slide.chart.kind !== "donut",
            showLegend: slide.chart.kind === "donut",
            legendPos: "r",
            showPercent: slide.chart.kind === "donut",
            catAxisLabelColor: c(t.muted),
            valAxisHidden: true,
            valGridLine: { style: "none" },
            dataLabelColor: c(t.text),
            dataLabelFontSize: 14,
            catAxisLabelFontSize: 14,
            legendColor: c(t.text),
            barGapWidthPct: 60,
            lineSize: 4,
          });
          break;
        }
        case "table": {
          if (!slide.table.headers.length && !slide.table.rows.length) break;
          const none: PptxGenJS.BorderProps = { type: "none" };
          const head: PptxGenJS.TableCell[] = slide.table.headers.map((h) => ({
            text: h,
            options: { bold: true, color: c(t.text), fontFace: t.headingFont, border: [none, none, { pt: 3, color: c(t.primary) }, none] },
          }));
          const body: PptxGenJS.TableRow[] = slide.table.rows.map((r, ri) =>
            r.map((cell, ci) => ({
              text: cell,
              options: { bold: ci === 0, color: c(t.text), fill: ri % 2 === 0 ? { color: c(t.surface) } : undefined },
            })),
          );
          s.addTable(head.length ? [head, ...body] : body, {
            x: IN(area.x),
            y: IN(area.y),
            w: IN(area.w),
            fontFace: t.bodyFont,
            fontSize: 16,
            border: { type: "none" },
            margin: 0.08,
          } as never);
          break;
        }
        case "timeline": {
          const n = Math.max(slide.timeline.length, 1);
          const w = 1104 / n;
          s.addShape(pptx.ShapeType.rect, { x: IN(88), y: IN(330), w: IN(1104), h: IN(4), fill: { color: c(t.muted) }, line: { color: c(t.muted) } });
          slide.timeline.forEach((it, i) => {
            const x = 88 + i * w;
            s.addShape(pptx.ShapeType.ellipse, { x: IN(x), y: IN(318), w: IN(28), h: IN(28), fill: { color: c(t.primary) }, line: { color: c(t.bg), width: 3 } });
            s.addText(it.label, { x: IN(x), y: IN(370), w: IN(w - 20), h: IN(50), fontFace: t.headingFont, fontSize: 22, bold: true, color: c(t.primary), margin: 0 });
            s.addText(it.text, { x: IN(x), y: IN(420), w: IN(w - 20), h: IN(120), fontFace: t.bodyFont, fontSize: 17, color: c(t.text), valign: "top", margin: 0 });
          });
          break;
        }
      }
      // footer
      s.addText(`${index + 1} / ${deck.slides.length}`, { x: IN(1092), y: IN(672), w: IN(100), h: IN(30), fontSize: 11, color: c(t.muted), align: "right", margin: 0 });
      if (deck.brand.footer) s.addText(deck.brand.footer, { x: IN(88), y: IN(672), w: IN(800), h: IN(30), fontSize: 11, color: c(t.muted), margin: 0 });
    }
  }
}

export async function buildPptx(deck: Deck): Promise<PptxGenJS> {
  const pptx = new PptxGenJS();
  pptx.layout = "LAYOUT_WIDE";
  pptx.title = deck.title;
  pptx.company = "Made with Slidesmith";
  const t = resolveTheme(deck);
  for (let i = 0; i < deck.slides.length; i++) await addSlide(pptx, deck.slides[i], deck, t, i);
  return pptx;
}

export async function exportPptx(deck: Deck): Promise<void> {
  const pptx = await buildPptx(deck);
  await pptx.writeFile({ fileName: `${fileName(deck.title)}.pptx` });
}

export { fileName as safeName };
