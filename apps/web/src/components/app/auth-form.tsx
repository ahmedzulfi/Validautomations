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
    <main className="mx-auto flex w-full max-w-md flex-col gap-8 px-4 pt-16 pb-8 sm:px-6 sm:pt-24">
      <div className="flex flex-col gap-2">
        <p className="text-lg font-semibold">Stash</p>
        <h1 className="text-xl leading-tight font-semibold tracking-tight sm:text-2xl">
          {signup ? 'Create your library' : 'Sign in to your library'}
        </h1>
        <p className="text-base text-fg-2">
          Your X bookmarks, LinkedIn saved posts and browser bookmarks in one place, sorted for you.
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
        <Link href={`${signup ? '/signin' : '/signup'}${next ? `?next=${encodeURIComponent(next)}` : ''}`} className="font-medium text-fg underline underline-offset-4">
          {signup ? 'Sign in' : 'Create an account'}
        </Link>
      </p>
    </main>
  );
}
