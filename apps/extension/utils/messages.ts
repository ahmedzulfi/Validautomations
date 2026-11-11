import type {IngestResponse, ItemInput, Source} from '@stash/shared';

export type CollectRequest = {type: 'stash:collect'; incremental: boolean};
export type CollectResult = {ok: true; found: number; added: number} | {ok: false; error: string; found: number; added: number};

export type ItemsMessage = {type: 'stash:items'; source: Source; items: ItemInput[]};
export type ItemsReply = {ok: true; result: IngestResponse} | {ok: false; error: string};

export type SyncMessage = {type: 'stash:sync'; source: SyncSource | 'all'};
export type SavePageMessage = {type: 'stash:save-page'; url: string; title: string};
export type SavePageReply = {ok: true; added: boolean} | {ok: false; error: string};

export type SyncSource = 'x' | 'linkedin' | 'reddit' | 'youtube' | 'instagram' | 'bookmark';

export type BackgroundMessage = ItemsMessage | SyncMessage | SavePageMessage;
