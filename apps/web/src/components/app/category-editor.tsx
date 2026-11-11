'use client';

import Link from 'next/link';
import {useRouter} from 'next/navigation';
import {useId, useState, useTransition} from 'react';
import {applyCategoriesAction, dismissProposalAction, suggestCategoriesAction} from '@/app/actions';
import {ActionReceipt} from '@/components/lime/ai/action-receipt';
import {Add, Close, Icon} from '@/components/lime/icon';
import {Badge} from '@/components/lime/ui/badge';
import {Button, buttonVariants} from '@/components/lime/ui/button';
import {Field, FieldLabel} from '@/components/lime/ui/field';
import {Input} from '@/components/lime/ui/input';

type Row = {key: string; name: string; description: string; count?: number};

type Props = {
  current: Array<{id: string; name: string; description: string; count: number}>;
  proposal: {id: string; categories: Array<{name: string; description: string}>} | null;
  itemCount: number;
  aiReady: boolean;
};

let nextKey = 0;
const newKey = () => `row-${++nextKey}`;

export function CategoryEditor({current, proposal, itemCount, aiReady}: Props) {
  const router = useRouter();
  const counts = new Map(current.map((c) => [c.name.toLowerCase(), c.count]));
  const initial: Row[] = proposal
    ? proposal.categories.map((c) => ({key: newKey(), ...c, count: counts.get(c.name.toLowerCase())}))
    : current.map((c) => ({key: c.id, name: c.name, description: c.description, count: c.count}));
  const [rows, setRows] = useState<Row[]>(initial);
  const [error, setError] = useState<string | null>(null);
  const [receipt, setReceipt] = useState<{count: number; removed: number} | null>(null);
  const [saving, startSaving] = useTransition();
  const [suggesting, startSuggesting] = useTransition();
  const [discarding, startDiscarding] = useTransition();
  const reviewing = Boolean(proposal);
  const dirty = reviewing || JSON.stringify(rows.map(({name, description}) => ({name, description}))) !==
    JSON.stringify(current.map(({name, description}) => ({name, description})));

  const update = (key: string, patch: Partial<Row>) => setRows((list) => list.map((r) => (r.key === key ? {...r, ...patch} : r)));
  const remove = (key: string) => setRows((list) => list.filter((r) => r.key !== key));
  const add = () => setRows((list) => [...list, {key: newKey(), name: '', description: ''}]);

  const suggest = () => startSuggesting(async () => {
    setError(null);
    const result = await suggestCategoriesAction();
    if ('error' in result) setError(result.error);
    else router.refresh();
  });

  const apply = () => startSaving(async () => {
    setError(null);
    const list = rows.filter((r) => r.name.trim()).map((r) => ({name: r.name.trim(), description: r.description.trim()}));
    const names = list.map((r) => r.name.toLowerCase());
    if (list.length === 0) return setError('Keep at least one category.');
    if (new Set(names).size !== names.length) return setError('Two categories have the same name. Rename or remove one.');
    const result = await applyCategoriesAction(proposal?.id ?? null, list);
    if (result.error) return setError(result.error);
    setReceipt({count: list.length, removed: result.removed ?? 0});
    router.refresh();
  });

  const discard = () => startDiscarding(async () => {
    if (proposal) await dismissProposalAction(proposal.id);
    router.refresh();
  });

  if (receipt) {
    return (
      <section className="flex flex-col gap-4">
        <ActionReceipt
          name="Stash"
          summary="Your categories are saved."
          lines={[
            {label: `Saved ${receipt.count} ${receipt.count === 1 ? 'category' : 'categories'}`},
            ...(receipt.removed ? [{label: `Removed ${receipt.removed}, and sent their saves back to be sorted`}] : []),
            {label: aiReady ? 'Started sorting your saves in the background' : 'Sorting needs OPENROUTER_API_KEY on the server', failed: !aiReady},
          ]}
        />
        <Link href="/library" className={buttonVariants({className: 'self-start'})}>Go to the library</Link>
      </section>
    );
  }

  if (!reviewing && rows.length === 0) {
    return (
      <section className="flex flex-col gap-4 rounded-panel bg-surface p-6">
        <h2 className="text-lg font-semibold">Start with a suggestion</h2>
        <p className="text-base text-fg-2">
          {itemCount > 0
            ? `The AI reads a sample of your ${itemCount.toLocaleString('en-US')} saves and suggests 8 to 15 categories that fit them. Nothing is sorted until you approve.`
            : 'Import some saves first. Then the AI can read them and suggest categories that fit.'}
        </p>
        {!aiReady && <p className="text-sm text-fg-2">Suggestions need OPENROUTER_API_KEY on the server. You can also add categories by hand.</p>}
        {error && <p role="alert" className="text-sm text-danger-text">{error}</p>}
        <div className="flex flex-wrap gap-2">
          {itemCount > 0 && aiReady
            ? <Button pending={suggesting} onClick={suggest}>Suggest categories</Button>
            : itemCount === 0 ? <Link href="/settings" className={buttonVariants()}>Import saves</Link> : null}
          <Button variant={itemCount > 0 && aiReady ? 'secondary' : 'primary'} onClick={add} icon={<Icon icon={Add} size={16}/>}>Add by hand</Button>
        </div>
      </section>
    );
  }

  return (
    <section className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <h2 className="text-lg font-semibold">{reviewing ? 'Review the suggested categories' : 'Your categories'}</h2>
        <p className="text-sm text-fg-2">
          {reviewing
            ? 'Rename, remove or add any. When you apply them, the AI sorts your saves onto these. Saves you placed yourself stay put.'
            : 'Rename or remove categories here. Removing one sends its saves back to be sorted again.'}
        </p>
      </div>

      <ol className="flex flex-col gap-3">
        {rows.map((row, i) => <CategoryRow key={row.key} row={row} index={i} onChange={update} onRemove={remove}/>)}
      </ol>

      <Button variant="ghost" className="self-start" onClick={add} icon={<Icon icon={Add} size={16}/>}>Add a category</Button>

      {error && <p role="alert" className="text-sm text-danger-text">{error}</p>}

      <div className="flex flex-wrap items-center gap-2 border-t border-line pt-4">
        <Button pending={saving} disabled={!dirty} onClick={apply}>{reviewing ? 'Apply categories' : 'Save changes'}</Button>
        {reviewing
          ? <Button variant="secondary" pending={discarding} onClick={discard}>Discard suggestion</Button>
          : aiReady && itemCount > 0 && <Button variant="secondary" pending={suggesting} onClick={suggest}>Suggest a new set</Button>}
      </div>
    </section>
  );
}

function CategoryRow({row, index, onChange, onRemove}: {row: Row; index: number; onChange: (key: string, patch: Partial<Row>) => void; onRemove: (key: string) => void}) {
  const id = useId();
  return (
    <li className="flex items-start gap-3 rounded-panel bg-surface p-4">
      <div className="grid min-w-0 flex-1 gap-3 sm:grid-cols-[minmax(0,2fr)_minmax(0,3fr)]">
        <Field name={`name-${id}`}>
          <FieldLabel>Name</FieldLabel>
          <Input value={row.name} maxLength={60} onChange={(e) => onChange(row.key, {name: e.target.value})} aria-describedby={`${id}-count`}/>
        </Field>
        <Field name={`description-${id}`}>
          <FieldLabel>What belongs here</FieldLabel>
          <Input value={row.description} maxLength={300} onChange={(e) => onChange(row.key, {description: e.target.value})}/>
        </Field>
      </div>
      <div className="flex shrink-0 flex-col items-end gap-2 pt-7">
        <Button variant="ghost" size="icon-sm" aria-label={`Remove ${row.name || `category ${index + 1}`}`} onClick={() => onRemove(row.key)}>
          <Icon icon={Close} size={16}/>
        </Button>
        {row.count !== undefined && <Badge id={`${id}-count`}>{row.count}</Badge>}
      </div>
    </li>
  );
}
