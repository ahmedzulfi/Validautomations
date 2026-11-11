//! Helpers shared by the app templates.

use egui::{pos2, vec2, Align2, Rect, Ui};
use glowkit_core::Theme;

use crate::shells::kit;
use crate::shells::{ShellNotification, ShellUser, Workspace};

/// Default user for the templates.
pub(crate) fn demo_user() -> ShellUser {
    ShellUser::new("Ada Lovelace", "ada@acme.dev")
}

/// Default workspaces for the templates.
pub(crate) fn demo_workspaces(first: &str, plan: &str) -> Vec<Workspace> {
    vec![Workspace::new(first, plan), Workspace::new("Acme Labs", "Startup"), Workspace::new("Personal", "Free")]
}

/// A small set of notifications.
pub(crate) fn demo_notes() -> Vec<ShellNotification> {
    use crate::shells::AppIcon;
    use glowkit_components::LineIcon;
    vec![
        ShellNotification::new(LineIcon::Users, "Mia Chen joined Design", "Say hi and share the onboarding checklist.", "2m ago"),
        ShellNotification::new(AppIcon::Cart, "New order #3210", "Olivia Martin purchased the Pro plan (annual).", "18m ago"),
        ShellNotification::new(LineIcon::Warning, "Usage at 82%", "Your workspace is close to its monthly API quota.", "1h ago"),
        ShellNotification::new(LineIcon::CircleCheck, "Deploy succeeded", "web-app v2.14.0 is live in production.", "Yesterday").read(),
    ]
}

/// Page header: title + subtitle on the left; `right` lays out controls right-to-left in the
/// remaining space. Returns the y where content starts.
pub(crate) fn page_header(ui: &mut Ui, rect: Rect, title: &str, subtitle: &str, right: impl FnOnce(&mut Ui)) -> f32 {
    let t = Theme::of(ui);
    let p = ui.painter().clone();
    let tw = (rect.width() * 0.5).max(200.0);
    kit::text_at(&p, rect.min, Align2::LEFT_TOP, title, 22.0, t.foreground, tw);
    if !subtitle.is_empty() {
        kit::text_at(&p, rect.min + vec2(0.0, 31.0), Align2::LEFT_TOP, subtitle, 13.5, t.muted_foreground, tw);
    }
    // Align::Min: field widgets allocate top-down from their frame top, so centering a row of
    // them misplaces them; 32-point controls fill this 32-point strip exactly.
    let rr = Rect::from_min_max(pos2(rect.min.x + tw + 12.0, rect.min.y + 6.0), pos2(rect.max.x, rect.min.y + 38.0));
    kit::child(ui, rr, |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            right(ui);
        });
    });
    rect.min.y + 56.0
}

/// Run an app stored in egui memory under the demo's id (the gallery preview helper).
#[cfg(feature = "demos")]
pub(crate) fn run_demo<A: Clone + Send + Sync + 'static>(ui: &mut Ui, init: fn() -> A, show: fn(&mut A, &mut Ui)) {
    let id = ui.scope_id().with("app_template");
    let mut app: A = glowkit_core::state(ui, id, init);
    let h = 720.0f32.min(ui.available_height().max(560.0));
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), egui::Sense::hover());
    kit::child(ui, rect, |ui| show(&mut app, ui));
    glowkit_core::set_state(ui, id, app);
}

/// Deterministic pseudo-random series (for realistic chart data).
pub(crate) fn series(seed: u64, n: usize, base: f64, trend: f64, noise: f64) -> Vec<f64> {
    let mut rng = glowkit_core::Rng::new(seed);
    let mut v = base;
    (0..n)
        .map(|_| {
            v += trend + (rng.f32() as f64 - 0.45) * noise;
            v.max(0.0)
        })
        .collect()
}
