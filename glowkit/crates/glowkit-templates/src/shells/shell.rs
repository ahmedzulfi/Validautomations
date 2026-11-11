//! [`AppShell`]: the dashboard frame (sidebar + topbar + content) in seven layouts.

use egui::{pos2, vec2, Align2, Color32, CursorIcon, Id, Key, Modifiers, Order, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_components::{
    toast, Breadcrumb, CommandItem, CommandPalette, DropdownMenu, LineIcon, MenuEntry, Placement, Popover, Toast, ToastPosition,
    Toaster, Tooltip,
};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

use super::icon::AppIcon;
use super::kit;
use super::nav::{nav_find, nav_leaves, nav_path, NavSection, ShellNotification, ShellUser, Workspace};
use super::sidebar::{chevron, sidebar, user_menu, SideCfg, RAIL_W};

/// How an [`AppShell`] arranges its chrome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ShellLayout {
    /// Full-height sidebar with a border, topbar above the content. Collapses to an icon rail.
    #[default]
    Sidebar,
    /// The content floats as a rounded card inset from the sidebar-colored frame.
    Inset,
    /// The sidebar itself floats as a rounded card with a shadow.
    Floating,
    /// An icon rail of sections plus a secondary panel listing the active section's items.
    Dual,
    /// No sidebar: brand, top-level navigation and actions in a single top bar.
    TopNav,
    /// Sidebar + content + a collapsible inspector panel on the right.
    Inspector,
    /// Mobile style: app bar on top, tab bar with icons at the bottom.
    BottomNav,
}

#[derive(Clone)]
struct ShellState {
    collapsed: bool,
    narrow_expanded: bool,
    palette: bool,
    theme: Option<Theme>,
    inspector: bool,
    dual_section: Option<usize>,
    upgrade_dismissed: bool,
    drawer: bool,
    last_route: String,
    route_t: f64,
}

impl Default for ShellState {
    fn default() -> Self {
        Self {
            collapsed: false,
            narrow_expanded: false,
            palette: false,
            theme: None,
            inspector: true,
            dual_section: None,
            upgrade_dismissed: false,
            drawer: false,
            last_route: String::new(),
            route_t: -10.0,
        }
    }
}

/// What the content closure of an [`AppShell`] gets besides its `Ui`.
pub struct ShellContext<'a> {
    /// The active route; assign to navigate.
    pub route: &'a mut String,
    /// The content area (after padding).
    pub rect: Rect,
    /// The whole shell; pass it to overlays' `within` so dialogs and sheets stay inside the app.
    pub bounds: Rect,
    /// True when the shell is narrow (phone-sized content).
    pub compact: bool,
    theme: &'a mut Option<Theme>,
}

impl ShellContext<'_> {
    /// Switch the app theme (installed globally, or scoped to the shell with
    /// [`AppShell::scoped_theme`]).
    pub fn set_theme(&mut self, theme: Theme) {
        *self.theme = Some(theme);
    }
    /// Navigate to `route`.
    pub fn navigate(&mut self, route: impl Into<String>) {
        *self.route = route.into();
    }
}

/// What happened in an [`AppShell`] this frame.
#[derive(Debug)]
pub struct ShellResponse<R> {
    /// What the content closure returned.
    pub inner: R,
    /// A command chosen in the palette that the shell did not handle itself (your `.command(..)`s).
    pub command: Option<String>,
    /// An item picked in the user / workspace menus ("Account", "Billing", "Log out", ...).
    pub menu: Option<String>,
    /// Index of a notification that was clicked (it is marked read).
    pub notification: Option<usize>,
    /// The route changed this frame (from the sidebar, palette, breadcrumb or bottom nav).
    pub navigated: bool,
    /// The whole shell rect.
    pub rect: Rect,
}

type Panel<'a> = Box<dyn FnOnce(&mut Ui, &mut String) + 'a>;
type Actions<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// A complete application frame: sidebar (workspace switcher, grouped nested navigation, upgrade
/// card, user card), topbar (sidebar toggle, breadcrumb, Ctrl+K search that opens a command
/// palette, notifications bell with a dropdown, theme toggle, user menu) and your content.
///
/// The navigation tree ([`NavSection`] / [`NavItem`](super::NavItem)) and the active route are
/// owned by you. The sidebar collapses to an icon rail on a spring (Ctrl+B or the toggle), auto
/// collapses when the window gets narrow, and turns into a drawer on phone widths. Toasts,
/// dialogs and the palette are kept inside the shell.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{AppShell, NavItem, NavSection, ShellLayout, ShellUser, Workspace};
/// use glowkit_components::LineIcon;
/// let mut route = String::from("dashboard");
/// let mut nav = vec![NavSection::new("Platform")
///     .item(NavItem::new("dashboard", "Dashboard").icon(LineIcon::Home))
///     .item(NavItem::new("inbox", "Inbox").icon(LineIcon::Inbox).badge("12"))];
/// AppShell::new(&mut route, &mut nav)
///     .layout(ShellLayout::Inset)
///     .workspaces(vec![Workspace::new("Acme Inc", "Enterprise")])
///     .user(ShellUser::new("Ada Lovelace", "ada@acme.dev"))
///     .show(ui, |ui, shell| {
///         ui.heading(format!("Route: {}", shell.route));
///     });
/// ```
#[must_use = "call `.show(ui, ..)`"]
pub struct AppShell<'a> {
    route: &'a mut String,
    nav: &'a mut Vec<NavSection>,
    layout: ShellLayout,
    workspaces: Vec<Workspace>,
    user: Option<ShellUser>,
    notifications: Option<&'a mut Vec<ShellNotification>>,
    commands: Vec<CommandItem>,
    breadcrumb: Option<Vec<String>>,
    upgrade: bool,
    inspector: Option<(String, Panel<'a>)>,
    actions: Option<Actions<'a>>,
    size: Option<egui::Vec2>,
    padding: f32,
    scoped_theme: bool,
    search: bool,
    theme_toggle: bool,
    title: Option<String>,
    id_salt: egui::IdSalt,
    rounded: bool,
    start_collapsed: bool,
}

impl<'a> AppShell<'a> {
    /// A shell over `nav` with `route` as the active route.
    pub fn new(route: &'a mut String, nav: &'a mut Vec<NavSection>) -> Self {
        Self {
            route,
            nav,
            layout: ShellLayout::Sidebar,
            workspaces: vec![Workspace::new("Acme Inc", "Enterprise")],
            user: None,
            notifications: None,
            commands: Vec::new(),
            breadcrumb: None,
            upgrade: false,
            inspector: None,
            actions: None,
            size: None,
            padding: 24.0,
            scoped_theme: false,
            search: true,
            theme_toggle: true,
            title: None,
            id_salt: egui::IdSalt::new("glowkit_app_shell"),
            rounded: true,
            start_collapsed: false,
        }
    }
    /// Layout variant (default [`ShellLayout::Sidebar`]).
    pub fn layout(mut self, layout: ShellLayout) -> Self {
        self.layout = layout;
        self
    }
    /// Workspaces for the switcher (the first is active until the user switches).
    pub fn workspaces(mut self, w: Vec<Workspace>) -> Self {
        self.workspaces = w;
        self
    }
    /// The signed-in user (sidebar footer card / topbar avatar menu).
    pub fn user(mut self, user: ShellUser) -> Self {
        self.user = Some(user);
        self
    }
    /// Notifications for the bell dropdown (caller owned; read state is written back).
    pub fn notifications(mut self, n: &'a mut Vec<ShellNotification>) -> Self {
        self.notifications = Some(n);
        self
    }
    /// Extra command palette entries (returned in [`ShellResponse::command`] when chosen).
    pub fn command(mut self, item: CommandItem) -> Self {
        self.commands.push(item);
        self
    }
    /// Override the breadcrumb (defaults to the nav path of the active route).
    pub fn breadcrumb(mut self, items: Vec<String>) -> Self {
        self.breadcrumb = Some(items);
        self
    }
    /// Show the "Upgrade to Pro" card above the user card.
    pub fn upgrade_card(mut self, yes: bool) -> Self {
        self.upgrade = yes;
        self
    }
    /// Right inspector panel (used by [`ShellLayout::Inspector`]); toggles with Ctrl+I.
    pub fn inspector(mut self, title: impl Into<String>, add: impl FnOnce(&mut Ui, &mut String) + 'a) -> Self {
        self.inspector = Some((title.into(), Box::new(add)));
        self
    }
    /// Extra widgets in the topbar, left of the search box (e.g. a "New" button).
    pub fn actions(mut self, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.actions = Some(Box::new(add));
        self
    }
    /// Fixed size (defaults to all available space).
    pub fn size(mut self, size: egui::Vec2) -> Self {
        self.size = Some(size);
        self
    }
    /// Padding around the content (default 24).
    pub fn padding(mut self, p: f32) -> Self {
        self.padding = p;
        self
    }
    /// Apply theme changes only while the shell draws (for embedding it inside another app,
    /// like the gallery). Default false: theme changes are installed globally.
    pub fn scoped_theme(mut self, yes: bool) -> Self {
        self.scoped_theme = yes;
        self
    }
    /// Show the topbar search box / Ctrl+K palette (default true).
    pub fn search(mut self, yes: bool) -> Self {
        self.search = yes;
        self
    }
    /// Show the light / dark toggle (default true).
    pub fn theme_toggle(mut self, yes: bool) -> Self {
        self.theme_toggle = yes;
        self
    }
    /// App title (used by the bottom-nav app bar; defaults to the active item's label).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
    /// Start with the sidebar collapsed to the icon rail.
    pub fn start_collapsed(mut self, yes: bool) -> Self {
        self.start_collapsed = yes;
        self
    }
    /// Round the outer corners of the shell (default true; turn off when it fills a window).
    pub fn rounded(mut self, yes: bool) -> Self {
        self.rounded = yes;
        self
    }
    /// Distinguish several shells in one ui.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = egui::IdSalt::new(salt);
        self
    }

    /// Draw the shell and its content. `content` gets a `Ui` confined to the content area and a
    /// [`ShellContext`] (route, rects, theme switching).
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui, &mut ShellContext<'_>) -> R) -> ShellResponse<R> {
        let ctx = ui.ctx().clone();
        let id = ui.scope_id().with_salt(self.id_salt);
        let start_collapsed = self.start_collapsed;
        let mut st: ShellState = glowkit_core::state(ui, id, || ShellState { collapsed: start_collapsed, ..Default::default() });
        // ---- theme ------------------------------------------------------------------------
        let outer_theme = Theme::current(&ctx);
        let mut restore = false;
        if let Some(th) = st.theme.clone() {
            if self.scoped_theme {
                if th != outer_theme {
                    th.install(&ctx);
                    restore = true;
                }
            } else {
                if th != outer_theme {
                    th.install(&ctx);
                }
                st.theme = None;
            }
        }
        let size = self.size.unwrap_or_else(|| ui.available_size());
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let mut sub = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        sub.set_clip_rect(rect.intersect(ui.clip_rect()));
        if restore {
            sub.set_style(ctx.global_style());
        }
        let resp = self.draw(&mut sub, rect, id, &mut st, content);
        glowkit_core::set_state(ui, id, st);
        if restore {
            outer_theme.install(&ctx);
        }
        resp
    }

    fn draw<R>(
        mut self,
        ui: &mut Ui,
        rect: Rect,
        id: Id,
        st: &mut ShellState,
        content: impl FnOnce(&mut Ui, &mut ShellContext<'_>) -> R,
    ) -> ShellResponse<R> {
        let ctx = ui.ctx().clone();
        let t = Theme::of(ui);
        let now = ui.input(|i| i.time);
        let p = ui.painter().clone();
        let outer_r = if self.rounded { t.radius } else { 0.0 };
        let chrome = kit::chrome_bg(&t);
        let mut command = None;
        let mut menu: Option<String> = None;
        let mut notification = None;
        let start_route = self.route.clone();
        let layout = self.layout;
        let has_sidebar = matches!(layout, ShellLayout::Sidebar | ShellLayout::Inset | ShellLayout::Floating | ShellLayout::Dual | ShellLayout::Inspector);
        let phone = has_sidebar && rect.width() < 720.0;
        let auto_rail = has_sidebar && rect.width() < 1040.0;
        // ---- shortcuts --------------------------------------------------------------------
        let (ctrl_b, ctrl_i) = ctx.input_mut(|i| (i.consume_key(Modifiers::COMMAND, Key::B), i.consume_key(Modifiers::COMMAND, Key::I)));
        if ctrl_b && has_sidebar {
            if phone {
                st.drawer = !st.drawer;
            } else if auto_rail {
                st.narrow_expanded = !st.narrow_expanded;
            } else {
                st.collapsed = !st.collapsed;
            }
        }
        if ctrl_i && self.inspector.is_some() {
            st.inspector = !st.inspector;
        }
        if self.search {
            CommandPalette::toggle_on_shortcut(&ctx, &mut st.palette);
        }
        let collapsed = if auto_rail { !st.narrow_expanded } else { st.collapsed };
        let c = kit::spring(&ctx, id.with("collapse"), if collapsed { 1.0 } else { 0.0 }, 260.0, 28.0).clamp(0.0, 1.0);

        // ---- background -------------------------------------------------------------------
        let frame_bg = match layout {
            ShellLayout::Inset | ShellLayout::Floating => chrome,
            _ => t.background,
        };
        p.rect_filled(rect, outer_r, frame_bg);

        // ---- regions ----------------------------------------------------------------------
        let side_full = 256.0;
        let side_w = if phone { 0.0 } else { side_full + (RAIL_W - side_full) * c };
        let mut side_rect = None;
        let mut rail_rect = None;
        let main_rect;
        match layout {
            ShellLayout::Sidebar | ShellLayout::Inspector => {
                let s = Rect::from_min_size(rect.min, vec2(side_w, rect.height()));
                if !phone {
                    p.rect_filled(s, egui::CornerRadius { nw: outer_r as u8, sw: outer_r as u8, ne: 0, se: 0 }, chrome);
                    p.line_segment([s.right_top(), s.right_bottom()], Stroke::new(1.0, t.border));
                    side_rect = Some(s);
                }
                main_rect = Rect::from_min_max(pos2(s.max.x, rect.min.y), rect.max);
            }
            ShellLayout::Inset => {
                let s = Rect::from_min_size(rect.min, vec2(side_w, rect.height()));
                if !phone {
                    side_rect = Some(s);
                }
                let m = Rect::from_min_max(pos2(if phone { rect.min.x + 8.0 } else { s.max.x }, rect.min.y + 8.0), rect.max - vec2(8.0, 8.0));
                paint::soft_shadow(&p, m, t.radius * 1.2, 10.0, vec2(0.0, 2.0), with_alpha(Color32::BLACK, if t.dark { 0.35 } else { 0.06 }));
                p.rect_filled(m, t.radius * 1.2, t.background);
                p.rect_stroke(m, t.radius * 1.2, Stroke::new(1.0, if t.dark { t.border } else { mix(t.border, t.background, 0.3) }), StrokeKind::Inside);
                main_rect = m;
            }
            ShellLayout::Floating => {
                let s = Rect::from_min_size(rect.min + vec2(8.0, 8.0), vec2(side_w, rect.height() - 16.0));
                if !phone {
                    paint::soft_shadow(&p, s, t.radius * 1.2, 16.0, vec2(0.0, 4.0), kit::shadow(&t));
                    p.rect_filled(s, t.radius * 1.2, if t.dark { mix(t.card, t.foreground, 0.02) } else { t.background });
                    p.rect_stroke(s, t.radius * 1.2, Stroke::new(1.0, t.border), StrokeKind::Inside);
                    side_rect = Some(s);
                }
                main_rect = Rect::from_min_max(pos2(if phone { rect.min.x } else { s.max.x + 8.0 }, rect.min.y), rect.max);
            }
            ShellLayout::Dual => {
                let rr = Rect::from_min_size(rect.min, vec2(if phone { 0.0 } else { 60.0 }, rect.height()));
                let panel_w = if phone { 0.0 } else { 232.0 * (1.0 - c) };
                if !phone {
                    p.rect_filled(rr, egui::CornerRadius { nw: outer_r as u8, sw: outer_r as u8, ne: 0, se: 0 }, mix(chrome, t.foreground, if t.dark { 0.015 } else { 0.02 }));
                    p.line_segment([rr.right_top(), rr.right_bottom()], Stroke::new(1.0, t.border));
                    rail_rect = Some(rr);
                    if panel_w > 1.0 {
                        let s = Rect::from_min_size(pos2(rr.max.x, rect.min.y), vec2(panel_w, rect.height()));
                        p.rect_filled(s, 0.0, chrome);
                        p.line_segment([s.right_top(), s.right_bottom()], Stroke::new(1.0, t.border));
                        side_rect = Some(s);
                    }
                }
                main_rect = Rect::from_min_max(pos2(rr.max.x + panel_w, rect.min.y), rect.max);
            }
            ShellLayout::TopNav | ShellLayout::BottomNav => {
                main_rect = rect;
            }
        }

        // inspector region
        let insp_e = if layout == ShellLayout::Inspector && self.inspector.is_some() && !phone {
            kit::spring(&ctx, id.with("insp"), if st.inspector { 1.0 } else { 0.0 }, 260.0, 28.0).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let insp_w = 300.0 * insp_e;
        let (main_rect, insp_rect) = if insp_w > 1.0 {
            (Rect::from_min_max(main_rect.min, pos2(main_rect.max.x - insp_w, main_rect.max.y)), Some(Rect::from_min_max(pos2(main_rect.max.x - insp_w, main_rect.min.y), main_rect.max)))
        } else {
            (main_rect, None)
        };

        // ---- sidebar ----------------------------------------------------------------------
        let mut upgrade_dismissed = st.upgrade_dismissed;
        let mut ws_idx: usize = glowkit_core::state(ui, id.with("ws"), || 0usize);
        let mut navigated = false;
        if let Some(s) = side_rect {
            let only = if layout == ShellLayout::Dual { Some(self.dual_section(st)) } else { None };
            let mut cfg_c = if layout == ShellLayout::Dual { 0.0 } else { c };
            if layout == ShellLayout::Dual {
                // secondary panel header
                let sec = &self.nav[only.unwrap_or(0)];
                let hr = Rect::from_min_size(s.min, vec2(232.0, 56.0));
                let hp = p.with_clip_rect(s);
                kit::text_at(&hp, pos2(hr.min.x + 16.0, hr.center().y - 7.0), Align2::LEFT_CENTER, &sec.title, 15.0, t.foreground, 200.0);
                let n = sec.items.len();
                kit::text_at(&hp, pos2(hr.min.x + 16.0, hr.center().y + 11.0), Align2::LEFT_CENTER, &format!("{n} pages"), 12.0, t.muted_foreground, 200.0);
                hp.line_segment([pos2(s.min.x, hr.max.y), pos2(s.max.x, hr.max.y)], Stroke::new(1.0, t.border));
                cfg_c = 0.0;
                let list = Rect::from_min_max(pos2(s.min.x, hr.max.y), pos2(s.min.x + 232.0, s.max.y));
                let out = kit::child(ui, s, |ui| {
                    sidebar(
                        ui,
                        list,
                        cfg_c,
                        SideCfg {
                            id: id.with("side"),
                            nav: self.nav,
                            route: self.route,
                            workspaces: &self.workspaces,
                            workspace: &mut ws_idx,
                            user: self.user.as_ref(),
                            upgrade: self.upgrade,
                            upgrade_dismissed: &mut upgrade_dismissed,
                            bg: chrome,
                            bounds: rect,
                            only_section: only,
                            header: false,
                            footer_user: false,
                            titles: false,
                        },
                    )
                });
                navigated |= out.navigated;
                if out.menu.is_some() {
                    menu = out.menu;
                }
            } else {
                let out = sidebar(
                    ui,
                    s,
                    cfg_c,
                    SideCfg {
                        id: id.with("side"),
                        nav: self.nav,
                        route: self.route,
                        workspaces: &self.workspaces,
                        workspace: &mut ws_idx,
                        user: self.user.as_ref(),
                        upgrade: self.upgrade,
                        upgrade_dismissed: &mut upgrade_dismissed,
                        bg: if layout == ShellLayout::Floating { if t.dark { mix(t.card, t.foreground, 0.02) } else { t.background } } else { chrome },
                        bounds: rect,
                        only_section: None,
                        header: true,
                        footer_user: true,
                        titles: true,
                    },
                );
                navigated |= out.navigated;
                if out.workspace_changed {
                    toast(&ctx, Toast::success(format!("Switched to {}", self.workspaces[ws_idx].name)).description("Your dashboards and members were reloaded."));
                }
                if out.menu.is_some() {
                    menu = out.menu;
                }
            }
        }
        if let Some(rr) = rail_rect {
            let out = self.dual_rail(ui, rr, id, st, &mut ws_idx, rect);
            if out.is_some() {
                menu = out;
            }
        }

        // ---- topbar -----------------------------------------------------------------------
        let top_h = match layout {
            ShellLayout::TopNav => 60.0,
            ShellLayout::BottomNav => 52.0,
            _ => 56.0,
        };
        let top_rect = Rect::from_min_size(main_rect.min, vec2(main_rect.width(), top_h));
        if layout == ShellLayout::BottomNav {
            self.app_bar(ui, top_rect, id, st, &mut notification, &mut menu, rect);
        } else {
            let top_out = self.topbar(ui, top_rect, id, st, phone, insp_rect.is_some() || self.inspector.is_some(), rect, &mut notification, &mut menu);
            navigated |= top_out;
        }
        let bottom_h = if layout == ShellLayout::BottomNav { 68.0 } else { 0.0 };
        if layout == ShellLayout::BottomNav {
            let br = Rect::from_min_max(pos2(main_rect.min.x, main_rect.max.y - bottom_h), main_rect.max);
            navigated |= self.bottom_bar(ui, br, id);
        }

        // ---- content ----------------------------------------------------------------------
        if *self.route != st.last_route {
            if !st.last_route.is_empty() {
                st.route_t = now;
            }
            st.last_route = self.route.clone();
        }
        let a = glowkit_core::ease(glowkit_core::Easing::OutCubic, ((now - st.route_t) as f32 / 0.28).clamp(0.0, 1.0));
        if a < 1.0 {
            ctx.request_repaint();
        }
        let body = Rect::from_min_max(pos2(main_rect.min.x, top_rect.max.y), pos2(main_rect.max.x, main_rect.max.y - bottom_h));
        let mut cr = body.shrink(self.padding);
        if layout == ShellLayout::TopNav && cr.width() > 1240.0 {
            let x = cr.center().x;
            cr = Rect::from_min_max(pos2(x - 620.0, cr.min.y), pos2(x + 620.0, cr.max.y));
        }
        let shifted = cr.translate(vec2(0.0, (1.0 - a) * 8.0));
        let mut theme_req = None;
        let inner = {
            let clip = body.intersect(ui.clip_rect());
            let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(shifted).id_salt(id.with("content")));
            cu.set_clip_rect(clip);
            cu.multiply_opacity(0.25 + 0.75 * a);
            let mut sc = ShellContext { route: self.route, rect: cr, bounds: rect, compact: rect.width() < 720.0, theme: &mut theme_req };
            content(&mut cu, &mut sc)
        };

        // ---- inspector --------------------------------------------------------------------
        if let (Some(ir), Some((title, add))) = (insp_rect, self.inspector.take()) {
            let ip = ui.painter_at(ir);
            ip.rect_filled(ir, egui::CornerRadius { ne: outer_r as u8, se: outer_r as u8, nw: 0, sw: 0 }, chrome);
            ip.line_segment([ir.left_top(), ir.left_bottom()], Stroke::new(1.0, t.border));
            let full = Rect::from_min_size(ir.min, vec2(300.0, ir.height()));
            kit::text_at(&ip, pos2(full.min.x + 18.0, full.min.y + 28.0), Align2::LEFT_CENTER, &title, 14.5, t.foreground, 220.0);
            let xr = Rect::from_center_size(pos2(full.max.x - 26.0, full.min.y + 28.0), vec2(28.0, 28.0));
            if xr.max.x <= ir.max.x + 0.5 {
                let x = kit::icon_button(ui, xr, id.with("insp_x"), LineIcon::X.into(), false);
                kit::tip(&x, "Close panel", Some("Ctrl+I"));
                if x.clicked() {
                    st.inspector = false;
                }
            }
            ip.line_segment([pos2(ir.min.x, full.min.y + 56.0), pos2(ir.max.x, full.min.y + 56.0)], Stroke::new(1.0, t.border));
            let body = Rect::from_min_max(pos2(full.min.x + 18.0, full.min.y + 72.0), pos2(full.max.x - 18.0, full.max.y - 16.0));
            let route = &mut *self.route;
            kit::child(ui, body.intersect(ir), |ui| {
                ui.set_width(body.width());
                ui.multiply_opacity(insp_e);
                kit::scroll(ui, "inspector", |ui| add(ui, route));
            });
        }

        // ---- phone drawer -----------------------------------------------------------------
        if has_sidebar {
            let de = kit::follow(&ctx, id.with("drawer"), if st.drawer && phone { 1.0 } else { 0.0 }, 14.0);
            if de > 0.01 {
                let dw = 272.0f32.min(rect.width() - 48.0);
                let mut close = false;
                let route = &mut *self.route;
                let nav = &mut *self.nav;
                let ws = &self.workspaces;
                let user = self.user.as_ref();
                let upgrade = self.upgrade;
                let mut picked = None;
                egui::Area::new(id.with("drawer_layer")).order(Order::Foreground).fixed_pos(rect.min).constrain(false).show(&ctx, |ui| {
                    ui.set_clip_rect(rect);
                    let bg = ui.interact(rect, id.with("drawer_bg"), Sense::click());
                    ui.painter().rect_filled(rect, outer_r, Color32::from_black_alpha((120.0 * de) as u8));
                    if bg.clicked() {
                        close = true;
                    }
                    let dr = Rect::from_min_size(pos2(rect.min.x - dw * (1.0 - de), rect.min.y), vec2(dw, rect.height()));
                    ui.painter().rect_filled(dr, egui::CornerRadius { nw: outer_r as u8, sw: outer_r as u8, ne: 0, se: 0 }, chrome);
                    ui.painter().line_segment([dr.right_top(), dr.right_bottom()], Stroke::new(1.0, t.border));
                    ui.interact(dr, id.with("drawer_block"), Sense::click());
                    let out = sidebar(
                        ui,
                        dr,
                        0.0,
                        SideCfg {
                            id: id.with("drawer_side"),
                            nav,
                            route,
                            workspaces: ws,
                            workspace: &mut ws_idx,
                            user,
                            upgrade,
                            upgrade_dismissed: &mut upgrade_dismissed,
                            bg: chrome,
                            bounds: rect,
                            only_section: None,
                            header: true,
                            footer_user: true,
                            titles: true,
                        },
                    );
                    if out.navigated {
                        close = true;
                    }
                    picked = out.menu;
                });
                if picked.is_some() {
                    menu = picked;
                }
                if close || ctx.input(|i| i.key_pressed(Key::Escape)) {
                    st.drawer = false;
                }
            }
        }

        // ---- command palette --------------------------------------------------------------
        if self.search {
            let leaves: Vec<(String, String, Option<AppIcon>, Option<String>)> =
                nav_leaves(self.nav).iter().map(|l| (l.label.clone(), l.route.clone(), l.icon, l.shortcut.clone())).collect();
            let pages: Vec<CommandItem> = leaves
                .iter()
                .map(|(label, route, icon, sc)| {
                    let mut ci = CommandItem::new(label.clone()).keywords(route.clone());
                    ci = ci.icon(match icon {
                        Some(AppIcon::Line(l)) => *l,
                        _ => LineIcon::ArrowRight,
                    });
                    if let Some(s) = sc {
                        ci = ci.shortcut(s.clone());
                    }
                    ci
                })
                .collect();
            let mut actions = vec![
                CommandItem::new("Toggle theme").icon(if t.dark { LineIcon::Sun } else { LineIcon::Moon }).keywords("dark light mode appearance"),
            ];
            if has_sidebar {
                actions.push(CommandItem::new("Toggle sidebar").icon(LineIcon::Sidebar).shortcut("Ctrl+B"));
            }
            if self.notifications.is_some() {
                actions.push(CommandItem::new("Mark notifications as read").icon(LineIcon::Bell));
            }
            let mut palette = CommandPalette::new(&mut st.palette).placeholder("Search pages and actions…").within(rect).group("Pages", pages).group("Actions", actions);
            if !self.commands.is_empty() {
                palette = palette.group("Commands", std::mem::take(&mut self.commands));
            }
            if let Some(chosen) = palette.show(ui) {
                if let Some((_, r, _, _)) = leaves.iter().find(|(l, _, _, _)| *l == chosen) {
                    *self.route = r.clone();
                } else {
                    match chosen.as_str() {
                        "Toggle theme" => theme_req = Some(if t.dark { Theme::light() } else { Theme::dark() }),
                        "Toggle sidebar" => {
                            if phone {
                                st.drawer = !st.drawer;
                            } else if auto_rail {
                                st.narrow_expanded = !st.narrow_expanded;
                            } else {
                                st.collapsed = !st.collapsed;
                            }
                        }
                        "Mark notifications as read" => {
                            if let Some(n) = self.notifications.as_deref_mut() {
                                n.iter_mut().for_each(|n| n.unread = false);
                            }
                            toast(&ctx, Toast::success("All caught up").description("Notifications marked as read."));
                        }
                        _ => command = Some(chosen),
                    }
                }
            }
        }
        if theme_req.is_none() {
            if let Some(th) = ctx.data_mut(|d| d.remove_temp::<Theme>(id.with("theme_req"))) {
                theme_req = Some(th);
            }
        }
        if let Some(th) = theme_req {
            st.theme = Some(th);
            ctx.request_repaint();
        }

        Toaster::new().within(rect.shrink(4.0)).position(ToastPosition::BottomRight).width(340.0f32.min(rect.width() - 24.0)).show(ui);

        st.upgrade_dismissed = upgrade_dismissed;
        glowkit_core::set_state(ui, id.with("ws"), ws_idx);
        navigated |= *self.route != start_route;
        ShellResponse { inner, command, menu, notification, navigated, rect }
    }

    fn dual_section(&self, st: &mut ShellState) -> usize {
        let from_route = self.nav.iter().position(|s| s.items.iter().any(|i| i.contains(self.route)));
        st.dual_section.or(from_route).unwrap_or(0).min(self.nav.len().saturating_sub(1))
    }

    /// The dual layout's icon rail: workspace logo, one icon per section, user avatar.
    fn dual_rail(&mut self, ui: &mut Ui, rr: Rect, id: Id, st: &mut ShellState, ws_idx: &mut usize, bounds: Rect) -> Option<String> {
        let t = Theme::of(ui);
        let p = ui.painter_at(rr);
        let mut menu = None;
        // logo + workspace menu
        let lr = Rect::from_center_size(pos2(rr.center().x, rr.min.y + 30.0), vec2(34.0, 34.0));
        let ws = &self.workspaces[(*ws_idx).min(self.workspaces.len().saturating_sub(1))];
        kit::logo(&p, lr, &t, &ws.name.chars().next().unwrap_or('A').to_string());
        let lresp = ui.interact(lr, id.with("rail_logo"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        Tooltip::new(ws.name.clone()).placement(Placement::Right).delay(0.15).show(&lresp);
        let mut entries = vec![MenuEntry::label("Workspaces")];
        for (i, w) in self.workspaces.iter().enumerate() {
            entries.push(MenuEntry::radio(w.name.clone(), i == *ws_idx));
        }
        if let Some(sel) = DropdownMenu::new(entries).width(220.0).within(bounds).show(&lresp) {
            if let Some(i) = self.workspaces.iter().position(|w| w.name == sel) {
                *ws_idx = i;
                toast(ui.ctx(), Toast::success(format!("Switched to {sel}")));
            }
        }
        p.line_segment([pos2(rr.min.x + 14.0, rr.min.y + 58.0), pos2(rr.max.x - 14.0, rr.min.y + 58.0)], Stroke::new(1.0, t.border));
        let cur = self.dual_section(st);
        let mut y = rr.min.y + 72.0;
        let mut active_y = None;
        for (si, sec) in self.nav.iter().enumerate() {
            let br = Rect::from_center_size(pos2(rr.center().x, y + 18.0), vec2(40.0, 36.0));
            let sid = id.with(("rail", si));
            let resp = ui.interact(br, sid, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let hv = kit::hover(ui, sid, resp.hovered());
            let on = si == cur;
            if on {
                active_y = Some(br.min.y);
            } else if hv > 0.01 {
                p.rect_filled(br, t.radius * 0.7, kit::hover_fill(&t, hv * 1.4));
            }
            let col = if on { t.accent } else { t.muted_foreground.lerp_to_gamma(t.foreground, hv) };
            sec.icon.unwrap_or(AppIcon::Line(LineIcon::Folder)).paint(&p, Rect::from_center_size(br.center(), vec2(18.0, 18.0)), col);
            Tooltip::new(sec.title.clone()).placement(Placement::Right).delay(0.1).show(&resp);
            if resp.clicked() {
                st.dual_section = Some(si);
                st.collapsed = false;
                st.narrow_expanded = true;
            }
            y += 44.0;
        }
        if let Some(ay) = active_y {
            let yy = kit::spring(ui.ctx(), id.with("rail_y"), ay, 380.0, 30.0);
            let r = Rect::from_min_size(pos2(rr.center().x - 20.0, yy), vec2(40.0, 36.0));
            p.rect_filled(r, t.radius * 0.7, with_alpha(t.accent, if t.dark { 0.16 } else { 0.1 }));
            p.rect_filled(Rect::from_min_size(pos2(rr.min.x + 1.0, yy + 10.0), vec2(3.0, 16.0)), 2.0, t.accent);
            // repaint icon on top of the highlight
            let sec = &self.nav[cur];
            sec.icon.unwrap_or(AppIcon::Line(LineIcon::Folder)).paint(&p, Rect::from_center_size(pos2(rr.center().x, ay + 18.0), vec2(18.0, 18.0)), t.accent);
        }
        if let Some(user) = &self.user {
            let ar = Rect::from_center_size(pos2(rr.center().x, rr.max.y - 30.0), vec2(32.0, 32.0));
            kit::avatar(ui, ar, &user.name);
            let resp = ui.interact(ar, id.with("rail_user"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            if let Some(sel) = DropdownMenu::new(user_menu(user)).width(230.0).within(bounds).show(&resp) {
                menu = Some(sel);
            }
            let hr = Rect::from_center_size(pos2(rr.center().x, rr.max.y - 76.0), vec2(36.0, 36.0));
            let h = kit::icon_button(ui, hr, id.with("rail_help"), LineIcon::Help.into(), false);
            kit::tip(&h, "Help & docs", None);
            if h.clicked() {
                toast(ui.ctx(), Toast::info("Help center").description("Docs, guides and keyboard shortcuts."));
            }
        }
        menu
    }

    /// The desktop topbar. Returns true when it navigated.
    #[allow(clippy::too_many_arguments)]
    fn topbar(
        &mut self,
        ui: &mut Ui,
        r: Rect,
        id: Id,
        st: &mut ShellState,
        phone: bool,
        inspector: bool,
        bounds: Rect,
        notification: &mut Option<usize>,
        menu: &mut Option<String>,
    ) -> bool {
        let t = Theme::of(ui);
        let p = ui.painter_at(r);
        let layout = self.layout;
        p.line_segment([pos2(r.min.x, r.max.y - 0.5), pos2(r.max.x, r.max.y - 0.5)], Stroke::new(1.0, t.border));
        let cy = r.center().y;
        let mut x = r.min.x + 14.0;
        let mut navigated = false;
        let narrow = r.width() < 760.0;
        if layout == ShellLayout::TopNav {
            let lr = Rect::from_center_size(pos2(x + 15.0, cy), vec2(30.0, 30.0));
            let ws = self.workspaces.first().map(|w| w.name.clone()).unwrap_or_default();
            kit::logo(&p, lr, &t, &ws.chars().next().unwrap_or('A').to_string());
            x = lr.max.x + 10.0;
            if !narrow {
                let br = kit::text_at(&p, pos2(x, cy), Align2::LEFT_CENTER, &ws, 15.0, t.foreground, 140.0);
                x = br.max.x + 22.0;
            }
            navigated |= self.top_tabs(ui, Rect::from_min_max(pos2(x, r.min.y), pos2(r.max.x - 300.0, r.max.y)), id, bounds);
        } else {
            let tr = Rect::from_center_size(pos2(x + 14.0, cy), vec2(30.0, 30.0));
            let tog = kit::icon_button(ui, tr, id.with("toggle"), if phone { LineIcon::Menu.into() } else { LineIcon::Sidebar.into() }, false);
            kit::tip(&tog, "Toggle sidebar", Some("Ctrl+B"));
            if tog.clicked() {
                if phone {
                    st.drawer = !st.drawer;
                } else if r.width() + 256.0 < 1040.0 + 256.0 && self.auto_rail_hint(bounds) {
                    st.narrow_expanded = !st.narrow_expanded;
                } else {
                    st.collapsed = !st.collapsed;
                }
            }
            x = tr.max.x + 10.0;
            p.line_segment([pos2(x, cy - 9.0), pos2(x, cy + 9.0)], Stroke::new(1.0, t.border));
            x += 12.0;
            let crumbs = self.breadcrumb.clone().unwrap_or_else(|| {
                let mut v = vec![self.workspaces.first().map(|w| w.name.clone()).unwrap_or_else(|| "Home".into())];
                v.extend(nav_path(self.nav, self.route));
                v
            });
            if !crumbs.is_empty() {
                let cw = (r.max.x - x - if narrow { 150.0 } else { 420.0 }).max(60.0);
                let cr = Rect::from_min_size(pos2(x, cy - 13.0), vec2(cw, 26.0));
                let items: Vec<&str> = crumbs.iter().map(|s| s.as_str()).collect();
                let clicked = kit::child(ui, cr, |ui| Breadcrumb::new(&items).max_items(if narrow { 2 } else { 4 }).size(13.5).show(ui));
                if let Some(i) = clicked {
                    // navigate to the first leaf under the clicked crumb
                    if i > 0 {
                        let path = nav_path(self.nav, self.route);
                        if let Some(label) = path.get(i - 1) {
                            let target = nav_leaves(self.nav).iter().find(|l| {
                                let lp = nav_path(self.nav, &l.route);
                                lp.get(i - 1) == Some(label)
                            }).map(|l| l.route.clone());
                            if let Some(tg) = target {
                                *self.route = tg;
                                navigated = true;
                            }
                        }
                    } else if let Some(first) = nav_leaves(self.nav).first() {
                        *self.route = first.route.clone();
                        navigated = true;
                    }
                }
            }
        }

        // ---- right cluster ----------------------------------------------------------------
        let mut rx = r.max.x - 14.0;
        let show_user = matches!(layout, ShellLayout::TopNav | ShellLayout::Dual) || phone;
        if let (true, Some(user)) = (show_user && layout != ShellLayout::Dual, self.user.clone()) {
            let ar = Rect::from_center_size(pos2(rx - 16.0, cy), vec2(32.0, 32.0));
            kit::avatar(ui, ar, &user.name);
            let resp = ui.interact(ar, id.with("top_user"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            if let Some(sel) = DropdownMenu::new(user_menu(&user)).width(230.0).within(bounds).show(&resp) {
                *menu = Some(sel);
            }
            rx = ar.min.x - 10.0;
        }
        if self.theme_toggle {
            let br = Rect::from_center_size(pos2(rx - 16.0, cy), vec2(32.0, 32.0));
            if theme_button(ui, br, id.with("theme_btn")).clicked() {
                let next = if t.dark { Theme::light() } else { Theme::dark() };
                ui.ctx().data_mut(|d| d.insert_temp(id.with("theme_req"), next));
            }
            rx = br.min.x - 4.0;
        }
        if let Some(notes) = self.notifications.as_deref_mut() {
            let br = Rect::from_center_size(pos2(rx - 16.0, cy), vec2(32.0, 32.0));
            if let Some(i) = bell(ui, br, id.with("bell"), notes, bounds) {
                *notification = Some(i);
            }
            rx = br.min.x - 4.0;
        }
        if inspector && layout == ShellLayout::Inspector && !phone {
            let br = Rect::from_center_size(pos2(rx - 16.0, cy), vec2(32.0, 32.0));
            let b = kit::icon_button(ui, br, id.with("insp_btn"), LineIcon::Layers.into(), st.inspector);
            kit::tip(&b, "Toggle details panel", Some("Ctrl+I"));
            if b.clicked() {
                st.inspector = !st.inspector;
            }
            rx = br.min.x - 4.0;
        }
        if self.search {
            if r.width() < 900.0 {
                let br = Rect::from_center_size(pos2(rx - 16.0, cy), vec2(32.0, 32.0));
                let b = kit::icon_button(ui, br, id.with("search_btn"), LineIcon::Search.into(), false);
                kit::tip(&b, "Search", Some("Ctrl+K"));
                if b.clicked() {
                    st.palette = true;
                }
                rx = br.min.x - 6.0;
            } else {
                let sw = 248.0;
                let sr = Rect::from_min_max(pos2(rx - 6.0 - sw, cy - 17.0), pos2(rx - 6.0, cy + 17.0));
                let resp = ui.interact(sr, id.with("search"), Sense::click()).on_hover_cursor(CursorIcon::Text);
                let hv = kit::hover(ui, id.with("search"), resp.hovered());
                p.rect_filled(sr, t.radius * 0.7, if t.dark { mix(t.background, t.foreground, 0.03 + 0.02 * hv) } else { mix(t.background, t.foreground, 0.02 + 0.015 * hv) });
                p.rect_stroke(sr, t.radius * 0.7, Stroke::new(1.0, t.border.lerp_to_gamma(t.muted_foreground, hv * 0.4)), StrokeKind::Inside);
                LineIcon::Search.paint(&p, Rect::from_center_size(pos2(sr.min.x + 18.0, cy), vec2(15.0, 15.0)), t.muted_foreground);
                p.text(pos2(sr.min.x + 34.0, cy), Align2::LEFT_CENTER, "Search…", egui::FontId::proportional(13.5), t.muted_foreground);
                kit::kbd(&p, pos2(sr.max.x - 8.0, cy), "Ctrl K", &t);
                if resp.clicked() {
                    st.palette = true;
                }
                rx = sr.min.x - 8.0;
            }
        }
        if let Some(actions) = self.actions.take() {
            let ar = Rect::from_min_max(pos2(rx - 320.0, r.min.y), pos2(rx, r.max.y));
            kit::child(ui, ar, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    actions(ui);
                });
            });
        }
        navigated
    }

    fn auto_rail_hint(&self, bounds: Rect) -> bool {
        bounds.width() < 1040.0
    }

    /// TopNav: top-level items as tabs with a sliding underline; parents open a dropdown.
    fn top_tabs(&mut self, ui: &mut Ui, area: Rect, id: Id, bounds: Rect) -> bool {
        let t = Theme::of(ui);
        let p = ui.painter_at(area);
        let mut x = area.min.x;
        let mut navigated = false;
        let mut active: Option<(f32, f32)> = None;
        let items: Vec<(String, String, bool, Vec<(String, String)>)> = self
            .nav
            .iter()
            .flat_map(|s| s.items.iter())
            .take(7)
            .map(|i| (i.label.clone(), i.route.clone(), i.contains(self.route), i.children.iter().map(|c| (c.label.clone(), c.route.clone())).collect()))
            .collect();
        for (k, (label, route, on, kids)) in items.iter().enumerate() {
            let w = kit::text_w(&p, label, 13.5) + 20.0 + if kids.is_empty() { 0.0 } else { 16.0 };
            if x + w > area.max.x {
                break;
            }
            let tr = Rect::from_min_size(pos2(x, area.center().y - 16.0), vec2(w, 32.0));
            let tid = id.with(("tab", k));
            let resp = ui.interact(tr, tid, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let hv = kit::hover(ui, tid, resp.hovered());
            if hv > 0.01 {
                p.rect_filled(tr, t.radius * 0.6, kit::hover_fill(&t, hv * 1.2));
            }
            let col = if *on { t.foreground } else { t.muted_foreground.lerp_to_gamma(t.foreground, hv) };
            p.text(pos2(tr.min.x + 10.0, tr.center().y), Align2::LEFT_CENTER, label, egui::FontId::proportional(13.5), col);
            if !kids.is_empty() {
                chevron(&p, pos2(tr.max.x - 14.0, tr.center().y + 1.0), 8.0, std::f32::consts::FRAC_PI_2, t.muted_foreground);
                let entries: Vec<MenuEntry> = kids.iter().map(|(l, _)| MenuEntry::item(l.clone())).collect();
                if let Some(sel) = DropdownMenu::new(entries).width(180.0).within(bounds).show(&resp) {
                    if let Some((_, rt)) = kids.iter().find(|(l, _)| *l == sel) {
                        *self.route = rt.clone();
                        navigated = true;
                    }
                }
            } else if resp.clicked() {
                *self.route = route.clone();
                navigated = true;
            }
            if *on {
                active = Some((tr.min.x + 8.0, w - 16.0));
            }
            x += w + 2.0;
        }
        if let Some((ax, aw)) = active {
            let sx = kit::spring(ui.ctx(), id.with("tab_x"), ax, 320.0, 28.0);
            let sw = kit::follow(ui.ctx(), id.with("tab_w"), aw, 18.0);
            p.rect_filled(Rect::from_min_size(pos2(sx, area.max.y - 2.0), vec2(sw, 2.0)), 1.0, t.accent);
        }
        navigated
    }

    /// BottomNav: mobile app bar.
    #[allow(clippy::too_many_arguments)]
    fn app_bar(&mut self, ui: &mut Ui, r: Rect, id: Id, _st: &mut ShellState, notification: &mut Option<usize>, menu: &mut Option<String>, bounds: Rect) {
        let t = Theme::of(ui);
        let p = ui.painter_at(r);
        p.line_segment([pos2(r.min.x, r.max.y - 0.5), pos2(r.max.x, r.max.y - 0.5)], Stroke::new(1.0, t.border));
        let cy = r.center().y;
        if let Some(user) = self.user.clone() {
            let ar = Rect::from_center_size(pos2(r.min.x + 30.0, cy), vec2(30.0, 30.0));
            kit::avatar(ui, ar, &user.name);
            let resp = ui.interact(ar, id.with("app_user"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            if let Some(sel) = DropdownMenu::new(user_menu(&user)).width(220.0).within(bounds).show(&resp) {
                *menu = Some(sel);
            }
        }
        let title = self.title.clone().unwrap_or_else(|| nav_find(self.nav, self.route).map(|i| i.label.clone()).unwrap_or_default());
        kit::text_at(&p, pos2(r.min.x + 56.0, cy), Align2::LEFT_CENTER, &title, 16.5, t.foreground, r.width() - 150.0);
        let mut rx = r.max.x - 12.0;
        if self.theme_toggle {
            let br = Rect::from_center_size(pos2(rx - 16.0, cy), vec2(32.0, 32.0));
            if theme_button(ui, br, id.with("theme_btn")).clicked() {
                let next = if t.dark { Theme::light() } else { Theme::dark() };
                ui.ctx().data_mut(|d| d.insert_temp(id.with("theme_req"), next));
            }
            rx = br.min.x - 2.0;
        }
        if let Some(notes) = self.notifications.as_deref_mut() {
            let br = Rect::from_center_size(pos2(rx - 16.0, cy), vec2(32.0, 32.0));
            if let Some(i) = bell(ui, br, id.with("bell"), notes, bounds) {
                *notification = Some(i);
            }
        }
    }

    /// BottomNav: the tab bar. Returns true when it navigated.
    fn bottom_bar(&mut self, ui: &mut Ui, r: Rect, id: Id) -> bool {
        let t = Theme::of(ui);
        let p = ui.painter_at(r);
        p.rect_filled(r, egui::CornerRadius { sw: if self.rounded { t.radius as u8 } else { 0 }, se: if self.rounded { t.radius as u8 } else { 0 }, nw: 0, ne: 0 }, kit::chrome_bg(&t));
        p.line_segment([r.left_top(), r.right_top()], Stroke::new(1.0, t.border));
        let items: Vec<(String, String, Option<AppIcon>, Option<String>, bool)> = self
            .nav
            .iter()
            .flat_map(|s| s.items.iter())
            .take(5)
            .map(|i| {
                let target = if i.children.is_empty() { i.route.clone() } else { i.children[0].route.clone() };
                (i.label.clone(), target, i.icon, i.badge.clone(), i.contains(self.route))
            })
            .collect();
        let n = items.len().max(1);
        let w = r.width() / n as f32;
        let mut navigated = false;
        let mut active_x = None;
        for (k, (label, route, icon, badge, on)) in items.iter().enumerate() {
            let cell = Rect::from_min_size(pos2(r.min.x + k as f32 * w, r.min.y), vec2(w, r.height()));
            let cid = id.with(("bn", k));
            let resp = ui.interact(cell, cid, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let hv = kit::hover(ui, cid, resp.hovered());
            let ic = pos2(cell.center().x, cell.min.y + 24.0);
            if *on {
                active_x = Some(ic.x);
            }
            let col = if *on { t.accent } else { t.muted_foreground.lerp_to_gamma(t.foreground, hv) };
            icon.unwrap_or(AppIcon::Line(LineIcon::Home)).paint(&p, Rect::from_center_size(ic, vec2(19.0, 19.0)), col);
            if let Some(b) = badge {
                let bc = ic + vec2(11.0, -8.0);
                let g = p.layout_no_wrap(b.clone(), egui::FontId::proportional(9.5), Color32::WHITE);
                let br = Rect::from_center_size(bc, vec2((g.size().x + 8.0).max(16.0), 16.0));
                p.rect_filled(br.expand(2.0), 10.0, kit::chrome_bg(&t));
                p.rect_filled(br, 8.0, t.destructive);
                p.galley(br.center() - g.size() / 2.0, g, Color32::WHITE);
            }
            kit::text_at(&p, pos2(cell.center().x, cell.min.y + 48.0), Align2::CENTER_CENTER, label, 11.5, if *on { t.foreground } else { col }, w - 6.0);
            if resp.clicked() && *self.route != *route {
                *self.route = route.clone();
                navigated = true;
            }
        }
        if let Some(ax) = active_x {
            let sx = kit::spring(ui.ctx(), id.with("bn_x"), ax, 300.0, 26.0);
            let pill = Rect::from_center_size(pos2(sx, r.min.y + 24.0), vec2(52.0, 30.0));
            // painted under the icons: use a separate layer-free trick (low alpha) so icons stay visible
            p.rect_filled(pill, 15.0, with_alpha(t.accent, if t.dark { 0.14 } else { 0.1 }));
        }
        navigated
    }
}

/// Animated sun / moon toggle.
fn theme_button(ui: &mut Ui, r: Rect, id: Id) -> egui::Response {
    let t = Theme::of(ui);
    let resp = ui.interact(r, id, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    let hv = kit::hover(ui, id, resp.hovered());
    let p = ui.painter();
    if hv > 0.01 {
        p.rect_filled(r, t.radius * 0.6, kit::hover_fill(&t, hv * 1.4));
    }
    let d = kit::follow(ui.ctx(), id.with("d"), if t.dark { 1.0 } else { 0.0 }, 10.0);
    let col = t.muted_foreground.lerp_to_gamma(t.foreground, hv);
    let s = 17.0;
    let mut sp = p.clone();
    sp.multiply_opacity(1.0 - d);
    LineIcon::Sun.paint(&sp, Rect::from_center_size(r.center() + vec2(0.0, d * 6.0), vec2(s * (1.0 - 0.3 * d), s * (1.0 - 0.3 * d))), col);
    let mut mp = p.clone();
    mp.multiply_opacity(d);
    LineIcon::Moon.paint(&mp, Rect::from_center_size(r.center() + vec2(0.0, (1.0 - d) * -6.0), vec2(s * (0.7 + 0.3 * d), s * (0.7 + 0.3 * d))), col);
    kit::tip(&resp, if t.dark { "Light mode" } else { "Dark mode" }, None);
    resp
}

/// Notifications bell with an unread badge and a dropdown. Returns a clicked notification.
fn bell(ui: &mut Ui, r: Rect, id: Id, notes: &mut [ShellNotification], bounds: Rect) -> Option<usize> {
    let t = Theme::of(ui);
    let open = Popover::is_open(ui.ctx(), id);
    kit::icon_button(ui, r, id, LineIcon::Bell.into(), open);
    let resp = ui.interact(r, id, Sense::click());
    let unread = notes.iter().filter(|n| n.unread).count();
    if unread > 0 {
        let p = ui.painter();
        let txt = if unread > 9 { "9+".to_owned() } else { unread.to_string() };
        let g = p.layout_no_wrap(txt, egui::FontId::proportional(9.5), Color32::WHITE);
        let bc = r.center() + vec2(7.0, -7.0);
        let br = Rect::from_center_size(bc, vec2((g.size().x + 7.0).max(15.0), 15.0));
        p.rect_filled(br.expand(1.5), 9.0, t.background);
        p.rect_filled(br, 8.0, t.destructive);
        p.galley(br.center() - g.size() / 2.0, g, Color32::WHITE);
    }
    if !open {
        kit::tip(&resp, "Notifications", None);
    }
    let mut clicked = None;
    Popover::new().width(360.0).placement(Placement::Bottom).arrow(false).within(bounds).show(&resp, |ui| {
        let t = Theme::of(ui);
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        let w = ui.available_width();
        let hr = kit::alloc_row(ui, 30.0);
        let p = ui.painter().clone();
        kit::text_at(&p, pos2(hr.min.x, hr.center().y), Align2::LEFT_CENTER, "Notifications", 14.5, t.foreground, 160.0);
        if unread > 0 {
            let lr = kit::pill(&p, pos2(hr.min.x + kit::text_w(&p, "Notifications", 14.5) + 8.0, hr.center().y), false, &format!("{unread} new"), t.accent, with_alpha(t.accent, 0.14), 10.5);
            let _ = lr;
            let mr = Rect::from_min_max(pos2(hr.max.x - 110.0, hr.min.y), hr.max);
            let mresp = ui.interact(mr, id.with("mark_all"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let h = kit::hover(ui, id.with("mark_all"), mresp.hovered());
            kit::text_at(&p, pos2(mr.max.x, mr.center().y), Align2::RIGHT_CENTER, "Mark all as read", 12.5, t.muted_foreground.lerp_to_gamma(t.accent, h), 110.0);
            if mresp.clicked() {
                notes.iter_mut().for_each(|n| n.unread = false);
            }
        }
        ui.add_space(8.0);
        let sep = kit::alloc_row(ui, 1.0);
        p.line_segment([pos2(sep.min.x - 12.0, sep.center().y), pos2(sep.max.x + 12.0, sep.center().y)], Stroke::new(1.0, t.border));
        ui.add_space(6.0);
        if notes.is_empty() {
            let er = kit::alloc_row(ui, 90.0);
            LineIcon::CircleCheck.paint(&p, Rect::from_center_size(er.center() - vec2(0.0, 14.0), vec2(22.0, 22.0)), t.muted_foreground);
            p.text(er.center() + vec2(0.0, 16.0), Align2::CENTER_CENTER, "You're all caught up", egui::FontId::proportional(13.0), t.muted_foreground);
        }
        for (i, n) in notes.iter_mut().enumerate().take(6) {
            let rr = kit::alloc_row(ui, 66.0);
            let rr = Rect::from_min_max(rr.min - vec2(6.0, 0.0), rr.max + vec2(6.0, 0.0));
            let (rresp, _) = kit::row(ui, rr, id.with(("n", i)), t.radius * 0.6);
            let p = ui.painter().clone();
            let ic = pos2(rr.min.x + 24.0, rr.min.y + 24.0);
            p.circle_filled(ic, 16.0, if n.unread { with_alpha(t.accent, 0.14) } else { t.muted });
            n.icon.paint(&p, Rect::from_center_size(ic, vec2(15.0, 15.0)), if n.unread { t.accent } else { t.muted_foreground });
            let tx = rr.min.x + 50.0;
            let time_w = kit::text_w(&p, &n.time, 11.5);
            kit::text_at(&p, pos2(rr.max.x - 8.0, rr.min.y + 14.0), Align2::RIGHT_CENTER, &n.time, 11.5, t.muted_foreground, 80.0);
            kit::text_at(&p, pos2(tx, rr.min.y + 14.0), Align2::LEFT_CENTER, &n.title, 13.5, t.foreground, rr.max.x - tx - time_w - 22.0);
            let g = kit::wrapped(&p, &n.body, 12.5, t.muted_foreground, rr.max.x - tx - 22.0, 2);
            p.galley(pos2(tx, rr.min.y + 24.0), g, t.muted_foreground);
            if n.unread {
                p.circle_filled(pos2(rr.max.x - 10.0, rr.min.y + 40.0), 3.5, t.accent);
            }
            if rresp.clicked() {
                n.unread = false;
                clicked = Some(i);
            }
            ui.add_space(2.0);
        }
        ui.add_space(6.0);
        let fr = kit::alloc_row(ui, 32.0);
        if kit::button_at(ui, fr, "View all notifications", glowkit_components::ButtonVariant::Outline).clicked() {
            Popover::set_open(ui.ctx(), id, false);
            toast(ui.ctx(), Toast::info("Opening inbox").description("All notifications live in your inbox."));
        }
        let _ = w;
    });
    clicked
}
