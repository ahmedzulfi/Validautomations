use crate::inputs::kit::{self, UiIcon};
use egui::text::{LayoutJob, TextFormat};
use egui::{
    pos2, vec2, Color32, CornerRadius, CursorIcon, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget,
};
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::Theme;

/// One event in a [`Timeline`].
#[derive(Clone, Debug, Default)]
pub struct TimelineItem {
    /// Event title.
    pub title: String,
    /// Time / date label.
    pub time: String,
    /// Muted body text.
    pub description: String,
    /// Optional icon inside the marker.
    pub icon: Option<UiIcon>,
}

impl TimelineItem {
    /// An event with title, time and description.
    pub fn new(title: impl Into<String>, time: impl Into<String>, description: impl Into<String>) -> Self {
        Self { title: title.into(), time: time.into(), description: description.into(), icon: None }
    }
    /// Icon inside the marker.
    pub fn icon(mut self, i: UiIcon) -> Self {
        self.icon = Some(i);
        self
    }
}

/// A vertical timeline whose connector fills up to the active event (animated), with completed
/// markers filled, the active one pulsing and upcoming ones hollow.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::{Timeline, TimelineItem};
/// ui.add(Timeline::new(vec![
///     TimelineItem::new("Order placed", "Mar 3", "We received your order."),
///     TimelineItem::new("Shipped", "Mar 4", "Your package is on its way."),
/// ]).active(1));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Timeline {
    items: Vec<TimelineItem>,
    active: usize,
    width: Option<f32>,
}

impl Timeline {
    /// A timeline of events (top to bottom).
    pub fn new(items: Vec<TimelineItem>) -> Self {
        Self { items, active: 0, width: None }
    }
    /// Index of the current event; earlier events render as completed.
    pub fn active(mut self, i: usize) -> Self {
        self.active = i;
        self
    }
    /// Width (default: available, at most 480).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
}

impl Widget for Timeline {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = self.width.unwrap_or_else(|| ui.available_width().min(480.0));
        let marker = 26.0;
        let text_x = marker + 16.0;
        let tw = w - text_x;
        let p0 = ui.painter().clone();
        let layouts: Vec<_> = self
            .items
            .iter()
            .map(|it| {
                let tg = p0.layout_no_wrap(it.title.clone(), FontId::proportional(14.0), t.foreground);
                let dg = p0.layout(it.description.clone(), FontId::proportional(13.0), t.muted_foreground, tw);
                (tg, dg)
            })
            .collect();
        let gap = 22.0;
        let heights: Vec<f32> =
            layouts.iter().map(|(tg, dg)| (tg.size().y + 4.0 + dg.size().y).max(marker) + gap).collect();
        let total: f32 = heights.iter().sum::<f32>() - gap;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, total.max(0.0)), Sense::hover());
        let prog = glowkit_core::anim::follow(ui, resp.id.with("p"), self.active as f32, 4.0);
        let time = glowkit_core::time(ui);
        let p = ui.painter();
        let mut ys = Vec::new();
        let mut y = rect.min.y;
        for h in &heights {
            ys.push(y);
            y += h;
        }
        let cx = rect.min.x + marker / 2.0;
        let track = if t.dark { t.secondary } else { mix(t.muted, t.input, 0.7) };
        for i in 0..self.items.len().saturating_sub(1) {
            let y0 = ys[i] + marker + 4.0;
            let y1 = ys[i + 1] - 4.0;
            p.line_segment([pos2(cx, y0), pos2(cx, y1)], Stroke::new(2.0, track));
            let f = (prog - i as f32).clamp(0.0, 1.0);
            if f > 0.0 {
                p.line_segment([pos2(cx, y0), pos2(cx, y0 + (y1 - y0) * f)], Stroke::new(2.0, t.primary));
            }
        }
        for (i, (it, (tg, dg))) in self.items.iter().zip(layouts).enumerate() {
            let c = pos2(cx, ys[i] + marker / 2.0);
            let done = prog >= i as f32 + 0.0 && i < self.active;
            let current = i == self.active;
            if current {
                let ph = (time * 0.7).fract();
                p.circle_filled(c, marker / 2.0 + 7.0 * ph, with_alpha(t.ring, 0.3 * (1.0 - ph)));
                glowkit_core::animate(ui);
            }
            let (fill, stroke, icol) = if done {
                (t.primary, t.primary, t.primary_foreground)
            } else if current {
                (t.background, t.primary, t.primary)
            } else {
                (t.background, track, t.muted_foreground)
            };
            p.circle(c, marker / 2.0, fill, Stroke::new(if current { 2.0 } else { 1.5 }, stroke));
            let ir = Rect::from_center_size(c, vec2(13.0, 13.0));
            match (it.icon, done) {
                (_, true) => kit::check_path(p, ir.expand(1.0), icol, 2.0, 1.0),
                (Some(icon), false) => icon.paint(p, ir, icol),
                (None, false) => {
                    p.circle_filled(c, 3.5, icol);
                }
            }
            let ty = ys[i] + (marker - tg.size().y) / 2.0 - 1.0;
            let tgw = tg.size().x;
            let tgh = tg.size().y;
            let tcol = if i <= self.active { t.foreground } else { t.muted_foreground };
            p.galley(pos2(rect.min.x + text_x, ty), tg, tcol);
            p.text(
                pos2(rect.min.x + text_x + tgw + 10.0, ty + tgh / 2.0),
                egui::Align2::LEFT_CENTER,
                &it.time,
                FontId::proportional(12.0),
                t.muted_foreground,
            );
            p.galley(pos2(rect.min.x + text_x, ty + tgh + 4.0), dg, t.muted_foreground);
        }
        resp
    }
}

/// Tiny Rust-ish syntax highlighter (keywords, types, strings, numbers, comments, macros, calls).
fn highlight(job: &mut LayoutJob, code: &str, t: &Theme, font: &FontId) {
    let (kw, ty, st, num, com, mac, func, txt, punct) = if t.dark {
        (
            hex("#C792EA"),
            hex("#82AAFF"),
            hex("#C3E88D"),
            hex("#F78C6C"),
            hex("#6B7280"),
            hex("#89DDFF"),
            hex("#FFCB6B"),
            hex("#E4E4E7"),
            hex("#A1A1AA"),
        )
    } else {
        (
            hex("#7C3AED"),
            hex("#2563EB"),
            hex("#15803D"),
            hex("#C2410C"),
            hex("#9CA3AF"),
            hex("#0E7490"),
            hex("#A16207"),
            hex("#18181B"),
            hex("#52525B"),
        )
    };
    const KW: [&str; 30] = [
        "fn", "let", "mut", "pub", "use", "mod", "struct", "enum", "impl", "for", "in", "if", "else", "match",
        "return", "while", "loop", "move", "self", "Self", "true", "false", "as", "const", "static", "trait", "where",
        "async", "await", "crate",
    ];
    let fmt = |c: Color32| TextFormat { font_id: font.clone(), color: c, ..Default::default() };
    let chars: Vec<char> = code.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            let end = chars[i..].iter().position(|&x| x == '\n').map(|p| i + p).unwrap_or(chars.len());
            job.append(&chars[i..end].iter().collect::<String>(), 0.0, fmt(com));
            i = end;
        } else if c == '"' {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '"' {
                if chars[j] == '\\' {
                    j += 1;
                }
                j += 1;
            }
            let end = (j + 1).min(chars.len());
            job.append(&chars[i..end].iter().collect::<String>(), 0.0, fmt(st));
            i = end;
        } else if c.is_ascii_digit() {
            let mut j = i;
            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '.' || chars[j] == '_') {
                j += 1;
            }
            job.append(&chars[i..j].iter().collect::<String>(), 0.0, fmt(num));
            i = j;
        } else if c.is_alphabetic() || c == '_' {
            let mut j = i;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let word: String = chars[i..j].iter().collect();
            let next = chars.get(j).copied();
            let color = if KW.contains(&word.as_str()) {
                kw
            } else if next == Some('!') {
                mac
            } else if word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                ty
            } else if next == Some('(') {
                func
            } else {
                txt
            };
            job.append(&word, 0.0, fmt(color));
            i = j;
        } else {
            let color = if "{}()[];,.:&<>=+-*|!#'".contains(c) { punct } else { txt };
            job.append(&c.to_string(), 0.0, fmt(color));
            i += 1;
        }
    }
}

/// A code block with a header (file name / language), line numbers, simple Rust syntax
/// highlighting and a copy button that confirms with a check mark.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::CodeBlock;
/// ui.add(CodeBlock::new("fn main() {\n    println!(\"hi\");\n}").title("main.rs"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CodeBlock {
    code: String,
    title: Option<String>,
    language: String,
    line_numbers: bool,
    highlight: Option<std::ops::RangeInclusive<usize>>,
    width: Option<f32>,
    plain: bool,
}

impl CodeBlock {
    /// A code block showing `code`.
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            title: None,
            language: "rust".into(),
            line_numbers: true,
            highlight: None,
            width: None,
            plain: false,
        }
    }
    /// File name shown in the header.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }
    /// Language label in the header (default "rust").
    pub fn language(mut self, l: impl Into<String>) -> Self {
        self.language = l.into();
        self
    }
    /// Show the line-number gutter (default on).
    pub fn line_numbers(mut self, l: bool) -> Self {
        self.line_numbers = l;
        self
    }
    /// Emphasize a range of lines (1-based, inclusive).
    pub fn highlight_lines(mut self, r: std::ops::RangeInclusive<usize>) -> Self {
        self.highlight = Some(r);
        self
    }
    /// Width (default: available).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Disable syntax colors.
    pub fn plain(mut self, p: bool) -> Self {
        self.plain = p;
        self
    }
}

impl Widget for CodeBlock {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = self.width.unwrap_or_else(|| ui.available_width());
        let font = FontId::monospace(13.0);
        let line_h = 20.0;
        let lines: Vec<&str> = self.code.lines().collect();
        let head_h = 40.0;
        let pad_y = 12.0;
        let h = head_h + pad_y * 2.0 + line_h * lines.len() as f32;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let bg = if t.dark { mix(t.background, Color32::BLACK, 0.2) } else { mix(t.muted, t.background, 0.35) };
        let cr = t.corner();
        let p = ui.painter();
        p.rect(rect, cr, bg, Stroke::new(1.0, t.border), StrokeKind::Inside);
        // header
        let head = Rect::from_min_size(rect.min, vec2(w, head_h));
        p.rect_filled(
            head.shrink(1.0),
            CornerRadius { nw: cr.nw, ne: cr.ne, sw: 0, se: 0 },
            if t.dark { with_alpha(t.muted, 0.6) } else { t.muted },
        );
        let y = kit::snap(ui, head.max.y) - 0.5;
        p.line_segment([pos2(rect.min.x, y), pos2(rect.max.x, y)], Stroke::new(1.0, t.border));
        UiIcon::Code.paint(
            p,
            Rect::from_center_size(pos2(head.min.x + 22.0, head.center().y), vec2(15.0, 15.0)),
            t.muted_foreground,
        );
        let label = self.title.clone().unwrap_or_else(|| self.language.clone());
        p.text(
            pos2(head.min.x + 38.0, head.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            FontId::proportional(13.0),
            t.foreground,
        );
        // copy button
        let cid = resp.id.with("copy");
        let now = ui.input(|i| i.time);
        let copied_at: f64 = glowkit_core::state(ui, cid, || -10.0);
        let br = Rect::from_center_size(pos2(head.max.x - 22.0, head.center().y), vec2(28.0, 28.0));
        let b = ui.interact(br, cid.with("btn"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        if b.clicked() {
            ui.ctx().copy_text(self.code.clone());
            glowkit_core::set_state(ui, cid, now);
        }
        let hov = kit::anim(ui, cid.with("h"), b.hovered(), 0.1);
        let p = ui.painter();
        if hov > 0.0 {
            p.rect_filled(br, CornerRadius::same(6), with_alpha(t.secondary, hov));
        }
        let since = (now - copied_at) as f32;
        if since < 1.6 {
            let k = (since / 0.25).clamp(0.0, 1.0);
            kit::check_path(p, br.shrink(7.0), t.success, 1.8, k);
            p.text(
                pos2(br.min.x - 6.0, head.center().y),
                egui::Align2::RIGHT_CENTER,
                "Copied!",
                FontId::proportional(12.0),
                with_alpha(t.success, (1.6 - since).clamp(0.0, 1.0)),
            );
            ui.ctx().request_repaint();
        } else {
            UiIcon::Copy.paint(p, br.shrink(7.0), t.muted_foreground.lerp_to_gamma(t.foreground, hov));
        }
        // body
        let gutter = if self.line_numbers { 16.0 + 8.0 * (lines.len().to_string().len() as f32) + 14.0 } else { 16.0 };
        let body_top = head.max.y + pad_y;
        let clip = p.with_clip_rect(
            Rect::from_min_max(pos2(rect.min.x + 1.0, head.max.y), rect.max - vec2(1.0, 1.0)).intersect(p.clip_rect()),
        );
        for (i, line) in lines.iter().enumerate() {
            let ly = body_top + i as f32 * line_h;
            let hl = self.highlight.as_ref().map(|r| r.contains(&(i + 1))).unwrap_or(false);
            if hl {
                clip.rect_filled(
                    Rect::from_min_size(pos2(rect.min.x + 1.0, ly), vec2(w - 2.0, line_h)),
                    CornerRadius::ZERO,
                    with_alpha(t.accent, if t.dark { 0.12 } else { 0.08 }),
                );
                clip.rect_filled(
                    Rect::from_min_size(pos2(rect.min.x + 1.0, ly), vec2(2.0, line_h)),
                    CornerRadius::ZERO,
                    t.accent,
                );
            }
            if self.line_numbers {
                clip.text(
                    pos2(rect.min.x + gutter - 14.0, ly + line_h / 2.0),
                    egui::Align2::RIGHT_CENTER,
                    (i + 1).to_string(),
                    font.clone(),
                    with_alpha(t.muted_foreground, if hl { 1.0 } else { 0.6 }),
                );
            }
            let mut job = LayoutJob::default();
            if self.plain {
                job.append(line, 0.0, TextFormat { font_id: font.clone(), color: t.foreground, ..Default::default() });
            } else {
                highlight(&mut job, line, &t, &font);
            }
            let g = ui.fonts_mut(|f| f.layout_job(job));
            clip.galley(pos2(rect.min.x + gutter, ly + (line_h - g.size().y) / 2.0), g, t.foreground);
        }
        resp
    }
}
