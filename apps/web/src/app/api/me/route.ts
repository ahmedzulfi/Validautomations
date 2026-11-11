import {authenticateRequest} from '@/lib/server/auth';
import {error, json, preflight} from '@/lib/server/http';
import {libraryStats} from '@/lib/server/items';

export const runtime = 'nodejs';

export function OPTIONS() {
  return preflight();
}

/** Lets the extension check its token and show who it is connected as. */
export async function GET(request: Request) {
  const user = await authenticateRequest(request);
  if (!user) return error(401, 'Sign in again: the token is missing or was revoked.');
  const stats = await libraryStats(user.id);
  return json({email: user.email, total: stats.total, bySource: stats.bySource});
}
