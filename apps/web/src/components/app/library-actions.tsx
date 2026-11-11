'use client';

import Link from 'next/link';
import {useTransition} from 'react';
import {startSortingAction} from '@/app/actions';
import {Button, buttonVariants} from '@/components/lime/ui/button';
import {useToast} from '@/components/lime/ui/toast';

type Props = {
  hasItems: boolean;
  hasCategories: boolean;
  unsorted: number;
  sorting: boolean;
  aiReady: boolean;
};

/** The one lime action of the library, which changes with what the person needs to do next. */
export function LibraryActions({hasItems, hasCategories, unsorted, sorting, aiReady}: Props) {
  const toast = useToast();
  const [pending, startTransition] = useTransition();

  if (!hasItems) {
    return <Link href="/settings" className={buttonVariants()}>Connect the extension</Link>;
  }
  if (!hasCategories) {
    return <Link href="/categories" className={buttonVariants()}>Set up categories</Link>;
  }
  if (unsorted > 0 && aiReady && !sorting) {
    return (
      <Button
        pending={pending}
        onClick={() => startTransition(async () => {
          const result = await startSortingAction();
          if (result.error) toast.show({title: result.error});
        })}
      >
        {`Sort ${unsorted.toLocaleString('en-US')} ${unsorted === 1 ? 'save' : 'saves'}`}
      </Button>
    );
  }
  return <Link href="/settings" className={buttonVariants()}>Add saves</Link>;
}
