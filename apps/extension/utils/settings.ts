import type {SyncSource} from './messages';

export type Settings = {
  serverUrl: string;
  token: string;
  email: string;
  autoSync: boolean;
};

export type SourceStatus = {
  state: 'idle' | 'running' | 'done' | 'failed';
  found: number;
  added: number;
  message: string | null;
  updatedAt: number;
  lastSuccessAt: number | null;
};

export const SOURCE_NAMES: Record<SyncSource, string> = {
  x: 'X bookmarks',
  linkedin: 'LinkedIn saved posts',
  reddit: 'Reddit saved',
  youtube: 'YouTube watch later and likes',
  instagram: 'Instagram saved',
  bookmark: 'Browser bookmarks',
};

export const SYNC_SOURCES: SyncSource[] = ['x', 'linkedin', 'reddit', 'youtube', 'instagram', 'bookmark'];

const SETTINGS_KEY = 'settings';
const statusKey = (s: SyncSource) => `status:${s}`;

export async function getSettings(): Promise<Settings | null> {
  const got = await browser.storage.local.get(SETTINGS_KEY);
  const s = got[SETTINGS_KEY] as Settings | undefined;
  return s?.serverUrl && s.token ? s : null;
}

export async function saveSettings(settings: Settings | null): Promise<void> {
  if (settings) await browser.storage.local.set({[SETTINGS_KEY]: settings});
  else await browser.storage.local.remove(SETTINGS_KEY);
}

export const emptyStatus = (): SourceStatus => ({state: 'idle', found: 0, added: 0, message: null, updatedAt: 0, lastSuccessAt: null});

export async function getStatus(source: SyncSource): Promise<SourceStatus> {
  const got = await browser.storage.local.get(statusKey(source));
  return (got[statusKey(source)] as SourceStatus | undefined) ?? emptyStatus();
}

export async function getAllStatus(): Promise<Record<SyncSource, SourceStatus>> {
  const entries = await Promise.all(SYNC_SOURCES.map(async (s) => [s, await getStatus(s)] as const));
  return Object.fromEntries(entries) as Record<SyncSource, SourceStatus>;
}

export async function setStatus(source: SyncSource, patch: Partial<SourceStatus>): Promise<SourceStatus> {
  const next = {...(await getStatus(source)), ...patch, updatedAt: Date.now()};
  await browser.storage.local.set({[statusKey(source)]: next});
  return next;
}

/** Trims a typed server address to its origin, adding https:// when the scheme is missing. */
export function normalizeServerUrl(input: string): string | null {
  const trimmed = input.trim();
  if (!trimmed) return null;
  const withScheme = /^https?:\/\//i.test(trimmed) ? trimmed : `${/^(localhost|127\.)/.test(trimmed) ? 'http' : 'https'}://${trimmed}`;
  try {
    return new URL(withScheme).origin;
  } catch {
    return null;
  }
}
