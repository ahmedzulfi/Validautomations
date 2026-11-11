import {describe, expect, it} from 'vitest';
import {ingestItems} from '@/lib/server/items';
import {checkLink, checkLinks, isPublicUrl, linkCheckState} from '@/lib/server/linkcheck';
import {makeUser, query} from './helpers';

const DNS: Record<string, string[]> = {
  'alive.example': ['93.184.216.34'],
  'gone.example': ['93.184.216.35'],
  'moved.example': ['93.184.216.36'],
  'flaky.example': ['93.184.216.37'],
  'nohead.example': ['93.184.216.38'],
  'sneaky.example': ['10.0.0.5'],
};
const resolve = async (host: string) => {
  const a = DNS[host];
  if (!a) throw new Error('ENOTFOUND');
  return a;
};
const calls: string[] = [];
const fakeFetch = async (url: string, init: RequestInit) => {
  calls.push(`${init.method} ${url}`);
  const host = new URL(url).hostname;
  if (host === 'alive.example') return new Response(null, {status: 200});
  if (host === 'gone.example') return new Response(null, {status: 404});
  if (host === 'moved.example') return new Response(null, {status: 301, headers: {location: url.includes('/old') ? 'https://alive.example/new' : 'http://sneaky.example/'}});
  if (host === 'flaky.example') return new Response(null, {status: 503});
  if (host === 'nohead.example') return new Response(null, {status: init.method === 'HEAD' ? 405 : 200});
  throw new Error('unexpected fetch ' + url);
};
const deps = {fetch: fakeFetch, resolve};

describe('isPublicUrl', () => {
  it.each([
    ['https://alive.example/', true],
    ['http://localhost:3000/', false],
    ['http://127.0.0.1/', false],
    ['http://169.254.169.254/latest/meta-data', false],
    ['http://[::1]/', false],
    ['http://192.168.1.1/', false],
    ['https://sneaky.example/', false],
    ['ftp://alive.example/', false],
  ])('%s → %s', async (url, expected) => {
    expect(await isPublicUrl(url, resolve)).toBe(expected);
  });
});

describe('checkLink', () => {
  it('reads the status, follows redirects and retries HEAD with GET', async () => {
    expect(await checkLink('https://alive.example/', deps)).toBe('ok');
    expect(await checkLink('https://gone.example/page', deps)).toBe('broken');
    expect(await checkLink('https://moved.example/old', deps)).toBe('ok');
    expect(await checkLink('https://nohead.example/', deps)).toBe('ok');
    expect(await checkLink('https://flaky.example/', deps)).toBe('unknown');
    expect(await checkLink('https://no-such-host.example/', deps)).toBe('broken');
  });

  it('never fetches a private address, even through a redirect', async () => {
    calls.length = 0;
    expect(await checkLink('https://moved.example/to-private', deps)).toBe('unknown');
    expect(calls.some((c) => c.includes('sneaky'))).toBe(false);
    expect(await checkLink('http://127.0.0.1:5432/', deps)).toBe('unknown');
  });
});

describe('checkLinks', () => {
  it('checks web links only, marks broken ones and keeps the old status when unsure', async () => {
    const user = await makeUser();
    await ingestItems(user, [
      {source: 'bookmark', sourceId: 'a', url: 'https://alive.example/a'},
      {source: 'bookmark', sourceId: 'g', url: 'https://gone.example/g'},
      {source: 'web', sourceId: 'f', url: 'https://flaky.example/f'},
      {source: 'x', sourceId: '1', url: 'https://x.com/i/status/1'},
    ]);
    expect(await checkLinks(user, deps)).toEqual({checked: 3, broken: 1});
    const rows = await query<{url: string; link_status: string | null}>('select url, link_status from items where user_id = $1 order by url', [user]);
    expect(rows).toEqual([
      {url: 'https://alive.example/a', link_status: 'ok'},
      {url: 'https://flaky.example/f', link_status: null},
      {url: 'https://gone.example/g', link_status: 'broken'},
      {url: 'https://x.com/i/status/1', link_status: null},
    ]);
    expect(await linkCheckState(user)).toEqual({running: false, checked: 3, broken: 1, due: 0});
    // Nothing is due again until next month.
    expect(await checkLinks(user, deps)).toEqual({checked: 0, broken: 0});
  });
});
