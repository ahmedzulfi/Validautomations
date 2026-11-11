use egui::{vec2, Align, Color32, Layout, Rect, Response, Sense, Ui, UiBuilder};
use glowkit_core::{paint, Theme};

/// An infinitely scrolling strip of any content (logos, testimonials, tags), horizontal or
/// vertical, with optional pause on hover and faded edges.
///
/// The closure is called once per visible copy, so keep it cheap and side-effect free.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_effects::Marquee::new().speed(50.0).pause_on_hover(true).show(ui, |ui| {
///     for name in ["Rust", "egui", "WebAssembly"] {
///         ui.label(name);
///     }
/// });
/// ```
#[must_use = "call `.show(ui, |ui| ...)`"]
pub struct Marquee {
    speed: f32,
    reverse: bool,
    vertical: bool,
    pause_on_hover: bool,
    hover_speed: f32,
    gap: f32,
    fade: f32,
    fade_color: Option<Color32>,
    length: Option<f32>,
    salt: u64,
}

impl Default for Marquee {
    fn default() -> Self {
        Self::new()
    }
}

impl Marquee {
    /// Horizontal marquee moving left at 40 pt/s with faded edges.
    pub fn new() -> Self {
        Self { speed: 40.0, reverse: false, vertical: false, pause_on_hover: true, hover_speed: 0.0, gap: 24.0, fade: 64.0, fade_color: None, length: None, salt: 0 }
    }
    /// Scroll speed in points per second.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s.max(0.0);
        self
    }
    /// Scroll right (or down, when vertical) instead.
    pub fn reverse(mut self, on: bool) -> Self {
        self.reverse = on;
        self
    }
    /// Scroll vertically. Set the visible height with [`Self::length`].
    pub fn vertical(mut self, on: bool) -> Self {
        self.vertical = on;
        self
    }
    /// Smoothly stop while hovered (default on).
    pub fn pause_on_hover(mut self, on: bool) -> Self {
        self.pause_on_hover = on;
        self
    }
    /// Speed multiplier while hovered when pausing is on (0 = full stop, 0.3 = slow down).
    pub fn hover_speed(mut self, f: f32) -> Self {
        self.hover_speed = f.clamp(0.0, 1.0);
        self
    }
    /// Gap between repeated copies of the content.
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g.max(0.0);
        self
    }
    /// Width of the faded edges (0 = no fade).
    pub fn fade(mut self, w: f32) -> Self {
        self.fade = w.max(0.0);
        self
    }
    /// Color the edges fade into. Defaults to the theme card color.
    pub fn fade_color(mut self, c: Color32) -> Self {
        self.fade_color = Some(c);
        self
    }
    /// Visible length: width when horizontal (default: all available), height when vertical
    /// (default: 300).
    pub fn length(mut self, l: f32) -> Self {
        self.length = Some(l.max(10.0));
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }

    /// Show the marquee; `add_contents` lays out one copy of the content.
    pub fn show(self, ui: &mut Ui, mut add_contents: impl FnMut(&mut Ui)) -> Response {
        let id = ui.scope_id().with(("marquee", self.salt));
        // (content length along the axis, cross size)
        let (content, cross): (f32, f32) = glowkit_core::state(ui, id.with("size"), || (0.0, 40.0));
        let size = if self.vertical {
            vec2(ui.available_width(), self.length.unwrap_or(300.0))
        } else {
            vec2(self.length.unwrap_or(ui.available_width()), cross.max(1.0))
        };
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let hovered = self.pause_on_hover && ui.rect_contains_pointer(rect);
        let factor = glowkit_core::anim::follow(ui, id.with("speed"), if hovered { self.hover_speed } else { 1.0 }, 5.0);
        let dt = glowkit_core::dt(ui);
        let period = content + self.gap;
        let dir = if self.reverse { -1.0 } else { 1.0 };
        let offset = glowkit_core::with_state(ui, id.with("off"), || 0.0f32, |o| {
            if period > 1.0 {
                *o = (*o + self.speed * factor * dt * dir).rem_euclid(period);
            }
            *o
        });
        if self.speed > 0.0 {
            glowkit_core::animate(ui);
        }
        let axis_len = if self.vertical { rect.height() } else { rect.width() };
        let copies = if period > 1.0 { ((axis_len / period).ceil() as usize + 2).min(64) } else { 1 };
        let clip = rect.intersect(ui.clip_rect());
        let mut measured = (0.0f32, 0.0f32);
        for k in 0..copies {
            let start = -offset + k as f32 * period;
            let max_rect = if self.vertical {
                Rect::from_min_size(rect.min + vec2(0.0, start), vec2(rect.width(), 100_000.0))
            } else {
                Rect::from_min_size(rect.min + vec2(start, 0.0), vec2(100_000.0, rect.height().max(cross)))
            };
            let layout = if self.vertical { Layout::top_down(Align::Center) } else { Layout::left_to_right(Align::Center) };
            let mut child = ui.new_child(UiBuilder::new().max_rect(max_rect).layout(layout).id_salt(("marquee_copy", k)));
            child.set_clip_rect(clip);
            add_contents(&mut child);
            if k == 0 {
                let r = child.min_rect();
                measured = if self.vertical { (r.height(), r.width()) } else { (r.width(), r.height()) };
            }
        }
        if (measured.0 - content).abs() > 0.5 || (measured.1 - cross).abs() > 0.5 {
            glowkit_core::set_state(ui, id.with("size"), measured);
            ui.ctx().request_repaint();
        }
        if self.fade > 0.0 {
            let bg = self.fade_color.unwrap_or_else(|| Theme::of(ui).card);
            let painter = ui.painter_at(rect);
            let edge = self.fade.min(axis_len / 3.0);
            if self.vertical {
                paint::fade_edges_vertical(&painter, rect, edge, bg);
            } else {
                paint::fade_edges_horizontal(&painter, rect, edge, bg);
            }
        }
        response
    }
}
