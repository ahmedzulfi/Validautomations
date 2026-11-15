<div align="center">

# ✦ glowkit

**Premium, animated UI for Rust apps, games and tools.**
AI agent interfaces, live dashboards, game menus and HUDs, node editors, charts, 3D and hundreds of
micro-interactions, for native Windows, macOS and Linux apps (and the web through WebAssembly).
100% Rust, built on [egui](https://github.com/emilk/egui). No Electron, no webview.

[Live gallery](https://glowkit-rs.github.io/glowkit/gallery/) · [Website](https://glowkit-rs.github.io/glowkit/) · [Catalog](CATALOG.md) · [Benchmarks](https://glowkit-rs.github.io/glowkit/benchmarks.html) · [Docs](https://docs.rs/glowkit) · [Customizing](crates/glowkit/docs/CUSTOMIZING.md) · [Contributing](CONTRIBUTING.md)

</div>

---

## Who it's for

Rust is where people are rewriting Electron apps into fast, native tools. glowkit gives those apps
the polish web apps get from React component libraries:

| You're building | glowkit gives you |
|---|---|
| **AI agent apps** (session managers, local LLM chat, agent fleets) | Streaming markdown chat, tool calls, approvals, diff review, traces, agent trees, run monitors, MCP and skills managers |
| **Dashboards** (trading, crypto, infra, analytics) | Animated charts, candlesticks, order books, Sankey, stat cards, data grids, live-updating numbers |
| **Games** (Bevy via `bevy_egui`) | Main and pause menus, HUD bars, minimaps, inventories, skill trees, dialogue, loot and achievement popups |
| **Dev tools** (editors, inspectors, profilers) | Node editor, infinite canvas, timeline, flame graphs, command palette, log consoles |
| **Any native app that should feel premium** | 600+ animated blocks: transitions, morphs, physics-based interactions, text effects, backgrounds, 3D |

## What's inside

* **682 showcased blocks and components** across AI agents, dashboards, game UI, micro-interactions,
  transitions, charts, canvas editors, 3D, text effects, backgrounds and full app templates.
* **A real motion engine.** Springs, keyframes, GSAP-style timelines, variants with stagger, enter/exit presence, layout animations, drag with inertia.
* **3D without the setup.** A built-in software 3D renderer draws lit, depth-sorted meshes straight into egui: no wgpu boilerplate.
* **Native everywhere.** One codebase runs as a native app on Windows, macOS and Linux, and in
  the browser via WebAssembly. No webview, no JavaScript.
* **Themeable.** Semantic design tokens (`background`, `foreground`, `primary`, `accent`,
  `border`, `ring`, gradients, chart palette...). Five presets, or your own. One call restyles
  every glowkit component *and* egui's built-in widgets.
* **Animation engine included.** 31 easing curves, springs, staggering, triggers
  (on appear / loop / hover / click), noise, gradients, glows and per-glyph text layout.
* **Modular.** Each category is its own crate behind a cargo feature, so you only compile what you use.
* **Parity with the web, measured.** [CATALOG.md](CATALOG.md) is generated from the code and maps
  components from 20+ popular web libraries (React Bits, Magic UI, Aceternity, Motion Primitives,
  AI Elements, prompt-kit, ElevenLabs UI, Bencho, beUI, Snapblocks, Originkit, BoardUI and more) to
  their glowkit equivalents. The lists are a snapshot; PRs that update them are welcome.
* **Original implementations.** glowkit recreates well-known UI *patterns* in Rust from scratch.

## Quick start

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
            let rect = ui.max_rect();
            Aurora::new().paint(ui, rect);
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

Runnable examples:

```bash
cargo run -p glowkit-example-hello     # animated hero screen with a toast
cargo run -p glowkit-example-ai-chat   # chat app with a streaming (fake) model
cargo run --release -p glowkit-gallery  # every component
```

## What's inside

| Crate | Feature | What |
|---|---|---|
| `glowkit-core` | always | Theme tokens, easings, springs, clocks, gradients, glows, noise, per-glyph text |
| `glowkit-text` | `text` | Shiny, gradient, blur, split, decrypt, typewriter, rotating words, count-up, glitch, circular text... |
| `glowkit-backgrounds` | `backgrounds` | Aurora, particles, grids, waves, beams, meteors, starfields, silk, plasma, warp speed... |
| `glowkit-effects` | `effects` | Animated buttons, border beams, spotlight / tilt / glare cards, cursor trails, dock, marquee, carousels... |
| `glowkit-components` | `components` | Buttons, inputs, switches, selects, date picker, tabs, dialogs, toasts, tables, spinners... |
| `glowkit-charts` | `charts` | Animated line, area, bar, pie, donut, radar, radial, gauge, heatmap, treemap, sparklines... |
| `glowkit-ai` | `ai` | Chat messages, streaming markdown, prompt input, reasoning, tool calls, sources, voice orb, waveform... |
| `glowkit-blocks` | `blocks` | Hero sections, feature grids, bento, pricing, testimonials, terminals, globes, device mockups |
| `glowkit-motion` | `motion` | Framer-Motion-style engine: animated values, keyframes, timelines, variants, presence, layout (FLIP), drag & gestures, scroll-linked motion, path motion, shape morphing |
| `glowkit-3d` | `three-d` | Real-time 3D inside egui: camera, orbit controls, meshes & OBJ loading, lighting & materials, particle clouds, terrains, 3D cards, globes, voxel text |
| `glowkit-canvas` | `canvas` | Infinite canvas, node editor (React Flow style), whiteboard, kanban, gantt, mind map, timeline editor, data grid, org chart, signature pad |
| `glowkit-templates` | `templates` | MVP-Blocks-style sections, full pages (landing, auth, 404, pricing...), dashboard shells and 15 app templates |

Browse everything in the **[live gallery](https://glowkit-rs.github.io/glowkit/gallery/)**, or run it locally:

```bash
cargo run --release -p glowkit-gallery
```

## Theming

```rust
// built-in presets: dark (default), light, ocean, ember, matrix
Theme::light().install(ctx);

// or your own brand
let mut t = Theme::dark();
t.accent = hex("#FF4F00");
t.gradient = [hex("#FF4F00"), hex("#FF0080"), hex("#7928CA")];
t.radius = 14.0;
t.install(ctx);
```

Every component reads its colors from the active theme and offers setters to override them.
The full [customization guide](crates/glowkit/docs/CUSTOMIZING.md) covers tokens, component
options, triggers, springs, the motion engine and composition.

## Use with AI agents

glowkit ships an MCP server so coding agents (Claude Code, Cursor, Codex...) use the real API
instead of guessing it: ranked component search, exact `use` paths (compiler-verified),
constructors and builder setters, full app templates, theme tokens and egui 0.37 migration notes.
The catalog is compiled into the binary: no network, no keys.

```bash
cargo install glowkit-mcp
claude mcp add glowkit -- glowkit-mcp        # Claude Code
```

Cursor: put `{"mcpServers": {"glowkit": {"command": "glowkit-mcp"}}}` in `.cursor/mcp.json`.
Codex and other clients: see [agents/mcp](agents/mcp/). Tools: `search_components`,
`get_component`, `get_template`, `list_categories`, `get_theme`, `get_setup`, `egui_037_notes`;
plus a `build-app` prompt.

* [agents/AGENTS.md](agents/AGENTS.md): drop-in instructions for your project (`AGENTS.md` / `CLAUDE.md`).
* [agents/skills/glowkit](agents/skills/glowkit/SKILL.md): an agent skill for building polished glowkit UIs
  (copy it to `.claude/skills/glowkit/`).
* [agents/starter](agents/starter/): a starter app (app shell, dashboard, settings) to copy.

## Running the web gallery

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130
./scripts/build-web.sh          # builds website/gallery/pkg
./scripts/serve.sh              # http://localhost:8787
```

## Performance

Components paint with egui's GPU tessellated meshes. Shader-like backgrounds use coarse vertex
grids (e.g. 48 x 28) with per-vertex color, so they stay cheap. Continuous animations request
repaints; one-shot animations stop repainting when they finish, so idle apps stay idle.

## Status

`0.1` is the first release. The API may change before `1.0`. See [CATALOG.md](CATALOG.md) for
coverage and open items, and [CONTRIBUTING.md](CONTRIBUTING.md) to help close them.

## Credits

glowkit is inspired by the work of the authors of React Bits, Magic UI, Aceternity UI, Motion
Primitives, shadcn/ui, Origin UI, Kibo UI, Animate UI, Cult UI, Uiverse, Vercel AI Elements,
prompt-kit, ElevenLabs UI, Evil Charts, Tremor and many more. No code from those projects is
included. All names belong to their respective owners and are used only to describe equivalence.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
