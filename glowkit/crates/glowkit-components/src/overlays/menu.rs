use super::kit;
use super::LineIcon;
use egui::{pos2, vec2, Align2, Context, CursorIcon, FontId, Id, Key, Order, Pos2, Rect, Response, Sense, Stroke};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// One row of a [`DropdownMenu`], [`ContextMenu`] or menubar menu.
///
/// ```no_run
/// use glowkit_components::{LineIcon, MenuEntry};
/// let entries = vec![
///     MenuEntry::label("My Account"),
///     MenuEntry::item("Profile").icon(LineIcon::User).shortcut("⇧⌘P"),
///     MenuEntry::checkbox("Show status bar", true),
///     MenuEntry::separator(),
///     MenuEntry::submenu("Invite users", vec![MenuEntry::item("Email"), MenuEntry::item("Message")]),
///     MenuEntry::item("Log out").icon(LineIcon::LogOut).destructive(),
/// ];
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum MenuEntry {
    /// A clickable item.
    Item {
        /// Text.
        label: String,
        /// Leading icon.
        icon: Option<LineIcon>,
        /// Right-aligned shortcut hint.
        shortcut: Option<String>,
        /// Red "danger" styling.
        destructive: bool,
        /// Greyed out and not clickable.
        disabled: bool,
    },
    /// A toggle with a check mark.
    Checkbox {
        /// Text.
        label: String,
        /// Current state (flip it yourself when the item's label is returned).
        checked: bool,
    },
    /// One option of a radio group (shows a dot when selected).
    Radio {
        /// Text.
        label: String,
        /// Whether this option is the selected one.
        selected: bool,
    },
    /// A small non-interactive heading.
    Label(String),
    /// A thin divider line.
    Separator,
    /// A nested menu that opens to the side.
    Submenu {
        /// Text.
        label: String,
        /// Leading icon.
        icon: Option<LineIcon>,
        /// Child entries.
        entries: Vec<MenuEntry>,
    },
}

impl MenuEntry {
    /// A plain clickable item.
    pub fn item(label: impl Into<String>) -> Self {
        MenuEntry::Item { label: label.into(), icon: None, shortcut: None, destructive: false, disabled: false }
    }
    /// A checkbox item.
    pub fn checkbox(label: impl Into<String>, checked: bool) -> Self {
        MenuEntry::Checkbox { label: label.into(), checked }
    }
    /// A radio item.
    pub fn radio(label: impl Into<String>, selected: bool) -> Self {
        MenuEntry::Radio { label: label.into(), selected }
    }
    /// A section heading.
    pub fn label(text: impl Into<String>) -> Self {
        MenuEntry::Label(text.into())
    }
    /// A divider.
    pub fn separator() -> Self {
        MenuEntry::Separator
    }
    /// A nested submenu.
    pub fn submenu(label: impl Into<String>, entries: Vec<MenuEntry>) -> Self {
        MenuEntry::Submenu { label: label.into(), icon: None, entries }
    }
    /// Set the leading icon (items and submenus).
    pub fn icon(mut self, i: LineIcon) -> Self {
        match &mut self {
            MenuEntry::Item { icon, .. } | MenuEntry::Submenu { icon, .. } => *icon = Some(i),
            _ => {}
        }
        self
    }
    /// Set the shortcut hint (items only).
    pub fn shortcut(mut self, s: impl Into<String>) -> Self {
        if let MenuEntry::Item { shortcut, .. } = &mut self {
            *shortcut = Some(s.into());
        }
        self
    }
    /// Style as destructive (items only).
    pub fn destructive(mut self) -> Self {
        if let MenuEntry::Item { destructive, .. } = &mut self {
            *destructive = true;
        }
        self
    }
    /// Disable (items only).
    pub fn disabled(mut self, yes: bool) -> Self {
        if let MenuEntry::Item { disabled, .. } = &mut self {
            *disabled = yes;
        }
        self
    }

    fn selectable(&self) -> bool {
        match self {
            MenuEntry::Item { disabled, .. } => !disabled,
            MenuEntry::Checkbox { .. } | MenuEntry::Radio { .. } | MenuEntry::Submenu { .. } => true,
            _ => false,
        }
    }
    fn text(&self) -> &str {
        match self {
            MenuEntry::Item { label, .. }
            | MenuEntry::Checkbox { label, .. }
            | MenuEntry::Radio { label, .. }
            | MenuEntry::Submenu { label, .. } => label,
            MenuEntry::Label(l) => l,
            MenuEntry::Separator => "",
        }
    }
    fn height(&self) -> f32 {
        match self {
            MenuEntry::Separator => 9.0,
            MenuEntry::Label(_) => 30.0,
            _ => 32.0,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct MenuState {
    hover: Option<usize>,
    sub_open: Option<usize>,
    sub_focus: bool,
}

/// What happened to a menu panel this frame.
#[derive(Default)]
pub(crate) struct MenuOutcome {
    pub activated: Option<String>,
    /// The whole menu should close (item chosen or Escape at the root).
    pub close: bool,
    /// Rects of this panel and any open submenus (for outside-click detection).
    pub rects: Vec<Rect>,
    /// Keyboard Left at root level (used by menubars to move between menus).
    pub key_left: bool,
    /// Keyboard Right on a non-submenu item at root level.
    pub key_right: bool,
}

fn has_icons(entries: &[MenuEntry]) -> bool {
    entries.iter().any(|e| {
        matches!(
            e,
            MenuEntry::Item { icon: Some(_), .. }
                | MenuEntry::Submenu { icon: Some(_), .. }
                | MenuEntry::Checkbox { .. }
                | MenuEntry::Radio { .. }
        )
    })
}

fn measure(ctx: &Context, entries: &[MenuEntry]) -> Vec2Size {
    let lead = if has_icons(entries) { 32.0 } else { 10.0 };
    let mut w: f32 = 0.0;
    let mut h = 8.0;
    for e in entries {
        let tw = ctx.fonts_mut(|f| {
            f.layout_no_wrap(e.text().to_owned(), FontId::proportional(13.5), egui::Color32::WHITE).size().x
        });
        let extra = match e {
            MenuEntry::Item { shortcut: Some(s), .. } => {
                ctx.fonts_mut(|f| {
                    f.layout_no_wrap(s.clone(), FontId::proportional(11.5), egui::Color32::WHITE).size().x
                }) + 28.0
            }
            MenuEntry::Submenu { .. } => 30.0,
            _ => 12.0,
        };
        w = w.max(lead + tw + extra + 12.0);
        h += e.height();
    }
    Vec2Size { w: w.max(196.0), h }
}

struct Vec2Size {
    w: f32,
    h: f32,
}

/// Render a menu panel with its top-left near `anchor` (dropdown placement) and handle input.
#[allow(clippy::too_many_arguments)]
pub(crate) fn menu_panel(
    ctx: &Context,
    id: Id,
    anchor: Rect,
    side: bool,
    entries: &[MenuEntry],
    p: f32,
    pop: f32,
    keyboard: bool,
    bounds: Rect,
    clip: Rect,
    min_width: f32,
) -> MenuOutcome {
    let t = Theme::current(ctx);
    let size = measure(ctx, entries);
    let w = size.w.max(min_width);
    let h = size.h;
    let mut out = MenuOutcome::default();
    let mut st: MenuState = ctx.data_mut(|d| d.get_temp(id.with("menu_state")).unwrap_or_default());

    // position
    let mut rect = if side {
        Rect::from_min_size(pos2(anchor.max.x + 4.0, anchor.min.y - 4.0), vec2(w, h))
    } else {
        Rect::from_min_size(pos2(anchor.min.x, anchor.max.y + 6.0), vec2(w, h))
    };
    let mut origin = rect.left_top();
    if side && rect.max.x > bounds.max.x - 4.0 {
        rect = rect.translate(vec2(-(w + anchor.width() + 8.0), 0.0));
        origin = rect.right_top();
    }
    if !side && rect.max.y > bounds.max.y - 4.0 && anchor.min.y - 6.0 - h > bounds.min.y {
        rect = Rect::from_min_size(pos2(rect.min.x, anchor.min.y - 6.0 - h), rect.size());
        origin = rect.left_bottom();
    }
    rect = kit::clamp_into(rect, bounds, 4.0);
    let slide = if side { 0.0 } else { (1.0 - p) * -4.0 };
    let rect = rect.translate(vec2(0.0, slide));
    out.rects.push(rect);

    // keyboard
    let selectable: Vec<usize> = entries.iter().enumerate().filter(|(_, e)| e.selectable()).map(|(i, _)| i).collect();
    let mut activate_kb = None;
    if keyboard && !st.sub_focus && p > 0.5 {
        ctx.input_mut(|i| {
            let step = |st: &mut MenuState, d: i32| {
                if selectable.is_empty() {
                    return;
                }
                let cur = st.hover.and_then(|h| selectable.iter().position(|&x| x == h));
                let n = selectable.len() as i32;
                let next = match cur {
                    Some(c) => (c as i32 + d).rem_euclid(n),
                    None => {
                        if d > 0 {
                            0
                        } else {
                            n - 1
                        }
                    }
                };
                st.hover = Some(selectable[next as usize]);
                st.sub_open = None;
            };
            if i.consume_key(egui::Modifiers::NONE, Key::ArrowDown) {
                step(&mut st, 1);
            }
            if i.consume_key(egui::Modifiers::NONE, Key::ArrowUp) {
                step(&mut st, -1);
            }
            if i.consume_key(egui::Modifiers::NONE, Key::Enter) || i.consume_key(egui::Modifiers::NONE, Key::Space) {
                activate_kb = st.hover;
            }
            if i.consume_key(egui::Modifiers::NONE, Key::ArrowRight) {
                if let Some(h) = st.hover {
                    if matches!(entries[h], MenuEntry::Submenu { .. }) {
                        st.sub_open = Some(h);
                        st.sub_focus = true;
                    } else {
                        out.key_right = true;
                    }
                } else {
                    out.key_right = true;
                }
            }
            if i.consume_key(egui::Modifiers::NONE, Key::ArrowLeft) {
                out.key_left = true;
            }
            if i.consume_key(egui::Modifiers::NONE, Key::Escape) {
                out.close = true;
            }
        });
    }

    let lead = if has_icons(entries) { 32.0 } else { 10.0 };
    let mut rows: Vec<(usize, Rect)> = Vec::new();
    let mut y = rect.min.y + 4.0;
    for (i, e) in entries.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.min.x + 4.0, y), vec2(rect.width() - 8.0, e.height()));
        rows.push((i, r));
        y += e.height();
    }

    let hl_target = st.hover.and_then(|h| rows.get(h).map(|(_, r)| r.min.y)).unwrap_or(f32::NAN);
    let hl_y = if hl_target.is_nan() { f32::NAN } else { kit::follow_ctx(ctx, id.with("hl_y"), hl_target, 30.0) };
    if hl_target.is_nan() {
        ctx.data_mut(|d| d.remove::<f32>(id.with("hl_y").with("kit_follow")));
    }
    let hl_a = kit::follow_ctx(ctx, id.with("hl_a"), if hl_target.is_nan() { 0.0 } else { 1.0 }, 22.0);

    let mut clicked: Option<usize> = activate_kb;
    let mut hovered_now: Option<usize> = None;
    let prev_hover = st.hover;
    let layer =
        kit::float_layer(ctx, id, Order::Foreground, rect.expand(16.0), clip, 0.96 + 0.04 * pop, origin, p, |ui| {
            ui.interact(rect, id.with("block"), Sense::click());
            let pnt = ui.painter().clone();
            kit::panel(&pnt, rect, &t, t.radius * 0.8);
            if !hl_y.is_nan() && hl_a > 0.01 {
                if let Some(h) = prev_hover {
                    let hr =
                        Rect::from_min_size(pos2(rect.min.x + 4.0, hl_y), vec2(rect.width() - 8.0, rows[h].1.height()));
                    let destructive = matches!(entries[h], MenuEntry::Item { destructive: true, .. });
                    let fill = if destructive { with_alpha(t.destructive, 0.12) } else { kit::row_highlight(&t) };
                    pnt.rect_filled(hr, t.radius * 0.5, fill.gamma_multiply(hl_a));
                }
            }
            for (i, r) in &rows {
                let e = &entries[*i];
                let resp = if e.selectable() {
                    Some(
                        ui.interact(*r, id.with(("row", *i)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand),
                    )
                } else {
                    None
                };
                if let Some(resp) = &resp {
                    if resp.hovered()
                        && ui.input(|inp| inp.pointer.delta() != egui::Vec2::ZERO || inp.pointer.any_pressed())
                    {
                        hovered_now = Some(*i);
                    }
                    if resp.clicked() {
                        clicked = Some(*i);
                    }
                }
                let fg = t.foreground;
                let text_pos = pos2(r.min.x + lead, r.center().y);
                match e {
                    MenuEntry::Separator => {
                        let yy = r.center().y;
                        pnt.line_segment([pos2(rect.min.x, yy), pos2(rect.max.x, yy)], Stroke::new(1.0, t.border));
                    }
                    MenuEntry::Label(l) => {
                        pnt.text(
                            pos2(r.min.x + 10.0, r.center().y),
                            Align2::LEFT_CENTER,
                            l,
                            FontId::proportional(12.5),
                            t.muted_foreground,
                        );
                    }
                    MenuEntry::Item { label, icon, shortcut, destructive, disabled } => {
                        let col = if *disabled {
                            with_alpha(t.muted_foreground, 0.5)
                        } else if *destructive {
                            t.destructive
                        } else {
                            fg
                        };
                        if let Some(ic) = icon {
                            let icol = if *destructive || *disabled {
                                col
                            } else {
                                t.muted_foreground.lerp_to_gamma(fg, if prev_hover == Some(*i) { 1.0 } else { 0.0 })
                            };
                            ic.paint(
                                &pnt,
                                Rect::from_center_size(pos2(r.min.x + 17.0, r.center().y), vec2(15.0, 15.0)),
                                icol,
                            );
                        }
                        pnt.text(text_pos, Align2::LEFT_CENTER, label, FontId::proportional(13.5), col);
                        if let Some(s) = shortcut {
                            kit::shortcut_text(&pnt, pos2(r.max.x - 10.0, r.center().y), s, &t);
                        }
                    }
                    MenuEntry::Checkbox { label, checked } => {
                        if *checked {
                            LineIcon::Check.paint_with(
                                &pnt,
                                Rect::from_center_size(pos2(r.min.x + 17.0, r.center().y), vec2(14.0, 14.0)),
                                fg,
                                1.7,
                            );
                        }
                        pnt.text(text_pos, Align2::LEFT_CENTER, label, FontId::proportional(13.5), fg);
                    }
                    MenuEntry::Radio { label, selected } => {
                        if *selected {
                            pnt.circle_filled(pos2(r.min.x + 17.0, r.center().y), 3.5, fg);
                        }
                        pnt.text(text_pos, Align2::LEFT_CENTER, label, FontId::proportional(13.5), fg);
                    }
                    MenuEntry::Submenu { label, icon, .. } => {
                        if let Some(ic) = icon {
                            ic.paint(
                                &pnt,
                                Rect::from_center_size(pos2(r.min.x + 17.0, r.center().y), vec2(15.0, 15.0)),
                                t.muted_foreground,
                            );
                        }
                        pnt.text(text_pos, Align2::LEFT_CENTER, label, FontId::proportional(13.5), fg);
                        LineIcon::ChevronRight.paint(
                            &pnt,
                            Rect::from_center_size(pos2(r.max.x - 14.0, r.center().y), vec2(14.0, 14.0)),
                            t.muted_foreground,
                        );
                    }
                }
            }
        });
    let _ = layer;
    ctx.move_to_top(egui::LayerId::new(Order::Foreground, id));

    if let Some(h) = hovered_now {
        st.hover = Some(h);
        if matches!(entries[h], MenuEntry::Submenu { .. }) {
            st.sub_open = Some(h);
        } else if st.sub_open.is_some() {
            st.sub_open = None;
        }
        st.sub_focus = false;
    }
    // pointer left the panel (and no submenu open): clear hover so the highlight fades out
    let pointer = ctx.pointer_hover_pos();
    if st.sub_open.is_none()
        && pointer.is_some_and(|pp| !rect.contains(pp))
        && ctx.input(|i| i.pointer.delta() != egui::Vec2::ZERO)
    {
        st.hover = None;
    }
    if let Some(c) = clicked {
        match &entries[c] {
            MenuEntry::Item { label, .. } => {
                out.activated = Some(label.clone());
                out.close = true;
            }
            MenuEntry::Checkbox { label, .. } | MenuEntry::Radio { label, .. } => out.activated = Some(label.clone()),
            MenuEntry::Submenu { .. } => {
                st.sub_open = Some(c);
                st.sub_focus = activate_kb.is_some();
            }
            _ => {}
        }
    }

    // submenu
    let sub_id = id.with("sub");
    let sub_target = st.sub_open;
    let shown_sub: Option<usize> = ctx.data_mut(|d| d.get_temp(id.with("shown_sub")).unwrap_or(None));
    let sub_idx = sub_target.or(shown_sub);
    let sp = kit::follow_ctx(ctx, sub_id.with("p"), if sub_target.is_some() { 1.0 } else { 0.0 }, 20.0);
    if sub_target.is_some() {
        ctx.data_mut(|d| d.insert_temp(id.with("shown_sub"), sub_target));
    }
    if let Some(si) = sub_idx {
        if sp > 0.01 {
            if let MenuEntry::Submenu { entries: children, .. } = &entries[si] {
                let row = rows[si].1;
                let sub_out = menu_panel(
                    ctx,
                    sub_id.with(si),
                    row,
                    true,
                    children,
                    sp * p,
                    sp,
                    keyboard && st.sub_focus,
                    bounds,
                    clip,
                    0.0,
                );
                if sub_out.key_left || (sub_out.close && sub_out.activated.is_none()) {
                    st.sub_open = None;
                    st.sub_focus = false;
                }
                if sub_out.activated.is_some() {
                    out.activated = sub_out.activated;
                    out.close = sub_out.close;
                }
                out.rects.extend(sub_out.rects);
            }
        } else if sub_target.is_none() {
            ctx.data_mut(|d| d.insert_temp(id.with("shown_sub"), None::<usize>));
        }
    }
    if out.close {
        st = MenuState::default();
    }
    ctx.data_mut(|d| d.insert_temp(id.with("menu_state"), st));
    out
}

/// Reset internal menu state (hover, open submenus).
pub(crate) fn reset_menu(ctx: &Context, id: Id) {
    ctx.data_mut(|d| d.insert_temp(id.with("menu_state"), MenuState::default()));
}

fn outside_click(ctx: &Context, rects: &[Rect], trigger: Option<Rect>) -> bool {
    let (pressed, pos) = ctx.input(|i| (i.pointer.any_pressed(), i.pointer.interact_pos()));
    pressed && pos.is_some_and(|p| !rects.iter().any(|r| r.contains(p)) && !trigger.is_some_and(|t| t.contains(p)))
}

/// A menu that drops down from a trigger widget: icons, shortcuts, labels, separators,
/// checkbox / radio items and nested submenus. Animated highlight follows the pointer;
/// full keyboard navigation (↑ ↓ Enter → ← Esc).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::{DropdownMenu, MenuEntry, LineIcon};
/// let trigger = ui.button("Open");
/// if let Some(item) = DropdownMenu::new(vec![
///     MenuEntry::item("Profile").icon(LineIcon::User).shortcut("⇧⌘P"),
///     MenuEntry::separator(),
///     MenuEntry::item("Log out").icon(LineIcon::LogOut),
/// ]).show(&trigger) {
///     println!("clicked {item}");
/// }
/// ```
#[must_use = "call `.show(&trigger)`"]
pub struct DropdownMenu {
    entries: Vec<MenuEntry>,
    width: f32,
    within: Option<Rect>,
}

impl DropdownMenu {
    /// A menu with the given entries.
    pub fn new(entries: Vec<MenuEntry>) -> Self {
        Self { entries, width: 0.0, within: None }
    }
    /// Minimum width (defaults to fit content, at least 196).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
    /// Keep the menu inside `rect`.
    pub fn within(mut self, rect: Rect) -> Self {
        self.within = Some(rect);
        self
    }
    /// Is the menu attached to `trigger_id` open?
    pub fn is_open(ctx: &Context, trigger_id: Id) -> bool {
        ctx.data_mut(|d| d.get_temp::<bool>(trigger_id.with("glowkit_dropdown_open")).unwrap_or(false))
    }
    /// Open or close the menu attached to `trigger_id`.
    pub fn set_open(ctx: &Context, trigger_id: Id, open: bool) {
        ctx.data_mut(|d| d.insert_temp(trigger_id.with("glowkit_dropdown_open"), open));
    }
    /// Show the menu for `trigger` (toggles on click). Returns the label of an activated item.
    pub fn show(self, trigger: &Response) -> Option<String> {
        let ctx = trigger.ctx.clone();
        let id = trigger.id.with("glowkit_dropdown");
        let mut open = Self::is_open(&ctx, trigger.id);
        if trigger.clicked() {
            open = !open;
            reset_menu(&ctx, id);
        }
        let rects: Vec<Rect> = ctx.data_mut(|d| d.get_temp(id.with("rects")).unwrap_or_default());
        if open && !trigger.clicked() && outside_click(&ctx, &rects, Some(trigger.rect)) {
            open = false;
        }
        let p = kit::follow_ctx(&ctx, id.with("p"), if open { 1.0 } else { 0.0 }, 18.0);
        let pop = kit::spring_ctx(&ctx, id.with("pop"), if open { 1.0 } else { 0.0 }, 420.0, 28.0);
        let mut result = None;
        if p > 0.01 || open {
            let bounds = self.within.unwrap_or_else(|| ctx.content_rect());
            let clip = self.within.map(|w| w.expand(40.0)).unwrap_or(bounds);
            let out = menu_panel(&ctx, id, trigger.rect, false, &self.entries, p, pop, open, bounds, clip, self.width);
            if open {
                ctx.data_mut(|d| d.insert_temp(id.with("rects"), out.rects.clone()));
            }
            if out.close {
                open = false;
            }
            result = out.activated;
        }
        Self::set_open(&ctx, trigger.id, open);
        result
    }
}

/// A menu that opens at the pointer on right-click (or long-press) over any widget.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::{ContextMenu, MenuEntry};
/// let area = ui.label("Right click here");
/// ContextMenu::new(vec![MenuEntry::item("Back"), MenuEntry::item("Reload")]).show(&area);
/// ```
#[must_use = "call `.show(&response)`"]
pub struct ContextMenu {
    entries: Vec<MenuEntry>,
    within: Option<Rect>,
}

impl ContextMenu {
    /// A context menu with the given entries.
    pub fn new(entries: Vec<MenuEntry>) -> Self {
        Self { entries, within: None }
    }
    /// Keep the menu inside `rect`.
    pub fn within(mut self, rect: Rect) -> Self {
        self.within = Some(rect);
        self
    }
    /// Open the menu programmatically at `pos` for the area with id `area_id`.
    pub fn open_at(ctx: &Context, area_id: Id, pos: Pos2) {
        ctx.data_mut(|d| {
            d.insert_temp(area_id.with("glowkit_ctx_open"), true);
            d.insert_temp(area_id.with("glowkit_ctx_pos"), pos);
        });
    }
    /// Attach to `response` (it should sense clicks). Returns the label of an activated item.
    pub fn show(self, response: &Response) -> Option<String> {
        let ctx = response.ctx.clone();
        let id = response.id.with("glowkit_ctxmenu");
        let (mut open, mut pos) = ctx.data_mut(|d| {
            (
                d.get_temp::<bool>(response.id.with("glowkit_ctx_open")).unwrap_or(false),
                d.get_temp::<Pos2>(response.id.with("glowkit_ctx_pos")),
            )
        });
        let mut just_opened = false;
        if response.secondary_clicked() || response.long_touched() {
            open = true;
            just_opened = true;
            pos = ctx.input(|i| i.pointer.interact_pos()).or(pos);
            reset_menu(&ctx, id);
            // restart the pop animation from the new position
            ctx.data_mut(|d| {
                d.insert_temp(id.with("p").with("kit_follow"), 0.0f32);
            });
        }
        let rects: Vec<Rect> = ctx.data_mut(|d| d.get_temp(id.with("rects")).unwrap_or_default());
        if open && !just_opened && outside_click(&ctx, &rects, None) {
            open = false;
        }
        let p = kit::follow_ctx(&ctx, id.with("p"), if open { 1.0 } else { 0.0 }, 20.0);
        let pop = kit::spring_ctx(&ctx, id.with("pop"), if open { 1.0 } else { 0.0 }, 420.0, 28.0);
        let mut result = None;
        if let Some(pos) = pos {
            if p > 0.01 || open {
                let bounds = self.within.unwrap_or_else(|| ctx.content_rect());
                let clip = self.within.map(|w| w.expand(40.0)).unwrap_or(bounds);
                let anchor = Rect::from_min_size(pos - vec2(0.0, 6.0), vec2(0.0, 0.0));
                let out = menu_panel(&ctx, id, anchor, false, &self.entries, p, pop, open, bounds, clip, 0.0);
                if open {
                    ctx.data_mut(|d| d.insert_temp(id.with("rects"), out.rects.clone()));
                }
                if out.close {
                    open = false;
                }
                result = out.activated;
            }
        }
        ctx.data_mut(|d| {
            d.insert_temp(response.id.with("glowkit_ctx_open"), open);
            if let Some(p) = pos {
                d.insert_temp(response.id.with("glowkit_ctx_pos"), p);
            }
        });
        result
    }
}
