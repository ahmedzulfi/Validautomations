use crate::util;
use egui::{pos2, vec2, Color32, Response, Sense, Ui, Widget};
use glowkit_core::{anim::approach, Theme};

#[derive(Clone, Default)]
struct VelState {
    offsets: Vec<f32>,
    boost: f32,
    dir: f32,
}

/// Big rows of text scrolling endlessly in alternating directions. Scrolling the mouse wheel
/// over it kicks the rows faster (and reverses them when scrolling up), like scroll-velocity
/// marquees on the web. The ends fade out without needing to know the background.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::ScrollVelocity::new(["Pure Rust", "Animated UI"]).size(64.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ScrollVelocity {
    rows: Vec<String>,
    size: f32,
    colors: Vec<Color32>,
    separator: String,
    speed: f32,
    bold: bool,
    fade: f32,
    width: Option<f32>,
}

impl ScrollVelocity {
    /// One scrolling row per string.
    pub fn new<S: Into<String>>(rows: impl IntoIterator<Item = S>) -> Self {
        Self {
            rows: rows.into_iter().map(Into::into).collect(),
            size: 56.0,
            colors: Vec::new(),
            separator: "  ✦  ".into(),
            speed: 70.0,
            bold: true,
            fade: 0.12,
            width: None,
        }
    }
    /// Font size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Row colors, cycled (defaults to foreground, then muted foreground).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    /// Text inserted between repetitions.
    pub fn separator(mut self, s: impl Into<String>) -> Self {
        self.separator = s.into();
        self
    }
    /// Base speed in points per second.
    pub fn speed(mut self, pts_per_sec: f32) -> Self {
        self.speed = pts_per_sec;
        self
    }
    /// Fake-bold the text (on by default).
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    /// Width of the faded ends as a fraction of the width (0 = hard edges).
    pub fn fade(mut self, fraction: f32) -> Self {
        self.fade = fraction.clamp(0.0, 0.5);
        self
    }
    /// Width in points (defaults to the available width).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
}

impl Widget for ScrollVelocity {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let bold = util::weight(self.bold, self.size);
        let painter = ui.painter().clone();
        let galleys: Vec<_> =
            self.rows.iter().map(|s| util::layout(&painter, format!("{s}{}", self.separator), &font)).collect();
        let row_h = self.size * 1.2;
        let gap = self.size * 0.1;
        let w = self.width.unwrap_or(ui.available_width());
        let h = row_h * galleys.len() as f32 + gap * galleys.len().saturating_sub(1) as f32;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        glowkit_core::animate(ui);
        let dt = glowkit_core::dt(ui);
        let scroll = if resp.hovered() { ui.input(|i| i.smooth_scroll_delta.y) } else { 0.0 };
        let n = galleys.len();
        let speed = self.speed;
        let st = glowkit_core::with_state(
            ui,
            resp.id.with("vel"),
            || VelState { offsets: vec![0.0; n], boost: 0.0, dir: 1.0 },
            |s| {
                s.offsets.resize(n, 0.0);
                if scroll.abs() > 0.1 {
                    s.boost = (s.boost + scroll.abs() * 0.06).min(12.0);
                    s.dir = if scroll < 0.0 { 1.0 } else { -1.0 };
                }
                s.boost = approach(s.boost, 0.0, 2.5, dt);
                for (i, (o, g)) in s.offsets.iter_mut().zip(&galleys).enumerate() {
                    let sign = if i % 2 == 0 { -1.0 } else { 1.0 };
                    let unit = g.size().x.max(1.0);
                    *o = (*o + sign * s.dir * speed * (1.0 + s.boost) * dt).rem_euclid(unit);
                }
                s.clone()
            },
        );
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let clip = rect.intersect(ui.clip_rect());
        let p = painter.with_clip_rect(clip);
        let fade = self.fade * rect.width();
        let edge = |x: f32| {
            if fade <= 0.0 {
                1.0
            } else {
                let d = (x - rect.min.x).min(rect.max.x - x);
                let k = (d / fade).clamp(0.0, 1.0);
                k * k * (3.0 - 2.0 * k)
            }
        };
        for (i, g) in galleys.iter().enumerate() {
            let color = if self.colors.is_empty() {
                if i % 2 == 0 {
                    t.foreground
                } else {
                    t.muted_foreground
                }
            } else {
                self.colors[i % self.colors.len()]
            };
            let unit = g.size().x.max(1.0);
            let y = rect.min.y + i as f32 * (row_h + gap) + (row_h - g.size().y) / 2.0;
            let mut x = rect.min.x - st.offsets[i];
            let steps = (bold / 0.7).ceil() as usize;
            while x < rect.max.x {
                if x + unit > rect.min.x {
                    for s in 0..=steps {
                        let dx = if steps == 0 { 0.0 } else { bold * s as f32 / steps as f32 };
                        let origin = pos2(x + dx, y);
                        let c = if s == 0 { color } else { color.gamma_multiply(0.7) };
                        util::paint_galley_mapped(&p, g, |v| origin + v, |q| c.gamma_multiply(edge(q.x)));
                    }
                }
                x += unit;
            }
        }
        resp
    }
}
