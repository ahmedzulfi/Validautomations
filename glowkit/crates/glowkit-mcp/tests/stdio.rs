//! End-to-end test: spawn the real `glowkit-mcp` binary and speak MCP (JSON-RPC over stdio) to it.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

struct Client {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl Client {
    fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_glowkit-mcp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn glowkit-mcp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self { child, stdin, stdout, next_id: 1 }
    }

    fn send(&mut self, msg: &Value) {
        writeln!(self.stdin, "{msg}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(&json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }

    /// Send a request and wait for the response with the same id (skipping notifications).
    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let mut line = String::new();
            let n = self.stdout.read_line(&mut line).unwrap();
            assert!(n > 0, "server closed stdout while waiting for {method}");
            let v: Value = serde_json::from_str(&line).unwrap_or_else(|e| panic!("bad JSON from server ({e}): {line}"));
            if v.get("id") == Some(&json!(id)) {
                return v;
            }
        }
    }

    fn call_tool(&mut self, name: &str, args: Value) -> Value {
        self.request("tools/call", json!({"name": name, "arguments": args}))
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn text_of(resp: &Value) -> String {
    let result = resp.get("result").unwrap_or_else(|| panic!("no result: {resp}"));
    result["content"]
        .as_array()
        .expect("content array")
        .iter()
        .filter_map(|c| c["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn handshake() -> Client {
    let mut c = Client::spawn();
    let init = c.request(
        "initialize",
        json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "stdio-test", "version": "0.0.0"}
        }),
    );
    let result = &init["result"];
    assert_eq!(result["serverInfo"]["name"], "glowkit-mcp", "{init}");
    assert!(result["capabilities"]["tools"].is_object(), "{init}");
    assert!(result["capabilities"]["resources"].is_object(), "{init}");
    assert!(result["capabilities"]["prompts"].is_object(), "{init}");
    assert!(result["instructions"].as_str().unwrap().contains("search_components"));
    c.notify("notifications/initialized", json!({}));
    c
}

#[test]
fn initialize_and_list_tools() {
    let mut c = handshake();
    let tools = c.request("tools/list", json!({}));
    let names: Vec<&str> =
        tools["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    for expected in [
        "search_components",
        "get_component",
        "list_categories",
        "get_template",
        "get_theme",
        "get_setup",
        "egui_037_notes",
    ] {
        assert!(names.contains(&expected), "missing tool {expected}: {names:?}");
    }
    let search =
        tools["result"]["tools"].as_array().unwrap().iter().find(|t| t["name"] == "search_components").unwrap();
    assert_eq!(search["inputSchema"]["required"], json!(["query"]), "{search}");
}

#[test]
fn search_and_get_component() {
    let mut c = handshake();
    let found = text_of(&c.call_tool("search_components", json!({"query": "shiny text", "limit": 3})));
    assert!(found.contains("(`shiny-text`)"), "{found}");

    let filtered = text_of(&c.call_tool("search_components", json!({"query": "line", "category": "Charts"})));
    assert!(filtered.contains("Charts"), "{filtered}");
    assert!(!filtered.contains("Text Animations"), "{filtered}");

    let comp = text_of(&c.call_tool("get_component", json!({"id": "shiny-text"})));
    for needle in ["use glowkit::text::ShinyText;", "ShinyText::new(", "fn speed(", "feature `text`", "Web equivalents"]
    {
        assert!(comp.contains(needle), "get_component missing {needle}:\n{comp}");
    }

    let err = c.call_tool("get_component", json!({"id": "shiny-txt"}));
    let msg = err.to_string();
    assert!(msg.contains("shiny-text"), "should suggest the right id: {msg}");
}

#[test]
fn templates_theme_setup_notes() {
    let mut c = handshake();
    let tpl = text_of(&c.call_tool("get_template", json!({"id": "app-crm"})));
    assert!(tpl.contains("impl eframe::App for MyApp"), "{tpl}");
    assert!(tpl.contains("self.app.show(ui);"), "{tpl}");

    let cats = text_of(&c.call_tool("list_categories", json!({})));
    assert!(cats.contains("Charts") && cats.contains("Templates"), "{cats}");

    let theme = text_of(&c.call_tool("get_theme", json!({})));
    assert!(theme.contains("Theme::ocean()") && theme.contains("accent"), "{theme}");

    let setup = text_of(&c.call_tool("get_setup", json!({})));
    assert!(setup.contains("eframe = \"0.37\"") && setup.contains("fn ui(&mut self"), "{setup}");

    let notes = text_of(&c.call_tool("egui_037_notes", json!({})));
    for needle in ["egui::Panel::left", "Id::unique", "StrokeKind", "CornerRadius", "with_state"] {
        assert!(notes.contains(needle), "notes missing {needle}");
    }
}

#[test]
fn resources_and_prompts() {
    let mut c = handshake();
    let list = c.request("resources/list", json!({}));
    let uris: Vec<&str> =
        list["result"]["resources"].as_array().unwrap().iter().map(|r| r["uri"].as_str().unwrap()).collect();
    assert!(uris.contains(&"glowkit://docs/customizing"), "{list}");

    let read = c.request("resources/read", json!({"uri": "glowkit://docs/customizing"}));
    let text = read["result"]["contents"][0]["text"].as_str().unwrap_or_default();
    assert!(text.contains("# Customizing glowkit"), "{read}");

    let prompts = c.request("prompts/list", json!({}));
    assert!(prompts.to_string().contains("build-app"), "{prompts}");
    let p = c.request("prompts/get", json!({"name": "build-app", "arguments": {"what": "a crypto portfolio tracker"}}));
    let msg = p["result"]["messages"][0]["content"]["text"].as_str().unwrap_or_default();
    assert!(msg.contains("crypto portfolio tracker") && msg.contains("search_components"), "{p}");
}
