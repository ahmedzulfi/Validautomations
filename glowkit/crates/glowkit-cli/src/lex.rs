//! A small, forgiving Rust tokenizer and top-level item splitter.
//!
//! It is not a full parser: it knows just enough Rust (comments, strings, raw strings, chars vs
//! lifetimes, brackets) to find item boundaries, identifiers and paths reliably.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Ident,
    Punct,
    Lit,
    Lifetime,
    /// `///` or `/** */` (outer doc comment, behaves like an attribute).
    OuterDoc,
    /// `//!` or `/*! */`.
    InnerDoc,
}

#[derive(Clone, Copy, Debug)]
pub struct Tok {
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
}

impl Tok {
    pub fn text<'a>(&self, src: &'a str) -> &'a str {
        &src[self.start..self.end]
    }
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

fn is_ident_cont(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

/// Tokenize `src`. Plain comments are dropped, doc comments are kept.
pub fn lex(src: &str) -> Vec<Tok> {
    let b = src.as_bytes();
    let n = b.len();
    let mut out = Vec::new();
    let mut i = 0;
    let ch = |i: usize| -> char { src[i..].chars().next().unwrap_or('\0') };
    while i < n {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        // comments
        if c == b'/' && i + 1 < n && b[i + 1] == b'/' {
            let s = i;
            while i < n && b[i] != b'\n' {
                i += 1;
            }
            let t = &src[s..i];
            if t.starts_with("///") && !t.starts_with("////") {
                out.push(Tok { kind: Kind::OuterDoc, start: s, end: i });
            } else if t.starts_with("//!") {
                out.push(Tok { kind: Kind::InnerDoc, start: s, end: i });
            }
            continue;
        }
        if c == b'/' && i + 1 < n && b[i + 1] == b'*' {
            let s = i;
            let mut depth = 0;
            while i < n {
                if b[i] == b'/' && i + 1 < n && b[i + 1] == b'*' {
                    depth += 1;
                    i += 2;
                } else if b[i] == b'*' && i + 1 < n && b[i + 1] == b'/' {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            let t = &src[s..i];
            if t.starts_with("/**") && !t.starts_with("/***") && t != "/**/" {
                out.push(Tok { kind: Kind::OuterDoc, start: s, end: i });
            } else if t.starts_with("/*!") {
                out.push(Tok { kind: Kind::InnerDoc, start: s, end: i });
            }
            continue;
        }
        // raw strings / byte strings / c strings
        if c == b'r' || c == b'b' || c == b'c' {
            let raw_at = if c == b'r' {
                Some(i + 1)
            } else if i + 1 < n && b[i + 1] == b'r' {
                Some(i + 2)
            } else {
                None
            };
            if let Some(j) = raw_at {
                let mut k = j;
                while k < n && b[k] == b'#' {
                    k += 1;
                }
                if k < n && b[k] == b'"' {
                    let hashes = k - j;
                    let mut e = k + 1;
                    while e < n {
                        if b[e] == b'"'
                            && e + hashes < n + 1
                            && b[e + 1..(e + 1 + hashes).min(n)].iter().filter(|&&x| x == b'#').count() == hashes
                        {
                            e += 1 + hashes;
                            break;
                        }
                        e += 1;
                    }
                    let e = e.min(n);
                    out.push(Tok { kind: Kind::Lit, start: i, end: e });
                    i = e;
                    continue;
                }
            } else if i + 1 < n && b[i + 1] == b'"' {
                let e = skip_string(b, i + 1);
                out.push(Tok { kind: Kind::Lit, start: i, end: e });
                i = e;
                continue;
            } else if c == b'b' && i + 1 < n && b[i + 1] == b'\'' {
                if let Some(e) = skip_char(src, i + 1) {
                    out.push(Tok { kind: Kind::Lit, start: i, end: e });
                    i = e;
                    continue;
                }
            }
        }
        if c == b'"' {
            let e = skip_string(b, i);
            out.push(Tok { kind: Kind::Lit, start: i, end: e });
            i = e;
            continue;
        }
        if c == b'\'' {
            if let Some(e) = skip_char(src, i) {
                out.push(Tok { kind: Kind::Lit, start: i, end: e });
                i = e;
                continue;
            }
            // lifetime / label
            let s = i;
            i += 1;
            while i < n && is_ident_cont(ch(i)) {
                i += ch(i).len_utf8();
            }
            out.push(Tok { kind: Kind::Lifetime, start: s, end: i });
            continue;
        }
        if c.is_ascii_digit() {
            let s = i;
            while i < n {
                let d = b[i];
                let more = d.is_ascii_alphanumeric()
                    || d == b'_'
                    || (d == b'.' && i + 1 < n && b[i + 1].is_ascii_digit())
                    || ((d == b'+' || d == b'-')
                        && (b[i - 1] == b'e' || b[i - 1] == b'E')
                        && !src[s..i].starts_with("0x"));
                if !more {
                    break;
                }
                i += 1;
            }
            out.push(Tok { kind: Kind::Lit, start: s, end: i });
            continue;
        }
        let cc = ch(i);
        if is_ident_start(cc) {
            let s = i;
            if cc == 'r' && i + 2 < n && b[i + 1] == b'#' && is_ident_start(ch(i + 2)) {
                i += 2;
            }
            while i < n && is_ident_cont(ch(i)) {
                i += ch(i).len_utf8();
            }
            out.push(Tok { kind: Kind::Ident, start: s, end: i });
            continue;
        }
        if c == b':' && i + 1 < n && b[i + 1] == b':' {
            out.push(Tok { kind: Kind::Punct, start: i, end: i + 2 });
            i += 2;
            continue;
        }
        let l = cc.len_utf8();
        out.push(Tok { kind: Kind::Punct, start: i, end: i + l });
        i += l;
    }
    out
}

fn skip_string(b: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'"' => return i + 1,
            _ => i += 1,
        }
    }
    b.len()
}

/// If a char literal starts at `i` (a `'`), return its end.
fn skip_char(src: &str, i: usize) -> Option<usize> {
    let b = src.as_bytes();
    let rest = &src[i + 1..];
    let mut it = rest.char_indices();
    let (_, c1) = it.next()?;
    if c1 == '\\' {
        // escape: find closing quote
        let mut j = i + 2;
        while j < b.len() && j < i + 14 {
            if b[j] == b'\'' && j > i + 2 {
                return Some(j + 1);
            }
            j += 1;
        }
        return None;
    }
    if c1 == '\'' {
        return None;
    }
    let (k, c2) = it.next()?;
    if c2 == '\'' {
        return Some(i + 1 + k + 1);
    }
    None
}

/// Visibility of an item.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Vis {
    Private,
    /// `pub(crate)`, `pub(super)`, `pub(in ..)`
    Restricted,
    Pub,
}

#[derive(Clone, Debug)]
pub struct Item {
    /// Index of the first token (including attributes / doc comments).
    pub first: usize,
    /// Index of the first token after attributes (visibility or keyword).
    pub head: usize,
    /// One past the last token.
    pub last: usize,
    pub vis: Vis,
    /// fn, struct, enum, union, trait, type, const, static, mod, use, impl, macro_rules, macro, extern, inner_attr, other
    pub kind: &'static str,
    pub name: Option<String>,
    /// Attribute token ranges `[first, last)` (outer attributes only, not doc comments).
    pub attrs: Vec<(usize, usize)>,
}

fn is_open(s: &str) -> bool {
    matches!(s, "(" | "[" | "{")
}
fn is_close(s: &str) -> bool {
    matches!(s, ")" | "]" | "}")
}

/// Index of the token after the bracket group opened at `i`.
pub fn skip_group(toks: &[Tok], src: &str, i: usize) -> usize {
    let mut depth = 0i32;
    let mut j = i;
    while j < toks.len() {
        let t = toks[j];
        if t.kind == Kind::Punct {
            let s = t.text(src);
            if is_open(s) {
                depth += 1;
            } else if is_close(s) {
                depth -= 1;
                if depth == 0 {
                    return j + 1;
                }
            }
        }
        j += 1;
    }
    toks.len()
}

/// Split tokens `[from, to)` into items (at bracket depth 0 relative to `from`).
pub fn items(toks: &[Tok], src: &str, from: usize, to: usize) -> Vec<Item> {
    let mut out = Vec::new();
    let mut i = from;
    let txt = |k: usize| -> &str {
        if k < to {
            toks[k].text(src)
        } else {
            ""
        }
    };
    while i < to {
        let first = i;
        let mut attrs = Vec::new();
        // inner attributes / inner docs as standalone items
        if toks[i].kind == Kind::InnerDoc {
            out.push(Item { first, head: i, last: i + 1, vis: Vis::Private, kind: "inner_attr", name: None, attrs });
            i += 1;
            continue;
        }
        if txt(i) == "#" && txt(i + 1) == "!" && txt(i + 2) == "[" {
            let e = skip_group(toks, src, i + 2);
            out.push(Item { first, head: i, last: e, vis: Vis::Private, kind: "inner_attr", name: None, attrs });
            i = e;
            continue;
        }
        // outer attributes and docs
        loop {
            if i < to && toks[i].kind == Kind::OuterDoc {
                i += 1;
            } else if txt(i) == "#" && txt(i + 1) == "[" {
                let e = skip_group(toks, src, i + 1);
                attrs.push((i, e));
                i = e;
            } else {
                break;
            }
        }
        if i >= to {
            if first < to {
                out.push(Item { first, head: to, last: to, vis: Vis::Private, kind: "other", name: None, attrs });
            }
            break;
        }
        if txt(i) == ";" {
            out.push(Item { first, head: i, last: i + 1, vis: Vis::Private, kind: "other", name: None, attrs });
            i += 1;
            continue;
        }
        let head = i;
        let mut vis = Vis::Private;
        if txt(i) == "pub" {
            vis = Vis::Pub;
            i += 1;
            if txt(i) == "(" {
                vis = Vis::Restricted;
                i = skip_group(toks, src, i);
            }
        }
        // qualifiers
        let mut kind: &'static str = "other";
        let mut name = None;
        loop {
            match txt(i) {
                "unsafe" | "async" | "default" | "auto" => i += 1,
                "const" if matches!(txt(i + 1), "fn" | "unsafe" | "async" | "extern") => i += 1,
                "extern" if txt(i + 1) != "crate" => {
                    i += 1;
                    if i < to && toks[i].kind == Kind::Lit {
                        i += 1;
                    }
                    if txt(i) == "{" {
                        kind = "extern";
                        break;
                    }
                }
                _ => break,
            }
        }
        if kind == "other" {
            let kw = txt(i);
            kind = match kw {
                "fn" => "fn",
                "struct" => "struct",
                "enum" => "enum",
                "union" if toks.get(i + 1).map(|t| t.kind) == Some(Kind::Ident) => "union",
                "trait" => "trait",
                "type" => "type",
                "const" => "const",
                "static" => "static",
                "mod" => "mod",
                "use" => "use",
                "impl" => "impl",
                "extern" => "extern",
                "macro_rules" if txt(i + 1) == "!" => "macro_rules",
                _ if toks[i].kind == Kind::Ident && txt(i + 1) == "!" => "macro",
                _ => "other",
            };
            match kind {
                "fn" | "struct" | "enum" | "union" | "trait" | "type" | "mod" => {
                    name = Some(txt(i + 1).trim_start_matches("r#").to_string());
                }
                "const" | "static" => {
                    let mut k = i + 1;
                    if txt(k) == "mut" {
                        k += 1;
                    }
                    name = Some(txt(k).to_string());
                }
                "macro_rules" => name = Some(txt(i + 2).to_string()),
                "macro" => name = Some(txt(i).to_string()),
                _ => {}
            }
        }
        let last = item_end(toks, src, i, to, kind);
        out.push(Item { first, head, last, vis, kind, name, attrs });
        i = last;
    }
    out
}

/// End (exclusive token index) of an item whose keyword starts at `i`.
fn item_end(toks: &[Tok], src: &str, i: usize, to: usize, kind: &str) -> usize {
    let semicolon_only = matches!(kind, "use" | "const" | "static" | "type" | "other");
    let mut depth = 0i32;
    let mut j = i;
    while j < to {
        let t = toks[j];
        if t.kind == Kind::Punct {
            let s = t.text(src);
            if is_open(s) {
                depth += 1;
            } else if is_close(s) {
                depth -= 1;
                if depth < 0 {
                    return j;
                }
                if depth == 0 && s == "}" && !semicolon_only {
                    // `foo! { }` / fn body / struct body; a trailing `;` belongs to it
                    if j + 1 < to && toks[j + 1].text(src) == ";" && kind == "macro" {
                        return j + 2;
                    }
                    return j + 1;
                }
            } else if s == ";" && depth == 0 {
                return j + 1;
            }
        }
        j += 1;
    }
    to
}

/// The self type name of an `impl` item (last path segment before generics), if simple.
pub fn impl_self_type(toks: &[Tok], src: &str, it: &Item) -> Option<String> {
    let mut j = it.head;
    while j < it.last && toks[j].text(src) != "impl" {
        j += 1;
    }
    j += 1;
    if toks.get(j).map(|t| t.text(src)) == Some("<") {
        j = skip_angle(toks, src, j);
    }
    // find `for` at angle depth 0 before `{`
    let mut start = j;
    let mut k = j;
    let mut ad = 0i32;
    while k < it.last {
        let s = toks[k].text(src);
        match s {
            "<" => ad += 1,
            ">" => ad -= 1,
            "for" if ad == 0 => start = k + 1,
            "{" | "where" if ad == 0 => break,
            _ => {}
        }
        k += 1;
    }
    let mut name = None;
    let mut m = start;
    let mut ad = 0i32;
    while m < k {
        let t = toks[m];
        let s = t.text(src);
        match s {
            "<" => ad += 1,
            ">" => ad -= 1,
            _ if ad == 0 && t.kind == Kind::Ident && s != "dyn" && s != "mut" => name = Some(s.to_string()),
            _ => {}
        }
        m += 1;
    }
    name
}

fn skip_angle(toks: &[Tok], src: &str, i: usize) -> usize {
    let mut d = 0;
    let mut j = i;
    while j < toks.len() {
        match toks[j].text(src) {
            "<" => d += 1,
            ">" => {
                d -= 1;
                if d == 0 {
                    return j + 1;
                }
            }
            _ => {}
        }
        j += 1;
    }
    j
}

/// A flattened `use` path: segments (globs end with `*`) and an optional `as` alias.
pub type UsePath = (Vec<String>, Option<String>);

/// Flatten a `use` tree into paths. `toks[from..to]` is the tree after `use` (without the `;`).
pub fn use_paths(toks: &[Tok], src: &str, from: usize, to: usize) -> Vec<UsePath> {
    let mut out = Vec::new();
    let mut prefix: Vec<String> = Vec::new();
    walk_use(toks, src, from, to, &mut prefix, &mut out);
    out
}

fn walk_use(toks: &[Tok], src: &str, from: usize, to: usize, prefix: &mut Vec<String>, out: &mut Vec<UsePath>) {
    // split by commas at depth 0
    let mut parts = Vec::new();
    let mut d = 0;
    let mut s = from;
    for (j, t) in toks.iter().enumerate().take(to).skip(from) {
        match t.text(src) {
            "{" => d += 1,
            "}" => d -= 1,
            "," if d == 0 => {
                parts.push((s, j));
                s = j + 1;
            }
            _ => {}
        }
    }
    parts.push((s, to));
    for (a, b) in parts {
        if a >= b {
            continue;
        }
        let base = prefix.len();
        let mut j = a;
        let mut done = false;
        let mut alias = None;
        while j < b {
            let t = toks[j].text(src);
            match t {
                "::" => {}
                "{" => {
                    let e = skip_group(toks, src, j);
                    walk_use(toks, src, j + 1, e - 1, prefix, out);
                    done = true;
                    break;
                }
                "as" => {
                    j += 1;
                    alias = toks.get(j).map(|t| t.text(src).to_string());
                }
                _ => prefix.push(t.to_string()),
            }
            j += 1;
        }
        if !done {
            out.push((prefix.clone(), alias));
        }
        prefix.truncate(base);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_tricky_things() {
        let src = r####"fn a<'a>(x: &'a str) -> char { let c = 'x'; let s = r#"he"llo"#; let e = '\n'; b'q'; "q\"uote" } // hi"####;
        let t = lex(src);
        let idents: Vec<&str> = t.iter().filter(|t| t.kind == Kind::Ident).map(|t| t.text(src)).collect();
        assert_eq!(idents, vec!["fn", "a", "x", "str", "char", "let", "c", "let", "s", "let", "e"]);
    }

    #[test]
    fn splits_items() {
        let src = "#[cfg(feature = \"demos\")]\nmod demos;\n/// doc\npub struct A { x: u8 }\nimpl<T> Foo for Bar<T> { }\npub(crate) fn f() {}\nconst X: [u8; 2] = [1, 2];\nfoo! { x }\n";
        let t = lex(src);
        let it = items(&t, src, 0, t.len());
        let kinds: Vec<&str> = it.iter().map(|i| i.kind).collect();
        assert_eq!(kinds, vec!["mod", "struct", "impl", "fn", "const", "macro"]);
        assert_eq!(impl_self_type(&t, src, &it[2]).as_deref(), Some("Bar"));
        assert_eq!(it[3].vis, Vis::Restricted);
    }

    #[test]
    fn use_trees() {
        let src = "a::{b, c::{d as e, *}}, f";
        let t = lex(src);
        let p = use_paths(&t, src, 0, t.len());
        assert_eq!(p.len(), 4);
        assert_eq!(p[2].0, vec!["a", "c", "*"]);
        assert_eq!(p[1].1.as_deref(), Some("e"));
    }
}
