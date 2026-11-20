import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { resolveTheme } from "../themes";
import type { Deck } from "../types";
import { SLIDE_H, SLIDE_W, SlideView } from "./SlideView";

/** Full-screen presenting: arrows/space to move, N for notes, Esc to leave. */
export function Presenter({ deck, start, onExit }: { deck: Deck; start: number; onExit: () => void }) {
  const [i, setI] = useState(start);
  const [dir, setDir] = useState(1);
  const [notes, setNotes] = useState(false);
  const [scale, setScale] = useState(1);
  const root = useRef<HTMLDivElement>(null);
  const theme = resolveTheme(deck);
  const n = deck.slides.length;

  const go = useCallback(
    (to: number) => {
      const next = Math.max(0, Math.min(n - 1, to));
      setDir(next >= i ? 1 : -1);
      setI(next);
    },
    [i, n],
  );

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (["ArrowRight", "ArrowDown", "PageDown", " ", "Enter"].includes(e.key)) {
        e.preventDefault();
        go(i + 1);
      } else if (["ArrowLeft", "ArrowUp", "PageUp", "Backspace"].includes(e.key)) {
        e.preventDefault();
        go(i - 1);
      } else if (e.key === "Home") go(0);
      else if (e.key === "End") go(n - 1);
      else if (e.key.toLowerCase() === "n") setNotes((v) => !v);
      else if (e.key === "Escape") onExit();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [go, i, n, onExit]);

  useEffect(() => {
    root.current?.requestFullscreen?.().catch(() => {});
    const onFs = () => {
      if (!document.fullscreenElement) onExit();
    };
    document.addEventListener("fullscreenchange", onFs);
    return () => {
      document.removeEventListener("fullscreenchange", onFs);
      if (document.fullscreenElement) document.exitFullscreen().catch(() => {});
    };
  }, [onExit]);

  useLayoutEffect(() => {
    const fit = () => {
      const h = window.innerHeight - (notes ? 180 : 0);
      setScale(Math.min(window.innerWidth / SLIDE_W, h / SLIDE_H));
    };
    fit();
    window.addEventListener("resize", fit);
    return () => window.removeEventListener("resize", fit);
  }, [notes]);

  const slide = deck.slides[i];
  return (
    <div className="presenter" ref={root} onClick={(e) => (e.clientX > window.innerWidth / 3 ? go(i + 1) : go(i - 1))}>
      <div className="presenter-stage">
        <div
          key={slide.id + i}
          className={`presenter-slide tr-${deck.transition} dir-${dir > 0 ? "fwd" : "back"}`}
          style={{ width: SLIDE_W * scale, height: SLIDE_H * scale }}
        >
          <div style={{ transform: `scale(${scale})`, transformOrigin: "top left", width: SLIDE_W, height: SLIDE_H }}>
            <SlideView slide={slide} theme={theme} brand={deck.brand} index={i} total={n} />
          </div>
        </div>
      </div>
      {notes && (
        <div className="presenter-notes" onClick={(e) => e.stopPropagation()}>
          <strong>
            Slide {i + 1} of {n}
          </strong>
          <p>{slide.notes || "No speaker notes for this slide."}</p>
        </div>
      )}
      <div className="presenter-progress" style={{ width: `${((i + 1) / n) * 100}%` }} />
      <div className="presenter-hint">← → move · N notes · Esc exit</div>
    </div>
  );
}
