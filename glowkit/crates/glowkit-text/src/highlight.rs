use crate::util;
use egui::{pos2, vec2, Color32, CornerRadius, Pos2, Rect, Response, Sense, Stroke, Ui, Widget};
use glowkit_core::{ease, paint, Clock, Easing, Theme, Trigger};
use std::f32::consts::TAU;

/// The kind of mark [`HighlightText`] draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HighlightStyle {
    /// A gradient marker swiped behind the phrase (text turns light on top of it).
    #[default]
    Marker,
    /// A thick rounded underline drawn from left to right.
    Underline,
    /// A rounded box traced around the phrase.
    Box,
    /// A hand-drawn loop circled around the phrase.
    Circle,
    /// A line struck through the phrase.
    StrikeThrough,
}

/// A sentence with one phrase emphasised by an animated mark: a marker swipe, an underline, a box,
/// a hand-drawn circle or a strike-through.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_text::{HighlightText, HighlightStyle};
/// ui.add(HighlightText::new("Ship it faster than ever").phrase("faster").style(HighlightStyle::Marker));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct HighlightText {
    text: String,
    phrase: Option<String>,
    size: f32,
    color: Option<Color32>,
    colors: Option<[Color32; 2]>,
    on_color: Option<Color32>,
    style: HighlightStyle,
    bold: bool,
    delay: f32,
    duration: f32,
    trigger: Trigger,
    wrap: Option<f32>,
}

impl HighlightText {
    /// Create the effect for `text` (the whole text is highlighted unless [`Self::phrase`] is set).
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            phrase: None,
            size: 32.0,
            color: None,
            colors: None,
            on_color: None,
            style: HighlightStyle::Marker,
            bold: true,
            delay: 0.35,
            duration: 1.1,
            trigger: Trigger::OnAppear,
            wrap: None,
        }
    }
    /// The part of the text to highlight (first occurrence).
    pub fn phrase(mut self, phrase: impl Into<String>) -> Self {
        self.phrase = Some(phrase.into());
        self
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Text color (defaults to the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Mark color; for [`HighlightStyle::Marker`] the two colors form a gradient.
    /// Defaults to the theme gradient.
    pub fn mark_colors(mut self, a: Color32, b: Color32) -> Self {
        self.colors = Some([a, b]);
        self
    }
    /// Color of the text on top of a marker (defaults to white or black for contrast).
    pub fn on_color(mut self, c: Color32) -> Self {
        self.on_color = Some(c);
        self
    }
    /// Kind of mark.
    pub fn style(mut self, s: HighlightStyle) -> Self {
        self.style = s;
        self
    }
    /// Fake-bold the text (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Seconds before the mark starts drawing.
    pub fn delay(mut self, seconds: f32) -> Self {
        self.delay = seconds.max(0.0);
        self
    }
    /// Seconds the mark takes to draw.
    pub fn duration(mut self, seconds: f32) -> Self {
        self.duration = seconds.max(0.05);
        self
    }
    /// When the mark draws.
    pub fn trigger(mut self, t: Trigger) -> Self {
        self.trigger = t;
        self
    }
    /// Wrap width in points (defaults to the available width).
    pub fn wrap(mut self, w: f32) -> Self {
        self.wrap = Some(w);
        self
    }
}

/// Draw the first `p` (0..1) of a polyline.
fn partial_line(painter: &egui::Painter, pts: &[Pos2], p: f32, stroke: Stroke) {
    if pts.len() < 2 || p <= 0.0 {
        return;
    }
    let lens: Vec<f32> = pts.windows(2).map(|w| w[0].distance(w[1])).collect();
    let total: f32 = lens.iter().sum();
    let mut left = total * p.min(1.0);
    let mut out = vec![pts[0]];
    for (w, l) in pts.windows(2).zip(lens) {
        if left >= l {
            out.push(w[1]);
            left -= l;
        } else {
            out.push(w[0].lerp(w[1], left / l.max(1e-4)));
            break;
        }
    }
    painter.line(out, stroke);
}

impl Widget for HighlightText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let wrap = self.wrap.unwrap_or(ui.available_width());
        let glyphs = crate::util::glyphs(ui.painter(), &self.text, font.clone(), Some(wrap));
        let pad = vec2(self.size * 0.3, self.size * 0.25);
        let (rect, resp) = ui.allocate_exact_size(glyphs.size + vec2(bold, 0.0) + pad * 2.0, Sense::click());
        let total = self.delay + self.duration;
        let e = Clock::triggered(ui, resp.id, self.trigger, total, resp.hovered(), resp.clicked());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter();
        let origin = rect.min + pad;
        // character range of the phrase
        let (start, end) = match &self.phrase {
            Some(ph) => match self.text.find(ph.as_str()) {
                Some(b) => {
                    let s = self.text[..b].chars().count();
                    (s, s + ph.chars().count())
                }
                None => (0, 0),
            },
            None => (0, self.text.chars().count()),
        };
        // one rect per line the phrase touches
        let mut rows: Vec<Rect> = Vec::new();
        let mut last_line = usize::MAX;
        for g in glyphs.glyphs.iter().filter(|g| g.index >= start && g.index < end) {
            let r = Rect::from_min_size(origin + g.offset, g.size);
            if g.line == last_line {
                if !g.ch.is_whitespace() {
                    let l = rows.last_mut().expect("row");
                    *l = l.union(r);
                }
            } else if !g.ch.is_whitespace() {
                rows.push(r);
                last_line = g.line;
            }
        }
        let lin = ((e - self.delay) / self.duration).clamp(0.0, 1.0);
        let p = ease(Easing::InOutCubic, lin);
        let total_w: f32 = rows.iter().map(|r| r.width()).sum::<f32>().max(1.0);
        // progress of each row, drawn one after another
        let mut acc = 0.0;
        let row_p: Vec<f32> = rows
            .iter()
            .map(|r| {
                let a = acc / total_w;
                acc += r.width();
                let b = acc / total_w;
                ((p - a) / (b - a).max(1e-4)).clamp(0.0, 1.0)
            })
            .collect();
        let [c0, c1] = self.colors.unwrap_or([t.gradient[0], t.gradient[1]]);
        let color = self.color.unwrap_or(t.foreground);
        let s = self.size;
        let mut covered: Vec<Rect> = Vec::new();
        match self.style {
            HighlightStyle::Marker => {
                for (r, rp) in rows.iter().zip(&row_p) {
                    if *rp <= 0.0 {
                        continue;
                    }
                    let full = Rect::from_min_max(
                        pos2(r.min.x - s * 0.14, r.min.y + s * 0.08),
                        pos2(r.max.x + bold + s * 0.14, r.max.y - s * 0.02),
                    );
                    let part = Rect::from_min_max(full.min, pos2(full.min.x + full.width() * rp, full.max.y));
                    let clip = painter.with_clip_rect(part.intersect(painter.clip_rect()));
                    paint::gradient_rect(&clip, full, s * 0.14, 0.0, &[c0, c1]);
                    covered.push(part);
                }
            }
            HighlightStyle::Underline => {
                for (r, rp) in rows.iter().zip(&row_p) {
                    if *rp <= 0.0 {
                        continue;
                    }
                    let th = (s * 0.09).max(3.0);
                    let y = r.max.y - s * 0.02;
                    let x0 = r.min.x - s * 0.04;
                    let x1 = x0 + (r.width() + bold + s * 0.08) * rp;
                    let line = Rect::from_min_max(pos2(x0, y - th / 2.0), pos2(x1.max(x0 + th), y + th / 2.0));
                    let full =
                        Rect::from_min_max(pos2(x0, line.min.y), pos2(x0 + r.width() + bold + s * 0.08, line.max.y));
                    let clip = painter.with_clip_rect(line.intersect(painter.clip_rect()));
                    paint::gradient_rect(&clip, full, th / 2.0, 0.0, &[c0, c1]);
                }
            }
            HighlightStyle::Box => {
                for (r, rp) in rows.iter().zip(&row_p) {
                    let b = Rect::from_min_max(
                        pos2(r.min.x - s * 0.18, r.min.y + s * 0.02),
                        pos2(r.max.x + bold + s * 0.18, r.max.y + s * 0.04),
                    );
                    let mut pts = paint::rounded_rect_points(b, s * 0.16, 6);
                    // start at the top-left corner (last point), go clockwise and close the loop
                    let last = pts.len() - 1;
                    pts.rotate_left(last);
                    pts.push(pts[0]);
                    partial_line(painter, &pts, *rp, Stroke::new((s * 0.06).max(2.0), c0));
                }
            }
            HighlightStyle::Circle => {
                for (r, rp) in rows.iter().zip(&row_p) {
                    let c = r.center() + vec2(bold / 2.0, s * 0.03);
                    let rad = vec2(r.width() / 2.0 + s * 0.45, r.height() / 2.0 + s * 0.12);
                    let sweep = TAU * 1.12;
                    let n = 90;
                    let pts: Vec<Pos2> = (0..=n)
                        .map(|i| {
                            let u = i as f32 / n as f32;
                            let a = -2.2 + u * sweep;
                            // hand-drawn: radius drifts so the loop doesn't close on itself
                            let wob = 1.0 + 0.06 * (u * 2.0 - 1.0) + 0.02 * (a * 3.0).sin();
                            c + vec2(a.cos() * rad.x * wob, a.sin() * rad.y * wob)
                        })
                        .collect();
                    partial_line(painter, &pts, *rp, Stroke::new((s * 0.055).max(2.0), c0));
                }
            }
            HighlightStyle::StrikeThrough => {}
        }
        // text
        let on = self.on_color.unwrap_or_else(|| glowkit_core::color::contrast_text(glowkit_core::mix(c0, c1, 0.5)));
        let tint = |q: Pos2| {
            if covered.iter().any(|r| r.contains(q)) {
                on
            } else {
                color
            }
        };
        let steps = (bold / 0.7).ceil() as usize;
        for i in 0..=steps {
            let dx = if steps == 0 { 0.0 } else { bold * i as f32 / steps as f32 };
            paint::galley_with(painter, origin + vec2(dx, 0.0), &glyphs.galley, tint);
        }
        if self.style == HighlightStyle::StrikeThrough {
            for (r, rp) in rows.iter().zip(&row_p) {
                if *rp <= 0.0 {
                    continue;
                }
                let th = (s * 0.07).max(2.5);
                let y = r.center().y + s * 0.06;
                let x0 = r.min.x - s * 0.08;
                let x1 = x0 + (r.width() + bold + s * 0.16) * rp;
                painter.rect_filled(
                    Rect::from_min_max(pos2(x0, y - th / 2.0), pos2(x1.max(x0 + th), y + th / 2.0)),
                    CornerRadius::same((th / 2.0) as u8),
                    c0,
                );
            }
        }
        resp
    }
}
