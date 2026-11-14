//! Streaming-safe Markdown rendering ([`Markdown`]) and the parser behind it ([`parse`]).

mod parse;

pub use parse::{parse, parse_inline, spans_text, Block, ColumnAlign, ListItem, Span, SpanStyle};

use crate::code::CodeBlock;
use crate::common::{self, Fx};
use crate::icons::{paint_icon, Icon};
use egui::text::LayoutJob;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Galley, Pos2, Rect, Response, Sense, Stroke, StrokeKind,
    TextFormat, Ui, UiBuilder, Widget,
};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;
use std::ops::Range;
use std::sync::Arc;

/// Renders Markdown the way AI chat products do: comfortable line height, highlighted code blocks
/// with a copy button, clean tables, lists, quotes and links.
///
/// It is built for streaming: pass the text received so far every frame and half-written syntax
/// (an open ```` ``` ```` fence, a dangling `**`) never flashes on screen. Optionally the newest
/// characters fade in ([`Markdown::fade_tail`]) and a blinking caret follows the text.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let streamed_so_far = String::new();
/// ui.add(glowkit_ai::Markdown::new("## Hello\nSome **bold** text and `code`."));
/// // while streaming:
/// ui.add(glowkit_ai::Markdown::new(&streamed_so_far).caret(true).fade_tail(24));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Markdown<'a> {
    text: &'a str,
    size: f32,
    color: Option<Color32>,
    caret: bool,
    fade_tail: usize,
}

impl<'a> Markdown<'a> {
    /// Render `text` as Markdown.
    pub fn new(text: &'a str) -> Self {
        Self { text, size: 14.0, color: None, caret: false, fade_tail: 0 }
    }
    /// Base font size of body text (headings scale from it).
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Body text color. Defaults to the theme foreground, slightly softened.
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }
    /// Show a blinking caret after the last character (while streaming).
    pub fn caret(mut self, on: bool) -> Self {
        self.caret = on;
        self
    }
    /// Fade in the last `chars` characters, so freshly streamed tokens ease in (0 = off).
    pub fn fade_tail(mut self, chars: usize) -> Self {
        self.fade_tail = chars;
        self
    }
}

fn spans_chars(s: &[Span]) -> usize {
    s.iter().map(|x| x.text.chars().count()).sum()
}

fn block_chars(b: &Block) -> usize {
    match b {
        Block::Heading { spans, .. } | Block::Paragraph(spans) => spans_chars(spans),
        Block::List(items) => items.iter().map(|i| spans_chars(&i.spans)).sum(),
        Block::Quote(inner) => inner.iter().map(block_chars).sum(),
        Block::Code { code, .. } => code.chars().count(),
        Block::Table { header, rows, .. } => {
            header.iter().map(|c| spans_chars(c)).sum::<usize>()
                + rows.iter().flatten().map(|c| spans_chars(c)).sum::<usize>()
        }
        Block::Rule => 0,
    }
}

/// A laid-out run of inline text plus the character ranges that need extra painting.
struct Laid {
    galley: Arc<Galley>,
    bold: Vec<Range<usize>>,
    code: Vec<Range<usize>>,
    links: Vec<(Range<usize>, String)>,
    chars: usize,
}

struct Renderer {
    t: Theme,
    size: f32,
    body: Color32,
    strong: Color32,
    total: usize,
    tail: usize,
    counter: usize,
    last_end: Option<Rect>,
}

impl Renderer {
    fn layout(&self, ui: &Ui, spans: &[Span], size: f32, color: Color32, all_bold: bool, wrap: f32) -> Laid {
        let t = &self.t;
        let mut job = LayoutJob::default();
        job.wrap.max_width = wrap;
        let lh = Some((size * 1.62).round());
        let mut bold = Vec::new();
        let mut code = Vec::new();
        let mut links = Vec::new();
        let mut idx = 0usize;
        let mut prev_code = false;
        for s in spans {
            let n = s.text.chars().count();
            let r = idx..idx + n;
            let mut fmt = TextFormat {
                font_id: FontId::proportional(size),
                color,
                line_height: lh,
                italics: s.style.italic,
                ..Default::default()
            };
            if s.style.bold || all_bold {
                bold.push(r.clone());
                if !all_bold {
                    fmt.color = self.strong;
                }
            }
            if s.style.code {
                fmt.font_id = FontId::monospace((size * 0.86).round());
                fmt.color = if t.dark { mix(self.strong, t.accent, 0.12) } else { mix(self.strong, t.accent, 0.18) };
                fmt.valign = egui::Align::Center;
                code.push(r.clone());
            }
            if let Some(url) = &s.link {
                fmt.color = t.accent;
                links.push((r.clone(), url.clone()));
            }
            if s.style.strike {
                fmt.strikethrough = Stroke::new(1.0, with_alpha(fmt.color, 0.7));
                fmt.color = with_alpha(fmt.color, 0.6);
            }
            let lead = if s.style.code || prev_code { 3.0 } else { 0.0 };
            prev_code = s.style.code;
            job.append(&s.text, lead, fmt);
            idx += n;
        }
        let galley = ui.painter().layout_job(job);
        Laid { galley, bold, code, links, chars: idx }
    }

    /// Allocate and paint a laid-out paragraph at the cursor (with `indent`).
    fn paint(&mut self, ui: &mut Ui, laid: &Laid, pos: Pos2) {
        let painter = ui.painter().clone();
        let t = &self.t;
        let g = &laid.galley;
        // inline code chips
        for r in &laid.code {
            for cr in common::char_rects(g, r.clone()) {
                let rr = Rect::from_min_max(
                    pos2(cr.min.x - 3.5, cr.center().y - self.size * 0.68),
                    pos2(cr.max.x + 3.5, cr.center().y + self.size * 0.68),
                )
                .translate(pos.to_vec2());
                let fill = if t.dark { with_alpha(t.foreground, 0.075) } else { with_alpha(t.foreground, 0.055) };
                painter.rect_filled(rr, CornerRadius::same(5), fill);
                painter.rect_stroke(
                    rr,
                    CornerRadius::same(5),
                    Stroke::new(1.0, with_alpha(t.border, 0.7)),
                    StrokeKind::Inside,
                );
            }
        }
        // links: underline + hover + click
        let pointer = ui.input(|i| i.pointer.hover_pos());
        for (k, (r, url)) in laid.links.iter().enumerate() {
            let rects: Vec<Rect> =
                common::char_rects(g, r.clone()).into_iter().map(|x| x.translate(pos.to_vec2())).collect();
            let hovered = pointer.is_some_and(|p| rects.iter().any(|x| x.contains(p)))
                && ui.rect_contains_pointer(ui.clip_rect());
            for x in &rects {
                let y = x.center().y + self.size * 0.62;
                painter.line_segment(
                    [pos2(x.min.x, y), pos2(x.max.x, y)],
                    Stroke::new(1.0, with_alpha(t.accent, if hovered { 0.9 } else { 0.35 })),
                );
            }
            if let Some(first) = rects.first() {
                let id = ui.unique_id().with(("md_link", self.counter, k));
                let resp = ui.interact(rects.iter().fold(*first, |a, b| a.union(*b)), id, Sense::click());
                if hovered {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    if resp.clicked() && !url.is_empty() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                    }
                }
            }
        }
        let start = self.counter;
        let total = self.total;
        let tail = self.tail;
        let bold = &laid.bold;
        common::paint_galley_fx(&painter, pos, g, |i| {
            let alpha =
                if tail == 0 { 1.0 } else { (total.saturating_sub(start + i) as f32 / tail as f32).clamp(0.0, 1.0) };
            Fx {
                alpha: alpha * alpha * (3.0 - 2.0 * alpha),
                bold: bold.iter().any(|r| r.contains(&i)),
                ..Default::default()
            }
        });
        self.counter += laid.chars;
        if laid.chars > 0 {
            self.last_end = Some(common::galley_end(g).translate(pos.to_vec2()));
        }
    }

    fn text_block(&mut self, ui: &mut Ui, spans: &[Span], size: f32, color: Color32, all_bold: bool) {
        let w = ui.available_width();
        let laid = self.layout(ui, spans, size, color, all_bold, w);
        let (rect, _) = ui.allocate_exact_size(vec2(w, laid.galley.size().y), Sense::hover());
        if ui.is_rect_visible(rect) {
            self.paint(ui, &laid, rect.min);
        } else {
            self.counter += laid.chars;
        }
    }

    fn blocks(&mut self, ui: &mut Ui, blocks: &[Block], color: Color32) {
        for (bi, b) in blocks.iter().enumerate() {
            if bi > 0 {
                let gap = match b {
                    Block::Heading { level: 1..=2, .. } => 20.0,
                    Block::Heading { .. } => 16.0,
                    _ => 12.0,
                };
                ui.add_space(gap);
            }
            self.block(ui, b, color);
        }
    }

    fn block(&mut self, ui: &mut Ui, b: &Block, color: Color32) {
        let t = self.t.clone();
        match b {
            Block::Heading { level, spans } => {
                let k = match level {
                    1 => 1.5,
                    2 => 1.28,
                    3 => 1.12,
                    _ => 1.0,
                };
                self.text_block(ui, spans, (self.size * k).round(), self.strong, true);
            }
            Block::Paragraph(spans) => {
                if !spans.is_empty() {
                    self.text_block(ui, spans, self.size, color, false);
                }
            }
            Block::Rule => {
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 9.0), Sense::hover());
                ui.painter().line_segment(
                    [pos2(r.min.x, r.center().y), pos2(r.max.x, r.center().y)],
                    Stroke::new(1.0, t.border),
                );
            }
            Block::Code { lang, code, .. } => {
                let local_total = self.total.saturating_sub(self.counter);
                ui.add(CodeBlock::new(code).language(lang).fade(local_total, self.tail));
                self.counter += code.chars().count();
                self.last_end = None;
            }
            Block::Quote(inner) => {
                let avail = ui.available_rect_before_wrap();
                let inner_rect = Rect::from_min_max(avail.min + vec2(16.0, 0.0), avail.max);
                let quote_color = mix(t.muted_foreground, t.foreground, 0.25);
                let r = ui.scope_builder(UiBuilder::new().max_rect(inner_rect), |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    self.blocks(ui, inner, quote_color);
                });
                let rr = r.response.rect;
                ui.painter().rect_filled(
                    Rect::from_min_max(pos2(avail.min.x, rr.min.y + 2.0), pos2(avail.min.x + 3.0, rr.max.y - 2.0)),
                    CornerRadius::same(2),
                    with_alpha(t.accent, 0.55),
                );
            }
            Block::List(items) => self.list(ui, items, color),
            Block::Table { header, align, rows } => self.table(ui, header, align, rows),
        }
    }

    fn list(&mut self, ui: &mut Ui, items: &[ListItem], color: Color32) {
        let t = self.t.clone();
        let num_w = {
            let max = items.iter().filter_map(|i| i.number).max().unwrap_or(1);
            ui.painter().layout_no_wrap(format!("{max}."), FontId::proportional(self.size), color).size().x + 8.0
        };
        for (k, it) in items.iter().enumerate() {
            if k > 0 {
                ui.add_space(5.0);
            }
            let indent = it.depth as f32 * 22.0;
            let marker_w = if it.checked.is_some() {
                24.0
            } else if it.number.is_some() {
                num_w.max(20.0)
            } else {
                20.0
            };
            let avail = ui.available_width();
            let text_w = (avail - indent - marker_w).max(40.0);
            let item_color = if it.checked == Some(true) { t.muted_foreground } else { color };
            let laid = self.layout(ui, &it.spans, self.size, item_color, false, text_w);
            let line_h = (self.size * 1.62).round();
            let h = laid.galley.size().y.max(line_h);
            let (rect, _) = ui.allocate_exact_size(vec2(avail, h), Sense::hover());
            if !ui.is_rect_visible(rect) {
                self.counter += laid.chars;
                continue;
            }
            let first_row_h = laid.galley.rows.first().map(|r| r.row.size.y).unwrap_or(line_h);
            let cy = rect.min.y + first_row_h / 2.0;
            let mx = rect.min.x + indent;
            let painter = ui.painter();
            let marker_col = mix(t.muted_foreground, color, 0.15);
            match (it.checked, it.number) {
                (Some(done), _) => {
                    let br = Rect::from_center_size(pos2(mx + 8.0, cy), vec2(15.0, 15.0));
                    if done {
                        painter.rect_filled(br, CornerRadius::same(4), t.accent);
                        paint_icon(painter, Icon::Check, br.shrink(2.5), t.accent_foreground);
                    } else {
                        painter.rect_stroke(
                            br,
                            CornerRadius::same(4),
                            Stroke::new(1.3, common::border_strong(&t)),
                            StrokeKind::Inside,
                        );
                    }
                }
                (None, Some(n)) => {
                    painter.text(
                        pos2(mx + marker_w - 7.0, cy),
                        Align2::RIGHT_CENTER,
                        format!("{n}."),
                        FontId::proportional(self.size),
                        marker_col,
                    );
                }
                (None, None) => {
                    let c = pos2(mx + 7.0, cy);
                    match it.depth % 3 {
                        0 => {
                            painter.circle_filled(c, 2.6, marker_col);
                        }
                        1 => {
                            painter.circle_stroke(c, 2.6, Stroke::new(1.2, marker_col));
                        }
                        _ => {
                            painter.rect_filled(
                                Rect::from_center_size(c, vec2(4.5, 4.5)),
                                CornerRadius::same(1),
                                marker_col,
                            );
                        }
                    }
                }
            }
            self.paint(ui, &laid, pos2(mx + marker_w, rect.min.y));
        }
    }

    fn table(&mut self, ui: &mut Ui, header: &[Vec<Span>], align: &[ColumnAlign], rows: &[Vec<Vec<Span>>]) {
        let t = self.t.clone();
        let cols = header.len();
        if cols == 0 {
            return;
        }
        let size = (self.size * 0.93).round();
        let pad = vec2(14.0, 9.0);
        let avail = ui.available_width();
        // natural widths
        let mut nat = vec![0.0f32; cols];
        for c in 0..cols {
            let mut w = self.layout(ui, &header[c], size, self.strong, true, f32::INFINITY).galley.size().x;
            for r in rows {
                if let Some(cell) = r.get(c) {
                    w = w.max(self.layout(ui, cell, size, self.body, false, f32::INFINITY).galley.size().x);
                }
            }
            nat[c] = w + pad.x * 2.0 + 2.0;
        }
        let sum: f32 = nat.iter().sum();
        let widths: Vec<f32> = if sum <= avail {
            // natural widths, gently stretched so short tables do not look cramped
            let target = sum.max((avail * 0.6).min(sum * 1.6)).min(avail);
            nat.iter().map(|w| w * target / sum).collect()
        } else {
            nat.iter().map(|w| (w * avail / sum).max(60.0)).collect()
        };
        let total_w: f32 = widths.iter().sum();
        // lay out all cells
        let lay_row = |r: &Renderer, ui: &Ui, cells: &[Vec<Span>], head: bool| -> (Vec<Laid>, f32) {
            let mut out = Vec::new();
            let mut h: f32 = 0.0;
            for (c, w) in widths.iter().enumerate() {
                let empty = Vec::new();
                let cell = cells.get(c).unwrap_or(&empty);
                let laid = if head {
                    r.layout(ui, cell, size, r.strong, true, w - pad.x * 2.0)
                } else {
                    r.layout(ui, cell, size, r.body, false, w - pad.x * 2.0)
                };
                h = h.max(laid.galley.size().y.max(size * 1.62));
                out.push(laid);
            }
            (out, h + pad.y * 2.0)
        };
        let head = lay_row(self, ui, header, true);
        let body: Vec<(Vec<Laid>, f32)> = rows.iter().map(|r| lay_row(self, ui, r, false)).collect();
        let total_h = head.1 + body.iter().map(|b| b.1).sum::<f32>();
        let (rect, _) = ui.allocate_exact_size(vec2(total_w, total_h), Sense::hover());
        let cr = t.corner_scaled(0.8);
        {
            let painter = ui.painter();
            painter.rect_filled(rect, cr, t.card);
            let hr = Rect::from_min_size(rect.min, vec2(total_w, head.1));
            let r8 = cr.nw;
            painter.rect_filled(hr, CornerRadius { nw: r8, ne: r8, sw: 0, se: 0 }, common::raised(&t));
        }
        let mut y = rect.min.y;
        let all = std::iter::once(&head).chain(body.iter());
        for (ri, (cells, h)) in all.enumerate() {
            if ri > 0 {
                ui.painter().line_segment(
                    [pos2(rect.min.x + 1.0, y), pos2(rect.max.x - 1.0, y)],
                    Stroke::new(1.0, if ri == 1 { t.border } else { with_alpha(t.border, 0.7) }),
                );
            }
            let mut x = rect.min.x;
            for (c, laid) in cells.iter().enumerate() {
                let w = widths[c];
                let gw = laid.galley.size().x;
                let inner_w = w - pad.x * 2.0;
                let dx = match align.get(c).copied().unwrap_or_default() {
                    ColumnAlign::Left => 0.0,
                    ColumnAlign::Center => (inner_w - gw) / 2.0,
                    ColumnAlign::Right => inner_w - gw,
                };
                let dy = (h - pad.y * 2.0 - laid.galley.size().y) / 2.0;
                self.paint(ui, laid, pos2(x + pad.x + dx.max(0.0), y + pad.y + dy.max(0.0)));
                x += w;
            }
            y += h;
        }
        ui.painter().rect_stroke(rect, cr, Stroke::new(1.0, t.border), StrokeKind::Inside);
    }
}

impl Widget for Markdown<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let blocks = parse(self.text);
        let total: usize = blocks.iter().map(block_chars).sum();
        let body = self.color.unwrap_or_else(|| {
            if t.dark {
                mix(t.foreground, t.muted_foreground, 0.12)
            } else {
                mix(t.foreground, t.muted_foreground, 0.1)
            }
        });
        let mut r = Renderer {
            strong: t.foreground,
            t: t.clone(),
            size: self.size,
            body,
            total,
            tail: self.fade_tail,
            counter: 0,
            last_end: None,
        };
        let resp = ui
            .scope(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                r.blocks(ui, &blocks, body);
                if blocks.is_empty() {
                    ui.allocate_exact_size(vec2(ui.available_width().min(4.0), self.size * 1.62), Sense::hover());
                }
            })
            .response;
        if self.caret {
            let end = r.last_end.unwrap_or_else(|| {
                Rect::from_min_size(resp.rect.left_bottom() - vec2(0.0, self.size * 1.62), vec2(0.0, self.size * 1.62))
            });
            let time = glowkit_core::time(ui);
            let phase = (time * 1.6).fract();
            let a = if phase < 0.5 {
                1.0
            } else {
                1.0 - glowkit_core::anim::smoothstep(0.5, 0.65, phase)
                    + glowkit_core::anim::smoothstep(0.85, 1.0, phase)
            };
            let h = self.size * 1.1;
            let cr = Rect::from_center_size(pos2(end.max.x + 3.5, end.center().y), vec2(2.0, h));
            ui.painter().rect_filled(cr, CornerRadius::same(1), with_alpha(t.foreground, a.clamp(0.0, 1.0) * 0.9));
            glowkit_core::animate(ui);
        }
        if self.fade_tail > 0 {
            ui.ctx().request_repaint();
        }
        resp
    }
}
