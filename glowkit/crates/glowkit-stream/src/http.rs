//! A small HTTP helper (feature `http`, reqwest): POST a request to an AG-UI or Vercel AI SDK
//! endpoint and stream the response into the bridge.

use crate::adapters::{SseDecoder, SseProtocol};
use crate::bridge::EventSender;
use crate::event::{AgentEvent, StopReason};
use futures_util::StreamExt;
use serde_json::Value;

/// Why a streamed request failed.
#[derive(Debug)]
pub enum HttpError {
    /// Network / HTTP error.
    Request(reqwest::Error),
    /// The server answered with a non-success status.
    Status(u16, String),
    /// The UI side of the bridge was dropped.
    Disconnected,
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpError::Request(e) => write!(f, "request failed: {e}"),
            HttpError::Status(code, body) => write!(f, "server returned {code}: {body}"),
            HttpError::Disconnected => f.write_str("the UI went away"),
        }
    }
}

impl std::error::Error for HttpError {}

impl From<reqwest::Error> for HttpError {
    fn from(e: reqwest::Error) -> Self {
        HttpError::Request(e)
    }
}

/// POST `body` as JSON to `url`, decode the SSE response with `protocol` and send every event to
/// `tx`. Network failures are also reported to the UI as an [`AgentEvent::Error`].
///
/// ```no_run
/// # async fn run(tx: glowkit_stream::bridge::EventSender) {
/// use glowkit_stream::adapters::{ag_ui, SseProtocol};
/// let body = ag_ui::run_input("thread-1", "run-1", vec![serde_json::json!({
///     "id": "u1", "role": "user", "content": "Plan my trip"})], vec![]);
/// let client = reqwest::Client::new();
/// glowkit_stream::http::stream_sse(&client, "http://localhost:8000/agent", &body, SseProtocol::AgUi, &tx)
///     .await
///     .ok();
/// # }
/// ```
pub async fn stream_sse(
    client: &reqwest::Client,
    url: &str,
    body: &Value,
    protocol: SseProtocol,
    tx: &EventSender,
) -> Result<(), HttpError> {
    let result = run(client, url, body, protocol, tx).await;
    if let Err(e) = &result {
        if !matches!(e, HttpError::Disconnected) {
            tx.send_all([AgentEvent::error(e.to_string()), AgentEvent::RunFinish { stop: StopReason::Error }]).ok();
        }
    }
    result
}

async fn run(
    client: &reqwest::Client,
    url: &str,
    body: &Value,
    protocol: SseProtocol,
    tx: &EventSender,
) -> Result<(), HttpError> {
    let resp = client.post(url).header("accept", "text/event-stream").json(body).send().await?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(HttpError::Status(status.as_u16(), text.chars().take(300).collect()));
    }
    let mut decoder = SseDecoder::new(protocol);
    let mut bytes = resp.bytes_stream();
    while let Some(chunk) = bytes.next().await {
        let events = decoder.feed(&chunk?);
        tx.send_all(events).map_err(|_| HttpError::Disconnected)?;
    }
    tx.send_all(decoder.finish()).map_err(|_| HttpError::Disconnected)?;
    Ok(())
}
