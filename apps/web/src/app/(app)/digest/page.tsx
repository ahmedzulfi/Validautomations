import {LinkButton} from '@/components/app/link-button';
import {requireUser} from '@/lib/server/auth';
import {buildDigest, publicBaseUrl, renderDigest} from '@/lib/server/digest';

export const metadata = {title: 'Digest preview · Stash'};

/** This week's digest exactly as the email would show it. */
export default async function DigestPreviewPage() {
  const user = await requireUser();
  const digest = await buildDigest(user.id);
  const email = digest ? renderDigest(digest, publicBaseUrl()) : null;
  return (
    <main className="mx-auto flex w-full max-w-2xl flex-col gap-6 px-4 pt-6 pb-12 sm:px-6">
      <div className="flex flex-col gap-1">
        <h1 className="text-xl leading-tight font-semibold tracking-tight sm:text-2xl">This week&apos;s digest</h1>
        <p className="text-base text-fg-2">
          {digest && digest.newCount > 0 ? `Subject: ${email!.subject}` : 'Nothing new was saved in the last 7 days, so no email would go out this week.'}
        </p>
      </div>
      {email && digest!.newCount > 0 && (
        <iframe title="Digest email preview" srcDoc={email.html} sandbox="" className="h-180 w-full rounded-panel border border-line bg-page"/>
      )}
      <LinkButton href="/settings#digest" variant="secondary" className="self-start">Back to settings</LinkButton>
    </main>
  );
}
