import {redirect} from 'next/navigation';
import {signInAction} from '@/app/actions';
import {AuthForm} from '@/components/app/auth-form';
import {currentUser} from '@/lib/server/auth';

export const metadata = {title: 'Sign in · Stash'};

export default async function SignInPage() {
  if (await currentUser()) redirect('/library');
  return <AuthForm mode="signin" action={signInAction}/>;
}
