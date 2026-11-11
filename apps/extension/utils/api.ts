import {chunk, MAX_INGEST_BATCH, type IngestResponse, type ItemInput, type SyncReport} from '@stash/shared';
import type {Settings} from './settings';

export class ApiError extends Error {
  constructor(message: string, readonly status: number) {
    super(message);
  }
}

async function call<T>(settings: Pick<Settings, 'serverUrl' | 'token'>, path: string, init: RequestInit = {}): Promise<T> {
  let response: Response;
  try {
    response = await fetch(`${settings.serverUrl}${path}`, {
      ...init,
      headers: {authorization: `Bearer ${settings.token}`, 'content-type': 'application/json', ...(init.headers ?? {})},
    });
  } catch {
    throw new ApiError(`Could not reach ${settings.serverUrl}. Check the address and that the server is running.`, 0);
  }
  const body = (await response.json().catch(() => ({}))) as {error?: string};
  if (!response.ok) throw new ApiError(body.error ?? `The server answered ${response.status}.`, response.status);
  return body as T;
}

export function fetchMe(settings: Pick<Settings, 'serverUrl' | 'token'>) {
  return call<{email: string; total: number}>(settings, '/api/me');
}

/** Sends items in batches the server accepts, and adds up what was new. */
export async function ingest(settings: Settings, items: ItemInput[]): Promise<IngestResponse> {
  const total: IngestResponse = {added: 0, existing: 0};
  for (const batch of chunk(items, MAX_INGEST_BATCH)) {
    const r = await call<IngestResponse>(settings, '/api/ingest', {method: 'POST', body: JSON.stringify({items: batch})});
    total.added += r.added;
    total.existing += r.existing;
  }
  return total;
}

export async function reportSync(settings: Settings, report: SyncReport): Promise<void> {
  await call(settings, '/api/sync-runs', {method: 'POST', body: JSON.stringify(report)}).catch(() => {});
}
