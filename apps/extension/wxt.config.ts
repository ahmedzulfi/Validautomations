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
  manifest: {
    name: 'Stash',
    description: 'Brings your X bookmarks, LinkedIn saved posts and browser bookmarks into your Stash library.',
    permissions: ['bookmarks', 'storage', 'alarms', 'tabs'],
    host_permissions: ['https://x.com/*', 'https://www.linkedin.com/*'],
    action: {default_title: 'Stash'},
  },
});
