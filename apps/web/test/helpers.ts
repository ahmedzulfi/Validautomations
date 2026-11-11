import {MockLanguageModelV4} from 'ai/test';
import type {ModelPicker} from '@/lib/server/ai';
import {query, queryOne} from '@/lib/server/db';

export async function makeUser(email = `u${Math.random().toString(36).slice(2)}@example.com`): Promise<string> {
  const row = await queryOne<{id: string}>(`insert into users (email, password_hash) values ($1, 'x') returning id`, [email]);
  return row!.id;
}

export async function makeCategories(userId: string, names: string[]): Promise<Map<string, string>> {
  const map = new Map<string, string>();
  for (const [i, name] of names.entries()) {
    const row = await queryOne<{id: string}>(
      'insert into categories (user_id, name, description, sort_order) values ($1, $2, $3, $4) returning id',
      [userId, name, `About ${name.toLowerCase()}`, i],
    );
    map.set(name, row!.id);
  }
  return map;
}

export type FakeCall = {modelId: string; system: string; prompt: string};
type FakeAnswer = {output: unknown; finishReason?: 'stop' | 'content-filter'};

/**
 * Stands in for OpenRouter: every model call runs `respond` with the system text and prompt the app
 * sent, and returns its object as the model's JSON answer.
 */
export function fakeModels(respond: (call: FakeCall) => FakeAnswer | Promise<FakeAnswer>) {
  const calls: FakeCall[] = [];
  const models: ModelPicker = (modelId) => new MockLanguageModelV4({
    modelId,
    doGenerate: async (options) => {
      const system = options.prompt.filter((m) => m.role === 'system').map((m) => m.content).join('\n');
      const prompt = options.prompt
        .filter((m) => m.role === 'user')
        .flatMap((m) => m.content)
        .map((part) => (part.type === 'text' ? part.text : ''))
        .join('\n');
      const call = {modelId, system, prompt};
      calls.push(call);
      const answer = await respond(call);
      return {
        content: [{type: 'text', text: JSON.stringify(answer.output)}],
        finishReason: {unified: answer.finishReason ?? 'stop', raw: answer.finishReason ?? 'stop'},
        usage: {
          inputTokens: {total: 10, noCache: 10, cacheRead: 0, cacheWrite: 0},
          outputTokens: {total: 10, text: 10, reasoning: 0},
        },
        warnings: [],
      };
    },
  });
  return {models, calls};
}

/** Item numbers in a classify prompt, in order. */
export function itemNumbers(call: FakeCall): number[] {
  return [...call.prompt.matchAll(/<item n="(\d+)">/g)].map((m) => Number(m[1]));
}

export {query};
