//! The smallest useful glowkit app: an animated hero screen.
//!
//! Run it with `cargo run -p glowkit-example-hello`.

use glowkit::backgrounds::Aurora;
use glowkit::components::{toast, Toast, Toaster};
use glowkit::effects::ShimmerButton;
use glowkit::prelude::*;
use glowkit::text::{RotatingText, ShinyText};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([960.0, 600.0]).with_title("Hello glowkit"),
        ..Default::default()
    };
    eframe::run_native(
        "Hello glowkit",
        options,
        Box::new(|cc| {
            // One call styles every glowkit component and egui's own widgets.
            Theme::dark().install(&cc.egui_ctx);
            Ok(Box::new(App))
        }),
    )
}

struct App;

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let theme = Theme::of(ui);
        egui::CentralPanel::default().frame(egui::Frame::new().fill(theme.background)).show(ui, |ui| {
            // Paint the background behind everything else in this panel.
            Aurora::new().paint(ui, ui.max_rect());

            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.28);
                ui.add(ShinyText::new("Animated UI, in pure Rust").size(44.0));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    // Center a row of mixed widgets.
                    let row_width = 360.0;
                    ui.add_space(((ui.available_width() - row_width) / 2.0).max(0.0));
                    ui.label(egui::RichText::new("Build apps that feel").size(22.0).color(theme.muted_foreground));
                    ui.add(RotatingText::new(["fast", "alive", "polished", "native"]).size(22.0));
                });
                ui.add_space(28.0);
                if ui.add(ShimmerButton::new("Get started")).clicked() {
                    toast(
                        ui.ctx(),
                        Toast::success("Welcome to glowkit").description("You clicked the shimmer button."),
                    );
                }
            });
        });
        // Toasts float above everything; show the stack once per frame.
        Toaster::new().show(ui);
    }
}
