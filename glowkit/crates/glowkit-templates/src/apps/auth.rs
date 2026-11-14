//! Auth-gated screens and empty states: sign-in gate, upgrade wall, access denied, first run.

use egui::{pos2, vec2, Align2, Color32, Rect, Stroke, StrokeKind, Ui};
use glowkit_components::{
    toast, Button, ButtonVariant, Checkbox, ControlSize, EmptyState, Input, LineIcon, PasswordInput, Separator, Toast,
    UiIcon,
};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

use super::common::{demo_notes, demo_user, demo_workspaces};
use crate::shells::kit;
use crate::shells::{AppIcon, AppShell, NavItem, NavSection, ShellLayout, ShellNotification};

/// Auth-gated app states inside an [`AppShell`]: while signed out every page shows a sign-in
/// card (email + password with validation, OAuth buttons, remember me); once signed in,
/// Reports is behind an upgrade wall over a blurred preview, Admin shows "access denied" with a
/// request-access flow, and Projects shows a first-run empty state that seeds sample data.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::AuthGateApp::default();
/// app.signed_in = true;
/// app.route = "reports".into();
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct AuthGateApp {
    /// Active route.
    pub route: String,
    /// Navigation.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// Is the user signed in?
    pub signed_in: bool,
    /// Has the workspace upgraded (unlocks Reports)?
    pub pro: bool,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    email: String,
    password: String,
    remember: bool,
    tried: bool,
    signing_in: Option<f64>,
    requested: bool,
    projects: Vec<String>,
}

impl Default for AuthGateApp {
    fn default() -> Self {
        Self {
            route: "projects".into(),
            nav: vec![
                NavSection::new("Workspace")
                    .item(NavItem::new("projects", "Projects").icon(LineIcon::Folder))
                    .item(NavItem::new("reports", "Reports").icon(LineIcon::Chart).badge("Pro"))
                    .item(NavItem::new("admin", "Admin").icon(AppIcon::Shield)),
                NavSection::new("Account").item(NavItem::new("settings", "Settings").icon(LineIcon::Settings)),
            ],
            notifications: demo_notes(),
            signed_in: false,
            pro: false,
            scoped_theme: false,
            email: "ada@acme.dev".into(),
            password: String::new(),
            remember: true,
            tried: false,
            signing_in: None,
            requested: false,
            projects: Vec::new(),
        }
    }
}

impl AuthGateApp {
    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        let now = ui.input(|i| i.time);
        if let Some(t0) = self.signing_in {
            if now - t0 > 1.1 {
                self.signing_in = None;
                self.signed_in = true;
                toast(ui.ctx(), Toast::success("Welcome back, Ada").description("Signed in to Acme Inc."));
            } else {
                ui.ctx().request_repaint();
            }
        }
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        let mut shell = AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::Inset)
            .workspaces(demo_workspaces("Acme Inc", if self.pro { "Pro" } else { "Free" }))
            .scoped_theme(self.scoped_theme)
            .id_salt("auth_app");
        if self.signed_in {
            shell = shell.user(demo_user()).notifications(&mut notes);
        }
        let r = shell.show(ui, |ui, sc| {
            if !self.signed_in {
                self.sign_in(ui, sc.rect);
                return;
            }
            match sc.route.as_str() {
                "reports" => self.upgrade_wall(ui, sc.rect),
                "admin" => self.denied(ui, sc.rect),
                _ => self.first_run(ui, sc.rect),
            }
        });
        if r.menu.as_deref() == Some("Log out") {
            self.signed_in = false;
            self.password.clear();
            self.tried = false;
            toast(ui.ctx(), Toast::info("Signed out"));
        }
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
    }

    fn sign_in(&mut self, ui: &mut Ui, rect: Rect) {
        let t = Theme::of(ui);
        let p = ui.painter().clone();
        // soft backdrop
        paint::glow(
            &p,
            rect.center() + vec2(-160.0, -120.0),
            260.0,
            with_alpha(t.gradient[0], if t.dark { 0.10 } else { 0.07 }),
        );
        paint::glow(
            &p,
            rect.center() + vec2(200.0, 140.0),
            240.0,
            with_alpha(t.gradient[1], if t.dark { 0.08 } else { 0.05 }),
        );
        let card = Rect::from_center_size(
            rect.center(),
            vec2(380.0f32.min(rect.width() - 32.0), 488.0f32.min(rect.height() - 8.0)),
        );
        paint::soft_shadow(&p, card, t.radius * 1.4, 24.0, vec2(0.0, 10.0), kit::shadow(&t));
        p.rect_filled(card, t.radius * 1.4, t.card);
        p.rect_stroke(card, t.radius * 1.4, Stroke::new(1.0, t.border), StrokeKind::Inside);
        let inner = card.shrink(28.0);
        let busy = self.signing_in.is_some();
        kit::child(ui, inner, |ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            let w = ui.available_width();
            let r = kit::alloc_row(ui, 40.0);
            kit::logo(
                ui.painter(),
                Rect::from_center_size(pos2(r.center().x, r.center().y), vec2(40.0, 40.0)),
                &t,
                "A",
            );
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("Sign in to Acme").size(20.0).color(t.foreground));
                ui.label(
                    egui::RichText::new("You need an account to view this workspace.")
                        .size(13.0)
                        .color(t.muted_foreground),
                );
            });
            let cols = kit::cols(kit::alloc_row(ui, 32.0), 2, 10.0);
            for (k, (label, icon)) in [("GitHub", UiIcon::Code), ("Google", UiIcon::Globe)].iter().enumerate() {
                if kit::put(
                    ui,
                    cols[k],
                    Button::new(*label)
                        .icon(*icon)
                        .variant(ButtonVariant::Outline)
                        .size(ControlSize::Sm)
                        .width(cols[k].width()),
                )
                .clicked()
                {
                    self.signing_in = Some(ui.input(|i| i.time));
                }
            }
            ui.add(Separator::horizontal().label("or continue with email"));
            let email_err = if self.tried && !self.email.contains('@') {
                Some("Enter a valid email address".to_string())
            } else {
                None
            };
            ui.add(
                Input::new(&mut self.email)
                    .label("Email")
                    .icon(UiIcon::Mail)
                    .error_opt(email_err)
                    .width(w)
                    .disabled(busy)
                    .id_salt("auth_email"),
            );
            let pw_err =
                if self.tried && self.password.len() < 6 { Some("At least 6 characters".to_string()) } else { None };
            ui.add(
                PasswordInput::new(&mut self.password)
                    .label("Password")
                    .error_opt(pw_err)
                    .width(w)
                    .disabled(busy)
                    .id_salt("auth_pw"),
            );
            let r = kit::alloc_row(ui, 22.0);
            kit::child(ui, Rect::from_min_size(r.min, vec2(160.0, 22.0)), |ui| {
                ui.add(Checkbox::new(&mut self.remember).label("Remember me").size(16.0))
            });
            kit::text_at(
                ui.painter(),
                r.right_center(),
                Align2::RIGHT_CENTER,
                "Forgot password?",
                12.5,
                t.accent,
                140.0,
            );
            let submit = ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.add(Button::new(if busy { "Signing in…" } else { "Sign in" }).loading(busy).width(w)).clicked()
                || (submit && !busy)
            {
                self.tried = true;
                if self.email.contains('@') && self.password.len() >= 6 {
                    self.signing_in = Some(ui.input(|i| i.time));
                } else {
                    toast(ui.ctx(), Toast::error("Check the highlighted fields"));
                }
            }
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("Tip: any email and a 6+ character password works.")
                        .size(11.5)
                        .color(t.muted_foreground),
                );
            });
        });
    }

    /// A page of skeleton "content" behind gates (the blurred preview).
    fn ghost_page(ui: &Ui, rect: Rect, t: &Theme) {
        let p = ui.painter();
        let sk = mix(t.background, t.foreground, if t.dark { 0.06 } else { 0.05 });
        p.rect_filled(Rect::from_min_size(rect.min, vec2(220.0, 18.0)), 6.0, sk);
        p.rect_filled(Rect::from_min_size(rect.min + vec2(0.0, 30.0), vec2(340.0, 10.0)), 5.0, sk);
        let row = Rect::from_min_size(rect.min + vec2(0.0, 64.0), vec2(rect.width(), 90.0));
        for r in kit::cols(row, 4, 14.0) {
            p.rect_stroke(r, t.radius, Stroke::new(1.0, t.border), StrokeKind::Inside);
            p.rect_filled(Rect::from_min_size(r.min + vec2(14.0, 16.0), vec2(80.0, 9.0)), 4.0, sk);
            p.rect_filled(Rect::from_min_size(r.min + vec2(14.0, 38.0), vec2(120.0, 20.0)), 6.0, sk);
        }
        let chart = Rect::from_min_max(rect.min + vec2(0.0, 170.0), rect.max);
        p.rect_stroke(chart, t.radius, Stroke::new(1.0, t.border), StrokeKind::Inside);
        let pts: Vec<egui::Pos2> = (0..24)
            .map(|i| {
                pos2(
                    chart.min.x + 20.0 + i as f32 * (chart.width() - 40.0) / 23.0,
                    chart.max.y
                        - 40.0
                        - (chart.height() - 100.0) * (0.3 + 0.25 * ((i as f32) * 0.6).sin() + i as f32 * 0.015),
                )
            })
            .collect();
        p.add(egui::Shape::line(pts, Stroke::new(2.0, with_alpha(t.chart[0], 0.6))));
    }

    fn upgrade_wall(&mut self, ui: &mut Ui, rect: Rect) {
        let t = Theme::of(ui);
        if self.pro {
            ui.add_space(0.0);
            let p = ui.painter().clone();
            kit::heading(&p, rect, "Reports", "Pro unlocked: cohort, retention and revenue reports.", &t);
            kit::child(
                ui,
                Rect::from_min_size(rect.min + vec2(0.0, 70.0), vec2(rect.width().min(520.0), 240.0)),
                |ui| {
                    ui.add(
                        EmptyState::new(UiIcon::Activity, "Your first report is generating")
                            .description("Data from the last 90 days is being processed. This usually takes a minute.")
                            .width(ui.available_width()),
                    );
                },
            );
            return;
        }
        Self::ghost_page(ui, rect, &t);
        let p = ui.painter().clone();
        // frosted veil
        kit::vgrad(&p, rect, with_alpha(t.background, 0.55), with_alpha(t.background, 0.92));
        let card = Rect::from_center_size(rect.center(), vec2(420.0f32.min(rect.width() - 32.0), 250.0));
        paint::soft_shadow(&p, card, t.radius * 1.4, 24.0, vec2(0.0, 10.0), kit::shadow(&t));
        p.rect_filled(card, t.radius * 1.4, t.card);
        let g = t.gradient;
        paint::gradient_border(&p, card, t.radius * 1.4, 1.5, |u| {
            glowkit_core::color::gradient(&[g[0], g[1], g[2], g[0]], u)
        });
        let ic = pos2(card.center().x, card.min.y + 44.0);
        p.circle_filled(ic, 24.0, with_alpha(t.accent, 0.14));
        LineIcon::Lock.paint(&p, Rect::from_center_size(ic, vec2(30.0, 30.0)), t.accent);
        kit::text_at(
            &p,
            pos2(card.center().x, card.min.y + 94.0),
            Align2::CENTER_CENTER,
            "Reports are a Pro feature",
            18.0,
            t.foreground,
            card.width() - 40.0,
        );
        let gl = kit::wrapped(
            &p,
            "Upgrade to unlock cohort, retention and revenue reports, scheduled exports and unlimited history.",
            13.0,
            t.muted_foreground,
            card.width() - 64.0,
            3,
        );
        p.galley(pos2(card.center().x - gl.size().x / 2.0, card.min.y + 112.0), gl, t.muted_foreground);
        let bw = 150.0;
        if kit::put(
            ui,
            Rect::from_min_size(pos2(card.center().x - bw - 6.0, card.max.y - 56.0), vec2(bw, 34.0)),
            Button::new("Upgrade to Pro").icon(UiIcon::Sparkles).size(ControlSize::Sm).width(bw),
        )
        .clicked()
        {
            self.pro = true;
            if let Some(it) = self.nav[0].items.get_mut(1) {
                it.badge = None;
            }
            toast(
                ui.ctx(),
                Toast::success("Welcome to Pro").description("14-day free trial started. Reports unlocked."),
            );
        }
        if kit::put(
            ui,
            Rect::from_min_size(pos2(card.center().x + 6.0, card.max.y - 56.0), vec2(bw, 34.0)),
            Button::new("Compare plans").variant(ButtonVariant::Outline).size(ControlSize::Sm).width(bw),
        )
        .clicked()
        {
            toast(ui.ctx(), Toast::new("Plans").description("Free · Pro $24 · Enterprise $79 per seat"));
        }
    }

    fn denied(&mut self, ui: &mut Ui, rect: Rect) {
        let t = Theme::of(ui);
        let r = Rect::from_center_size(rect.center(), vec2(rect.width().min(460.0), 300.0));
        let p = ui.painter().clone();
        let ic = pos2(r.center().x, r.min.y + 36.0);
        p.circle_filled(ic, 30.0, with_alpha(t.destructive, 0.12));
        p.circle_stroke(ic, 38.0, Stroke::new(1.0, with_alpha(t.destructive, 0.2)));
        AppIcon::Shield.paint(&p, Rect::from_center_size(ic, vec2(26.0, 26.0)), t.destructive);
        kit::text_at(
            &p,
            pos2(r.center().x, r.min.y + 96.0),
            Align2::CENTER_CENTER,
            "You don't have access to Admin",
            18.0,
            t.foreground,
            r.width(),
        );
        let g = kit::wrapped(&p, "Admin is limited to workspace owners. Ask an owner to grant you the Admin role, or switch to a workspace you manage.", 13.0, t.muted_foreground, r.width() - 40.0, 3);
        p.galley(pos2(r.center().x - g.size().x / 2.0, r.min.y + 116.0), g, t.muted_foreground);
        let bw = 160.0;
        let label = if self.requested { "Request sent" } else { "Request access" };
        if kit::put(
            ui,
            Rect::from_min_size(pos2(r.center().x - bw - 6.0, r.min.y + 186.0), vec2(bw, 34.0)),
            Button::new(label)
                .icon(if self.requested { UiIcon::Check } else { UiIcon::Mail })
                .size(ControlSize::Sm)
                .width(bw)
                .disabled(self.requested),
        )
        .clicked()
        {
            self.requested = true;
            toast(ui.ctx(), Toast::success("Request sent").description("Grace Hopper (owner) will be notified."));
        }
        if kit::put(
            ui,
            Rect::from_min_size(pos2(r.center().x + 6.0, r.min.y + 186.0), vec2(bw, 34.0)),
            Button::new("Go to Projects").variant(ButtonVariant::Outline).size(ControlSize::Sm).width(bw),
        )
        .clicked()
        {
            toast(ui.ctx(), Toast::new("Use the sidebar to navigate"));
        }
        kit::text_at(
            &p,
            pos2(r.center().x, r.min.y + 246.0),
            Align2::CENTER_CENTER,
            "Error 403 · ref 7F3A-21C9",
            11.5,
            t.muted_foreground,
            r.width(),
        );
        let _ = Color32::WHITE;
    }

    fn first_run(&mut self, ui: &mut Ui, rect: Rect) {
        let t = Theme::of(ui);
        let p = ui.painter().clone();
        kit::heading(
            &p,
            rect,
            "Projects",
            if self.projects.is_empty() { "Nothing here yet. Let's fix that." } else { "Your projects" },
            &t,
        );
        if self.projects.is_empty() {
            let r = Rect::from_center_size(rect.center() + vec2(0.0, 20.0), vec2(rect.width().min(560.0), 300.0));
            let out = kit::child(ui, r, |ui| {
                ui.add(EmptyState::new(UiIcon::Folder, "Create your first project").side_icons(UiIcon::File, UiIcon::Image).description("Projects group dashboards, data sources and teammates. Start blank or load a sample to explore.").action("New project").secondary_action("Load sample data").dashed(true).width(r.width()))
            });
            if out.clicked() {
                self.projects = vec!["Website redesign".into(), "Mobile app v3".into(), "Brand refresh".into()];
                toast(ui.ctx(), Toast::success("Sample data loaded").description("3 projects were created."));
            }
            return;
        }
        let row = Rect::from_min_size(rect.min + vec2(0.0, 70.0), vec2(rect.width(), 120.0));
        let n = self.projects.len();
        for (i, cr) in kit::cols(row, n.max(3), 14.0).iter().enumerate().take(n) {
            kit::card(&p, *cr, &t);
            kit::logo(&p, Rect::from_min_size(cr.min + vec2(16.0, 16.0), vec2(36.0, 36.0)), &t, &self.projects[i][..1]);
            kit::text_at(
                &p,
                cr.min + vec2(16.0, 74.0),
                Align2::LEFT_CENTER,
                &self.projects[i],
                14.5,
                t.foreground,
                cr.width() - 32.0,
            );
            kit::text_at(
                &p,
                cr.min + vec2(16.0, 96.0),
                Align2::LEFT_CENTER,
                "Updated just now",
                12.0,
                t.muted_foreground,
                cr.width() - 32.0,
            );
        }
    }
}
