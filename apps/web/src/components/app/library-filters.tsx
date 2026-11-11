'use client';

import {SOURCE_LABELS, SOURCES, type Source} from '@stash/shared';
import {usePathname, useRouter, useSearchParams} from 'next/navigation';
import {ChevronDown, Icon} from '@/components/lime/icon';
import {Button} from '@/components/lime/ui/button';
import {DropdownMenu, DropdownMenuContent, DropdownMenuRadioGroup, DropdownMenuRadioItem, DropdownMenuTrigger} from '@/components/lime/ui/dropdown-menu';
import {Field, FieldLabel} from '@/components/lime/ui/field';
import {Input} from '@/components/lime/ui/input';
import {Segmented, SegmentedItem} from '@/components/lime/ui/segmented';

const FILTERS: readonly Source[] = SOURCES;

export function SourceFilter({value, counts}: {value: string; counts: Partial<Record<Source, number>>}) {
  const router = useRouter();
  const pathname = usePathname();
  const params = useSearchParams();
  const shown = FILTERS.filter((s) => (counts[s] ?? 0) > 0);
  if (shown.length < 2) return null;
  const choose = (next: string) => {
    const sp = new URLSearchParams(params);
    if (next === 'all') sp.delete('source');
    else sp.set('source', next);
    sp.delete('page');
    router.push(`${pathname}${sp.size ? `?${sp}` : ''}`);
  };
  // Lime's Segmented holds two to five options; past that, the sources move into a menu.
  if (shown.length > 4) {
    return (
      <DropdownMenu>
        <DropdownMenuTrigger render={<Button variant="outline" size="sm" className="self-start"/>}>
          {value === 'all' ? 'All sources' : SOURCE_LABELS[value as Source]}
          <Icon icon={ChevronDown} size={16}/>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuRadioGroup value={value} onValueChange={(v) => choose(String(v))}>
            <DropdownMenuRadioItem value="all">All sources</DropdownMenuRadioItem>
            {shown.map((s) => (
              <DropdownMenuRadioItem key={s} value={s}>{SOURCE_LABELS[s]} ({counts[s]})</DropdownMenuRadioItem>
            ))}
          </DropdownMenuRadioGroup>
        </DropdownMenuContent>
      </DropdownMenu>
    );
  }
  return (
    <Segmented aria-label="Source" value={value} size="sm" onValueChange={choose}>
      <SegmentedItem value="all">All</SegmentedItem>
      {shown.map((s) => <SegmentedItem key={s} value={s}>{SOURCE_LABELS[s]}</SegmentedItem>)}
    </Segmented>
  );
}

export function SearchBox({defaultValue, byMeaning}: {defaultValue: string; byMeaning?: boolean}) {
  const params = useSearchParams();
  return (
    <form action="/library" role="search" className="w-full">
      {params.get('view') && <input type="hidden" name="view" value={params.get('view')!}/>}
      {params.get('source') && <input type="hidden" name="source" value={params.get('source')!}/>}
      {byMeaning && <input type="hidden" name="match" value="meaning"/>}
      <Field name="q">
        <FieldLabel className="sr-only">Search your saves</FieldLabel>
        <Input name="q" type="search" defaultValue={defaultValue} placeholder="Search by words, author or tag"/>
      </Field>
    </form>
  );
}
