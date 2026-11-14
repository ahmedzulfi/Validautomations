use super::kit::{self, Btn};
use super::*;
use egui::{vec2, Align, Layout, RichText, Ui};
use glowkit_core::demo::{Category::Overlays as C, Demo};
use glowkit_core::Theme;

/// A `bool` stored in egui memory under the demo's id (so it survives between frames).
fn flag(ui: &Ui, key: &str, init: bool) -> bool {
    glowkit_core::state(ui, ui.scope_id().with(key), || init)
}

fn set_flag(ui: &Ui, key: &str, v: bool) {
    glowkit_core::set_state(ui, ui.scope_id().with(key), v);
}

/// Row of trigger buttons laid over the top-left of the demo canvas.
fn trigger_row(ui: &mut Ui, canvas: egui::Rect, add: impl FnOnce(&mut Ui)) {
    let r =
        egui::Rect::from_min_size(canvas.min + vec2(16.0, canvas.height() - 54.0), vec2(canvas.width() - 32.0, 40.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r).layout(Layout::left_to_right(Align::Center)));
    add(&mut child);
}

fn field(ui: &mut Ui, label: &str, value: &mut String) {
    let t = Theme::of(ui);
    ui.horizontal(|ui| {
        ui.add_sized(vec2(84.0, 32.0), egui::Label::new(RichText::new(label).size(13.5).color(t.foreground)));
        ui.add_sized(vec2(ui.available_width(), 32.0), egui::TextEdit::singleline(value).margin(vec2(10.0, 8.0)));
    });
}

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "dialog",
            name: "Dialog",
            category: C,
            description: "A modal card that springs in from 95% scale over a fading backdrop. Closes on Escape, the X or a backdrop click.",
            covers: &["shadcn/ui: Dialog", "Animate UI: Dialog", "Origin UI: Dialog", "Radix: Dialog", "Motion Primitives: Dialog", "Aceternity UI: Animated Modal"],
            code: r##"let r = Dialog::new(&mut open, "Edit profile")
    .description("Make changes to your profile here. Click save when you're done.")
    .primary("Save changes")
    .show(ui, |ui| {
        ui.text_edit_singleline(&mut name);
    });
if r.primary_clicked { save(); }"##,
            show: |ui| {
                let canvas = kit::demo_canvas(ui, 400.0);
                let mut open = flag(ui, "open", true);
                trigger_row(ui, canvas, |ui| {
                    if kit::button(ui, "Edit profile", Btn::Outline).clicked() {
                        open = true;
                    }
                });
                let mut name: String = glowkit_core::state(ui, ui.scope_id().with("name"), || "Pedro Duarte".to_owned());
                let mut user: String = glowkit_core::state(ui, ui.scope_id().with("user"), || "@peduarte".to_owned());
                Dialog::new(&mut open, "Edit profile")
                    .description("Make changes to your profile here. Click save when you're done.")
                    .primary("Save changes")
                    .secondary("Cancel")
                    .within(canvas)
                    .show(ui, |ui| {
                        field(ui, "Name", &mut name);
                        field(ui, "Username", &mut user);
                    });
                glowkit_core::set_state(ui, ui.scope_id().with("name"), name);
                glowkit_core::set_state(ui, ui.scope_id().with("user"), user);
                set_flag(ui, "open", open);
            },
        },
        Demo {
            id: "alert-dialog",
            name: "Alert Dialog",
            category: C,
            description: "A blocking confirmation with Cancel / Continue actions and an optional destructive style.",
            covers: &["shadcn/ui: Alert Dialog", "Animate UI: Alert Dialog", "Origin UI: Alert Dialog"],
            code: r##"let r = AlertDialog::new(&mut open, "Delete this project?")
    .description("This permanently deletes the project and all of its data.")
    .confirm("Delete project")
    .destructive(true)
    .show(ui);
if r.confirmed { delete(); }"##,
            show: |ui| {
                let canvas = kit::demo_canvas(ui, 400.0);
                let mut open = flag(ui, "open", true);
                let mut plain = flag(ui, "plain", false);
                trigger_row(ui, canvas, |ui| {
                    if kit::button(ui, "Delete project", Btn::Destructive).clicked() {
                        open = true;
                    }
                    if kit::button(ui, "Show confirmation", Btn::Outline).clicked() {
                        plain = true;
                    }
                });
                let _ = AlertDialog::new(&mut open, "Delete this project?")
                    .description("This action cannot be undone. This will permanently delete the project and remove its data from our servers.")
                    .confirm("Delete project")
                    .destructive(true)
                    .within(canvas)
                    .show(ui);
                let _ = AlertDialog::new(&mut plain, "Are you absolutely sure?")
                    .description("You are about to publish this release to all users.")
                    .confirm("Publish")
                    .within(canvas)
                    .show(ui);
                set_flag(ui, "open", open);
                set_flag(ui, "plain", plain);
            },
        },
        Demo {
            id: "sheet",
            name: "Sheet / Drawer",
            category: C,
            description: "A panel that slides in on a spring from any edge over a dimmed backdrop; bottom sheets get a grab handle.",
            covers: &["shadcn/ui: Sheet", "shadcn/ui: Drawer", "Vaul: Drawer", "Animate UI: Sheet", "Origin UI: Drawer"],
            code: r##"Sheet::new(&mut open, "Edit profile")
    .side(SheetSide::Right)
    .description("Make changes to your profile here.")
    .primary("Save changes")
    .show(ui, |ui| { ui.label("…"); });"##,
            show: |ui| {
                let canvas = kit::demo_canvas(ui, 400.0);
                let id = ui.scope_id().with("side");
                let mut side: Option<u8> = glowkit_core::state(ui, id, || Some(0));
                trigger_row(ui, canvas, |ui| {
                    for (i, l) in ["Right", "Left", "Top", "Bottom"].iter().enumerate() {
                        if kit::button(ui, l, Btn::Outline).clicked() {
                            side = Some(i as u8);
                        }
                    }
                });
                for (i, s) in [SheetSide::Right, SheetSide::Left, SheetSide::Top, SheetSide::Bottom].into_iter().enumerate() {
                    let mut open = side == Some(i as u8);
                    let horizontal = matches!(s, SheetSide::Left | SheetSide::Right);
                    let mut sh = Sheet::new(&mut open, if horizontal { "Edit profile" } else { "Move goal" })
                        .side(s)
                        .description(if horizontal { "Make changes to your profile here. Click save when you're done." } else { "Set your daily activity goal." })
                        .within(canvas)
                        .size(if horizontal { 340.0 } else { 220.0 });
                    if horizontal {
                        sh = sh.primary("Save changes").secondary("Cancel");
                    }
                    sh.show(ui, |ui| {
                        let t = Theme::of(ui);
                        if horizontal {
                            for (k, v) in [("Name", "Pedro Duarte"), ("Username", "@peduarte"), ("Email", "pedro@example.com")] {
                                ui.label(RichText::new(k).size(13.0).color(t.muted_foreground));
                                ui.label(RichText::new(v).size(14.5));
                                ui.add_space(6.0);
                            }
                        } else {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("350").size(40.0));
                                ui.label(RichText::new("CALORIES / DAY").size(12.0).color(t.muted_foreground));
                            });
                        }
                    });
                    if side == Some(i as u8) && !open {
                        side = None;
                    }
                }
                glowkit_core::set_state(ui, id, side);
            },
        },
        Demo {
            id: "toast",
            name: "Toast (Sonner)",
            category: C,
            description: "Stacked toasts that fan out on hover, auto-dismiss with a progress line, swipe away, and come in success / error / info / warning / loading flavours.",
            covers: &["Sonner: Toaster", "shadcn/ui: Sonner", "shadcn/ui: Toast", "Animate UI: Toast", "Origin UI: Toast"],
            code: r##"if ui.button("Save").clicked() {
    toast(ui.ctx(), Toast::success("Event created")
        .description("Sunday, December 03 at 9:00 AM")
        .action("Undo"));
}
let id = toast(ui.ctx(), Toast::loading("Uploading…"));
// later: Toaster::update(ui.ctx(), id, Toast::success("Uploaded"));

// once per frame, e.g. at the end of your app:
let action_clicks = Toaster::new().position(ToastPosition::BottomRight).show(ui);"##,
            show: |ui| {
                let canvas = kit::demo_canvas(ui, 420.0);
                let ctx = ui.ctx().clone();
                let now = ctx.input(|i| i.time);
                let sid = ui.scope_id().with("spawn");
                let (mut next_at, mut n): (f64, u32) = glowkit_core::state(ui, sid, || (now, 0));
                let samples = |k: u32| match k % 6 {
                    0 => Toast::success("Event has been created").description("Sunday, December 03, 2023 at 9:00 AM").action("Undo").duration(Some(7.0)),
                    1 => Toast::info("Update available").description("Version 2.4 is ready to install.").duration(Some(7.0)),
                    2 => Toast::new("Your message has been sent.").duration(Some(7.0)),
                    3 => Toast::warning("Storage almost full").description("You have used 92% of your quota.").duration(Some(7.0)),
                    4 => Toast::error("Payment failed").description("Your card was declined.").action("Retry").duration(Some(7.0)),
                    _ => Toast::success("Profile saved").duration(Some(7.0)),
                };
                if now >= next_at {
                    toast(&ctx, samples(n));
                    n += 1;
                    next_at = now + if n < 3 { 0.45 } else { 3.2 };
                }
                ctx.request_repaint_after(std::time::Duration::from_secs_f64((next_at - now).max(0.05)));
                glowkit_core::set_state(ui, sid, (next_at, n));
                let lid = ui.scope_id().with("loading");
                let mut loading: Option<(ToastId, f64)> = glowkit_core::state(ui, lid, || None);
                trigger_row(ui, canvas, |ui| {
                    if kit::button(ui, "Default", Btn::Outline).clicked() {
                        toast(&ctx, Toast::new("Event has been created"));
                    }
                    if kit::button(ui, "Success", Btn::Outline).clicked() {
                        toast(&ctx, Toast::success("Changes saved").description("Your profile is up to date."));
                    }
                    if kit::button(ui, "Error", Btn::Outline).clicked() {
                        toast(&ctx, Toast::error("Something went wrong").description("There was a problem with your request."));
                    }
                    if kit::button(ui, "Warning", Btn::Outline).clicked() {
                        toast(&ctx, Toast::warning("Heads up").description("Your trial ends in 3 days."));
                    }
                    if kit::button(ui, "Promise", Btn::Outline).clicked() {
                        loading = Some((toast(&ctx, Toast::loading("Uploading 3 files…")), now));
                    }
                    if kit::button(ui, "With action", Btn::Outline).clicked() {
                        toast(&ctx, Toast::new("Message archived").action("Undo"));
                    }
                });
                if let Some((id, at)) = loading {
                    if now - at > 2.0 {
                        Toaster::update(&ctx, id, Toast::success("Upload complete").description("3 files uploaded."));
                        loading = None;
                    } else {
                        ctx.request_repaint();
                    }
                }
                glowkit_core::set_state(ui, lid, loading);
                let _ = Toaster::new().within(canvas).show(ui);
            },
        },
        Demo {
            id: "tooltip",
            name: "Tooltip",
            category: C,
            description: "A small inverted label that fades and scales in next to a widget after a hover delay, with an arrow; flips when out of room.",
            covers: &["shadcn/ui: Tooltip", "Animate UI: Tooltip", "Origin UI: Tooltip", "Radix: Tooltip"],
            code: r##"let r = ui.button("Hover");
tooltip(&r, "Add to library");
Tooltip::new("Opens on the right").placement(Placement::Right).show(&r);"##,
            show: |ui| {
                ui.add_space(130.0);
                ui.horizontal(|ui| {
                    let total = 4.0 * 104.0 + 3.0 * 12.0;
                    ui.add_space((ui.available_width() - total) / 2.0);
                    ui.spacing_mut().item_spacing.x = 12.0;
                    for (label, pl, text, pinned) in [
                        ("Top", Placement::Top, "Add to library", true),
                        ("Bottom", Placement::Bottom, "Opens below", false),
                        ("Left", Placement::Left, "On the left", false),
                        ("Right", Placement::Right, "Keyboard: ⌘K", true),
                    ] {
                        let r = ui.add_sized(vec2(104.0, 34.0), egui::Button::new(label));
                        Tooltip::new(text).placement(pl).always_open(pinned).show(&r);
                    }
                });
                ui.add_space(26.0);
                let t = Theme::of(ui);
                ui.vertical_centered(|ui| ui.label(RichText::new("Hover any button").color(t.muted_foreground).size(12.5)));
            },
        },
        Demo {
            id: "popover",
            name: "Popover",
            category: C,
            description: "A card anchored to its trigger that scales in from the trigger side with an arrow; closes on outside click or Escape.",
            covers: &["shadcn/ui: Popover", "Animate UI: Popover", "Origin UI: Popover", "Radix: Popover", "Motion Primitives: Morphing Popover"],
            code: r##"let trigger = ui.button("Open popover");
Popover::new().width(300.0).show(&trigger, |ui| {
    ui.label("Dimensions");
    ui.text_edit_singleline(&mut width);
});"##,
            show: |ui| {
                ui.add_space(40.0);
                let t = Theme::of(ui);
                let r = ui.vertical_centered(|ui| ui.add_sized(vec2(140.0, 34.0), egui::Button::new("Open popover"))).inner;
                if !flag(ui, "init", false) {
                    Popover::set_open(ui.ctx(), r.id, true);
                    set_flag(ui, "init", true);
                }
                let wid = ui.scope_id().with("dims");
                let mut dims: [String; 3] = glowkit_core::state(ui, wid, || ["100%".to_owned(), "300px".to_owned(), "25px".to_owned()]);
                Popover::new().width(320.0).show(&r, |ui| {
                    ui.label(RichText::new("Dimensions").size(15.0).color(t.foreground));
                    ui.label(RichText::new("Set the dimensions for the layer.").size(13.0).color(t.muted_foreground));
                    ui.add_space(6.0);
                    for (i, k) in ["Width", "Max. height", "Radius"].iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.add_sized(vec2(90.0, 30.0), egui::Label::new(RichText::new(*k).size(13.0)));
                            ui.add_sized(vec2(ui.available_width(), 30.0), egui::TextEdit::singleline(&mut dims[i]).margin(vec2(8.0, 6.0)));
                        });
                    }
                });
                glowkit_core::set_state(ui, wid, dims);
            },
        },
        Demo {
            id: "hover-card",
            name: "Hover Card",
            category: C,
            description: "A rich preview card that appears after hovering a link for a moment and stays while you move onto it.",
            covers: &["shadcn/ui: Hover Card", "Origin UI: Hover Card", "Animate UI: Hover Card", "Radix: Hover Card"],
            code: r##"let link = ui.link("@glowkit");
HoverCard::new().show(&link, |ui| {
    ui.label("glowkit");
    ui.label("Animated components for egui.");
});"##,
            show: |ui| {
                ui.add_space(30.0);
                let t = Theme::of(ui);
                let card = |ui: &mut Ui, name: &str, handle: &str, bio: &str, hue: usize| {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(44.0, 44.0), egui::Sense::hover());
                        glowkit_core::paint::gradient_circle(ui.painter(), r.center(), 22.0, &[t.chart[hue], t.chart[(hue + 2) % 6]]);
                        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, &name[..1], egui::FontId::proportional(18.0), egui::Color32::WHITE);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.label(RichText::new(name).size(15.0).color(t.foreground));
                            ui.label(RichText::new(handle).size(12.5).color(t.muted_foreground));
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(RichText::new(bio).size(13.5).color(t.foreground));
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), egui::Sense::hover());
                        LineIcon::Calendar.paint(ui.painter(), r, t.muted_foreground);
                        ui.label(RichText::new("Joined December 2021").size(12.0).color(t.muted_foreground));
                    });
                };
                ui.columns(2, |cols| {
                    cols[0].vertical_centered(|ui| {
                        let l = ui.link(RichText::new("@glowkit").size(15.0));
                        HoverCard::new().always_open(true).show(&l, |ui| card(ui, "glowkit", "@glowkit", "Beautiful animated components for egui, in pure Rust.", 0));
                    });
                    cols[1].vertical_centered(|ui| {
                        let l = ui.link(RichText::new("@egui (hover me)").size(15.0));
                        HoverCard::new().show(&l, |ui| card(ui, "egui", "@emilk/egui", "An easy-to-use immediate mode GUI in Rust.", 1));
                    });
                });
            },
        },
        Demo {
            id: "dropdown-menu",
            name: "Dropdown Menu",
            category: C,
            description: "Menus with icons, shortcuts, labels, separators, checkbox / radio items and submenus; a highlight glides between rows and the keyboard works (↑ ↓ Enter → ← Esc).",
            covers: &["shadcn/ui: Dropdown Menu", "Animate UI: Dropdown Menu", "Origin UI: Dropdown", "Radix: Dropdown Menu"],
            code: r##"let trigger = ui.button("Open");
let picked = DropdownMenu::new(vec![
    MenuEntry::label("My Account"),
    MenuEntry::item("Profile").icon(LineIcon::User).shortcut("Ctrl+P"),
    MenuEntry::submenu("Invite users", vec![MenuEntry::item("Email"), MenuEntry::item("Message")]),
    MenuEntry::separator(),
    MenuEntry::item("Log out").icon(LineIcon::LogOut).destructive(),
]).show(&trigger);"##,
            show: |ui| {
                let sid = ui.scope_id().with("view");
                let (mut status, mut activity, mut panel, mut pos): (bool, bool, bool, u8) = glowkit_core::state(ui, sid, || (true, false, false, 0));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() * 0.18);
                    let r = ui.add_sized(vec2(150.0, 34.0), egui::Button::new("☰  Open menu"));
                    if !flag(ui, "init", false) {
                        DropdownMenu::set_open(ui.ctx(), r.id, true);
                        set_flag(ui, "init", true);
                    }
                    let _ = DropdownMenu::new(vec![
                        MenuEntry::label("My Account"),
                        MenuEntry::item("Profile").icon(LineIcon::User).shortcut("Shift+Ctrl+P"),
                        MenuEntry::item("Billing").icon(LineIcon::CreditCard).shortcut("Ctrl+B"),
                        MenuEntry::item("Settings").icon(LineIcon::Settings).shortcut("Ctrl+S"),
                        MenuEntry::separator(),
                        MenuEntry::item("Team").icon(LineIcon::Users),
                        MenuEntry::submenu("Invite users", vec![
                            MenuEntry::item("Email").icon(LineIcon::Mail),
                            MenuEntry::item("Message").icon(LineIcon::Inbox),
                            MenuEntry::separator(),
                            MenuEntry::item("More…").icon(LineIcon::Plus),
                        ]).icon(LineIcon::Plus),
                        MenuEntry::item("API").icon(LineIcon::Command).disabled(true),
                        MenuEntry::separator(),
                        MenuEntry::item("Log out").icon(LineIcon::LogOut).shortcut("Shift+Ctrl+Q").destructive(),
                    ])
                    .show(&r);
                    ui.add_space(ui.available_width() * 0.35);
                    let r2 = ui.add_sized(vec2(110.0, 34.0), egui::Button::new("View  ▾"));
                    if let Some(item) = DropdownMenu::new(vec![
                        MenuEntry::label("Appearance"),
                        MenuEntry::checkbox("Status Bar", status),
                        MenuEntry::checkbox("Activity Bar", activity),
                        MenuEntry::checkbox("Panel", panel),
                        MenuEntry::separator(),
                        MenuEntry::label("Panel Position"),
                        MenuEntry::radio("Top", pos == 0),
                        MenuEntry::radio("Bottom", pos == 1),
                        MenuEntry::radio("Right", pos == 2),
                    ])
                    .show(&r2)
                    {
                        match item.as_str() {
                            "Status Bar" => status = !status,
                            "Activity Bar" => activity = !activity,
                            "Panel" => panel = !panel,
                            "Top" => pos = 0,
                            "Bottom" => pos = 1,
                            _ => pos = 2,
                        }
                    }
                });
                glowkit_core::set_state(ui, sid, (status, activity, panel, pos));
            },
        },
        Demo {
            id: "context-menu",
            name: "Context Menu",
            category: C,
            description: "Right-click anywhere in the area to open a menu at the pointer, with submenus, checkbox and radio items.",
            covers: &["shadcn/ui: Context Menu", "Animate UI: Context Menu", "Radix: Context Menu"],
            code: r##"let area = ui.allocate_response(size, Sense::click());
if let Some(item) = ContextMenu::new(vec![
    MenuEntry::item("Back").shortcut("Ctrl+["),
    MenuEntry::item("Reload").shortcut("Ctrl+R"),
    MenuEntry::separator(),
    MenuEntry::checkbox("Show Bookmarks", true),
]).show(&area) { /* … */ }"##,
            show: |ui| {
                let t = Theme::of(ui);
                let sid = ui.scope_id().with("cm");
                let (mut bookmarks, mut urls, mut person): (bool, bool, u8) = glowkit_core::state(ui, sid, || (true, false, 0));
                let w = ui.available_width();
                let (rect, resp) = ui.allocate_exact_size(vec2(w, 380.0), egui::Sense::click());
                let area = egui::Rect::from_min_size(rect.min + vec2(20.0, 10.0), vec2(w * 0.42, 300.0));
                let pnt = ui.painter();
                let n = ((area.width() + area.height()) * 2.0 / 12.0) as usize;
                let pts = glowkit_core::paint::subdivide_closed(&glowkit_core::paint::rounded_rect_points(area, t.radius, 8), 6.0);
                for (i, p) in pts.iter().enumerate() {
                    if i % 2 == 0 && i + 1 < pts.len() {
                        pnt.line_segment([*p, pts[i + 1]], egui::Stroke::new(1.0, t.border));
                    }
                }
                let _ = n;
                pnt.text(area.center(), egui::Align2::CENTER_CENTER, "Right click here", egui::FontId::proportional(14.0), t.muted_foreground);
                if !flag(ui, "init", false) {
                    ContextMenu::open_at(ui.ctx(), resp.id, area.center() + vec2(30.0, -110.0));
                    set_flag(ui, "init", true);
                }
                if let Some(item) = ContextMenu::new(vec![
                    MenuEntry::item("Back").shortcut("Ctrl+["),
                    MenuEntry::item("Forward").shortcut("Ctrl+]").disabled(true),
                    MenuEntry::item("Reload").shortcut("Ctrl+R"),
                    MenuEntry::submenu("More Tools", vec![
                        MenuEntry::item("Save Page As…").shortcut("Shift+Ctrl+S"),
                        MenuEntry::item("Create Shortcut…"),
                        MenuEntry::item("Name Window…"),
                        MenuEntry::separator(),
                        MenuEntry::item("Developer Tools"),
                    ]),
                    MenuEntry::separator(),
                    MenuEntry::checkbox("Show Bookmarks Bar", bookmarks),
                    MenuEntry::checkbox("Show Full URLs", urls),
                    MenuEntry::separator(),
                    MenuEntry::label("People"),
                    MenuEntry::radio("Pedro Duarte", person == 0),
                    MenuEntry::radio("Colm Tuite", person == 1),
                ])
                .show(&resp)
                {
                    match item.as_str() {
                        "Show Bookmarks Bar" => bookmarks = !bookmarks,
                        "Show Full URLs" => urls = !urls,
                        "Pedro Duarte" => person = 0,
                        "Colm Tuite" => person = 1,
                        _ => {}
                    }
                }
                glowkit_core::set_state(ui, sid, (bookmarks, urls, person));
            },
        },
        Demo {
            id: "notice-banner",
            name: "Announcement Banner",
            category: C,
            description: "A dismissible top-of-page banner with a badge and an action link; gradient, subtle, accent and warning styles. Collapses smoothly when closed.",
            covers: &["Origin UI: Banner", "shadcn blocks: Announcement Bar", "Kibo UI: Announcement", "Magic UI: Announcement"],
            code: r##"NoticeBanner::new(&mut visible, "glowkit 1.0 is here, with 150+ components")
    .badge("New")
    .action("Read the changelog")
    .style(BannerStyle::Gradient)
    .show(ui);"##,
            show: |ui| {
                let sid = ui.scope_id().with("vis");
                let mut vis: [bool; 4] = glowkit_core::state(ui, sid, || [true; 4]);
                ui.add_space(10.0);
                let _ = NoticeBanner::new(&mut vis[0], "glowkit 1.0 is here, with 150+ animated components").badge("New").action("Read the changelog").show(ui);
                ui.add_space(14.0);
                let _ = NoticeBanner::new(&mut vis[1], "We're hiring Rust engineers in Lisbon and remote").icon(LineIcon::Sparkles).action("See open roles").style(BannerStyle::Subtle).show(ui);
                ui.add_space(14.0);
                let _ = NoticeBanner::new(&mut vis[2], "Black Friday: 40% off all plans this week").badge("Sale").action("Upgrade").style(BannerStyle::Accent).show(ui);
                ui.add_space(14.0);
                let _ = NoticeBanner::new(&mut vis[3], "Scheduled maintenance on Sunday, 02:00–04:00 UTC").icon(LineIcon::Warning).style(BannerStyle::Warning).show(ui);
                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    if kit::button(ui, "Reset banners", Btn::Outline).clicked() {
                        vis = [true; 4];
                    }
                });
                glowkit_core::set_state(ui, sid, vis);
            },
        },
        Demo {
            id: "spotlight-tour",
            name: "Spotlight Tour",
            category: C,
            description: "A guided onboarding tour: the page dims except a glowing cut-out that glides between targets, with a step card, progress dots and Back / Next.",
            covers: &["Origin UI: Tour", "Kibo UI: Tour / Coachmark", "Shepherd.js: Tour", "React Joyride"],
            code: r##"SpotlightTour::new(&mut step, vec![
    TourStep::new(search.rect, "Quick search", "Find any file or command with Ctrl+K."),
    TourStep::new(new_btn.rect, "Create a project", "Start from a template or a blank canvas."),
    TourStep::new(avatar.rect, "Your account", "Manage your profile and billing here.").placement(Placement::Left),
]).show(ui);"##,
            show: |ui| {
                let canvas = kit::demo_canvas(ui, 400.0);
                let t = Theme::of(ui);
                let p = ui.painter();
                // three fake targets on the canvas
                let search = egui::Rect::from_min_size(canvas.min + vec2(canvas.width() * 0.32, 8.0), vec2(220.0, 28.0));
                p.rect_filled(search, 6.0, t.muted);
                p.rect_stroke(search, 6.0, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                LineIcon::Search.paint(p, egui::Rect::from_center_size(search.left_center() + vec2(16.0, 0.0), vec2(14.0, 14.0)), t.muted_foreground);
                p.text(search.left_center() + vec2(32.0, 0.0), egui::Align2::LEFT_CENTER, "Search…", egui::FontId::proportional(13.0), t.muted_foreground);
                let newb = egui::Rect::from_min_size(canvas.min + vec2(canvas.width() - 190.0, 8.0), vec2(118.0, 28.0));
                p.rect_filled(newb, 6.0, t.primary);
                p.text(newb.center(), egui::Align2::CENTER_CENTER, "+ New project", egui::FontId::proportional(13.0), t.primary_foreground);
                let avatar = egui::Rect::from_center_size(egui::pos2(canvas.max.x - 22.0, canvas.min.y + 22.0), vec2(26.0, 26.0));
                glowkit_core::paint::gradient_circle(p, avatar.center(), 13.0, &t.gradient[..2]);
                let sid = ui.scope_id().with("step");
                let mut step: Option<usize> = glowkit_core::state(ui, sid, || Some(0));
                trigger_row(ui, canvas, |ui| {
                    if kit::button(ui, "Restart tour", Btn::Outline).clicked() {
                        step = Some(0);
                    }
                });
                let _ = SpotlightTour::new(&mut step, vec![
                    TourStep::new(search, "Quick search", "Find any file, person or command from anywhere with Ctrl+K."),
                    TourStep::new(newb, "Create a project", "Start from a template or a blank canvas. You can invite your team later."),
                    TourStep::new(avatar, "Your account", "Manage your profile, billing and notification settings here.").placement(Placement::Left),
                ])
                .within(canvas)
                .show(ui);
                glowkit_core::set_state(ui, sid, step);
            },
        },
        Demo {
            id: "morphing-island",
            name: "Dynamic Island",
            category: C,
            description: "A black pill that morphs between idle, compact status, call, media player and notification layouts with springy resizing and cross-faded content.",
            covers: &["Cult UI: Dynamic Island", "Aceternity: Dynamic Island", "Apple: Dynamic Island (pattern)", "Motion Primitives: Toolbar Dynamic", "Motion Primitives: Toolbar Expandable"],
            code: r##"ui.add(MorphingIsland::new(IslandView::Music {
    title: "Midnight City".into(),
    artist: "M83".into(),
    progress: 0.42,
}));"##,
            show: |ui| {
                let t = Theme::of(ui);
                let now = ui.input(|i| i.time);
                let sid = ui.scope_id().with("mode");
                let (mut mode, mut since, mut auto): (usize, f64, bool) = glowkit_core::state(ui, sid, || (0, now, true));
                if auto && now - since > 2.8 {
                    mode = (mode + 1) % 5;
                    since = now;
                }
                if auto {
                    ui.ctx().request_repaint_after(std::time::Duration::from_secs_f64(0.2));
                }
                let view = match mode {
                    0 => IslandView::Music { title: "Midnight City".into(), artist: "M83".into(), progress: ((now * 0.01) % 1.0) as f32 + 0.3 },
                    1 => IslandView::Call { name: "Ana Souza".into(), subtitle: "mobile".into() },
                    2 => IslandView::Compact { icon: LineIcon::Bell, color: t.success, text: "Ring".into(), trailing: "Silent off".into() },
                    3 => IslandView::Notification { title: "New message".into(), body: "Hey! Are we still on for the design review at 3?".into() },
                    _ => IslandView::Idle,
                };
                ui.add_space(20.0);
                ui.add(MorphingIsland::new(view));
                ui.add_space(24.0);
                ui.horizontal(|ui| {
                    let labels = ["Music", "Call", "Compact", "Notification", "Idle"];
                    let total: f32 = labels.iter().map(|l| kit::button_width(ui, l, 13.5) + 8.0).sum::<f32>() + 90.0;
                    ui.add_space((ui.available_width() - total) / 2.0);
                    for (i, l) in labels.iter().enumerate() {
                        if kit::button(ui, l, if mode == i { Btn::Primary } else { Btn::Outline }).clicked() {
                            mode = i;
                            auto = false;
                        }
                    }
                    if kit::button(ui, if auto { "Pause" } else { "Auto" }, Btn::Ghost).clicked() {
                        auto = !auto;
                        since = now;
                    }
                });
                glowkit_core::set_state(ui, sid, (mode, since, auto));
            },
        },
    ]
}
