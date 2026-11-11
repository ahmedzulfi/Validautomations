import type {ItemInput} from '../types';
import {canonicalUrl} from '../url';
import {isHttpUrl, toIso} from './util';

/** The node shape of chrome.bookmarks.getTree() (and Firefox's browser.bookmarks). */
export type BookmarkNode = {
  id: string;
  title: string;
  url?: string;
  dateAdded?: number;
  children?: BookmarkNode[];
};

/**
 * Flattens the browser's bookmark tree into items, keeping each one's folder path as a hint for the AI.
 * The browser's own top-level containers ("Bookmarks bar", "Other bookmarks") are left out of the path.
 */
export function flattenBookmarkTree(roots: BookmarkNode[]): ItemInput[] {
  const items: ItemInput[] = [];
  const seen = new Set<string>();
  const visit = (node: BookmarkNode, path: string[], depth: number) => {
    if (node.url) {
      if (!isHttpUrl(node.url)) return;
      const id = canonicalUrl(node.url);
      if (seen.has(id)) return;
      seen.add(id);
      items.push({
        source: 'bookmark',
        sourceId: id,
        url: node.url,
        title: node.title?.trim() || null,
        folderPath: path.length ? path : null,
        savedAt: toIso(node.dateAdded) ?? null,
      });
      return;
    }
    // depth 0 is the invisible root, depth 1 the browser's containers.
    const nextPath = depth >= 2 && node.title ? [...path, node.title] : path;
    for (const child of node.children ?? []) visit(child, nextPath, depth + 1);
  };
  for (const root of roots) visit(root, [], 0);
  return items;
}

/**
 * Parses the "bookmarks.html" every browser exports (the Netscape bookmark format), without a DOM, so it
 * works on the server. Folders become the folder path.
 */
export function parseBookmarksHtml(html: string): ItemInput[] {
  const items: ItemInput[] = [];
  const seen = new Set<string>();
  const stack: string[] = [];
  let pendingFolder: string | null = null;
  const tag = /<(\/?)(dl|h3|a)\b([^>]*)>([^<]*)/gi;
  let m: RegExpExecArray | null;
  const folders: Array<string | null> = [];
  while ((m = tag.exec(html))) {
    const closing = m[1] === '/';
    const name = m[2]!.toLowerCase();
    const attrs = m[3] ?? '';
    const text = decode(m[4] ?? '').trim();
    if (name === 'h3' && !closing) {
      pendingFolder = text;
    } else if (name === 'dl' && !closing) {
      folders.push(pendingFolder);
      if (pendingFolder !== null) stack.push(pendingFolder);
      pendingFolder = null;
    } else if (name === 'dl' && closing) {
      const f = folders.pop();
      if (f !== null && f !== undefined) stack.pop();
    } else if (name === 'a' && !closing) {
      const href = attr(attrs, 'href');
      if (!isHttpUrl(href)) continue;
      const id = canonicalUrl(href);
      if (seen.has(id)) continue;
      seen.add(id);
      const added = attr(attrs, 'add_date');
      const path = stack.filter((f) => !BROWSER_CONTAINERS.has(f.toLowerCase()));
      items.push({
        source: 'bookmark',
        sourceId: id,
        url: href,
        title: text || null,
        folderPath: path.length ? path : null,
        savedAt: added && /^\d+$/.test(added) ? toIso(Number(added) * 1000) ?? null : null,
      });
    }
  }
  return items;
}

const BROWSER_CONTAINERS = new Set([
  'bookmarks bar', 'bookmarks toolbar', 'other bookmarks', 'mobile bookmarks', 'bookmarks menu', 'favorites bar', 'favourites bar',
]);

function attr(attrs: string, name: string): string | undefined {
  const m = attrs.match(new RegExp(`\\b${name}\\s*=\\s*("([^"]*)"|'([^']*)'|([^\\s>]+))`, 'i'));
  const raw = m ? (m[2] ?? m[3] ?? m[4]) : undefined;
  return raw === undefined ? undefined : decode(raw);
}

function decode(s: string): string {
  return s
    .replace(/&#(\d+);/g, (_, n: string) => String.fromCodePoint(Number(n)))
    .replace(/&#x([0-9a-f]+);/gi, (_, n: string) => String.fromCodePoint(parseInt(n, 16)))
    .replace(/&quot;/g, '"').replace(/&#39;/g, "'").replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&');
}
