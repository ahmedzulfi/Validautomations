/**
 * AI layer: Claude turns notes/prompts into structured decks and edits them.
 *
 * Runs in the browser with the user's own API key (bring-your-own-key).
 * The key is kept in this browser's localStorage and sent only to Anthropic.
 */
import Anthropic from "@anthropic-ai/sdk";
import { betaZodOutputFormat } from "@anthropic-ai/sdk/helpers/beta/zod";
import { z } from "zod";
import { normalizeSlide } from "./slides";
import { LAYOUTS, type Deck, type Slide } from "./types";

import { DEFAULT_MODEL, type AISettings } from "./ai-config";
export { DEFAULT_MODEL, MODELS, type AISettings } from "./ai-config";

// ---------- schema the model must fill ----------

const AISlide = z.object({
  layout: z.enum(LAYOUTS),
  title: z.string(),
  subtitle: z.string(),
  body: z.string(),
  bullets: z.array(z.string()),
  columns: z.array(z.object({ heading: z.string(), bullets: z.array(z.string()) })),
  stats: z.array(z.object({ value: z.string(), label: z.string() })),
  quote: z.string(),
  attribution: z.string(),
  chart: z.object({
    kind: z.enum(["bar", "line", "donut"]),
    labels: z.array(z.string()),
    values: z.array(z.number()),
    unit: z.string(),
  }),
  table: z.object({ headers: z.array(z.string()), rows: z.array(z.array(z.string())) }),
  timeline: z.array(z.object({ label: z.string(), text: z.string() })),
  imagePrompt: z.string(),
  notes: z.string(),
});

const AIDeck = z.object({
  title: z.string(),
  slides: z.array(AISlide),
});

type AISlideT = z.infer<typeof AISlide>;

const LAYOUT_GUIDE = `Layouts and the fields each one uses (leave every other field empty: "" or []; chart.kind "bar"):
- title: title, subtitle. Use once, first.
- section: title, subtitle. A chapter break in longer decks.
- bullets: title, optional body (one sentence), bullets (3-5, each under 12 words).
- two-column: title, columns (exactly 2, each with heading + 2-4 bullets). For comparisons, before/after, problem/solution.
- stats: title, stats (2-4 items; value is short like "3x", "$4.2M", "87%"; label is a few words).
- quote: quote, attribution. Customer voice, a striking statement.
- image: title, body (1-2 sentences), imagePrompt (a concrete description of a photo or illustration that would fit).
- chart: title, optional body, chart (labels and values of equal length, 3-8 points; unit like "%" or "$M" or "").
- table: title, table (headers + 2-5 rows, same number of cells per row). Pricing, feature comparison.
- timeline: title, timeline (3-6 items; label is a date/phase, text a few words). Roadmaps, history.
- closing: title, subtitle. The ask, next step or thank-you. Use once, last.`;

const SYSTEM = `You are a presentation designer and story editor. You turn raw material into clear, persuasive slide decks.

Principles:
- One idea per slide. Slide titles are short claims ("Churn fell 40% after onboarding v2"), not labels ("Results").
- Follow a story arc that suits the material, e.g. hook -> problem -> solution -> proof -> plan -> ask.
- Vary layouts. Prefer stats, charts, two-column, quotes and timelines over bullets when the content allows. Never more than two bullet slides in a row.
- Keep text tight: bullets under 12 words, no bullet slide with more than 5 bullets, no paragraphs on slides. Detail goes in speaker notes.
- Use facts and numbers from the provided material. If you need an illustrative number that is not in the material, say so in that slide's notes ("Illustrative figure").
- Speaker notes: 2-4 sentences the presenter can say out loud.
- Write in the language of the user's material.

${LAYOUT_GUIDE}`;

// ---------- client ----------

function client(settings: AISettings): Anthropic {
  if (!settings.apiKey) throw new AIError("Add your Anthropic API key in Settings to use AI features.");
  return new Anthropic({ apiKey: settings.apiKey, dangerouslyAllowBrowser: true });
}

export class AIError extends Error {}

/** Server-side refusal fallbacks exist for Opus/Sonnet 5.5, not Haiku. */
function fallbackParams(model: string) {
  return model.includes("haiku")
    ? {}
    : { betas: ["server-side-fallback-2026-07-01"], fallbacks: "default" as const };
}

export interface Attachment {
  name: string;
  /** "text" holds the file's text; "pdf" holds base64 (no data: prefix). */
  kind: "text" | "pdf";
  data: string;
}

type UserContent = string | Anthropic.Beta.Messages.BetaContentBlockParam[];

async function callStructured<T extends z.ZodType>(
  settings: AISettings,
  schema: T,
  user: UserContent,
  effort: "low" | "medium" | "high",
): Promise<z.infer<T>> {
  const c = client(settings);
  const model = settings.model || DEFAULT_MODEL;
  let response;
  try {
    response = await c.beta.messages.parse({
      model,
      max_tokens: 16000,
      system: SYSTEM,
      messages: [{ role: "user", content: user }],
      output_config: { effort, format: betaZodOutputFormat(schema) },
      ...fallbackParams(model),
    });
  } catch (err) {
    throw new AIError(describeError(err));
  }
  if (response.stop_reason === "refusal") {
    throw new AIError("Claude declined this request. Try rephrasing the material.");
  }
  if (response.stop_reason === "max_tokens") {
    throw new AIError("The deck was too long to finish in one go. Ask for fewer slides.");
  }
  if (!response.parsed_output) throw new AIError("Claude's reply couldn't be read as a deck. Please try again.");
  return response.parsed_output as z.infer<T>;
}

function describeError(err: unknown): string {
  if (err instanceof Anthropic.AuthenticationError) return "Your API key was rejected. Check it in Settings.";
  if (err instanceof Anthropic.PermissionDeniedError) return "This API key isn't allowed to use that model.";
  if (err instanceof Anthropic.NotFoundError) return "That model wasn't found. Pick another in Settings.";
  if (err instanceof Anthropic.RateLimitError) return "Rate limited by the API. Wait a moment and try again.";
  if (err instanceof Anthropic.APIConnectionError) return "Couldn't reach the Anthropic API. Check your connection.";
  if (err instanceof Anthropic.APIError) return `API error ${err.status ?? ""}: ${err.message}`;
  return err instanceof Error ? err.message : String(err);
}

function toSlide(s: AISlideT, keepId?: string): Slide {
  const slide = normalizeSlide({ ...s, image: { url: "", prompt: s.imagePrompt } });
  if (keepId) slide.id = keepId;
  return slide;
}

// ---------- public API ----------

export interface GenerateInput {
  prompt: string;
  source: string;
  slideCount: number;
  tone: string;
  audience: string;
  attachments?: Attachment[];
}

export async function generateDeck(input: GenerateInput, settings: AISettings): Promise<{ title: string; slides: Slide[] }> {
  const parts = [
    `Create a deck of about ${input.slideCount} slides.`,
    input.prompt && `What the deck is for:\n${input.prompt}`,
    input.audience && `Audience: ${input.audience}`,
    input.tone && `Tone: ${input.tone}`,
    input.source && `Source material (use it as the factual basis):\n<material>\n${input.source}\n</material>`,
  ].filter(Boolean);
  const files = input.attachments ?? [];
  for (const f of files.filter((f) => f.kind === "text")) {
    parts.push(`Attached file "${f.name}":\n<file>\n${f.data}\n</file>`);
  }
  const pdfs = files.filter((f) => f.kind === "pdf");
  if (pdfs.length) parts.push(`Attached PDFs (above): ${pdfs.map((f) => f.name).join(", ")}. Use them as source material.`);
  const content: UserContent = pdfs.length
    ? [
        ...pdfs.map(
          (f) =>
            ({
              type: "document",
              title: f.name,
              source: { type: "base64", media_type: "application/pdf", data: f.data },
            }) as Anthropic.Beta.Messages.BetaContentBlockParam,
        ),
        { type: "text", text: parts.join("\n\n") },
      ]
    : parts.join("\n\n");
  const out = await callStructured(settings, AIDeck, content, "medium");
  return { title: out.title, slides: out.slides.map((s) => toSlide(s)) };
}

function slideForPrompt(s: Slide) {
  const { id: _id, image, ...rest } = s;
  return { ...rest, imagePrompt: image.prompt };
}

export async function editSlide(slide: Slide, instruction: string, deck: Deck, settings: AISettings): Promise<Slide> {
  const context = deck.slides.map((s, i) => `${i + 1}. [${s.layout}] ${s.title}`).join("\n");
  const user = `Deck "${deck.title}" outline:\n${context}\n\nRewrite this one slide following the instruction. You may change its layout if that serves the instruction better.\n\nInstruction: ${instruction}\n\nCurrent slide JSON:\n${JSON.stringify(slideForPrompt(slide), null, 2)}`;
  const out = await callStructured(settings, AISlide, user, "low");
  const next = toSlide(out, slide.id);
  // Keep a user-chosen image unless the layout changed away from it.
  if (slide.image.url && next.layout === "image") next.image.url = slide.image.url;
  return next;
}

export async function editDeck(deck: Deck, instruction: string, settings: AISettings): Promise<{ title: string; slides: Slide[] }> {
  const user = `Revise this whole deck following the instruction. Keep slides that don't need to change as they are.\n\nInstruction: ${instruction}\n\nCurrent deck JSON:\n${JSON.stringify({ title: deck.title, slides: deck.slides.map(slideForPrompt) }, null, 2)}`;
  const out = await callStructured(settings, AIDeck, user, "medium");
  return { title: out.title, slides: out.slides.map((s) => toSlide(s)) };
}
