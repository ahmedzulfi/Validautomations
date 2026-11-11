'use client';

import {useRouter} from 'next/navigation';
import {useEffect, useTransition} from 'react';
import {startSortingAction} from '@/app/actions';
import {Button} from '@/components/lime/ui/button';
import {Progress} from '@/components/lime/ui/progress';

/**
 * Shows the background sort while it runs, refreshing the page every few seconds so items move into
 * their categories as they are sorted. Shows what went wrong if it stopped.
 */
export function SortingStatus({running, error, unsorted, total}: {running: boolean; error: string | null; unsorted: number; total: number}) {
  const router = useRouter();
  const [pending, startTransition] = useTransition();

  useEffect(() => {
    if (!running) return;
    const timer = setInterval(() => router.refresh(), 3000);
    return () => clearInterval(timer);
  }, [running, router]);

  if (running) {
    const done = Math.max(0, total - unsorted);
    return (
      <div role="status" className="flex flex-col gap-2 rounded-panel bg-surface p-4">
        <p className="text-sm text-fg">
          Sorting your saves. {unsorted.toLocaleString('en-US')} left. You can keep browsing.
        </p>
        <Progress value={total ? (done / total) * 100 : 0} label="Sorting progress" valueText={`${done} of ${total} sorted`}/>
      </div>
    );
  }
  if (error) {
    return (
      <div role="alert" className="flex flex-wrap items-center justify-between gap-3 rounded-panel bg-danger-tint p-4">
        <p className="text-sm text-danger-text">Sorting stopped: {error}</p>
        <Button variant="outline" size="sm" pending={pending} onClick={() => startTransition(async () => {
          await startSortingAction();
          router.refresh();
        })}>
          Try again
        </Button>
      </div>
    );
  }
  return null;
}
