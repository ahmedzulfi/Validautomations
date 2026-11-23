<div align="center">

# glowkit

**Animated, polished UI components for native Rust apps, built on egui.**

[![crates.io](https://img.shields.io/crates/v/glowkit.svg)](https://crates.io/crates/glowkit)
[![docs.rs](https://img.shields.io/docsrs/glowkit)](https://docs.rs/glowkit)
[![CI](https://github.com/ahmedzulfi/glowkit/actions/workflows/ci.yml/badge.svg)](https://github.com/ahmedzulfi/glowkit/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![made with egui](https://img.shields.io/badge/made%20with-egui-6e56cf)](https://github.com/emilk/egui)

[Website](https://ahmedzulfi.github.io/glowkit/) · [Live gallery](https://ahmedzulfi.github.io/glowkit/gallery/) · [Catalog](CATALOG.md) · [Benchmarks](https://ahmedzulfi.github.io/glowkit/benchmarks.html) · [Docs](https://docs.rs/glowkit)

<img src="https://raw.githubusercontent.com/ahmedzulfi/glowkit/main/assets/readme/hero.webp" alt="glowkit components recorded from the live gallery: a streaming agent thread, an agent sessions app, a trading terminal, a game menu, a refracting glass panel and a 3D particle morph" width="100%">

</div>

glowkit gives Rust desktop apps, AI agent tools, dashboards and games the kind of polish web apps
get from React component libraries: 683 animated components for Windows, macOS, Linux and the web.

## Why glowkit

* **Native, not a webview.** Pure Rust on [egui](https://github.com/emilk/egui) 0.37. One codebase
  runs as a native app on Windows, macOS and Linux, and in the browser through WebAssembly.
* **Big and themeable.** 683 components across 21 categories, from buttons to full app templates.
  Semantic design tokens restyle every glowkit component and egui's own widgets in one call.
* **Real agent UIs.** Streaming Markdown, reasoning, tool calls and approvals, driven by real
  protocols: AG-UI, the Vercel AI SDK stream, the Agent Client Protocol, async-openai and rig.
* **Motion that stays cheap.** Springs, timelines, layout animations, a software 3D renderer and
  refracting glass, measured per component in [benchmarks](https://ahmedzulfi.github.io/glowkit/benchmarks.html).
  One-shot animations stop repainting when they finish, so idle apps stay idle.

## A few of them

Recorded from the [live gallery](https://ahmedzulfi.github.io/glowkit/gallery/) (WebAssembly build of the same code).

<table>
<tr>
<td width="33%"><img src="https://raw.githubusercontent.com/ahmedzulfi/glowkit/main/assets/readme/agent-thread.webp" alt="An agent thread streaming text, a tool call and an approval from a recorded AG-UI run"><br><sub><b>Agent thread</b>: a recorded AG-UI run, live</sub></td>
<td width="33%"><img src="https://raw.githubusercontent.com/ahmedzulfi/glowkit/main/assets/readme/agent-sessions.webp" alt="Agent sessions app template"><br><sub><b>Agent sessions</b>: app template</sub></td>
<td width="33%"><img src="https://raw.githubusercontent.com/ahmedzulfi/glowkit/main/assets/readme/trading.webp" alt="Trading terminal template with a live order book and candlesticks"><br><sub><b>Trading terminal</b>: dashboard template</sub></td>
</tr>
<tr>
<td><img src="https://raw.githubusercontent.com/ahmedzulfi/glowkit/main/assets/readme/game-menu.webp" alt="Synthwave game main menu"><br><sub><b>Game menu</b>: one of 25 game UI blocks</sub></td>
<td><img src="https://raw.githubusercontent.com/ahmedzulfi/glowkit/main/assets/readme/glass.webp" alt="Glass panel refracting text and color as it drifts behind it"><br><sub><b>Glass surface</b>: refraction and dispersion</sub></td>
<td><img src="https://raw.githubusercontent.com/ahmedzulfi/glowkit/main/assets/readme/3d.webp" alt="Particle cloud morphing between 3D shapes"><br><sub><b>Particle morph</b>: software 3D, no GPU setup</sub></td>
</tr>
<tr>
<td><img src="https://raw.githubusercontent.com/ahmedzulfi/glowkit/main/assets/readme/motion.webp" alt="Cards rearranging between list and grid layouts with spring animations"><br><sub><b>Layout animation</b>: the motion engine</sub></td>
<td><img src="https://raw.githubusercontent.com/ahmedzulfi/glowkit/main/assets/readme/text.webp" alt="Text scrambling through random symbols and decoding into ACCESS GRANTED"><br><sub><b>Decrypted text</b>: one of 57 text effects</sub></td>
<td><img src="https://raw.githubusercontent.com/ahmedzulfi/glowkit/main/assets/readme/charts.webp" alt="Area charts animating between this year and last year"><br><sub><b>Area charts</b>: animated data changes</sub></td>
</tr>
</table>

Everything else is in the [gallery](https://ahmedzulfi.github.io/glowkit/gallery/) and in
[CATALOG.md](CATALOG.md).

## Quick start

Requires Rust 1.95 or newer (egui 0.37's minimum).

```toml
[dependencies]
glowkit = "0.1"
eframe = "0.37"
```

```rust
use glowkit::prelude::*;
use glowkit::{backgrounds::Aurora, effects::ShimmerButton, text::ShinyText};

fn main() -> eframe::Result {
    eframe::run_native("My app", Default::default(), Box::new(|cc| {
        Theme::dark().install(&cc.egui_ctx);
        Ok(Box::new(App))
    }))
}

struct App;

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            Aurora::new().paint(ui, ui.max_rect());
            ui.vertical_centered(|ui| {
                ui.add_space(120.0);
                ui.add(ShinyText::new("Hello from Rust").size(48.0));
                if ui.add(ShimmerButton::new("Get started")).clicked() {
                    // ...
                }
            });
        });
    }
}
```

All categories are on by default. To compile less, pick features:
`glowkit = { version = "0.1", default-features = false, features = ["components", "charts"] }`.

```bash
cargo run -p glowkit-example-hello       # animated hero screen with a toast
cargo run -p glowkit-example-ai-chat     # chat app with a fake streaming model
cargo run --release -p glowkit-gallery   # every component, natively
```

Theming is one call: `Theme::light().install(ctx)`, one of five presets, or your own tokens
(`accent`, `gradient`, `radius`...). See the [customization guide](crates/glowkit/docs/CUSTOMIZING.md).

## Connect real agents

[`glowkit-stream`](crates/glowkit-stream) drives the AI components from real LLM and agent streams.
An adapter turns what your agent speaks into one event model, a `Session` folds the events into a
thread, and `AgentThread` renders it: streaming Markdown, reasoning, tool calls with live
arguments, approvals that go back to the agent, plans, sources and errors.

```rust
rx.drain_into(&mut session);                        // events from any thread or runtime
let out = AgentThread::new(&mut session).show(ui);  // the whole thread
for decision in out.decisions { /* send the approval back */ }
```

| Recipe | Talks to |
|---|---|
| [Local LLM chat with Ollama](docs/recipes/ollama-chat.md) | Ollama, vLLM, LM Studio, OpenRouter (async-openai) |
| [A rig agent UI](docs/recipes/rig-agent.md) | a rig agent in your process |
| [A desktop front-end for an AG-UI agent](docs/recipes/ag-ui-desktop.md) | LangGraph, CrewAI, Mastra, Pydantic AI, ADK |
| [A native client for a Vercel AI SDK backend](docs/recipes/vercel-ai-sdk-client.md) | any `toUIMessageStreamResponse()` route |
| [An ACP coding-agent viewer](docs/recipes/acp-viewer.md) | Agent Client Protocol agents |

```bash
cargo run -p glowkit-example-agent-ag-ui -- --replay    # offline replay of a recorded AG-UI run
cargo run -p glowkit-example-agent-vercel -- --replay   # same for a Vercel AI SDK stream
cargo run -p glowkit-example-agent-ollama               # a local model through Ollama
```

The `rig` adapter is the one exception to the 1.95 minimum: rig 0.45 and later need rustc 1.97.1.
On an older toolchain, pin rig 0.44 (`cargo update -p rig-core --precise 0.44.0`).

## Crates

Each category is its own crate behind a feature of the `glowkit` umbrella crate.

| Crate | Feature | What's in it |
|---|---|---|
| [`glowkit`](https://docs.rs/glowkit) | | Umbrella crate: re-exports everything below |
| [`glowkit-core`](https://docs.rs/glowkit-core) | always | Theme tokens, 31 easings, springs, clocks, gradients, glows, noise, per-glyph text |
| [`glowkit-text`](https://docs.rs/glowkit-text) | `text` | Shiny, gradient, blur, split, decrypt, typewriter, count-up, glitch, chrome... |
| [`glowkit-backgrounds`](https://docs.rs/glowkit-backgrounds) | `backgrounds` | Aurora, particles, grids, waves, beams, starfields, silk, plasma... |
| [`glowkit-effects`](https://docs.rs/glowkit-effects) | `effects` | Animated buttons, border beams, spotlight and tilt cards, glass, docks, menus |
| [`glowkit-components`](https://docs.rs/glowkit-components) | `components` | Inputs, selects, date picker, tabs, dialogs, toasts, tables, loaders |
| [`glowkit-charts`](https://docs.rs/glowkit-charts) | `charts` | Line, area, bar, pie, radar, gauge, heatmap, treemap, candlestick, sparkline... |
| [`glowkit-ai`](https://docs.rs/glowkit-ai) | `ai` | Chat, streaming Markdown, prompt input, reasoning, tool calls, voice orb |
| [`glowkit-agents`](https://docs.rs/glowkit-agents) | `agents` | Agent trees, traces, approvals, diff review, run monitors, copilot panels |
| [`glowkit-stream`](https://docs.rs/glowkit-stream) | `stream` | Real agent streams: event model, thread reducer, AG-UI / Vercel / ACP / async-openai / rig adapters |
| [`glowkit-motion`](https://docs.rs/glowkit-motion) | `motion` | Animated values, keyframes, timelines, variants, presence, layout (FLIP), drag, gestures |
| [`glowkit-3d`](https://docs.rs/glowkit-3d) | `three-d` | Software 3D in egui: camera, orbit controls, meshes and OBJ, lighting, globes, particles |
| [`glowkit-canvas`](https://docs.rs/glowkit-canvas) | `canvas` | Infinite canvas, node editor, whiteboard, kanban, gantt, mind map, data grid |
| [`glowkit-interactions`](https://docs.rs/glowkit-interactions) | `interactions` | Micro-interactions, transitions and game UI (menus, HUDs, inventories, skill trees) |
| [`glowkit-blocks`](https://docs.rs/glowkit-blocks) | `blocks` | Hero sections, feature grids, bento, pricing, terminals, device mockups |
| [`glowkit-templates`](https://docs.rs/glowkit-templates) | `templates` | Full pages, dashboard shells and app templates (agent sessions, trading, CRM...) |
| [`glowkit-mcp`](https://crates.io/crates/glowkit-mcp) | | MCP server for coding agents |
| [`glowkit-cli`](https://crates.io/crates/glowkit-cli) | | `glowkit add` / `glowkit init`: copy components as source you own |

## For AI coding agents

* **MCP server.** `cargo install glowkit-mcp`, then `claude mcp add glowkit -- glowkit-mcp`
  (Claude Code), or `{"mcpServers": {"glowkit": {"command": "glowkit-mcp"}}}` in
  `.cursor/mcp.json`. Ranked component search, exact `use` paths, builder setters, app templates,
  theme tokens and egui 0.37 notes, all compiled in: no network, no keys. Other clients:
  [agents/mcp](agents/mcp/).
* **CLI.** `cargo install glowkit-cli`, then `glowkit add shimmer-button` copies a component's
  source into your project, and `glowkit init my-app --template dashboard` scaffolds an app.
* **Skill and instructions.** [agents/skills/glowkit](agents/skills/glowkit/SKILL.md) is an agent
  skill for building glowkit UIs (copy it to `.claude/skills/glowkit/`), and
  [agents/AGENTS.md](agents/AGENTS.md) is a drop-in `AGENTS.md` / `CLAUDE.md`. There is also a
  [starter app](agents/starter/).

## Links

* [Website](https://ahmedzulfi.github.io/glowkit/) and [live gallery](https://ahmedzulfi.github.io/glowkit/gallery/)
* [CATALOG.md](CATALOG.md): every component, mapped to its equivalents in 20+ web libraries
* [Benchmarks](https://ahmedzulfi.github.io/glowkit/benchmarks.html): frame time, vertices and allocations per component
* [Customization guide](crates/glowkit/docs/CUSTOMIZING.md) and [API docs](https://docs.rs/glowkit)
* [Ports](ports/): experimental GPUI and Dioxus versions of the design language
* [CONTRIBUTING.md](CONTRIBUTING.md) · [CHANGELOG.md](CHANGELOG.md)

Building the web gallery yourself:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130
./scripts/build-web.sh    # builds website/gallery/pkg
./scripts/serve.sh        # http://localhost:8787
```

## Status

`0.1` is the first release. The API may change before `1.0`. Bug reports, component requests and
pull requests are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md).

## Credits

glowkit is inspired by the work of the authors of React Bits, Magic UI, Aceternity UI, Motion
Primitives, shadcn/ui, Origin UI, Kibo UI, Animate UI, Cult UI, Uiverse, Vercel AI Elements,
prompt-kit, ElevenLabs UI, Evil Charts, Tremor and many more. glowkit recreates well-known UI
patterns in Rust from scratch: no code from those projects is included. All names belong to their
respective owners and are used only to describe equivalence. Thanks to
[Emil Ernerfeldt](https://github.com/emilk) and the egui contributors for egui itself.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
Unless you state otherwise, any contribution you submit for inclusion in glowkit is dual-licensed
the same way, without additional terms or conditions.
