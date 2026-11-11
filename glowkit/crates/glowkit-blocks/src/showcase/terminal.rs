use super::util::check;
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::{anim::smoothstep, color::with_alpha, hex, paint, Clock, Theme};

#[derive(Clone, Debug, PartialEq)]
enum LineKind {
    Command,
    Output,
    Spinner { done: String },
}

/// One line of a [`Terminal`] script.
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalLine {
    kind: LineKind,
    text: String,
    color: Option<Color32>,
    prefix: Option<String>,
}

impl TerminalLine {
    /// A command typed out after the prompt, character by character.
    pub fn command(text: impl Into<String>) -> Self {
        Self { kind: LineKind::Command, text: text.into(), color: None, prefix: None }
    }
    /// A line of output that fades in after the previous line.
    pub fn output(text: impl Into<String>) -> Self {
        Self { kind: LineKind::Output, text: text.into(), color: None, prefix: None }
    }
    /// A green "✔ text" success line.
    pub fn success(text: impl Into<String>) -> Self {
        Self { kind: LineKind::Output, text: text.into(), color: None, prefix: Some("✔".into()) }.with_success()
    }
    /// A spinner line: shows a spinner with `text`, then turns into a ✔ with `done`.
    pub fn spinner(text: impl Into<String>, done: impl Into<String>) -> Self {
        Self { kind: LineKind::Spinner { done: done.into() }, text: text.into(), color: None, prefix: None }
    }
    /// Text color for this line (defaults depend on the kind).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    fn with_success(mut self) -> Self {
        self.color = Some(Color32::PLACEHOLDER);
        self
    }
    fn duration(&self, type_speed: f32) -> f32 {
        match self.kind {
            LineKind::Command => self.text.chars().count() as f32 * type_speed + 0.45,
            LineKind::Output => 0.22,
            LineKind::Spinner { .. } => 1.4,
        }
    }
}

/// A macOS-style terminal window that types out commands and then reveals their output line by
/// line, with spinners that resolve into check marks. Loops by default.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_blocks::{Terminal, TerminalLine};
/// ui.add(Terminal::new()
///     .line(TerminalLine::command("cargo add glowkit"))
///     .line(TerminalLine::spinner("Resolving dependencies", "Resolved 42 packages"))
///     .line(TerminalLine::success("Added glowkit v0.1.0")));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Terminal {
    lines: Vec<TerminalLine>,
    size: egui::Vec2,
    title: String,
    prompt: String,
    type_speed: f32,
    looping: bool,
    pause: f32,
    font_size: f32,
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new()
    }
}

impl Terminal {
    /// An empty terminal; add lines with [`Terminal::line`].
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            size: vec2(520.0, 320.0),
            title: "zsh".into(),
            prompt: "❯".into(),
            type_speed: 0.045,
            looping: true,
            pause: 2.5,
            font_size: 13.0,
        }
    }
    /// Append a line to the script.
    pub fn line(mut self, line: TerminalLine) -> Self {
        self.lines.push(line);
        self
    }
    /// Replace the whole script.
    pub fn lines(mut self, lines: impl IntoIterator<Item = TerminalLine>) -> Self {
        self.lines = lines.into_iter().collect();
        self
    }
    /// Window size in points.
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.size = vec2(w, h);
        self
    }
    /// Title shown in the window chrome.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
    /// Prompt glyph shown before commands.
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }
    /// Seconds per typed character.
    pub fn type_speed(mut self, seconds_per_char: f32) -> Self {
        self.type_speed = seconds_per_char.max(0.001);
        self
    }
    /// Restart the script after it finishes (with `pause` seconds in between).
    pub fn looping(mut self, looping: bool, pause: f32) -> Self {
        self.looping = looping;
        self.pause = pause.max(0.0);
        self
    }
    /// Monospace font size.
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }
}

impl Widget for Terminal {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(self.size, Sense::click());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let total: f32 = self.lines.iter().map(|l| l.duration(self.type_speed)).sum::<f32>() + 0.3;
        let mut el = Clock::elapsed(ui, resp.id);
        if resp.clicked() {
            Clock::restart(ui, resp.id);
            el = 0.0;
        }
        if self.looping {
            el %= total + self.pause;
            glowkit_core::animate(ui);
        } else if el < total + 0.5 {
            glowkit_core::animate(ui);
        }
        let time = glowkit_core::time(ui);
        let p = ui.painter_at(rect.expand(30.0));
        let r = t.radius.max(8.0);

        // window
        let win_bg = if t.dark { t.card } else { hex("#FFFFFF") };
        paint::soft_shadow(&p, rect, r, 28.0, vec2(0.0, 10.0), Color32::from_black_alpha(if t.dark { 120 } else { 30 }));
        p.rect_filled(rect, r, win_bg);
        let bar = Rect::from_min_size(rect.min, vec2(rect.width(), 36.0));
        p.rect_filled(bar, egui::CornerRadius { nw: r as u8, ne: r as u8, sw: 0, se: 0 }, t.muted.gamma_multiply(if t.dark { 0.6 } else { 0.7 }));
        p.line_segment([bar.left_bottom(), bar.right_bottom()], Stroke::new(1.0, t.border));
        for (i, c) in [hex("#FF5F57"), hex("#FEBC2E"), hex("#28C840")].into_iter().enumerate() {
            let cc = pos2(bar.min.x + 18.0 + i as f32 * 20.0, bar.center().y);
            p.circle_filled(cc, 6.0, c);
            p.circle_stroke(cc, 6.0, Stroke::new(0.6, Color32::from_black_alpha(30)));
        }
        p.text(bar.center(), Align2::CENTER_CENTER, &self.title, FontId::proportional(12.5), t.muted_foreground);
        p.rect_stroke(rect, r, Stroke::new(1.0, t.border), StrokeKind::Inside);

        // body
        let body = Rect::from_min_max(pos2(rect.min.x + 18.0, bar.max.y + 14.0), pos2(rect.max.x - 18.0, rect.max.y - 12.0));
        let font = FontId::monospace(self.font_size);
        let lh = self.font_size * 1.65;
        let fg = t.foreground;
        let muted = t.muted_foreground;
        let prompt_col = t.success;
        let accent = t.accent;

        // figure out progress per line
        let mut start = 0.0;
        let mut rows: Vec<(usize, f32)> = Vec::new(); // (line index, local progress seconds)
        for (i, l) in self.lines.iter().enumerate() {
            let d = l.duration(self.type_speed);
            if el >= start {
                rows.push((i, el - start));
            }
            start += d;
        }
        let shown = rows.len() as f32;
        // auto-scroll when content overflows
        let capacity = (body.height() / lh).floor().max(1.0);
        let scroll = ((shown - capacity).max(0.0)) * lh;
        let clip = p.with_clip_rect(body.intersect(p.clip_rect()));
        let mut y = body.min.y - scroll;
        let cursor_on = (time * 2.0).fract() < 0.55;
        for (i, local) in rows.iter().copied() {
            let l = &self.lines[i];
            let is_last = i + 1 == rows.len();
            let x0 = body.min.x;
            match &l.kind {
                LineKind::Command => {
                    let n = l.text.chars().count();
                    let typed = ((local / self.type_speed) as usize).min(n);
                    let txt: String = l.text.chars().take(typed).collect();
                    let pr = clip.text(pos2(x0, y), Align2::LEFT_TOP, &self.prompt, font.clone(), prompt_col);
                    let tx = pr.max.x + 8.0;
                    // first word (the program) gets the accent color
                    let (head, tail) = match txt.find(' ') {
                        Some(k) => (&txt[..k], &txt[k..]),
                        None => (txt.as_str(), ""),
                    };
                    let hr = clip.text(pos2(tx, y), Align2::LEFT_TOP, head, font.clone(), l.color.unwrap_or(accent));
                    let tr = clip.text(pos2(hr.max.x, y), Align2::LEFT_TOP, tail, font.clone(), fg);
                    let typing = typed < n || is_last;
                    if typing && (cursor_on || typed < n) {
                        let cx = if txt.is_empty() { tx } else { tr.max.x.max(hr.max.x) + 1.0 };
                        clip.rect_filled(Rect::from_min_size(pos2(cx, y + 1.0), vec2(self.font_size * 0.6, self.font_size * 1.2)), 1.0, with_alpha(fg, 0.75));
                    }
                }
                LineKind::Output => {
                    let k = smoothstep(0.0, 0.22, local);
                    let yy = y + (1.0 - k) * 6.0;
                    let mut x = x0;
                    let col = match l.color {
                        Some(c) if c == Color32::PLACEHOLDER => t.success,
                        Some(c) => c,
                        None => muted,
                    };
                    if let Some(pre) = &l.prefix {
                        let pr = clip.text(pos2(x, yy), Align2::LEFT_TOP, pre, font.clone(), col.gamma_multiply(k));
                        x = pr.max.x + 8.0;
                    }
                    let txt_col = if l.color == Some(Color32::PLACEHOLDER) { fg } else { col };
                    clip.text(pos2(x, yy), Align2::LEFT_TOP, &l.text, font.clone(), txt_col.gamma_multiply(k));
                }
                LineKind::Spinner { done } => {
                    let d = 1.2;
                    let c = pos2(x0 + self.font_size * 0.45, y + lh * 0.38);
                    if local < d {
                        // braille-like dotted spinner
                        let s = self.font_size * 0.42;
                        for j in 0..8 {
                            let a = j as f32 / 8.0 * std::f32::consts::TAU;
                            let head = (time * 10.0) as i32;
                            let age = ((head - j).rem_euclid(8)) as f32 / 8.0;
                            clip.circle_filled(c + vec2(a.cos(), a.sin()) * s, 1.3, with_alpha(accent, 1.0 - age * 0.85));
                        }
                        clip.text(pos2(x0 + self.font_size * 1.4, y), Align2::LEFT_TOP, &l.text, font.clone(), muted);
                        let dots = ".".repeat(((time * 3.0) as usize) % 4);
                        let w = clip.layout_no_wrap(l.text.clone(), font.clone(), muted).size().x;
                        clip.text(pos2(x0 + self.font_size * 1.4 + w, y), Align2::LEFT_TOP, dots, font.clone(), muted);
                    } else {
                        let k = smoothstep(d, d + 0.25, local);
                        check(&clip, c, self.font_size * 0.9, Stroke::new(1.8, t.success), k);
                        clip.text(pos2(x0 + self.font_size * 1.4, y), Align2::LEFT_TOP, done, font.clone(), fg.gamma_multiply(0.4 + 0.6 * k));
                    }
                }
            }
            y += lh;
        }
        // idle prompt after the script ends
        if el >= total - 0.3 && rows.len() == self.lines.len() {
            let pr = clip.text(pos2(body.min.x, y), Align2::LEFT_TOP, &self.prompt, font.clone(), prompt_col);
            if cursor_on {
                clip.rect_filled(Rect::from_min_size(pos2(pr.max.x + 8.0, y + 1.0), vec2(self.font_size * 0.6, self.font_size * 1.2)), 1.0, with_alpha(fg, 0.75));
            }
        }
        resp
    }
}
