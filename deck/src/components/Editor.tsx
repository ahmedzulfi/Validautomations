import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { AISettings } from "../ai-config";
import { download, fileName } from "../export/download";
import { deckToMarkdown } from "../outline";
import { changeLayout, starterSlide } from "../slides";
import { resolveTheme } from "../themes";
import { LAYOUTS, LAYOUT_LABELS, type Deck, type Layout, type Slide } from "../types";
import { uid } from "../lib/ids";
import { Inspector } from "./Inspector";
import { Presenter } from "./Presenter";
import { ScaledSlide } from "./ScaledSlide";
import { SlideView } from "./SlideView";

interface Props {
  deck: Deck;
  settings: AISettings;
  onChange: (deck: Deck) => void;
  onBack: () => void;
  onSettings: () => void;
}

const HISTORY_LIMIT = 100;

export function Editor({ deck, settings, onChange, onBack, onSettings }: Props) {
  const [current, setCurrent] = useState(0);
  const [presenting, setPresenting] = useState<number | null>(null);
  const [aiBusy, setAiBusy] = useState(false);
  const [deckAi, setDeckAi] = useState("");
  const [toast, setToast] = useState("");
  const [menu, setMenu] = useState<"" | "export" | "add">("");
  const [printing, setPrinting] = useState(false);
  const past = useRef<Deck[]>([]);
  const future = useRef<Deck[]>([]);
  const lastPush = useRef(0);
  const dragFrom = useRef<number | null>(null);

  const theme = resolveTheme(deck);
  const idx = Math.min(current, deck.slides.length - 1);
  const slide = deck.slides[idx];

  const flash = (msg: string) => {
    setToast(msg);
    window.setTimeout(() => setToast(""), 3500);
  };

  /** Commit a new deck state; rapid edits (typing) coalesce into one undo step. */
  const commit = useCallback(
    (next: Deck, coalesce = false) => {
      const now = Date.now();
      if (!coalesce || now - lastPush.current > 800) {
        past.current = [...past.current.slice(-HISTORY_LIMIT), deck];
        future.current = [];
      }
      lastPush.current = now;
      onChange({ ...next, updatedAt: now });
    },
    [deck, onChange],
  );

  const undo = useCallback(() => {
    const prev = past.current.pop();
    if (!prev) return;
    future.current.push(deck);
    onChange(prev);
  }, [deck, onChange]);

  const redo = useCallback(() => {
    const next = future.current.pop();
    if (!next) return;
    past.current.push(deck);
    onChange(next);
  }, [deck, onChange]);

  const setSlides = (slides: Slide[], coalesce = false) => commit({ ...deck, slides }, coalesce);

  const updateSlide = (patch: Partial<Slide>) => {
    let next: Slide = { ...slide, ...patch };
    if (patch.layout && patch.layout !== slide.layout) next = changeLayout({ ...slide, ...patch, layout: slide.layout }, patch.layout);
    setSlides(
      deck.slides.map((s, i) => (i === idx ? next : s)),
      !patch.layout,
    );
  };

  const addSlide = (layout: Layout) => {
    const s = starterSlide(layout);
    const slides = [...deck.slides];
    slides.splice(idx + 1, 0, s);
    setSlides(slides);
    setCurrent(idx + 1);
    setMenu("");
  };

  const duplicate = (i: number) => {
    const slides = [...deck.slides];
    slides.splice(i + 1, 0, { ...structuredClone(deck.slides[i]), id: uid("s_") });
    setSlides(slides);
    setCurrent(i + 1);
  };

  const remove = (i: number) => {
    if (deck.slides.length <= 1) return;
    setSlides(deck.slides.filter((_, j) => j !== i));
    setCurrent(Math.max(0, Math.min(i, deck.slides.length - 2)));
  };

  const move = (from: number, to: number) => {
    if (from === to || to < 0 || to >= deck.slides.length) return;
    const slides = [...deck.slides];
    const [s] = slides.splice(from, 1);
    slides.splice(to, 0, s);
    setSlides(slides);
    setCurrent(to);
  };

  // keyboard shortcuts
  useEffect(() => {
    if (presenting !== null) return;
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName)) return;
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === "z") {
        e.preventDefault();
        if (e.shiftKey) redo();
        else undo();
      } else if (mod && e.key.toLowerCase() === "y") {
        e.preventDefault();
        redo();
      } else if (mod && e.key.toLowerCase() === "d") {
        e.preventDefault();
        duplicate(idx);
      } else if (e.key === "ArrowDown" || e.key === "ArrowRight") {
        e.preventDefault();
        setCurrent(Math.min(deck.slides.length - 1, idx + 1));
      } else if (e.key === "ArrowUp" || e.key === "ArrowLeft") {
        e.preventDefault();
        setCurrent(Math.max(0, idx - 1));
      } else if (e.key === "Delete" || e.key === "Backspace") {
        e.preventDefault();
        remove(idx);
      } else if (e.key === "F5" || (mod && e.key === "Enter")) {
        e.preventDefault();
        setPresenting(idx);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  // print -> PDF: render all slides into a print-only root, then open the print dialog
  useEffect(() => {
    if (!printing) return;
    const done = () => setPrinting(false);
    window.addEventListener("afterprint", done, { once: true });
    const t = window.setTimeout(() => window.print(), 400); // let fonts/images settle
    return () => {
      window.clearTimeout(t);
      window.removeEventListener("afterprint", done);
    };
  }, [printing]);

  const aiSlide = async (instruction: string) => {
    setAiBusy(true);
    try {
      const { editSlide } = await import("../ai");
      const next = await editSlide(slide, instruction, deck, settings);
      setSlides(deck.slides.map((s, i) => (i === idx ? next : s)));
      flash("Slide updated. Undo with Ctrl+Z.");
    } catch (e) {
      flash(e instanceof Error ? e.message : String(e));
    } finally {
      setAiBusy(false);
    }
  };

  const aiDeck = async () => {
    if (!deckAi.trim()) return;
    if (!settings.apiKey) return onSettings();
    setAiBusy(true);
    try {
      const { editDeck } = await import("../ai");
      const out = await editDeck(deck, deckAi, settings);
      // keep user images on slides that stayed image slides at the same position
      const slides = out.slides.map((s, i) => {
        const old = deck.slides[i];
        if (old && old.image.url && s.layout === "image" && !s.image.url) return { ...s, image: { ...s.image, url: old.image.url } };
        return s;
      });
      commit({ ...deck, title: out.title || deck.title, slides });
      setCurrent(0);
      setDeckAi("");
      flash("Deck revised. Undo with Ctrl+Z.");
    } catch (e) {
      flash(e instanceof Error ? e.message : String(e));
    } finally {
      setAiBusy(false);
    }
  };

  const doExport = async (kind: "pptx" | "pdf" | "html" | "md" | "json") => {
    setMenu("");
    const name = fileName(deck.title);
    try {
      if (kind === "pptx") {
        flash("Building PowerPoint…");
        const { exportPptx } = await import("../export/pptx");
        await exportPptx(deck);
      } else if (kind === "pdf") setPrinting(true);
      else if (kind === "html") {
        const { deckToHtml } = await import("../export/html");
        download(`${name}.html`, deckToHtml(deck), "text/html");
      }
      else if (kind === "md") download(`${name}.md`, deckToMarkdown(deck), "text/markdown");
      else download(`${name}.json`, JSON.stringify(deck, null, 2), "application/json");
    } catch (e) {
      flash(`Export failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  };

  if (presenting !== null) return <Presenter deck={deck} start={presenting} onExit={() => setPresenting(null)} />;

  return (
    <div className="editor">
      <header className="topbar">
        <button className="btn ghost" onClick={onBack} aria-label="Back to all decks">
          ← Decks
        </button>
        <input
          className="deck-title"
          value={deck.title}
          aria-label="Deck title"
          onChange={(e) => commit({ ...deck, title: e.target.value }, true)}
        />
        <div className="topbar-actions">
          <button className="btn ghost" onClick={undo} disabled={!past.current.length} title="Undo (Ctrl+Z)">
            Undo
          </button>
          <button className="btn ghost" onClick={redo} disabled={!future.current.length} title="Redo (Ctrl+Shift+Z)">
            Redo
          </button>
          <div className="menu-wrap">
            <button className="btn ghost" onClick={() => setMenu(menu === "export" ? "" : "export")} aria-expanded={menu === "export"}>
              Export ▾
            </button>
            {menu === "export" && (
              <div className="menu" role="menu">
                <button role="menuitem" onClick={() => doExport("pptx")}>
                  PowerPoint (.pptx) <small>editable</small>
                </button>
                <button role="menuitem" onClick={() => doExport("pdf")}>
                  PDF <small>via print dialog</small>
                </button>
                <button role="menuitem" onClick={() => doExport("html")}>
                  Web page (.html) <small>publish anywhere</small>
                </button>
                <button role="menuitem" onClick={() => doExport("md")}>
                  Markdown outline
                </button>
                <button role="menuitem" onClick={() => doExport("json")}>
                  Slidesmith file (.json)
                </button>
              </div>
            )}
          </div>
          <button className="btn primary" onClick={() => setPresenting(idx)}>
            ▶ Present
          </button>
        </div>
      </header>

      <div className="workspace">
        <nav className="thumbs" aria-label="Slides">
          <ol>
            {deck.slides.map((s, i) => (
              <li
                key={s.id}
                className={"thumb" + (i === idx ? " on" : "")}
                draggable
                onDragStart={() => (dragFrom.current = i)}
                onDragOver={(e) => e.preventDefault()}
                onDrop={() => {
                  if (dragFrom.current !== null) move(dragFrom.current, i);
                  dragFrom.current = null;
                }}
              >
                <span className="thumb-n">{i + 1}</span>
                <button className="thumb-btn" onClick={() => setCurrent(i)} aria-label={`Slide ${i + 1}: ${s.title || LAYOUT_LABELS[s.layout]}`} aria-current={i === idx}>
                  <ScaledSlide width={168}>
                    <SlideView slide={s} theme={theme} brand={deck.brand} index={i} total={deck.slides.length} />
                  </ScaledSlide>
                </button>
                <div className="thumb-tools">
                  <button title="Move up" aria-label="Move up" onClick={() => move(i, i - 1)} disabled={i === 0}>
                    ↑
                  </button>
                  <button title="Move down" aria-label="Move down" onClick={() => move(i, i + 1)} disabled={i === deck.slides.length - 1}>
                    ↓
                  </button>
                  <button title="Duplicate (Ctrl+D)" aria-label="Duplicate" onClick={() => duplicate(i)}>
                    ⧉
                  </button>
                  <button title="Delete" aria-label="Delete slide" onClick={() => remove(i)} disabled={deck.slides.length <= 1}>
                    ×
                  </button>
                </div>
              </li>
            ))}
          </ol>
          <div className="menu-wrap add-wrap">
            <button className="btn ghost wide" onClick={() => setMenu(menu === "add" ? "" : "add")} aria-expanded={menu === "add"}>
              + Add slide
            </button>
            {menu === "add" && (
              <div className="menu up" role="menu">
                {LAYOUTS.map((l) => (
                  <button role="menuitem" key={l} onClick={() => addSlide(l)}>
                    {LAYOUT_LABELS[l]}
                  </button>
                ))}
              </div>
            )}
          </div>
        </nav>

        <main className="stage" onClick={() => menu && setMenu("")}>
          <div className="deck-ai">
            <input
              value={deckAi}
              disabled={aiBusy}
              placeholder={settings.apiKey ? "Ask AI to revise the whole deck, e.g. “Cut to 6 slides for a 5-minute talk”" : "Add an API key in Settings to revise the deck with AI"}
              onChange={(e) => setDeckAi(e.target.value)}
              onKeyDown={(e) => {
                e.stopPropagation();
                if (e.key === "Enter") aiDeck();
              }}
            />
            <button className="btn primary sm" disabled={aiBusy || !deckAi.trim()} onClick={aiDeck}>
              {aiBusy ? "Working…" : "Revise deck"}
            </button>
          </div>
          <div className="canvas">
            <ScaledSlide className="main-slide">
              <SlideView slide={slide} theme={theme} brand={deck.brand} index={idx} total={deck.slides.length} onEdit={updateSlide} />
            </ScaledSlide>
            {aiBusy && <div className="canvas-busy">Claude is rewriting…</div>}
          </div>
          <p className="stage-hint">Click any text on the slide to edit it · Ctrl+Z undo · Ctrl+Enter present</p>
        </main>

        <Inspector deck={deck} slide={slide} onSlide={updateSlide} onDeck={(p) => commit({ ...deck, ...p })} onAiSlide={aiSlide} aiBusy={aiBusy} hasKey={!!settings.apiKey} />
      </div>

      {toast && (
        <div className="toast" role="status">
          {toast}
        </div>
      )}

      {printing &&
        createPortal(
          <div className="print-root">
            {deck.slides.map((s, i) => (
              <div className="print-page" key={s.id}>
                <SlideView slide={s} theme={theme} brand={deck.brand} index={i} total={deck.slides.length} />
              </div>
            ))}
          </div>,
          document.body,
        )}
    </div>
  );
}
