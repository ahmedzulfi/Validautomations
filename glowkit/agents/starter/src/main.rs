//! glowkit starter: an app shell (sidebar, top bar, command palette, notifications) with two
//! screens. Run with `cargo run --release`.

mod screens;

use glowkit::prelude::*;
use glowkit::components::LineIcon;
use glowkit::templates::{AppShell, NavItem, NavSection, ShellLayout, ShellNotification, ShellUser, Workspace};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([720.0, 480.0])
            .with_title("glowkit starter"),
        ..Default::default()
    };
    eframe::run_native(
        "glowkit-starter",
        options,
        Box::new(|cc| {
            // One call styles every glowkit component and egui's own widgets.
            brand_theme().install(&cc.egui_ctx);
            Ok(Box::new(App::default()))
        }),
    )
}

/// The app theme: a built-in preset with a brand accent. Presets: dark, light, ocean, ember, matrix.
pub fn brand_theme() -> Theme {
    let mut t = Theme::dark();
    t.accent = hex("#8B5CF6");
    t.ring = t.accent;
    t.radius = 10.0;
    t
}

/// All UI state lives here, so it survives from frame to frame.
struct App {
    route: String,
    nav: Vec<NavSection>,
    notifications: Vec<ShellNotification>,
    dashboard: screens::dashboard::Dashboard,
    settings: screens::settings::Settings,
}

impl Default for App {
    fn default() -> Self {
        Self {
            route: "dashboard".into(),
            nav: vec![
                NavSection::new("Workspace")
                    .item(NavItem::new("dashboard", "Dashboard").icon(LineIcon::Home))
                    .item(NavItem::new("settings", "Settings").icon(LineIcon::Settings)),
            ],
            notifications: Vec::new(),
            dashboard: Default::default(),
            settings: Default::default(),
        }
    }
}

impl eframe::App for App {
    // eframe 0.37: `ui`, not `update`.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let Self { route, nav, notifications, dashboard, settings } = self;
        egui::CentralPanel::default().frame(egui::Frame::new()).show(ui, |ui| {
            AppShell::new(route, nav)
                .layout(ShellLayout::Sidebar)
                .workspaces(vec![Workspace::new("Acme Inc", "Pro plan")])
                .user(ShellUser::new("Ada Lovelace", "ada@acme.dev"))
                .notifications(notifications)
                .rounded(false)
                .show(ui, |ui, shell| match shell.route.as_str() {
                    "settings" => settings.show(ui),
                    _ => dashboard.show(ui),
                });
        });
    }
}
