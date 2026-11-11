'use client';

import Link from 'next/link';
import {usePathname} from 'next/navigation';
import {signOutAction} from '@/app/actions';
import {Avatar} from '@/components/lime/ui/avatar';
import {Button} from '@/components/lime/ui/button';
import {DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuLabel, DropdownMenuGroup, DropdownMenuSeparator, DropdownMenuTrigger} from '@/components/lime/ui/dropdown-menu';
import {cn} from '@/lib/lime/utils';

const LINKS = [
  {href: '/library', label: 'Library'},
  {href: '/ask', label: 'Ask'},
  {href: '/categories', label: 'Categories'},
  {href: '/settings', label: 'Settings'},
];

export function AppHeader({email}: {email: string}) {
  const pathname = usePathname();
  return (
    <header className="sticky top-0 z-10 border-b border-line bg-page">
      <div className="mx-auto flex h-14 w-full max-w-6xl items-center gap-2 px-4 sm:px-6">
        <Link href="/library" className="me-2 text-lg font-semibold tracking-tight">Stash</Link>
        <nav aria-label="Main" className="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto">
          {LINKS.map((link) => {
            const active = pathname.startsWith(link.href);
            return (
              <Link
                key={link.href}
                href={link.href}
                aria-current={active ? 'page' : undefined}
                className={cn(
                  'rounded-control px-3 py-1.5 text-sm font-medium whitespace-nowrap transition-colors duration-(--dur-fast)',
                  active ? 'bg-surface-2 text-fg' : 'text-fg-2 hover:bg-surface hover:text-fg',
                )}
              >
                {link.label}
              </Link>
            );
          })}
        </nav>
        <DropdownMenu>
          <DropdownMenuTrigger render={<Button variant="ghost" size="icon" aria-label="Account"/>}>
            <Avatar name={email} size="sm"/>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuGroup>
              <DropdownMenuLabel>{email}</DropdownMenuLabel>
            </DropdownMenuGroup>
            <DropdownMenuSeparator/>
            <DropdownMenuItem onClick={() => signOutAction()}>Sign out</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </header>
  );
}
