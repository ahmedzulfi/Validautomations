import type {ItemInput, Media} from '../types';
import {firstLine, isHttpUrl, isObject, str, toIso} from './util';

/**
 * Reads a page of Reddit's own listing JSON for saved things (/user/NAME/saved.json?raw_json=1).
 * Saved posts (t3) and saved comments (t1) both become items.
 */
export function parseRedditSaved(json: unknown): ItemInput[] {
  const children = isObject(json) && isObject(json['data']) && Array.isArray(json['data']['children']) ? json['data']['children'] : [];
  const items: ItemInput[] = [];
  for (const child of children) {
    if (!isObject(child) || !isObject(child['data'])) continue;
    const d = child['data'];
    const name = str(d['name']);
    const permalink = str(d['permalink']);
    if (!name || !permalink) continue;
    const url = `https://www.reddit.com${permalink}`;
    const author = str(d['author']);
    const sub = str(d['subreddit_name_prefixed']);
    const postedAt = typeof d['created_utc'] === 'number' ? toIso(d['created_utc'] * 1000) : undefined;

    if (child['kind'] === 't1') {
      const body = str(d['body'])?.trim();
      const on = str(d['link_title']);
      items.push({
        source: 'reddit', sourceId: name, url,
        title: on ? `Comment on: ${on}` : firstLine(body) ?? null,
        text: body ?? null,
        authorName: author ? `u/${author}` : null,
        authorHandle: sub ?? null,
        postedAt: postedAt ?? null,
      });
      continue;
    }
    if (child['kind'] !== 't3') continue;
    const external = str(d['url']);
    const links = isHttpUrl(external) && !/^https?:\/\/(www\.|old\.)?reddit\.com\//.test(external) && !/^https?:\/\/(i|v)\.redd\.it\//.test(external) ? [external] : [];
    const media: Media[] = [];
    const preview = isObject(d['preview']) && Array.isArray(d['preview']['images']) ? d['preview']['images'][0] : undefined;
    const image = isObject(preview) && isObject(preview['source']) ? str(preview['source']['url']) : undefined;
    if (isHttpUrl(image)) media.push({type: 'image', url: image});
    const selftext = str(d['selftext'])?.trim();
    items.push({
      source: 'reddit', sourceId: name, url,
      title: str(d['title']) ?? null,
      text: selftext || (links[0] ?? null),
      authorName: author ? `u/${author}` : null,
      authorHandle: sub ?? null,
      media: media.length ? media : null,
      links: links.length ? links : null,
      postedAt: postedAt ?? null,
    });
  }
  return items;
}

/** The cursor for the next page, or null at the end. */
export function redditNextAfter(json: unknown): string | null {
  return isObject(json) && isObject(json['data']) ? str(json['data']['after']) ?? null : null;
}
