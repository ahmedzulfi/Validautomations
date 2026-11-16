# Local LLM chat with Ollama

**What you get:** a native chat window for a model running on your machine. Answers stream in as
Markdown, and models that think out loud show their reasoning in a collapsible panel while it
streams. It works with any OpenAI-compatible server: Ollama, vLLM, LM Studio, OpenRouter.

## Cargo.toml

```toml
[dependencies]
glowkit = { version = "0.1", features = ["ai"] }
glowkit-stream = { version = "0.1", features = ["egui", "async-openai"] }
eframe = "0.37"
async-openai = { version = "0.43", features = ["chat-completion", "byot"] }
futures-util = "0.3"
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread"] }
```

## Code

```rust
use async_openai::{config::OpenAIConfig, Client};
use futures_util::StreamExt;
use glowkit::{ai::PromptInput, prelude::*};
use glowkit_stream::{adapters::openai::OpenAiChatAdapter, bridge, AgentThread, Role, Session};
use serde_json::{json, Value};

struct App {
    session: Session,
    input: String,
    tx: bridge::EventSender,
    rx: bridge::EventReceiver,
    rt: tokio::runtime::Runtime,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        self.rx.drain_into(&mut self.session);
        egui::Panel::bottom("prompt").show(ui, |ui| {
            let out = PromptInput::new(&mut self.input).generating(self.session.is_running()).show(ui, |_| {});
            if let Some(prompt) = out.submitted {
                self.session.push_user(prompt);
                let messages: Vec<Value> = self.session.messages.iter()
                    .map(|m| json!({"role": if m.role == Role::User { "user" } else { "assistant" }, "content": m.text()}))
                    .collect();
                let tx = self.tx.clone();
                self.rt.spawn(async move {
                    let base = std::env::var("OPENAI_BASE_URL").unwrap_or("http://localhost:11434/v1".into());
                    let client = Client::with_config(OpenAIConfig::new().with_api_base(base).with_api_key("local"));
                    let model = std::env::var("MODEL").expect("set MODEL to a model you pulled");
                    let request = json!({"model": model, "messages": messages, "stream": true});
                    // raw JSON chunks keep `reasoning_content` / `reasoning`
                    let mut chunks = client.chat().create_stream_byot::<_, Value>(request).await.unwrap();
                    let mut adapter = OpenAiChatAdapter::new();
                    while let Some(Ok(chunk)) = chunks.next().await {
                        tx.send_all(adapter.push_raw(&chunk)).ok();
                    }
                    tx.send_all(adapter.finish()).ok();
                });
            }
        });
        egui::CentralPanel::default().show(ui, |ui| {
            let _ = AgentThread::new(&mut self.session).max_width(760.0).show(ui);
        });
    }
}

fn main() -> eframe::Result {
    eframe::run_native("Local chat", Default::default(), Box::new(|cc| {
        Theme::dark().install(&cc.egui_ctx);
        let (tx, rx) = bridge::channel();
        let tx = tx.with_repaint(&cc.egui_ctx);
        let rt = tokio::runtime::Runtime::new()?;
        Ok(Box::new(App { session: Session::new(), input: String::new(), tx, rx, rt }))
    }))
}
```

## Run it

```bash
ollama serve                    # and `ollama pull <model>` once
MODEL=<model> cargo run --release

# the full example also picks the first listed model when MODEL is unset:
cargo run -p glowkit-example-agent-ollama
OPENAI_BASE_URL=http://localhost:1234/v1 cargo run -p glowkit-example-agent-ollama   # LM Studio
```

Errors (server down, unknown model) appear inline in the thread as error cards.
