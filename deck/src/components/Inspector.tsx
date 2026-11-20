import { useState } from "react";
import { readDataUrl } from "../export/download";
import { FONT_CHOICES, THEMES, resolveTheme } from "../themes";
import { LAYOUTS, LAYOUT_LABELS, type ChartKind, type Deck, type Layout, type Slide, type Theme } from "../types";

interface Props {
  deck: Deck;
  slide: Slide;
  onSlide: (patch: Partial<Slide>) => void;
  onDeck: (patch: Partial<Deck>) => void;
  onAiSlide: (instruction: string) => Promise<void>;
  aiBusy: boolean;
  hasKey: boolean;
}

const QUICK = ["Make it shorter", "More persuasive", "Turn into big numbers", "Simplify the wording", "Add a concrete example"];

export function Inspector(props: Props) {
  const [tab, setTab] = useState<"slide" | "design">("slide");
  return (
    <aside className="inspector">
      <div className="tabs" role="tablist">
        <button role="tab" aria-selected={tab === "slide"} className={tab === "slide" ? "on" : ""} onClick={() => setTab("slide")}>
          Slide
        </button>
        <button role="tab" aria-selected={tab === "design"} className={tab === "design" ? "on" : ""} onClick={() => setTab("design")}>
          Design
        </button>
      </div>
      {tab === "slide" ? <SlidePanel {...props} /> : <DesignPanel deck={props.deck} onDeck={props.onDeck} />}
    </aside>
  );
}

function SlidePanel({ slide, onSlide, onAiSlide, aiBusy, hasKey }: Props) {
  const [instruction, setInstruction] = useState("");
  const run = async (text: string) => {
    if (!text.trim()) return;
    await onAiSlide(text);
    setInstruction("");
  };
  return (
    <div className="panel">
      <div className="group ai-group">
        <label className="label">Edit this slide with AI</label>
        <textarea
          rows={2}
          value={instruction}
          disabled={aiBusy}
          placeholder={hasKey ? "e.g. Make the title a claim, add a stat" : "Add an API key in Settings to use AI"}
          onChange={(e) => setInstruction(e.target.value)}
          onKeyDown={(e) => {
            e.stopPropagation();
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              run(instruction);
            }
          }}
        />
        <div className="quick">
          {QUICK.map((q) => (
            <button key={q} className="pill" disabled={aiBusy || !hasKey} onClick={() => run(q)}>
              {q}
            </button>
          ))}
        </div>
        {aiBusy && <div className="progress-line" aria-hidden />}
      </div>

      <div className="group">
        <label className="label">Layout</label>
        <div className="layout-grid">
          {LAYOUTS.map((l) => (
            <button key={l} className={"layout-btn" + (slide.layout === l ? " on" : "")} onClick={() => onSlide({ layout: l })}>
              <LayoutIcon layout={l} />
              {LAYOUT_LABELS[l]}
            </button>
          ))}
        </div>
      </div>

      <LayoutFields slide={slide} onSlide={onSlide} />

      <div className="group">
        <label className="label" htmlFor="notes">
          Speaker notes
        </label>
        <textarea id="notes" rows={4} value={slide.notes} onChange={(e) => onSlide({ notes: e.target.value })} onKeyDown={(e) => e.stopPropagation()} />
      </div>
    </div>
  );
}

const lines = (s: string) => s.split("\n").map((x) => x.trim()).filter(Boolean);

function ListField({ label, value, onChange }: { label: string; value: string[]; onChange: (v: string[]) => void }) {
  // Local text so typing a newline doesn't get normalized away mid-edit.
  const [text, setText] = useState(value.join("\n"));
  const [synced, setSynced] = useState(value);
  if (synced !== value && lines(text).join("\n") !== value.join("\n")) {
    setSynced(value);
    setText(value.join("\n"));
  }
  return (
    <div className="group">
      <label className="label">{label}</label>
      <textarea
        rows={Math.max(3, value.length + 1)}
        value={text}
        onKeyDown={(e) => e.stopPropagation()}
        onChange={(e) => {
          setText(e.target.value);
          onChange(lines(e.target.value));
        }}
      />
      <span className="hint">One per line</span>
    </div>
  );
}

function PairsField({
  label,
  a,
  b,
  rows,
  onChange,
  max = 8,
}: {
  label: string;
  a: string;
  b: string;
  rows: [string, string][];
  onChange: (rows: [string, string][]) => void;
  max?: number;
}) {
  return (
    <div className="group">
      <label className="label">{label}</label>
      <div className="pairs">
        <span className="pairs-h">{a}</span>
        <span className="pairs-h">{b}</span>
        <span />
        {rows.map((r, i) => (
          <PairRow
            key={i}
            row={r}
            onChange={(nr) => onChange(rows.map((x, j) => (j === i ? nr : x)))}
            onRemove={() => onChange(rows.filter((_, j) => j !== i))}
          />
        ))}
      </div>
      {rows.length < max && (
        <button className="btn ghost sm" onClick={() => onChange([...rows, ["", ""]])}>
          + Add row
        </button>
      )}
    </div>
  );
}

function PairRow({ row, onChange, onRemove }: { row: [string, string]; onChange: (r: [string, string]) => void; onRemove: () => void }) {
  return (
    <>
      <input value={row[0]} onChange={(e) => onChange([e.target.value, row[1]])} onKeyDown={(e) => e.stopPropagation()} />
      <input value={row[1]} onChange={(e) => onChange([row[0], e.target.value])} onKeyDown={(e) => e.stopPropagation()} />
      <button className="icon-btn" aria-label="Remove row" onClick={onRemove}>
        ×
      </button>
    </>
  );
}

function LayoutFields({ slide, onSlide }: { slide: Slide; onSlide: (p: Partial<Slide>) => void }) {
  switch (slide.layout) {
    case "bullets":
      return <ListField key={slide.id} label="Bullets" value={slide.bullets} onChange={(bullets) => onSlide({ bullets })} />;
    case "two-column":
      return (
        <>
          {slide.columns.slice(0, 2).map((c, i) => (
            <ListField
              key={slide.id + i}
              label={`${c.heading || (i ? "Right" : "Left")} column`}
              value={c.bullets}
              onChange={(b) => onSlide({ columns: slide.columns.map((x, j) => (j === i ? { ...x, bullets: b } : x)) })}
            />
          ))}
        </>
      );
    case "stats":
      return (
        <PairsField
          label="Big numbers"
          a="Value"
          b="Label"
          max={4}
          rows={slide.stats.map((s) => [s.value, s.label])}
          onChange={(r) => onSlide({ stats: r.map(([value, label]) => ({ value, label })) })}
        />
      );
    case "timeline":
      return (
        <PairsField
          label="Timeline"
          a="When"
          b="What"
          max={6}
          rows={slide.timeline.map((t) => [t.label, t.text])}
          onChange={(r) => onSlide({ timeline: r.map(([label, text]) => ({ label, text })) })}
        />
      );
    case "chart":
      return (
        <>
          <div className="group row">
            <label className="field-inline">
              Type
              <select value={slide.chart.kind} onChange={(e) => onSlide({ chart: { ...slide.chart, kind: e.target.value as ChartKind } })}>
                <option value="bar">Bar</option>
                <option value="line">Line</option>
                <option value="donut">Donut</option>
              </select>
            </label>
            <label className="field-inline">
              Unit
              <input
                value={slide.chart.unit}
                placeholder="%, $M…"
                onKeyDown={(e) => e.stopPropagation()}
                onChange={(e) => onSlide({ chart: { ...slide.chart, unit: e.target.value } })}
              />
            </label>
          </div>
          <PairsField
            label="Data"
            a="Label"
            b="Value"
            max={12}
            rows={slide.chart.labels.map((l, i) => [l, String(slide.chart.values[i] ?? "")])}
            onChange={(r) =>
              onSlide({
                chart: {
                  ...slide.chart,
                  labels: r.map((x) => x[0]),
                  values: r.map((x) => {
                    const n = parseFloat(x[1].replace(/[^0-9.-]/g, ""));
                    return Number.isFinite(n) ? n : 0;
                  }),
                },
              })
            }
          />
        </>
      );
    case "table":
      return <TableField slide={slide} onSlide={onSlide} />;
    case "image":
      return <ImageField slide={slide} onSlide={onSlide} />;
    default:
      return null;
  }
}

function TableField({ slide, onSlide }: { slide: Slide; onSlide: (p: Partial<Slide>) => void }) {
  const toText = () => [slide.table.headers, ...slide.table.rows].map((r) => r.join(" | ")).join("\n");
  const [text, setText] = useState(toText);
  return (
    <div className="group">
      <label className="label">Table</label>
      <textarea
        rows={6}
        className="mono"
        value={text}
        onKeyDown={(e) => e.stopPropagation()}
        onChange={(e) => {
          setText(e.target.value);
          const rows = lines(e.target.value).map((l) => l.split("|").map((c) => c.trim()));
          onSlide({ table: { headers: rows[0] ?? [], rows: rows.slice(1) } });
        }}
      />
      <span className="hint">First line is the header. Separate cells with |</span>
    </div>
  );
}

function ImageField({ slide, onSlide }: { slide: Slide; onSlide: (p: Partial<Slide>) => void }) {
  return (
    <div className="group">
      <label className="label">Image</label>
      <input
        placeholder="Paste an image URL"
        value={slide.image.url.startsWith("data:") ? "" : slide.image.url}
        onKeyDown={(e) => e.stopPropagation()}
        onChange={(e) => onSlide({ image: { ...slide.image, url: e.target.value.trim() } })}
      />
      <div className="row">
        <label className="btn ghost sm">
          Upload…
          <input
            type="file"
            accept="image/*"
            hidden
            onChange={async (e) => {
              const f = e.target.files?.[0];
              if (f) onSlide({ image: { ...slide.image, url: await readDataUrl(f) } });
            }}
          />
        </label>
        {slide.image.url && (
          <button className="btn ghost sm" onClick={() => onSlide({ image: { ...slide.image, url: "" } })}>
            Remove image
          </button>
        )}
      </div>
      <label className="label sub">Image idea (alt text)</label>
      <textarea
        rows={2}
        value={slide.image.prompt}
        onKeyDown={(e) => e.stopPropagation()}
        onChange={(e) => onSlide({ image: { ...slide.image, prompt: e.target.value } })}
      />
      {slide.image.prompt && (
        <a className="hint link" href={`https://unsplash.com/s/photos/${encodeURIComponent(slide.image.prompt.split(" ").slice(0, 4).join(" "))}`} target="_blank" rel="noreferrer">
          Find a matching photo on Unsplash ↗
        </a>
      )}
    </div>
  );
}

function DesignPanel({ deck, onDeck }: { deck: Deck; onDeck: (p: Partial<Deck>) => void }) {
  const theme = resolveTheme(deck);
  const override = (k: keyof Theme, v: string) => onDeck({ themeOverrides: { ...deck.themeOverrides, [k]: v } });
  return (
    <div className="panel">
      <div className="group">
        <label className="label">Theme</label>
        <div className="theme-list">
          {THEMES.map((t) => (
            <button
              key={t.id}
              className={"theme-row" + (deck.themeId === t.id ? " on" : "")}
              onClick={() => onDeck({ themeId: t.id, themeOverrides: {} })}
              style={{ background: t.bg, color: t.text, fontFamily: `"${t.headingFont}"` }}
            >
              <span>{t.name}</span>
              <span className="dots">
                <i style={{ background: t.primary }} />
                <i style={{ background: t.accent }} />
                <i style={{ background: t.heroBg }} />
              </span>
            </button>
          ))}
        </div>
      </div>

      <div className="group">
        <label className="label">Brand kit</label>
        <div className="colors">
          {(
            [
              ["primary", "Primary"],
              ["accent", "Accent"],
              ["bg", "Background"],
              ["text", "Text"],
              ["heroBg", "Title slides"],
              ["heroText", "Title text"],
            ] as [keyof Theme, string][]
          ).map(([k, label]) => (
            <label key={k} className="color">
              <input type="color" value={String(theme[k])} onChange={(e) => override(k, e.target.value)} />
              {label}
            </label>
          ))}
        </div>
        <div className="row">
          <label className="field-inline">
            Headings
            <select value={theme.headingFont} onChange={(e) => override("headingFont", e.target.value)}>
              {FONT_CHOICES.map((f) => (
                <option key={f}>{f}</option>
              ))}
            </select>
          </label>
          <label className="field-inline">
            Body
            <select value={theme.bodyFont} onChange={(e) => override("bodyFont", e.target.value)}>
              {FONT_CHOICES.map((f) => (
                <option key={f}>{f}</option>
              ))}
            </select>
          </label>
        </div>
        {Object.keys(deck.themeOverrides).length > 0 && (
          <button className="btn ghost sm" onClick={() => onDeck({ themeOverrides: {} })}>
            Reset to theme defaults
          </button>
        )}
      </div>

      <div className="group">
        <label className="label">Logo</label>
        <div className="row">
          <label className="btn ghost sm">
            {deck.brand.logo ? "Replace logo…" : "Upload logo…"}
            <input
              type="file"
              accept="image/*"
              hidden
              onChange={async (e) => {
                const f = e.target.files?.[0];
                if (f) onDeck({ brand: { ...deck.brand, logo: await readDataUrl(f) } });
              }}
            />
          </label>
          {deck.brand.logo && (
            <button className="btn ghost sm" onClick={() => onDeck({ brand: { ...deck.brand, logo: "" } })}>
              Remove
            </button>
          )}
        </div>
        <label className="label sub">Footer text</label>
        <input
          value={deck.brand.footer}
          placeholder="Company · Confidential"
          onKeyDown={(e) => e.stopPropagation()}
          onChange={(e) => onDeck({ brand: { ...deck.brand, footer: e.target.value } })}
        />
      </div>

      <div className="group">
        <label className="label">Transition when presenting</label>
        <div className="seg">
          {(["none", "fade", "slide"] as const).map((t) => (
            <button key={t} className={deck.transition === t ? "on" : ""} onClick={() => onDeck({ transition: t })}>
              {t[0].toUpperCase() + t.slice(1)}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

function LayoutIcon({ layout }: { layout: Layout }) {
  const r = (x: number, y: number, w: number, h: number, o = 1) => <rect x={x} y={y} width={w} height={h} rx={1} opacity={o} />;
  const shapes: Record<Layout, JSX.Element> = {
    title: <>{r(4, 8, 20, 4)}{r(4, 14, 12, 2, 0.5)}</>,
    section: <>{r(4, 6, 6, 2, 0.5)}{r(4, 10, 20, 4)}</>,
    bullets: <>{r(4, 4, 16, 3)}{r(4, 10, 2, 2)}{r(8, 10, 14, 2, 0.5)}{r(4, 14, 2, 2)}{r(8, 14, 12, 2, 0.5)}</>,
    "two-column": <>{r(4, 4, 16, 3)}{r(4, 10, 9, 9, 0.5)}{r(15, 10, 9, 9, 0.5)}</>,
    stats: <>{r(4, 4, 16, 3)}{r(4, 10, 6, 5)}{r(11, 10, 6, 5)}{r(18, 10, 6, 5)}</>,
    quote: <>{r(4, 6, 3, 3)}{r(6, 10, 18, 3)}{r(6, 15, 8, 2, 0.5)}</>,
    image: <>{r(4, 4, 10, 3)}{r(4, 9, 8, 2, 0.5)}{r(15, 4, 9, 15, 0.5)}</>,
    chart: <>{r(4, 4, 16, 3)}{r(5, 14, 3, 5)}{r(10, 11, 3, 8)}{r(15, 8, 3, 11)}{r(20, 12, 3, 7, 0.5)}</>,
    table: <>{r(4, 4, 16, 3)}{r(4, 9, 20, 2)}{r(4, 12, 20, 2, 0.5)}{r(4, 15, 20, 2, 0.5)}</>,
    timeline: <>{r(4, 4, 16, 3)}{r(4, 12, 20, 1, 0.5)}{r(5, 11, 3, 3)}{r(13, 11, 3, 3)}{r(21, 11, 3, 3)}</>,
    closing: <>{r(6, 9, 16, 4)}{r(9, 15, 10, 2, 0.5)}</>,
  };
  return (
    <svg viewBox="0 0 28 22" width="28" height="22" fill="currentColor" aria-hidden>
      <rect x="0.5" y="0.5" width="27" height="21" rx="2" fill="none" stroke="currentColor" opacity={0.3} />
      {shapes[layout]}
    </svg>
  );
}
