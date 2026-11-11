'use server';

import {chunk, MAX_INGEST_BATCH, parseBookmarksHtml} from '@stash/shared';
import {revalidatePath} from 'next/cache';
import {redirect} from 'next/navigation';
import {after} from 'next/server';
import {z} from 'zod';
import {aiConfigured, type ProposedCategory} from '@/lib/server/ai';
import {createApiToken, createSession, destroySession, requireUser} from '@/lib/server/auth';
import {hashPassword, verifyPassword} from '@/lib/server/crypto';
import {query, queryOne} from '@/lib/server/db';
import {deleteItem, ingestItems, moveItem} from '@/lib/server/items';
import {
  applyTaxonomy, deleteCategory, describeAiError, dismissProposal, proposeTaxonomy, renameCategory, sortPending,
} from '@/lib/server/organize';

export type FormState = {error?: string; message?: string} | undefined;

const credentials = z.object({
  email: z.string().trim().toLowerCase().email('Enter an email address, like you@example.com.'),
  password: z.string().min(8, 'Use at least 8 characters for the password.').max(200),
});

export async function signUpAction(_: FormState, form: FormData): Promise<FormState> {
  const parsed = credentials.safeParse({email: form.get('email'), password: form.get('password')});
  if (!parsed.success) return {error: parsed.error.issues[0]!.message};
  const {email, password} = parsed.data;
  const existing = await queryOne('select id from users where lower(email) = $1', [email]);
  if (existing) return {error: 'There is already an account with that email. Sign in instead.'};
  const user = await queryOne<{id: string}>('insert into users (email, password_hash) values ($1, $2) returning id', [email, await hashPassword(password)]);
  await createSession(user!.id);
  redirect('/library');
}

export async function signInAction(_: FormState, form: FormData): Promise<FormState> {
  const parsed = credentials.safeParse({email: form.get('email'), password: form.get('password')});
  if (!parsed.success) return {error: 'Enter your email and password.'};
  const user = await queryOne<{id: string; password_hash: string}>('select id, password_hash from users where lower(email) = $1', [parsed.data.email]);
  if (!user || !(await verifyPassword(parsed.data.password, user.password_hash))) {
    return {error: 'That email and password do not match an account.'};
  }
  await createSession(user.id);
  redirect('/library');
}

export async function signOutAction(): Promise<void> {
  await destroySession();
  redirect('/signin');
}

export async function moveItemAction(itemId: string, categoryId: string | null): Promise<void> {
  const user = await requireUser();
  await moveItem(user.id, itemId, categoryId);
  revalidatePath('/library');
}

export async function deleteItemAction(itemId: string): Promise<void> {
  const user = await requireUser();
  await deleteItem(user.id, itemId);
  revalidatePath('/library');
}

export type SuggestResult = {proposalId: string; categories: ProposedCategory[]} | {error: string};

export async function suggestCategoriesAction(): Promise<SuggestResult> {
  const user = await requireUser();
  if (!aiConfigured()) return {error: 'Add OPENROUTER_API_KEY to the server environment to get suggestions. You can still add categories by hand.'};
  try {
    const proposal = await proposeTaxonomy(user.id);
    revalidatePath('/categories');
    return {proposalId: proposal.id, categories: proposal.categories};
  } catch (err) {
    return {error: describeAiError(err)};
  }
}

const categoryList = z.array(z.object({name: z.string().max(80), description: z.string().max(400)})).max(40);

export async function applyCategoriesAction(proposalId: string | null, categories: ProposedCategory[]): Promise<{error?: string; created?: number; removed?: number}> {
  const user = await requireUser();
  const parsed = categoryList.safeParse(categories);
  if (!parsed.success) return {error: 'Some categories are not valid. Check the names and try again.'};
  try {
    const result = await applyTaxonomy(user.id, parsed.data, proposalId);
    after(() => sortPending(user.id));
    revalidatePath('/library');
    revalidatePath('/categories');
    return result;
  } catch (err) {
    return {error: err instanceof Error ? err.message : 'Could not save the categories.'};
  }
}

export async function dismissProposalAction(proposalId: string): Promise<void> {
  const user = await requireUser();
  await dismissProposal(user.id, proposalId);
  revalidatePath('/categories');
}

export async function startSortingAction(): Promise<{error?: string}> {
  const user = await requireUser();
  if (!aiConfigured()) return {error: 'Add OPENROUTER_API_KEY to the server environment to sort with AI.'};
  await query('update users set sorting_error = null where id = $1', [user.id]);
  after(() => sortPending(user.id));
  revalidatePath('/library');
  return {};
}

export async function renameCategoryAction(id: string, name: string, description: string): Promise<{error?: string}> {
  const user = await requireUser();
  try {
    await renameCategory(user.id, id, name, description);
  } catch (err) {
    return {error: isUniqueViolation(err) ? 'You already have a category with that name.' : err instanceof Error ? err.message : 'Could not rename it.'};
  }
  revalidatePath('/library');
  revalidatePath('/categories');
  return {};
}

export async function deleteCategoryAction(id: string): Promise<void> {
  const user = await requireUser();
  await deleteCategory(user.id, id);
  after(() => sortPending(user.id));
  revalidatePath('/library');
  revalidatePath('/categories');
}

export async function createTokenAction(_: FormState, form: FormData): Promise<FormState & {token?: string}> {
  const user = await requireUser();
  const name = String(form.get('name') ?? '').trim().slice(0, 60) || 'Browser extension';
  const token = await createApiToken(user.id, name);
  revalidatePath('/settings');
  return {token, message: 'Copy this token now. It will not be shown again.'};
}

export async function revokeTokenAction(id: string): Promise<void> {
  const user = await requireUser();
  await query('delete from api_tokens where user_id = $1 and id = $2', [user.id, id]);
  revalidatePath('/settings');
}

const MAX_BOOKMARK_FILE = 20 * 1024 * 1024;

export async function importBookmarksFileAction(form: FormData): Promise<{error?: string; added?: number; existing?: number}> {
  const user = await requireUser();
  const file = form.get('file');
  if (!(file instanceof File) || file.size === 0) return {error: 'Choose a bookmarks file to import.'};
  if (file.size > MAX_BOOKMARK_FILE) return {error: 'That file is over 20 MB. Export only the bookmarks, not the whole profile.'};
  const items = parseBookmarksHtml(await file.text());
  if (items.length === 0) return {error: 'No bookmarks found in that file. Use the HTML file your browser exports.'};
  let added = 0;
  let existing = 0;
  for (const batch of chunk(items, MAX_INGEST_BATCH)) {
    const r = await ingestItems(user.id, batch);
    added += r.added;
    existing += r.existing;
  }
  await query('insert into sync_runs (user_id, source, items_seen, items_added) values ($1, $2, $3, $4)', [user.id, 'bookmark', items.length, added]);
  if (added > 0) after(() => sortPending(user.id));
  revalidatePath('/library');
  revalidatePath('/settings');
  return {added, existing};
}

export async function deleteAllDataAction(): Promise<void> {
  const user = await requireUser();
  await query('delete from items where user_id = $1', [user.id]);
  await query('delete from categories where user_id = $1', [user.id]);
  await query('delete from taxonomy_proposals where user_id = $1', [user.id]);
  await query('delete from sync_runs where user_id = $1', [user.id]);
  revalidatePath('/library');
  redirect('/library');
}

function isUniqueViolation(err: unknown): boolean {
  return typeof err === 'object' && err !== null && (err as {code?: string}).code === '23505';
}
