import type { CSSProperties, KeyboardEvent, ReactNode } from "react";
import { themeVars } from "../themes";
import type { Brand, Slide, Theme } from "../types";
import { Chart } from "./Chart";

export const SLIDE_W = 1280;
export const SLIDE_H = 720;

type Edit = (patch: Partial<Slide>) => void;

interface Props {
  slide: Slide;
  theme: Theme;
  brand: Brand;
  index: number;
  total: number;
  /** When provided, text on the slide becomes editable in place. */
  onEdit?: Edit;
}

/** Inline-editable text. Without onChange it's plain text, so the same tree renders for export. */
function T({
  value,
  onChange,
  className,
  placeholder,
  as = "div",
  multiline = false,
}: {
  value: string;
  onChange?: (v: string) => void;
  className?: string;
  placeholder?: string;
  as?: "div" | "span" | "h1" | "h2" | "p" | "li" | "blockquote";
  multiline?: boolean;
}) {
  const Tag = as;
  if (!onChange) return value ? <Tag className={className}>{value}</Tag> : null;
  const onKeyDown = (e: KeyboardEvent<HTMLElement>) => {
    if (e.key === "Enter" && !(multiline && e.shiftKey)) {
      e.preventDefault();
      e.currentTarget.blur();
    }
    if (e.key === "Escape") e.currentTarget.blur();
    e.stopPropagation(); // keep editor shortcuts from firing while typing
  };
  return (
    <Tag
      key={value}
      className={(className ?? "") + " s-editable"}
      contentEditable
      suppressContentEditableWarning
      spellCheck
      data-placeholder={placeholder}
      onKeyDown={onKeyDown}
      onBlur={(e) => {
        const next = e.currentTarget.innerText.replace(/\n+$/, "");
        if (next !== value) onChange(next);
      }}
    >
      {value}
    </Tag>
  );
}

function Bullets({ items, onChange, className }: { items: string[]; onChange?: (b: string[]) => void; className?: string }) {
  if (!items.length && !onChange) return null;
  const list = onChange && !items.length ? [""] : items;
  return (
    <ul className={"s-bullets " + (className ?? "")}>
      {list.map((b, i) => (
        <T
          key={i + ":" + b}
          as="li"
          value={b}
          placeholder="Bullet point"
          onChange={
            onChange &&
            ((v) => {
              const next = [...list];
              if (v.trim()) next[i] = v;
              else next.splice(i, 1);
              onChange(next.filter((x) => x.trim()));
            })
          }
        />
      ))}
    </ul>
  );
}

function ImageBox({ slide }: { slide: Slide }) {
  if (slide.image.url) return <img className="s-image" src={slide.image.url} alt={slide.image.prompt || slide.title} />;
  return (
    <div className="s-image s-image-placeholder" aria-label="Image placeholder">
      <span>{slide.image.prompt || "Add an image in the panel"}</span>
    </div>
  );
}

export function SlideView({ slide, theme, brand, index, total, onEdit }: Props) {
  const e = onEdit;
  const set = (k: keyof Slide) => (e ? (v: string) => e({ [k]: v } as Partial<Slide>) : undefined);
  const hero = slide.layout === "title" || slide.layout === "section" || slide.layout === "closing";

  let content: ReactNode;
  switch (slide.layout) {
    case "title":
      content = (
        <div className="s-title-wrap">
          <T as="h1" className="s-h1" value={slide.title} onChange={set("title")} placeholder="Title" />
          <T className="s-subtitle" value={slide.subtitle} onChange={set("subtitle")} placeholder="Subtitle" />
        </div>
      );
      break;
    case "section":
      content = (
        <div className="s-section-wrap">
          <div className="s-section-num">{String(index + 1).padStart(2, "0")}</div>
          <T as="h1" className="s-h1" value={slide.title} onChange={set("title")} placeholder="Section title" />
          <T className="s-subtitle" value={slide.subtitle} onChange={set("subtitle")} placeholder="What this section covers" />
        </div>
      );
      break;
    case "closing":
      content = (
        <div className="s-title-wrap s-closing">
          <T as="h1" className="s-h1" value={slide.title} onChange={set("title")} placeholder="Thank you" />
          <T className="s-subtitle" value={slide.subtitle} onChange={set("subtitle")} placeholder="Next step or contact" />
        </div>
      );
      break;
    case "quote":
      content = (
        <div className="s-quote-wrap">
          <div className="s-quote-mark" aria-hidden>
            “
          </div>
          <T as="blockquote" className="s-quote" value={slide.quote} onChange={set("quote")} placeholder="Quote" multiline />
          <T className="s-attribution" value={slide.attribution} onChange={set("attribution")} placeholder="Who said it" />
        </div>
      );
      break;
    default:
      content = (
        <div className="s-content">
          <T as="h2" className="s-h2" value={slide.title} onChange={set("title")} placeholder="Slide title" />
          <Body slide={slide} onEdit={e} />
        </div>
      );
  }

  return (
    <div
      className={`slide layout-${slide.layout}${hero ? " is-hero" : ""}${theme.dark ? " is-dark" : ""}`}
      style={themeVars(theme) as CSSProperties}
      data-slide-index={index}
    >
      {content}
      {!hero && (
        <div className="s-footer">
          {brand.logo ? <img className="s-logo" src={brand.logo} alt="" /> : <span />}
          <span className="s-footer-text">{brand.footer}</span>
          <span className="s-page">
            {index + 1} / {total}
          </span>
        </div>
      )}
      {hero && brand.logo && <img className="s-logo s-logo-hero" src={brand.logo} alt="" />}
    </div>
  );
}

function Body({ slide, onEdit: e }: { slide: Slide; onEdit?: Edit }) {
  const body = (
    <T className="s-body" value={slide.body} onChange={e && ((v) => e({ body: v }))} placeholder="Optional one-line summary" multiline />
  );
  switch (slide.layout) {
    case "bullets":
      return (
        <>
          {(slide.body || e) && body}
          <Bullets items={slide.bullets} onChange={e && ((b) => e({ bullets: b }))} />
        </>
      );
    case "two-column":
      return (
        <div className="s-columns">
          {slide.columns.slice(0, 2).map((c, i) => (
            <div className="s-column" key={i}>
              <T
                className="s-column-h"
                value={c.heading}
                placeholder="Column heading"
                onChange={
                  e &&
                  ((v) => {
                    const cols = slide.columns.map((x) => ({ ...x }));
                    cols[i].heading = v;
                    e({ columns: cols });
                  })
                }
              />
              <Bullets
                items={c.bullets}
                onChange={
                  e &&
                  ((b) => {
                    const cols = slide.columns.map((x) => ({ ...x }));
                    cols[i].bullets = b;
                    e({ columns: cols });
                  })
                }
              />
            </div>
          ))}
        </div>
      );
    case "stats":
      return (
        <div className={`s-stats n-${Math.max(slide.stats.length, 1)}`}>
          {slide.stats.map((st, i) => (
            <div className="s-stat" key={i}>
              <T
                className="s-stat-value"
                value={st.value}
                placeholder="42%"
                onChange={
                  e &&
                  ((v) => {
                    const stats = slide.stats.map((x) => ({ ...x }));
                    stats[i].value = v;
                    e({ stats });
                  })
                }
              />
              <T
                className="s-stat-label"
                value={st.label}
                placeholder="What it measures"
                onChange={
                  e &&
                  ((v) => {
                    const stats = slide.stats.map((x) => ({ ...x }));
                    stats[i].label = v;
                    e({ stats });
                  })
                }
              />
            </div>
          ))}
        </div>
      );
    case "image":
      return (
        <div className="s-image-row">
          <div className="s-image-text">
            {body}
            <Bullets items={slide.bullets} onChange={e && slide.bullets.length ? (b) => e({ bullets: b }) : undefined} />
          </div>
          <ImageBox slide={slide} />
        </div>
      );
    case "chart":
      return (
        <>
          {(slide.body || e) && body}
          <div className="s-chart-wrap">
            <Chart data={slide.chart} />
          </div>
        </>
      );
    case "table":
      return (
        <div className="s-table-wrap">
          <table className="s-table">
            {slide.table.headers.length > 0 && (
              <thead>
                <tr>
                  {slide.table.headers.map((h, i) => (
                    <th key={i}>{h}</th>
                  ))}
                </tr>
              </thead>
            )}
            <tbody>
              {slide.table.rows.map((r, i) => (
                <tr key={i}>
                  {r.map((c, j) => (
                    <td key={j}>{c}</td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      );
    case "timeline":
      return (
        <div className={`s-timeline n-${slide.timeline.length}`}>
          {slide.timeline.map((t, i) => (
            <div className="s-tl-item" key={i}>
              <div className="s-tl-dot" />
              <T
                className="s-tl-label"
                value={t.label}
                placeholder="When"
                onChange={
                  e &&
                  ((v) => {
                    const tl = slide.timeline.map((x) => ({ ...x }));
                    tl[i].label = v;
                    e({ timeline: tl });
                  })
                }
              />
              <T
                className="s-tl-text"
                value={t.text}
                placeholder="What happens"
                onChange={
                  e &&
                  ((v) => {
                    const tl = slide.timeline.map((x) => ({ ...x }));
                    tl[i].text = v;
                    e({ timeline: tl });
                  })
                }
              />
            </div>
          ))}
        </div>
      );
    default:
      return null;
  }
}
