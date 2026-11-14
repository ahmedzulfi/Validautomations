use super::kit::{self, Btn};
use super::LineIcon;
use crate::Icon;
use egui::{
    pos2, vec2, Color32, CornerRadius, Id, Key, LayerId, Order, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder,
};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// What happened in a [`Dialog`] or [`Sheet`] this frame.
#[derive(Debug)]
pub struct DialogResponse<R> {
    /// Whatever the body closure returned (`None` while fully closed).
    pub inner: Option<R>,
    /// The primary (confirm) action was clicked.
    pub primary_clicked: bool,
    /// The secondary (cancel) action was clicked.
    pub secondary_clicked: bool,
    /// Closed by Escape, the close button or a backdrop click.
    pub dismissed: bool,
}

impl<R> DialogResponse<R> {
    fn none() -> Self {
        Self { inner: None, primary_clicked: false, secondary_clicked: false, dismissed: false }
    }
}

/// Paints the dimmed backdrop; returns true if it was clicked.
fn backdrop(ui: &Ui, id: Id, area: Rect, clip: Rect, p: f32, blocking: bool, rounded: bool) -> bool {
    let t = Theme::of(ui);
    let r = kit::float_layer(ui.ctx(), id, Order::Foreground, area, clip, 1.0, area.center(), p, |ui| {
        let resp = ui.interact(area, id.with("hit"), if blocking { Sense::click() } else { Sense::hover() });
        ui.painter().rect_filled(area, if rounded { t.radius } else { 0.0 }, kit::backdrop_color(&t));
        resp.clicked()
    });
    ui.ctx().move_to_top(LayerId::new(Order::Foreground, id));
    r.inner
}

fn footer_buttons(
    ui: &mut Ui,
    right: egui::Pos2,
    id: Id,
    primary: Option<&str>,
    secondary: Option<&str>,
    destructive: bool,
) -> (bool, bool) {
    let mut x = right.x;
    let (mut pc, mut sc) = (false, false);
    if let Some(label) = primary {
        let w = kit::button_width(ui, label, 13.5);
        let r = Rect::from_min_max(pos2(x - w, right.y - 34.0), pos2(x, right.y));
        pc =
            kit::button_at(ui, r, id.with("primary"), label, if destructive { Btn::Destructive } else { Btn::Primary })
                .clicked();
        x -= w + 8.0;
    }
    if let Some(label) = secondary {
        let w = kit::button_width(ui, label, 13.5);
        let r = Rect::from_min_max(pos2(x - w, right.y - 34.0), pos2(x, right.y));
        sc = kit::button_at(ui, r, id.with("secondary"), label, Btn::Outline).clicked();
    }
    (pc, sc)
}

/// A modal dialog: the page dims behind a fading backdrop while a card springs in from 95%
/// scale. Has a title, optional description, any body content and up to two footer actions.
/// Closes on Escape, on the close button and on a backdrop click.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut open = true;
/// use glowkit_components::Dialog;
/// let r = Dialog::new(&mut open, "Edit profile")
///     .description("Make changes to your profile here. Click save when you're done.")
///     .primary("Save changes")
///     .secondary("Cancel")
///     .show(ui, |ui| { ui.label("Body goes here"); });
/// if r.primary_clicked { /* save */ }
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct Dialog<'a> {
    open: &'a mut bool,
    title: String,
    description: Option<String>,
    width: f32,
    primary: Option<String>,
    secondary: Option<String>,
    destructive: bool,
    within: Option<Rect>,
    close_button: bool,
    dismissable: bool,
    icon: Option<(Icon, Option<Color32>)>,
    id: Option<Id>,
}

impl<'a> Dialog<'a> {
    /// A dialog controlled by `open`, with a title.
    pub fn new(open: &'a mut bool, title: impl Into<String>) -> Self {
        Self {
            open,
            title: title.into(),
            description: None,
            width: 440.0,
            primary: None,
            secondary: None,
            destructive: false,
            within: None,
            close_button: true,
            dismissable: true,
            icon: None,
            id: None,
        }
    }
    /// Muted text under the title.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }
    /// Card width in points (default 440).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
    /// Label of the primary (right-most) footer button. Clicking it closes the dialog.
    pub fn primary(mut self, label: impl Into<String>) -> Self {
        self.primary = Some(label.into());
        self
    }
    /// Label of the secondary (outline) footer button. Clicking it closes the dialog.
    pub fn secondary(mut self, label: impl Into<String>) -> Self {
        self.secondary = Some(label.into());
        self
    }
    /// Style the primary action as destructive (red).
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self
    }
    /// Confine the backdrop and dialog to `rect` instead of the whole window (handy for previews).
    pub fn within(mut self, rect: Rect) -> Self {
        self.within = Some(rect);
        self
    }
    /// Show the X button in the top-right corner (default true).
    pub fn close_button(mut self, show: bool) -> Self {
        self.close_button = show;
        self
    }
    /// Allow closing with Escape / backdrop click (default true).
    pub fn dismissable(mut self, dismissable: bool) -> Self {
        self.dismissable = dismissable;
        self
    }
    /// Show an icon in a tinted circle above the title. `None` color uses the accent (or red when destructive).
    pub fn icon(mut self, icon: impl Into<Icon>, color: Option<Color32>) -> Self {
        let icon: Icon = icon.into();
        self.icon = Some((icon, color));
        self
    }
    /// Explicit id (defaults to one derived from the parent `Ui` and the title).
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Show the dialog (call every frame). `add` builds the body between description and footer.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> DialogResponse<R> {
        let t = Theme::of(ui);
        let id = self.id.unwrap_or_else(|| ui.scope_id().with(("glowkit_dialog", &self.title)));
        let ctx = ui.ctx().clone();
        if *self.open && self.dismissable && ctx.input(|i| i.key_pressed(Key::Escape)) {
            *self.open = false;
            let mut r = DialogResponse::none();
            r.dismissed = true;
            return r;
        }
        let p = kit::presence(ui, id, *self.open, 14.0);
        let pop = kit::pop_spring(ui, id, *self.open);
        if p < 0.004 && !*self.open {
            return DialogResponse::none();
        }
        let area = kit::screen(&ctx, self.within);
        let clip = if self.within.is_some() { area.intersect(ui.clip_rect()) } else { area };
        let mut out = DialogResponse::none();
        if backdrop(ui, id.with("backdrop"), area, clip, p, *self.open, self.within.is_some()) && self.dismissable {
            *self.open = false;
            out.dismissed = true;
        }

        let pad = 24.0;
        let w = self.width.min(area.width() - 32.0);
        let inner_w = w - pad * 2.0;
        let title_g =
            kit::wrap(ui, &self.title, 18.0, t.foreground, inner_w - if self.close_button { 24.0 } else { 0.0 });
        let desc_g = self.description.as_ref().map(|d| kit::wrap(ui, d, 14.0, t.muted_foreground, inner_w));
        let body_h: f32 = glowkit_core::state(ui, id.with("body_h"), || 0.0);
        let icon_h = if self.icon.is_some() { 52.0 } else { 0.0 };
        let has_footer = self.primary.is_some() || self.secondary.is_some();
        let mut h = pad + icon_h + title_g.size().y;
        if let Some(d) = &desc_g {
            h += 6.0 + d.size().y;
        }
        if body_h > 0.0 {
            h += 18.0 + body_h;
        }
        if has_footer {
            h += 24.0 + 34.0;
        }
        h += pad;
        let rect = Rect::from_center_size(area.center() + vec2(0.0, (1.0 - p) * 10.0), vec2(w, h));
        let scale = 0.95 + 0.05 * pop;
        let lid = id.with("panel");
        let destructive = self.destructive;
        let primary = self.primary.clone();
        let secondary = self.secondary.clone();
        let close_button = self.close_button;
        let icon = self.icon;
        let res = kit::float_layer(&ctx, lid, Order::Foreground, rect, clip, scale, rect.center(), p, |ui| {
            ui.interact(rect, lid.with("block"), Sense::click());
            kit::panel(ui.painter(), rect, &t, t.radius * 1.2);
            let mut y = rect.min.y + pad;
            let x = rect.min.x + pad;
            if let Some((ic, col)) = icon {
                let col = col.unwrap_or(if destructive { t.destructive } else { t.accent });
                let c = pos2(x + 20.0, y + 20.0);
                ui.painter().circle_filled(c, 20.0, with_alpha(col, 0.14));
                ic.paint(ui.painter(), Rect::from_center_size(c, vec2(20.0, 20.0)), col);
                y += icon_h;
            }
            ui.painter().galley(pos2(x, y), title_g.clone(), t.foreground);
            y += title_g.size().y;
            if let Some(d) = &desc_g {
                y += 6.0;
                ui.painter().galley(pos2(x, y), d.clone(), t.muted_foreground);
                y += d.size().y;
            }
            let mut closed = false;
            if close_button {
                let cr = Rect::from_min_size(pos2(rect.max.x - 40.0, rect.min.y + 14.0), vec2(26.0, 26.0));
                closed = kit::icon_button(ui, cr, lid.with("close"), LineIcon::X, t.muted_foreground).clicked();
            }
            let body_top = y + 18.0;
            let mut body = ui.new_child(
                UiBuilder::new()
                    .max_rect(Rect::from_min_max(pos2(x, body_top), pos2(rect.max.x - pad, rect.max.y + 2000.0))),
            );
            body.set_clip_rect(ui.clip_rect());
            let inner = add(&mut body);
            let used = body.min_rect().height();
            let used = if body.min_rect().width() <= 0.5 && used <= 0.5 { 0.0 } else { used };
            let (pc, sc) = footer_buttons(
                ui,
                pos2(rect.max.x - pad, rect.max.y - pad),
                lid,
                primary.as_deref(),
                secondary.as_deref(),
                destructive,
            );
            (inner, used, closed, pc, sc)
        });
        let (inner, used, closed, pc, sc) = res.inner;
        ctx.move_to_top(LayerId::new(Order::Foreground, lid));
        if (used - body_h).abs() > 0.5 {
            glowkit_core::set_state(ui, id.with("body_h"), used);
            ctx.request_repaint();
        }
        out.inner = Some(inner);
        if closed {
            out.dismissed = true;
            *self.open = false;
        }
        if pc {
            out.primary_clicked = true;
            *self.open = false;
        }
        if sc {
            out.secondary_clicked = true;
            *self.open = false;
        }
        out
    }
}

/// The result of an [`AlertDialog`] this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct AlertResponse {
    /// The confirm action was clicked.
    pub confirmed: bool,
    /// The cancel action was clicked.
    pub cancelled: bool,
}

/// A blocking confirmation dialog ("Are you absolutely sure?") with Cancel / Continue actions.
/// Unlike [`Dialog`] it can't be dismissed by clicking the backdrop. Supports a destructive
/// style with a warning icon.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut open = true;
/// use glowkit_components::AlertDialog;
/// let r = AlertDialog::new(&mut open, "Delete project?")
///     .description("This action cannot be undone.")
///     .confirm("Delete")
///     .destructive(true)
///     .show(ui);
/// if r.confirmed { /* delete */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct AlertDialog<'a> {
    open: &'a mut bool,
    title: String,
    description: Option<String>,
    confirm: String,
    cancel: String,
    destructive: bool,
    within: Option<Rect>,
    icon: bool,
}

impl<'a> AlertDialog<'a> {
    /// A confirmation dialog controlled by `open`.
    pub fn new(open: &'a mut bool, title: impl Into<String>) -> Self {
        Self {
            open,
            title: title.into(),
            description: None,
            confirm: "Continue".into(),
            cancel: "Cancel".into(),
            destructive: false,
            within: None,
            icon: true,
        }
    }
    /// Explanation under the title.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }
    /// Confirm button label (default "Continue").
    pub fn confirm(mut self, label: impl Into<String>) -> Self {
        self.confirm = label.into();
        self
    }
    /// Cancel button label (default "Cancel").
    pub fn cancel(mut self, label: impl Into<String>) -> Self {
        self.cancel = label.into();
        self
    }
    /// Red confirm button and warning icon.
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self
    }
    /// Show the leading icon (default true).
    pub fn icon(mut self, show: bool) -> Self {
        self.icon = show;
        self
    }
    /// Confine to `rect` instead of the whole window.
    pub fn within(mut self, rect: Rect) -> Self {
        self.within = Some(rect);
        self
    }
    /// Show the dialog (call every frame).
    pub fn show(self, ui: &mut Ui) -> AlertResponse {
        let mut d = Dialog::new(self.open, self.title)
            .width(420.0)
            .primary(self.confirm)
            .secondary(self.cancel)
            .destructive(self.destructive)
            .close_button(false)
            .dismissable(false);
        if let Some(desc) = self.description {
            d = d.description(desc);
        }
        if let Some(w) = self.within {
            d = d.within(w);
        }
        if self.icon {
            d = d.icon(if self.destructive { LineIcon::Warning } else { LineIcon::Info }, None);
        }
        let r = d.show(ui, |_| ());
        AlertResponse { confirmed: r.primary_clicked, cancelled: r.secondary_clicked }
    }
}

/// Which edge a [`Sheet`] slides in from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum SheetSide {
    /// Slide in from the left edge.
    Left,
    /// Slide in from the right edge (default).
    #[default]
    Right,
    /// Drop down from the top edge.
    Top,
    /// Rise from the bottom edge (a drawer).
    Bottom,
}

/// A side panel (sheet / drawer) that slides in from any edge on a spring over a dimmed
/// backdrop. Bottom sheets get a grab handle, like a mobile drawer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut open = true;
/// use glowkit_components::{Sheet, SheetSide};
/// Sheet::new(&mut open, "Settings")
///     .side(SheetSide::Right)
///     .description("Tune your workspace.")
///     .show(ui, |ui| { ui.label("Content"); });
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct Sheet<'a> {
    open: &'a mut bool,
    title: String,
    description: Option<String>,
    side: SheetSide,
    size: f32,
    within: Option<Rect>,
    primary: Option<String>,
    secondary: Option<String>,
}

impl<'a> Sheet<'a> {
    /// A sheet controlled by `open`, with a title.
    pub fn new(open: &'a mut bool, title: impl Into<String>) -> Self {
        Self {
            open,
            title: title.into(),
            description: None,
            side: SheetSide::Right,
            size: 360.0,
            within: None,
            primary: None,
            secondary: None,
        }
    }
    /// Muted text under the title.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }
    /// Edge to slide from.
    pub fn side(mut self, side: SheetSide) -> Self {
        self.side = side;
        self
    }
    /// Width (left/right) or height (top/bottom) in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Confine to `rect` instead of the whole window.
    pub fn within(mut self, rect: Rect) -> Self {
        self.within = Some(rect);
        self
    }
    /// Primary footer button (closes the sheet).
    pub fn primary(mut self, label: impl Into<String>) -> Self {
        self.primary = Some(label.into());
        self
    }
    /// Secondary footer button (closes the sheet).
    pub fn secondary(mut self, label: impl Into<String>) -> Self {
        self.secondary = Some(label.into());
        self
    }

    /// Show the sheet (call every frame).
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> DialogResponse<R> {
        let t = Theme::of(ui);
        let id = ui.scope_id().with(("glowkit_sheet", &self.title, self.side));
        let ctx = ui.ctx().clone();
        if *self.open && ctx.input(|i| i.key_pressed(Key::Escape)) {
            *self.open = false;
            let mut r = DialogResponse::none();
            r.dismissed = true;
            return r;
        }
        let p = kit::presence(ui, id, *self.open, 12.0);
        let s = glowkit_core::anim::spring(ui, id.with("slide"), if *self.open { 1.0 } else { 0.0 }, 260.0, 28.0);
        if p < 0.004 && !*self.open {
            return DialogResponse::none();
        }
        let area = kit::screen(&ctx, self.within);
        let clip = if self.within.is_some() { area.intersect(ui.clip_rect()) } else { area };
        let mut out = DialogResponse::none();
        if backdrop(ui, id.with("backdrop"), area, clip, p, *self.open, self.within.is_some()) {
            *self.open = false;
            out.dismissed = true;
        }
        let horizontal = matches!(self.side, SheetSide::Left | SheetSide::Right);
        let size = if horizontal { self.size.min(area.width() * 0.9) } else { self.size.min(area.height() * 0.9) };
        let hidden = (1.0 - s) * (size + 30.0);
        let rect = match self.side {
            SheetSide::Right => {
                Rect::from_min_max(pos2(area.max.x - size + hidden, area.min.y), pos2(area.max.x + hidden, area.max.y))
            }
            SheetSide::Left => {
                Rect::from_min_max(pos2(area.min.x - hidden, area.min.y), pos2(area.min.x + size - hidden, area.max.y))
            }
            SheetSide::Top => {
                Rect::from_min_max(pos2(area.min.x, area.min.y - hidden), pos2(area.max.x, area.min.y + size - hidden))
            }
            SheetSide::Bottom => {
                Rect::from_min_max(pos2(area.min.x, area.max.y - size + hidden), pos2(area.max.x, area.max.y + hidden))
            }
        };
        // extend a little past the edge so spring overshoot never shows a gap
        let paint_rect = if self.within.is_some() {
            rect
        } else {
            match self.side {
                SheetSide::Right => rect.with_max_x(rect.max.x + 40.0),
                SheetSide::Left => rect.with_min_x(rect.min.x - 40.0),
                SheetSide::Top => rect.with_min_y(rect.min.y - 40.0),
                SheetSide::Bottom => rect.with_max_y(rect.max.y + 40.0),
            }
        };
        let r = t.radius.round().clamp(0.0, 255.0) as u8;
        let corners = if self.within.is_some() {
            CornerRadius::same(r)
        } else {
            match self.side {
                SheetSide::Right => CornerRadius { nw: r, sw: r, ne: 0, se: 0 },
                SheetSide::Left => CornerRadius { ne: r, se: r, nw: 0, sw: 0 },
                SheetSide::Top => CornerRadius { sw: r, se: r, nw: 0, ne: 0 },
                SheetSide::Bottom => CornerRadius { nw: r, ne: r, sw: 0, se: 0 },
            }
        };
        let lid = id.with("panel");
        let title = self.title.clone();
        let desc = self.description.clone();
        let (primary, secondary) = (self.primary.clone(), self.secondary.clone());
        let side = self.side;
        let res = kit::float_layer(
            &ctx,
            lid,
            Order::Foreground,
            paint_rect,
            clip,
            1.0,
            rect.center(),
            p.clamp(0.6, 1.0),
            |ui| {
                ui.interact(rect, lid.with("block"), Sense::click());
                let pnt = ui.painter();
                glowkit_core::paint::soft_shadow(
                    pnt,
                    paint_rect,
                    t.radius,
                    30.0,
                    vec2(0.0, 0.0),
                    kit::shadow_color(&t),
                );
                pnt.rect_filled(paint_rect, corners, t.popover);
                pnt.rect_stroke(paint_rect, corners, Stroke::new(1.0, t.border), StrokeKind::Inside);
                let pad = 24.0;
                let mut y = rect.min.y + pad;
                if side == SheetSide::Bottom {
                    let hr = Rect::from_center_size(pos2(rect.center().x, rect.min.y + 12.0), vec2(44.0, 5.0));
                    pnt.rect_filled(hr, 3.0, t.muted_foreground.gamma_multiply(0.45));
                    y += 6.0;
                }
                let x = rect.min.x + pad;
                let inner_w = rect.width() - pad * 2.0;
                let tg = kit::wrap(ui, &title, 18.0, t.foreground, inner_w - 30.0);
                pnt.galley(pos2(x, y), tg.clone(), t.foreground);
                y += tg.size().y;
                if let Some(d) = &desc {
                    let dg = kit::wrap(ui, d, 14.0, t.muted_foreground, inner_w);
                    y += 6.0;
                    pnt.galley(pos2(x, y), dg.clone(), t.muted_foreground);
                    y += dg.size().y;
                }
                let cr = Rect::from_min_size(pos2(rect.max.x - 42.0, rect.min.y + 16.0), vec2(26.0, 26.0));
                let closed = kit::icon_button(ui, cr, lid.with("close"), LineIcon::X, t.muted_foreground).clicked();
                let footer_h = if primary.is_some() || secondary.is_some() { 34.0 + pad } else { 0.0 };
                let body_rect =
                    Rect::from_min_max(pos2(x, y + 20.0), pos2(rect.max.x - pad, rect.max.y - pad - footer_h));
                let mut body = ui.new_child(UiBuilder::new().max_rect(body_rect));
                body.set_clip_rect(body_rect.expand(4.0).intersect(ui.clip_rect()));
                let inner = add(&mut body);
                let (pc, sc) = footer_buttons(
                    ui,
                    pos2(rect.max.x - pad, rect.max.y - pad),
                    lid,
                    primary.as_deref(),
                    secondary.as_deref(),
                    false,
                );
                (inner, closed, pc, sc)
            },
        );
        ctx.move_to_top(LayerId::new(Order::Foreground, lid));
        let (inner, closed, pc, sc) = res.inner;
        out.inner = Some(inner);
        if closed {
            out.dismissed = true;
            *self.open = false;
        }
        if pc {
            out.primary_clicked = true;
            *self.open = false;
        }
        if sc {
            out.secondary_clicked = true;
            *self.open = false;
        }
        out
    }
}
