import {CategoryEditor} from '@/components/app/category-editor';
import {aiConfigured} from '@/lib/server/ai';
import {requireUser} from '@/lib/server/auth';
import {libraryStats, listCategories} from '@/lib/server/items';
import {pendingProposal} from '@/lib/server/organize';

export const metadata = {title: 'Categories · Stash'};

export default async function CategoriesPage() {
  const user = await requireUser();
  const [categories, proposal, stats] = await Promise.all([
    listCategories(user.id),
    pendingProposal(user.id),
    libraryStats(user.id),
  ]);
  return (
    <main className="mx-auto flex w-full max-w-2xl flex-col gap-8 px-4 pt-6 pb-12 sm:px-6">
      <div className="flex flex-col gap-1">
        <h1 className="text-xl leading-tight font-semibold tracking-tight sm:text-2xl">Categories</h1>
        <p className="text-base text-fg-2">
          The shelves your saves are sorted onto. The AI suggests a set that fits what you save; you edit and approve it before anything moves.
        </p>
      </div>
      <CategoryEditor
        key={proposal?.id ?? 'current'}
        current={categories.map((c) => ({id: c.id, name: c.name, description: c.description, count: c.count}))}
        proposal={proposal ? {id: proposal.id, categories: proposal.categories} : null}
        itemCount={stats.total}
        aiReady={aiConfigured()}
      />
    </main>
  );
}
