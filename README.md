# Stash

Everything you saved, in one place, sorted for you: X bookmarks, LinkedIn saved posts, Reddit saved, YouTube Watch later and likes, Instagram saved, GitHub stars, browser bookmarks and Pocket or Raindrop exports, in one library sorted into your own categories by AI.

- **A browser extension** (Chrome, Firefox; Safari by conversion) reads your own saved lists while you are logged in and sends them to your Stash server. It only reads, and only when you sync (or once a day if you turn that on).
- **A web app** keeps the library:
  - The AI suggests categories that fit what *you* save; you approve or edit them, then it sorts everything. Unsure items wait in an inbox, and your corrections teach it.
  - Search by words or by meaning, and **Ask** questions answered from your own saves, with the saves it used.
  - Select many items and move or remove them at once.
  - Three older saves resurface each day, and an optional weekly digest email sums up what you saved.
  - A broken-link checker finds dead bookmarks.
  - Installable on a phone: share any link to Stash from the share sheet.
  - Export to Markdown or CSV, delete everything, dark mode.
- Styled with the [Lime](https://design.how/systems/lime) design system from design.how (MIT).

The plan behind it is in [PLAN.md](PLAN.md).

## What's in the repo

```
apps/web         Next.js app: landing page, library, ask, categories, settings, privacy, and the API
apps/extension   Browser extension (WXT, Manifest V3): importers for every source; STORE_LISTING.md
packages/shared  Item types, URL normalizing, and the parsers for every source and file format
assets           The app icon (SVG); PNGs are rendered into the apps
```

## Run it locally

You need Node 22+, pnpm 10, and Postgres 14+ with the pgvector extension (the Docker image below has it; Supabase and Neon include it).

```bash
pnpm install
docker compose up -d                 # Postgres + pgvector on localhost:5432 (skip if you have your own)
cp apps/web/.env.example apps/web/.env.local   # then fill it in
pnpm db:migrate
pnpm dev                             # http://localhost:3000
```

Open http://localhost:3000, create an account, and you're in.

### Environment

| Variable | What it is |
|---|---|
| `DATABASE_URL` | Postgres connection string, with pgvector available. |
| `OPENROUTER_API_KEY` | Turns on AI: category suggestions, sorting, search by meaning and Ask (through [OpenRouter](https://openrouter.ai) with the Vercel AI SDK). Without it, everything else works and you make categories by hand. Requests only go to providers that do not store or train on the data. |
| `STASH_TAXONOMY_MODEL` | Optional. Suggests categories. Default `anthropic/claude-opus-5.5`. |
| `STASH_CLASSIFY_MODEL` | Optional. Sorts each item. Default `anthropic/claude-haiku-5.5` (fast and cheap). |
| `STASH_ASK_MODEL` | Optional. Answers questions. Defaults to the taxonomy model. |
| `STASH_EMBEDDING_MODEL` | Optional. For search by meaning. Default `openai/text-embedding-3-small`; any model that can return 1536 numbers. |
| `OPENROUTER_BASE_URL` | Optional. A different OpenRouter-compatible endpoint. |
| `STASH_PUBLIC_URL` | The public address: shown for the extension, used in digest links. |
| `RESEND_API_KEY`, `STASH_EMAIL_FROM` | Weekly digest email through [Resend](https://resend.com). Without them, the digest is off. |
| `CRON_SECRET` | Protects `/api/cron/digest`. Vercel Cron sends it automatically (schedule in `apps/web/vercel.json`, Mondays). Elsewhere, call it weekly with `Authorization: Bearer <secret>`. |
| `GITHUB_TOKEN` | Optional. Raises GitHub's limit for importing stars. `GITHUB_API_URL` points at GitHub Enterprise. |
| `STASH_CONTACT_EMAIL` | Optional. Shown on the privacy page. |

## Install the extension

```bash
pnpm --filter extension build        # Chrome: apps/extension/.output/chrome-mv3
pnpm --filter extension build:firefox  # Firefox 128+: apps/extension/.output/firefox-mv3
```

1. Open `chrome://extensions`, turn on **Developer mode**, choose **Load unpacked**, and pick `apps/extension/.output/chrome-mv3`. (Firefox: `about:debugging`, This Firefox, Load Temporary Add-on, pick the `manifest.json`.)
2. In Stash, open **Settings**, create a token, and copy it.
3. Click the Stash icon, paste the server address and the token, and press **Connect**.
4. Press **Sync everything**. Stay logged in to the sites in that browser.

`pnpm --filter extension zip` makes the store upload. Everything the Chrome Web Store and Firefox Add-ons ask for (descriptions, permission reasons, privacy answers, Safari steps) is in [apps/extension/STORE_LISTING.md](apps/extension/STORE_LISTING.md).

### How the importers work

- **Browser bookmarks:** read with the `bookmarks` API, folders kept as a hint for the AI. New bookmarks are sent as you make them.
- **X:** opens `x.com/i/bookmarks` and scrolls it. A script in the page copies the JSON X's own app loads (the `Bookmarks` GraphQL call); nothing is scraped from the screen and no extra requests are made.
- **LinkedIn:** the same on `linkedin.com/my-items/saved-posts`, reading the first page from the HTML and the rest from the requests behind "Show more results".
- **YouTube:** the same on the Watch later and Liked videos playlists, reading `ytInitialData` and the `/youtubei/v1/browse` responses.
- **Reddit:** reads `/user/<you>/saved.json` with your login, 100 at a time.
- **Instagram:** reads the saved-posts feed Instagram's web app uses, with your login.
- **GitHub stars:** imported by the server from GitHub's public API (Settings, Import GitHub stars).
- **Files:** the HTML export from any browser, or a CSV from Pocket, Raindrop.io or anything with a url column (Settings, Import a file).
- After the first sync, a sync stops as soon as it reaches posts already in your library.

If a site changes its web app, the fix belongs in `packages/shared/src/parsers/`, which has fixture tests.

## Checks

```bash
pnpm test                            # parsers, extension logic, and the web app against a real Postgres
pnpm typecheck
pnpm --filter web lint:lime          # Lime design-system rules
pnpm build
```

The web tests need Postgres with pgvector. They use `TEST_DATABASE_URL`, default `postgres://stash:stash@localhost:5432/stash_test`, and reset that database each run.

## Deploy

The web app is a standard Next.js app: Vercel, Railway, Fly or a container all work. Set the environment variables, run `pnpm db:migrate` against the production database once (and after each update), and point the extension at the deployed address. Sorting, embedding and link checks run in the background after a request finishes (`after()`), so on serverless hosts give functions a long enough maximum duration (the ingest route asks for 300 seconds).

## Privacy

Each account only ever sees its own saves. The extension stores its token in the browser only and talks to your server only. The server refuses to open links that point at private networks. The privacy policy is served at `/privacy`; read it and add your contact address before launching.
