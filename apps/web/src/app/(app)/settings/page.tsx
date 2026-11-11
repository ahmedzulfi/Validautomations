import {SOURCE_LABELS, type Source} from '@stash/shared';
import {headers} from 'next/headers';
import {LinkButton} from '@/components/app/link-button';
import {List, ListItem} from '@/components/lime/ui/list';
import {Status} from '@/components/lime/ui/status';
import {DeleteEverything, ImportBookmarks, ThemeSetting, TokenManager} from '@/components/app/settings-panels';
import {requireUser} from '@/lib/server/auth';
import {query} from '@/lib/server/db';
import {lastSyncRuns} from '@/lib/server/items';

export const metadata = {title: 'Settings · Stash'};

const SYNC_SOURCES: Source[] = ['x', 'linkedin', 'bookmark', 'web'];
const when = new Intl.DateTimeFormat('en-US', {month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit', timeZone: 'UTC', timeZoneName: 'short'});

export default async function SettingsPage() {
  const user = await requireUser();
  const [tokens, runs, h] = await Promise.all([
    query<{id: string; name: string; tokenHint: string; createdAt: string; lastUsedAt: string | null}>(
      `select id, name, token_hint as "tokenHint", created_at as "createdAt", last_used_at as "lastUsedAt"
       from api_tokens where user_id = $1 order by created_at desc`,
      [user.id],
    ),
    lastSyncRuns(user.id),
    headers(),
  ]);
  const host = h.get('x-forwarded-host') ?? h.get('host') ?? 'localhost:3000';
  const proto = h.get('x-forwarded-proto') ?? (host.startsWith('localhost') || host.startsWith('127.') ? 'http' : 'https');
  const serverUrl = process.env.STASH_PUBLIC_URL ?? `${proto}://${host}`;
  const runBySource = new Map(runs.map((r) => [r.source, r]));

  return (
    <main className="mx-auto flex w-full max-w-2xl flex-col gap-10 px-4 pt-6 pb-12 sm:px-6">
      <h1 className="text-xl leading-tight font-semibold tracking-tight sm:text-2xl">Settings</h1>

      <section className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold">Connect the browser extension</h2>
        <p className="text-base text-fg-2">
          The extension reads your X bookmarks, LinkedIn saved posts and browser bookmarks while you are logged in, and sends them here. It only reads, and only when you press sync.
        </p>
        <ol className="flex list-decimal flex-col gap-1 ps-5 text-base text-fg-2">
          <li>Install the Stash extension (see the extension folder in the project, or the Chrome Web Store once it is listed).</li>
          <li>Create a token below and copy it.</li>
          <li>Open the extension, paste this server address and the token, and press Connect.</li>
        </ol>
        <TokenManager serverUrl={serverUrl} tokens={tokens.map((t) => ({...t, createdAt: when.format(new Date(t.createdAt)), lastUsedAt: t.lastUsedAt ? when.format(new Date(t.lastUsedAt)) : null}))}/>
      </section>

      <section className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold">Last synced</h2>
        <List variant="grouped">
          {SYNC_SOURCES.map((s) => {
            const run = runBySource.get(s);
            return (
              <ListItem
                key={s}
                title={SOURCE_LABELS[s]}
                description={run
                  ? run.error ? run.error : `${when.format(new Date(run.finishedAt))}: ${run.itemsSeen} found, ${run.itemsAdded} new`
                  : 'Not synced yet'}
                trailing={run ? <Status status={run.error ? 'failed' : 'done'} label={run.error ? 'Failed' : 'Synced'}/> : <Status status="queued" label="Never"/>}
              />
            );
          })}
        </List>
      </section>

      <section id="import" className="flex scroll-mt-20 flex-col gap-3">
        <h2 className="text-lg font-semibold">Import a bookmarks file</h2>
        <p className="text-base text-fg-2">
          Every browser can export its bookmarks as an HTML file: in Chrome, open the bookmark manager, then the menu, then Export bookmarks. Safari and Firefox have the same option.
        </p>
        <ImportBookmarks/>
      </section>

      <section className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold">Export your library</h2>
        <p className="text-base text-fg-2">Download everything, with categories, tags and summaries. Your saves are yours.</p>
        <div className="flex flex-wrap gap-2">
          <LinkButton external href="/api/export?format=md" variant="secondary">Download Markdown</LinkButton>
          <LinkButton external href="/api/export?format=csv" variant="secondary">Download CSV</LinkButton>
        </div>
      </section>

      <section className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold">Appearance</h2>
        <ThemeSetting/>
      </section>

      <section className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold">Delete your library</h2>
        <p className="text-base text-fg-2">Removes every save, category and sync record from this account. Your account and extension tokens stay.</p>
        <DeleteEverything/>
      </section>
    </main>
  );
}
