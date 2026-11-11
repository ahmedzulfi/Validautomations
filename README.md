# Stash

Your X bookmarks, LinkedIn saved posts and browser bookmarks in one place, sorted into categories by AI.

- **A browser extension** reads your own saved lists while you are logged in (X bookmarks, LinkedIn saved posts, browser bookmarks) and sends them to your Stash server. It only reads, and only when you sync.
- **A web app** keeps the library: the AI suggests categories that fit what *you* save, you approve or edit them, and it sorts everything. Anything it is unsure about waits in an inbox. Search, filter by source, move items, export to Markdown or CSV.
- Styled with the [Lime](https://design.how/systems/lime) design system from design.how (MIT).

The plan behind it is in [PLAN.md](PLAN.md).

## What's in the repo

```
apps/web         Next.js app: library, categories, settings, and the API the extension calls
apps/extension   Chrome extension (WXT, Manifest V3): importers for X, LinkedIn and browser bookmarks
packages/shared  Item types, URL normalizing, and the parsers for X, LinkedIn and bookmark files
```

## Run it locally

You need Node 22+, pnpm 10, and Postgres 14+ (or Docker).

```bash
pnpm install
docker compose up -d                 # Postgres on localhost:5432 (skip if you have your own)
cp apps/web/.env.example apps/web/.env.local   # then fill it in
pnpm db:migrate
pnpm dev                             # http://localhost:3000
```

Open http://localhost:3000, create an account, and you're in.

### Environment

| Variable | What it is |
|---|---|
| `DATABASE_URL` | Postgres connection string. Supabase, Neon or any Postgres works. |
| `OPENROUTER_API_KEY` | Turns on category suggestions and sorting (calls go through [OpenRouter](https://openrouter.ai) with the Vercel AI SDK). Without it, everything else works and you can make categories by hand. |
| `STASH_TAXONOMY_MODEL` | Optional. The OpenRouter model that suggests categories. Default `anthropic/claude-opus-5.5`. |
| `STASH_CLASSIFY_MODEL` | Optional. The OpenRouter model that sorts each item. Default `anthropic/claude-haiku-5.5` (fast and cheap). Any OpenRouter model that supports structured outputs works. |
| `OPENROUTER_BASE_URL` | Optional. A different OpenRouter-compatible endpoint. |
| `STASH_PUBLIC_URL` | Optional. The address shown to paste into the extension, if it differs from what the server sees. |

## Install the extension

```bash
pnpm --filter extension build        # writes apps/extension/.output/chrome-mv3
```

1. Open `chrome://extensions`, turn on **Developer mode**, choose **Load unpacked**, and pick `apps/extension/.output/chrome-mv3`.
2. In Stash, open **Settings**, create a token, and copy it.
3. Click the Stash icon, paste the server address and the token, and press **Connect**.
4. Press **Sync everything**. Stay logged in to X and LinkedIn in that browser.

`pnpm --filter extension dev` runs it with live reload. `build:firefox` makes a Firefox build.

### How the importers work

- **Browser bookmarks:** read with the `bookmarks` API, folders kept as a hint for the AI. New bookmarks are sent as you make them.
- **X:** the extension opens `x.com/i/bookmarks` and scrolls it. A script in the page copies the JSON X's own app loads (the `Bookmarks` GraphQL call), so nothing is scraped from the screen and no extra requests are made.
- **LinkedIn:** the same on `linkedin.com/my-items/saved-posts`, reading the first page from the HTML and the rest from the requests behind "Show more results".
- After the first sync, a sync stops as soon as it reaches posts already in your library.
- An optional daily sync runs in the background.

If X or LinkedIn change their web apps, the fix belongs in `packages/shared/src/parsers/`, which has fixture tests.

## Checks

```bash
pnpm test                            # parsers, collector, and the web app against a real Postgres
pnpm typecheck
pnpm --filter web lint:lime          # Lime design-system rules
pnpm build
```

The web tests need Postgres. They use `TEST_DATABASE_URL`, default `postgres://stash:stash@localhost:5432/stash_test`, and reset that database each run.

## Deploy

The web app is a standard Next.js app: Vercel, Railway, Fly or a container all work. Set the environment variables, run `pnpm db:migrate` against the production database once (and after each update), and point the extension at the deployed address. Sorting runs in the background after a request finishes (`after()`), so on serverless hosts give the functions a long enough maximum duration (the ingest route asks for 300 seconds).

## Privacy

Each account only ever sees its own saves. The extension stores its token in the browser only and talks to your server only. Settings has **Export** and **Delete my library**.
