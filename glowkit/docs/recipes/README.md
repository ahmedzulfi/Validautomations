# Recipes: connect real agents

Each recipe is a complete, runnable app: what you get, the `Cargo.toml`, about 50 lines of code
and how to run it. They all use [`glowkit-stream`](../../crates/glowkit-stream): an adapter turns
the agent's stream into events, a `Session` folds them into a thread, and `AgentThread` draws it.

| Recipe | Talks to | Example |
|---|---|---|
| [Local LLM chat with Ollama](ollama-chat.md) | Ollama, vLLM, LM Studio, OpenRouter (OpenAI-compatible) | `examples/agent-ollama` |
| [A rig agent UI](rig-agent.md) | a [rig](https://docs.rs/rig-agent) agent in your process | `examples/agent-rig` |
| [A desktop front-end for an AG-UI agent](ag-ui-desktop.md) | LangGraph, CrewAI, Mastra, Pydantic AI, ADK... | `examples/agent-ag-ui` |
| [A native client for a Vercel AI SDK backend](vercel-ai-sdk-client.md) | any `toUIMessageStreamResponse()` route | `examples/agent-vercel` |
| [An ACP coding-agent viewer](acp-viewer.md) | agents speaking the Agent Client Protocol | gallery: "Live agent stream" |
