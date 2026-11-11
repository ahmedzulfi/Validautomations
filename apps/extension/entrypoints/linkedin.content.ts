import {parseLinkedInSavedPosts} from '@stash/shared';
import {scrollToEnd, startContentSync} from '../utils/content-sync';

export default defineContentScript({
  matches: ['https://www.linkedin.com/*'],
  runAt: 'document_start',
  main() {
    startContentSync({
      source: 'linkedin',
      tag: 'linkedin',
      parse: parseLinkedInSavedPosts,
      loadMore: () => {
        // The saved list ends in a "Show more results" button rather than loading on scroll.
        const more = [...document.querySelectorAll('button')].find((b) => /show more results/i.test(b.textContent ?? ''));
        if (more && !more.disabled) more.click();
        else scrollToEnd();
      },
      loggedOutReason: () =>
        /^\/(login|authwall|checkpoint|uas\/login)/.test(location.pathname)
          ? 'Log in to LinkedIn in this browser, then sync again.'
          : null,
      // LinkedIn puts the first page of results in the HTML, in <code> elements holding JSON.
      readEmbedded: () => {
        const out: unknown[] = [];
        for (const el of document.querySelectorAll('code')) {
          const text = el.textContent?.trim();
          if (!text || !text.startsWith('{') || !text.includes('EntityResultViewModel')) continue;
          try {
            out.push(JSON.parse(text));
          } catch {
            // Not JSON.
          }
        }
        return out;
      },
    });
  },
});
