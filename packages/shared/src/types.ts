import {z} from 'zod';

export const SOURCES = ['x', 'linkedin', 'bookmark', 'web'] as const;
export type Source = (typeof SOURCES)[number];

export const SOURCE_LABELS: Record<Source, string> = {
  x: 'X',
  linkedin: 'LinkedIn',
  bookmark: 'Bookmarks',
  web: 'Saved pages',
};

export const mediaSchema = z.object({
  type: z.enum(['image', 'video']),
  url: z.string().url(),
});
export type Media = z.infer<typeof mediaSchema>;

/** One saved thing, as an importer produces it, before it reaches the database. */
export const itemInputSchema = z.object({
  source: z.enum(SOURCES),
  /** The id the source uses (tweet id, activity urn, normalized url for bookmarks). */
  sourceId: z.string().min(1).max(500),
  url: z.string().url().max(4000),
  title: z.string().max(1000).nullish(),
  text: z.string().max(20000).nullish(),
  authorName: z.string().max(300).nullish(),
  authorHandle: z.string().max(300).nullish(),
  authorAvatar: z.string().url().max(4000).nullish(),
  media: z.array(mediaSchema).max(10).nullish(),
  links: z.array(z.string().url().max(4000)).max(20).nullish(),
  folderPath: z.array(z.string().max(300)).max(20).nullish(),
  /** When the person saved it, if the source says. ISO 8601. */
  savedAt: z.string().datetime({offset: true}).nullish(),
  /** When it was originally posted, if the source says. ISO 8601. */
  postedAt: z.string().datetime({offset: true}).nullish(),
});
export type ItemInput = z.infer<typeof itemInputSchema>;

export const MAX_INGEST_BATCH = 200;

export const ingestRequestSchema = z.object({
  items: z.array(itemInputSchema).min(1).max(MAX_INGEST_BATCH),
});
export type IngestRequest = z.infer<typeof ingestRequestSchema>;

export type IngestResponse = {
  /** Items that were new to this person's library. */
  added: number;
  /** Items they already had (merged, nothing lost). */
  existing: number;
};

export const syncReportSchema = z.object({
  source: z.enum(SOURCES),
  itemsSeen: z.number().int().min(0),
  itemsAdded: z.number().int().min(0),
  error: z.string().max(2000).nullish(),
});
export type SyncReport = z.infer<typeof syncReportSchema>;
