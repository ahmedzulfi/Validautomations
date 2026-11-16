//! Source adapters: each converts one protocol or library stream into [`AgentEvent`]s.
//!
//! | Feature | Module | Source |
//! |---|---|---|
//! | `ag-ui` | [`ag_ui`] | AG-UI 1.x events (own serde types) |
//! | `vercel` | [`vercel`] | Vercel AI SDK v7 UI message stream (own serde types) |
//! | `acp-wire` / `acp` | [`acp`] | Agent Client Protocol JSON-RPC / `agent-client-protocol` 5.x types |
//! | `async-openai` | [`openai`] | Chat Completions chunks and Responses API events |
//! | `rig` | [`rig`] | rig-core 0.46 stream events and rig-agent multi-turn items |

#[allow(unused_imports)]
use crate::event::AgentEvent;

#[cfg(feature = "acp-wire")]
pub mod acp;
#[cfg(feature = "ag-ui")]
pub mod ag_ui;
#[cfg(feature = "async-openai")]
pub mod openai;
#[cfg(feature = "rig")]
pub mod rig;
#[cfg(feature = "vercel")]
pub mod vercel;

/// The SSE-based protocols [`SseDecoder`] understands.
#[cfg(any(feature = "ag-ui", feature = "vercel"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SseProtocol {
    /// AG-UI events.
    #[cfg(feature = "ag-ui")]
    AgUi,
    /// Vercel AI SDK UI message stream.
    #[cfg(feature = "vercel")]
    Vercel,
}

#[cfg(any(feature = "ag-ui", feature = "vercel"))]
#[derive(Clone, Debug)]
enum Wire {
    #[cfg(feature = "ag-ui")]
    AgUi(ag_ui::AgUiAdapter),
    #[cfg(feature = "vercel")]
    Vercel(vercel::VercelAdapter),
}

/// Bytes in, [`AgentEvent`]s out: an incremental [`SseParser`](crate::sse::SseParser) feeding a
/// protocol adapter. Handles chunk boundaries anywhere; malformed or unknown events are skipped.
///
/// ```
/// use glowkit_stream::adapters::{SseDecoder, SseProtocol};
/// let mut d = SseDecoder::new(SseProtocol::Vercel);
/// let mut events = d.feed(b"data: {\"type\":\"text-delta\",\"id\":\"t\",");
/// events.extend(d.feed(b"\"delta\":\"Hi\"}\n\ndata: [DONE]\n\n"));
/// assert_eq!(events, vec![glowkit_stream::AgentEvent::text("t", "Hi")]);
/// ```
#[cfg(any(feature = "ag-ui", feature = "vercel"))]
#[derive(Clone, Debug)]
pub struct SseDecoder {
    parser: crate::sse::SseParser,
    wire: Wire,
    /// Count of `data:` payloads that failed to parse (for diagnostics).
    pub skipped: usize,
}

#[cfg(any(feature = "ag-ui", feature = "vercel"))]
impl SseDecoder {
    /// A decoder for `protocol`.
    pub fn new(protocol: SseProtocol) -> Self {
        let wire = match protocol {
            #[cfg(feature = "ag-ui")]
            SseProtocol::AgUi => Wire::AgUi(ag_ui::AgUiAdapter::new()),
            #[cfg(feature = "vercel")]
            SseProtocol::Vercel => Wire::Vercel(vercel::VercelAdapter::new()),
        };
        Self { parser: crate::sse::SseParser::new(), wire, skipped: 0 }
    }

    /// Feed the next chunk of the response body.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<AgentEvent> {
        let events = self.parser.feed(bytes);
        let mut out = Vec::new();
        for e in events {
            self.data(&e.data, &mut out);
        }
        out
    }

    /// The body ended.
    pub fn finish(&mut self) -> Vec<AgentEvent> {
        let mut out = Vec::new();
        if let Some(e) = self.parser.finish() {
            self.data(&e.data, &mut out);
        }
        out
    }

    /// Decode a complete recorded body at once.
    pub fn decode_all(protocol: SseProtocol, body: &[u8]) -> Vec<AgentEvent> {
        let mut d = Self::new(protocol);
        let mut out = d.feed(body);
        out.extend(d.finish());
        out
    }

    fn data(&mut self, data: &str, out: &mut Vec<AgentEvent>) {
        if data.trim() == "[DONE]" {
            return;
        }
        let r = match &mut self.wire {
            #[cfg(feature = "ag-ui")]
            Wire::AgUi(a) => a.push_json(data),
            #[cfg(feature = "vercel")]
            Wire::Vercel(a) => a.push_json(data),
        };
        match r {
            Ok(ev) => out.extend(ev),
            Err(_) => self.skipped += 1,
        }
    }
}
