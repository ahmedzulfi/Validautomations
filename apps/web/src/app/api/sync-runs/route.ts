import {syncReportSchema} from '@stash/shared';
import {authenticateRequest} from '@/lib/server/auth';
import {error, json, preflight} from '@/lib/server/http';
import {recordSyncRun} from '@/lib/server/items';

export const runtime = 'nodejs';

export function OPTIONS() {
  return preflight();
}

/** The extension reports how a sync went, so the web app can show when each source last synced. */
export async function POST(request: Request) {
  const user = await authenticateRequest(request);
  if (!user) return error(401, 'Sign in again: the token is missing or was revoked.');
  const parsed = syncReportSchema.safeParse(await request.json().catch(() => null));
  if (!parsed.success) return error(400, 'The sync report is not valid.');
  await recordSyncRun(user.id, parsed.data);
  return json({ok: true});
}
