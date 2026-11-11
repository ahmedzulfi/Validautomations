import Link from 'next/link';
import {redirect} from 'next/navigation';
import {LinkButton} from '@/components/app/link-button';
import {currentUser} from '@/lib/server/auth';

export const metadata = {
  title: 'Stash: everything you saved, in one place',
  description: 'Your bookmarks from X, LinkedIn, Reddit, YouTube, Instagram, GitHub and your browser in one library, sorted into your own categories by AI.',
};

const SOURCES = ['X bookmarks', 'LinkedIn saved posts', 'Reddit saved', 'YouTube watch later and likes', 'Instagram saved', 'GitHub stars', 'Browser bookmarks', 'Pocket and Raindrop exports'];

const STEPS = [
  {title: 'Bring it all in', body: 'The browser extension reads your saved lists while you are logged in: it opens each list and scrolls it, the way you would. It only reads, and only when you sync.'},
  {title: 'Approve your categories', body: 'The AI reads a sample of what you save and suggests categories that fit you, not generic topics. Rename, remove or add any before anything moves.'},
  {title: 'Find anything again', body: 'Everything is sorted, tagged and summarised. Search by words or by meaning, or ask a question and get an answer from your own saves.'},
];

const FEATURES = [
  {title: 'An inbox for the unsure ones', body: 'When the AI is not confident, the save waits in your inbox instead of landing in the wrong place. Move it once and the AI learns from you.'},
  {title: 'Ask your saves', body: '“What did I save about pricing pages?” gets a short answer with links to the exact posts it came from.'},
  {title: 'Old saves come back', body: 'Three older saves each day in the library, and a weekly email of what you saved, so good finds do not rot.'},
  {title: 'Broken links found for you', body: 'Stash checks old bookmarks and lists the dead ones, so you can clear them out in one go.'},
  {title: 'Save from your phone', body: 'Install Stash from your phone’s browser and it appears in the share sheet. Share a link, and it is in your library.'},
  {title: 'Yours to take with you', body: 'Export everything to Markdown or CSV any time, or delete it all in one step. You can run Stash on your own server.'},
];

export default async function Home() {
  if (await currentUser()) redirect('/library');
  return (
    <>
      <header className="mx-auto flex h-14 w-full max-w-5xl items-center justify-between px-4 sm:px-6">
        <span className="text-lg font-semibold tracking-tight">Stash</span>
        <Link href="/signin" className="text-sm font-medium text-fg underline-offset-4 hover:underline">Sign in</Link>
      </header>

      <main className="mx-auto flex w-full max-w-5xl flex-col gap-20 px-4 pt-12 pb-16 sm:px-6 sm:pt-20">
        <section className="flex max-w-2xl flex-col gap-5">
          <h1 className="text-2xl leading-tight font-semibold tracking-tight sm:text-3xl">Everything you saved, in one place. Sorted for you.</h1>
          <p className="text-lg text-fg-2">
            Your bookmarks from X, LinkedIn, Reddit, YouTube, Instagram, GitHub and your browser, in one library. The AI sorts them into categories that fit what you actually save, and you can ask it to find anything.
          </p>
          <div className="flex flex-wrap gap-3">
            <LinkButton href="/signup" size="lg">Create your library</LinkButton>
            <LinkButton href="/signin" size="lg" variant="secondary">Sign in</LinkButton>
          </div>
        </section>

        <section className="flex flex-col gap-4">
          <h2 className="text-xl font-semibold">Where your saves come from</h2>
          <ul className="flex flex-wrap gap-2">
            {SOURCES.map((s) => (
              <li key={s} className="rounded-control border border-line-strong px-3 py-1.5 text-sm text-fg">{s}</li>
            ))}
          </ul>
        </section>

        <section className="flex flex-col gap-6">
          <h2 className="text-xl font-semibold">How it works</h2>
          <ol className="grid gap-8 sm:grid-cols-3">
            {STEPS.map((step, i) => (
              <li key={step.title} className="flex flex-col gap-2">
                <h3 className="text-lg font-semibold">{i + 1}. {step.title}</h3>
                <p className="text-base text-fg-2">{step.body}</p>
              </li>
            ))}
          </ol>
        </section>

        <section className="flex flex-col gap-6">
          <h2 className="text-xl font-semibold">What else it does</h2>
          <ul className="grid gap-x-10 gap-y-8 sm:grid-cols-2">
            {FEATURES.map((f) => (
              <li key={f.title} className="flex flex-col gap-1">
                <h3 className="text-lg font-semibold">{f.title}</h3>
                <p className="text-base text-fg-2">{f.body}</p>
              </li>
            ))}
          </ul>
        </section>

        <section className="flex flex-col gap-3 border-t border-line pt-10">
          <h2 className="text-xl font-semibold">Your saves stay yours</h2>
          <p className="max-w-2xl text-base text-fg-2">
            The extension talks only to your Stash server, and only reads. Nothing is sold or used to train models. Each account sees only its own saves.{' '}
            <Link href="/privacy" className="font-medium text-fg underline underline-offset-4">Read the privacy policy</Link>.
          </p>
          <div className="pt-2">
            <LinkButton href="/signup" variant="secondary">Create your library</LinkButton>
          </div>
        </section>
      </main>

      <footer className="mx-auto flex w-full max-w-5xl flex-wrap gap-4 border-t border-line px-4 py-6 text-sm text-fg-2 sm:px-6">
        <span>Stash</span>
        <Link href="/privacy" className="hover:text-fg">Privacy</Link>
        <Link href="/signin" className="hover:text-fg">Sign in</Link>
      </footer>
    </>
  );
}
