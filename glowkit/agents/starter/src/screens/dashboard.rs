//! A sample dashboard: animated headline, KPI cards, tabs and an area chart.

use glowkit::prelude::*;
use glowkit::charts::{AreaChart, Series};
use glowkit::components::{toast, Button, ButtonVariant, StatCard, Tabs, TabsStyle, Toast, UiIcon};
use glowkit::text::BlurText;

const MONTHS: [&str; 6] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];

pub struct Dashboard {
    range: usize,
    series: Vec<Series>,
}

impl Default for Dashboard {
    fn default() -> Self {
        Self { range: 0, series: data(0) }
    }
}

/// Sample data for a date range (replace with your own).
fn data(range: usize) -> Vec<Series> {
    let k = 1.0 + range as f64 * 0.35;
    vec![
        Series::new("Desktop", [186.0, 305.0, 237.0, 273.0, 209.0, 314.0].map(|v| v * k)),
        Series::new("Mobile", [80.0, 200.0, 120.0, 190.0, 130.0, 240.0].map(|v| v * k)),
    ]
}

impl Dashboard {
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let t = Theme::of(ui);
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(BlurText::new("Good morning, Ada").size(28.0).trigger(Trigger::OnAppear));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(Button::new("Export").variant(ButtonVariant::Outline).icon(UiIcon::Download)).clicked() {
                        toast(ui.ctx(), Toast::success("Report exported").description("Saved to your downloads."));
                    }
                });
            });
            ui.label(egui::RichText::new("Here is what happened across your workspace.").color(t.muted_foreground));
            ui.add_space(20.0);

            // KPI row: fixed-width cards that wrap on narrow windows.
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(16.0, 16.0);
                ui.add(
                    StatCard::new("Revenue", 45231.89)
                        .prefix("$")
                        .decimals(2)
                        .delta(20.1)
                        .icon(UiIcon::Dollar)
                        .sparkline(&[12.0, 18.0, 15.0, 22.0, 20.0, 28.0, 31.0])
                        .width(260.0),
                );
                ui.add(StatCard::new("Active users", 2350.0).delta(12.4).icon(UiIcon::Users).width(260.0));
                ui.add(StatCard::new("Churn", 2.1).suffix("%").decimals(1).delta(-0.4).invert_delta(true).width(260.0));
            });
            ui.add_space(24.0);

            if ui.add(Tabs::new(&mut self.range, &["Last 6 months", "This year", "All time"]).style(TabsStyle::Pill)).changed() {
                self.series = data(self.range);
            }
            ui.add_space(12.0);
            ui.add(AreaChart::new(&self.series).labels(&MONTHS).height(300.0));
        });
    }
}
