# A desktop front-end for an AG-UI agent

**What you get:** a native desktop app for any agent that speaks [AG-UI](https://docs.ag-ui.com):
LangGraph, CrewAI, Mastra, Pydantic AI, Google ADK, LlamaIndex and others ship AG-UI endpoints.
Streamed text and reasoning, steps, tool calls with live arguments and results, and
human-in-the-loop approvals: when a run pauses with an interrupt, an approval card appears, and
the user's answer resumes the run. State snapshots and deltas, activities, sub-agents and custom
events are kept on the session for your own panels.

## Cargo.toml

```toml
[dependencies]
glowkit = { version = "0.1", features = ["ai"] }
glowkit-stream = { version = "0.1", features = ["egui", "http"] }   # http enables ag-ui
eframe = "0.37"
reqwest = "0.13"
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread"] }
```

## Code

```rust
use glowkit::{ai::PromptInput, prelude::*};
use glowkit_stream::adapters::{ag_ui, SseProtocol};
use glowkit_stream::{bridge, http, AgentThread, Session};
use serde_json::Value;

struct App {
    url: String,
    session: Session,
    input: String,
    runs: u32,
    tx: bridge::EventSender,
    rx: bridge::EventReceiver,
    rt: tokio::runtime::Runtime,
    client: reqwest::Client,
}

impl App {
    /// One AG-UI run: the conversation so far, plus answers to the last run's interrupts.
    fn run(&mut self, resume: Vec<Value>) {
        self.runs += 1;
        let body = ag_ui::run_input("thread-1", &format!("run-{}", self.runs), ag_ui::messages(&self.session), resume);
        let (client, tx, url) = (self.client.clone(), self.tx.clone(), self.url.clone());
        self.rt.spawn(async move { http::stream_sse(&client, &url, &body, SseProtocol::AgUi, &tx).await });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        self.rx.drain_into(&mut self.session);
        egui::Panel::bottom("prompt").show(ui, |ui| {
            let out = PromptInput::new(&mut self.input).generating(self.session.is_running()).show(ui, |_| {});
            if let Some(prompt) = out.submitted {
                self.session.push_user(prompt);
                self.run(vec![]);
            }
        });
        egui::CentralPanel::default().show(ui, |ui| {
            let out = AgentThread::new(&mut self.session).max_width(760.0).show(ui);
            if !out.decisions.is_empty() {
                self.run(out.decisions.iter().map(|d| ag_ui::resume_entry(&d.approval_id, d.approved)).collect());
            }
        });
    }
}

fn main() -> eframe::Result {
    let url = std::env::args().nth(1).expect("usage: app <AG-UI endpoint URL>");
    eframe::run_native("AG-UI agent", Default::default(), Box::new(|cc| {
        Theme::dark().install(&cc.egui_ctx);
        let (tx, rx) = bridge::channel();
        Ok(Box::new(App { url, session: Session::new(), input: String::new(), runs: 0,
            tx: tx.with_repaint(&cc.egui_ctx), rx, rt: tokio::runtime::Runtime::new()?, client: reqwest::Client::new() }))
    }))
}
```

## Run it

```bash
# offline: replays a bundled AG-UI recording (reasoning, two tool calls, an approval, an error)
cargo run -p glowkit-example-agent-ag-ui -- --replay

# against your agent, e.g. a LangGraph / CrewAI / Mastra / Pydantic AI AG-UI endpoint
cargo run -p glowkit-example-agent-ag-ui -- http://localhost:8000/agent
```

The adapter uses its own serde types written from the AG-UI 1.0 schema. Unknown event types are
ignored, `TEXT_MESSAGE_CHUNK` / `TOOL_CALL_CHUNK` are handled, and the deprecated `THINKING_*`
events are accepted as aliases.
