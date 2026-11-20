import { useEffect, useRef, useState } from "react";
import { MODELS, type AISettings } from "./ai-config";
import { Editor } from "./components/Editor";
import { Home } from "./components/Home";
import { loadDecks, loadSettings, saveDecks, saveSettings } from "./store";
import type { Deck } from "./types";

function readRoute(): string | null {
  const m = /^#\/deck\/(.+)$/.exec(location.hash);
  return m ? decodeURIComponent(m[1]) : null;
}

export default function App() {
  const [decks, setDecks] = useState<Deck[]>(loadDecks);
  const [openId, setOpenId] = useState<string | null>(readRoute);
  const [settings, setSettings] = useState<AISettings>(loadSettings);
  const [showSettings, setShowSettings] = useState(false);
  const [saveFailed, setSaveFailed] = useState(false);
  const saveTimer = useRef<number>();

  // debounce writes so typing doesn't serialize every deck on each keystroke
  useEffect(() => {
    window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => setSaveFailed(!saveDecks(decks)), 300);
  }, [decks]);

  useEffect(() => {
    const onHash = () => setOpenId(readRoute());
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  const open = (id: string | null) => {
    location.hash = id ? `#/deck/${encodeURIComponent(id)}` : "";
    setOpenId(id);
    window.scrollTo(0, 0);
  };

  const deck = decks.find((d) => d.id === openId);

  return (
    <>
      {deck ? (
        <Editor
          key={deck.id}
          deck={deck}
          settings={settings}
          onChange={(next) => setDecks((all) => all.map((d) => (d.id === next.id ? next : d)))}
          onBack={() => open(null)}
          onSettings={() => setShowSettings(true)}
        />
      ) : (
        <Home
          decks={decks}
          settings={settings}
          onOpen={open}
          onCreate={(d) => {
            setDecks((all) => [d, ...all]);
            open(d.id);
          }}
          onDelete={(id) => setDecks((all) => all.filter((d) => d.id !== id))}
          onSettings={() => setShowSettings(true)}
        />
      )}
      {saveFailed && (
        <div className="banner" role="alert">
          This browser refused to save (storage full or blocked). Export your deck so you don't lose work.
        </div>
      )}
      {showSettings && (
        <SettingsDialog
          settings={settings}
          onClose={() => setShowSettings(false)}
          onSave={(s) => {
            setSettings(s);
            saveSettings(s);
            setShowSettings(false);
          }}
        />
      )}
    </>
  );
}

function SettingsDialog({ settings, onSave, onClose }: { settings: AISettings; onSave: (s: AISettings) => void; onClose: () => void }) {
  const [key, setKey] = useState(settings.apiKey);
  const [model, setModel] = useState(settings.model);
  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal" role="dialog" aria-modal="true" aria-labelledby="settings-h" onClick={(e) => e.stopPropagation()}>
        <h2 id="settings-h">Settings</h2>
        <label className="label" htmlFor="api-key">
          Anthropic API key
        </label>
        <input id="api-key" type="password" autoComplete="off" value={key} onChange={(e) => setKey(e.target.value.trim())} placeholder="sk-ant-…" />
        <p className="hint">
          Stored only in this browser and sent only to api.anthropic.com. Get a key at{" "}
          <a href="https://console.anthropic.com/settings/keys" target="_blank" rel="noreferrer">
            console.anthropic.com
          </a>
          . Everything except AI generation works without one.
        </p>
        <label className="label" htmlFor="model">
          Model
        </label>
        <select id="model" value={model} onChange={(e) => setModel(e.target.value)}>
          {MODELS.map((m) => (
            <option key={m.id} value={m.id}>
              {m.label}
            </option>
          ))}
        </select>
        <div className="modal-actions">
          {settings.apiKey && (
            <button className="btn ghost" onClick={() => onSave({ apiKey: "", model })}>
              Remove key
            </button>
          )}
          <button className="btn ghost" onClick={onClose}>
            Cancel
          </button>
          <button className="btn primary" onClick={() => onSave({ apiKey: key, model })}>
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
