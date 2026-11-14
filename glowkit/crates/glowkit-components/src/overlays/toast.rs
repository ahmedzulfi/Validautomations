use super::kit;
use super::LineIcon;
use egui::{pos2, vec2, Align2, Color32, Context, CursorIcon, FontId, Id, Order, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{anim::Spring, paint, Theme};

/// Visual flavour of a [`Toast`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ToastKind {
    /// Neutral message without an icon.
    #[default]
    Default,
    /// Green check.
    Success,
    /// Red cross.
    Error,
    /// Blue info.
    Info,
    /// Amber warning.
    Warning,
    /// Spinning loader; never auto-dismisses until updated with [`Toaster::update`].
    Loading,
}

impl ToastKind {
    fn color(self, t: &Theme) -> Color32 {
        match self {
            ToastKind::Default | ToastKind::Loading => t.foreground,
            ToastKind::Success => t.success,
            ToastKind::Error => t.destructive,
            ToastKind::Info => t.info,
            ToastKind::Warning => t.warning,
        }
    }
    fn icon(self) -> Option<LineIcon> {
        match self {
            ToastKind::Default | ToastKind::Loading => None,
            ToastKind::Success => Some(LineIcon::CircleCheck),
            ToastKind::Error => Some(LineIcon::CircleX),
            ToastKind::Info => Some(LineIcon::Info),
            ToastKind::Warning => Some(LineIcon::Warning),
        }
    }
}

/// Handle to a toast, returned by [`toast`]. Use it with [`Toaster::update`] / [`Toaster::dismiss`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToastId(pub u64);

/// A notification for the [`Toaster`] stack. Build one and pass it to [`toast`].
///
/// ```no_run
/// # let ctx: &egui::Context = todo!();
/// use glowkit_components::{toast, Toast};
/// toast(ctx, Toast::success("Event created").description("Sunday, December 03 at 9:00 AM").action("Undo"));
/// ```
#[derive(Clone, Debug)]
pub struct Toast {
    kind: ToastKind,
    title: String,
    description: Option<String>,
    action: Option<String>,
    duration: Option<f32>,
}

impl Toast {
    /// A neutral toast.
    pub fn new(title: impl Into<String>) -> Self {
        Self { kind: ToastKind::Default, title: title.into(), description: None, action: None, duration: Some(4.0) }
    }
    /// A success toast (green check).
    pub fn success(title: impl Into<String>) -> Self {
        Self::new(title).kind(ToastKind::Success)
    }
    /// An error toast (red cross).
    pub fn error(title: impl Into<String>) -> Self {
        Self::new(title).kind(ToastKind::Error)
    }
    /// An info toast (blue).
    pub fn info(title: impl Into<String>) -> Self {
        Self::new(title).kind(ToastKind::Info)
    }
    /// A warning toast (amber).
    pub fn warning(title: impl Into<String>) -> Self {
        Self::new(title).kind(ToastKind::Warning)
    }
    /// A loading toast with a spinner that stays until updated or dismissed.
    pub fn loading(title: impl Into<String>) -> Self {
        Self { duration: None, ..Self::new(title).kind(ToastKind::Loading) }
    }
    /// Change the kind.
    pub fn kind(mut self, kind: ToastKind) -> Self {
        self.kind = kind;
        self
    }
    /// Secondary text under the title.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }
    /// An action button (e.g. "Undo"); clicks are reported by [`Toaster::show`].
    pub fn action(mut self, label: impl Into<String>) -> Self {
        self.action = Some(label.into());
        self
    }
    /// Seconds before auto-dismiss (default 4). `None` keeps it until dismissed.
    pub fn duration(mut self, seconds: Option<f32>) -> Self {
        self.duration = seconds;
        self
    }
}

#[derive(Clone, Debug)]
struct Entry {
    id: ToastId,
    toast: Toast,
    elapsed: f32,
    appear: f32,
    leaving: Option<f32>,
    y: Spring,
    scale: Spring,
    drag_x: f32,
    height: f32,
    rev: u32,
    born: Option<f64>,
}

#[derive(Clone, Debug, Default)]
struct Store {
    next: u64,
    entries: Vec<Entry>,
}

fn store_id() -> Id {
    Id::unique("glowkit_toaster_store")
}

fn with_store<R>(ctx: &Context, f: impl FnOnce(&mut Store) -> R) -> R {
    ctx.data_mut(|d| f(d.get_temp_mut_or_insert_with(store_id(), Store::default)))
}

/// Queue a toast. It appears in the next [`Toaster::show`]. Returns a handle for updates.
///
/// ```no_run
/// # let ctx: &egui::Context = todo!();
/// let id = glowkit_components::toast(ctx, glowkit_components::Toast::loading("Uploading…"));
/// ```
pub fn toast(ctx: &Context, toast: Toast) -> ToastId {
    let id = with_store(ctx, |s| {
        s.next += 1;
        let id = ToastId(s.next);
        s.entries.insert(
            0,
            Entry {
                id,
                toast,
                elapsed: 0.0,
                appear: 0.0,
                leaving: None,
                y: Spring::new(0.0),
                scale: Spring::new(1.0),
                drag_x: 0.0,
                height: 56.0,
                rev: 0,
                born: None,
            },
        );
        id
    });
    ctx.request_repaint();
    id
}

/// Where the toast stack sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ToastPosition {
    /// Top-left corner.
    TopLeft,
    /// Top edge, centered.
    TopCenter,
    /// Top-right corner.
    TopRight,
    /// Bottom-left corner.
    BottomLeft,
    /// Bottom edge, centered.
    BottomCenter,
    /// Bottom-right corner (default).
    #[default]
    BottomRight,
}

/// Renders the toast stack (Sonner style): toasts pile up as a neat deck that fans out when
/// hovered, auto-dismiss with a thin progress line (paused while hovered), can be swiped away
/// sideways, and support success / error / info / warning / loading variants plus an action.
///
/// Call [`Toaster::show`] once per frame (e.g. at the end of your app's `ui`), and push
/// toasts from anywhere with [`toast`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::{toast, Toast, Toaster};
/// if ui.button("Save").clicked() {
///     toast(ui.ctx(), Toast::success("Saved").description("Your changes are live."));
/// }
/// let undo_clicked = Toaster::new().show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct Toaster {
    position: ToastPosition,
    within: Option<Rect>,
    width: f32,
    visible: usize,
    expand: bool,
    rich_colors: bool,
    close_button: bool,
}

impl Default for Toaster {
    fn default() -> Self {
        Self::new()
    }
}

impl Toaster {
    /// A bottom-right toaster.
    pub fn new() -> Self {
        Self {
            position: ToastPosition::BottomRight,
            within: None,
            width: 356.0,
            visible: 3,
            expand: false,
            rich_colors: false,
            close_button: true,
        }
    }
    /// Stack position.
    pub fn position(mut self, position: ToastPosition) -> Self {
        self.position = position;
        self
    }
    /// Confine the stack to `rect` instead of the window.
    pub fn within(mut self, rect: Rect) -> Self {
        self.within = Some(rect);
        self
    }
    /// Toast width in points (default 356).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
    /// How many toasts are visible in the collapsed deck (default 3).
    pub fn visible_toasts(mut self, n: usize) -> Self {
        self.visible = n.max(1);
        self
    }
    /// Always show the stack fanned out (default: only on hover).
    pub fn expand(mut self, expand: bool) -> Self {
        self.expand = expand;
        self
    }
    /// Tint each toast with its kind's color.
    pub fn rich_colors(mut self, rich: bool) -> Self {
        self.rich_colors = rich;
        self
    }
    /// Show a close button on hover (default true).
    pub fn close_button(mut self, show: bool) -> Self {
        self.close_button = show;
        self
    }

    /// Replace the content of an existing toast (e.g. loading → success). Restarts its timer.
    pub fn update(ctx: &Context, id: ToastId, toast: Toast) {
        with_store(ctx, |s| {
            if let Some(e) = s.entries.iter_mut().find(|e| e.id == id) {
                e.toast = toast;
                e.elapsed = 0.0;
                e.rev += 1;
                e.scale.velocity -= 1.2;
            }
        });
        ctx.request_repaint();
    }

    /// Start the exit animation of a toast.
    pub fn dismiss(ctx: &Context, id: ToastId) {
        with_store(ctx, |s| {
            if let Some(e) = s.entries.iter_mut().find(|e| e.id == id) {
                e.leaving.get_or_insert(0.0);
            }
        });
        ctx.request_repaint();
    }

    /// Dismiss every toast.
    pub fn dismiss_all(ctx: &Context) {
        with_store(ctx, |s| {
            s.entries.iter_mut().for_each(|e| {
                e.leaving.get_or_insert(0.0);
            })
        });
        ctx.request_repaint();
    }

    /// Number of toasts currently alive (including ones animating out).
    pub fn count(ctx: &Context) -> usize {
        with_store(ctx, |s| s.entries.len())
    }

    /// Render and animate the stack. Returns the toasts whose action button was clicked.
    pub fn show(self, ui: &mut Ui) -> Vec<ToastId> {
        let ctx = ui.ctx().clone();
        let t = Theme::of(ui);
        let dt = glowkit_core::dt(ui);
        let time = glowkit_core::time(ui);
        let now = ctx.input(|i| i.time);
        let area = kit::screen(&ctx, self.within);
        let clip = if self.within.is_some() { area.intersect(ui.clip_rect()) } else { area };
        let mut entries = with_store(&ctx, |s| s.entries.clone());
        if entries.is_empty() {
            return Vec::new();
        }
        let id = Id::unique("glowkit_toaster_layer");
        let bottom = matches!(
            self.position,
            ToastPosition::BottomLeft | ToastPosition::BottomCenter | ToastPosition::BottomRight
        );
        let w = self.width.min(area.width() - 32.0);
        let margin = 18.0;
        let x0 = match self.position {
            ToastPosition::TopLeft | ToastPosition::BottomLeft => area.min.x + margin,
            ToastPosition::TopCenter | ToastPosition::BottomCenter => area.center().x - w / 2.0,
            _ => area.max.x - margin - w,
        };
        let anchor_y = if bottom { area.max.y - margin } else { area.min.y + margin };
        let dir = if bottom { -1.0 } else { 1.0 };

        // measure
        let pad = 16.0;
        for e in &mut entries {
            let icon_w = if e.toast.kind == ToastKind::Default { 0.0 } else { 26.0 };
            let act_w = e.toast.action.as_ref().map(|a| kit::button_width(ui, a, 12.5) - 8.0 + 10.0).unwrap_or(0.0);
            let tw = w - pad * 2.0 - icon_w - act_w;
            let tg = kit::wrap(ui, &e.toast.title, 13.5, t.foreground, tw);
            let mut h = tg.size().y;
            if let Some(d) = &e.toast.description {
                h += 3.0 + kit::wrap(ui, d, 12.8, t.muted_foreground, tw).size().y;
            }
            e.height = (h + pad * 2.0 - 4.0).max(52.0);
        }

        let hovered_prev: bool = glowkit_core::state(ui, id.with("hovered"), || false);
        let expanded = self.expand || hovered_prev;
        let front_h = entries.iter().find(|e| e.leaving.is_none()).map(|e| e.height).unwrap_or(56.0);

        // targets
        let mut offset = 0.0;
        let mut alive_idx = 0usize;
        let gap = 12.0;
        let mut targets = Vec::with_capacity(entries.len());
        for e in &entries {
            if e.leaving.is_some() {
                targets.push((offset, 1.0, 1.0));
                continue;
            }
            let i = alive_idx as f32;
            let (off, scale, vis) = if expanded {
                let o = offset;
                offset += e.height + gap;
                (o, 1.0, if alive_idx < self.visible + 3 { 1.0 } else { 0.0 })
            } else {
                (i * 14.0, 1.0 - i * 0.05, if alive_idx < self.visible { 1.0 } else { 0.0 })
            };
            targets.push((off, scale, vis));
            alive_idx += 1;
        }

        let mut any_anim = false;
        let mut bbox = Rect::NOTHING;
        for (e, &(off, scale, _)) in entries.iter_mut().zip(&targets) {
            e.y.target = off;
            e.y.stiffness = 300.0;
            e.y.damping = 30.0;
            e.y.step(dt);
            e.scale.target = scale;
            e.scale.stiffness = 300.0;
            e.scale.damping = 26.0;
            e.scale.step(dt);
            let born = *e.born.get_or_insert(now);
            e.appear = (((now - born) as f32) / 0.32).clamp(0.0, 1.0);
            if let Some(l) = &mut e.leaving {
                *l += dt * 4.0;
            }
            if !e.y.is_settled() || !e.scale.is_settled() || e.appear < 1.0 || e.leaving.is_some() {
                any_anim = true;
            }
            let h = if expanded { e.height } else { front_h };
            let y_top = if bottom { anchor_y - e.y.value - h } else { anchor_y + e.y.value };
            bbox = bbox.union(Rect::from_min_size(pos2(x0, y_top), vec2(w, h)));
        }

        let hover_rect = bbox.expand2(vec2(0.0, if expanded { 10.0 } else { 0.0 }));
        let pointer_in = ctx.pointer_hover_pos().is_some_and(|p| hover_rect.contains(p) && clip.contains(p));
        glowkit_core::set_state(ui, id.with("hovered"), pointer_in);

        let mut clicked = Vec::new();
        let mut to_dismiss = Vec::new();
        let layer_rect = bbox.expand(24.0);
        let close_button = self.close_button;
        let rich = self.rich_colors;
        kit::float_layer(&ctx, id, Order::Tooltip, layer_rect, clip, 1.0, layer_rect.center(), 1.0, |ui| {
            // paint back-to-front
            let n = entries.len();
            for k in (0..n).rev() {
                let e = &mut entries[k];
                let (_, _, vis) = targets[k];
                let h = if expanded || k == 0 || e.leaving.is_some() { e.height } else { front_h };
                let appear = glowkit_core::ease(glowkit_core::Easing::OutCubic, e.appear);
                let leave = e.leaving.map(|l| l.min(1.0)).unwrap_or(0.0);
                let mut alpha = appear * (1.0 - leave) * vis;
                let enter_shift = (1.0 - appear) * 24.0 * -dir;
                let leave_shift = leave * 16.0 * -dir;
                let y_top = if bottom { anchor_y - e.y.value - h } else { anchor_y + e.y.value };
                let rect = Rect::from_min_size(pos2(x0 + e.drag_x, y_top + enter_shift + leave_shift), vec2(w, h));
                let s = e.scale.value.clamp(0.5, 1.2);
                // scale around the edge nearest the anchor
                let pivot = if bottom { rect.center_bottom() } else { rect.center_top() };
                let rect = Rect::from_min_max(pivot + (rect.min - pivot) * s, pivot + (rect.max - pivot) * s);
                alpha *= (1.0 - e.drag_x.abs() / 220.0).clamp(0.0, 1.0);
                if alpha <= 0.01 {
                    continue;
                }
                let mut p = ui.painter().clone();
                p.multiply_opacity(alpha);
                let col = e.toast.kind.color(&t);
                let fill = if rich && e.toast.kind != ToastKind::Default && e.toast.kind != ToastKind::Loading {
                    mix(t.popover, col, if t.dark { 0.14 } else { 0.08 })
                } else {
                    t.popover
                };
                let border = if rich && e.toast.kind != ToastKind::Default && e.toast.kind != ToastKind::Loading {
                    mix(t.border, col, 0.35)
                } else {
                    t.border
                };
                paint::soft_shadow(
                    &p,
                    rect,
                    t.radius,
                    18.0,
                    vec2(0.0, 6.0),
                    with_alpha(Color32::BLACK, if t.dark { 0.35 } else { 0.08 }),
                );
                p.rect_filled(rect, t.radius, fill);
                p.rect_stroke(rect, t.radius, Stroke::new(1.0, border), StrokeKind::Inside);
                // content is only shown on the front toast while collapsed
                let content_alpha = if expanded || k == 0 { 1.0 } else { 0.0 };
                let content_alpha = glowkit_core::anim::follow(ui, id.with(("content", e.id.0)), content_alpha, 16.0);
                let resp = ui.interact(rect, id.with(("toast", e.id.0)), Sense::click_and_drag());
                if e.leaving.is_none() && (k == 0 || expanded) {
                    if resp.dragged() {
                        e.drag_x += resp.drag_delta().x;
                    } else if e.drag_x.abs() > 90.0 {
                        e.leaving = Some(0.0);
                    } else {
                        e.drag_x *= (-dt * 18.0).exp();
                        if e.drag_x.abs() < 0.3 {
                            e.drag_x = 0.0;
                        } else {
                            any_anim = true;
                        }
                    }
                }
                if content_alpha > 0.02 {
                    let mut cp = p.clone();
                    cp.multiply_opacity(content_alpha);
                    cp.set_clip_rect(rect.intersect(clip));
                    let mut x = rect.min.x + pad;
                    let top = rect.min.y + pad - 2.0;
                    if e.toast.kind == ToastKind::Loading {
                        kit::spinner(&cp, pos2(x + 8.0, top + 9.0), 7.5, t.muted_foreground, time);
                        any_anim = true;
                        x += 26.0;
                    } else if let Some(ic) = e.toast.kind.icon() {
                        ic.paint(&cp, Rect::from_center_size(pos2(x + 8.0, top + 9.0), vec2(17.0, 17.0)), col);
                        x += 26.0;
                    }
                    let act_w = e.toast.action.as_ref().map(|a| kit::button_width(ui, a, 12.5) - 8.0).unwrap_or(0.0);
                    let tw = rect.max.x - pad - x - if act_w > 0.0 { act_w + 10.0 } else { 0.0 };
                    let tg = kit::wrap(ui, &e.toast.title, 13.5, t.foreground, tw);
                    let mut ty = top;
                    if e.toast.description.is_none() {
                        ty = rect.center().y - tg.size().y / 2.0;
                    }
                    let th = tg.size().y;
                    cp.galley(pos2(x, ty), tg, t.foreground);
                    if let Some(d) = &e.toast.description {
                        let dg = kit::wrap(ui, d, 12.8, t.muted_foreground, tw);
                        cp.galley(pos2(x, ty + th + 3.0), dg, t.muted_foreground);
                    }
                    if let Some(a) = &e.toast.action {
                        let br = Rect::from_min_size(
                            pos2(rect.max.x - pad - act_w, rect.center().y - 13.0),
                            vec2(act_w, 26.0),
                        );
                        let bresp = ui
                            .interact(br, id.with(("action", e.id.0)), Sense::click())
                            .on_hover_cursor(CursorIcon::PointingHand);
                        let hv = kit::hover_anim(ui, id.with(("action", e.id.0)), bresp.hovered());
                        cp.rect_filled(
                            br,
                            6.0,
                            glowkit_core::color::shade(t.primary, if t.dark { -0.1 } else { 0.15 } * hv),
                        );
                        cp.text(
                            br.center(),
                            Align2::CENTER_CENTER,
                            a,
                            FontId::proportional(12.5),
                            t.primary_foreground,
                        );
                        if bresp.clicked() && e.leaving.is_none() {
                            clicked.push(e.id);
                            e.leaving = Some(0.0);
                        }
                    }
                    // progress line
                    if let Some(d) = e.toast.duration {
                        let frac = (1.0 - e.elapsed / d).clamp(0.0, 1.0);
                        let line = Rect::from_min_size(
                            pos2(rect.min.x + t.radius, rect.max.y - 2.5),
                            vec2((rect.width() - t.radius * 2.0) * frac, 1.5),
                        );
                        cp.rect_filled(
                            line,
                            1.0,
                            with_alpha(if e.toast.kind == ToastKind::Default { t.muted_foreground } else { col }, 0.35),
                        );
                    }
                    // close button
                    let show_close = close_button && (resp.hovered() || pointer_in && expanded);
                    let ca = kit::hover_anim(ui, id.with(("close_vis", e.id.0)), show_close);
                    if ca > 0.02 && e.leaving.is_none() {
                        let c = rect.left_top() + vec2(2.0, 2.0);
                        let cr = Rect::from_center_size(c, vec2(20.0, 20.0));
                        let mut xp = cp.clone();
                        xp.multiply_opacity(ca);
                        xp.set_clip_rect(clip);
                        let cresp = ui
                            .interact(cr, id.with(("close", e.id.0)), Sense::click())
                            .on_hover_cursor(CursorIcon::PointingHand);
                        xp.circle_filled(c, 10.0, if cresp.hovered() { t.muted } else { t.popover });
                        xp.circle_stroke(c, 10.0, Stroke::new(1.0, t.border));
                        LineIcon::X.paint_with(&xp, Rect::from_center_size(c, vec2(11.0, 11.0)), t.foreground, 1.4);
                        if cresp.clicked() {
                            to_dismiss.push(e.id);
                        }
                    }
                }
            }
        });

        // timers (paused while hovering the stack)
        for e in &mut entries {
            if !pointer_in && e.leaving.is_none() {
                e.elapsed += dt;
            }
            if let Some(d) = e.toast.duration {
                if e.elapsed >= d && e.leaving.is_none() {
                    e.leaving = Some(0.0);
                }
                any_anim = true;
            }
            if to_dismiss.contains(&e.id) {
                e.leaving.get_or_insert(0.0);
            }
        }
        // merge back (toasts may have been pushed during this frame)
        with_store(&ctx, |s| {
            for e in entries {
                if e.leaving.is_some_and(|l| l >= 1.0) {
                    s.entries.retain(|x| x.id != e.id);
                } else if let Some(x) = s.entries.iter_mut().find(|x| x.id == e.id) {
                    if x.rev == e.rev {
                        *x = e;
                    } else {
                        // updated mid-frame: keep the new content, take the animation state
                        *x = Entry { toast: x.toast.clone(), elapsed: 0.0, rev: x.rev, ..e };
                    }
                }
            }
        });
        if any_anim {
            ctx.request_repaint();
        }
        clicked
    }
}
