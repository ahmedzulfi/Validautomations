import type {ItemInput} from '../types';
import {isHttpUrl, isObject, str, walk} from './util';

/**
 * Reads YouTube's own data for a playlist page (Watch later, Liked videos): the ytInitialData the page
 * embeds, and the /youtubei/v1/browse responses it loads while scrolling. Every playlistVideoRenderer
 * (and the newer lockupViewModel) becomes an item.
 */
export function parseYouTubePlaylist(json: unknown): ItemInput[] {
  const seen = new Set<string>();
  const items: ItemInput[] = [];
  walk(json, (obj) => {
    const item = isObject(obj['playlistVideoRenderer']) ? fromRenderer(obj['playlistVideoRenderer'])
      : isObject(obj['lockupViewModel']) ? fromLockup(obj['lockupViewModel'])
      : null;
    if (item && !seen.has(item.sourceId)) {
      seen.add(item.sourceId);
      items.push(item);
    }
  });
  return items;
}

function text(node: unknown): string | undefined {
  if (!isObject(node)) return undefined;
  if (typeof node['simpleText'] === 'string') return node['simpleText'];
  if (typeof node['content'] === 'string') return node['content'];
  if (Array.isArray(node['runs'])) return node['runs'].map((r) => (isObject(r) ? str(r['text']) ?? '' : '')).join('') || undefined;
  return undefined;
}

function lastThumbnail(node: unknown): string | undefined {
  const thumbs = isObject(node) && Array.isArray(node['thumbnails']) ? node['thumbnails'] : [];
  const last = thumbs[thumbs.length - 1];
  const url = isObject(last) ? str(last['url'])?.split('?')[0] : undefined;
  return isHttpUrl(url) ? url : undefined;
}

function fromRenderer(r: Record<string, unknown>): ItemInput | null {
  const videoId = str(r['videoId']);
  if (!videoId) return null;
  const title = text(r['title']);
  // Deleted and private videos stay in playlists with placeholder titles; skip them.
  if (!title || /^\[(Deleted|Private) video\]$/.test(title)) return null;
  const byline = isObject(r['shortBylineText']) && Array.isArray(r['shortBylineText']['runs']) ? r['shortBylineText']['runs'][0] : undefined;
  const channel = isObject(byline) ? str(byline['text']) : undefined;
  let handle: string | undefined;
  if (isObject(byline)) {
    walk(byline['navigationEndpoint'], (o) => {
      handle ??= str(o['canonicalBaseUrl'])?.replace(/^\//, '');
    });
  }
  const length = text(r['lengthText']);
  const thumb = lastThumbnail(r['thumbnail']);
  return {
    source: 'youtube',
    sourceId: videoId,
    url: `https://www.youtube.com/watch?v=${videoId}`,
    title,
    text: [channel ? `A video by ${channel}` : null, length ? `${length} long` : null].filter(Boolean).join(', ') || null,
    authorName: channel ?? null,
    authorHandle: handle ?? null,
    media: thumb ? [{type: 'image', url: thumb}] : null,
  };
}

function fromLockup(l: Record<string, unknown>): ItemInput | null {
  const videoId = str(l['contentId']);
  if (!videoId || l['contentType'] !== 'LOCKUP_CONTENT_TYPE_VIDEO') return null;
  let title: string | undefined;
  let channel: string | undefined;
  walk(l['metadata'], (o) => {
    if (isObject(o['lockupMetadataViewModel'])) title ??= text(o['lockupMetadataViewModel']['title']);
    if (Array.isArray(o['metadataParts']) && !channel) {
      const first = o['metadataParts'][0];
      channel = isObject(first) ? text(first['text']) : undefined;
    }
  });
  if (!title) return null;
  return {
    source: 'youtube',
    sourceId: videoId,
    url: `https://www.youtube.com/watch?v=${videoId}`,
    title,
    text: channel ? `A video by ${channel}` : null,
    authorName: channel ?? null,
    media: [{type: 'image', url: `https://i.ytimg.com/vi/${videoId}/hqdefault.jpg`}],
  };
}

/** Pulls the JSON assigned to ytInitialData out of a page's script text. */
export function extractYtInitialData(scriptText: string): unknown | null {
  const start = scriptText.search(/(?:var |window\[["'])ytInitialData(?:["']\])?\s*=\s*\{/);
  if (start < 0) return null;
  const open = scriptText.indexOf('{', start);
  let depth = 0;
  let inString = false;
  for (let i = open; i < scriptText.length; i++) {
    const ch = scriptText[i];
    if (inString) {
      if (ch === '\\') i++;
      else if (ch === '"') inString = false;
    } else if (ch === '"') inString = true;
    else if (ch === '{') depth++;
    else if (ch === '}' && --depth === 0) {
      try {
        return JSON.parse(scriptText.slice(open, i + 1));
      } catch {
        return null;
      }
    }
  }
  return null;
}
