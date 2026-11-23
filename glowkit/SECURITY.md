# Security policy

## Supported versions

glowkit is pre-1.0. Security fixes go into the latest published `0.x` release.

| Version | Supported |
|---|---|
| 0.1.x | yes |

## Reporting a vulnerability

Please do not open a public issue for security problems.

Report privately through GitHub's
[private vulnerability reporting](https://github.com/ahmedzulfi/glowkit/security/advisories/new)
(Security tab, "Report a vulnerability"). Include the affected crate and version, what an attacker
can do, and steps or code to reproduce it.

You can expect an acknowledgement within a week. Once a fix is ready, it is released and an
advisory is published, crediting you unless you prefer otherwise.

## Scope

Most of glowkit is UI drawing code with no I/O. The parts most worth a look are the ones that
handle untrusted input:

* `glowkit-stream`: parsing agent streams (SSE, AG-UI, Vercel AI SDK, ACP JSON-RPC) and the
  optional `http` client.
* The streaming Markdown renderer in `glowkit-ai`, which renders model output.
* `glowkit-mcp` (an MCP server over stdio) and `glowkit-cli` (writes component source files into
  your project).

Bugs in the dependencies (egui, reqwest, rmcp and so on) should be reported to those projects.
