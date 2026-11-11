import 'server-only';
import {canonicalUrl, type IngestResponse, type ItemInput, type Media, type Source} from '@stash/shared';
import {query, queryOne} from './db';

export type Item = {
  id: string;
  sources: Source[];
  url: string;
  title: string | null;
  body: string | null;
  authorName: string | null;
  authorHandle: string | null;
  authorAvatar: string | null;
  media: Media[] | null;
  links: string[] | null;
  folderPath: string[] | null;
  savedAt: string | null;
  postedAt: string | null;
  importedAt: string;
  categoryId: string | null;
  categoryAssignedBy: 'ai' | 'user' | null;
  tags: string[];
  summary: string | null;
  classifiedAt: string | null;
};

export type Category = {
  id: string;
  name: string;
  description: string;
  sortOrder: number;
  count: number;
};

const ITEM_COLUMNS = `
  id, sources, url, title, body, author_name as "authorName", author_handle as "authorHandle",
  author_avatar as "authorAvatar", media, links, folder_path as "folderPath",
  saved_at as "savedAt", posted_at as "postedAt", imported_at as "importedAt",
  category_id as "categoryId", category_assigned_by as "categoryAssignedBy", tags, summary,
  classified_at as "classifiedAt"`;

/**
 * Adds saved things to a person's library. The same link saved twice (or from two sources) becomes one
 * item: its sources are merged and fields it was missing are filled, and nothing already there is lost.
 */
export async function ingestItems(userId: string, items: ItemInput[]): Promise<IngestResponse> {
  const byUrl = new Map<string, ItemInput & {canonical: string}>();
  for (const item of items) {
    const canonical = canonicalUrl(item.url);
    if (!byUrl.has(canonical)) byUrl.set(canonical, {...item, canonical});
  }
  if (byUrl.size === 0) return {added: 0, existing: 0};

  const rows = [...byUrl.values()].map((i) => ({
    source: i.source,
    source_id: `${i.source}:${i.sourceId}`,
    canonical_url: i.canonical,
    url: i.url,
    title: i.title ?? null,
    body: i.text ?? null,
    author_name: i.authorName ?? null,
    author_handle: i.authorHandle ?? null,
    author_avatar: i.authorAvatar ?? null,
    media: i.media?.length ? i.media : null,
    links: i.links?.length ? i.links : null,
    folder_path: i.folderPath?.length ? i.folderPath : null,
    saved_at: i.savedAt ?? null,
    posted_at: i.postedAt ?? null,
  }));

  const result = await query<{inserted: boolean}>(
    `with input as (
       select * from jsonb_to_recordset($2::jsonb) as x(
         source text, source_id text, canonical_url text, url text, title text, body text,
         author_name text, author_handle text, author_avatar text, media jsonb, links jsonb,
         folder_path text[], saved_at timestamptz, posted_at timestamptz)
     )
     insert into items as i (user_id, sources, source_ids, canonical_url, url, title, body, author_name,
       author_handle, author_avatar, media, links, folder_path, saved_at, posted_at)
     select $1, array[source], array[source_id], canonical_url, url, title, body, author_name,
       author_handle, author_avatar, media, links, folder_path, saved_at, posted_at
     from input
     on conflict (user_id, canonical_url) do update set
       sources = (select array_agg(distinct s order by s) from unnest(i.sources || excluded.sources) s),
       source_ids = (select array_agg(distinct s order by s) from unnest(i.source_ids || excluded.source_ids) s),
       title = coalesce(i.title, excluded.title),
       body = case when length(coalesce(excluded.body, '')) > length(coalesce(i.body, '')) then excluded.body else i.body end,
       author_name = coalesce(i.author_name, excluded.author_name),
       author_handle = coalesce(i.author_handle, excluded.author_handle),
       author_avatar = coalesce(excluded.author_avatar, i.author_avatar),
       media = coalesce(i.media, excluded.media),
       links = coalesce(i.links, excluded.links),
       folder_path = coalesce(i.folder_path, excluded.folder_path),
       saved_at = least(i.saved_at, excluded.saved_at),
       posted_at = coalesce(i.posted_at, excluded.posted_at)
     returning (xmax = 0) as inserted`,
    [userId, JSON.stringify(rows)],
  );
  const added = result.filter((r) => r.inserted).length;
  return {added, existing: result.length - added};
}

export type ItemFilter = {
  /** A category id, 'inbox' for items the AI could not place, or 'all'. */
  view: string;
  source?: Source | null;
  q?: string | null;
  page?: number;
};

export const PAGE_SIZE = 48;

export async function listItems(userId: string, filter: ItemFilter): Promise<{items: Item[]; total: number}> {
  const where: string[] = ['user_id = $1'];
  const params: unknown[] = [userId];
  const add = (sql: string, value: unknown) => {
    params.push(value);
    where.push(sql.replace('?', `$${params.length}`));
  };
  if (filter.view === 'inbox') where.push('category_id is null');
  else if (filter.view !== 'all') add('category_id = ?', filter.view);
  if (filter.source) add('? = any(sources)', filter.source);
  const q = filter.q?.trim();
  let order = 'coalesce(saved_at, imported_at) desc, id';
  if (q) {
    add(`(search @@ websearch_to_tsquery('simple', ?) or ? = any(tags))`, q);
    // The second placeholder reuses the same text.
    where[where.length - 1] = where[where.length - 1]!.replace('?', `$${params.length}`);
    order = `ts_rank(search, websearch_to_tsquery('simple', $${params.length})) desc, ${order}`;
  }
  const page = Math.max(1, filter.page ?? 1);
  const whereSql = where.join(' and ');
  const [items, count] = await Promise.all([
    query<Item>(`select ${ITEM_COLUMNS} from items where ${whereSql} order by ${order} limit ${PAGE_SIZE} offset ${(page - 1) * PAGE_SIZE}`, params),
    queryOne<{total: number}>(`select count(*)::int as total from items where ${whereSql}`, params),
  ]);
  return {items, total: count?.total ?? 0};
}

export async function listCategories(userId: string): Promise<Category[]> {
  return query<Category>(
    `select c.id, c.name, c.description, c.sort_order as "sortOrder", count(i.id)::int as count
     from categories c left join items i on i.category_id = c.id
     where c.user_id = $1
     group by c.id
     order by c.sort_order, lower(c.name)`,
    [userId],
  );
}

export type LibraryStats = {total: number; inbox: number; unclassified: number; bySource: Partial<Record<Source, number>>};

export async function libraryStats(userId: string): Promise<LibraryStats> {
  const row = await queryOne<{total: number; inbox: number; unclassified: number}>(
    `select count(*)::int as total,
            count(*) filter (where category_id is null)::int as inbox,
            count(*) filter (where classified_at is null)::int as unclassified
     from items where user_id = $1`,
    [userId],
  );
  const sources = await query<{source: Source; count: number}>(
    `select s as source, count(*)::int as count from items, unnest(sources) s where user_id = $1 group by s`,
    [userId],
  );
  return {
    total: row?.total ?? 0,
    inbox: row?.inbox ?? 0,
    unclassified: row?.unclassified ?? 0,
    bySource: Object.fromEntries(sources.map((s) => [s.source, s.count])),
  };
}

/** The person moved an item. Their choice is final: the AI will not move it again. */
export async function moveItem(userId: string, itemId: string, categoryId: string | null): Promise<void> {
  await query(
    `update items set category_id = $3, category_assigned_by = 'user', category_assigned_at = now(),
       category_confidence = null, classified_at = coalesce(classified_at, now())
     where user_id = $1 and id = $2
       and ($3::uuid is null or exists (select 1 from categories where id = $3 and user_id = $1))`,
    [userId, itemId, categoryId],
  );
}

export async function deleteItem(userId: string, itemId: string): Promise<void> {
  await query('delete from items where user_id = $1 and id = $2', [userId, itemId]);
}

export async function recordSyncRun(userId: string, run: {source: Source; itemsSeen: number; itemsAdded: number; error?: string | null}): Promise<void> {
  await query('insert into sync_runs (user_id, source, items_seen, items_added, error) values ($1, $2, $3, $4, $5)', [
    userId, run.source, run.itemsSeen, run.itemsAdded, run.error ?? null,
  ]);
}

export type SyncRun = {source: Source; itemsSeen: number; itemsAdded: number; error: string | null; finishedAt: string};

export async function lastSyncRuns(userId: string): Promise<SyncRun[]> {
  return query<SyncRun>(
    `select distinct on (source) source, items_seen as "itemsSeen", items_added as "itemsAdded", error, finished_at as "finishedAt"
     from sync_runs where user_id = $1 order by source, finished_at desc`,
    [userId],
  );
}

export async function exportItems(userId: string): Promise<Array<Item & {category: string | null}>> {
  return query<Item & {category: string | null}>(
    `select ${ITEM_COLUMNS.replace(/\bid\b,/, 'i.id,')}, c.name as category
     from items i left join categories c on c.id = i.category_id
     where i.user_id = $1 order by coalesce(i.saved_at, i.imported_at) desc`,
    [userId],
  );
}
