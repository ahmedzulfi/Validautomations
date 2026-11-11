import {redirect} from 'next/navigation';
import {signUpAction} from '@/app/actions';
import {AuthForm} from '@/components/app/auth-form';
import {currentUser} from '@/lib/server/auth';

export const metadata = {title: 'Create account · Stash'};

export default async function SignUpPage() {
  if (await currentUser()) redirect('/library');
  return <AuthForm mode="signup" action={signUpAction}/>;
}
