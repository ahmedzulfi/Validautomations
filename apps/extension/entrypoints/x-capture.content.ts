import {installInterceptor} from '../utils/intercept';

// Page world, before X's app starts: copy the Bookmarks timeline responses X already loads.
export default defineContentScript({
  matches: ['https://x.com/*'],
  runAt: 'document_start',
  world: 'MAIN',
  main() {
    installInterceptor('x', (url) => /\/graphql\/[^/]+\/Bookmarks\b/.test(url));
  },
});
