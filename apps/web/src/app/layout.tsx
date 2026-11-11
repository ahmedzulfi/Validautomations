import type {Metadata, Viewport} from 'next';
import '@/components/lime/styles.css';
import './globals.css';
import {Providers} from '@/components/app/providers';

export const metadata: Metadata = {
  title: 'Stash',
  description: 'Your X bookmarks, LinkedIn saved posts and browser bookmarks in one place, sorted for you.',
};

export const viewport: Viewport = {
  width: 'device-width',
  initialScale: 1,
};

// Lime's dark theme follows data-theme on <html>. This picks the saved choice, or the system setting,
// before the first paint.
const themeScript = `try{var t=localStorage.getItem('stash-theme');if(t!=='light'&&t!=='dark'){t=matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light'}document.documentElement.dataset.theme=t}catch(e){}`;

export default function RootLayout({children}: Readonly<{children: React.ReactNode}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <script dangerouslySetInnerHTML={{__html: themeScript}}/>
      </head>
      <body className="lime">
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
