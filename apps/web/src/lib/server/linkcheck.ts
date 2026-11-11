import 'server-only';
import {lookup} from 'node:dns/promises';
import {isIP} from 'node:net';
import {query, queryOne} from './db';

/** Only links to the open web are checked; posts on X, Reddit and the like sit behind logins and limits. */
const CHECKED_SOURCES = ['bookmark', 'web', 'github'];
const RECHECK_DAYS = 30;
const TIMEOUT_MS = 10000;
const MAX_REDIRECTS = 5;

export type LinkState = 'ok' | 'broken' | 'unknown';

function isPrivateAddress(ip: string): boolean {
  if (isIP(ip) === 6) {
    const v = ip.toLowerCase();
    if (v === '::1' || v === '::') return true;
    if (v.startsWith('fc') || v.startsWith('fd') || v.startsWith('fe80')) return true;
    const mapped = v.match(/^::ffff:(\d+\.\d+\.\d+\.\d+)$/);
    return mapped ? isPrivateAddress(mapped[1]!) : false;
  }
  const [a, b] = ip.split('.').map(Number) as [number, number];
  return a === 10 || a === 127 || a === 0 || (a === 169 && b === 254) || (a === 172 && b >= 16 && b <= 31)
    || (a === 192 && b === 168) || (a === 100 && b >= 64 && b <= 127) || a >= 224;
}

/**
 * Whether a URL points at the public internet. The server fetches links people saved, so it must never
 * be pointed at itself or its private network (cloud metadata, databases, admin panels).
 */
export async function isPublicUrl(raw: string, resolve: (host: string) => Promise<string[]> = defaultResolve): Promise<boolean> {
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    return false;
  }
  if (url.protocol !== 'http:' && url.protocol !== 'https:') return false;
  const host = url.hostname.replace(/^\[|\]$/g, '');
  if (host === 'localhost' || host.endsWith('.localhost') || host.endsWith('.internal') || host.endsWith('.local')) return false;
  if (isIP(host)) return !isPrivateAddress(host);
  try {
    const addresses = await resolve(host);
    return addresses.length > 0 && addresses.every((a) => !isPrivateAddress(a));
  } catch {
    return false;
  }
}

async function defaultResolve(host: string): Promise<string[]> {
  return (await lookup(host, {all: true})).map((r) => r.address);
}

type Fetcher = (url: string, init: RequestInit) => Promise<Response>;
type Deps = {fetch?: Fetcher; resolve?: (host: string) => Promise<string[]>};

/**
 * Checks one link. Broken means the site says the page is gone (404, 410) or the name no longer resolves.
 * Anything unclear (timeouts, 5xx, blocks) is unknown, so a flaky site is never marked broken.
 */
export async function checkLink(url: string, deps: Deps = {}): Promise<LinkState> {
  const doFetch = deps.fetch ?? ((u: string, init: RequestInit) => fetch(u, init));
  let current = url;
  for (let hop = 0; hop <= MAX_REDIRECTS; hop++) {
    let host: string;
    try {
      host = new URL(current).hostname;
    } catch {
      return 'broken';
    }
    if (!(await isPublicUrl(current, deps.resolve))) {
      // A name that no longer resolves is the classic dead link; a private address is just not checked.
      const resolves = await (deps.resolve ?? defaultResolve)(host).then((a) => a.length > 0, () => false);
      return resolves || isIP(host) ? 'unknown' : 'broken';
    }
    let response: Response;
    try {
      response = await request(doFetch, current, 'HEAD');
      // Some servers refuse HEAD; ask again with GET before deciding.
      if ([400, 403, 405, 501].includes(response.status)) response = await request(doFetch, current, 'GET');
    } catch {
      return 'unknown';
    }
    if (response.status >= 300 && response.status < 400) {
      const location = response.headers.get('location');
      if (!location) return 'unknown';
      current = new URL(location, current).toString();
      continue;
    }
    if (response.status === 404 || response.status === 410) return 'broken';
    if (response.status >= 200 && response.status < 300) return 'ok';
    return 'unknown';
  }
  return 'unknown';
}

async function request(doFetch: Fetcher, url: string, method: 'HEAD' | 'GET'): Promise<Response> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), TIMEOUT_MS);
  try {
    const response = await doFetch(url, {
      method,
      redirect: 'manual',
      signal: controller.signal,
      headers: {'user-agent': 'Mozilla/5.0 (compatible; StashLinkCheck/1.0)', accept: 'text/html,*/*'},
    });
    // Only the status matters; do not download the page.
    await response.body?.cancel().catch(() => {});
    return response;
  } finally {
    clearTimeout(timer);
  }
}

export type LinkCheckState = {running: boolean; checked: number; broken: number; due: number};

export async function linkCheckState(userId: string): Promise<LinkCheckState> {
  const row = await queryOne<LinkCheckState>(
    `select coalesce((select link_check_until > now() from users where id = $1), false) as running,
            count(*) filter (where link_checked_at is not null)::int as checked,
            count(*) filter (where link_status = 'broken')::int as broken,
            count(*) filter (where link_checked_at is null or link_checked_at < now() - interval '${RECHECK_DAYS} days')::int as due
     from items where user_id = $1 and sources && $2::text[]`,
    [userId, CHECKED_SOURCES],
  );
  return row ?? {running: false, checked: 0, broken: 0, due: 0};
}

/**
 * Checks the person's links that were never checked or not in the last month, a few at a time. Only one
 * check runs per person at once.
 */
export async function checkLinks(userId: string, deps: Deps = {}, maxItems = 3000, concurrency = 6): Promise<{checked: number; broken: number}> {
  const claimed = await queryOne(
    `update users set link_check_until = now() + interval '30 minutes'
     where id = $1 and (link_check_until is null or link_check_until < now()) returning id`,
    [userId],
  );
  if (!claimed) return {checked: 0, broken: 0};
  let checked = 0;
  let broken = 0;
  try {
    const due = await query<{id: string; url: string}>(
      `select id, url from items
       where user_id = $1 and sources && $2::text[]
         and (link_checked_at is null or link_checked_at < now() - interval '${RECHECK_DAYS} days')
       order by link_checked_at nulls first, coalesce(saved_at, imported_at) limit $3`,
      [userId, CHECKED_SOURCES, maxItems],
    );
    let next = 0;
    const worker = async () => {
      while (next < due.length) {
        const item = due[next++]!;
        const state = await checkLink(item.url, deps);
        await query(
          `update items set link_checked_at = now(), link_status = case when $3 = 'unknown' then link_status else $3 end
           where user_id = $1 and id = $2`,
          [userId, item.id, state],
        );
        checked++;
        if (state === 'broken') broken++;
      }
    };
    await Promise.all(Array.from({length: Math.min(concurrency, due.length)}, worker));
  } finally {
    await query('update users set link_check_until = null where id = $1', [userId]);
  }
  return {checked, broken};
}
