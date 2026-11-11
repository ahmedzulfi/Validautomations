import {AppHeader} from '@/components/app/app-header';
import {requireUser} from '@/lib/server/auth';

export default async function AppLayout({children}: {children: React.ReactNode}) {
  const user = await requireUser();
  return (
    <>
      <AppHeader email={user.email}/>
      {children}
    </>
  );
}
