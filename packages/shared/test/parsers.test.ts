import {readFileSync} from 'node:fs';
import {describe, expect, it} from 'vitest';
import {canonicalUrl, flattenBookmarkTree, itemInputSchema, parseBookmarksHtml, parseLinkedInSavedPosts, parseXBookmarks, xBottomCursor} from '../src';

const fixture = (name: string) => readFileSync(new URL(`./fixtures/${name}`, import.meta.url), 'utf8');

describe('parseXBookmarks', () => {
  const json = JSON.parse(fixture('x-bookmarks.json'));
  const items = parseXBookmarks(json);

  it('returns one item per post and skips tombstones', () => {
    expect(items.map((i) => i.sourceId)).toEqual(['1850000000000000001', '1850000000000000002']);
  });

  it('reads the new user shape, expands links and drops media links', () => {
    const first = items[0]!;
    expect(first.authorName).toBe('Maya Chen');
    expect(first.authorHandle).toBe('@mayabuilds');
    expect(first.url).toBe('https://x.com/mayabuilds/status/1850000000000000001');
    expect(first.text).toContain('https://mayachen.dev/pricing-notes');
    expect(first.text).not.toContain('t.co');
    expect(first.links).toEqual(['https://mayachen.dev/pricing-notes?utm_source=twitter']);
    expect(first.media).toEqual([{type: 'image', url: 'https://pbs.twimg.com/media/pricing.jpg'}]);
    expect(first.postedAt).toBe('2025-10-21T14:03:11.000Z');
  });

  it('unwraps limited-visibility posts and prefers the long text', () => {
    const second = items[1]!;
    expect(second.authorHandle).toBe('@theop');
    expect(second.text).toBe('A long thread on Postgres indexes & when partial indexes beat composite ones.\n\nFirst: measure.');
    expect(second.title).toBe('A long thread on Postgres indexes & when partial indexes beat composite ones.');
  });

  it('produces items the ingest API accepts', () => {
    for (const item of items) expect(itemInputSchema.safeParse(item).success).toBe(true);
  });

  it('finds the next-page cursor', () => {
    expect(xBottomCursor(json)).toBe('HBaAgLydwdDx');
  });

  it('returns nothing for unrelated JSON', () => {
    expect(parseXBookmarks({data: {viewer: {}}})).toEqual([]);
  });
});

describe('parseLinkedInSavedPosts', () => {
  const items = parseLinkedInSavedPosts(JSON.parse(fixture('linkedin-saved.json')));

  it('returns saved posts only', () => {
    expect(items.map((i) => i.sourceId)).toEqual(['urn:li:activity:7254000000000000001', 'urn:li:ugcPost:7254000000000000002']);
  });

  it('maps author, text, avatar and post image', () => {
    const first = items[0]!;
    expect(first.authorName).toBe('Priya Raman');
    expect(first.authorHandle).toBe('Head of Growth at Fieldnote');
    expect(first.title).toBe('We cut onboarding from 9 steps to 4. Activation went up 18%.');
    expect(first.url).toBe('https://www.linkedin.com/feed/update/urn:li:activity:7254000000000000001/');
    expect(first.authorAvatar).toBe('https://media.licdn.com/dms/image/v2/abc/profile-displayphoto-shrink_400_400/0/1?e=1&v=beta');
    expect(first.media).toEqual([{type: 'image', url: 'https://media.licdn.com/dms/image/v2/post/feedshare_800/0/2'}]);
  });

  it('reads an encoded urn from the link', () => {
    expect(items[1]!.url).toBe('https://www.linkedin.com/feed/update/urn:li:ugcPost:7254000000000000002/');
  });

  it('produces items the ingest API accepts', () => {
    for (const item of items) expect(itemInputSchema.safeParse(item).success).toBe(true);
  });
});

describe('parseBookmarksHtml', () => {
  const items = parseBookmarksHtml(fixture('bookmarks.html'));

  it('keeps folders, skips browser containers, duplicates and non-web links', () => {
    expect(items.map((i) => [i.title, i.folderPath])).toEqual([
      ['Hacker News', null],
      ['Refactoring UI & more', ['Design']],
      ['A post', null],
    ]);
  });

  it('reads the date added', () => {
    expect(items[0]!.savedAt).toBe('2023-11-14T22:15:00.000Z');
  });

  it('uses the canonical url as the id', () => {
    expect(items[2]!.sourceId).toBe('https://x.com/i/status/1850000000000000001');
  });
});

describe('flattenBookmarkTree', () => {
  it('flattens the browser tree with folder paths', () => {
    const items = flattenBookmarkTree([
      {id: '0', title: '', children: [
        {id: '1', title: 'Bookmarks bar', children: [
          {id: '3', title: 'Reading', children: [
            {id: '4', title: 'Essay', url: 'https://paulgraham.com/greatwork.html', dateAdded: 1700000000000},
          ]},
          {id: '5', title: 'Docs', url: 'https://nextjs.org/docs'},
          {id: '6', title: 'Local', url: 'chrome://settings'},
        ]},
      ]},
    ]);
    expect(items).toEqual([
      expect.objectContaining({title: 'Essay', folderPath: ['Reading'], savedAt: '2023-11-14T22:13:20.000Z'}),
      expect.objectContaining({title: 'Docs', folderPath: null}),
    ]);
  });
});

describe('canonicalUrl', () => {
  it.each([
    ['https://www.Example.com/a/?utm_source=x&b=2&a=1#top', 'https://example.com/a?a=1&b=2'],
    ['http://example.com/', 'https://example.com'],
    ['https://twitter.com/jack/status/20?s=20', 'https://x.com/i/status/20'],
    ['https://example.com/?s=search', 'https://example.com/?s=search'],
    ['not a url', 'not a url'],
  ])('%s → %s', (input, expected) => {
    expect(canonicalUrl(input)).toBe(expected);
  });
});
