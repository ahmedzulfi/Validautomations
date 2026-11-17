'use client';

import Link from 'next/link';
import {useTransition} from 'react';
import {startSortingAction} from '@/app/actions';
import {useToast} from '@/components/lime/ui/toast';

type Props = {
  hasItems: boolean;
  hasCategories: boolean;
  unsorted: number;
  sorting: boolean;
  aiReady: boolean;
};

/** The one main action of the library, on its banner, which changes with what the person needs to do next. */
export function LibraryActions({hasItems, hasCategories, unsorted, sorting, aiReady}: Props) {
  const toast = useToast();
  const [pending, startTransition] = useTransition();

  // Sits on the gradient banner, so it is a white button with violet text.
  if (!hasItems) {
    return <Link href="/settings" className="stash-hero-action">Connect the extension</Link>;
  }
  if (!hasCategories) {
    return <Link href="/categories" className="stash-hero-action">Set up categories</Link>;
  }
  if (unsorted > 0 && aiReady && !sorting) {
    return (
      // eslint-disable-next-line lime/no-native-control -- Lime has no white button for the gradient banner; .stash-hero-action brings its own focus ring and busy state.
      <button
        type="button"
        className="stash-hero-action"
        aria-busy={pending || undefined}
        disabled={pending}
        onClick={() => startTransition(async () => {
          const result = await startSortingAction();
          if (result.error) toast.show({title: result.error});
        })}
      >
        {pending ? 'Starting…' : `Sort ${unsorted.toLocaleString('en-US')} ${unsorted === 1 ? 'save' : 'saves'}`}
      </button>
    );
  }
  return <Link href="/settings" className="stash-hero-action">Add saves</Link>;
}
