# glowkit-mcp

An [MCP](https://modelcontextprotocol.io) server that lets AI coding agents (Claude Code, Cursor,
Codex and others) use [glowkit](https://github.com/ahmedzulfi/glowkit) correctly instead of guessing
its API. It runs over stdio and has the whole component catalog compiled in: no network, no API
keys, no checkout needed.

```bash
cargo install glowkit-mcp
claude mcp add glowkit -- glowkit-mcp        # Claude Code
codex mcp add glowkit -- glowkit-mcp         # Codex CLI
```

Cursor: put `{"mcpServers": {"glowkit": {"command": "glowkit-mcp"}}}` in `.cursor/mcp.json`.
Other clients take the same shape: command `glowkit-mcp`, no arguments. More setups are in
[agents/mcp](https://github.com/ahmedzulfi/glowkit/tree/main/agents/mcp).

## What it offers

* **Tools:** `search_components` (ranked search), `get_component` (exact `use` path, constructor,
  builder setters, example), `list_categories`, `get_template` (full app templates), `get_theme`
  (design tokens), `get_setup` (Cargo.toml and `main.rs` for a new app), `egui_037_notes`
  (migration notes for egui 0.37).
* **Resources:** `glowkit://docs/customizing`, `glowkit://docs/conventions`,
  `glowkit://docs/egui-0.37`.
* **Prompt:** `build-app`.

Check it works with `glowkit-mcp --version`.

## License

MIT or Apache-2.0, at your option.
