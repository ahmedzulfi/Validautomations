import { useRef, useState } from "react";
import type { AISettings } from "../ai-config";
import type { Attachment } from "../ai";
import { readDataUrl, readFile } from "../export/download";
import { outlineToSlides } from "../outline";
import { newDeck, normalizeDeck } from "../slides";
import { TEMPLATES } from "../templates";
import { THEMES, resolveTheme } from "../themes";
import type { Deck } from "../types";
import { ScaledSlide } from "./ScaledSlide";
import { SlideView } from "./SlideView";

interface Props {
  decks: Deck[];
  settings: AISettings;
  onOpen: (id: string) => void;
  onCreate: (deck: Deck) => void;
  onDelete: (id: string) => void;
  onSettings: () => void;
}

const TONES = ["Confident", "Friendly", "Formal", "Bold", "Academic"];

export function Home({ decks, settings, onOpen, onCreate, onDelete, onSettings }: Props) {
  const [prompt, setPrompt] = useState("");
  const [files, setFiles] = useState<Attachment[]>([]);
  const [count, setCount] = useState(8);
  const [tone, setTone] = useState("Confident");
  const [audience, setAudience] = useState("");
  const [themeId, setThemeId] = useState("paper");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const fileInput = useRef<HTMLInputElement>(null);
  const importInput = useRef<HTMLInputElement>(null);

  const addFiles = async (list: FileList | null) => {
    if (!list) return;
    const next: Attachment[] = [];
    for (const f of Array.from(list)) {
      if (f.size > 20 * 1024 * 1024) {
        setError(`${f.name} is over 20 MB.`);
        continue;
      }
      if (f.type === "application/pdf" || f.name.toLowerCase().endsWith(".pdf")) {
        const url = await readDataUrl(f);
        next.push({ name: f.name, kind: "pdf", data: url.slice(url.indexOf(",") + 1) });
      } else {
        next.push({ name: f.name, kind: "text", data: await readFile(f) });
      }
    }
    setFiles((cur) => [...cur, ...next]);
  };

  const generate = async () => {
    setError("");
    if (!settings.apiKey) {
      onSettings();
      return;
    }
    if (!prompt.trim() && !files.length) {
      setError("Describe the deck or attach some material first.");
      return;
    }
    setBusy(true);
    try {
      const { generateDeck } = await import("../ai");
      const { title, slides } = await generateDeck(
        { prompt, source: "", slideCount: count, tone, audience, attachments: files },
        settings,
      );
      onCreate(newDeck(title, slides, themeId));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const fromOutline = () => {
    setError("");
    const text = [prompt, ...files.filter((f) => f.kind === "text").map((f) => f.data)].join("\n\n---\n\n").trim();
    if (!text) {
      setError("Paste an outline (headings and bullets) or attach a .md file.");
      return;
    }
    const { title, slides } = outlineToSlides(text);
    onCreate(newDeck(title, slides, themeId));
  };

  const importDeck = async (list: FileList | null) => {
    const f = list?.[0];
    if (!f) return;
    try {
      const text = await readFile(f);
      if (f.name.toLowerCase().endsWith(".json")) {
        const d = normalizeDeck(JSON.parse(text));
        d.id = newDeck("", []).id; // never clobber an existing deck
        onCreate(d);
      } else {
        const { title, slides } = outlineToSlides(text);
        onCreate(newDeck(title, slides, themeId));
      }
    } catch {
      setError("That file couldn't be imported. Use a Slidesmith .json export or a Markdown outline.");
    }
  };

  return (
    <div className="home">
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" aria-hidden />
          Slidesmith
        </div>
        <div className="topbar-actions">
          <button className="btn ghost" onClick={() => importInput.current?.click()}>
            Import
          </button>
          <input ref={importInput} type="file" accept=".json,.md,.markdown,.txt" hidden onChange={(e) => importDeck(e.target.files)} />
          <button className="btn ghost" onClick={onSettings}>
            Settings{settings.apiKey ? "" : " · add API key"}
          </button>
        </div>
      </header>

      <section className="hero">
        <h1>Turn notes into a deck you'd actually present.</h1>
        <p className="hero-sub">Describe it, paste rough notes or attach docs. Claude drafts the story and slides; you edit everything.</p>

        <div className={"composer" + (busy ? " is-busy" : "")}>
          <textarea
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            placeholder={"e.g. Turn these discovery notes into a sales proposal for Walmart…\n\nOr paste a Markdown outline and choose “Build from outline”."}
            rows={5}
            disabled={busy}
            onKeyDown={(e) => {
              if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) generate();
            }}
          />
          {files.length > 0 && (
            <div className="chips">
              {files.map((f, i) => (
                <span className="chip" key={i}>
                  <span className="chip-kind">{f.kind === "pdf" ? "PDF" : "TXT"}</span>
                  {f.name}
                  <button aria-label={`Remove ${f.name}`} onClick={() => setFiles(files.filter((_, j) => j !== i))}>
                    ×
                  </button>
                </span>
              ))}
            </div>
          )}
          <div className="composer-bar">
            <button className="btn ghost sm" onClick={() => fileInput.current?.click()} disabled={busy}>
              + Attach
            </button>
            <input ref={fileInput} type="file" multiple accept=".pdf,.md,.markdown,.txt,.csv,.json" hidden onChange={(e) => addFiles(e.target.files)} />
            <label className="field-inline">
              Slides
              <input type="number" min={3} max={30} value={count} onChange={(e) => setCount(Math.max(3, Math.min(30, +e.target.value || 8)))} />
            </label>
            <label className="field-inline">
              Tone
              <select value={tone} onChange={(e) => setTone(e.target.value)}>
                {TONES.map((t) => (
                  <option key={t}>{t}</option>
                ))}
              </select>
            </label>
            <label className="field-inline grow">
              Audience
              <input value={audience} onChange={(e) => setAudience(e.target.value)} placeholder="Investors, my team, students…" />
            </label>
          </div>
          <div className="theme-picker" role="radiogroup" aria-label="Theme">
            {THEMES.map((t) => (
              <button
                key={t.id}
                role="radio"
                aria-checked={themeId === t.id}
                className={"theme-swatch" + (themeId === t.id ? " on" : "")}
                onClick={() => setThemeId(t.id)}
                title={t.name}
              >
                <span style={{ background: t.bg }}>
                  <i style={{ background: t.primary }} />
                  <i style={{ background: t.accent }} />
                </span>
                {t.name}
              </button>
            ))}
          </div>
          <div className="composer-actions">
            <button className="btn ghost" onClick={fromOutline} disabled={busy}>
              Build from outline (no AI)
            </button>
            <button className="btn primary" onClick={generate} disabled={busy}>
              {busy ? "Writing your deck…" : settings.apiKey ? "Generate with AI" : "Add API key to generate"}
            </button>
          </div>
          {busy && <div className="progress-line" aria-hidden />}
          {error && (
            <p className="error" role="alert">
              {error}
            </p>
          )}
        </div>
      </section>

      <section className="shelf">
        <h2>Start from a template</h2>
        <div className="grid">
          {TEMPLATES.map((t) => {
            const { title, slides } = outlineToSlides(t.outline);
            const preview = newDeck(title, slides, t.themeId);
            return (
              <button key={t.id} className="card" onClick={() => onCreate(newDeck(title, outlineToSlides(t.outline).slides, t.themeId))}>
                <ScaledSlide>
                  <SlideView slide={preview.slides[0]} theme={resolveTheme(preview)} brand={preview.brand} index={0} total={slides.length} />
                </ScaledSlide>
                <span className="card-title">{t.name}</span>
                <span className="card-sub">{t.description}</span>
              </button>
            );
          })}
        </div>
      </section>

      <section className="shelf">
        <h2>Your decks</h2>
        {decks.length === 0 ? (
          <p className="empty">Nothing yet. Generate one above or start from a template. Decks are saved in this browser.</p>
        ) : (
          <div className="grid">
            {decks.map((d) => (
              <div key={d.id} className="card">
                <button className="card-open" onClick={() => onOpen(d.id)} aria-label={`Open ${d.title}`}>
                  <ScaledSlide>
                    <SlideView slide={d.slides[0]} theme={resolveTheme(d)} brand={d.brand} index={0} total={d.slides.length} />
                  </ScaledSlide>
                  <span className="card-title">{d.title}</span>
                  <span className="card-sub">
                    {d.slides.length} slides · {new Date(d.updatedAt).toLocaleDateString()}
                  </span>
                </button>
                <button
                  className="card-delete"
                  aria-label={`Delete ${d.title}`}
                  onClick={() => {
                    if (confirm(`Delete “${d.title}”? This can't be undone.`)) onDelete(d.id);
                  }}
                >
                  Delete
                </button>
              </div>
            ))}
          </div>
        )}
      </section>
      <footer className="foot">Open source · Your decks and API key stay in this browser · AI by Claude</footer>
    </div>
  );
}
