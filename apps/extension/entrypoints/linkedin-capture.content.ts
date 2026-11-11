import {installInterceptor} from '../utils/intercept';

// Page world, before LinkedIn's app starts: copy the Voyager responses that carry saved posts.
export default defineContentScript({
  matches: ['https://www.linkedin.com/*'],
  runAt: 'document_start',
  world: 'MAIN',
  main() {
    installInterceptor('linkedin', (url) => url.includes('/voyager/api/') && /search|graphql/i.test(url));
  },
});
