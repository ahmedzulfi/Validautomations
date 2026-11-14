//! Tiny Rust syntax highlighter for code panels.

use glowkit::egui::{self, Color32, FontId};
use glowkit::Theme;

/// A tiny Rust syntax highlighter for the code panel.
pub fn highlight_rust(job: &mut egui::text::LayoutJob, code: &str, t: &Theme) {
    use glowkit::color::hex;
    let font = FontId::monospace(12.5);
    let kw = [
        "use", "let", "mut", "fn", "pub", "struct", "impl", "for", "in", "if", "else", "match", "return", "move",
        "self", "Self", "true", "false", "as", "const", "while", "loop",
    ];
    let (c_kw, c_str, c_com, c_num, c_ty, c_fn, c_txt) = if t.dark {
        (hex("#C792EA"), hex("#C3E88D"), hex("#636A80"), hex("#F78C6C"), hex("#FFCB6B"), hex("#82AAFF"), hex("#D4D4D8"))
    } else {
        (hex("#7C3AED"), hex("#15803D"), hex("#9CA3AF"), hex("#C2410C"), hex("#B45309"), hex("#1D4ED8"), hex("#27272A"))
    };
    let fmt = |s: &str, c: Color32, job: &mut egui::text::LayoutJob| {
        job.append(s, 0.0, egui::TextFormat { font_id: font.clone(), color: c, ..Default::default() });
    };
    for line in code.split_inclusive('\n') {
        let (body, comment) = match line.find("//") {
            Some(i) if !line[..i].contains('"') => (&line[..i], Some(&line[i..])),
            _ => (line, None),
        };
        let chars: Vec<char> = body.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '"' {
                let mut j = i + 1;
                while j < chars.len() && chars[j] != '"' {
                    if chars[j] == '\\' {
                        j += 1;
                    }
                    j += 1;
                }
                let s: String = chars[i..(j + 1).min(chars.len())].iter().collect();
                fmt(&s, c_str, job);
                i = j + 1;
            } else if c.is_alphabetic() || c == '_' {
                let mut j = i;
                while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                    j += 1;
                }
                let w: String = chars[i..j].iter().collect();
                let next = chars.get(j).copied();
                let col = if kw.contains(&w.as_str()) {
                    c_kw
                } else if w.chars().next().is_some_and(|c| c.is_uppercase()) {
                    c_ty
                } else if next == Some('(') || next == Some('!') {
                    c_fn
                } else {
                    c_txt
                };
                fmt(&w, col, job);
                i = j;
            } else if c.is_ascii_digit() {
                let mut j = i;
                while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '.' || chars[j] == '_') {
                    j += 1;
                }
                let w: String = chars[i..j].iter().collect();
                fmt(&w, c_num, job);
                i = j;
            } else {
                let mut j = i;
                while j < chars.len() && !(chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == '"') {
                    j += 1;
                }
                let w: String = chars[i..j].iter().collect();
                fmt(&w, c_txt, job);
                i = j;
            }
        }
        if let Some(cm) = comment {
            fmt(cm, c_com, job);
        }
    }
}

