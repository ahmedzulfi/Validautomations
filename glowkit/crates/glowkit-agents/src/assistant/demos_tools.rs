use super::common::*;
use super::demos::{column, ghost, mem, set_mem};
use super::*;
use egui::{pos2, vec2, Align2, CornerRadius, FontId, Rect, Sense, Ui};
use glowkit_ai::Icon;
use glowkit_charts::Series;
use glowkit_core::color::with_alpha;
use glowkit_core::demo::{Category, Demo};
use glowkit_core::Theme;
use glowkit_effects::ControlSlider;

const C: Category = Category::Ai;

#[derive(Clone, Copy)]
struct Loop {
    cycle: u64,
    start: f64,
}

/// Loop clock for a tool scene: restarts every `period` seconds unless the user touched the
/// preview recently. Returns (cycle, seconds into the cycle).
fn scene_clock(ui: &Ui, key: &str, period: f32) -> (u64, f32) {
    let now = ui.input(|i| i.time);
    let area: Rect = mem(ui, &format!("{key}_area"), || Rect::NOTHING);
    let idle = idle_for(ui, ui.scope_id().with((key, "idle")), user_active(ui, area));
    let mut l: Loop = mem(ui, key, || Loop { cycle: 0, start: now });
    let e = (now - l.start) as f32;
    if e > period && idle > 6.0 {
        l = Loop { cycle: l.cycle + 1, start: now };
        set_mem(ui, key, l);
    }
    glowkit_core::animate(ui);
    (l.cycle, (now - l.start) as f32)
}

/// A user prompt, the assistant's tool-call status line, then the tool's UI (`body`) once the
/// call "returns" at 1.5 s.
fn agent_scene(ui: &mut Ui, key: &str, width: f32, prompt: &str, tool: &str, e: f32, body: impl FnOnce(&mut Ui)) {
    let t = Theme::of(ui);
    column(ui, width, |ui| {
        let top = ui.cursor().min;
        let w = ui.available_width();
        // user bubble
        let a = ((e - 0.05) * 5.0).clamp(0.0, 1.0);
        let g = ui.painter().layout(prompt.to_owned(), FontId::proportional(13.5), t.foreground, w * 0.7);
        let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y + 18.0), Sense::hover());
        let b = Rect::from_min_size(pos2(r.max.x - g.size().x - 28.0, r.min.y + (1.0 - a) * 8.0), g.size() + vec2(28.0, 18.0));
        let p = ui.painter();
        p.rect_filled(b, CornerRadius { nw: 16, ne: 16, sw: 16, se: 5 }, with_alpha(well(&t), a));
        p.galley(b.min + vec2(14.0, 9.0), g, with_alpha(t.foreground, a));
        ui.add_space(4.0);
        // assistant status
        let (r, _) = ui.allocate_exact_size(vec2(w, 28.0), Sense::hover());
        let k = ((e - 0.45) * 5.0).clamp(0.0, 1.0);
        if k > 0.0 {
            let p = ui.painter();
            bot_avatar(p, &t, pos2(r.min.x + 13.0, r.center().y), 13.0);
            let x = r.min.x + 36.0;
            let time = glowkit_core::time(ui);
            if e < 1.5 {
                spinner(p, pos2(x + 6.0, r.center().y), 5.5, time, t.muted_foreground);
                let s = format!("Calling {tool}…");
                text(p, pos2(x + 18.0, r.center().y), Align2::LEFT_CENTER, &s, 12.5, with_alpha(t.muted_foreground, k));
            } else {
                check_disc(p, pos2(x + 6.0, r.center().y), 6.5, t.success, egui::Color32::WHITE, ((e - 1.5) * 3.0).clamp(0.0, 1.0));
                let s = format!("Used {tool}");
                let tr = text(p, pos2(x + 18.0, r.center().y), Align2::LEFT_CENTER, &s, 12.5, t.muted_foreground);
                mono(p, pos2(tr.max.x + 8.0, r.center().y), Align2::LEFT_CENTER, "1.1s", 10.5, with_alpha(t.muted_foreground, 0.7));
            }
        }
        if e > 1.5 {
            ui.horizontal_top(|ui| {
                ui.add_space(36.0);
                ui.vertical(|ui| {
                    ui.set_width(w - 36.0);
                    body(ui);
                });
            });
        } else {
            ui.add_space(200.0);
        }
        let area = Rect::from_min_max(top, pos2(top.x + w, ui.cursor().min.y));
        set_mem(ui, &format!("{key}_area"), area);
    });
}

fn sliders(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.add_space(14.0);
    column(ui, 560.0, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            add(ui);
        });
    });
}

// ---------------------------------------------------------------------------------------------

fn demo_table(ui: &mut Ui) {
    let (cycle, e) = scene_clock(ui, "tbl", 11.0);
    let mut rows_n: f32 = mem(ui, "rows", || 6.0);
    let cols = [("Region", ColumnKind::Text), ("Owner", ColumnKind::Text), ("Revenue", ColumnKind::Number), ("QoQ", ColumnKind::Delta)];
    let rows: Vec<Vec<String>> = [
        ("EMEA", "Ines", "$4.82M", "+21.4%"),
        ("North America", "Tomas", "$9.10M", "+8.2%"),
        ("LATAM", "Juno", "$1.36M", "+31.0%"),
        ("APAC", "Wren", "$3.07M", "-2.6%"),
        ("Nordics", "Ola", "$0.94M", "+12.9%"),
        ("DACH", "Lena", "$2.21M", "+4.1%"),
        ("UK & Ireland", "Sam", "$2.68M", "-0.8%"),
        ("India", "Ravi", "$0.71M", "+44.3%"),
    ]
    .iter()
    .map(|(a, b, c, d)| vec![a.to_string(), b.to_string(), c.to_string(), d.to_string()])
    .collect();
    agent_scene(ui, "tbl", 600.0, "Which regions grew fastest last quarter?", "warehouse.query", e, |ui| {
        ui.add(
            DataTableResult::new("warehouse.query", &cols, &rows)
                .subtitle("SELECT region, revenue, qoq FROM sales_q3")
                .max_rows(rows_n as usize)
                .id_salt(("tbl", cycle)),
        );
    });
    sliders(ui, |ui| {
        ui.add(ControlSlider::new("Visible rows", &mut rows_n, 3.0..=8.0).width(240.0));
    });
    set_mem(ui, "rows", rows_n);
}

fn demo_chart(ui: &mut Ui) {
    let (cycle, e) = scene_clock(ui, "chart", 10.0);
    let mut kind: f32 = mem(ui, "kind", || 0.0);
    let series = [
        Series::new("Signups", vec![320.0, 410.0, 380.0, 520.0, 610.0, 580.0, 730.0, 860.0]),
        Series::new("Activated", vec![180.0, 240.0, 230.0, 300.0, 390.0, 370.0, 470.0, 560.0]),
    ];
    let labels = ["W1", "W2", "W3", "W4", "W5", "W6", "W7", "W8"];
    let k = match kind as i32 {
        0 => ChartKind::Area,
        1 => ChartKind::Line,
        _ => ChartKind::Bar,
    };
    agent_scene(ui, "chart", 600.0, "Plot weekly signups vs. activations", "analytics.timeseries", e, |ui| {
        ui.add(
            ChartResult::new("analytics.timeseries", &series)
                .labels(&labels)
                .headline("860", "+18.6%")
                .caption("Signups this week, vs. last week")
                .kind(k)
                .chart_height(150.0)
                .id_salt(("chart", cycle, kind as i32)),
        );
    });
    sliders(ui, |ui| {
        ui.add(ControlSlider::new("Chart (area / line / bar)", &mut kind, 0.0..=2.0).width(300.0));
    });
    set_mem(ui, "kind", kind);
}

fn demo_link(ui: &mut Ui) {
    let (cycle, e) = scene_clock(ui, "link", 10.0);
    agent_scene(ui, "link", 640.0, "Find me a good read on calm interface design", "web.fetch", e, |ui| {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            let w = (ui.available_width() - 14.0) / 2.0;
            ui.add(
                LinkPreviewResult::new("fieldnotes.dev", "Designing calm agent interfaces", "Why quiet, predictable UI earns more trust than flashy motion when an agent acts for you.")
                    .scene(6)
                    .width(w)
                    .id_salt(("l1", cycle)),
            );
            if e > 1.8 {
                ui.add(
                    LinkPreviewResult::new("studio.press", "The art of the empty state", "First screens that teach by example: three suggestions beat a blank prompt every time.")
                        .scene(7)
                        .width(w)
                        .id_salt(("l2", cycle)),
                );
            }
        });
    });
}

fn demo_gallery(ui: &mut Ui) {
    let (cycle, e) = scene_clock(ui, "gal", 11.0);
    let caps = ["Alpenglow ridge", "Turquoise lagoon", "Aurora over the fjord", "Dune shadows", "Misty evergreens", "Neon alley", "Glacier lake", "Lavender rows"];
    let open = if (5.0..8.5).contains(&e) { Some(2) } else { None };
    let tile_pos: Rect = mem(ui, "gal_rect", || Rect::NOTHING);
    agent_scene(ui, "gal", 560.0, "Show me moody landscape references", "images.search", e, |ui| {
        let top = ui.cursor().min;
        ui.add(ImageGalleryResult::new("images.search", &caps).open(open).id_salt(("gal", cycle)));
        set_mem(ui, "gal_rect", Rect::from_min_size(top, vec2(ui.available_width(), 300.0)));
    });
    // ghost cursor clicks the third tile, then the close button
    if tile_pos.is_positive() {
        let w = tile_pos.width() - 28.0;
        let big_w = w * 0.58;
        let tile = pos2(tile_pos.min.x + 14.0 + big_w + 6.0 + (w - big_w) * 0.75, tile_pos.min.y + 54.0 + w * 0.14);
        let close = pos2(tile_pos.min.x + w - 8.0, tile_pos.min.y + 62.0);
        let (target, vis, press) = if e > 3.6 && e < 5.2 {
            (tile, true, e > 4.8)
        } else if (5.2..8.7).contains(&e) {
            (close, true, e > 8.3)
        } else {
            (tile, false, false)
        };
        ghost(ui, "gal", target, vis, press);
    }
}

fn demo_weather(ui: &mut Ui) {
    let (cycle, e) = scene_clock(ui, "wx", 10.0);
    let mut sky_f: f32 = mem(ui, "sky", || 0.0);
    let mut temp: f32 = mem(ui, "temp", || 24.0);
    let sky = match sky_f as i32 {
        0 => Sky::Sunny,
        1 => Sky::PartlyCloudy,
        2 => Sky::Rain,
        _ => Sky::Night,
    };
    let hourly = [temp - 5.0, temp - 3.0, temp - 1.0, temp, temp + 1.0, temp - 1.0, temp - 3.0];
    agent_scene(ui, "wx", 560.0, "What's the weather like in Lisbon today?", "weather.now", e, |ui| {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            ui.add(WeatherResult::new("Lisbon, PT", temp, sky).hourly(&hourly).range(temp - 6.0, temp + 1.0).width(330.0).id_salt(("wx", cycle, sky_f as i32)));
            ui.vertical(|ui| {
                ui.add_space(6.0);
                let t = Theme::of(ui);
                let tk = ((e - 2.3) * 3.0).clamp(0.0, 1.0);
                ui.multiply_opacity(tk);
                ui.set_width(150.0);
                ui.label(egui::RichText::new("Bright and mild. A light jacket for the evening, and sunglasses until sunset.").size(13.0).color(t.foreground));
            });
        });
    });
    sliders(ui, |ui| {
        ui.add(ControlSlider::new("Sky", &mut sky_f, 0.0..=3.0).width(200.0));
        ui.add(ControlSlider::new("Temperature", &mut temp, -5.0..=38.0).suffix("°").width(240.0));
    });
    set_mem(ui, "sky", sky_f);
    set_mem(ui, "temp", temp);
}

fn demo_form(ui: &mut Ui) {
    let (cycle, e) = scene_clock(ui, "form", 12.0);
    let fields = [
        FormField::text("Name on the booking", "Ada Lovelace"),
        FormField::choice("Party size", &["2", "4", "6", "8+"]),
        FormField::text("Time", "19:30"),
        FormField::toggle("Window table if available"),
    ];
    let mut st: (u64, FormState) = mem(ui, "fs", || (0, FormState::default()));
    if st.0 != cycle {
        st = (cycle, FormState::default());
    }
    let mut focus = None;
    // autoplay: type, pick, toggle, submit
    let auto = e < 11.0;
    if auto && !st.1.submitted {
        st.1.values.resize(4, String::new());
        let name = "Mara Quinn";
        if e > 2.4 {
            focus = Some(0);
            st.1.values[0] = take_chars(name, ((e - 2.4) * 14.0) as usize).to_string();
        }
        if e > 3.6 {
            st.1.values[1] = "4".into();
            focus = None;
        }
        if e > 4.4 {
            focus = Some(2);
            st.1.values[2] = take_chars("20:00", ((e - 4.4) * 10.0) as usize).to_string();
        }
        if e > 5.3 {
            focus = None;
            st.1.values[3] = "true".into();
        }
    }
    let mut submit_rect = Rect::NOTHING;
    agent_scene(ui, "form", 560.0, "Book dinner at Osteria Lume for Friday", "reservations.request", e, |ui| {
        let (_, sr) = FormResult::new("Reserve a table", &fields, &mut st.1)
            .description("Osteria Lume · Friday · a few details first")
            .submit_label("Confirm booking")
            .highlight_field(focus)
            .id_salt(("form", cycle))
            .show(ui);
        submit_rect = sr;
    });
    if auto && submit_rect.is_positive() && !st.1.submitted && e > 5.6 {
        ghost(ui, "form", submit_rect.center(), true, e > 6.6);
        if e > 6.8 {
            st.1.submitted = true;
        }
    } else {
        ghost(ui, "form", submit_rect.center(), false, false);
    }
    set_mem(ui, "fs", st);
}

fn demo_options(ui: &mut Ui) {
    let (cycle, e) = scene_clock(ui, "opt", 11.0);
    let mut multi: f32 = mem(ui, "multi", || 0.0);
    let opts = [
        ToolOption::new("Rebase onto main", "Linear history, rewrites 4 local commits").tag("Recommended"),
        ToolOption::new("Merge main in", "Keeps both histories, adds a merge commit"),
        ToolOption::new("Squash and rebase", "One clean commit, loses intermediate steps"),
    ];
    let mut st: (u64, i32, OptionState) = mem(ui, "os", || (0, 0, OptionState::default()));
    if st.0 != cycle || st.1 != multi as i32 {
        st = (cycle, multi as i32, OptionState::default());
    }
    let mut rows = Vec::new();
    let mut confirm = Rect::NOTHING;
    agent_scene(ui, "opt", 560.0, "Bring my branch up to date with main", "git.plan", e, |ui| {
        let (_, r, c) = OptionListResult::new("How should I integrate main?", &opts, &mut st.2)
            .multiple(multi > 0.5)
            .id_salt(("opt", cycle, multi as i32))
            .show(ui);
        rows = r;
        confirm = c;
    });
    if !st.2.confirmed && rows.len() == 3 && e < 10.0 {
        let (target, press) = if e < 4.2 {
            (rows[0].center() + vec2(60.0, 0.0), e > 3.9)
        } else {
            (confirm.center(), e > 5.4)
        };
        if e > 3.0 {
            ghost(ui, "opt", target, true, press);
        }
        if e > 4.05 && !st.2.selected[0] {
            st.2.selected[0] = true;
        }
        if e > 5.6 {
            st.2.confirmed = true;
        }
    } else {
        ghost(ui, "opt", pos2(0.0, 0.0), false, false);
    }
    sliders(ui, |ui| {
        ui.add(ControlSlider::new("Multiple choice", &mut multi, 0.0..=1.0).width(220.0));
    });
    set_mem(ui, "multi", multi);
    set_mem(ui, "os", st);
}

fn demo_plan(ui: &mut Ui) {
    let (cycle, e) = scene_clock(ui, "plan", 13.0);
    let titles = [
        ("Reproduce the flaky checkout test", "Ran it 40× locally, failed 3 times"),
        ("Find the race in the cart store", "Two writers update totals without a lock"),
        ("Serialize cart updates", "Queue writes through a single reducer"),
        ("Add a regression test", "Hammers the cart with 200 parallel updates"),
        ("Open a pull request", "With the repro and timings in the description"),
    ];
    let pos = ((e - 2.0) / 1.7).max(0.0);
    let steps: Vec<PlanStep> = titles
        .iter()
        .enumerate()
        .map(|(i, (a, b))| {
            let s = if (i as f32) < pos.floor() {
                PlanStatus::Done
            } else if i == pos.floor() as usize {
                PlanStatus::Active
            } else {
                PlanStatus::Todo
            };
            PlanStep::new(*a).detail(*b).status(s)
        })
        .collect();
    agent_scene(ui, "plan", 560.0, "The checkout test is flaky again, can you fix it?", "agent.plan", e, |ui| {
        ui.add(PlanResult::new("Fix flaky checkout test", &steps).id_salt(("plan", cycle)));
    });
}

fn demo_steps(ui: &mut Ui) {
    let (_cycle, e) = scene_clock(ui, "steps", 9.5);
    let mut speed: f32 = mem(ui, "spd", || 1.0);
    let t = Theme::of(ui);
    let labels = ["Search", "Read 6 sources", "Compare", "Draft answer", "Cite"];
    let prog = ((e - 0.6) * 0.62 * speed).clamp(0.0, 5.0);
    let el = (prog.fract() / (0.62 * speed)).max(0.0);
    column(ui, 640.0, |ui| {
        let top = ui.cursor().min;
        ui.add_space(20.0);
        ui.add(ProgressSteps::new(&labels, prog).elapsed(el));
        ui.add_space(18.0);
        let labels2 = ["Plan", "Edit files", "Run tests", "Ship"];
        ui.add(ProgressSteps::new(&labels2, (prog * 0.8).min(4.0)).accent(t.success).id_salt("steps2"));
        ui.add_space(14.0);
        set_mem(ui, "steps_area", Rect::from_min_max(top, ui.cursor().min + vec2(640.0, 0.0)));
    });
    sliders(ui, |ui| {
        ui.add(ControlSlider::new("Agent speed", &mut speed, 0.3..=3.0).decimals(1).suffix("×").width(240.0));
    });
    set_mem(ui, "spd", speed);
}

const PY: &str = "import statistics as st\n\nlatencies = [112, 98, 131, 87, 240, 102, 95]\nprint(\"p50:\", st.median(latencies), \"ms\")\nprint(\"p95:\", st.quantiles(latencies, n=20)[-1], \"ms\")\nprint(\"outliers:\", [x for x in latencies if x > 200])";

fn demo_code(ui: &mut Ui) {
    let (cycle, e) = scene_clock(ui, "code", 11.0);
    let mut lt: f32 = mem(ui, "lt", || 0.35);
    let out = ["p50: 102 ms", "p95: 131 ms", "outliers: [240]", "✓ finished"];
    let mut run_rect = Rect::NOTHING;
    agent_scene(ui, "code", 600.0, "What are the p50 and p95 latencies in this sample?", "python.run", e, |ui| {
        let (_, r) = CodeResult::new("python", PY, &out)
            .filename("latency.py")
            .line_time(lt)
            .run_now(e > 3.4 && e < 3.5)
            .id_salt(("code", cycle))
            .show(ui);
        run_rect = r;
    });
    ghost(ui, "code", run_rect.center(), e > 2.2 && e < 3.9, e > 3.2 && e < 3.5);
    sliders(ui, |ui| {
        ui.add(ControlSlider::new("Line delay", &mut lt, 0.05..=1.0).decimals(2).suffix("s").width(240.0));
    });
    set_mem(ui, "lt", lt);
}

fn demo_location(ui: &mut Ui) {
    let (cycle, e) = scene_clock(ui, "loc", 12.0);
    let places = [
        Place::new("Blue Door Coffee", "14 Harbor Street · open until 6pm", "6 min"),
        Place::new("Kiln & Crumb", "2 Mill Lane · great pastries", "11 min"),
        Place::new("Fika Corner", "88 Canal Road · quiet, laptops ok", "14 min"),
    ];
    let sel = if e < 5.0 { 0 } else if e < 8.0 { 1 } else { 2 };
    let mut rows = Vec::new();
    agent_scene(ui, "loc", 520.0, "Any good coffee within walking distance?", "maps.search", e, |ui| {
        let (_, r) = LocationResult::new("Coffee near you", &places).selected(Some(sel)).id_salt(("loc", cycle)).show(ui);
        rows = r;
    });
    if rows.len() == 3 && e > 4.0 && e < 10.0 {
        ghost(ui, "loc", rows[sel.max(1)].center() + vec2(40.0, 0.0), true, false);
    } else {
        ghost(ui, "loc", pos2(0.0, 0.0), false, false);
    }
    let _ = Icon::Globe;
}

pub(super) fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "asst-tool-table",
            name: "Data Table Result",
            category: C,
            description: "A tool result as a sortable data table: rows slide in, numbers align, deltas are colored pills.",
            covers: &["Tool UI: Data Table", "assistant-ui: Data table", "CopilotKit: Generative UI (table)"],
            code: r##"let cols = [("Region", ColumnKind::Text), ("Revenue", ColumnKind::Number), ("QoQ", ColumnKind::Delta)];
ui.add(DataTableResult::new("warehouse.query", &cols, &rows).subtitle("SELECT ...").max_rows(6));"##,
            show: demo_table,
        },
        Demo {
            id: "asst-tool-chart",
            name: "Chart Result",
            category: C,
            description: "The agent answers with a chart card: headline value, change pill and an animated area, line or bar chart.",
            covers: &["Tool UI: Chart", "assistant-ui: Chart", "CopilotKit: Generative UI (chart)"],
            code: r##"let s = [Series::new("Signups", vec![320.0, 410.0, 520.0, 860.0])];
ui.add(ChartResult::new("analytics.timeseries", &s).labels(&["W1", "W2", "W3", "W4"])
    .headline("860", "+18.6%").kind(ChartKind::Area));"##,
            show: demo_chart,
        },
        Demo {
            id: "asst-tool-link",
            name: "Link Preview Result",
            category: C,
            description: "Unfurled links as cards: cover image with a slow hover zoom, favicon and domain, title and description.",
            covers: &["Tool UI: Link Preview", "assistant-ui: Link preview"],
            code: r##"if ui.add(LinkPreviewResult::new("fieldnotes.dev", "Designing calm agent interfaces", "Why quiet UI earns trust.")).clicked() {
    // open the link
}"##,
            show: demo_link,
        },
        Demo {
            id: "asst-tool-gallery",
            name: "Image Gallery Result",
            category: C,
            description: "Image results as a hero-plus-grid gallery with shimmer loading, +N tile and a lightbox that grows from the tile.",
            covers: &["Tool UI: Image Gallery", "assistant-ui: Image"],
            code: r##"let captions = ["Alpenglow", "Lagoon", "Aurora", "Dunes", "Forest", "Neon"];
ui.add(ImageGalleryResult::new("images.search", &captions));"##,
            show: demo_gallery,
        },
        Demo {
            id: "asst-tool-weather",
            name: "Weather Card",
            category: C,
            description: "A generative weather card: condition-tinted sky, animated painted glyph, counting temperature and an hourly curve.",
            covers: &["Tool UI: Weather Widget", "assistant-ui: Current weather", "CopilotKit: Generative UI (weather)"],
            code: r##"let hourly = [18.0, 20.0, 23.0, 24.0, 22.0];
ui.add(WeatherResult::new("Lisbon, PT", 24.0, Sky::Sunny).hourly(&hourly).range(16.0, 25.0));"##,
            show: demo_weather,
        },
        Demo {
            id: "asst-tool-form",
            name: "Form Result (HITL)",
            category: C,
            description: "The agent pauses for details: text, segmented choice and switch fields, then folds into a receipt on submit.",
            covers: &["Tool UI: Form", "CopilotKit: Human-in-the-loop", "assistant-ui: Table reservation"],
            code: r##"let fields = [FormField::text("Name", "Ada"), FormField::choice("Party size", &["2", "4", "6"]), FormField::toggle("Window table")];
FormResult::new("Reserve a table", &fields, &mut state).submit_label("Confirm booking").show(ui);
if state.submitted { /* resume the agent with state.values */ }"##,
            show: demo_form,
        },
        Demo {
            id: "asst-tool-options",
            name: "Option List Result",
            category: C,
            description: "The agent asks you to pick one or many options; confirming collapses the list to the chosen rows.",
            covers: &["Tool UI: Option List", "assistant-ui: Option list", "CopilotKit: Human-in-the-loop (choice)"],
            code: r##"let opts = [ToolOption::new("Rebase", "Linear history").tag("Recommended"), ToolOption::new("Merge", "Keep both")];
OptionListResult::new("How should I integrate main?", &opts, &mut state).multiple(false).show(ui);"##,
            show: demo_options,
        },
        Demo {
            id: "asst-tool-plan",
            name: "Plan Result",
            category: C,
            description: "A live to-do plan: progress bar, spinning active step with detail, self-drawing checks as steps finish.",
            covers: &["Tool UI: Plan", "assistant-ui: Agent plan", "CopilotKit: Agentic plan"],
            code: r##"let steps = [PlanStep::new("Reproduce").status(PlanStatus::Done), PlanStep::new("Fix").detail("Serialize writes").status(PlanStatus::Active)];
ui.add(PlanResult::new("Fix flaky checkout test", &steps));"##,
            show: demo_plan,
        },
        Demo {
            id: "asst-progress-steps",
            name: "Progress Steps",
            category: C,
            description: "A horizontal tracker of an agent's phases: a gliding rail, pulsing current node and checks as phases complete.",
            covers: &["Tool UI: Progress Tracker", "nextui (agent): Agent Steps", "assistant-ui: Job progress"],
            code: r##"let steps = ["Search", "Read 6 sources", "Draft", "Cite"];
ui.add(ProgressSteps::new(&steps, progress).elapsed(secs_in_step));"##,
            show: demo_steps,
        },
        Demo {
            id: "asst-tool-code",
            name: "Code Result",
            category: C,
            description: "Generated code with a Run button: a console slides open and output streams in, ending with an exit chip.",
            covers: &["Tool UI: Code Block", "assistant-ui: Code runner", "nextui (agent): Code Execution"],
            code: r##"let out = ["p50: 102 ms", "p95: 131 ms"];
let (_resp, _run_rect) = CodeResult::new("python", code, &out).filename("latency.py").show(ui);"##,
            show: demo_code,
        },
        Demo {
            id: "asst-tool-location",
            name: "Location Result",
            category: C,
            description: "A painted map with a self-drawing route, bouncing pin and a place list that moves the pin on hover.",
            covers: &["Tool UI: Map", "assistant-ui: Geo map", "CopilotKit: Generative UI (map)"],
            code: r##"let places = [Place::new("Blue Door Coffee", "14 Harbor St", "6 min")];
let (_resp, _rows) = LocationResult::new("Coffee near you", &places).show(ui);"##,
            show: demo_location,
        },
    ]
}
