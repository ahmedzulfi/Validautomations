# glowkit MCP server: client configuration

`glowkit-mcp` is a stdio MCP server with the whole glowkit catalog compiled in: no network,
no API keys, no checkout needed.

```bash
cargo install glowkit-mcp                    # from crates.io
cargo install --path crates/glowkit-mcp      # or from a glowkit checkout
```

| Client | Setup |
|---|---|
| Claude Code | `claude mcp add glowkit -- glowkit-mcp` ([claude-code.sh](claude-code.sh)); `--scope project` writes a shared `.mcp.json` |
| Cursor | copy [cursor-mcp.json](cursor-mcp.json) to `.cursor/mcp.json` in your project (or `~/.cursor/mcp.json`) |
| Codex CLI | `codex mcp add glowkit -- glowkit-mcp`, or paste [codex-config.toml](codex-config.toml) into `~/.codex/config.toml` |
| Others (Claude Desktop, VS Code, Windsurf, Zed...) | the generic [mcp.json](mcp.json) shape: command `glowkit-mcp`, no args |

If `glowkit-mcp` is not on the `PATH` the client sees (GUI apps often have a minimal one), use
the absolute path, e.g. `~/.cargo/bin/glowkit-mcp`.

## What it offers

* Tools: `search_components`, `get_component`, `list_categories`, `get_template`, `get_theme`,
  `get_setup`, `egui_037_notes`.
* Resources: `glowkit://docs/customizing`, `glowkit://docs/conventions`, `glowkit://docs/egui-0.37`.
* Prompt: `build-app` (argument `what`, optional `platform`).

Check it works: `glowkit-mcp --version`. The data is regenerated from the sources with
`python3 scripts/gen-mcp-data.py`.
