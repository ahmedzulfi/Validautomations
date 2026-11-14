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
    ]
}
