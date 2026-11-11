import path from 'node:path';
import tailwindcss from '@tailwindcss/vite';
import {defineConfig} from 'wxt';

// The popup uses the web app's Lime components directly, so the two never drift apart. Lime's files
// import each other through @/components/lime and @/lib/lime, which point into apps/web here.
const web = path.resolve(__dirname, '../web/src');

export default defineConfig({
  alias: {
    '@/components/lime': path.join(web, 'components/lime'),
    '@/lib/lime': path.join(web, 'lib/lime'),
  },
  vite: () => ({
    plugins: [tailwindcss()],
    // The Lime files come from the web app, whose tsconfig leaves JSX to Next; compile it here.
    esbuild: {jsx: 'automatic'},
    build: {
      rollupOptions: {
        onwarn(warning, warn) {
          // Base UI marks its files "use client" for React Server Components; meaningless in an extension.
          if (warning.code === 'MODULE_LEVEL_DIRECTIVE' || warning.code === 'SOURCEMAP_ERROR') return;
          warn(warning);
        },
      },
    },
  }),
  manifest: ({browser}) => ({
    name: 'Stash',
    description: 'Brings your saves from X, LinkedIn, Reddit, YouTube, Instagram and your browser into your Stash library, sorted by AI.',
    permissions: ['bookmarks', 'storage', 'alarms', 'tabs'],
    host_permissions: ['https://x.com/*', 'https://www.linkedin.com/*', 'https://www.reddit.com/*', 'https://www.youtube.com/*', 'https://www.instagram.com/*'],
    action: {default_title: 'Stash'},
    ...(browser === 'firefox'
      ? {
          browser_specific_settings: {
            gecko: {
              // Change this id to your own before submitting to addons.mozilla.org.
              id: 'stash@stash.example',
              strict_min_version: '128.0',
              // Bookmarks and the content of saved posts go to the person's own Stash server.
              data_collection_permissions: {required: ['bookmarksInfo', 'websiteContent']},
            },
          },
        }
      : {}),
  }),
});
