use super::*;
use crate::inputs::demos::{caption, center, col, with};
use crate::inputs::{Button, ButtonVariant, UiIcon};
use egui::{vec2, Align, Layout, RichText, Ui};
use glowkit_core::demo::{Category::DataDisplay as C, Demo};
use glowkit_core::Theme;

fn users() -> Vec<Vec<TableCell>> {
    let t = [
        ("Ada Lovelace", "ada@analytical.io", "Owner", BadgeVariant::Default, "Active", 0, 1840.0),
        ("Alan Turing", "alan@bletchley.uk", "Admin", BadgeVariant::Secondary, "Active", 0, 1290.5),
        ("Grace Hopper", "grace@cobol.dev", "Developer", BadgeVariant::Outline, "Away", 1, 940.0),
        ("Linus Torvalds", "linus@kernel.org", "Developer", BadgeVariant::Outline, "Active", 0, 2210.0),
        ("Margaret Hamilton", "margaret@apollo.space", "Admin", BadgeVariant::Secondary, "Offline", 2, 610.25),
        ("Dennis Ritchie", "dmr@bell-labs.com", "Viewer", BadgeVariant::Outline, "Active", 0, 120.0),
        ("Barbara Liskov", "barbara@mit.edu", "Developer", BadgeVariant::Outline, "Away", 1, 1505.0),
        ("Ken Thompson", "ken@golang.org", "Viewer", BadgeVariant::Outline, "Offline", 2, 75.5),
        ("Hedy Lamarr", "hedy@spectrum.fm", "Admin", BadgeVariant::Secondary, "Active", 0, 3020.0),
        ("Tim Berners-Lee", "tim@w3.org", "Developer", BadgeVariant::Outline, "Active", 0, 860.0),
        ("Radia Perlman", "radia@spanning.net", "Viewer", BadgeVariant::Outline, "Away", 1, 430.0),
        ("John Carmack", "john@idsoftware.com", "Developer", BadgeVariant::Outline, "Active", 0, 1999.99),
    ];
    t.iter()
        .map(|(n, e, role, v, status, s, amt)| {
            let col = [glowkit_core::hex("#22C55E"), glowkit_core::hex("#F59E0B"), glowkit_core::hex("#71717A")][*s];
            vec![
                TableCell::User { name: n.to_string(), email: e.to_string() },
                TableCell::Badge(role.to_string(), *v),
                TableCell::Status(status.to_string(), col),
                TableCell::Number(*amt, format!("${}", super::display::group_thousands(*amt, 0))),
            ]
        })
        .collect()
}

fn files() -> Vec<TreeNode> {
    vec![TreeNode::branch(
        "glowkit",
        vec![
            TreeNode::branch(
                "crates",
                vec![
                    TreeNode::branch(
                        "glowkit-core",
                        vec![
                            TreeNode::leaf("theme.rs").detail("6 KB"),
                            TreeNode::leaf("anim.rs").detail("12 KB"),
                            TreeNode::leaf("paint.rs").detail("21 KB"),
                        ],
                    ),
                    TreeNode::branch(
                        "glowkit-components",
                        vec![
                            TreeNode::leaf("lib.rs").detail("1 KB"),
                            TreeNode::branch("inputs", vec![TreeNode::leaf("button.rs"), TreeNode::leaf("switch.rs")]),
                        ],
                    ),
                ],
            ),
            TreeNode::branch("apps", vec![TreeNode::leaf("gallery").icon(UiIcon::Grid)]),
            TreeNode::leaf("Cargo.toml").icon(UiIcon::Settings).detail("2 KB"),
            TreeNode::leaf("README.md").detail("8 KB"),
        ],
    )]
}

const CODE: &str = r#"use glowkit::prelude::*;

/// A settings row with a springy switch.
fn settings(ui: &mut egui::Ui, state: &mut State) {
    Theme::dark().install(ui.ctx());
    ui.add(Switch::new(&mut state.wifi).label("Wi-Fi"));
    if ui.add(Button::new("Save").icon(UiIcon::Check)).clicked() {
        println!("saved {} settings", 3);
    }
}"#;

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "badge",
            name: "Badge",
            category: C,
            description: "Status labels in solid, soft and outline styles, with dots, icons and rolling animated counts.",
            covers: &["shadcn/ui: Badge", "Origin UI: Badges", "Kibo UI: Status", "Animate UI: Counting Number (badge)"],
            code: r##"ui.add(Badge::new("New"));
ui.add(Badge::new("Active").variant(BadgeVariant::Success).dot(true));
ui.add(Badge::new("Verified").variant(BadgeVariant::Info).icon(UiIcon::CheckCircle));
ui.add(Badge::count(unread));"##,
            show: |ui| {
                ui.add_space(50.0);
                with(ui, "badge", || 3i64, |ui, n| {
                    center(ui, 560.0, |ui| {
                        ui.spacing_mut().item_spacing = vec2(8.0, 14.0);
                        caption(ui, "Variants");
                        ui.horizontal(|ui| {
                            ui.add(Badge::new("Default"));
                            ui.add(Badge::new("Secondary").variant(BadgeVariant::Secondary));
                            ui.add(Badge::new("Outline").variant(BadgeVariant::Outline));
                            ui.add(Badge::new("Destructive").variant(BadgeVariant::Destructive));
                            ui.add(Badge::new("Accent").variant(BadgeVariant::Accent));
                        });
                        caption(ui, "Status");
                        ui.horizontal(|ui| {
                            ui.add(Badge::new("Active").variant(BadgeVariant::Success).dot(true).pill(true));
                            ui.add(Badge::new("Pending").variant(BadgeVariant::Warning).dot(true).pill(true));
                            ui.add(Badge::new("Failed").variant(BadgeVariant::Destructive).dot(true).pill(true));
                            ui.add(Badge::new("Verified").variant(BadgeVariant::Info).icon(UiIcon::CheckCircle));
                            ui.add(Badge::new("Pro").color(glowkit_core::hex("#EC4899")).icon(UiIcon::Sparkles));
                        });
                        caption(ui, "Animated count");
                        ui.horizontal(|ui| {
                            ui.add(Badge::count(*n));
                            ui.add(Badge::count(*n * 12).variant(BadgeVariant::Destructive));
                            ui.add_space(8.0);
                            if ui.add(Button::icon_only(UiIcon::Minus).small().variant(ButtonVariant::Outline)).clicked() {
                                *n = (*n - 1).max(0);
                            }
                            if ui.add(Button::icon_only(UiIcon::Plus).small().variant(ButtonVariant::Outline)).clicked() {
                                *n += 1;
                            }
                        });
                    });
                });
            },
        },
        Demo {
            id: "chip",
            name: "Chip",
            category: C,
            description: "Compact tags: removable chips, filter chips that draw a check when selected, and avatar chips.",
            covers: &["Origin UI: Tags / Chips", "Kibo UI: Pill", "Material: Chips"],
            code: r##"if ui.add(Chip::new(name).removable(true)).clicked() { remove(name) }
if ui.add(Chip::new("Rust").selected(on)).clicked() { on = !on }
ui.add(Chip::new("Ada Lovelace").avatar("Ada Lovelace"));"##,
            show: |ui| {
                ui.add_space(60.0);
                with(ui, "chips", || (vec!["Design", "Engineering", "Marketing", "Sales"].into_iter().map(String::from).collect::<Vec<_>>(), [true, false, true, false, false]), |ui, (tags, filters)| {
                    center(ui, 520.0, |ui| {
                        ui.spacing_mut().item_spacing = vec2(8.0, 14.0);
                        caption(ui, "Removable");
                        ui.horizontal(|ui| {
                            let mut remove = None;
                            for (i, tg) in tags.iter().enumerate() {
                                if ui.add(Chip::new(tg.clone()).removable(true)).clicked() {
                                    remove = Some(i);
                                }
                            }
                            if let Some(i) = remove {
                                tags.remove(i);
                            }
                            if tags.is_empty() && ui.add(Button::new("Reset").small().variant(ButtonVariant::Ghost)).clicked() {
                                *tags = vec!["Design", "Engineering", "Marketing", "Sales"].into_iter().map(String::from).collect();
                            }
                        });
                        caption(ui, "Filters");
                        ui.horizontal(|ui| {
                            for (i, name) in ["Rust", "TypeScript", "Go", "Python", "Zig"].iter().enumerate() {
                                if ui.add(Chip::new(*name).selected(filters[i])).clicked() {
                                    filters[i] = !filters[i];
                                }
                            }
                        });
                        caption(ui, "People");
                        ui.horizontal(|ui| {
                            ui.add(Chip::new("Ada Lovelace").avatar("Ada Lovelace"));
                            ui.add(Chip::new("Alan Turing").avatar("Alan Turing").removable(true));
                            ui.add(Chip::new("Grace Hopper").avatar("Grace Hopper"));
                        });
                    });
                });
            },
        },
        Demo {
            id: "avatar",
            name: "Avatar & Avatar Group",
            category: C,
            description: "Initial avatars with name-derived gradients, presence dots, sizes and overlapping groups that lift on hover.",
            covers: &["shadcn/ui: Avatar", "Origin UI: Avatar", "Origin UI: Avatar group", "Kibo UI: Avatar Stack"],
            code: r##"ui.add(Avatar::new("Ada Lovelace").size(48.0).status(AvatarStatus::Online));
ui.add(Avatar::new("Grace Hopper").style(AvatarStyle::Soft).square(true));
ui.add(AvatarGroup::new(&["Ada", "Alan", "Grace", "Linus", "Margaret", "Dennis"]).max(4));"##,
            show: |ui| {
                ui.add_space(50.0);
                center(ui, 560.0, |ui| {
                    ui.spacing_mut().item_spacing = vec2(14.0, 16.0);
                    caption(ui, "Sizes, styles & status");
                    ui.horizontal(|ui| {
                        ui.set_min_height(56.0);
                        ui.add(Avatar::new("Ada Lovelace").size(56.0).status(AvatarStatus::Online));
                        ui.add(Avatar::new("Alan Turing").size(48.0).status(AvatarStatus::Away));
                        ui.add(Avatar::new("Grace Hopper").size(40.0).status(AvatarStatus::Busy));
                        ui.add(Avatar::new("Linus Torvalds").size(32.0).status(AvatarStatus::Offline));
                        ui.add(Avatar::new("Margaret Hamilton").size(48.0).style(AvatarStyle::Soft));
                        ui.add(Avatar::new("Dennis Ritchie").size(48.0).style(AvatarStyle::Muted));
                        ui.add(Avatar::new("Hedy Lamarr").size(48.0).square(true).status(AvatarStatus::Online));
                    });
                    caption(ui, "Groups");
                    ui.horizontal(|ui| {
                        ui.add(AvatarGroup::new(&["Ada Lovelace", "Alan Turing", "Grace Hopper", "Linus Torvalds", "Margaret Hamilton", "Dennis Ritchie", "Ken Thompson"]).max(4).size(40.0));
                        ui.add_space(20.0);
                        ui.add(AvatarGroup::new(&["Barbara Liskov", "Tim Berners-Lee", "Radia Perlman"]).style(AvatarStyle::Soft).size(36.0).overlap(0.2));
                    });
                });
            },
        },
        Demo {
            id: "avatar-circles",
            name: "Avatar Circles",
            category: C,
            description: "A tight row of overlapping avatars ending in a bold +N bubble, for social proof.",
            covers: &["Magic UI: Avatar Circles"],
            code: r##"ui.add(AvatarCircles::new(&["Ada Lovelace", "Alan Turing", "Grace Hopper", "Linus Torvalds", "Hedy Lamarr"], 99));"##,
            show: |ui| {
                ui.add_space(110.0);
                center(ui, 300.0, |ui| {
                    let t = Theme::of(ui);
                    ui.add(AvatarCircles::new(&["Ada Lovelace", "Alan Turing", "Grace Hopper", "Linus Torvalds", "Hedy Lamarr", "Ken Thompson"], 99));
                    ui.add_space(8.0);
                    ui.label(RichText::new("Trusted by 2,400+ developers").color(t.muted_foreground));
                });
            },
        },
        Demo {
            id: "card",
            name: "Card",
            category: C,
            description: "Cards with header, description, action, body and footer slots; hoverable cards lift their shadow.",
            covers: &["shadcn/ui: Card", "Origin UI: Cards", "Kibo UI: Card"],
            code: r##"Card::new()
    .title("Create project")
    .description("Deploy your new project in one click.")
    .footer(|ui| {
        ui.add(Button::new("Cancel").variant(ButtonVariant::Outline));
        ui.add(Button::new("Deploy"));
    })
    .show(ui, |ui| {
        ui.add(Input::new(&mut name).label("Name").placeholder("Name of your project"));
    });"##,
            show: |ui| {
                ui.add_space(4.0);
                with(ui, "card", || (String::from("glowkit-gallery"), Some(0usize)), |ui, (name, fw)| {
                    center(ui, 760.0, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 24.0;
                            col(ui, 360.0, |ui| {
                                Card::new()
                                    .title("Create project")
                                    .description("Deploy your new project in one click.")
                                    .width(360.0)
                                    .footer(|ui| {
                                        ui.add(Button::new("Cancel").variant(ButtonVariant::Outline));
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            ui.add(Button::new("Deploy").icon(UiIcon::Zap));
                                        });
                                    })
                                    .show(ui, |ui| {
                                        ui.spacing_mut().item_spacing.y = 14.0;
                                        ui.add(crate::Input::new(name).label("Name").placeholder("Name of your project").width(316.0));
                                        ui.add(crate::Select::new(fw, ["Next.js", "SvelteKit", "Leptos", "Dioxus"]).label("Framework").width(316.0).id_salt("card-fw"));
                                    });
                            });
                            col(ui, 360.0, |ui| {
                                ui.spacing_mut().item_spacing.y = 18.0;
                                Card::new()
                                    .title("Notifications")
                                    .description("You have 3 unread messages.")
                                    .width(360.0)
                                    .hoverable(true)
                                    .action(|ui| {
                                        ui.add(Badge::count(3));
                                    })
                                    .footer_border(true)
                                    .footer(|ui| {
                                        ui.add(Button::new("Mark all as read").icon(UiIcon::Check).width(316.0));
                                    })
                                    .show(ui, |ui| {
                                        let t = Theme::of(ui);
                                        for (title, when) in [("Your call has been confirmed.", "1 hour ago"), ("You have a new message!", "1 hour ago")] {
                                            ui.horizontal_top(|ui| {
                                                let (r, _) = ui.allocate_exact_size(vec2(8.0, 18.0), egui::Sense::hover());
                                                ui.painter().circle_filled(r.center(), 4.0, t.info);
                                                ui.vertical(|ui| {
                                                    ui.spacing_mut().item_spacing.y = 2.0;
                                                    ui.label(RichText::new(title).size(13.5).color(t.foreground));
                                                    ui.label(RichText::new(when).size(12.5).color(t.muted_foreground));
                                                });
                                            });
                                        }
                                    });
                            });
                        });
                    });
                });
            },
        },
        Demo {
            id: "alert",
            name: "Alert",
            category: C,
            description: "Callouts with an icon, title and description in neutral, info, success, warning and destructive styles.",
            covers: &["shadcn/ui: Alert", "Origin UI: Alerts", "Kibo UI: Banner (inline)"],
            code: r##"ui.add(Alert::new("Heads up!").description("You can add components to your app using the CLI."));
ui.add(Alert::new("Payment failed").description("Your card was declined.").variant(AlertVariant::Destructive));"##,
            show: |ui| {
                ui.add_space(4.0);
                center(ui, 760.0, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing = vec2(16.0, 12.0);
                        col(ui, 372.0, |ui| {
                            ui.spacing_mut().item_spacing.y = 12.0;
                            ui.add(Alert::new("Heads up!").description("You can add components to your app using the CLI.").width(372.0));
                            ui.add(Alert::new("New version available").description("glowkit 0.2 ships 40 new components.").variant(AlertVariant::Info).width(372.0));
                            ui.add(Alert::new("Deployment complete").description("Your site is live at glowkit.dev.").variant(AlertVariant::Success).width(372.0));
                        });
                        col(ui, 372.0, |ui| {
                            ui.spacing_mut().item_spacing.y = 12.0;
                            ui.add(Alert::new("Storage almost full").description("You've used 92% of your 10 GB plan.").variant(AlertVariant::Warning).width(372.0));
                            ui.add(Alert::new("Payment failed").description("Your card was declined. Update your billing details to keep your plan.").variant(AlertVariant::Destructive).width(372.0));
                            ui.add(Alert::new("With an accent bar").variant(AlertVariant::Info).accent_bar(true).width(372.0));
                        });
                    });
                });
            },
        },
        Demo {
            id: "kbd",
            name: "Kbd & Separator",
            category: C,
            description: "Keyboard key caps for shortcuts, and hairline separators (horizontal, vertical, labeled).",
            covers: &["shadcn/ui: Kbd", "shadcn/ui: Separator", "Origin UI: Kbd"],
            code: r##"ui.horizontal(|ui| { ui.add(Kbd::new("⌘")); ui.add(Kbd::new("K")); });
ui.add(Separator::horizontal().label("or continue with"));
ui.add(Separator::vertical());"##,
            show: |ui| {
                ui.add_space(50.0);
                center(ui, 420.0, |ui| {
                    let t = Theme::of(ui);
                    ui.spacing_mut().item_spacing = vec2(6.0, 14.0);
                    for (label, keys) in [("Command palette", &["⌘", "K"][..]), ("Search", &["Ctrl", "Shift", "F"][..]), ("Save", &["⌘", "S"][..])] {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(label).color(t.foreground));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                for k in keys.iter().rev() {
                                    ui.add(Kbd::new(*k));
                                }
                            });
                        });
                    }
                    ui.add(Separator::horizontal().label("or continue with").margin(6.0));
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        for (i, l) in ["Blog", "Docs", "Source", "Changelog"].iter().enumerate() {
                            if i > 0 {
                                ui.add(Separator::vertical().length(16.0));
                            }
                            ui.label(RichText::new(*l).color(t.muted_foreground));
                        }
                    });
                });
            },
        },
        Demo {
            id: "data-table",
            name: "Data Table",
            category: C,
            description: "Sortable columns with an animated indicator, row hover, selection with select-all, rich cells and pagination.",
            covers: &["shadcn/ui: Data Table", "shadcn/ui: Table", "Origin UI: Tables", "Kibo UI: Table", "Dice UI: Data Table"],
            code: r##"let cols = vec![
    TableColumn::new("User").flex(2.2),
    TableColumn::new("Role"),
    TableColumn::new("Status"),
    TableColumn::new("Spend").right(),
];
ui.add(DataTable::new(cols, &rows).selection(&mut selected).page_size(5));"##,
            show: |ui| {
                ui.add_space(4.0);
                with(ui, "table", || vec![false, true, false, false, false], |ui, sel| {
                    let rows = users();
                    center(ui, 820.0, |ui| {
                        ui.add(DataTable::new(vec![TableColumn::new("User").flex(2.2), TableColumn::new("Role"), TableColumn::new("Status"), TableColumn::new("Spend").right()], &rows).selection(sel).page_size(5).width(820.0).id_salt("users"));
                    });
                });
            },
        },
        Demo {
            id: "table",
            name: "Table",
            category: C,
            description: "A simple striped invoice table with badges and right-aligned amounts.",
            covers: &["shadcn/ui: Table", "Origin UI: Table (striped)"],
            code: r##"ui.add(DataTable::new(cols, &invoices).striped(true));"##,
            show: |ui| {
                ui.add_space(4.0);
                let rows: Vec<Vec<TableCell>> = [("INV001", "Paid", BadgeVariant::Success, "Credit Card", 250.0), ("INV002", "Pending", BadgeVariant::Warning, "PayPal", 150.0), ("INV003", "Unpaid", BadgeVariant::Destructive, "Bank Transfer", 350.0), ("INV004", "Paid", BadgeVariant::Success, "Credit Card", 450.0), ("INV005", "Paid", BadgeVariant::Success, "PayPal", 550.0)]
                    .iter()
                    .map(|(id, s, v, m, a)| vec![TableCell::Strong(id.to_string()), TableCell::Badge(s.to_string(), *v), TableCell::Muted(m.to_string()), TableCell::Number(*a, format!("${a:.2}"))])
                    .collect();
                center(ui, 640.0, |ui| {
                    ui.add(DataTable::new(vec![TableColumn::new("Invoice"), TableColumn::new("Status"), TableColumn::new("Method").flex(1.4), TableColumn::new("Amount").right()], &rows).striped(true).row_height(44.0).width(640.0).id_salt("invoices"));
                });
            },
        },
        Demo {
            id: "timeline",
            name: "Timeline",
            category: C,
            description: "A vertical timeline whose connector fills up to the current event, which pulses softly.",
            covers: &["Origin UI: Timeline", "Kibo UI: Timeline", "Aceternity: Timeline"],
            code: r##"ui.add(Timeline::new(vec![
    TimelineItem::new("Order placed", "Mar 3, 9:24", "We received your order #4021."),
    TimelineItem::new("Shipped", "Mar 4, 14:10", "Your package left our warehouse."),
    TimelineItem::new("Out for delivery", "Mar 6, 08:02", "The courier is on the way."),
]).active(step));"##,
            show: |ui| {
                ui.add_space(4.0);
                with(ui, "tl", || 2usize, |ui, step| {
                    center(ui, 600.0, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 30.0;
                            col(ui, 400.0, |ui| {
                                ui.add(
                                    Timeline::new(vec![
                                        TimelineItem::new("Order placed", "Mar 3, 9:24", "We received your order #4021 and sent a confirmation email."),
                                        TimelineItem::new("Payment confirmed", "Mar 3, 9:25", "Visa ending in 4242 was charged $129.00."),
                                        TimelineItem::new("Shipped", "Mar 4, 14:10", "Your package left our warehouse in Rotterdam.").icon(UiIcon::Inbox),
                                        TimelineItem::new("Out for delivery", "Mar 6, 08:02", "The courier is on the way to your address.").icon(UiIcon::Home),
                                        TimelineItem::new("Delivered", "Expected Mar 6", "Signature required on delivery.").icon(UiIcon::CheckCircle),
                                    ])
                                    .active(*step)
                                    .width(400.0),
                                );
                            });
                            col(ui, 140.0, |ui| {
                                ui.spacing_mut().item_spacing.y = 8.0;
                                ui.add_space(4.0);
                                if ui.add(Button::new("Advance").icon(UiIcon::ArrowDown).width(140.0)).clicked() {
                                    *step = (*step + 1) % 6;
                                }
                                if ui.add(Button::new("Reset").variant(ButtonVariant::Outline).width(140.0)).clicked() {
                                    *step = 0;
                                }
                            });
                        });
                    });
                });
            },
        },
        Demo {
            id: "stat-card",
            name: "Stat Card",
            category: C,
            description: "KPI cards whose values count up, with colored delta pills and sparklines that draw themselves in.",
            covers: &["shadcn/ui: Dashboard cards", "Origin UI: Stats", "Kibo UI: KPI", "Tremor: Metric card"],
            code: r##"ui.add(StatCard::new("Total revenue", 45231.89).prefix("$").decimals(2)
    .delta(20.1).icon(UiIcon::Dollar)
    .sparkline(&[12.0, 18.0, 15.0, 22.0, 20.0, 28.0, 31.0]));"##,
            show: |ui| {
                ui.add_space(70.0);
                center(ui, 852.0, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 16.0;
                        ui.add(StatCard::new("Total revenue", 45231.89).prefix("$").decimals(2).delta(20.1).icon(UiIcon::Dollar).sparkline(&[12.0, 18.0, 15.0, 22.0, 20.0, 28.0, 26.0, 31.0]).width(272.0));
                        ui.add(StatCard::new("Active users", 2350.0).prefix("+").delta(180.1).icon(UiIcon::Users).sparkline(&[5.0, 7.0, 6.0, 9.0, 12.0, 11.0, 15.0, 18.0]).color(Theme::of(ui).chart[1]).width(272.0));
                        ui.add(StatCard::new("Churn rate", 2.4).suffix("%").decimals(1).delta(-0.6).invert_delta(true).icon(UiIcon::Activity).sparkline(&[4.0, 3.6, 3.8, 3.1, 2.9, 3.0, 2.6, 2.4]).width(272.0));
                    });
                });
            },
        },
        Demo {
            id: "empty-state",
            name: "Empty State",
            category: C,
            description: "A friendly placeholder with a fanned stack of icon tiles that spreads on hover, copy and actions.",
            covers: &["shadcn/ui: Empty", "Origin UI: Empty states", "Kibo UI: Empty"],
            code: r##"if ui.add(EmptyState::new(UiIcon::Folder, "No projects yet")
    .description("You haven't created any projects. Get started by creating your first one.")
    .action("Create project")
    .secondary_action("Import")).clicked() { /* create */ }"##,
            show: |ui| {
                ui.add_space(20.0);
                center(ui, 520.0, |ui| {
                    ui.add(EmptyState::new(UiIcon::Folder, "No projects yet").side_icons(UiIcon::Code, UiIcon::Image).description("You haven't created any projects yet. Get started by creating your first project.").action("Create project").secondary_action("Import").width(520.0));
                });
            },
        },
        Demo {
            id: "tree-view",
            name: "Tree View",
            category: C,
            description: "A file-explorer tree with guides, rotating chevrons, animated expand / collapse and selection.",
            covers: &["Kibo UI: Tree", "Origin UI: Tree", "Magic UI: File Tree", "shadcn/ui: Tree (community)"],
            code: r##"let nodes = vec![TreeNode::branch("src", vec![
    TreeNode::leaf("main.rs"),
    TreeNode::branch("ui", vec![TreeNode::leaf("button.rs")]),
])];
ui.add(TreeView::new(&nodes, &mut selected).open_depth(2));"##,
            show: |ui| {
                ui.add_space(4.0);
                with(ui, "tree", || Some("glowkit/crates/glowkit-core/theme.rs".to_string()), |ui, sel| {
                    let nodes = files();
                    center(ui, 600.0, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 30.0;
                            col(ui, 320.0, |ui| {
                                ui.add(TreeView::new(&nodes, sel).open_depth(3).width(320.0));
                            });
                            col(ui, 240.0, |ui| {
                                let t = Theme::of(ui);
                                caption(ui, "Selected");
                                ui.label(RichText::new(sel.clone().unwrap_or_else(|| "—".into())).monospace().size(12.0).color(t.foreground));
                            });
                        });
                    });
                });
            },
        },
        Demo {
            id: "code-block",
            name: "Code Block",
            category: C,
            description: "Syntax-highlighted code with line numbers, highlighted lines, a file header and a copy button.",
            covers: &["shadcn/ui: Code Block", "Kibo UI: Code Block", "Magic UI: Script Copy Button", "Aceternity: Code Block"],
            code: r##"ui.add(CodeBlock::new(source).title("settings.rs").highlight_lines(6..=6));"##,
            show: |ui| {
                ui.add_space(20.0);
                center(ui, 640.0, |ui| {
                    ui.add(CodeBlock::new(CODE).title("settings.rs").highlight_lines(6..=6).width(640.0));
                });
            },
        },
        Demo {
            id: "accordion",
            name: "Accordion",
            category: C,
            description: "Stacked sections that expand with animated height, a rotating chevron and hover underline.",
            covers: &["shadcn/ui: Accordion", "Animate UI: Accordion", "Origin UI: Accordion", "Motion Primitives: Accordion"],
            code: r##"Accordion::new("faq")
    .item_text("Is it accessible?", "Yes. It supports keyboard focus and screen readers.")
    .item_text("Is it styled?", "Yes. It comes with default styles that match the other components.")
    .item_text("Is it animated?", "Yes. Height and opacity animate when sections open.")
    .show(ui);"##,
            show: |ui| {
                ui.add_space(10.0);
                center(ui, 520.0, |ui| {
                    Accordion::new("faq-demo")
                        .item_text("Is it accessible?", "Yes. Every header is a focusable button and the content is announced when it expands.")
                        .item_text("Is it styled?", "Yes. It reads every color and radius from the active theme, so it matches the rest of glowkit in light and dark mode.")
                        .item_text("Is it animated?", "Yes. Sections open with an eased height and fade, and the chevron rotates in sync.")
                        .item("Can it hold anything?", |ui| {
                            ui.horizontal(|ui| {
                                ui.add(Badge::new("Widgets").variant(BadgeVariant::Secondary));
                                ui.add(Badge::new("Forms").variant(BadgeVariant::Secondary));
                                ui.add(Button::new("Even buttons").small().variant(ButtonVariant::Outline));
                            });
                        })
                        .width(520.0)
                        .show(ui);
                });
            },
        },
        Demo {
            id: "collapsible",
            name: "Collapsible",
            category: C,
            description: "A header with a toggle that reveals more rows with an animated height.",
            covers: &["shadcn/ui: Collapsible", "Animate UI: Collapsible", "Motion Primitives: Disclosure"],
            code: r##"Collapsible::new("@peduarte starred 3 repositories").show(ui, |ui| {
    for repo in ["@radix-ui/colors", "@stitches/react"] { ui.label(repo); }
});"##,
            show: |ui| {
                ui.add_space(50.0);
                center(ui, 360.0, |ui| {
                    let t = Theme::of(ui);
                    let row = |ui: &mut Ui, s: &str| {
                        let (r, _) = ui.allocate_exact_size(vec2(360.0, 38.0), egui::Sense::hover());
                        ui.painter().rect(r, egui::CornerRadius::same(8), egui::Color32::TRANSPARENT, t.border_stroke(), egui::StrokeKind::Inside);
                        ui.painter().text(r.left_center() + vec2(14.0, 0.0), egui::Align2::LEFT_CENTER, s, egui::FontId::monospace(13.0), t.foreground);
                    };
                    Collapsible::new("@peduarte starred 3 repositories").default_open(true).width(360.0).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        row(ui, "@radix-ui/primitives");
                        row(ui, "@radix-ui/colors");
                        row(ui, "@stitches/react");
                    });
                });
            },
        },
        Demo {
            id: "description-list",
            name: "Description List",
            category: C,
            description: "Label / value rows separated by hairlines, side by side or stacked, for detail pages.",
            covers: &["Tailwind UI: Description lists", "Origin UI: Key-value", "Kibo UI: Description List"],
            code: r##"ui.add(DescriptionList::new(&[
    ("Full name", "Ada Lovelace"),
    ("Role", "Lead engineer"),
    ("Email", "ada@analytical.io"),
]));"##,
            show: |ui| {
                ui.add_space(30.0);
                center(ui, 540.0, |ui| {
                    crate::inputs::demos::title(ui, "Applicant information", "Personal details and application.");
                    ui.add_space(8.0);
                    ui.add(DescriptionList::new(&[("Full name", "Ada Lovelace"), ("Application for", "Lead engine engineer"), ("Email address", "ada@analytical.io"), ("Salary expectation", "$120,000"), ("About", "Wrote the first published algorithm intended for a machine; enjoys poetry, mathematics and long walks.")]).width(540.0));
                });
            },
        },
        Demo {
            id: "aspect-ratio",
            name: "Aspect Ratio",
            category: C,
            description: "A container that locks content to a ratio (16:9, 1:1, 4:3) and clips it to rounded corners, for media, maps and embeds.",
            covers: &["shadcn/ui: Aspect Ratio", "Radix UI: Aspect Ratio", "Bencho: Aspect ratio"],
            code: r##"AspectRatio::new(16.0 / 9.0).max_width(420.0).show(ui, |ui| {
    let r = ui.max_rect();
    // paint an image, video frame or map into r
});"##,
            show: |ui| {
                ui.add_space(24.0);
                let t = Theme::of(ui);
                let time = glowkit_core::time(ui);
                glowkit_core::animate(ui);
                center(ui, 820.0, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 20.0;
                        col(ui, 400.0, |ui| {
                            AspectRatio::video().show(ui, |ui| {
                                let r = ui.max_rect();
                                let p = ui.painter();
                                let shift = (time * 0.15).sin() * 20.0;
                                glowkit_core::paint::gradient_rect(p, r, 0.0, 120.0 + shift, &[t.gradient[0], t.gradient[1], t.gradient[2]]);
                                // hills
                                for (k, a) in [(0.0f32, 0.35f32), (1.0, 0.55)] {
                                    let pts: Vec<egui::Pos2> = (0..=40)
                                        .map(|i| {
                                            let x = i as f32 / 40.0;
                                            let y = 0.62 + k * 0.12 + 0.08 * ((x * 5.0 + k * 2.0 + time * 0.2).sin());
                                            egui::pos2(r.min.x + x * r.width(), r.min.y + y * r.height())
                                        })
                                        .collect();
                                    glowkit_core::paint::area_fill(p, &pts, r.max.y, egui::Color32::from_black_alpha((a * 255.0) as u8), egui::Color32::from_black_alpha((a * 255.0) as u8));
                                }
                                let c = r.center();
                                p.circle_filled(c, 26.0, egui::Color32::from_white_alpha(230));
                                p.add(egui::Shape::convex_polygon(vec![c + vec2(-7.0, -11.0), c + vec2(12.0, 0.0), c + vec2(-7.0, 11.0)], egui::Color32::from_rgb(20, 20, 24), egui::Stroke::NONE));
                                let bar = egui::Rect::from_min_max(egui::pos2(r.min.x + 16.0, r.max.y - 18.0), egui::pos2(r.max.x - 16.0, r.max.y - 14.0));
                                p.rect_filled(bar, 2.0, egui::Color32::from_white_alpha(70));
                                let prog = (time * 0.05).fract();
                                p.rect_filled(egui::Rect::from_min_size(bar.min, vec2(bar.width() * prog, bar.height())), 2.0, egui::Color32::WHITE);
                                p.text(egui::pos2(r.min.x + 16.0, r.min.y + 14.0), egui::Align2::LEFT_TOP, "Northern Lights · 4K", egui::FontId::proportional(13.0), egui::Color32::WHITE);
                            });
                            ui.add_space(6.0);
                            caption(ui, "16 : 9 video");
                        });
                        col(ui, 200.0, |ui| {
                            AspectRatio::square().show(ui, |ui| {
                                let r = ui.max_rect();
                                let p = ui.painter();
                                p.rect_filled(r, 0.0, if t.dark { glowkit_core::hex("#18181B") } else { glowkit_core::hex("#F4F4F5") });
                                for i in 0..6 {
                                    let rr = 18.0 + i as f32 * 14.0;
                                    p.circle_stroke(r.center(), rr, egui::Stroke::new(6.0, glowkit_core::color::with_alpha(t.chart[i % 6], 0.85)));
                                }
                                glowkit_core::paint::glow(p, r.center(), 70.0, glowkit_core::color::with_alpha(t.accent, 0.4));
                                p.circle_filled(r.center(), 8.0, t.foreground);
                            });
                            ui.add_space(6.0);
                            caption(ui, "1 : 1 album art");
                        });
                        col(ui, 180.0, |ui| {
                            AspectRatio::new(3.0 / 4.0).show(ui, |ui| {
                                let r = ui.max_rect();
                                let p = ui.painter();
                                p.rect_filled(r, 0.0, if t.dark { glowkit_core::hex("#0F172A") } else { glowkit_core::hex("#E0F2FE") });
                                let grid = glowkit_core::color::with_alpha(t.muted_foreground, 0.18);
                                for i in 0..12 {
                                    let x = r.min.x + i as f32 * 18.0;
                                    p.line_segment([egui::pos2(x, r.min.y), egui::pos2(x, r.max.y)], egui::Stroke::new(1.0, grid));
                                    let y = r.min.y + i as f32 * 22.0;
                                    p.line_segment([egui::pos2(r.min.x, y), egui::pos2(r.max.x, y)], egui::Stroke::new(1.0, grid));
                                }
                                let road: Vec<egui::Pos2> = (0..=30).map(|i| { let y = i as f32 / 30.0; egui::pos2(r.min.x + r.width() * (0.3 + 0.25 * (y * 4.0).sin()), r.min.y + y * r.height()) }).collect();
                                p.add(egui::Shape::line(road, egui::Stroke::new(5.0, glowkit_core::color::with_alpha(t.warning, 0.8))));
                                let pin = r.min + vec2(r.width() * 0.62, r.height() * 0.42);
                                glowkit_core::paint::glow(p, pin, 30.0 + 6.0 * (time * 3.0).sin(), glowkit_core::color::with_alpha(t.destructive, 0.5));
                                p.circle_filled(pin, 7.0, t.destructive);
                                p.circle_filled(pin, 2.5, egui::Color32::WHITE);
                            });
                            ui.add_space(6.0);
                            caption(ui, "3 : 4 map");
                        });
                    });
                });
            },
        },
        Demo {
            id: "item",
            name: "Item",
            category: C,
            description: "Flexible list rows with an icon, avatar or thumbnail, a title and description, and trailing actions, in default, outline and muted styles.",
            covers: &["shadcn/ui: Item", "Origin UI: List items"],
            code: r##"Item::new("Two-factor authentication")
    .description("Verify via email or phone number.")
    .icon(UiIcon::Lock)
    .variant(ItemVariant::Outline)
    .actions(|ui| { ui.add(Button::new("Enable").small()); })
    .show(ui);"##,
            show: |ui| {
                ui.add_space(16.0);
                with(ui, "item_demo", || (1usize, false), |ui, (sel, enabled)| {
                    center(ui, 860.0, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 28.0;
                            col(ui, 420.0, |ui| {
                                ui.spacing_mut().item_spacing.y = 10.0;
                                caption(ui, "Outline · Muted · Small · Default");
                                let en = *enabled;
                                Item::new("Two-factor authentication")
                                    .description("Verify via email or phone number.")
                                    .media(ItemMedia::TintedIcon(UiIcon::Lock.into(), Theme::of(ui).accent))
                                    .variant(ItemVariant::Outline)
                                    .actions_width(96.0)
                                    .actions(|ui| {
                                        if ui.add(Button::new(if en { "Enabled" } else { "Enable" }).small().variant(if en { ButtonVariant::Secondary } else { ButtonVariant::Default }).icon(if en { UiIcon::Check } else { UiIcon::Plus })).clicked() {
                                            *enabled = !en;
                                        }
                                    })
                                    .show(ui);
                                Item::new("Your profile has been verified.")
                                    .media(ItemMedia::TintedIcon(UiIcon::CheckCircle.into(), Theme::of(ui).success))
                                    .variant(ItemVariant::Muted)
                                    .small()
                                    .chevron(true)
                                    .show(ui);
                                Item::new("Usage report.pdf")
                                    .description("2.4 MB · Updated 3 days ago")
                                    .icon(UiIcon::File)
                                    .variant(ItemVariant::Outline)
                                    .small()
                                    .actions_width(76.0)
                                    .actions(|ui| {
                                        ui.add(Button::icon_only(UiIcon::MoreHorizontal).small().variant(ButtonVariant::Ghost));
                                        ui.add(Button::icon_only(UiIcon::Download).small().variant(ButtonVariant::Ghost));
                                    })
                                    .show(ui);
                                Item::new("Billing & invoices")
                                    .description("Default variant: no surface until hovered.")
                                    .icon(UiIcon::Dollar)
                                    .small()
                                    .clickable(true)
                                    .actions_width(70.0)
                                    .actions(|ui| {
                                        ui.add(Badge::new("3 due").variant(BadgeVariant::Warning).pill(true));
                                    })
                                    .show(ui);
                            });
                            col(ui, 400.0, |ui| {
                                caption(ui, "Group with avatars & thumbnails");
                                let t = Theme::of(ui);
                                egui::Frame::new().fill(t.card).stroke(t.border_stroke()).corner_radius(t.corner()).inner_margin(egui::Margin::same(6)).show(ui, |ui| {
                                    ui.spacing_mut().item_spacing.y = 2.0;
                                    let people = [("Ada Lovelace", "ada@analytical.io"), ("Alan Turing", "alan@bletchley.uk"), ("Grace Hopper", "grace@cobol.dev")];
                                    for (i, (n, e)) in people.iter().enumerate() {
                                        if Item::new(*n).description(*e).media(ItemMedia::Avatar(n.to_string())).small().chevron(true).selected(*sel == i).show(ui).clicked() {
                                            *sel = i;
                                        }
                                    }
                                    ui.add(Separator::horizontal().margin(4.0));
                                    for (i, (n, d)) in [("Midnight Drive", "Synthwave · 3:42"), ("Golden Hour", "Lo-fi · 2:58")].iter().enumerate() {
                                        Item::new(*n).description(*d).media(ItemMedia::Image(i as u32 + 2)).small().actions_width(40.0).actions(|ui| {
                                            ui.add(Button::icon_only(UiIcon::Heart).small().variant(ButtonVariant::Ghost));
                                        }).show(ui);
                                    }
                                });
                            });
                        });
                    });
                });
            },
        },
        Demo {
            id: "resizable",
            name: "Resizable",
            category: C,
            description: "Horizontal and vertical split panels with draggable grip handles, min/max sizes and nested groups. Double-click a handle to even it out.",
            covers: &["shadcn/ui: Resizable", "react-resizable-panels: PanelGroup"],
            code: r##"ResizablePanels::horizontal(&mut sizes).height(300.0).min(0.15).show(ui, |ui, i| {
    if i == 1 {
        ResizablePanels::vertical(&mut inner).frame(false).show(ui, |ui, j| { /* ... */ });
    }
});"##,
            show: |ui| {
                ui.add_space(16.0);
                with(ui, "resizable_demo", || ([0.26f32, 0.74], [0.62f32, 0.38], [0.5f32, 0.5]), |ui, (outer, inner, side)| {
                    center(ui, 860.0, |ui| {
                        let t = Theme::of(ui);
                        let label = |ui: &mut Ui, title: &str, pct: f32| {
                            let r = ui.max_rect();
                            ui.painter().text(r.center() - vec2(0.0, 9.0), egui::Align2::CENTER_CENTER, title, egui::FontId::proportional(14.5), t.foreground);
                            ui.painter().text(r.center() + vec2(0.0, 11.0), egui::Align2::CENTER_CENTER, format!("{:.0}%", pct * 100.0), egui::FontId::monospace(11.5), t.muted_foreground);
                        };
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 20.0;
                            col(ui, 560.0, |ui| {
                                caption(ui, "Nested groups");
                                let (o, inn) = (*outer, *inner);
                                ResizablePanels::horizontal(outer).height(300.0).min(0.18).max(0.6).id_salt("outer").show(ui, |ui, i| {
                                    if i == 0 {
                                        let r = ui.max_rect();
                                        ui.painter().rect_filled(r, 0.0, glowkit_core::color::with_alpha(t.muted, 0.5));
                                        let mut y = r.min.y + 18.0;
                                        for (k, name) in ["Inbox", "Drafts", "Sent", "Archive"].iter().enumerate() {
                                            let rr = egui::Rect::from_min_size(egui::pos2(r.min.x + 10.0, y - 4.0), vec2(r.width() - 20.0, 26.0));
                                            if k == 0 { ui.painter().rect_filled(rr, 6.0, glowkit_core::color::with_alpha(t.accent, 0.14)); }
                                            ui.painter().text(egui::pos2(r.min.x + 22.0, y + 9.0), egui::Align2::LEFT_CENTER, name, egui::FontId::proportional(13.0), if k == 0 { t.accent } else { t.muted_foreground });
                                            y += 32.0;
                                        }
                                        ui.painter().text(egui::pos2(r.center().x, r.max.y - 16.0), egui::Align2::CENTER_CENTER, format!("{:.0}%", o[0] * 100.0), egui::FontId::monospace(11.5), t.muted_foreground);
                                    } else {
                                        ResizablePanels::vertical(inner).frame(false).min(0.2).id_salt("inner").show(ui, |ui, j| {
                                            label(ui, if j == 0 { "Content" } else { "Terminal" }, inn[j]);
                                        });
                                    }
                                });
                            });
                            col(ui, 280.0, |ui| {
                                caption(ui, "Vertical");
                                let sv = *side;
                                ResizablePanels::vertical(side).height(300.0).min_sizes(&[0.25, 0.25]).id_salt("side").show(ui, |ui, i| {
                                    label(ui, if i == 0 { "Header" } else { "Body" }, sv[i]);
                                });
                            });
                        });
                    });
                });
            },
        },
        Demo {
            id: "scroll-area",
            name: "Scroll Area",
            category: C,
            description: "A themed scroll container with a thin overlay scrollbar that fades in on hover or scroll, widens under the pointer and can be dragged.",
            covers: &["shadcn/ui: Scroll Area", "Radix UI: Scroll Area"],
            code: r##"StyledScrollArea::vertical().max_height(280.0).show(ui, |ui| {
    for tag in &tags {
        ui.label(tag);
    }
});"##,
            show: |ui| {
                ui.add_space(16.0);
                let t = Theme::of(ui);
                let time = glowkit_core::time(ui);
                center(ui, 820.0, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 24.0;
                        col(ui, 240.0, |ui| {
                            caption(ui, "Vertical · auto-hide");
                            egui::Frame::new().fill(t.card).stroke(t.border_stroke()).corner_radius(t.corner()).inner_margin(egui::Margin::same(1)).show(ui, |ui| {
                                ui.set_width(240.0);
                                StyledScrollArea::vertical().max_height(290.0).fade_edges(t.card).id_salt("tags").show(ui, |ui| {
                                    // gentle autoplay so the bar is visible in previews
                                    let cyc = time % 8.0;
                                    if cyc < 3.0 {
                                        ui.scroll_with_delta(vec2(0.0, -1.2));
                                        glowkit_core::animate(ui);
                                    } else if (4.0..7.0).contains(&cyc) {
                                        ui.scroll_with_delta(vec2(0.0, 1.2));
                                        glowkit_core::animate(ui);
                                    } else {
                                        glowkit_core::animate(ui);
                                    }
                                    ui.spacing_mut().item_spacing.y = 0.0;
                                    let w = ui.available_width();
                                    let (hr, _) = ui.allocate_exact_size(vec2(w, 44.0), egui::Sense::hover());
                                    ui.painter().text(egui::pos2(hr.min.x + 16.0, hr.center().y + 2.0), egui::Align2::LEFT_CENTER, "Tags", egui::FontId::proportional(14.5), t.foreground);
                                    for i in (0..40).rev() {
                                        let (r, _) = ui.allocate_exact_size(vec2(w, 36.0), egui::Sense::hover());
                                        ui.painter().text(egui::pos2(r.min.x + 16.0, r.center().y), egui::Align2::LEFT_CENTER, format!("v1.2.0-beta.{i}"), egui::FontId::proportional(13.0), t.foreground);
                                        ui.painter().line_segment([egui::pos2(r.min.x + 16.0, r.max.y), egui::pos2(r.max.x - 16.0, r.max.y)], egui::Stroke::new(1.0, t.border));
                                    }
                                });
                            });
                        });
                        col(ui, 540.0, |ui| {
                            caption(ui, "Horizontal · always visible");
                            egui::Frame::new().fill(t.card).stroke(t.border_stroke()).corner_radius(t.corner()).inner_margin(egui::Margin::same(1)).show(ui, |ui| {
                                ui.set_width(540.0);
                                StyledScrollArea::horizontal().scrollbar(ScrollbarMode::Always).fade_edges(t.card).id_salt("art").show(ui, |ui| {
                                    ui.add_space(14.0);
                                    ui.horizontal(|ui| {
                                        ui.add_space(14.0);
                                        ui.spacing_mut().item_spacing.x = 14.0;
                                        let names = ["Ornella Binni", "Tom Byrom", "Vladimir Malyav", "Mira Okafor", "Jun Park", "Lena Voss"];
                                        for (i, n) in names.iter().enumerate() {
                                            ui.vertical(|ui| {
                                                AspectRatio::new(3.0 / 4.0).width(150.0).surface(t.card).show(ui, |ui| {
                                                    let r = ui.max_rect();
                                                    let h = i as f32 * 53.0 + 200.0;
                                                    glowkit_core::paint::gradient_rect(ui.painter(), r, 0.0, 150.0, &[glowkit_core::hsv(h, 0.55, 0.9), glowkit_core::hsv(h + 60.0, 0.7, 0.55)]);
                                                    glowkit_core::paint::glow(ui.painter(), r.min + vec2(r.width() * 0.7, r.height() * 0.3), 50.0, egui::Color32::from_white_alpha(70));
                                                });
                                                ui.label(egui::RichText::new(format!("Photo by {n}")).size(12.0).color(t.muted_foreground));
                                            });
                                        }
                                        ui.add_space(4.0);
                                    });
                                    ui.add_space(16.0);
                                });
                            });
                        });
                    });
                });
            },
        },
    ]
}
