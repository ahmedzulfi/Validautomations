//! Developer console template: usage, API keys and a live log stream.

use egui::{pos2, vec2, Align2, CursorIcon, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_charts::{AreaChart, BarChart, Series, Status, Tracker, TrackerBlock};
use glowkit_components::{toast, AlertDialog, Button, ButtonVariant, ControlSize, Dialog, Input, LineIcon, Select, Toast, ToggleGroup, UiIcon};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

use super::analytics::placeholder;
use super::common::{demo_notes, demo_user, page_header, series};
use crate::shells::kit::{self, Tone};
use crate::shells::{AppIcon, AppShell, NavItem, NavSection, ShellLayout, ShellNotification, Workspace};

/// An API key in the [`DevConsoleApp`].
#[derive(Clone, Debug, PartialEq)]
pub struct ApiKey {
    /// Key name.
    pub name: String,
    /// Secret (shown masked).
    pub secret: String,
    /// Scope label.
    pub scope: String,
    /// Created label.
    pub created: String,
    /// Last used label.
    pub last_used: String,
}

#[derive(Clone, Debug)]
struct LogLine {
    time: String,
    level: u8,
    method: &'static str,
    path: &'static str,
    status: u16,
    ms: u32,
}

/// A developer console inside a dual-sidebar [`AppShell`]: a Usage page (requests area chart,
/// error bars, latency / quota tiles, a 90-day uptime tracker), API keys (create dialog that
/// reveals the secret once, copy, revoke with confirmation) and a live log stream with level
/// filters, search, pause / resume and auto-follow.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::DevConsoleApp::default();
/// app.route = "logs".into();
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct DevConsoleApp {
    /// Active route ("usage", "keys", "logs").
    pub route: String,
    /// Navigation.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// API keys.
    pub keys: Vec<ApiKey>,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    logs: Vec<LogLine>,
    next_log: f64,
    seq: u64,
    paused: bool,
    levels: Vec<bool>,
    query: String,
    create_open: bool,
    new_name: String,
    new_scope: Option<usize>,
    revealed: Option<String>,
    revoke: Option<usize>,
    env: Option<usize>,
}

const ROUTES: [(&str, &str); 8] = [
    ("GET", "/v1/users"),
    ("POST", "/v1/charges"),
    ("GET", "/v1/charges/ch_3QxA"),
    ("GET", "/v1/projects"),
    ("PATCH", "/v1/projects/prj_42"),
    ("DELETE", "/v1/sessions/s_9"),
    ("POST", "/v1/webhooks/test"),
    ("GET", "/v1/usage?range=30d"),
];

fn make_log(seq: u64) -> LogLine {
    let mut rng = glowkit_core::Rng::new(seq * 7919 + 13);
    let (method, path) = ROUTES[rng.below(ROUTES.len())];
    let roll = rng.f32();
    let (level, status) = if roll < 0.08 { (2, [500u16, 502, 504][rng.below(3)]) } else if roll < 0.2 { (1, [404u16, 429, 401][rng.below(3)]) } else { (0, [200u16, 201, 204][rng.below(3)]) };
    let s = 10 * 3600 + 40 * 60 + seq as u32;
    LogLine { time: format!("{:02}:{:02}:{:02}.{:03}", s / 3600, (s / 60) % 60, s % 60, rng.below(1000)), level, method, path, status, ms: 12 + rng.below(if level == 2 { 2400 } else { 380 }) as u32 }
}

impl Default for DevConsoleApp {
    fn default() -> Self {
        let k = |n: &str, s: &str, sc: &str, c: &str, l: &str| ApiKey { name: n.into(), secret: s.into(), scope: sc.into(), created: c.into(), last_used: l.into() };
        Self {
            route: "usage".into(),
            nav: vec![
                NavSection::new("Project")
                    .icon(AppIcon::Server)
                    .item(NavItem::new("usage", "Usage").icon(LineIcon::Chart))
                    .item(NavItem::new("keys", "API keys").icon(AppIcon::Key))
                    .item(NavItem::new("logs", "Logs").icon(UiIcon::Terminal).badge("Live"))
                    .item(NavItem::new("webhooks", "Webhooks").icon(UiIcon::Link)),
                NavSection::new("Data").icon(AppIcon::Database).item(NavItem::new("tables", "Tables").icon(AppIcon::Database)).item(NavItem::new("storage", "Storage").icon(AppIcon::Archive)),
                NavSection::new("Settings").icon(LineIcon::Settings).item(NavItem::new("settings", "General").icon(LineIcon::Settings)).item(NavItem::new("members", "Members").icon(LineIcon::Users)),
            ],
            notifications: demo_notes(),
            keys: vec![
                k("Production server", "sk_live_51Qx8aKd2v9QfZ3m", "Full access", "Mar 2, 2026", "2 min ago"),
                k("CI pipeline", "sk_live_51Qx8bRt7p1LmW0c", "Read only", "Jun 18, 2026", "1 hour ago"),
                k("Analytics export", "sk_live_51Qx8cHn4s8KdV2e", "Usage: read", "Sep 9, 2026", "Yesterday"),
                k("Local development", "sk_test_51Qx8dPq0r3TgY7u", "Full access", "Oct 30, 2026", "Never"),
            ],
            scoped_theme: false,
            logs: (0..26).map(make_log).collect(),
            next_log: 0.0,
            seq: 26,
            paused: false,
            levels: vec![true, true, true],
            query: String::new(),
            create_open: false,
            new_name: String::new(),
            new_scope: Some(0),
            revealed: None,
            revoke: None,
            env: Some(0),
        }
    }
}

impl DevConsoleApp {
    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        // the log stream keeps flowing even when another page is open
        let now = ui.input(|i| i.time);
        if !self.paused {
            if now >= self.next_log {
                self.logs.push(make_log(self.seq));
                self.seq += 1;
                if self.logs.len() > 400 {
                    self.logs.remove(0);
                }
                self.next_log = now + 0.45;
            }
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
        }
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::Dual)
            .workspaces(vec![Workspace::new("acme-api", "Production"), Workspace::new("acme-api-staging", "Staging")])
            .user(demo_user())
            .notifications(&mut notes)
            .scoped_theme(self.scoped_theme)
            .id_salt("devconsole_app")
            .show(ui, |ui, sc| match sc.route.as_str() {
                "usage" => self.usage(ui, sc.rect),
                "keys" => self.keys_page(ui, sc.rect, sc.bounds),
                "logs" => self.logs_page(ui, sc.rect),
                other => placeholder(ui, sc.rect, other),
            });
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
    }

    fn usage(&mut self, ui: &mut Ui, rect: Rect) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("dev_usage");
        let loading = kit::loading(ui, id, 0.6);
        let top = page_header(ui, rect, "Usage", "API traffic for acme-api over the last 30 days.", |ui| {
            kit::put(ui, Rect::from_min_size(ui.cursor().min - vec2(150.0, 0.0), vec2(150.0, 32.0)), Select::new(&mut self.env, ["Production", "Staging", "All environments"]).size(ControlSize::Sm).width(150.0).id_salt("dev_env"));
        });
        let body = Rect::from_min_max(pos2(rect.min.x, top), rect.max);
        kit::child(ui, body, |ui| {
            kit::scroll(ui, "dev_usage_scroll", |ui| {
                ui.spacing_mut().item_spacing.y = 14.0;
                let row = kit::alloc_row(ui, 86.0);
                let tiles = [("Requests", "4.82M", "+12.4%", Tone::Success), ("Error rate", "0.42%", "-0.08 pts", Tone::Success), ("p95 latency", "184 ms", "+9 ms", Tone::Danger), ("Quota used", "48%", "of 10M / month", Tone::Neutral)];
                for (i, r) in kit::cols(row, 4, 14.0).iter().enumerate() {
                    if loading {
                        kit::skel_r(ui, *r, t.radius);
                        continue;
                    }
                    kit::card(ui.painter(), *r, &t);
                    let (l, v, d, tone) = tiles[i];
                    let p = ui.painter();
                    kit::text_at(p, r.min + vec2(16.0, 20.0), Align2::LEFT_CENTER, l, 12.5, t.muted_foreground, r.width() - 32.0);
                    let vr = kit::text_at(p, r.min + vec2(16.0, 52.0), Align2::LEFT_CENTER, v, 22.0, t.foreground, r.width() - 32.0);
                    kit::text_at(p, pos2(vr.max.x + 10.0, r.min.y + 55.0), Align2::LEFT_CENTER, d, 11.5, kit::tone(&t, tone), r.max.x - vr.max.x - 26.0);
                }
                let row = kit::alloc_row(ui, 290.0);
                let parts = kit::split_x(row, &[0.0, 360.0], 14.0);
                let labels: Vec<String> = (1..=30).map(|d| format!("Oct {d}")).collect();
                if loading {
                    parts.iter().for_each(|r| kit::skel_r(ui, *r, t.radius));
                } else {
                    kit::titled_card(ui, parts[0], "Requests", "Successful vs. failed, per day", |ui, b| {
                        let ok = series(3, 30, 140_000.0, 900.0, 26_000.0);
                        let s = [Series::new("2xx", ok), Series::new("4xx", series(4, 30, 9_000.0, 20.0, 3_000.0)), Series::new("5xx", series(5, 30, 600.0, 2.0, 500.0))];
                        ui.add(AreaChart::new(&s).labels(&labels).height(b.height()).id_salt("dev_req"));
                    });
                    kit::titled_card(ui, parts[1], "Errors by endpoint", "Last 7 days", |ui, b| {
                        let s = [Series::new("Errors", vec![412.0, 268.0, 190.0, 96.0, 41.0])];
                        ui.add(BarChart::new(&s).labels(&["/charges", "/users", "/webhooks", "/projects", "/usage"]).horizontal().color_by_category(true).value_labels(true).legend(false).height(b.height()).id_salt("dev_err"));
                    });
                }
                let row = kit::alloc_row(ui, 118.0);
                if loading {
                    kit::skel_r(ui, row, t.radius);
                } else {
                    kit::titled_card(ui, row, "Uptime", "99.982% over the last 90 days", |ui, b| {
                        let blocks: Vec<TrackerBlock> = (0..90)
                            .map(|i| {
                                let st = if i == 61 { Status::Down } else if i == 23 || i == 77 { Status::Warning } else { Status::Ok };
                                TrackerBlock::new(st, format!("Day {} · {}", i + 1, match st { Status::Down => "Partial outage", Status::Warning => "Degraded", _ => "Operational" }))
                            })
                            .collect();
                        ui.add(Tracker::new(&blocks).height(b.height().min(34.0)));
                    });
                }
            });
        });
    }

    fn keys_page(&mut self, ui: &mut Ui, rect: Rect, bounds: Rect) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("dev_keys");
        let loading = kit::loading(ui, id, 0.5);
        let mut create = false;
        let top = page_header(ui, rect, "API keys", "Secret keys authenticate requests from your servers. Keep them out of client code.", |ui| {
            create = ui.add(Button::new("Create key").icon(UiIcon::Plus).size(ControlSize::Sm)).clicked();
        });
        if create {
            self.create_open = true;
            self.new_name.clear();
            self.revealed = None;
        }
        let tr = Rect::from_min_max(pos2(rect.min.x, top + 4.0), pos2(rect.max.x, top + 4.0 + 40.0 + self.keys.len() as f32 * 58.0));
        kit::card(ui.painter(), tr, &t);
        let widths = [0.0, 260.0, 120.0, 120.0, 110.0, 76.0];
        let head = Rect::from_min_size(tr.min, vec2(tr.width(), 40.0));
        let p = ui.painter().clone();
        for (c, h) in kit::split_x(head.shrink2(vec2(16.0, 0.0)), &widths, 12.0).iter().zip(["Name", "Secret key", "Scope", "Created", "Last used", ""]) {
            kit::text_at(&p, c.left_center(), Align2::LEFT_CENTER, h, 12.0, t.muted_foreground, c.width());
        }
        kit::hline(&p, tr.min.x, tr.max.x, head.max.y, &t);
        for (i, k) in self.keys.iter().enumerate() {
            let rr = Rect::from_min_size(pos2(tr.min.x, head.max.y + i as f32 * 58.0), vec2(tr.width(), 58.0));
            let cells = kit::split_x(rr.shrink2(vec2(16.0, 0.0)), &widths, 12.0);
            if loading {
                kit::skel(ui, Rect::from_min_size(pos2(cells[0].min.x, rr.center().y - 5.0), vec2(140.0, 10.0)));
                kit::skel(ui, Rect::from_min_size(pos2(cells[1].min.x, rr.center().y - 5.0), vec2(200.0, 10.0)));
                continue;
            }
            if i > 0 {
                p.line_segment([pos2(tr.min.x + 1.0, rr.min.y), pos2(tr.max.x - 1.0, rr.min.y)], Stroke::new(1.0, with_alpha(t.border, 0.6)));
            }
            AppIcon::Key.paint(&p, Rect::from_center_size(pos2(cells[0].min.x + 8.0, rr.center().y), vec2(15.0, 15.0)), t.muted_foreground);
            kit::text_at(&p, pos2(cells[0].min.x + 26.0, rr.center().y), Align2::LEFT_CENTER, &k.name, 13.5, t.foreground, cells[0].width() - 28.0);
            let masked = format!("{}…{}", &k.secret[..8], &k.secret[k.secret.len() - 4..]);
            let mr = Rect::from_min_size(pos2(cells[1].min.x, rr.center().y - 13.0), vec2(cells[1].width() - 34.0, 26.0));
            p.rect_filled(mr, 6.0, t.muted);
            kit::mono_at(&p, pos2(mr.min.x + 10.0, mr.center().y), Align2::LEFT_CENTER, &masked, 12.0, t.foreground, mr.width() - 16.0);
            let cb = kit::icon_button(ui, Rect::from_center_size(pos2(mr.max.x + 18.0, rr.center().y), vec2(28.0, 28.0)), id.with(("copy", i)), AppIcon::Line(LineIcon::Copy), false);
            kit::tip(&cb, "Copy key", None);
            if cb.clicked() {
                ui.ctx().copy_text(k.secret.clone());
                toast(ui.ctx(), Toast::success("Key copied").description(format!("{} · keep it secret", k.name)));
            }
            let live = k.secret.starts_with("sk_live");
            kit::status_pill(&p, cells[2].left_center(), false, &k.scope, kit::tone(&t, if k.scope == "Full access" { if live { Tone::Warning } else { Tone::Info } } else { Tone::Neutral }), &t);
            kit::text_at(&p, cells[3].left_center(), Align2::LEFT_CENTER, &k.created, 12.5, t.muted_foreground, cells[3].width());
            kit::text_at(&p, cells[4].left_center(), Align2::LEFT_CENTER, &k.last_used, 12.5, t.muted_foreground, cells[4].width());
            if kit::put(ui, Rect::from_min_size(pos2(cells[5].max.x - 76.0, rr.center().y - 14.0), vec2(76.0, 28.0)), Button::new("Revoke").variant(ButtonVariant::Ghost).size(ControlSize::Sm).width(76.0)).clicked() {
                self.revoke = Some(i);
            }
        }
        // create dialog
        let mut open = self.create_open;
        let revealed = self.revealed.clone();
        let (name, scope) = (&mut self.new_name, &mut self.new_scope);
        let r = Dialog::new(&mut open, if revealed.is_some() { "Save your key" } else { "Create API key" })
            .description(if revealed.is_some() { "This is the only time the full key is shown. Store it in your secrets manager." } else { "Give the key a name so you can recognise it later." })
            .primary(if revealed.is_some() { "Done" } else { "Create key" })
            .width(460.0)
            .within(bounds)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                let w = ui.available_width();
                if let Some(sk) = &revealed {
                    let r = kit::alloc_row(ui, 40.0);
                    ui.painter().rect_filled(r, 8.0, t.muted);
                    ui.painter().rect_stroke(r, 8.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
                    kit::mono_at(ui.painter(), pos2(r.min.x + 12.0, r.center().y), Align2::LEFT_CENTER, sk, 12.5, t.foreground, r.width() - 60.0);
                    if kit::icon_button(ui, Rect::from_center_size(pos2(r.max.x - 22.0, r.center().y), vec2(28.0, 28.0)), ui.scope_id().with("dev_reveal_copy"), AppIcon::Line(LineIcon::Copy), false).clicked() {
                        ui.ctx().copy_text(sk.clone());
                        toast(ui.ctx(), Toast::success("Key copied"));
                    }
                } else {
                    ui.add(Input::new(name).label("Name").placeholder("e.g. Billing worker").width(w).id_salt("dev_key_name"));
                    ui.add(Select::new(scope, ["Full access", "Read only", "Usage: read"]).label("Permissions").width(w).id_salt("dev_key_scope"));
                }
            });
        if r.primary_clicked {
            if self.revealed.is_some() {
                self.revealed = None;
                open = false;
            } else {
                let n = if self.new_name.trim().is_empty() { "Untitled key".to_string() } else { self.new_name.trim().to_string() };
                let secret = format!("sk_live_51Qx{:x}{}", self.seq * 2654435761 % 0xffffffff, "Hn4sKd");
                self.keys.push(ApiKey { name: n.clone(), secret: secret.clone(), scope: ["Full access", "Read only", "Usage: read"][self.new_scope.unwrap_or(0)].into(), created: "Today".into(), last_used: "Never".into() });
                self.revealed = Some(secret);
                open = true;
                toast(ui.ctx(), Toast::success(format!("{n} created")));
            }
        }
        if !open {
            self.revealed = None;
        }
        self.create_open = open;
        // revoke confirm
        if let Some(i) = self.revoke {
            let mut o = true;
            let name = self.keys.get(i).map(|k| k.name.clone()).unwrap_or_default();
            let r = AlertDialog::new(&mut o, format!("Revoke “{name}”?")).description("Requests using this key will fail immediately. This cannot be undone.").confirm("Revoke key").destructive(true).within(bounds).show(ui);
            if r.confirmed {
                self.keys.remove(i);
                toast(ui.ctx(), Toast::success("Key revoked").description(name));
                o = false;
            }
            if !o {
                self.revoke = None;
            }
        }
    }

    fn logs_page(&mut self, ui: &mut Ui, rect: Rect) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("dev_logs");
        let p = ui.painter().clone();
        let time = glowkit_core::time(ui);
        // header
        kit::text_at(&p, rect.min, Align2::LEFT_TOP, "Logs", 22.0, t.foreground, 200.0);
        let live_x = rect.min.x + 64.0;
        let pulse = if self.paused { 0.0 } else { 0.5 + 0.5 * (time * 4.0).sin() };
        p.circle_filled(pos2(live_x, rect.min.y + 13.0), 4.0 + 3.0 * pulse, with_alpha(if self.paused { t.muted_foreground } else { t.success }, 0.25));
        p.circle_filled(pos2(live_x, rect.min.y + 13.0), 4.0, if self.paused { t.muted_foreground } else { t.success });
        kit::text_at(&p, pos2(live_x + 12.0, rect.min.y + 13.0), Align2::LEFT_CENTER, if self.paused { "Paused" } else { "Live" }, 12.5, t.muted_foreground, 80.0);
        let bar = Rect::from_min_size(pos2(rect.min.x, rect.min.y + 42.0), vec2(rect.width(), 32.0));
        kit::put(ui, Rect::from_min_size(bar.min, vec2(280.0, 32.0)), Input::new(&mut self.query).placeholder("Filter by path or status…").icon(UiIcon::Filter).clearable(true).size(ControlSize::Sm).width(280.0).id_salt("dev_log_q"));
        let mut lv = self.levels.clone();
        kit::put(ui, Rect::from_min_size(bar.min + vec2(292.0, 0.0), vec2(230.0, 32.0)), ToggleGroup::multi(&mut lv, ["Info", "Warn", "Error"]).size(ControlSize::Sm));
        self.levels = lv;
        let pr = Rect::from_min_size(pos2(bar.max.x - 110.0, bar.min.y), vec2(110.0, 32.0));
        let pb = kit::put(ui, pr, Button::new(if self.paused { "Resume" } else { "Pause" }).icon(if self.paused { UiIcon::ArrowRight } else { UiIcon::Clock }).variant(ButtonVariant::Outline).size(ControlSize::Sm).width(110.0));
        kit::tip(&pb, "Pause the stream", Some("P"));
        if pb.clicked() || (ui.input(|i| i.key_pressed(egui::Key::P)) && ui.memory(|m| m.focused().is_none())) {
            self.paused = !self.paused;
        }
        if kit::put(ui, Rect::from_min_size(pos2(pr.min.x - 88.0, bar.min.y), vec2(80.0, 32.0)), Button::new("Clear").variant(ButtonVariant::Ghost).size(ControlSize::Sm).width(80.0)).clicked() {
            self.logs.clear();
        }
        // console
        let con = Rect::from_min_max(pos2(rect.min.x, bar.max.y + 12.0), rect.max);
        let bg = if t.dark { glowkit_core::color::mix(t.background, t.foreground, 0.025) } else { glowkit_core::color::mix(t.background, t.foreground, 0.02) };
        p.rect_filled(con, t.radius, bg);
        p.rect_stroke(con, t.radius, Stroke::new(1.0, t.border), StrokeKind::Inside);
        let q = self.query.to_lowercase();
        let rows: Vec<&LogLine> = self.logs.iter().filter(|l| self.levels[l.level as usize] && (q.is_empty() || l.path.contains(&q) || l.status.to_string().contains(&q) || l.method.to_lowercase().contains(&q))).collect();
        kit::child(ui, con.shrink2(vec2(4.0, 6.0)), |ui| {
            egui::ScrollArea::vertical().id_salt("dev_log_scroll").auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for (k, l) in rows.iter().enumerate() {
                    let r = kit::alloc_row(ui, 24.0);
                    let rid = id.with(("l", k));
                    let resp = ui.interact(r, rid, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                    let hv = kit::hover(ui, rid, resp.hovered());
                    let pp = ui.painter();
                    let col = [t.muted_foreground, t.warning, t.destructive][l.level as usize];
                    if l.level == 2 {
                        pp.rect_filled(r, 0.0, with_alpha(t.destructive, 0.06));
                    }
                    if hv > 0.01 {
                        pp.rect_filled(r, 0.0, kit::hover_fill(&t, hv));
                    }
                    let y = r.center().y;
                    let mut x = r.min.x + 10.0;
                    kit::mono_at(pp, pos2(x, y), Align2::LEFT_CENTER, &l.time, 12.0, t.muted_foreground, 110.0);
                    x += 104.0;
                    kit::mono_at(pp, pos2(x, y), Align2::LEFT_CENTER, ["INFO", "WARN", "ERROR"][l.level as usize], 12.0, col, 60.0);
                    x += 56.0;
                    kit::mono_at(pp, pos2(x, y), Align2::LEFT_CENTER, &l.status.to_string(), 12.0, if l.level == 0 { t.success } else { col }, 40.0);
                    x += 40.0;
                    kit::mono_at(pp, pos2(x, y), Align2::LEFT_CENTER, l.method, 12.0, t.info, 60.0);
                    x += 60.0;
                    let msr = kit::mono_at(pp, pos2(r.max.x - 10.0, y), Align2::RIGHT_CENTER, &format!("{} ms", l.ms), 12.0, if l.ms > 1000 { t.warning } else { t.muted_foreground }, 80.0);
                    kit::mono_at(pp, pos2(x, y), Align2::LEFT_CENTER, l.path, 12.0, t.foreground, msr.min.x - x - 10.0);
                    if resp.clicked() {
                        toast(ui.ctx(), Toast::new(format!("{} {} → {}", l.method, l.path, l.status)).description(format!("{} · {} ms · req_{:06x}", l.time, l.ms, k * 7919)));
                    }
                }
                if rows.is_empty() {
                    let r = kit::alloc_row(ui, 120.0);
                    ui.painter().text(r.center(), Align2::CENTER_CENTER, "Waiting for requests…", egui::FontId::monospace(12.5), t.muted_foreground);
                }
            });
        });
    }
}
