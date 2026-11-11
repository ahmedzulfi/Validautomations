import type {IngestResponse, ItemInput} from '@stash/shared';
import type {CollectResult} from './messages';

export type CollectorOptions = {
  /** Turns one captured response into items. */
  parse: (data: unknown) => ItemInput[];
  /** Sends a batch to the server and says how many were new. */
  send: (items: ItemInput[]) => Promise<IngestResponse>;
  /** Asks the page for more, by scrolling or pressing its "show more" button. */
  loadMore: () => void;
  /** A message when the page shows the person is not logged in, else null. */
  loggedOutReason: () => string | null;
  sleep?: (ms: number) => Promise<void>;
  /** Pause between scrolls, plus a random extra so it reads like a person. */
  roundDelayMs?: number;
  /** Rounds in a row with nothing new before deciding the list has ended. */
  maxIdleRounds?: number;
  maxItems?: number;
  maxRounds?: number;
};

/**
 * Gathers a saved list by scrolling it the way a person would, collecting what the page loads. In an
 * incremental sync it stops at the first batch that is entirely already in the library, since the
 * list is newest first.
 */
export class Collector {
  private seen = new Set<string>();
  private pending: ItemInput[] = [];
  private readonly o: Required<CollectorOptions>;

  constructor(options: CollectorOptions) {
    this.o = {
      sleep: (ms) => new Promise((r) => setTimeout(r, ms)),
      roundDelayMs: 1500,
      maxIdleRounds: 4,
      maxItems: 5000,
      maxRounds: 600,
      ...options,
    };
  }

  /** Called for every captured response, before and during a run. */
  accept(data: unknown): number {
    let added = 0;
    for (const item of this.o.parse(data)) {
      if (this.seen.has(item.sourceId)) continue;
      this.seen.add(item.sourceId);
      this.pending.push(item);
      added++;
    }
    return added;
  }

  async run(incremental: boolean): Promise<CollectResult> {
    let found = 0;
    let added = 0;
    let idle = 0;
    try {
      for (let round = 0; round < this.o.maxRounds; round++) {
        const loggedOut = this.o.loggedOutReason();
        if (loggedOut) return {ok: false, error: loggedOut, found, added};

        if (this.pending.length) {
          const batch = this.pending.splice(0, this.pending.length);
          const result = await this.o.send(batch);
          found += batch.length;
          added += result.added;
          idle = 0;
          if (incremental && result.added === 0) break;
          if (found >= this.o.maxItems) break;
        } else if (round > 0) {
          idle++;
          if (idle >= this.o.maxIdleRounds) break;
        }

        this.o.loadMore();
        await this.o.sleep(this.o.roundDelayMs + Math.random() * this.o.roundDelayMs * 0.6);
      }
      if (this.pending.length) {
        const batch = this.pending.splice(0, this.pending.length);
        const result = await this.o.send(batch);
        found += batch.length;
        added += result.added;
      }
      return {ok: true, found, added};
    } catch (err) {
      return {ok: false, error: err instanceof Error ? err.message : 'Sync stopped.', found, added};
    }
  }
}
