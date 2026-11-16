# An ACP coding-agent viewer

**What you get:** a native window on top of a coding agent that speaks the
[Agent Client Protocol](https://agentclientprotocol.com) over stdio. The agent's plan becomes a
live checklist, file reads, edits (shown as diffs) and commands become tool cards with their
status, thoughts stream into a reasoning panel, `session/request_permission` becomes an approval
card with the agent's own options ("Allow once", "Always allow", "Reject"), and the context-window
usage drives a meter. No async runtime needed: two threads and the bridge.

## Cargo.toml

```toml
[dependencies]
glowkit = { version = "0.1", features = ["ai"] }
glowkit-stream = { version = "0.1", features = ["egui", "acp-wire"] }   # or "acp" for the typed SDK
eframe = "0.37"
serde_json = "1"
```

## Code

```rust
use glowkit::{ai::PromptInput, prelude::*};
use glowkit_stream::adapters::acp::{permission_response, AcpAdapter};
use glowkit_stream::{bridge, AgentThread, Session};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};

type Pipe = Arc<Mutex<ChildStdin>>;
fn send(pipe: &Pipe, msg: Value) {
    let mut p = pipe.lock().unwrap();
    writeln!(p, "{msg}").ok();
    p.flush().ok();
}

struct App { session: Session, input: String, rx: bridge::EventReceiver, pipe: Pipe, sid: Arc<Mutex<String>>, next_id: u64 }

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        self.rx.drain_into(&mut self.session);
        egui::Panel::bottom("prompt").show(ui, |ui| {
            let out = PromptInput::new(&mut self.input).generating(self.session.is_running()).show(ui, |_| {});
            if let Some(text) = out.submitted {
                self.session.push_user(text.clone());
                self.next_id += 1;
                let sid = self.sid.lock().unwrap().clone();
                send(&self.pipe, json!({"jsonrpc": "2.0", "id": self.next_id, "method": "session/prompt",
                    "params": {"sessionId": sid, "prompt": [{"type": "text", "text": text}]}}));
            }
        });
        egui::CentralPanel::default().show(ui, |ui| {
            for d in AgentThread::new(&mut self.session).max_width(760.0).show(ui).decisions {
                send(&self.pipe, permission_response(&d.approval_id, d.option_id.as_deref()));
            }
        });
    }
}

fn main() -> eframe::Result {
    let mut args = std::env::args().skip(1);
    let program = args.next().expect("usage: app <agent command> [args...]");
    let mut child = Command::new(program).args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    let pipe: Pipe = Arc::new(Mutex::new(child.stdin.take().unwrap()));
    let stdout = child.stdout.take().unwrap();
    send(&pipe, json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {"protocolVersion": 1, "clientCapabilities": {}}}));
    let cwd = std::env::current_dir().unwrap();
    send(&pipe, json!({"jsonrpc": "2.0", "id": 1, "method": "session/new", "params": {"cwd": cwd, "mcpServers": []}}));

    eframe::run_native("ACP agent", Default::default(), Box::new(move |cc| {
        Theme::dark().install(&cc.egui_ctx);
        let (tx, rx) = bridge::channel();
        let tx = tx.with_repaint(&cc.egui_ctx);
        let sid = Arc::new(Mutex::new(String::new()));
        let sid2 = sid.clone();
        std::thread::spawn(move || {
            let mut adapter = AcpAdapter::new();
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
                if let Some(id) = msg.pointer("/result/sessionId").and_then(Value::as_str) {
                    *sid2.lock().unwrap() = id.to_owned();
                }
                if tx.send_all(adapter.push_message(&msg)).is_err() { break; }
            }
        });
        Ok(Box::new(App { session: Session::new(), input: String::new(), rx, pipe, sid, next_id: 1 }))
    }))
}
```

With the `acp` feature you can use the official `agent-client-protocol` SDK for the connection
instead, and feed its typed values to `AcpAdapter::session_notification`,
`AcpAdapter::request_permission` and `AcpAdapter::stop_reason`.

## Run it

```bash
cargo run --release -- <your ACP agent command and its arguments>
```

To see the view without an agent, open the gallery's **Live agent stream** demo and pick
"ACP-style": it replays a recorded ACP session (plan, file read, an edit behind a permission
request, a failing test run) through the same adapter:

```bash
cargo run --release -p glowkit-gallery
```

This viewer advertises no client capabilities, so agents will not ask it to read or write files
or run terminals; add handlers for `fs/*` and `terminal/*` requests to support those.
