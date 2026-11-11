'use server';

import {revalidatePath} from 'next/cache';
import {redirect} from 'next/navigation';
import {after} from 'next/server';
import {z} from 'zod';
import type {ItemCardData} from '@/components/app/item-card';
import {toCardData} from '@/lib/card-data';
import {aiConfigured, type ProposedCategory} from '@/lib/server/ai';
import {askLibrary} from '@/lib/server/search';
import {emailConfigured, sendDigest} from '@/lib/server/digest';
import {checkLinks} from '@/lib/server/linkcheck';
import {createApiToken, createSession, destroySession, requireUser} from '@/lib/server/auth';
import {hashPassword, verifyPassword} from '@/lib/server/crypto';
import {query, queryOne} from '@/lib/server/db';
import {deleteItem, deleteItems, moveItem, moveItems} from '@/lib/server/items';
import {importBookmarksFile, importGitHubStars, ImportError} from '@/lib/server/importers';
import {
  applyTaxonomy, deleteCategory, describeAiError, dismissProposal, organizeInBackground, proposeTaxonomy, renameCategory,
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
  redirect(safeNext(form.get('next')));
}

export async function signInAction(_: FormState, form: FormData): Promise<FormState> {
  const parsed = credentials.safeParse({email: form.get('email'), password: form.get('password')});
  if (!parsed.success) return {error: 'Enter your email and password.'};
  const user = await queryOne<{id: string; password_hash: string}>('select id, password_hash from users where lower(email) = $1', [parsed.data.email]);
  if (!user || !(await verifyPassword(parsed.data.password, user.password_hash))) {
    return {error: 'That email and password do not match an account.'};
  }
  await createSession(user.id);
  redirect(safeNext(form.get('next')));
}

/** Only paths on this site, so a crafted link cannot send someone elsewhere after they sign in. */
function safeNext(value: FormDataEntryValue | null): string {
  const next = typeof value === 'string' ? value : '';
  return next.startsWith('/') && !next.startsWith('//') && !next.startsWith('/\\') ? next : '/library';
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

const idList = z.array(z.string().uuid()).max(500);

export async function moveItemsAction(itemIds: string[], categoryId: string | null): Promise<number> {
  const user = await requireUser();
  const moved = await moveItems(user.id, idList.parse(itemIds), categoryId === null ? null : z.string().uuid().parse(categoryId));
  revalidatePath('/library');
  return moved;
}

export async function deleteItemsAction(itemIds: string[]): Promise<number> {
  const user = await requireUser();
  const removed = await deleteItems(user.id, idList.parse(itemIds));
  revalidatePath('/library');
  return removed;
}

export type AskResult = {answer: string; sources: ItemCardData[]} | {error: string};

export async function askAction(question: string): Promise<AskResult> {
  const user = await requireUser();
  const q = question.trim().slice(0, 500);
  if (!q) return {error: 'Type a question first.'};
  if (!aiConfigured()) return {error: 'Add OPENROUTER_API_KEY to the server environment to ask your saves.'};
  try {
    const {answer, sources} = await askLibrary(user.id, q);
    return {answer, sources: sources.map(toCardData)};
  } catch (err) {
    return {error: describeAiError(err)};
  }
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
    after(() => organizeInBackground(user.id));
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
  after(() => organizeInBackground(user.id));
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
  after(() => organizeInBackground(user.id));
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

const MAX_IMPORT_FILE = 20 * 1024 * 1024;

export async function importBookmarksFileAction(form: FormData): Promise<{error?: string; added?: number; existing?: number}> {
  const user = await requireUser();
  const file = form.get('file');
  if (!(file instanceof File) || file.size === 0) return {error: 'Choose a file to import.'};
  if (file.size > MAX_IMPORT_FILE) return {error: 'That file is over 20 MB. Export only the bookmarks, not the whole profile.'};
  try {
    const r = await importBookmarksFile(user.id, file.name, await file.text());
    if (r.added > 0) after(() => organizeInBackground(user.id));
    revalidatePath('/library');
    revalidatePath('/settings');
    return {added: r.added, existing: r.existing};
  } catch (err) {
    if (err instanceof ImportError) return {error: err.message};
    throw err;
  }
}

export async function importGitHubStarsAction(_: FormState, form: FormData): Promise<FormState> {
  const user = await requireUser();
  try {
    const r = await importGitHubStars(user.id, String(form.get('username') ?? ''));
    if (r.added > 0) after(() => organizeInBackground(user.id));
    revalidatePath('/library');
    revalidatePath('/settings');
    return {message: r.found === 0
      ? 'That account has no starred repositories yet.'
      : `Imported ${r.added} new starred ${r.added === 1 ? 'repository' : 'repositories'}${r.existing ? `, ${r.existing} you already had` : ''}.`};
  } catch (err) {
    if (err instanceof ImportError) return {error: err.message};
    throw err;
  }
}

export async function setDigestAction(enabled: boolean): Promise<void> {
  const user = await requireUser();
  await query('update users set digest_enabled = $2 where id = $1', [user.id, enabled]);
  revalidatePath('/settings');
}

export async function sendDigestNowAction(): Promise<{error?: string; message?: string}> {
  const user = await requireUser();
  if (!emailConfigured()) return {error: 'Set RESEND_API_KEY and STASH_EMAIL_FROM on the server to send email.'};
  try {
    const sent = await sendDigest(user.id);
    return {message: sent ? `Sent to ${user.email}.` : 'Nothing new was saved this week, so there was nothing to send.'};
  } catch (err) {
    return {error: err instanceof Error ? err.message : 'Could not send the digest.'};
  }
}

export async function startLinkCheckAction(): Promise<void> {
  const user = await requireUser();
  after(() => checkLinks(user.id));
  revalidatePath('/settings');
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
