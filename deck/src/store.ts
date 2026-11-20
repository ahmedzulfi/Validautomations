import { DEFAULT_MODEL, type AISettings } from "./ai-config";
import { normalizeDeck } from "./slides";
import type { Deck } from "./types";

const DECKS_KEY = "slidesmith.decks.v1";
const SETTINGS_KEY = "slidesmith.settings.v1";

function read<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}

function write(key: string, value: unknown): boolean {
  try {
    localStorage.setItem(key, JSON.stringify(value));
    return true;
  } catch {
    return false; // storage full or blocked; the app keeps working in memory
  }
}

export function loadDecks(): Deck[] {
  const raw = read<unknown[]>(DECKS_KEY, []);
  return (Array.isArray(raw) ? raw : []).map(normalizeDeck).sort((a, b) => b.updatedAt - a.updatedAt);
}

export function saveDecks(decks: Deck[]): boolean {
  return write(DECKS_KEY, decks);
}

export function loadSettings(): AISettings {
  const s = read<Partial<AISettings>>(SETTINGS_KEY, {});
  return { apiKey: s.apiKey ?? "", model: s.model ?? DEFAULT_MODEL };
}

export function saveSettings(s: AISettings): void {
  write(SETTINGS_KEY, s);
}
