import type {ItemInput, Media} from '../types';
import {firstLine, isHttpUrl, isObject, str, walk} from './util';

/**
 * Reads the JSON LinkedIn's web app loads for "My items > Saved posts" (Voyager search results,
 * either fetched or embedded in the page) and returns one item per post. Each saved post arrives as an
 * EntityResultViewModel: title is the author, summary the post text, navigationUrl the post.
 */
export function parseLinkedInSavedPosts(json: unknown): ItemInput[] {
  const seen = new Set<string>();
  const items: ItemInput[] = [];
  walk(json, (obj) => {
    const type = str(obj['$type']) ?? str(obj['_type']) ?? '';
    if (!type.endsWith('EntityResultViewModel')) return;
    const item = entityToItem(obj);
    if (item && !seen.has(item.sourceId)) {
      seen.add(item.sourceId);
      items.push(item);
    }
  });
  return items;
}

const POST_URN = /urn:li:(?:activity|ugcPost|share):\d+/;

function entityToItem(e: Record<string, unknown>): ItemInput | null {
  const navigationUrl = str(e['navigationUrl']);
  const urn = (navigationUrl && decodeURIComponent(navigationUrl).match(POST_URN)?.[0])
    ?? str(e['entityUrn'])?.match(POST_URN)?.[0]
    ?? str(e['trackingUrn'])?.match(POST_URN)?.[0];
  if (!urn) return null;

  const url = `https://www.linkedin.com/feed/update/${urn}/`;
  const author = textOf(e['title']);
  const headline = textOf(e['primarySubtitle']);
  const text = textOf(e['summary']);
  const avatar = vectorImageUrl(e['image']);
  const embedded = isObject(e['entityEmbeddedObject']) ? e['entityEmbeddedObject'] : undefined;
  const embeddedImage = embedded ? vectorImageUrl(embedded['image']) : undefined;
  const media: Media[] = embeddedImage ? [{type: 'image', url: embeddedImage}] : [];
  const linkTitle = embedded ? textOf(embedded['title']) : undefined;

  return {
    source: 'linkedin',
    sourceId: urn,
    url,
    title: firstLine(text) ?? linkTitle ?? (author ? `Post by ${author}` : null),
    text: text ?? linkTitle ?? null,
    authorName: author ?? null,
    authorHandle: headline ?? null,
    authorAvatar: avatar ?? null,
    media: media.length ? media : null,
  };
}

function textOf(v: unknown): string | undefined {
  if (typeof v === 'string') return v.trim() || undefined;
  if (isObject(v)) return str(v['text'])?.trim() || undefined;
  return undefined;
}

/** The largest rendition of the first vectorImage under a node, as a full URL. */
function vectorImageUrl(node: unknown): string | undefined {
  let found: string | undefined;
  walk(node, (obj) => {
    if (found) return;
    const vi = obj['vectorImage'];
    if (!isObject(vi)) return;
    const root = str(vi['rootUrl']) ?? '';
    const artifacts = Array.isArray(vi['artifacts']) ? vi['artifacts'].filter(isObject) : [];
    const best = artifacts.sort((a, b) => Number(b['width'] ?? 0) - Number(a['width'] ?? 0))[0];
    const segment = best ? str(best['fileIdentifyingUrlPathSegment']) : undefined;
    const url = segment ? root + segment : undefined;
    if (isHttpUrl(url)) found = url;
  });
  return found;
}
