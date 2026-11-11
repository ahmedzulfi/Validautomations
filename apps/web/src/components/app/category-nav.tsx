import Link from 'next/link';
import {chipClass} from '@/components/lime/ui/chip';
import {cn} from '@/lib/lime/utils';

type Props = {
  categories: Array<{id: string; name: string; count: number}>;
  view: string;
  total: number;
  inbox: number;
  broken?: number;
  compact?: boolean;
  className?: string;
};

/**
 * The views of the library: everything, the inbox, then each category with its count. A column on wide
 * screens; a scrolling row of chips on a phone.
 */
export function CategoryNav({categories, view, total, inbox, broken = 0, compact, className}: Props) {
  const rows = [
    {key: 'all', href: '/library', label: 'All saves', count: total},
    {key: 'inbox', href: '/library?view=inbox', label: 'Inbox', count: inbox},
    ...categories.map((c) => ({key: c.id, href: `/library?view=${c.id}`, label: c.name, count: c.count})),
    ...(broken > 0 || view === 'broken' ? [{key: 'broken', href: '/library?view=broken', label: 'Broken links', count: broken}] : []),
  ];

  if (compact) {
    return (
      <nav aria-label="Categories" className={cn('-mx-4 gap-2 overflow-x-auto px-4 pb-1', className)}>
        {rows.map((row) => (
          <Link
            key={row.key}
            href={row.href}
            aria-current={view === row.key ? 'page' : undefined}
            data-pressed={view === row.key ? '' : undefined}
            className={cn(chipClass, 'shrink-0')}
          >
            {row.label}
            <span className="text-fg-3 tabular-nums">{row.count}</span>
          </Link>
        ))}
      </nav>
    );
  }

  return (
    <nav aria-label="Categories" className={cn('sticky top-20 flex-col gap-1 self-start', className)}>
      <h2 className="px-3 pb-2 text-lg font-semibold">Categories</h2>
      {rows.map((row, i) => (
        <div key={row.key} className="contents">
          {(i === 2 || (row.key === 'broken' && i > 2)) && <div role="separator" className="mx-3 my-2 h-px bg-line"/>}
          <Link
            href={row.href}
            aria-current={view === row.key ? 'page' : undefined}
            className={cn(
              'flex items-center justify-between gap-3 rounded-md px-3 py-2 text-sm transition-colors duration-(--dur-fast)',
              view === row.key ? 'bg-accent-wash font-medium text-fg' : 'text-fg-2 hover:bg-surface hover:text-fg',
            )}
          >
            <span className="truncate">{row.label}</span>
            <span className="text-fg-3 tabular-nums">{row.count}</span>
          </Link>
        </div>
      ))}
      {categories.length === 0 && (
        <p className="px-3 pt-2 text-sm text-fg-2">
          No categories yet. <Link href="/categories" className="font-medium text-fg underline underline-offset-4">Set them up</Link>
        </p>
      )}
    </nav>
  );
}
