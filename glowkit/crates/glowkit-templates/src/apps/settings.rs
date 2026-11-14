//! Settings pages template: profile, account, notifications, appearance, billing and team.

use egui::{pos2, vec2, Align2, Color32, CursorIcon, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_components::{
    toast, AlertDialog, Badge, BadgeVariant, Button, ButtonVariant, CalendarDate, Checkbox, ControlSize, DatePicker, Input, LineIcon, RadioGroup, RadioOption,
    SegmentedControl, Select, Slider, Switch, Textarea, Toast, UiIcon,
};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

use super::common::{demo_notes, demo_user, demo_workspaces};
use crate::shells::kit;
use crate::shells::{AppIcon, AppShell, NavItem, NavSection, ShellContext, ShellLayout, ShellNotification};

/// A team member row in the [`SettingsApp`].
#[derive(Clone, Debug, PartialEq)]
pub struct TeamMember {
    /// Name.
    pub name: String,
    /// Email.
    pub email: String,
    /// Role index into Owner / Admin / Member / Viewer.
    pub role: Option<usize>,
    /// Invitation pending.
    pub pending: bool,
}

/// Settings pages inside an [`AppShell`]: Profile (form with validation), Account (birthday
/// picker, language, danger zone), Notifications (radio group, switches), Appearance (a theme
/// picker that really installs [`Theme`] presets, plus corner radius and density), Billing (plan
/// cards with a monthly / yearly switch, payment method, invoices) and Team (roles, invites).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::SettingsApp::default();
/// app.route = "settings/appearance".into();
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct SettingsApp {
    /// Active route ("settings/profile", ...).
    pub route: String,
    /// Settings navigation.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// Team members.
    pub members: Vec<TeamMember>,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    name: String,
    username: String,
    email: Option<usize>,
    bio: String,
    website: String,
    birthday: Option<CalendarDate>,
    language: Option<usize>,
    delete_open: bool,
    notify: usize,
    switches: [bool; 4],
    mobile: bool,
    radius: f32,
    density: usize,
    yearly: bool,
    plan: usize,
    invite: String,
    dirty: bool,
}

const ROLES: [&str; 4] = ["Owner", "Admin", "Member", "Viewer"];

impl Default for SettingsApp {
    fn default() -> Self {
        let m = |n: &str, e: &str, r: usize, p: bool| TeamMember { name: n.into(), email: e.into(), role: Some(r), pending: p };
        Self {
            route: "settings/appearance".into(),
            nav: vec![
                NavSection::new("Settings")
                    .item(NavItem::new("settings/profile", "Profile").icon(LineIcon::User))
                    .item(NavItem::new("settings/account", "Account").icon(LineIcon::Settings))
                    .item(NavItem::new("settings/notifications", "Notifications").icon(LineIcon::Bell))
                    .item(NavItem::new("settings/appearance", "Appearance").icon(AppIcon::Palette)),
                NavSection::new("Workspace")
                    .item(NavItem::new("settings/billing", "Billing").icon(LineIcon::CreditCard))
                    .item(NavItem::new("settings/team", "Team").icon(LineIcon::Users).badge("6"))
                    .item(NavItem::new("settings/security", "Security").icon(AppIcon::Shield)),
            ],
            notifications: demo_notes(),
            members: vec![
                m("Ada Lovelace", "ada@acme.dev", 0, false),
                m("Grace Hopper", "grace@acme.dev", 1, false),
                m("Alan Turing", "alan@acme.dev", 2, false),
                m("Mia Chen", "mia@acme.dev", 2, false),
                m("Liam Johnson", "liam@acme.dev", 3, false),
                m("Sofia Davis", "sofia@acme.dev", 2, true),
            ],
            scoped_theme: false,
            name: "Ada Lovelace".into(),
            username: "ada".into(),
            email: Some(0),
            bio: "Building analytical engines. Writing the first programs.".into(),
            website: "https://ada.dev".into(),
            birthday: Some(CalendarDate::new(1990, 12, 10)),
            language: Some(0),
            delete_open: false,
            notify: 1,
            switches: [true, false, true, true],
            mobile: true,
            radius: 10.0,
            density: 1,
            yearly: true,
            plan: 1,
            invite: String::new(),
            dirty: false,
        }
    }
}

impl SettingsApp {
    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        if let Some(it) = nav.get_mut(1).and_then(|s| s.items.get_mut(1)) {
            it.badge = Some(self.members.len().to_string());
        }
        AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::Sidebar)
            .workspaces(demo_workspaces("Acme Inc", "Enterprise"))
            .user(demo_user())
            .notifications(&mut notes)
            .scoped_theme(self.scoped_theme)
            .id_salt("settings_app")
            .show(ui, |ui, sc| self.page(ui, sc));
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
    }

    fn page(&mut self, ui: &mut Ui, sc: &mut ShellContext<'_>) {
        let t = Theme::of(ui);
        let rect = sc.rect;
        let route = sc.route.clone();
        let (title, sub) = match route.as_str() {
            "settings/profile" => ("Profile", "This is how others will see you on the site."),
            "settings/account" => ("Account", "Update your account settings. Set your preferred language and timezone."),
            "settings/notifications" => ("Notifications", "Configure how you receive notifications."),
            "settings/appearance" => ("Appearance", "Customize the look of the app. Changes apply instantly."),
            "settings/billing" => ("Billing", "Manage your plan, payment method and invoices."),
            "settings/team" => ("Team", "Invite teammates and manage their roles."),
            _ => ("Security", "Two-factor authentication, sessions and API access."),
        };
        let p = ui.painter().clone();
        kit::text_at(&p, rect.min, Align2::LEFT_TOP, title, 20.0, t.foreground, rect.width());
        kit::text_at(&p, rect.min + vec2(0.0, 30.0), Align2::LEFT_TOP, sub, 13.5, t.muted_foreground, rect.width());
        kit::hline(&p, rect.min.x, rect.max.x, rect.min.y + 66.0, &t);
        let id = ui.scope_id().with(("settings", route.as_str()));
        let loading = kit::loading(ui, id, 0.35);
        let body = Rect::from_min_max(pos2(rect.min.x, rect.min.y + 84.0), rect.max);
        if loading {
            for i in 0..5 {
                let r = Rect::from_min_size(body.min + vec2(0.0, i as f32 * 74.0), vec2(body.width().min(640.0), 60.0));
                kit::skel(ui, Rect::from_min_size(r.min, vec2(120.0, 10.0)));
                kit::skel_r(ui, Rect::from_min_size(r.min + vec2(0.0, 20.0), vec2(r.width(), 34.0)), t.radius * 0.6);
            }
            return;
        }
        let bounds = sc.bounds;
        kit::child(ui, body, |ui| {
            kit::scroll(ui, "settings_scroll", |ui| {
                ui.spacing_mut().item_spacing.y = 16.0;
                match route.as_str() {
                    "settings/profile" => self.profile(ui),
                    "settings/account" => self.account(ui, bounds),
                    "settings/notifications" => self.notifications_page(ui),
                    "settings/appearance" => self.appearance(ui, sc),
                    "settings/billing" => self.billing(ui),
                    "settings/team" => self.team(ui),
                    _ => {
                        ui.add(glowkit_components::EmptyState::new(UiIcon::Lock, "Security settings").description("Two-factor authentication and session management live here.").action("Enable 2FA").width(ui.available_width().min(520.0)));
                    }
                }
                ui.add_space(12.0);
            });
        });
    }

    fn save_row(&mut self, ui: &mut Ui, what: &str) {
        let w = ui.available_width().min(640.0);
        let r = kit::alloc_row(ui, 34.0);
        let br = Rect::from_min_size(r.min, vec2(140.0, 32.0));
        let _ = w;
        if ui.put(br, Button::new(format!("Update {what}")).size(ControlSize::Sm).width(140.0)).clicked() {
            self.dirty = false;
            toast(ui.ctx(), Toast::success(format!("{} updated", what[..1].to_uppercase() + &what[1..])).description("Your changes were saved."));
        }
    }

    fn profile(&mut self, ui: &mut Ui) {
        let t = Theme::of(ui);
        let w = ui.available_width().min(640.0);
        let r = kit::alloc_row(ui, 64.0);
        kit::avatar(ui, Rect::from_min_size(r.min, vec2(64.0, 64.0)), &self.name);
        let br = Rect::from_min_size(pos2(r.min.x + 80.0, r.min.y + 6.0), vec2(130.0, 32.0));
        if ui.put(br, Button::new("Change photo").icon(UiIcon::Upload).variant(ButtonVariant::Outline).size(ControlSize::Sm).width(130.0)).clicked() {
            toast(ui.ctx(), Toast::new("Choose a photo").description("PNG or JPG, up to 2 MB."));
        }
        kit::text_at(ui.painter(), pos2(r.min.x + 80.0, r.min.y + 52.0), Align2::LEFT_CENTER, "JPG, GIF or PNG. 2 MB max.", 12.0, t.muted_foreground, 260.0);
        let name_err = if self.name.trim().len() < 2 { Some("Name must be at least 2 characters.".to_string()) } else { None };
        ui.add(Input::new(&mut self.name).label("Display name").description("This is your public display name.").error_opt(name_err).width(w).id_salt("set_name"));
        ui.add(Input::new(&mut self.username).label("Username").prefix("acme.dev/").width(w).id_salt("set_user"));
        ui.add(Select::new(&mut self.email, ["ada@acme.dev", "ada.lovelace@gmail.com", "ada@analytical.engine"]).label("Email").width(w).id_salt("set_email"));
        ui.add(Textarea::new(&mut self.bio).label("Bio").description("You can @mention other users and organizations.").rows(3).max_len(160).width(w).id_salt("set_bio"));
        ui.add(Input::new(&mut self.website).label("Website").icon(UiIcon::Link).width(w).id_salt("set_web"));
        self.save_row(ui, "profile");
    }

    fn account(&mut self, ui: &mut Ui, bounds: Rect) {
        let t = Theme::of(ui);
        let w = ui.available_width().min(640.0);
        ui.add(Input::new(&mut self.name).label("Name").width(w).id_salt("acc_name"));
        ui.add(glowkit_components::FieldLabel::new("Date of birth"));
        ui.add(DatePicker::new(&mut self.birthday).width(260.0));
        ui.add(Select::new(&mut self.language, ["English", "Français", "Deutsch", "Español", "日本語"]).label("Language").width(260.0).id_salt("acc_lang"));
        self.save_row(ui, "account");
        // danger zone
        let r = kit::alloc_row(ui, 92.0);
        let r = Rect::from_min_size(r.min, vec2(w, 92.0));
        let p = ui.painter().clone();
        p.rect_filled(r, t.radius, with_alpha(t.destructive, 0.06));
        p.rect_stroke(r, t.radius, Stroke::new(1.0, with_alpha(t.destructive, 0.4)), StrokeKind::Inside);
        kit::text_at(&p, r.min + vec2(18.0, 26.0), Align2::LEFT_CENTER, "Delete account", 14.5, t.foreground, w - 200.0);
        let g = kit::wrapped(&p, "Permanently remove your account and all of its data. This cannot be undone.", 12.5, t.muted_foreground, w - 200.0, 2);
        p.galley(r.min + vec2(18.0, 44.0), g, t.muted_foreground);
        let br = Rect::from_min_size(pos2(r.max.x - 158.0, r.center().y - 16.0), vec2(140.0, 32.0));
        if ui.put(br, Button::new("Delete account").variant(ButtonVariant::Destructive).size(ControlSize::Sm).width(140.0)).clicked() {
            self.delete_open = true;
        }
        let res = AlertDialog::new(&mut self.delete_open, "Delete your account?").description("All projects, dashboards and API keys will be permanently deleted. This action cannot be undone.").confirm("Delete account").destructive(true).within(bounds).show(ui);
        if res.confirmed {
            toast(ui.ctx(), Toast::error("Account scheduled for deletion").description("You have 14 days to change your mind.").action("Undo"));
        }
    }

    fn notifications_page(&mut self, ui: &mut Ui) {
        let t = Theme::of(ui);
        let w = ui.available_width().min(640.0);
        ui.add(glowkit_components::FieldLabel::new("Notify me about…"));
        ui.add(RadioGroup::new(&mut self.notify, [RadioOption::new("All new messages", "Every message in every channel."), RadioOption::new("Direct messages and mentions", "Only when someone talks to you."), RadioOption::new("Nothing", "Turn off all notifications.")]));
        let r = kit::alloc_row(ui, 22.0);
        kit::text_at(ui.painter(), r.left_center(), Align2::LEFT_CENTER, "Email notifications", 15.0, t.foreground, w);
        let items = [
            ("Communication emails", "Receive emails about your account activity."),
            ("Marketing emails", "Receive emails about new products, features, and more."),
            ("Social emails", "Receive emails for friend requests, follows, and more."),
            ("Security emails", "Receive emails about your account activity and security."),
        ];
        for (i, (title, desc)) in items.iter().enumerate() {
            let r = kit::alloc_row(ui, 72.0);
            let r = Rect::from_min_size(r.min, vec2(w, 72.0));
            let p = ui.painter().clone();
            p.rect_stroke(r, t.radius, Stroke::new(1.0, t.border), StrokeKind::Inside);
            kit::text_at(&p, r.min + vec2(16.0, 26.0), Align2::LEFT_CENTER, title, 14.0, t.foreground, w - 100.0);
            kit::text_at(&p, r.min + vec2(16.0, 46.0), Align2::LEFT_CENTER, desc, 12.5, t.muted_foreground, w - 100.0);
            let sr = Rect::from_min_size(pos2(r.max.x - 60.0, r.center().y - 12.0), vec2(44.0, 24.0));
            let locked = i == 3;
            if ui.put(sr, Switch::new(&mut self.switches[i]).disabled(locked)).changed() {
                toast(ui.ctx(), Toast::new(format!("{title} {}", if self.switches[i] { "on" } else { "off" })));
            }
        }
        ui.add(Checkbox::new(&mut self.mobile).label("Use different settings for my mobile devices").description("You can manage your mobile notifications in the mobile settings page."));
        self.save_row(ui, "notifications");
    }

    fn appearance(&mut self, ui: &mut Ui, sc: &mut ShellContext<'_>) {
        let t = Theme::of(ui);
        let w = ui.available_width().min(760.0);
        let r = kit::alloc_row(ui, 18.0);
        kit::text_at(ui.painter(), r.left_center(), Align2::LEFT_CENTER, "Theme", 14.0, t.foreground, 200.0);
        let presets = Theme::presets();
        let cols = 5usize.min(((w + 12.0) / 140.0).floor().max(2.0) as usize);
        for chunk in presets.chunks(cols) {
            let row = kit::alloc_row(ui, 128.0);
            let row = Rect::from_min_size(row.min, vec2(w, 128.0));
            for (k, cr) in kit::cols(row, cols, 12.0).iter().enumerate() {
                let Some(th) = chunk.get(k) else { continue };
                let active = th.name == t.name;
                let cid = ui.scope_id().with(("theme", th.name));
                let resp = ui.interact(*cr, cid, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                let hv = kit::hover(ui, cid, resp.hovered());
                let p = ui.painter().clone();
                let prev = Rect::from_min_size(cr.min, vec2(cr.width(), 96.0));
                theme_preview(&p, prev, th);
                let ring = if active { t.ring } else { t.border.lerp_to_gamma(t.muted_foreground, hv * 0.6) };
                p.rect_stroke(prev.expand(if active { 2.0 } else { 0.0 }), t.radius + if active { 2.0 } else { 0.0 }, Stroke::new(if active { 2.0 } else { 1.0 }, ring), StrokeKind::Inside);
                if active {
                    let c = prev.right_top() + vec2(-14.0, 14.0);
                    p.circle_filled(c, 9.0, t.ring);
                    UiIcon::Check.paint(&p, Rect::from_center_size(c, vec2(11.0, 11.0)), Color32::WHITE);
                }
                kit::text_at(&p, pos2(cr.center().x, cr.max.y - 12.0), Align2::CENTER_CENTER, th.name, 13.0, if active { t.foreground } else { t.muted_foreground }, cr.width());
                if resp.clicked() && !active {
                    let mut nt = th.clone();
                    nt.radius = self.radius;
                    sc.set_theme(nt);
                    toast(ui.ctx(), Toast::success(format!("{} theme applied", th.name)));
                }
            }
        }
        ui.add_space(4.0);
        let r = kit::alloc_row(ui, 18.0);
        kit::text_at(ui.painter(), r.left_center(), Align2::LEFT_CENTER, "Corner radius", 14.0, t.foreground, 200.0);
        kit::text_at(ui.painter(), pos2(r.min.x + w, r.center().y), Align2::RIGHT_CENTER, &format!("{:.0} pt", self.radius), 12.5, t.muted_foreground, 100.0);
        let before = self.radius;
        let sr = kit::alloc_row(ui, 24.0);
        ui.put(Rect::from_min_size(sr.min, vec2(w, 24.0)), Slider::new(&mut self.radius, 0.0..=20.0).step(1.0).width(w));
        if (before - self.radius).abs() > 0.01 {
            let mut nt = t.clone();
            nt.radius = self.radius;
            sc.set_theme(nt);
        }
        let r = kit::alloc_row(ui, 18.0);
        kit::text_at(ui.painter(), r.left_center(), Align2::LEFT_CENTER, "Density", 14.0, t.foreground, 200.0);
        let dr = kit::alloc_row(ui, 32.0);
        ui.put(Rect::from_min_size(dr.min, vec2(300.0, 32.0)), SegmentedControl::new(&mut self.density, ["Compact", "Default", "Comfortable"]).size(ControlSize::Sm).width(300.0));
        let r = kit::alloc_row(ui, 40.0);
        let p = ui.painter().clone();
        let g = kit::wrapped(&p, "Themes are installed with Theme::install, so every glowkit component and stock egui widget restyles at once.", 12.5, t.muted_foreground, w, 2);
        p.galley(r.min, g, t.muted_foreground);
    }

    fn billing(&mut self, ui: &mut Ui) {
        let t = Theme::of(ui);
        let w = ui.available_width().min(900.0);
        let r = kit::alloc_row(ui, 28.0);
        kit::text_at(ui.painter(), r.left_center(), Align2::LEFT_CENTER, "Plans", 14.0, t.foreground, 200.0);
        let sr = Rect::from_min_size(pos2(r.min.x + w - 190.0, r.min.y + 2.0), vec2(190.0, 24.0));
        ui.put(sr, Switch::new(&mut self.yearly).label("Yearly (save 20%)").size(ControlSize::Sm));
        let plans = [("Free", 0.0, "For individuals getting started", &["3 projects", "1 GB storage", "Community support"][..]), ("Pro", 24.0, "For growing teams", &["Unlimited projects", "100 GB storage", "Priority support", "SSO"][..]), ("Enterprise", 79.0, "Advanced security and control", &["Everything in Pro", "Audit logs", "Dedicated manager", "99.99% SLA"][..])];
        let row = kit::alloc_row(ui, 270.0);
        let row = Rect::from_min_size(row.min, vec2(w, 270.0));
        for (i, cr) in kit::cols(row, 3, 14.0).iter().enumerate() {
            let (name, price, desc, feats) = plans[i];
            let cur = self.plan == i;
            let p = ui.painter().clone();
            if i == 1 {
                paint::soft_shadow(&p, *cr, t.radius, 18.0, vec2(0.0, 6.0), with_alpha(t.accent, 0.18));
            }
            p.rect_filled(*cr, t.radius, t.card);
            if i == 1 {
                let g = t.gradient;
                paint::gradient_border(&p, *cr, t.radius, 1.5, |u| glowkit_core::color::gradient(&[g[0], g[1], g[2], g[0]], u));
            } else {
                p.rect_stroke(*cr, t.radius, Stroke::new(1.0, t.border), StrokeKind::Inside);
            }
            kit::text_at(&p, cr.min + vec2(18.0, 26.0), Align2::LEFT_CENTER, name, 15.0, t.foreground, cr.width() - 120.0);
            if cur {
                ui.put(Rect::from_min_size(pos2(cr.max.x - 110.0, cr.min.y + 14.0), vec2(92.0, 24.0)), Badge::new("Current").variant(BadgeVariant::Accent));
            } else if i == 1 {
                ui.put(Rect::from_min_size(pos2(cr.max.x - 110.0, cr.min.y + 14.0), vec2(92.0, 24.0)), Badge::new("Popular").variant(BadgeVariant::Secondary));
            }
            kit::text_at(&p, cr.min + vec2(18.0, 46.0), Align2::LEFT_CENTER, desc, 12.0, t.muted_foreground, cr.width() - 36.0);
            let shown = if self.yearly { price * 0.8 } else { price };
            let pr = kit::text_at(&p, cr.min + vec2(18.0, 84.0), Align2::LEFT_CENTER, &format!("${shown:.0}"), 30.0, t.foreground, 140.0);
            kit::text_at(&p, pos2(pr.max.x + 6.0, cr.min.y + 90.0), Align2::LEFT_CENTER, "/ seat / month", 12.0, t.muted_foreground, 140.0);
            for (k, f) in feats.iter().enumerate() {
                let y = cr.min.y + 124.0 + k as f32 * 24.0;
                UiIcon::Check.paint(&p, Rect::from_center_size(pos2(cr.min.x + 26.0, y), vec2(14.0, 14.0)), t.success);
                kit::text_at(&p, pos2(cr.min.x + 40.0, y), Align2::LEFT_CENTER, f, 13.0, t.foreground, cr.width() - 56.0);
            }
            let br = Rect::from_min_size(pos2(cr.min.x + 16.0, cr.max.y - 48.0), vec2(cr.width() - 32.0, 32.0));
            let label = if cur { "Current plan" } else if i > self.plan { "Upgrade" } else { "Downgrade" };
            if ui.put(br, Button::new(label).variant(if cur { ButtonVariant::Outline } else if i == 1 { ButtonVariant::Default } else { ButtonVariant::Secondary }).size(ControlSize::Sm).width(br.width()).disabled(cur)).clicked() {
                self.plan = i;
                toast(ui.ctx(), Toast::success(format!("You're on {name}")).description(format!("Billed {}.", if self.yearly { "yearly" } else { "monthly" })));
            }
        }
        // payment method
        let r = kit::alloc_row(ui, 72.0);
        let r = Rect::from_min_size(r.min, vec2(w, 72.0));
        let p = ui.painter().clone();
        kit::card(&p, r, &t);
        let cr = Rect::from_min_size(r.min + vec2(16.0, 18.0), vec2(54.0, 36.0));
        paint::gradient_rect(&p, cr, 6.0, 30.0, &[mix(t.foreground, t.background, 0.3), mix(t.foreground, t.background, 0.55)]);
        kit::text_at(&p, cr.center(), Align2::CENTER_CENTER, "VISA", 11.0, t.background, 50.0);
        kit::text_at(&p, pos2(cr.max.x + 14.0, r.center().y - 8.0), Align2::LEFT_CENTER, "Visa ending in 4021", 14.0, t.foreground, 260.0);
        kit::text_at(&p, pos2(cr.max.x + 14.0, r.center().y + 10.0), Align2::LEFT_CENTER, "Expires 08/2028 · billing@acme.dev", 12.0, t.muted_foreground, 300.0);
        if ui.put(Rect::from_min_size(pos2(r.max.x - 96.0, r.center().y - 16.0), vec2(80.0, 32.0)), Button::new("Edit").variant(ButtonVariant::Outline).size(ControlSize::Sm).width(80.0)).clicked() {
            toast(ui.ctx(), Toast::new("Update payment method").description("A secure card form would open here."));
        }
        // invoices
        let r = kit::alloc_row(ui, 18.0);
        kit::text_at(ui.painter(), r.left_center(), Align2::LEFT_CENTER, "Invoices", 14.0, t.foreground, 200.0);
        for (k, (date, amount)) in [("Nov 1, 2026", "$192.00"), ("Oct 1, 2026", "$192.00"), ("Sep 1, 2026", "$168.00")].iter().enumerate() {
            let rr = kit::alloc_row(ui, 40.0);
            let rr = Rect::from_min_size(rr.min, vec2(w, 40.0));
            let (resp, _) = kit::row(ui, rr, ui.scope_id().with(("inv", k)), t.radius * 0.6);
            let p = ui.painter().clone();
            AppIcon::Receipt.paint(&p, Rect::from_center_size(pos2(rr.min.x + 18.0, rr.center().y), vec2(16.0, 16.0)), t.muted_foreground);
            kit::text_at(&p, pos2(rr.min.x + 40.0, rr.center().y), Align2::LEFT_CENTER, &format!("Invoice #{}", 2026_11 - k as i32), 13.5, t.foreground, 200.0);
            kit::text_at(&p, pos2(rr.min.x + w * 0.4, rr.center().y), Align2::LEFT_CENTER, date, 13.0, t.muted_foreground, 160.0);
            kit::status_pill(&p, pos2(rr.min.x + w * 0.62, rr.center().y), false, "Paid", t.success, &t);
            kit::text_at(&p, pos2(rr.max.x - 44.0, rr.center().y), Align2::RIGHT_CENTER, amount, 13.5, t.foreground, 100.0);
            UiIcon::Download.paint(&p, Rect::from_center_size(pos2(rr.max.x - 18.0, rr.center().y), vec2(15.0, 15.0)), t.muted_foreground);
            if resp.clicked() {
                toast(ui.ctx(), Toast::success("Invoice downloaded").description(format!("invoice-{}.pdf", 2026_11 - k as i32)));
            }
        }
    }

    fn team(&mut self, ui: &mut Ui) {
        let t = Theme::of(ui);
        let w = ui.available_width().min(760.0);
        let r = kit::alloc_row(ui, 34.0);
        ui.put(Rect::from_min_size(r.min, vec2(w - 130.0, 32.0)), Input::new(&mut self.invite).placeholder("Invite by email…").icon(UiIcon::Mail).size(ControlSize::Sm).width(w - 130.0).id_salt("team_invite"));
        let valid = self.invite.contains('@') && self.invite.contains('.');
        if ui.put(Rect::from_min_size(pos2(r.min.x + w - 120.0, r.min.y), vec2(120.0, 32.0)), Button::new("Send invite").icon(UiIcon::Plus).size(ControlSize::Sm).width(120.0).disabled(!valid)).clicked() {
            let email = self.invite.trim().to_string();
            let name = email.split('@').next().unwrap_or("new").to_string();
            self.members.push(TeamMember { name: name[..1].to_uppercase() + &name[1..], email: email.clone(), role: Some(2), pending: true });
            self.invite.clear();
            toast(ui.ctx(), Toast::success("Invitation sent").description(email));
        }
        let mut remove = None;
        let list = kit::alloc_row(ui, self.members.len() as f32 * 60.0);
        let list = Rect::from_min_size(list.min, vec2(w, list.height()));
        kit::card(ui.painter(), list, &t);
        for (i, m) in self.members.iter_mut().enumerate() {
            let rr = Rect::from_min_size(pos2(list.min.x, list.min.y + i as f32 * 60.0), vec2(w, 60.0));
            let p = ui.painter().clone();
            if i > 0 {
                p.line_segment([pos2(rr.min.x + 1.0, rr.min.y), pos2(rr.max.x - 1.0, rr.min.y)], Stroke::new(1.0, with_alpha(t.border, 0.7)));
            }
            kit::avatar(ui, Rect::from_center_size(pos2(rr.min.x + 34.0, rr.center().y), vec2(36.0, 36.0)), &m.name);
            let nr = kit::text_at(&p, pos2(rr.min.x + 62.0, rr.center().y - 8.0), Align2::LEFT_CENTER, &m.name, 14.0, t.foreground, w * 0.35);
            if m.pending {
                kit::pill(&p, pos2(nr.max.x + 8.0, nr.center().y), false, "Invited", t.warning, with_alpha(t.warning, 0.14), 10.5);
            }
            kit::text_at(&p, pos2(rr.min.x + 62.0, rr.center().y + 10.0), Align2::LEFT_CENTER, &m.email, 12.5, t.muted_foreground, w * 0.4);
            let owner = m.role == Some(0);
            let sel_r = Rect::from_min_size(pos2(rr.max.x - 200.0, rr.center().y - 16.0), vec2(140.0, 32.0));
            if owner {
                kit::text_at(&p, pos2(sel_r.min.x + 12.0, rr.center().y), Align2::LEFT_CENTER, "Owner", 13.0, t.muted_foreground, 120.0);
            } else {
                let before = m.role;
                ui.put(sel_r, Select::new(&mut m.role, ROLES).size(ControlSize::Sm).width(140.0).id_salt(("role", i)));
                if before != m.role {
                    toast(ui.ctx(), Toast::new(format!("{} is now {}", m.name, ROLES[m.role.unwrap_or(2)])));
                }
                let xr = Rect::from_center_size(pos2(rr.max.x - 30.0, rr.center().y), vec2(30.0, 30.0));
                let x = kit::icon_button(ui, xr, ui.scope_id().with(("rm", i)), AppIcon::Line(LineIcon::Trash), false);
                kit::tip(&x, "Remove member", None);
                if x.clicked() {
                    remove = Some(i);
                }
            }
        }
        if let Some(i) = remove {
            let m = self.members.remove(i);
            toast(ui.ctx(), Toast::new(format!("{} removed", m.name)).description("They lost access immediately.").action("Undo"));
        }
    }
}

/// A miniature app screenshot painted in `th`'s colors.
fn theme_preview(p: &egui::Painter, r: Rect, th: &Theme) {
    p.rect_filled(r, th.radius.min(12.0), th.background);
    let pc = p.with_clip_rect(r.shrink(1.0).intersect(p.clip_rect()));
    let side = Rect::from_min_size(r.min, vec2(r.width() * 0.28, r.height()));
    pc.rect_filled(side, egui::CornerRadius { nw: th.radius.min(12.0) as u8, sw: th.radius.min(12.0) as u8, ne: 0, se: 0 }, mix(th.background, th.foreground, 0.04));
    for i in 0..4 {
        pc.rect_filled(Rect::from_min_size(side.min + vec2(7.0, 10.0 + i as f32 * 12.0), vec2(side.width() - 14.0, 5.0)), 2.5, if i == 0 { th.accent } else { mix(th.background, th.foreground, 0.16) });
    }
    let main = Rect::from_min_max(pos2(side.max.x + 8.0, r.min.y + 9.0), r.max - vec2(8.0, 8.0));
    pc.rect_filled(Rect::from_min_size(main.min, vec2(main.width() * 0.5, 6.0)), 3.0, th.foreground);
    let card = Rect::from_min_max(pos2(main.min.x, main.min.y + 14.0), main.max);
    pc.rect_filled(card, 5.0, th.card);
    pc.rect_stroke(card, 5.0, Stroke::new(1.0, th.border), StrokeKind::Inside);
    let n = 6;
    let bw = (card.width() - 16.0) / n as f32;
    for i in 0..n {
        let h = (card.height() - 14.0) * (0.3 + 0.12 * ((i * 5) % 7) as f32);
        let b = Rect::from_min_max(pos2(card.min.x + 8.0 + i as f32 * bw + 1.5, card.max.y - 6.0 - h), pos2(card.min.x + 8.0 + (i + 1) as f32 * bw - 1.5, card.max.y - 6.0));
        pc.rect_filled(b, 2.0, th.chart[i % 6]);
    }
}
