import 'server-only';
import {chunk, MAX_INGEST_BATCH, parseBookmarksCsv, parseBookmarksHtml, parseGitHubStars, type IngestResponse, type ItemInput} from '@stash/shared';
import {ingestItems, recordSyncRun} from './items';

const GITHUB_USERNAME = /^[a-z\d](?:[a-z\d]|-(?=[a-z\d])){0,38}$/i;
const MAX_STAR_PAGES = 50;

export class ImportError extends Error {}

async function ingestAll(userId: string, items: ItemInput[]): Promise<IngestResponse> {
  const total: IngestResponse = {added: 0, existing: 0};
  for (const batch of chunk(items, MAX_INGEST_BATCH)) {
    const r = await ingestItems(userId, batch);
    total.added += r.added;
    total.existing += r.existing;
  }
  return total;
}

/**
 * Imports the repositories a GitHub user has starred. Stars are public, so only the username is needed;
 * GITHUB_TOKEN raises the rate limit from 60 to 5000 requests an hour.
 */
export async function importGitHubStars(userId: string, username: string, fetchImpl: typeof fetch = fetch): Promise<IngestResponse & {found: number}> {
  const name = username.trim().replace(/^@/, '');
  if (!GITHUB_USERNAME.test(name)) throw new ImportError('Enter a GitHub username, like octocat.');
  const items: ItemInput[] = [];
  for (let page = 1; page <= MAX_STAR_PAGES; page++) {
    const apiBase = (process.env.GITHUB_API_URL || 'https://api.github.com').replace(/\/$/, '');
    const response = await fetchImpl(`${apiBase}/users/${encodeURIComponent(name)}/starred?per_page=100&page=${page}`, {
      headers: {
        accept: 'application/vnd.github.star+json',
        'user-agent': 'Stash',
        ...(process.env.GITHUB_TOKEN ? {authorization: `Bearer ${process.env.GITHUB_TOKEN}`} : {}),
      },
    }).catch(() => null);
    if (!response) throw new ImportError('Could not reach GitHub. Try again in a minute.');
    if (response.status === 404) throw new ImportError(`There is no GitHub user called ${name}.`);
    if (response.status === 403 || response.status === 429) throw new ImportError('GitHub is limiting requests right now. Try again later, or set GITHUB_TOKEN on the server.');
    if (!response.ok) throw new ImportError(`GitHub answered ${response.status}. Try again.`);
    const batch = parseGitHubStars(await response.json());
    items.push(...batch);
    if (batch.length < 100) break;
  }
  const result = await ingestAll(userId, items);
  await recordSyncRun(userId, {source: 'github', itemsSeen: items.length, itemsAdded: result.added});
  return {...result, found: items.length};
}

/** Imports a bookmarks export: the browser's HTML file, or a CSV from Pocket, Raindrop or anywhere with a url column. */
export async function importBookmarksFile(userId: string, name: string, text: string): Promise<IngestResponse & {found: number; format: string}> {
  const isCsv = /\.csv$/i.test(name) || (!/<a\s/i.test(text) && /^[^\n]*\burl\b/i.test(text));
  const {items, format} = isCsv ? parseBookmarksCsv(text) : {items: parseBookmarksHtml(text), format: 'html'};
  if (items.length === 0) {
    throw new ImportError(isCsv
      ? 'No links found in that CSV. It needs a column called url.'
      : 'No bookmarks found in that file. Use the HTML file your browser exports, or a CSV with a url column.');
  }
  const result = await ingestAll(userId, items);
  await recordSyncRun(userId, {source: 'bookmark', itemsSeen: items.length, itemsAdded: result.added});
  return {...result, found: items.length, format: format ?? 'csv'};
}
