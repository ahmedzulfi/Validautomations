import path from 'node:path';
import {defineConfig} from 'vitest/config';

export default defineConfig({
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
      'server-only': path.resolve(__dirname, 'test/empty.ts'),
    },
  },
  test: {
    include: ['test/**/*.test.ts'],
    globalSetup: ['test/global-setup.ts'],
    env: {DATABASE_URL: process.env.TEST_DATABASE_URL ?? 'postgres://stash:stash@localhost:5432/stash_test'},
    fileParallelism: false,
    testTimeout: 20000,
  },
});
