use super::util::{bold_text, check, idle_time};
use egui::text::{LayoutJob, TextFormat};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget};
use glowkit_core::{anim::smoothstep, color::with_alpha, hex, paint, Theme};

/// Syntax colors for the tiny highlighter: (keyword, string, comment, number, function, type, text).
fn syntax(t: &Theme) -> [Color32; 7] {
    if t.dark {
        [hex("#C792EA"), hex("#C3E88D"), hex("#676E95"), hex("#F78C6C"), hex("#82AAFF"), hex("#FFCB6B"), hex("#D4D4D8")]
    } else {
        [hex("#8B2FC9"), hex("#2E7D32"), hex("#9CA3AF"), hex("#C2410C"), hex("#1D4ED8"), hex("#B45309"), hex("#27272A")]
    }
}

const KEYWORDS: &[&str] = &[
    "use", "let", "mut", "fn", "pub", "struct", "impl", "for", "in", "if", "else", "match", "return", "move", "self", "Self", "true", "false", "as",
    "const", "while", "loop", "async", "await", "import", "from", "export", "default", "function", "var", "new", "class", "type", "interface",
];

/// Highlight one line of code into a layout job.
fn highlight(line: &str, font: &FontId, colors: &[Color32; 7], alpha: f32) -> LayoutJob {
    let [kw, st, com, num, func, ty, txt] = colors.map(|c| with_alpha(c, alpha));
    let mut job = LayoutJob::default();
    let fmt = |c: Color32| TextFormat { font_id: font.clone(), color: c, ..Default::default() };
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            let s: String = chars[i..].iter().collect();
            job.append(&s, 0.0, fmt(com));
            break;
        }
        if c == '"' || c == '\'' || c == '`' {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != c {
                j += 1;
            }
            let s: String = chars[i..(j + 1).min(chars.len())].iter().collect();
            job.append(&s, 0.0, fmt(st));
            i = j + 1;
            continue;
        }
        if c.is_ascii_digit() {
            let mut j = i;
            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '.' || chars[j] == '_') {
                j += 1;
            }
            let s: String = chars[i..j].iter().collect();
            job.append(&s, 0.0, fmt(num));
            i = j;
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let mut j = i;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let s: String = chars[i..j].iter().collect();
            let col = if KEYWORDS.contains(&s.as_str()) {
                kw
            } else if chars.get(j) == Some(&'(') || chars.get(j) == Some(&'!') {
                func
            } else if s.chars().next().is_some_and(|c| c.is_uppercase()) {
                ty
            } else {
                txt
            };
            job.append(&s, 0.0, fmt(col));
            i = j;
            continue;
        }
        let mut j = i;
        while j < chars.len() && !(chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == '"' || chars[j] == '\'' || chars[j] == '`' || (chars[j] == '/' && chars.get(j + 1) == Some(&'/'))) {
            j += 1;
        }
        let s: String = chars[i..j.max(i + 1)].iter().collect();
        job.append(&s, 0.0, fmt(txt));
        i = j.max(i + 1);
    }
    job
}

#[derive(Clone, Copy, Debug)]
struct CopyState {
    tab: usize,
    copied_at: f64,
    last_auto: f64,
}

/// An install-command box with package-manager tabs (e.g. cargo / npm / pnpm / bun) and a copy
/// button whose clipboard icon morphs into a check mark with a "Copied!" bubble.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_blocks::ScriptCopyBox::new(&[("cargo", "cargo add glowkit"), ("npm", "npm i glowkit")]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ScriptCopyBox {
    commands: Vec<(String, String)>,
    width: f32,
    autoplay: bool,
}

impl ScriptCopyBox {
    /// Tabs as `(label, command)` pairs.
    pub fn new(commands: &[(&str, &str)]) -> Self {
        Self { commands: commands.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(), width: 460.0, autoplay: false }
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// Cycle tabs and show the copy animation when idle (for demos).
    pub fn autoplay(mut self, b: bool) -> Self {
        self.autoplay = b;
        self
    }
}

fn clipboard_icon(p: &egui::Painter, c: egui::Pos2, s: f32, col: Color32, bg: Color32, a: f32) {
    let st = Stroke::new(1.5, with_alpha(col, a));
    let back = Rect::from_center_size(c + vec2(-s * 0.15, -s * 0.15), vec2(s * 0.6, s * 0.7));
    let front = Rect::from_center_size(c + vec2(s * 0.15, s * 0.15), vec2(s * 0.6, s * 0.7));
    p.rect_stroke(back, 2.0, st, StrokeKind::Middle);
    p.rect_filled(front, 2.0, with_alpha(bg, a));
    p.rect_stroke(front, 2.0, st, StrokeKind::Middle);
}

impl Widget for ScriptCopyBox {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let h = 104.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(self.width, h), Sense::hover());
        let id = resp.id;
        let now = ui.input(|i| i.time);
        let n = self.commands.len().max(1);
        let mut st: CopyState = glowkit_core::state(ui, id.with("st"), || CopyState { tab: 0, copied_at: -10.0, last_auto: now });
        let idle = idle_time(ui, id, resp.hovered());
        if self.autoplay && idle > 1.5 {
            glowkit_core::animate(ui);
            let since = now - st.last_auto;
            if since > 1.2 && now - st.copied_at > 1.2 && st.copied_at < st.last_auto {
                st.copied_at = now;
            }
            if since > 3.0 {
                st.tab = (st.tab + 1) % n;
                st.last_auto = now;
            }
        }
        if !ui.is_rect_visible(rect) {
            glowkit_core::set_state(ui, id.with("st"), st);
            return resp;
        }
        let p = ui.painter_at(rect.expand(40.0));
        let bg = if t.dark { hex("#0D0D10") } else { hex("#FAFAFA") };
        paint::soft_shadow(&p, rect, 12.0, 18.0, vec2(0.0, 6.0), Color32::from_black_alpha(if t.dark { 90 } else { 18 }));
        p.rect_filled(rect, 12.0, bg);
        p.rect_stroke(rect, 12.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
        // tabs
        let head = Rect::from_min_size(rect.min, vec2(rect.width(), 44.0));
        p.line_segment([head.left_bottom(), head.right_bottom()], Stroke::new(1.0, t.border));
        let font = FontId::proportional(13.0);
        let mut x = head.min.x + 12.0;
        let mut tab_rects = Vec::new();
        for (i, (label, _)) in self.commands.iter().enumerate() {
            let g = p.layout_no_wrap(label.clone(), font.clone(), t.foreground);
            let r = Rect::from_min_size(pos2(x, head.center().y - 13.0), vec2(g.size().x + 22.0, 26.0));
            tab_rects.push(r);
            let tr = ui.interact(r, id.with(("tab", i)), Sense::click());
            if tr.clicked() {
                st.tab = i;
                st.last_auto = now;
            }
            if tr.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            x = r.max.x + 4.0;
        }
        let sel = tab_rects.get(st.tab).copied().unwrap_or(head);
        let ix = glowkit_core::anim::spring(ui, id.with("ix"), sel.min.x, 260.0, 24.0);
        let iw = glowkit_core::anim::spring(ui, id.with("iw"), sel.width(), 260.0, 24.0);
        let ind = Rect::from_min_size(pos2(ix, sel.min.y), vec2(iw, sel.height()));
        p.rect_filled(ind, 7.0, if t.dark { hex("#1F1F24") } else { Color32::WHITE });
        p.rect_stroke(ind, 7.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
        // underline accent on the selected tab
        p.line_segment([pos2(ind.min.x + 8.0, head.max.y - 0.5), pos2(ind.max.x - 8.0, head.max.y - 0.5)], Stroke::new(2.0, t.accent));
        for (i, (label, _)) in self.commands.iter().enumerate() {
            let on = i == st.tab;
            p.text(tab_rects[i].center(), Align2::CENTER_CENTER, label, font.clone(), if on { t.foreground } else { t.muted_foreground });
        }
        // traffic-like dots on the right
        for i in 0..3 {
            p.circle_filled(pos2(head.max.x - 18.0 - i as f32 * 12.0, head.center().y), 3.0, with_alpha(t.muted_foreground, 0.35));
        }
        // command
        let cmd = self.commands.get(st.tab).map(|c| c.1.clone()).unwrap_or_default();
        let body_y = (head.max.y + rect.max.y) / 2.0;
        let mono = FontId::monospace(14.0);
        let cols = syntax(&t);
        let pr = p.text(pos2(rect.min.x + 18.0, body_y), Align2::LEFT_CENTER, "$", mono.clone(), t.muted_foreground);
        let mut job = LayoutJob::default();
        for (i, w) in cmd.split(' ').enumerate() {
            let c = match i {
                0 => cols[4],
                1 => cols[0],
                _ if w.starts_with('-') => cols[3],
                _ => t.foreground,
            };
            if i > 0 {
                job.append(" ", 0.0, TextFormat { font_id: mono.clone(), color: t.foreground, ..Default::default() });
            }
            job.append(w, 0.0, TextFormat { font_id: mono.clone(), color: c, ..Default::default() });
        }
        let g = ui.fonts_mut(|f| f.layout_job(job));
        p.galley(pos2(pr.max.x + 10.0, body_y - g.size().y / 2.0), g, t.foreground);
        // copy button
        let bc = pos2(rect.max.x - 30.0, body_y);
        let br = Rect::from_center_size(bc, Vec2::splat(34.0));
        let b = ui.interact(br, id.with("copy"), Sense::click());
        if b.clicked() {
            ui.ctx().copy_text(cmd.clone());
            st.copied_at = now;
        }
        let ce = (now - st.copied_at) as f32;
        let copied = ce < 1.6;
        if copied {
            glowkit_core::animate(ui);
        }
        let hv = glowkit_core::anim::follow(ui, id.with("bh"), if b.hovered() { 1.0 } else { 0.0 }, 14.0);
        p.rect_filled(br, 8.0, with_alpha(t.foreground, 0.04 + 0.06 * hv));
        let k = if copied { smoothstep(0.0, 0.18, ce) * (1.0 - smoothstep(1.4, 1.6, ce)) } else { 0.0 };
        clipboard_icon(&p, bc, 18.0 * (1.0 - 0.4 * k), t.muted_foreground.lerp_to_gamma(t.foreground, hv), bg, 1.0 - k);
        if k > 0.0 {
            p.circle_filled(bc, 11.0 * k, with_alpha(t.success, 0.18 * k));
            check(&p, bc, 15.0, Stroke::new(2.0, t.success), smoothstep(0.05, 0.3, ce));
            // bubble
            let bub = Rect::from_center_size(bc - vec2(0.0, 34.0 + 6.0 * k), vec2(64.0, 24.0));
            p.rect_filled(bub, 6.0, with_alpha(t.foreground, k));
            p.text(bub.center(), Align2::CENTER_CENTER, "Copied!", FontId::proportional(12.0), with_alpha(t.background, k));
        }
        if b.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        glowkit_core::set_state(ui, id.with("st"), st);
        resp
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    None,
    Removed,
    Added,
    Focus,
}

fn parse(code: &str) -> Vec<(String, Mark)> {
    code.lines()
        .map(|l| {
            for (tag, m) in [("// [!code --]", Mark::Removed), ("// [!code ++]", Mark::Added), ("// [!code focus]", Mark::Focus)] {
                if let Some(s) = l.strip_suffix(tag) {
                    return (s.trim_end().to_owned(), m);
                }
            }
            (l.to_owned(), Mark::None)
        })
        .collect()
}

/// Side-by-side "before" and "after" code panes. Lines annotated with `// [!code --]` /
/// `// [!code ++]` get removed / added highlights; with `// [!code focus]`, other lines are
/// dimmed and blurred until you hover the pane.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_blocks::CodeComparison::new(
///     "let x = 1; // [!code --]",
///     "let x = 2; // [!code ++]",
/// ).filename("main.rs"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CodeComparison {
    before: String,
    after: String,
    filename: String,
    size: Option<Vec2>,
    font_size: f32,
}

impl CodeComparison {
    /// Compare `before` with `after` (annotations are stripped from the display).
    pub fn new(before: impl Into<String>, after: impl Into<String>) -> Self {
        Self { before: before.into(), after: after.into(), filename: "main.rs".into(), size: None, font_size: 12.5 }
    }
    /// File name shown in both pane headers.
    pub fn filename(mut self, s: impl Into<String>) -> Self {
        self.filename = s.into();
        self
    }
    /// Size (defaults to available width x a height that fits the code).
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.size = Some(vec2(w, h));
        self
    }
    /// Code font size.
    pub fn font_size(mut self, s: f32) -> Self {
        self.font_size = s;
        self
    }
}

impl Widget for CodeComparison {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let before = parse(&self.before);
        let after = parse(&self.after);
        let lh = self.font_size * 1.7;
        let lines = before.len().max(after.len()) as f32;
        let size = self.size.unwrap_or(vec2(ui.available_width(), 46.0 + 20.0 + lines * lh));
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let p = ui.painter_at(rect.expand(20.0));
        let bg = if t.dark { hex("#0D0D10") } else { hex("#FFFFFF") };
        paint::soft_shadow(&p, rect, 12.0, 20.0, vec2(0.0, 6.0), Color32::from_black_alpha(if t.dark { 90 } else { 18 }));
        p.rect_filled(rect, 12.0, bg);
        let mid = rect.center().x;
        let colors = syntax(&t);
        let font = FontId::monospace(self.font_size);
        let pointer = ui.input(|i| i.pointer.hover_pos());
        for (side, code) in [(0, &before), (1, &after)] {
            let pane = if side == 0 { Rect::from_min_max(rect.min, pos2(mid, rect.max.y)) } else { Rect::from_min_max(pos2(mid, rect.min.y), rect.max) };
            let head = Rect::from_min_size(pane.min, vec2(pane.width(), 42.0));
            let cr = if side == 0 { CornerRadius { nw: 12, ne: 0, sw: 0, se: 0 } } else { CornerRadius { nw: 0, ne: 12, sw: 0, se: 0 } };
            p.rect_filled(head, cr, if t.dark { hex("#141418") } else { hex("#F7F7F8") });
            p.line_segment([head.left_bottom(), head.right_bottom()], Stroke::new(1.0, t.border));
            p.text(pos2(head.min.x + 16.0, head.center().y), Align2::LEFT_CENTER, &self.filename, FontId::proportional(12.5), t.muted_foreground);
            let (badge, bc) = if side == 0 { ("before", t.destructive) } else { ("after", t.success) };
            let bg_r = Rect::from_min_size(pos2(head.max.x - 16.0 - 54.0, head.center().y - 10.0), vec2(54.0, 20.0));
            p.rect_filled(bg_r, 99.0, with_alpha(bc, 0.14));
            p.text(bg_r.center(), Align2::CENTER_CENTER, badge, FontId::proportional(11.0), bc);
            let hovered = pointer.is_some_and(|q| pane.contains(q));
            let hk = glowkit_core::anim::follow(ui, resp.id.with(("hv", side)), if hovered { 1.0 } else { 0.0 }, 8.0);
            let has_focus = code.iter().any(|l| l.1 == Mark::Focus);
            let body = Rect::from_min_max(pos2(pane.min.x, head.max.y + 10.0), pane.max);
            for (i, (line, mark)) in code.iter().enumerate() {
                let y = body.min.y + i as f32 * lh;
                let row = Rect::from_min_size(pos2(pane.min.x, y), vec2(pane.width(), lh));
                let (row_bg, sign) = match mark {
                    Mark::Removed => (Some(t.destructive), "-"),
                    Mark::Added => (Some(t.success), "+"),
                    _ => (None, ""),
                };
                if let Some(c) = row_bg {
                    p.rect_filled(row, 0.0, with_alpha(c, 0.12));
                    p.line_segment([row.left_top() + vec2(1.0, 0.0), row.left_bottom() + vec2(1.0, 0.0)], Stroke::new(2.0, c));
                }
                let dim = has_focus && *mark != Mark::Focus;
                let alpha = if dim { 0.35 + 0.65 * hk } else { 1.0 };
                p.text(pos2(pane.min.x + 30.0, row.center().y), Align2::RIGHT_CENTER, format!("{}", i + 1), font.clone(), with_alpha(t.muted_foreground, 0.5 * alpha));
                if !sign.is_empty() {
                    p.text(pos2(pane.min.x + 40.0, row.center().y), Align2::LEFT_CENTER, sign, font.clone(), row_bg.unwrap_or(t.foreground));
                }
                let job = highlight(line, &font, &colors, 1.0);
                let g = ui.fonts_mut(|f| f.layout_job(job));
                let pos = pos2(pane.min.x + 54.0, row.center().y - g.size().y / 2.0);
                if dim && hk < 0.99 {
                    let blur = (1.0 - hk) * 1.6;
                    // dimmed + blurred, colors kept
                    for (dx, dy) in [(-blur, 0.0), (blur, 0.0), (0.0, -blur), (0.0, blur)] {
                        p.galley_with_override_text_color(pos + vec2(dx, dy), g.clone(), with_alpha(t.muted_foreground, 0.12 * (1.0 - hk)));
                    }
                    let job = highlight(line, &font, &colors, alpha);
                    let g2 = ui.fonts_mut(|f| f.layout_job(job));
                    p.galley(pos, g2, t.foreground);
                } else {
                    p.galley(pos, g, t.foreground);
                }
            }
        }
        p.line_segment([pos2(mid, rect.min.y), pos2(mid, rect.max.y)], Stroke::new(1.0, t.border));
        // VS badge
        let vc = pos2(mid, rect.center().y);
        p.circle_filled(vc, 17.0, if t.dark { hex("#18181B") } else { Color32::WHITE });
        p.circle_stroke(vc, 17.0, Stroke::new(1.0, t.border));
        bold_text(&p, vc, Align2::CENTER_CENTER, "VS", FontId::proportional(11.5), t.muted_foreground);
        p.rect_stroke(rect, 12.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
        resp
    }
}

/// A dismissible announcement banner with a gradient background. Closing it slides it up and
/// collapses its height so the content below moves up smoothly.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut open = true;
/// ui.add(glowkit_blocks::StickyBanner::new(&mut open, "glowkit 1.0 is out!").link("Read the announcement"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct StickyBanner<'a> {
    open: &'a mut bool,
    text: String,
    link: String,
    colors: Option<[Color32; 3]>,
    height: f32,
}

impl<'a> StickyBanner<'a> {
    /// A banner shown while `open` is true.
    pub fn new(open: &'a mut bool, text: impl Into<String>) -> Self {
        Self { open, text: text.into(), link: String::new(), colors: None, height: 44.0 }
    }
    /// Underlined link text after the message (with an arrow).
    pub fn link(mut self, s: impl Into<String>) -> Self {
        self.link = s.into();
        self
    }
    /// Gradient stops (defaults to the theme gradient).
    pub fn colors(mut self, a: Color32, b: Color32, c: Color32) -> Self {
        self.colors = Some([a, b, c]);
        self
    }
    /// Banner height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
}

impl Widget for StickyBanner<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("sticky_banner");
        let k = glowkit_core::anim::spring(ui, id, if *self.open { 1.0 } else { 0.0 }, 200.0, 26.0).clamp(0.0, 1.0);
        let w = ui.available_width();
        let (rect, mut resp) = ui.allocate_exact_size(vec2(w, self.height * k), Sense::hover());
        if k <= 0.001 || !ui.is_rect_visible(rect) {
            return resp;
        }
        let full = Rect::from_min_size(pos2(rect.min.x, rect.max.y - self.height), vec2(w, self.height));
        let p = ui.painter_at(rect);
        let stops = self.colors.unwrap_or([t.gradient[0], t.gradient[1], t.gradient[2]]);
        let time = glowkit_core::time(ui);
        glowkit_core::animate(ui);
        // flowing gradient
        paint::shader_grid(&p, full, 24, 1, |uv| with_alpha(glowkit_core::color::gradient_cyclic(&stops, uv.x * 0.6 - time * 0.08), k));
        paint::glow_ellipse(&p, full.center(), vec2(w * 0.4, self.height), with_alpha(Color32::WHITE, 0.12 * k));
        let ink = Color32::WHITE;
        let g = p.layout_no_wrap(self.text.clone(), FontId::proportional(14.0), ink);
        let lg = p.layout_no_wrap(if self.link.is_empty() { String::new() } else { format!("{} →", self.link) }, FontId::proportional(14.0), ink);
        let total = g.size().x + if self.link.is_empty() { 0.0 } else { 14.0 + lg.size().x };
        let x0 = full.center().x - total / 2.0;
        let ty = full.center().y - g.size().y / 2.0;
        p.galley(pos2(x0 + 0.4, ty), g.clone(), with_alpha(ink, k));
        p.galley(pos2(x0, ty), g.clone(), with_alpha(ink, k));
        if !self.link.is_empty() {
            let lx = x0 + g.size().x + 14.0;
            let lr = Rect::from_min_size(pos2(lx, ty), lg.size());
            let l = ui.interact(lr, id.with("link"), Sense::click());
            let lift = if l.hovered() { 2.0 } else { 0.0 };
            p.galley(pos2(lx + lift, ty), lg.clone(), with_alpha(ink, k));
            p.line_segment([pos2(lx, lr.max.y), pos2(lr.max.x + lift, lr.max.y)], Stroke::new(1.0, with_alpha(ink, 0.7 * k)));
            if l.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if l.clicked() {
                resp = l;
            }
        }
        // close button
        let cc = pos2(full.max.x - 26.0, full.center().y);
        let cr = Rect::from_center_size(cc, Vec2::splat(26.0));
        let c = ui.interact(cr, id.with("close"), Sense::click());
        let hv = glowkit_core::anim::follow(ui, id.with("chv"), if c.hovered() { 1.0 } else { 0.0 }, 14.0);
        p.circle_filled(cc, 13.0, with_alpha(Color32::WHITE, 0.12 + 0.12 * hv));
        let s = 4.5;
        let st = Stroke::new(1.6, with_alpha(ink, k));
        p.line_segment([cc + vec2(-s, -s), cc + vec2(s, s)], st);
        p.line_segment([cc + vec2(s, -s), cc + vec2(-s, s)], st);
        if c.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if c.clicked() {
            *self.open = false;
            resp.mark_changed();
        }
        resp
    }
}
