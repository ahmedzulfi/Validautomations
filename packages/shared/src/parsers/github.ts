import type {ItemInput} from '../types';
import {isHttpUrl, isObject, str, toIso} from './util';

/**
 * Reads GitHub's starred-repositories API (GET /users/NAME/starred with the star+json media type, which
 * adds when each repo was starred). Plain responses without starred_at work too.
 */
export function parseGitHubStars(json: unknown): ItemInput[] {
  if (!Array.isArray(json)) return [];
  const items: ItemInput[] = [];
  for (const entry of json) {
    if (!isObject(entry)) continue;
    const repo = isObject(entry['repo']) ? entry['repo'] : entry;
    const fullName = str(repo['full_name']);
    const url = str(repo['html_url']);
    if (!fullName || !isHttpUrl(url)) continue;
    const owner = isObject(repo['owner']) ? repo['owner'] : {};
    const avatar = str(owner['avatar_url']);
    const homepage = str(repo['homepage']);
    const topics = Array.isArray(repo['topics']) ? repo['topics'].filter((t): t is string => typeof t === 'string') : [];
    const language = str(repo['language']);
    const description = str(repo['description']);
    const details = [description, language ? `Written in ${language}.` : null, topics.length ? `Topics: ${topics.join(', ')}.` : null].filter(Boolean).join(' ');
    items.push({
      source: 'github',
      sourceId: fullName.toLowerCase(),
      url,
      title: fullName,
      text: details || null,
      authorName: str(owner['login']) ?? null,
      authorAvatar: isHttpUrl(avatar) ? avatar : null,
      links: isHttpUrl(homepage) ? [homepage] : null,
      savedAt: toIso(str(entry['starred_at'])) ?? null,
    });
  }
  return items;
}
