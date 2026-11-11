'use client';

import {useEffect} from 'react';
import {ToastProvider} from '@/components/lime/ui/toast';
import {LimePortalProvider} from '@/lib/lime/portal';

export function Providers({children}: {children: React.ReactNode}) {
  useEffect(() => {
    if ('serviceWorker' in navigator && process.env.NODE_ENV === 'production') {
      navigator.serviceWorker.register('/sw.js').catch(() => {});
    }
  }, []);
  return (
    <LimePortalProvider>
      <ToastProvider>
        <div className="min-h-dvh bg-page text-fg">{children}</div>
      </ToastProvider>
    </LimePortalProvider>
  );
}
