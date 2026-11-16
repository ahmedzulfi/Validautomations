//! Local LLM chat: Ollama (or vLLM, LM Studio, OpenRouter... any OpenAI-compatible server)
//! streamed into a native UI, reasoning included.
//!
//! ```text
//! cargo run -p glowkit-example-agent-ollama
//! OPENAI_BASE_URL=http://localhost:1234/v1 MODEL=my-model cargo run -p glowkit-example-agent-ollama
//! ```
//! Without `MODEL`, the first model the server lists is used.

use async_openai::{config::OpenAIConfig, Client};
use futures_util::StreamExt;
use glowkit::ai::PromptInput;
use glowkit::prelude::*;
use glowkit_stream::adapters::openai::OpenAiChatAdapter;
use glowkit_stream::bridge::{self, EventReceiver, EventSender};
use glowkit_stream::{AgentEvent, AgentThread, Role, Session};
use serde_json::{json, Value};

fn main() -> eframe::Result {
    eframe::run_native(
        "Local chat",
        Default::default(),
        Box::new(|cc| {
            Theme::dark().install(&cc.egui_ctx);
            Ok(Box::new(App::new(&cc.egui_ctx)))
        }),
    )
}

struct App {
    session: Session,
    input: String,
    client: Client<OpenAIConfig>,
    tx: EventSender,
    rx: EventReceiver,
    rt: tokio::runtime::Runtime,
}

impl App {
    fn new(ctx: &egui::Context) -> Self {
        let base = std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "http://localhost:11434/v1".into());
        let client = Client::with_config(OpenAIConfig::new().with_api_base(base).with_api_key("local"));
        let (tx, rx) = bridge::channel();
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        Self { session: Session::new(), input: String::new(), client, tx: tx.with_repaint(ctx), rx, rt }
    }

    fn ask(&mut self, prompt: String) {
        self.session.push_user(prompt);
        let messages: Vec<Value> = self
            .session
            .messages
            .iter()
            .map(|m| json!({"role": if m.role == Role::User { "user" } else { "assistant" }, "content": m.text()}))
            .collect();
        let (client, tx) = (self.client.clone(), self.tx.clone());
        self.rt.spawn(async move {
            if let Err(e) = stream(&client, messages, &tx).await {
                tx.send_all([
                    AgentEvent::error(e.to_string()),
                    AgentEvent::RunFinish { stop: glowkit_stream::StopReason::Error },
                ])
                .ok();
            }
        });
    }
}

/// Stream one answer. Raw JSON chunks keep `reasoning_content` / `reasoning`, which the typed
/// chunk drops.
async fn stream(
    client: &Client<OpenAIConfig>,
    messages: Vec<Value>,
    tx: &EventSender,
) -> Result<(), Box<dyn std::error::Error>> {
    let model = match std::env::var("MODEL") {
        Ok(m) => m,
        Err(_) => {
            client.models().list().await?.data.first().map(|m| m.id.clone()).ok_or("the server lists no models")?
        }
    };
    let request =
        json!({"model": model, "messages": messages, "stream": true, "stream_options": {"include_usage": true}});
    let mut chunks = client.chat().create_stream_byot::<_, Value>(request).await?;
    let mut adapter = OpenAiChatAdapter::new();
    while let Some(chunk) = chunks.next().await {
        tx.send_all(adapter.push_raw(&chunk?))?;
    }
    tx.send_all(adapter.finish())?;
    Ok(())
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let theme = Theme::of(ui);
        self.rx.drain_into(&mut self.session);

        egui::Panel::bottom("prompt").frame(egui::Frame::new().fill(theme.background).inner_margin(16)).show(
            ui,
            |ui| {
                let out = PromptInput::new(&mut self.input).generating(self.session.is_running()).show(ui, |_| {});
                if let Some(prompt) = out.submitted {
                    self.ask(prompt);
                }
            },
        );

        egui::CentralPanel::default().frame(egui::Frame::new().fill(theme.background).inner_margin(16)).show(
            ui,
            |ui| {
                let _ = AgentThread::new(&mut self.session).max_width(760.0).show(ui);
            },
        );
    }
}
