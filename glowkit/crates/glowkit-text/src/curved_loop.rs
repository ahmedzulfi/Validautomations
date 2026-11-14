use crate::util;
use egui::{pos2, vec2, Color32, Pos2, Response, Sense, Ui, Widget};
use glowkit_core::{anim::approach, color::gradient_cyclic, Theme};

#[derive(Clone, Copy, Debug)]
struct LoopState {
    offset: f32,
    dir: f32,
    fling: f32,
}

/// A text marquee that flows endlessly along a curved path, like a ribbon of letters bending
/// across the page. Drag it to scrub; let go and it keeps running in the direction you threw it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::CurvedLoopText::new("Welcome to glowkit ✦").curve(120.0).speed(80.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CurvedLoopText {
    text: String,
    separator: String,
    size: f32,
    speed: f32,
    curve: f32,
    color: Option<Color32>,
    colors: Vec<Color32>,
    bold: bool,
    draggable: bool,
    reverse: bool,
    height: Option<f32>,
    width: Option<f32>,
    fade: f32,
}

impl CurvedLoopText {
    /// Create a curved marquee repeating `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            separator: "  ✦  ".into(),
            size: 64.0,
            speed: 70.0,
            curve: 90.0,
            color: None,
            colors: Vec::new(),
            bold: true,
            draggable: true,
            reverse: false,
            height: None,
            width: None,
            fade: 0.1,
        }
    }
    /// Font size in points (default 64).
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Scroll speed in points per second (default 70).
    pub fn speed(mut self, pts_per_sec: f32) -> Self {
        self.speed = pts_per_sec.abs();
        self
    }
    /// How far the middle of the path sags below its ends, in points; negative bends upward
    /// (default 90).
    pub fn curve(mut self, amount: f32) -> Self {
        self.curve = amount;
        self
    }
    /// Text inserted between repetitions (default `"  ✦  "`).
    pub fn separator(mut self, s: impl Into<String>) -> Self {
        self.separator = s.into();
        self
    }
    /// Solid text color (defaults to the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Color the letters with a gradient that flows along the path instead of a solid color.
    pub fn gradient(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    /// Fake-bold the letters (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Whether the pointer can drag the text (default true).
    pub fn draggable(mut self, on: bool) -> Self {
        self.draggable = on;
        self
    }
    /// Flow toward the right instead of the left (default false).
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }
    /// Widget height in points (default: font size + curve + padding).
    pub fn height(mut self, h: f32) -> Self {
        self.height = Some(h);
        self
    }
    /// Widget width in points (defaults to the available width).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Width of the faded ends as a fraction of the width (default 0.1).
    pub fn fade(mut self, fraction: f32) -> Self {
        self.fade = fraction.clamp(0.0, 0.5);
        self
    }
}

/// Sample a quadratic bezier into points with cumulative arc lengths.
fn path(a: Pos2, c: Pos2, b: Pos2, n: usize) -> (Vec<Pos2>, Vec<f32>) {
    let pts = glowkit_core::paint::quad_bezier(a, c, b, n);
    let mut len = Vec::with_capacity(pts.len());
    let mut acc = 0.0;
    for (i, p) in pts.iter().enumerate() {
        if i > 0 {
            acc += p.distance(pts[i - 1]);
        }
        len.push(acc);
    }
    (pts, len)
}

/// Point and tangent angle at arc length `s`.
fn at(pts: &[Pos2], len: &[f32], s: f32) -> (Pos2, f32) {
    let i = len.partition_point(|l| *l < s).clamp(1, pts.len() - 1);
    let seg = (len[i] - len[i - 1]).max(1e-4);
    let t = ((s - len[i - 1]) / seg).clamp(0.0, 1.0);
    let d = pts[i] - pts[i - 1];
    (pts[i - 1] + d * t, d.y.atan2(d.x))
}

impl Widget for CurvedLoopText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let unit_text = format!("{}{}", self.text, self.separator);
        let glyphs = util::glyphs(ui.painter(), &unit_text, font.clone(), None);
        let unit = glyphs.size.x.max(1.0);
        let w = self.width.unwrap_or(ui.available_width());
        let h = self.height.unwrap_or(self.size * 1.6 + self.curve.abs() + 24.0);
        let sense = if self.draggable { Sense::drag() } else { Sense::hover() };
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), sense);
        glowkit_core::animate(ui);
        let dt = glowkit_core::dt(ui);
        let drag = if resp.dragged() { resp.drag_delta().x } else { 0.0 };
        let dragging = resp.dragged();
        if self.draggable && resp.hovered() {
            ui.ctx().set_cursor_icon(if dragging { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        let base_dir = if self.reverse { -1.0 } else { 1.0 };
        let speed = self.speed;
        let st = glowkit_core::with_state(
            ui,
            resp.id.with("curved_loop"),
            || LoopState { offset: 0.0, dir: base_dir, fling: 0.0 },
            |s| {
                if dragging {
                    // text follows the pointer; remember the throw direction
                    s.offset -= drag;
                    if drag.abs() > 0.3 {
                        s.dir = -drag.signum();
                        s.fling = (-drag / dt.max(1e-3)).clamp(-3000.0, 3000.0);
                    }
                } else {
                    s.fling = approach(s.fling, 0.0, 4.0, dt);
                    let v = if s.fling.abs() > speed { s.fling } else { s.dir * speed };
                    s.offset += v * dt;
                }
                s.offset = s.offset.rem_euclid(unit);
                *s
            },
        );
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter_at(rect.expand(self.size));
        // path: a gentle arc across the widget, extending past both ends so letters enter and
        // leave smoothly
        let over = self.size * 1.5;
        let y0 = if self.curve >= 0.0 {
            rect.min.y + self.size * 0.8 + 6.0
        } else {
            rect.max.y - self.size * 0.8 - 6.0
        };
        let a = pos2(rect.min.x - over, y0);
        let b = pos2(rect.max.x + over, y0);
        let c = pos2(rect.center().x, y0 + self.curve * 2.0);
        let (pts, len) = path(a, c, b, 96);
        let total = *len.last().unwrap_or(&1.0);
        let fade = self.fade * rect.width();
        let edge = |x: f32| {
            if fade <= 0.0 {
                return 1.0;
            }
            let d = (x - rect.min.x).min(rect.max.x - x);
            let k = (d / fade).clamp(0.0, 1.0);
            k * k * (3.0 - 2.0 * k)
        };
        let solid = self.color.unwrap_or(t.foreground);
        let time = glowkit_core::time(ui);
        let reps = (total / unit).ceil() as i32 + 2;
        for k in -1..reps {
            for g in &glyphs.glyphs {
                if g.ch.is_whitespace() {
                    continue;
                }
                let s = k as f32 * unit + g.offset.x + g.size.x / 2.0 - st.offset;
                if s < 0.0 || s > total {
                    continue;
                }
                let (p, ang) = at(&pts, &len, s);
                let alpha = edge(p.x);
                if alpha <= 0.01 {
                    continue;
                }
                // glyph centered over the path: baseline sits on it
                let n = vec2(-ang.sin(), ang.cos());
                let center = p - n * (g.size.y * 0.32);
                let color = if self.colors.is_empty() {
                    solid
                } else {
                    gradient_cyclic(&self.colors, s / 900.0 - time * 0.08)
                };
                util::paint_char_xf(&painter, g.ch, &font, center, 1.0, ang, color.gamma_multiply(alpha), bold);
            }
        }
        resp
    }
}
