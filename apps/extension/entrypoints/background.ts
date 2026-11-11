import {canonicalUrl, flattenBookmarkTree, type ItemInput} from '@stash/shared';
import {ingest, reportSync} from '../utils/api';
import {instagramPager, pageThrough, redditPager} from '../utils/fetch-sync';
import type {BackgroundMessage, CollectRequest, CollectResult, ItemsReply, SavePageReply, SyncSource} from '../utils/messages';
import {getSettings, getStatus, setStatus, type Settings} from '../utils/settings';

type PageSource = 'x' | 'linkedin' | 'youtube';
const PAGES: Record<PageSource, string[]> = {
  x: ['https://x.com/i/bookmarks'],
  linkedin: ['https://www.linkedin.com/my-items/saved-posts/'],
  youtube: ['https://www.youtube.com/playlist?list=WL', 'https://www.youtube.com/playlist?list=LL'],
};
const AUTO_SYNC_ALARM = 'stash-auto-sync';
const SYNC_ORDER: SyncSource[] = ['bookmark', 'x', 'linkedin', 'reddit', 'youtube', 'instagram'];

export default defineBackground(() => {
  browser.runtime.onInstalled.addListener(() => {
    void scheduleAutoSync();
  });
  browser.runtime.onStartup.addListener(() => {
    void scheduleAutoSync();
  });

  browser.runtime.onMessage.addListener((raw: unknown, _sender, sendResponse: (reply: unknown) => void) => {
    const message = raw as BackgroundMessage;
    let reply: Promise<unknown>;
    if (message?.type === 'stash:items') reply = receiveItems(message.items);
    else if (message?.type === 'stash:save-page') reply = savePage(message.url, message.title);
    else if (message?.type === 'stash:sync') {
      // Runs on after the popup closes; progress is written to storage, which the popup reads.
      void (message.source === 'all' ? syncAll(true) : syncOne(message.source, true));
      reply = Promise.resolve({ok: true});
    } else return undefined;
    void reply.then(sendResponse);
    // Keeps the channel open until sendResponse runs (works on every Chrome version).
    return true;
  });

  browser.alarms.onAlarm.addListener((alarm) => {
    if (alarm.name === AUTO_SYNC_ALARM) void autoSync();
  });

  // A bookmark made while connected goes straight to the library.
  browser.bookmarks.onCreated.addListener(async (_id, node) => {
    const settings = await getSettings();
    if (!settings || !node.url || !/^https?:/.test(node.url)) return;
    await ingest(settings, [{source: 'bookmark', sourceId: canonicalUrl(node.url), url: node.url, title: node.title || null, savedAt: new Date(node.dateAdded ?? Date.now()).toISOString()}]).catch(() => {});
  });

  browser.storage.onChanged.addListener((changes) => {
    if (changes.settings) void scheduleAutoSync();
  });
});

async function scheduleAutoSync(): Promise<void> {
  const settings = await getSettings();
  await browser.alarms.clear(AUTO_SYNC_ALARM);
  if (settings?.autoSync) await browser.alarms.create(AUTO_SYNC_ALARM, {periodInMinutes: 24 * 60, delayInMinutes: 60});
}

async function autoSync(): Promise<void> {
  const settings = await getSettings();
  if (settings?.autoSync) await syncAll(false);
}

async function receiveItems(items: ItemInput[]): Promise<ItemsReply> {
  const settings = await getSettings();
  if (!settings) return {ok: false, error: 'Connect the extension to your Stash server first.'};
  try {
    return {ok: true, result: await ingest(settings, items)};
  } catch (err) {
    return {ok: false, error: err instanceof Error ? err.message : 'Could not send items.'};
  }
}

async function syncAll(manual: boolean): Promise<void> {
  for (const source of SYNC_ORDER) await syncOne(source, manual);
}

const running = new Set<SyncSource>();

async function syncOne(source: SyncSource, manual: boolean): Promise<void> {
  const settings = await getSettings();
  if (!settings || running.has(source)) return;
  running.add(source);
  await setStatus(source, {state: 'running', found: 0, added: 0, message: null});
  let result: CollectResult;
  try {
    const incremental = (await getStatus(source)).lastSuccessAt !== null;
    const send = async (items: ItemInput[]) => ingest(settings, items);
    if (source === 'bookmark') result = await syncBookmarks(settings);
    else if (source === 'reddit') result = await pageThrough({fetchPage: redditPager(), send, incremental});
    else if (source === 'instagram') result = await pageThrough({fetchPage: instagramPager(), send, incremental, delayMs: 2500, maxPages: 60});
    else result = await syncFromPages(source, manual, incremental);
  } catch (err) {
    result = {ok: false, error: err instanceof Error ? err.message : 'Sync stopped.', found: 0, added: 0};
  } finally {
    running.delete(source);
  }
  await setStatus(source, result.ok
    ? {state: 'done', found: result.found, added: result.added, message: null, lastSuccessAt: Date.now()}
    : {state: 'failed', found: result.found, added: result.added, message: result.error});
  await reportSync(settings, {source, itemsSeen: result.found, itemsAdded: result.added, error: result.ok ? null : result.error});
}

async function syncBookmarks(settings: Settings): Promise<CollectResult> {
  const tree = await browser.bookmarks.getTree();
  const items = flattenBookmarkTree(tree);
  const r = await ingest(settings, items);
  return {ok: true, found: items.length, added: r.added};
}

/** Collects each of a source's saved lists in turn, adding up what was found. */
async function syncFromPages(source: PageSource, manual: boolean, incremental: boolean): Promise<CollectResult> {
  let found = 0;
  let added = 0;
  for (const url of PAGES[source]) {
    const r = await syncFromPage(url, manual, incremental);
    found += r.found;
    added += r.added;
    if (!r.ok) return {...r, found, added};
  }
  return {ok: true, found, added};
}

/**
 * Opens a saved list (or uses a tab already showing it), and asks the content script there to collect
 * it. A manual sync shows the tab so the person sees what is happening; the daily sync keeps it in the
 * background. A tab Stash opened is closed afterwards.
 */
async function syncFromPage(url: string, manual: boolean, incremental: boolean): Promise<CollectResult> {
  const existing = (await browser.tabs.query({url: `${url.split('?')[0]}*`})).find((t) => t.url?.startsWith(url));
  const tab = existing ?? (await browser.tabs.create({url, active: manual}));
  if (!tab.id) throw new Error('Could not open the page.');
  if (existing) await browser.tabs.reload(tab.id);
  await waitForTabLoad(tab.id);
  try {
    const request: CollectRequest = {type: 'stash:collect', incremental};
    return await sendWithRetry<CollectResult>(tab.id, request);
  } finally {
    if (!existing) await browser.tabs.remove(tab.id).catch(() => {});
  }
}

function waitForTabLoad(tabId: number, timeoutMs = 45000): Promise<void> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      browser.tabs.onUpdated.removeListener(listener);
      reject(new Error('The page took too long to load. Check your connection and try again.'));
    }, timeoutMs);
    const listener = (id: number, info: {status?: string}) => {
      if (id === tabId && info.status === 'complete') {
        clearTimeout(timer);
        browser.tabs.onUpdated.removeListener(listener);
        resolve();
      }
    };
    browser.tabs.onUpdated.addListener(listener);
    void browser.tabs.get(tabId).then((t) => {
      if (t.status === 'complete') listener(tabId, {status: 'complete'});
    });
  });
}

async function sendWithRetry<T>(tabId: number, message: unknown, tries = 10): Promise<T> {
  for (let i = 0; i < tries; i++) {
    try {
      const reply = (await browser.tabs.sendMessage(tabId, message)) as T | undefined;
      if (reply !== undefined) return reply;
    } catch {
      // The content script is not listening yet.
    }
    await new Promise((r) => setTimeout(r, 1000));
  }
  throw new Error('The page did not respond. Reload it and sync again.');
}

async function savePage(url: string, title: string): Promise<SavePageReply> {
  const settings = await getSettings();
  if (!settings) return {ok: false, error: 'Connect the extension to your Stash server first.'};
  if (!/^https?:/.test(url)) return {ok: false, error: 'Only web pages can be saved.'};
  try {
    const r = await ingest(settings, [{source: 'web', sourceId: canonicalUrl(url), url, title: title || null, savedAt: new Date().toISOString()}]);
    return {ok: true, added: r.added > 0};
  } catch (err) {
    return {ok: false, error: err instanceof Error ? err.message : 'Could not save this page.'};
  }
}
