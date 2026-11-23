# Changelog

All notable changes to glowkit are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0] - 2026-11-23

First public release: 16 library crates, an MCP server and a CLI, for egui 0.37 and Rust 1.95+.
The gallery showcases 683 components in 21 categories (701 including plain building blocks).

### Added

- **Core** (`glowkit-core`): semantic theme tokens with five presets (dark, light, ocean, ember,
  matrix) that also restyle egui's built-in widgets; 31 easing curves, springs, clocks and
  triggers (on appear, loop, hover, click); gradient, glow and shadow painting, gradient text,
  fake blur, shader-style vertex grids, noise, OKLab color mixing and ligature-safe per-glyph text.
- **Text** (`glowkit-text`): 57 animated text effects, from shiny and decrypted text to chrome,
  streaming and reasoning text.
- **Backgrounds** (`glowkit-backgrounds`): 63 animated backgrounds (aurora, particles, grids,
  waves, beams, starfields, silk, plasma and more).
- **Effects** (`glowkit-effects`): animated buttons, cards and borders, cursor effects, menus,
  docks, carousels, scroll reveals and refracting glass surfaces.
- **Components** (`glowkit-components`): everyday inputs and forms, navigation, overlays and
  toasts, data display and loaders.
- **Charts** (`glowkit-charts`): 29 animated chart demos including line, area, bar, pie, radar,
  gauge, heatmap, treemap, funnel, Sankey and candlestick charts.
- **AI** (`glowkit-ai`): chat messages, a streaming-safe Markdown renderer, prompt input,
  reasoning, tool calls, sources, voice orb, waveform and a complete chat demo.
- **Agents** (`glowkit-agents`): agent trees, trace waterfalls, approval queues, diff review, run
  monitors, MCP server and skills managers, fleet grids, copilot panels and tool UIs.
- **Agent streams** (`glowkit-stream`): a normalized event model, a pure `Session` reducer, a
  thread-safe UI bridge and the `AgentThread` view, with adapters for AG-UI 1.x, the Vercel AI SDK
  v7 UI message stream, the Agent Client Protocol (wire and typed), async-openai 0.43 and rig
  0.44 to 0.46, plus an SSE client and recorded fixtures for offline replay.
- **Motion** (`glowkit-motion`): a Framer-Motion-style engine with animated values, keyframes,
  timelines, variants and stagger, presence, layout (FLIP) animations, drag with inertia,
  gestures, scroll-linked and path motion, and shape morphing.
- **3D** (`glowkit-3d`): a software 3D renderer that draws lit, depth-sorted meshes straight into
  egui (camera, orbit controls, OBJ loading, materials) and 34 3D demos.
- **Canvas** (`glowkit-canvas`): infinite canvas, node editor, whiteboard, kanban, gantt, mind
  map, timeline editor, data grid, org chart and signature pad.
- **Interactions** (`glowkit-interactions`): 71 micro-interactions and transitions, and 25 game
  UI blocks (main and pause menus, HUDs, minimaps, inventories, skill trees, dialogue, loot).
- **Blocks and templates** (`glowkit-blocks`, `glowkit-templates`): page sections, full pages,
  dashboard shells and 28 app template screens (agent sessions, agent fleet, local LLM, trading
  terminal, CRM, analytics and more).
- **Umbrella crate** (`glowkit`): every category behind a cargo feature, all on by default.
- **Tools**: `glowkit-mcp`, an MCP server with the catalog compiled in, and `glowkit-cli`
  (`glowkit list / info / add / init`), which copies components into your project as source.
- Gallery app (native and WebAssembly) with theme builder and live playgrounds, a headless
  benchmark harness (`glowkit-bench`), runnable examples, recipes for connecting real agents, a
  generated parity catalog (CATALOG.md) and a doc-tested customization guide.

[Unreleased]: https://github.com/ahmedzulfi/glowkit/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ahmedzulfi/glowkit/releases/tag/v0.1.0
