import 'server-only';
import {chunk, type Source} from '@stash/shared';
import {
  aiConfigured, answerFromSaves, embeddingText, openRouterEmbedder, openRouterModels, toVectorLiteral,
  type Embedder, type ItemForAi, type ModelPicker,
} from './ai';
import {query, queryOne} from './db';
import {ITEM_COLUMNS, type Item} from './items';

const EMBED_BATCH = 64;

/**
 * Embeds every item that has no vector yet, so search by meaning can find it. Runs after sorting, so the
 * AI's summary and tags are part of what is embedded. Stops quietly on an error; the next run retries.
 */
export async function embedPending(userId: string, embedder?: Embedder, maxItems = 5000): Promise<{embedded: number; error: string | null}> {
  if (!embedder && !aiConfigured()) return {embedded: 0, error: null};
  const embed = embedder ?? openRouterEmbedder();
  let embedded = 0;
  try {
    while (embedded < maxItems) {
      const rows = await query<ItemForAi & {id: string; summary: string | null; tags: string[]}>(
        `select id, sources[1] as source, title, body, author_name as "authorName", url, folder_path as "folderPath", summary, tags
         from items where user_id = $1 and embedded_at is null
         order by coalesce(saved_at, imported_at) desc limit ${EMBED_BATCH * 4}`,
        [userId],
      );
      if (rows.length === 0) break;
      for (const batch of chunk(rows, EMBED_BATCH)) {
        const vectors = await embed(batch.map(embeddingText));
        await query(
          `update items i set embedding = r.embedding::vector, embedded_at = now()
           from jsonb_to_recordset($2::jsonb) as r(id uuid, embedding text)
           where i.user_id = $1 and i.id = r.id`,
          [userId, JSON.stringify(batch.map((item, j) => ({id: item.id, embedding: toVectorLiteral(vectors[j]!)})))],
        );
        embedded += batch.length;
      }
    }
    return {embedded, error: null};
  } catch (err) {
    return {embedded, error: err instanceof Error ? err.message : 'Embedding stopped.'};
  }
}

export type MeaningResult = Item & {similarity: number};

/** The saves closest in meaning to a question or phrase, best first. */
export async function searchByMeaning(
  userId: string,
  text: string,
  options: {view?: string; source?: Source | null; limit?: number; embedder?: Embedder} = {},
): Promise<MeaningResult[]> {
  const embed = options.embedder ?? openRouterEmbedder();
  const [vector] = await embed([text]);
  const params: unknown[] = [userId, toVectorLiteral(vector!)];
  const where = ['user_id = $1', 'embedding is not null'];
  if (options.view === 'inbox') where.push('category_id is null');
  else if (options.view === 'broken') where.push(`link_status = 'broken'`);
  else if (options.view && options.view !== 'all') {
    params.push(options.view);
    where.push(`category_id = $${params.length}`);
  }
  if (options.source) {
    params.push(options.source);
    where.push(`$${params.length} = any(sources)`);
  }
  return query<MeaningResult>(
    `select ${ITEM_COLUMNS}, 1 - (embedding <=> $2::vector) as similarity
     from items where ${where.join(' and ')}
     order by embedding <=> $2::vector
     limit ${Math.min(100, options.limit ?? 48)}`,
    params,
  );
}

export async function embeddingCoverage(userId: string): Promise<{embedded: number; total: number}> {
  const row = await queryOne<{embedded: number; total: number}>(
    'select count(*) filter (where embedded_at is not null)::int as embedded, count(*)::int as total from items where user_id = $1',
    [userId],
  );
  return row ?? {embedded: 0, total: 0};
}

export type Answer = {answer: string; sources: MeaningResult[]};

/** Answers a question from the person's own saves, with the saves it used. */
export async function askLibrary(userId: string, question: string, deps: {models?: ModelPicker; embedder?: Embedder} = {}): Promise<Answer> {
  const nearest = await searchByMeaning(userId, question, {limit: 12, embedder: deps.embedder});
  if (nearest.length === 0) return {answer: 'There is nothing to search yet. Import some saves first.', sources: []};
  const saves = nearest.map((i) => ({source: i.sources[0] ?? 'web', title: i.title, body: i.body, authorName: i.authorName, url: i.url, folderPath: i.folderPath}));
  const {answer, cited} = await answerFromSaves(deps.models ?? openRouterModels(), question, saves);
  // Number the cited saves 1, 2, 3 in the order shown, and rewrite the answer's citations to match.
  const order = cited.length ? cited : [];
  const renumber = new Map(order.map((n, i) => [n, i + 1]));
  const text = answer.replace(/\[(\d+)\]/g, (m, n: string) => (renumber.has(Number(n)) ? `[${renumber.get(Number(n))}]` : ''));
  return {answer: text.replace(/\s+([.,;:])/g, '$1').trim(), sources: order.map((n) => nearest[n - 1]!)};
}
