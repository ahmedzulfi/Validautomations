//! A native client for a Vercel AI SDK backend: any route that returns
//! `result.toUIMessageStreamResponse()` (the stream `useChat` reads) works unchanged.
//!
//! ```text
//! cargo run -p glowkit-example-agent-vercel -- http://localhost:3000/api/chat
//! cargo run -p glowkit-example-agent-vercel -- --replay      # offline: a bundled recording
//! ```

use glowkit::ai::PromptInput;
use glowkit::prelude::*;
use glowkit_stream::adapters::{vercel, SseProtocol};
use glowkit_stream::bridge::{self, EventReceiver, EventSender};
use glowkit_stream::{fixtures, http, replay::Replay, AgentThread, Session};

fn main() -> eframe::Result {
    let arg = std::env::args().nth(1).unwrap_or_else(|| "--replay".into());
    eframe::run_native(
        "AI SDK client",
        Default::default(),
        Box::new(move |cc| {
            Theme::dark().install(&cc.egui_ctx);
            Ok(Box::new(App::new(&cc.egui_ctx, arg)))
        }),
    )
}

struct App {
    session: Session,
    input: String,
    url: Option<String>,
    replay: Option<Replay>,
    tx: EventSender,
    rx: EventReceiver,
    rt: tokio::runtime::Runtime,
    client: reqwest::Client,
}

impl App {
    fn new(ctx: &egui::Context, arg: String) -> Self {
        let (tx, rx) = bridge::channel();
        let mut session = Session::new();
        let (url, replay) = if arg == "--replay" {
            session.push_user(fixtures::TRAVEL_PROMPT);
            (None, Some(Replay::new(fixtures::vercel_events())))
        } else {
            (Some(arg), None)
        };
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        Self {
            session,
            input: String::new(),
            url,
            replay,
            tx: tx.with_repaint(ctx),
            rx,
            rt,
            client: reqwest::Client::new(),
        }
    }

    /// POST the conversation as UI messages, like `useChat` does.
    fn send(&self) {
        let Some(url) = self.url.clone() else { return };
        let body = serde_json::json!({ "messages": vercel::ui_messages(&self.session) });
        let (client, tx) = (self.client.clone(), self.tx.clone());
        self.rt.spawn(async move { http::stream_sse(&client, &url, &body, SseProtocol::Vercel, &tx).await });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let theme = Theme::of(ui);
        self.rx.drain_into(&mut self.session);
        if let Some(r) = &mut self.replay {
            r.advance(ui.input(|i| i.time), &mut self.session);
            if !r.is_done() {
                ui.ctx().request_repaint();
            }
        }

        egui::Panel::bottom("prompt").frame(egui::Frame::new().fill(theme.background).inner_margin(16)).show(
            ui,
            |ui| {
                let out = PromptInput::new(&mut self.input).generating(self.session.is_running()).show(ui, |_| {});
                if let Some(prompt) = out.submitted {
                    self.session.push_user(prompt);
                    self.send();
                }
            },
        );

        egui::CentralPanel::default().frame(egui::Frame::new().fill(theme.background).inner_margin(16)).show(
            ui,
            |ui| {
                let out = AgentThread::new(&mut self.session).max_width(760.0).show(ui);
                if out.decisions.iter().any(|d| !d.approved) {
                    if let Some(r) = &mut self.replay {
                        r.stop(); // the recording only continues the approved branch
                    }
                }
                if !out.decisions.is_empty() {
                    // answered approvals ride along in the next request's tool parts
                    self.send();
                }
            },
        );
    }
}
