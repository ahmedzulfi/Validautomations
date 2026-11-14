use super::*;
use egui::{pos2, vec2, Align, Layout, Rect, Sense, Ui, UiBuilder, Vec2};
use glowkit_core::demo::{Category, Demo};
use glowkit_effects::ControlSlider;

const C: Category = Category::MicroInteractions;

fn mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, init: impl FnOnce() -> T) -> T {
    glowkit_core::state(ui, ui.scope_id().with(key), init)
}

fn set_mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, v: T) {
    glowkit_core::set_state(ui, ui.scope_id().with(key), v);
}

/// Center whatever `add` lays out (left to right) inside a full-width box at least `height` tall.
fn centered(ui: &mut Ui, key: &str, height: f32, add: impl FnOnce(&mut Ui)) {
    let id = ui.scope_id().with(("beui_center", key));
    let size: Vec2 = glowkit_core::state(ui, id, || Vec2::ZERO);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height.max(size.y)), Sense::hover());
    let inner = Rect::from_center_size(rect.center(), size);
    let r = ui.scope_builder(
        UiBuilder::new().max_rect(Rect::from_min_max(inner.min, rect.max)).layout(Layout::left_to_right(Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing = vec2(28.0, 18.0);
            add(ui);
            ui.min_rect().size()
        },
    );
    if (r.inner - size).length() > 0.5 {
        glowkit_core::set_state(ui, id, r.inner);
        ui.ctx().request_repaint();
    }
}

/// Run `f` inside `rect`, centered both ways using last frame's measured size.
fn centered_in(ui: &mut Ui, rect: Rect, key: &str, f: impl FnOnce(&mut Ui)) {
    let mid = ui.scope_id().with(("beui_centered_in", key));
    let measured: Vec2 = glowkit_core::state(ui, mid, || rect.size() * 0.5);
    let top = ((rect.height() - measured.y) / 2.0).max(0.0);
    let child = Rect::from_min_max(rect.min + vec2(0.0, top), rect.max);
    let size = ui
        .scope_builder(UiBuilder::new().max_rect(child).layout(Layout::top_down(Align::Center)), |ui| {
            f(ui);
            ui.min_rect().size()
        })
        .inner;
    if (size - measured).length() > 0.5 {
        glowkit_core::set_state(ui, mid, size);
        ui.ctx().request_repaint();
    }
}

/// The component on the left, a column of parameter sliders on the right (below when narrow).
fn bench<P>(ui: &mut Ui, height: f32, p: &mut P, component: impl FnOnce(&mut Ui, &mut P), controls: impl FnOnce(&mut Ui, &mut P)) {
    let w = ui.available_width();
    let wide = w > 620.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    if wide {
        let panel_w = 220.0;
        let left = Rect::from_min_size(rect.min, vec2(w - panel_w - 28.0, height));
        let right = Rect::from_min_max(pos2(rect.max.x - panel_w, rect.min.y), rect.max);
        centered_in(ui, left, "stage", |ui| component(ui, p));
        centered_in(ui, right, "panel", |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            controls(ui, p);
        });
    } else {
        centered_in(ui, rect, "stage", |ui| component(ui, p));
        ui.add_space(12.0);
        ui.spacing_mut().item_spacing.y = 8.0;
        controls(ui, p);
    }
}

/// A small segmented switch matching `ControlSlider`'s look; `value` holds the option index.
fn choice(ui: &mut Ui, label: &str, value: &mut f32, options: &[&str]) {
    use egui::{Align2, FontId};
    use glowkit_core::color::mix;
    let t = glowkit_core::Theme::of(ui);
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
    let font = FontId::proportional(13.0);
    ui.painter().text(pos2(rect.min.x + 2.0, rect.center().y), Align2::LEFT_CENTER, label, font.clone(), mix(t.muted_foreground, t.foreground, 0.55));
    let label_w = ui.painter().layout_no_wrap(label.into(), font.clone(), t.foreground).size().x;
    let text_w: Vec<f32> = options.iter().map(|o| ui.painter().layout_no_wrap((*o).into(), font.clone(), t.foreground).size().x).collect();
    let room = w - label_w - 20.0 - text_w.iter().sum::<f32>();
    let pad = (room / options.len() as f32).clamp(8.0, 20.0);
    let opt_w: Vec<f32> = text_w.iter().map(|x| x + pad).collect();
    let total: f32 = opt_w.iter().sum::<f32>() + 6.0;
    let seg = Rect::from_min_max(pos2(rect.max.x - total, rect.min.y), rect.max);
    let track = mix(t.muted, t.foreground, if t.dark { 0.09 } else { 0.065 });
    ui.painter().rect_filled(seg, egui::CornerRadius::same(17), track);
    let sel = (*value as usize).min(options.len() - 1);
    let mut x = seg.min.x + 3.0;
    let mut sel_x = x;
    let mut rs = Vec::new();
    for (i, ow) in opt_w.iter().enumerate() {
        let r = Rect::from_min_size(pos2(x, seg.min.y + 3.0), vec2(*ow, seg.height() - 6.0));
        let resp = ui.interact(r, ui.scope_id().with(("beui_choice", label, i)), Sense::click());
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            *value = i as f32;
        }
        if i == sel {
            sel_x = x;
        }
        rs.push(r);
        x += ow;
    }
    let kx = glowkit_core::anim::spring(ui, ui.scope_id().with(("beui_choice_x", label)), sel_x, 420.0, 30.0);
    let kw = glowkit_core::anim::spring(ui, ui.scope_id().with(("beui_choice_w", label)), opt_w[sel], 420.0, 30.0);
    let knob = Rect::from_min_size(pos2(kx, seg.min.y + 3.0), vec2(kw, seg.height() - 6.0));
    let raised = if t.dark { glowkit_core::color::shade(t.muted, 0.17) } else { t.card };
    ui.painter().rect_filled(knob, egui::CornerRadius::same(14), raised);
    for (i, r) in rs.iter().enumerate() {
        let c = if i == sel { t.foreground } else { t.muted_foreground };
        ui.painter().text(r.center(), Align2::CENTER_CENTER, options[i], font.clone(), c);
    }
}

pub(crate) fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "beui-arc-picker",
            name: "Arc Picker",
            category: C,
            description: "Options sit on the rim of a big wheel, turned to follow the curve; roll it and the chosen one lights up between springy brackets.",
            covers: &["beUI: Arc Picker"],
            code: r##"let mut sel = 4usize;
ui.add(ArcPicker::new(&mut sel, ["Dawn", "Morning", "Noon", "Golden hour", "Dusk"])
    .side(ArcSide::Right)
    .radius(260.0));"##,
            show: |ui| {
                let mut p = mem(ui, "p", || [0.0f32, 260.0, 11.0]);
                let mut sel: usize = mem(ui, "sel", || 4);
                bench(ui, 360.0, &mut p, |ui, p| {
                    let side = [ArcSide::Right, ArcSide::Left, ArcSide::Top, ArcSide::Bottom][p[0] as usize % 4];
                    ui.add(
                        ArcPicker::new(
                            &mut sel,
                            ["Twilight", "Late night", "Dawn", "Morning", "Midday", "Noon", "Afternoon", "Golden hour", "Sunset", "Blue hour", "Dusk", "Midnight"],
                        )
                        .side(side)
                        .radius(p[1])
                        .spacing(p[2])
                        .size(vec2(ui.available_width(), 360.0)),
                    );
                }, |ui, p| {
                    choice(ui, "Side", &mut p[0], &["Right", "Left", "Top", "Bottom"]);
                    ui.add(ControlSlider::new("Radius", &mut p[1], 140.0..=520.0).suffix("px"));
                    ui.add(ControlSlider::new("Spacing", &mut p[2], 5.0..=20.0).suffix("°"));
                });
                set_mem(ui, "p", p);
                set_mem(ui, "sel", sel);
            },
        },

        Demo {
            id: "beui-wheel-picker",
            name: "Wheel Picker",
            category: C,
            description: "An iOS-style drum picker: columns roll under the finger, rows tip away toward the rim and click into a raised middle band.",
            covers: &["beUI: Wheel Picker"],
            code: r##"let mut sel = [5usize, 8, 24];
ui.add(WheelPicker::new(&mut sel)
    .column(months, 128.0)
    .column(days, 56.0)
    .column(years, 80.0));"##,
            show: |ui| {
                const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
                let mut sel: [usize; 3] = mem(ui, "sel", || [5, 8, 24]);
                let mut p = mem(ui, "p", || [38.0f32, 17.0]);
                bench(ui, 360.0, &mut p, |ui, p| {
                    ui.vertical_centered(|ui| {
                        let t = glowkit_core::Theme::of(ui);
                        let label = format!("{} {}, {}", MONTHS[sel[0]], sel[1] + 1, 1980 + sel[2]);
                        let mut job = egui::text::LayoutJob::default();
                        job.append("Born  ", 0.0, egui::TextFormat::simple(egui::FontId::proportional(14.0), t.muted_foreground));
                        job.append(&label, 0.0, egui::TextFormat::simple(egui::FontId::proportional(14.0), t.foreground));
                        ui.label(job);
                        ui.add_space(10.0);
                        ui.add(
                            WheelPicker::new(&mut sel)
                                .column(MONTHS, 128.0)
                                .column((1..=31).map(|d| d.to_string()), 56.0)
                                .column((1980..=2030).map(|y| y.to_string()), 80.0)
                                .row_height(p[0])
                                .font_size(p[1]),
                        );
                    });
                }, |ui, p| {
                    ui.add(ControlSlider::new("Row height", &mut p[0], 28.0..=48.0).suffix("px"));
                    ui.add(ControlSlider::new("Font", &mut p[1], 12.0..=24.0).suffix("px"));
                });
                set_mem(ui, "sel", sel);
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "beui-sortable-stack",
            name: "Sortable Stack",
            category: C,
            description: "Grab a task by its grip: it lifts off the stack while the others glide out of its way, and settles into its new slot when dropped.",
            covers: &["beUI: Sortable Stack"],
            code: r##"ui.add(SortableStack::new(&mut tasks).heading("In the studio", "Make room for what matters."));"##,
            show: |ui| {
                let mut tasks: Vec<SortableTask> = mem(ui, "tasks", || {
                    use glowkit_core::color::hex;
                    vec![
                        SortableTask::new(BeIcon::Doc, hex("#F59E0B"), "Collect references", "Find the right direction"),
                        SortableTask::new(BeIcon::Palette, hex("#F43F5E"), "Explore the visual system", "Type, color, and composition"),
                        SortableTask::new(BeIcon::Layers, hex("#3B82F6"), "Build the first prototype", "Make the interaction tangible"),
                        SortableTask::new(BeIcon::Rocket, hex("#10B981"), "Ship something useful", "Polish the details and release"),
                    ]
                });
                centered(ui, "stack", 380.0, |ui| {
                    ui.add(SortableStack::new(&mut tasks).width(440.0).heading("In the studio", "Make room for what matters."));
                });
                set_mem(ui, "tasks", tasks);
            },
        },
        Demo {
            id: "beui-card-dealer",
            name: "Card Dealer",
            category: C,
            description: "A deck deals cards that arc into a fanned hand, flipping face up in flight; sort, shuffle and collect send every card gliding on its own springs.",
            covers: &["beUI: Card Dealer"],
            code: r##"ui.add(CardDealer::new().max_hand(7).spread(8.0).card_width(74.0));"##,
            show: |ui| {
                let mut p = mem(ui, "p", || [8.0f32, 7.0, 74.0]);
                bench(ui, 360.0, &mut p, |ui, p| {
                    ui.add(CardDealer::new().spread(p[0]).max_hand(p[1] as usize).card_width(p[2]).size(vec2(ui.available_width(), 350.0)));
                }, |ui, p| {
                    ui.add(ControlSlider::new("Spread", &mut p[0], 0.0..=16.0).suffix("°"));
                    ui.add(ControlSlider::new("Hand", &mut p[1], 3.0..=10.0));
                    ui.add(ControlSlider::new("Card", &mut p[2], 50.0..=96.0).suffix("px"));
                });
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "beui-bloom-menu",
            name: "Bloom Menu",
            category: C,
            description: "A Create pill blooms into a grid of actions: its box morphs open, the label glides into the title, the plus turns into the close cross and tiles pop in.",
            covers: &["beUI: Bloom Menu"],
            code: r##"let picked = BloomMenu::new("Create")
    .item(BeIcon::Doc, "Doc")
    .item(BeIcon::Grid, "Board")
    .item(BeIcon::Table, "Table")
    .item(BeIcon::Folder, "Folder")
    .item(BeIcon::Bell, "Reminder")
    .item(BeIcon::Link, "Link")
    .show(ui);"##,
            show: |ui| {
                let mut p = mem(ui, "p", || [0.5f32, 18.0, 2.0]);
                bench(ui, 340.0, &mut p, |ui, p| {
                    ui.vertical_centered(|ui| {
                        let _ = BloomMenu::new("Create")
                            .item(BeIcon::Doc, "Doc")
                            .item(BeIcon::Grid, "Board")
                            .item(BeIcon::Table, "Table")
                            .item(BeIcon::Folder, "Folder")
                            .item(BeIcon::Bell, "Reminder")
                            .item(BeIcon::Link, "Link")
                            .bounce(p[0])
                            .corner(p[1])
                            .columns(p[2] as usize + 1)
                            .show(ui);
                    });
                }, |ui, p| {
                    ui.add(ControlSlider::new("Bounce", &mut p[0], 0.0..=1.0).decimals(2));
                    ui.add(ControlSlider::new("Corner", &mut p[1], 4.0..=32.0).suffix("px"));
                    choice(ui, "Columns", &mut p[2], &["1", "2", "3"]);
                });
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "beui-cylinder-carousel",
            name: "Cylinder Carousel",
            category: Category::ThreeD,
            description: "Round artworks wrapped around a drum: drag to roll it and they swing round, swelling, narrowing and dimming with the angle before snapping into place.",
            covers: &["beUI: Cylinder Carousel"],
            code: r##"ui.add(CylinderCarousel::new().mode(CylinderMode::Concave).count(12).item_size(84.0));"##,
            show: |ui| {
                let mut p = mem(ui, "p", || [0.0f32, 12.0, 90.0]);
                bench(ui, 340.0, &mut p, |ui, p| {
                    let mode = if p[0] < 0.5 { CylinderMode::Concave } else { CylinderMode::Convex };
                    ui.add(CylinderCarousel::new().mode(mode).count(p[1] as usize).item_size(p[2]).size(vec2(ui.available_width(), 320.0)));
                }, |ui, p| {
                    choice(ui, "Bend", &mut p[0], &["Concave", "Convex"]);
                    ui.add(ControlSlider::new("Items", &mut p[1], 6.0..=20.0));
                    ui.add(ControlSlider::new("Size", &mut p[2], 50.0..=130.0).suffix("px"));
                });
                set_mem(ui, "p", p);
            },
        },

        Demo {
            id: "beui-expandable-action-bar",
            name: "Expandable Action Bar",
            category: C,
            description: "An icon toolbar that ripples open into labelled actions with keyboard hints; badges ride along and one highlight glides between actions.",
            covers: &["beUI: Expandable Action Bar"],
            code: r##"let clicked = ExpandableActionBar::new()
    .item(BeIcon::Send, "Send", "S")
    .item(BeIcon::Copy, "Copy", "C")
    .item(BeIcon::Download, "Export", "E")
    .item(BeIcon::Archive, "Archive", "")
    .badge_item(BeIcon::Bell, "Alerts", 3)
    .item(BeIcon::Settings, "Settings", "")
    .show(ui);"##,
            show: |ui| {
                let mut p = mem(ui, "p", || [0.03f32]);
                bench(ui, 300.0, &mut p, |ui, p| {
                    let _ = ExpandableActionBar::new()
                        .item(BeIcon::Send, "Send", "S")
                        .item(BeIcon::Copy, "Copy", "C")
                        .item(BeIcon::Download, "Export", "E")
                        .item(BeIcon::Archive, "Archive", "")
                        .badge_item(BeIcon::Bell, "Alerts", 3)
                        .item(BeIcon::Settings, "Settings", "")
                        .stagger(p[0])
                        .show(ui);
                }, |ui, p| {
                    ui.add(ControlSlider::new("Stagger", &mut p[0], 0.0..=0.12).decimals(2).suffix("s"));
                });
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "beui-expandable-control",
            name: "Expandable Control",
            category: C,
            description: "Round icon buttons that spring open into labelled pills, and chips that grow a remove cross when picked.",
            covers: &["beUI: Expandable Control"],
            code: r##"ui.add(ExpandableControl::new(&mut on, "Notifications").icon(BeIcon::Bell));
ui.add(ExpandableControl::new(&mut picked, "Rust").tint(true));"##,
            show: |ui| {
                let mut s: [bool; 6] = mem(ui, "s", || [false; 6]);
                centered(ui, "ctl", 300.0, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.spacing_mut().item_spacing.y = 18.0;
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            ui.add(ExpandableControl::new(&mut s[0], "Notifications").icon(BeIcon::Bell).autoplay(true));
                            ui.add(ExpandableControl::new(&mut s[1], "Bookmark").icon(BeIcon::Bookmark).autoplay(true));
                            ui.add(ExpandableControl::new(&mut s[2], "Share").icon(BeIcon::Share).tint(true).autoplay(true));
                        });
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            ui.add(ExpandableControl::new(&mut s[3], "React").height(34.0).autoplay(true));
                            ui.add(ExpandableControl::new(&mut s[4], "Rust").height(34.0).tint(true).autoplay(true));
                            ui.add(ExpandableControl::new(&mut s[5], "WebAssembly").height(34.0).autoplay(true));
                        });
                    });
                });
                set_mem(ui, "s", s);
            },
        },
        Demo {
            id: "beui-morphing-search",
            name: "Morphing Search",
            category: C,
            description: "A frosted search bar floating over a gallery rises into a panel of recent searches when focused, rows sliding in under a gliding highlight.",
            covers: &["beUI: Morphing Search"],
            code: r##"MorphingSearch::new(&mut query)
    .tags(&mut tags)
    .recent(["dashboards", "empty states", "pricing pages"])
    .show(ui);"##,
            show: |ui| {
                let mut q: String = mem(ui, "q", String::new);
                let mut tags: Vec<String> = mem(ui, "tags", || vec!["onboarding".to_string()]);
                centered(ui, "search", 380.0, |ui| {
                    let t = glowkit_core::Theme::of(ui);
                    let w = 520.0f32.min(ui.available_width());
                    let (r, _) = ui.allocate_exact_size(vec2(w, 370.0), Sense::hover());
                    let p = ui.painter();
                    p.rect(r, egui::CornerRadius::same(24), egui::Color32::TRANSPARENT, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                    let tiles = [["#F43F5E", "#F59E0B"], ["#0EA5E9", "#6366F1"], ["#10B981", "#14B8A6"], ["#EAB308", "#F97316"], ["#8B5CF6", "#EC4899"], ["#2DD4BF", "#3B82F6"]];
                    let tw = (w - 16.0 * 2.0 - 12.0 * 2.0) / 3.0;
                    let th = (370.0 - 32.0 - 12.0) / 2.0;
                    for (i, c) in tiles.iter().enumerate() {
                        let tr = Rect::from_min_size(r.min + vec2(16.0 + (i % 3) as f32 * (tw + 12.0), 16.0 + (i / 3) as f32 * (th + 12.0)), vec2(tw, th));
                        let cols = [glowkit_core::color::with_alpha(glowkit_core::hex(c[0]), 0.55), glowkit_core::color::with_alpha(glowkit_core::hex(c[1]), 0.4)];
                        glowkit_core::paint::gradient_rect(p, tr, 16.0, 135.0, &cols);
                    }
                    let bw = w - 120.0;
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos2(r.center().x - bw / 2.0, r.max.y - 76.0), vec2(bw, 50.0))));
                    let _ = MorphingSearch::new(&mut q).tags(&mut tags).width(bw).recent(["dashboards", "empty states", "pricing pages", "onboarding flows"]).show(&mut child);
                });
                set_mem(ui, "q", q);
                set_mem(ui, "tags", tags);
            },
        },
        Demo {
            id: "beui-morphing-modal",
            name: "Morphing Modal",
            category: Category::Overlays,
            description: "A button that becomes its own sheet; picking a row re-measures the sheet, its height easing to the new page as the content slides across.",
            covers: &["beUI: Morphing Modal"],
            code: r##"let done = MorphingModal::new("Open wallet options")
    .option(BeIcon::Eye, "View private key", "Private key", "Anyone with it controls the wallet.", "Reveal", false)
    .option(BeIcon::Trash, "Remove wallet", "Remove this wallet?", "You can import it again later.", "Remove", true)
    .show(ui);"##,
            show: |ui| {
                let _ = MorphingModal::new("Open wallet options")
                    .option(BeIcon::Eye, "View private key", "Private key", "Anyone who sees your private key controls this wallet. Make sure nobody is looking at your screen.", "Reveal key", false)
                    .option(BeIcon::Shield, "View recovery phrase", "Recovery phrase", "Twelve words that restore this wallet on any device.", "Show phrase", false)
                    .option(BeIcon::Trash, "Remove wallet", "Remove this wallet?", "It disappears from this device. You can import it again later with your recovery phrase, but pending transfers will be cancelled.", "Remove", true)
                    .size(vec2(ui.available_width(), 380.0))
                    .show(ui);
            },
        },
        Demo {
            id: "beui-center-morph-modal",
            name: "Center Morph Modal",
            category: Category::Overlays,
            description: "A dialog that unfolds from the middle: the button flattens into a bright seam, stretches sideways, then opens along the crease as the content rises in.",
            covers: &["beUI: Center Morph Modal"],
            code: r##"let go = CenterMorphModal::new("Open modal")
    .eyebrow("glowkit Pro")
    .title("Ship the whole experience.")
    .body("Go beyond single components with animated sections and complete templates.")
    .bullet("Premium animated sections")
    .bullet("Complete app templates")
    .action("Explore Pro")
    .show(ui);"##,
            show: |ui| {
                let mut p = mem(ui, "p", || [0.12f32, 360.0]);
                bench(ui, 380.0, &mut p, |ui, p| {
                    let _ = CenterMorphModal::new("Open modal")
                        .eyebrow("glowkit Pro")
                        .title("Ship the whole experience.")
                        .body("Go beyond single components with premium animated sections and complete app templates.")
                        .bullet("Premium animated sections")
                        .bullet("Complete app templates")
                        .bullet("Editable source, no lock-in")
                        .action("Explore Pro")
                        .seam_delay(p[0])
                        .width(p[1])
                        .size(vec2(ui.available_width(), 380.0))
                        .show(ui);
                }, |ui, p| {
                    ui.add(ControlSlider::new("Seam hold", &mut p[0], 0.0..=0.5).decimals(2).suffix("s"));
                    ui.add(ControlSlider::new("Width", &mut p[1], 280.0..=440.0).suffix("px"));
                });
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "beui-shared-layout-bg",
            name: "Shared Layout Background",
            category: Category::Navigation,
            description: "One soft background slides and resizes between list rows or nav links under the pointer, instead of each item lighting up on its own.",
            covers: &["beUI: Shared Layout Background"],
            code: r##"let clicked = SharedLayoutBg::new()
    .item("Inbox", "12 unread threads, 3 mentions today.")
    .item("Drafts", "4 posts waiting for a final pass.")
    .show(ui);
SharedLayoutBg::new().horizontal(true).item("Home", "").item("Docs", "").show(ui);"##,
            show: |ui| {
                let mut p = mem(ui, "p", || [380.0f32]);
                bench(ui, 340.0, &mut p, |ui, p| {
                    ui.spacing_mut().item_spacing.y = 22.0;
                    let _ = SharedLayoutBg::new()
                        .horizontal(true)
                        .item("Overview", "")
                        .item("Components", "")
                        .item("Blocks", "")
                        .item("Changelog", "")
                        .stiffness(p[0])
                        .show(ui);
                    let _ = SharedLayoutBg::new()
                        .item("Inbox", "12 unread threads, 3 mentions today.")
                        .item("Drafts", "4 posts waiting for a final pass.")
                        .item("Releases", "Last shipped 2 days ago, v0.4.1.")
                        .item("Billing", "Plan renews on the 1st of next month.")
                        .width(420.0)
                        .stiffness(p[0])
                        .show(ui);
                }, |ui, p| {
                    ui.add(ControlSlider::new("Stiffness", &mut p[0], 80.0..=900.0));
                });
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "beui-knockout-bracket",
            name: "Knockout Bracket",
            category: Category::Charts,
            description: "A round tournament draw: badges ring the outside and winners fly inward round by round until the champion lands glowing in the middle. Hover a team to trace its road.",
            covers: &["beUI: Knockout Bracket"],
            code: r##"ui.add(KnockoutBracket::new().seed(7).pace(0.85));"##,
            show: |ui| {
                let mut p = mem(ui, "p", || [3.0f32, 0.85]);
                bench(ui, 400.0, &mut p, |ui, p| {
                    ui.add(KnockoutBracket::new().seed(p[0] as u64).pace(p[1]).size(vec2(ui.available_width(), 400.0)));
                }, |ui, p| {
                    ui.add(ControlSlider::new("Seed", &mut p[0], 1.0..=20.0));
                    ui.add(ControlSlider::new("Pace", &mut p[1], 0.3..=2.0).decimals(2).suffix("s"));
                });
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "beui-availability-scheduler",
            name: "Availability Scheduler",
            category: Category::Inputs,
            description: "Weekly hours with springy rows: switching a day off folds its pickers into Unavailable, extra ranges grow the row, and time pickers spring open in place.",
            covers: &["beUI: Availability Scheduler"],
            code: r##"let mut week = DayAvailability::work_week();
ui.add(AvailabilityScheduler::new(&mut week).step(30));"##,
            show: |ui| {
                let mut week: Vec<DayAvailability> = mem(ui, "week", DayAvailability::work_week);
                centered(ui, "sched", 420.0, |ui| {
                    ui.add(AvailabilityScheduler::new(&mut week).width(580.0f32.min(ui.available_width())));
                });
                set_mem(ui, "week", week);
            },
        },

        Demo {
            id: "beui-project-folder",
            name: "Project Folder",
            category: Category::CardsAndBorders,
            description: "A folder with pastel pages peeking out: hover and they fan up on rippling springs, press and they slide out into a row as the frosted front sinks.",
            covers: &["beUI: Project Folder"],
            code: r##"ui.add(ProjectFolder::new("Brand direction").files(5).updated("Updated recently"));"##,
            show: |ui| {
                let mut p = mem(ui, "p", || [5.0f32, 1.0, 290.0]);
                bench(ui, 360.0, &mut p, |ui, p| {
                    ui.add(ProjectFolder::new("Brand direction").files(p[0] as usize).fan(p[1]).width(p[2]));
                }, |ui, p| {
                    ui.add(ControlSlider::new("Files", &mut p[0], 1.0..=7.0));
                    ui.add(ControlSlider::new("Fan", &mut p[1], 0.0..=2.0).decimals(1));
                    ui.add(ControlSlider::new("Width", &mut p[2], 220.0..=360.0).suffix("px"));
                });
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "beui-wallet-card",
            name: "Wallet Card",
            category: Category::CardsAndBorders,
            description: "A wallet card whose balance rolls like an odometer when it changes, folds into dots behind the eye toggle, and carries a gain or loss pill that flips with it.",
            covers: &["beUI: Wallet Card"],
            code: r##"let mut balance = 12_480.32;
ui.add(WalletCard::new(&mut balance).name("Main Wallet"));"##,
            show: |ui| {
                let mut bal: f64 = mem(ui, "bal", || 12_480.32);
                centered(ui, "wallet", 360.0, |ui| {
                    ui.add(WalletCard::new(&mut bal).name("Main Wallet"));
                });
                set_mem(ui, "bal", bal);
            },
        },
        Demo {
            id: "beui-prediction-market",
            name: "Prediction Market",
            category: Category::DataDisplay,
            description: "Market cards whose probability bars spring to new lengths and whose price buttons roll their percentages as the odds move; press a side to buy it.",
            covers: &["beUI: Prediction Market"],
            code: r##"ui.add(
    PredictionMarket::new("Harbor City vs. North Ridge", "Sunday · 7:30 PM · Basketball")
        .badge("H", egui::Color32::from_rgb(16, 120, 70))
        .outcome("Harbor", egui::Color32::from_rgb(52, 211, 153), 0.51)
        .outcome("Ridge", egui::Color32::from_rgb(251, 191, 36), 0.49),
);"##,
            show: |ui| {
                use glowkit_core::hex;
                centered(ui, "markets", 360.0, |ui| {
                    ui.add(
                        PredictionMarket::new("Harbor City vs. North Ridge", "Sunday · 7:30 PM · Basketball")
                            .badge("H", hex("#0F7A4A"))
                            .outcome("Harbor", hex("#34D399"), 0.51)
                            .outcome("Ridge", hex("#FBBF24"), 0.49)
                            .width(330.0),
                    );
                    ui.add(
                        PredictionMarket::new("Bay Rays vs. Summit Owls", "Mid 6th · MLB")
                            .badge("R", hex("#1E3A8A"))
                            .live(true)
                            .outcome("Rays", hex("#60A5FA"), 0.46)
                            .outcome("Owls", hex("#F43F5E"), 0.54)
                            .volume(458_000.0)
                            .width(330.0),
                    );
                });
            },
        },
        Demo {
            id: "beui-file-diff",
            name: "File Diff",
            category: Category::Ai,
            description: "An agent's file change: the header springs open into the diff, removed lines strike through and added lines type themselves in, then the check lights up.",
            covers: &["beUI: File Diff"],
            code: r##"ui.add(
    FileDiff::new("src/runner.rs")
        .context("pub fn run(job: &Job) -> Result<()> {")
        .removed("    job.start()?;")
        .added("    job.start_with_retry(3)?;")
        .added("    log::info!(\"started {}\", job.id);")
        .context("    Ok(())")
        .context("}"),
);"##,
            show: |ui| {
                let mut p = mem(ui, "p", || [60.0f32]);
                bench(ui, 340.0, &mut p, |ui, p| {
                    ui.add(
                        FileDiff::new("src/runner.rs")
                            .start_line(41)
                            .context("pub fn run(job: &Job) -> Result<()> {")
                            .context("    let cfg = job.config();")
                            .removed("    job.start()?;")
                            .added("    job.start_with_retry(cfg.retries)?;")
                            .added("    log::info!(\"started {}\", job.id);")
                            .context("    Ok(())")
                            .context("}")
                            .speed(p[0])
                            .width(500.0f32.min(ui.available_width())),
                    );
                }, |ui, p| {
                    ui.add(ControlSlider::new("Typing", &mut p[0], 15.0..=160.0).suffix(" cps"));
                });
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "beui-bouncy-accordion",
            name: "Bouncy Accordion",
            category: C,
            description: "Rows share one rounded block until one opens: it breaks away into its own card, gaps and corners springing around it as its text drops in with a bounce.",
            covers: &["beUI: Bouncy Accordion"],
            code: r##"ui.add(
    BouncyAccordion::new()
        .item(BeIcon::Doc, "Release brief", "Collect launch notes, owners and risks.")
        .item(BeIcon::Shield, "Launch checklist", "Everything that must be green first.")
        .bounce(0.5),
);"##,
            show: |ui| {
                let mut p = mem(ui, "p", || [0.5f32, 10.0, 22.0]);
                bench(ui, 420.0, &mut p, |ui, p| {
                    ui.add(
                        BouncyAccordion::new()
                            .item(BeIcon::Doc, "Release Brief", "Collect launch notes, owners and risks in one compact handoff before the release window opens.")
                            .item(BeIcon::Shield, "Launch Checklist", "Everything that has to be green before we flip the switch, with an owner next to each line.")
                            .item(BeIcon::Antenna, "Campaign Notes", "Messaging, channels and the order the announcements go out in.")
                            .item(BeIcon::Calendar, "Rollout Calendar", "Plan announcements, staging checks, reminders and quiet periods around the same timeline.")
                            .item(BeIcon::Cube, "Ship Build", "Tag, sign and upload the release candidate.")
                            .item(BeIcon::Folder, "Archive Assets", "Move the final files to long-term storage.")
                            .bounce(p[0])
                            .gap(p[1])
                            .radius(p[2]),
                    );
                }, |ui, p| {
                    ui.add(ControlSlider::new("Bounce", &mut p[0], 0.0..=1.0).decimals(2));
                    ui.add(ControlSlider::new("Gap", &mut p[1], 0.0..=24.0).suffix("px"));
                    ui.add(ControlSlider::new("Radius", &mut p[2], 6.0..=30.0).suffix("px"));
                });
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "beui-bounce-sidebar",
            name: "Bounce Sidebar",
            category: Category::Navigation,
            description: "Sidebar links with a dot that hops to the one you pick: it leaps out in an arc, stretches in flight and lands with a squash.",
            covers: &["beUI: Bounce Sidebar"],
            code: r##"let mut active = 1usize;
ui.add(BounceSidebar::new(&mut active, ["Overview", "Components", "Motion", "Templates", "Changelog"]));"##,
            show: |ui| {
                let mut p = mem(ui, "p", || [1.0f32, 42.0]);
                let mut active: usize = mem(ui, "a", || 1);
                bench(ui, 320.0, &mut p, |ui, p| {
                    ui.add(BounceSidebar::new(&mut active, ["Overview", "Components", "Motion", "Templates", "Changelog"]).hop(p[0]).row_height(p[1]));
                }, |ui, p| {
                    ui.add(ControlSlider::new("Hop", &mut p[0], 0.0..=2.0).decimals(1));
                    ui.add(ControlSlider::new("Row", &mut p[1], 30.0..=56.0).suffix("px"));
                });
                set_mem(ui, "p", p);
                set_mem(ui, "a", active);
            },
        },
        Demo {
            id: "beui-expanding-arrow-button",
            name: "Expanding Arrow Button",
            category: Category::Buttons,
            description: "A dark pill with a bright arrow block that swallows the button on hover, its dotted chevron turning into a marching train.",
            covers: &["beUI: Expanding Arrow Button"],
            code: r##"if ui.add(ExpandingArrowButton::new("Book a demo")).clicked() { /* go */ }
ui.add(ExpandingArrowButton::new("Get started").accent(egui::Color32::from_rgb(125, 211, 252)));"##,
            show: |ui| {
                use glowkit_core::hex;
                centered(ui, "arrows", 300.0, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.spacing_mut().item_spacing.y = 22.0;
                        ui.add(ExpandingArrowButton::new("Book a demo"));
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 18.0;
                            ui.add(ExpandingArrowButton::new("Get started").accent(hex("#7DD3FC")).height(48.0));
                            ui.add(ExpandingArrowButton::new("Join the beta").accent(hex("#F9A8D4")).height(48.0));
                        });
                    });
                });
            },
        },
        Demo {
            id: "beui-overflow-actions",
            name: "Overflow Actions",
            category: C,
            description: "A pill of actions with a round more button: press it and the hidden actions slide out one by one as the pill stretches and the dots fold into a cross.",
            covers: &["beUI: Overflow Actions"],
            code: r##"let clicked = OverflowActions::new()
    .action(BeIcon::Eye, "Preview")
    .action(BeIcon::Pin, "Pin")
    .more(BeIcon::Share, "Share")
    .more(BeIcon::Calendar, "Schedule")
    .more(BeIcon::Trash, "Delete")
    .show(ui);"##,
            show: |ui| {
                let mut p = mem(ui, "p", || [0.05f32]);
                bench(ui, 300.0, &mut p, |ui, p| {
                    let _ = OverflowActions::new()
                        .action(BeIcon::Eye, "Preview")
                        .action(BeIcon::Pin, "Pin")
                        .more(BeIcon::Share, "Share")
                        .more(BeIcon::Calendar, "Schedule")
                        .more(BeIcon::Trash, "Delete")
                        .stagger(p[0])
                        .show(ui);
                }, |ui, p| {
                    ui.add(ControlSlider::new("Stagger", &mut p[0], 0.0..=0.2).decimals(2).suffix("s"));
                });
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "beui-feedback-widget",
            name: "Feedback Widget",
            category: Category::Overlays,
            description: "A corner chat button that springs open into the feedback form, then shrinks into a thank-you pill with a drawn check before folding back.",
            covers: &["beUI: Feedback Widget"],
            code: r##"let mut draft = String::new();
if let Some(msg) = FeedbackWidget::new(&mut draft).show(ui) {
    // send msg
}"##,
            show: |ui| {
                let mut draft: String = mem(ui, "d", String::new);
                centered(ui, "fb", 360.0, |ui| {
                    let _ = FeedbackWidget::new(&mut draft).size(vec2(460.0f32.min(ui.available_width()), 320.0)).show(ui);
                });
                set_mem(ui, "d", draft);
            },
        },
        Demo {
            id: "beui-swipeable-list",
            name: "Swipeable List",
            category: C,
            description: "Swipe a row left to slide it over snooze and delete; fling it and it flies off while the rows below spring up to close the gap.",
            covers: &["beUI: Swipeable List"],
            code: r##"ui.add(
    SwipeableList::new("Priority queue")
        .row(BeIcon::Doc, "Launch brief", "Finalize the announcement copy", "9:41")
        .row(BeIcon::Mail, "Client feedback", "Three comments need a response", "11:08"),
);"##,
            show: |ui| {
                centered(ui, "swipe", 420.0, |ui| {
                    ui.add(
                        SwipeableList::new("Priority queue")
                            .row(BeIcon::Doc, "Launch brief", "Finalize the announcement copy", "9:41")
                            .row(BeIcon::Mail, "Client feedback", "Three comments need a response", "11:08")
                            .row(BeIcon::User, "Design review", "Check spacing before handoff", "13:20")
                            .row(BeIcon::Flag, "Flagged run", "Retry queue has one failed job", "Now"),
                    );
                });
            },
        },
        Demo {
            id: "beui-notification-stack",
            name: "Notification Stack",
            category: Category::Overlays,
            description: "Notifications stacked like a deck over a count badge; press and they fan down into a full list as the container grows around them.",
            covers: &["beUI: Notification Stack"],
            code: r##"ui.add(
    NotificationStack::new()
        .notice("Orders import failed", "42s · TimeoutError at Step 2", 2)
        .notice("SLA breach", "2m 11s · Data enrichment", 0)
        .notice("Product sync auto-fixed", "5m · 404 on GET /products", 0),
);"##,
            show: |ui| {
                centered(ui, "notes", 330.0, |ui| {
                    ui.add(
                        NotificationStack::new()
                            .notice("Orders import failed", "42s · TimeoutError at Step 2", 2)
                            .notice("SLA breach", "2m 11s · Data enrichment", 0)
                            .notice("Product sync auto-fixed", "5m · 404 on GET /products", 0),
                    );
                });
            },
        },
        Demo {
            id: "beui-adaptive-stepper",
            name: "Adaptive Stepper",
            category: Category::Inputs,
            description: "A number stepper whose digits roll in the direction you count, whose pill grows with the digits, and whose dead button folds away at the ends of the range.",
            covers: &["beUI: Adaptive Stepper"],
            code: r##"let mut qty = 2i64;
ui.add(AdaptiveStepper::new(&mut qty).range(0, 120));"##,
            show: |ui| {
                let mut p = mem(ui, "p", || [120.0f32, 48.0]);
                let mut v: i64 = mem(ui, "v", || 2);
                bench(ui, 300.0, &mut p, |ui, p| {
                    ui.add(AdaptiveStepper::new(&mut v).range(0, p[0] as i64).height(p[1]));
                }, |ui, p| {
                    ui.add(ControlSlider::new("Max", &mut p[0], 5.0..=2000.0));
                    ui.add(ControlSlider::new("Height", &mut p[1], 32.0..=64.0).suffix("px"));
                });
                set_mem(ui, "p", p);
                set_mem(ui, "v", v);
            },
        },
    ]
}
