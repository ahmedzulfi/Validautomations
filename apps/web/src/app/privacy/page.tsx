import Link from 'next/link';

export const metadata = {title: 'Privacy · Stash'};

// Plain-language policy describing what this code does. Whoever runs a Stash server should read it,
// fill in the contact line, and have it reviewed for their own situation before publishing.
const CONTACT = process.env.STASH_CONTACT_EMAIL;
const UPDATED = 'November 2026';

export default function PrivacyPage() {
  return (
    <main className="mx-auto flex w-full max-w-2xl flex-col gap-8 px-4 pt-10 pb-16 sm:px-6">
      <Link href="/" className="text-lg font-semibold tracking-tight">Stash</Link>
      <div className="flex flex-col gap-2">
        <h1 className="text-2xl leading-tight font-semibold tracking-tight">Privacy policy</h1>
        <p className="text-sm text-fg-2">Last updated {UPDATED}.</p>
      </div>

      <Section title="What Stash collects">
        <p>Your email address and a scrambled form of your password, so you can sign in.</p>
        <p>The things you choose to bring in: posts and links you saved on X, LinkedIn, Reddit, YouTube, Instagram and GitHub, your browser bookmarks, files you import, and links you share or save. For each one Stash keeps the link, its text, the author&apos;s name and picture, and when you saved it.</p>
        <p>What Stash works out about them: the category, tags, a one-line summary, a vector used for search by meaning, and whether the link still opens.</p>
      </Section>

      <Section title="How the browser extension works">
        <p>The extension only acts when you press sync, or once a day if you turn that on. While you are logged in, it opens your own saved lists on those sites and reads the data the page already loads (for Reddit and Instagram, it reads your saved list with your login), then sends the items to your Stash server. It never posts, likes, unsaves or changes anything, and never sees your passwords. Its token is kept in your browser only.</p>
      </Section>

      <Section title="Who else sees it">
        <p>To sort and search, Stash sends the text of your saves to an AI model through OpenRouter, which passes it to the model provider (by default Anthropic for sorting and OpenAI for search). Stash asks OpenRouter to use only providers that do not store or train on what is sent.</p>
        <p>If you turn on the weekly digest, your email address and that week&apos;s saves go to the email service (Resend) to send it.</p>
        <p>To check for broken links, the Stash server opens each saved web link, as a browser would.</p>
        <p>Nothing is sold, and nothing is shared for advertising.</p>
      </Section>

      <Section title="Your choices">
        <p>Export your whole library to Markdown or CSV from Settings, any time. Delete your whole library from Settings in one step. Revoke an extension&apos;s access by revoking its token. Turn the digest off from Settings or from the link in any digest email.</p>
      </Section>

      <Section title="Keeping it safe">
        <p>Each account can see only its own saves. Passwords are stored scrambled with scrypt, sessions and extension tokens are stored only as hashes, and the server refuses to open links that point at private networks.</p>
      </Section>

      <Section title="Contact">
        <p>{CONTACT ? <>Questions or requests: <a className="font-medium text-fg underline underline-offset-4" href={`mailto:${CONTACT}`}>{CONTACT}</a>.</> : 'Questions or requests: contact whoever runs this Stash server.'}</p>
      </Section>
    </main>
  );
}

function Section({title, children}: {title: string; children: React.ReactNode}) {
  return (
    <section className="flex flex-col gap-3 text-base text-fg-2">
      <h2 className="text-lg font-semibold text-fg">{title}</h2>
      {children}
    </section>
  );
}
