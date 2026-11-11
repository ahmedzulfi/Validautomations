import {redirect} from 'next/navigation';
import {signInAction} from '@/app/actions';
import {AuthForm} from '@/components/app/auth-form';
import {currentUser} from '@/lib/server/auth';

export const metadata = {title: 'Sign in · Stash'};

export default async function SignInPage({searchParams}: {searchParams: Promise<{next?: string}>}) {
  const {next} = await searchParams;
  if (await currentUser()) redirect(next?.startsWith('/') && !next.startsWith('//') ? next : '/library');
  return <AuthForm mode="signin" next={next} action={signInAction}/>;
}
