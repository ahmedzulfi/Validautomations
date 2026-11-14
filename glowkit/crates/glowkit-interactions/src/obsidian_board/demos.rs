use super::*;
use egui::{vec2, Align, Layout, Rect, Sense, Ui, UiBuilder};
use glowkit_core::demo::{Category, Demo};
use glowkit_effects::ControlSlider;

fn mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, init: impl FnOnce() -> T) -> T {
    glowkit_core::state(ui, ui.scope_id().with(key), init)
}

fn set_mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, v: T) {
    glowkit_core::set_state(ui, ui.scope_id().with(key), v);
}

/// Parameter values for a demo, persisted per demo.
fn params<const N: usize>(ui: &Ui, key: &str, init: [f32; N]) -> [f32; N] {
    mem(ui, key, || init)
}

/// Run `f` inside `rect`, centered using last frame's measured size.
fn centered(ui: &mut Ui, rect: Rect, key: &str, f: impl FnOnce(&mut Ui)) {
    let mid = ui.scope_id().with(("centered", key));
    let measured: f32 = glowkit_core::state(ui, mid, || rect.height() * 0.5);
    let top = ((rect.height() - measured) / 2.0).max(0.0);
    let child = Rect::from_min_max(rect.min + vec2(0.0, top), rect.max);
    let h = ui
        .scope_builder(UiBuilder::new().max_rect(child).layout(Layout::top_down(Align::Center)), |ui| {
            f(ui);
            ui.min_rect().height()
        })
        .inner;
    glowkit_core::set_state(ui, mid, h);
}

/// The live component centered in a stage of `height`, with a row of parameter sliders below
/// (`n` sliders, stacked on narrow screens).
fn bench<P>(ui: &mut Ui, height: f32, n: usize, p: &mut P, component: impl FnOnce(&mut Ui, &mut P), controls: impl FnOnce(&mut Ui, &mut P)) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    centered(ui, rect, "stage", |ui| component(ui, p));
    ui.add_space(14.0);
    let total = n as f32 * SW + 14.0 * (n as f32 - 1.0);
    if w >= total + 8.0 {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            ui.add_space(((w - total) / 2.0 - 8.0).max(0.0));
            controls(ui, p);
        });
    } else {
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            controls(ui, p);
        });
    }
}

const SW: f32 = 220.0;

/// A slider of the standard demo width.
fn slider<'a>(label: &str, v: &'a mut f32, r: std::ops::RangeInclusive<f32>) -> ControlSlider<'a> {
    ControlSlider::new(label, v, r).width(SW)
}

pub(super) fn all() -> Vec<Demo> {
    vec![
    Demo {
        id: "board-sankey-card",
        name: "Sankey Chart Card",
        category: Category::Charts,
        description: "A true sankey in a calm dashboard card: pill nodes sized by flow, smooth ribbons as thick as their value, hover to isolate a node or a ribbon.",
        covers: &["BoardUI: Sankey Chart Card"],
        code: r##"ui.add(BoardSankey::new("Tracked time").unit("h").flows(vec![
    BoardFlow::new("Focus", "Writing", 12.0),
    BoardFlow::new("Focus", "Browsing", 8.0),
    BoardFlow::new("Meetings", "Video calls", 10.0),
]));"##,
        show: |ui| {
            let mut p = params(ui, "p", [42.0, 14.0, 660.0]);
            bench(
                ui,
                450.0,
                3,
                &mut p,
                |ui, p| {
                    ui.add(BoardSankey::new("Tracked time").opacity(p[0] / 100.0).node_gap(p[1]).size(vec2(p[2].min(ui.available_width()), 440.0)).autoplay(true));
                },
                |ui, p| {
                    ui.add(slider("Ribbons", &mut p[0], 10.0..=90.0).suffix("%"));
                    ui.add(slider("Node gap", &mut p[1], 0.0..=30.0).suffix("px"));
                    ui.add(slider("Width", &mut p[2], 420.0..=760.0).suffix("px"));
                },
            );
            set_mem(ui, "p", p);
        },
    },
    Demo {
        id: "board-activity-rings-card",
        name: "Activity Rings Card",
        category: Category::Charts,
        description: "Concentric goal rings under three stat tiles: rings sweep in with a stagger, lap past 100%, and hovering a ring or tile focuses it.",
        covers: &["BoardUI: Activity Rings Card"],
        code: r##"ui.add(BoardActivityRings::new("Activity").rings(vec![
    BoardRing::new("Move", "1,592 kcal", 0.86),
    BoardRing::new("Exercise", "1h 45m", 0.74),
    BoardRing::new("Running", "5.2 km", 0.62),
]));"##,
        show: |ui| {
            let mut p = params(ui, "p", [84.0, 76.0, 18.0]);
            bench(
                ui,
                400.0,
                3,
                &mut p,
                |ui, p| {
                    let kcal = p[0] / 100.0 * 600.0;
                    let mins = p[1] / 100.0 * 60.0 * 1.6;
                    ui.add(
                        BoardActivityRings::new("Activity")
                            .rings(vec![
                                BoardRing::new("Move", format!("{} kcal", kcal.round()), p[0] / 100.0),
                                BoardRing::new("Exercise", format!("{}h {:02}m", (mins / 60.0) as u32, (mins % 60.0) as u32), p[1] / 100.0 * 1.6),
                                BoardRing::new("Running", "5.2 km", 0.66),
                            ])
                            .thickness(p[2])
                            .autoplay(true),
                    );
                },
                |ui, p| {
                    ui.add(slider("Move", &mut p[0], 0.0..=180.0).suffix("%"));
                    ui.add(slider("Exercise", &mut p[1], 0.0..=100.0).suffix("%"));
                    ui.add(slider("Thickness", &mut p[2], 8.0..=26.0).suffix("px"));
                },
            );
            set_mem(ui, "p", p);
        },
    },
    Demo {
        id: "board-sleep-score-card",
        name: "Sleep Score Card",
        category: Category::Charts,
        description: "A segmented ring gauge around a count-up score: one arc per sub-score, weeks paged with the date pill, hovering a row thickens its arc.",
        covers: &["BoardUI: Sleep Score Card"],
        code: r##"ui.add(BoardSleepScore::new().weeks(vec![(
    "29 Jun - 5 Jul".into(),
    vec![
        BoardSleepMetric::new("Duration: 7h 50m", 49.0, 50.0),
        BoardSleepMetric::new("Bedtime: 20m earlier", 29.0, 30.0),
        BoardSleepMetric::new("Interruptions: 5m awake", 20.0, 20.0),
    ],
)]));"##,
        show: |ui| {
            let mut p = params(ui, "p", [13.0, 16.0]);
            bench(
                ui,
                360.0,
                2,
                &mut p,
                |ui, p| {
                    ui.add(BoardSleepScore::new().thickness(p[0]).gap(p[1]).autoplay(true));
                },
                |ui, p| {
                    ui.add(slider("Thickness", &mut p[0], 4.0..=24.0).suffix("px"));
                    ui.add(slider("Gap", &mut p[1], 0.0..=40.0).suffix("°"));
                },
            );
            set_mem(ui, "p", p);
        },
    },
    Demo {
        id: "board-most-active-days-card",
        name: "Most Active Days Card",
        category: Category::Charts,
        description: "A scrolling month calendar of mini activity rings with a month switcher; the selected day drives a full rings card beside it.",
        covers: &["BoardUI: Most Active Days Card"],
        code: r##"ui.add(BoardActiveDays::new(vec![
    BoardMonth::generated("June", 0, 30, 30, 3),
    BoardMonth::generated("July", 2, 31, 12, 7),
]));"##,
        show: |ui| {
            let mut p = params(ui, "p", [340.0]);
            bench(
                ui,
                350.0,
                1,
                &mut p,
                |ui, p| {
                    let w = ui.available_width().min(760.0);
                    ui.add(
                        BoardActiveDays::new(vec![BoardMonth::generated("June", 0, 30, 30, 3), BoardMonth::generated("July", 2, 31, 12, 7)])
                            .size(w, p[0])
                            .autoplay(true),
                    );
                },
                |ui, p| {
                    ui.add(slider("Height", &mut p[0], 290.0..=350.0).suffix("px"));
                },
            );
            set_mem(ui, "p", p);
        },
    },
    Demo {
        id: "board-contributions-card",
        name: "Contributions Card",
        category: Category::Charts,
        description: "A year of activity as a 53-week heat grid under a headline and stat tiles; tabs re-aggregate with a diagonal colour ripple and cells show their day on hover.",
        covers: &["BoardUI: Contributions Card"],
        code: r##"let counts: Vec<u32> = (0..365).map(|d| (d * 7 % 11) as u32).collect();
ui.add(BoardContributions::new("Contributions this year").counts(counts, 3).accent(egui::Color32::from_rgb(194, 122, 255)));"##,
        show: |ui| {
            let mut p = params(ui, "p", [0.0, 680.0]);
            bench(
                ui,
                370.0,
                2,
                &mut p,
                |ui, p| {
                    let accents = ["#C27AFF", "#2B7FFF", "#00C951", "#FF8A1F", "#FB64B6", "#00D3BB", "#615FFF", "#F0174B", "#8EDB00"];
                    let a = glowkit_core::hex(accents[(p[0] as usize).min(8)]);
                    ui.add(BoardContributions::new("Contributions this year").accent(a).width(p[1].min(ui.available_width())).autoplay(true));
                },
                |ui, p| {
                    ui.add(slider("Accent", &mut p[0], 0.0..=8.0));
                    ui.add(slider("Width", &mut p[1], 420.0..=760.0).suffix("px"));
                },
            );
            set_mem(ui, "p", p);
        },
    },
    Demo {
        id: "board-earnings-chart-card",
        name: "Earnings Chart Card",
        category: Category::Charts,
        description: "Rounded pill bars over a money axis with a Weekly / Monthly / Yearly switch that regrows the bars, a count-up headline and an outline on the hovered bar.",
        covers: &["BoardUI: Earnings Chart Card"],
        code: r##"ui.add(BoardEarnings::new("Earned so far").series(vec![
    BoardSeries::new("Monthly", &["Jan", "Feb", "Mar"], vec![2400.0, 6100.0, 8300.0]).delta("+14.8%"),
]));"##,
        show: |ui| {
            let mut p = params(ui, "p", [560.0, 330.0]);
            bench(
                ui,
                340.0,
                2,
                &mut p,
                |ui, p| {
                    ui.add(BoardEarnings::new("Earned so far").size(vec2(p[0].min(ui.available_width()), p[1])).autoplay(true));
                },
                |ui, p| {
                    ui.add(slider("Width", &mut p[0], 360.0..=760.0).suffix("px"));
                    ui.add(slider("Height", &mut p[1], 260.0..=340.0).suffix("px"));
                },
            );
            set_mem(ui, "p", p);
        },
    },
    Demo {
        id: "board-steps-card",
        name: "Steps Card",
        category: Category::Charts,
        description: "Seven teal pill bars under a count-up step total; the date pill pages through weeks and the hovered day gets an outline while the headline follows.",
        covers: &["BoardUI: Steps Card"],
        code: r##"ui.add(BoardSteps::new().weeks(vec![
    ("29 Jun - 5 Jul".into(), [6200.0, 2700.0, 2400.0, 7900.0, 8800.0, 6500.0, 4100.0]),
]));"##,
        show: |ui| {
            let mut p = params(ui, "p", [10000.0, 540.0]);
            bench(
                ui,
                330.0,
                2,
                &mut p,
                |ui, p| {
                    ui.add(BoardSteps::new().goal(Some(p[0])).size(vec2(p[1].min(ui.available_width()), 320.0)).autoplay(true));
                },
                |ui, p| {
                    ui.add(slider("Goal", &mut p[0], 8000.0..=16000.0).step(500.0));
                    ui.add(slider("Width", &mut p[1], 340.0..=720.0).suffix("px"));
                },
            );
            set_mem(ui, "p", p);
        },
    },
    Demo {
        id: "board-orders-chart-card",
        name: "Orders Chart Card",
        category: Category::Charts,
        description: "Monthly orders as this-year bars beside last-year bars; the headline counts up with a delta chip and hovering a month swaps in that month's figures.",
        covers: &["BoardUI: Orders Chart Card"],
        code: r##"let this = [1700.0, 1800.0, 2000.0, 1900.0, 2100.0, 2200.0, 2400.0, 2300.0, 2400.0, 2500.0, 2400.0, 2450.0];
let last = [1500.0, 1500.0, 1700.0, 1650.0, 1750.0, 1800.0, 1850.0, 1900.0, 1950.0, 2000.0, 1850.0, 1850.0];
ui.add(BoardOrders::new("Orders").data(this, last));"##,
        show: |ui| {
            let mut p = params(ui, "p", [20.0, 600.0]);
            bench(
                ui,
                350.0,
                2,
                &mut p,
                |ui, p| {
                    let g = 1.0 + p[0] / 100.0;
                    let last = [1540.0, 1510.0, 1690.0, 1640.0, 1750.0, 1810.0, 1870.0, 1930.0, 1960.0, 2020.0, 1850.0, 1838.0];
                    let this = std::array::from_fn(|i| last[i] * g * (0.94 + 0.1 * ((i as f32) * 1.7).sin().abs()));
                    ui.add(BoardOrders::new("Orders").data(this, last).size(vec2(p[1].min(ui.available_width()), 340.0)).autoplay(true));
                },
                |ui, p| {
                    ui.add(slider("Growth", &mut p[0], -20.0..=40.0).suffix("%"));
                    ui.add(slider("Width", &mut p[1], 380.0..=760.0).suffix("px"));
                },
            );
            set_mem(ui, "p", p);
        },
    },
    Demo {
        id: "board-stage-bars-card",
        name: "Stage Bars Card",
        category: Category::Charts,
        description: "A funnel as a list of rounded pills with value and share of the top stage, plus stat tiles; widths grow in and hovering a stage deepens it and fades the rest.",
        covers: &["BoardUI: Stage Bars Card"],
        code: r##"ui.add(BoardStageBars::new("Pipeline").stages(vec![
    BoardStage::new("Visits", 1180.0),
    BoardStage::new("Sign-up", 790.0),
    BoardStage::new("Active", 460.0),
]));"##,
        show: |ui| {
            let mut p = params(ui, "p", [67.0, 20.0, 450.0]);
            bench(
                ui,
                420.0,
                3,
                &mut p,
                |ui, p| {
                    let conv = p[0] / 100.0;
                    let names = ["Visits", "Sign-up", "Active", "Pro", "Team", "Enterprise"];
                    let mut v = 1180.0;
                    let stages = names
                        .iter()
                        .enumerate()
                        .map(|(i, n)| {
                            if i > 0 {
                                v *= conv * (0.95 - 0.06 * i as f32).max(0.3) + 0.1;
                            }
                            BoardStage::new(*n, v.round())
                        })
                        .collect();
                    ui.add(BoardStageBars::new("Pipeline").stages(stages).bar_height(p[1]).width(p[2]).autoplay(true));
                },
                |ui, p| {
                    ui.add(slider("Conversion", &mut p[0], 30.0..=95.0).suffix("%"));
                    ui.add(slider("Bar height", &mut p[1], 12.0..=28.0).suffix("px"));
                    ui.add(slider("Width", &mut p[2], 360.0..=560.0).suffix("px"));
                },
            );
            set_mem(ui, "p", p);
        },
    },
    Demo {
        id: "board-meter",
        name: "Meter",
        category: Category::DataDisplay,
        description: "Quota meters that fill with a spring and shift from blue to amber to red as the limit nears, with a breathing glow when critical.",
        covers: &["BoardUI: Meter"],
        code: r##"ui.add(BoardMeter::new("Storage", 7.2, 10.0).unit("GB"));
ui.add(BoardMeter::new("Seats", 19.0, 20.0).integer().hint("Add seats before inviting more teammates."));
ui.add(BoardMeter::new("API requests this month", 96.0, 100.0).percent());"##,
        show: |ui| {
            let mut p = params(ui, "p", [72.0, 19.0, 96.0]);
            bench(
                ui,
                250.0,
                3,
                &mut p,
                |ui, p| {
                    ui.spacing_mut().item_spacing.y = 14.0;
                    ui.add(BoardMeter::new("Storage", p[0] / 10.0, 10.0).unit("GB"));
                    ui.add(BoardMeter::new("Seats", p[1], 20.0).integer().hint("Add seats before inviting more teammates."));
                    ui.add(BoardMeter::new("API requests this month", p[2], 100.0).percent());
                },
                |ui, p| {
                    ui.add(slider("Storage", &mut p[0], 0.0..=100.0).suffix("%"));
                    ui.add(slider("Seats", &mut p[1], 0.0..=20.0));
                    ui.add(slider("API", &mut p[2], 0.0..=100.0).suffix("%"));
                },
            );
            set_mem(ui, "p", p);
        },
    },
    Demo {
        id: "board-agent-limits-card",
        name: "Agent Limits Card",
        category: Category::Ai,
        description: "A context-window bar segmented by token bucket plus plan limits with reset times; clicking the header grows the card to reveal the per-bucket breakdown.",
        covers: &["BoardUI: Agent Limits Card"],
        code: r##"ui.add(BoardAgentLimits::new(1_000_000.0)
    .buckets(vec![BoardBucket::new("Messages", 520_000.0), BoardBucket::new("Tools", 180_000.0)])
    .limits(vec![BoardLimit::new("5-hour limit", "Resets in 2 hr 46 min", 0.38)]));"##,
        show: |ui| {
            let mut p = params(ui, "p", [38.0, 100.0]);
            bench(
                ui,
                420.0,
                2,
                &mut p,
                |ui, p| {
                    let k = p[1] / 100.0;
                    let buckets = [("Messages", 516.0), ("System tools", 96.0), ("Memory files", 68.0), ("Skills", 52.0), ("MCP tools", 38.0), ("Custom agents", 22.0), ("System prompt", 14.0)]
                        .iter()
                        .map(|(n, v)| BoardBucket::new(*n, v * 1000.0 * if *n == "Messages" { k } else { 1.0 }))
                        .collect();
                    ui.add(
                        BoardAgentLimits::new(1_000_000.0)
                            .buckets(buckets)
                            .limits(vec![
                                BoardLimit::new("5-hour limit", "Resets in 2 hr 46 min", p[0] / 100.0),
                                BoardLimit::new("Weekly · all models", "Resets Tue 3:00 PM", 0.03),
                                BoardLimit::new("Weekly · Pro", "Resets Tue 3:00 PM", 0.05),
                            ])
                            .autoplay(true),
                    );
                },
                |ui, p| {
                    ui.add(slider("5-hour", &mut p[0], 0.0..=100.0).suffix("%"));
                    ui.add(slider("Messages", &mut p[1], 0.0..=150.0).suffix("%"));
                },
            );
            set_mem(ui, "p", p);
        },
    },
    Demo {
        id: "board-agent-progress",
        name: "Agent Progress",
        category: Category::Ai,
        description: "A collapsible multi-step progress block: steps stagger in, the active one spins a ring synced to its progress, done steps check off with a strikethrough.",
        covers: &["BoardUI: Agent Progress"],
        code: r##"let steps = ["Read project files", "Update tokens", "Run the build"];
ui.add(BoardAgentProgress::new(&steps).progress(1.4));"##,
        show: |ui| {
            let mut p = params(ui, "p", [1.0]);
            bench(
                ui,
                300.0,
                1,
                &mut p,
                |ui, p| {
                    let steps = ["Read project files", "Update light mode tokens", "Implement dark mode tokens", "Register the theme toggle", "Run lint and production build"];
                    let cycle = 5.0 / p[0].max(0.2) + 3.0;
                    let tt = (glowkit_core::time(ui) % cycle) * p[0].max(0.2);
                    let prog = (tt - 0.3).clamp(0.0, 5.0);
                    glowkit_core::animate(ui);
                    ui.add(BoardAgentProgress::new(&steps).progress(prog).width(380.0));
                },
                |ui, p| {
                    ui.add(slider("Speed", &mut p[0], 0.3..=3.0).decimals(1).suffix("x"));
                },
            );
            set_mem(ui, "p", p);
        },
    },
    Demo {
        id: "board-agent-thinking",
        name: "Agent Thinking",
        category: Category::Ai,
        description: "Four thinking indicators (dot wave, dot spin, stars, infinity comet), each with a shimmering label and an elapsed timer.",
        covers: &["BoardUI: Agent Thinking"],
        code: r##"ui.add(BoardAgentThinking::new("Thinking").style(BoardThinkingStyle::Infinity));"##,
        show: |ui| {
            let mut p = params(ui, "p", [1.0]);
            bench(
                ui,
                260.0,
                1,
                &mut p,
                |ui, p| {
                    ui.spacing_mut().item_spacing.y = 18.0;
                    let labels = ["Thinking", "Reasoning", "Exploring", "Planning"];
                    for (i, s) in BoardThinkingStyle::ALL.iter().enumerate() {
                        let secs = (glowkit_core::time(ui) * 0.9 + i as f32 * 2.3) % 14.0;
                        ui.add(BoardAgentThinking::new(labels[i]).style(*s).speed(p[0]).elapsed(secs));
                    }
                },
                |ui, p| {
                    ui.add(slider("Speed", &mut p[0], 0.3..=3.0).decimals(1).suffix("x"));
                },
            );
            set_mem(ui, "p", p);
        },
    },
    Demo {
        id: "board-composer-loader",
        name: "Composer Loader",
        category: Category::Ai,
        description: "A chat composer whose rim carries an iridescent band at constant speed, led by a crisp line with a soft bloom bleeding inward; press send to toggle.",
        covers: &["BoardUI: Composer Loader"],
        code: r##"let mut loading = true;
ui.add(BoardComposerLoader::new(&mut loading).placeholder("Ask me anything"));"##,
        show: |ui| {
            let mut p = params(ui, "p", [260.0, 50.0, 0.0]);
            let mut loading = mem(ui, "loading", || true);
            bench(
                ui,
                220.0,
                3,
                &mut p,
                |ui, p| {
                    let sets: [&[&str]; 6] = [
                        &["#5EEAD4", "#60A5FA", "#A78BFA", "#F472B6", "#FBBF24"],
                        &["#93C5FD", "#3B82F6", "#1D4ED8"],
                        &["#C4B5FD", "#8B5CF6", "#6D28D9"],
                        &["#F9A8D4", "#EC4899", "#BE185D"],
                        &["#86EFAC", "#22C55E", "#15803D"],
                        &["#FDBA74", "#F97316", "#C2410C"],
                    ];
                    let cols = sets[(p[2] as usize).min(5)].iter().map(|h| glowkit_core::hex(h)).collect();
                    ui.add(BoardComposerLoader::new(&mut loading).speed(p[0]).tail(p[1] / 100.0).colors(cols).width(ui.available_width().min(560.0)));
                },
                |ui, p| {
                    ui.add(slider("Speed", &mut p[0], 60.0..=900.0).suffix("pt/s"));
                    ui.add(slider("Band", &mut p[1], 10.0..=90.0).suffix("%"));
                    ui.add(slider("Palette", &mut p[2], 0.0..=5.0));
                },
            );
            set_mem(ui, "loading", loading);
            set_mem(ui, "p", p);
        },
    },
    Demo {
        id: "board-web-search",
        name: "Web Search",
        category: Category::Ai,
        description: "An agent's research trail: queries and opened pages hung off a curved guide, with result counts and overlapping site marks, streaming in under a shimmering Working timer.",
        covers: &["BoardUI: Web Search"],
        code: r##"ui.add(BoardWebSearch::new(vec![
    BoardSearchStep::query("component library pricing", 5).sites(&["reddit", "github"]),
    BoardSearchStep::page("Pricing · Example", "example.com"),
]).progress(1.5));"##,
        show: |ui| {
            let mut p = params(ui, "p", [1.0]);
            bench(
                ui,
                260.0,
                1,
                &mut p,
                |ui, p| {
                    let steps = vec![
                        BoardSearchStep::query("what teams actually pay for in a component library", 5).sites(&["reddit", "news", "github"]),
                        BoardSearchStep::query("dashboard ui kit pricing tiers 2026", 8).sites(&["medium", "dev"]),
                        BoardSearchStep::page("Why we stopped building our own charts", "blog"),
                        BoardSearchStep::query("design system adoption survey results", 4).sites(&["stack", "github", "figma", "x"]),
                        BoardSearchStep::page("State of frontend tooling report", "survey"),
                    ];
                    let n = steps.len() as f32;
                    let speed = p[0].max(0.2);
                    let cycle = (n + 1.0) * 1.2 / speed + 2.5;
                    let tt = glowkit_core::time(ui) % cycle;
                    glowkit_core::animate(ui);
                    let prog = (tt * speed / 1.2).min(n + 1.0);
                    ui.add(BoardWebSearch::new(steps).progress(prog).elapsed(tt * 1.7).width(ui.available_width().min(600.0)));
                },
                |ui, p| {
                    ui.add(slider("Speed", &mut p[0], 0.3..=3.0).decimals(1).suffix("x"));
                },
            );
            set_mem(ui, "p", p);
        },
    },
    Demo {
        id: "board-questionnaire",
        name: "Questionnaire",
        category: Category::Ai,
        description: "Questions an agent asks before committing to a plan: bordered answer rows with checkboxes or number keys, an Other field, step pills, and slides between questions.",
        covers: &["BoardUI: Questionnaire"],
        code: r##"let qs = vec![BoardQuestion::new("Which layout?", &[("Grid", "Cards in columns"), ("List", "One per row")])];
let mut answers = BoardQuestionnaireState::default();
ui.add(BoardQuestionnaire::new(&qs, &mut answers));"##,
        show: |ui| {
            let qs = vec![
                BoardQuestion::new(
                    "Which controls should stay in the top bar?",
                    &[
                        ("Theme switcher", "Light, dark and system appearance."),
                        ("Workspace picker", "Jump between projects without leaving the page."),
                        ("Notifications", "The bell with unread activity."),
                    ],
                )
                .multi(true),
                BoardQuestion::new(
                    "How should the dashboard load first?",
                    &[("Last visited view", "Restore where you left off."), ("Overview", "Always start from the summary cards.")],
                ),
                BoardQuestion::new(
                    "Which chart style fits the brand?",
                    &[("Rounded pills", "Soft tracks with filled bars."), ("Thin lines", "Sharp trend lines and gridlines."), ("Mixed", "Pills for totals, lines for trends.")],
                )
                .other(false),
                BoardQuestion::new("Ship it behind a flag?", &[("Yes", "Roll out to 10% first."), ("No", "Release to everyone.")]),
            ];
            let mut st: BoardQuestionnaireState = mem(ui, "st", BoardQuestionnaireState::default);
            let mut p = params(ui, "p", [620.0]);
            bench(
                ui,
                380.0,
                1,
                &mut p,
                |ui, p| {
                    ui.add(BoardQuestionnaire::new(&qs, &mut st).width(p[0].min(ui.available_width())).autoplay(true));
                },
                |ui, p| {
                    ui.add(slider("Width", &mut p[0], 380.0..=720.0).suffix("px"));
                },
            );
            set_mem(ui, "st", st);
            set_mem(ui, "p", p);
        },
    },
    Demo {
        id: "board-notification-center",
        name: "Notification Center",
        category: Category::Overlays,
        description: "An activity inbox with sliding filter tabs and counts, grouped timeline cards with avatars or badges, unread dots, inline actions, mark-all-read and an empty state.",
        covers: &["BoardUI: Notification Center"],
        code: r##"let mut inbox = vec![BoardNotice::mention("LK", "Livia mentioned you", "Can you review this?", "2m").actions(&["Reply"])];
ui.add(BoardNotifications::new(&mut inbox));"##,
        show: |ui| {
            let mut items: Vec<BoardNotice> = mem(ui, "items", || {
                vec![
                    BoardNotice::mention("LK", "Lena mentioned you", "Can you check the new empty state before we ship the dashboard?", "2m").actions(&["Reply", "View thread"]),
                    BoardNotice::system(BoardNoticeIcon::Download, "Workspace backup is ready", "The July 23 backup finished and is ready to download.", "18m").actions(&["Download"]),
                    BoardNotice::system(BoardNoticeIcon::Join, "You joined Project Sea", "Maya added you as an editor. You now have access to all project files.", "1h"),
                    BoardNotice::mention("JD", "Jonah requested your review", "Pull request #284 updates the notification preferences flow.", "Mon").actions(&["Review changes"]).group("Earlier").unread(false),
                    BoardNotice::system(BoardNoticeIcon::Settings, "Billing details updated", "Your invoices will now be sent to finance@studio.dev.", "Sun").group("Earlier"),
                    BoardNotice::mention("AR", "Ari replied in #design", "Love the new chart cards, shipping them this week.", "Sat").group("Earlier").unread(false),
                ]
            });
            let mut p = params(ui, "p", [520.0]);
            // reset the demo inbox a while after everything was read
            let all_read = items.iter().all(|n| !n.unread);
            let since: f32 = mem(ui, "read_at", || f32::MAX);
            let now = glowkit_core::time(ui);
            if all_read && since == f32::MAX {
                set_mem(ui, "read_at", now);
            } else if all_read && now - since > 4.0 {
                for (i, n) in items.iter_mut().enumerate() {
                    n.unread = i < 3 || i == 4;
                }
                set_mem(ui, "read_at", f32::MAX);
            }
            bench(
                ui,
                540.0,
                1,
                &mut p,
                |ui, p| {
                    ui.add(BoardNotifications::new(&mut items).size(vec2(430.0, p[0])).autoplay(true));
                },
                |ui, p| {
                    ui.add(slider("Height", &mut p[0], 360.0..=540.0).suffix("px"));
                },
            );
            set_mem(ui, "items", items);
            set_mem(ui, "p", p);
        },
    },
    Demo {
        id: "board-ai-profile",
        name: "AI Profile",
        category: Category::Dashboards,
        description: "A contributions profile for AI work: cover and identity card, headline and stat tiles, an activity heat map, a 30-day agents bar chart and a tokens trend line.",
        covers: &["BoardUI: AI Profile"],
        code: r##"ui.add(BoardAiProfile::new("Mira Okafor", "mira"));"##,
        show: |ui| {
            let mut p = params(ui, "p", [1.0, 0.0]);
            bench(
                ui,
                600.0,
                2,
                &mut p,
                |ui, p| {
                    let accents = ["#C27AFF", "#2B7FFF", "#00C951", "#FF8A1F", "#FB64B6", "#615FFF"];
                    let w = ui.available_width().min(920.0);
                    ui.add(
                        BoardAiProfile::new("Mira Okafor", "mira")
                            .scene(p[0] as usize)
                            .accent(glowkit_core::hex(accents[(p[1] as usize).min(5)]))
                            .width(w)
                            .autoplay(true),
                    );
                },
                |ui, p| {
                    ui.add(slider("Cover", &mut p[0], 0.0..=7.0));
                    ui.add(slider("Accent", &mut p[1], 0.0..=5.0));
                },
            );
            set_mem(ui, "p", p);
        },
    },
    ]
}
