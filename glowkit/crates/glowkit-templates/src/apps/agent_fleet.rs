//! Agent "fleet" manager: skills, plugins, MCP servers and projects with install / enable
//! toggles, MCP server health, search and a details drawer.

use egui::{pos2, vec2, Align2, Color32, FontId, Id, Key, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_components::{toast, Button, ButtonVariant, ControlSize, LineIcon, Switch, Toast, UiIcon};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

use super::common::{demo_notes, demo_user, demo_workspaces, page_header};
use super::prokit as pk;
use crate::shells::kit;
use crate::shells::{AppIcon, AppShell, NavItem, NavSection, ShellLayout, ShellNotification};

/// What a [`FleetItem`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FleetKind {
    /// A reusable instruction pack the agent loads on demand.
    Skill,
    /// A plugin adding commands, hooks or UI.
    Plugin,
    /// A Model Context Protocol server exposing tools.
    Mcp,
    /// A project (repository) the agents work in.
    Project,
}

/// Health of an MCP server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpHealth {
    /// Responding normally.
    Healthy,
    /// Slow or flapping.
    Degraded,
    /// Not responding; reconnecting.
    Down,
}

/// One entry in the [`AgentFleetApp`].
#[derive(Clone, Debug)]
pub struct FleetItem {
    /// Kind of entry.
    pub kind: FleetKind,
    /// Name ("rust-reviewer").
    pub name: String,
    /// Publisher.
    pub author: String,
    /// Installed (or latest) version.
    pub version: String,
    /// One-paragraph description.
    pub description: String,
    /// Short tags.
    pub tags: Vec<String>,
    /// Installed on this machine.
    pub installed: bool,
    /// Enabled for agents.
    pub enabled: bool,
    /// A newer version, if any.
    pub update: Option<String>,
    /// Install count.
    pub downloads: f64,
    /// Average rating (0..5).
    pub rating: f32,
    /// What it provides (tools, commands, hooks).
    pub provides: Vec<String>,
    /// Permissions it asks for.
    pub permissions: Vec<String>,
    /// MCP transport ("stdio", "http", "sse") or project path.
    pub transport: String,
    /// MCP health.
    pub health: McpHealth,
    latency: Vec<f64>,
    install_started: Option<f64>,
}

#[allow(clippy::too_many_arguments)]
fn item(
    kind: FleetKind,
    name: &str,
    author: &str,
    version: &str,
    description: &str,
    tags: &[&str],
    installed: bool,
    downloads: f64,
    rating: f32,
) -> FleetItem {
    FleetItem {
        kind,
        name: name.into(),
        author: author.into(),
        version: version.into(),
        description: description.into(),
        tags: tags.iter().map(|s| s.to_string()).collect(),
        installed,
        enabled: installed,
        update: None,
        downloads,
        rating,
        provides: Vec::new(),
        permissions: Vec::new(),
        transport: String::new(),
        health: McpHealth::Healthy,
        latency: Vec::new(),
        install_started: None,
    }
}

impl FleetItem {
    fn provides(mut self, p: &[&str]) -> Self {
        self.provides = p.iter().map(|s| s.to_string()).collect();
        self
    }
    fn perms(mut self, p: &[&str]) -> Self {
        self.permissions = p.iter().map(|s| s.to_string()).collect();
        self
    }
    fn update(mut self, v: &str) -> Self {
        self.update = Some(v.into());
        self
    }
    fn off(mut self) -> Self {
        self.enabled = false;
        self
    }
    fn mcp(mut self, transport: &str, health: McpHealth, base: f64) -> Self {
        self.transport = transport.into();
        self.health = health;
        self.latency = super::common::series(base as u64 + 7, 32, base, 0.0, base * 0.5);
        self
    }
}

fn default_items() -> Vec<FleetItem> {
    use FleetKind::*;
    vec![
        item(Skill, "rust-reviewer", "Tidepool Labs", "1.4.2", "Reviews diffs for unsafe blocks, unwraps in library code, semver breaks and missing docs, and leaves inline suggestions.", &["rust", "review"], true, 18_400.0, 4.8)
            .provides(&["/review", "pre-commit hook"]).perms(&["Read files", "Read git history"]).update("1.5.0"),
        item(Skill, "test-writer", "Quill", "0.9.0", "Writes table-driven unit tests and property tests for the function under the cursor, then runs them.", &["testing"], true, 32_100.0, 4.6)
            .provides(&["/tests"]).perms(&["Read files", "Write files", "Run shell commands"]),
        item(Skill, "changelog-scribe", "community", "2.1.0", "Turns merged pull requests into a grouped, human-readable changelog entry in your house style.", &["docs", "release"], true, 9_800.0, 4.4)
            .provides(&["/changelog"]).perms(&["Read git history"]).off(),
        item(Skill, "perf-sleuth", "Glint", "0.4.1", "Reads flame graphs and traces, finds the hot path and proposes measured fixes with before / after numbers.", &["perf", "profiling"], false, 4_200.0, 4.7)
            .provides(&["/profile"]).perms(&["Read files", "Run shell commands"]),
        item(Skill, "migration-guide", "Tidepool Labs", "1.0.3", "Plans dependency upgrades: reads release notes, lists breaking changes and applies the mechanical edits.", &["deps"], false, 7_600.0, 4.3)
            .provides(&["/upgrade"]).perms(&["Read files", "Write files", "Network access"]),
        item(Skill, "security-audit", "Northwind Sec", "3.2.0", "Checks for secrets in history, risky dependencies and injection-prone code paths; produces a findings report.", &["security"], false, 12_900.0, 4.9)
            .provides(&["/audit"]).perms(&["Read files", "Read git history", "Network access"]),
        item(Plugin, "worktrees", "Quill", "1.2.0", "Gives every agent session its own git worktree and cleans them up when the session is archived.", &["git"], true, 41_000.0, 4.9)
            .provides(&["/worktree", "session hook"]).perms(&["Write files", "Run shell commands"]),
        item(Plugin, "ci-watch", "community", "0.8.4", "Streams CI status into the session and asks the agent to fix failing jobs automatically.", &["ci"], true, 15_300.0, 4.5)
            .provides(&["/ci", "status line"]).perms(&["Network access"]).update("0.9.0"),
        item(Plugin, "desktop-notify", "Quill", "1.0.1", "Native notifications when an agent needs approval or finishes a long task.", &["ux"], true, 22_800.0, 4.7)
            .provides(&["notification hook"]).perms(&["Show notifications"]),
        item(Plugin, "screenshot-diff", "Pixelpond", "0.3.0", "Captures UI screenshots before and after a change and attaches a visual diff to the session.", &["ui", "testing"], false, 3_100.0, 4.2)
            .provides(&["/snap"]).perms(&["Run shell commands", "Write files"]),
        item(Mcp, "filesystem", "Quill", "2.0.0", "Scoped read / write access to project folders with an allow-list and size limits.", &["core"], true, 88_000.0, 4.9)
            .provides(&["read_file", "write_file", "list_dir", "search"]).perms(&["Read files", "Write files"]).mcp("stdio", McpHealth::Healthy, 6.0),
        item(Mcp, "postgres-readonly", "Tidepool Labs", "1.3.1", "Read-only SQL against a replica, with schema introspection and row limits.", &["data"], true, 14_500.0, 4.6)
            .provides(&["query", "describe_table", "list_schemas"]).perms(&["Network access"]).mcp("stdio", McpHealth::Healthy, 38.0),
        item(Mcp, "docs-search", "Lumen", "0.12.0", "Semantic search over your internal docs and API references.", &["search"], true, 9_900.0, 4.4)
            .provides(&["search_docs", "fetch_page"]).perms(&["Network access"]).mcp("http", McpHealth::Healthy, 120.0),
        item(Mcp, "browser", "community", "0.6.2", "Drives a headless browser: navigate, click, read the DOM and take screenshots.", &["web"], true, 27_300.0, 4.3)
            .provides(&["navigate", "click", "screenshot", "read_dom"]).perms(&["Network access", "Run shell commands"]).mcp("stdio", McpHealth::Degraded, 420.0),
        item(Mcp, "metrics-api", "Northwind", "2.4.0", "Queries dashboards and time series from your metrics backend.", &["observability"], true, 6_200.0, 4.1)
            .provides(&["query_series", "list_dashboards"]).perms(&["Network access"]).mcp("sse", McpHealth::Down, 90.0),
        item(Mcp, "vector-memory", "Lumen", "0.2.0", "Long-term memory for agents backed by a local vector index.", &["memory"], false, 2_400.0, 4.0)
            .provides(&["remember", "recall"]).perms(&["Write files"]).mcp("stdio", McpHealth::Healthy, 12.0),
        {
            let mut p = item(Project, "tidepool", "~/code/tidepool", "main", "Sync engine and server. 3 agents active; tests required before merge.", &["rust", "server"], true, 3.0, 0.0)
                .provides(&["rust-reviewer", "test-writer", "filesystem", "postgres-readonly"]);
            p.transport = "~/code/tidepool".into();
            p
        },
        {
            let mut p = item(Project, "lumen-desktop", "~/code/lumen-desktop", "main", "Desktop client built with egui. Screenshot diffs on UI changes.", &["rust", "ui"], true, 1.0, 0.0)
                .provides(&["rust-reviewer", "worktrees", "filesystem", "browser"]);
            p.transport = "~/code/lumen-desktop".into();
            p
        },
        {
            let mut p = item(Project, "glint-render", "~/code/glint-render", "dev", "GPU renderer. Agents may run benchmarks but never push.", &["rust", "gpu"], true, 1.0, 0.0)
                .provides(&["perf-sleuth", "filesystem"]);
            p.transport = "~/code/glint-render".into();
            p
        },
        {
            let mut p = item(Project, "ops-runbooks", "~/code/ops-runbooks", "main", "Incident runbooks and dashboards. Read-only for agents.", &["docs"], true, 0.0, 0.0)
                .provides(&["docs-search", "metrics-api"])
                .off();
            p.transport = "~/code/ops-runbooks".into();
            p
        },
    ]
}

/// A "fleet" manager for coding agents: browse and install skills and plugins, watch MCP
/// servers' health (live latency, degraded / down states with reconnects), and see which
/// projects use what. Cards have install buttons with progress and enable switches; a search
/// box and filter tabs narrow the grid; clicking a card opens a details drawer (Esc closes).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::AgentFleetApp::default();
/// app.route = "mcp".into(); // "skills" | "plugins" | "mcp" | "projects"
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct AgentFleetApp {
    /// Active route.
    pub route: String,
    /// Shell navigation.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// Everything in the fleet.
    pub items: Vec<FleetItem>,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    query: String,
    filter: usize,
    open: Option<usize>,
    open_t: f64,
    last_tick: f64,
}

impl Default for AgentFleetApp {
    fn default() -> Self {
        Self {
            route: "skills".into(),
            nav: Vec::new(),
            notifications: demo_notes(),
            items: default_items(),
            scoped_theme: false,
            query: String::new(),
            filter: 0,
            open: None,
            open_t: 0.0,
            last_tick: 0.0,
        }
    }
}

const FILTERS: [&str; 4] = ["All", "Installed", "Updates", "Disabled"];

fn kind_of(route: &str) -> Option<FleetKind> {
    match route {
        "skills" => Some(FleetKind::Skill),
        "plugins" => Some(FleetKind::Plugin),
        "mcp" => Some(FleetKind::Mcp),
        "projects" => Some(FleetKind::Project),
        _ => None,
    }
}

fn health_color(h: McpHealth, t: &Theme) -> Color32 {
    match h {
        McpHealth::Healthy => t.success,
        McpHealth::Degraded => t.warning,
        McpHealth::Down => t.destructive,
    }
}

fn tile_color(name: &str, t: &Theme) -> Color32 {
    let h = name.bytes().fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    t.chart[(h % 6) as usize]
}

impl AgentFleetApp {
    fn rebuild_nav(&mut self) {
        let count = |k: FleetKind| self.items.iter().filter(|i| i.kind == k && i.installed).count().to_string();
        let down = self
            .items
            .iter()
            .filter(|i| i.kind == FleetKind::Mcp && i.installed && i.enabled && i.health != McpHealth::Healthy)
            .count();
        let mut mcp = NavItem::new("mcp", "MCP servers").icon(AppIcon::Server);
        mcp = if down > 0 { mcp.badge(format!("{down} ⚠")) } else { mcp.badge(count(FleetKind::Mcp)) };
        self.nav = vec![
            NavSection::new("Fleet")
                .item(NavItem::new("skills", "Skills").icon(UiIcon::Sparkles).badge(count(FleetKind::Skill)))
                .item(NavItem::new("plugins", "Plugins").icon(AppIcon::Package).badge(count(FleetKind::Plugin)))
                .item(mcp)
                .item(NavItem::new("projects", "Projects").icon(LineIcon::Folder).badge(count(FleetKind::Project))),
            NavSection::new("Workspace")
                .item(NavItem::new("registry", "Registry").icon(LineIcon::Globe))
                .item(NavItem::new("policies", "Policies").icon(AppIcon::Shield))
                .item(NavItem::new("settings", "Settings").icon(LineIcon::Settings)),
        ];
    }

    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        if self.nav.is_empty() {
            self.rebuild_nav();
        }
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        let prev = route.clone();
        AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::Sidebar)
            .workspaces(demo_workspaces("Quill Fleet", "Team"))
            .user(demo_user())
            .notifications(&mut notes)
            .padding(0.0)
            .scoped_theme(self.scoped_theme)
            .id_salt("agent_fleet_app")
            .show(ui, |ui, sc| {
                if let Some(k) = kind_of(sc.route.as_str()) {
                    self.page(ui, sc.rect, k);
                } else {
                    super::analytics::placeholder(ui, sc.rect.shrink(24.0), sc.route);
                }
            });
        if route != prev {
            self.open = None;
            self.filter = 0;
        }
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
        self.rebuild_nav();
    }

    fn tick(&mut self, now: f64, ctx: &egui::Context) {
        // installs
        for it in &mut self.items {
            if let Some(t0) = it.install_started {
                if now - t0 > 2.4 {
                    it.install_started = None;
                    it.installed = true;
                    it.enabled = true;
                    toast(
                        ctx,
                        Toast::success(format!("{} installed", it.name))
                            .description(format!("v{} is enabled for all projects.", it.version)),
                    );
                }
            }
        }
        if now - self.last_tick < 0.8 {
            return;
        }
        self.last_tick = now;
        let step = (now / 0.8) as u64;
        for (k, it) in self.items.iter_mut().enumerate() {
            if it.kind != FleetKind::Mcp || it.latency.is_empty() {
                continue;
            }
            let base = it.latency.iter().sum::<f64>() / it.latency.len() as f64;
            let r = glowkit_core::rng::hash01(step * 31 + k as u64) as f64;
            let v = match it.health {
                McpHealth::Healthy => base * (0.75 + r * 0.5),
                McpHealth::Degraded => base * (0.5 + r * 1.4),
                McpHealth::Down => 0.0,
            };
            it.latency.remove(0);
            it.latency.push(v.max(0.5));
            // the "down" server recovers after a while, then the degraded one wobbles
            if it.health == McpHealth::Down && step % 24 == 23 {
                it.health = McpHealth::Healthy;
            } else if it.name == "metrics-api" && it.health == McpHealth::Healthy && step % 24 == 11 {
                it.health = McpHealth::Down;
            }
        }
    }

    fn visible(&self, kind: FleetKind) -> Vec<usize> {
        let q = self.query.to_lowercase();
        (0..self.items.len())
            .filter(|&i| {
                let it = &self.items[i];
                let f = match self.filter {
                    1 => it.installed,
                    2 => it.update.is_some(),
                    3 => it.installed && !it.enabled,
                    _ => true,
                };
                it.kind == kind
                    && f
                    && (q.is_empty()
                        || it.name.to_lowercase().contains(&q)
                        || it.description.to_lowercase().contains(&q)
                        || it.tags.iter().any(|t| t.contains(&q)))
            })
            .collect()
    }

    fn page(&mut self, ui: &mut Ui, rect: Rect, kind: FleetKind) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("fleet");
        let now = ui.input(|i| i.time);
        self.tick(now, &ui.ctx().clone());
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        let inner = rect.shrink2(vec2(24.0, 20.0));
        let (title, sub) = match kind {
            FleetKind::Skill => ("Skills", "Instruction packs your agents load on demand."),
            FleetKind::Plugin => ("Plugins", "Commands, hooks and status lines for agent sessions."),
            FleetKind::Mcp => ("MCP servers", "Tool servers your agents connect to, with live health."),
            FleetKind::Project => ("Projects", "Repositories and what each one's agents may use."),
        };
        let mut sync = false;
        let top = page_header(ui, inner, title, sub, |ui| {
            let b = ui.add(Button::new("Add from URL").icon(UiIcon::Plus).size(ControlSize::Sm));
            if b.clicked() {
                toast(
                    ui.ctx(),
                    Toast::new("Paste a registry URL").description("git, https or a local path are supported."),
                );
            }
            sync = ui
                .add(Button::new("Sync").icon(UiIcon::Download).variant(ButtonVariant::Outline).size(ControlSize::Sm))
                .clicked();
        });
        if sync {
            toast(ui.ctx(), Toast::success("Fleet synced").description("3 devices · 2 updates available"));
        }
        // toolbar
        let ty = top + 6.0;
        let sr = Rect::from_min_size(pos2(inner.min.x, ty), vec2(260.0, 32.0));
        pk::search(ui, sr, &mut self.query, &format!("Search {}…", title.to_lowercase()), "fleet_search");
        let tr = Rect::from_min_max(pos2(sr.max.x + 18.0, ty), pos2(sr.max.x + 18.0 + 300.0, ty + 32.0));
        pk::mini_tabs(ui, tr, id.with("filter"), &FILTERS, &mut self.filter);
        let vis = self.visible(kind);
        let p = ui.painter().clone();
        kit::text_at(
            &p,
            pos2(inner.max.x, ty + 16.0),
            Align2::RIGHT_CENTER,
            &format!("{} shown", vis.len()),
            12.0,
            t.muted_foreground,
            120.0,
        );
        kit::hline(&p, inner.min.x, inner.max.x, ty + 32.0, &t);
        // keyboard
        if self.open.is_some() && ui.input(|i| i.key_pressed(Key::Escape)) {
            self.open = None;
        }
        let gr = Rect::from_min_max(pos2(rect.min.x, ty + 44.0), rect.max);
        let mut clicked = None;
        kit::child(ui, gr, |ui| {
            egui::ScrollArea::vertical().id_salt(("fleet_grid", kind as u8)).auto_shrink([false, false]).show(
                ui,
                |ui| {
                    let w = ui.available_width();
                    let cw = w - 48.0;
                    let ncol = if cw > 820.0 {
                        3
                    } else if cw > 520.0 {
                        2
                    } else {
                        1
                    };
                    let card_h = match kind {
                        FleetKind::Mcp => 214.0,
                        FleetKind::Project => 172.0,
                        _ => 176.0,
                    };
                    if vis.is_empty() {
                        let (er, _) = ui.allocate_exact_size(vec2(w, 200.0), Sense::hover());
                        let er = Rect::from_center_size(er.center(), vec2(360.0, 160.0));
                        kit::child(ui, er, |ui| {
                            ui.add(
                                glowkit_components::EmptyState::new(UiIcon::Search, "Nothing matches")
                                    .description("Try another search or filter.")
                                    .width(360.0),
                            );
                        });
                        return;
                    }
                    for chunk in vis.chunks(ncol) {
                        let (row, _) = ui.allocate_exact_size(vec2(w, card_h + 14.0), Sense::hover());
                        let row = Rect::from_min_max(
                            pos2(row.min.x + 24.0, row.min.y + 2.0),
                            pos2(row.max.x - 24.0, row.max.y - 12.0),
                        );
                        let cells = kit::cols(row, ncol, 14.0);
                        for (k, &i) in chunk.iter().enumerate() {
                            if self.card(ui, cells[k], id.with(("card", i)), i, now) {
                                clicked = Some(i);
                            }
                        }
                    }
                    ui.add_space(12.0);
                },
            );
        });
        if let Some(i) = clicked {
            self.open = Some(i);
            self.open_t = now;
        }
        // drawer
        self.drawer(ui, rect, id.with("drawer"), now);
    }

    /// Paint one card; returns true when the card body was clicked.
    fn card(&mut self, ui: &mut Ui, r: Rect, id: Id, i: usize, now: f64) -> bool {
        let t = Theme::of(ui);
        let resp = ui.interact(r, id, Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let hv = kit::hover(ui, id, resp.hovered());
        let open = self.open == Some(i);
        let p = ui.painter().clone();
        let lift = hv * 2.0;
        let r = r.translate(vec2(0.0, -lift));
        if hv > 0.01 {
            paint::soft_shadow(
                &p,
                r,
                t.radius,
                14.0 * hv,
                vec2(0.0, 4.0 * hv),
                with_alpha(Color32::BLACK, if t.dark { 0.4 } else { 0.08 } * hv),
            );
        }
        p.rect_filled(r, t.radius, t.card);
        let border = if open { t.ring } else { t.border.lerp_to_gamma(mix(t.border, t.foreground, 0.25), hv) };
        p.rect_stroke(r, t.radius, Stroke::new(1.0, border), StrokeKind::Inside);
        let it = self.items[i].clone();
        let x0 = r.min.x + 16.0;
        let x1 = r.max.x - 16.0;
        // icon tile
        let tile = Rect::from_min_size(pos2(x0, r.min.y + 16.0), vec2(36.0, 36.0));
        let tc = tile_color(&it.name, &t);
        p.rect_filled(tile, 9.0, pk::soft(&t, tc, 1.6));
        p.rect_stroke(tile, 9.0, Stroke::new(1.0, with_alpha(tc, 0.3)), StrokeKind::Inside);
        let icon: AppIcon = match it.kind {
            FleetKind::Skill => UiIcon::Sparkles.into(),
            FleetKind::Plugin => AppIcon::Package,
            FleetKind::Mcp => AppIcon::Server,
            FleetKind::Project => LineIcon::Folder.into(),
        };
        icon.paint(&p, tile.shrink(9.0), pk::tone_fg(&t, tc));
        // right control
        let ctrl_x = x1;
        let cy = r.min.y + 26.0;
        let mut ctrl_w = 0.0;
        if it.install_started.is_some() {
            ctrl_w = 70.0;
            let e = ((now - it.install_started.unwrap_or(now)) / 2.4).clamp(0.0, 1.0) as f32;
            kit::text_at(
                &p,
                pos2(ctrl_x, cy),
                Align2::RIGHT_CENTER,
                &format!("{:.0}%", e * 100.0),
                11.5,
                t.muted_foreground,
                60.0,
            );
        } else if it.installed {
            ctrl_w = 40.0;
            let sw = Rect::from_min_size(pos2(ctrl_x - 36.0, cy - 10.0), vec2(36.0, 20.0));
            let mut on = it.enabled;
            let s = kit::put(ui, sw, Switch::new(&mut on));
            kit::tip(&s, if on { "Disable" } else { "Enable" }, None);
            if on != it.enabled {
                self.items[i].enabled = on;
                toast(ui.ctx(), Toast::new(format!("{} {}", it.name, if on { "enabled" } else { "disabled" })));
            }
        } else if it.kind != FleetKind::Project {
            ctrl_w = 76.0;
            let br = Rect::from_min_max(pos2(ctrl_x - 72.0, cy - 13.0), pos2(ctrl_x, cy + 13.0));
            if pk::small_btn(
                ui,
                br,
                id.with("install"),
                "Install",
                Some(UiIcon::Download.into()),
                Some(t.primary),
                None,
            )
            .clicked()
            {
                self.items[i].install_started = Some(now);
            }
        }
        let tx = tile.max.x + 12.0;
        let tw = ctrl_x - ctrl_w - tx - 8.0;
        let nr = kit::text_at(&p, pos2(tx, r.min.y + 24.0), Align2::LEFT_CENTER, &it.name, 14.0, t.foreground, tw);
        if it.kind == FleetKind::Mcp && it.installed {
            let hc = if it.enabled { health_color(it.health, &t) } else { t.muted_foreground };
            pk::dot(
                &p,
                pos2(nr.max.x + 9.0, nr.center().y),
                3.5,
                hc,
                (it.enabled && it.health != McpHealth::Healthy).then_some(now),
            );
        }
        let meta = match it.kind {
            FleetKind::Project => it.transport.clone(),
            _ => format!("{} · v{}", it.author, it.version),
        };
        if it.kind == FleetKind::Project {
            pk::mono(&p, pos2(tx, r.min.y + 43.0), Align2::LEFT_CENTER, &meta, 11.0, t.muted_foreground, tw);
        } else {
            kit::text_at(&p, pos2(tx, r.min.y + 43.0), Align2::LEFT_CENTER, &meta, 11.5, t.muted_foreground, tw);
        }
        // description
        let g = kit::wrapped(&p, &it.description, 12.5, t.muted_foreground, r.width() - 32.0, 2);
        p.galley(pos2(x0, r.min.y + 64.0), g, t.muted_foreground);
        // middle band
        let mut y = r.min.y + 112.0;
        match it.kind {
            FleetKind::Mcp => {
                let band = Rect::from_min_max(pos2(x0, y), pos2(x1, y + 40.0));
                p.rect_filled(band, 7.0, mix(t.card, t.foreground, if t.dark { 0.04 } else { 0.025 }));
                if !it.installed {
                    kit::text_at(
                        &p,
                        band.center(),
                        Align2::CENTER_CENTER,
                        "Not installed",
                        11.5,
                        t.muted_foreground,
                        band.width(),
                    );
                } else if !it.enabled {
                    kit::text_at(
                        &p,
                        band.center(),
                        Align2::CENTER_CENTER,
                        "Stopped",
                        11.5,
                        t.muted_foreground,
                        band.width(),
                    );
                } else if it.health == McpHealth::Down {
                    let left = 12 - ((now / 0.8) as u64 % 12);
                    pk::dot(&p, pos2(band.min.x + 14.0, band.center().y), 3.0, t.destructive, Some(now));
                    kit::text_at(
                        &p,
                        pos2(band.min.x + 26.0, band.center().y),
                        Align2::LEFT_CENTER,
                        &format!("Connection refused · retrying in {left}s"),
                        11.5,
                        pk::tone_fg(&t, t.destructive),
                        band.width() - 36.0,
                    );
                } else {
                    let last = *it.latency.last().unwrap_or(&0.0);
                    let p50 = {
                        let mut v = it.latency.clone();
                        v.sort_by(|a, b| a.total_cmp(b));
                        v[v.len() / 2]
                    };
                    let lr = kit::text_at(
                        &p,
                        pos2(band.min.x + 10.0, band.min.y + 13.0),
                        Align2::LEFT_CENTER,
                        &format!("{last:.0} ms"),
                        12.5,
                        t.foreground,
                        80.0,
                    );
                    kit::text_at(
                        &p,
                        pos2(band.min.x + 10.0, band.min.y + 29.0),
                        Align2::LEFT_CENTER,
                        &format!("p50 {p50:.0} ms"),
                        10.5,
                        t.muted_foreground,
                        80.0,
                    );
                    let sr = Rect::from_min_max(
                        pos2(lr.min.x + 84.0, band.min.y + 8.0),
                        pos2(band.max.x - 10.0, band.max.y - 8.0),
                    );
                    pk::spark(&p, sr, &it.latency, health_color(it.health, &t), true);
                }
                y += 50.0;
            }
            FleetKind::Project => {
                let mut x = x0;
                for s in &it.provides {
                    let w = kit::text_w(&p, s, 10.5) + 10.0;
                    if x + w > x1 {
                        break;
                    }
                    let ic = self.items.iter().find(|o| &o.name == s).map_or(t.primary, |o| tile_color(&o.name, &t));
                    let tr = pk::tag(&p, pos2(x, y + 8.0), false, s, ic, &t, false);
                    x = tr.max.x + 6.0;
                }
                y += 22.0;
            }
            _ => {}
        }
        // tags
        let mut x = x0;
        if it.kind != FleetKind::Project {
            for tg in &it.tags {
                let tr = pk::tag(&p, pos2(x, y + 8.0), false, tg, t.muted_foreground, &t, false);
                x = tr.max.x + 6.0;
            }
            if let Some(u) = &it.update {
                pk::tag(&p, pos2(x, y + 8.0), false, &format!("Update {u}"), t.info, &t, false);
            }
        }
        // footer
        let fy = r.max.y - 20.0;
        kit::hline(&p, r.min.x + 1.0, r.max.x - 1.0, fy - 16.0, &t);
        match it.kind {
            FleetKind::Project => {
                let agents = it.downloads as usize;
                let col = if agents > 0 { t.success } else { t.muted_foreground };
                pk::dot(&p, pos2(x0 + 4.0, fy), 3.5, col, (agents > 0).then_some(now));
                kit::text_at(
                    &p,
                    pos2(x0 + 14.0, fy),
                    Align2::LEFT_CENTER,
                    &format!("{agents} agent{} running", if agents == 1 { "" } else { "s" }),
                    11.5,
                    t.muted_foreground,
                    140.0,
                );
                kit::text_at(
                    &p,
                    pos2(x1, fy),
                    Align2::RIGHT_CENTER,
                    if it.enabled { "Agents allowed" } else { "Paused" },
                    11.5,
                    t.muted_foreground,
                    120.0,
                );
            }
            _ => {
                AppIcon::Ui(UiIcon::Download).paint(
                    &p,
                    Rect::from_center_size(pos2(x0 + 6.0, fy), vec2(12.0, 12.0)),
                    t.muted_foreground,
                );
                let dr = kit::text_at(
                    &p,
                    pos2(x0 + 16.0, fy),
                    Align2::LEFT_CENTER,
                    &glowkit_charts::format_compact(it.downloads),
                    11.5,
                    t.muted_foreground,
                    60.0,
                );
                AppIcon::Ui(UiIcon::Star).paint(
                    &p,
                    Rect::from_center_size(pos2(dr.max.x + 18.0, fy), vec2(12.0, 12.0)),
                    t.warning,
                );
                kit::text_at(
                    &p,
                    pos2(dr.max.x + 28.0, fy),
                    Align2::LEFT_CENTER,
                    &format!("{:.1}", it.rating),
                    11.5,
                    t.muted_foreground,
                    40.0,
                );
                let right = if it.kind == FleetKind::Mcp {
                    format!("{} · {} tools", it.transport, it.provides.len())
                } else {
                    format!("{} provided", it.provides.len())
                };
                kit::text_at(&p, pos2(x1, fy), Align2::RIGHT_CENTER, &right, 11.5, t.muted_foreground, 140.0);
            }
        }
        // install progress strip
        if let Some(t0) = it.install_started {
            let e = ((now - t0) / 2.4).clamp(0.0, 1.0) as f32;
            let e = 1.0 - (1.0 - e).powi(2);
            let pr = Rect::from_min_max(pos2(r.min.x + 1.0, r.max.y - 4.0), pos2(r.max.x - 1.0, r.max.y - 1.0));
            p.rect_filled(Rect::from_min_size(pr.min, vec2(pr.width() * e, pr.height())), 2.0, t.primary);
            ui.ctx().request_repaint();
        }
        resp.clicked()
    }

    fn drawer(&mut self, ui: &mut Ui, rect: Rect, id: Id, now: f64) {
        let target = if self.open.is_some() { 1.0 } else { 0.0 };
        let k = kit::spring(ui.ctx(), id.with("k"), target, 260.0, 28.0);
        if k < 0.005 {
            return;
        }
        let t = Theme::of(ui);
        let p = ui.painter().clone();
        let w = 380.0f32.min(rect.width() - 40.0);
        let x = rect.max.x - w * k.min(1.0);
        let r = Rect::from_min_max(pos2(x, rect.min.y), pos2(x + w, rect.max.y));
        p.rect_filled(rect, 0.0, with_alpha(Color32::BLACK, if t.dark { 0.35 } else { 0.1 } * k.clamp(0.0, 1.0)));
        let scrim =
            ui.interact(Rect::from_min_max(rect.min, pos2(r.min.x, rect.max.y)), id.with("scrim"), Sense::click());
        if scrim.clicked() {
            self.open = None;
        }
        // eat clicks inside the drawer
        let _ = ui.interact(r, id.with("body"), Sense::click());
        paint::soft_shadow(&p, r, 0.0, 22.0, vec2(-6.0, 0.0), kit::shadow(&t));
        p.rect_filled(r, 0.0, t.popover);
        kit::vline(&p, r.min.x, r.min.y, r.max.y, &t);
        let Some(i) = self.open.or(self.last_open(id, ui)) else { return };
        if self.open.is_some() {
            glowkit_core::set_state(ui, id.with("last"), i);
        }
        let it = self.items[i].clone();
        let x0 = r.min.x + 22.0;
        let x1 = r.max.x - 22.0;
        let tile = Rect::from_min_size(pos2(x0, r.min.y + 22.0), vec2(44.0, 44.0));
        let tc = tile_color(&it.name, &t);
        p.rect_filled(tile, 11.0, pk::soft(&t, tc, 1.6));
        let icon: AppIcon = match it.kind {
            FleetKind::Skill => UiIcon::Sparkles.into(),
            FleetKind::Plugin => AppIcon::Package,
            FleetKind::Mcp => AppIcon::Server,
            FleetKind::Project => LineIcon::Folder.into(),
        };
        icon.paint(&p, tile.shrink(11.0), pk::tone_fg(&t, tc));
        let close = pk::icon_btn(
            ui,
            Rect::from_min_size(pos2(x1 - 28.0, r.min.y + 18.0), vec2(28.0, 28.0)),
            id.with("close"),
            LineIcon::X.into(),
            "Close",
            Some("Esc"),
        );
        if close.clicked() {
            self.open = None;
        }
        kit::text_at(
            &p,
            pos2(tile.max.x + 14.0, tile.min.y + 13.0),
            Align2::LEFT_CENTER,
            &it.name,
            17.0,
            t.foreground,
            x1 - tile.max.x - 50.0,
        );
        let meta = if it.kind == FleetKind::Project {
            it.transport.clone()
        } else {
            format!("{} · v{}", it.author, it.version)
        };
        kit::text_at(
            &p,
            pos2(tile.max.x + 14.0, tile.min.y + 34.0),
            Align2::LEFT_CENTER,
            &meta,
            12.0,
            t.muted_foreground,
            x1 - tile.max.x - 20.0,
        );
        let mut y = tile.max.y + 20.0;
        // actions
        if it.installed {
            let sw = Rect::from_min_size(pos2(x0, y), vec2(150.0, 22.0));
            let mut on = it.enabled;
            kit::child(ui, sw, |ui| {
                ui.add(Switch::new(&mut on).label(if it.kind == FleetKind::Project {
                    "Agents allowed"
                } else {
                    "Enabled"
                }))
            });
            if on != it.enabled {
                self.items[i].enabled = on;
            }
            let mut bx = x1;
            if it.kind != FleetKind::Project {
                let br = Rect::from_min_max(pos2(bx - 96.0, y - 4.0), pos2(bx, y + 26.0));
                if pk::small_btn(ui, br, id.with("rm"), "Uninstall", None, None, None).clicked() {
                    self.items[i].installed = false;
                    self.items[i].enabled = false;
                    toast(ui.ctx(), Toast::new(format!("{} uninstalled", it.name)));
                }
                bx = br.min.x - 8.0;
            }
            if let Some(u) = &it.update {
                let br = Rect::from_min_max(pos2(bx - 100.0, y - 4.0), pos2(bx, y + 26.0));
                if pk::small_btn(ui, br, id.with("up"), &format!("Update {u}"), None, Some(t.primary), None).clicked() {
                    self.items[i].version = u.clone();
                    self.items[i].update = None;
                    toast(ui.ctx(), Toast::success(format!("{} updated to {u}", it.name)));
                }
            } else if it.kind == FleetKind::Mcp {
                let br = Rect::from_min_max(pos2(bx - 86.0, y - 4.0), pos2(bx, y + 26.0));
                if pk::small_btn(ui, br, id.with("rs"), "Restart", Some(AppIcon::Refresh), None, None).clicked() {
                    self.items[i].health = McpHealth::Healthy;
                    toast(ui.ctx(), Toast::success(format!("{} restarted", it.name)));
                }
            }
        } else if self.items[i].install_started.is_none() {
            let br = Rect::from_min_max(pos2(x0, y - 4.0), pos2(x1, y + 28.0));
            if pk::small_btn(
                ui,
                br,
                id.with("inst"),
                &format!("Install {}", it.name),
                Some(UiIcon::Download.into()),
                Some(t.primary),
                None,
            )
            .clicked()
            {
                self.items[i].install_started = Some(now);
            }
        } else {
            let e = ((now - self.items[i].install_started.unwrap_or(now)) / 2.4).clamp(0.0, 1.0) as f32;
            kit::text_at(&p, pos2(x0, y + 4.0), Align2::LEFT_CENTER, "Installing…", 12.0, t.foreground, 120.0);
            pk::meter(&p, Rect::from_min_max(pos2(x0, y + 16.0), pos2(x1, y + 21.0)), e, t.primary, &t);
        }
        y += 44.0;
        kit::hline(&p, r.min.x, r.max.x, y, &t);
        y += 18.0;
        let g = kit::wrapped(&p, &it.description, 13.0, t.foreground, x1 - x0, 5);
        let gh = g.size().y;
        p.galley(pos2(x0, y), g, t.foreground);
        y += gh + 20.0;
        // health
        if it.kind == FleetKind::Mcp && it.installed {
            pk::label(&p, pos2(x0, y), "Health", &t, 120.0);
            let hc = health_color(it.health, &t);
            kit::status_pill(
                &p,
                pos2(x1, y),
                true,
                match it.health {
                    McpHealth::Healthy => "Healthy",
                    McpHealth::Degraded => "Degraded",
                    McpHealth::Down => "Down",
                },
                hc,
                &t,
            );
            y += 18.0;
            let sr = Rect::from_min_max(pos2(x0, y), pos2(x1, y + 48.0));
            pk::spark(&p, sr, &it.latency, hc, true);
            y += 60.0;
        }
        let section = |p: &egui::Painter, y: &mut f32, title: &str, rows: &[String], icon: AppIcon| {
            pk::label(p, pos2(x0, *y), title, &t, 200.0);
            *y += 20.0;
            for s in rows {
                icon.paint(p, Rect::from_center_size(pos2(x0 + 7.0, *y + 2.0), vec2(13.0, 13.0)), t.muted_foreground);
                kit::text_at(p, pos2(x0 + 22.0, *y + 2.0), Align2::LEFT_CENTER, s, 12.5, t.foreground, x1 - x0 - 24.0);
                *y += 24.0;
            }
            *y += 10.0;
        };
        let prov_title = match it.kind {
            FleetKind::Mcp => "Tools",
            FleetKind::Project => "Uses",
            _ => "Provides",
        };
        section(
            &p,
            &mut y,
            prov_title,
            &it.provides,
            if it.kind == FleetKind::Mcp { AppIcon::Ui(UiIcon::Terminal) } else { AppIcon::Ui(UiIcon::Check) },
        );
        if !it.permissions.is_empty() {
            section(&p, &mut y, "Permissions", &it.permissions, AppIcon::Shield);
        }
        if it.kind == FleetKind::Mcp {
            pk::label(&p, pos2(x0, y), "Configuration", &t, 200.0);
            y += 18.0;
            let cr = Rect::from_min_max(pos2(x0, y), pos2(x1, y + 64.0));
            p.rect_filled(cr, 7.0, mix(t.popover, t.foreground, 0.035));
            let cmd = match it.transport.as_str() {
                "stdio" => format!("command  quill-mcp-{}", it.name),
                _ => format!("url      https://mcp.internal/{}", it.name),
            };
            pk::mono(
                &p,
                pos2(cr.min.x + 10.0, cr.min.y + 16.0),
                Align2::LEFT_CENTER,
                &format!("transport {}", it.transport),
                11.5,
                t.foreground,
                cr.width() - 20.0,
            );
            pk::mono(
                &p,
                pos2(cr.min.x + 10.0, cr.min.y + 32.0),
                Align2::LEFT_CENTER,
                &cmd,
                11.5,
                t.foreground,
                cr.width() - 20.0,
            );
            pk::mono(
                &p,
                pos2(cr.min.x + 10.0, cr.min.y + 48.0),
                Align2::LEFT_CENTER,
                "env      API_TOKEN=••••••••3f2a",
                11.5,
                t.muted_foreground,
                cr.width() - 20.0,
            );
        }
        let _ = FontId::default();
    }

    fn last_open(&self, id: Id, ui: &Ui) -> Option<usize> {
        let v: usize = glowkit_core::state(ui, id.with("last"), || usize::MAX);
        (v < self.items.len()).then_some(v)
    }
}
