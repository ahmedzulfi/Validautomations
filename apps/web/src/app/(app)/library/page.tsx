import {SOURCE_LABELS, SOURCES, type Source} from '@stash/shared';
import {CategoryNav} from '@/components/app/category-nav';
import {ItemCard, type ItemCardData} from '@/components/app/item-card';
import {LibraryActions} from '@/components/app/library-actions';
import {SearchBox, SourceFilter} from '@/components/app/library-filters';
import {SortingStatus} from '@/components/app/sorting-status';
import {LinkButton} from '@/components/app/link-button';
import {Empty, EmptyActions, EmptyDescription, EmptyTitle} from '@/components/lime/ui/empty';
import {aiConfigured} from '@/lib/server/ai';
import {requireUser} from '@/lib/server/auth';
import {libraryStats, listCategories, listItems, PAGE_SIZE, type Item} from '@/lib/server/items';
import {sortingState} from '@/lib/server/organize';

export const metadata = {title: 'Library · Stash'};

type SearchParams = Promise<Record<string, string | string[] | undefined>>;

const one = (v: string | string[] | undefined) => (Array.isArray(v) ? v[0] : v) ?? '';
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export default async function LibraryPage({searchParams}: {searchParams: SearchParams}) {
  const user = await requireUser();
  const params = await searchParams;
  const rawView = one(params.view);
  const view = rawView === 'inbox' || UUID.test(rawView) ? rawView : 'all';
  const source = (SOURCES as readonly string[]).includes(one(params.source)) ? (one(params.source) as Source) : null;
  const q = one(params.q).slice(0, 200);
  const page = Math.max(1, Number.parseInt(one(params.page), 10) || 1);

  const [categories, stats, sorting, {items, total}] = await Promise.all([
    listCategories(user.id),
    libraryStats(user.id),
    sortingState(user.id),
    listItems(user.id, {view, source, q, page}),
  ]);

  const current = categories.find((c) => c.id === view);
  const heading = q ? `Results for “${q}”` : view === 'inbox' ? 'Inbox' : current?.name ?? 'All saves';
  const description = view === 'inbox'
    ? 'Saves the AI could not place with confidence, and anything not sorted yet. Move them where they belong.'
    : current?.description;
  const pages = Math.max(1, Math.ceil(total / PAGE_SIZE));
  const pageHref = (p: number) => {
    const sp = new URLSearchParams();
    if (view !== 'all') sp.set('view', view);
    if (source) sp.set('source', source);
    if (q) sp.set('q', q);
    if (p > 1) sp.set('page', String(p));
    return `/library${sp.size ? `?${sp}` : ''}`;
  };
  const cardData = items.map((item) => toCardData(item));
  const categoryOptions = categories.map((c) => ({id: c.id, name: c.name}));

  return (
    <main className="mx-auto flex w-full max-w-6xl gap-8 px-4 pt-6 pb-12 sm:px-6">
      <CategoryNav
        className="hidden w-60 shrink-0 lg:flex"
        categories={categories}
        view={view}
        total={stats.total}
        inbox={stats.inbox}
      />
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
            aiReady={aiConfigured()}
          />
        </div>

        <CategoryNav
          className="flex lg:hidden"
          compact
          categories={categories}
          view={view}
          total={stats.total}
          inbox={stats.inbox}
        />

        <SortingStatus running={sorting.running} error={sorting.error} unsorted={stats.unclassified} total={stats.total}/>

        {stats.total > 0 && (
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <SearchBox defaultValue={q}/>
            <SourceFilter value={source ?? 'all'} counts={stats.bySource}/>
          </div>
        )}

        {stats.total === 0 ? (
          <Empty className="py-16">
            <EmptyTitle>Your library is empty</EmptyTitle>
            <EmptyDescription>
              Connect the browser extension to bring in your X bookmarks, LinkedIn saved posts and browser bookmarks. Or import a bookmarks file.
            </EmptyDescription>
            <EmptyActions>
              <LinkButton href="/settings#import" variant="secondary">Import a bookmarks file</LinkButton>
            </EmptyActions>
          </Empty>
        ) : items.length === 0 ? (
          <Empty className="py-16">
            <EmptyTitle>{q ? 'Nothing matches that search' : view === 'inbox' ? 'Your inbox is clear' : 'Nothing here yet'}</EmptyTitle>
            <EmptyDescription>
              {q ? 'Try fewer words, or search all saves.' : source ? `No ${SOURCE_LABELS[source]} saves in this view.` : 'Saves you move here, or the AI places here, show up in this list.'}
            </EmptyDescription>
            {(q || source) && (
              <EmptyActions>
                <LinkButton href="/library" variant="secondary">Show all saves</LinkButton>
              </EmptyActions>
            )}
          </Empty>
        ) : (
          <section aria-label="Saves" className="flex flex-col gap-6">
            <p className="text-sm text-fg-2">{total === 1 ? '1 save' : `${total.toLocaleString('en-US')} saves`}</p>
            <ul className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
              {cardData.map((item) => (
                <li key={item.id} className="flex">
                  <ItemCard item={item} categories={categoryOptions}/>
                </li>
              ))}
            </ul>
            {pages > 1 && (
              <nav aria-label="Pages" className="flex items-center justify-between gap-4">
                {page > 1
                  ? <LinkButton href={pageHref(page - 1)} variant="outline">Newer</LinkButton>
                  : <span/>}
                <p className="text-sm text-fg-2">Page {page} of {pages}</p>
                {page < pages
                  ? <LinkButton href={pageHref(page + 1)} variant="outline">Older</LinkButton>
                  : <span/>}
              </nav>
            )}
          </section>
        )}
      </div>
    </main>
  );
}

const dateFormat = new Intl.DateTimeFormat('en-US', {month: 'short', day: 'numeric', year: 'numeric', timeZone: 'UTC'});

function toCardData(item: Item): ItemCardData {
  const date = item.postedAt ?? item.savedAt;
  let host = '';
  try {
    host = new URL(item.url).hostname.replace(/^www\./, '');
  } catch {}
  return {
    id: item.id,
    url: item.url,
    host,
    sources: item.sources,
    title: item.title,
    body: item.body || null,
    summary: item.summary,
    authorName: item.authorName,
    authorHandle: item.authorHandle,
    authorAvatar: item.authorAvatar,
    image: item.media?.find((m) => m.type === 'image')?.url ?? null,
    folderPath: item.folderPath,
    tags: item.tags,
    dateLabel: date ? dateFormat.format(new Date(date)) : null,
    categoryId: item.categoryId,
    placedBy: item.categoryAssignedBy,
  };
}
