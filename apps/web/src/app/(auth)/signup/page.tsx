import {redirect} from 'next/navigation';
import {signUpAction} from '@/app/actions';
import {AuthForm} from '@/components/app/auth-form';
import {currentUser} from '@/lib/server/auth';

export const metadata = {title: 'Create account · Stash'};

export default async function SignUpPage({searchParams}: {searchParams: Promise<{next?: string}>}) {
  const {next} = await searchParams;
  if (await currentUser()) redirect(next?.startsWith('/') && !next.startsWith('//') ? next : '/library');
  return <AuthForm mode="signup" next={next} action={signUpAction}/>;
}
