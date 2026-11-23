# Reddit, This Week in Rust and Show HN drafts

Plain, technical, no marketing voice. These communities respond to specifics and to honesty about
limits. Answer every top-level comment in the first few hours.

Before posting anywhere, decide how you want to describe the role coding agents played in
building glowkit. Someone will ask, and a straight answer up front goes over much better than one
dragged out in the comments. A line for it is marked `[agents note]` below.

---

## r/rust

**Title:** glowkit: 683 animated egui components for desktop apps, agent UIs, dashboards and games

**Body:**

I've been building a component library for egui and it's ready for a first release.

The idea: web apps get shadcn/ui, Magic UI, React Bits and friends, and native Rust apps mostly
build their own buttons. glowkit tries to close that gap. It's 16 crates behind features of one
umbrella crate, on egui 0.37:

- everyday components (inputs, dialogs, toasts, tables) and 57 text effects, 63 backgrounds
- animated charts (area, candlestick, heatmap, treemap, Sankey...)
- a motion engine: springs, keyframes, timelines, presence, layout (FLIP) animations, drag with inertia
- a software 3D renderer that draws lit, depth-sorted meshes into egui as a normal mesh
- canvas tools: node editor, whiteboard, kanban, gantt, data grid
- 25 game UI blocks and a set of full app templates
- AI agent UI, plus `glowkit-stream`, which drives it from real streams: adapters for AG-UI,
  the Vercel AI SDK UI message stream, ACP, async-openai and rig. A pure `Session` reducer folds
  events into a thread; `AgentThread` draws it and hands approval decisions back.

Some details that might interest people here:

- Everything also builds for `wasm32-unknown-unknown`; the live gallery is the same code.
- One-shot animations stop requesting repaints when they finish, so idle apps don't spin.
- Every component is benchmarked headlessly (frame time, vertices, allocations) and the numbers
  are on the site. The worst offender was a refracting glass panel at 180 ms per frame; rim-only
  subdivision, culling before subdividing and cached meshes got it to about 10 ms at p95.
- MSRV is 1.95 because of egui 0.37. The rig adapter needs 1.97.1 for rig 0.45+.

Known limits: the API will change before 1.0, the 3D is CPU-side and meant for UI-scale scenes,
and some effects (the two refracting glass panels in particular) are expensive enough that you'd use one per screen,
not twenty. Some web components can't be reproduced well without real shaders; CATALOG.md lists
what's missing.

[agents note]

- Code (MIT/Apache): https://github.com/ahmedzulfi/glowkit
- Live gallery (WASM): https://ahmedzulfi.github.io/glowkit/gallery/
- Benchmarks: https://ahmedzulfi.github.io/glowkit/benchmarks.html

Feedback on API shape is the most useful thing right now, especially from people already shipping
egui apps.

---

## r/egui

**Title:** I built a large animated component library for egui 0.37 (683 components, MIT/Apache)

**Body:**

glowkit is a set of crates of animated components for egui 0.37: buttons, cards, text effects,
backgrounds, charts, a motion engine, a node editor, game UI, AI chat and agent views, and full
app templates. Live gallery (the WASM build): https://ahmedzulfi.github.io/glowkit/gallery/

How it fits into egui:

- Components are builder structs that implement `egui::Widget`, so it's `ui.add(ShinyText::new("Hi").size(32.0))`.
- Per-widget animation state lives in egui's memory keyed by the response id.
- `Theme::dark().install(ctx)` sets semantic tokens (background, accent, border, chart palette...)
  and also restyles egui's built-in widgets, so mixed UIs look consistent.
- Time comes from `ui.input` time, not `Instant`, so everything works on the web too.

Things I learned that might help other egui authors: never call `ui` or `ctx` methods inside a
memory closure (it holds the context lock and deadlocks; this froze my 3D orbit controls for a
while), and request repaints only while something is actually moving.

There's an MCP server and an agent skill so coding agents use the real API instead of guessing
from older egui versions.

[agents note]

Repo: https://github.com/ahmedzulfi/glowkit. I'd love to hear which components you'd actually use
and what's missing.

---

## This Week in Rust (submission blurb)

Submit through a PR to the This Week in Rust repository, under "Project/Tooling Updates", or
whichever section the current issue uses for new crates. One line:

> [glowkit 0.1: 683 animated egui components for desktop apps, AI agent UIs, dashboards and games](https://github.com/ahmedzulfi/glowkit)

If they take a short description:

> glowkit is a set of crates of animated components for egui 0.37 (charts, motion, software 3D,
> node editor, game UI, app templates), plus glowkit-stream, which renders real agent streams
> (AG-UI, Vercel AI SDK, ACP, async-openai, rig) in a native UI. MIT/Apache, runs natively and
> on the web.

---

## Show HN

**Title:** Show HN: Glowkit, animated UI components for native Rust apps

(Under 80 characters. Alternative: "Show HN: Glowkit, 683 animated egui components for native Rust apps")

**URL:** https://github.com/ahmedzulfi/glowkit

**Text:**

Hi HN. glowkit is a component library for egui, the immediate-mode GUI library for Rust. It
targets the kind of app people are rewriting from Electron into native Rust: agent tools,
dashboards, dev tools and games.

The gallery runs in the browser as WebAssembly, so you can try everything without installing
anything: https://ahmedzulfi.github.io/glowkit/gallery/

What's in it: 683 components (charts, text effects, a motion engine, a software 3D renderer, a
node editor, game menus and HUDs, full app templates) and an AI agent UI kit. The agent part is
the piece I'd most like feedback on: glowkit-stream turns AG-UI, Vercel AI SDK, ACP, async-openai
or rig streams into one event model, folds them into a thread with a pure reducer, and renders
it, including tool calls with half-streamed arguments and approvals that go back to the agent.

Some honest caveats:

- egui is immediate mode, so every animated component keeps a little state keyed by widget id
  and repaints while it moves. Idle apps do go idle, but a screen full of effects costs CPU.
- Per-component benchmarks are public. 598 of 700 components stay under 1 ms per frame at p95;
  the two glass showpieces cost 10 to 14 ms.
- The 3D is a CPU renderer for UI-scale scenes, not a game engine.
- It's 0.1 and the API will change.

[agents note]

MIT or Apache-2.0. Happy to answer questions about how any of it works.
