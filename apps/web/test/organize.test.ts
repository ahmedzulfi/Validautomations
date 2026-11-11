import {APICallError} from 'ai';
import {describe, expect, it} from 'vitest';
import {classifyBatch, MIN_CONFIDENCE} from '@/lib/server/ai';
import {ingestItems, moveItem} from '@/lib/server/items';
import {applyTaxonomy, proposeTaxonomy, sortingState, sortPending} from '@/lib/server/organize';
import {fakeModels, itemNumbers, makeCategories, makeUser, query} from './helpers';

const item = (text: string) => ({source: 'x', title: text, body: text, authorName: null, url: 'https://x.com/i/status/1', folderPath: null});

describe('classifyBatch', () => {
  it('returns one result per item in order, sends low confidence to the inbox and cleans tags', async () => {
    const {models, calls} = fakeModels(() => ({
      output: {results: [
        {n: 2, category: 'Databases', confidence: 0.95, tags: ['#Postgres', 'postgres', 'Indexes'], summary: 'Indexes'},
        {n: 1, category: 'Design', confidence: MIN_CONFIDENCE - 0.1, tags: [], summary: 'Unsure'},
      ]},
    }));
    const results = await classifyBatch(models, [{name: 'Design', description: ''}, {name: 'Databases', description: ''}], [item('a'), item('b'), item('c')]);
    expect(results).toEqual([
      {category: null, confidence: MIN_CONFIDENCE - 0.1, tags: [], summary: 'Unsure'},
      {category: 'Databases', confidence: 0.95, tags: ['postgres', 'indexes'], summary: 'Indexes'},
      {category: null, confidence: 0, tags: [], summary: ''},
    ]);
    expect(calls[0]!.modelId).toBe('anthropic/claude-haiku-5.5');
  });
});

describe('sortPending', () => {
  async function setup() {
    const user = await makeUser();
    const cats = await makeCategories(user, ['Design', 'Databases']);
    await ingestItems(user, Array.from({length: 30}, (_, i) => ({
      source: 'x' as const,
      sourceId: String(i),
      url: `https://x.com/a/status/${i}`,
      text: i % 2 ? `Postgres tip ${i}` : `Typography tip ${i}`,
    })));
    return {user, cats};
  }

  const byText = fakeModels((call) => {
    const blocks = call.prompt.split('<item n="').slice(1);
    return {
      output: {results: itemNumbers(call).map((n, i) => ({
        n,
        category: blocks[i]!.includes('Postgres') ? 'Databases' : 'Design',
        confidence: 0.9,
        tags: ['tip'],
        summary: 'A tip',
      }))},
    };
  });

  it('sorts every unsorted item in batches and keeps the person\'s own choices', async () => {
    const {user, cats} = await setup();
    const [first] = await query<{id: string}>(`select id from items where user_id = $1 and body = 'Postgres tip 1'`, [user]);
    await moveItem(user, first!.id, cats.get('Design')!);

    const result = await sortPending(user, byText.models);
    expect(result).toMatchObject({processed: 29, placed: 29, inbox: 0, error: null});

    const counts = await query<{name: string; n: number}>(
      `select c.name, count(*)::int as n from items i join categories c on c.id = i.category_id where i.user_id = $1 group by c.name order by c.name`,
      [user],
    );
    expect(counts).toEqual([{name: 'Databases', n: 14}, {name: 'Design', n: 16}]);
    const [kept] = await query<{category_assigned_by: string}>('select category_assigned_by from items where id = $1', [first!.id]);
    expect(kept!.category_assigned_by).toBe('user');
    expect(await sortingState(user)).toEqual({running: false, error: null});

    // Nothing left: a second run does no work.
    expect((await sortPending(user, byText.models)).processed).toBe(0);
  });

  it('shows the person\'s corrections to the model as examples', async () => {
    const {user, cats} = await setup();
    const [first] = await query<{id: string}>(`select id from items where user_id = $1 and body = 'Postgres tip 1'`, [user]);
    await moveItem(user, first!.id, cats.get('Design')!);
    await sortPending(user, byText.models);
    expect(byText.calls.at(-1)!.system).toContain('<example category="Design">');
  });

  it('puts a refused batch in the inbox instead of retrying it forever', async () => {
    const {user} = await setup();
    const {models} = fakeModels(() => ({output: {}, finishReason: 'content-filter'}));
    const result = await sortPending(user, models);
    expect(result).toMatchObject({processed: 30, placed: 0, inbox: 30, error: null});
  });

  it('stops on an API error, keeps the rest unsorted and says why', async () => {
    const {user} = await setup();
    const {models} = fakeModels(() => {
      throw new APICallError({message: 'bad key', url: 'https://openrouter.ai/api/v1/chat/completions', requestBodyValues: {}, statusCode: 401, isRetryable: false});
    });
    const result = await sortPending(user, models);
    expect(result.error).toMatch(/OpenRouter API key was not accepted/);
    expect(await sortingState(user)).toEqual({running: false, error: result.error});
    const [{n}] = await query<{n: number}>('select count(*)::int as n from items where user_id = $1 and classified_at is null', [user]) as [{n: number}];
    expect(n).toBe(30);
  });

  it('does nothing without categories', async () => {
    const user = await makeUser();
    await ingestItems(user, [{source: 'bookmark', sourceId: 'a', url: 'https://example.com/a'}]);
    expect((await sortPending(user, byText.models)).skipped).toBe('no-categories');
  });
});

describe('taxonomy', () => {
  it('proposes categories from a sample and applies the edited list', async () => {
    const user = await makeUser();
    await ingestItems(user, [{source: 'bookmark', sourceId: 'a', url: 'https://example.com/a', title: 'Grid systems'}]);
    const {models, calls} = fakeModels(() => ({
      output: {categories: [
        {name: 'Design systems', description: 'Tokens and components'},
        {name: 'design systems', description: 'Duplicate'},
        {name: 'Other', description: 'Should be dropped'},
      ]},
    }));
    const proposal = await proposeTaxonomy(user, models);
    expect(proposal.categories).toEqual([{name: 'Design systems', description: 'Tokens and components'}]);
    expect(calls[0]!.modelId).toBe('anthropic/claude-opus-5.5');

    await applyTaxonomy(user, [{name: 'Design systems', description: 'Tokens'}, {name: 'Hiring', description: 'People'}], proposal.id);
    const names = await query<{name: string}>('select name from categories where user_id = $1 order by sort_order', [user]);
    expect(names.map((n) => n.name)).toEqual(['Design systems', 'Hiring']);
    const [p] = await query<{status: string}>('select status from taxonomy_proposals where id = $1', [proposal.id]);
    expect(p!.status).toBe('applied');
  });

  it('removing a category sends its items back to be sorted', async () => {
    const user = await makeUser();
    const cats = await makeCategories(user, ['Keep', 'Drop']);
    await ingestItems(user, [{source: 'bookmark', sourceId: 'a', url: 'https://example.com/a'}]);
    await query('update items set category_id = $2, category_assigned_by = $3, classified_at = now() where user_id = $1', [user, cats.get('Drop'), 'user']);
    const r = await applyTaxonomy(user, [{name: 'Keep', description: ''}]);
    expect(r.removed).toBe(1);
    const [row] = await query<{category_id: string | null; classified_at: string | null}>('select category_id, classified_at from items where user_id = $1', [user]);
    expect(row).toEqual({category_id: null, classified_at: null});
  });
});
