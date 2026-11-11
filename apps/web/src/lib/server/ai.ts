import {createOpenRouter} from '@openrouter/ai-sdk-provider';
import {embedMany, generateText, JSONParseError, NoObjectGeneratedError, NoOutputGeneratedError, Output, TypeValidationError, type LanguageModel} from 'ai';
import {z} from 'zod/v4';

// Calls go through OpenRouter with the Vercel AI SDK, so any model OpenRouter offers can be used.
// Building a person's category list is a judgment call over a sample, done rarely, so it uses a large
// model. Sorting every item is high-volume and simple, so it uses a small, fast one.
export const TAXONOMY_MODEL = process.env.STASH_TAXONOMY_MODEL || 'anthropic/claude-opus-5.5';
export const CLASSIFY_MODEL = process.env.STASH_CLASSIFY_MODEL || 'anthropic/claude-haiku-5.5';

/** Below this confidence an item goes to the inbox instead of a category. */
export const MIN_CONFIDENCE = 0.6;
export const UNCATEGORIZED = 'Uncategorized';

export function aiConfigured(): boolean {
  return Boolean(process.env.OPENROUTER_API_KEY);
}

/** Turns a model id into a model to call. Tests pass their own. */
export type ModelPicker = (modelId: string) => LanguageModel;

let provider: ReturnType<typeof createOpenRouter> | null = null;
export function openRouterModels(): ModelPicker {
  provider ??= createOpenRouter({
    apiKey: process.env.OPENROUTER_API_KEY,
    baseURL: process.env.OPENROUTER_BASE_URL || undefined,
    appName: 'Stash',
    appUrl: process.env.STASH_PUBLIC_URL || undefined,
  });
  const p = provider;
  // Only route to providers that do not store or train on what is sent (OpenRouter's data policy filter).
  return (modelId) => p(modelId, {provider: {data_collection: 'deny'}});
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

/** The model declined, or its answer did not fit the schema. Retrying the same input will not help. */
export class AiNoAnswerError extends Error {}

type StructuredCall<T> = {
  model: LanguageModel;
  schema: z.ZodType<T>;
  system: string;
  prompt: string;
  effort: 'low' | 'medium' | 'high';
};

async function structured<T>({model, schema, system, prompt, effort}: StructuredCall<T>): Promise<T> {
  try {
    const result = await generateText({
      model,
      system,
      prompt,
      output: Output.object({schema}),
      maxOutputTokens: 16000,
      maxRetries: 2,
      providerOptions: {openrouter: {reasoning: {effort}}},
    });
    if (result.finishReason === 'content-filter') throw new AiNoAnswerError('The model declined this request.');
    return result.output as T;
  } catch (err) {
    if (err instanceof AiNoAnswerError) throw err;
    // A refusal, or an answer that is not valid JSON for the schema. Retrying the same input will not help.
    if (NoObjectGeneratedError.isInstance(err) || NoOutputGeneratedError.isInstance(err) || TypeValidationError.isInstance(err) || JSONParseError.isInstance(err)) {
      throw new AiNoAnswerError('The model returned an answer that could not be read.');
    }
    throw err;
  }
}

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
export async function suggestTaxonomy(models: ModelPicker, sample: ItemForAi[], existing: string[] = []): Promise<ProposedCategory[]> {
  const listing = sample.map((item, i) => `<item n="${i + 1}">\n${describeItem(item, 300)}\n</item>`).join('\n');
  const parsed = await structured({
    model: models(TAXONOMY_MODEL),
    schema: TaxonomySchema,
    effort: 'medium',
    system:
      'You organize a person\'s saved posts and bookmarks into a small set of categories. ' +
      'Propose 8 to 15 categories that cover most of their saves, based on what they actually save, not generic topics. ' +
      'Each category should be distinct enough that an item clearly belongs to one. ' +
      'Prefer the person\'s own interests and projects over broad buckets like "Technology" or "Misc". ' +
      'Names are 2 to 4 words in sentence case with no emoji. Do not include an "Uncategorized" or "Other" category.',
    prompt:
      (existing.length ? `They already have these categories, keep the ones that still fit: ${existing.join(', ')}.\n\n` : '') +
      `Here are ${sample.length} of their saved items:\n\n${listing}`,
  });
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
  models: ModelPicker,
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

  const parsed = await structured({
    model: models(CLASSIFY_MODEL),
    schema: BatchSchema,
    effort: 'low',
    system:
      'You sort a person\'s saved posts and bookmarks into their own categories.\n\n' +
      `Their categories:\n${categoryList}\n\n` +
      `Pick the single best category for each item. If none fits, or the item has too little text to tell, use "${UNCATEGORIZED}". ` +
      'Give a confidence from 0 to 1, up to 5 short lower-case tags, and a one-line summary in plain words. ' +
      'Return exactly one result for every item number.' + examples,
    prompt: listing,
  });

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

// ---- Search by meaning ----------------------------------------------------------------------------

export const EMBEDDING_MODEL = process.env.STASH_EMBEDDING_MODEL || 'openai/text-embedding-3-small';
/** Must match the vector column (db/migrations/003). Models that take a size are asked for this one. */
export const EMBEDDING_DIMENSIONS = 1536;

/** Turns texts into vectors, one per text, in order. Tests pass their own. */
export type Embedder = (values: string[]) => Promise<number[][]>;

export function openRouterEmbedder(): Embedder {
  provider ??= createOpenRouter({
    apiKey: process.env.OPENROUTER_API_KEY,
    baseURL: process.env.OPENROUTER_BASE_URL || undefined,
    appName: 'Stash',
    appUrl: process.env.STASH_PUBLIC_URL || undefined,
  });
  const model = provider.textEmbeddingModel(EMBEDDING_MODEL, {
    extraBody: {dimensions: EMBEDDING_DIMENSIONS},
    provider: {data_collection: 'deny'},
  });
  return async (values) => {
    const {embeddings} = await embedMany({model, values, maxRetries: 2, maxParallelCalls: 2});
    for (const e of embeddings) {
      if (e.length !== EMBEDDING_DIMENSIONS) {
        throw new Error(`The embedding model returned ${e.length} numbers, not ${EMBEDDING_DIMENSIONS}. Pick a model that can return ${EMBEDDING_DIMENSIONS}.`);
      }
    }
    return embeddings;
  };
}

/** The text an item is embedded from: what it says, plus what the AI learned about it. */
export function embeddingText(item: ItemForAi & {summary?: string | null; tags?: string[]}): string {
  const parts = [describeItem(item, 1500)];
  if (item.summary) parts.push(`summary: ${item.summary}`);
  if (item.tags?.length) parts.push(`tags: ${item.tags.join(', ')}`);
  return parts.join('\n');
}

export function toVectorLiteral(v: number[]): string {
  return `[${v.map((n) => (Number.isFinite(n) ? n : 0)).join(',')}]`;
}

// ---- Ask your saves -------------------------------------------------------------------------------

export const ASK_MODEL = process.env.STASH_ASK_MODEL || TAXONOMY_MODEL;

const AnswerSchema = z.object({
  answer: z.string().describe('A short answer in plain words, citing saves like [2]'),
  cited: z.array(z.number().int()).describe('The numbers of the saves the answer uses'),
});

/**
 * Answers a question from the person's own saves only. The saves are numbered; the answer cites them,
 * and says so when the saves do not answer the question.
 */
export async function answerFromSaves(models: ModelPicker, question: string, saves: ItemForAi[]): Promise<{answer: string; cited: number[]}> {
  const listing = saves.map((item, i) => `<save n="${i + 1}">\n${describeItem(item, 900)}\n</save>`).join('\n');
  const parsed = await structured({
    model: models(ASK_MODEL),
    schema: AnswerSchema,
    effort: 'low',
    system:
      'You answer questions using only the person\'s saved posts and bookmarks given below. ' +
      'Cite the saves you use with their numbers in brackets, like [2]. ' +
      'If the saves do not answer the question, say so plainly and point to the closest ones. ' +
      'Keep it to a few sentences or a short list. No emoji.\n\n' + listing,
    prompt: question,
  });
  return {
    answer: parsed.answer.trim(),
    cited: [...new Set(parsed.cited.filter((n) => n >= 1 && n <= saves.length))],
  };
}
