#!/usr/bin/env sh
# Register the glowkit MCP server with Claude Code (run once).
# Install the server first:  cargo install glowkit-mcp
#   (or from a checkout:     cargo install --path crates/glowkit-mcp)
claude mcp add glowkit -- glowkit-mcp

# Share it with everyone working in this project instead (writes .mcp.json):
# claude mcp add --scope project glowkit -- glowkit-mcp
