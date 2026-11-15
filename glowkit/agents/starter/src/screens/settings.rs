//! A sample settings form: inputs, switches and a save button with a toast.

use glowkit::prelude::*;
use glowkit::components::{toast, Button, Input, Switch, Toast};

pub struct Settings {
    name: String,
    email: String,
    notifications: bool,
    weekly_digest: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { name: "Ada Lovelace".into(), email: "ada@acme.dev".into(), notifications: true, weekly_digest: false }
    }
}

impl Settings {
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let t = Theme::of(ui);
        ui.heading("Settings");
        ui.label(egui::RichText::new("Manage your profile and notifications.").color(t.muted_foreground));
        ui.add_space(20.0);
        ui.set_max_width(420.0);
        ui.add(Input::new(&mut self.name).label("Name"));
        ui.add_space(12.0);
        ui.add(Input::new(&mut self.email).label("Email").placeholder("you@company.com"));
        ui.add_space(20.0);
        ui.add(Switch::new(&mut self.notifications).label("Push notifications").description("Alerts on your devices."));
        ui.add_space(8.0);
        ui.add(Switch::new(&mut self.weekly_digest).label("Weekly digest").description("A summary every Monday."));
        ui.add_space(24.0);
        if ui.add(Button::new("Save changes")).clicked() {
            toast(ui.ctx(), Toast::success("Settings saved"));
        }
    }
}
