import type {IngestResponse, ItemInput, Source} from '@stash/shared';
import {Collector} from './collector';
import {STASH_MESSAGE, type CaptureMessage} from './intercept';
import type {CollectRequest, CollectResult, ItemsMessage, ItemsReply} from './messages';

type Options = {
  source: Source;
  tag: string;
  parse: (data: unknown) => ItemInput[];
  loadMore: () => void;
  loggedOutReason: () => string | null;
  /** Data the page embedded in its HTML before any fetch, read when a sync starts. */
  readEmbedded?: () => unknown[];
};

/**
 * The isolated-world half of an importer: receives what the page-world interceptor captured, and when
 * the background asks, scrolls the list and sends items on through the background.
 */
export function startContentSync(o: Options): void {
  const collector = new Collector({
    parse: o.parse,
    loadMore: o.loadMore,
    loggedOutReason: o.loggedOutReason,
    send: async (items): Promise<IngestResponse> => {
      const message: ItemsMessage = {type: 'stash:items', source: o.source, items};
      const reply = (await browser.runtime.sendMessage(message)) as ItemsReply | undefined;
      if (!reply) throw new Error('The extension did not answer. Reload the page and try again.');
      if (!reply.ok) throw new Error(reply.error);
      return reply.result;
    },
  });

  window.addEventListener('message', (event) => {
    if (event.source !== window) return;
    const data = event.data as CaptureMessage | undefined;
    if (!data || data[STASH_MESSAGE] !== true || data.tag !== o.tag) return;
    collector.accept(data.data);
  });

  let running: Promise<CollectResult> | null = null;
  browser.runtime.onMessage.addListener((message: unknown, _sender, sendResponse: (r: CollectResult) => void) => {
    const m = message as CollectRequest;
    if (m?.type !== 'stash:collect') return undefined;
    if (!running) {
      for (const embedded of o.readEmbedded?.() ?? []) collector.accept(embedded);
      running = collector.run(m.incremental).finally(() => {
        running = null;
      });
    }
    void running.then(sendResponse);
    // Keeps the channel open until sendResponse runs (works on every Chrome version).
    return true;
  });
}

export function scrollToEnd(): void {
  window.scrollTo({top: document.documentElement.scrollHeight, behavior: 'smooth'});
}
