# Stash: an AI organizer for everything you've saved

**Working name:** Stash (rename it whenever you like)

**The problem:** your saved stuff is scattered. X bookmarks, LinkedIn saved posts and browser favorites all live in separate silos, none of them searchable, and nothing in them is organized.

**The product:** one place that pulls them all in, sorts them into categories with AI and lets you find anything again.

---

## Status: Phase 1 built (see README.md to run it)

Done: all three importers (X, LinkedIn, browser bookmarks, plus a bookmarks-file upload), dedup across sources, AI category suggestions with review and approval, background sorting with an inbox for unsure items, corrections fed back to the AI, library with search, source filter and move, export to Markdown and CSV, delete everything, dark mode, "Save this page" and a daily auto-sync in the extension. Also from Phase 2: live sync of new bookmarks, and export.

Changes from the plan below, and why:
- **Auth is built in** (email and password, sessions in Postgres, API tokens for the extension) instead of Supabase Auth. Any Postgres works, Supabase included, and the whole flow can be tested locally without a hosted service.
- **Search is Postgres full-text search** for now. Semantic search with embeddings (pgvector) stays in Phase 2.
- **One category per item, plus tags in an array**, rather than many-to-many tables. Simpler, and it matches how the library is browsed.
- **Models:** categories are suggested by `claude-opus-5-5` (rare, judgment-heavy) and items sorted by `claude-haiku-5-5` (high volume). Both can be changed with environment variables.
- **No Turborepo:** pnpm workspaces are enough at this size.
- **Lime gaps:** Lime's Approval request component is built around money, so the category review uses Lime's form fields and List instead. The Action receipt confirms the result.

Not yet done: Phase 2 semantic search and weekly digest, Phase 3 sources and browsers, and the Chrome Web Store listing.

---

## 1. Core decisions

| Decision | Choice | Why |
|---|---|---|
| How data gets in | **Browser extension** (Chrome first, Manifest V3) | X and LinkedIn have no usable API for saved posts. The extension reads the user's *own* saved pages while they're logged in. Browser bookmarks come free through `chrome.bookmarks`. |
| Where you browse it | **Web app** | Bigger screen, works on any device, easy to share later. |
| Extension framework | **WXT** (TypeScript + React) | The best MV3 developer experience available, with hot reload and cross-browser builds (Firefox later at almost no extra cost). |
| Web app | **Next.js** (App Router) + Tailwind + shadcn | A standard stack with fast iteration. |
| Design system | **[Lime](https://design.how/systems/lime) from design.how** (MIT) | Warm, light and made for consumer apps. It installs through shadcn and ships its own agent skill and lint, so every screen stays consistent. See section 5. |
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

## 5. Design system: Lime (design.how)

Lime is a white page, warm-grey surfaces and near-black text, with **one glowing lime action per view**. It suits a calm, personal library. Source: [arla6ka/design.how/systems/lime](https://github.com/arla6ka/design.how/tree/main/systems/lime), MIT licensed.

### Install (in `apps/web`, and again in `apps/extension` for the popup)
```bash
npx shadcn@latest init
npx shadcn@latest add https://design.how/r/lime/theme.json   # tokens, styles, Lime skill
npx shadcn@latest add https://design.how/r/lime/button.json  # then each component by name
npx shadcn@latest add https://design.how/r/lime/lint.json    # lint rules
npx eslint -c lime.eslint.config.mjs .                       # run in CI
```
Root layout: `<body className="lime"><LimePortalProvider>{children}</LimePortalProvider></body>`. Without the provider, menus and dialogs lose their tokens.

### Key tokens (never use raw values in code; use the classes)
| Role | Token → value |
|---|---|
| Page / surfaces | `--page` #ffffff, `--surface` #f6f5f2, `--surface-2` #efeeea |
| Text | `--fg` #0f0f0d, `--fg-2` #4a4943, `--fg-3` #6b6a63 |
| The one action | `--accent` #e4f222 (lime), text on it `--on-accent` #0f0f0d |
| Selected row | `--accent-wash` #f5f9c4 |
| Lines | `--line` #e6e4df |
| Font | Hanken Grotesk 400/500/600 (never 700) |
| Radius | field 12px, panel 16px, sheet 24px, controls fully round |
| Controls | 32 / 40 / 48px tall |

Dark mode: put `data-theme="dark"` on `<html>`. It doesn't follow the system setting by itself, so add a small toggle that reads `prefers-color-scheme` on first load.

### How Stash's screens map to Lime
| Stash screen / piece | Lime pieces |
|---|---|
| App frame | Page shell, `main` at `max-w-5xl` for the library grid and `max-w-2xl` for settings |
| Category sidebar (desktop) | List + ListItem, chosen row on `--accent-wash`; Sheet on phones |
| Source filter (All / X / LinkedIn / Bookmarks) | Segmented |
| Tags | Chip |
| Saved item | Card (only around the item itself, never a card in a card), Avatar for the author, Badge for the source |
| Move to category | Dropdown menu or Combobox |
| Uncategorized inbox | List; Empty when it's clear |
| Taxonomy review after the first import | **Approval request** block, so the AI proposes categories and you approve, edit or reject before anything moves |
| "Sorted 214 items" result | Action receipt with Undo |
| Ask your saves (Phase 2) | Chat input, Message, Follow ups |
| Weekly digest / resurface | Insight |
| Sync progress | Progress, Status, Toast |
| Import bookmarks `.html` | Dropzone |
| Settings (sync schedule, auto-sort rules) | Switch, Autonomy rule, "Settings rules" block |
| Extension popup | Same theme: sign-in, one lime **Sync now** button, Status per source |

**Where the one glowing action goes:** in the library it's **Add / Save**; on the taxonomy review it's **Apply categories**; in the popup it's **Sync now**. Everything else is secondary, outline or ghost.

### Gaps (Lime is built for a banking app, so build these from Lime parts)
- **⌘K search:** Lime has no command palette. Use a Dialog with an Input and a List of results.
- **Bulk select and move:** no bulk action bar. Use a sticky bottom bar on `bg-page` with `border-t border-line`, holding a count and a Dropdown menu.
- **Masonry grid of saves:** CSS grid of Cards with the Lime spacing scale.
- **Skip** the money components: Amount, Balance, Bill schedule, Keypad, Receipt match.

### Rules to keep (from Lime's skill)
- Sentence case, a verb on every button ("Sort my saves", not "OK"), no emoji, no all-caps labels.
- No gradients, blobs, glass or extra glows; no rows of stat cards ("1,204 saves / 38 categories / ..."). Say it in a sentence instead.
- Every input sits in a Field with a visible label.
- Before calling a screen done: lint passes, and it's checked at 390px and 1280px wide.

---

## 6. Features by phase

### Phase 0: Setup (≈2 days)
- Monorepo, Supabase project, auth (Google and email magic link)
- Lime theme installed, Lime lint running in CI
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

## 7. Repo layout

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

## 8. Risks and how to handle them

| Risk | Mitigation |
|---|---|
| X or LinkedIn change their front end and break the parser | Isolated parsers, saved fixture responses in tests, DOM fallback, and a visible "sync failed" state rather than silent failure |
| Account flagged for automation | User-initiated syncs only, slow scrolling, no write actions, incremental syncs |
| Terms-of-service gray area | Users only read their own data in their own browser. Be transparent in the privacy policy. Don't run headless scrapers server-side. |
| Bad AI categories | User approves the taxonomy first, low-confidence items go to the inbox, corrections feed back into prompts |
| Privacy (people's saves are personal) | RLS, no selling or training on data, a clear "delete everything" button |

---

## 9. First tasks, in order

1. Scaffold the monorepo (pnpm, Turborepo, WXT, Next.js, shared package).
2. Install Lime (theme, lint, first components) in the web app and the extension popup. Add the Lime lint to CI.
3. Supabase schema migration and auth in the web app.
4. Browser-bookmarks importer → `/api/ingest` → items show up in a plain list.
5. Classification pipeline with Claude, and categories in the sidebar.
6. X importer, with a fixture-tested parser.
7. LinkedIn importer, same pattern.
8. Search, the inbox and drag-to-recategorize.

Step 4 is the first real milestone: real data flowing end to end. Everything after that is adding sources and polish.
