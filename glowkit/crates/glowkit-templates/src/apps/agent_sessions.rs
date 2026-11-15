//! Coding-agent sessions manager: sessions across devices, a live transcript with tool calls,
//! a diff panel, approvals, usage limits and a quick "new session" composer.

use egui::{pos2, vec2, Align2, Color32, FontId, Id, Key, Modifiers, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_ai::{ContextMeter, Icon, Markdown, ToolCall, ToolStatus};
use glowkit_components::{toast, LineIcon, Toast, UiIcon};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

use super::common::{demo_notes, demo_user};
use super::prokit as pk;
use crate::shells::kit;
use crate::shells::{AppIcon, AppShell, NavItem, NavSection, ShellLayout, ShellNotification, Workspace};

/// Run state of an [`CodingSession`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentRunState {
    /// The agent is working (streaming text, running tools).
    Live,
    /// The agent is blocked on an approval or a question.
    NeedsInput,
    /// Finished its turn; waiting for the next prompt.
    Idle,
    /// Merged / archived.
    Done,
    /// Stopped with an error.
    Failed,
}

impl AgentRunState {
    fn label(self) -> &'static str {
        match self {
            Self::Live => "Working",
            Self::NeedsInput => "Needs input",
            Self::Idle => "Idle",
            Self::Done => "Done",
            Self::Failed => "Failed",
        }
    }
    fn color(self, t: &Theme) -> Color32 {
        match self {
            Self::Live => t.success,
            Self::NeedsInput => t.warning,
            Self::Idle => t.muted_foreground,
            Self::Done => t.info,
            Self::Failed => t.destructive,
        }
    }
}

#[derive(Clone, Debug)]
enum Ev {
    User(String),
    Text(String),
    /// tool, target, output, diff file index revealed by this call
    Tool(&'static str, String, String, Option<usize>),
    Approval(String, String),
}

#[derive(Clone, Debug)]
struct TEv {
    ev: Ev,
    born: f64,
}

#[derive(Clone, Debug)]
struct DiffFile {
    path: String,
    lines: Vec<(char, String)>,
    visible: bool,
}

impl DiffFile {
    fn counts(&self) -> (usize, usize) {
        (
            self.lines.iter().filter(|l| l.0 == '+').count(),
            self.lines.iter().filter(|l| l.0 == '-').count(),
        )
    }
}

/// One coding-agent session in the [`AgentSessionsApp`].
#[derive(Clone, Debug)]
pub struct CodingSession {
    /// Short task title.
    pub title: String,
    /// Repository / project name.
    pub project: String,
    /// Git branch the agent works on.
    pub branch: String,
    /// Index into the device list.
    pub device: usize,
    /// Agent + model label ("Quill · Nova 2").
    pub agent: String,
    /// Current run state.
    pub state: AgentRunState,
    /// Started ("12m", "2h").
    pub age: String,
    /// Context tokens used.
    pub tokens: f64,
    events: Vec<TEv>,
    script: Vec<Ev>,
    next_at: f64,
    diff: Vec<DiffFile>,
    approval: Option<(String, String)>,
    after_approve: Vec<Ev>,
    after_deny: Vec<Ev>,
    cycle: Vec<Ev>,
}

const DEVICES: [(&str, &str); 3] =
    [("atlas", "MacBook Pro · macOS 15"), ("forge", "Workstation · Linux"), ("runner-7", "Cloud sandbox")];
const PROJECTS: [&str; 3] = ["tidepool", "lumen-desktop", "glint-render"];
const AGENTS: [&str; 3] = ["Quill · Nova 2", "Quill · Nova Flash", "Relay · Orbit 5"];
const CONTEXT_MAX: f64 = 200_000.0;

fn old(ev: Ev) -> TEv {
    TEv { ev, born: -1000.0 }
}

fn diff(path: &str, src: &str) -> DiffFile {
    DiffFile {
        path: path.into(),
        lines: src
            .lines()
            .map(|l| {
                let mut c = l.chars();
                let k = c.next().unwrap_or(' ');
                (k, c.as_str().to_string())
            })
            .collect(),
        visible: false,
    }
}

#[allow(clippy::too_many_arguments)]
fn session(
    title: &str,
    project: &str,
    branch: &str,
    device: usize,
    agent: &str,
    state: AgentRunState,
    age: &str,
    tokens: f64,
) -> CodingSession {
    CodingSession {
        title: title.into(),
        project: project.into(),
        branch: branch.into(),
        device,
        agent: agent.into(),
        state,
        age: age.into(),
        tokens,
        events: Vec::new(),
        script: Vec::new(),
        next_at: 0.0,
        diff: Vec::new(),
        approval: None,
        after_approve: Vec::new(),
        after_deny: Vec::new(),
        cycle: Vec::new(),
    }
}

fn st(x: &str) -> String {
    x.to_string()
}

fn default_sessions() -> Vec<CodingSession> {
    let mut v = Vec::new();
    // ---- forge: the live one ------------------------------------------------------------
    let mut a = session(
        "Retry with backoff in sync client",
        "tidepool",
        "feat/sync-retry",
        1,
        AGENTS[0],
        AgentRunState::Live,
        "14m",
        61_400.0,
    );
    a.events = vec![
        old(Ev::User(st("The sync client drops requests when the server returns 503. Add retry with exponential backoff and jitter, and cover it with tests."))),
        old(Ev::Text(st("I'll look at how requests are sent first."))),
        old(Ev::Tool("Read", st("src/sync/client.rs"), st("212 lines"), None)),
        old(Ev::Tool("Grep", st("fn send_batch"), st("3 matches in 2 files"), None)),
        old(Ev::Text(st("`send_batch` calls the transport once and maps any non-2xx to `SyncError::Server`. I'll add a `RetryPolicy` with capped exponential backoff and full jitter, and only retry idempotent requests on **429 / 502 / 503 / 504**."))),
    ];
    a.script = vec![
        Ev::Tool("Write", st("src/sync/retry.rs"), st("Created file · 58 lines"), Some(0)),
        Ev::Tool("Edit", st("src/sync/client.rs"), st("+14 −3"), Some(1)),
        Ev::Text(st("Wired the policy into `send_batch`. It honors `Retry-After` when the server sends one, otherwise waits `base · 2^attempt` with jitter, capped at 8 s.")),
        Ev::Tool("Write", st("tests/retry.rs"), st("Created file · 41 lines"), Some(2)),
        Ev::Text(st("Added tests for the backoff schedule and for giving up after `max_attempts`. I'd like to run the crate's test suite to confirm nothing else regressed.")),
        Ev::Approval(st("cargo test -p tidepool-sync --all-features"), st("Runs the sync crate's tests (about 40 s, no network).")),
    ];
    a.after_approve = vec![
        Ev::Tool("Bash", st("cargo test -p tidepool-sync --all-features"), st("test result: ok. 47 passed; 0 failed; finished in 38.2s"), None),
        Ev::Text(st("All **47 tests pass**, including the 4 new retry tests. Ready for review: 3 files changed. Want me to open a pull request?")),
    ];
    a.after_deny = vec![Ev::Text(st("Understood, I won't run the tests. The changes are ready for review; run `cargo test -p tidepool-sync` when convenient."))];
    a.diff = vec![
        diff("src/sync/retry.rs", "@@ -0,0 +1,58 @@\n+use std::time::Duration;\n+\n+/// Capped exponential backoff with full jitter.\n+#[derive(Clone, Debug)]\n+pub struct RetryPolicy {\n+    pub base: Duration,\n+    pub cap: Duration,\n+    pub max_attempts: u32,\n+}\n+\n+impl RetryPolicy {\n+    pub fn delay(&self, attempt: u32, rng: &mut impl Rng) -> Duration {\n+        let exp = self.base.saturating_mul(1 << attempt.min(16));\n+        let max = exp.min(self.cap);\n+        max.mul_f64(rng.gen_range(0.0..=1.0))\n+    }\n+\n+    pub fn retryable(status: u16) -> bool {\n+        matches!(status, 429 | 502 | 503 | 504)\n+    }\n+}"),
        diff("src/sync/client.rs", "@@ -88,9 +88,20 @@ impl SyncClient {\n     pub async fn send_batch(&self, batch: Batch) -> Result<Ack, SyncError> {\n-        let res = self.transport.send(&batch).await?;\n-        if !res.status().is_success() {\n-            return Err(SyncError::Server(res.status()));\n+        let mut attempt = 0;\n+        loop {\n+            let res = self.transport.send(&batch).await?;\n+            let status = res.status().as_u16();\n+            if res.status().is_success() {\n+                return Ok(res.json().await?);\n+            }\n+            if !RetryPolicy::retryable(status) || attempt + 1 >= self.retry.max_attempts {\n+                return Err(SyncError::Server(res.status()));\n+            }\n+            let wait = retry_after(&res).unwrap_or_else(|| self.retry.delay(attempt, &mut rand::rng()));\n+            tokio::time::sleep(wait).await;\n+            attempt += 1;\n         }\n-        Ok(res.json().await?)\n     }"),
        diff("tests/retry.rs", "@@ -0,0 +1,41 @@\n+use tidepool_sync::RetryPolicy;\n+\n+#[test]\n+fn delay_is_capped() {\n+    let p = RetryPolicy { base: ms(100), cap: ms(8000), max_attempts: 6 };\n+    for attempt in 0..32 {\n+        assert!(p.delay(attempt, &mut seeded()) <= ms(8000));\n+    }\n+}\n+\n+#[tokio::test]\n+async fn gives_up_after_max_attempts() {\n+    let server = MockServer::always(503).await;\n+    let err = client(&server).send_batch(batch()).await.unwrap_err();\n+    assert_eq!(server.hits(), 6);\n+}"),
    ];
    v.push(a);

    let mut b = session(
        "Profile shader compile stalls",
        "glint-render",
        "perf/shader-cache",
        1,
        AGENTS[2],
        AgentRunState::Live,
        "41m",
        122_800.0,
    );
    b.events = vec![
        old(Ev::User(st("First frame hitches for ~300 ms on cold start. Find out why and propose a fix."))),
        old(Ev::Tool("Bash", st("cargo run --release --features trace -- --frames 120"), st("trace written to target/trace-0412.json"), None)),
        old(Ev::Text(st("The hitch is pipeline compilation on the render thread: 23 pipelines are compiled lazily on first use. A persistent pipeline cache plus warming the 6 hottest pipelines at load would remove it."))),
    ];
    b.script = vec![
        Ev::Tool("Read", st("src/pipeline/cache.rs"), st("96 lines"), None),
        Ev::Tool("Edit", st("src/pipeline/cache.rs"), st("+22 −4"), Some(0)),
        Ev::Text(st("Added an on-disk pipeline cache keyed by adapter + driver version. Now measuring the cold-start frame again.")),
        Ev::Tool("Bash", st("cargo run --release --features trace -- --frames 120"), st("first frame 41 ms (was 312 ms)"), None),
        Ev::Text(st("Cold start is down from **312 ms to 41 ms**. Leaving the warm-up list configurable in `RenderConfig`.")),
    ];
    b.cycle = vec![
        Ev::Text(st("Re-running the trace with the warm-up list to double-check the p99 frame.")),
        Ev::Tool("Bash", st("cargo run --release --features trace -- --frames 600"), st("p50 6.9 ms · p99 11.8 ms · max 41 ms"), None),
        Ev::Tool("Read", st("target/trace-0412.json"), st("2,418 spans"), None),
        Ev::Text(st("Frame times are stable. The remaining 41 ms spike is the first shadow-map allocation; tracking it separately.")),
    ];
    b.diff = vec![diff("src/pipeline/cache.rs", "@@ -12,6 +12,24 @@\n pub struct PipelineCache {\n-    map: HashMap<PipelineKey, Pipeline>,\n+    map: HashMap<PipelineKey, Pipeline>,\n+    disk: Option<PathBuf>,\n+    driver: DriverId,\n }\n+\n+impl PipelineCache {\n+    pub fn load(dir: &Path, driver: DriverId) -> Self {\n+        let blob = std::fs::read(dir.join(driver.cache_name())).ok();\n+        Self::from_blob(blob, driver)\n+    }\n+}")];
    v.push(b);

    // ---- atlas ----------------------------------------------------------------------------
    let mut c = session(
        "Migrate settings page to form API",
        "lumen-desktop",
        "refactor/settings-forms",
        0,
        AGENTS[0],
        AgentRunState::NeedsInput,
        "6m",
        38_900.0,
    );
    c.events = vec![
        old(Ev::User(st("Move the settings page to the new `Form` builder and delete the old field macros."))),
        old(Ev::Tool("Grep", st("field!("), st("31 matches in 7 files"), None)),
        old(Ev::Tool("Edit", st("src/ui/settings.rs"), st("+48 −112"), Some(0))),
        old(Ev::Text(st("The settings page now uses `Form`. The old macros live in `lumen-macros`, which is also used by the plugin SDK. Removing it needs a dependency update."))),
    ];
    c.approval = Some((st("cargo remove lumen-macros && cargo update -p lumen-sdk"), st("Removes a dependency and updates Cargo.lock.")));
    c.after_approve = vec![
        Ev::Tool("Bash", st("cargo remove lumen-macros && cargo update -p lumen-sdk"), st("Removed lumen-macros · Updating lumen-sdk v0.14.1 -> v0.15.0"), None),
        Ev::Text(st("Dependency removed and the workspace builds cleanly. Settings is fully on the new form API.")),
    ];
    c.after_deny = vec![Ev::Text(st("Okay, I'll keep `lumen-macros` for now and only migrate the settings page."))];
    c.diff = vec![diff("src/ui/settings.rs", "@@ -20,14 +20,9 @@ fn settings(ui: &mut Ui, s: &mut Settings) {\n-    field!(ui, \"Theme\", s.theme, ThemeChoice::ALL);\n-    field!(ui, \"Font size\", s.font_size, 10..=24);\n-    field!(ui, \"Autosave\", s.autosave);\n+    Form::new(\"settings\")\n+        .select(\"Theme\", &mut s.theme, ThemeChoice::ALL)\n+        .slider(\"Font size\", &mut s.font_size, 10..=24)\n+        .toggle(\"Autosave\", &mut s.autosave)\n+        .show(ui);")];
    c.diff[0].visible = true;
    v.push(c);

    let mut d = session(
        "Fix flaky snapshot test",
        "lumen-desktop",
        "fix/snapshot-flake",
        0,
        AGENTS[1],
        AgentRunState::Idle,
        "1h",
        21_300.0,
    );
    d.events = vec![
        old(Ev::User(st("`sidebar_snapshot` fails about 1 in 10 runs on CI. Fix it."))),
        old(Ev::Tool("Bash", st("cargo nextest run sidebar_snapshot --retries 0 -j 1 --run-ignored all"), st("failed 3 / 30"), None)),
        old(Ev::Text(st("The snapshot captures a frame while the collapse spring is still settling. I pinned the animation clock in tests so the frame is deterministic; 200 consecutive runs pass."))),
    ];
    d.diff = vec![diff("tests/snapshots.rs", "@@ -8,6 +8,7 @@\n fn sidebar_snapshot() {\n     let mut h = Harness::new(app());\n+    h.ctx.set_animation_time(0.0);\n     h.run();\n     h.snapshot(\"sidebar\");\n }")];
    d.diff[0].visible = true;
    v.push(d);

    // ---- runner-7 ------------------------------------------------------------------------
    let mut e = session("Bump MSRV to 1.85 and fix lints", "tidepool", "chore/msrv-1.85", 2, AGENTS[1], AgentRunState::Done, "3h", 54_000.0);
    e.events = vec![
        old(Ev::User(st("Bump the MSRV to 1.85 and fix the new clippy lints."))),
        old(Ev::Tool("Bash", st("cargo clippy --workspace --all-targets -- -D warnings"), st("0 warnings"), None)),
        old(Ev::Text(st("MSRV is 1.85 across the workspace and clippy is clean. Merged as #482."))),
    ];
    v.push(e);
    let mut f = session("Write release notes for v0.9", "tidepool", "docs/release-0.9", 2, AGENTS[2], AgentRunState::Failed, "5h", 12_100.0);
    f.events = vec![
        old(Ev::User(st("Draft release notes for v0.9 from the merged PRs."))),
        old(Ev::Tool("Bash", st("gh pr list --state merged --base main --limit 200"), st("error: authentication required for api endpoint"), None)),
        old(Ev::Text(st("I couldn't list merged pull requests: the sandbox has no credentials for the code host. Add a token to the runner's secrets and resume."))),
    ];
    v.push(f);
    v
}

/// A desktop app for supervising coding-agent sessions across your machines: sessions grouped
/// by device with live / needs-input / idle states, the selected session's streaming
/// transcript with tool calls, a diff of its changes, an approve / deny bar for risky commands,
/// usage limits and a quick "new session" composer. Product and model names are fictional.
///
/// Keys: ↑ ↓ (J K) switch sessions, Enter approves, Esc denies, Ctrl+N starts a new session.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::AgentSessionsApp::default();
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct AgentSessionsApp {
    /// Active route.
    pub route: String,
    /// Shell navigation.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// All sessions.
    pub sessions: Vec<CodingSession>,
    /// Index of the selected session.
    pub selected: usize,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    query: String,
    filter: usize,
    reply: String,
    diff_file: usize,
    composer: Option<NewSession>,
    started: Option<f64>,
}

#[derive(Clone, Debug, Default)]
struct NewSession {
    opened: Option<f64>,
    prompt: String,
    project: usize,
    device: usize,
    agent: usize,
}

impl Default for AgentSessionsApp {
    fn default() -> Self {
        Self {
            route: "sessions".into(),
            nav: vec![
                NavSection::new("Agents")
                    .item(NavItem::new("sessions", "Sessions").icon(AppIcon::Bot).badge("2"))
                    .item(NavItem::new("devices", "Devices").icon(AppIcon::Server))
                    .item(NavItem::new("usage", "Usage").icon(UiIcon::Activity))
                    .item(NavItem::new("history", "History").icon(UiIcon::Clock)),
                NavSection::new("Workspace")
                    .item(NavItem::new("repos", "Repositories").icon(UiIcon::Code))
                    .item(NavItem::new("settings", "Settings").icon(LineIcon::Settings)),
            ],
            notifications: demo_notes(),
            sessions: default_sessions(),
            selected: 0,
            scoped_theme: false,
            query: String::new(),
            filter: 0,
            reply: String::new(),
            diff_file: 0,
            composer: None,
            started: None,
        }
    }
}

const FILTERS: [&str; 4] = ["All", "Working", "Needs input", "Idle"];

impl AgentSessionsApp {
    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        let mut new_clicked = false;
        AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::Inset)
            .start_collapsed(true)
            .workspaces(vec![Workspace::new("Quill Desk", "Team plan")])
            .user(demo_user())
            .notifications(&mut notes)
            .padding(0.0)
            .scoped_theme(self.scoped_theme)
            .id_salt("agent_sessions_app")
            .actions(|ui| {
                let b = ui.add(
                    glowkit_components::Button::new("New session")
                        .icon(UiIcon::Plus)
                        .variant(glowkit_components::ButtonVariant::Outline)
                        .size(glowkit_components::ControlSize::Sm),
                );
                kit::tip(&b, "Start an agent session", Some("Ctrl+N"));
                new_clicked = b.clicked();
            })
            .show(ui, |ui, sc| {
                if sc.route.as_str() == "sessions" {
                    self.body(ui, sc.rect);
                } else {
                    super::analytics::placeholder(ui, sc.rect.shrink(24.0), sc.route);
                }
            });
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
        if new_clicked {
            self.open_composer();
        }
    }

    fn open_composer(&mut self) {
        let dev = self.sessions.get(self.selected).map_or(0, |s| s.device);
        self.composer = Some(NewSession { device: dev, ..Default::default() });
    }

    fn tick(&mut self, now: f64, dt: f32) {
        let t0 = *self.started.get_or_insert(now);
        for s in &mut self.sessions {
            if s.state == AgentRunState::Live {
                s.tokens = (s.tokens + dt as f64 * 260.0).min(CONTEXT_MAX * 0.97);
                if s.next_at == 0.0 {
                    s.next_at = t0 + 1.4;
                }
                if now >= s.next_at {
                    if s.script.is_empty() && !s.cycle.is_empty() {
                        s.script = s.cycle.clone();
                    }
                    if s.script.is_empty() {
                        s.state = AgentRunState::Idle;
                        continue;
                    }
                    let ev = s.script.remove(0);
                    let dur = match &ev {
                        Ev::Text(x) => x.chars().count() as f64 / 110.0 + 0.9,
                        Ev::Tool(..) => 1.9,
                        _ => 1.0,
                    };
                    if let Ev::Tool(_, _, _, Some(fi)) = &ev {
                        if let Some(f) = s.diff.get_mut(*fi) {
                            f.visible = true;
                        }
                    }
                    if let Ev::Approval(cmd, why) = &ev {
                        s.approval = Some((cmd.clone(), why.clone()));
                        s.state = AgentRunState::NeedsInput;
                    } else {
                        s.events.push(TEv { ev, born: now });
                    }
                    s.next_at = now + dur;
                }
            }
        }
    }

    fn resolve(&mut self, idx: usize, approve: bool, now: f64, ctx: &egui::Context) {
        let Some(s) = self.sessions.get_mut(idx) else { return };
        let Some((cmd, _)) = s.approval.take() else { return };
        s.script = if approve { std::mem::take(&mut s.after_approve) } else { std::mem::take(&mut s.after_deny) };
        s.events.push(TEv { ev: Ev::Text(if approve { format!("✓ Approved `{cmd}`") } else { format!("✕ Denied `{cmd}`") }), born: -1000.0 });
        s.state = AgentRunState::Live;
        s.next_at = now + 0.4;
        if approve {
            toast(ctx, Toast::success("Command approved").description(format!("{} continues on {}", s.title, DEVICES[s.device].0)));
        } else {
            toast(ctx, Toast::new("Command denied").description("The agent will continue without it."));
        }
    }

    fn visible(&self) -> Vec<usize> {
        let q = self.query.to_lowercase();
        (0..self.sessions.len())
            .filter(|&i| {
                let s = &self.sessions[i];
                let f = match self.filter {
                    1 => s.state == AgentRunState::Live,
                    2 => s.state == AgentRunState::NeedsInput,
                    3 => matches!(s.state, AgentRunState::Idle | AgentRunState::Done | AgentRunState::Failed),
                    _ => true,
                };
                f && (q.is_empty() || s.title.to_lowercase().contains(&q) || s.project.contains(&q))
            })
            .collect()
    }

    fn body(&mut self, ui: &mut Ui, rect: Rect) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("agent_sessions");
        let now = ui.input(|i| i.time);
        let dt = ui.input(|i| i.stable_dt).min(0.1);
        self.tick(now, dt);
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
        // keys
        let typing = pk::typing(ui);
        let vis = self.visible();
        if self.composer.is_none() {
            if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::N)) {
                self.open_composer();
            } else if !typing {
                let (down, up, enter, esc) = ui.input(|i| {
                    (
                        i.key_pressed(Key::ArrowDown) || i.key_pressed(Key::J),
                        i.key_pressed(Key::ArrowUp) || i.key_pressed(Key::K),
                        i.key_pressed(Key::Enter),
                        i.key_pressed(Key::Escape),
                    )
                });
                let pos = vis.iter().position(|v| *v == self.selected);
                if down && !vis.is_empty() {
                    self.selected = vis[pos.map_or(0, |p| (p + 1).min(vis.len() - 1))];
                    self.diff_file = 0;
                }
                if up && !vis.is_empty() {
                    self.selected = vis[pos.map_or(0, |p| p.saturating_sub(1))];
                    self.diff_file = 0;
                }
                if (enter || esc) && self.sessions.get(self.selected).is_some_and(|s| s.approval.is_some()) {
                    self.resolve(self.selected, enter, now, &ui.ctx().clone());
                }
            }
        }
        let parts = kit::split_x(rect, &[272.0, 0.0, 330.0], 0.0);
        let p = ui.painter().clone();
        p.rect_filled(parts[0], egui::CornerRadius { nw: (t.radius * 1.2) as u8, sw: (t.radius * 1.2) as u8, ne: 0, se: 0 }, kit::chrome_bg(&t));
        kit::vline(&p, parts[0].max.x, rect.min.y, rect.max.y, &t);
        kit::vline(&p, parts[2].min.x, rect.min.y, rect.max.y, &t);
        self.list_pane(ui, parts[0], id.with("list"), now);
        self.transcript_pane(ui, parts[1], id.with("main"), now);
        self.side_pane(ui, parts[2], id.with("side"), now);
        if self.composer.is_some() {
            self.composer_overlay(ui, rect, id.with("composer"), now);
        }
    }

    // ---------------------------------------------------------------------------------------
    // session list
    // ---------------------------------------------------------------------------------------

    fn list_pane(&mut self, ui: &mut Ui, r: Rect, id: Id, now: f64) {
        let t = Theme::of(ui);
        let p = ui.painter().clone();
        let x0 = r.min.x + 14.0;
        let x1 = r.max.x - 14.0;
        kit::text_at(&p, pos2(x0, r.min.y + 26.0), Align2::LEFT_CENTER, "Sessions", 15.0, t.foreground, 120.0);
        let live = self.sessions.iter().filter(|s| s.state == AgentRunState::Live).count();
        let waiting = self.sessions.iter().filter(|s| s.state == AgentRunState::NeedsInput).count();
        let tr = kit::text_at(
            &p,
            pos2(x1, r.min.y + 26.0),
            Align2::RIGHT_CENTER,
            &format!("{live} working · {waiting} waiting"),
            11.5,
            t.muted_foreground,
            150.0,
        );
        let _ = tr;
        pk::search(ui, Rect::from_min_max(pos2(x0, r.min.y + 46.0), pos2(x1, r.min.y + 78.0)), &mut self.query, "Filter sessions", "as_search");
        // filter chips
        let mut x = x0;
        let cy = r.min.y + 98.0;
        for (i, f) in FILTERS.iter().enumerate() {
            let w = pk::chip_w(&p, f);
            if x + w > x1 + 2.0 {
                break;
            }
            let cr = Rect::from_min_size(pos2(x, cy - 11.0), vec2(w, 22.0));
            let accent = match i {
                1 => t.success,
                2 => t.warning,
                _ => t.primary,
            };
            if pk::chip(ui, cr, id.with(("f", i)), f, self.filter == i, accent).clicked() {
                self.filter = i;
            }
            x += w + 6.0;
        }
        let lr = Rect::from_min_max(pos2(r.min.x, r.min.y + 118.0), pos2(r.max.x, r.max.y - 34.0));
        kit::hline(&p, r.min.x, r.max.x, lr.min.y, &t);
        let vis = self.visible();
        let mut clicked = None;
        kit::child(ui, lr, |ui| {
            egui::ScrollArea::vertical().id_salt("as_list").auto_shrink([false, false]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.add_space(6.0);
                let w = ui.available_width();
                for (d, (dname, dsub)) in DEVICES.iter().enumerate() {
                    let rows: Vec<usize> = vis.iter().copied().filter(|i| self.sessions[*i].device == d).collect();
                    if rows.is_empty() {
                        continue;
                    }
                    let (hr, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
                    let p = ui.painter();
                    let icon: AppIcon = match d {
                        0 => LineIcon::Command.into(),
                        1 => AppIcon::Server,
                        _ => LineIcon::Globe.into(),
                    };
                    icon.paint(p, Rect::from_center_size(pos2(hr.min.x + 22.0, hr.center().y), vec2(13.0, 13.0)), t.muted_foreground);
                    let nr = kit::text_at(p, pos2(hr.min.x + 34.0, hr.center().y), Align2::LEFT_CENTER, dname, 12.0, t.foreground, 90.0);
                    kit::text_at(p, pos2(nr.max.x + 6.0, hr.center().y), Align2::LEFT_CENTER, dsub, 11.0, t.muted_foreground, hr.max.x - nr.max.x - 44.0);
                    pk::dot(p, pos2(hr.max.x - 20.0, hr.center().y), 3.0, t.success, None);
                    for i in rows {
                        let (rr, resp) = ui.allocate_exact_size(vec2(w, 54.0), Sense::click());
                        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        let rr = rr.shrink2(vec2(8.0, 2.0));
                        let hv = kit::hover(ui, id.with(("row", i)), resp.hovered());
                        let p = ui.painter();
                        let sel = i == self.selected;
                        if sel {
                            p.rect_filled(rr, 7.0, kit::active_fill(&t, kit::chrome_bg(&t)));
                        } else if hv > 0.01 {
                            p.rect_filled(rr, 7.0, kit::hover_fill(&t, hv));
                        }
                        let s = &self.sessions[i];
                        let col = s.state.color(&t);
                        let pulse = (s.state == AgentRunState::Live).then_some(now);
                        pk::dot(p, pos2(rr.min.x + 13.0, rr.min.y + 17.0), 3.5, col, pulse);
                        let age = kit::text_at(p, pos2(rr.max.x - 8.0, rr.min.y + 17.0), Align2::RIGHT_CENTER, &s.age, 11.0, t.muted_foreground, 40.0);
                        kit::text_at(p, pos2(rr.min.x + 26.0, rr.min.y + 17.0), Align2::LEFT_CENTER, &s.title, 12.8, t.foreground, age.min.x - rr.min.x - 32.0);
                        let line2 = format!("{} · {}", s.project, s.branch);
                        let right = if s.state == AgentRunState::NeedsInput {
                            pk::tag(p, pos2(rr.max.x - 8.0, rr.min.y + 37.0), true, "Approve", t.warning, &t, false).min.x
                        } else if s.state == AgentRunState::Failed {
                            pk::tag(p, pos2(rr.max.x - 8.0, rr.min.y + 37.0), true, "Error", t.destructive, &t, false).min.x
                        } else {
                            kit::text_at(p, pos2(rr.max.x - 8.0, rr.min.y + 37.0), Align2::RIGHT_CENTER, &format!("{:.0}k", s.tokens / 1000.0), 11.0, t.muted_foreground, 50.0).min.x
                        };
                        kit::text_at(p, pos2(rr.min.x + 26.0, rr.min.y + 37.0), Align2::LEFT_CENTER, &line2, 11.5, t.muted_foreground, right - rr.min.x - 34.0);
                        if resp.clicked() {
                            clicked = Some(i);
                        }
                    }
                    ui.add_space(6.0);
                }
                if vis.is_empty() {
                    let (er, _) = ui.allocate_exact_size(vec2(w, 80.0), Sense::hover());
                    ui.painter().text(er.center(), Align2::CENTER_CENTER, "No sessions match", FontId::proportional(12.5), t.muted_foreground);
                }
            });
        });
        if let Some(i) = clicked {
            self.selected = i;
            self.diff_file = 0;
        }
        // footer hints
        kit::hline(&p, r.min.x, r.max.x, r.max.y - 34.0, &t);
        pk::hints(&p, pos2(x0, r.max.y - 17.0), &[("↑↓", "switch"), ("Ctrl N", "new")], &t, x1);
    }

    // ---------------------------------------------------------------------------------------
    // transcript
    // ---------------------------------------------------------------------------------------

    fn transcript_pane(&mut self, ui: &mut Ui, r: Rect, id: Id, now: f64) {
        let t = Theme::of(ui);
        let p = ui.painter().clone();
        let sel = self.selected.min(self.sessions.len().saturating_sub(1));
        let Some(s) = self.sessions.get(sel).cloned() else { return };
        let x0 = r.min.x + 20.0;
        let x1 = r.max.x - 20.0;
        // header
        let hy = r.min.y + 20.0;
        let pill = kit::status_pill(&p, pos2(x1, hy + 2.0), true, s.state.label(), s.state.color(&t), &t);
        let meter_r = Rect::from_min_size(pos2(pill.min.x - 98.0, hy - 13.0), vec2(90.0, 30.0));
        kit::child(ui, meter_r, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(
                    ContextMeter::new(s.tokens as u64, CONTEXT_MAX as u64)
                        .segment("Input", (s.tokens * 0.62) as u64)
                        .segment("Output", (s.tokens * 0.21) as u64)
                        .segment("Cached", (s.tokens * 0.17) as u64)
                        .model(&s.agent)
                        .cost(s.tokens / 1_000_000.0 * 9.0),
                );
            });
        });
        kit::text_at(&p, pos2(x0, hy), Align2::LEFT_CENTER, &s.title, 15.5, t.foreground, meter_r.min.x - x0 - 12.0);
        let mut x = x0;
        let sub_y = hy + 22.0;
        for (txt, mono) in [(s.project.as_str(), false), (s.branch.as_str(), true)] {
            let tr = pk::tag(&p, pos2(x, sub_y), false, txt, t.primary, &t, mono);
            x = tr.max.x + 6.0;
        }
        kit::text_at(
            &p,
            pos2(x + 2.0, sub_y),
            Align2::LEFT_CENTER,
            &format!("{} · on {}", s.agent, DEVICES[s.device].0),
            11.5,
            t.muted_foreground,
            x1 - x - 4.0,
        );
        let head_b = r.min.y + 58.0;
        kit::hline(&p, r.min.x, r.max.x, head_b, &t);
        // bottom: approval + composer
        let has_appr = s.approval.is_some();
        let appr_k = kit::spring(ui.ctx(), id.with(("appr", sel)), if has_appr { 1.0 } else { 0.0 }, 300.0, 26.0).clamp(0.0, 1.2);
        let comp_h = 62.0;
        let appr_h = 118.0 * appr_k.min(1.0);
        let tr_rect = Rect::from_min_max(pos2(r.min.x, head_b + 1.0), pos2(r.max.x, r.max.y - comp_h - appr_h));
        let mut copy_cmd = None;
        kit::child(ui, tr_rect, |ui| {
            egui::ScrollArea::vertical()
                .id_salt(("as_transcript", sel))
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let w = ui.available_width();
                    ui.add_space(14.0);
                    let inner = egui::Rect::from_min_size(ui.cursor().min + vec2(20.0, 0.0), vec2(w - 40.0, 0.0));
                    let _ = inner;
                    egui::Frame::NONE.inner_margin(egui::Margin { left: 20, right: 20, top: 0, bottom: 0 }).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 12.0;
                        let n = s.events.len();
                        for (k, e) in s.events.iter().enumerate() {
                            let age = (now - e.born) as f32;
                            match &e.ev {
                                Ev::User(text) => user_block(ui, text, &t),
                                Ev::Text(text) => {
                                    let total = text.chars().count();
                                    let shown = if e.born < 0.0 { total } else { ((age * 110.0) as usize).min(total) };
                                    let part: String = text.chars().take(shown).collect();
                                    let streaming = shown < total;
                                    ui.add(Markdown::new(&part).size(13.5).caret(streaming).fade_tail(if streaming { 10 } else { 0 }));
                                }
                                Ev::Tool(name, target, out, _) => {
                                    let running = e.born > 0.0 && age < 1.3 && k + 1 == n;
                                    let icon = match *name {
                                        "Bash" => Icon::Terminal,
                                        "Read" => Icon::File,
                                        "Grep" => Icon::Search,
                                        _ => Icon::Pencil,
                                    };
                                    let failed = out.starts_with("error");
                                    let mut tc = ToolCall::new(name)
                                        .description(target)
                                        .icon(icon)
                                        .input(target)
                                        .status(if running {
                                            ToolStatus::Running
                                        } else if failed {
                                            ToolStatus::Error
                                        } else {
                                            ToolStatus::Completed
                                        })
                                        .id_salt(("as_tool", sel, k));
                                    if !running {
                                        tc = if failed { tc.error(out) } else { tc.output(out) };
                                    }
                                    let resp = ui.add(tc);
                                    if resp.secondary_clicked() {
                                        copy_cmd = Some(target.clone());
                                    }
                                }
                                Ev::Approval(..) => {}
                            }
                        }
                        if s.state == AgentRunState::Live && s.events.last().is_some_and(|e| (now - e.born) > 1.6 || e.born < 0.0) {
                            let (lr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
                            let p = ui.painter();
                            for k in 0..3 {
                                let ph = ((now * 2.4 - k as f64 * 0.25).sin() * 0.5 + 0.5) as f32;
                                p.circle_filled(pos2(lr.min.x + 5.0 + k as f32 * 10.0, lr.center().y), 3.0, with_alpha(t.muted_foreground, 0.3 + 0.6 * ph));
                            }
                            kit::text_at(p, pos2(lr.min.x + 36.0, lr.center().y), Align2::LEFT_CENTER, "Thinking…", 12.0, t.muted_foreground, 120.0);
                        }
                        ui.add_space(10.0);
                    });
                });
        });
        if let Some(c) = copy_cmd {
            ui.ctx().copy_text(c);
            toast(ui.ctx(), Toast::success("Copied command"));
        }
        // approval bar
        if appr_h > 1.0 {
            let ar = Rect::from_min_max(pos2(x0 - 6.0, r.max.y - comp_h - appr_h + 4.0), pos2(x1 + 6.0, r.max.y - comp_h - 2.0));
            let (cmd, why) = s.approval.clone().unwrap_or_default();
            let resolved = kit::child(ui, ar, |ui| approval_bar(ui, ar, id.with(("ab", sel)), &cmd, &why, &s.agent));
            if let Some(ok) = resolved {
                self.resolve(sel, ok, now, &ui.ctx().clone());
            }
        }
        // composer
        let cr = Rect::from_min_max(pos2(x0, r.max.y - comp_h + 12.0), pos2(x1, r.max.y - 14.0));
        let send_r = Rect::from_min_max(pos2(cr.max.x - 74.0, cr.min.y + 4.0), pos2(cr.max.x - 4.0, cr.max.y - 4.0));
        let field = Rect::from_min_max(cr.min, cr.max);
        let agent_short = s.agent.split(' ').next().unwrap_or("agent").to_string();
        let (submitted, _) = pk::text_line(ui, field, id.with("reply"), &mut self.reply, &format!("Reply to {agent_short}…"), false);
        let can = !self.reply.trim().is_empty();
        let sb = pk::small_btn(ui, send_r, id.with("send"), "Send", Some(AppIcon::Send), Some(if can { t.primary } else { t.muted }), None);
        if (submitted || sb.clicked()) && can {
            let text = std::mem::take(&mut self.reply);
            let sess = &mut self.sessions[sel];
            sess.events.push(TEv { ev: Ev::User(text.clone()), born: -1000.0 });
            sess.state = AgentRunState::Live;
            sess.approval = None;
            sess.next_at = now + 0.8;
            sess.script = vec![
                Ev::Text(st("On it. Checking the relevant code first.")),
                Ev::Tool("Grep", text.split_whitespace().last().unwrap_or("TODO").trim_matches('`').to_string(), st("4 matches in 3 files"), None),
                Ev::Text(format!("Done. I followed up on *\u{201c}{}\u{201d}* and the build is green. Anything else?", text.chars().take(48).collect::<String>())),
            ];
        }
    }

    // ---------------------------------------------------------------------------------------
    // usage + changes
    // ---------------------------------------------------------------------------------------

    fn side_pane(&mut self, ui: &mut Ui, r: Rect, id: Id, now: f64) {
        let t = Theme::of(ui);
        let p = ui.painter().clone();
        let x0 = r.min.x + 16.0;
        let x1 = r.max.x - 16.0;
        // usage
        pk::label(&p, pos2(x0, r.min.y + 20.0), "Usage", &t, 100.0);
        kit::text_at(&p, pos2(x1, r.min.y + 20.0), Align2::RIGHT_CENTER, "resets in 2h 14m", 11.0, t.muted_foreground, 140.0);
        let live_tokens: f64 = self.sessions.iter().map(|s| s.tokens).sum();
        let rows = [
            ("5-hour window", 0.52 + (live_tokens - 310_000.0).max(0.0) / 2_000_000.0, t.primary),
            ("Weekly limit", 0.31 + (live_tokens - 310_000.0).max(0.0) / 8_000_000.0, t.chart[1]),
            ("Premium model", 0.74, t.warning),
        ];
        let mut y = r.min.y + 44.0;
        for (k, (name, v, c)) in rows.iter().enumerate() {
            let v = kit::follow(ui.ctx(), id.with(("u", k)), *v as f32, 6.0);
            kit::text_at(&p, pos2(x0, y), Align2::LEFT_CENTER, name, 12.0, t.foreground, 160.0);
            kit::text_at(&p, pos2(x1, y), Align2::RIGHT_CENTER, &format!("{:.0}%", v * 100.0), 12.0, t.muted_foreground, 50.0);
            pk::meter(&p, Rect::from_min_max(pos2(x0, y + 11.0), pos2(x1, y + 16.0)), v, *c, &t);
            y += 34.0;
        }
        // throughput spark
        let vals: Vec<f64> = (0..40).map(|i| {
            let k = i as f64 + (now * 2.0).floor();
            380.0 + (k * 0.7).sin() * 90.0 + (k * 1.9).cos() * 50.0 + glowkit_core::rng::hash01(k as u64) as f64 * 80.0
        }).collect();
        let sr = Rect::from_min_max(pos2(x0 + 150.0, y - 4.0), pos2(x1, y + 22.0));
        pk::spark(&p, sr, &vals, t.success, true);
        kit::text_at(&p, pos2(x0, y + 1.0), Align2::LEFT_CENTER, &format!("{} tokens today", kit::thousands(live_tokens * 4.1, 0)), 12.0, t.foreground, 145.0);
        kit::text_at(&p, pos2(x0, y + 18.0), Align2::LEFT_CENTER, &format!("{:.0} tok/s across agents", vals[39]), 11.0, t.muted_foreground, 145.0);
        let top = y + 40.0;
        kit::hline(&p, r.min.x, r.max.x, top, &t);
        // changes
        let sel = self.selected.min(self.sessions.len().saturating_sub(1));
        let Some(s) = self.sessions.get(sel) else { return };
        let files: Vec<usize> = (0..s.diff.len()).filter(|i| s.diff[*i].visible).collect();
        let (add, del) = files.iter().fold((0, 0), |a, i| {
            let c = s.diff[*i].counts();
            (a.0 + c.0, a.1 + c.1)
        });
        let hy = top + 22.0;
        let lr = pk::label(&p, pos2(x0, hy), "Changes", &t, 100.0);
        let ar = kit::text_at(&p, pos2(lr.max.x + 8.0, hy), Align2::LEFT_CENTER, &format!("+{add}"), 11.5, t.success, 50.0);
        kit::text_at(&p, pos2(ar.max.x + 4.0, hy), Align2::LEFT_CENTER, &format!("−{del}"), 11.5, t.destructive, 50.0);
        let pr_btn = Rect::from_min_max(pos2(x1 - 92.0, hy - 12.0), pos2(x1, hy + 12.0));
        if !files.is_empty() && pk::small_btn(ui, pr_btn, id.with("pr"), "Open PR", Some(LineIcon::Share.into()), None, None).clicked() {
            toast(ui.ctx(), Toast::success("Pull request opened").description(format!("{} → main · {} files", s.branch, files.len())));
        }
        let mut y = hy + 22.0;
        if files.is_empty() {
            p.text(pos2(r.center().x, y + 60.0), Align2::CENTER_CENTER, "No file changes yet", FontId::proportional(12.5), t.muted_foreground);
            return;
        }
        let cur = files.iter().position(|f| *f == self.diff_file).map_or(files[0], |k| files[k]);
        for &fi in &files {
            let f = &s.diff[fi];
            let rr = Rect::from_min_max(pos2(r.min.x + 8.0, y), pos2(r.max.x - 8.0, y + 26.0));
            let (resp, _) = kit::row(ui, rr, id.with(("file", sel, fi)), 6.0);
            if fi == cur {
                p.rect_filled(rr, 6.0, kit::active_fill(&t, t.background));
            }
            let (a, d) = f.counts();
            let dr = kit::text_at(&p, pos2(rr.max.x - 8.0, rr.center().y), Align2::RIGHT_CENTER, &format!("−{d}"), 11.0, t.destructive, 40.0);
            let ar = kit::text_at(&p, pos2(dr.min.x - 6.0, rr.center().y), Align2::RIGHT_CENTER, &format!("+{a}"), 11.0, t.success, 40.0);
            AppIcon::Ui(UiIcon::File).paint(&p, Rect::from_center_size(pos2(rr.min.x + 14.0, rr.center().y), vec2(12.0, 12.0)), t.muted_foreground);
            let (dir, name) = f.path.rsplit_once('/').map_or(("", f.path.as_str()), |(a, b)| (a, b));
            let nr = kit::text_at(&p, pos2(rr.min.x + 26.0, rr.center().y), Align2::LEFT_CENTER, name, 12.0, t.foreground, ar.min.x - rr.min.x - 40.0);
            kit::text_at(&p, pos2(nr.max.x + 6.0, rr.center().y), Align2::LEFT_CENTER, dir, 11.0, t.muted_foreground, (ar.min.x - nr.max.x - 14.0).max(0.0));
            if resp.clicked() {
                self.diff_file = fi;
            }
            y += 28.0;
        }
        // diff body
        let dr = Rect::from_min_max(pos2(r.min.x + 8.0, y + 6.0), pos2(r.max.x - 8.0, r.max.y - 10.0));
        p.rect_filled(dr, 8.0, if t.dark { mix(t.background, Color32::BLACK, 0.25) } else { mix(t.background, t.foreground, 0.02) });
        p.rect_stroke(dr, 8.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
        let f = s.diff[cur].clone();
        kit::child(ui, dr.shrink(1.0), |ui| {
            egui::ScrollArea::both().id_salt(("as_diff", sel, cur)).auto_shrink([false, false]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let w = ui.available_width().max(520.0);
                let mut old_n = 1;
                let mut new_n = 1;
                for (k, txt) in &f.lines {
                    let (lr, _) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
                    let p = ui.painter();
                    let (bg, fg, sign) = match k {
                        '+' => (with_alpha(t.success, if t.dark { 0.14 } else { 0.1 }), pk::tone_fg(&t, t.success), "+"),
                        '-' => (with_alpha(t.destructive, if t.dark { 0.14 } else { 0.09 }), pk::tone_fg(&t, t.destructive), "−"),
                        '@' => (with_alpha(t.info, 0.08), t.info, ""),
                        _ => (Color32::TRANSPARENT, t.foreground, " "),
                    };
                    p.rect_filled(lr, 0.0, bg);
                    if *k == '@' {
                        if let Some((a, b)) = txt.trim_start_matches('@').trim().split_once(' ') {
                            let _ = b;
                            let nums: Vec<i32> = a.split([',', '-', '+']).filter_map(|x| x.parse().ok()).collect();
                            if nums.len() >= 2 {
                                old_n = nums[0].max(1);
                            }
                        }
                        pk::mono(p, pos2(lr.min.x + 8.0, lr.center().y), Align2::LEFT_CENTER, &format!("@{txt}"), 11.0, fg, w - 16.0);
                        new_n = old_n;
                        continue;
                    }
                    let (on, nn) = match k {
                        '+' => (String::new(), new_n.to_string()),
                        '-' => (old_n.to_string(), String::new()),
                        _ => (old_n.to_string(), new_n.to_string()),
                    };
                    match k {
                        '+' => new_n += 1,
                        '-' => old_n += 1,
                        _ => {
                            old_n += 1;
                            new_n += 1;
                        }
                    }
                    let nc = with_alpha(t.muted_foreground, 0.7);
                    pk::mono(p, pos2(lr.min.x + 26.0, lr.center().y), Align2::RIGHT_CENTER, &on, 10.5, nc, 30.0);
                    pk::mono(p, pos2(lr.min.x + 52.0, lr.center().y), Align2::RIGHT_CENTER, &nn, 10.5, nc, 30.0);
                    pk::mono(p, pos2(lr.min.x + 60.0, lr.center().y), Align2::LEFT_CENTER, sign, 11.5, fg, 12.0);
                    pk::mono(p, pos2(lr.min.x + 72.0, lr.center().y), Align2::LEFT_CENTER, txt, 11.5, if *k == ' ' { t.muted_foreground } else { fg }, w - 80.0);
                }
            });
        });
    }

    // ---------------------------------------------------------------------------------------
    // new session composer
    // ---------------------------------------------------------------------------------------

    fn composer_overlay(&mut self, ui: &mut Ui, bounds: Rect, id: Id, now: f64) {
        let t = Theme::of(ui);
        let opened = self.composer.as_mut().map_or(now, |c| *c.opened.get_or_insert(now));
        let first = opened == now;
        let e = ((now - opened) as f32 / 0.2).clamp(0.0, 1.0);
        let k = 1.0 - (1.0 - e).powi(3);
        if e < 1.0 {
            ui.ctx().request_repaint();
        }
        // scrim (blocks clicks behind)
        let scrim = ui.interact(bounds, id.with("scrim"), Sense::click());
        let p = ui.painter().clone();
        p.rect_filled(bounds, 0.0, with_alpha(Color32::BLACK, if t.dark { 0.45 } else { 0.18 } * k.min(1.0)));
        let w = 560.0f32.min(bounds.width() - 40.0);
        let h = 280.0;
        let c = pos2(bounds.center().x, bounds.min.y + 70.0 + h / 2.0 - (1.0 - k) * 16.0);
        let r = Rect::from_center_size(c, vec2(w, h));
        glowkit_core::paint::soft_shadow(&p, r, 12.0, 24.0, vec2(0.0, 8.0), kit::shadow(&t));
        p.rect_filled(r, 12.0, t.popover);
        p.rect_stroke(r, 12.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
        let x0 = r.min.x + 20.0;
        let x1 = r.max.x - 20.0;
        AppIcon::Bot.paint(&p, Rect::from_center_size(pos2(x0 + 8.0, r.min.y + 28.0), vec2(16.0, 16.0)), t.primary);
        kit::text_at(&p, pos2(x0 + 24.0, r.min.y + 28.0), Align2::LEFT_CENTER, "New agent session", 15.0, t.foreground, 260.0);
        pk::key_hint(&p, pos2(x1 - 34.0, r.min.y + 28.0), "Esc", &t);
        let Some(mut ns) = self.composer.take() else { return };
        let (sub, _) = pk::text_line(
            ui,
            Rect::from_min_max(pos2(x0, r.min.y + 52.0), pos2(x1, r.min.y + 90.0)),
            id.with("prompt"),
            &mut ns.prompt,
            "Describe the task, e.g. \u{201c}Add pagination to the audit log endpoint\u{201d}",
            false,
        );
        if first {
            ui.memory_mut(|m| m.request_focus(id.with("prompt")));
        }
        let groups: [(&str, &[&str], &mut usize); 3] = [
            ("Project", &PROJECTS, &mut ns.project),
            ("Device", &["atlas", "forge", "runner-7"], &mut ns.device),
            ("Agent", &AGENTS, &mut ns.agent),
        ];
        let mut y = r.min.y + 118.0;
        for (gi, (name, opts, sel)) in groups.into_iter().enumerate() {
            pk::label(&p, pos2(x0, y), name, &t, 70.0);
            let mut x = x0 + 76.0;
            for (oi, o) in opts.iter().enumerate() {
                let cw = pk::chip_w(&p, o) + 6.0;
                if x + cw > x1 {
                    break;
                }
                let cr = Rect::from_min_size(pos2(x, y - 13.0), vec2(cw, 26.0));
                if pk::chip(ui, cr, id.with(("g", gi, oi)), o, *sel == oi, t.primary).clicked() {
                    *sel = oi;
                }
                x += cw + 6.0;
            }
            y += 40.0;
        }
        kit::hline(&p, r.min.x, r.max.x, r.max.y - 56.0, &t);
        kit::text_at(&p, pos2(x0, r.max.y - 28.0), Align2::LEFT_CENTER, "Runs in a fresh worktree; you approve shell commands.", 11.5, t.muted_foreground, w - 280.0);
        let start = pk::small_btn(ui, Rect::from_min_max(pos2(x1 - 150.0, r.max.y - 44.0), pos2(x1, r.max.y - 12.0)), id.with("start"), "Start session", None, Some(t.primary), Some("Ctrl ⏎"));
        let cancel = pk::small_btn(ui, Rect::from_min_max(pos2(x1 - 236.0, r.max.y - 44.0), pos2(x1 - 158.0, r.max.y - 12.0)), id.with("cancel"), "Cancel", None, None, None);
        let ctrl_enter = ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter));
        let esc = ui.input(|i| i.key_pressed(Key::Escape));
        let go = start.clicked() || ctrl_enter || sub;
        if go && !ns.prompt.trim().is_empty() {
            let title: String = ns.prompt.trim().chars().take(42).collect();
            let mut s = session(&title, PROJECTS[ns.project], &format!("agent/{}", title.to_lowercase().split_whitespace().take(3).collect::<Vec<_>>().join("-")), ns.device, AGENTS[ns.agent], AgentRunState::Live, "now", 4_200.0);
            s.events.push(TEv { ev: Ev::User(ns.prompt.trim().to_string()), born: -1000.0 });
            s.next_at = now + 0.6;
            s.script = vec![
                Ev::Text(st("I'll start by mapping the project layout.")),
                Ev::Tool("Bash", st("git ls-files | head -200"), st("187 files"), None),
                Ev::Tool("Read", st("README.md"), st("140 lines"), None),
                Ev::Text(st("Here's my plan:\n\n1. Find where this behavior lives\n2. Make the smallest change that works\n3. Add a regression test and run the suite\n\nStarting with step 1.")),
            ];
            self.sessions.insert(0, s);
            self.selected = 0;
            self.diff_file = 0;
            self.filter = 0;
            toast(ui.ctx(), Toast::success("Session started").description(format!("{} on {}", AGENTS[ns.agent], DEVICES[ns.device].0)));
            return;
        } else if go {
            toast(ui.ctx(), Toast::new("Describe the task first"));
        }
        let outside = scrim.clicked() && !ui.input(|i| i.pointer.interact_pos().is_some_and(|q| r.contains(q)));
        if cancel.clicked() || esc || outside {
            return;
        }
        self.composer = Some(ns);
    }
}

fn user_block(ui: &mut Ui, text: &str, t: &Theme) {
    let w = ui.available_width();
    let g = kit::wrapped(ui.painter(), text, 13.0, t.foreground, w - 40.0, 6);
    let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y + 20.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, 8.0, mix(t.background, t.foreground, if t.dark { 0.06 } else { 0.04 }));
    p.rect_filled(Rect::from_min_size(r.min + vec2(0.0, 8.0), vec2(2.5, r.height() - 16.0)), 2.0, t.primary);
    p.text(pos2(r.min.x + 14.0, r.min.y + 10.0 + 8.0), Align2::CENTER_CENTER, "›", FontId::monospace(13.0), t.muted_foreground);
    p.galley(pos2(r.min.x + 26.0, r.min.y + 10.0), g, t.foreground);
}

/// The approve / deny card. Returns Some(true) approve, Some(false) deny.
fn approval_bar(ui: &mut Ui, r: Rect, id: Id, cmd: &str, why: &str, agent: &str) -> Option<bool> {
    let t = Theme::of(ui);
    let p = ui.painter().clone();
    let now = ui.input(|i| i.time);
    let glow = ((now * 2.0).sin() * 0.5 + 0.5) as f32;
    p.rect_filled(r, 10.0, mix(t.card, t.warning, if t.dark { 0.07 } else { 0.05 }));
    p.rect_stroke(r, 10.0, Stroke::new(1.0, with_alpha(t.warning, 0.35 + 0.25 * glow)), StrokeKind::Inside);
    let x0 = r.min.x + 14.0;
    let x1 = r.max.x - 14.0;
    let y = r.min.y + 22.0;
    p.circle_filled(pos2(x0 + 9.0, y), 11.0, pk::soft(&t, t.warning, 1.6));
    AppIcon::Shield.paint(&p, Rect::from_center_size(pos2(x0 + 9.0, y), vec2(13.0, 13.0)), pk::tone_fg(&t, t.warning));
    // buttons
    let ap = Rect::from_min_max(pos2(x1 - 104.0, y - 14.0), pos2(x1, y + 14.0));
    let dn = Rect::from_min_max(pos2(ap.min.x - 82.0, y - 14.0), pos2(ap.min.x - 8.0, y + 14.0));
    let agent_short = agent.split(' ').next().unwrap_or("Agent");
    kit::text_at(&p, pos2(x0 + 28.0, y - 7.0), Align2::LEFT_CENTER, &format!("{agent_short} wants to run a command"), 13.0, t.foreground, dn.min.x - x0 - 36.0);
    kit::text_at(&p, pos2(x0 + 28.0, y + 9.0), Align2::LEFT_CENTER, why, 11.5, t.muted_foreground, dn.min.x - x0 - 36.0);
    let mut out = None;
    if pk::small_btn(ui, ap, id.with("ok"), "Approve", None, Some(t.primary), Some("⏎")).clicked() {
        out = Some(true);
    }
    if pk::small_btn(ui, dn, id.with("no"), "Deny", None, None, Some("Esc")).clicked() {
        out = Some(false);
    }
    // command box
    let cb = Rect::from_min_max(pos2(x0, r.min.y + 48.0), pos2(x1, r.max.y - 12.0));
    if cb.height() > 10.0 {
        p.rect_filled(cb, 7.0, if t.dark { mix(t.background, Color32::BLACK, 0.3) } else { mix(t.background, t.foreground, 0.035) });
        p.rect_stroke(cb, 7.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
        let pr = pk::mono(&p, pos2(cb.min.x + 12.0, cb.center().y), Align2::LEFT_CENTER, "$", 12.5, t.success, 20.0);
        let always = Rect::from_min_max(pos2(cb.max.x - 128.0, cb.center().y - 12.0), pos2(cb.max.x - 36.0, cb.center().y + 12.0));
        pk::mono(&p, pos2(pr.max.x + 8.0, cb.center().y), Align2::LEFT_CENTER, cmd, 12.5, t.foreground, always.min.x - pr.max.x - 16.0);
        let (al, hv) = kit::row(ui, always, id.with("always"), 6.0);
        kit::text_at(&p, always.center(), Align2::CENTER_CENTER, "Always allow", 11.5, t.muted_foreground.lerp_to_gamma(t.foreground, hv), always.width());
        kit::tip(&al, "Allow this command for the rest of the session", None);
        if al.clicked() {
            out = Some(true);
        }
        let cr = Rect::from_center_size(pos2(cb.max.x - 18.0, cb.center().y), vec2(24.0, 24.0));
        let c = pk::icon_btn(ui, cr, id.with("copy"), LineIcon::Copy.into(), "Copy command", None);
        if c.clicked() {
            ui.ctx().copy_text(cmd.to_string());
            toast(ui.ctx(), Toast::success("Copied command"));
        }
    }
    out
}
