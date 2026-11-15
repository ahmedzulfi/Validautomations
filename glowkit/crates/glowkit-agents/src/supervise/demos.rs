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

fn smooth(x: f32) -> f32 {
    glowkit_core::anim::smoothstep(0.0, 1.0, x)
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
    let mut writer = node("Writer", "Sonnet 4.5", "Draft copy for the three plans", 6.0, 24.0, 0.004);
    if (14.0..18.0).contains(&t) {
        writer.state = RunState::NeedsInput;
        writer.task = "Asks: keep the annual discount at 20%?".into();
    }
    let mut root = node("Orchestrator", "Opus 4.1", "Launch the new pricing page by Friday", 0.0, 28.0, 0.012);
    if t >= 28.0 {
        root.task = "All sub-agents reported back. Page is live.".into();
    }
    root.child(
        node("Researcher", "Sonnet 4.5", "Compare pricing of five competitors", 0.5, 13.0, 0.005)
            .child(node("Scraper", "Haiku 4.5", "Read pricing pages of Linear, Notion and three more", 1.0, 9.0, 0.0015))
            .child(node("Analyst", "Sonnet 4.5", "Tabulate seats, tiers and annual discounts", 9.0, 12.5, 0.003)),
    )
    .child(writer)
    .child(
        node("Coder", "GPT-5", "Build the page and wire up checkout", 3.0, 27.0, 0.006)
            .child(node("Tester", "Haiku 4.5", "Run the checkout e2e suite", 18.0, 26.0, 0.002)),
    )
}

pub(super) fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "agent-tree",
            name: "Agent Tree",
            category: C,
            description: "An orchestrator and its sub-agents as a live tree of cards: status, model, task, cost and time on each, lights running along connectors to working agents, branches that fold.",
            covers: &["affectUI: Agent Tree", "affectUI: Agent Crew"],
            code: r##"let root = AgentNode::new("Orchestrator", "Opus 4.1").state(RunState::Running)
    .task("Launch the new pricing page")
    .child(AgentNode::new("Researcher", "Sonnet 4.5").state(RunState::Running).cost(0.06)
        .child(AgentNode::new("Scraper", "Haiku 4.5").state(RunState::Done)))
    .child(AgentNode::new("Writer", "Sonnet 4.5").state(RunState::NeedsInput));
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
            code: r##"ui.add(HandoffCard::new(("Researcher", "Sonnet 4.5"), ("Writer", "Haiku 4.5"))
    .reason("Compared 5 competitors; Notion and Linear are closest to us")
    .item(HandoffItem::new("competitors.md", 2400.0))
    .item(HandoffItem::new("raw page HTML", 18200.0).dropped())
    .progress(progress));"##,
            show: |ui| {
                let mut p = params(ui, [1.0]);
                let t = sim_clock(ui, p[0]);
                let prog = ((t % 7.0) / 3.6).min(1.0);
                ui.add(
                    HandoffCard::new(("Researcher", "Sonnet 4.5"), ("Writer", "Haiku 4.5"))
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
    ]
}

#[allow(dead_code)]
fn _unused(x: f32) -> f32 {
    smooth(x)
}
