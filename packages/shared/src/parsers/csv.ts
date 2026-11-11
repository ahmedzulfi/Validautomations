import type {ItemInput} from '../types';
import {canonicalUrl} from '../url';
import {isHttpUrl, toIso} from './util';

/** Splits CSV text into rows of cells, following RFC 4180 quoting. */
export function parseCsv(text: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let cell = '';
  let quoted = false;
  const input = text.replace(/^﻿/, '');
  for (let i = 0; i < input.length; i++) {
    const ch = input[i]!;
    if (quoted) {
      if (ch === '"' && input[i + 1] === '"') {
        cell += '"';
        i++;
      } else if (ch === '"') quoted = false;
      else cell += ch;
    } else if (ch === '"') quoted = true;
    else if (ch === ',') {
      row.push(cell);
      cell = '';
    } else if (ch === '\n' || ch === '\r') {
      if (ch === '\r' && input[i + 1] === '\n') i++;
      row.push(cell);
      rows.push(row);
      row = [];
      cell = '';
    } else cell += ch;
  }
  if (cell || row.length) {
    row.push(cell);
    rows.push(row);
  }
  return rows.filter((r) => r.some((c) => c.trim()));
}

/**
 * Reads a bookmarks export in CSV: Pocket (title, url, time_added, tags), Raindrop.io (title, url,
 * folder, tags, created, excerpt, note) or any CSV with a url column. Returns the items and which format
 * it recognised.
 */
export function parseBookmarksCsv(text: string): {items: ItemInput[]; format: 'pocket' | 'raindrop' | 'csv' | null} {
  const [header, ...rows] = parseCsv(text);
  if (!header) return {items: [], format: null};
  const cols = header.map((h) => h.trim().toLowerCase());
  const col = (...names: string[]) => cols.findIndex((c) => names.includes(c));
  const urlCol = col('url', 'link', 'href', 'address');
  if (urlCol < 0) return {items: [], format: null};
  const format = cols.includes('time_added') ? 'pocket' : cols.includes('folder') && cols.includes('created') ? 'raindrop' : 'csv';
  const titleCol = col('title', 'name');
  const tagsCol = col('tags');
  const folderCol = col('folder', 'collection');
  const textCols = ['excerpt', 'description', 'note', 'notes'].map((n) => cols.indexOf(n)).filter((i) => i >= 0);
  const dateCol = col('time_added', 'created', 'created_at', 'date', 'added');

  const seen = new Set<string>();
  const items: ItemInput[] = [];
  for (const row of rows) {
    const url = row[urlCol]?.trim();
    if (!isHttpUrl(url)) continue;
    const id = canonicalUrl(url);
    if (seen.has(id)) continue;
    seen.add(id);
    const rawDate = dateCol >= 0 ? row[dateCol]?.trim() : undefined;
    const savedAt = rawDate ? (/^\d{9,10}$/.test(rawDate) ? toIso(Number(rawDate) * 1000) : /^\d{12,13}$/.test(rawDate) ? toIso(Number(rawDate)) : toIso(rawDate)) : undefined;
    const folder = folderCol >= 0 ? row[folderCol]?.trim() : '';
    const tags = tagsCol >= 0 ? row[tagsCol]?.split(/[|,]/).map((t) => t.trim()).filter(Boolean) ?? [] : [];
    const note = [...new Set(textCols.map((i) => row[i]?.trim()).filter(Boolean))].join('\n');
    const body = [note, tags.length ? `Tags: ${tags.join(', ')}` : ''].filter(Boolean).join('\n');
    const title = titleCol >= 0 ? row[titleCol]?.trim() : '';
    items.push({
      source: 'bookmark',
      sourceId: id,
      url,
      title: title && title !== url ? title.slice(0, 1000) : null,
      text: body ? body.slice(0, 20000) : null,
      folderPath: folder && folder !== 'Unsorted' && folder !== 'Unread' ? folder.split('/').map((f) => f.trim()).filter(Boolean).slice(0, 20) : null,
      savedAt: savedAt ?? null,
    });
  }
  return {items, format};
}
