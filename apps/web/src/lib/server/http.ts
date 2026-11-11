import 'server-only';

// The browser extension calls these routes from its own origin (chrome-extension://...), so they answer
// CORS. They authenticate with a Bearer token, never cookies, so allowing any origin exposes nothing.
export const CORS_HEADERS: Record<string, string> = {
  'access-control-allow-origin': '*',
  'access-control-allow-methods': 'GET, POST, OPTIONS',
  'access-control-allow-headers': 'authorization, content-type',
  'access-control-max-age': '86400',
};

export function json(body: unknown, init: ResponseInit = {}): Response {
  return Response.json(body, {...init, headers: {...CORS_HEADERS, ...(init.headers ?? {})}});
}

export function error(status: number, message: string): Response {
  return json({error: message}, {status});
}

export function preflight(): Response {
  return new Response(null, {status: 204, headers: CORS_HEADERS});
}
