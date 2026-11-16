//! Best-effort parsing of incomplete JSON, for previewing tool arguments while they stream.

use serde_json::Value;

/// Parse a JSON prefix such as `{"city": "Par` into the most complete value it can
/// (`{"city": "Par"}`). Returns `None` when nothing useful can be recovered yet.
///
/// ```
/// use glowkit_stream::parse_partial_json;
/// let v = parse_partial_json(r#"{"query": "rust ui", "limit": 1"#).unwrap();
/// assert_eq!(v["query"], "rust ui");
/// ```
pub fn parse_partial_json(text: &str) -> Option<Value> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(v) = serde_json::from_str(text) {
        return Some(v);
    }
    let scan = Scan::new(text);
    if let Some(v) = close(text, &scan) {
        return Some(v);
    }
    // Cut back to the last structural boundary and try again.
    for &cut in scan.cuts.iter().rev() {
        let prefix = &text[..cut];
        if let Some(v) = close(prefix, &Scan::new(prefix)) {
            return Some(v);
        }
    }
    None
}

struct Scan {
    stack: Vec<u8>,
    in_string: bool,
    escaped: bool,
    /// Byte offsets where the text can be cut and still be closed: just before a `,` and just
    /// after an opening bracket.
    cuts: Vec<usize>,
}

impl Scan {
    fn new(text: &str) -> Self {
        let mut s = Scan { stack: Vec::new(), in_string: false, escaped: false, cuts: Vec::new() };
        for (i, b) in text.bytes().enumerate() {
            if s.in_string {
                if s.escaped {
                    s.escaped = false;
                } else if b == b'\\' {
                    s.escaped = true;
                } else if b == b'"' {
                    s.in_string = false;
                }
                continue;
            }
            match b {
                b'"' => s.in_string = true,
                b'{' | b'[' => {
                    s.stack.push(b);
                    s.cuts.push(i + 1);
                }
                b'}' | b']' => {
                    s.stack.pop();
                }
                b',' => s.cuts.push(i),
                _ => {}
            }
        }
        s
    }
}

fn close(prefix: &str, scan: &Scan) -> Option<Value> {
    let mut out = String::with_capacity(prefix.len() + scan.stack.len() + 2);
    out.push_str(prefix);
    if scan.in_string {
        if scan.escaped {
            out.pop();
        }
        out.push('"');
    }
    let trimmed = out.trim_end().trim_end_matches(',').trim_end().len();
    out.truncate(trimmed);
    for &open in scan.stack.iter().rev() {
        out.push(if open == b'{' { '}' } else { ']' });
    }
    serde_json::from_str(&out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn complete_json_parses() {
        assert_eq!(parse_partial_json(r#"{"a":1}"#), Some(json!({"a": 1})));
    }

    #[test]
    fn open_string_is_closed() {
        assert_eq!(parse_partial_json(r#"{"city": "Par"#), Some(json!({"city": "Par"})));
    }

    #[test]
    fn dangling_key_is_dropped() {
        assert_eq!(parse_partial_json(r#"{"a": 1, "ci"#), Some(json!({"a": 1})));
        assert_eq!(parse_partial_json(r#"{"a": 1, "city":"#), Some(json!({"a": 1})));
    }

    #[test]
    fn nested_and_arrays() {
        assert_eq!(parse_partial_json(r#"{"xs": [1, 2, {"k": tr"#), Some(json!({"xs": [1, 2, {}]})));
        assert_eq!(parse_partial_json(r#"[1, 2,"#), Some(json!([1, 2])));
    }

    #[test]
    fn escapes_survive() {
        assert_eq!(parse_partial_json(r#"{"s": "a\"b\"#), Some(json!({"s": "a\"b"})));
    }

    #[test]
    fn nothing_useful() {
        assert_eq!(parse_partial_json(""), None);
        assert_eq!(parse_partial_json("   "), None);
    }
}
