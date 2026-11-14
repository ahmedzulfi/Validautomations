//! Code block with a header (language label + copy button) and lightweight syntax highlighting.

use crate::common::{self, Fx};
use crate::icons::{paint_icon, Icon};
use egui::text::LayoutJob;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke, StrokeKind, TextFormat, Ui,
    Widget,
};
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::Theme;

/// Token classes produced by [`highlight`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// Plain text and punctuation.
    Text,
    /// Language keyword.
    Keyword,
    /// String or char literal.
    String,
    /// Comment.
    Comment,
    /// Numeric literal.
    Number,
    /// Identifier followed by `(`.
    Function,
    /// Capitalized identifier (types, classes).
    Type,
    /// Rust macro (`name!`) or Python decorator (`@name`).
    Macro,
}

const RUST_KW: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self",
    "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
];
const JS_KW: &[&str] = &[
    "async",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "default",
    "delete",
    "do",
    "else",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "from",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "interface",
    "let",
    "new",
    "null",
    "of",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "type",
    "typeof",
    "undefined",
    "var",
    "void",
    "while",
    "yield",
];
const PY_KW: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del",
    "elif", "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda", "nonlocal",
    "not", "or", "pass", "raise", "return", "try", "while", "with", "yield", "self",
];
const SH_KW: &[&str] = &[
    "if", "then", "else", "fi", "for", "do", "done", "while", "case", "esac", "function", "export", "echo", "cd",
    "sudo",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lang {
    Rust,
    Js,
    Py,
    Shell,
    Other,
}

fn lang_of(name: &str) -> Lang {
    match name.to_ascii_lowercase().as_str() {
        "rust" | "rs" => Lang::Rust,
        "js" | "javascript" | "ts" | "typescript" | "jsx" | "tsx" | "json" => Lang::Js,
        "py" | "python" => Lang::Py,
        "sh" | "bash" | "shell" | "zsh" | "console" => Lang::Shell,
        _ => Lang::Other,
    }
}

/// Split `code` into highlighted tokens (byte ranges) for `lang` (`rust`, `js`/`ts`, `py`, `sh`;
/// anything else gets strings, numbers and comments only).
pub fn highlight(code: &str, lang: &str) -> Vec<(std::ops::Range<usize>, TokenKind)> {
    let lang = lang_of(lang);
    let kws: &[&str] = match lang {
        Lang::Rust => RUST_KW,
        Lang::Js => JS_KW,
        Lang::Py => PY_KW,
        Lang::Shell => SH_KW,
        Lang::Other => &[],
    };
    let line_comment: &[&str] = match lang {
        Lang::Rust | Lang::Js => &["//"],
        Lang::Py | Lang::Shell => &["#"],
        Lang::Other => &["//", "#"],
    };
    let b = code.as_bytes();
    let n = b.len();
    let mut out = Vec::new();
    let mut i = 0;
    let mut text_start = 0;
    let push = |out: &mut Vec<_>, text_start: &mut usize, s: usize, e: usize, k: TokenKind| {
        if *text_start < s {
            out.push((*text_start..s, TokenKind::Text));
        }
        out.push((s..e, k));
        *text_start = e;
    };
    while i < n {
        let c = b[i];
        // comments
        if line_comment.iter().any(|lc| code[i..].starts_with(lc))
            && !(lang == Lang::Other && c == b'#' && i > 0 && b[i - 1] != b'\n' && b[i - 1] != b' ')
        {
            let e = code[i..].find('\n').map(|k| i + k).unwrap_or(n);
            push(&mut out, &mut text_start, i, e, TokenKind::Comment);
            i = e;
            continue;
        }
        if (lang == Lang::Rust || lang == Lang::Js) && code[i..].starts_with("/*") {
            let e = code[i + 2..].find("*/").map(|k| i + 2 + k + 2).unwrap_or(n);
            push(&mut out, &mut text_start, i, e, TokenKind::Comment);
            i = e;
            continue;
        }
        // strings
        let quote = c == b'"' || c == b'`' && lang == Lang::Js || c == b'\'' && lang != Lang::Rust;
        let rust_char = lang == Lang::Rust && c == b'\'' && i + 2 < n && (b[i + 2] == b'\'' || b[i + 1] == b'\\');
        if quote || rust_char {
            let mut j = i + 1;
            while j < n && b[j] != c {
                if b[j] == b'\\' {
                    j += 1;
                }
                if b[j.min(n - 1)] == b'\n' && c != b'`' && lang != Lang::Py {
                    break;
                }
                j += 1;
            }
            let e = (j + 1).min(n);
            push(&mut out, &mut text_start, i, e, TokenKind::String);
            i = e;
            continue;
        }
        if c.is_ascii_digit() && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_')) {
            let mut j = i;
            while j < n && (b[j].is_ascii_alphanumeric() || b[j] == b'.' || b[j] == b'_') {
                if b[j] == b'.' && !(j + 1 < n && b[j + 1].is_ascii_digit()) {
                    break;
                }
                j += 1;
            }
            push(&mut out, &mut text_start, i, j, TokenKind::Number);
            i = j;
            continue;
        }
        if lang == Lang::Py && c == b'@' {
            let mut j = i + 1;
            while j < n && (b[j].is_ascii_alphanumeric() || b[j] == b'_' || b[j] == b'.') {
                j += 1;
            }
            push(&mut out, &mut text_start, i, j, TokenKind::Macro);
            i = j;
            continue;
        }
        if c.is_ascii_alphabetic() || c == b'_' {
            let mut j = i;
            while j < n && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
                j += 1;
            }
            let word = &code[i..j];
            let kind = if kws.contains(&word) {
                Some(TokenKind::Keyword)
            } else if lang == Lang::Rust && j < n && b[j] == b'!' {
                Some(TokenKind::Macro)
            } else if lang != Lang::Other && j < n && b[j] == b'(' {
                Some(TokenKind::Function)
            } else if lang != Lang::Other
                && lang != Lang::Shell
                && word.as_bytes()[0].is_ascii_uppercase()
                && word.len() > 1
            {
                Some(TokenKind::Type)
            } else {
                None
            };
            if let Some(k) = kind {
                let e = if k == TokenKind::Macro { j + 1 } else { j };
                push(&mut out, &mut text_start, i, e, k);
                i = e;
            } else {
                i = j;
            }
            continue;
        }
        // advance one UTF-8 char
        i += code[i..].chars().next().map(|ch| ch.len_utf8()).unwrap_or(1);
    }
    if text_start < n {
        out.push((text_start..n, TokenKind::Text));
    }
    out
}

/// Colors used for syntax highlighting, tuned per theme brightness.
pub(crate) fn token_color(k: TokenKind, t: &Theme) -> Color32 {
    let d = t.dark;
    match k {
        TokenKind::Text => with_alpha(t.foreground, if d { 0.88 } else { 0.9 }),
        TokenKind::Keyword => {
            if d {
                hex("#C4A5FF")
            } else {
                hex("#7C3AED")
            }
        }
        TokenKind::String => {
            if d {
                hex("#9AE6B4")
            } else {
                hex("#15803D")
            }
        }
        TokenKind::Comment => {
            if d {
                hex("#6B6B76")
            } else {
                hex("#A1A1AA")
            }
        }
        TokenKind::Number => {
            if d {
                hex("#FDBA74")
            } else {
                hex("#C2410C")
            }
        }
        TokenKind::Function => {
            if d {
                hex("#93C5FD")
            } else {
                hex("#1D4ED8")
            }
        }
        TokenKind::Type => {
            if d {
                hex("#7DD3FC")
            } else {
                hex("#0E7490")
            }
        }
        TokenKind::Macro => {
            if d {
                hex("#F9A8D4")
            } else {
                hex("#BE185D")
            }
        }
    }
}

/// Background of code surfaces.
pub(crate) fn code_bg(t: &Theme) -> Color32 {
    if t.dark {
        mix(t.background, t.card, 0.55)
    } else {
        mix(t.muted, t.card, 0.45)
    }
}

/// Build a highlighted layout job.
pub(crate) fn code_job(code: &str, lang: &str, size: f32, t: &Theme, highlight_on: bool) -> LayoutJob {
    let mut job = LayoutJob::default();
    let font = FontId::monospace(size);
    let lh = Some((size * 1.6).round());
    if highlight_on {
        for (r, k) in highlight(code, lang) {
            job.append(
                &code[r],
                0.0,
                TextFormat { font_id: font.clone(), color: token_color(k, t), line_height: lh, ..Default::default() },
            );
        }
    } else {
        job.append(
            code,
            0.0,
            TextFormat { font_id: font, color: token_color(TokenKind::Text, t), line_height: lh, ..Default::default() },
        );
    }
    job.wrap.max_width = f32::INFINITY;
    job
}

/// A fenced code block: a header with the language and a copy button, then the highlighted
/// code in a horizontally scrolling body. Used by [`crate::Markdown`] for ```` ``` ```` blocks.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_ai::CodeBlock::new("fn main() {\n    println!(\"hi\");\n}").language("rust"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CodeBlock<'a> {
    code: &'a str,
    lang: &'a str,
    size: f32,
    line_numbers: bool,
    highlight: bool,
    header: bool,
    fade: Option<(usize, usize)>,
}

impl<'a> CodeBlock<'a> {
    /// A code block showing `code`.
    pub fn new(code: &'a str) -> Self {
        Self { code, lang: "", size: 12.5, line_numbers: false, highlight: true, header: true, fade: None }
    }
    /// Language name shown in the header and used for highlighting (`rust`, `ts`, `py`, ...).
    pub fn language(mut self, lang: &'a str) -> Self {
        self.lang = lang;
        self
    }
    /// Font size of the code.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Show a line-number gutter.
    pub fn line_numbers(mut self, on: bool) -> Self {
        self.line_numbers = on;
        self
    }
    /// Enable keyword highlighting (default on).
    pub fn highlight(mut self, on: bool) -> Self {
        self.highlight = on;
        self
    }
    /// Show the header bar with language and copy button (default on).
    pub fn header(mut self, on: bool) -> Self {
        self.header = on;
        self
    }
    /// Fade in the last `tail` of `total` characters (used for streaming reveals).
    pub(crate) fn fade(mut self, total: usize, tail: usize) -> Self {
        self.fade = Some((total, tail));
        self
    }
}

impl Widget for CodeBlock<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let width = ui.available_width();
        let header_h = if self.header { 36.0 } else { 0.0 };
        let job = code_job(self.code, self.lang, self.size, &t, self.highlight);
        let galley = ui.painter().layout_job(job);
        let lines = self.code.lines().count().max(1);
        let gutter = if self.line_numbers { (lines.to_string().len() as f32) * self.size * 0.62 + 18.0 } else { 0.0 };
        let pad = 14.0;
        let body_h = galley.size().y.max(self.size * 1.6) + pad * 2.0 - 2.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(width, header_h + body_h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let cr = t.corner_scaled(0.9);
        let painter = ui.painter();
        painter.rect_filled(rect, cr, code_bg(&t));
        painter.rect_stroke(rect, cr, Stroke::new(1.0, t.border), StrokeKind::Inside);
        if self.header {
            let hr = Rect::from_min_size(rect.min, vec2(width, header_h));
            painter.line_segment(
                [pos2(hr.min.x + 1.0, hr.max.y), pos2(hr.max.x - 1.0, hr.max.y)],
                Stroke::new(1.0, with_alpha(t.border, 0.8)),
            );
            let label = if self.lang.is_empty() { "text" } else { self.lang };
            common::mono(
                painter,
                pos2(hr.min.x + 14.0, hr.center().y),
                Align2::LEFT_CENTER,
                label.to_lowercase(),
                11.5,
                t.muted_foreground,
            );
            // copy button
            let id = resp.id.with("copy");
            let now = ui.input(|i| i.time);
            let copied_at: f64 = glowkit_core::state(ui, id, || -100.0);
            let copied = now - copied_at < 1.6;
            let label = if copied { "Copied" } else { "Copy" };
            let g = common::galley(ui, label, 12.0, t.muted_foreground);
            let bw = g.size().x + 34.0;
            let br = Rect::from_min_size(pos2(hr.max.x - bw - 6.0, hr.center().y - 13.0), vec2(bw, 26.0));
            let bresp = ui.interact(br, id.with("btn"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            let h = common::anim_bool(ui, id.with("h"), bresp.hovered(), 0.12);
            let painter = ui.painter();
            if h > 0.0 {
                painter.rect_filled(br, CornerRadius::same(6), common::hover_fill(&t).gamma_multiply(h));
            }
            let col = if copied { t.success } else { t.muted_foreground.lerp_to_gamma(t.foreground, h) };
            paint_icon(
                painter,
                if copied { Icon::Check } else { Icon::Copy },
                Rect::from_center_size(pos2(br.min.x + 15.0, br.center().y), vec2(13.0, 13.0)),
                col,
            );
            common::text(painter, pos2(br.min.x + 26.0, br.center().y), Align2::LEFT_CENTER, label, 12.0, col);
            if bresp.clicked() {
                ui.ctx().copy_text(self.code.to_owned());
                glowkit_core::set_state(ui, id, now);
            }
            if copied {
                ui.ctx().request_repaint();
            }
        }
        // body
        let body = Rect::from_min_max(pos2(rect.min.x, rect.min.y + header_h), rect.max);
        let inner = body.shrink2(vec2(pad, pad - 1.0));
        if self.line_numbers {
            let painter = ui.painter();
            for (k, row) in galley.rows.iter().enumerate().take(lines) {
                let _ = row;
                let y = inner.min.y + k as f32 * (self.size * 1.6).round();
                common::mono(
                    painter,
                    pos2(inner.min.x + gutter - 14.0, y + (self.size * 1.6).round() / 2.0),
                    Align2::RIGHT_CENTER,
                    k + 1,
                    self.size * 0.92,
                    with_alpha(t.muted_foreground, 0.55),
                );
            }
        }
        let code_rect = Rect::from_min_max(pos2(inner.min.x + gutter, inner.min.y), pos2(body.max.x, body.max.y));
        let overflow = galley.size().x > code_rect.width() - pad;
        let mut child =
            ui.new_child(egui::UiBuilder::new().max_rect(code_rect).layout(egui::Layout::top_down(egui::Align::Min)));
        child.set_clip_rect(code_rect.intersect(ui.clip_rect()));
        let fade = self.fade;
        let paint = move |ui: &mut Ui| {
            let (r, _) = ui.allocate_exact_size(galley.size() + vec2(pad, 0.0), Sense::hover());
            match fade {
                Some((total, tail)) if tail > 0 => common::paint_galley_fx(ui.painter(), r.min, &galley, |i| Fx {
                    alpha: ((total.saturating_sub(i)) as f32 / tail as f32).clamp(0.0, 1.0),
                    ..Default::default()
                }),
                _ => ui.painter().galley(r.min, galley.clone(), t.foreground),
            }
        };
        if overflow {
            egui::ScrollArea::horizontal()
                .id_salt(resp.id.with("scroll"))
                .auto_shrink([false, true])
                .show(&mut child, paint);
        } else {
            paint(&mut child);
        }
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_rust() {
        let code = "fn main() { let x = 42; println!(\"hi\"); } // done";
        let toks = highlight(code, "rust");
        let kinds: Vec<(&str, TokenKind)> =
            toks.iter().map(|(r, k)| (&code[r.clone()], *k)).filter(|(_, k)| *k != TokenKind::Text).collect();
        assert!(kinds.contains(&("fn", TokenKind::Keyword)));
        assert!(kinds.contains(&("main", TokenKind::Function)));
        assert!(kinds.contains(&("42", TokenKind::Number)));
        assert!(kinds.contains(&("println!", TokenKind::Macro)));
        assert!(kinds.contains(&("\"hi\"", TokenKind::String)));
        assert!(kinds.contains(&("// done", TokenKind::Comment)));
        // tokens cover the input exactly
        let joined: String = toks.iter().map(|(r, _)| &code[r.clone()]).collect();
        assert_eq!(joined, code);
    }

    #[test]
    fn highlights_python_and_js() {
        let code = "@cache\ndef f(x):  # hi\n    return 'a'";
        let toks = highlight(code, "py");
        assert!(toks.iter().any(|(r, k)| &code[r.clone()] == "@cache" && *k == TokenKind::Macro));
        assert!(toks.iter().any(|(r, k)| &code[r.clone()] == "def" && *k == TokenKind::Keyword));
        assert!(toks.iter().any(|(r, k)| &code[r.clone()] == "'a'" && *k == TokenKind::String));
        let code = "const a = `t${1}`; /* c */";
        let toks = highlight(code, "ts");
        assert!(toks.iter().any(|(r, k)| &code[r.clone()] == "/* c */" && *k == TokenKind::Comment));
        let joined: String = toks.iter().map(|(r, _)| &code[r.clone()]).collect();
        assert_eq!(joined, code);
        // unterminated string / unicode never panics
        let _ = highlight("let s = \"héllo", "rust");
        let _ = highlight("'", "py");
    }
}
