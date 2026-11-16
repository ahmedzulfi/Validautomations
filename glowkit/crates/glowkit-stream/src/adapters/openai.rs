//! [`async-openai`](https://docs.rs/async-openai) 0.43 streams: Chat Completions chunks (also
//! from OpenAI-compatible servers such as Ollama, vLLM, LM Studio or OpenRouter) and Responses
//! API events.
//!
//! OpenAI-compatible servers stream reasoning in fields the typed chunk does not have
//! (`reasoning_content`, `reasoning`). Use the `byot` ("bring your own type") stream with
//! `serde_json::Value` and [`OpenAiChatAdapter::push_raw`] to keep it.

use crate::event::{AgentEvent, StopReason, Usage};
use async_openai::types::chat::{CreateChatCompletionStreamResponse, FinishReason};
use async_openai::types::responses::{OutputItem, ResponseStreamEvent};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

/// A normalized view of one chat chunk, shared by the typed and the raw paths.
#[derive(Default)]
struct Chunk {
    id: String,
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Default)]
struct Choice {
    index: u32,
    content: Option<String>,
    reasoning: Option<String>,
    tools: Vec<ToolDelta>,
    finish: Option<String>,
}

#[derive(Default)]
struct ToolDelta {
    index: u32,
    id: Option<String>,
    name: Option<String>,
    args: Option<String>,
}

#[derive(Default, Clone, Debug)]
struct ToolSlot {
    id: String,
    args: String,
    done: bool,
}

/// Converts a Chat Completions stream into [`AgentEvent`]s: text, reasoning, tool calls whose
/// arguments stream by index, the finish reason and usage. One adapter per request.
///
/// ```
/// use glowkit_stream::adapters::openai::OpenAiChatAdapter;
/// let mut a = OpenAiChatAdapter::new();
/// let raw = serde_json::json!({"id": "c1", "choices": [{"index": 0,
///     "delta": {"reasoning_content": "Let me think"}}]});
/// let ev = a.push_raw(&raw);
/// assert!(matches!(&ev[1], glowkit_stream::AgentEvent::ReasoningDelta { .. }));
/// ```
#[derive(Clone, Debug, Default)]
pub struct OpenAiChatAdapter {
    started: bool,
    finished: bool,
    text_id: Option<String>,
    reasoning_id: Option<String>,
    segment: u32,
    tools: BTreeMap<u32, ToolSlot>,
}

impl OpenAiChatAdapter {
    /// A fresh adapter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Convert a typed chunk (`client.chat().create_stream(..)`).
    pub fn push_chunk(&mut self, c: &CreateChatCompletionStreamResponse) -> Vec<AgentEvent> {
        let chunk = Chunk {
            id: c.id.clone(),
            usage: c.usage.as_ref().map(|u| Usage {
                input_tokens: Some(u.prompt_tokens.into()),
                output_tokens: Some(u.completion_tokens.into()),
                total_tokens: Some(u.total_tokens.into()),
                ..Default::default()
            }),
            choices: c
                .choices
                .iter()
                .map(|ch| Choice {
                    index: ch.index,
                    content: ch.delta.content.clone(),
                    reasoning: None,
                    tools: ch
                        .delta
                        .tool_calls
                        .iter()
                        .flatten()
                        .map(|t| ToolDelta {
                            index: t.index,
                            id: t.id.clone(),
                            name: t.function.as_ref().and_then(|f| f.name.clone()),
                            args: t.function.as_ref().and_then(|f| f.arguments.clone()),
                        })
                        .collect(),
                    finish: ch.finish_reason.as_ref().map(|f| {
                        match f {
                            FinishReason::Stop => "stop",
                            FinishReason::Length => "length",
                            FinishReason::ToolCalls | FinishReason::FunctionCall => "tool_calls",
                            FinishReason::ContentFilter => "content_filter",
                        }
                        .to_owned()
                    }),
                })
                .collect(),
        };
        self.push(chunk)
    }

    /// Convert a raw JSON chunk (`create_stream_byot::<_, serde_json::Value>`). Lenient: reads
    /// `reasoning_content` / `reasoning` / `thinking` deltas and tolerates missing fields.
    pub fn push_raw(&mut self, v: &Value) -> Vec<AgentEvent> {
        let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).map(str::to_owned);
        let n = |v: &Value, k: &str| v.get(k).and_then(Value::as_u64);
        let chunk = Chunk {
            id: s(v, "id").unwrap_or_default(),
            usage: v.get("usage").filter(|u| u.is_object()).map(|u| Usage {
                input_tokens: n(u, "prompt_tokens"),
                output_tokens: n(u, "completion_tokens"),
                total_tokens: n(u, "total_tokens"),
                ..Default::default()
            }),
            choices: v
                .get("choices")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|ch| {
                    let d = ch.get("delta").or_else(|| ch.get("message")).unwrap_or(&Value::Null);
                    Choice {
                        index: n(ch, "index").unwrap_or(0) as u32,
                        content: s(d, "content"),
                        reasoning: s(d, "reasoning_content").or_else(|| s(d, "reasoning")).or_else(|| s(d, "thinking")),
                        tools: d
                            .get("tool_calls")
                            .and_then(Value::as_array)
                            .into_iter()
                            .flatten()
                            .enumerate()
                            .map(|(i, t)| {
                                let f = t.get("function").unwrap_or(&Value::Null);
                                ToolDelta {
                                    index: n(t, "index").map(|x| x as u32).unwrap_or(i as u32),
                                    id: s(t, "id"),
                                    name: s(f, "name"),
                                    // some servers send whole objects instead of string fragments
                                    args: f.get("arguments").map(|a| match a {
                                        Value::String(x) => x.clone(),
                                        other => other.to_string(),
                                    }),
                                }
                            })
                            .collect(),
                        finish: s(ch, "finish_reason"),
                    }
                })
                .collect(),
        };
        self.push(chunk)
    }

    /// Call when the stream ends: closes open parts and emits the run finish if no finish
    /// reason arrived.
    pub fn finish(&mut self) -> Vec<AgentEvent> {
        let mut out = Vec::new();
        if !self.finished && self.started {
            self.end_parts(&mut out);
            out.push(AgentEvent::RunFinish { stop: StopReason::EndTurn });
            self.finished = true;
        }
        out
    }

    fn push(&mut self, chunk: Chunk) -> Vec<AgentEvent> {
        let mut out = Vec::new();
        if !self.started {
            self.started = true;
            out.push(AgentEvent::RunStart { run_id: Some(chunk.id.clone()).filter(|i| !i.is_empty()) });
        }
        // multiple choices (n > 1) are rare in chat UIs: show the first
        for ch in chunk.choices.into_iter().filter(|c| c.index == 0) {
            if let Some(r) = ch.reasoning.filter(|r| !r.is_empty()) {
                let id =
                    self.reasoning_id.get_or_insert_with(|| format!("{}-reasoning-{}", chunk.id, self.segment)).clone();
                out.push(AgentEvent::reasoning(id, r));
            }
            if let Some(t) = ch.content.filter(|t| !t.is_empty()) {
                if let Some(r) = self.reasoning_id.take() {
                    out.push(AgentEvent::ReasoningEnd { id: r });
                    self.segment += 1;
                }
                let id = self.text_id.get_or_insert_with(|| format!("{}-text-{}", chunk.id, self.segment)).clone();
                out.push(AgentEvent::text(id, t));
            }
            for t in ch.tools {
                let slot = self.tools.entry(t.index).or_default();
                if slot.id.is_empty() {
                    slot.id = t.id.clone().unwrap_or_else(|| format!("{}-call-{}", chunk.id, t.index));
                }
                let id = slot.id.clone();
                if t.name.is_some() || t.id.is_some() {
                    out.push(AgentEvent::ToolCallStart { id: id.clone(), name: t.name.unwrap_or_default() });
                }
                if let Some(a) = t.args.filter(|a| !a.is_empty()) {
                    slot.args.push_str(&a);
                    out.push(AgentEvent::ToolArgsDelta { id, delta: a });
                }
            }
            if let Some(f) = ch.finish {
                self.end_parts(&mut out);
                out.push(AgentEvent::RunFinish { stop: StopReason::from_wire(&f) });
                self.finished = true;
            }
        }
        if let Some(u) = chunk.usage {
            out.push(AgentEvent::Usage(u));
        }
        out
    }

    fn end_parts(&mut self, out: &mut Vec<AgentEvent>) {
        if let Some(r) = self.reasoning_id.take() {
            out.push(AgentEvent::ReasoningEnd { id: r });
        }
        if let Some(t) = self.text_id.take() {
            out.push(AgentEvent::TextEnd { id: t });
        }
        for slot in self.tools.values_mut().filter(|s| !s.done) {
            slot.done = true;
            let input = if slot.args.trim().is_empty() {
                Some(Value::Object(Default::default()))
            } else {
                serde_json::from_str(&slot.args).ok()
            };
            if let Some(input) = input {
                out.push(AgentEvent::ToolInputReady { id: slot.id.clone(), name: None, input });
            }
        }
    }
}

/// Converts Responses API stream events (`client.responses().create_stream(..)`).
#[derive(Clone, Debug, Default)]
pub struct OpenAiResponsesAdapter {
    /// output item id -> function call id
    calls: HashMap<String, String>,
    called_tools: bool,
}

impl OpenAiResponsesAdapter {
    /// A fresh adapter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Convert one event.
    pub fn push(&mut self, ev: &ResponseStreamEvent) -> Vec<AgentEvent> {
        use ResponseStreamEvent as E;
        let call_id = |s: &Self, item: &str| s.calls.get(item).cloned().unwrap_or_else(|| item.to_owned());
        match ev {
            E::ResponseCreated(e) => vec![AgentEvent::RunStart { run_id: Some(e.response.id.clone()) }],
            E::ResponseOutputTextDelta(e) => vec![AgentEvent::text(&e.item_id, &e.delta)],
            E::ResponseOutputTextDone(e) => vec![AgentEvent::TextEnd { id: e.item_id.clone() }],
            E::ResponseRefusalDelta(e) => vec![AgentEvent::text(&e.item_id, &e.delta)],
            E::ResponseReasoningSummaryTextDelta(e) => vec![AgentEvent::reasoning(&e.item_id, &e.delta)],
            E::ResponseReasoningTextDelta(e) => vec![AgentEvent::reasoning(&e.item_id, &e.delta)],
            E::ResponseReasoningSummaryTextDone(e) => vec![AgentEvent::ReasoningEnd { id: e.item_id.clone() }],
            E::ResponseReasoningTextDone(e) => vec![AgentEvent::ReasoningEnd { id: e.item_id.clone() }],
            E::ResponseOutputItemAdded(e) => match &e.item {
                OutputItem::FunctionCall(fc) => {
                    self.called_tools = true;
                    if let Some(item) = &fc.id {
                        self.calls.insert(item.clone(), fc.call_id.clone());
                    }
                    vec![AgentEvent::ToolCallStart { id: fc.call_id.clone(), name: fc.name.clone() }]
                }
                _ => Vec::new(),
            },
            E::ResponseFunctionCallArgumentsDelta(e) => {
                vec![AgentEvent::ToolArgsDelta { id: call_id(self, &e.item_id), delta: e.delta.clone() }]
            }
            E::ResponseFunctionCallArgumentsDone(e) => match serde_json::from_str(&e.arguments) {
                Ok(input) => {
                    vec![AgentEvent::ToolInputReady { id: call_id(self, &e.item_id), name: e.name.clone(), input }]
                }
                Err(_) => Vec::new(),
            },
            E::ResponseCompleted(e) => {
                let mut out = usage(e.response.usage.as_ref());
                let stop = if self.called_tools { StopReason::ToolCalls } else { StopReason::EndTurn };
                out.push(AgentEvent::RunFinish { stop });
                out
            }
            E::ResponseIncomplete(e) => {
                let mut out = usage(e.response.usage.as_ref());
                let reason = e.response.incomplete_details.as_ref().map(|d| d.reason.as_str()).unwrap_or("length");
                out.push(AgentEvent::RunFinish { stop: StopReason::from_wire(reason) });
                out
            }
            E::ResponseFailed(e) => {
                let message =
                    e.response.error.as_ref().map(|er| er.message.clone()).unwrap_or_else(|| "Response failed".into());
                vec![AgentEvent::error(message), AgentEvent::RunFinish { stop: StopReason::Error }]
            }
            E::ResponseError(e) => vec![AgentEvent::Error { message: e.message.clone(), code: e.code.clone() }],
            _ => Vec::new(),
        }
    }
}

fn usage(u: Option<&async_openai::types::responses::ResponseUsage>) -> Vec<AgentEvent> {
    u.map(|u| {
        AgentEvent::Usage(Usage {
            input_tokens: Some(u.input_tokens.into()),
            output_tokens: Some(u.output_tokens.into()),
            total_tokens: Some(u.total_tokens.into()),
            ..Default::default()
        })
    })
    .into_iter()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{Session, ToolState};
    use serde_json::json;

    fn chunk(delta: Value, finish: Option<&str>) -> Value {
        json!({"id":"chatcmpl-1","object":"chat.completion.chunk","created":1,"model":"nova-2",
               "choices":[{"index":0,"delta":delta,"finish_reason":finish}]})
    }

    #[test]
    fn typed_chunks_with_parallel_tool_calls() {
        let raw = [
            chunk(json!({"role":"assistant","content":"Checking "}), None),
            chunk(json!({"content":"both."}), None),
            chunk(
                json!({"tool_calls":[{"index":0,"id":"call_a","type":"function","function":{"name":"weather","arguments":""}}]}),
                None,
            ),
            chunk(
                json!({"tool_calls":[{"index":1,"id":"call_b","type":"function","function":{"name":"time","arguments":"{\"tz\":"}}]}),
                None,
            ),
            chunk(json!({"tool_calls":[{"index":0,"function":{"arguments":"{\"city\":\"Oslo\"}"}}]}), None),
            chunk(json!({"tool_calls":[{"index":1,"function":{"arguments":"\"CET\"}"}}]}), None),
            chunk(json!({}), Some("tool_calls")),
            json!({"id":"chatcmpl-1","object":"chat.completion.chunk","created":1,"model":"nova-2","choices":[],
                   "usage":{"prompt_tokens":30,"completion_tokens":12,"total_tokens":42}}),
        ];
        let mut a = OpenAiChatAdapter::new();
        let mut s = Session::new();
        for r in raw {
            let typed: CreateChatCompletionStreamResponse = serde_json::from_value(r).expect("valid chunk");
            s.extend(a.push_chunk(&typed));
        }
        s.extend(a.finish());
        assert_eq!(s.messages[0].text(), "Checking both.");
        let wa = s.tool_call("call_a").unwrap();
        assert_eq!((wa.name.as_str(), wa.input_final), ("weather", true));
        assert_eq!(wa.input, Some(json!({"city": "Oslo"})));
        assert_eq!(s.tool_call("call_b").unwrap().input, Some(json!({"tz": "CET"})));
        assert_eq!(wa.state, ToolState::Pending);
        assert_eq!(s.usage.total(), Some(42));
        assert_eq!(s.status, crate::session::RunStatus::Finished(StopReason::ToolCalls));
    }

    #[test]
    fn raw_reasoning_fallback_for_compatible_servers() {
        let mut a = OpenAiChatAdapter::new();
        let mut s = Session::new();
        // vLLM / LM Studio style, then Ollama / OpenRouter style, then answer text
        s.extend(a.push_raw(&json!({"id":"x","choices":[{"index":0,"delta":{"reasoning_content":"Think "}}]})));
        s.extend(a.push_raw(&json!({"id":"x","choices":[{"index":0,"delta":{"reasoning":"harder"}}]})));
        s.extend(a.push_raw(&json!({"id":"x","choices":[{"index":0,"delta":{"content":"42"}}]})));
        s.extend(a.push_raw(&json!({"id":"x","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]})));
        let parts = &s.messages[0].parts;
        assert!(matches!(&parts[0], crate::Part::Reasoning(r) if r.text == "Think harder" && !r.streaming));
        assert_eq!(s.messages[0].text(), "42");
        assert_eq!(s.status, crate::session::RunStatus::Finished(StopReason::EndTurn));
    }

    #[test]
    fn responses_api_events() {
        let resp = |status: &str| {
            json!({"id":"resp_1","object":"response","created_at":1,"model":"nova-2","status":status,"output":[],
                   "parallel_tool_calls":true,"tool_choice":"auto","tools":[],
                   "usage":{"input_tokens":9,"input_tokens_details":{"cached_tokens":0},"output_tokens":3,
                            "output_tokens_details":{"reasoning_tokens":0},"total_tokens":12}})
        };
        let evs = [
            json!({"type":"response.created","sequence_number":0,"response":resp("in_progress")}),
            json!({"type":"response.reasoning_summary_text.delta","sequence_number":1,"item_id":"rs_1","output_index":0,"summary_index":0,"delta":"Plan"}),
            json!({"type":"response.output_item.added","sequence_number":2,"output_index":1,
                   "item":{"type":"function_call","id":"fc_1","call_id":"call_1","name":"lookup","arguments":""}}),
            json!({"type":"response.function_call_arguments.delta","sequence_number":3,"item_id":"fc_1","output_index":1,"delta":"{\"q\":1}"}),
            json!({"type":"response.function_call_arguments.done","sequence_number":4,"item_id":"fc_1","output_index":1,"arguments":"{\"q\":1}"}),
            json!({"type":"response.output_text.delta","sequence_number":5,"item_id":"msg_1","output_index":2,"content_index":0,"delta":"Done"}),
            json!({"type":"response.completed","sequence_number":6,"response":resp("completed")}),
        ];
        let mut a = OpenAiResponsesAdapter::new();
        let mut s = Session::new();
        for e in evs {
            let typed: ResponseStreamEvent = serde_json::from_value(e).expect("valid responses event");
            s.extend(a.push(&typed));
        }
        let c = s.tool_call("call_1").unwrap();
        assert_eq!((c.name.as_str(), c.input.clone()), ("lookup", Some(json!({"q": 1}))));
        assert_eq!(s.messages[0].text(), "Done");
        assert_eq!(s.usage.total(), Some(12));
        assert_eq!(s.status, crate::session::RunStatus::Finished(StopReason::ToolCalls));
    }
}
