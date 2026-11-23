# Follow-up posts (weeks 1 to 3)

Standalone posts for after the launch thread, roughly one every day or two. Each is under 280
characters (links count as 23). Media is in `launch/media/`. Post the "how I built it" ones in the
morning US time; they tend to get replies from people who want details, so be around for an hour.

Check lengths with `python3 launch/count.py launch/posts.md`.

---

## 1. Agent thread, real protocols (day 2)

A recorded agent run, replayed through the same reducer a live one uses: reasoning, streamed Markdown, a tool call, an approval that pauses the agent until you answer.

Switch the source between AG-UI, the Vercel AI SDK stream and ACP. Same widget.

[media: 01-agent-thread.mp4]

## 2. How I built it: streaming Markdown (day 3)

Streaming Markdown means drawing half a document every frame: an open code fence, half a table, a dangling **.

So the parser is tolerant. An open fence is already a code block, unclosed bold runs to the paragraph's end, and a fresh marker stays hidden until its text arrives.

[media: 01-agent-thread.mp4]

## 3. Trading terminal template (day 4)

A trading terminal in egui: candlesticks, a live order book, positions, a watchlist with sparklines.

It's one of the app templates in glowkit. Everything is fake data driven by a seeded RNG, so you can replace it piece by piece with your own feed.

[media: 03-trading-terminal.mp4]

## 4. How I built it: the motion engine (day 5)

How I built a Framer-Motion-style engine for an immediate-mode UI: egui has no retained tree, so every animated value lives in per-widget state keyed by id.

Springs integrate per frame. Layout animations remember last frame's rect and animate the difference.

[media: 13-layout-animation.mp4]

## 5. Game menus (day 6)

Game menus are where UI polish shows the most, so glowkit has 25 game UI blocks: menus, HUDs, minimaps, inventories, skill trees, loot and achievement popups.

Three looks per block (sci-fi, fantasy, minimal), all from the same theme tokens.

[media: 05-game-menu.mp4]

## 6. How I built it: glass refraction perf (day 8)

The glass panel took 180 ms per frame when I first got it looking right.

What fixed it: only subdivide the mesh near the rim, where refraction actually bends things. Cull before subdividing, not after. Cache the prepared mesh. Now about 10 ms at p95.

[media: 07-glass-surface.mp4]

## 7. How I built it: software 3D (day 9)

How the 3D in glowkit works: no wgpu, no shaders. Vertices get projected on the CPU, triangles are lit and depth-sorted, and the result goes to egui as an ordinary mesh.

It won't run a game, but it's plenty for globes, particle clouds and 3D cards.

[media: 09-particle-morph.mp4]

## 8. Globe (day 10)

A dotted 3D globe, drawn into egui by a software renderer, lit and depth-sorted on the CPU.

The same renderer does model viewers with OBJ loading, terrains, voxel text and a solar system. No GPU setup code in your app.

[media: 08-globe-3d.mp4]

## 9. Text effects (day 11)

57 text effects in glowkit. This one scrambles through random symbols and locks each character into place; hover it and it decodes again from the center.

Text is laid out per glyph with ligatures intact, so effects can animate single characters without breaking shaping.

[media: 10-decrypted-text.mp4]

## 10. Dashboard template (day 12)

An analytics dashboard in egui, from the glowkit templates: sidebar shell, KPI cards, a revenue chart with a hover readout.

Charts animate between data sets instead of snapping, which matters more than I expected once the numbers update live.

[media: 04-analytics-dashboard.mp4]

## 11. Charts (day 14)

Charts that animate when the data changes: area, line, bar, radar, gauge, heatmap, treemap, funnel, candlestick and more.

They read colors from the theme's chart palette, so a light or dark switch restyles every series at once.

[media: 12-area-chart.mp4]

## 12. Node editor (day 15)

A node editor in egui: draggable nodes and ports, bezier, smooth step, step or straight edges, pan and zoom, undo, and a minimap.

There's also an agent pipeline view built on it, where stages light up as a run flows through.

[media: 11-node-editor.mp4]

## 13. Agent supervision blocks (day 16)

If you're building tools that watch many agents: fleet grids with status rings, trace waterfalls, run timelines, approval queues, diff review, cost meters.

All plain egui widgets with fictional data, ready to wire up.

[media: 15-trace-waterfall.mp4]

## 14. MCP server (day 18)

Coding agents kept guessing egui APIs from old versions, so glowkit ships an MCP server.

It knows every component's exact use path, constructor and setters, plus egui 0.37 migration notes. The data is compiled into the binary: no network, no keys.

cargo install glowkit-mcp

[media: 02-agent-sessions.mp4]

## 15. Game HUD (day 20)

A full game HUD from glowkit blocks: health and shield bars, a minimap, a hotbar with cooldowns, an ammo counter, a kill feed.

Each piece is its own widget, so you can take the minimap and leave the rest.

[media: 06-game-hud.mp4]
