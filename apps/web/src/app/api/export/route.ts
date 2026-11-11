import {SOURCE_LABELS} from '@stash/shared';
import {authenticateRequest} from '@/lib/server/auth';
import {error} from '@/lib/server/http';
import {exportItems} from '@/lib/server/items';

export const runtime = 'nodejs';

/** Downloads the whole library as CSV or Markdown. */
export async function GET(request: Request) {
  const user = await authenticateRequest(request);
  if (!user) return error(401, 'Sign in first.');
  const format = new URL(request.url).searchParams.get('format') === 'md' ? 'md' : 'csv';
  const items = await exportItems(user.id);
  const date = new Date().toISOString().slice(0, 10);

  if (format === 'csv') {
    const header = ['title', 'url', 'category', 'tags', 'sources', 'author', 'summary', 'text', 'saved_at'];
    const lines = [header.join(',')];
    for (const i of items) {
      lines.push([
        i.title, i.url, i.category, i.tags.join(' '), i.sources.join(' '), i.authorName, i.summary, i.body, i.savedAt ?? i.importedAt,
      ].map(csvCell).join(','));
    }
    return new Response(lines.join('\n'), {
      headers: {'content-type': 'text/csv; charset=utf-8', 'content-disposition': `attachment; filename="stash-${date}.csv"`},
    });
  }

  const groups = new Map<string, typeof items>();
  for (const i of items) {
    const key = i.category ?? 'Not sorted yet';
    groups.set(key, [...(groups.get(key) ?? []), i]);
  }
  let md = `# My saves\n\nExported from Stash on ${date}. ${items.length} items.\n`;
  for (const [category, list] of groups) {
    md += `\n## ${category}\n\n`;
    for (const i of list) {
      const firstLine = i.body?.split('\n').find((l) => l.trim())?.trim();
      const raw = i.title ?? (firstLine && firstLine.length > 100 ? `${firstLine.slice(0, 99)}…` : firstLine) ?? i.url;
      const title = raw.replace(/[[\]]/g, '');
      const meta = [i.sources.map((s) => SOURCE_LABELS[s]).join(', '), i.authorName, i.tags.map((t) => `#${t}`).join(' ')].filter(Boolean).join(' · ');
      md += `- [${title}](${i.url})${meta ? ` (${meta})` : ''}${i.summary ? `\n  ${i.summary}` : ''}\n`;
    }
  }
  return new Response(md, {
    headers: {'content-type': 'text/markdown; charset=utf-8', 'content-disposition': `attachment; filename="stash-${date}.md"`},
  });
}

function csvCell(value: unknown): string {
  if (value === null || value === undefined) return '';
  const s = String(value);
  return /[",\n\r]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}
