use super::*;
use egui::Ui;
use glowkit_ai::Icon;
use glowkit_core::demo::{Category::Ai as C, Demo};
use glowkit_effects::ControlSlider;

fn mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, init: impl FnOnce() -> T) -> T {
    glowkit_core::state(ui, ui.scope_id().with(key), init)
}

fn set_mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, v: T) {
    glowkit_core::set_state(ui, ui.scope_id().with(key), v);
}

/// Seconds since this demo was first shown, scaled by `speed`, accumulated frame by frame so
/// changing the speed never jumps.
fn sim_clock(ui: &Ui, speed: f32) -> f32 {
    let dt = glowkit_core::dt(ui);
    glowkit_core::animate(ui);
    glowkit_core::with_state(ui, ui.scope_id().with("sim_clock"), || 0.0_f32, |t| {
        *t += dt * speed;
        *t
    })
}

/// Parameter values for a demo, persisted per demo.
fn params<const N: usize>(ui: &Ui, init: [f32; N]) -> [f32; N] {
    mem(ui, "params", || init)
}

/// Component on top, a centered row of parameter sliders (each `sw` wide) under it.
fn with_controls<const N: usize>(ui: &mut Ui, p: &mut [f32; N], sw: f32, controls: impl FnOnce(&mut Ui, &mut [f32; N], f32)) {
    ui.add_space(14.0);
    let gap = 10.0;
    let avail = ui.available_width();
    let sw = sw.min((avail - gap * (N as f32 - 1.0)) / N as f32);
    let total = sw * N as f32 + gap * (N as f32 - 1.0);
    ui.horizontal(|ui| {
        ui.add_space(((avail - total) / 2.0).max(0.0));
        ui.spacing_mut().item_spacing.x = gap;
        controls(ui, p, sw);
    });
    set_mem(ui, "params", *p);
}


/// State that turns on at `start`, finishes at `end` (with an optional needs-you window).
fn phase(t: f32, start: f32, end: f32) -> RunState {
    if t < start {
        RunState::Queued
    } else if t < end {
        RunState::Running
    } else {
        RunState::Done
    }
}

fn spent(t: f32, start: f32, end: f32, rate: f32) -> (f32, f32) {
    let s = (t.min(end) - start).max(0.0);
    (s * rate, s)
}

// ---------------------------------------------------------------------------------------------

fn tree_sim(t: f32) -> AgentNode {
    let t = t % 30.0;
    let node = |name: &str, model: &str, task: &str, s: f32, e: f32, rate: f32| {
        let (c, secs) = spent(t, s, e, rate);
        AgentNode::new(name, model).task(task).state(phase(t, s, e)).cost(c).secs(secs)
    };
    let mut writer = node("Writer", "Nova 2", "Draft copy for the three plans", 6.0, 24.0, 0.004);
    if (14.0..18.0).contains(&t) {
        writer.state = RunState::NeedsInput;
        writer.task = "Asks: keep the annual discount at 20%?".into();
    }
    let mut root = node("Orchestrator", "Nova Ultra", "Launch the new pricing page by Friday", 0.0, 28.0, 0.012);
    if t >= 28.0 {
        root.task = "All sub-agents reported back. Page is live.".into();
    }
    root.child(
        node("Researcher", "Nova 2", "Compare pricing of five competitors", 0.5, 13.0, 0.005)
            .child(node("Scraper", "Nova Flash", "Read pricing pages of Linear, Notion and three more", 1.0, 9.0, 0.0015))
            .child(node("Analyst", "Nova 2", "Tabulate seats, tiers and annual discounts", 9.0, 12.5, 0.003)),
    )
    .child(writer)
    .child(
        node("Coder", "Orbit 5", "Build the page and wire up checkout", 3.0, 27.0, 0.006)
            .child(node("Tester", "Nova Flash", "Run the checkout e2e suite", 18.0, 26.0, 0.002)),
    )
}

pub(super) fn all() -> Vec<Demo> {
    let mut v = vec![
        Demo {
            id: "agent-tree",
            name: "Agent Tree",
            category: C,
            description: "An orchestrator and its sub-agents as a live tree of cards: status, model, task, cost and time on each, lights running along connectors to working agents, branches that fold.",
            covers: &["affectUI: Agent Tree", "affectUI: Agent Crew"],
            code: r##"let root = AgentNode::new("Orchestrator", "Nova Ultra").state(RunState::Running)
    .task("Launch the new pricing page")
    .child(AgentNode::new("Researcher", "Nova 2").state(RunState::Running).cost(0.06)
        .child(AgentNode::new("Scraper", "Nova Flash").state(RunState::Done)))
    .child(AgentNode::new("Writer", "Nova 2").state(RunState::NeedsInput));
ui.add(AgentTree::new(&root).title("Agents"));"##,
            show: |ui| {
                let mut p = params(ui, [1.0, 210.0]);
                let t = sim_clock(ui, p[0]);
                let root = tree_sim(t);
                ui.add(AgentTree::new(&root).card_width(p[1]));
                with_controls(ui, &mut p, 200.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=3.0).decimals(1).suffix("x").width(w));
                    ui.add(ControlSlider::new("Card", &mut p[1], 160.0..=280.0).suffix("px").width(w));
                });
            },
        },
        Demo {
            id: "handoff-card",
            name: "Handoff Card",
            category: C,
            description: "A relay hand-off between two agents: a baton travels the track, the receiver lights up, and the context that went along drops in while what got lost is struck through.",
            covers: &["affectUI: Agent Handoff", "Creance: AgentHandoffBanner"],
            code: r##"ui.add(HandoffCard::new(("Researcher", "Nova 2"), ("Writer", "Nova Flash"))
    .reason("Compared 5 competitors; Notion and Linear are closest to us")
    .item(HandoffItem::new("competitors.md", 2400.0))
    .item(HandoffItem::new("raw page HTML", 18200.0).dropped())
    .progress(progress));"##,
            show: |ui| {
                let mut p = params(ui, [1.0]);
                let t = sim_clock(ui, p[0]);
                let prog = ((t % 7.0) / 3.6).min(1.0);
                ui.add(
                    HandoffCard::new(("Researcher", "Nova 2"), ("Writer", "Nova Flash"))
                        .reason("Compared 5 competitors: Notion and Linear are closest to our tiers")
                        .item(HandoffItem::new("competitors.md", 2400.0))
                        .item(HandoffItem::new("pricing-table.csv", 860.0).icon(Icon::Code))
                        .item(HandoffItem::new("Brand voice notes", 1200.0).icon(Icon::Bookmark))
                        .item(HandoffItem::new("raw page HTML", 18200.0).icon(Icon::Globe).dropped())
                        .progress(prog),
                );
                with_controls(ui, &mut p, 220.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=3.0).decimals(1).suffix("x").width(w));
                });
            },
        },
    ];
    v.extend(more());
    v
}

// ---------------------------------------------------------------------------------------------
// Simulation helpers

/// Seconds since the pointer last moved over this demo.
fn idle(ui: &Ui) -> f32 {
    let now = glowkit_core::time(ui);
    let inside = ui.rect_contains_pointer(ui.max_rect());
    let id = ui.scope_id().with("demo_idle");
    let since = glowkit_core::with_state(ui, id, || now - 10.0, |s| {
        if inside {
            *s = now;
        }
        *s
    });
    now - since
}

/// Fires once every `period` seconds of `t` (edge detector stored under `key`).
fn tick(ui: &Ui, key: &str, t: f32, period: f32) -> bool {
    let n = (t / period).floor() as i64;
    let id = ui.scope_id().with(("tick", key));
    glowkit_core::with_state(ui, id, || n, |last| {
        if *last != n {
            *last = n;
            true
        } else {
            false
        }
    })
}

fn noise(x: f32) -> f32 {
    glowkit_core::noise::noise01(x, 0.37, 1.3)
}

type SpanSpec = (&'static str, &'static str, SpanKind, usize, f32, f32, f32, f32);

fn trace_sim(t: f32) -> Vec<TraceSpan> {
    let t = t % 22.0;
    let spec: [SpanSpec; 12] = [
        ("Orchestrator", "Nova Ultra", SpanKind::Agent, 0, 0.0, 17.5, 0.0, 0.0),
        ("Plan the launch", "Nova Ultra", SpanKind::Llm, 1, 0.3, 4.2, 3900.0, 0.092),
        ("Researcher", "Nova 2", SpanKind::Agent, 1, 4.7, 7.4, 0.0, 0.0),
        ("web_search", "linear pricing", SpanKind::Tool, 2, 5.0, 2.6, 0.0, 0.004),
        ("web_search", "notion pricing", SpanKind::Tool, 2, 5.2, 3.1, 0.0, 0.004),
        ("vector_search", "past launches", SpanKind::Retrieval, 2, 8.4, 1.2, 600.0, 0.001),
        ("Summarize findings", "Nova 2", SpanKind::Llm, 2, 9.8, 2.2, 2100.0, 0.018),
        ("Writer", "Nova Flash", SpanKind::Agent, 1, 12.2, 5.0, 0.0, 0.0),
        ("read_file", "brand-voice.md", SpanKind::Tool, 2, 12.4, 0.6, 0.0, 0.0),
        ("figma.get_frames", "pricing v3", SpanKind::Tool, 2, 12.6, 1.8, 0.0, 0.0),
        ("Draft copy", "Nova Flash", SpanKind::Llm, 2, 14.5, 2.5, 1600.0, 0.006),
        ("Review copy", "Nova Ultra", SpanKind::Llm, 1, 17.0, 0.4, 900.0, 0.021),
    ];
    spec.iter()
        .filter(|s| t >= s.4)
        .map(|s| {
            let mut sp = TraceSpan::new(s.0, s.2, s.3, s.4).detail(s.1);
            if t >= s.4 + s.5 {
                sp = sp.dur(s.5).usage(s.6, s.7);
            } else {
                let f = (t - s.4) / s.5;
                sp = sp.usage(s.6 * f, s.7 * f);
            }
            if s.0 == "figma.get_frames" && t >= s.4 + s.5 {
                sp = sp.error(true);
            }
            sp
        })
        .collect()
}

fn pool_request(n: u64) -> ApprovalRequest {
    let pool = [
        ("Deploy Fixer", "shell", "pnpm add zod@3.23.8", "server/validate.ts imports zod, which is missing", Risk::Medium),
        ("Refund Resolver", "stripe.refunds.create", "charge=ch_3P9 amount=1240.00 reason=duplicate", "Customer was double charged on ORD-99214", Risk::High),
        ("Docs Writer", "github.create_pr", "docs: add rate limit section", "Explains the new 429 behaviour", Risk::Low),
        ("Data Loader", "postgres.query", "DELETE FROM leads WHERE stale = true", "1,200 stale lead records", Risk::High),
        ("Lead Enricher", "hubspot.update", "contacts=84 fields=company,size", "Backfill titles on 84 leads", Risk::Low),
        ("Upgrader", "shell", "npm i react-router@7 --save", "Upgrade to the new data router", Risk::Medium),
    ];
    let p = pool[(n as usize) % pool.len()];
    ApprovalRequest::new(n, p.0, p.1, p.2).reason(p.3).risk(p.4).waiting(10.0 + (n % 5) as f32 * 47.0)
}

fn diff_files() -> Vec<DiffFile> {
    vec![
        DiffFile::new("web/package.json")
            .note("server/validate.ts imports zod, which was missing, so the production build failed.")
            .hunk(DiffHunk::parse(8, 8, "\"dependencies\"", "   \"dependencies\": {\n     \"react\": \"^19.0.0\",\n+    \"zod\": \"^3.23.8\",\n     \"react-dom\": \"^19.0.0\"")),
        DiffFile::new("deploy/config.yaml")
            .note("Raise retries so a flaky registry does not fail the deploy, and install from the lockfile.")
            .hunk(DiffHunk::parse(10, 10, "deploy:", " deploy:\n   service: web\n-  timeout: 30\n-  retries: 0\n+  timeout: 60\n+  retries: 3\n   region: us-east-1"))
            .hunk(DiffHunk::parse(24, 25, "build:", " build:\n-  command: npm install && npm run build\n+  command: npm ci && npm run build\n   cache: true")),
    ]
}

fn runs_sim(t: f32) -> Vec<RunInfo> {
    let spec = [
        ("RUN-4822", "Refund Resolver", 0.0, 6, ["Plan refund", "Validate policy", "Capture refund", "Verify", "Notify", "Write back"]),
        ("RUN-4820", "Invoice Reconciler", 7.0, 5, ["Fetch invoices", "Match payments", "Flag gaps", "Draft report", "Send", ""]),
        ("RUN-4817", "Lead Enricher", 3.0, 4, ["Pull leads", "Enrich", "Approve CRM write", "Sync", "", ""]),
        ("RUN-4815", "Deploy Fixer", 12.0, 5, ["Read logs", "Patch deps", "Run tests", "Deploy", "Verify", ""]),
        ("RUN-4811", "Docs Writer", 18.0, 3, ["Outline", "Draft", "Open PR", "", "", ""]),
        ("RUN-4809", "Outreach Composer", 24.0, 4, ["Segment", "Draft emails", "Tone check", "Schedule", "", ""]),
    ];
    spec.iter()
        .enumerate()
        .map(|(i, s)| {
            let local = (t - s.2).rem_euclid(44.0);
            let per = 5.5;
            let step = ((local / per) as usize + 1).min(s.3);
            let mut state = if local < 0.8 { RunState::Queued } else if local < per * s.3 as f32 { RunState::Running } else { RunState::Done };
            if i == 2 && (8.0..17.0).contains(&local) {
                state = RunState::NeedsInput;
            }
            if i == 3 && local > 13.0 && local < 30.0 {
                state = RunState::Failed;
            }
            let el = local.min(per * s.3 as f32);
            let el = if state == RunState::Failed { 13.0 } else { el };
            let hist: Vec<f32> = (0..14).map(|k| noise(i as f32 * 3.1 + (el - 14.0 + k as f32) * 0.35) * (1.0 + k as f32 * 0.1)).collect();
            RunInfo::new(s.0, s.1, state)
                .step(step, s.3, s.4[(step - 1).min(5)])
                .usage(el + 40.0 * i as f32, el * 310.0 + 800.0, el * 0.0042 + 0.01)
                .history(hist)
        })
        .collect()
}

fn timeline_sim(t: f32) -> (Vec<TimelineStep>, bool) {
    let t = t % 34.0;
    let d = [4.0, 5.0, 6.0, 5.0, 4.0];
    let fail_from = 9.0 + 3.0;
    let fail_to = 19.0;
    let mut steps = Vec::new();
    let mut start = 0.0;
    let titles = ["Investigate the failed deploy", "Install the missing dependency", "Patch the deploy config", "Run the full test suite", "Deploy to production"];
    let tools: [&[(&str, Icon, &str)]; 5] = [
        &[("read_logs", Icon::Terminal, "service=web since=15m"), ("search_code", Icon::Search, "import { z } from \"zod\"")],
        &[("shell", Icon::Terminal, "pnpm add zod@3.23.8")],
        &[("read_file", Icon::File, "deploy/config.yaml"), ("edit_file", Icon::Pencil, "retries: 0 → 3")],
        &[("run_tests", Icon::Play, "412 tests · 3 workers")],
        &[("deploy", Icon::External, "web@prod")],
    ];
    let mut failed_now = false;
    for (i, title) in titles.iter().enumerate() {
        let mut dur = d[i];
        if i == 2 {
            dur += fail_to - fail_from;
        }
        let local = t - start;
        let state = if local < 0.0 {
            RunState::Queued
        } else if local < dur {
            RunState::Running
        } else {
            RunState::Done
        };
        let mut st = TimelineStep::new(*title, state).secs(local.clamp(0.0, dur));
        let mut state2 = state;
        if i == 2 && t > fail_from && t < fail_to {
            state2 = RunState::Failed;
            failed_now = true;
            st = st.note("edit_file failed: deploy/config.yaml was changed on main since it was read. Re-read and patch again.");
        }
        st.state = state2;
        for (k, (name, ic, det)) in tools[i].iter().enumerate() {
            let ts = k as f32 * dur / tools[i].len() as f32;
            let te = (k + 1) as f32 * dur / tools[i].len() as f32;
            let s = if state2 == RunState::Failed {
                if k == 0 { RunState::Done } else { RunState::Failed }
            } else if local < ts {
                continue;
            } else if local < te && state == RunState::Running {
                RunState::Running
            } else {
                RunState::Done
            };
            st = st.tool(StepTool::new(*name, *ic, s).secs((local - ts).clamp(0.0, te - ts)).detail(*det));
        }
        steps.push(st);
        start += dur;
    }
    (steps, failed_now)
}

fn log_sim(t: f32) -> Vec<LogLine> {
    let t = t % 180.0;
    let msgs = [
        (LogLevel::Info, "planner", "Plan has 5 steps, starting with read_logs"),
        (LogLevel::Debug, "llm", "request sonnet-4.5 · 3,412 tokens in"),
        (LogLevel::Info, "shell", "$ pnpm install --frozen-lockfile"),
        (LogLevel::Debug, "shell", "resolved 812 packages in 2.1s"),
        (LogLevel::Warn, "shell", "npm WARN deprecated glob@7.2.3: no longer supported"),
        (LogLevel::Info, "tests", "PASS  src/checkout.test.ts (41 tests)"),
        (LogLevel::Info, "tests", "PASS  src/pricing.test.ts (18 tests)"),
        (LogLevel::Error, "tests", "FAIL  src/webhooks.test.ts › retries on 503"),
        (LogLevel::Info, "agent", "Patching webhooks retry backoff (attempt 2)"),
        (LogLevel::Debug, "llm", "response 812 tokens out · 2.3s"),
        (LogLevel::Info, "git", "Committed 3 files on fix/webhook-retries"),
        (LogLevel::Warn, "mcp:github", "rate limit 80% used, slowing requests"),
    ];
    let mut out = Vec::new();
    let mut at = 0.0;
    let mut k = 0usize;
    while at <= t {
        let m = msgs[k % msgs.len()];
        out.push(LogLine::new(at, m.0, m.1, m.2));
        k += 1;
        at += 0.45 + 1.3 * noise(k as f32 * 1.7);
    }
    out
}

fn sessions_sim(t: f32) -> Vec<AgentSession> {
    let c = t % 36.0;
    let st = |a: f32, b: f32, inside: RunState, outside: RunState| if c >= a && c < b { inside } else { outside };
    vec![
        AgentSession::new(1, "Fix the failing production deploy", st(0.0, 20.0, RunState::Running, RunState::Done))
            .repo("acme/web · fix/zod")
            .device(Device::Laptop, "MacBook Pro")
            .preview(if c < 20.0 { "Running the full test suite, 312 of 412 passed…" } else { "Deployed web@prod. All checks green." })
            .activity(if c < 20.0 { 0 } else { 1 }, if c < 20.0 { 5.0 } else { c * 20.0 }),
        AgentSession::new(2, "Seed the staging database", st(6.0, 24.0, RunState::NeedsInput, RunState::Running))
            .repo("acme/data · main")
            .device(Device::Cloud, "sandbox-2")
            .preview(if (6.0..24.0).contains(&c) { "Which region should I seed first?" } else { "Loading 2.1 GB of fixtures into us-east-1…" })
            .activity(if (6.0..24.0).contains(&c) { 2 } else { 0 }, 40.0),
        AgentSession::new(3, "Upgrade react-router to v7", st(14.0, 36.0, RunState::Failed, RunState::Running))
            .repo("acme/web · chore/router-7")
            .device(Device::Terminal, "codex · devbox")
            .preview(if c >= 14.0 { "Type error in routes.tsx: loader returns Promise<void>" } else { "Migrating 23 route modules to the data router…" })
            .activity(if c >= 14.0 { 3 } else { 0 }, 300.0),
        AgentSession::new(4, "Write release notes for v2.4", RunState::Idle).repo("acme/docs · main").device(Device::Phone, "iPhone").preview("Draft ready for review: 23 merged PRs grouped into 5 sections").activity(0, 3600.0 * 3.0),
        AgentSession::new(5, "Weekly metrics digest", RunState::Done).repo("acme/analytics").device(Device::Cloud, "scheduled").preview("Posted to #metrics: signups +12% week over week").activity(0, 86400.0),
        AgentSession::new(6, "Triage new GitHub issues", st(26.0, 36.0, RunState::Running, RunState::Idle)).repo("acme/web").device(Device::Laptop, "Studio").preview("Labelled 14 issues, 2 look like duplicates").activity(0, 7200.0),
    ]
}

fn fleet_sim(t: f32) -> Vec<FleetAgent> {
    let names = [
        ("Deploy Fixer", "DevOps", "Reading build logs"),
        ("Docs Writer", "Docs", "Drafting the API guide"),
        ("Data Loader", "Data", "Which region first?"),
        ("Upgrader", "Maintenance", "react-router v7 type errors"),
        ("Refund Resolver", "Support", "Capturing a partial refund"),
        ("Lead Enricher", "Sales", "Enriching 84 leads"),
        ("Invoice Reconciler", "Finance", "Matching 312 payments"),
        ("Test Runner", "QA", "Waiting for a build"),
    ];
    names
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let cyc = 14.0 + i as f32 * 2.3;
            let local = (t + i as f32 * 5.1) % cyc;
            let mut state = if local < cyc * 0.75 { RunState::Running } else { RunState::Done };
            if i == 2 && (t % 30.0) < 18.0 {
                state = RunState::NeedsInput;
            }
            if i == 3 {
                state = if (t % 40.0) < 22.0 { RunState::Failed } else { RunState::Running };
            }
            if i == 7 {
                state = RunState::Idle;
            }
            let prog = (local / (cyc * 0.75)).min(1.0);
            let act = if state == RunState::Done { "Finished, waiting for the next job" } else { n.2 };
            FleetAgent::new(n.0, n.1, state).activity(prog, act).stats(12 + (i as u32 * 7) % 30 + (t / 10.0) as u32, 0.78 + 0.2 * noise(i as f32))
        })
        .collect()
}

fn plan_sim(t: f32) -> Vec<PlanCard> {
    let t = t % 30.0;
    let spec = [
        ("Read the failing build log", "Deploy Fixer", Icon::Terminal, 0.0, 3.5),
        ("Find where zod is imported", "Deploy Fixer", Icon::Search, 3.5, 3.0),
        ("Add zod to web dependencies", "Deploy Fixer", Icon::Code, 6.5, 4.0),
        ("Raise deploy retries to 3", "Deploy Fixer", Icon::Pencil, 10.5, 3.5),
        ("Approve the production deploy", "You", Icon::Lock, 14.0, 5.0),
        ("Deploy and watch error rates", "Deploy Fixer", Icon::External, 19.0, 6.0),
    ];
    spec.iter()
        .enumerate()
        .map(|(i, s)| {
            let local = t - s.3;
            let mut state = if local < 0.0 { RunState::Queued } else if local < s.4 { RunState::Running } else { RunState::Done };
            if i == 4 && state == RunState::Running {
                state = RunState::NeedsInput;
            }
            PlanCard::new(i as u64, s.0, state).agent(s.1, s.2).progress((local / s.4).clamp(0.0, 1.0), local.clamp(0.0, s.4))
        })
        .collect()
}

fn more() -> Vec<Demo> {
    vec![
        Demo {
            id: "trace-waterfall",
            name: "Trace Waterfall",
            category: C,
            description: "A run's trace as a waterfall: summary strip, folding span tree, bars by kind on a time axis with duration and cost, the slowest call tagged, a live now-line and hover details.",
            covers: &["affectUI: Trace Waterfall", "affectUI: Cost Waterfall", "ReUI: Run Step Trace"],
            code: r##"let spans = vec![
    TraceSpan::new("Orchestrator", SpanKind::Agent, 0, 0.0).dur(17.5),
    TraceSpan::new("Plan the launch", SpanKind::Llm, 1, 0.3).dur(4.2).usage(3900.0, 0.092),
    TraceSpan::new("web_search", SpanKind::Tool, 1, 4.7), // still running
];
ui.add(TraceWaterfall::new(&spans).title("Launch the pricing page").now(t));"##,
            show: |ui| {
                let mut p = params(ui, [1.0, 30.0]);
                let t = sim_clock(ui, p[0]);
                let spans = trace_sim(t + 11.0);
                ui.add(TraceWaterfall::new(&spans).title("Launch the new pricing page by Friday").now((t + 11.0) % 22.0).row_height(p[1]));
                with_controls(ui, &mut p, 200.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=3.0).decimals(1).suffix("x").width(w));
                    ui.add(ControlSlider::new("Row", &mut p[1], 24.0..=40.0).suffix("px").width(w));
                });
            },
        },
        Demo {
            id: "approval-queue",
            name: "Approval Queue",
            category: C,
            description: "Pending tool calls to approve, deny or edit, with risk badges, the exact command, waiting time, batch actions and rows that resolve and fold away.",
            covers: &["ReUI: Action Approval Inbox", "affectUI: Approval Gates", "affectUI: Approval Card", "Creance: ApprovalPrompt", "Creance: RiskLabel"],
            code: r##"let mut queue = vec![
    ApprovalRequest::new(1, "Deploy Fixer", "shell", "pnpm add zod@3.23.8").risk(Risk::Medium),
    ApprovalRequest::new(2, "Refund Resolver", "stripe.refunds.create", "amount=1240.00").risk(Risk::High),
];
ui.add(ApprovalQueue::new(&mut queue));"##,
            show: |ui| {
                let mut p = params(ui, [1.0]);
                let t = sim_clock(ui, p[0]);
                let dt = glowkit_core::dt(ui) * p[0];
                let idle_s = idle(ui);
                let mut q: Vec<ApprovalRequest> = mem(ui, "q", || (0..4).map(pool_request).collect());
                let mut next: u64 = mem(ui, "next", || 4);
                let mut gone: Vec<(u64, f32)> = mem(ui, "gone", Vec::new);
                for r in q.iter_mut().filter(|r| r.decision.is_none()) {
                    r.waiting += dt;
                }
                let pending = q.iter().filter(|r| r.decision.is_none()).count();
                if tick(ui, "add", t, 3.5) && pending < 4 {
                    q.push(pool_request(next));
                    next += 1;
                }
                if idle_s > 2.0 && tick(ui, "auto", t, 2.6) {
                    if let Some(r) = q.iter_mut().find(|r| r.decision.is_none()) {
                        r.decision = Some(if r.risk == Risk::High { Decision::Denied } else { Decision::Approved });
                    }
                }
                for r in q.iter().filter(|r| r.decision.is_some()) {
                    if !gone.iter().any(|g| g.0 == r.id) {
                        gone.push((r.id, t));
                    }
                }
                q.retain(|r| !gone.iter().any(|g| g.0 == r.id && t - g.1 > 2.2 / p[0].max(0.2)));
                gone.retain(|g| q.iter().any(|r| r.id == g.0));
                ui.add(ApprovalQueue::new(&mut q));
                set_mem(ui, "q", q);
                set_mem(ui, "next", next);
                set_mem(ui, "gone", gone);
                with_controls(ui, &mut p, 220.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Speed", &mut p[0], 0.2..=3.0).decimals(1).suffix("x").width(w));
                });
            },
        },
        Demo {
            id: "diff-review",
            name: "Diff Review",
            category: C,
            description: "An agent's file changes for review: unified or side-by-side diff, per-hunk accept and reject that fold the hunk, J/K/Y/N keys, and an apply footer.",
            covers: &["Creance: DiffReview", "Creance: DiffLine", "affectUI: Inline Diff", "affectUI: Review Center"],
            code: r##"let mut files = vec![DiffFile::new("web/package.json")
    .note("server/validate.ts imports zod, which was missing")
    .hunk(DiffHunk::parse(8, 8, "\"dependencies\"", "   \"react\": \"^19.0.0\",\n+  \"zod\": \"^3.23.8\",\n   \"react-dom\": \"^19.0.0\""))];
ui.add(DiffReview::new(&mut files));"##,
            show: |ui| {
                let mut p = params(ui, [0.0]);
                let now = glowkit_core::time(ui);
                let idle_s = idle(ui);
                let mut files: Vec<DiffFile> = mem(ui, "files", diff_files);
                let all_done = files.iter().all(|f| f.hunks.iter().all(|h| h.decision.is_some()));
                if idle_s > 2.5 && tick(ui, "auto", now, 2.4) {
                    if all_done {
                        files = diff_files();
                    } else if let Some(h) = files.iter_mut().flat_map(|f| f.hunks.iter_mut()).find(|h| h.decision.is_none()) {
                        h.decision = Some(!h.header.starts_with("build"));
                    }
                }
                ui.add(DiffReview::new(&mut files).split(p[0] > 0.5));
                set_mem(ui, "files", files);
                with_controls(ui, &mut p, 220.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Split view", &mut p[0], 0.0..=1.0).width(w));
                });
            },
        },
        Demo {
            id: "run-monitor",
            name: "Run Monitor",
            category: C,
            description: "Live agent runs in one table: filter tabs with rolling counts, status, step progress, ticking elapsed time, tokens and a cost sparkline; rows glide as runs change state.",
            covers: &["ReUI: Agent Run Queue", "ReUI: Agent Fleet Overview", "affectUI: Running Now", "affectUI: Agent Inbox"],
            code: r##"let runs = vec![
    RunInfo::new("RUN-4822", "Refund Resolver", RunState::Running).step(3, 6, "Capture refund").usage(84.0, 12_400.0, 0.21),
    RunInfo::new("RUN-4817", "Lead Enricher", RunState::NeedsInput).step(3, 4, "Approve CRM write"),
];
ui.add(RunMonitor::new(&runs));"##,
            show: |ui| {
                let mut p = params(ui, [1.0, 6.0]);
                let t = sim_clock(ui, p[0]);
                let runs = runs_sim(t + 14.0);
                ui.add(RunMonitor::new(&runs).max_rows(p[1] as usize));
                with_controls(ui, &mut p, 200.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=3.0).decimals(1).suffix("x").width(w));
                    ui.add(ControlSlider::new("Rows", &mut p[1], 2.0..=6.0).width(w));
                });
            },
        },
        Demo {
            id: "run-timeline",
            name: "Run Timeline",
            category: C,
            description: "An agent run as a quiet trail of steps: tool calls with live spinners, finished steps that fold, a failed step with its error and Retry, and a light running down the rail.",
            covers: &["Creance: RunTimeline", "Creance: ToolCallCard", "affectUI: Agent Trail", "Creative Tim: Tool Use"],
            code: r##"let steps = vec![
    TimelineStep::new("Investigate the failed deploy", RunState::Done).secs(4.0)
        .tool(StepTool::new("read_logs", Icon::Terminal, RunState::Done).secs(1.2)),
    TimelineStep::new("Install the missing dependency", RunState::Running)
        .tool(StepTool::new("shell", Icon::Terminal, RunState::Running).detail("pnpm add zod")),
];
let out = RunTimeline::new("Fix the failing deploy", &steps).show(ui);"##,
            show: |ui| {
                let mut p = params(ui, [1.0, 560.0]);
                let t = sim_clock(ui, p[0]);
                let (steps, _) = timeline_sim(t + 13.0);
                let out = RunTimeline::new("Fix the failing production deploy", &steps).width(p[1]).show(ui);
                if out.retry.is_some() {
                    // jump past the failure window
                    glowkit_core::with_state(ui, ui.scope_id().with("sim_clock"), || 0.0_f32, |c| {
                        let base = (*c / 34.0).floor() * 34.0;
                        *c = base + 6.0;
                    });
                }
                with_controls(ui, &mut p, 200.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=3.0).decimals(1).suffix("x").width(w));
                    ui.add(ControlSlider::new("Width", &mut p[1], 380.0..=760.0).suffix("px").width(w));
                });
            },
        },
        Demo {
            id: "cost-meter",
            name: "Cost Meter",
            category: C,
            description: "Spend against a budget: a big gliding total, a budget bar that turns amber and then red with a pulsing over-limit tag, tokens in and out, and spend split by model.",
            covers: &["Creance: BudgetMeter"],
            code: r##"ui.add(CostMeter::new(10.0)
    .model("Nova Ultra", 42_000.0, 6_100.0, 2.41)
    .model("Nova 2", 120_000.0, 18_000.0, 1.62)
    .model("Nova Flash", 300_000.0, 22_000.0, 0.38));"##,
            show: |ui| {
                let mut p = params(ui, [1.0, 6.0]);
                let t = (sim_clock(ui, p[0]) + 26.0) % 40.0;
                let k = t / 40.0;
                ui.add(
                    CostMeter::new(p[1])
                        .title("Spend", "Today")
                        .model("Nova Ultra", 42_000.0 * k, 6_100.0 * k, 4.1 * k)
                        .model("Nova 2", 120_000.0 * k, 18_000.0 * k, 2.2 * k + 0.2)
                        .model("Nova Flash", 300_000.0 * k, 22_000.0 * k, 0.6 * k + 0.05),
                );
                with_controls(ui, &mut p, 200.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=3.0).decimals(1).suffix("x").width(w));
                    ui.add(ControlSlider::new("Budget $", &mut p[1], 2.0..=12.0).width(w));
                });
            },
        },
        Demo {
            id: "log-stream",
            name: "Log Stream",
            category: C,
            description: "Live tailing logs: level filter chips with counts, a Follow switch, lines that slide in with a highlight, and a Jump to latest pill when you scroll back.",
            covers: &[],
            code: r##"ui.add(LogStream::new(&lines).height(300.0));"##,
            show: |ui| {
                let mut p = params(ui, [1.0, 300.0]);
                let t = sim_clock(ui, p[0]);
                let lines = log_sim(t + 40.0);
                ui.add(LogStream::new(&lines).title("Deploy Fixer · logs").height(p[1]));
                with_controls(ui, &mut p, 200.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=3.0).decimals(1).suffix("x").width(w));
                    ui.add(ControlSlider::new("Height", &mut p[1], 200.0..=420.0).suffix("px").width(w));
                });
            },
        },
        Demo {
            id: "sandbox-status",
            name: "Sandbox Status",
            category: C,
            description: "The container an agent works in: state, uptime, gliding CPU, memory and disk gauges that warn past 75%, a CPU sparkline, network, ports and actions.",
            covers: &[],
            code: r##"ui.add(SandboxStatus::new("sbx-7f3a2c").image("ubuntu-24.04 · python 3.12 · node 22")
    .state(RunState::Running).cpu(0.42).memory(2.8, 8.0).disk(11.0, 40.0)
    .uptime(1834.0).network(1.2, 0.3).port(3000, "preview"));"##,
            show: |ui| {
                let mut p = params(ui, [1.0, 1.0]);
                let t = sim_clock(ui, p[0]);
                let cpu = |x: f32| (0.15 + 0.75 * noise(x * 0.25) * p[1]).clamp(0.02, 0.99);
                let hist: Vec<f32> = (0..30).map(|k| cpu(t - 30.0 + k as f32)).collect();
                ui.add(
                    SandboxStatus::new("sbx-7f3a2c")
                        .image("ubuntu-24.04 · python 3.12 · node 22")
                        .state(RunState::Running)
                        .cpu(cpu(t))
                        .memory(2.2 + 4.5 * noise(t * 0.05 + 3.0) * p[1], 8.0)
                        .disk(11.0 + t * 0.02 % 20.0, 40.0)
                        .uptime(1834.0 + t)
                        .network(0.4 + 2.0 * noise(t * 0.4 + 9.0), 0.1 + 0.6 * noise(t * 0.3 + 1.0))
                        .cpu_history(hist)
                        .port(3000, "preview")
                        .port(5432, "postgres"),
                );
                with_controls(ui, &mut p, 200.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=3.0).decimals(1).suffix("x").width(w));
                    ui.add(ControlSlider::new("Load", &mut p[1], 0.2..=1.3).decimals(1).width(w));
                });
            },
        },
        Demo {
            id: "agent-session-list",
            name: "Agent Session List",
            category: C,
            description: "Agent sessions across laptops, cloud sandboxes, CLIs and phones: live, idle and needs-input states, unread pills, search and tabs, rows gliding between sections.",
            covers: &["affectUI: Agent Inbox", "Creance: NeedsYouInbox", "Creance: StatusBadge"],
            code: r##"let mut selected = Some(1);
ui.add(AgentSessionList::new(&sessions, &mut selected));"##,
            show: |ui| {
                let mut p = params(ui, [1.0, 470.0]);
                let t = sim_clock(ui, p[0]);
                let sessions = sessions_sim(t + 8.0);
                let mut sel: Option<u64> = mem(ui, "sel", || Some(1));
                ui.add(AgentSessionList::new(&sessions, &mut sel).height(p[1]));
                set_mem(ui, "sel", sel);
                with_controls(ui, &mut p, 200.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=3.0).decimals(1).suffix("x").width(w));
                    ui.add(ControlSlider::new("Height", &mut p[1], 300.0..=560.0).suffix("px").width(w));
                });
            },
        },
        Demo {
            id: "session-detail-header",
            name: "Session Detail Header",
            category: C,
            description: "The control bar of one agent session: title, repo, branch and device, live state and clock, step pips, token budget, a Needs-you pill, Pause and a hold-to-stop kill switch.",
            covers: &["Creance: AgentRunLayout", "Creance: KillSwitch", "affectUI: Run Controls"],
            code: r##"let action = SessionDetailHeader::new("Deploy Fixer", "Fix the failing production deploy")
    .meta("acme/web", "fix/missing-zod", "MacBook Pro")
    .state(RunState::Running, 142.0).step(3, 5, "Running the test suite")
    .budget(18_400.0, 50_000.0).needs_you(2).model("Nova 2")
    .show(ui);"##,
            show: |ui| {
                let mut p = params(ui, [1.0]);
                let dt = glowkit_core::dt(ui) * p[0];
                glowkit_core::animate(ui);
                // (elapsed, paused, stopped_for)
                let (mut el, mut paused, mut stopped): (f32, bool, f32) = mem(ui, "s", || (40.0, false, 0.0));
                if stopped > 0.0 {
                    stopped += dt;
                    if stopped > 3.0 {
                        el = 0.0;
                        stopped = 0.0;
                    }
                } else if !paused {
                    el += dt;
                }
                let step = ((el / 9.0) as usize % 5) + 1;
                let labels = ["Reading the build log", "Installing zod", "Patching the deploy config", "Running the test suite", "Deploying to production"];
                let state = if stopped > 0.0 { RunState::Done } else if paused { RunState::Paused } else { RunState::Running };
                let act = SessionDetailHeader::new("Deploy Fixer", "Fix the failing production deploy")
                    .meta("acme/web", "fix/missing-zod", "MacBook Pro")
                    .state(state, el)
                    .step(step, 5, labels[step - 1])
                    .budget(4_000.0 + el * 260.0 % 46_000.0, 50_000.0)
                    .needs_you(if step >= 3 { 2 } else { 0 })
                    .model("Nova 2")
                    .show(ui);
                match act {
                    Some(SessionAction::Pause) => paused = true,
                    Some(SessionAction::Resume) => paused = false,
                    Some(SessionAction::Stop) => stopped = 0.01,
                    _ => {}
                }
                set_mem(ui, "s", (el, paused, stopped));
                with_controls(ui, &mut p, 220.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=3.0).decimals(1).suffix("x").width(w));
                });
            },
        },
        Demo {
            id: "mcp-server-list",
            name: "MCP Server List",
            category: C,
            description: "MCP servers with connect status, transport, tool counts, latency bars and switches; switching one on plays a handshake, rows unfold to their tools.",
            covers: &["ReUI: Tool Access Matrix", "affectUI: Tool Access Map"],
            code: r##"let mut servers = vec![
    McpServer::new("GitHub", "http", "https://api.githubcopilot.com/mcp/").tools(&["create_issue", "search_code"]),
    McpServer::new("Postgres", "stdio", "npx @mcp/postgres $DATABASE_URL").enabled(false),
];
ui.add(McpServerList::new(&mut servers));"##,
            show: |ui| {
                let mut p = params(ui, [1.0]);
                let t = sim_clock(ui, p[0]);
                let idle_s = idle(ui);
                let mut servers: Vec<McpServer> = mem(ui, "servers", || {
                    vec![
                        McpServer::new("GitHub", "http", "https://api.githubcopilot.com/mcp/").tools(&["create_issue", "create_pr", "search_code", "get_file", "list_commits", "add_comment"]).latency(vec![48.0, 52.0, 61.0, 44.0, 58.0, 72.0, 50.0, 47.0]),
                        McpServer::new("Linear", "http", "https://mcp.linear.app/sse").tools(&["list_issues", "save_issue", "list_projects"]).latency(vec![120.0, 180.0, 340.0, 260.0, 150.0, 130.0, 310.0, 140.0]),
                        McpServer::new("Postgres", "stdio", "npx @mcp/postgres $DATABASE_URL").tools(&["query", "list_tables", "describe"]).enabled(false),
                        McpServer::new("Figma", "http", "http://127.0.0.1:3845/mcp").tools(&["get_frames", "get_code"]).error("Desktop app not running"),
                        McpServer::new("Filesystem", "stdio", "npx @mcp/fs ~/projects").tools(&["read_file", "write_file", "list_dir", "search"]).latency(vec![6.0, 8.0, 5.0, 7.0, 9.0, 6.0, 5.0, 8.0]),
                    ]
                });
                if tick(ui, "lat", t, 0.8) {
                    for (i, s) in servers.iter_mut().enumerate() {
                        let v = 30.0 + 90.0 * noise(t * 0.5 + i as f32 * 7.0) + if i == 1 { 220.0 * noise(t * 0.2) } else { 0.0 };
                        s.latency.push(v);
                        if s.latency.len() > 12 {
                            s.latency.remove(0);
                        }
                    }
                }
                if idle_s > 2.5 && tick(ui, "auto", t, 5.0) {
                    servers[2].enabled = !servers[2].enabled;
                }
                ui.add(McpServerList::new(&mut servers));
                set_mem(ui, "servers", servers);
                with_controls(ui, &mut p, 220.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=3.0).decimals(1).suffix("x").width(w));
                });
            },
        },
        Demo {
            id: "skills-browser",
            name: "Skills Browser",
            category: C,
            description: "Agent skills and plugins as cards with search and category tabs; Install fills across the button and morphs into Installed, which offers Remove on hover.",
            covers: &[],
            code: r##"let mut skills = vec![
    Skill::new("PDF tools", "Lumen Labs", "Documents").desc("Read, fill and merge PDFs").installs(48_200),
    Skill::new("Postgres", "Community", "Data").desc("Query and migrate databases").installed(true),
];
ui.add(SkillsBrowser::new(&mut skills));"##,
            show: |ui| {
                let mut p = params(ui, [250.0]);
                let now = glowkit_core::time(ui);
                let idle_s = idle(ui);
                let fresh = || {
                    vec![
                        Skill::new("PDF tools", "Lumen Labs", "Documents").desc("Read, fill, split and merge PDF files, including forms.").icon(Icon::File).installs(48_200).installed(true),
                        Skill::new("Spreadsheets", "Lumen Labs", "Documents").desc("Build and edit xlsx files with formulas and charts.").icon(Icon::Code).installs(39_400),
                        Skill::new("Postgres", "Community", "Data").desc("Query, explain and migrate Postgres databases safely.").icon(Icon::Terminal).installs(12_900),
                        Skill::new("Web research", "Community", "Research").desc("Search, read and cite sources with quotes and dates.").icon(Icon::Globe).installs(27_100),
                        Skill::new("Code review", "Lumen Labs", "Engineering").desc("Review diffs for bugs, reuse and simplification.").icon(Icon::Eye).installs(51_800),
                        Skill::new("Release notes", "Community", "Engineering").desc("Group merged PRs into readable release notes.").icon(Icon::Bookmark).installs(6_300),
                    ]
                };
                let mut skills: Vec<Skill> = mem(ui, "skills", fresh);
                if idle_s > 2.5 && tick(ui, "auto", now, 3.0) {
                    match skills.iter().position(|s| !s.installed) {
                        Some(i) if i % 2 == 1 || skills.iter().filter(|s| !s.installed).count() > 3 => {
                            // let the browser animate the install by flipping through a click-free path
                            skills[i].installed = true;
                        }
                        Some(i) => skills[i].installed = true,
                        None => skills = fresh(),
                    }
                }
                ui.add(SkillsBrowser::new(&mut skills).card_width(p[0]));
                set_mem(ui, "skills", skills);
                with_controls(ui, &mut p, 220.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Card", &mut p[0], 210.0..=340.0).suffix("px").width(w));
                });
            },
        },
        Demo {
            id: "agent-fleet-grid",
            name: "Agent Fleet Grid",
            category: C,
            description: "Many agents as tiles with status rings: progress arcs with a glowing head while working, a pulsing amber ring when one needs you, red on failure, plus runs and success rate.",
            covers: &["ReUI: Agent Fleet Overview", "affectUI: Agent Fleet", "affectUI: Fan-out Grid", "Creative Tim: Workflow Status"],
            code: r##"let fleet = vec![
    FleetAgent::new("Deploy Fixer", "DevOps", RunState::Running).activity(0.4, "Reading build logs").stats(31, 0.97),
    FleetAgent::new("Data Loader", "Data", RunState::NeedsInput).activity(0.7, "Which region first?"),
];
ui.add(AgentFleetGrid::new(&fleet));"##,
            show: |ui| {
                let mut p = params(ui, [1.0, 200.0]);
                let t = sim_clock(ui, p[0]);
                let fleet = fleet_sim(t);
                ui.add(AgentFleetGrid::new(&fleet).tile_width(p[1]));
                with_controls(ui, &mut p, 200.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=3.0).decimals(1).suffix("x").width(w));
                    ui.add(ControlSlider::new("Tile", &mut p[1], 170.0..=280.0).suffix("px").width(w));
                });
            },
        },
        Demo {
            id: "eval-scoreboard",
            name: "Eval Scoreboard",
            category: C,
            description: "An eval report: overall pass rate with its delta and trend, suites as stacked pass / flaky / fail bars that grow as cases finish, and regressions tagged.",
            covers: &["ReUI: Agent Eval Report"],
            code: r##"let suites = vec![
    EvalSuite::new("Refund flows", 48).results(46, 1, 1).delta(2.1),
    EvalSuite::new("Outreach tone", 42).results(30, 8, 0).delta(-4.8),
];
ui.add(EvalScoreboard::new(&suites).baseline("v2.0.3").trend(vec![91.0, 93.2, 94.6, 92.1, 94.0]));"##,
            show: |ui| {
                let mut p = params(ui, [1.0]);
                let t = (sim_clock(ui, p[0]) + 10.0) % 30.0;
                let prog = (t / 18.0).min(1.0);
                let suite = |name: &str, total: u32, pass: f32, fail: f32, flaky: f32, delta: f32, lag: f32| {
                    let f = ((prog - lag) / (1.0 - lag)).clamp(0.0, 1.0);
                    let done = (total as f32 * f) as u32;
                    let fl = (done as f32 * flaky) as u32;
                    let fa = (done as f32 * fail) as u32;
                    let _ = pass;
                    EvalSuite::new(name, total).results(done - fl - fa, fa, fl).delta(delta)
                };
                let suites = vec![
                    suite("Refund flows", 48, 0.96, 0.02, 0.02, 2.1, 0.0),
                    suite("Data hygiene", 54, 0.96, 0.0, 0.04, 0.9, 0.1),
                    suite("Outreach tone", 42, 0.8, 0.19, 0.0, -4.8, 0.2),
                    suite("Tool selection", 60, 0.97, 0.03, 0.0, 1.4, 0.35),
                    suite("Long context recall", 20, 0.9, 0.05, 0.05, 0.3, 0.5),
                ];
                ui.add(EvalScoreboard::new(&suites).baseline("v2.0.3").trend(vec![90.2, 91.0, 92.8, 92.1, 93.6, 94.4, 93.9, 94.6, 92.0]));
                with_controls(ui, &mut p, 220.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=3.0).decimals(1).suffix("x").width(w));
                });
            },
        },
        Demo {
            id: "memory-viewer",
            name: "Memory Viewer",
            category: C,
            description: "What an agent remembers: kind badges, source, a recency dot that cools with age, confidence bars, pins that float to the top and new memories sliding in.",
            covers: &["ReUI: Agent Memory", "affectUI: Memory Panel"],
            code: r##"let mut mem = vec![
    MemoryEntry::new(1, MemoryKind::Preference, "Acme prefers email over phone").meta("Chat · Support", 3600.0, 0.96),
    MemoryEntry::new(2, MemoryKind::Policy, "Refunds above $500 need finance approval").pinned(true),
];
ui.add(MemoryViewer::new(&mut mem));"##,
            show: |ui| {
                let mut p = params(ui, [1.0, 6.0]);
                let t = sim_clock(ui, p[0]);
                let dt = glowkit_core::dt(ui) * p[0];
                let pool = [
                    (MemoryKind::Fact, "Globex pays invoices through Coupa, net 45", "Invoice Reconciler"),
                    (MemoryKind::Procedure, "Re-run flaky webhook tests once before failing a build", "Deploy Fixer"),
                    (MemoryKind::Preference, "Sam wants PR descriptions under 10 lines", "Chat · Sam"),
                    (MemoryKind::Fact, "Staging seeds live in s3://acme-fixtures/2026", "Data Loader"),
                ];
                let mut entries: Vec<MemoryEntry> = mem(ui, "e", || {
                    vec![
                        MemoryEntry::new(1, MemoryKind::Policy, "Refunds above $500 need a finance second approval").meta("Policy doc · billing", 86400.0 * 30.0, 0.98).pinned(true),
                        MemoryEntry::new(2, MemoryKind::Preference, "Acme Corp prefers email over phone calls").meta("Chat · Support", 3600.0 * 5.0, 0.96),
                        MemoryEntry::new(3, MemoryKind::Procedure, "Refund disputes older than 7 days go to the billing queue").meta("Refund Resolver", 86400.0 * 2.0, 0.91),
                        MemoryEntry::new(4, MemoryKind::Fact, "Order ORD-99102 uses a manual refund record").meta("Refund Resolver", 86400.0 * 26.0, 0.74),
                    ]
                });
                for e in entries.iter_mut() {
                    e.age += dt * 60.0;
                }
                if tick(ui, "add", t, 6.0) && t > 1.0 {
                    let n = (t / 6.0) as u64 + 10;
                    let pe = pool[(n as usize) % pool.len()];
                    entries.push(MemoryEntry::new(n, pe.0, pe.1).meta(pe.2, 5.0, 0.8 + 0.18 * noise(n as f32)));
                    if entries.len() > 8 {
                        if let Some(i) = entries.iter().position(|e| !e.pinned) {
                            entries.remove(i);
                        }
                    }
                }
                ui.add(MemoryViewer::new(&mut entries).max_rows(p[1] as usize));
                set_mem(ui, "e", entries);
                with_controls(ui, &mut p, 200.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=3.0).decimals(1).suffix("x").width(w));
                    ui.add(ControlSlider::new("Rows", &mut p[1], 3.0..=8.0).width(w));
                });
            },
        },
        Demo {
            id: "schedule-list",
            name: "Schedule List",
            category: C,
            description: "Scheduled agent jobs with countdown rings, ticking next-run times, pips for recent outcomes, Run now and on/off switches; jobs fire and record their result.",
            covers: &["affectUI: Scheduled Runs"],
            code: r##"let mut jobs = vec![ScheduledJob::new("Morning digest", "Digest Writer", "Weekdays at 08:30").next(5400.0, 86400.0)];
let out = ScheduleList::new(&mut jobs).show(ui);"##,
            show: |ui| {
                let mut p = params(ui, [60.0]);
                let dt = glowkit_core::dt(ui) * p[0];
                glowkit_core::animate(ui);
                let mut jobs: Vec<(ScheduledJob, f32)> = mem(ui, "jobs", || {
                    use RunState::{Done as D, Failed as F};
                    vec![
                        (ScheduledJob::new("Triage new issues", "Issue Triager", "Every 15 minutes").next(140.0, 900.0).history(vec![D, D, D, F, D, D, D]), 0.0),
                        (ScheduledJob::new("Morning metrics digest", "Digest Writer", "Weekdays at 08:30").next(3600.0 * 5.2, 86400.0).history(vec![D, D, D, D, D]), 0.0),
                        (ScheduledJob::new("Dependency upgrades", "Upgrader", "Mondays at 06:00").next(86400.0 * 2.4, 86400.0 * 7.0).history(vec![D, F, F, D]), 0.0),
                        (ScheduledJob::new("Nightly evals", "Eval Runner", "Daily at 02:00").next(3600.0 * 9.0, 86400.0).history(vec![D, D, F, D, D, D, D, D]), 0.0),
                    ]
                });
                for (k, (j, run_left)) in jobs.iter_mut().enumerate() {
                    if j.running {
                        *run_left -= dt / p[0];
                        if *run_left <= 0.0 {
                            j.running = false;
                            j.history.push(if k == 2 { RunState::Failed } else { RunState::Done });
                            j.next_in = j.period;
                        }
                    } else if j.enabled {
                        j.next_in -= dt;
                        if j.next_in <= 0.0 {
                            j.running = true;
                            *run_left = 3.0;
                        }
                    }
                }
                let mut list: Vec<ScheduledJob> = jobs.iter().map(|j| j.0.clone()).collect();
                let out = ScheduleList::new(&mut list).show(ui);
                for (k, j) in list.into_iter().enumerate() {
                    jobs[k].0.enabled = j.enabled;
                }
                if let Some(i) = out.run_now {
                    jobs[i].0.running = true;
                    jobs[i].1 = 3.0;
                }
                set_mem(ui, "jobs", jobs);
                with_controls(ui, &mut p, 220.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Time warp", &mut p[0], 1.0..=600.0).suffix("x").width(w));
                });
            },
        },
        Demo {
            id: "human-in-the-loop-prompt",
            name: "Human-in-the-Loop Prompt",
            category: C,
            description: "An agent asks a question in its own card: numbered options with one Recommended, a row for your own words, 1-9 keys, and a morph into a one-line receipt with Undo.",
            covers: &["Creance: QuestionCard", "affectUI: Decision Card"],
            code: r##"let mut answer: Option<HitlAnswer> = None;
ui.add(HumanInTheLoopPrompt::new(&mut answer, "Which region should I seed first?")
    .agent("Data Loader", "Seed the staging database")
    .option("us-east-1", "Closest to the API · 2.1 GB of fixtures")
    .option("eu-west-1", "Only the GDPR tenants · 640 MB")
    .recommended(0));"##,
            show: |ui| {
                let mut p = params(ui, [520.0]);
                let now = glowkit_core::time(ui);
                let idle_s = idle(ui);
                let (mut answer, mut at): (Option<HitlAnswer>, f32) = mem(ui, "a", || (None, now));
                if answer.is_none() && idle_s > 2.5 && now - at > 4.0 {
                    answer = Some(HitlAnswer::Choice(0));
                    at = now;
                } else if answer.is_some() && idle_s > 2.5 && now - at > 3.5 {
                    answer = None;
                    at = now;
                }
                let before = answer.clone();
                ui.add(
                    HumanInTheLoopPrompt::new(&mut answer, "Which region should I seed first?")
                        .agent("Data Loader", "Seed the staging database")
                        .option("us-east-1", "Closest to the API · 2.1 GB of fixtures")
                        .option("eu-west-1", "Only the GDPR tenants · 640 MB")
                        .option("Both, in parallel", "Twice the load on the shared replica")
                        .recommended(0)
                        .width(p[0]),
                );
                if answer != before {
                    at = now;
                }
                set_mem(ui, "a", (answer, at));
                with_controls(ui, &mut p, 220.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Width", &mut p[0], 380.0..=640.0).suffix("px").width(w));
                });
            },
        },
        Demo {
            id: "error-recovery-card",
            name: "Error Recovery Card",
            category: C,
            description: "A failed step waiting for you: error code and message, attempt pips, last log lines, the agent's suggested fix, Retry / Skip / Abort and an auto-retry countdown ring.",
            covers: &["ReUI: Failed Run Triage", "Creance: ErrorCard", "affectUI: Fallback Chain"],
            code: r##"let out = ErrorRecoveryCard::new("Capture the partial refund", "402 card_declined")
    .message("The processor declined the partial capture on the original card.")
    .step(3, 6).attempt(2, 3).auto_retry(Some(7.0))
    .suggestion("Retry with a manual amount, or credit the account instead.")
    .show(ui);"##,
            show: |ui| {
                let mut p = params(ui, [1.0, 10.0]);
                let dt = glowkit_core::dt(ui) * p[0];
                glowkit_core::animate(ui);
                let mut c: f32 = mem(ui, "c", || 2.0);
                c += dt;
                let wait = p[1];
                let cycle = wait + 2.5 + 3.0;
                if c > cycle {
                    c = 0.0;
                }
                let retrying = c >= wait && c < wait + 2.5;
                let recovered = c >= wait + 2.5;
                let out = ErrorRecoveryCard::new("Capture the partial refund", "402 card_declined")
                    .message("The processor declined the partial capture on the original card, so the refund could not be completed.")
                    .step(3, 6)
                    .attempt(if recovered || retrying { 3 } else { 2 }, 3)
                    .log("12:38:04  payments.refunds.create amount=1240.00")
                    .log("12:38:10  ERR 402 card_declined (attempt 2 of 3)")
                    .log("12:38:10  backoff 8s before next attempt")
                    .suggestion("Retry with a manual amount, or credit the account instead.")
                    .auto_retry(if !retrying && !recovered { Some(wait - c) } else { None })
                    .retrying(retrying)
                    .recovered(recovered)
                    .show(ui);
                match out {
                    Some(Recovery::Retry) => c = wait,
                    Some(_) => c = wait + 2.5,
                    None => {}
                }
                set_mem(ui, "c", c);
                with_controls(ui, &mut p, 200.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=3.0).decimals(1).suffix("x").width(w));
                    ui.add(ControlSlider::new("Backoff", &mut p[1], 4.0..=20.0).suffix("s").width(w));
                });
            },
        },
        Demo {
            id: "plan-board",
            name: "Plan Board",
            category: C,
            description: "An agent's plan as a four-column board: step cards with agent, tool and live progress lift and glide from Planned to Running to Review to Done.",
            covers: &["affectUI: Agent Plan", "Creance: PlanEditor", "Creative Tim: Multi-step Agent", "ReUI: Playbook Builder"],
            code: r##"let steps = vec![
    PlanCard::new(1, "Read the failing build log", RunState::Done).agent("Deploy Fixer", Icon::Terminal),
    PlanCard::new(2, "Add zod to web dependencies", RunState::Running).agent("Deploy Fixer", Icon::Code).progress(0.4, 2.0),
    PlanCard::new(3, "Approve the production deploy", RunState::Queued).agent("You", Icon::Lock),
];
ui.add(PlanBoard::new("Fix the failing production deploy", &steps));"##,
            show: |ui| {
                let mut p = params(ui, [1.0]);
                let t = sim_clock(ui, p[0]);
                let steps = plan_sim(t + 12.0);
                ui.add(PlanBoard::new("Fix the failing production deploy", &steps));
                with_controls(ui, &mut p, 220.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=3.0).decimals(1).suffix("x").width(w));
                });
            },
        },
        Demo {
            id: "context-window-viz",
            name: "Context Window",
            category: C,
            description: "What fills the model's context: a stacked bar by source with the auto-compact mark and a field of cells; hover a source to light it up, watch compaction squeeze the chat.",
            covers: &["affectUI: Context Window", "affectUI: Context Map"],
            code: r##"let parts = vec![
    ContextPart::new("System prompt", 3_200.0),
    ContextPart::new("Tools", 14_800.0),
    ContextPart::new("Files", 61_000.0),
    ContextPart::new("Conversation", 48_000.0),
];
ui.add(ContextWindowViz::new(&parts, 200_000.0).compact_at(0.9));"##,
            show: |ui| {
                let mut p = params(ui, [1.0, 90.0]);
                let t = (sim_clock(ui, p[0]) + 14.0) % 40.0;
                let grow = (t / 30.0).min(1.0);
                let compacted = t > 30.0 && grow >= p[1] / 100.0 - 0.3;
                let conv = if compacted { 9_000.0 } else { 18_000.0 + 92_000.0 * grow };
                let results = if compacted { 6_000.0 } else { 8_000.0 + 30_000.0 * grow };
                let parts = vec![
                    ContextPart::new("System prompt", 3_200.0),
                    ContextPart::new("Tools", 14_800.0),
                    ContextPart::new("Memory", 5_400.0),
                    ContextPart::new("Files", 24_000.0 + 18_000.0 * grow),
                    ContextPart::new(if compacted { "Conversation (summary)" } else { "Conversation" }, conv),
                    ContextPart::new("Tool results", results),
                ];
                ui.add(ContextWindowViz::new(&parts, 200_000.0).compact_at(p[1] / 100.0));
                with_controls(ui, &mut p, 200.0, |ui, p, w| {
                    ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=3.0).decimals(1).suffix("x").width(w));
                    ui.add(ControlSlider::new("Compact at", &mut p[1], 60.0..=95.0).suffix("%").width(w));
                });
            },
        },
    ]
}
