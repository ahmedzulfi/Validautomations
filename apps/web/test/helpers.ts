import type Anthropic from '@anthropic-ai/sdk';
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

type ParseParams = {model: string; system?: unknown; messages: Array<{content: unknown}>};
type FakeResponse = {stop_reason: string; parsed_output: unknown};

/** A stand-in for the Anthropic client: each call to messages.parse runs `respond` on the request. */
export function fakeClient(respond: (params: ParseParams) => FakeResponse | Promise<FakeResponse>) {
  const calls: ParseParams[] = [];
  const client = {
    messages: {
      parse: async (params: ParseParams) => {
        calls.push(params);
        return respond(params);
      },
    },
  } as unknown as Anthropic;
  return {client, calls};
}

/** Item numbers in a classify prompt, in order. */
export function itemNumbers(params: ParseParams): number[] {
  const text = String(params.messages[0]!.content);
  return [...text.matchAll(/<item n="(\d+)">/g)].map((m) => Number(m[1]));
}

export {query};
