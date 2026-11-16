# A native client for a Vercel AI SDK backend

**What you get:** a desktop client for the backend you already wrote for the web. Any route that
returns `result.toUIMessageStreamResponse()` (what `useChat` reads) works unchanged: text and
reasoning parts, multi-step tool calls with streamed input, tool approvals
(`needsApproval: true`), sources, `data-*` parts and errors.

## Cargo.toml

```toml
[dependencies]
glowkit = { version = "0.1", features = ["ai"] }
glowkit-stream = { version = "0.1", features = ["egui", "http"] }   # http enables vercel
eframe = "0.37"
reqwest = "0.13"
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread"] }
```

## Code

```rust
use glowkit::{ai::PromptInput, prelude::*};
use glowkit_stream::adapters::{vercel, SseProtocol};
use glowkit_stream::{bridge, http, AgentThread, Session};

struct App {
    url: String,
    session: Session,
    input: String,
    tx: bridge::EventSender,
    rx: bridge::EventReceiver,
    rt: tokio::runtime::Runtime,
    client: reqwest::Client,
}

impl App {
    /// POST the conversation as UI messages, like `useChat` does.
    fn send(&self) {
        let body = serde_json::json!({ "messages": vercel::ui_messages(&self.session) });
        let (client, tx, url) = (self.client.clone(), self.tx.clone(), self.url.clone());
        self.rt.spawn(async move { http::stream_sse(&client, &url, &body, SseProtocol::Vercel, &tx).await });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        self.rx.drain_into(&mut self.session);
        egui::Panel::bottom("prompt").show(ui, |ui| {
            let out = PromptInput::new(&mut self.input).generating(self.session.is_running()).show(ui, |_| {});
            if let Some(prompt) = out.submitted {
                self.session.push_user(prompt);
                self.send();
            }
        });
        egui::CentralPanel::default().show(ui, |ui| {
            let out = AgentThread::new(&mut self.session).max_width(760.0).show(ui);
            if !out.decisions.is_empty() {
                // answered approvals travel as `approval-responded` tool parts
                self.send();
            }
        });
    }
}

fn main() -> eframe::Result {
    let url = std::env::args().nth(1).expect("usage: app http://localhost:3000/api/chat");
    eframe::run_native("AI SDK client", Default::default(), Box::new(|cc| {
        Theme::dark().install(&cc.egui_ctx);
        let (tx, rx) = bridge::channel();
        Ok(Box::new(App { url, session: Session::new(), input: String::new(),
            tx: tx.with_repaint(&cc.egui_ctx), rx, rt: tokio::runtime::Runtime::new()?, client: reqwest::Client::new() }))
    }))
}
```

## Run it

```bash
cargo run -p glowkit-example-agent-vercel -- --replay                        # offline recording
cargo run -p glowkit-example-agent-vercel -- http://localhost:3000/api/chat  # your Next.js / Node route
```

`vercel::ui_messages` builds AI SDK `UIMessage`s from the session (text, reasoning and
`tool-<name>` parts with their state). If your route expects extra fields (an id, a model
choice), wrap them around the `messages` array.
