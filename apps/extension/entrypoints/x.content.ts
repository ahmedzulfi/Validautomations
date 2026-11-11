import {parseXBookmarks} from '@stash/shared';
import {scrollToEnd, startContentSync} from '../utils/content-sync';

export default defineContentScript({
  matches: ['https://x.com/*'],
  runAt: 'document_start',
  main() {
    startContentSync({
      source: 'x',
      tag: 'x',
      parse: parseXBookmarks,
      loadMore: scrollToEnd,
      loggedOutReason: () =>
        /^\/(i\/flow\/login|login|logout)/.test(location.pathname)
          ? 'Log in to X in this browser, then sync again.'
          : null,
    });
  },
});
