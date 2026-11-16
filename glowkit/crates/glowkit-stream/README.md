# glowkit-stream

Real LLM and agent streams, rendered natively. `glowkit-stream` turns what your agent speaks into
one normalized event model, folds it into a thread, and draws that thread with the
[glowkit-ai](../glowkit-ai) components: streaming Markdown, reasoning, tool calls with live
arguments, approvals, plans, sources and errors.

```text
agent / LLM ──► adapter ──► AgentEvent ──► bridge (any thread) ──► Session ──► AgentThread (egui)
                                                                     ▲                 │
                                                                     └── approvals ◄───┘
```

* **`AgentEvent`**: run start / finish (with a stop reason), steps, text and reasoning deltas,
  tool call start / argument deltas / parsed input / result, approval request and decision, plan,
  sources, usage, errors and custom events.
* **`Session`**: a pure reducer (`Clone + Send + Sync`, no egui) that builds messages made of
  parts: `Text`, `Reasoning`, `ToolCall` (pending / running / done / error / denied), `Approval`,
  `Plan`, `Source`, `Error`. It tolerates missing start events and out-of-order events, keeps
  parallel tool calls apart by id and previews half-streamed JSON arguments.
* **`bridge`**: a `std::sync::mpsc` sender / receiver. Push from tokio, async-std, smol or a plain
  thread; `with_repaint(ctx)` wakes egui; drain into the session once per frame.
* **`AgentThread`** (feature `egui`): renders the session and returns the user's approval
  decisions so you can send them back.
* **Adapters**, each behind a feature:

| Feature | Source | Dependencies |
|---|---|---|
| `ag-ui` | [AG-UI](https://docs.ag-ui.com) 1.x events (LangGraph, CrewAI, Mastra, Pydantic AI, ADK...) | none (own serde types) |
| `vercel` | Vercel AI SDK v7 UI message stream (`toUIMessageStreamResponse()`) | none (own serde types) |
| `acp-wire` | Agent Client Protocol JSON-RPC (`session/update`, `session/request_permission`) | none |
| `acp` | typed ACP, `agent-client-protocol` 5.x schema types | agent-client-protocol |
| `async-openai` | Chat Completions chunks (+ raw `reasoning_content` fallback for Ollama, vLLM, LM Studio, OpenRouter) and Responses API events | async-openai (types only) |
| `rig` | `rig_core::streaming::StreamEvent` and `rig_agent::agent::MultiTurnStreamItem` | rig-core, rig-agent |
| `http` | `http::stream_sse`: POST to an AG-UI or Vercel endpoint, stream into the bridge | reqwest |

The default feature set is empty: the core needs only `serde` and `serde_json`, and builds for
`wasm32-unknown-unknown`.

## Quick start

```toml
[dependencies]
glowkit = { version = "0.1", features = ["ai"] }
glowkit-stream = { version = "0.1", features = ["egui", "ag-ui"] }
eframe = "0.37"
```

```rust,ignore
use glowkit_stream::adapters::{SseDecoder, SseProtocol};
use glowkit_stream::{bridge, AgentThread, Session};

// once
let (tx, mut rx) = bridge::channel();
let tx = tx.with_repaint(&ctx);

// on your async runtime: bytes in, events out
let mut decoder = SseDecoder::new(SseProtocol::AgUi);
while let Some(chunk) = body.next().await {
    tx.send_all(decoder.feed(&chunk?))?;
}

// in your egui app, every frame
rx.drain_into(&mut session);
let out = AgentThread::new(&mut session).show(ui);
for decision in out.decisions {
    // send it back: ag_ui::resume_entry, vercel::ui_messages, acp::permission_response...
}
```

## Offline replay

`fixtures` bundles realistic recorded runs (AG-UI, Vercel and ACP) and `replay::Replay` plays any
event list back on a clock, holding at approvals until the user answers. The gallery's
"Live agent stream" demo and every example's `--replay` mode use them.

## Recipes

* [Local LLM chat with Ollama](../../docs/recipes/ollama-chat.md)
* [A rig agent UI](../../docs/recipes/rig-agent.md)
* [A desktop front-end for an AG-UI agent](../../docs/recipes/ag-ui-desktop.md)
* [A native client for a Vercel AI SDK backend](../../docs/recipes/vercel-ai-sdk-client.md)
* [An ACP coding-agent viewer](../../docs/recipes/acp-viewer.md)

## Versions

| Library | Version | Notes |
|---|---|---|
| async-openai | 0.43 | `chat-completion-types`, `response-types` |
| agent-client-protocol | 5.x | schema types under `agent_client_protocol::v1::schema` |
| rig-core / rig-agent | 0.44 to 0.46 | verified against both 0.44 and 0.46; rig 0.45+ needs rustc 1.97.1 |
| AG-UI | 1.0 | written from the 1.0 JSON schema; deprecated `THINKING_*` accepted |
| Vercel AI SDK | v7 | UI message stream v1 (`x-vercel-ai-ui-message-stream: v1`) |
