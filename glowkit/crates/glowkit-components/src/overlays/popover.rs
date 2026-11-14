use super::kit;
use egui::{pos2, vec2, Context, FontId, Id, Order, Rect, Response, Sense, Ui, UiBuilder, Vec2};
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

/// Shows an anchored, animated floating card. Returns the inner result and the panel rect.
#[allow(clippy::too_many_arguments)]
pub(crate) fn anchored_card<R>(
    ctx: &Context,
    id: Id,
    anchor: Rect,
    p: f32,
    pop: f32,
    placement: Placement,
    width: f32,
    arrow: bool,
    bounds: Rect,
    clip: Rect,
    add: impl FnOnce(&mut Ui) -> R,
) -> (R, Rect) {
    let t = Theme::current(ctx);
    let pad = 16.0;
    let h: f32 = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(id.with("card_h"), || 60.0));
    let gap = if arrow { 10.0 } else { 6.0 };
    let (rect, used) = place(anchor, vec2(width, h), placement, gap, bounds);
    let slide = -used.dir() * (1.0 - p) * 6.0;
    let rect = rect.translate(slide);
    let origin = rect.center() - used.dir() * rect.size() * 0.5;
    let scale = 0.94 + 0.06 * pop;
    let res = kit::float_layer(ctx, id, Order::Foreground, rect.expand(12.0), clip, scale, origin, p, |ui| {
        ui.interact(rect, id.with("block"), Sense::click());
        kit::panel(ui.painter(), rect, &t, t.radius);
        if arrow {
            panel_arrow(ui.painter(), rect, anchor, used, t.popover, t.border, 6.0);
        }
        let inner_rect = Rect::from_min_max(rect.min + vec2(pad, pad), pos2(rect.max.x - pad, rect.max.y + 1000.0));
        let mut child = ui.new_child(UiBuilder::new().max_rect(inner_rect));
        child.set_clip_rect(rect.intersect(clip));
        let r = add(&mut child);
        (r, child.min_rect().height())
    });
    let (r, used_h) = res.inner;
    let new_h = (used_h + pad * 2.0).max(24.0);
    if (new_h - h).abs() > 0.5 {
        ctx.data_mut(|d| d.insert_temp(id.with("card_h"), new_h));
        ctx.request_repaint();
    }
    ctx.move_to_top(egui::LayerId::new(Order::Foreground, id));
    (r, rect)
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

fn popover_open_id(trigger: Id) -> Id {
    trigger.with("glowkit_popover_open")
}

/// A floating card anchored to a trigger widget: click the trigger to toggle it, click outside
/// or press Escape to close. Scales in from the trigger side with a small arrow.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::Popover;
/// let trigger = ui.button("Open popover");
/// Popover::new().width(300.0).show(&trigger, |ui| {
///     ui.label("Dimensions");
/// });
/// ```
#[must_use = "call `.show(&trigger, ...)`"]
pub struct Popover {
    placement: Placement,
    width: f32,
    arrow: bool,
    within: Option<Rect>,
}

impl Default for Popover {
    fn default() -> Self {
        Self::new()
    }
}

impl Popover {
    /// A bottom-placed popover, 280 points wide.
    pub fn new() -> Self {
        Self { placement: Placement::Bottom, width: 280.0, arrow: true, within: None }
    }
    /// Preferred side.
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }
    /// Card width in points.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
    /// Draw the pointer arrow (default true).
    pub fn arrow(mut self, arrow: bool) -> Self {
        self.arrow = arrow;
        self
    }
    /// Keep the card inside `rect` (and clip it there).
    pub fn within(mut self, rect: Rect) -> Self {
        self.within = Some(rect);
        self
    }
    /// Is the popover attached to `trigger_id` open?
    pub fn is_open(ctx: &Context, trigger_id: Id) -> bool {
        ctx.data_mut(|d| d.get_temp::<bool>(popover_open_id(trigger_id)).unwrap_or(false))
    }
    /// Open or close the popover attached to `trigger_id`.
    pub fn set_open(ctx: &Context, trigger_id: Id, open: bool) {
        ctx.data_mut(|d| d.insert_temp(popover_open_id(trigger_id), open));
    }
    /// Show the popover for `trigger` (toggles on click). Returns the body's result while visible.
    pub fn show<R>(self, trigger: &Response, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let ctx = trigger.ctx.clone();
        let tid = trigger.id;
        let mut open = Self::is_open(&ctx, tid);
        if trigger.clicked() {
            open = !open;
        }
        let id = tid.with("glowkit_popover");
        let last_rect: Option<Rect> = ctx.data_mut(|d| d.get_temp(id.with("rect")));
        if open && !trigger.clicked() {
            let (pressed, pos, esc) =
                ctx.input(|i| (i.pointer.any_pressed(), i.pointer.interact_pos(), i.key_pressed(egui::Key::Escape)));
            if esc {
                open = false;
            }
            if pressed {
                if let (Some(pos), Some(r)) = (pos, last_rect) {
                    if !r.contains(pos) && !trigger.rect.contains(pos) {
                        open = false;
                    }
                }
            }
        }
        Self::set_open(&ctx, tid, open);
        let p = kit::follow_ctx(&ctx, id.with("p"), if open { 1.0 } else { 0.0 }, 16.0);
        let pop = kit::spring_ctx(&ctx, id.with("pop"), if open { 1.0 } else { 0.0 }, 420.0, 28.0);
        if p < 0.01 && !open {
            return None;
        }
        let bounds = self.within.unwrap_or_else(|| ctx.content_rect());
        let clip = match self.within {
            Some(w) => w.expand(30.0),
            None => bounds,
        };
        let (r, rect) =
            anchored_card(&ctx, id, trigger.rect, p, pop, self.placement, self.width, self.arrow, bounds, clip, add);
        ctx.data_mut(|d| d.insert_temp(id.with("rect"), rect));
        Some(r)
    }
}

/// A rich preview card that appears after hovering a trigger for a moment (like a profile
/// preview on a link), and stays while the pointer moves onto it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::HoverCard;
/// let link = ui.link("@nextjs");
/// HoverCard::new().show(&link, |ui| { ui.label("The React Framework"); });
/// ```
#[must_use = "call `.show(&trigger, ...)`"]
pub struct HoverCard {
    placement: Placement,
    width: f32,
    open_delay: f32,
    close_delay: f32,
    within: Option<Rect>,
    force_open: bool,
}

impl Default for HoverCard {
    fn default() -> Self {
        Self::new()
    }
}

impl HoverCard {
    /// A bottom-placed hover card, 320 points wide.
    pub fn new() -> Self {
        Self {
            placement: Placement::Bottom,
            width: 320.0,
            open_delay: 0.5,
            close_delay: 0.25,
            within: None,
            force_open: false,
        }
    }
    /// Preferred side.
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }
    /// Card width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
    /// Seconds of hovering before it opens (default 0.5).
    pub fn open_delay(mut self, seconds: f32) -> Self {
        self.open_delay = seconds;
        self
    }
    /// Seconds before it closes once the pointer leaves (default 0.25).
    pub fn close_delay(mut self, seconds: f32) -> Self {
        self.close_delay = seconds;
        self
    }
    /// Keep the card inside `rect`.
    pub fn within(mut self, rect: Rect) -> Self {
        self.within = Some(rect);
        self
    }
    /// Keep it open regardless of hover (for demos).
    pub fn always_open(mut self, open: bool) -> Self {
        self.force_open = open;
        self
    }
    /// Show the card for `trigger`. Returns the body's result while visible.
    pub fn show<R>(self, trigger: &Response, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let ctx = trigger.ctx.clone();
        let id = trigger.id.with("glowkit_hovercard");
        let now = ctx.input(|i| i.time);
        let card_rect: Option<Rect> = ctx.data_mut(|d| d.get_temp(id.with("rect")));
        let pointer = ctx.pointer_hover_pos();
        let over = trigger.hovered()
            || (pointer.is_some_and(|p| card_rect.is_some_and(|r| r.expand(4.0).contains(p)))
                && ctx.data_mut(|d| d.get_temp::<bool>(id.with("open")).unwrap_or(false)));
        let (mut since, mut left, mut open) = ctx.data_mut(|d| {
            (
                d.get_temp::<Option<f64>>(id.with("since")).flatten(),
                d.get_temp::<Option<f64>>(id.with("left")).flatten(),
                d.get_temp::<bool>(id.with("open")).unwrap_or(false),
            )
        });
        if over {
            left = None;
            let s = *since.get_or_insert(now);
            if !open {
                let waited = (now - s) as f32;
                if waited >= self.open_delay {
                    open = true;
                } else {
                    ctx.request_repaint_after(std::time::Duration::from_secs_f32(self.open_delay - waited + 0.01));
                }
            }
        } else {
            since = None;
            if open {
                let l = *left.get_or_insert(now);
                let waited = (now - l) as f32;
                if waited >= self.close_delay {
                    open = false;
                    left = None;
                } else {
                    ctx.request_repaint_after(std::time::Duration::from_secs_f32(self.close_delay - waited + 0.01));
                }
            }
        }
        ctx.data_mut(|d| {
            d.insert_temp(id.with("since"), since);
            d.insert_temp(id.with("left"), left);
            d.insert_temp(id.with("open"), open);
        });
        let open = open || self.force_open;
        let p = kit::follow_ctx(&ctx, id.with("p"), if open { 1.0 } else { 0.0 }, 16.0);
        let pop = kit::spring_ctx(&ctx, id.with("pop"), if open { 1.0 } else { 0.0 }, 380.0, 26.0);
        if p < 0.01 && !open {
            return None;
        }
        let bounds = self.within.unwrap_or_else(|| ctx.content_rect());
        let clip = match self.within {
            Some(w) => w.expand(30.0),
            None => bounds,
        };
        let (r, rect) =
            anchored_card(&ctx, id, trigger.rect, p, pop, self.placement, self.width, false, bounds, clip, add);
        ctx.data_mut(|d| d.insert_temp(id.with("rect"), rect));
        Some(r)
    }
}
