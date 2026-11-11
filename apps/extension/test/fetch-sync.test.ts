import type {ItemInput} from '@stash/shared';
import {describe, expect, it} from 'vitest';
import {LoggedOutError, pageThrough} from '../utils/fetch-sync';

const item = (id: string): ItemInput => ({source: 'reddit', sourceId: id, url: `https://www.reddit.com/r/x/comments/${id}/`});
const PAGES: Record<string, {items: ItemInput[]; next: string | null}> = {
  start: {items: [item('a'), item('b')], next: 'p2'},
  p2: {items: [item('c')], next: 'p3'},
  p3: {items: [item('d')], next: null},
};

function setup(known: string[] = []) {
  const seen = new Set(known);
  const fetched: Array<string | null> = [];
  return {
    fetched,
    options: {
      fetchPage: async (cursor: string | null) => {
        fetched.push(cursor);
        return PAGES[cursor ?? 'start']!;
      },
      send: async (items: ItemInput[]) => {
        const added = items.filter((i) => !seen.has(i.sourceId)).length;
        items.forEach((i) => seen.add(i.sourceId));
        return {added, existing: items.length - added};
      },
      sleep: async () => {},
    },
  };
}

describe('pageThrough', () => {
  it('follows the cursor to the end', async () => {
    const {options, fetched} = setup();
    expect(await pageThrough({...options, incremental: false})).toEqual({ok: true, found: 4, added: 4});
    expect(fetched).toEqual([null, 'p2', 'p3']);
  });

  it('stops an incremental sync at the first page that is all known', async () => {
    const {options, fetched} = setup(['c', 'd']);
    expect(await pageThrough({...options, incremental: true})).toEqual({ok: true, found: 3, added: 2});
    expect(fetched).toEqual([null, 'p2']);
  });

  it('reports being logged out', async () => {
    const r = await pageThrough({
      fetchPage: async () => {
        throw new LoggedOutError('Log in to Reddit in this browser, then sync again.');
      },
      send: async () => ({added: 0, existing: 0}),
      incremental: false,
      sleep: async () => {},
    });
    expect(r).toEqual({ok: false, error: 'Log in to Reddit in this browser, then sync again.', found: 0, added: 0});
  });
});
