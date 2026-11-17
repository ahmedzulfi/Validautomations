import Link from 'next/link';
import {redirect} from 'next/navigation';
import {LinkButton} from '@/components/app/link-button';
import {SOURCE_LABELS, type Source} from '@stash/shared';
import {Folder, Icon, Locked, Renew, Search, Send, Upload, Warning} from '@/components/lime/icon';
import {hueFor} from '@/lib/hue';
import {currentUser} from '@/lib/server/auth';

export const metadata = {
  title: 'Stash: everything you saved, in one place',
  description: 'Your bookmarks from X, LinkedIn, Reddit, YouTube, Instagram, GitHub and your browser in one library, sorted into your own categories by AI.',
};

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

const SOURCE_BADGES: Array<[Source, string]> = [
  ['x', 'X'], ['linkedin', 'LinkedIn'], ['reddit', 'Reddit'], ['youtube', 'YouTube'], ['instagram', 'Instagram'],
  ['github', 'GitHub'], ['bookmark', 'Browser bookmarks'], ['web', 'Pocket and Raindrop'],
];
const STEP_ICONS = [Upload, Folder, Search];
const FEATURE_ICONS = [Folder, Search, Renew, Warning, Send, Locked];

/** A few example saves, drawn the way the library draws them, for the hero. */
const PREVIEW = [
  {hue: '0', source: 'x' as Source, who: 'Maya Chen', text: 'Pricing pages that convert: show the plan most people pick first.', cat: 'Product and growth'},
  {hue: '1', source: 'reddit' as Source, who: 'r/PostgreSQL', text: 'How do you structure a schema for multi-tenant SaaS?', cat: 'Databases'},
  {hue: '2', source: 'youtube' as Source, who: 'Design Talks', text: 'Typography for interfaces, in 20 minutes', cat: 'Design'},
];

export default async function Home() {
  if (await currentUser()) redirect('/library');
  return (
    <>
      <header className="mx-auto flex h-16 w-full max-w-6xl items-center justify-between px-4 sm:px-6">
        <span className="flex items-center gap-2">
          <span className="stash-logo" aria-hidden="true">S</span>
          <span className="stash-gradient-text text-lg font-semibold tracking-tight">Stash</span>
        </span>
        <Link href="/signin" className="rounded-control px-4 py-2 text-sm font-semibold text-accent-line hover:bg-accent-wash">Sign in</Link>
      </header>

      <main className="mx-auto flex w-full max-w-6xl flex-col gap-24 px-4 pt-10 pb-20 sm:px-6 sm:pt-16">
        <section className="grid items-center gap-12 lg:grid-cols-2">
          <div className="flex flex-col gap-6">
            <h1 className="text-3xl leading-tight font-semibold tracking-tight">
              Everything you saved, in one place.{' '}
              <span className="stash-gradient-text">Sorted for you.</span>
            </h1>
            <p className="text-lg text-fg-2">
              Your bookmarks from X, LinkedIn, Reddit, YouTube, Instagram, GitHub and your browser, in one colourful library. The AI sorts them into categories that fit what you actually save, and you can ask it to find anything.
            </p>
            <div className="flex flex-wrap gap-3">
              <LinkButton href="/signup" size="lg">Create your library</LinkButton>
              <LinkButton href="/signin" size="lg" variant="outline">Sign in</LinkButton>
            </div>
            <ul aria-label="Sources" className="flex flex-wrap gap-2 pt-2">
              {SOURCE_BADGES.map(([s, label]) => <li key={s}><span className="src-badge" data-source={s}>{label}</span></li>)}
            </ul>
          </div>

          <div aria-hidden="true" className="stash-hero relative flex flex-col gap-4 rounded-sheet p-6 sm:p-8">
            <div className="flex flex-wrap gap-2">
              <span className="stash-hero-pill">1,284 saves</span>
              <span className="stash-hero-pill">9 categories</span>
              <span className="stash-hero-pill">12 in your inbox</span>
            </div>
            {PREVIEW.map((p, i) => (
              <div key={p.text} data-hue={p.hue} className={`stash-card overflow-hidden text-fg ${i === 1 ? 'sm:ms-8' : i === 2 ? 'sm:ms-16' : ''}`}>
                <div className="hue-strip"/>
                <div className="flex flex-col gap-2 p-4">
                  <span className="text-sm font-semibold">{p.who}</span>
                  <span className="text-base leading-snug">{p.text}</span>
                  <span className="flex flex-wrap gap-2">
                    <span className="src-badge" data-source={p.source}>{SOURCE_LABELS[p.source]}</span>
                    <span className="hue-chip"><span className="hue-dot"/>{p.cat}</span>
                  </span>
                </div>
              </div>
            ))}
          </div>
        </section>

        <section className="flex flex-col gap-8">
          <h2 className="text-2xl font-semibold tracking-tight">How it works</h2>
          <ol className="grid gap-5 sm:grid-cols-3">
            {STEPS.map((step, i) => (
              <li key={step.title} data-hue={hueFor(i)} className="stash-card flex flex-col gap-4 p-6">
                <span className="hue-tile"><Icon icon={STEP_ICONS[i]!} size={20}/></span>
                <h3 className="text-lg font-semibold">{i + 1}. {step.title}</h3>
                <p className="text-base text-fg-2">{step.body}</p>
              </li>
            ))}
          </ol>
        </section>

        <section className="flex flex-col gap-8">
          <h2 className="text-2xl font-semibold tracking-tight">What else it does</h2>
          <ul className="grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
            {FEATURES.map((f, i) => (
              <li key={f.title} data-hue={hueFor(i + 3)} className="stash-card stash-card-lift flex flex-col gap-3 p-6">
                <span className="hue-tile"><Icon icon={FEATURE_ICONS[i]!} size={20}/></span>
                <h3 className="text-lg font-semibold">{f.title}</h3>
                <p className="text-base text-fg-2">{f.body}</p>
              </li>
            ))}
          </ul>
        </section>

        <section className="stash-hero flex flex-col items-start gap-4 rounded-sheet p-8 sm:p-10">
          <h2 className="text-2xl font-semibold tracking-tight">Your saves stay yours</h2>
          <p className="stash-hero-muted max-w-2xl text-base">
            The extension talks only to your Stash server, and only reads. Nothing is sold or used to train models, and each account sees only its own saves.{' '}
            <Link href="/privacy" className="font-semibold underline underline-offset-4">Read the privacy policy</Link>.
          </p>
          <Link href="/signup" className="stash-hero-action">Create your library</Link>
        </section>
      </main>

      <footer className="mx-auto flex w-full max-w-6xl flex-wrap gap-6 border-t border-line px-4 py-8 text-sm text-fg-2 sm:px-6">
        <span className="font-semibold text-fg">Stash</span>
        <Link href="/privacy" className="hover:text-fg">Privacy</Link>
        <Link href="/signin" className="hover:text-fg">Sign in</Link>
      </footer>
    </>
  );
}
