import {AskView} from '@/components/app/ask-view';
import {hueFor} from '@/lib/hue';
import {aiConfigured} from '@/lib/server/ai';
import {requireUser} from '@/lib/server/auth';
import {listCategories} from '@/lib/server/items';
import {embeddingCoverage} from '@/lib/server/search';

export const metadata = {title: 'Ask · Stash'};

export default async function AskPage() {
  const user = await requireUser();
  const [coverage, categories] = await Promise.all([embeddingCoverage(user.id), listCategories(user.id)]);
  return (
    <main className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-4 pt-6 pb-12 sm:px-6">
      <div className="stash-hero flex flex-col gap-2 rounded-sheet p-6 sm:p-8">
        <h1 className="text-2xl leading-tight font-semibold tracking-tight sm:text-3xl">Ask your saves</h1>
        <p className="stash-hero-muted text-base">
          Ask in your own words. The answer comes only from what you saved, with links to the saves it used.
        </p>
      </div>
      <AskView
        aiReady={aiConfigured()}
        coverage={coverage}
        categories={categories.map((c, i) => ({id: c.id, name: c.name, hue: hueFor(i)}))}
      />
    </main>
  );
}
