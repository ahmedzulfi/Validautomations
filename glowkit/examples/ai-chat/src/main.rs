//! A minimal AI chat app: conversation, streaming answers and a prompt input.
//!
//! The "model" here is fake: it streams a canned Markdown answer word by word. Swap
//! `FakeModel` for a call to your LLM provider (on a background thread, sending tokens back
//! through a channel) and the UI stays the same.
//!
//! Run it with `cargo run -p glowkit-example-ai-chat`.

use glowkit::ai::{ChatMessage, Conversation, PromptInput, Role};
use glowkit::prelude::*;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([820.0, 720.0]).with_title("glowkit chat"),
        ..Default::default()
    };
    eframe::run_native(
        "glowkit chat",
        options,
        Box::new(|cc| {
            Theme::dark().install(&cc.egui_ctx);
            Ok(Box::<App>::default())
        }),
    )
}

struct Message {
    role: Role,
    text: String,
}

/// Streams a canned answer a few words per frame.
struct FakeModel {
    words: Vec<String>,
    next: usize,
    last_emit: f64,
}

impl FakeModel {
    fn answer(prompt: &str) -> Self {
        let text = format!(
            "You asked: **{prompt}**\n\nHere is what a streamed answer looks like:\n\n\
             1. Markdown renders *while* it streams\n2. Code blocks get a copy button\n\n\
             ```rust\nfn main() {{\n    println!(\"hello from glowkit\");\n}}\n```\n\n\
             Swap this fake model for a real one and you're done."
        );
        let words = text.split_inclusive(' ').map(str::to_owned).collect();
        Self { words, next: 0, last_emit: 0.0 }
    }

    /// Returns the next chunk if it's time to emit one.
    fn poll(&mut self, now: f64) -> Option<String> {
        if self.next >= self.words.len() || now - self.last_emit < 0.03 {
            return None;
        }
        self.last_emit = now;
        let end = (self.next + 2).min(self.words.len());
        let chunk = self.words[self.next..end].concat();
        self.next = end;
        Some(chunk)
    }

    fn done(&self) -> bool {
        self.next >= self.words.len()
    }
}

#[derive(Default)]
struct App {
    messages: Vec<Message>,
    input: String,
    model: Option<FakeModel>,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let theme = Theme::of(ui);
        let now = ui.input(|i| i.time);

        // Feed streamed tokens into the last assistant message.
        if let Some(model) = &mut self.model {
            if let Some(chunk) = model.poll(now) {
                if let Some(last) = self.messages.last_mut() {
                    last.text.push_str(&chunk);
                }
            }
            if model.done() {
                self.model = None;
            } else {
                ui.ctx().request_repaint();
            }
        }
        let generating = self.model.is_some();

        egui::Panel::bottom("prompt").frame(egui::Frame::new().fill(theme.background).inner_margin(16)).show(
            ui,
            |ui| {
                let out = PromptInput::new(&mut self.input).generating(generating).show(ui, |_| {});
                if let Some(prompt) = out.submitted {
                    self.messages.push(Message { role: Role::User, text: prompt.clone() });
                    self.messages.push(Message { role: Role::Assistant, text: String::new() });
                    self.model = Some(FakeModel::answer(&prompt));
                }
                if out.stop {
                    self.model = None;
                }
            },
        );

        egui::CentralPanel::default().frame(egui::Frame::new().fill(theme.background).inner_margin(16)).show(
            ui,
            |ui| {
                let count = self.messages.len();
                Conversation::new().show(ui, |ui| {
                    if self.messages.is_empty() {
                        ui.label(egui::RichText::new("Ask anything to get started.").color(theme.muted_foreground));
                    }
                    for (i, m) in self.messages.iter().enumerate() {
                        let streaming = generating && i + 1 == count;
                        ChatMessage::new(m.role, &m.text).streaming(streaming).show(ui, |_| {});
                        ui.add_space(12.0);
                    }
                });
            },
        );
    }
}
