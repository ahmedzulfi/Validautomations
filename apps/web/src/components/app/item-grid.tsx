'use client';

import {useRouter} from 'next/navigation';
import {useState, useTransition} from 'react';
import {deleteItemsAction, moveItemsAction} from '@/app/actions';
import {Button} from '@/components/lime/ui/button';
import {
  DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger,
} from '@/components/lime/ui/dropdown-menu';
import {useToast} from '@/components/lime/ui/toast';
import {ItemCard, type ItemCardData} from './item-card';

type Props = {
  items: Array<ItemCardData & {note?: string}>;
  categories: Array<{id: string; name: string}>;
  /** False for small showcase lists that should not offer select mode. */
  selectable?: boolean;
};

/**
 * The grid of saves, with a select mode for acting on many at once: a sticky bar at the bottom holds the
 * count and the actions. Lime has no bulk action bar, so this one is built from its Button and menu.
 */
export function ItemGrid({items, categories, selectable = true}: Props) {
  const router = useRouter();
  const toast = useToast();
  const [selecting, setSelecting] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [pending, startTransition] = useTransition();
  const count = selected.size;
  const allSelected = count === items.length && count > 0;

  const toggle = (id: string, on: boolean) => setSelected((prev) => {
    const next = new Set(prev);
    if (on) next.add(id);
    else next.delete(id);
    return next;
  });
  const stop = () => {
    setSelecting(false);
    setSelected(new Set());
  };
  const move = (categoryId: string | null, name: string) => startTransition(async () => {
    const ids = [...selected];
    const moved = await moveItemsAction(ids, categoryId);
    stop();
    router.refresh();
    toast.show({title: `Moved ${moved} ${moved === 1 ? 'save' : 'saves'} to ${name}`});
  });
  const remove = () => startTransition(async () => {
    const removed = await deleteItemsAction([...selected]);
    stop();
    router.refresh();
    toast.show({title: `Removed ${removed} ${removed === 1 ? 'save' : 'saves'} from your library`});
  });

  return (
    <div className="flex flex-col gap-4">
      {selectable && <div className="flex justify-end">
        {selecting
          ? <Button variant="ghost" size="sm" onClick={stop}>Done selecting</Button>
          : <Button variant="outline" size="sm" onClick={() => setSelecting(true)}>Select</Button>}
      </div>}

      <ul className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
        {items.map((item) => (
          <li key={item.id} className="flex">
            <ItemCard
              item={item}
              categories={categories}
              note={item.note}
              selecting={selecting}
              selected={selected.has(item.id)}
              onSelectedChange={(on) => toggle(item.id, on)}
            />
          </li>
        ))}
      </ul>

      {selecting && (
        <div role="region" aria-label="Selected saves" className="sticky bottom-0 z-10 -mx-4 border-t border-line bg-page px-4 pt-3 pb-4 sm:mx-0 sm:px-0">
          <div className="flex flex-wrap items-center gap-2">
            <p className="me-auto text-sm text-fg" aria-live="polite">
              {count === 0 ? 'Select saves to move or remove' : `${count} selected`}
            </p>
            <Button variant="ghost" size="sm" onClick={() => setSelected(allSelected ? new Set() : new Set(items.map((i) => i.id)))}>
              {allSelected ? 'Clear selection' : 'Select all on this page'}
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger render={<Button variant="solid" size="sm" disabled={count === 0} pending={pending}/>}>
                Move to
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" side="top">
                {categories.map((c) => (
                  <DropdownMenuItem key={c.id} onClick={() => move(c.id, c.name)}>{c.name}</DropdownMenuItem>
                ))}
                {categories.length > 0 && <DropdownMenuSeparator/>}
                <DropdownMenuItem onClick={() => move(null, 'Inbox')}>Inbox</DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
            <Button variant="outline" size="sm" disabled={count === 0 || pending} onClick={remove}>Remove</Button>
          </div>
        </div>
      )}
    </div>
  );
}
