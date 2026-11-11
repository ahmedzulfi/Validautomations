import type {ItemCardData} from '@/components/app/item-card';
import type {Item} from '@/lib/server/items';

const dateFormat = new Intl.DateTimeFormat('en-US', {month: 'short', day: 'numeric', year: 'numeric', timeZone: 'UTC'});

export function toCardData(item: Item): ItemCardData {
  const date = item.postedAt ?? item.savedAt;
  let host = '';
  try {
    host = new URL(item.url).hostname.replace(/^www\./, '');
  } catch {}
  return {
    id: item.id,
    url: item.url,
    host,
    sources: item.sources,
    title: item.title,
    body: item.body || null,
    summary: item.summary,
    authorName: item.authorName,
    authorHandle: item.authorHandle,
    authorAvatar: item.authorAvatar,
    image: item.media?.find((m) => m.type === 'image')?.url ?? null,
    folderPath: item.folderPath,
    tags: item.tags,
    dateLabel: date ? dateFormat.format(new Date(date)) : null,
    categoryId: item.categoryId,
    placedBy: item.categoryAssignedBy,
  };
}
