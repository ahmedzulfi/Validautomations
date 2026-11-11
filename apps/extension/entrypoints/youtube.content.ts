import {extractYtInitialData, parseYouTubePlaylist} from '@stash/shared';
import {scrollToEnd, startContentSync} from '../utils/content-sync';

export default defineContentScript({
  matches: ['https://www.youtube.com/*'],
  runAt: 'document_start',
  main() {
    startContentSync({
      source: 'youtube',
      tag: 'youtube',
      parse: parseYouTubePlaylist,
      loadMore: scrollToEnd,
      // Watch later and Liked videos are private: logged out, YouTube shows a sign-in link instead.
      loggedOutReason: () =>
        document.querySelector('a[href*="accounts.google.com/ServiceLogin"]') && !document.querySelector('ytd-playlist-video-renderer')
          ? 'Log in to YouTube in this browser, then sync again.'
          : null,
      // The first page of the playlist is in the HTML, assigned to ytInitialData.
      readEmbedded: () => {
        for (const script of document.querySelectorAll('script')) {
          const text = script.textContent ?? '';
          if (!text.includes('ytInitialData')) continue;
          const data = extractYtInitialData(text);
          if (data) return [data];
        }
        return [];
      },
    });
  },
});
