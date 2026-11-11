import {SOURCE_LABELS, SOURCES, type Source} from '@stash/shared';
import {CategoryNav} from '@/components/app/category-nav';
import {ItemGrid} from '@/components/app/item-grid';
import {LibraryActions} from '@/components/app/library-actions';
import {SearchBox, SourceFilter} from '@/components/app/library-filters';
import {LinkButton} from '@/components/app/link-button';
import {SortingStatus} from '@/components/app/sorting-status';
import {Empty, EmptyActions, EmptyDescription, EmptyTitle} from '@/components/lime/ui/empty';
import {toCardData} from '@/lib/card-data';
import {aiConfigured} from '@/lib/server/ai';
import {requireUser} from '@/lib/server/auth';
import {libraryStats, listCategories, listItems, PAGE_SIZE, resurfaceItems, type Item} from '@/lib/server/items';
import {describeAiError, sortingState} from '@/lib/server/organize';
import {searchByMeaning} from '@/lib/server/search';

export const metadata = {title: 'Library · Stash'};

type SearchParams = Promise<Record<string, string | string[] | undefined>>;

const one = (v: string | string[] | undefined) => (Array.isArray(v) ? v[0] : v) ?? '';
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

function closeness(similarity: number): string {
  if (similarity >= 0.5) return 'Close match';
  if (similarity >= 0.35) return 'Related';
  return 'Loosely related';
}

export default async function LibraryPage({searchParams}: {searchParams: SearchParams}) {
  const user = await requireUser();
  const params = await searchParams;
  const rawView = one(params.view);
  const view = rawView === 'inbox' || rawView === 'broken' || UUID.test(rawView) ? rawView : 'all';
  const source = (SOURCES as readonly string[]).includes(one(params.source)) ? (one(params.source) as Source) : null;
  const q = one(params.q).slice(0, 200);
  const aiReady = aiConfigured();
  const byMeaning = Boolean(q) && aiReady && one(params.match) === 'meaning';
  const page = Math.max(1, Number.parseInt(one(params.page), 10) || 1);

  const [categories, stats, sorting] = await Promise.all([listCategories(user.id), libraryStats(user.id), sortingState(user.id)]);

  let items: Array<Item & {note?: string}> = [];
  let total = 0;
  let searchError: string | null = null;
  if (byMeaning) {
    try {
      const results = await searchByMeaning(user.id, q, {view, source});
      items = results.map((r) => ({...r, note: closeness(r.similarity)}));
      total = items.length;
    } catch (err) {
      searchError = describeAiError(err);
    }
  } else {
    ({items, total} = await listItems(user.id, {view, source, q, page}));
  }
  const showResurface = view === 'all' && !q && !source && page === 1 && stats.total >= 10;
  const resurfaced = showResurface ? await resurfaceItems(user.id) : [];

  const current = categories.find((c) => c.id === view);
  const heading = q ? `Results for “${q}”` : view === 'inbox' ? 'Inbox' : view === 'broken' ? 'Broken links' : current?.name ?? 'All saves';
  const description = view === 'inbox'
    ? 'Saves the AI could not place with confidence, and anything not sorted yet. Move them where they belong.'
    : view === 'broken'
      ? 'Links that no longer open. Select them to remove them, or keep the ones you still want a record of.'
      : current?.description;
  const pages = byMeaning ? 1 : Math.max(1, Math.ceil(total / PAGE_SIZE));
  const href = (changes: Record<string, string | null>) => {
    const sp = new URLSearchParams();
    const base: Record<string, string | null> = {view: view !== 'all' ? view : null, source, q: q || null, match: byMeaning ? 'meaning' : null, page: null};
    for (const [k, v] of Object.entries({...base, ...changes})) if (v) sp.set(k, v);
    return `/library${sp.size ? `?${sp}` : ''}`;
  };
  const categoryOptions = categories.map((c) => ({id: c.id, name: c.name}));

  return (
    <main className="mx-auto flex w-full max-w-6xl gap-8 px-4 pt-6 pb-12 sm:px-6">
      <CategoryNav className="hidden w-60 shrink-0 lg:flex" categories={categories} view={view} total={stats.total} inbox={stats.inbox} broken={stats.broken}/>
      <div className="flex min-w-0 flex-1 flex-col gap-6">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="flex min-w-0 flex-col gap-1">
            <h1 className="text-xl leading-tight font-semibold tracking-tight sm:text-2xl">{heading}</h1>
            {description && <p className="max-w-2xl text-sm text-fg-2">{description}</p>}
          </div>
          <LibraryActions
            hasItems={stats.total > 0}
            hasCategories={categories.length > 0}
            unsorted={stats.unclassified}
            sorting={sorting.running}
            aiReady={aiReady}
          />
        </div>

        <CategoryNav className="flex lg:hidden" compact categories={categories} view={view} total={stats.total} inbox={stats.inbox} broken={stats.broken}/>

        <SortingStatus running={sorting.running} error={sorting.error} unsorted={stats.unclassified} total={stats.total}/>

        {stats.total > 0 && (
          <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <div className="flex w-full flex-col gap-1 sm:max-w-sm">
              <SearchBox defaultValue={q} byMeaning={byMeaning}/>
              {q && aiReady && (
                <p className="text-sm text-fg-2">
                  {byMeaning ? 'Searching by meaning. ' : 'Searching for these words. '}
                  <LinkButton href={href({match: byMeaning ? null : 'meaning'})} variant="link" className="text-sm">
                    {byMeaning ? 'Search for the words instead' : 'Search by meaning instead'}
                  </LinkButton>
                </p>
              )}
            </div>
            <SourceFilter value={source ?? 'all'} counts={stats.bySource}/>
          </div>
        )}

        {resurfaced.length > 0 && (
          <section className="flex flex-col gap-3">
            <div className="flex flex-col gap-1">
              <h2 className="text-lg font-semibold">From your archive</h2>
              <p className="text-sm text-fg-2">Three older saves to look at again. A new set each day.</p>
            </div>
            <ItemGrid items={resurfaced.map(toCardData)} categories={categoryOptions} selectable={false}/>
          </section>
        )}

        {searchError ? (
          <div role="alert" className="rounded-panel bg-danger-tint p-4 text-sm text-danger-text">Search by meaning failed: {searchError}</div>
        ) : stats.total === 0 ? (
          <Empty className="py-16">
            <EmptyTitle>Your library is empty</EmptyTitle>
            <EmptyDescription>
              Connect the browser extension to bring in your saves from X, LinkedIn, Reddit, YouTube, Instagram and your browser. Or import a file.
            </EmptyDescription>
            <EmptyActions>
              <LinkButton href="/settings#import" variant="secondary">Import a file</LinkButton>
            </EmptyActions>
          </Empty>
        ) : items.length === 0 ? (
          <Empty className="py-16">
            <EmptyTitle>{q ? 'Nothing matches that search' : view === 'inbox' ? 'Your inbox is clear' : view === 'broken' ? 'No broken links' : 'Nothing here yet'}</EmptyTitle>
            <EmptyDescription>
              {q
                ? byMeaning ? 'Saves are made searchable by meaning in the background after they arrive. Try again in a minute.' : 'Try fewer words, or search by meaning.'
                : source ? `No ${SOURCE_LABELS[source]} saves in this view.` : 'Saves you move here, or the AI places here, show up in this list.'}
            </EmptyDescription>
            {(q || source) && (
              <EmptyActions>
                <LinkButton href="/library" variant="secondary">Show all saves</LinkButton>
              </EmptyActions>
            )}
          </Empty>
        ) : (
          <section aria-label="Saves" className="flex flex-col gap-4">
            {resurfaced.length > 0 && <h2 className="text-lg font-semibold">All saves</h2>}
            <p className="text-sm text-fg-2">
              {byMeaning ? `The ${total} saves closest in meaning, best first` : total === 1 ? '1 save' : `${total.toLocaleString('en-US')} saves`}
            </p>
            <ItemGrid items={items.map((i) => ({...toCardData(i), note: i.note}))} categories={categoryOptions}/>
            {pages > 1 && (
              <nav aria-label="Pages" className="flex items-center justify-between gap-4">
                {page > 1 ? <LinkButton href={href({page: page - 1 > 1 ? String(page - 1) : null})} variant="outline">Newer</LinkButton> : <span/>}
                <p className="text-sm text-fg-2">Page {page} of {pages}</p>
                {page < pages ? <LinkButton href={href({page: String(page + 1)})} variant="outline">Older</LinkButton> : <span/>}
              </nav>
            )}
          </section>
        )}
      </div>
    </main>
  );
}
