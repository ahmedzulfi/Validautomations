import {describe, expect, it} from 'vitest';
import {buildDigest, renderDigest, sendDigest, sendDueDigests} from '@/lib/server/digest';
import {ingestItems} from '@/lib/server/items';
import {makeUser, query} from './helpers';

describe('weekly digest', () => {
  it('sums up the week, escapes what people saved, and sends only to those who want it', async () => {
    const user = await makeUser();
    const other = await makeUser();
    await query('update users set digest_enabled = true where id = $1', [user]);
    await ingestItems(user, [
      {source: 'x', sourceId: '1', url: 'https://x.com/i/status/1', text: 'Partial indexes <script>alert(1)</script>', authorName: 'Arun'},
      {source: 'bookmark', sourceId: 'old', url: 'https://example.com/old', title: 'An old favourite'},
    ]);
    await query(`update items set imported_at = now() - interval '90 days', saved_at = now() - interval '90 days' where user_id = $1 and url = 'https://example.com/old'`, [user]);
    await ingestItems(other, [{source: 'x', sourceId: '2', url: 'https://x.com/i/status/2', text: 'Not for this digest'}]);

    const digest = await buildDigest(user);
    expect(digest).toMatchObject({newCount: 1, categories: [{name: 'Inbox', count: 1}]});
    expect(digest!.resurfaced.map((i) => i.title)).toEqual(['An old favourite']);

    const {subject, html, text} = renderDigest(digest!, 'https://stash.example.com');
    expect(subject).toBe('You saved 1 thing this week');
    expect(html).not.toContain('<script>');
    expect(html).toContain('&lt;script&gt;');
    expect(text).toContain('https://stash.example.com/library');

    const sent: string[] = [];
    const sender = async (m: {to: string}) => {
      sent.push(m.to);
    };
    expect(await sendDueDigests(sender)).toEqual({sent: 1, skipped: 0, failed: 0});
    expect(sent).toHaveLength(1);
    // Not again within the week.
    expect(await sendDueDigests(sender)).toEqual({sent: 0, skipped: 0, failed: 0});
  });

  it('sends nothing when nothing new was saved', async () => {
    const user = await makeUser();
    expect(await sendDigest(user, async () => {
      throw new Error('should not send');
    })).toBe(false);
  });
});
