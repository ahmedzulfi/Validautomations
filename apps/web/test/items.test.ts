import {describe, expect, it} from 'vitest';
import {ingestItems, libraryStats, listItems, moveItem} from '@/lib/server/items';
import {makeCategories, makeUser, query} from './helpers';

describe('ingestItems', () => {
  it('adds new items and recognises ones already saved', async () => {
    const user = await makeUser();
    const first = await ingestItems(user, [
      {source: 'x', sourceId: '1', url: 'https://x.com/maya/status/1', text: 'Short'},
      {source: 'bookmark', sourceId: 'https://example.com/a', url: 'https://www.example.com/a/?utm_source=x', title: 'Example'},
    ]);
    expect(first).toEqual({added: 2, existing: 0});

    const second = await ingestItems(user, [
      {source: 'x', sourceId: '1', url: 'https://twitter.com/maya_renamed/status/1', text: 'Short, now with the long text', authorName: 'Maya'},
      {source: 'bookmark', sourceId: 'https://example.com/b', url: 'https://example.com/b', title: 'B'},
    ]);
    expect(second).toEqual({added: 1, existing: 1});

    const rows = await query<{body: string; author_name: string; sources: string[]}>(
      `select body, author_name, sources from items where user_id = $1 and canonical_url = 'https://x.com/i/status/1'`,
      [user],
    );
    expect(rows).toEqual([{body: 'Short, now with the long text', author_name: 'Maya', sources: ['x']}]);
  });

  it('merges the same link saved from two sources into one item', async () => {
    const user = await makeUser();
    await ingestItems(user, [{source: 'x', sourceId: '99', url: 'https://x.com/a/status/99', text: 'Post'}]);
    const r = await ingestItems(user, [{source: 'bookmark', sourceId: 'https://x.com/i/status/99', url: 'https://twitter.com/a/status/99', title: 'Bookmarked post', folderPath: ['Reading']}]);
    expect(r).toEqual({added: 0, existing: 1});
    const [row] = await query<{sources: string[]; title: string; folder_path: string[]}>('select sources, title, folder_path from items where user_id = $1', [user]);
    expect(row).toEqual({sources: ['bookmark', 'x'], title: 'Bookmarked post', folder_path: ['Reading']});
  });

  it('handles the same link twice in one batch', async () => {
    const user = await makeUser();
    const r = await ingestItems(user, [
      {source: 'bookmark', sourceId: 'a', url: 'https://example.com/same'},
      {source: 'bookmark', sourceId: 'b', url: 'https://example.com/same/'},
    ]);
    expect(r).toEqual({added: 1, existing: 0});
  });

  it('keeps each person\'s library separate', async () => {
    const a = await makeUser();
    const b = await makeUser();
    await ingestItems(a, [{source: 'bookmark', sourceId: 'x', url: 'https://example.com/shared'}]);
    const r = await ingestItems(b, [{source: 'bookmark', sourceId: 'x', url: 'https://example.com/shared'}]);
    expect(r).toEqual({added: 1, existing: 0});
    expect((await libraryStats(a)).total).toBe(1);
  });
});

describe('listItems', () => {
  it('filters by view, source and search', async () => {
    const user = await makeUser();
    const cats = await makeCategories(user, ['Design', 'Databases']);
    await ingestItems(user, [
      {source: 'x', sourceId: '10', url: 'https://x.com/a/status/10', text: 'Partial indexes in Postgres are underrated'},
      {source: 'linkedin', sourceId: 'urn:li:activity:1', url: 'https://www.linkedin.com/feed/update/urn:li:activity:1/', text: 'Our onboarding redesign'},
      {source: 'bookmark', sourceId: 'c', url: 'https://refactoringui.com', title: 'Refactoring UI'},
    ]);
    const [design] = await query<{id: string}>(`select id from items where user_id = $1 and source_ids @> array['linkedin:urn:li:activity:1']`, [user]);
    await moveItem(user, design!.id, cats.get('Design')!);

    expect((await listItems(user, {view: 'all'})).total).toBe(3);
    expect((await listItems(user, {view: 'inbox'})).total).toBe(2);
    expect((await listItems(user, {view: cats.get('Design')!})).items.map((i) => i.body)).toEqual(['Our onboarding redesign']);
    expect((await listItems(user, {view: 'all', source: 'x'})).total).toBe(1);
    expect((await listItems(user, {view: 'all', q: 'postgres'})).items.map((i) => i.sources)).toEqual([['x']]);
    expect((await listItems(user, {view: 'all', q: 'refactoring'})).total).toBe(1);
  });

  it('does not let someone move an item into another person\'s category', async () => {
    const a = await makeUser();
    const b = await makeUser();
    const bCats = await makeCategories(b, ['Theirs']);
    await ingestItems(a, [{source: 'bookmark', sourceId: 'z', url: 'https://example.com/z'}]);
    const [item] = await query<{id: string}>('select id from items where user_id = $1', [a]);
    await moveItem(a, item!.id, bCats.get('Theirs')!);
    const [row] = await query<{category_id: string | null}>('select category_id from items where id = $1', [item!.id]);
    expect(row!.category_id).toBeNull();
  });
});

describe('search by words', () => {
  it('matches other forms of a word', async () => {
    const user = await makeUser();
    await ingestItems(user, [{source: 'x', sourceId: '1', url: 'https://x.com/i/status/1', text: 'Partial indexes beat composite ones'}]);
    expect((await listItems(user, {view: 'all', q: 'indexing'})).total).toBe(1);
    expect((await listItems(user, {view: 'all', q: 'index'})).total).toBe(1);
    expect((await listItems(user, {view: 'all', q: 'composite indexing'})).total).toBe(1);
    expect((await listItems(user, {view: 'all', q: 'postgres'})).total).toBe(0);
  });
});
