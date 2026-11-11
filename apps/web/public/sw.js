// Stash's service worker. It exists so the app can be installed and appear in the share sheet; it
// does not cache anything, so the library is always live.
self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', (event) => event.waitUntil(self.clients.claim()));
self.addEventListener('fetch', () => {});
