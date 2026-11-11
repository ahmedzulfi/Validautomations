use egui::{pos2, vec2, Align2, Rect, Ui};
use glowkit_charts::{AreaChart, Series, StatTile};
use glowkit_components::{toast, CommandItem, DescriptionList, LineIcon, Switch, Toast, UiIcon};
use glowkit_core::demo::{Category, Demo};
use glowkit_core::Theme;

use super::kit;
use super::*;

/// Demo state for the shell previews.
#[derive(Clone)]
pub(crate) struct ShellDemo {
    pub route: String,
    pub nav: Vec<NavSection>,
    pub notes: Vec<ShellNotification>,
    pub notify_email: bool,
    pub notify_push: bool,
}

pub(crate) fn demo_nav() -> Vec<NavSection> {
    vec![
        NavSection::new("Platform")
            .icon(LineIcon::Home)
            .item(NavItem::new("dashboard", "Dashboard").icon(LineIcon::Home).shortcut("G D"))
            .item(NavItem::new("analytics", "Analytics").icon(LineIcon::Chart).shortcut("G A"))
            .item(NavItem::new("inbox", "Inbox").icon(LineIcon::Inbox).badge("12"))
            .item(
                NavItem::new("projects", "Projects")
                    .icon(LineIcon::Folder)
                    .expanded(true)
                    .child(NavItem::new("projects/active", "Active").badge("8"))
                    .child(NavItem::new("projects/review", "In review"))
                    .child(NavItem::new("projects/archived", "Archived")),
            )
            .item(NavItem::new("team", "Team").icon(LineIcon::Users).child(NavItem::new("team/members", "Members")).child(NavItem::new("team/roles", "Roles & access"))),
        NavSection::new("Workspace")
            .icon(LineIcon::Layers)
            .item(NavItem::new("calendar", "Calendar").icon(LineIcon::Calendar))
            .item(NavItem::new("documents", "Documents").icon(LineIcon::File).badge("New"))
            .item(
                NavItem::new("settings", "Settings")
                    .icon(LineIcon::Settings)
                    .child(NavItem::new("settings/general", "General"))
                    .child(NavItem::new("settings/billing", "Billing"))
                    .child(NavItem::new("settings/integrations", "Integrations")),
            ),
    ]
}

pub(crate) fn demo_notes() -> Vec<ShellNotification> {
    vec![
        ShellNotification::new(LineIcon::Users, "Mia Chen joined Design", "Say hi and share the onboarding checklist.", "2m ago"),
        ShellNotification::new(AppIcon::Cart, "New order #3210", "Olivia Martin purchased the Pro plan (annual).", "18m ago"),
        ShellNotification::new(LineIcon::Warning, "Usage at 82%", "Your workspace is close to its monthly API quota.", "1h ago"),
        ShellNotification::new(LineIcon::CircleCheck, "Deploy succeeded", "web-app v2.14.0 is live in production.", "Yesterday").read(),
    ]
}

impl ShellDemo {
    fn new(route: &str) -> Self {
        Self { route: route.into(), nav: demo_nav(), notes: demo_notes(), notify_email: true, notify_push: false }
    }
}

const SPARK_A: [f64; 12] = [12.0, 14.0, 13.0, 17.0, 16.0, 19.0, 22.0, 21.0, 25.0, 24.0, 28.0, 31.0];
const SPARK_B: [f64; 12] = [30.0, 28.0, 29.0, 26.0, 27.0, 24.0, 25.0, 22.0, 23.0, 21.0, 20.0, 18.0];
const SPARK_C: [f64; 12] = [8.0, 9.0, 12.0, 11.0, 13.0, 15.0, 14.0, 17.0, 19.0, 18.0, 21.0, 23.0];
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// A generic dashboard page used inside the shell previews.
pub(crate) fn sample_page(ui: &mut Ui, sc: &mut ShellContext<'_>, nav: &[NavSection]) {
    let t = Theme::of(ui);
    let rect = sc.rect;
    let label = nav_find(nav, sc.route).map(|i| i.label.clone()).unwrap_or_else(|| "Overview".into());
    let p = ui.painter().clone();
    let id = ui.scope_id().with(("sample", sc.route.as_str()));
    let loading = kit::loading(ui, id, 0.55);
    let y = kit::heading(&p, rect, &label, "Here's what's happening across your workspace this week.", &t);
    let body = Rect::from_min_max(pos2(rect.min.x, y + 18.0), rect.max);
    let n = if rect.width() > 900.0 { 4 } else if rect.width() > 560.0 { 2 } else { 1 };
    let tiles_h = 104.0;
    let tiles = kit::cols(Rect::from_min_size(body.min, vec2(body.width(), tiles_h)), n, 14.0);
    let data: [(&str, f64, &str, f64, &[f64]); 4] = [
        ("Revenue", 48_210.0, "$", 12.4, &SPARK_A),
        ("Active users", 2_340.0, "", 8.1, &SPARK_C),
        ("Churn", 2.1, "%", -0.6, &SPARK_B),
        ("Open tickets", 37.0, "", -12.0, &SPARK_B),
    ];
    for (i, r) in tiles.iter().enumerate() {
        if loading {
            kit::skel_r(ui, *r, t.radius);
            continue;
        }
        let (title, v, unit, d, sp) = data[i];
        let mut tile = StatTile::new(title, v).delta(d).caption("vs last week").spark(sp).size(r.size());
        if unit == "$" {
            tile = tile.prefix("$");
        } else if unit == "%" {
            tile = tile.suffix("%").decimals(1).invert_delta(true);
        }
        if title == "Open tickets" {
            tile = tile.invert_delta(true);
        }
        ui.put(*r, tile);
    }
    let lower = Rect::from_min_max(pos2(body.min.x, body.min.y + tiles_h + 14.0), body.max);
    if lower.height() < 120.0 {
        return;
    }
    let parts = if lower.width() > 820.0 { kit::split_x(lower, &[0.0, 320.0], 14.0) } else { vec![lower] };
    if loading {
        for r in &parts {
            kit::skel_r(ui, *r, t.radius);
        }
        return;
    }
    kit::titled_card(ui, parts[0], "Revenue", "Monthly recurring revenue, last 12 months", |ui, b| {
        let s = [Series::new("This year", [18.0, 21.0, 19.5, 24.0, 26.0, 25.0, 29.0, 31.0, 30.0, 34.0, 37.0, 41.0].map(|v| v * 1000.0)), Series::new("Last year", [14.0, 15.0, 17.0, 16.0, 19.0, 21.0, 20.0, 22.0, 24.0, 23.0, 26.0, 28.0].map(|v| v * 1000.0))];
        ui.add(AreaChart::new(&s).labels(&MONTHS).height(b.height()).id_salt("shell_rev"));
    });
    if parts.len() > 1 {
        kit::titled_card(ui, parts[1], "Recent activity", "", |ui, b| {
            let p = ui.painter().clone();
            let rows = [
                ("Olivia Martin", "upgraded to Pro", "2m"),
                ("Jackson Lee", "invited 3 teammates", "14m"),
                ("Isabella Nguyen", "closed ticket #482", "1h"),
                ("William Kim", "published Q3 report", "3h"),
                ("Sofia Davis", "connected Stripe", "5h"),
                ("Liam Johnson", "commented on Roadmap", "1d"),
            ];
            let rh = 44.0;
            for (i, (who, what, when)) in rows.iter().enumerate() {
                let r = Rect::from_min_size(pos2(b.min.x, b.min.y + i as f32 * rh), vec2(b.width(), rh - 4.0));
                if r.max.y > b.max.y {
                    break;
                }
                let (resp, _) = kit::row(ui, r.expand2(vec2(6.0, 0.0)), ui.id().with(("act", i)), t.radius * 0.6);
                kit::avatar(ui, Rect::from_center_size(pos2(r.min.x + 16.0, r.center().y), vec2(30.0, 30.0)), who);
                kit::text_at(&p, pos2(r.min.x + 40.0, r.center().y - 8.0), Align2::LEFT_CENTER, who, 13.0, t.foreground, b.width() - 90.0);
                kit::text_at(&p, pos2(r.min.x + 40.0, r.center().y + 9.0), Align2::LEFT_CENTER, what, 12.0, t.muted_foreground, b.width() - 90.0);
                kit::text_at(&p, pos2(r.max.x, r.center().y - 8.0), Align2::RIGHT_CENTER, when, 11.5, t.muted_foreground, 40.0);
                if resp.clicked() {
                    toast(ui.ctx(), Toast::new(format!("{who} {what}")).description("Opened activity details."));
                }
            }
        });
    }
}

/// Shared demo runner: state in egui memory, a configured shell around the sample page.
fn run(ui: &mut Ui, init_route: &str, layout: ShellLayout, upgrade: bool, collapsed: bool) {
    let id = ui.scope_id().with("shell_demo");
    let mut st: ShellDemo = glowkit_core::state(ui, id, || ShellDemo::new(init_route));
    let h = 720.0f32.min(ui.available_height().max(560.0));
    let nav_copy = st.nav.clone();
    let shell = AppShell::new(&mut st.route, &mut st.nav)
        .workspaces(vec![Workspace::new("Acme Inc", "Enterprise"), Workspace::new("Acme Corp.", "Startup"), Workspace::new("Evil Corp.", "Free")])
        .user(ShellUser::new("Ada Lovelace", "ada@acme.dev"))
        .notifications(&mut st.notes)
        .command(CommandItem::new("Create project").icon(LineIcon::Plus).shortcut("Ctrl+N"))
        .command(CommandItem::new("Invite teammate").icon(LineIcon::Users))
        .scoped_theme(true)
        .size(vec2(ui.available_width(), h))
        .layout(layout)
        .upgrade_card(upgrade)
        .start_collapsed(collapsed);
    let (email, push) = (&mut st.notify_email, &mut st.notify_push);
    let shell = if layout == ShellLayout::Inspector { shell.inspector("Details", |ui, route| inspector_panel(ui, route, email, push)) } else { shell };
    let r = shell.show(ui, |ui, sc| sample_page(ui, sc, &nav_copy));
    feedback(ui, &r);
    glowkit_core::set_state(ui, id, st);
}

/// Toast feedback for menu / palette picks in the demos.
pub(crate) fn feedback<R>(ui: &Ui, r: &ShellResponse<R>) {
    if let Some(c) = &r.command {
        toast(ui.ctx(), Toast::success(c.clone()).description("Command palette action ran."));
    }
    if let Some(m) = &r.menu {
        let tt = match m.as_str() {
            "Log out" => Toast::info("Signed out").description("See you soon, Ada."),
            "Upgrade plan" => Toast::success("Pro trial started").description("14 days of Pro, on the house."),
            other => Toast::new(other.to_owned()).description("Opened from the menu."),
        };
        toast(ui.ctx(), tt);
    }
}

fn inspector_panel(ui: &mut Ui, route: &mut String, email: &mut bool, push: &mut bool) {
    let t = Theme::of(ui);
    ui.spacing_mut().item_spacing.y = 10.0;
    let r = kit::alloc_row(ui, 52.0);
    let p = ui.painter().clone();
    kit::logo(&p, Rect::from_min_size(r.min, vec2(44.0, 44.0)), &t, "W");
    kit::text_at(&p, pos2(r.min.x + 56.0, r.min.y + 13.0), Align2::LEFT_CENTER, "Website redesign", 14.5, t.foreground, r.width() - 60.0);
    kit::text_at(&p, pos2(r.min.x + 56.0, r.min.y + 32.0), Align2::LEFT_CENTER, &format!("Route · {route}"), 12.0, t.muted_foreground, r.width() - 60.0);
    ui.add(DescriptionList::new(&[("Status", "In progress"), ("Owner", "Ada Lovelace"), ("Due", "Nov 28, 2026"), ("Budget", "$24,000"), ("Priority", "High")]).key_width(80.0).width(ui.available_width()));
    ui.add_space(4.0);
    let r = kit::alloc_row(ui, 18.0);
    kit::text_at(ui.painter(), r.left_center(), Align2::LEFT_CENTER, "Progress", 12.5, t.muted_foreground, 100.0);
    kit::text_at(ui.painter(), r.right_center(), Align2::RIGHT_CENTER, "68%", 12.5, t.foreground, 60.0);
    ui.add(glowkit_components::Progress::new(0.68).height(6.0).width(ui.available_width()));
    ui.add_space(6.0);
    ui.add(Switch::new(email).label("Email updates").description("Daily digest of changes"));
    ui.add(Switch::new(push).label("Push notifications"));
    ui.add_space(6.0);
    ui.add(glowkit_components::AvatarGroup::new(&["Ada Lovelace", "Grace Hopper", "Alan Turing", "Mia Chen"]).max(4).size(30.0));
    ui.add_space(6.0);
    if ui.add(glowkit_components::Button::new("Open project").icon(UiIcon::ArrowRight).width(ui.available_width())).clicked() {
        *route = "projects/active".into();
    }
}

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "shell-sidebar",
            name: "App Shell: Sidebar",
            category: Category::Dashboards,
            description: "Collapsible sidebar (Ctrl+B springs it to an icon rail) with workspace switcher, nested groups, badges, upgrade and user cards; topbar with breadcrumb, Ctrl+K search, notifications and theme toggle.",
            covers: &["shadcn/ui: Sidebar (sidebar-01..05)", "shadcn/ui: Dashboard 01", "Tailwind UI: Sidebar layout", "Aceternity: Sidebar"],
            code: r##"AppShell::new(&mut route, &mut nav)
    .layout(ShellLayout::Sidebar)
    .workspaces(vec![Workspace::new("Acme Inc", "Enterprise")])
    .user(ShellUser::new("Ada Lovelace", "ada@acme.dev"))
    .notifications(&mut notifications)
    .upgrade_card(true)
    .show(ui, |ui, shell| match shell.route.as_str() {
        "dashboard" => dashboard(ui),
        _ => {}
    });"##,
            show: |ui| run(ui, "dashboard", ShellLayout::Sidebar, true, false),
        },
        Demo {
            id: "shell-inset",
            name: "App Shell: Inset",
            category: Category::Dashboards,
            description: "The content floats as a rounded card inset from a sidebar-tinted frame, like a native desktop app.",
            covers: &["shadcn/ui: Sidebar inset (sidebar-07)", "shadcn/ui: Dashboard 01"],
            code: r##"AppShell::new(&mut route, &mut nav)
    .layout(ShellLayout::Inset)
    .user(ShellUser::new("Ada Lovelace", "ada@acme.dev"))
    .show(ui, |ui, shell| page(ui, shell));"##,
            show: |ui| run(ui, "analytics", ShellLayout::Inset, false, false),
        },
        Demo {
            id: "shell-floating",
            name: "App Shell: Floating Sidebar",
            category: Category::Dashboards,
            description: "The sidebar is a floating rounded card with a soft shadow; content sits beside it.",
            covers: &["shadcn/ui: Sidebar floating (sidebar-08)"],
            code: r##"AppShell::new(&mut route, &mut nav)
    .layout(ShellLayout::Floating)
    .show(ui, |ui, shell| page(ui, shell));"##,
            show: |ui| run(ui, "projects/active", ShellLayout::Floating, true, false),
        },
        Demo {
            id: "shell-rail",
            name: "App Shell: Icon Rail",
            category: Category::Dashboards,
            description: "The same sidebar starting collapsed: icons with tooltips, badge dots, and a spring back to full width on Ctrl+B.",
            covers: &["shadcn/ui: Sidebar collapsible icon (sidebar-07)", "Material: Navigation rail"],
            code: r##"AppShell::new(&mut route, &mut nav)
    .start_collapsed(true)
    .show(ui, |ui, shell| page(ui, shell));"##,
            show: |ui| run(ui, "inbox", ShellLayout::Sidebar, false, true),
        },
        Demo {
            id: "shell-dual",
            name: "App Shell: Dual Sidebar",
            category: Category::Dashboards,
            description: "An icon rail of sections plus a secondary panel listing the active section's pages; Ctrl+B hides the panel.",
            covers: &["shadcn/ui: Sidebar with nested sidebars (sidebar-09)", "Linear: Dual navigation", "Discord: Server rail"],
            code: r##"let mut nav = vec![
    NavSection::new("Platform").icon(LineIcon::Home).item(/* ... */),
    NavSection::new("Workspace").icon(LineIcon::Layers).item(/* ... */),
];
AppShell::new(&mut route, &mut nav)
    .layout(ShellLayout::Dual)
    .show(ui, |ui, shell| page(ui, shell));"##,
            show: |ui| run(ui, "dashboard", ShellLayout::Dual, false, false),
        },
        Demo {
            id: "shell-topnav",
            name: "App Shell: Top Navigation",
            category: Category::Dashboards,
            description: "No sidebar: brand, top-level tabs with a sliding underline (parents open a dropdown), search, bell, theme and avatar in one bar.",
            covers: &["Tailwind UI: Stacked layout", "shadcn/ui: Dashboard example (top nav)"],
            code: r##"AppShell::new(&mut route, &mut nav)
    .layout(ShellLayout::TopNav)
    .show(ui, |ui, shell| page(ui, shell));"##,
            show: |ui| run(ui, "analytics", ShellLayout::TopNav, false, false),
        },
        Demo {
            id: "shell-inspector",
            name: "App Shell: Inspector Panel",
            category: Category::Dashboards,
            description: "Sidebar + content + a right inspector panel that springs open and closed (Ctrl+I or the layers button).",
            covers: &["shadcn/ui: Sidebar right (sidebar-15)", "Figma: Properties panel"],
            code: r##"AppShell::new(&mut route, &mut nav)
    .layout(ShellLayout::Inspector)
    .inspector("Details", |ui, route| {
        ui.label(format!("Selected: {route}"));
    })
    .show(ui, |ui, shell| page(ui, shell));"##,
            show: |ui| run(ui, "projects/active", ShellLayout::Inspector, false, false),
        },
        Demo {
            id: "shell-bottom-nav",
            name: "App Shell: Mobile Bottom Nav",
            category: Category::Dashboards,
            description: "Phone layout: app bar with avatar, title, bell and theme toggle; a bottom tab bar with icons, badges and a springy active pill.",
            covers: &["Material: Bottom navigation", "iOS: Tab bar", "Tailwind UI: Mobile app shell"],
            code: r##"AppShell::new(&mut route, &mut nav)
    .layout(ShellLayout::BottomNav)
    .size(egui::vec2(390.0, 720.0))
    .show(ui, |ui, shell| page(ui, shell));"##,
            show: |ui| {
                let h = 700.0f32.min(ui.available_height().max(560.0));
                let w = ui.available_width();
                let (rect, _) = ui.allocate_exact_size(vec2(w, h), egui::Sense::hover());
                let phone_w = 360.0;
                let gap = 48.0;
                let x0 = rect.center().x - phone_w - gap / 2.0;
                for (k, route) in ["dashboard", "inbox"].iter().enumerate() {
                    let pr = Rect::from_min_size(pos2(x0 + k as f32 * (phone_w + gap), rect.min.y), vec2(phone_w, h));
                    let t = Theme::of(ui);
                    let frame = pr.expand(6.0);
                    ui.painter().rect_filled(frame, 34.0, if t.dark { glowkit_core::hex("#1C1C20") } else { glowkit_core::hex("#D4D4D8") });
                    kit::child(ui, pr, |ui| {
                        let id = ui.scope_id().with(("phone", k));
                        let mut st: ShellDemo = glowkit_core::state(ui, id, || ShellDemo::new(route));
                        let nav_copy = st.nav.clone();
                        let r = AppShell::new(&mut st.route, &mut st.nav)
                            .layout(ShellLayout::BottomNav)
                            .user(ShellUser::new("Ada Lovelace", "ada@acme.dev"))
                            .notifications(&mut st.notes)
                            .search(false)
                            .scoped_theme(true)
                            .padding(16.0)
                            .id_salt(("phone_shell", k))
                            .size(pr.size())
                            .show(ui, |ui, sc| mobile_page(ui, sc, &nav_copy));
                        feedback(ui, &r);
                        glowkit_core::set_state(ui, id, st);
                    });
                }
            },
        },
    ]
}

/// A compact page for the phone previews.
fn mobile_page(ui: &mut Ui, sc: &mut ShellContext<'_>, nav: &[NavSection]) {
    let t = Theme::of(ui);
    let rect = sc.rect;
    let p = ui.painter().clone();
    let id = ui.scope_id().with(("m", sc.route.as_str()));
    let loading = kit::loading(ui, id, 0.5);
    let label = nav_find(nav, sc.route).map(|i| i.label.clone()).unwrap_or_default();
    let card = Rect::from_min_size(rect.min, vec2(rect.width(), 104.0));
    if loading {
        kit::skel_r(ui, card, t.radius);
    } else {
        ui.put(card, StatTile::new(format!("{label} · this week"), 12_480.0).prefix("$").delta(9.2).spark(&SPARK_A).size(card.size()));
    }
    let mut y = card.max.y + 18.0;
    kit::text_at(&p, pos2(rect.min.x, y), Align2::LEFT_TOP, "Recent", 15.0, t.foreground, 200.0);
    kit::text_at(&p, pos2(rect.max.x, y + 2.0), Align2::RIGHT_TOP, "See all", 12.5, t.accent, 80.0);
    y += 30.0;
    let items = [
        ("Olivia Martin", "Pro plan · annual", "+$1,999.00"),
        ("Jackson Lee", "Team seats × 3", "+$39.00"),
        ("Isabella Nguyen", "Refund", "-$299.00"),
        ("William Kim", "Starter plan", "+$99.00"),
        ("Sofia Davis", "Add-on: storage", "+$39.00"),
        ("Liam Johnson", "Pro plan · monthly", "+$199.00"),
    ];
    for (i, (who, what, amt)) in items.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.min.x, y), vec2(rect.width(), 52.0));
        if r.max.y > rect.max.y {
            break;
        }
        if loading {
            kit::skel_r(ui, Rect::from_center_size(pos2(r.min.x + 18.0, r.center().y), vec2(36.0, 36.0)), 18.0);
            kit::skel(ui, Rect::from_min_size(pos2(r.min.x + 48.0, r.center().y - 10.0), vec2(r.width() * 0.5, 9.0)));
            kit::skel(ui, Rect::from_min_size(pos2(r.min.x + 48.0, r.center().y + 4.0), vec2(r.width() * 0.3, 8.0)));
        } else {
            let (resp, _) = kit::row(ui, r.expand2(vec2(6.0, 0.0)), id.with(i), t.radius * 0.6);
            kit::avatar(ui, Rect::from_center_size(pos2(r.min.x + 18.0, r.center().y), vec2(36.0, 36.0)), who);
            kit::text_at(&p, pos2(r.min.x + 48.0, r.center().y - 8.0), Align2::LEFT_CENTER, who, 13.5, t.foreground, r.width() - 140.0);
            kit::text_at(&p, pos2(r.min.x + 48.0, r.center().y + 9.0), Align2::LEFT_CENTER, what, 12.0, t.muted_foreground, r.width() - 140.0);
            let neg = amt.starts_with('-');
            kit::text_at(&p, pos2(r.max.x, r.center().y), Align2::RIGHT_CENTER, amt, 13.5, if neg { t.destructive } else { t.foreground }, 90.0);
            if resp.clicked() {
                toast(ui.ctx(), Toast::new(format!("{who}: {amt}")).description(*what));
            }
        }
        y += 54.0;
    }
}
