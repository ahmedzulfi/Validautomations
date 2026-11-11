import {readFileSync} from 'node:fs';
import {describe, expect, it} from 'vitest';
import {
  extractYtInitialData, instagramNextMaxId, itemInputSchema, parseBookmarksCsv, parseCsv, parseGitHubStars,
  parseInstagramSaved, parseRedditSaved, parseYouTubePlaylist, redditNextAfter, type ItemInput,
} from '../src';

const fixture = (name: string) => readFileSync(new URL(`./fixtures/${name}`, import.meta.url), 'utf8');
const allValid = (items: ItemInput[]) => items.forEach((i) => expect(itemInputSchema.safeParse(i).success, JSON.stringify(i)).toBe(true));

describe('parseRedditSaved', () => {
  const json = JSON.parse(fixture('reddit-saved.json'));
  const items = parseRedditSaved(json);
  it('reads saved posts and comments', () => {
    expect(items.map((i) => i.sourceId)).toEqual(['t3_1a2b3c', 't3_4d5e6f', 't1_ggg']);
    expect(items[0]).toMatchObject({
      url: 'https://www.reddit.com/r/selfhosted/comments/1a2b3c/i_built_a_selfhosted_readitlater_app/',
      authorName: 'u/tinkerer', authorHandle: 'r/selfhosted', links: ['https://github.com/example/readlater'],
      text: 'https://github.com/example/readlater', postedAt: '2024-10-21T08:40:00.000Z',
    });
    expect(items[1]!.links).toBeNull();
    expect(items[2]).toMatchObject({title: 'Comment on: How do you structure a Postgres schema for multi-tenant SaaS?', text: 'Use row level security and a tenant_id on every table.'});
    allValid(items);
  });
  it('finds the next page', () => {
    expect(redditNextAfter(json)).toBe('t3_abc999');
    expect(redditNextAfter({data: {after: null, children: []}})).toBeNull();
  });
});

describe('YouTube', () => {
  it('pulls ytInitialData out of the page and reads the playlist, skipping deleted videos', () => {
    const data = extractYtInitialData(fixture('youtube-wl.html'));
    const items = parseYouTubePlaylist(data);
    expect(items).toEqual([expect.objectContaining({
      sourceId: 'dQw4w9WgXcQ', url: 'https://www.youtube.com/watch?v=dQw4w9WgXcQ', title: 'Designing data-intensive apps, a talk',
      authorName: 'Tech Talks', authorHandle: '@techtalks', text: 'A video by Tech Talks, 42:10 long',
      media: [{type: 'image', url: 'https://i.ytimg.com/vi/dQw4w9WgXcQ/hqdefault.jpg'}],
    })]);
    allValid(items);
  });
  it('reads the newer lockup shape from scroll responses', () => {
    const items = parseYouTubePlaylist(JSON.parse(fixture('youtube-continuation.json')));
    expect(items).toEqual([expect.objectContaining({sourceId: 'abcdefghijk', title: 'Pricing strategy for SaaS founders', authorName: 'Founder Lessons'})]);
    allValid(items);
  });
  it('returns null when the page has no data', () => {
    expect(extractYtInitialData('<script>var x = 1;</script>')).toBeNull();
  });
});

describe('parseInstagramSaved', () => {
  const json = JSON.parse(fixture('instagram-saved.json'));
  it('reads posts and reels', () => {
    const items = parseInstagramSaved(json);
    expect(items.map((i) => i.url)).toEqual(['https://www.instagram.com/p/C9abcDEF/', 'https://www.instagram.com/reel/Dreel123/']);
    expect(items[0]).toMatchObject({title: 'Five layout rules I use on every landing page.', authorName: 'Ana Ruiz', authorHandle: '@designwithana'});
    expect(items[1]).toMatchObject({title: 'Post by @chefmarco', authorName: 'chefmarco', media: [{type: 'video', url: 'https://scontent.cdninstagram.com/reel.jpg'}]});
    allValid(items);
  });
  it('finds the next page', () => {
    expect(instagramNextMaxId(json)).toBe('QVFE123');
    expect(instagramNextMaxId({more_available: false, next_max_id: 'x'})).toBeNull();
  });
});

describe('parseGitHubStars', () => {
  it('reads starred repos with and without the star date', () => {
    const items = parseGitHubStars(JSON.parse(fixture('github-stars.json')));
    expect(items[0]).toMatchObject({
      sourceId: 'pgvector/pgvector', title: 'pgvector/pgvector', savedAt: '2025-09-01T10:00:00.000Z',
      text: 'Open-source vector similarity search for Postgres Written in C. Topics: postgres, vector-search.', links: null,
    });
    expect(items[1]).toMatchObject({sourceId: 'vercel/ai', links: ['https://ai-sdk.dev'], savedAt: null});
    allValid(items);
  });
});

describe('CSV imports', () => {
  it('splits quoted cells and escaped quotes', () => {
    expect(parseCsv('a,"b,c","say ""hi"""\r\n1,2,3\n')).toEqual([['a', 'b,c', 'say "hi"'], ['1', '2', '3']]);
  });
  it('reads a Pocket export', () => {
    const {items, format} = parseBookmarksCsv(fixture('pocket.csv'));
    expect(format).toBe('pocket');
    expect(items[0]).toMatchObject({title: 'Why "boring" tech wins', text: 'Tags: engineering, essays', savedAt: '2023-11-14T22:13:20.000Z'});
    expect(items[1]!.title).toBeNull();
    allValid(items);
  });
  it('reads a Raindrop export with folders and skips bad rows', () => {
    const {items, format} = parseBookmarksCsv(fixture('raindrop.csv'));
    expect(format).toBe('raindrop');
    expect(items).toEqual([expect.objectContaining({title: 'Refactoring UI', folderPath: ['Design', 'Books'], text: 'Design tips for developers\nTags: design, ui', savedAt: '2024-03-01T12:00:00.000Z'})]);
  });
  it('rejects a CSV without a url column', () => {
    expect(parseBookmarksCsv('name,age\nA,3')).toEqual({items: [], format: null});
  });
});
