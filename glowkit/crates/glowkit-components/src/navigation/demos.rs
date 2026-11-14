use super::*;
use crate::overlays::kit::{self, Btn};
use crate::overlays::{LineIcon, MenuEntry};
use egui::{pos2, vec2, Align2, FontId, Rect, RichText, Stroke, Ui, UiBuilder};
use glowkit_core::color::mix;
use glowkit_core::demo::{Category::Navigation as C, Demo};
use glowkit_core::Theme;

fn st<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, init: impl FnOnce() -> T) -> T {
    glowkit_core::state(ui, ui.scope_id().with(key), init)
}

fn set<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, v: T) {
    glowkit_core::set_state(ui, ui.scope_id().with(key), v);
}

fn once(ui: &Ui, key: &str) -> bool {
    let done = st(ui, key, || false);
    if !done {
        set(ui, key, true);
    }
    !done
}

fn caption(ui: &mut Ui, text: &str) {
    let t = Theme::of(ui);
    ui.label(RichText::new(text).size(12.0).color(t.muted_foreground));
}

/// Fake landing-page content for scroll demos.
fn landing_content(ui: &mut Ui, width: f32) {
    let t = Theme::of(ui);
    let sk = mix(t.background, t.foreground, if t.dark { 0.07 } else { 0.06 });
    ui.add_space(110.0);
    let (r, _) = ui.allocate_exact_size(vec2(width, 1400.0), egui::Sense::hover());
    let p = ui.painter();
    p.text(
        pos2(r.center().x, r.min.y + 10.0),
        Align2::CENTER_CENTER,
        "Build beautiful apps in Rust",
        FontId::proportional(30.0),
        t.foreground,
    );
    p.text(
        pos2(r.center().x, r.min.y + 50.0),
        Align2::CENTER_CENTER,
        "Scroll this area to see the navbar react.",
        FontId::proportional(15.0),
        t.muted_foreground,
    );
    for i in 0..12 {
        let y = r.min.y + 100.0 + i as f32 * 108.0;
        let w = r.width() - 80.0;
        let card = Rect::from_min_size(pos2(r.min.x + 40.0, y), vec2(w, 92.0));
        p.rect_stroke(card, t.radius, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        p.rect_filled(Rect::from_min_size(card.min + vec2(18.0, 18.0), vec2(w * 0.3, 12.0)), 6.0, sk);
        p.rect_filled(Rect::from_min_size(card.min + vec2(18.0, 42.0), vec2(w * 0.7, 9.0)), 4.0, sk);
        p.rect_filled(Rect::from_min_size(card.min + vec2(18.0, 60.0), vec2(w * 0.55, 9.0)), 4.0, sk);
    }
}

/// A scrollable landing page inside `canvas` that auto-scrolls (paused while hovered).
/// Returns the scroll offset.
fn auto_scroll_canvas(ui: &mut Ui, canvas: Rect, start_scrolled: bool) -> f32 {
    let hovered = ui.rect_contains_pointer(canvas);
    let dt = glowkit_core::dt(ui);
    let mut clock: f32 = st(ui, "auto_clock", || 0.0);
    if !hovered {
        clock += dt;
        ui.ctx().request_repaint();
    }
    set(ui, "auto_clock", clock);
    // dwell, scroll down, dwell, scroll part-way up, repeat (9 s cycle)
    let c = (clock + if start_scrolled { 4.5 } else { 0.0 }) % 9.0;
    let s = |a: f32, b: f32| glowkit_core::anim::smoothstep(a, b, c);
    let auto = 420.0 * s(2.5, 4.0) - 260.0 * s(6.0, 7.2) - 160.0 * s(8.2, 9.0);
    let mut child = ui.new_child(UiBuilder::new().max_rect(canvas.shrink(1.0)));
    child.set_clip_rect(canvas.shrink(1.0).intersect(ui.clip_rect()));
    let mut sa = egui::ScrollArea::vertical().id_salt("landing").auto_shrink([false, false]);
    if !hovered {
        sa = sa.vertical_scroll_offset(auto);
    }
    let out = sa.show(&mut child, |ui| landing_content(ui, canvas.width() - 20.0));
    out.state.clamped_offset().y
}

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "tabs",
            name: "Tabs",
            category: C,
            description: "Tab strips whose active indicator glides between tabs on a spring: segmented, underline and pill styles.",
            covers: &["shadcn/ui: Tabs", "Animate UI: Tabs", "Origin UI: Tabs", "Aceternity: Animated Tabs", "Motion Primitives: Animated Background"],
            code: r##"ui.add(Tabs::new(&mut tab, &["Account", "Password", "Settings"]));
ui.add(Tabs::new(&mut tab2, &["Overview", "Analytics", "Reports"])
    .icons(&[LineIcon::Home, LineIcon::Chart, LineIcon::File])
    .style(TabsStyle::Underline));
ui.add(Tabs::new(&mut tab3, &["All", "Active", "Archived"]).style(TabsStyle::Pill));"##,
            show: |ui| {
                let t = Theme::of(ui);
                let mut tabs: [usize; 3] = st(ui, "tabs", || [0, 1, 0]);
                ui.add_space(10.0);
                ui.columns(2, |cols| {
                    let ui = &mut cols[0];
                    caption(ui, "Segmented");
                    ui.add(Tabs::new(&mut tabs[0], &["Account", "Password", "Settings"]));
                    ui.add_space(10.0);
                    let body = match tabs[0] {
                        0 => ("Account", "Make changes to your account here. Click save when you're done."),
                        1 => ("Password", "Change your password here. After saving, you'll be logged out."),
                        _ => ("Settings", "Manage notification and privacy preferences."),
                    };
                    egui::Frame::new().stroke(Stroke::new(1.0, t.border)).corner_radius(t.corner()).inner_margin(16).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(body.0).size(16.0));
                        ui.label(RichText::new(body.1).size(13.0).color(t.muted_foreground));
                    });
                    let ui = &mut cols[1];
                    caption(ui, "Underline with icons");
                    ui.add(Tabs::new(&mut tabs[1], &["Overview", "Analytics", "Reports"]).icons(&[LineIcon::Home, LineIcon::Chart, LineIcon::File]).style(TabsStyle::Underline));
                    ui.add_space(28.0);
                    caption(ui, "Pill");
                    ui.add(Tabs::new(&mut tabs[2], &["All", "Active", "Drafts", "Archived"]).style(TabsStyle::Pill));
                    ui.add_space(28.0);
                    caption(ui, "Full width");
                    let mut v: usize = st(ui, "fw", || 1);
                    ui.add(Tabs::new(&mut v, &["Monthly", "Yearly"]).full_width(true));
                    set(ui, "fw", v);
                });
                set(ui, "tabs", tabs);
            },
        },
        Demo {
            id: "breadcrumb",
            name: "Breadcrumb",
            category: C,
            description: "Breadcrumb trails with chevron, slash or dot separators, animated hover underlines and a collapsible “…” that opens a menu of hidden items.",
            covers: &["shadcn/ui: Breadcrumb", "Origin UI: Breadcrumb", "Kibo UI: Breadcrumb"],
            code: r##"Breadcrumb::new(&["Home", "Components", "Breadcrumb"]).home_icon(true).show(ui);
Breadcrumb::new(&["Home", "Docs", "Building", "Data", "Fetching"])
    .max_items(3)
    .separator(BreadcrumbSeparator::Slash)
    .show(ui);"##,
            show: |ui| {
                let mut last: String = st(ui, "last", String::new);
                ui.add_space(30.0);
                ui.vertical_centered(|ui| {
                    let mut row = |ui: &mut Ui, label: &str, items: &[&str], b: Breadcrumb| {
                        caption(ui, label);
                        let w = 420.0;
                        ui.allocate_ui(vec2(w, 34.0), |ui| {
                            if let Some(i) = b.show(ui) {
                                last = format!("Navigated to “{}”", items[i]);
                            }
                        });
                        ui.add_space(22.0);
                    };
                    let a = ["Home", "Components", "Breadcrumb"];
                    row(ui, "Chevron + home icon", &a, Breadcrumb::new(&a).home_icon(true));
                    let b = ["Home", "Docs", "Building Your Application", "Data Fetching", "Caching"];
                    row(ui, "Collapsed (click the …)", &b, Breadcrumb::new(&b).max_items(3).separator(BreadcrumbSeparator::Slash));
                    let c = ["Workspace", "Projects", "glowkit", "Settings"];
                    row(ui, "Dot separator", &c, Breadcrumb::new(&c).separator(BreadcrumbSeparator::Dot));
                    let t = Theme::of(ui);
                    ui.label(RichText::new(&last).size(12.5).color(t.accent));
                });
                set(ui, "last", last);
            },
        },
        Demo {
            id: "pagination",
            name: "Pagination",
            category: C,
            description: "Previous / numbered pages / Next with smart ellipses; the active page indicator slides between numbers.",
            covers: &["shadcn/ui: Pagination", "Origin UI: Pagination", "Kibo UI: Pagination"],
            code: r##"ui.add(Pagination::new(&mut page, 20));
ui.add(Pagination::new(&mut page2, 8).labels(false).filled(true));"##,
            show: |ui| {
                let t = Theme::of(ui);
                let mut pages: [usize; 2] = st(ui, "pages", || [5, 2]);
                ui.add_space(70.0);
                ui.vertical_centered(|ui| {
                    ui.add(Pagination::new(&mut pages[0], 20));
                    ui.add_space(6.0);
                    ui.label(RichText::new(format!("Page {} of 20", pages[0])).size(12.5).color(t.muted_foreground));
                    ui.add_space(30.0);
                    ui.add(Pagination::new(&mut pages[1], 8).labels(false).filled(true));
                });
                set(ui, "pages", pages);
            },
        },
        Demo {
            id: "sidebar",
            name: "Sidebar",
            category: C,
            description: "A collapsible app sidebar with brand header, sections, badges, a sliding active indicator and a user footer; collapses on a spring into an icon rail with tooltips.",
            covers: &["shadcn/ui: Sidebar", "Aceternity: Sidebar", "Origin UI: Sidebar", "Animate UI: Sidebar"],
            code: r##"Sidebar::new(&mut selected, &mut collapsed)
    .brand("Acme Inc", "Enterprise")
    .section("Platform", vec![
        SidebarItem::new(LineIcon::Home, "Dashboard"),
        SidebarItem::new(LineIcon::Inbox, "Inbox").badge("12"),
        SidebarItem::new(LineIcon::Calendar, "Calendar"),
    ])
    .section("Projects", vec![SidebarItem::new(LineIcon::Layers, "Design system")])
    .user("Ana Souza", "ana@example.com")
    .show(ui);"##,
            show: |ui| {
                let t = Theme::of(ui);
                let (mut sel, mut col, mut sel2, mut col2): (usize, bool, usize, bool) = st(ui, "sb", || (1, false, 0, true));
                let full = ui.available_width();
                let h = 400.0;
                let (area, _) = ui.allocate_exact_size(vec2(full, h), egui::Sense::hover());
                let frames = [Rect::from_min_size(area.min, vec2(full * 0.58, h)), Rect::from_min_max(pos2(area.min.x + full * 0.58 + 16.0, area.min.y), area.max)];
                for (k, f) in frames.iter().enumerate() {
                    ui.painter().rect_filled(*f, t.radius, t.background);
                    let mut child = ui.new_child(UiBuilder::new().max_rect(f.shrink(1.0)).layout(egui::Layout::left_to_right(egui::Align::Min)));
                    child.set_clip_rect(f.shrink(1.0).intersect(ui.clip_rect()));
                    let (s, c) = if k == 0 { (&mut sel, &mut col) } else { (&mut sel2, &mut col2) };
                    let sb = Sidebar::new(s, c)
                        .brand("Acme Inc", "Enterprise")
                        .section("Platform", vec![
                            SidebarItem::new(LineIcon::Home, "Dashboard"),
                            SidebarItem::new(LineIcon::Inbox, "Inbox").badge("12"),
                            SidebarItem::new(LineIcon::Calendar, "Calendar"),
                            SidebarItem::new(LineIcon::Chart, "Analytics"),
                        ])
                        .section("Projects", vec![SidebarItem::new(LineIcon::Layers, "Design system"), SidebarItem::new(LineIcon::Settings, "Settings").badge("New")])
                        .user("Ana Souza", "ana@example.com")
                        .height(h - 2.0);
                    sb.show(&mut child);
                    // fake content
                    let cr = Rect::from_min_max(pos2(child.cursor().min.x + 24.0, f.min.y + 24.0), f.max - vec2(24.0, 24.0));
                    let sk = mix(t.background, t.foreground, if t.dark { 0.07 } else { 0.06 });
                    let p = ui.painter().with_clip_rect(*f);
                    p.text(cr.min, Align2::LEFT_TOP, if k == 0 { "Inbox" } else { "Collapsed rail" }, FontId::proportional(20.0), t.foreground);
                    for i in 0..5 {
                        let rr = Rect::from_min_size(cr.min + vec2(0.0, 46.0 + i as f32 * 58.0), vec2(cr.width(), 48.0));
                        p.rect_stroke(rr, t.radius * 0.8, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                        p.rect_filled(Rect::from_min_size(rr.min + vec2(14.0, 12.0), vec2(rr.width() * 0.35, 9.0)), 4.0, sk);
                        p.rect_filled(Rect::from_min_size(rr.min + vec2(14.0, 28.0), vec2(rr.width() * 0.6, 8.0)), 4.0, sk);
                    }
                    ui.painter().rect_stroke(*f, t.radius, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                }
                set(ui, "sb", (sel, col, sel2, col2));
            },
        },
        Demo {
            id: "navbar",
            name: "Navbar",
            category: C,
            description: "A top app bar: brand, links with a hover highlight that glides between them and an accent underline on the active page, plus actions.",
            covers: &["shadcn blocks: Navbar", "Aceternity: Navbar Menu", "Origin UI: Navbar", "Motion Primitives: Animated Background"],
            code: r##"let r = Navbar::new(&mut page, &["Home", "Features", "Pricing", "Docs", "Blog"])
    .brand("glowkit")
    .actions(&["Log in", "Get started"])
    .show(ui);"##,
            show: |ui| {
                let t = Theme::of(ui);
                let mut page: usize = st(ui, "page", || 0);
                let canvas = ui.available_rect_before_wrap();
                let (frame, _) = ui.allocate_exact_size(vec2(canvas.width(), 380.0), egui::Sense::hover());
                ui.painter().rect_filled(frame, t.radius, t.background);
                ui.painter().rect_stroke(frame, t.radius, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                let mut child = ui.new_child(UiBuilder::new().max_rect(frame.shrink(1.0)));
                let _ = Navbar::new(&mut page, &["Home", "Features", "Pricing", "Docs", "Blog"]).brand("glowkit").actions(&["Log in", "Get started"]).show(&mut child);
                let pages = ["Home", "Features", "Pricing", "Docs", "Blog"];
                let p = ui.painter();
                let c = frame.center() + vec2(0.0, 20.0);
                p.text(c - vec2(0.0, 30.0), Align2::CENTER_CENTER, pages[page], FontId::proportional(34.0), t.foreground);
                p.text(c + vec2(0.0, 12.0), Align2::CENTER_CENTER, "Hover the links: the highlight follows your pointer.", FontId::proportional(15.0), t.muted_foreground);
                set(ui, "page", page);
            },
        },
        Demo {
            id: "floating-navbar",
            name: "Floating Navbar",
            category: C,
            description: "A floating pill navbar that slides away while you scroll down and comes back as soon as you scroll up.",
            covers: &["Aceternity: Floating Navbar", "Magic UI: Dock (nav)", "Cult UI: Floating Nav"],
            code: r##"let visible = auto_hide_on_scroll(ui, "nav", scroll_offset_y);
FloatingNavbar::new(&mut active, &[
    (LineIcon::Home, "Home"),
    (LineIcon::User, "About"),
    (LineIcon::Mail, "Contact"),
])
.cta("Login")
.visible(visible)
.show(ui, content_rect);"##,
            show: |ui| {
                let t = Theme::of(ui);
                let (canvas, _) = ui.allocate_exact_size(vec2(ui.available_width(), 400.0), egui::Sense::hover());
                ui.painter().rect_filled(canvas, t.radius, t.background);
                let off = auto_scroll_canvas(ui, canvas, false);
                let visible = auto_hide_on_scroll(ui, "nav", off);
                let mut active: usize = st(ui, "active", || 0);
                let _ = FloatingNavbar::new(&mut active, &[(LineIcon::Home, "Home"), (LineIcon::User, "About"), (LineIcon::Mail, "Contact")]).cta("Login").visible(visible).show(ui, canvas);
                ui.painter().rect_stroke(canvas, t.radius, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                set(ui, "active", active);
            },
        },
        Demo {
            id: "resizable-navbar",
            name: "Resizable Navbar",
            category: C,
            description: "A full-width header that shrinks into a narrower floating, rounded bar with border and shadow once the page scrolls.",
            covers: &["Aceternity: Resizable Navbar", "Aceternity: Navbar (sticky)", "Tailwind UI: Sticky header"],
            code: r##"ResizableNavbar::new(&mut active, &["Features", "Pricing", "Contact"])
    .brand("glowkit")
    .cta("Book a call")
    .show(ui, content_rect, scroll_offset_y);"##,
            show: |ui| {
                let t = Theme::of(ui);
                let (canvas, _) = ui.allocate_exact_size(vec2(ui.available_width(), 400.0), egui::Sense::hover());
                ui.painter().rect_filled(canvas, t.radius, t.background);
                let off = auto_scroll_canvas(ui, canvas, true);
                let mut active: usize = st(ui, "active", || 0);
                let _ = ResizableNavbar::new(&mut active, &["Features", "Pricing", "Customers", "Contact"]).brand("glowkit").cta("Book a call").show(ui, canvas.shrink(1.0), off);
                ui.painter().rect_stroke(canvas, t.radius, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                set(ui, "active", active);
            },
        },
        Demo {
            id: "navigation-menu",
            name: "Navigation Menu",
            category: C,
            description: "Website navigation: hovering a trigger opens a panel of rich link cards that morphs size and slides content as you move between triggers.",
            covers: &["shadcn/ui: Navigation Menu", "Radix: Navigation Menu", "Aceternity: Navbar Menu", "Origin UI: Navbar with dropdown"],
            code: r##"NavigationMenu::new()
    .featured_panel("Getting started", ("glowkit", "Beautiful components for egui."), vec![
        NavMenuLink::new("Introduction", "Re-usable components built with egui."),
        NavMenuLink::new("Installation", "How to add glowkit to your app."),
    ])
    .panel("Components", 2, vec![NavMenuLink::new("Dialog", "A window overlaid on the page.").icon(LineIcon::Layers)])
    .link("Docs")
    .show(ui);"##,
            show: |ui| {
                let t = Theme::of(ui);
                if once(ui, "init") {
                    NavigationMenu::set_open(ui, Some(0));
                }
                ui.add_space(4.0);
                let picked = NavigationMenu::new()
                    .featured_panel("Getting started", ("glowkit", "Beautifully designed, animated components for egui. Pure Rust."), vec![
                        NavMenuLink::new("Introduction", "Re-usable components built with egui and glowkit-core."),
                        NavMenuLink::new("Installation", "How to install dependencies and structure your app."),
                        NavMenuLink::new("Typography", "Styles for headings, paragraphs, lists and more."),
                    ])
                    .panel("Components", 2, vec![
                        NavMenuLink::new("Alert Dialog", "A modal dialog that interrupts the user with important content.").icon(LineIcon::Warning),
                        NavMenuLink::new("Hover Card", "Preview content available behind a link.").icon(LineIcon::Layers),
                        NavMenuLink::new("Progress", "Shows the completion progress of a task.").icon(LineIcon::Chart),
                        NavMenuLink::new("Scroll Area", "Visually or semantically separates content.").icon(LineIcon::Menu),
                        NavMenuLink::new("Tabs", "Layered sections of content, one shown at a time.").icon(LineIcon::Folder),
                        NavMenuLink::new("Tooltip", "A popup that shows information on hover.").icon(LineIcon::Info),
                    ])
                    .panel("Resources", 1, vec![
                        NavMenuLink::new("Blog", "News, releases and deep dives.").icon(LineIcon::Book),
                        NavMenuLink::new("Community", "Join the discussion.").icon(LineIcon::Users),
                    ])
                    .link("Docs")
                    .show(ui);
                if let Some(p) = picked {
                    set(ui, "picked", p);
                }
                let picked: String = st(ui, "picked", String::new);
                ui.add_space(250.0);
                ui.label(RichText::new(if picked.is_empty() { "Hover a trigger to open its panel.".to_owned() } else { format!("Clicked: {picked}") }).size(12.5).color(t.muted_foreground));
            },
        },
        Demo {
            id: "menubar",
            name: "Menubar",
            category: C,
            description: "A desktop-style menubar: click a title, then glide across titles to switch menus; ← → ↑ ↓ Enter work too. Supports submenus, checkboxes and radios.",
            covers: &["shadcn/ui: Menubar", "Radix: Menubar", "Animate UI: Menubar"],
            code: r##"let picked = Menubar::new()
    .menu("File", vec![
        MenuEntry::item("New Tab").shortcut("Ctrl+T"),
        MenuEntry::item("New Window").shortcut("Ctrl+N"),
        MenuEntry::separator(),
        MenuEntry::submenu("Share", vec![MenuEntry::item("Email link"), MenuEntry::item("Messages")]),
    ])
    .menu("Edit", vec![MenuEntry::item("Undo").shortcut("Ctrl+Z")])
    .show(ui);"##,
            show: |ui| {
                let t = Theme::of(ui);
                if once(ui, "init") {
                    Menubar::set_open(ui, Some(0));
                }
                let (mut bm, mut urls, mut profile): (bool, bool, u8) = st(ui, "mb", || (true, false, 1));
                ui.add_space(4.0);
                let picked = Menubar::new()
                    .menu("File", vec![
                        MenuEntry::item("New Tab").icon(LineIcon::Plus).shortcut("Ctrl+T"),
                        MenuEntry::item("New Window").icon(LineIcon::Copy).shortcut("Ctrl+N"),
                        MenuEntry::item("New Incognito Window").icon(LineIcon::Lock).disabled(true),
                        MenuEntry::separator(),
                        MenuEntry::submenu("Share", vec![MenuEntry::item("Email link").icon(LineIcon::Mail), MenuEntry::item("Messages").icon(LineIcon::Inbox), MenuEntry::item("Notes").icon(LineIcon::File)]).icon(LineIcon::Share),
                        MenuEntry::separator(),
                        MenuEntry::item("Print…").icon(LineIcon::File).shortcut("Ctrl+P"),
                    ])
                    .menu("Edit", vec![
                        MenuEntry::item("Undo").shortcut("Ctrl+Z"),
                        MenuEntry::item("Redo").shortcut("Shift+Ctrl+Z"),
                        MenuEntry::separator(),
                        MenuEntry::submenu("Find", vec![MenuEntry::item("Search the web"), MenuEntry::item("Find…"), MenuEntry::item("Find Next")]),
                        MenuEntry::separator(),
                        MenuEntry::item("Cut"),
                        MenuEntry::item("Copy"),
                        MenuEntry::item("Paste"),
                    ])
                    .menu("View", vec![
                        MenuEntry::checkbox("Always Show Bookmarks Bar", bm),
                        MenuEntry::checkbox("Always Show Full URLs", urls),
                        MenuEntry::separator(),
                        MenuEntry::item("Reload").shortcut("Ctrl+R"),
                        MenuEntry::item("Toggle Fullscreen"),
                    ])
                    .menu("Profiles", vec![
                        MenuEntry::radio("Andy", profile == 0),
                        MenuEntry::radio("Benoit", profile == 1),
                        MenuEntry::radio("Luis", profile == 2),
                        MenuEntry::separator(),
                        MenuEntry::item("Edit…"),
                        MenuEntry::item("Add Profile…"),
                    ])
                    .show(ui);
                if let Some(p) = &picked {
                    match p.as_str() {
                        "Always Show Bookmarks Bar" => bm = !bm,
                        "Always Show Full URLs" => urls = !urls,
                        "Andy" => profile = 0,
                        "Benoit" => profile = 1,
                        "Luis" => profile = 2,
                        _ => set(ui, "picked", p.clone()),
                    }
                }
                set(ui, "mb", (bm, urls, profile));
                let picked: String = st(ui, "picked", String::new);
                ui.add_space(300.0);
                ui.label(RichText::new(if picked.is_empty() { String::new() } else { format!("Picked: {picked}") }).size(12.5).color(t.muted_foreground));
            },
        },
        Demo {
            id: "stepper",
            name: "Stepper",
            category: C,
            description: "A wizard header: numbered steps joined by a line that fills as you advance, check marks for completed steps and a pulsing ring on the current one.",
            covers: &["Origin UI: Stepper", "Kibo UI: Stepper", "Mantine: Stepper", "MUI: Stepper"],
            code: r##"ui.add(Stepper::new(&mut step, &[
    ("Account", "Create your login"),
    ("Profile", "Tell us about you"),
    ("Billing", "Choose a plan"),
    ("Done", "Start building"),
]));"##,
            show: |ui| {
                let (mut step, mut vstep): (usize, usize) = st(ui, "steps", || (2, 1));
                ui.add_space(14.0);
                let steps = [("Account", "Create your login"), ("Profile", "Tell us about you"), ("Billing", "Choose a plan"), ("Done", "Start building")];
                ui.add(Stepper::new(&mut step, &steps));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - 160.0) / 2.0);
                    if kit::button(ui, "Back", Btn::Outline).clicked() && step > 0 {
                        step -= 1;
                    }
                    if kit::button(ui, if step >= 3 { "Finish" } else { "Next" }, Btn::Primary).clicked() && step < 4 {
                        step += 1;
                    }
                });
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() * 0.3);
                    ui.vertical(|ui| {
                        ui.add(Stepper::new(&mut vstep, &[("Order placed", "Mar 12, 10:24"), ("Shipped", "Mar 13, 08:10"), ("Delivered", "Expected Mar 15")]).vertical(true));
                    });
                });
                set(ui, "steps", (step, vstep));
            },
        },
        Demo {
            id: "command-palette",
            name: "Command Palette",
            category: C,
            description: "A ⌘K / Ctrl+K palette: modal search with grouped fuzzy results, a gliding highlight and full keyboard navigation.",
            covers: &["shadcn/ui: Command", "cmdk: Command Menu", "Origin UI: Command", "Kibo UI: Command"],
            code: r##"CommandPalette::toggle_on_shortcut(ui.ctx(), &mut open);
if let Some(cmd) = CommandPalette::new(&mut open)
    .group("Suggestions", vec![
        CommandItem::new("Calendar").icon(LineIcon::Calendar),
        CommandItem::new("Search Emoji").icon(LineIcon::Star),
    ])
    .group("Settings", vec![
        CommandItem::new("Profile").icon(LineIcon::User).shortcut("Ctrl+P"),
    ])
    .show(ui)
{
    run(cmd);
}"##,
            show: |ui| {
                let t = Theme::of(ui);
                let canvas = kit::demo_canvas(ui, 400.0);
                let mut open: bool = st(ui, "open", || true);
                let br = Rect::from_min_size(canvas.min + vec2(16.0, canvas.height() - 52.0), vec2(250.0, 36.0));
                let resp = ui.interact(br, ui.scope_id().with("trigger"), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                let p = ui.painter();
                p.rect_filled(br, t.radius * 0.7, t.background);
                p.rect_stroke(br, t.radius * 0.7, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                LineIcon::Search.paint(p, Rect::from_center_size(br.left_center() + vec2(18.0, 0.0), vec2(14.0, 14.0)), t.muted_foreground);
                p.text(br.left_center() + vec2(34.0, 0.0), Align2::LEFT_CENTER, "Search commands…", FontId::proportional(13.5), t.muted_foreground);
                kit::kbd(p, br.right_center() - vec2(8.0, 0.0), "Ctrl K", &t);
                if resp.clicked() {
                    open = true;
                }
                CommandPalette::toggle_on_shortcut(ui.ctx(), &mut open);
                if let Some(c) = CommandPalette::new(&mut open)
                    .group("Suggestions", vec![
                        CommandItem::new("Calendar").icon(LineIcon::Calendar),
                        CommandItem::new("Search Emoji").icon(LineIcon::Star).keywords("smile"),
                        CommandItem::new("Calculator").icon(LineIcon::Grid),
                    ])
                    .group("Settings", vec![
                        CommandItem::new("Profile").icon(LineIcon::User).shortcut("Ctrl+P"),
                        CommandItem::new("Billing").icon(LineIcon::CreditCard).shortcut("Ctrl+B"),
                        CommandItem::new("Settings").icon(LineIcon::Settings).shortcut("Ctrl+S"),
                    ])
                    .group("Theme", vec![CommandItem::new("Toggle dark mode").icon(LineIcon::Moon), CommandItem::new("Light mode").icon(LineIcon::Sun)])
                    .within(canvas)
                    .max_height(250.0)
                    .show(ui)
                {
                    crate::overlays::toast(ui.ctx(), crate::overlays::Toast::success(format!("Ran “{c}”")));
                }
                let _ = crate::overlays::Toaster::new().within(canvas).show(ui);
                set(ui, "open", open);
            },
        },
        Demo {
            id: "animated-link",
            name: "Animated Link",
            category: C,
            description: "Text links with animated hover treatments: an underline that draws in and leaves the other way, a highlighter sweep, and an underlined link with a turning arrow.",
            covers: &["Origin UI: Link", "Cult UI: Animated Link", "Hover.dev: Underline links", "Kibo UI: Link"],
            code: r##"ui.add(UnderlineLink::new("Read the docs"));
ui.add(UnderlineLink::new("Highlight me").style(LinkStyle::Highlight));
ui.add(UnderlineLink::new("Visit the blog").style(LinkStyle::Arrow));"##,
            show: |ui| {
                let t = Theme::of(ui);
                ui.add_space(110.0);
                ui.columns(3, |cols| {
                    for (i, (style, text, cap)) in [
                        (LinkStyle::Underline, "Read the documentation", "Underline"),
                        (LinkStyle::Highlight, "Highlight on hover", "Highlight"),
                        (LinkStyle::Arrow, "Visit the blog", "Arrow"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        cols[i].vertical_centered(|ui| {
                            ui.label(RichText::new(cap).size(12.0).color(t.muted_foreground));
                            ui.add_space(8.0);
                            ui.add(UnderlineLink::new(text).style(style).size(17.0));
                        });
                    }
                });
            },
        },
        Demo {
            id: "table-of-contents",
            name: "Table of Contents",
            category: C,
            description: "An “On this page” list with a rail whose accent indicator follows the section you're reading (scroll-spy); click to jump.",
            covers: &["Fumadocs: TOC", "Kibo UI: Table of Contents", "Docusaurus: TOC", "Nextra: TOC"],
            code: r##"if let Some(i) = TableOfContents::new(&[
    (0, "Introduction"), (0, "Installation"), (1, "With cargo"), (0, "Usage"),
], active).show(ui) {
    jump_to_section(i);
}"##,
            show: |ui| {
                let t = Theme::of(ui);
                let sections: [(u8, &str); 7] = [(0, "Introduction"), (0, "Installation"), (1, "With cargo"), (1, "From source"), (0, "Usage"), (1, "Theming"), (0, "FAQ")];
                let (frame, _) = ui.allocate_exact_size(vec2(ui.available_width(), 380.0), egui::Sense::hover());
                let left = Rect::from_min_max(frame.min, pos2(frame.max.x - 260.0, frame.max.y));
                let right = Rect::from_min_max(pos2(frame.max.x - 230.0, frame.min.y + 6.0), frame.max);
                ui.painter().rect_stroke(left, t.radius, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                let mut jump: Option<usize> = st(ui, "jump", || Some(2));
                let tops_key = "tops";
                let tops: Vec<f32> = st(ui, tops_key, Vec::new);
                let mut child = ui.new_child(UiBuilder::new().max_rect(left.shrink(1.0)));
                child.set_clip_rect(left.shrink(1.0).intersect(ui.clip_rect()));
                let mut sa = egui::ScrollArea::vertical().id_salt("toc_scroll").auto_shrink([false, false]);
                if let Some(j) = jump {
                    if let Some(y) = tops.get(j) {
                        sa = sa.vertical_scroll_offset(*y - 8.0);
                        jump = None;
                    }
                }
                let sk = mix(t.background, t.foreground, if t.dark { 0.07 } else { 0.06 });
                let mut new_tops = Vec::new();
                let out = sa.show(&mut child, |ui| {
                    let origin = ui.min_rect().min.y;
                    ui.add_space(16.0);
                    for (i, (lvl, title)) in sections.iter().enumerate() {
                        let y0 = ui.cursor().min.y - origin;
                        new_tops.push(y0);
                        ui.horizontal(|ui| {
                            ui.add_space(24.0);
                            ui.label(RichText::new(*title).size(if *lvl == 0 { 22.0 } else { 17.0 }).color(t.foreground));
                        });
                        ui.add_space(8.0);
                        let lines = 4 + (i % 3) * 2;
                        for k in 0..lines {
                            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), egui::Sense::hover());
                            let w = (r.width() - 48.0) * (0.55 + 0.4 * glowkit_core::rng::hash01((i * 31 + k) as u64));
                            ui.painter().rect_filled(Rect::from_min_size(r.min + vec2(24.0, 4.0), vec2(w, 8.0)), 4.0, sk);
                        }
                        ui.add_space(26.0);
                    }
                    ui.add_space(200.0);
                });
                let off = out.state.clamped_offset().y;
                let mut active = 0;
                for (i, y) in new_tops.iter().enumerate() {
                    if *y <= off + 40.0 {
                        active = i;
                    }
                }
                set(ui, tops_key, new_tops);
                let mut rc = ui.new_child(UiBuilder::new().max_rect(right));
                if let Some(i) = TableOfContents::new(&sections, active).show(&mut rc) {
                    jump = Some(i);
                }
                if jump.is_some() {
                    ui.ctx().request_repaint();
                }
                set(ui, "jump", jump);
            },
        },
        Demo {
            id: "bottom-nav",
            name: "Bottom Navigation",
            category: C,
            description: "A mobile tab bar where the active item grows into a tinted pill revealing its label, gliding between items on a spring.",
            covers: &["Animate UI: Bottom Navigation", "Material 3: Navigation Bar", "Cult UI: Expandable Tabs", "Origin UI: Tab bar"],
            code: r##"ui.add(BottomNav::new(&mut tab, &[
    (LineIcon::Home, "Home"),
    (LineIcon::Search, "Search"),
    (LineIcon::Bell, "Alerts"),
    (LineIcon::User, "Profile"),
]));"##,
            show: |ui| {
                let t = Theme::of(ui);
                let mut tabs: [usize; 2] = st(ui, "bn", || [0, 2]);
                ui.add_space(70.0);
                ui.add(BottomNav::new(&mut tabs[0], &[(LineIcon::Home, "Home"), (LineIcon::Search, "Search"), (LineIcon::Bell, "Alerts"), (LineIcon::User, "Profile")]));
                ui.add_space(40.0);
                ui.add(BottomNav::new(&mut tabs[1], &[(LineIcon::Inbox, "Inbox"), (LineIcon::Calendar, "Calendar"), (LineIcon::Music, "Music"), (LineIcon::Heart, "Saved"), (LineIcon::Settings, "Settings")]).accent(t.chart[2]));
                set(ui, "bn", tabs);
            },
        },
    ]
}
