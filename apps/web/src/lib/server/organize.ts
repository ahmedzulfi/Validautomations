import 'server-only';
import {chunk} from '@stash/shared';
import {APICallError, LoadAPIKeyError, RetryError} from 'ai';
import {
  AiNoAnswerError, aiConfigured, classifyBatch, cleanCategories, openRouterModels,
  suggestTaxonomy, type Correction, type ItemForAi, type ModelPicker, type ProposedCategory,
} from './ai';
import {query, queryOne} from './db';
import {embedPending} from './search';

const BATCH_SIZE = 25;
const PARALLEL_BATCHES = 3;
const SAMPLE_SIZE = 200;
const SORT_WINDOW_MINUTES = 15;

export type Proposal = {id: string; categories: ProposedCategory[]; createdAt: string};

export async function pendingProposal(userId: string): Promise<Proposal | null> {
  return queryOne<Proposal>(
    `select id, categories, created_at as "createdAt" from taxonomy_proposals
     where user_id = $1 and status = 'pending' order by created_at desc limit 1`,
    [userId],
  );
}

/** Asks the AI for categories that fit this person's saves, and keeps them for review. */
export async function proposeTaxonomy(userId: string, models: ModelPicker = openRouterModels()): Promise<Proposal> {
  const sample = await query<ItemForAi>(
    `select sources[1] as source, title, body, author_name as "authorName", url, folder_path as "folderPath"
     from items where user_id = $1 order by random() limit ${SAMPLE_SIZE}`,
    [userId],
  );
  if (sample.length === 0) throw new Error('Import some saves first, so there is something to sort.');
  const existing = (await query<{name: string}>('select name from categories where user_id = $1 order by sort_order', [userId])).map((r) => r.name);
  const categories = await suggestTaxonomy(models, sample, existing);
  await query(`update taxonomy_proposals set status = 'dismissed' where user_id = $1 and status = 'pending'`, [userId]);
  const row = await queryOne<Proposal>(
    `insert into taxonomy_proposals (user_id, categories) values ($1, $2) returning id, categories, created_at as "createdAt"`,
    [userId, JSON.stringify(categories)],
  );
  return row!;
}

export async function dismissProposal(userId: string, proposalId: string): Promise<void> {
  await query(`update taxonomy_proposals set status = 'dismissed' where user_id = $1 and id = $2`, [userId, proposalId]);
}

/**
 * Makes the given list the person's categories, in order. Categories left out are removed and their
 * items go back to be sorted again. Items the person placed themselves stay where they are, unless their
 * category was removed.
 */
export async function applyTaxonomy(userId: string, input: ProposedCategory[], proposalId?: string | null): Promise<{created: number; removed: number}> {
  const categories = cleanCategories(input);
  if (categories.length === 0) throw new Error('Keep at least one category.');
  const existing = await query<{id: string; name: string}>('select id, name from categories where user_id = $1', [userId]);
  const keep = new Set(categories.map((c) => c.name.toLowerCase()));
  const removed = existing.filter((c) => !keep.has(c.name.toLowerCase()));
  if (removed.length) {
    await query(
      `update items set category_id = null, category_assigned_by = null, classified_at = null
       where user_id = $1 and category_id = any($2::uuid[])`,
      [userId, removed.map((c) => c.id)],
    );
    await query('delete from categories where user_id = $1 and id = any($2::uuid[])', [userId, removed.map((c) => c.id)]);
  }
  const before = existing.length - removed.length;
  for (const [i, c] of categories.entries()) {
    await query(
      `insert into categories (user_id, name, description, sort_order, is_ai_generated) values ($1, $2, $3, $4, $5)
       on conflict (user_id, lower(name)) do update set name = excluded.name, description = excluded.description, sort_order = excluded.sort_order`,
      [userId, c.name, c.description, i, Boolean(proposalId)],
    );
  }
  // The AI sorts everything it placed again against the new list. The person's own choices stay.
  await query(
    `update items set classified_at = null
     where user_id = $1 and category_assigned_by is distinct from 'user'`,
    [userId],
  );
  if (proposalId) await query(`update taxonomy_proposals set status = 'applied' where user_id = $1 and id = $2`, [userId, proposalId]);
  return {created: categories.length - before, removed: removed.length};
}

export async function renameCategory(userId: string, id: string, name: string, description: string): Promise<void> {
  const clean = cleanCategories([{name, description}])[0];
  if (!clean) throw new Error('Give the category a name.');
  await query('update categories set name = $3, description = $4 where user_id = $1 and id = $2', [userId, id, clean.name, clean.description]);
}

export async function deleteCategory(userId: string, id: string): Promise<void> {
  await query(
    `update items set category_id = null, category_assigned_by = null, classified_at = null where user_id = $1 and category_id = $2`,
    [userId, id],
  );
  await query('delete from categories where user_id = $1 and id = $2', [userId, id]);
}

export type SortingState = {running: boolean; error: string | null};

export async function sortingState(userId: string): Promise<SortingState> {
  const row = await queryOne<{running: boolean; error: string | null}>(
    'select coalesce(sorting_until > now(), false) as running, sorting_error as error from users where id = $1',
    [userId],
  );
  return row ?? {running: false, error: null};
}

/** Claims the sorting job for this person, so two requests never sort the same items at once. */
async function claimSorting(userId: string): Promise<boolean> {
  const row = await queryOne(
    `update users set sorting_until = now() + interval '${SORT_WINDOW_MINUTES} minutes', sorting_error = null
     where id = $1 and (sorting_until is null or sorting_until < now()) returning id`,
    [userId],
  );
  return Boolean(row);
}

async function releaseSorting(userId: string, error: string | null): Promise<void> {
  await query('update users set sorting_until = null, sorting_error = $2 where id = $1', [userId, error]);
}

export type SortResult = {processed: number; placed: number; inbox: number; error: string | null; skipped?: 'busy' | 'no-categories' | 'no-ai'};

type PendingItem = ItemForAi & {id: string};

/**
 * Sorts every item that has not been sorted yet into the person's categories. Safe to call often: it
 * does nothing if a sort is already running for this person or there is nothing to sort.
 */
export async function sortPending(userId: string, models?: ModelPicker, maxItems = 5000): Promise<SortResult> {
  if (!models && !aiConfigured()) return {processed: 0, placed: 0, inbox: 0, error: null, skipped: 'no-ai'};
  const categories = await query<{id: string; name: string; description: string}>(
    'select id, name, description from categories where user_id = $1 order by sort_order',
    [userId],
  );
  if (categories.length === 0) return {processed: 0, placed: 0, inbox: 0, error: null, skipped: 'no-categories'};
  if (!(await claimSorting(userId))) return {processed: 0, placed: 0, inbox: 0, error: null, skipped: 'busy'};

  const ai = models ?? openRouterModels();
  const idByName = new Map(categories.map((c) => [c.name, c.id]));
  const result: SortResult = {processed: 0, placed: 0, inbox: 0, error: null};
  try {
    const corrections = (await query<ItemForAi & {category: string}>(
      `select i.sources[1] as source, i.title, i.body, i.author_name as "authorName", i.url, i.folder_path as "folderPath", c.name as category
       from items i join categories c on c.id = i.category_id
       where i.user_id = $1 and i.category_assigned_by = 'user'
       order by i.category_assigned_at desc nulls last limit 12`,
      [userId],
    )).map(({category, ...item}): Correction => ({item, category}));

    while (result.processed < maxItems) {
      const pending = await query<PendingItem>(
        `select id, sources[1] as source, title, body, author_name as "authorName", url, folder_path as "folderPath"
         from items where user_id = $1 and classified_at is null
         order by coalesce(saved_at, imported_at) desc limit ${BATCH_SIZE * PARALLEL_BATCHES}`,
        [userId],
      );
      if (pending.length === 0) break;
      const batches = chunk(pending, BATCH_SIZE);
      const outcomes = await Promise.allSettled(batches.map((batch) => classifyBatch(ai, categories, batch, corrections)));
      let fatal: unknown = null;
      for (const [i, outcome] of outcomes.entries()) {
        const batch = batches[i]!;
        if (outcome.status === 'rejected') {
          const reason = outcome.reason;
          if (reason instanceof AiNoAnswerError) {
            // Leave these for the person to place, rather than retrying them forever.
            await saveResults(userId, batch.map((item) => ({id: item.id, categoryId: null, confidence: 0, tags: [], summary: null})));
            result.processed += batch.length;
            result.inbox += batch.length;
          } else {
            fatal = reason;
          }
          continue;
        }
        const rows = batch.map((item, j) => {
          const c = outcome.value[j]!;
          return {id: item.id, categoryId: c.category ? idByName.get(c.category) ?? null : null, confidence: c.confidence, tags: c.tags, summary: c.summary || null};
        });
        await saveResults(userId, rows);
        result.processed += rows.length;
        result.placed += rows.filter((r) => r.categoryId).length;
        result.inbox += rows.filter((r) => !r.categoryId).length;
      }
      if (fatal) throw fatal;
      await query(`update users set sorting_until = now() + interval '${SORT_WINDOW_MINUTES} minutes' where id = $1`, [userId]);
    }
    await releaseSorting(userId, null);
  } catch (err) {
    result.error = describeAiError(err);
    await releaseSorting(userId, result.error);
  }
  return result;
}

async function saveResults(userId: string, rows: Array<{id: string; categoryId: string | null; confidence: number; tags: string[]; summary: string | null}>): Promise<void> {
  await query(
    `update items i set
       category_id = case when i.category_assigned_by = 'user' then i.category_id else r.category_id end,
       category_assigned_by = case when i.category_assigned_by = 'user' then 'user' when r.category_id is null then null else 'ai' end,
       category_confidence = r.confidence,
       category_assigned_at = now(),
       tags = coalesce(r.tags, '{}'),
       summary = coalesce(r.summary, i.summary),
       classified_at = now()
     from jsonb_to_recordset($2::jsonb) as r(id uuid, category_id uuid, confidence real, tags text[], summary text)
     where i.user_id = $1 and i.id = r.id`,
    [userId, JSON.stringify(rows.map((r) => ({id: r.id, category_id: r.categoryId, confidence: r.confidence, tags: r.tags, summary: r.summary})))],
  );
}

export function describeAiError(err: unknown): string {
  const cause = RetryError.isInstance(err) ? err.lastError : err;
  if (LoadAPIKeyError.isInstance(cause)) return 'Set OPENROUTER_API_KEY on the server to use AI sorting.';
  if (APICallError.isInstance(cause)) {
    const status = cause.statusCode;
    if (status === 401 || status === 403) return 'The OpenRouter API key was not accepted. Check OPENROUTER_API_KEY.';
    if (status === 402) return 'The OpenRouter account is out of credits. Add credits at openrouter.ai and try again.';
    if (status === 404) return 'OpenRouter does not have that model. Check STASH_TAXONOMY_MODEL and STASH_CLASSIFY_MODEL.';
    if (status === 429) return 'The AI is rate limited right now. Try sorting again in a minute.';
    if (status === undefined) return 'Could not reach OpenRouter. Check the server\'s network and try again.';
    return `OpenRouter returned an error (${status}). Try again.`;
  }
  if (cause instanceof Error) return cause.message;
  return 'Sorting stopped for an unknown reason. Try again.';
}

/** Everything that happens to new saves after they arrive: sort them, then make them searchable by meaning. */
export async function organizeInBackground(userId: string): Promise<void> {
  await sortPending(userId);
  await embedPending(userId);
}
