'use client';

import {SOURCE_LABELS, type Source} from '@stash/shared';
import Link from 'next/link';
import {useOptimistic, useState, useTransition} from 'react';
import {deleteItemAction, moveItemAction} from '@/app/actions';
import {Icon, OverflowMenuHorizontal} from '@/components/lime/icon';
import {Avatar} from '@/components/lime/ui/avatar';
import {Badge} from '@/components/lime/ui/badge';
import {Button} from '@/components/lime/ui/button';
import {Card} from '@/components/lime/ui/card';
import {
  DropdownMenu, DropdownMenuContent, DropdownMenuGroup, DropdownMenuItem, DropdownMenuLabel,
  DropdownMenuRadioGroup, DropdownMenuRadioItem, DropdownMenuSeparator, DropdownMenuTrigger,
} from '@/components/lime/ui/dropdown-menu';
import {useToast} from '@/components/lime/ui/toast';

export type ItemCardData = {
  id: string;
  url: string;
  host: string;
  sources: Source[];
  title: string | null;
  body: string | null;
  summary: string | null;
  authorName: string | null;
  authorHandle: string | null;
  authorAvatar: string | null;
  image: string | null;
  folderPath: string[] | null;
  tags: string[];
  dateLabel: string | null;
  categoryId: string | null;
  placedBy: 'ai' | 'user' | null;
};

const INBOX = 'inbox';

export function ItemCard({item, categories}: {item: ItemCardData; categories: Array<{id: string; name: string}>}) {
  const toast = useToast();
  const [, startTransition] = useTransition();
  const [categoryId, setCategoryId] = useOptimistic(item.categoryId);
  const [imageFailed, setImageFailed] = useState(false);
  const current = categories.find((c) => c.id === categoryId);
  const isPost = item.sources.includes('x') || item.sources.includes('linkedin');
  const title = item.title ?? item.host;

  const move = (value: string) => {
    const next = value === INBOX ? null : value;
    if (next === categoryId) return;
    const previous = categoryId;
    const name = categories.find((c) => c.id === next)?.name ?? 'Inbox';
    startTransition(async () => {
      setCategoryId(next);
      await moveItemAction(item.id, next);
      toast.show({
        title: `Moved to ${name}`,
        action: {label: 'Undo', onClick: () => startTransition(() => moveItemAction(item.id, previous))},
      });
    });
  };

  const remove = () => {
    startTransition(async () => {
      await deleteItemAction(item.id);
      toast.show({title: 'Removed from your library'});
    });
  };

  return (
    <Card variant="outline" className="flex w-full min-w-0 flex-col gap-3 p-4">
      <div className="flex items-start gap-3">
        {isPost && item.authorName ? (
          <Avatar name={item.authorName} src={item.authorAvatar ?? undefined} size="sm"/>
        ) : null}
        <div className="flex min-w-0 flex-1 flex-col">
          <span className="truncate text-sm font-medium text-fg">{isPost ? item.authorName ?? item.host : item.host}</span>
          <span className="truncate text-sm text-fg-3">
            {[isPost ? item.authorHandle : item.folderPath?.join(' / '), item.dateLabel].filter(Boolean).join(' · ')}
          </span>
        </div>
        <DropdownMenu>
          <DropdownMenuTrigger render={<Button variant="ghost" size="icon-sm" aria-label={`Actions for ${title}`}/>}>
            <Icon icon={OverflowMenuHorizontal} size={16}/>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuGroup>
              <DropdownMenuLabel>Move to</DropdownMenuLabel>
              <DropdownMenuRadioGroup value={categoryId ?? INBOX} onValueChange={(v) => move(String(v))}>
                {categories.map((c) => (
                  <DropdownMenuRadioItem key={c.id} value={c.id}>{c.name}</DropdownMenuRadioItem>
                ))}
                <DropdownMenuRadioItem value={INBOX}>Inbox</DropdownMenuRadioItem>
              </DropdownMenuRadioGroup>
            </DropdownMenuGroup>
            <DropdownMenuSeparator/>
            <DropdownMenuItem destructive onClick={remove}>Remove from library</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      <a href={item.url} target="_blank" rel="noreferrer" className="group/link flex flex-col gap-2 rounded-md">
        {isPost ? (
          <p className="line-clamp-6 text-base leading-snug whitespace-pre-line text-fg group-hover/link:underline group-hover/link:underline-offset-4">
            {item.body ?? item.title ?? item.summary ?? item.host}
          </p>
        ) : (
          <>
            <h3 className="line-clamp-3 text-base leading-snug font-medium text-fg group-hover/link:underline group-hover/link:underline-offset-4">
              {title}
            </h3>
            {(item.body ?? item.summary) && <p className="line-clamp-4 text-sm text-fg-2">{item.body ?? item.summary}</p>}
          </>
        )}
      </a>

      {item.image && !imageFailed && (
        // eslint-disable-next-line @next/next/no-img-element
        <img
          src={item.image}
          alt=""
          loading="lazy"
          onError={() => setImageFailed(true)}
          className="aspect-video w-full rounded-field bg-surface object-cover"
        />
      )}

      <div className="mt-auto flex flex-wrap items-center gap-2 pt-1">
        {item.sources.map((s) => <Badge key={s}>{SOURCE_LABELS[s]}</Badge>)}
        {current
          ? <Badge variant="accent" title={item.placedBy === 'ai' ? 'Placed by AI' : 'Placed by you'}>{current.name}</Badge>
          : <Badge variant="outline">Inbox</Badge>}
        {item.tags.slice(0, 3).map((tag) => (
          <Link key={tag} href={`/library?q=${encodeURIComponent(tag)}`} className="text-sm text-fg-2 hover:text-fg hover:underline hover:underline-offset-4">
            {tag}
          </Link>
        ))}
      </div>
    </Card>
  );
}
