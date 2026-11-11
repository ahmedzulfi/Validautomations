'use client';

import {ToastProvider} from '@/components/lime/ui/toast';
import {LimePortalProvider} from '@/lib/lime/portal';

export function Providers({children}: {children: React.ReactNode}) {
  return (
    <LimePortalProvider>
      <ToastProvider>
        <div className="min-h-dvh bg-page text-fg">{children}</div>
      </ToastProvider>
    </LimePortalProvider>
  );
}
