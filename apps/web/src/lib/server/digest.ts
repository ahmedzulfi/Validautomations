import 'server-only';
import {SOURCE_LABELS, type Source} from '@stash/shared';
import {query, queryOne} from './db';
import {resurfaceItems, type Item} from './items';

export type Digest = {
  email: string;
  newCount: number;
  categories: Array<{name: string; count: number}>;
  picks: Item[];
  resurfaced: Item[];
};

export function emailConfigured(): boolean {
  return Boolean(process.env.RESEND_API_KEY && process.env.STASH_EMAIL_FROM);
}

/** What a person saved in the last week, the newest few, and a few older ones worth another look. */
export async function buildDigest(userId: string): Promise<Digest | null> {
  const user = await queryOne<{email: string}>('select email from users where id = $1', [userId]);
  if (!user) return null;
  const [count, categories, picks, resurfaced] = await Promise.all([
    queryOne<{n: number}>(`select count(*)::int as n from items where user_id = $1 and imported_at > now() - interval '7 days'`, [userId]),
    query<{name: string; count: number}>(
      `select coalesce(c.name, 'Inbox') as name, count(*)::int as count
       from items i left join categories c on c.id = i.category_id
       where i.user_id = $1 and i.imported_at > now() - interval '7 days'
       group by 1 order by 2 desc, 1 limit 6`,
      [userId],
    ),
    query<Item>(
      `select id, sources, url, title, body, author_name as "authorName", summary
       from items where user_id = $1 and imported_at > now() - interval '7 days'
       order by (summary is not null) desc, coalesce(saved_at, imported_at) desc limit 5`,
      [userId],
    ),
    resurfaceItems(userId, 3, 60),
  ]);
  return {email: user.email, newCount: count?.n ?? 0, categories, picks, resurfaced};
}

const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
const clip = (s: string, n: number) => (s.length > n ? `${s.slice(0, n - 1).trimEnd()}…` : s);

function itemTitle(i: Item): string {
  const first = i.title ?? i.body?.split('\n').find((l) => l.trim()) ?? i.url;
  return clip(first.trim(), 110);
}

function itemMeta(i: Item): string {
  return [i.sources.map((s: Source) => SOURCE_LABELS[s]).join(', '), i.authorName].filter(Boolean).join(' · ');
}

/** The digest as an email: inline styles in Lime's colours, since email clients ignore stylesheets. */
export function renderDigest(d: Digest, baseUrl: string): {subject: string; html: string; text: string} {
  const subject = d.newCount === 1 ? 'You saved 1 thing this week' : `You saved ${d.newCount} things this week`;
  const font = `font-family:'Hanken Grotesk',-apple-system,'Segoe UI',sans-serif`;
  const row = (i: Item) => `
    <tr><td style="padding:12px 0;border-top:1px solid #e6e4df">
      <a href="${esc(i.url)}" style="color:#0f0f0d;font-size:16px;font-weight:500;text-decoration:none">${esc(itemTitle(i))}</a>
      <div style="color:#6b6a63;font-size:14px;margin-top:2px">${esc(itemMeta(i))}</div>
      ${i.summary ? `<div style="color:#4a4943;font-size:14px;margin-top:4px">${esc(i.summary)}</div>` : ''}
    </td></tr>`;
  const section = (title: string, items: Item[]) => items.length
    ? `<h2 style="font-size:18px;font-weight:600;margin:28px 0 4px">${esc(title)}</h2><table role="presentation" width="100%" cellspacing="0" cellpadding="0">${items.map(row).join('')}</table>`
    : '';
  const breakdown = d.categories.length
    ? `<p style="color:#4a4943;font-size:16px;margin:8px 0 0">Mostly ${d.categories.map((c) => `${esc(c.name.toLowerCase())} (${c.count})`).join(', ')}.</p>`
    : '';
  const html = `<!doctype html><html><body style="margin:0;background:#ffffff">
<div style="max-width:560px;margin:0 auto;padding:32px 20px;${font};color:#0f0f0d">
  <p style="font-size:18px;font-weight:600;margin:0 0 24px">Stash</p>
  <h1 style="font-size:24px;line-height:1.2;font-weight:600;margin:0">${esc(subject)}</h1>
  ${breakdown}
  ${section('The newest', d.picks)}
  ${section('From your archive', d.resurfaced)}
  <p style="margin:32px 0 0"><a href="${esc(baseUrl)}/library" style="display:inline-block;background:#e4f222;color:#0f0f0d;font-weight:500;font-size:16px;padding:12px 20px;border-radius:999px;text-decoration:none">Open your library</a></p>
  <p style="color:#6b6a63;font-size:14px;margin:32px 0 0">You get this because the weekly digest is on. <a href="${esc(baseUrl)}/settings#digest" style="color:#4a4943">Turn it off</a>.</p>
</div></body></html>`;
  const lines = (title: string, items: Item[]) => items.length ? `\n${title}\n${items.map((i) => `- ${itemTitle(i)}\n  ${i.url}`).join('\n')}\n` : '';
  const text = `${subject}\n${d.categories.length ? `Mostly ${d.categories.map((c) => `${c.name.toLowerCase()} (${c.count})`).join(', ')}.\n` : ''}${lines('The newest', d.picks)}${lines('From your archive', d.resurfaced)}\nOpen your library: ${baseUrl}/library\nTurn the digest off: ${baseUrl}/settings#digest\n`;
  return {subject, html, text};
}

type Sender = (message: {to: string; subject: string; html: string; text: string}) => Promise<void>;

/** Sends through Resend's HTTP API. */
export const resendSender: Sender = async ({to, subject, html, text}) => {
  const response = await fetch('https://api.resend.com/emails', {
    method: 'POST',
    headers: {authorization: `Bearer ${process.env.RESEND_API_KEY}`, 'content-type': 'application/json'},
    body: JSON.stringify({from: process.env.STASH_EMAIL_FROM, to, subject, html, text}),
  });
  if (!response.ok) throw new Error(`The email service answered ${response.status}: ${(await response.text()).slice(0, 200)}`);
};

export function publicBaseUrl(): string {
  return (process.env.STASH_PUBLIC_URL ?? (process.env.VERCEL_PROJECT_PRODUCTION_URL ? `https://${process.env.VERCEL_PROJECT_PRODUCTION_URL}` : 'http://localhost:3000')).replace(/\/$/, '');
}

/** Sends one person's digest now, whatever their schedule. Returns false when there was nothing new. */
export async function sendDigest(userId: string, send: Sender = resendSender, baseUrl = publicBaseUrl()): Promise<boolean> {
  const digest = await buildDigest(userId);
  if (!digest || digest.newCount === 0) return false;
  const {subject, html, text} = renderDigest(digest, baseUrl);
  await send({to: digest.email, subject, html, text});
  await query('update users set digest_last_sent_at = now() where id = $1', [userId]);
  return true;
}

/** The weekly run: everyone who turned the digest on and has not had one in six days. */
export async function sendDueDigests(send: Sender = resendSender, baseUrl = publicBaseUrl()): Promise<{sent: number; skipped: number; failed: number}> {
  const due = await query<{id: string}>(
    `select id from users where digest_enabled and (digest_last_sent_at is null or digest_last_sent_at < now() - interval '6 days')`,
  );
  const result = {sent: 0, skipped: 0, failed: 0};
  for (const {id} of due) {
    try {
      if (await sendDigest(id, send, baseUrl)) result.sent++;
      else result.skipped++;
    } catch {
      result.failed++;
    }
  }
  return result;
}
