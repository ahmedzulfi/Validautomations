import type {ItemInput, Media} from '../types';
import {firstLine, isHttpUrl, isObject, str, toIso, walk} from './util';

/**
 * Reads the JSON that x.com's own web app loads for the bookmarks page (the GraphQL `Bookmarks`
 * operation) and returns one item per post. It looks for every `tweet_results.result` in the tree, so
 * small changes to where X nests the timeline do not break it.
 */
export function parseXBookmarks(json: unknown): ItemInput[] {
  const seen = new Set<string>();
  const items: ItemInput[] = [];
  walk(json, (obj) => {
    const results = obj['tweet_results'];
    if (!isObject(results)) return;
    const item = tweetToItem(results['result']);
    if (item && !seen.has(item.sourceId)) {
      seen.add(item.sourceId);
      items.push(item);
    }
  });
  return items;
}

/** The cursor X uses to load the next page of bookmarks, if there is one. */
export function xBottomCursor(json: unknown): string | undefined {
  let cursor: string | undefined;
  walk(json, (obj) => {
    if (obj['cursorType'] === 'Bottom' && typeof obj['value'] === 'string') cursor = obj['value'];
  });
  return cursor;
}

function tweetToItem(raw: unknown): ItemInput | null {
  if (!isObject(raw)) return null;
  // Posts with limited visibility come wrapped once more.
  const tweet = raw['__typename'] === 'TweetWithVisibilityResults' && isObject(raw['tweet']) ? raw['tweet'] : raw;
  if (tweet['__typename'] && tweet['__typename'] !== 'Tweet') return null;
  const legacy = isObject(tweet['legacy']) ? tweet['legacy'] : {};
  const id = str(tweet['rest_id']) ?? str(legacy['id_str']);
  if (!id) return null;

  const user = dig(tweet, ['core', 'user_results', 'result']);
  const userCore = isObject(user) && isObject(user['core']) ? user['core'] : {};
  const userLegacy = isObject(user) && isObject(user['legacy']) ? user['legacy'] : {};
  const name = str(userCore['name']) ?? str(userLegacy['name']);
  const handle = str(userCore['screen_name']) ?? str(userLegacy['screen_name']);
  const avatarRaw = str(dig(user, ['avatar', 'image_url'])) ?? str(userLegacy['profile_image_url_https']);

  const longText = str(dig(tweet, ['note_tweet', 'note_tweet_results', 'result', 'text']));
  const entities = isObject(legacy['entities']) ? legacy['entities'] : {};
  const urlEntities = Array.isArray(entities['urls']) ? entities['urls'].filter(isObject) : [];
  const mediaEntities = Array.isArray(entities['media']) ? entities['media'].filter(isObject) : [];

  let text = longText ?? str(legacy['full_text']) ?? '';
  for (const u of urlEntities) {
    const short = str(u['url']);
    const expanded = str(u['expanded_url']);
    if (short && expanded) text = text.split(short).join(expanded);
  }
  for (const m of mediaEntities) {
    const short = str(m['url']);
    if (short) text = text.split(short).join('');
  }
  text = decodeEntities(text).trim();

  const links = urlEntities
    .map((u) => str(u['expanded_url']))
    .filter(isHttpUrl)
    .filter((u) => !/^https?:\/\/(x|twitter)\.com\//.test(u));

  const extended = dig(legacy, ['extended_entities', 'media']);
  const media: Media[] = [];
  if (Array.isArray(extended)) {
    for (const m of extended.filter(isObject)) {
      const url = str(m['media_url_https']);
      if (!isHttpUrl(url)) continue;
      media.push({type: m['type'] === 'photo' ? 'image' : 'video', url});
    }
  }

  return {
    source: 'x',
    sourceId: id,
    url: `https://x.com/${handle ?? 'i'}/status/${id}`,
    title: firstLine(text) ?? null,
    text: text || null,
    authorName: name ?? null,
    authorHandle: handle ? `@${handle}` : null,
    authorAvatar: isHttpUrl(avatarRaw) ? avatarRaw : null,
    media: media.length ? media.slice(0, 10) : null,
    links: links.length ? [...new Set(links)].slice(0, 20) : null,
    postedAt: toIso(str(legacy['created_at'])) ?? null,
  };
}

function dig(obj: unknown, path: string[]): unknown {
  let cur: unknown = obj;
  for (const key of path) {
    if (!isObject(cur)) return undefined;
    cur = cur[key];
  }
  return cur;
}

function decodeEntities(s: string): string {
  return s.replace(/&amp;/g, '&').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&#39;/g, "'");
}
