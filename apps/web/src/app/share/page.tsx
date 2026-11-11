import {canonicalUrl} from '@stash/shared';
import {redirect} from 'next/navigation';
import {after} from 'next/server';
import {LinkButton} from '@/components/app/link-button';
import {currentUser} from '@/lib/server/auth';
import {ingestItems} from '@/lib/server/items';
import {organizeInBackground} from '@/lib/server/organize';

export const metadata = {title: 'Save to Stash'};

type SearchParams = Promise<Record<string, string | string[] | undefined>>;
const one = (v: string | string[] | undefined) => (Array.isArray(v) ? v[0] : v) ?? '';

/** Pulls the shared link out of what the phone sent: apps put it in url, in text, or both. */
function sharedLink(url: string, text: string): string | null {
  for (const candidate of [url, ...(text.match(/https?:\/\/\S+/g) ?? [])]) {
    try {
      const u = new URL(candidate.trim().replace(/[)\].,]+$/, ''));
      if (u.protocol === 'http:' || u.protocol === 'https:') return u.toString();
    } catch {}
  }
  return null;
}

/** Where the phone's share sheet sends a link. Saving the same link twice is harmless. */
export default async function SharePage({searchParams}: {searchParams: SearchParams}) {
  const params = await searchParams;
  const user = await currentUser();
  if (!user) {
    const qs = new URLSearchParams(Object.entries(params).map(([k, v]) => [k, one(v)])).toString();
    redirect(`/signin?next=${encodeURIComponent(`/share?${qs}`)}`);
  }
  const title = one(params.title).slice(0, 500);
  const text = one(params.text).slice(0, 5000);
  const link = sharedLink(one(params.url), text);

  let message: string;
  if (!link) {
    message = 'Nothing to save: the share did not include a link.';
  } else {
    const note = text.replace(link, '').trim();
    const r = await ingestItems(user.id, [{
      source: 'web', sourceId: canonicalUrl(link), url: link,
      title: title || null, text: note || null, savedAt: new Date().toISOString(),
    }]);
    if (r.added) after(() => organizeInBackground(user.id));
    message = r.added ? 'Saved to your library.' : 'Already in your library.';
  }

  return (
    <main className="mx-auto flex w-full max-w-md flex-col gap-6 px-4 pt-16 pb-8 sm:px-6">
      <h1 className="text-xl leading-tight font-semibold tracking-tight">{message}</h1>
      {link && <p className="truncate text-base text-fg-2">{title || link}</p>}
      <LinkButton href="/library" size="lg" className="w-full">Open the library</LinkButton>
    </main>
  );
}
