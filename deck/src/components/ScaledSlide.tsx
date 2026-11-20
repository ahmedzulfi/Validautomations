import { useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { SLIDE_H, SLIDE_W } from "./SlideView";

/** Renders a fixed 1280x720 slide scaled to fit its container's width (or a fixed width). */
export function ScaledSlide({ children, width, className }: { children: ReactNode; width?: number; className?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  const [measured, setMeasured] = useState(width ?? 0);

  useLayoutEffect(() => {
    if (width) return;
    const el = ref.current;
    if (!el) return;
    const update = () => setMeasured(el.clientWidth);
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    return () => ro.disconnect();
  }, [width]);

  const w = width ?? measured;
  const scale = w / SLIDE_W;
  return (
    <div
      ref={ref}
      className={"scaled-slide " + (className ?? "")}
      style={{ width: width ?? "100%", height: width ? (width * SLIDE_H) / SLIDE_W : undefined, aspectRatio: width ? undefined : "16 / 9" }}
    >
      {w > 0 && <div style={{ transform: `scale(${scale})`, transformOrigin: "top left", width: SLIDE_W, height: SLIDE_H }}>{children}</div>}
    </div>
  );
}
