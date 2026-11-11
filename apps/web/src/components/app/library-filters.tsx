'use client';

import {SOURCE_LABELS, type Source} from '@stash/shared';
import {usePathname, useRouter, useSearchParams} from 'next/navigation';
import {Field, FieldLabel} from '@/components/lime/ui/field';
import {Input} from '@/components/lime/ui/input';
import {Segmented, SegmentedItem} from '@/components/lime/ui/segmented';

const FILTERS: Array<Source> = ['x', 'linkedin', 'bookmark', 'web'];

export function SourceFilter({value, counts}: {value: string; counts: Partial<Record<Source, number>>}) {
  const router = useRouter();
  const pathname = usePathname();
  const params = useSearchParams();
  const shown = FILTERS.filter((s) => (counts[s] ?? 0) > 0);
  if (shown.length < 2) return null;
  return (
    <Segmented
      aria-label="Source"
      value={value}
      size="sm"
      onValueChange={(next) => {
        const sp = new URLSearchParams(params);
        if (next === 'all') sp.delete('source');
        else sp.set('source', next);
        sp.delete('page');
        router.push(`${pathname}${sp.size ? `?${sp}` : ''}`);
      }}
    >
      <SegmentedItem value="all">All</SegmentedItem>
      {shown.map((s) => <SegmentedItem key={s} value={s}>{SOURCE_LABELS[s]}</SegmentedItem>)}
    </Segmented>
  );
}

export function SearchBox({defaultValue}: {defaultValue: string}) {
  const params = useSearchParams();
  return (
    <form action="/library" role="search" className="w-full sm:max-w-sm">
      {params.get('view') && <input type="hidden" name="view" value={params.get('view')!}/>}
      {params.get('source') && <input type="hidden" name="source" value={params.get('source')!}/>}
      <Field name="q">
        <FieldLabel className="sr-only">Search your saves</FieldLabel>
        <Input name="q" type="search" defaultValue={defaultValue} placeholder="Search by words, author or tag"/>
      </Field>
    </form>
  );
}
