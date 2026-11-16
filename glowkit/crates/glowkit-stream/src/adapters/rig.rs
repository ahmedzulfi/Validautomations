//! [rig](https://docs.rs/rig-core) 0.46: completion streams (`rig_core::streaming::StreamEvent`)
//! and agent runs (`rig_agent::agent::MultiTurnStreamItem`).
//!
//! A rig tool call learns its call id only when its part ends, so the adapter keys a streaming
//! call by its part and maps the real call id onto it when the id arrives; results committed
//! later find the right card.

use crate::event::{AgentEvent, StopReason, Usage};
use rig_agent::agent::MultiTurnStreamItem;
use rig_core::message::{AssistantContent, ToolResultContent};
use rig_core::streaming::{Item, PartKind, StreamEvent};
use serde_json::Value;
use std::collections::HashMap;

/// Converts rig stream items into [`AgentEvent`]s. One adapter per run.
#[derive(Clone, Debug, Default)]
pub struct RigAdapter {
    started: bool,
    turn: usize,
    /// part index of the current turn -> (kind, our id)
    parts: HashMap<usize, (PartKind, String)>,
    /// rig call id -> our tool id
    calls: HashMap<String, String>,
}

impl RigAdapter {
    /// A fresh adapter.
    pub fn new() -> Self {
        Self::default()
    }

    fn start(&mut self, out: &mut Vec<AgentEvent>) {
        if !self.started {
            self.started = true;
            out.push(AgentEvent::RunStart { run_id: None });
        }
    }

    fn part_id(&self, index: usize, kind: &str) -> String {
        format!("rig-{}-{kind}-{index}", self.turn)
    }

    /// Convert one completion stream event.
    pub fn push_event(&mut self, ev: &StreamEvent) -> Vec<AgentEvent> {
        let mut out = Vec::new();
        self.start(&mut out);
        let idx = ev.part().index();
        match ev {
            StreamEvent::Start { kind, name, .. } => {
                let tag = match kind {
                    PartKind::Text => "text",
                    PartKind::Reasoning => "reasoning",
                    PartKind::ToolCall => "call",
                    PartKind::Image => "image",
                    PartKind::Opaque => "opaque",
                };
                let id = self.part_id(idx, tag);
                self.parts.insert(idx, (*kind, id.clone()));
                match kind {
                    PartKind::Text => out.push(AgentEvent::TextStart { id }),
                    PartKind::ToolCall => out.push(AgentEvent::ToolCallStart {
                        id,
                        name: name.as_ref().map(|n| n.as_str().to_owned()).unwrap_or_default(),
                    }),
                    _ => {}
                }
            }
            StreamEvent::Text { text, .. } => out.push(AgentEvent::text(self.id_for(idx, "text"), text)),
            StreamEvent::Reasoning { text, .. } => out.push(AgentEvent::reasoning(self.id_for(idx, "reasoning"), text)),
            StreamEvent::Arguments { json, .. } => {
                out.push(AgentEvent::ToolArgsDelta { id: self.id_for(idx, "call"), delta: json.clone() })
            }
            StreamEvent::End { content, .. } => match content {
                AssistantContent::Text(_) => out.push(AgentEvent::TextEnd { id: self.id_for(idx, "text") }),
                AssistantContent::Reasoning(_) => {
                    out.push(AgentEvent::ReasoningEnd { id: self.id_for(idx, "reasoning") })
                }
                AssistantContent::ToolCall(tc) => {
                    let id = self.id_for(idx, "call");
                    self.calls.insert(tc.id.wire().into_owned(), id.clone());
                    out.push(AgentEvent::ToolInputReady {
                        id,
                        name: Some(tc.function.name.as_str().to_owned()),
                        input: Value::Object(tc.function.arguments.clone()),
                    });
                }
                _ => {}
            },
        }
        out
    }

    fn id_for(&self, idx: usize, tag: &str) -> String {
        self.parts.get(&idx).map(|(_, id)| id.clone()).unwrap_or_else(|| self.part_id(idx, tag))
    }

    fn tool_id(&mut self, call: &str) -> String {
        self.calls.get(call).cloned().unwrap_or_else(|| call.to_owned())
    }

    /// Convert one agent stream item (`agent.prompt(..).stream()`).
    pub fn push_item(&mut self, item: &MultiTurnStreamItem) -> Vec<AgentEvent> {
        let mut out = Vec::new();
        self.start(&mut out);
        match item {
            MultiTurnStreamItem::StreamAssistantItem(Item::Event(ev)) => out.extend(self.push_event(ev)),
            MultiTurnStreamItem::StreamAssistantItem(_) => {}
            MultiTurnStreamItem::ToolCall { tool_call } => {
                let id = self.tool_id(&tool_call.id.wire());
                out.push(AgentEvent::ToolInputReady {
                    id,
                    name: Some(tool_call.function.name.as_str().to_owned()),
                    input: Value::Object(tool_call.function.arguments.clone()),
                });
            }
            MultiTurnStreamItem::ToolExecutionCommitted { tool_call } => {
                out.push(AgentEvent::ToolRunning { id: self.tool_id(&tool_call.id.wire()) })
            }
            MultiTurnStreamItem::ToolResult { tool_result } => {
                let id = self.tool_id(&tool_result.call.wire());
                out.push(AgentEvent::ToolResult {
                    id,
                    output: result_value(&tool_result.content),
                    is_error: tool_result.is_error,
                });
            }
            MultiTurnStreamItem::CompletionCall(call) => {
                // the next model turn numbers its parts from zero again
                self.turn += 1;
                self.parts.clear();
                out.push(AgentEvent::Usage(Usage {
                    input_tokens: call.usage.input_tokens,
                    output_tokens: call.usage.output_tokens,
                    total_tokens: call.usage.total_tokens,
                    ..Default::default()
                }));
            }
            MultiTurnStreamItem::ModelTurnRetried { turn } => {
                self.turn += 1;
                self.parts.clear();
                out.push(AgentEvent::Custom { name: "rig/model_turn_retried".into(), value: (*turn).into() });
            }
            MultiTurnStreamItem::FinalResponse(resp) => {
                out.push(AgentEvent::Usage(Usage {
                    input_tokens: resp.usage.input_tokens,
                    output_tokens: resp.usage.output_tokens,
                    total_tokens: resp.usage.total_tokens,
                    ..Default::default()
                }));
                out.push(AgentEvent::RunFinish { stop: StopReason::EndTurn });
            }
            _ => {} // non_exhaustive: newer items are ignored
        }
        out
    }

    /// The stream yielded an error (it is terminal).
    pub fn push_error(&mut self, err: &dyn std::fmt::Display) -> Vec<AgentEvent> {
        let mut out = Vec::new();
        self.start(&mut out);
        out.push(AgentEvent::error(err.to_string()));
        out.push(AgentEvent::RunFinish { stop: StopReason::Error });
        out
    }
}

fn result_value(content: &[ToolResultContent]) -> Value {
    let mut texts = Vec::new();
    for c in content {
        if let Some(v) = c.as_json() {
            if content.len() == 1 {
                return v.clone();
            }
            texts.push(v.to_string());
        } else if let Some(t) = c.as_text() {
            texts.push(t.to_owned());
        } else {
            texts.push("[image]".to_owned());
        }
    }
    let text = texts.join("\n");
    serde_json::from_str::<Value>(&text).ok().filter(|v| v.is_object() || v.is_array()).unwrap_or(Value::String(text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{Session, ToolState};
    use serde_json::json;

    fn item(v: Value) -> MultiTurnStreamItem {
        serde_json::from_value(v).expect("valid rig item")
    }

    fn ev(v: Value) -> Value {
        json!({"type": "streamAssistantItem", "item": "event", "value": v})
    }

    #[test]
    fn completion_events_convert() {
        let mut a = RigAdapter::new();
        let mut s = Session::new();
        for e in [
            json!({"event":"start","part":0,"kind":"reasoning"}),
            json!({"event":"reasoning","part":0,"text":"hmm"}),
            json!({"event":"end","part":0,"content":{"type":"reasoning","text":"hmm"}}),
            json!({"event":"start","part":1,"kind":"text"}),
            json!({"event":"text","part":1,"text":"Hello"}),
            json!({"event":"end","part":1,"content":{"type":"text","text":"Hello"}}),
        ] {
            let e: StreamEvent = serde_json::from_value(e).expect("valid rig event");
            s.extend(a.push_event(&e));
        }
        assert_eq!(s.messages[0].text(), "Hello");
        assert!(matches!(&s.messages[0].parts[0], crate::Part::Reasoning(r) if r.text == "hmm" && !r.streaming));
    }

    #[test]
    fn agent_items_map_call_ids() {
        let call = json!({"id":{"provider":"call_9"},"function":{"name":"add","arguments":{"a":1,"b":2}}});
        let items = [
            ev(json!({"event":"start","part":0,"kind":"tool_call","name":"add"})),
            ev(json!({"event":"arguments","part":0,"json":"{\"a\":1,"})),
            ev(json!({"event":"arguments","part":0,"json":"\"b\":2}"})),
            ev(
                json!({"event":"end","part":0,"content":{"type":"toolcall","id":{"provider":"call_9"},"function":{"name":"add","arguments":{"a":1,"b":2}}}}),
            ),
            json!({"type":"toolCall","tool_call":call}),
            json!({"type":"toolExecutionCommitted","tool_call":call}),
            json!({"type":"toolResult","tool_result":{"call":{"provider":"call_9"},"name":"add","content":[{"type":"text","text":"3"}]}}),
            ev(json!({"event":"start","part":0,"kind":"text"})),
            ev(json!({"event":"text","part":0,"text":"It's 3."})),
        ];
        let mut a = RigAdapter::new();
        let mut s = Session::new();
        for (i, raw) in items.into_iter().enumerate() {
            if i == 7 {
                // a completion call finished between the turns
                s.extend(a.push_item(&item(json!({"type":"completionCall","call_index":0,"usage":{"input_tokens":5,"output_tokens":2},"raw":{}}))));
            }
            s.extend(a.push_item(&item(raw)));
        }
        s.extend(a.push_error(&"provider timed out"));
        let calls: Vec<_> = s.tool_calls().collect();
        assert_eq!(calls.len(), 1, "the committed call reuses the streamed card");
        assert_eq!(calls[0].name, "add");
        assert_eq!(calls[0].state, ToolState::Done);
        assert_eq!(calls[0].output, Some(Value::String("3".into())));
        assert_eq!(s.messages[0].text(), "It's 3.");
        assert_eq!(s.usage.input_tokens, Some(5));
        assert_eq!(s.last_error.as_deref(), Some("provider timed out"));
    }
}
