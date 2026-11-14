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
    ]
}
