import {ingestRequestSchema} from '@stash/shared';
import {after} from 'next/server';
import {authenticateRequest} from '@/lib/server/auth';
import {error, json, preflight} from '@/lib/server/http';
import {ingestItems} from '@/lib/server/items';
import {organizeInBackground} from '@/lib/server/organize';

export const runtime = 'nodejs';
export const maxDuration = 300;

export function OPTIONS() {
  return preflight();
}

/** Receives a batch of saves from the extension (or any client with a token) and adds them. */
export async function POST(request: Request) {
  const user = await authenticateRequest(request);
  if (!user) return error(401, 'Sign in again: the token is missing or was revoked.');
  let body: unknown;
  try {
    body = await request.json();
  } catch {
    return error(400, 'The request body is not JSON.');
  }
  const parsed = ingestRequestSchema.safeParse(body);
  if (!parsed.success) return error(400, `Some items are not valid: ${parsed.error.issues.slice(0, 3).map((i) => `${i.path.join('.')}: ${i.message}`).join('; ')}`);
  const result = await ingestItems(user.id, parsed.data.items);
  if (result.added > 0) after(() => organizeInBackground(user.id));
  return json(result);
}
