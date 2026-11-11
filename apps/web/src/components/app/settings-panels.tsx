'use client';

import {useActionState, useEffect, useState, useTransition} from 'react';
import {createTokenAction, deleteAllDataAction, importBookmarksFileAction, importGitHubStarsAction, revokeTokenAction, sendDigestNowAction, setDigestAction, startLinkCheckAction} from '@/app/actions';
import {useRouter} from 'next/navigation';
import {Switch} from '@/components/lime/ui/switch';
import {Progress} from '@/components/lime/ui/progress';
import {
  AlertDialog, AlertDialogClose, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader,
  AlertDialogTitle, AlertDialogTrigger,
} from '@/components/lime/ui/alert-dialog';
import {Button} from '@/components/lime/ui/button';
import {CopyButton} from '@/components/lime/ui/copy-button';
import {Dropzone} from '@/components/lime/ui/dropzone';
import {Field, FieldDescription, FieldLabel} from '@/components/lime/ui/field';
import {Input} from '@/components/lime/ui/input';
import {List, ListItem} from '@/components/lime/ui/list';
import {Segmented, SegmentedItem} from '@/components/lime/ui/segmented';
import {useToast} from '@/components/lime/ui/toast';

type Token = {id: string; name: string; tokenHint: string; createdAt: string; lastUsedAt: string | null};

export function TokenManager({serverUrl, tokens}: {serverUrl: string; tokens: Token[]}) {
  const [state, formAction, pending] = useActionState(createTokenAction, undefined);
  const [revoking, startRevoke] = useTransition();
  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2 rounded-field bg-surface px-4 py-3">
        <div className="flex min-w-0 flex-1 flex-col">
          <span className="text-sm text-fg-2">Server address</span>
          <code className="truncate font-mono text-sm text-fg">{serverUrl}</code>
        </div>
        <CopyButton value={serverUrl} label="Copy server address"/>
      </div>

      {state?.token ? (
        <div role="status" className="flex flex-col gap-2 rounded-field bg-accent-wash px-4 py-3">
          <p className="text-sm text-fg">{state.message}</p>
          <div className="flex items-center gap-2">
            <code className="min-w-0 flex-1 truncate font-mono text-sm text-fg">{state.token}</code>
            <CopyButton value={state.token} label="Copy token"/>
          </div>
        </div>
      ) : null}

      <form action={formAction} className="flex flex-col gap-3 sm:flex-row sm:items-end">
        <Field name="name" className="flex-1">
          <FieldLabel>Token name</FieldLabel>
          <Input name="name" placeholder="Chrome on my laptop" maxLength={60}/>
          <FieldDescription>So you can tell your devices apart later.</FieldDescription>
        </Field>
        <Button type="submit" pending={pending} className="sm:mb-6">Create token</Button>
      </form>

      {tokens.length > 0 && (
        <List variant="grouped" aria-label="Your tokens">
          {tokens.map((t) => (
            <ListItem
              key={t.id}
              title={t.name}
              description={`Ends in ${t.tokenHint}. Created ${t.createdAt}. ${t.lastUsedAt ? `Last used ${t.lastUsedAt}.` : 'Not used yet.'}`}
              trailing={
                <Button variant="outline" size="sm" pending={revoking} onClick={() => startRevoke(() => revokeTokenAction(t.id))}>
                  Revoke
                </Button>
              }
            />
          ))}
        </List>
      )}
    </div>
  );
}

export function ImportBookmarks() {
  const toast = useToast();
  const [pending, startTransition] = useTransition();
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const upload = (files: File[]) => {
    const file = files[0];
    if (!file) return;
    setError(null);
    setResult(null);
    const form = new FormData();
    form.set('file', file);
    startTransition(async () => {
      const r = await importBookmarksFileAction(form);
      if (r.error) return setError(r.error);
      const text = `Imported ${r.added} new ${r.added === 1 ? 'link' : 'links'}${r.existing ? `, ${r.existing} you already had` : ''}.`;
      setResult(text);
      toast.show({title: text});
    });
  };
  return (
    <div className="flex flex-col gap-2">
      <Dropzone
        accept=".html,.htm,.csv,text/html,text/csv"
        maxSize={20 * 1024 * 1024}
        disabled={pending}
        onFiles={upload}
        onReject={() => setError('Use an .html or .csv export, up to 20 MB.')}
        title={pending ? 'Importing…' : 'Drop a bookmarks file here'}
        hint="Browser HTML export, or CSV from Pocket, Raindrop or elsewhere, up to 20 MB"
      />
      {result && <p role="status" className="text-sm text-fg">{result}</p>}
      {error && <p role="alert" className="text-sm text-danger-text">{error}</p>}
    </div>
  );
}

type Theme = 'system' | 'light' | 'dark';

export function ThemeSetting() {
  const [theme, setTheme] = useState<Theme>('system');
  useEffect(() => {
    try {
      const saved = localStorage.getItem('stash-theme');
      if (saved === 'light' || saved === 'dark') setTheme(saved);
    } catch {}
  }, []);
  const choose = (value: string) => {
    const next = value as Theme;
    setTheme(next);
    const resolved = next === 'system' ? (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light') : next;
    document.documentElement.dataset.theme = resolved;
    try {
      if (next === 'system') localStorage.removeItem('stash-theme');
      else localStorage.setItem('stash-theme', next);
    } catch {}
  };
  return (
    <Segmented aria-label="Theme" value={theme} onValueChange={choose} className="self-start">
      <SegmentedItem value="system">System</SegmentedItem>
      <SegmentedItem value="light">Light</SegmentedItem>
      <SegmentedItem value="dark">Dark</SegmentedItem>
    </Segmented>
  );
}

export function DeleteEverything() {
  const [pending, startTransition] = useTransition();
  return (
    <AlertDialog>
      <AlertDialogTrigger render={<Button variant="outline" className="self-start"/>}>Delete my library</AlertDialogTrigger>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete your whole library?</AlertDialogTitle>
          <AlertDialogDescription>
            Every save, category and sync record goes, and this cannot be undone. Export first if you want a copy. Your saves on X, LinkedIn and in your browser are not touched.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogClose render={<Button variant="secondary"/>}>Keep my library</AlertDialogClose>
          <Button variant="destructive" pending={pending} onClick={() => startTransition(() => deleteAllDataAction())}>Delete my library</Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

export function GitHubImport() {
  const [state, formAction, pending] = useActionState(importGitHubStarsAction, undefined);
  return (
    <div className="flex flex-col gap-2">
      <form action={formAction} className="flex flex-col gap-3 sm:flex-row sm:items-end">
        <Field name="username" className="flex-1">
          <FieldLabel>GitHub username</FieldLabel>
          <Input name="username" placeholder="octocat" autoComplete="off" required maxLength={39}/>
        </Field>
        <Button type="submit" variant="secondary" pending={pending}>Import stars</Button>
      </form>
      {state?.message && <p role="status" className="text-sm text-fg">{state.message}</p>}
      {state?.error && <p role="alert" className="text-sm text-danger-text">{state.error}</p>}
    </div>
  );
}

export function LinkChecker({running, checked, broken, due}: {running: boolean; checked: number; broken: number; due: number}) {
  const router = useRouter();
  const [pending, startTransition] = useTransition();
  useEffect(() => {
    if (!running) return;
    const timer = setInterval(() => router.refresh(), 3000);
    return () => clearInterval(timer);
  }, [running, router]);
  const total = checked + due;
  return (
    <div className="flex flex-col gap-3">
      {running ? (
        <div role="status" className="flex flex-col gap-2">
          <p className="text-sm text-fg">Checking links. {due} left.</p>
          <Progress value={total ? (checked / total) * 100 : 0} label="Link check progress" valueText={`${checked} of ${total} checked`}/>
        </div>
      ) : (
        <p className="text-sm text-fg-2">
          {checked === 0 ? 'No links checked yet.' : `${checked} checked, ${broken} broken.`} {due > 0 ? `${due} ready to check.` : 'Everything has been checked this month.'}
        </p>
      )}
      <div className="flex flex-wrap gap-2">
        <Button variant="secondary" pending={pending} disabled={running || due === 0} onClick={() => startTransition(async () => {
          await startLinkCheckAction();
          router.refresh();
        })}>
          Check links
        </Button>
        {broken > 0 && <a href="/library?view=broken" className="self-center text-sm font-medium text-fg underline underline-offset-4">See {broken} broken {broken === 1 ? 'link' : 'links'}</a>}
      </div>
    </div>
  );
}

export function DigestSetting({enabled, lastSentAt, emailReady, email}: {enabled: boolean; lastSentAt: string | null; emailReady: boolean; email: string}) {
  const toast = useToast();
  const [on, setOn] = useState(enabled);
  const [sending, startSending] = useTransition();
  return (
    <div className="flex flex-col gap-3">
      <label className="flex items-center justify-between gap-3 rounded-field bg-surface px-4 py-3 text-base text-fg">
        <span className="flex flex-col">
          Email me a weekly digest
          <span className="text-sm text-fg-2">{emailReady ? `To ${email}. ${lastSentAt ? `Last sent ${lastSentAt}.` : 'Not sent yet.'}` : 'Email is not set up on this server (RESEND_API_KEY and STASH_EMAIL_FROM).'}</span>
        </span>
        <Switch checked={on} disabled={!emailReady} onCheckedChange={(v: boolean) => {
          setOn(v);
          void setDigestAction(v);
        }}/>
      </label>
      <div className="flex flex-wrap gap-2">
        <a href="/digest" className="self-center text-sm font-medium text-fg underline underline-offset-4">Preview this week</a>
        {emailReady && (
          <Button variant="ghost" size="sm" pending={sending} onClick={() => startSending(async () => {
            const r = await sendDigestNowAction();
            toast.show({title: r.error ?? r.message ?? 'Done'});
          })}>
            Send it to me now
          </Button>
        )}
      </div>
    </div>
  );
}
