//! An incremental Server-Sent Events parser: feed it bytes as they arrive, get events out.

/// One Server-Sent Event.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SseEvent {
    /// The `event:` field (`None` means the default "message").
    pub event: Option<String>,
    /// The `data:` lines joined with `\n`.
    pub data: String,
    /// The `id:` field.
    pub id: Option<String>,
}

impl SseEvent {
    /// Whether this is the `[DONE]` sentinel OpenAI-style and Vercel streams end with.
    pub fn is_done(&self) -> bool {
        self.data.trim() == "[DONE]"
    }
}

/// Incremental SSE parser (per the WHATWG event-stream rules): handles events split across
/// chunks (even mid-line or mid-UTF-8 character), `\n`, `\r\n` and `\r` line endings, multi-line
/// `data:`, comments (`: keep-alive`) and `retry:` fields.
///
/// ```
/// use glowkit_stream::sse::SseParser;
/// let mut p = SseParser::new();
/// assert!(p.feed(b"data: {\"a\"").is_empty());
/// let events = p.feed(b": 1}\n\n");
/// assert_eq!(events[0].data, "{\"a\": 1}");
/// ```
#[derive(Clone, Debug, Default)]
pub struct SseParser {
    buf: Vec<u8>,
    data: Option<String>,
    event: Option<String>,
    id: Option<String>,
    /// The last chunk ended with `\r`: a `\n` at the start of the next one belongs to it.
    pending_cr: bool,
    started: bool,
    data_lines: usize,
}

impl SseParser {
    /// A fresh parser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed the next chunk of bytes; returns every event completed by it.
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        let mut out = Vec::new();
        let mut bytes = chunk;
        if self.pending_cr {
            self.pending_cr = false;
            if bytes.first() == Some(&b'\n') {
                bytes = &bytes[1..];
            }
        }
        self.buf.extend_from_slice(bytes);
        let mut start = 0;
        let mut i = 0;
        while i < self.buf.len() {
            let b = self.buf[i];
            if b == b'\n' || b == b'\r' {
                let line_end = i;
                let mut next = i + 1;
                if b == b'\r' {
                    if next < self.buf.len() {
                        if self.buf[next] == b'\n' {
                            next += 1;
                        }
                    } else {
                        self.pending_cr = true;
                    }
                }
                let line = String::from_utf8_lossy(&self.buf[start..line_end]).into_owned();
                if let Some(ev) = self.line(&line) {
                    out.push(ev);
                }
                start = next;
                i = next;
            } else {
                i += 1;
            }
        }
        self.buf.drain(..start);
        out
    }

    /// Call when the stream ends: dispatches a final event that was not followed by a blank line.
    pub fn finish(&mut self) -> Option<SseEvent> {
        if !self.buf.is_empty() {
            let line = String::from_utf8_lossy(&std::mem::take(&mut self.buf)).into_owned();
            if let Some(ev) = self.line(&line) {
                return Some(ev);
            }
        }
        self.dispatch()
    }

    fn line(&mut self, line: &str) -> Option<SseEvent> {
        let mut line = line;
        if !self.started {
            self.started = true;
            line = line.strip_prefix('\u{feff}').unwrap_or(line);
        }
        if line.is_empty() {
            return self.dispatch();
        }
        if line.starts_with(':') {
            return None;
        }
        let (field, value) = match line.find(':') {
            Some(i) => {
                let v = &line[i + 1..];
                (&line[..i], v.strip_prefix(' ').unwrap_or(v))
            }
            None => (line, ""),
        };
        match field {
            "data" => {
                let d = self.data.get_or_insert_with(String::new);
                if self.data_lines > 0 {
                    d.push('\n');
                }
                d.push_str(value);
                self.data_lines += 1;
            }
            "event" => self.event = Some(value.to_owned()),
            "id" if !value.contains('\0') => self.id = Some(value.to_owned()),
            _ => {} // retry and unknown fields are ignored
        }
        None
    }

    fn dispatch(&mut self) -> Option<SseEvent> {
        self.data_lines = 0;
        let event = self.event.take();
        let data = self.data.take().filter(|d| !d.is_empty())?;
        Some(SseEvent { event, data, id: self.id.clone() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(p: &mut SseParser, chunks: &[&[u8]]) -> Vec<SseEvent> {
        let mut out = Vec::new();
        for c in chunks {
            out.extend(p.feed(c));
        }
        out.extend(p.finish());
        out
    }

    #[test]
    fn single_event() {
        let ev = all(&mut SseParser::new(), &[b"data: hello\n\n"]);
        assert_eq!(ev, vec![SseEvent { event: None, data: "hello".into(), id: None }]);
    }

    #[test]
    fn split_at_every_byte() {
        let src = b"event: delta\nid: 7\ndata: {\"x\": \"caf\xc3\xa9\"}\n\ndata: [DONE]\n\n";
        let chunks: Vec<&[u8]> = src.chunks(1).collect();
        let ev = all(&mut SseParser::new(), &chunks);
        assert_eq!(ev.len(), 2);
        assert_eq!(ev[0].event.as_deref(), Some("delta"));
        assert_eq!(ev[0].id.as_deref(), Some("7"));
        assert_eq!(ev[0].data, "{\"x\": \"café\"}");
        assert!(ev[1].is_done());
    }

    #[test]
    fn multi_line_data_and_comments() {
        let ev =
            all(&mut SseParser::new(), &[b": keep-alive\n", b"data: line one\ndata: line two\n", b"retry: 10\n\n"]);
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].data, "line one\nline two");
    }

    #[test]
    fn crlf_split_between_chunks() {
        let ev = all(&mut SseParser::new(), &[b"data: a\r", b"\n\r", b"\ndata: b\r\n\r\n"]);
        assert_eq!(ev.iter().map(|e| e.data.as_str()).collect::<Vec<_>>(), ["a", "b"]);
    }

    #[test]
    fn bare_cr_lines() {
        let ev = all(&mut SseParser::new(), &[b"data: a\r\rdata: b\r\r"]);
        assert_eq!(ev.len(), 2);
    }

    #[test]
    fn trailing_event_without_blank_line_is_flushed() {
        let ev = all(&mut SseParser::new(), &[b"data: tail"]);
        assert_eq!(ev[0].data, "tail");
    }

    #[test]
    fn blank_lines_without_data_dispatch_nothing() {
        let ev = all(&mut SseParser::new(), &[b"\n\nevent: ping\n\n"]);
        assert!(ev.is_empty());
    }

    #[test]
    fn no_space_after_colon_and_bom() {
        let ev = all(&mut SseParser::new(), &["\u{feff}data:x\n\n".as_bytes()]);
        assert_eq!(ev[0].data, "x");
    }
}
