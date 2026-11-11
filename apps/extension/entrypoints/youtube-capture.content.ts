import {installInterceptor} from '../utils/intercept';

// Page world, before YouTube's app starts: copy the playlist pages it loads while scrolling.
export default defineContentScript({
  matches: ['https://www.youtube.com/*'],
  runAt: 'document_start',
  world: 'MAIN',
  main() {
    installInterceptor('youtube', (url) => url.includes('/youtubei/v1/browse'));
  },
});
