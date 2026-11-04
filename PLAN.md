# Stash: an AI organizer for everything you've saved

**Working name:** Stash (rename it whenever you like)

**The problem:** your saved stuff is scattered. X bookmarks, LinkedIn saved posts and browser favorites all live in separate silos, none of them searchable, and nothing in them is organized.

**The product:** one place that pulls them all in, sorts them into categories with AI and lets you find anything again.

---

## 1. Core decisions

| Decision | Choice | Why |
|---|---|---|
| How data gets in | **Browser extension** (Chrome first, Manifest V3) | X and LinkedIn have no usable API for saved posts. The extension reads the user's *own* saved pages while they're logged in. Browser bookmarks come free through `chrome.bookmarks`. |
| Where you browse it | **Web app** | Bigger screen, works on any device, easy to share later. |
| Extension framework | **WXT** (TypeScript + React) | The best MV3 developer experience available, with hot reload and cross-browser builds (Firefox later at almost no extra cost). |
| Web app | **Next.js** (App Router) + Tailwind + shadcn/ui | A standard stack with fast iteration. |
| Backend / DB / auth | **Supabase**: Postgres, Auth, Row Level Security, **pgvector** | One service covers auth, data and vector search. |
| AI categorizing | **Claude API**, Haiku for bulk tagging, Sonnet for building taxonomies | Cheap and fast for thousands of items. Structured JSON output. |
| Semantic search | Embeddings (Voyage AI) stored in pgvector | Lets "that post about pricing pages" work even with no keyword match. |
| Monorepo | pnpm + Turborepo (`apps/extension`, `apps/web`, `packages/shared`) | Shared types between the extension and the app. |

---

## 2. How each source gets imported

### Browser bookmarks (easiest, so build it first)
- `chrome.bookmarks.getTree()` returns every bookmark with its folder path.
- Keep the user's existing folder names as a hint for the AI.
- Fallback: upload a bookmarks `.html` export, which also works for Safari and Firefox users.

### X bookmarks
- A content script runs on `x.com/i/bookmarks`.
- It **intercepts X's own GraphQL `Bookmarks` responses**, injecting a page-world script that wraps `fetch`/`XHR`. That gives clean JSON (text, author, media, links, date) instead of fragile DOM scraping.
- It auto-scrolls to page through everything, slowly and with jitter.
- Later syncs stop when they reach an item already imported (incremental sync).

### LinkedIn saved posts
- A content script runs on `linkedin.com/my-items/saved-posts/`.
- Same approach: intercept the `voyager` API responses the page already makes, with DOM parsing as a fallback.
- Same auto-scroll and incremental-stop logic.

### Ground rules for imports (important)
- Imports only run when the user starts them, or on a gentle schedule while they're logged in. No servers scraping on the user's behalf, and no stored passwords.
- Read-only: never like, unsave or post anything.
- Rate-limited and human-paced, to avoid account flags.
- Parsers break when X or LinkedIn change their front ends. Isolate each one in its own module with fixture tests, so fixing one is a 10-minute job.

---

## 3. Data model (Postgres)

```
items
  id, user_id, source ('x' | 'linkedin' | 'bookmark' | 'manual'),
  source_id (unique per user+source, for dedup), url, title, text,
  author_name, author_handle, media jsonb, saved_at, imported_at,
  content_hash, embedding vector(1024), summary, status

categories
  id, user_id, name, description, color, is_ai_generated, sort_order

item_categories   (item_id, category_id, confidence, assigned_by: 'ai' | 'user')
tags              (id, user_id, name)
item_tags         (item_id, tag_id)
sync_runs         (id, user_id, source, started_at, finished_at, items_added, error)
```

RLS on every table: `user_id = auth.uid()`.

---

## 4. How the AI organizes things

1. **Bootstrap the taxonomy.** After the first import, send a sample (~200 items, titles and snippets) to Sonnet and ask for 8–15 categories that fit *this person's* saves, with a one-line description of each. The user can rename, merge or delete them before anything is applied.
2. **Classify in batches.** Send items to Haiku 20–50 at a time and get back structured JSON:
   `{ item_id, category_id, confidence, tags[≤5], summary (1 line) }`.
   The user's category list and descriptions are in the prompt, and the "category" field is constrained to those ids (plus `"uncategorized"`).
3. **Learn from corrections.** When a user moves an item, store it as `assigned_by: 'user'` and include a few recent corrections as examples in future prompts. Never overwrite a user's choice.
4. **Skip repeat work.** Hash each item's content and never re-classify unchanged items.
5. **Embed** every item for semantic search and "related items".

Rough cost: at Haiku pricing, a few thousand items cost a few cents to categorize. Leave a cap in the code anyway.

---

## 5. Features by phase

### Phase 0: Setup (≈2 days)
- Monorepo, Supabase project, auth (Google and email magic link)
- Extension skeleton that signs in with the same Supabase session
- CI: typecheck, lint and tests on push

### Phase 1: MVP, "it actually works for me" (≈2–3 weeks)
- [ ] Import browser bookmarks
- [ ] Import X bookmarks (full and incremental)
- [ ] Import LinkedIn saved posts (full and incremental)
- [ ] Dedup across sources (same URL means one item, with multiple sources shown)
- [ ] AI taxonomy bootstrap, then batch classification
- [ ] Web app: sidebar of categories, item grid/list, source filter, keyword search
- [ ] Drag or select to move items between categories
- [ ] "Uncategorized" inbox for low-confidence items

**Done when:** you can import your own X, LinkedIn and Chrome saves, and 80% or more of the auto-categories feel right without edits.

### Phase 2: Make it great (≈2 weeks)
- [ ] Semantic search ("ask your saves")
- [ ] One-click "save this page" from the extension, auto-categorized on arrival
- [ ] Background sync (extension alarm, only while logged in to each site)
- [ ] Broken-link checker for old bookmarks
- [ ] Weekly digest email: "you saved 34 things, here are the 5 best on topics you care about"
- [ ] Export to Markdown, CSV or Notion

### Phase 3: More sources and polish
- Reddit saved, YouTube Watch Later, GitHub stars, Instagram saved, Pocket/Raindrop import files
- Firefox and Safari extension builds
- Mobile share-sheet capture (PWA first)
- "Resurface" mode: a few old saves each day so they don't rot

### Phase 4: If it becomes a product
- Free tier (500 items) and paid tier (unlimited, semantic search, digests)
- Chrome Web Store listing, privacy policy, landing page

---

## 6. Repo layout

```
apps/
  extension/        WXT + React
    entrypoints/
      background.ts       sync orchestration, auth, API calls
      popup/              sign in, "sync now", status
      x.content.ts        X bookmarks importer
      linkedin.content.ts LinkedIn saved-posts importer
    lib/parsers/          pure functions: raw API JSON → Item (unit tested)
  web/              Next.js app
    app/(app)/library     main UI
    app/api/classify      server route calling Claude
    app/api/ingest        receives batches from the extension
packages/
  shared/           Item types, zod schemas, category prompt builders
supabase/
  migrations/       SQL schema + RLS + pgvector
```

---

## 7. Risks and how to handle them

| Risk | Mitigation |
|---|---|
| X or LinkedIn change their front end and break the parser | Isolated parsers, saved fixture responses in tests, DOM fallback, and a visible "sync failed" state rather than silent failure |
| Account flagged for automation | User-initiated syncs only, slow scrolling, no write actions, incremental syncs |
| Terms-of-service gray area | Users only read their own data in their own browser. Be transparent in the privacy policy. Don't run headless scrapers server-side. |
| Bad AI categories | User approves the taxonomy first, low-confidence items go to the inbox, corrections feed back into prompts |
| Privacy (people's saves are personal) | RLS, no selling or training on data, a clear "delete everything" button |

---

## 8. First tasks, in order

1. Scaffold the monorepo (pnpm, Turborepo, WXT, Next.js, shared package).
2. Supabase schema migration and auth in the web app.
3. Browser-bookmarks importer → `/api/ingest` → items show up in a plain list.
4. Classification pipeline with Claude, and categories in the sidebar.
5. X importer, with a fixture-tested parser.
6. LinkedIn importer, same pattern.
7. Search, the inbox and drag-to-recategorize.

Step 3 is the first real milestone: real data flowing end to end. Everything after that is adding sources and polish.
