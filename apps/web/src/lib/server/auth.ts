import 'server-only';
import {cookies} from 'next/headers';
import {redirect} from 'next/navigation';
import {cache} from 'react';
import {query, queryOne} from './db';
import {randomToken, sha256} from './crypto';

export const SESSION_COOKIE = 'stash_session';
const SESSION_DAYS = 30;

export type User = {id: string; email: string};

export async function createSession(userId: string): Promise<void> {
  const token = randomToken();
  const expires = new Date(Date.now() + SESSION_DAYS * 24 * 60 * 60 * 1000);
  await query('insert into sessions (token_hash, user_id, expires_at) values ($1, $2, $3)', [sha256(token), userId, expires]);
  const jar = await cookies();
  jar.set(SESSION_COOKIE, token, {
    httpOnly: true,
    sameSite: 'lax',
    secure: process.env.NODE_ENV === 'production',
    path: '/',
    expires,
  });
}

export async function destroySession(): Promise<void> {
  const jar = await cookies();
  const token = jar.get(SESSION_COOKIE)?.value;
  if (token) await query('delete from sessions where token_hash = $1', [sha256(token)]);
  jar.delete(SESSION_COOKIE);
}

async function userFromSessionToken(token: string | undefined): Promise<User | null> {
  if (!token) return null;
  return queryOne<User>(
    `select u.id, u.email from sessions s join users u on u.id = s.user_id
     where s.token_hash = $1 and s.expires_at > now()`,
    [sha256(token)],
  );
}

/** The signed-in person for this request, from the session cookie. Cached per request. */
export const currentUser = cache(async (): Promise<User | null> => {
  const jar = await cookies();
  return userFromSessionToken(jar.get(SESSION_COOKIE)?.value);
});

export async function requireUser(): Promise<User> {
  const user = await currentUser();
  if (!user) redirect('/signin');
  return user;
}

export const API_TOKEN_PREFIX = 'stash_';

export async function createApiToken(userId: string, name: string): Promise<string> {
  const token = randomToken(API_TOKEN_PREFIX);
  await query('insert into api_tokens (user_id, name, token_hash, token_hint) values ($1, $2, $3, $4)', [
    userId, name, sha256(token), token.slice(-4),
  ]);
  return token;
}

async function userFromApiToken(token: string): Promise<User | null> {
  return queryOne<User>(
    `update api_tokens t set last_used_at = now() from users u
     where t.token_hash = $1 and u.id = t.user_id
     returning u.id, u.email`,
    [sha256(token)],
  );
}

/** For API routes: a Bearer token from the extension, or the session cookie from the web app. */
export async function authenticateRequest(request: Request): Promise<User | null> {
  const header = request.headers.get('authorization') ?? '';
  const match = header.match(/^Bearer\s+(\S+)$/i);
  if (match) return userFromApiToken(match[1]!);
  const cookie = request.headers.get('cookie') ?? '';
  const session = cookie.split(/;\s*/).find((c) => c.startsWith(`${SESSION_COOKIE}=`))?.slice(SESSION_COOKIE.length + 1);
  return userFromSessionToken(session ? decodeURIComponent(session) : undefined);
}
