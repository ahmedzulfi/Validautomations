//! A [rig](https://docs.rs/rig-agent) agent with a tool, streamed into a native UI: reasoning,
//! the tool call with its arguments and result, then the answer.
//!
//! ```text
//! MODEL=<a model you pulled with `ollama pull`> cargo run -p glowkit-example-agent-rig
//! ```
//! `OLLAMA_API_BASE_URL` points it at another Ollama daemon.

use futures_util::StreamExt;
use glowkit::ai::PromptInput;
use glowkit::prelude::*;
use glowkit_stream::adapters::rig::RigAdapter;
use glowkit_stream::bridge::{self, EventReceiver, EventSender};
use glowkit_stream::{AgentEvent, AgentThread, Session};
use rig_agent::tool::{Tool, ToolContext};
use rig_agent::{Agent, AgentBuilder};
use rig_core::providers::ollama::Ollama;
use std::sync::Arc;

/// A tool the model can call: adds two numbers.
struct Add;

#[derive(serde::Deserialize)]
struct AddArgs {
    a: f64,
    b: f64,
}

impl Tool for Add {
    const NAME: &'static str = "add";
    type Args = AddArgs;
    type Output = String;
    type Error = std::convert::Infallible;

    fn description(&self) -> String {
        "Add two numbers.".into()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {"a": {"type": "number"}, "b": {"type": "number"}}, "required": ["a", "b"]})
    }

    async fn call(&self, _cx: &mut ToolContext, args: AddArgs) -> Result<String, Self::Error> {
        Ok((args.a + args.b).to_string())
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "rig agent",
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
    agent: Option<Arc<Agent>>,
    tx: EventSender,
    rx: EventReceiver,
    rt: tokio::runtime::Runtime,
}

impl App {
    fn new(ctx: &egui::Context) -> Self {
        let (tx, rx) = bridge::channel();
        let agent = std::env::var("MODEL").ok().map(|model| {
            let provider = Ollama::from_env().unwrap_or_else(|_| Ollama::new());
            let agent = AgentBuilder::new(provider.completion(model))
                .preamble("You are a concise assistant. Use the add tool for arithmetic.")
                .tool(Add)
                .build();
            Arc::new(agent)
        });
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        Self { session: Session::new(), input: String::new(), agent, tx: tx.with_repaint(ctx), rx, rt }
    }

    fn ask(&mut self, prompt: String) {
        self.session.push_user(prompt.clone());
        let Some(agent) = self.agent.clone() else {
            self.session.apply(AgentEvent::error("Set MODEL to a model you have pulled (see `ollama list`)."));
            return;
        };
        let tx = self.tx.clone();
        self.rt.spawn(async move {
            let mut adapter = RigAdapter::new();
            let mut items = agent.prompt(prompt).stream();
            while let Some(item) = items.next().await {
                let events = match item {
                    Ok(item) => adapter.push_item(&item),
                    Err(e) => adapter.push_error(&e),
                };
                if tx.send_all(events).is_err() {
                    return; // the window closed
                }
            }
        });
    }
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
