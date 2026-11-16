//! The [Agent Client Protocol](https://agentclientprotocol.com) (ACP): coding agents talking
//! JSON-RPC to an editor or client.
//!
//! * [`AcpAdapter::push_message`] reads raw JSON-RPC messages (feature `acp-wire`, serde only):
//!   `session/update` notifications, `session/request_permission` requests and the
//!   `session/prompt` response carrying the stop reason.
//! * With feature `acp`, typed methods take the `agent-client-protocol` 5.x schema types
//!   ([`AcpAdapter::session_notification`], [`AcpAdapter::request_permission`],
//!   [`AcpAdapter::stop_reason`]).
//!
//! Text chunks are split into a new part whenever a tool call or plan arrives in between, so
//! the thread reads in the order the agent worked.

use crate::event::{AgentEvent, ApprovalOption, PlanEntry, PlanStatus, StopReason, Usage};
use serde_json::{json, Value};

/// Converts ACP traffic into [`AgentEvent`]s.
#[derive(Clone, Debug, Default)]
pub struct AcpAdapter {
    segment: u32,
    open_text: Option<String>,
    open_thought: Option<String>,
    running: bool,
}

impl AcpAdapter {
    /// A fresh adapter (one per ACP session).
    pub fn new() -> Self {
        Self::default()
    }

    /// Convert one JSON-RPC message (either direction). Messages it does not understand yield
    /// nothing.
    ///
    /// ```
    /// use glowkit_stream::adapters::acp::AcpAdapter;
    /// use serde_json::json;
    /// let mut a = AcpAdapter::new();
    /// let ev = a.push_message(&json!({
    ///     "jsonrpc": "2.0", "method": "session/update",
    ///     "params": {"sessionId": "s", "update": {
    ///         "sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": "Hi"}}}
    /// }));
    /// assert_eq!(ev.len(), 2); // RunStart, then the text
    /// ```
    pub fn push_message(&mut self, msg: &Value) -> Vec<AgentEvent> {
        let mut out = Vec::new();
        let method = msg.get("method").and_then(Value::as_str);
        let params = msg.get("params").unwrap_or(&Value::Null);
        match method {
            Some("session/prompt") => self.begin(&mut out, true),
            Some("session/update") => {
                if let Some(u) = params.get("update") {
                    self.wire_update(u, &mut out);
                }
            }
            Some("session/request_permission") => {
                let tc = params.get("toolCall").unwrap_or(&Value::Null);
                let rpc_id = msg.get("id").map(id_string).unwrap_or_default();
                let tool_id = str_field(tc, "toolCallId").unwrap_or_default();
                // the tool call may only be described here
                self.wire_tool(tc, true, &mut out);
                let options = params
                    .get("options")
                    .and_then(Value::as_array)
                    .map(|opts| {
                        opts.iter()
                            .map(|o| option(str_field(o, "optionId"), str_field(o, "name"), str_field(o, "kind")))
                            .collect()
                    })
                    .unwrap_or_default();
                self.permission(rpc_id, tool_id, str_field(tc, "title"), options, &mut out);
            }
            Some(_) => {}
            None => {
                // a response: the prompt result carries the stop reason
                if let Some(reason) = msg.pointer("/result/stopReason").and_then(Value::as_str) {
                    self.stop(reason, &mut out);
                } else if let Some(err) = msg.get("error") {
                    let message = str_field(err, "message").unwrap_or_else(|| "ACP error".into());
                    let code = err.get("code").map(|c| c.to_string());
                    self.close(&mut out);
                    out.push(AgentEvent::Error { message, code });
                    out.push(AgentEvent::RunFinish { stop: StopReason::Error });
                    self.running = false;
                }
            }
        }
        out
    }

    /// Convert one `session/update` payload (the `update` object).
    pub fn push_update(&mut self, update: &Value) -> Vec<AgentEvent> {
        let mut out = Vec::new();
        self.wire_update(update, &mut out);
        out
    }

    fn wire_update(&mut self, u: &Value, out: &mut Vec<AgentEvent>) {
        let kind = u.get("sessionUpdate").and_then(Value::as_str).unwrap_or_default();
        match kind {
            "agent_message_chunk" | "agent_thought_chunk" => {
                let text = content_text(u.get("content").unwrap_or(&Value::Null));
                let mid = str_field(u, "messageId");
                self.chunk(mid.as_deref(), &text, kind == "agent_thought_chunk", out);
            }
            "tool_call" => self.wire_tool(u, true, out),
            "tool_call_update" => self.wire_tool(u, false, out),
            "plan" => {
                let entries = u
                    .get("entries")
                    .and_then(Value::as_array)
                    .map(|es| {
                        es.iter()
                            .map(|e| {
                                PlanEntry::new(
                                    str_field(e, "content").unwrap_or_default(),
                                    plan_status(str_field(e, "status").as_deref()),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                self.plan(entries, out);
            }
            "usage_update" => {
                let n = |k: &str| u.get(k).and_then(Value::as_u64);
                self.usage(n("used"), n("size"), out);
            }
            "user_message_chunk" => {}
            other => out.push(AgentEvent::Custom { name: format!("acp/{other}"), value: u.clone() }),
        }
    }

    fn wire_tool(&mut self, u: &Value, create: bool, out: &mut Vec<AgentEvent>) {
        let Some(id) = str_field(u, "toolCallId") else { return };
        let title = str_field(u, "title").or_else(|| str_field(u, "name"));
        let output = tool_output(u.get("rawOutput"), u.get("content"));
        let fields = ToolFields {
            title: title.as_deref(),
            status: str_field(u, "status").as_deref().map(str::to_owned),
            raw_input: u.get("rawInput").filter(|v| !v.is_null()).cloned(),
            output,
        };
        self.tool(&id, create, fields, out);
    }

    // ----- shared core, used by the wire and the typed paths ----------------------------------

    fn begin(&mut self, out: &mut Vec<AgentEvent>, force: bool) {
        if force || !self.running {
            if force && self.running {
                self.close(out);
            }
            self.running = true;
            self.segment += 1;
            out.push(AgentEvent::RunStart { run_id: None });
        }
    }

    fn close(&mut self, out: &mut Vec<AgentEvent>) {
        if let Some(id) = self.open_text.take() {
            out.push(AgentEvent::TextEnd { id });
        }
        if let Some(id) = self.open_thought.take() {
            out.push(AgentEvent::ReasoningEnd { id });
        }
    }

    /// Start a new text segment (after a tool call or plan).
    fn split(&mut self, out: &mut Vec<AgentEvent>) {
        self.close(out);
        self.segment += 1;
    }

    fn chunk(&mut self, message_id: Option<&str>, text: &str, thought: bool, out: &mut Vec<AgentEvent>) {
        self.begin(out, false);
        if text.is_empty() {
            return;
        }
        let base = message_id.unwrap_or("acp");
        let id = format!("{base}-{}-{}", if thought { "thought" } else { "text" }, self.segment);
        if thought {
            if let Some(t) = self.open_text.take() {
                out.push(AgentEvent::TextEnd { id: t });
                self.segment += 1;
                return self.chunk(message_id, text, thought, out);
            }
            self.open_thought = Some(id.clone());
            out.push(AgentEvent::reasoning(id, text));
        } else {
            if let Some(r) = self.open_thought.take() {
                out.push(AgentEvent::ReasoningEnd { id: r });
            }
            self.open_text = Some(id.clone());
            out.push(AgentEvent::text(id, text));
        }
    }

    fn tool(&mut self, id: &str, create: bool, f: ToolFields<'_>, out: &mut Vec<AgentEvent>) {
        self.begin(out, false);
        if create {
            self.split(out);
        }
        if let Some(t) = f.title {
            out.push(AgentEvent::ToolCallStart { id: id.to_owned(), name: t.to_owned() });
        } else if create {
            out.push(AgentEvent::ToolCallStart { id: id.to_owned(), name: String::new() });
        }
        if let Some(input) = f.raw_input {
            out.push(AgentEvent::ToolInputReady { id: id.to_owned(), name: None, input });
        }
        match f.status.as_deref() {
            Some("in_progress") => out.push(AgentEvent::ToolRunning { id: id.to_owned() }),
            Some("completed") => out.push(AgentEvent::ToolResult {
                id: id.to_owned(),
                output: f.output.unwrap_or(Value::Null),
                is_error: false,
            }),
            Some("failed") => out.push(AgentEvent::ToolResult {
                id: id.to_owned(),
                output: f.output.unwrap_or_else(|| Value::String("Tool failed".into())),
                is_error: true,
            }),
            // not in the 1.x schema, but some agents send it
            Some("cancelled") | Some("canceled") => out.push(AgentEvent::decision(id, false)),
            _ => {}
        }
    }

    fn plan(&mut self, entries: Vec<PlanEntry>, out: &mut Vec<AgentEvent>) {
        self.begin(out, false);
        out.push(AgentEvent::Plan { entries });
    }

    fn usage(&mut self, used: Option<u64>, size: Option<u64>, out: &mut Vec<AgentEvent>) {
        out.push(AgentEvent::Usage(Usage { context_used: used, context_size: size, ..Default::default() }));
    }

    fn permission(
        &mut self,
        rpc_id: String,
        tool_call_id: String,
        title: Option<String>,
        options: Vec<ApprovalOption>,
        out: &mut Vec<AgentEvent>,
    ) {
        self.begin(out, false);
        out.push(AgentEvent::ApprovalRequest {
            id: rpc_id,
            tool_call_id: Some(tool_call_id).filter(|t| !t.is_empty()),
            title: title.map(|t| format!("Allow: {t}?")),
            description: None,
            options,
        });
    }

    fn stop(&mut self, reason: &str, out: &mut Vec<AgentEvent>) {
        self.close(out);
        self.running = false;
        out.push(AgentEvent::RunFinish { stop: StopReason::from_wire(reason) });
    }
}

struct ToolFields<'a> {
    title: Option<&'a str>,
    status: Option<String>,
    raw_input: Option<Value>,
    output: Option<Value>,
}

fn option(id: Option<String>, name: Option<String>, kind: Option<String>) -> ApprovalOption {
    let id = id.unwrap_or_default();
    ApprovalOption {
        label: name.unwrap_or_else(|| id.clone()),
        approves: kind.as_deref().is_some_and(|k| k.starts_with("allow")),
        id,
    }
}

fn plan_status(s: Option<&str>) -> PlanStatus {
    match s {
        Some("in_progress") => PlanStatus::InProgress,
        Some("completed") => PlanStatus::Completed,
        Some("failed") => PlanStatus::Failed,
        _ => PlanStatus::Pending,
    }
}

fn str_field(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::to_owned)
}

fn id_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// The text of a content block (`{"type":"text","text":..}`); other blocks get a short label.
fn content_text(block: &Value) -> String {
    match block.get("type").and_then(Value::as_str) {
        Some("text") => str_field(block, "text").unwrap_or_default(),
        Some("resource_link") => {
            format!("[{}]", str_field(block, "name").or_else(|| str_field(block, "uri")).unwrap_or_default())
        }
        Some("resource") => block.pointer("/resource/text").and_then(Value::as_str).unwrap_or_default().to_owned(),
        Some(other) => format!("[{other}]"),
        None => String::new(),
    }
}

/// A tool's output: `rawOutput` when present, else its content blocks rendered as text
/// (diffs as unified-style text).
fn tool_output(raw: Option<&Value>, content: Option<&Value>) -> Option<Value> {
    if let Some(r) = raw.filter(|r| !r.is_null()) {
        return Some(r.clone());
    }
    let items = content?.as_array()?;
    let mut text = Vec::new();
    for c in items {
        match c.get("type").and_then(Value::as_str) {
            Some("content") => text.push(content_text(c.get("content").unwrap_or(&Value::Null))),
            Some("diff") => {
                let path = str_field(c, "path").unwrap_or_default();
                let old = str_field(c, "oldText").unwrap_or_default();
                let new = str_field(c, "newText").unwrap_or_default();
                text.push(render_diff(&path, &old, &new));
            }
            Some("terminal") => text.push(format!("[terminal {}]", str_field(c, "terminalId").unwrap_or_default())),
            _ => {}
        }
    }
    (!text.is_empty()).then(|| Value::String(text.join("\n")))
}

fn render_diff(path: &str, old: &str, new: &str) -> String {
    let mut s = format!("--- {path}\n+++ {path}\n");
    for l in old.lines() {
        s.push_str(&format!("-{l}\n"));
    }
    for l in new.lines() {
        s.push_str(&format!("+{l}\n"));
    }
    s
}

/// The JSON-RPC response to a `session/request_permission` request: `approval_id` is the
/// request id (as [`AgentEvent::ApprovalRequest::id`] carries it) and `option_id` the chosen
/// option, or `None` to report the prompt as cancelled.
pub fn permission_response(approval_id: &str, option_id: Option<&str>) -> Value {
    let id: Value = approval_id.parse::<i64>().map(Value::from).unwrap_or_else(|_| Value::String(approval_id.into()));
    let outcome = match option_id {
        Some(o) => json!({"outcome": "selected", "optionId": o}),
        None => json!({"outcome": "cancelled"}),
    };
    json!({"jsonrpc": "2.0", "id": id, "result": {"outcome": outcome}})
}

#[cfg(feature = "acp")]
mod typed {
    use super::*;
    use agent_client_protocol::v1::schema::{
        ContentBlock, RequestPermissionRequest, SessionNotification, SessionUpdate, StopReason as AcpStop,
        ToolCallContent, ToolCallStatus,
    };

    fn wire<T: serde::Serialize>(t: &T) -> Option<String> {
        serde_json::to_value(t).ok().and_then(|v| v.as_str().map(str::to_owned))
    }

    fn block_text(b: &ContentBlock) -> String {
        match b {
            ContentBlock::Text(t) => t.text.clone(),
            other => serde_json::to_value(other).map(|v| content_text(&v)).unwrap_or_default(),
        }
    }

    fn contents_output(content: &[ToolCallContent]) -> Option<Value> {
        let mut text = Vec::new();
        for c in content {
            match c {
                ToolCallContent::Content(c) => text.push(block_text(&c.content)),
                ToolCallContent::Diff(d) => text.push(render_diff(
                    &d.path.display().to_string(),
                    d.old_text.as_deref().unwrap_or(""),
                    &d.new_text,
                )),
                ToolCallContent::Terminal(t) => text.push(format!("[terminal {}]", t.terminal_id)),
                _ => {}
            }
        }
        (!text.is_empty()).then(|| Value::String(text.join("\n")))
    }

    fn status(s: &ToolCallStatus) -> Option<String> {
        wire(s)
    }

    impl AcpAdapter {
        /// Convert a typed `session/update` notification.
        pub fn session_notification(&mut self, n: &SessionNotification) -> Vec<AgentEvent> {
            self.session_update(&n.update)
        }

        /// Convert a typed session update.
        pub fn session_update(&mut self, update: &SessionUpdate) -> Vec<AgentEvent> {
            let mut out = Vec::new();
            match update {
                SessionUpdate::AgentMessageChunk(c) => {
                    let mid = c.message_id.as_ref().and_then(wire);
                    self.chunk(mid.as_deref(), &block_text(&c.content), false, &mut out);
                }
                SessionUpdate::AgentThoughtChunk(c) => {
                    let mid = c.message_id.as_ref().and_then(wire);
                    self.chunk(mid.as_deref(), &block_text(&c.content), true, &mut out);
                }
                SessionUpdate::ToolCall(tc) => {
                    let output = tc.raw_output.clone().or_else(|| contents_output(&tc.content));
                    let fields = ToolFields {
                        title: Some(tc.title.as_str()),
                        status: status(&tc.status),
                        raw_input: tc.raw_input.clone(),
                        output,
                    };
                    self.tool(&tc.tool_call_id.0, true, fields, &mut out);
                }
                SessionUpdate::ToolCallUpdate(u) => {
                    let f = &u.fields;
                    let output = f.raw_output.clone().or_else(|| f.content.as_deref().and_then(contents_output));
                    let fields = ToolFields {
                        title: f.title.as_deref(),
                        status: f.status.as_ref().and_then(status),
                        raw_input: f.raw_input.clone(),
                        output,
                    };
                    self.tool(&u.tool_call_id.0, false, fields, &mut out);
                }
                SessionUpdate::Plan(p) => {
                    let entries = p
                        .entries
                        .iter()
                        .map(|e| PlanEntry::new(e.content.clone(), plan_status(wire(&e.status).as_deref())))
                        .collect();
                    self.plan(entries, &mut out);
                }
                SessionUpdate::UsageUpdate(u) => self.usage(Some(u.used), Some(u.size), &mut out),
                SessionUpdate::UserMessageChunk(_) => {}
                other => {
                    let value = serde_json::to_value(other).unwrap_or(Value::Null);
                    let kind = value.get("sessionUpdate").and_then(Value::as_str).unwrap_or("update").to_owned();
                    out.push(AgentEvent::Custom { name: format!("acp/{kind}"), value });
                }
            }
            out
        }

        /// Convert a typed `session/request_permission` request; `rpc_id` is the JSON-RPC request
        /// id, used as the approval id so you can answer with [`permission_response`].
        pub fn request_permission(&mut self, rpc_id: impl ToString, req: &RequestPermissionRequest) -> Vec<AgentEvent> {
            let mut out = Vec::new();
            let tc = &req.tool_call;
            let f = &tc.fields;
            let fields = ToolFields {
                title: f.title.as_deref(),
                status: f.status.as_ref().and_then(status),
                raw_input: f.raw_input.clone(),
                output: None,
            };
            self.tool(&tc.tool_call_id.0, true, fields, &mut out);
            let options = req
                .options
                .iter()
                .map(|o| option(Some(o.option_id.0.to_string()), Some(o.name.clone()), wire(&o.kind)))
                .collect();
            self.permission(rpc_id.to_string(), tc.tool_call_id.0.to_string(), f.title.clone(), options, &mut out);
            out
        }

        /// The prompt turn ended (`PromptResponse::stop_reason`).
        pub fn stop_reason(&mut self, reason: &AcpStop) -> Vec<AgentEvent> {
            let mut out = Vec::new();
            self.stop(&wire(reason).unwrap_or_else(|| "end_turn".into()), &mut out);
            out
        }

        /// A new prompt turn begins (call when you send `session/prompt`).
        pub fn prompt_started(&mut self) -> Vec<AgentEvent> {
            let mut out = Vec::new();
            self.begin(&mut out, true);
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{Part, Session, ToolState};

    fn update(u: Value) -> Value {
        json!({"jsonrpc": "2.0", "method": "session/update", "params": {"sessionId": "s1", "update": u}})
    }

    #[test]
    fn wire_messages_build_a_thread() {
        let mut a = AcpAdapter::new();
        let msgs = [
            json!({"jsonrpc":"2.0","id":1,"method":"session/prompt","params":{"sessionId":"s1","prompt":[]}}),
            update(json!({"sessionUpdate":"agent_thought_chunk","content":{"type":"text","text":"Look at the test"}})),
            update(json!({"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"Reading "}})),
            update(json!({"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"the file."}})),
            update(
                json!({"sessionUpdate":"tool_call","toolCallId":"t1","title":"Read lib.rs","kind":"read","status":"pending","rawInput":{"path":"lib.rs"}}),
            ),
            update(
                json!({"sessionUpdate":"tool_call_update","toolCallId":"t1","status":"completed","content":[{"type":"content","content":{"type":"text","text":"fn main() {}"}}]}),
            ),
            update(json!({"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"Found it."}})),
            update(
                json!({"sessionUpdate":"plan","entries":[{"content":"Fix","priority":"high","status":"in_progress"}]}),
            ),
            update(json!({"sessionUpdate":"usage_update","used":1200,"size":200000})),
            update(json!({"sessionUpdate":"available_commands_update","availableCommands":[]})),
            json!({"jsonrpc":"2.0","id":1,"result":{"stopReason":"end_turn"}}),
        ];
        let mut s = Session::new();
        for m in &msgs {
            s.extend(a.push_message(m));
        }
        let parts = &s.messages[0].parts;
        assert!(matches!(&parts[0], Part::Reasoning(r) if r.text == "Look at the test"));
        assert!(matches!(&parts[1], Part::Text(t) if t.text == "Reading the file."));
        assert!(matches!(&parts[2], Part::ToolCall(t) if t.name == "Read lib.rs" && t.state == ToolState::Done));
        assert!(matches!(&parts[3], Part::Text(t) if t.text == "Found it."));
        assert!(matches!(&parts[4], Part::Plan(e) if e[0].status == PlanStatus::InProgress));
        assert_eq!(s.tool_call("t1").unwrap().output, Some(Value::String("fn main() {}".into())));
        assert_eq!(s.usage.context_size, Some(200000));
        assert_eq!(s.custom.len(), 1);
        assert_eq!(s.status, crate::session::RunStatus::Finished(StopReason::EndTurn));
    }

    #[test]
    fn permission_request_and_response() {
        let mut a = AcpAdapter::new();
        let req = json!({"jsonrpc":"2.0","id":7,"method":"session/request_permission","params":{
            "sessionId":"s1",
            "toolCall":{"toolCallId":"t2","title":"Edit lib.rs","kind":"edit","status":"pending",
                "content":[{"type":"diff","path":"lib.rs","oldText":"a","newText":"b"}]},
            "options":[{"optionId":"allow","name":"Allow once","kind":"allow_once"},{"optionId":"no","name":"Reject","kind":"reject_once"}]}});
        let mut s = Session::new();
        s.extend(a.push_message(&req));
        let ap = s.pending_approvals().next().unwrap().clone();
        assert_eq!(ap.id, "7");
        assert_eq!(ap.tool_call_id.as_deref(), Some("t2"));
        assert_eq!(ap.options.iter().filter(|o| o.approves).count(), 1);
        let resp = permission_response(&ap.id, Some("allow"));
        assert_eq!(resp["id"], 7);
        assert_eq!(resp["result"]["outcome"]["optionId"], "allow");
        s.extend(
            a.push_message(&update(json!({"sessionUpdate":"tool_call_update","toolCallId":"t2","status":"failed"}))),
        );
        assert_eq!(s.tool_call("t2").unwrap().state, ToolState::Error);
        assert_eq!(s.approval("7").unwrap().decision, Some(true));
    }

    #[test]
    fn error_response_fails_the_run() {
        let mut a = AcpAdapter::new();
        let mut s = Session::new();
        s.extend(a.push_message(&update(
            json!({"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"x"}}),
        )));
        s.extend(a.push_message(&json!({"jsonrpc":"2.0","id":1,"error":{"code":-32603,"message":"agent crashed"}})));
        assert_eq!(s.last_error.as_deref(), Some("agent crashed"));
    }

    #[cfg(feature = "acp")]
    #[test]
    fn typed_schema_types_convert() {
        use agent_client_protocol::v1::schema::{RequestPermissionRequest, SessionNotification, StopReason as S};
        let mut a = AcpAdapter::new();
        let mut s = Session::new();
        s.extend(a.prompt_started());
        let notes = [
            json!({"sessionId":"s1","update":{"sessionUpdate":"agent_thought_chunk","content":{"type":"text","text":"plan"}}}),
            json!({"sessionId":"s1","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"On it."}}}),
            json!({"sessionId":"s1","update":{"sessionUpdate":"tool_call","toolCallId":"c1","title":"Run tests","kind":"execute","status":"in_progress","rawInput":{"cmd":"cargo test"}}}),
            json!({"sessionId":"s1","update":{"sessionUpdate":"tool_call_update","toolCallId":"c1","status":"completed","content":[{"type":"content","content":{"type":"text","text":"ok"}}]}}),
            json!({"sessionId":"s1","update":{"sessionUpdate":"plan","entries":[{"content":"a","priority":"low","status":"completed"}]}}),
            json!({"sessionId":"s1","update":{"sessionUpdate":"usage_update","used":10,"size":100}}),
        ];
        for n in notes {
            let n: SessionNotification = serde_json::from_value(n).expect("valid ACP notification");
            s.extend(a.session_notification(&n));
        }
        let req: RequestPermissionRequest = serde_json::from_value(json!({
            "sessionId":"s1","toolCall":{"toolCallId":"c2","title":"Delete build/"},
            "options":[{"optionId":"ok","name":"Allow","kind":"allow_once"},{"optionId":"no","name":"Reject","kind":"reject_always"}]
        }))
        .expect("valid permission request");
        s.extend(a.request_permission(42, &req));
        s.extend(a.stop_reason(&S::Cancelled));

        let c1 = s.tool_call("c1").unwrap();
        assert_eq!((c1.name.as_str(), c1.state), ("Run tests", ToolState::Done));
        assert_eq!(c1.input, Some(json!({"cmd": "cargo test"})));
        assert_eq!(s.messages[0].text(), "On it.");
        assert_eq!(s.approval("42").unwrap().options.len(), 2);
        assert_eq!(s.usage.context_used, Some(10));
        assert_eq!(s.status, crate::session::RunStatus::Finished(StopReason::Cancelled));
    }
}
