# Launch thread for X

Post 1 goes out on its own; the rest go in as replies to it. Each post is under 280 characters
(X counts every link as 23). Media lives in `launch/media/`. Upload the MP4s, not GIFs: X
re-encodes GIFs badly.

Run `python3 launch/count.py launch/x-thread.md` after editing to re-check lengths.

---

## 1/10 (opening hook)

Native Rust apps deserve the polish web apps get from React component libraries.

So I built one for egui. glowkit: 683 animated components for desktop apps, AI agent UIs, dashboards and games. No webview, no JavaScript. MIT/Apache.

[media: 00-montage.mp4]

### Alternative hook A

Web devs get shadcn, Magic UI and a dozen animated component libraries. Rust desktop apps mostly get a button and a slider.

I wanted that gap closed, so I spent the last few months building glowkit: 683 animated components for egui.

[media: 00-montage.mp4]

### Alternative hook B

Everything in this clip is egui. Agent threads, a trading terminal, game menus, refracting glass, software 3D.

683 components, running natively on Windows, macOS and Linux, and in the browser through WebAssembly. It's called glowkit and it's open source.

[media: 00-montage.mp4]

---

## 2/10

The part I care about most: agent UIs.

Streaming Markdown, reasoning, tool calls with live arguments, approvals that go back to the agent. glowkit-stream has adapters for AG-UI, the Vercel AI SDK stream, ACP, async-openai and rig. Real protocols, not a mock.

[media: 01-agent-thread.mp4]

## 3/10

Under that is one event model. An adapter turns what your agent speaks into events, a Session folds them into a thread, and one widget draws it:

rx.drain_into(&mut session);
AgentThread::new(&mut session).show(ui);

Approvals come back out so you can answer the agent.

[media: 14-agent-fleet.mp4]

## 4/10

There are full app templates too: an agent session manager, an MCP server dashboard, a local LLM chat, a trading terminal, CRM, analytics.

Copy one, swap the fake data for yours, and you have a native app that doesn't look like a debug panel.

[media: 02-agent-sessions.mp4]

## 5/10

25 game UI blocks: main and pause menus, HUDs, minimaps, inventories, skill trees, dialogue, loot popups.

They're ordinary egui widgets with a theme, so they go wherever egui already runs in your game.

[media: 05-game-menu.mp4]

## 6/10

Under it all is a small motion engine: springs, keyframes, timelines, stagger, enter/exit presence, layout animations, drag with inertia.

One-shot animations stop requesting repaints when they finish, so an idle app stays idle.

[media: 13-layout-animation.mp4]

## 7/10

Some of it was me finding out how far an immediate-mode painter goes.

A glass panel that refracts what's behind it, with color fringes. A software 3D renderer that draws lit, depth-sorted meshes straight into egui. No shaders, no GPU pipeline to set up.

[media: 07-glass-surface.mp4]

## 8/10

Every component is benchmarked headlessly: frame time, vertex count, allocations.

The glass panel started at 180 ms per frame. Refining only the rim, culling before subdividing and caching meshes got it to about 10 ms at p95. All the numbers are public.

[media: 09-particle-morph.mp4]

## 9/10

If you build with coding agents: there's an MCP server with the whole catalog compiled in (exact use paths, setters, templates), a CLI that copies components into your project as source you own, and an agent skill.

No network, no keys.

[media: 11-node-editor.mp4]

## 10/10

glowkit is 0.1 and I'm one person, so expect rough edges. Issues and PRs are very welcome.

Code: https://github.com/ahmedzulfi/glowkit
Live gallery (Rust compiled to WASM): https://ahmedzulfi.github.io/glowkit/gallery/

Built on egui by @ernerfeldt.

[media: 12-area-chart.mp4]
