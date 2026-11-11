import type {ItemInput, Media} from '../types';
import {firstLine, isHttpUrl, isObject, str, toIso, walk} from './util';

/**
 * Reads Instagram's saved-posts JSON (the web app's /api/v1/feed/saved/posts/ response, or the GraphQL
 * connection it uses). Any media object (it has a shortcode and an owner) becomes an item.
 */
export function parseInstagramSaved(json: unknown): ItemInput[] {
  const seen = new Set<string>();
  const items: ItemInput[] = [];
  walk(json, (obj) => {
    const code = str(obj['code']) ?? str(obj['shortcode']);
    const user = isObject(obj['user']) ? obj['user'] : isObject(obj['owner']) ? obj['owner'] : null;
    if (!code || !user || seen.has(code)) return;
    seen.add(code);
    const caption = isObject(obj['caption']) ? str(obj['caption']['text']) : undefined;
    const username = str(user['username']);
    const media: Media[] = [];
    const candidates = isObject(obj['image_versions2']) && Array.isArray(obj['image_versions2']['candidates']) ? obj['image_versions2']['candidates'] : [];
    const first = candidates[0];
    const image = isObject(first) ? str(first['url']) : str(obj['display_url']);
    if (isHttpUrl(image)) media.push({type: obj['media_type'] === 2 || obj['video_versions'] ? 'video' : 'image', url: image});
    const avatar = str(user['profile_pic_url']);
    items.push({
      source: 'instagram',
      sourceId: code,
      url: obj['product_type'] === 'clips' ? `https://www.instagram.com/reel/${code}/` : `https://www.instagram.com/p/${code}/`,
      title: firstLine(caption) ?? (username ? `Post by @${username}` : null),
      text: caption ?? null,
      authorName: str(user['full_name']) || username || null,
      authorHandle: username ? `@${username}` : null,
      authorAvatar: isHttpUrl(avatar) ? avatar : null,
      media: media.length ? media : null,
      postedAt: typeof obj['taken_at'] === 'number' ? toIso(obj['taken_at'] * 1000) ?? null : null,
    });
  });
  return items;
}

/** The cursor for the next page of saved posts, or null at the end. */
export function instagramNextMaxId(json: unknown): string | null {
  if (!isObject(json)) return null;
  if (json['more_available'] === false) return null;
  return str(json['next_max_id']) ?? null;
}
