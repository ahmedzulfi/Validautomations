import Link from 'next/link';
import {cn} from '@/lib/lime/utils';
import {hueFor} from '@/lib/hue';

type Props = {
  categories: Array<{id: string; name: string; count: number}>;
  view: string;
  total: number;
  inbox: number;
  broken?: number;
  compact?: boolean;
  className?: string;
};

type Row = {key: string; href: string; label: string; count: number; hue: string};

/**
 * The views of the library: everything, the inbox, then each category in its own colour. A card in a
 * column on wide screens; a scrolling row of coloured chips on a phone.
 */
export function CategoryNav({categories, view, total, inbox, broken = 0, compact, className}: Props) {
  const top: Row[] = [
    {key: 'all', href: '/library', label: 'All saves', count: total, hue: '7'},
    {key: 'inbox', href: '/library?view=inbox', label: 'Inbox', count: inbox, hue: '2'},
  ];
  const cats: Row[] = categories.map((c, i) => ({key: c.id, href: `/library?view=${c.id}`, label: c.name, count: c.count, hue: hueFor(i)}));
  const extra: Row[] = broken > 0 || view === 'broken' ? [{key: 'broken', href: '/library?view=broken', label: 'Broken links', count: broken, hue: '6'}] : [];

  if (compact) {
    return (
      <nav aria-label="Categories" className={cn('-mx-4 gap-2 overflow-x-auto px-4 pb-1', className)}>
        {[...top, ...cats, ...extra].map((row) => (
          <Link
            key={row.key}
            href={row.href}
            data-hue={row.hue}
            aria-current={view === row.key ? 'page' : undefined}
            className="hue-row flex shrink-0 items-center gap-2 rounded-control border border-line bg-raised px-3 py-1.5 text-sm whitespace-nowrap text-fg-2"
          >
            <span className="hue-dot" aria-hidden="true"/>
            {row.label}
            <span className="tabular-nums opacity-70">{row.count}</span>
          </Link>
        ))}
      </nav>
    );
  }

  const link = (row: Row) => (
    <Link
      key={row.key}
      href={row.href}
      data-hue={row.hue}
      aria-current={view === row.key ? 'page' : undefined}
      className="hue-row flex items-center gap-3 rounded-field px-3 py-2 text-sm text-fg-2 transition-colors duration-(--dur-fast) hover:bg-surface hover:text-fg"
    >
      <span className="hue-dot" aria-hidden="true"/>
      <span className="min-w-0 flex-1 truncate">{row.label}</span>
      <span className="rounded-control bg-surface px-2 text-xs tabular-nums">{row.count}</span>
    </Link>
  );

  return (
    <nav aria-label="Categories" className={cn('stash-card sticky top-24 flex-col gap-1 self-start p-3', className)}>
      <h2 className="px-3 pt-1 pb-2 text-lg font-semibold">Your shelves</h2>
      {top.map(link)}
      {cats.length > 0 && <div role="separator" className="mx-3 my-2 h-px bg-line"/>}
      {cats.map(link)}
      {extra.length > 0 && <div role="separator" className="mx-3 my-2 h-px bg-line"/>}
      {extra.map(link)}
      {categories.length === 0 && (
        <p className="px-3 pt-2 pb-1 text-sm text-fg-2">
          No categories yet. <Link href="/categories" className="font-medium text-accent-line underline underline-offset-4">Set them up</Link>
        </p>
      )}
    </nav>
  );
}
