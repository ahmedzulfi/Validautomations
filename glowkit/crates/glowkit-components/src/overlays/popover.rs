use super::kit;
use egui::{pos2, vec2, FontId, Order, Rect, Response, Vec2};
use glowkit_core::Theme;

/// Which side of its anchor a floating element appears on. Flips automatically when there is
/// no room.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Placement {
    /// Above the anchor.
    #[default]
    Top,
    /// Below the anchor.
    Bottom,
    /// Left of the anchor.
    Left,
    /// Right of the anchor.
    Right,
}

impl Placement {
    fn flip(self) -> Self {
        match self {
            Placement::Top => Placement::Bottom,
            Placement::Bottom => Placement::Top,
            Placement::Left => Placement::Right,
            Placement::Right => Placement::Left,
        }
    }
    /// Unit vector pointing from the anchor toward the floating element.
    pub(crate) fn dir(self) -> Vec2 {
        match self {
            Placement::Top => vec2(0.0, -1.0),
            Placement::Bottom => vec2(0.0, 1.0),
            Placement::Left => vec2(-1.0, 0.0),
            Placement::Right => vec2(1.0, 0.0),
        }
    }
}

fn raw_place(anchor: Rect, size: Vec2, p: Placement, gap: f32) -> Rect {
    match p {
        Placement::Top => {
            Rect::from_min_size(pos2(anchor.center().x - size.x / 2.0, anchor.min.y - gap - size.y), size)
        }
        Placement::Bottom => Rect::from_min_size(pos2(anchor.center().x - size.x / 2.0, anchor.max.y + gap), size),
        Placement::Left => {
            Rect::from_min_size(pos2(anchor.min.x - gap - size.x, anchor.center().y - size.y / 2.0), size)
        }
        Placement::Right => Rect::from_min_size(pos2(anchor.max.x + gap, anchor.center().y - size.y / 2.0), size),
    }
}

/// Position a floating rect of `size` next to `anchor`, flipping if it would leave `bounds`.
pub(crate) fn place(anchor: Rect, size: Vec2, p: Placement, gap: f32, bounds: Rect) -> (Rect, Placement) {
    let mut r = raw_place(anchor, size, p, gap);
    let mut used = p;
    if !bounds.shrink(4.0).contains_rect(r) {
        let f = raw_place(anchor, size, p.flip(), gap);
        let fits = |x: Rect| match p {
            Placement::Top | Placement::Bottom => x.min.y >= bounds.min.y && x.max.y <= bounds.max.y,
            _ => x.min.x >= bounds.min.x && x.max.x <= bounds.max.x,
        };
        if !fits(r) && fits(f) {
            r = f;
            used = p.flip();
        }
    }
    (kit::clamp_into(r, bounds, 6.0), used)
}

/// Paint the arrow of a floating panel pointing at `anchor`.
fn panel_arrow(
    painter: &egui::Painter,
    panel: Rect,
    anchor: Rect,
    used: Placement,
    fill: egui::Color32,
    border: egui::Color32,
    size: f32,
) {
    let d = used.dir();
    let tip = match used {
        Placement::Top => pos2(anchor.center().x.clamp(panel.min.x + 14.0, panel.max.x - 14.0), panel.max.y + size),
        Placement::Bottom => pos2(anchor.center().x.clamp(panel.min.x + 14.0, panel.max.x - 14.0), panel.min.y - size),
        Placement::Left => pos2(panel.max.x + size, anchor.center().y.clamp(panel.min.y + 12.0, panel.max.y - 12.0)),
        Placement::Right => pos2(panel.min.x - size, anchor.center().y.clamp(panel.min.y + 12.0, panel.max.y - 12.0)),
    };
    kit::arrow(painter, tip, -d, size, fill, border);
}

/// A small label that fades and scales in next to a widget after a short hover delay, with a
/// little arrow pointing at it. Uses an inverted (primary) color scheme like shadcn/ui.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::{Placement, Tooltip};
/// let r = ui.button("Hover me");
/// Tooltip::new("Add to library").placement(Placement::Bottom).show(&r);
/// ```
#[must_use = "call `.show(&response)`"]
pub struct Tooltip {
    text: String,
    placement: Placement,
    delay: f32,
    arrow: bool,
    force_open: bool,
}

impl Tooltip {
    /// A tooltip with the given text.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), placement: Placement::Top, delay: 0.35, arrow: true, force_open: false }
    }
    /// Preferred side (flips when there is no room).
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }
    /// Hover delay in seconds (default 0.35).
    pub fn delay(mut self, seconds: f32) -> Self {
        self.delay = seconds.max(0.0);
        self
    }
    /// Draw the pointer arrow (default true).
    pub fn arrow(mut self, arrow: bool) -> Self {
        self.arrow = arrow;
        self
    }
    /// Keep it visible regardless of hover (for demos and onboarding).
    pub fn always_open(mut self, open: bool) -> Self {
        self.force_open = open;
        self
    }
    /// Attach to a widget's response (call every frame after adding the widget).
    pub fn show(self, response: &Response) {
        let ctx = &response.ctx;
        let id = response.id.with("glowkit_tooltip");
        let now = ctx.input(|i| i.time);
        let hovered = response.hovered();
        let since: Option<f64> = ctx.data_mut(|d| {
            let s = d.get_temp_mut_or_insert_with(id.with("since"), || None::<f64>);
            if hovered {
                s.get_or_insert(now);
            } else {
                *s = None;
            }
            *s
        });
        let mut open = self.force_open;
        if let Some(s) = since {
            let waited = (now - s) as f32;
            if waited >= self.delay {
                open = true;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_secs_f32(self.delay - waited + 0.01));
            }
        }
        let p = kit::follow_ctx(ctx, id.with("p"), if open { 1.0 } else { 0.0 }, 20.0);
        if p < 0.01 {
            return;
        }
        let t = Theme::current(ctx);
        let font = FontId::proportional(12.5);
        let galley = ctx.fonts_mut(|f| f.layout(self.text.clone(), font, t.primary_foreground, 260.0));
        let size = galley.size() + vec2(20.0, 12.0);
        let bounds = ctx.content_rect();
        let gap = if self.arrow { 8.0 } else { 6.0 };
        let (rect, used) = place(response.rect, size, self.placement, gap, bounds);
        let rect = rect.translate(-used.dir() * (1.0 - p) * 5.0);
        let origin = rect.center() - used.dir() * rect.size() * 0.5;
        let arrow = self.arrow;
        let anchor = response.rect;
        kit::float_layer(ctx, id, Order::Tooltip, rect.expand(8.0), bounds, 0.92 + 0.08 * p, origin, p, |ui| {
            let pnt = ui.painter();
            glowkit_core::paint::soft_shadow(
                pnt,
                rect,
                6.0,
                10.0,
                vec2(0.0, 3.0),
                egui::Color32::from_black_alpha(if t.dark { 90 } else { 40 }),
            );
            pnt.rect_filled(rect, 6.0, t.primary);
            if arrow {
                panel_arrow(pnt, rect, anchor, used, t.primary, t.primary, 5.0);
            }
            pnt.galley(rect.min + vec2(10.0, 6.0), galley, t.primary_foreground);
        });
    }
}

/// Shorthand: attach a [`Tooltip`] with default settings to `response`.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_components::tooltip(&ui.button("?"), "Help center");
/// ```
pub fn tooltip(response: &Response, text: impl Into<String>) {
    Tooltip::new(text).show(response);
}
