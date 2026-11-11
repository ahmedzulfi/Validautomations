import Anthropic from '@anthropic-ai/sdk';
import {zodOutputFormat} from '@anthropic-ai/sdk/helpers/zod';
import {z} from 'zod/v4';

// Building a person's category list is a judgment call over a sample, done rarely, so it uses the main
// model. Sorting every item is high-volume and simple, so it uses the small, fast one.
export const TAXONOMY_MODEL = process.env.STASH_TAXONOMY_MODEL || 'claude-opus-5-5';
export const CLASSIFY_MODEL = process.env.STASH_CLASSIFY_MODEL || 'claude-haiku-5-5';

/** Below this confidence an item goes to the inbox instead of a category. */
export const MIN_CONFIDENCE = 0.6;
export const UNCATEGORIZED = 'Uncategorized';

export function aiConfigured(): boolean {
  return Boolean(process.env.ANTHROPIC_API_KEY || process.env.ANTHROPIC_AUTH_TOKEN);
}

let cachedClient: Anthropic | null = null;
export function anthropic(): Anthropic {
  cachedClient ??= new Anthropic();
  return cachedClient;
}

/** What the model sees of one saved item. Long posts are cut to keep each batch small. */
export type ItemForAi = {
  source: string;
  title: string | null;
  body: string | null;
  authorName: string | null;
  url: string;
  folderPath: string[] | null;
};

export function describeItem(item: ItemForAi, maxBody = 600): string {
  const parts: string[] = [`source: ${item.source}`];
  if (item.authorName) parts.push(`author: ${item.authorName}`);
  if (item.folderPath?.length) parts.push(`browser folder: ${item.folderPath.join(' / ')}`);
  let host = '';
  try {
    host = new URL(item.url).hostname.replace(/^www\./, '');
  } catch {}
  if (host) parts.push(`site: ${host}`);
  if (item.title) parts.push(`title: ${item.title}`);
  const body = item.body && item.body !== item.title ? item.body.replace(/\s+/g, ' ').trim() : '';
  if (body) parts.push(`text: ${body.length > maxBody ? body.slice(0, maxBody) + '…' : body}`);
  return parts.join('\n');
}

export class AiRefusalError extends Error {}
export class AiEmptyError extends Error {}

const TaxonomySchema = z.object({
  categories: z.array(z.object({
    name: z.string().describe('2 to 4 words, sentence case, no emoji'),
    description: z.string().describe('One sentence saying what belongs here'),
  })),
});
export type ProposedCategory = {name: string; description: string};

/**
 * Reads a sample of someone's saves and proposes categories that fit what this person actually saves.
 * The person reviews them before anything is sorted.
 */
export async function suggestTaxonomy(client: Anthropic, sample: ItemForAi[], existing: string[] = []): Promise<ProposedCategory[]> {
  const listing = sample.map((item, i) => `<item n="${i + 1}">\n${describeItem(item, 300)}\n</item>`).join('\n');
  const response = await client.messages.parse({
    model: TAXONOMY_MODEL,
    max_tokens: 16000,
    output_config: {effort: 'medium', format: zodOutputFormat(TaxonomySchema)},
    system:
      'You organize a person\'s saved posts and bookmarks into a small set of categories. ' +
      'Propose 8 to 15 categories that cover most of their saves, based on what they actually save, not generic topics. ' +
      'Each category should be distinct enough that an item clearly belongs to one. ' +
      'Prefer the person\'s own interests and projects over broad buckets like "Technology" or "Misc". ' +
      'Names are 2 to 4 words in sentence case with no emoji. Do not include an "Uncategorized" or "Other" category.',
    messages: [{
      role: 'user',
      content:
        (existing.length ? `They already have these categories, keep the ones that still fit: ${existing.join(', ')}.\n\n` : '') +
        `Here are ${sample.length} of their saved items:\n\n${listing}`,
    }],
  });
  if (response.stop_reason === 'refusal') throw new AiRefusalError('The model declined to suggest categories.');
  const parsed = response.parsed_output;
  if (!parsed) throw new AiEmptyError('The model returned no categories.');
  return cleanCategories(parsed.categories);
}

export function cleanCategories(categories: ProposedCategory[]): ProposedCategory[] {
  const seen = new Set<string>();
  const out: ProposedCategory[] = [];
  for (const c of categories) {
    const name = c.name.replace(/\s+/g, ' ').trim().slice(0, 60);
    const key = name.toLowerCase();
    if (!name || seen.has(key) || key === 'uncategorized' || key === 'other') continue;
    seen.add(key);
    out.push({name, description: c.description.trim().slice(0, 300)});
  }
  return out.slice(0, 20);
}

export type Classification = {
  category: string | null;
  confidence: number;
  tags: string[];
  summary: string;
};

export type Correction = {item: ItemForAi; category: string};

/**
 * Sorts a batch of items into the person's categories. Returns one result per input item, in order. An
 * item the model is unsure about comes back with category null so it lands in the inbox.
 */
export async function classifyBatch(
  client: Anthropic,
  categories: ProposedCategory[],
  items: ItemForAi[],
  corrections: Correction[] = [],
): Promise<Classification[]> {
  if (items.length === 0) return [];
  const names = categories.map((c) => c.name);
  const BatchSchema = z.object({
    results: z.array(z.object({
      n: z.number().int().describe('The item number'),
      category: z.enum([UNCATEGORIZED, ...names] as [string, ...string[]]),
      confidence: z.number().describe('0 to 1: how sure you are the category is right'),
      tags: z.array(z.string()).describe('Up to 5 short lower-case topic tags'),
      summary: z.string().describe('One line, under 120 characters, saying what this is about'),
    })),
  });

  const categoryList = categories.map((c) => `- ${c.name}: ${c.description}`).join('\n');
  const examples = corrections.length
    ? '\n\nThe person moved these items themselves, so follow their lead on similar items:\n' +
      corrections.map((c) => `<example category="${c.category}">\n${describeItem(c.item, 200)}\n</example>`).join('\n')
    : '';
  const listing = items.map((item, i) => `<item n="${i + 1}">\n${describeItem(item)}\n</item>`).join('\n');

  const response = await client.messages.parse({
    model: CLASSIFY_MODEL,
    max_tokens: 16000,
    output_config: {effort: 'low', format: zodOutputFormat(BatchSchema)},
    system:
      'You sort a person\'s saved posts and bookmarks into their own categories.\n\n' +
      `Their categories:\n${categoryList}\n\n` +
      `Pick the single best category for each item. If none fits, or the item has too little text to tell, use "${UNCATEGORIZED}". ` +
      'Give a confidence from 0 to 1, up to 5 short lower-case tags, and a one-line summary in plain words. ' +
      'Return exactly one result for every item number.' + examples,
    messages: [{role: 'user', content: listing}],
  });
  if (response.stop_reason === 'refusal') throw new AiRefusalError('The model declined to sort this batch.');
  const parsed = response.parsed_output;
  if (!parsed) throw new AiEmptyError('The model returned no results.');

  const byN = new Map(parsed.results.map((r) => [r.n, r]));
  return items.map((_, i) => {
    const r = byN.get(i + 1);
    if (!r) return {category: null, confidence: 0, tags: [], summary: ''};
    const confidence = Math.max(0, Math.min(1, r.confidence));
    const placed = r.category !== UNCATEGORIZED && names.includes(r.category) && confidence >= MIN_CONFIDENCE;
    return {
      category: placed ? r.category : null,
      confidence,
      tags: [...new Set(r.tags.map((t) => t.toLowerCase().replace(/^#/, '').trim()).filter(Boolean))].slice(0, 5),
      summary: r.summary.trim().slice(0, 200),
    };
  });
}
