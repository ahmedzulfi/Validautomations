'use client';

import Link from 'next/link';
import {useActionState, useState} from 'react';
import {Button} from '@/components/lime/ui/button';
import {Field, FieldDescription, FieldLabel} from '@/components/lime/ui/field';
import {Input} from '@/components/lime/ui/input';
import type {FormState} from '@/app/actions';

type Props = {
  mode: 'signin' | 'signup';
  /** Where to go after signing in, such as a link shared from a phone. */
  next?: string;
  action: (state: FormState, form: FormData) => Promise<FormState>;
};

export function AuthForm({mode, action, next}: Props) {
  const [state, formAction, pending] = useActionState(action, undefined);
  // Controlled, so a failed attempt does not clear what the person typed (React resets uncontrolled fields).
  const [email, setEmail] = useState('');
  const signup = mode === 'signup';
  return (
    <main className="mx-auto grid min-h-dvh w-full max-w-6xl items-center gap-8 px-4 py-10 sm:px-6 lg:grid-cols-2 lg:gap-16">
      <section className="stash-hero hidden flex-col gap-6 rounded-sheet p-10 lg:flex">
        <Link href="/" className="flex items-center gap-2 text-lg font-semibold">
          <span className="stash-logo" aria-hidden="true">S</span>Stash
        </Link>
        <h2 className="text-3xl leading-tight font-semibold tracking-tight">Everything you saved, in one place. Sorted for you.</h2>
        <p className="stash-hero-muted text-lg">X, LinkedIn, Reddit, YouTube, Instagram, GitHub and your browser bookmarks, in one colourful library that sorts itself.</p>
        <div className="flex flex-wrap gap-2">
          {['Sorted by AI', 'Search by meaning', 'Ask your saves', 'Weekly digest'].map((t) => <span key={t} className="stash-hero-pill">{t}</span>)}
        </div>
      </section>

      <div className="stash-card mx-auto flex w-full max-w-md flex-col gap-8 p-6 sm:p-8">
      <div className="flex flex-col gap-2">
        <Link href="/" className="flex items-center gap-2 lg:hidden" aria-label="Stash home">
          <span className="stash-logo" aria-hidden="true">S</span>
          <span className="stash-gradient-text text-lg font-semibold">Stash</span>
        </Link>
        <h1 className="text-xl leading-tight font-semibold tracking-tight sm:text-2xl">
          {signup ? 'Create your library' : 'Welcome back'}
        </h1>
        <p className="text-base text-fg-2">
          {signup ? 'It takes a minute. Then bring in your saves.' : 'Sign in to your library.'}
        </p>
      </div>
      <form action={formAction} className="flex flex-col gap-4">
        {next && <input type="hidden" name="next" value={next}/>}
        <Field name="email">
          <FieldLabel>Email</FieldLabel>
          <Input name="email" type="email" autoComplete="email" required size="lg" value={email} onChange={(e) => setEmail(e.target.value)}/>
        </Field>
        <Field name="password">
          <FieldLabel>Password</FieldLabel>
          <Input name="password" type="password" autoComplete={signup ? 'new-password' : 'current-password'} required minLength={8} size="lg"/>
          {signup && <FieldDescription>At least 8 characters.</FieldDescription>}
        </Field>
        {state?.error && <p role="alert" className="text-sm text-danger-text">{state.error}</p>}
        <Button type="submit" size="lg" className="w-full" pending={pending}>
          {signup ? 'Create account' : 'Sign in'}
        </Button>
      </form>
      <p className="text-sm text-fg-2">
        {signup ? 'Already have an account? ' : 'New here? '}
        <Link href={`${signup ? '/signin' : '/signup'}${next ? `?next=${encodeURIComponent(next)}` : ''}`} className="font-semibold text-accent-line underline underline-offset-4">
          {signup ? 'Sign in' : 'Create an account'}
        </Link>
      </p>
      </div>
    </main>
  );
}
