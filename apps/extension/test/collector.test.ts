import type {ItemInput} from '@stash/shared';
import {describe, expect, it} from 'vitest';
import {Collector} from '../utils/collector';

const item = (id: number): ItemInput => ({source: 'x', sourceId: String(id), url: `https://x.com/i/status/${id}`});

/** A fake saved list: each loadMore() makes the page deliver the next page of items. */
function setup(pages: number[][], known: Set<string> = new Set(), opts: {loggedOutAfter?: number} = {}) {
  const sent: string[][] = [];
  let page = 0;
  let loads = 0;
  const collector: Collector = new Collector({
    parse: (data) => (data as number[]).map(item),
    send: async (items) => {
      sent.push(items.map((i) => i.sourceId));
      const added = items.filter((i) => !known.has(i.sourceId)).length;
      items.forEach((i) => known.add(i.sourceId));
      return {added, existing: items.length - added};
    },
    loadMore: () => {
      loads++;
      if (page < pages.length) collector.accept(pages[page++]);
    },
    loggedOutReason: () => (opts.loggedOutAfter !== undefined && loads >= opts.loggedOutAfter ? 'Log in first.' : null),
    sleep: async () => {},
    roundDelayMs: 0,
    maxIdleRounds: 3,
  });
  return {collector, sent, loads: () => loads};
}

describe('Collector', () => {
  it('collects every page until the list stops growing', async () => {
    const {collector, sent} = setup([[1, 2], [3, 4], [5]]);
    expect(await collector.run(false)).toEqual({ok: true, found: 5, added: 5});
    expect(sent).toEqual([['1', '2'], ['3', '4'], ['5']]);
  });

  it('sends what the page loaded before the run started', async () => {
    const {collector, sent} = setup([]);
    collector.accept([7, 8]);
    expect(await collector.run(false)).toEqual({ok: true, found: 2, added: 2});
    expect(sent[0]).toEqual(['7', '8']);
  });

  it('never sends the same item twice', async () => {
    const {collector} = setup([[1, 2], [2, 3], [3]]);
    expect(await collector.run(false)).toMatchObject({found: 3});
  });

  it('stops an incremental sync at the first batch that is all known', async () => {
    const {collector, sent, loads} = setup([[1, 2], [3, 4], [5, 6]], new Set(['3', '4', '5', '6']));
    expect(await collector.run(true)).toEqual({ok: true, found: 4, added: 2});
    expect(sent).toEqual([['1', '2'], ['3', '4']]);
    expect(loads()).toBe(2);
  });

  it('reports when the person is logged out', async () => {
    const {collector} = setup([[1], [2], [3]], new Set(), {loggedOutAfter: 2});
    expect(await collector.run(false)).toEqual({ok: false, error: 'Log in first.', found: 1, added: 1});
  });

  it('reports a server error with what it sent so far', async () => {
    const collector = new Collector({
      parse: (d) => (d as number[]).map(item),
      send: async () => {
        throw new Error('The server answered 500.');
      },
      loadMore: () => {},
      loggedOutReason: () => null,
      sleep: async () => {},
    });
    collector.accept([1]);
    expect(await collector.run(false)).toEqual({ok: false, error: 'The server answered 500.', found: 0, added: 0});
  });
});
