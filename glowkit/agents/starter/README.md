# glowkit starter

A ready-to-run native app with [glowkit](https://github.com/ahmedzulfi/glowkit): an app shell
with a collapsible sidebar, command palette (Ctrl+K), notifications and theme toggle, plus a
dashboard (KPI cards, tabs, animated area chart) and a settings form.

```bash
cargo run --release
```

Copy this folder to start a new project, rename the package in `Cargo.toml`, and add screens in
`src/screens/`. `AGENTS.md` tells coding agents how to work on it; connect the glowkit MCP server
(`claude mcp add glowkit -- glowkit-mcp`) so they can look up components.
