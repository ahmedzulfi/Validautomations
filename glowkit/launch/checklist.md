# Launch-day checklist

Times are suggestions. Tuesday to Thursday mornings US Eastern time work best for X, Reddit and
Hacker News.

## The day before

- [ ] Push the repo to `github.com/ahmedzulfi/glowkit` (default branch `main`, since the README
      images load from `raw.githubusercontent.com/ahmedzulfi/glowkit/main/...`).
- [ ] CI is green on Linux, Windows and macOS, including the MSRV job.
- [ ] Pages deploy works and https://ahmedzulfi.github.io/glowkit/ and `/gallery/` load in a fresh
      browser profile (and on a phone).
- [ ] README renders on GitHub: hero and all nine grid images animate, every link resolves.
- [ ] `./scripts/publish.sh --dry-run` passes (or packages) every crate.
- [ ] Re-read `launch/x-thread.md` and `launch/posts.md` once more; run
      `python3 launch/count.py launch/x-thread.md launch/posts.md`.
- [ ] Upload the thread media to X drafts ahead of time to make sure each MP4 is accepted.

## Repository settings

- [ ] **Description** and **website**: paste from `launch/github-about.txt`; website
      `https://ahmedzulfi.github.io/glowkit/`.
- [ ] **Topics**: `rust`, `egui`, `gui`, `ui-components`, `ai-agents`, `animation`,
      `desktop-app`, `wasm`.
- [ ] **Social preview**: Settings, General, Social preview: upload `website/social-preview.png`.
- [ ] **Pages**: Settings, Pages, Source: **GitHub Actions** (the workflow is `pages.yml`).
- [ ] **Features**: enable Discussions (the issue template links to it) and Issues; turn off Wiki
      and Projects if unused.
- [ ] **Security**: Settings, Code security: enable private vulnerability reporting (SECURITY.md
      points there), Dependabot alerts and security updates.
- [ ] **Branch protection** on `main`: require the CI checks to pass.
- [ ] **Labels**: make sure `bug`, `enhancement`, `good first issue` and `help wanted` exist.
- [ ] Create a few `good first issue` items from the open rows in CATALOG.md, so people who
      arrive from the launch have something to pick up.
- [ ] **Pin the repo** on your GitHub profile.

## Publish the crates

- [ ] `cargo login` with a crates.io token.
- [ ] `./scripts/publish.sh` (publishes in dependency order with a 30 s pause; `FROM=<crate>`
      resumes after a failure).
- [ ] Check https://crates.io/crates/glowkit and that docs.rs builds for `glowkit`,
      `glowkit-core` and `glowkit-stream` (docs.rs can take a while; the badges go green after).
- [ ] Tag the release: `git tag v0.1.0 && git push --tags`, then create a GitHub release from
      the CHANGELOG 0.1.0 entry with `launch/media/00-montage.mp4` attached.

## Post order and timing (launch day)

| Time (ET) | Where | What |
|---|---|---|
| 08:30 | crates.io | Crates published, docs.rs building, release tagged |
| 09:00 | X | Launch thread (`launch/x-thread.md`), posted as one thread. Pin the first post to your profile |
| 09:30 | Hacker News | Show HN (`launch/reddit-hn.md`). Don't ask anyone to upvote; HN penalizes it |
| 10:00 | r/rust | Post from `launch/reddit-hn.md`, with the current project flair |
| 13:00 | r/egui | Shorter egui-focused post |
| same week | This Week in Rust | Open the PR with the blurb before the Tuesday cutoff |
| days 2 to 20 | X | One follow-up from `launch/posts.md` every day or two |

Optional, later in the week: the Rust users forum (users.rust-lang.org, "Announcements"), the egui
GitHub Discussions "Show and tell" category, Lobsters (if you have an account), Mastodon and
Bluesky with the same thread.

## Accounts to tag on X

Tag at most two per post, and only where the post is actually about their work.

| Account | Why | Where |
|---|---|---|
| @ernerfeldt | Emil Ernerfeldt, author of egui | Last post of the thread |
| @rustlang | Official Rust account | Don't tag; they rarely share third-party projects. Fine to use `#rustlang` |
| @ThisWeekInRust | Weekly newsletter | Reply to their post once your PR is merged |
| `[AG-UI / CopilotKit handle: verify]` | AG-UI protocol | Follow-up post 1 (agent thread) |
| `[Vercel AI SDK handle: verify]` | AI SDK stream support | Follow-up post 1 |
| `[Zed handle: verify]` | Agent Client Protocol | Follow-up post 1 or a dedicated ACP post |
| `[rig handle: verify]` | rig agent framework | When posting about the rig recipe |
| `[bevy_egui maintainer: verify]` | Game UI | Game menu or HUD posts |

Check each placeholder handle on the project's own website or GitHub README before using it.

## Replying to comments

- [ ] Block out the first three hours after each post to answer. Early replies matter most.
- [ ] Thank people for bug reports and turn each into a GitHub issue; reply with the link.
- [ ] Answer "why not Slint / iced / Dioxus / Tauri" plainly: glowkit is for people who already
      chose egui or want immediate mode; it isn't trying to replace those.
- [ ] Performance questions: link the benchmarks page and give real numbers, including the slow
      components.
- [ ] Asked how much was built with coding agents: answer directly and the same way everywhere.
- [ ] Don't argue with bad-faith comments. One factual reply at most, then move on.
- [ ] Keep a running list of feature requests in a pinned Discussion, and close the loop when
      something ships ("this is in 0.1.1 now").
- [ ] End of day: retweet or quote the best demo replies, and note which clip did best so the
      follow-up schedule leads with similar ones.
