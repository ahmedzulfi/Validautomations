import type {MetadataRoute} from 'next';

// Installing Stash from the browser adds it to the phone's share sheet: share a link from any app and
// it is saved to the library (see /share).
export default function manifest(): MetadataRoute.Manifest {
  return {
    name: 'Stash',
    short_name: 'Stash',
    description: 'Everything you saved, in one place, sorted for you.',
    start_url: '/library',
    display: 'standalone',
    background_color: '#ffffff',
    theme_color: '#ffffff',
    icons: [
      {src: '/icon-192.png', sizes: '192x192', type: 'image/png'},
      {src: '/icon-512.png', sizes: '512x512', type: 'image/png'},
      {src: '/icon-maskable-512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable'},
    ],
    share_target: {action: '/share', method: 'GET', params: {title: 'title', text: 'text', url: 'url'}},
  } as MetadataRoute.Manifest;
}
