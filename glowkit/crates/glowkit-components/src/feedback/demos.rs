use super::*;
use crate::inputs::demos::{caption, center, col, panel, with};
use crate::inputs::{Button, ButtonVariant, UiIcon};
use egui::{vec2, Align, Layout, RichText, Ui};
use glowkit_core::demo::{Category::Loaders as C, Demo};
use glowkit_core::Theme;

fn spinner_cell(ui: &mut Ui, kind: SpinnerKind, size: f32) {
    let t = Theme::of(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(96.0, 92.0), egui::Sense::hover());
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(10),
        if t.dark { t.background } else { t.muted },
        egui::Stroke::new(1.0, t.border),
        egui::StrokeKind::Inside,
    );
    Spinner::new(kind)
        .size(size)
        .paint(ui, egui::Rect::from_center_size(rect.center() - vec2(0.0, 10.0), vec2(size, size)));
    ui.painter().text(
        egui::pos2(rect.center().x, rect.max.y - 14.0),
        egui::Align2::CENTER_CENTER,
        kind.name(),
        egui::FontId::proportional(11.5),
        t.muted_foreground,
    );
}

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "spinner",
            name: "Spinners",
            category: C,
            description: "Eighteen loader styles in one widget: rings, arcs, dots, bars, pulses, orbits, waves, grids, helixes and more.",
            covers: &["shadcn/ui: Spinner", "Uiverse: Loaders", "SpinKit: Spinners", "React Spinners", "Origin UI: Spinner"],
            code: r##"ui.add(Spinner::new(SpinnerKind::Orbit).size(32.0));
ui.add(Spinner::new(SpinnerKind::Gradient).size(32.0).speed(1.5));
ui.add(Spinner::new(SpinnerKind::Bars).color(hex("#22C55E")));"##,
            show: |ui| {
                center(ui, 6.0 * 96.0 + 5.0 * 10.0, |ui| {
                    ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
                    for row in SpinnerKind::ALL.chunks(6) {
                        ui.horizontal(|ui| {
                            for &k in row {
                                spinner_cell(ui, k, 30.0);
                            }
                        });
                    }
                });
            },
        },
        Demo {
            id: "progress",
            name: "Progress",
            category: C,
            description: "Linear progress bars that ease to their value, with labels, stripes, gradients and an indeterminate mode.",
            covers: &["shadcn/ui: Progress", "Origin UI: Progress", "Uiverse: Progress bars", "Kibo UI: Progress"],
            code: r##"ui.add(Progress::new(0.66).label("Uploading files").show_percent(true));
ui.add(Progress::new(0.4).striped(true).gradient(true));
ui.add(Progress::indeterminate().height(4.0));"##,
            show: |ui| {
                ui.add_space(30.0);
                let time = glowkit_core::time(ui);
                let cycle = ((time / 6.0).fract() * 1.3).min(1.0);
                center(ui, 440.0, |ui| {
                    ui.spacing_mut().item_spacing.y = 22.0;
                    ui.add(Progress::new(cycle).label("Uploading 12 files").show_percent(true).width(440.0));
                    ui.add(Progress::new(0.42).label("Storage").show_percent(true).gradient(true).height(10.0).width(440.0));
                    ui.add(Progress::new(0.68).striped(true).color(Theme::of(ui).success).height(12.0).width(440.0));
                    caption(ui, "Indeterminate");
                    ui.add(Progress::indeterminate().height(4.0).width(440.0));
                });
                glowkit_core::animate(ui);
            },
        },
        Demo {
            id: "progress-ring",
            name: "Progress Ring",
            category: C,
            description: "Circular progress with a centered value, sub-label, gradient arcs and an indeterminate spin.",
            covers: &["Origin UI: Circular progress", "Kibo UI: Progress (circular)", "Magic UI: Animated Circular Progress Bar"],
            code: r##"ui.add(ProgressRing::new(0.72).size(110.0).thickness(10.0).gradient(true).sublabel("Complete"));
ui.add(ProgressRing::new(0.45).size(72.0));
ui.add(ProgressRing::indeterminate().size(40.0).thickness(4.0));"##,
            show: |ui| {
                ui.add_space(90.0);
                let time = glowkit_core::time(ui);
                let v = 0.15 + 0.8 * (0.5 - 0.5 * (time * 0.6).cos());
                center(ui, 560.0, |ui| {
                    ui.horizontal(|ui| {
                        ui.set_min_height(130.0);
                        ui.spacing_mut().item_spacing.x = 36.0;
                        ui.add(ProgressRing::new(v).size(130.0).thickness(11.0).gradient(true).sublabel("Complete"));
                        ui.add(ProgressRing::new(0.45).size(84.0));
                        ui.add(ProgressRing::new(0.9).size(84.0).color(Theme::of(ui).success).label("9/10").thickness(5.0));
                        ui.add(ProgressRing::indeterminate().size(44.0).thickness(4.0));
                        ui.add(ProgressRing::indeterminate().size(44.0).thickness(4.0).gradient(true));
                    });
                });
                glowkit_core::animate(ui);
            },
        },
        Demo {
            id: "skeleton",
            name: "Skeleton",
            category: C,
            description: "Placeholder blocks with one synchronized shimmer sweeping across the whole layout.",
            covers: &["shadcn/ui: Skeleton", "Origin UI: Skeleton", "Uiverse: Skeleton loaders"],
            code: r##"ui.horizontal(|ui| {
    ui.add(Skeleton::circle(44.0));
    ui.vertical(|ui| {
        ui.add(Skeleton::line(180.0));
        ui.add(Skeleton::line(120.0));
    });
});
ui.add(Skeleton::new(egui::vec2(320.0, 160.0)));"##,
            show: |ui| {
                ui.add_space(20.0);
                center(ui, 700.0, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 24.0;
                        panel(ui, 330.0, |ui| {
                            ui.spacing_mut().item_spacing.y = 12.0;
                            ui.add(Skeleton::new(vec2(290.0, 140.0)));
                            ui.add(Skeleton::new(vec2(200.0, 16.0)));
                            ui.add(Skeleton::line(290.0));
                            ui.add(Skeleton::line(240.0));
                        });
                        col(ui, 340.0, |ui| {
                            ui.spacing_mut().item_spacing.y = 22.0;
                            for w in [200.0, 150.0, 180.0] {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 12.0;
                                    ui.add(Skeleton::circle(44.0));
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 8.0;
                                        ui.add_space(6.0);
                                        ui.add(Skeleton::line(w));
                                        ui.add(Skeleton::line(w * 0.6));
                                    });
                                });
                            }
                        });
                    });
                });
            },
        },
        Demo {
            id: "step-progress",
            name: "Step Progress",
            category: C,
            description: "A stepper whose connectors fill and check marks draw as you advance; the current step pulses.",
            covers: &["Origin UI: Stepper", "Kibo UI: Stepper", "Dice UI: Stepper", "shadcn/ui: Stepper"],
            code: r##"ui.add(StepProgress::new(&["Account", "Profile", "Billing", "Confirm"], step)
    .descriptions(&["Create login", "Your details", "Payment", "Review"]));"##,
            show: |ui| {
                ui.add_space(60.0);
                with(ui, "step", || 1usize, |ui, step| {
                    center(ui, 620.0, |ui| {
                        ui.add(StepProgress::new(&["Account", "Profile", "Billing", "Confirm"], *step).descriptions(&["Create login", "Your details", "Payment method", "Review order"]).width(620.0));
                        ui.add_space(30.0);
                        ui.horizontal(|ui| {
                            ui.add_space(190.0);
                            ui.spacing_mut().item_spacing.x = 10.0;
                            if ui.add(Button::new("Back").variant(ButtonVariant::Outline).icon(UiIcon::ArrowLeft).disabled(*step == 0).width(110.0)).clicked() {
                                *step -= 1;
                            }
                            let last = *step >= 4;
                            if ui.add(Button::new(if *step >= 3 { "Finish" } else { "Next" }).trailing_icon(UiIcon::ArrowRight).disabled(last).width(110.0)).clicked() {
                                *step += 1;
                            }
                        });
                    });
                });
            },
        },
        Demo {
            id: "loading-dots",
            name: "Loading Dots",
            category: C,
            description: "Inline busy text with bouncing ellipsis dots, plus spinners inside buttons and status rows.",
            covers: &["Uiverse: Loading text", "Magic UI: Typing dots", "Animate UI: Loading"],
            code: r##"ui.add(LoadingDots::new("Generating response"));
ui.add(Button::new("Deploying").loading(true));"##,
            show: |ui| {
                ui.add_space(90.0);
                center(ui, 420.0, |ui| {
                    let t = Theme::of(ui);
                    ui.spacing_mut().item_spacing.y = 22.0;
                    ui.add(LoadingDots::new("Generating response").size(16.0));
                    ui.add(LoadingDots::new("Thinking").size(22.0).color(t.foreground));
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        ui.add(Button::new("Deploying").loading(true));
                        ui.add(Spinner::new(SpinnerKind::Ring).size(18.0));
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.label(RichText::new("Syncing 3 of 12…").color(t.muted_foreground));
                        });
                    });
                });
            },
        },
    ]
}
