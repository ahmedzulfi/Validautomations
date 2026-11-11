import {describe, expect, it} from 'vitest';
import {EMBEDDING_DIMENSIONS, type Embedder} from '@/lib/server/ai';
import {ingestItems} from '@/lib/server/items';
import {askLibrary, embedPending, embeddingCoverage, searchByMeaning} from '@/lib/server/search';
import {fakeModels, makeUser, query} from './helpers';

/** A stand-in embedding: words hashed into buckets, so texts sharing words point the same way. */
const bagOfWords: Embedder = async (values) => values.map((text) => {
  const v = new Array<number>(EMBEDDING_DIMENSIONS).fill(0);
  for (const word of text.toLowerCase().match(/[a-z]{3,}/g) ?? []) {
    let h = 0;
    for (const ch of word) h = (h * 31 + ch.charCodeAt(0)) % EMBEDDING_DIMENSIONS;
    v[h] += 1;
  }
  return v;
});

async function seeded() {
  const user = await makeUser();
  await ingestItems(user, [
    {source: 'x', sourceId: '1', url: 'https://x.com/i/status/1', text: 'Postgres partial indexes beat composite indexes for sparse queries'},
    {source: 'x', sourceId: '2', url: 'https://x.com/i/status/2', text: 'Pricing page experiment: three plans convert better than four'},
    {source: 'linkedin', sourceId: 'urn:li:activity:3', url: 'https://www.linkedin.com/feed/update/urn:li:activity:3/', text: 'Hiring engineers: one take-home, one pairing session'},
    {source: 'bookmark', sourceId: 'b', url: 'https://example.com/weather', title: 'Weather forecast'},
  ]);
  return user;
}

describe('search by meaning', () => {
  it('embeds every item once, then finds the closest', async () => {
    const user = await seeded();
    expect(await embedPending(user, bagOfWords)).toEqual({embedded: 4, error: null});
    expect(await embedPending(user, bagOfWords)).toEqual({embedded: 0, error: null});
    expect(await embeddingCoverage(user)).toEqual({embedded: 4, total: 4});

    const results = await searchByMeaning(user, 'which postgres indexes', {embedder: bagOfWords});
    expect(results[0]!.body).toContain('Postgres partial indexes');
    expect(results[0]!.similarity).toBeGreaterThan(results[1]!.similarity);

    const linkedin = await searchByMeaning(user, 'postgres indexes', {embedder: bagOfWords, source: 'linkedin'});
    expect(linkedin.map((r) => r.sources)).toEqual([['linkedin']]);
  });

  it('stops quietly and keeps the rest for later when embedding fails', async () => {
    const user = await seeded();
    const failing: Embedder = async () => {
      throw new Error('OpenRouter is down');
    };
    expect(await embedPending(user, failing)).toEqual({embedded: 0, error: 'OpenRouter is down'});
    const [{n}] = await query<{n: number}>('select count(*)::int as n from items where user_id = $1 and embedded_at is null', [user]) as [{n: number}];
    expect(n).toBe(4);
  });
});

describe('askLibrary', () => {
  it('answers from the closest saves and renumbers the citations to the saves it shows', async () => {
    const user = await seeded();
    await embedPending(user, bagOfWords);
    const {models, calls} = fakeModels((call) => {
      const blocks = call.system.split('<save n="').slice(1);
      const n = blocks.findIndex((b) => b.includes('Pricing page')) + 1;
      return {output: {answer: `Three plans beat four [${n}].`, cited: [n, 99]}};
    });
    const result = await askLibrary(user, 'what did I save about pricing plans', {models, embedder: bagOfWords});
    expect(result.answer).toBe('Three plans beat four [1].');
    expect(result.sources.map((s) => s.body)).toEqual(['Pricing page experiment: three plans convert better than four']);
    expect(calls[0]!.modelId).toBe('anthropic/claude-opus-5.5');
  });

  it('says so when there is nothing to search', async () => {
    const user = await makeUser();
    const {models} = fakeModels(() => ({output: {answer: '', cited: []}}));
    expect((await askLibrary(user, 'anything', {models, embedder: bagOfWords})).sources).toEqual([]);
  });
});
