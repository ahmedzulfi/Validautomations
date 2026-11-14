use crate::*;
use egui::{vec2, RichText, Ui};
use glowkit_core::demo::{Category::Charts as C, Demo};
use glowkit_core::Theme;

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const MONTHS6: [&str; 6] = ["January", "February", "March", "April", "May", "June"];

fn desktop() -> Vec<f64> {
    vec![186.0, 305.0, 237.0, 73.0, 209.0, 214.0]
}
fn mobile() -> Vec<f64> {
    vec![80.0, 200.0, 120.0, 190.0, 130.0, 140.0]
}

fn dollars(v: f64) -> String {
    format!("${}", format_compact(v))
}

/// A muted caption above a variant.
fn caption(ui: &mut Ui, text: &str) {
    let t = Theme::of(ui);
    ui.label(RichText::new(text).size(12.0).color(t.muted_foreground));
    ui.add_space(2.0);
}

/// Lay out `n` equal columns with a gap.
fn cols(ui: &mut Ui, n: usize, mut f: impl FnMut(usize, &mut Ui)) {
    ui.columns(n, |c| {
        for (i, col) in c.iter_mut().enumerate() {
            col.push_id(i, |ui| f(i, ui));
        }
    });
}

/// A small persistent toggle stored in the demo's ui data.
fn toggle_state(ui: &mut Ui, key: &str) -> bool {
    let id = egui::Id::unique(("glowkit_charts_demo_toggle", key));
    ui.data_mut(|d| *d.get_temp_mut_or_insert_with(id, || false))
}

fn set_toggle(ui: &mut Ui, key: &str, v: bool) {
    let id = egui::Id::unique(("glowkit_charts_demo_toggle", key));
    ui.data_mut(|d| d.insert_temp(id, v));
}

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "line-chart",
            name: "Line Chart",
            category: C,
            description: "Multi-series lines that draw in, with a snapping crosshair tooltip, clickable legend, glow, gradient and dashed variants.",
            covers: &["shadcn/ui: Line Chart", "Recharts: LineChart", "Tremor: Line Chart", "Evil Charts: Glowing Line", "Evil Charts: Dotted Line", "Nivo: Line", "Unovis: Line", "shadcn/ui: Chart"],
            code: r##"let series = [Series::new("Desktop", desktop), Series::new("Mobile", mobile)];
ui.add(LineChart::new(&series).labels(&MONTHS).dots(true).height(260.0));
ui.add(LineChart::new(&revenue).glow(true).gradient_stroke(true).curve(Curve::Natural));
ui.add(LineChart::new(&plan).series_style(1, LineStyle::Dashed).curve(Curve::Step));"##,
            show: |ui| {
                let s = [Series::new("Desktop", desktop()), Series::new("Mobile", mobile())];
                cols(ui, 2, |i, ui| match i {
                    0 => {
                        caption(ui, "Monotone with dots · hover for the tooltip, click the legend");
                        ui.add(LineChart::new(&s).labels(&MONTHS6.map(|m| &m[..3])).dots(true).height(270.0));
                    }
                    _ => {
                        caption(ui, "Glow + gradient stroke");
                        let rev = [Series::new("Revenue", [4.2, 5.8, 5.1, 7.4, 6.9, 9.2, 8.6, 10.4, 11.8, 10.9, 13.2, 15.1].map(|v| v * 1000.0))];
                        ui.add(LineChart::new(&rev).labels(&MONTHS).glow(true).gradient_stroke(true).line_width(2.5).curve(Curve::Natural).format(dollars).legend(false).height(270.0));
                    }
                });
                ui.add_space(16.0);
                cols(ui, 2, |i, ui| match i {
                    0 => {
                        caption(ui, "Actual vs. dashed forecast");
                        let plan = [
                            Series::new("Actual", [32.0, 41.0, 38.0, 52.0, 49.0, 61.0, 58.0, 66.0]),
                            Series::new("Forecast", [30.0, 36.0, 42.0, 48.0, 54.0, 60.0, 66.0, 72.0]),
                        ];
                        ui.add(LineChart::new(&plan).labels(&MONTHS[..8]).series_style(1, LineStyle::Dashed).curve(Curve::Linear).dots(true).height(220.0));
                    }
                    _ => {
                        caption(ui, "Step curve, dotted");
                        let st = [Series::new("Active servers", [4.0, 4.0, 6.0, 6.0, 9.0, 7.0, 7.0, 10.0, 12.0, 12.0, 11.0, 14.0])];
                        ui.add(LineChart::new(&st).labels(&MONTHS).curve(Curve::Step).line_style(LineStyle::Dotted).line_width(2.5).legend(false).height(220.0));
                    }
                });
            },
        },
        Demo {
            id: "area-chart",
            name: "Area Chart",
            category: C,
            description: "Gradient-filled areas that rise from the baseline: overlapping, stacked and expanded to 100%, with a dataset switch that animates.",
            covers: &["shadcn/ui: Area Chart", "Recharts: AreaChart", "Tremor: Area Chart", "Evil Charts: Gradient Area", "Nivo: Stream (stacked area)", "Unovis: Area"],
            code: r##"ui.add(AreaChart::new(&series).labels(&MONTHS).height(260.0));
ui.add(AreaChart::new(&series).stacking(Stacking::Stacked));
ui.add(AreaChart::new(&series).stacking(Stacking::Expanded).curve(Curve::Natural));"##,
            show: |ui| {
                let last_year = toggle_state(ui, "last_year");
                let (d, m) = if last_year { (vec![120.0, 180.0, 260.0, 210.0, 150.0, 240.0], vec![140.0, 110.0, 160.0, 230.0, 210.0, 180.0]) } else { (desktop(), mobile()) };
                let s = [Series::new("Desktop", d), Series::new("Mobile", m)];
                ui.horizontal(|ui| {
                    caption(ui, "Gradient fill · ");
                    if ui.selectable_label(!last_year, "This year").clicked() {
                        set_toggle(ui, "last_year", false);
                    }
                    if ui.selectable_label(last_year, "Last year").clicked() {
                        set_toggle(ui, "last_year", true);
                    }
                });
                ui.add(AreaChart::new(&s).labels(&MONTHS6.map(|m| &m[..3])).height(240.0).id_salt("main"));
                ui.add_space(16.0);
                let traffic = [
                    Series::new("Organic", [420.0, 460.0, 510.0, 480.0, 560.0, 620.0, 640.0, 700.0, 690.0, 760.0, 820.0, 880.0]),
                    Series::new("Paid", [210.0, 260.0, 240.0, 300.0, 330.0, 310.0, 380.0, 360.0, 420.0, 400.0, 460.0, 520.0]),
                    Series::new("Referral", [90.0, 110.0, 130.0, 120.0, 150.0, 170.0, 160.0, 190.0, 210.0, 200.0, 240.0, 260.0]),
                ];
                cols(ui, 2, |i, ui| match i {
                    0 => {
                        caption(ui, "Stacked");
                        ui.add(AreaChart::new(&traffic).labels(&MONTHS).stacking(Stacking::Stacked).height(230.0));
                    }
                    _ => {
                        caption(ui, "Expanded (100%)");
                        ui.add(AreaChart::new(&traffic).labels(&MONTHS).stacking(Stacking::Expanded).curve(Curve::Natural).height(230.0));
                    }
                });
            },
        },
        Demo {
            id: "bar-chart",
            name: "Bar Chart",
            category: C,
            description: "Bars that grow in with a stagger: grouped, stacked, horizontal, negative values, value labels, hover highlight and animated striped fills.",
            covers: &["shadcn/ui: Bar Chart", "Recharts: BarChart", "Tremor: Bar Chart", "Evil Charts: Highlighted Bar", "Evil Charts: Hatched Bar", "Nivo: Bar", "Unovis: Grouped Bar / Stacked Bar"],
            code: r##"ui.add(BarChart::new(&series).labels(&MONTHS).radius(6.0));
ui.add(BarChart::new(&series).stacking(Stacking::Stacked).value_labels(true));
ui.add(BarChart::new(&browsers).horizontal().color_by_category(true).value_labels(true));
ui.add(BarChart::new(&profit).fill(BarFill::Striped).highlight_hover(true));"##,
            show: |ui| {
                let shuffled = toggle_state(ui, "shuffle");
                let s = if shuffled {
                    [Series::new("Desktop", [120.0, 260.0, 300.0, 140.0, 180.0, 250.0]), Series::new("Mobile", [150.0, 120.0, 210.0, 230.0, 90.0, 170.0])]
                } else {
                    [Series::new("Desktop", desktop()), Series::new("Mobile", mobile())]
                };
                let short = MONTHS6.map(|m| &m[..3]);
                cols(ui, 3, |i, ui| match i {
                    0 => {
                        ui.horizontal(|ui| {
                            caption(ui, "Grouped");
                            if ui.small_button("Shuffle data").clicked() {
                                set_toggle(ui, "shuffle", !shuffled);
                            }
                        });
                        ui.add(BarChart::new(&s).labels(&short).height(250.0));
                    }
                    1 => {
                        caption(ui, "Stacked with totals");
                        ui.add(BarChart::new(&s).labels(&short).stacking(Stacking::Stacked).value_labels(true).radius(5.0).height(250.0));
                    }
                    _ => {
                        caption(ui, "Horizontal, colored per bar");
                        let b = [Series::new("Visitors", [275.0, 200.0, 187.0, 173.0, 90.0])];
                        ui.add(BarChart::new(&b).labels(&["Chrome", "Safari", "Firefox", "Edge", "Other"]).horizontal().color_by_category(true).value_labels(true).height(250.0));
                    }
                });
                ui.add_space(14.0);
                cols(ui, 2, |i, ui| match i {
                    0 => {
                        caption(ui, "Negative values · hover highlights the month");
                        let p = [Series::new("Net profit", [4.2, -1.8, 3.1, 6.4, -3.2, 2.5, 5.1, 7.3, -0.9, 4.4, 6.8, 9.1].map(|v| v * 1000.0))];
                        ui.add(BarChart::new(&p).labels(&MONTHS).highlight_hover(true).format(dollars).legend(false).height(200.0));
                    }
                    _ => {
                        caption(ui, "Animated striped & gradient fills");
                        let g = [Series::new("Planned", [40.0, 55.0, 48.0, 70.0, 62.0, 80.0]), Series::new("Shipped", [35.0, 50.0, 52.0, 64.0, 66.0, 74.0])];
                        ui.columns(2, |c| {
                            c[0].add(BarChart::new(&g[..1]).labels(&short).fill(BarFill::Striped).legend(false).height(200.0));
                            c[1].add(BarChart::new(&g[1..]).labels(&short).fill(BarFill::Gradient).legend(false).radius(8.0).height(200.0));
                        });
                    }
                });
            },
        },
        Demo {
            id: "pie-chart",
            name: "Pie & Donut Chart",
            category: C,
            description: "Slices sweep in; the hovered slice pops out; donuts show a center total that follows the hovered slice; padding, labels and rounded ends.",
            covers: &["shadcn/ui: Pie Chart", "shadcn/ui: Donut (Pie with text)", "Recharts: PieChart", "Tremor: Donut Chart", "Nivo: Pie", "Unovis: Donut", "Evil Charts: Rounded Pie"],
            code: r##"ui.add(PieChart::new(&browsers).labels(PieLabels::Percent));
ui.add(DonutChart::new(&devices).center_caption("Visitors"));
ui.add(DonutChart::new(&devices).rounded(true).pad_angle(4.0));
ui.add(PieChart::new(&browsers).labels(PieLabels::Outside));"##,
            show: |ui| {
                let browsers = [Datum::new("Chrome", 275.0), Datum::new("Safari", 200.0), Datum::new("Firefox", 187.0), Datum::new("Edge", 173.0), Datum::new("Other", 90.0)];
                let devices = [Datum::new("Desktop", 1260.0), Datum::new("Mobile", 940.0), Datum::new("Tablet", 310.0)];
                cols(ui, 2, |i, ui| match i {
                    0 => {
                        caption(ui, "Pie with percentages · hover a slice, click the legend");
                        ui.add(PieChart::new(&browsers).labels(PieLabels::Percent).height(280.0));
                    }
                    _ => {
                        caption(ui, "Donut with center total");
                        ui.add(DonutChart::new(&devices).center_caption("Visitors").height(280.0));
                    }
                });
                ui.add_space(16.0);
                cols(ui, 2, |i, ui| match i {
                    0 => {
                        caption(ui, "Rounded, padded donut");
                        ui.add(DonutChart::new(&browsers).rounded(true).pad_angle(2.0).inner_radius(0.76).center_caption("Sessions").legend(false).height(250.0));
                    }
                    _ => {
                        caption(ui, "Outside labels");
                        ui.add(PieChart::new(&browsers).labels(PieLabels::Outside).inner_radius(0.5).pad_angle(1.5).legend(false).height(250.0));
                    }
                });
            },
        },
        Demo {
            id: "radar-chart",
            name: "Radar Chart",
            category: C,
            description: "Translucent polygons unfold from the center over a web grid; hover an axis to compare series.",
            covers: &["shadcn/ui: Radar Chart", "Recharts: RadarChart", "Nivo: Radar", "Evil Charts: Radar"],
            code: r##"ui.add(RadarChart::new(&series).labels(&["Speed", "Power", "Range", "Comfort", "Safety", "Price"]));
ui.add(RadarChart::new(&series).grid_shape(RadarGrid::Circle).dots(false));"##,
            show: |ui| {
                let axes = ["Speed", "Power", "Range", "Comfort", "Safety", "Price"];
                let s = [Series::new("Model S", [88.0, 92.0, 76.0, 70.0, 86.0, 45.0]), Series::new("Model X", [64.0, 70.0, 90.0, 88.0, 72.0, 80.0])];
                let one = [Series::new("Skills", [80.0, 65.0, 90.0, 72.0, 58.0, 84.0, 69.0])];
                cols(ui, 2, |i, ui| match i {
                    0 => {
                        caption(ui, "Two series, polygon web");
                        ui.add(RadarChart::new(&s).labels(&axes).height(330.0));
                    }
                    _ => {
                        caption(ui, "Circular grid");
                        ui.add(RadarChart::new(&one).labels(&["Rust", "Go", "TypeScript", "Python", "SQL", "Design", "DevOps"]).grid_shape(RadarGrid::Circle).levels(5).fill_opacity(0.35).height(330.0));
                    }
                });
            },
        },
        Demo {
            id: "radial-bar-chart",
            name: "Radial Bar Chart",
            category: C,
            description: "Concentric rings sweep in one after another over faint tracks, with labels at each ring's start.",
            covers: &["shadcn/ui: Radial Chart", "Recharts: RadialBarChart", "Nivo: Radial Bar", "Evil Charts: Radial"],
            code: r##"ui.add(RadialBarChart::new(&browsers).sweep(270.0));
ui.add(RadialBarChart::new(&goals).max(100.0).sweep(360.0).center_title("78%"));"##,
            show: |ui| {
                let browsers = [Datum::new("Chrome", 275.0), Datum::new("Safari", 200.0), Datum::new("Firefox", 187.0), Datum::new("Edge", 173.0), Datum::new("Other", 90.0)];
                let goals = [Datum::new("Move", 82.0), Datum::new("Exercise", 64.0), Datum::new("Stand", 91.0)];
                cols(ui, 2, |i, ui| match i {
                    0 => {
                        caption(ui, "270° with ring labels");
                        ui.add(RadialBarChart::new(&browsers).height(320.0));
                    }
                    _ => {
                        caption(ui, "Activity rings");
                        let t = Theme::of(ui);
                        let g = [goals[0].clone().color(t.chart[2]), goals[1].clone().color(t.chart[4]), goals[2].clone().color(t.chart[1])];
                        ui.add(RadialBarChart::new(&g).max(100.0).sweep(360.0).inner_radius(0.42).ring_labels(false).center_title("79%").center_caption("Daily goal").height(320.0));
                    }
                });
            },
        },
        Demo {
            id: "gauge",
            name: "Gauge",
            category: C,
            description: "Semicircular gauges with a gradient arc, tick marks and a needle that springs to its value.",
            covers: &["Nivo: Radial Bar (gauge)", "Evil Charts: Gauge", "Tremor: Progress Circle"],
            code: r##"ui.add(Gauge::new(72.0).size(300.0).thickness(20.0).label("CPU load").unit("%"));
ui.add(Gauge::new(3.4).range(0.0, 5.0).needle(false).colors(&[cyan, violet]));"##,
            show: |ui| {
                let t = Theme::of(ui);
                ui.add_space(56.0);
                cols(ui, 3, |i, ui| {
                    ui.vertical_centered(|ui| match i {
                        0 => {
                            ui.add(Gauge::new(72.0).size(300.0).thickness(20.0).label("CPU load").unit("%"));
                        }
                        1 => {
                            ui.add(Gauge::new(38.0).size(300.0).thickness(20.0).label("Memory").unit("%"));
                        }
                        _ => {
                            ui.add(Gauge::new(4.3).range(0.0, 5.0).needle(false).colors(&[t.chart[1], t.chart[0]]).format(|v| format!("{v:.1}")).size(300.0).thickness(20.0).label("Customer rating").unit(" / 5"));
                        }
                    });
                });
            },
        },
        Demo {
            id: "sparkline",
            name: "Sparkline",
            category: C,
            description: "Tiny line, area and bar charts for tables and cards.",
            covers: &["Tremor: Spark Chart", "Recharts: Tiny Line/Area/Bar", "Nivo: Line (sparkline)"],
            code: r##"ui.add(Sparkline::new(&prices).kind(SparkKind::Area).color(green));
ui.add(Sparkline::new(&changes).kind(SparkKind::Bar));"##,
            show: |ui| {
                let t = Theme::of(ui);
                let rows: [(&str, &str, f64, [f64; 12]); 5] = [
                    ("AAPL", "Apple Inc.", 2.4, [180.0, 182.0, 181.0, 185.0, 184.0, 188.0, 187.0, 190.0, 189.0, 193.0, 192.0, 196.0]),
                    ("MSFT", "Microsoft", 1.1, [410.0, 405.0, 412.0, 409.0, 415.0, 418.0, 414.0, 420.0, 419.0, 417.0, 423.0, 425.0]),
                    ("NVDA", "NVIDIA", -3.2, [880.0, 900.0, 870.0, 860.0, 875.0, 850.0, 845.0, 830.0, 840.0, 820.0, 835.0, 812.0]),
                    ("TSLA", "Tesla", 5.7, [170.0, 168.0, 175.0, 172.0, 180.0, 178.0, 186.0, 184.0, 190.0, 188.0, 195.0, 199.0]),
                    ("AMZN", "Amazon", -0.6, [178.0, 180.0, 179.0, 182.0, 181.0, 180.0, 183.0, 181.0, 180.0, 179.0, 181.0, 177.0]),
                ];
                ui.add_space(10.0);
                let nowrap = |text: RichText| egui::Label::new(text).wrap_mode(egui::TextWrapMode::Extend);
                egui::Grid::new(ui.scope_id().with("spark_table")).num_columns(6).spacing(vec2(44.0, 16.0)).min_col_width(60.0).show(ui, |ui| {
                    for h in ["Symbol", "Price", "30D trend", "Volume", "Weekly close", "Change"] {
                        ui.add(nowrap(RichText::new(h).size(12.0).color(t.muted_foreground)));
                    }
                    ui.end_row();
                    for (k, (sym, name, ch, vals)) in rows.into_iter().enumerate() {
                        let col = if ch >= 0.0 { t.success } else { t.destructive };
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.add(nowrap(RichText::new(sym).strong().size(13.5)));
                            ui.add(nowrap(RichText::new(name).size(11.5).color(t.muted_foreground)));
                        });
                        ui.add(nowrap(RichText::new(format!("${:.2}", vals[11])).size(13.5)));
                        ui.add(Sparkline::new(&vals).kind(if ch >= 0.0 { SparkKind::Area } else { SparkKind::Line }).color(col).size(vec2(200.0, 36.0)));
                        let vol: Vec<f64> = (0..16).map(|i| 40.0 + ((i * 7 + k * 5) % 11) as f64 * 6.0 + if i > 11 { 25.0 } else { 0.0 }).collect();
                        ui.add(Sparkline::new(&vol).kind(SparkKind::Bar).color(t.chart[1]).size(vec2(150.0, 30.0)));
                        ui.add(Sparkline::new(&vals).kind(SparkKind::Line).curve(Curve::Step).end_dot(false).color(t.muted_foreground).size(vec2(110.0, 26.0)));
                        ui.add(nowrap(RichText::new(format!("{}{:.1}%", if ch >= 0.0 { "+" } else { "" }, ch)).color(col).size(13.5)));
                        ui.end_row();
                    }
                });
            },
        },
        Demo {
            id: "scatter-chart",
            name: "Scatter & Bubble Chart",
            category: C,
            description: "Points and size-scaled bubbles pop in with a stagger; the nearest point is ringed with a tooltip.",
            covers: &["Recharts: ScatterChart", "Tremor: Scatter Chart", "Nivo: ScatterPlot", "Nivo: Swarm/Bubble", "Unovis: Scatter"],
            code: r##"let s = [ScatterSeries::new("Enterprise", points).sizes(deal_sizes)];
ui.add(ScatterChart::new(&s).x_title("Sales cycle (days)").y_title("Win rate (%)"));"##,
            show: |ui| {
                let mut rng = glowkit_core::Rng::new(7);
                let mk = |rng: &mut glowkit_core::Rng, n: usize, cx: f64, cy: f64, sx: f64, sy: f64| -> Vec<[f64; 2]> {
                    (0..n).map(|_| [cx + (rng.f32() as f64 - 0.5) * sx, cy + (rng.f32() as f64 - 0.5) * sy]).collect()
                };
                let a = mk(&mut rng, 26, 30.0, 62.0, 30.0, 26.0);
                let b = mk(&mut rng, 26, 62.0, 38.0, 34.0, 28.0);
                let sz_a: Vec<f64> = (0..26).map(|_| 5.0 + rng.f32() as f64 * 60.0).collect();
                let sz_b: Vec<f64> = (0..26).map(|_| 5.0 + rng.f32() as f64 * 60.0).collect();
                let scatter = [ScatterSeries::new("SMB", a.clone()), ScatterSeries::new("Enterprise", b.clone())];
                let bubble = [ScatterSeries::new("SMB", a).sizes(sz_a), ScatterSeries::new("Enterprise", b).sizes(sz_b)];
                cols(ui, 2, |i, ui| match i {
                    0 => {
                        caption(ui, "Scatter");
                        ui.add(ScatterChart::new(&scatter).x_title("Sales cycle (days)").y_title("Win rate (%)").radius(5.0, 5.0).height(320.0));
                    }
                    _ => {
                        caption(ui, "Bubble (size = deal value, $k)");
                        ui.add(ScatterChart::new(&bubble).x_title("Sales cycle (days)").y_title("Win rate (%)").radius(4.0, 20.0).height(320.0));
                    }
                });
            },
        },
        Demo {
            id: "heatmap",
            name: "Heatmap",
            category: C,
            description: "A labelled matrix whose cells fade in along a diagonal, with a color key and hover tooltips.",
            covers: &["Nivo: HeatMap", "Unovis: Heatmap", "Evil Charts: Heatmap"],
            code: r##"ui.add(Heatmap::new(&rows).row_labels(&["Mon", "Tue", ...]).col_labels(&hours).show_values(true));"##,
            show: |ui| {
                let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
                let hours: Vec<String> = (8..20).map(|h| format!("{h}:00")).collect();
                let rows: Vec<Vec<f64>> = (0..7)
                    .map(|d| {
                        (0..12)
                            .map(|h| {
                                let peak = (-((h as f64 - 5.0) / 3.0).powi(2)).exp();
                                let wk = if d >= 5 { 0.35 } else { 1.0 };
                                ((peak * 90.0 + 10.0 + ((d * 7 + h * 3) % 11) as f64 * 2.0) * wk).round()
                            })
                            .collect()
                    })
                    .collect();
                caption(ui, "Support tickets by weekday and hour");
                ui.add(Heatmap::new(&rows).row_labels(&days).col_labels(&hours).show_values(true).height(330.0));
            },
        },
        Demo {
            id: "calendar-heatmap",
            name: "Calendar Heatmap",
            category: C,
            description: "A GitHub-style contribution graph: a year of days in weekly columns with five levels, month labels and day tooltips.",
            covers: &["Nivo: Calendar", "Nivo: TimeRange", "GitHub: contribution graph"],
            code: r##"ui.add(CalendarHeatmap::new(&daily).start_date(2025, 1, 1));"##,
            show: |ui| {
                let mut rng = glowkit_core::Rng::new(42);
                let vals: Vec<f64> = (0..365)
                    .map(|i| {
                        let wd = (i + 3) % 7;
                        let busy = if wd >= 5 { 0.3 } else { 1.0 };
                        let season = 0.6 + 0.4 * ((i as f32 / 365.0 * std::f32::consts::TAU).sin() * 0.5 + 0.5);
                        let r = rng.f32();
                        if r < 0.18 * (2.0 - busy as f32) { 0.0 } else { (r * 12.0 * season * busy as f32).round() as f64 }
                    })
                    .collect();
                ui.add_space(20.0);
                caption(ui, "Contributions in 2025");
                ui.add(CalendarHeatmap::new(&vals).start_date(2025, 1, 1).cell_size(18.0));
                ui.add_space(24.0);
                caption(ui, "Custom color");
                let t = Theme::of(ui);
                ui.add(CalendarHeatmap::new(&vals[..182]).start_date(2025, 1, 1).color(t.chart[0]).cell_size(16.0).unit("deploys").legend(false));
            },
        },
        Demo {
            id: "funnel-chart",
            name: "Funnel Chart",
            category: C,
            description: "Conversion stages as centered bars joined by smooth tapered bands, with drop-off and share of total.",
            covers: &["Nivo: Funnel", "Tremor: Funnel Chart", "Unovis: Funnel"],
            code: r##"ui.add(FunnelChart::new(&[
    Datum::new("Visited", 12_400.0), Datum::new("Signed up", 4_210.0),
    Datum::new("Activated", 2_380.0), Datum::new("Paid", 980.0),
]));"##,
            show: |ui| {
                let d = [Datum::new("Visited site", 12_400.0), Datum::new("Signed up", 4_210.0), Datum::new("Activated", 2_380.0), Datum::new("Subscribed", 980.0), Datum::new("Renewed", 610.0)];
                cols(ui, 2, |i, ui| match i {
                    0 => {
                        caption(ui, "Single color");
                        ui.add(FunnelChart::new(&d).height(340.0));
                    }
                    _ => {
                        caption(ui, "Multicolor");
                        ui.add(FunnelChart::new(&d[..4]).multicolor(true).height(340.0));
                    }
                });
            },
        },
        Demo {
            id: "bar-list",
            name: "Bar List & Progress List",
            category: C,
            description: "Ranked horizontal bars with labels inside and values on the right, plus labelled progress bars against targets.",
            covers: &["Tremor: Bar List", "Tremor: Category Bar / Progress Bar", "shadcn/ui: Progress"],
            code: r##"ui.add(BarList::new(&pages).sorted(true).share(true));
ui.add(ProgressList::new(&spend, &budgets).format(dollars));"##,
            show: |ui| {
                let pages = [
                    Datum::new("/home", 4_830.0),
                    Datum::new("/pricing", 2_751.0),
                    Datum::new("/docs/getting-started", 1_920.0),
                    Datum::new("/blog/launch-week", 1_471.0),
                    Datum::new("/changelog", 812.0),
                    Datum::new("/careers", 390.0),
                ];
                let spend = [Datum::new("Marketing", 42_300.0), Datum::new("Engineering", 118_000.0), Datum::new("Sales", 61_200.0), Datum::new("Support", 27_400.0)];
                let budgets = [60_000.0, 150_000.0, 58_000.0, 40_000.0];
                cols(ui, 2, |i, ui| match i {
                    0 => {
                        caption(ui, "Top pages · visitors");
                        ui.add(BarList::new(&pages).sorted(true).share(true));
                    }
                    _ => {
                        caption(ui, "Budget used");
                        ui.add(ProgressList::new(&spend, &budgets).format(dollars));
                    }
                });
            },
        },
        Demo {
            id: "treemap",
            name: "Treemap",
            category: C,
            description: "A squarified treemap whose tiles scale in by size, with labels that appear when they fit.",
            covers: &["Nivo: TreeMap", "Recharts: Treemap", "Unovis: Treemap"],
            code: r##"ui.add(Treemap::new(&costs).format(dollars).height(320.0));"##,
            show: |ui| {
                let costs = [
                    Datum::new("Compute", 42_000.0),
                    Datum::new("Storage", 21_500.0),
                    Datum::new("Database", 16_800.0),
                    Datum::new("Network", 12_300.0),
                    Datum::new("CDN", 8_100.0),
                    Datum::new("Monitoring", 5_400.0),
                    Datum::new("Queues", 3_900.0),
                    Datum::new("DNS", 1_200.0),
                    Datum::new("Secrets", 800.0),
                ];
                cols(ui, 2, |i, ui| match i {
                    0 => {
                        caption(ui, "Cloud spend by service");
                        ui.add(Treemap::new(&costs).format(dollars).height(330.0));
                    }
                    _ => {
                        caption(ui, "Single color by value");
                        let t = Theme::of(ui);
                        ui.add(Treemap::new(&costs).color(t.chart[1]).format(dollars).height(330.0).id_salt("mono"));
                    }
                });
            },
        },
        Demo {
            id: "candlestick-chart",
            name: "Candlestick Chart",
            category: C,
            description: "OHLC candles growing from their midpoints, with a price-tagged crosshair and an OHLC tooltip.",
            covers: &["Nivo: (custom) Candlestick", "TradingView Lightweight Charts: Candlestick", "Unovis: Candlestick"],
            code: r##"ui.add(CandlestickChart::new(&candles).labels(&dates).height(320.0));"##,
            show: |ui| {
                let mut rng = glowkit_core::Rng::new(3);
                let mut price = 182.0;
                let mut candles = Vec::new();
                let mut dates = Vec::new();
                for i in 0..40 {
                    let open = price;
                    let drift = ((i as f64) / 6.0).sin() * 1.4 + 0.35;
                    let close = open + drift + (rng.f32() as f64 - 0.5) * 6.0;
                    let high = open.max(close) + rng.f32() as f64 * 2.8;
                    let low = open.min(close) - rng.f32() as f64 * 2.8;
                    candles.push(Candle::new(open, high, low, close));
                    price = close;
                    dates.push(format!("{} {}", if i < 20 { "Sep" } else { "Oct" }, i % 20 + 1));
                }
                caption(ui, "ACME · daily");
                ui.add(CandlestickChart::new(&candles).labels(&dates).format(|v| format!("${v:.2}")).height(340.0));
            },
        },
        Demo {
            id: "combo-chart",
            name: "Combo Chart",
            category: C,
            description: "Columns and a glowing line sharing categories, optionally on two y axes.",
            covers: &["Recharts: ComposedChart", "Tremor: Combo Chart", "Nivo: Bar + Line"],
            code: r##"let bars = [Series::new("Revenue", revenue)];
let lines = [Series::new("Margin", margin)];
ui.add(ComboChart::new(&bars, &lines).labels(&MONTHS).dual_axis(true).line_format(|v| format!("{v:.0}%")));"##,
            show: |ui| {
                let bars = [Series::new("Revenue", [18.0, 22.0, 21.0, 27.0, 31.0, 29.0, 35.0, 38.0, 36.0, 42.0, 47.0, 52.0].map(|v| v * 1000.0)), Series::new("Costs", [14.0, 15.0, 17.0, 18.0, 20.0, 21.0, 22.0, 24.0, 25.0, 26.0, 28.0, 29.0].map(|v| v * 1000.0))];
                let lines = [Series::new("Margin", [22.0, 32.0, 19.0, 33.0, 35.0, 28.0, 37.0, 37.0, 31.0, 38.0, 40.0, 44.0])];
                caption(ui, "Revenue and costs (left axis) with margin % (right axis)");
                ui.add(ComboChart::new(&bars, &lines).labels(&MONTHS).dual_axis(true).format(dollars).line_format(|v| format!("{v:.0}%")).height(340.0));
            },
        },
        Demo {
            id: "stat-tile",
            name: "KPI Stat Tile",
            category: C,
            description: "KPI cards with a counting number, a colored delta badge and a sparkline.",
            covers: &["Tremor: KPI Card", "shadcn/ui: Dashboard cards", "Tremor: Badge Delta"],
            code: r##"ui.add(StatTile::new("Total revenue", 45_231.89).prefix("$").decimals(2)
    .delta(20.1).caption("vs last month").spark(&revenue));"##,
            show: |ui| {
                ui.add_space(20.0);
                let w = ((ui.available_width() - 3.0 * 16.0) / 4.0).max(200.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    ui.add(StatTile::new("Total revenue", 45_231.89).prefix("$").decimals(2).delta(20.1).caption("vs last month").spark(&[12.0, 18.0, 15.0, 22.0, 20.0, 28.0, 26.0, 34.0]).size(vec2(w, 176.0)));
                    ui.add(StatTile::new("Subscriptions", 2_350.0).prefix("+").delta(180.1).caption("vs last month").spark(&[5.0, 9.0, 7.0, 12.0, 15.0, 14.0, 21.0, 24.0]).size(vec2(w, 176.0)));
                    ui.add(StatTile::new("Churn rate", 2.4).suffix("%").decimals(1).delta(-0.6).invert_delta(true).caption("vs last month").spark(&[3.4, 3.1, 3.2, 2.9, 2.8, 2.7, 2.5, 2.4]).spark_kind(SparkKind::Line).size(vec2(w, 176.0)));
                    ui.add(StatTile::new("Avg. latency", 182.0).suffix(" ms").delta(12.4).invert_delta(true).caption("vs last week").spark(&[150.0, 160.0, 155.0, 170.0, 165.0, 178.0, 175.0, 182.0]).spark_kind(SparkKind::Bar).size(vec2(w, 176.0)));
                });
            },
        },
        Demo {
            id: "tracker",
            name: "Tracker & Donut Progress",
            category: C,
            description: "A status tracker of colored blocks with tooltips (uptime), and circular progress rings.",
            covers: &["Tremor: Tracker", "Tremor: Progress Circle", "shadcn/ui: Radial (progress)"],
            code: r##"ui.add(Tracker::new(&days).height(32.0));
ui.add(DonutProgress::new(0.72).label("Storage"));"##,
            show: |ui| {
                let t = Theme::of(ui);
                let blocks: Vec<TrackerBlock> = (0..90)
                    .map(|i| {
                        let st = match i {
                            17 | 18 => Status::Warning,
                            41 => Status::Down,
                            63 => Status::Warning,
                            _ => Status::Ok,
                        };
                        let txt = match st {
                            Status::Ok => "Operational",
                            Status::Warning => "Degraded performance",
                            Status::Down => "Major outage",
                            Status::Empty => "No data",
                        };
                        TrackerBlock::new(st, format!("Day {}: {txt}", i + 1))
                    })
                    .collect();
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("api.example.com").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("99.82% uptime").color(t.muted_foreground));
                    });
                });
                ui.add(Tracker::new(&blocks).height(34.0));
                ui.horizontal(|ui| {
                    ui.label(RichText::new("90 days ago").size(11.0).color(t.muted_foreground));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("Today").size(11.0).color(t.muted_foreground));
                    });
                });
                ui.add_space(28.0);
                cols(ui, 4, |i, ui| {
                    ui.vertical_centered(|ui| match i {
                        0 => {
                            ui.add(DonutProgress::new(0.72).label("Storage"));
                        }
                        1 => {
                            ui.add(DonutProgress::new(0.45).color(t.chart[1]).label("Bandwidth"));
                        }
                        2 => {
                            ui.add(DonutProgress::new(0.91).color(t.success).label("Tests passing"));
                        }
                        _ => {
                            ui.add(DonutProgress::new(0.64).gradient(&t.gradient).label("Quarterly goal"));
                        }
                    });
                });
            },
        },
    ]
}
