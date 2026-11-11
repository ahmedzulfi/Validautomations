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
    let r = egui::Rect::from_min_size(canvas.min + vec2(16.0, canvas.height() - 54.0), vec2(canvas.width() - 32.0, 40.0));
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
            covers: &["shadcn/ui: Dialog", "Animate UI: Dialog", "Origin UI: Dialog", "Radix: Dialog"],
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
    ]
}
