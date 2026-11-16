# A rig agent UI

**What you get:** a native window for a [rig](https://docs.rs/rig-agent) agent running in your
own process. Each turn shows the model's reasoning, every tool call as a card (arguments streaming
in, then the result), and the final answer, all as it happens. Token usage feeds the thread too.

## Cargo.toml

```toml
[dependencies]
glowkit = { version = "0.1", features = ["ai"] }
glowkit-stream = { version = "0.1", features = ["egui", "rig"] }
eframe = "0.37"
rig-core = { version = "0.46", features = ["reqwest", "rustls"] }
rig-agent = "0.46"
futures-util = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread"] }
```

## Code

```rust
use futures_util::StreamExt;
use glowkit::{ai::PromptInput, prelude::*};
use glowkit_stream::{adapters::rig::RigAdapter, bridge, AgentThread, Session};
use rig_agent::tool::{Tool, ToolContext};
use rig_agent::{Agent, AgentBuilder};
use rig_core::providers::ollama::Ollama;
use std::sync::Arc;

struct Add;
#[derive(serde::Deserialize)]
struct AddArgs { a: f64, b: f64 }

impl Tool for Add {
    const NAME: &'static str = "add";
    type Args = AddArgs;
    type Output = String;
    type Error = std::convert::Infallible;
    fn description(&self) -> String { "Add two numbers.".into() }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {"a": {"type": "number"}, "b": {"type": "number"}}})
    }
    async fn call(&self, _: &mut ToolContext, args: AddArgs) -> Result<String, Self::Error> {
        Ok((args.a + args.b).to_string())
    }
}

struct App { session: Session, input: String, agent: Arc<Agent>,
             tx: bridge::EventSender, rx: bridge::EventReceiver, rt: tokio::runtime::Runtime }

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        self.rx.drain_into(&mut self.session);
        egui::Panel::bottom("prompt").show(ui, |ui| {
            let out = PromptInput::new(&mut self.input).generating(self.session.is_running()).show(ui, |_| {});
            if let Some(prompt) = out.submitted {
                self.session.push_user(prompt.clone());
                let (agent, tx) = (self.agent.clone(), self.tx.clone());
                self.rt.spawn(async move {
                    let mut adapter = RigAdapter::new();
                    let mut items = agent.prompt(prompt).stream();
                    while let Some(item) = items.next().await {
                        let events = match item {
                            Ok(item) => adapter.push_item(&item),
                            Err(e) => adapter.push_error(&e),
                        };
                        tx.send_all(events).ok();
                    }
                });
            }
        });
        egui::CentralPanel::default().show(ui, |ui| {
            let _ = AgentThread::new(&mut self.session).max_width(760.0).show(ui);
        });
    }
}

fn main() -> eframe::Result {
    eframe::run_native("rig agent", Default::default(), Box::new(|cc| {
        Theme::dark().install(&cc.egui_ctx);
        let model = std::env::var("MODEL").expect("set MODEL");
        let agent = AgentBuilder::new(Ollama::new().completion(model)).tool(Add).build();
        let (tx, rx) = bridge::channel();
        Ok(Box::new(App { session: Session::new(), input: String::new(), agent: Arc::new(agent),
            tx: tx.with_repaint(&cc.egui_ctx), rx, rt: tokio::runtime::Runtime::new()? }))
    }))
}
```

`RigAdapter` also takes plain completion streams: `adapter.push_event(&stream_event)` for
`rig_core::streaming::StreamEvent`.

## Run it

```bash
MODEL=<a model you pulled with ollama> cargo run -p glowkit-example-agent-rig
```

Ask "what is 21.5 + 20.5?" to see the tool card. rig 0.45 and later need rustc 1.97.1 or newer.
