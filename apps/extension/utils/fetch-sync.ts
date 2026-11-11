import {instagramNextMaxId, parseInstagramSaved, parseRedditSaved, redditNextAfter, type IngestResponse, type ItemInput} from '@stash/shared';
import type {CollectResult} from './messages';

type Page = {items: ItemInput[]; next: string | null};

type PagerOptions = {
  fetchPage: (cursor: string | null) => Promise<Page>;
  send: (items: ItemInput[]) => Promise<IngestResponse>;
  incremental: boolean;
  delayMs?: number;
  maxPages?: number;
  sleep?: (ms: number) => Promise<void>;
};

/**
 * Walks a site's own saved-items JSON a page at a time, with the person's logged-in session, at a
 * person's pace. In an incremental sync it stops at the first page that is all already in the library.
 */
export async function pageThrough(o: PagerOptions): Promise<CollectResult> {
  const sleep = o.sleep ?? ((ms: number) => new Promise((r) => setTimeout(r, ms)));
  let cursor: string | null = null;
  let found = 0;
  let added = 0;
  try {
    for (let page = 0; page < (o.maxPages ?? 100); page++) {
      const {items, next} = await o.fetchPage(cursor);
      if (items.length) {
        const r = await o.send(items);
        found += items.length;
        added += r.added;
        if (o.incremental && r.added === 0) break;
      }
      if (!next || next === cursor) break;
      cursor = next;
      await sleep((o.delayMs ?? 1200) + Math.random() * 800);
    }
    return {ok: true, found, added};
  } catch (err) {
    return {ok: false, error: err instanceof Error ? err.message : 'Sync stopped.', found, added};
  }
}

export class LoggedOutError extends Error {}

async function getJson(url: string, init: RequestInit = {}): Promise<{status: number; body: unknown}> {
  const response = await fetch(url, {credentials: 'include', ...init});
  const body = await response.json().catch(() => null);
  return {status: response.status, body};
}

/** Reddit: who is logged in, then their saved listing, 100 at a time. */
export function redditPager(): (cursor: string | null) => Promise<Page> {
  let username: string | null = null;
  return async (cursor) => {
    if (!username) {
      const me = await getJson('https://www.reddit.com/api/me.json');
      const name = (me.body as {data?: {name?: string}} | null)?.data?.name;
      if (!name) throw new LoggedOutError('Log in to Reddit in this browser, then sync again.');
      username = name;
    }
    const url = `https://www.reddit.com/user/${encodeURIComponent(username)}/saved.json?limit=100&raw_json=1${cursor ? `&after=${encodeURIComponent(cursor)}` : ''}`;
    const {status, body} = await getJson(url);
    if (status === 403 || status === 401) throw new LoggedOutError('Log in to Reddit in this browser, then sync again.');
    if (status !== 200) throw new Error(`Reddit answered ${status}. Try again later.`);
    return {items: parseRedditSaved(body), next: redditNextAfter(body)};
  };
}

// The id Instagram's own web app sends with its API requests.
const INSTAGRAM_WEB_APP_ID = '936619743392459';

/** Instagram: the saved-posts feed the web app reads, page by page. */
export function instagramPager(): (cursor: string | null) => Promise<Page> {
  return async (cursor) => {
    const url = `https://www.instagram.com/api/v1/feed/saved/posts/${cursor ? `?max_id=${encodeURIComponent(cursor)}` : ''}`;
    const {status, body} = await getJson(url, {headers: {'x-ig-app-id': INSTAGRAM_WEB_APP_ID, 'x-requested-with': 'XMLHttpRequest'}});
    const b = body as {require_login?: boolean; message?: string} | null;
    if (status === 401 || status === 403 || b?.require_login || b?.message === 'login_required') {
      throw new LoggedOutError('Log in to Instagram in this browser, then sync again.');
    }
    if (status === 429) throw new Error('Instagram is limiting requests. Try again in an hour.');
    if (status !== 200) throw new Error(`Instagram answered ${status}. Try again later.`);
    return {items: parseInstagramSaved(body), next: instagramNextMaxId(body)};
  };
}
