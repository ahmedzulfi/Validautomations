//! A small, streaming-tolerant Markdown parser.
//!
//! It understands the subset LLMs actually emit: ATX headings, paragraphs, bold / italic /
//! strikethrough / inline code / links, bullet, numbered and task lists (nested by indentation),
//! block quotes, fenced code blocks, GitHub-style tables and horizontal rules.
//!
//! Because responses arrive token by token, the parser never shows half-typed syntax:
//! an unterminated ```` ``` ```` fence is still a code block (with `closed: false`), an unclosed
//! `**bold` or `` `code `` runs to the end of the paragraph, dangling markers at the very end are
//! hidden, and a table whose separator row has not arrived yet already renders as a table.

/// Inline style flags of a [`Span`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SpanStyle {
    /// `**strong**`
    pub bold: bool,
    /// `*emphasis*`
    pub italic: bool,
    /// `` `code` ``
    pub code: bool,
    /// `~~strikethrough~~`
    pub strike: bool,
}

/// A run of inline text with one style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    /// The visible text (markers removed).
    pub text: String,
    /// Style flags.
    pub style: SpanStyle,
    /// Link target, if this span is (part of) a link.
    pub link: Option<String>,
}

impl Span {
    fn plain(text: impl Into<String>) -> Self {
        Self { text: text.into(), style: SpanStyle::default(), link: None }
    }
}

/// Column alignment of a table, from the separator row (`:--`, `:-:`, `--:`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColumnAlign {
    /// Left aligned (default).
    #[default]
    Left,
    /// Centered.
    Center,
    /// Right aligned.
    Right,
}

/// One list entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListItem {
    /// Nesting level, 0 = top level.
    pub depth: usize,
    /// `Some(n)` for an ordered item, `None` for a bullet.
    pub number: Option<u64>,
    /// `Some(done)` for a task item (`- [ ]` / `- [x]`).
    pub checked: Option<bool>,
    /// Item text.
    pub spans: Vec<Span>,
}

/// A parsed Markdown block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    /// `#` .. `######`
    Heading {
        /// 1..=6
        level: u8,
        /// Heading text.
        spans: Vec<Span>,
    },
    /// A paragraph of inline text.
    Paragraph(Vec<Span>),
    /// Consecutive list items (bullets and numbers may mix, nesting is per item).
    List(Vec<ListItem>),
    /// `>` block quote, containing blocks.
    Quote(Vec<Block>),
    /// Fenced code block.
    Code {
        /// Info string's first word, e.g. `rust`.
        lang: String,
        /// The code, without the fences.
        code: String,
        /// `false` while the closing fence has not arrived yet (streaming).
        closed: bool,
    },
    /// GitHub-style table.
    Table {
        /// Header cells.
        header: Vec<Vec<Span>>,
        /// Alignment per column.
        align: Vec<ColumnAlign>,
        /// Body rows (each padded / truncated to the header's column count).
        rows: Vec<Vec<Vec<Span>>>,
    },
    /// `---`
    Rule,
}

/// Parse a Markdown document into blocks.
///
/// ```
/// use glowkit_ai::markdown::{parse, Block};
/// let blocks = parse("# Title\n\nSome **bold** text");
/// assert!(matches!(blocks[0], Block::Heading { level: 1, .. }));
/// ```
pub fn parse(src: &str) -> Vec<Block> {
    let src = src.replace("\r\n", "\n").replace('\t', "    ");
    let lines: Vec<&str> = src.split('\n').collect();
    parse_lines(&lines)
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

/// Opening code fence: returns (fence char, run length, info string).
fn fence_open(line: &str) -> Option<(char, usize, String)> {
    let t = line.trim_start();
    let c = t.chars().next()?;
    if c != '`' && c != '~' {
        return None;
    }
    let run = t.chars().take_while(|&x| x == c).count();
    if run < 3 {
        return None;
    }
    let info = t[run..].trim();
    if c == '`' && info.contains('`') {
        return None;
    }
    Some((c, run, info.split_whitespace().next().unwrap_or("").to_owned()))
}

fn fence_close(line: &str, c: char, len: usize) -> bool {
    let t = line.trim();
    let run = t.chars().take_while(|&x| x == c).count();
    run >= len && t.chars().all(|x| x == c)
}

fn heading(line: &str) -> Option<(u8, &str)> {
    if indent_of(line) > 3 {
        return None;
    }
    let t = line.trim_start();
    let n = t.chars().take_while(|&c| c == '#').count();
    if n == 0 || n > 6 {
        return None;
    }
    let rest = &t[n..];
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    // strip an optional closing run of #'s
    let mut text = rest.trim();
    let stripped = text.trim_end_matches('#');
    if stripped.is_empty() || stripped.ends_with(' ') {
        text = stripped.trim_end();
    }
    Some((n as u8, text))
}

fn is_rule(line: &str) -> bool {
    if indent_of(line) > 3 {
        return false;
    }
    let t = line.trim();
    let Some(c) = t.chars().next() else { return false };
    if !matches!(c, '-' | '*' | '_') {
        return false;
    }
    t.chars().all(|x| x == c || x == ' ') && t.chars().filter(|&x| x == c).count() >= 3
}

/// List marker: (indent, number, content after the marker).
fn list_marker(line: &str) -> Option<(usize, Option<u64>, &str)> {
    let indent = indent_of(line);
    let t = &line[indent..];
    let mut chars = t.char_indices();
    let (_, c) = chars.next()?;
    if matches!(c, '-' | '*' | '+') {
        let rest = &t[1..];
        if rest.is_empty() {
            return Some((indent, None, ""));
        }
        if rest.starts_with(' ') {
            return Some((indent, None, rest.trim_start()));
        }
        return None;
    }
    let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 || digits > 9 {
        return None;
    }
    let after = &t[digits..];
    if after.starts_with('.') || after.starts_with(')') {
        let rest = &after[1..];
        if rest.is_empty() || rest.starts_with(' ') {
            let n = t[..digits].parse().unwrap_or(1);
            return Some((indent, Some(n), rest.trim_start()));
        }
    }
    None
}

fn split_row(line: &str) -> Vec<String> {
    let mut t = line.trim();
    if let Some(s) = t.strip_prefix('|') {
        t = s;
    }
    if let Some(s) = t.strip_suffix('|') {
        if !s.ends_with('\\') {
            t = s;
        }
    }
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut in_code = false;
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                cur.push('|');
                chars.next();
            }
            '`' => {
                in_code = !in_code;
                cur.push(c);
            }
            '|' if !in_code => cells.push(std::mem::take(&mut cur).trim().to_owned()),
            _ => cur.push(c),
        }
    }
    cells.push(cur.trim().to_owned());
    cells
}

fn is_separator(line: &str) -> bool {
    let t = line.trim();
    if !t.contains('-') {
        return false;
    }
    if !t.chars().all(|c| matches!(c, '|' | ':' | '-' | ' ')) {
        return false;
    }
    split_row(t).iter().all(|c| {
        let c = c.trim();
        !c.is_empty() && c.trim_matches(':').chars().all(|x| x == '-') && c.contains('-')
    })
}

fn is_partial_separator(line: &str) -> bool {
    let t = line.trim();
    !t.is_empty() && t.starts_with('|') && t.chars().all(|c| matches!(c, '|' | ':' | '-' | ' '))
}

/// Does a table start at line `i`? Returns the number of header+separator lines (1 when the
/// separator has not streamed in yet).
fn table_start(lines: &[&str], i: usize) -> Option<usize> {
    let line = lines[i];
    if !line.contains('|') || indent_of(line) > 3 {
        return None;
    }
    let next = lines.get(i + 1).copied();
    match next {
        Some(n) if is_separator(n) => Some(2),
        // streaming: the header row is the last line, or the separator is half-written
        None if line.trim_start().starts_with('|') => Some(1),
        Some(n) if i + 2 == lines.len() && is_partial_separator(n) && line.trim_start().starts_with('|') => Some(2),
        _ => None,
    }
}

fn starts_block(lines: &[&str], i: usize) -> bool {
    let line = lines[i];
    fence_open(line).is_some()
        || heading(line).is_some()
        || is_rule(line)
        || line.trim_start().starts_with('>')
        || list_marker(line).is_some()
        || table_start(lines, i).is_some()
}

fn parse_lines(lines: &[&str]) -> Vec<Block> {
    let mut blocks = Vec::new();
    let n = lines.len();
    let mut i = 0;
    while i < n {
        let line = lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        // fenced code
        if let Some((fc, flen, lang)) = fence_open(line) {
            let ind = indent_of(line);
            let mut code = Vec::new();
            let mut closed = false;
            i += 1;
            while i < n {
                if fence_close(lines[i], fc, flen) {
                    closed = true;
                    i += 1;
                    break;
                }
                let l = lines[i];
                let strip = indent_of(l).min(ind);
                code.push(&l[strip..]);
                i += 1;
            }
            // while streaming, a partial closing fence ("`" or "``") must not show up as code
            if !closed {
                if let Some(last) = code.last() {
                    let t = last.trim();
                    if !t.is_empty() && t.len() < flen && t.chars().all(|c| c == fc) {
                        code.pop();
                    }
                }
            }
            blocks.push(Block::Code { lang, code: code.join("\n"), closed });
            continue;
        }
        if let Some((level, text)) = heading(line) {
            blocks.push(Block::Heading { level, spans: parse_inline(text) });
            i += 1;
            continue;
        }
        if is_rule(line) {
            blocks.push(Block::Rule);
            i += 1;
            continue;
        }
        if line.trim_start().starts_with('>') {
            let mut inner = Vec::new();
            while i < n && line_is_quote(lines[i]) {
                let t = lines[i].trim_start();
                let t = &t[1..];
                inner.push(t.strip_prefix(' ').unwrap_or(t));
                i += 1;
            }
            blocks.push(Block::Quote(parse_lines(&inner)));
            continue;
        }
        if let Some(head) = table_start(lines, i) {
            let header_cells = split_row(line);
            let cols = header_cells.len();
            let align = if head == 2 {
                let sep = split_row(lines[i + 1]);
                (0..cols)
                    .map(|c| {
                        let s = sep.get(c).map(|s| s.trim()).unwrap_or("");
                        match (s.starts_with(':'), s.ends_with(':') && s.len() > 1) {
                            (true, true) => ColumnAlign::Center,
                            (false, true) => ColumnAlign::Right,
                            _ => ColumnAlign::Left,
                        }
                    })
                    .collect()
            } else {
                vec![ColumnAlign::Left; cols]
            };
            let header = header_cells.iter().map(|c| parse_inline(c)).collect();
            i += head;
            let mut rows = Vec::new();
            while i < n && lines[i].contains('|') && !lines[i].trim().is_empty() {
                let mut cells: Vec<Vec<Span>> = split_row(lines[i]).iter().map(|c| parse_inline(c)).collect();
                cells.resize(cols, Vec::new());
                rows.push(cells);
                i += 1;
            }
            blocks.push(Block::Table { header, align, rows });
            continue;
        }
        if list_marker(line).is_some() {
            let mut items: Vec<(ListItem, String)> = Vec::new();
            let mut stack: Vec<usize> = Vec::new();
            while i < n {
                let l = lines[i];
                if l.trim().is_empty() {
                    // a blank line continues the list only if more list content follows
                    let mut j = i + 1;
                    while j < n && lines[j].trim().is_empty() {
                        j += 1;
                    }
                    if j < n
                        && (list_marker(lines[j]).is_some()
                            || (indent_of(lines[j]) >= 2 && fence_open(lines[j]).is_none()))
                    {
                        i = j;
                        continue;
                    }
                    break;
                }
                if let Some((indent, number, rest)) = list_marker(l) {
                    if is_rule(l) {
                        break;
                    }
                    loop {
                        match stack.last() {
                            None => {
                                stack.push(indent);
                                break;
                            }
                            Some(&top) if indent > top + 1 => {
                                stack.push(indent);
                                break;
                            }
                            Some(&top) if indent + 1 >= top => break,
                            _ => {
                                stack.pop();
                            }
                        }
                    }
                    let (checked, rest) = task_marker(rest);
                    items.push((
                        ListItem { depth: stack.len() - 1, number, checked, spans: Vec::new() },
                        rest.to_owned(),
                    ));
                    i += 1;
                    continue;
                }
                // continuation line (lazy or indented) unless it starts another block
                if starts_block(lines, i) {
                    break;
                }
                if let Some((_, text)) = items.last_mut() {
                    text.push(' ');
                    text.push_str(l.trim());
                }
                i += 1;
            }
            let items = items
                .into_iter()
                .map(|(mut it, text)| {
                    it.spans = parse_inline(&text);
                    it
                })
                .collect();
            blocks.push(Block::List(items));
            continue;
        }
        // paragraph
        let mut text = String::new();
        while i < n && !lines[i].trim().is_empty() {
            if !text.is_empty() && starts_block(lines, i) {
                break;
            }
            let l = lines[i];
            if !text.is_empty() {
                let hard = text.ends_with("  ") || text.ends_with('\\');
                let trimmed = text.trim_end().trim_end_matches('\\').to_owned();
                text = trimmed;
                text.push(if hard { '\n' } else { ' ' });
            }
            text.push_str(l.trim_start());
            i += 1;
        }
        blocks.push(Block::Paragraph(parse_inline(text.trim_end())));
    }
    blocks
}

fn line_is_quote(l: &str) -> bool {
    indent_of(l) <= 3 && l.trim_start().starts_with('>')
}

fn task_marker(rest: &str) -> (Option<bool>, &str) {
    for (pat, v) in [("[ ] ", false), ("[x] ", true), ("[X] ", true)] {
        if let Some(r) = rest.strip_prefix(pat) {
            return (Some(v), r);
        }
    }
    for (pat, v) in [("[ ]", false), ("[x]", true), ("[X]", true)] {
        if rest == pat {
            return (Some(v), "");
        }
    }
    (None, rest)
}

struct InlineState {
    out: Vec<Span>,
    buf: String,
    style: SpanStyle,
}

impl InlineState {
    fn flush(&mut self) {
        if !self.buf.is_empty() {
            let text = std::mem::take(&mut self.buf);
            push_span(&mut self.out, Span { text, style: self.style, link: None });
        }
    }
}

fn push_span(out: &mut Vec<Span>, s: Span) {
    if s.text.is_empty() {
        return;
    }
    if let Some(last) = out.last_mut() {
        if last.style == s.style && last.link == s.link {
            last.text.push_str(&s.text);
            return;
        }
    }
    out.push(s);
}

fn run_len(chars: &[char], i: usize, c: char) -> usize {
    chars[i..].iter().take_while(|&&x| x == c).count()
}

/// Find the `]` matching the `[` at `open` (brackets may nest; code spans are skipped).
fn match_bracket(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 0;
    let mut i = open;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 1,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn is_url_char(c: char) -> bool {
    !c.is_whitespace() && !matches!(c, '<' | '>' | '"' | '`')
}

/// Parse inline Markdown into styled spans.
///
/// ```
/// use glowkit_ai::markdown::parse_inline;
/// let spans = parse_inline("a **b** `c`");
/// assert_eq!(spans.len(), 4);
/// assert!(spans[1].style.bold && spans[3].style.code);
/// ```
pub fn parse_inline(src: &str) -> Vec<Span> {
    let chars: Vec<char> = src.chars().collect();
    let n = chars.len();
    let mut st = InlineState { out: Vec::new(), buf: String::new(), style: SpanStyle::default() };
    let mut i = 0;
    while i < n {
        let c = chars[i];
        match c {
            '\\' if i + 1 < n && chars[i + 1].is_ascii_punctuation() => {
                st.buf.push(chars[i + 1]);
                i += 2;
            }
            '`' => {
                let run = run_len(&chars, i, '`');
                let mut j = i + run;
                let mut found = None;
                while j < n {
                    if chars[j] == '`' {
                        let r = run_len(&chars, j, '`');
                        if r == run {
                            found = Some(j);
                            break;
                        }
                        j += r;
                    } else {
                        j += 1;
                    }
                }
                let end = found.unwrap_or(n);
                let mut code: String = chars[i + run..end].iter().collect();
                if code.len() > 2 && code.starts_with(' ') && code.ends_with(' ') {
                    code = code[1..code.len() - 1].to_owned();
                }
                st.flush();
                push_span(&mut st.out, Span { text: code, style: SpanStyle { code: true, ..st.style }, link: None });
                i = found.map(|j| j + run).unwrap_or(n);
            }
            '*' | '_' => {
                let run = run_len(&chars, i, c);
                if run > 3 {
                    st.buf.extend(std::iter::repeat_n(c, run));
                    i += run;
                    continue;
                }
                let prev = if i > 0 { chars[i - 1] } else { ' ' };
                let at_end = i + run >= n;
                let next = chars.get(i + run).copied().unwrap_or(' ');
                let can_open = !next.is_whitespace() && (c == '*' || !prev.is_alphanumeric());
                let can_close = !prev.is_whitespace() && (c == '*' || !next.is_alphanumeric());
                let s = st.style;
                let closes = can_close
                    && match run {
                        1 => s.italic,
                        2 => s.bold,
                        _ => s.bold || s.italic,
                    };
                if closes {
                    st.flush();
                    match run {
                        1 => st.style.italic = false,
                        2 => st.style.bold = false,
                        _ => {
                            st.style.bold = false;
                            st.style.italic = false;
                        }
                    }
                } else if can_open {
                    st.flush();
                    match run {
                        1 => st.style.italic = true,
                        2 => st.style.bold = true,
                        _ => {
                            st.style.bold = true;
                            st.style.italic = true;
                        }
                    }
                } else if at_end {
                    // a dangling marker that just streamed in: hide it until its text arrives
                } else {
                    st.buf.extend(std::iter::repeat_n(c, run));
                }
                i += run;
            }
            '~' if i + 1 < n && chars[i + 1] == '~' => {
                let prev = if i > 0 { chars[i - 1] } else { ' ' };
                let next = chars.get(i + 2).copied().unwrap_or(' ');
                if st.style.strike && !prev.is_whitespace() {
                    st.flush();
                    st.style.strike = false;
                } else if !next.is_whitespace() {
                    st.flush();
                    st.style.strike = true;
                } else if i + 2 < n {
                    st.buf.push_str("~~");
                }
                i += 2;
            }
            '~' if i + 1 == n => i += 1,
            '[' => {
                let close = match_bracket(&chars, i);
                match close {
                    Some(cl) if chars.get(cl + 1) == Some(&'(') => {
                        // find the closing paren (allowing nested parens)
                        let mut depth = 0;
                        let mut j = cl + 1;
                        let mut end = None;
                        while j < n {
                            match chars[j] {
                                '(' => depth += 1,
                                ')' => {
                                    depth -= 1;
                                    if depth == 0 {
                                        end = Some(j);
                                        break;
                                    }
                                }
                                ' ' if depth == 1 => {}
                                _ => {}
                            }
                            j += 1;
                        }
                        let url_end = end.unwrap_or(n);
                        let raw: String = chars[cl + 2..url_end].iter().collect();
                        // drop an optional "title"
                        let url = raw.split_whitespace().next().unwrap_or("").to_owned();
                        let label: String = chars[i + 1..cl].iter().collect();
                        st.flush();
                        for mut s in parse_inline(&label) {
                            s.style.bold |= st.style.bold;
                            s.style.italic |= st.style.italic;
                            s.style.strike |= st.style.strike;
                            s.link = Some(url.clone());
                            push_span(&mut st.out, s);
                        }
                        i = end.map(|e| e + 1).unwrap_or(n);
                    }
                    _ => {
                        st.buf.push('[');
                        i += 1;
                    }
                }
            }
            '<' => {
                let rest: String = chars[i + 1..].iter().take_while(|&&x| x != '>' && !x.is_whitespace()).collect();
                let end = i + 1 + rest.chars().count();
                if (rest.starts_with("http://") || rest.starts_with("https://") || rest.starts_with("mailto:"))
                    && chars.get(end) == Some(&'>')
                {
                    st.flush();
                    push_span(&mut st.out, Span { text: rest.clone(), style: st.style, link: Some(rest) });
                    i = end + 1;
                } else {
                    st.buf.push('<');
                    i += 1;
                }
            }
            'h' if (i == 0 || chars[i - 1].is_whitespace() || chars[i - 1] == '(')
                && (starts_with(&chars, i, "https://") || starts_with(&chars, i, "http://")) =>
            {
                let mut j = i;
                while j < n && is_url_char(chars[j]) {
                    j += 1;
                }
                // trailing punctuation is not part of the url
                while j > i && matches!(chars[j - 1], '.' | ',' | ';' | ':' | '!' | '?' | ')' | '*' | '_') {
                    j -= 1;
                }
                let url: String = chars[i..j].iter().collect();
                st.flush();
                push_span(&mut st.out, Span { text: url.clone(), style: st.style, link: Some(url) });
                i = j;
            }
            _ => {
                st.buf.push(c);
                i += 1;
            }
        }
    }
    st.flush();
    if st.out.is_empty() && !src.is_empty() && st.buf.is_empty() {
        // everything was markup (e.g. a lone "**"): show nothing rather than garbage
        return Vec::new();
    }
    st.out
}

fn starts_with(chars: &[char], i: usize, pat: &str) -> bool {
    (i..).zip(pat.chars()).all(|(k, p)| chars.get(k) == Some(&p))
}

/// Concatenated visible text of spans.
pub fn spans_text(spans: &[Span]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

#[allow(dead_code)]
pub(crate) fn plain(text: &str) -> Vec<Span> {
    vec![Span::plain(text)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(spans: &[Span]) -> String {
        spans_text(spans)
    }

    #[test]
    fn headings_and_paragraphs() {
        let b = parse("# Title\n## Sub ##\nHello\nworld\n\nNext");
        assert_eq!(b.len(), 4);
        assert!(matches!(&b[0], Block::Heading { level: 1, spans } if text(spans) == "Title"));
        assert!(matches!(&b[1], Block::Heading { level: 2, spans } if text(spans) == "Sub"));
        assert!(matches!(&b[2], Block::Paragraph(s) if text(s) == "Hello world"));
        assert!(matches!(&b[3], Block::Paragraph(s) if text(s) == "Next"));
        assert!(matches!(&parse("#hashtag")[0], Block::Paragraph(_)));
    }

    #[test]
    fn emphasis() {
        let s = parse_inline("a **bold** and *it* and ***both*** ~~gone~~");
        assert_eq!(text(&s), "a bold and it and both gone");
        assert!(s.iter().any(|x| x.text == "bold" && x.style.bold && !x.style.italic));
        assert!(s.iter().any(|x| x.text == "it" && x.style.italic));
        assert!(s.iter().any(|x| x.text == "both" && x.style.bold && x.style.italic));
        assert!(s.iter().any(|x| x.text == "gone" && x.style.strike));
    }

    #[test]
    fn literal_stars_and_snake_case() {
        assert_eq!(text(&parse_inline("2 * 3 * 4")), "2 * 3 * 4");
        let s = parse_inline("call snake_case_name now");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].text, "call snake_case_name now");
        assert_eq!(text(&parse_inline(r"\*not\*")), "*not*");
    }

    #[test]
    fn streaming_unclosed_bold_and_code() {
        let s = parse_inline("this is **almost bo");
        assert_eq!(text(&s), "this is almost bo");
        assert!(s.last().unwrap().style.bold);
        // the opener alone at the end is hidden
        assert_eq!(text(&parse_inline("this is **")), "this is ");
        assert_eq!(text(&parse_inline("this is *")), "this is ");
        let c = parse_inline("run `cargo bu");
        assert!(c.last().unwrap().style.code);
        assert_eq!(c.last().unwrap().text, "cargo bu");
        assert!(parse_inline("**").is_empty());
    }

    #[test]
    fn inline_code_keeps_markers() {
        let s = parse_inline("use `**kwargs` here");
        assert_eq!(s[1].text, "**kwargs");
        assert!(s[1].style.code);
        let s = parse_inline("``a ` b``");
        assert_eq!(s[0].text, "a ` b");
    }

    #[test]
    fn links() {
        let s = parse_inline("see [the **docs**](https://x.dev \"t\") now");
        assert_eq!(text(&s), "see the docs now");
        assert!(s.iter().filter(|x| x.link.as_deref() == Some("https://x.dev")).count() == 2);
        assert!(s.iter().any(|x| x.text == "docs" && x.style.bold));
        // partial url while streaming still renders as a link
        let s = parse_inline("[docs](https://ex");
        assert_eq!(s[0].link.as_deref(), Some("https://ex"));
        // citations stay literal
        assert_eq!(text(&parse_inline("as shown [1].")), "as shown [1].");
        let s = parse_inline("visit https://example.com.");
        assert_eq!(s[1].link.as_deref(), Some("https://example.com"));
        assert_eq!(s[2].text, ".");
        let s = parse_inline("<https://a.b>");
        assert_eq!(s[0].link.as_deref(), Some("https://a.b"));
    }

    #[test]
    fn code_fences() {
        let b = parse("Intro\n```rust\nfn main() {}\n```\nAfter");
        assert_eq!(b.len(), 3);
        assert_eq!(b[1], Block::Code { lang: "rust".into(), code: "fn main() {}".into(), closed: true });
        // unclosed fence while streaming
        let b = parse("```py\nprint(1)\n``");
        assert_eq!(b, vec![Block::Code { lang: "py".into(), code: "print(1)".into(), closed: false }]);
        let b = parse("```");
        assert_eq!(b, vec![Block::Code { lang: String::new(), code: String::new(), closed: false }]);
        // markdown inside code is untouched
        let b = parse("~~~\n# not a heading\n**x**\n~~~");
        assert!(matches!(&b[0], Block::Code { code, .. } if code == "# not a heading\n**x**"));
    }

    #[test]
    fn lists() {
        let b = parse("- a\n- b\n  - nested\n    more text\n- [x] done\n- [ ] todo\n\n1. one\n2) two");
        let Block::List(items) = &b[0] else { panic!("{b:?}") };
        assert_eq!(items.len(), 7);
        assert_eq!(items[2].depth, 1);
        assert_eq!(text(&items[2].spans), "nested more text");
        assert_eq!(items[3].checked, Some(true));
        assert_eq!(items[4].checked, Some(false));
        // a blank line between the bullets and the numbers keeps them in one list block
        assert_eq!(items[6].number, Some(2));
        let b = parse("3. three\n4. four");
        let Block::List(items) = &b[0] else { panic!() };
        assert_eq!(items[0].number, Some(3));
    }

    #[test]
    fn rules_vs_lists() {
        assert_eq!(parse("---"), vec![Block::Rule]);
        assert_eq!(parse("* * *"), vec![Block::Rule]);
        assert!(matches!(&parse("- item")[0], Block::List(_)));
        let b = parse("text\n\n***\n\nmore");
        assert_eq!(b[1], Block::Rule);
    }

    #[test]
    fn quotes() {
        let b = parse("> quoted **text**\n> - item\nafter");
        let Block::Quote(inner) = &b[0] else { panic!() };
        assert!(matches!(&inner[0], Block::Paragraph(s) if text(s) == "quoted text"));
        assert!(matches!(&inner[1], Block::List(_)));
        assert!(matches!(&b[1], Block::Paragraph(_)));
    }

    #[test]
    fn tables() {
        let b = parse("| Name | Score |\n|:-----|------:|\n| a | 1 |\n| `b|c` | 2 |\n\nafter");
        let Block::Table { header, align, rows } = &b[0] else { panic!("{b:?}") };
        assert_eq!(header.len(), 2);
        assert_eq!(align, &vec![ColumnAlign::Left, ColumnAlign::Right]);
        assert_eq!(rows.len(), 2);
        assert_eq!(text(&rows[1][0]), "b|c");
        assert!(matches!(&b[1], Block::Paragraph(_)));
    }

    #[test]
    fn streaming_tables() {
        // header only so far
        let b = parse("| Name | Score |");
        assert!(matches!(&b[0], Block::Table { rows, .. } if rows.is_empty()));
        // half-written separator
        let b = parse("| Name | Score |\n|---");
        assert!(matches!(&b[0], Block::Table { .. }));
        // short rows are padded
        let b = parse("| a | b | c |\n|---|---|---|\n| 1 |");
        let Block::Table { rows, .. } = &b[0] else { panic!() };
        assert_eq!(rows[0].len(), 3);
    }

    #[test]
    fn every_prefix_parses() {
        let doc = "# Plan\n\nSome **bold** and `code`.\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n```rust\nfn x() {}\n```\n\n> quote\n\n1. one\n   - two\n\n---\n[link](https://x.y)";
        let chars: Vec<char> = doc.chars().collect();
        for k in 0..=chars.len() {
            let prefix: String = chars[..k].iter().collect();
            let blocks = parse(&prefix);
            // no markers may leak into visible text of paragraphs
            for b in &blocks {
                if let Block::Paragraph(s) = b {
                    let t = text(s);
                    assert!(!t.contains("**"), "leaked ** at {k}: {t:?}");
                }
            }
        }
    }
}
