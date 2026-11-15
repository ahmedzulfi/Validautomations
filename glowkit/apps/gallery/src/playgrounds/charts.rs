//! Chart playgrounds.

use super::{variant, Code, Params, Playground};
use glowkit::charts::*;
use glowkit::color::hex;
use glowkit::core::playground::{lit_f32, lit_str};
use glowkit::egui::{vec2, Ui};

pub fn all() -> Vec<Playground> {
    vec![
        Playground { id: "line-chart", show: line },
        Playground { id: "area-chart", show: area },
        Playground { id: "bar-chart", show: bar },
        Playground { id: "pie-chart", show: pie },
        Playground { id: "gauge", show: gauge },
        Playground { id: "sparkline", show: sparkline },
    ]
}

const MONTHS: [&str; 6] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];
const DESKTOP: [f64; 6] = [186.0, 305.0, 237.0, 73.0, 209.0, 214.0];
const MOBILE: [f64; 6] = [80.0, 200.0, 120.0, 190.0, 130.0, 140.0];
const CURVES: [&str; 4] = ["Linear", "Monotone", "Natural", "Step"];
const STACKING: [&str; 3] = ["None", "Stacked", "Expanded"];

fn curve(i: usize) -> Curve {
    [Curve::Linear, Curve::Monotone, Curve::Natural, Curve::Step][i]
}

fn stacking(i: usize) -> Stacking {
    [Stacking::None, Stacking::Stacked, Stacking::Expanded][i]
}

const SERIES_CODE: &str = "let series = [\n    Series::new(\"Desktop\", [186.0, 305.0, 237.0, 73.0, 209.0, 214.0]),\n    Series::new(\"Mobile\", [80.0, 200.0, 120.0, 190.0, 130.0, 140.0]),\n];";

fn series() -> [Series; 2] {
    [Series::new("Desktop", DESKTOP), Series::new("Mobile", MOBILE)]
}

fn line(ui: &mut Ui, p: &mut Params) -> String {
    let c = p.choice("Curve", &CURVES, 1);
    let width = p.f32("Line width", 2.0, 0.5..=6.0, 0.1);
    let style = p.choice("Line style", &["Solid", "Dashed", "Dotted"], 0);
    let dots = p.bool("Dots", true);
    let glow = p.bool("Glow", false);
    let grad = p.bool("Gradient stroke", false);
    let height = 320.0;
    let s = series();
    ui.add(
        LineChart::new(&s)
            .labels(&MONTHS)
            .curve(curve(c))
            .line_width(width)
            .line_style([LineStyle::Solid, LineStyle::Dashed, LineStyle::Dotted][style])
            .dots(dots)
            .glow(glow)
            .gradient_stroke(grad)
            .height(height),
    );
    Code::new("glowkit::charts::{Curve, LineChart, LineStyle, Series}", "LineChart::new(&series)")
        .pre(SERIES_CODE)
        .call("labels", "&[\"Jan\", \"Feb\", \"Mar\", \"Apr\", \"May\", \"Jun\"]")
        .call("curve", variant("Curve", &CURVES, c))
        .f("line_width", width)
        .call("line_style", variant("LineStyle", &["Solid", "Dashed", "Dotted"], style))
        .b("dots", dots)
        .b("glow", glow)
        .b("gradient_stroke", grad)
        .f("height", height)
        .add()
}

fn area(ui: &mut Ui, p: &mut Params) -> String {
    let st = p.choice("Stacking", &STACKING, 0);
    let c = p.choice("Curve", &CURVES, 2);
    let line = p.bool("Line", true);
    let dots = p.bool("Dots", false);
    let glow = p.bool("Glow", false);
    let height = 320.0;
    let s = series();
    ui.add(
        AreaChart::new(&s)
            .labels(&MONTHS)
            .stacking(stacking(st))
            .curve(curve(c))
            .line(line)
            .dots(dots)
            .glow(glow)
            .height(height),
    );
    Code::new("glowkit::charts::{AreaChart, Curve, Series, Stacking}", "AreaChart::new(&series)")
        .pre(SERIES_CODE)
        .call("labels", "&[\"Jan\", \"Feb\", \"Mar\", \"Apr\", \"May\", \"Jun\"]")
        .call("stacking", variant("Stacking", &STACKING, st))
        .call("curve", variant("Curve", &CURVES, c))
        .b("line", line)
        .b("dots", dots)
        .b("glow", glow)
        .f("height", height)
        .add()
}

fn bar(ui: &mut Ui, p: &mut Params) -> String {
    let horizontal = p.bool("Horizontal", false);
    let st = p.choice("Stacking", &STACKING, 0);
    let fill = p.choice("Fill", &["Solid", "Gradient", "Striped"], 0);
    let radius = p.f32("Corner radius", 6.0, 0.0..=16.0, 0.5);
    let values = p.bool("Value labels", false);
    let hover = p.bool("Highlight hover", true);
    let height = 320.0;
    let s = series();
    let mut chart = BarChart::new(&s)
        .labels(&MONTHS)
        .stacking(stacking(st))
        .fill([BarFill::Solid, BarFill::Gradient, BarFill::Striped][fill])
        .radius(radius)
        .value_labels(values)
        .highlight_hover(hover)
        .height(height);
    if horizontal {
        chart = chart.horizontal();
    }
    ui.add(chart);
    Code::new("glowkit::charts::{BarChart, BarFill, Series, Stacking}", "BarChart::new(&series)")
        .pre(SERIES_CODE)
        .call("labels", "&[\"Jan\", \"Feb\", \"Mar\", \"Apr\", \"May\", \"Jun\"]")
        .call_if(horizontal, "horizontal", "")
        .call("stacking", variant("Stacking", &STACKING, st))
        .call("fill", variant("BarFill", &["Solid", "Gradient", "Striped"], fill))
        .f("radius", radius)
        .b("value_labels", values)
        .b("highlight_hover", hover)
        .f("height", height)
        .add()
}

fn pie(ui: &mut Ui, p: &mut Params) -> String {
    let inner = p.f32("Inner radius (donut)", 0.6, 0.0..=0.9, 0.01);
    let pad = p.f32("Pad angle", 2.0, 0.0..=10.0, 0.5);
    let labels = p.choice("Labels", &["None", "Percent", "Value", "Outside"], 0);
    let rounded = p.bool("Rounded", true);
    let caption = p.text("Center caption", "Visitors");
    let pop = p.f32("Hover pop-out", 8.0, 0.0..=20.0, 0.5);
    let data = [
        Datum::new("Chrome", 275.0),
        Datum::new("Safari", 200.0),
        Datum::new("Firefox", 187.0),
        Datum::new("Edge", 173.0),
        Datum::new("Other", 90.0),
    ];
    let lab = [PieLabels::None, PieLabels::Percent, PieLabels::Value, PieLabels::Outside][labels];
    let mut chart =
        PieChart::new(&data).inner_radius(inner).pad_angle(pad).labels(lab).rounded(rounded).pop_out(pop).height(340.0);
    if inner > 0.0 && !caption.is_empty() {
        chart = chart.center_caption(&caption);
    }
    ui.add(chart);
    Code::new("glowkit::charts::{Datum, PieChart, PieLabels}", "PieChart::new(&data)")
        .pre("let data = [\n    Datum::new(\"Chrome\", 275.0),\n    Datum::new(\"Safari\", 200.0),\n    Datum::new(\"Firefox\", 187.0),\n    Datum::new(\"Edge\", 173.0),\n    Datum::new(\"Other\", 90.0),\n];")
        .f("inner_radius", inner)
        .f("pad_angle", pad)
        .call("labels", variant("PieLabels", &["None", "Percent", "Value", "Outside"], labels))
        .b("rounded", rounded)
        .f("pop_out", pop)
        .call_if(inner > 0.0 && !caption.is_empty(), "center_caption", lit_str(&caption))
        .call("height", "340.0")
        .add()
}

fn gauge(ui: &mut Ui, p: &mut Params) -> String {
    let value = p.f32("Value", 72.0, 0.0..=100.0, 0.5);
    let size = p.f32("Size", 300.0, 140.0..=420.0, 5.0);
    let thickness = p.f32("Thickness", 20.0, 4.0..=48.0, 0.5);
    let needle = p.bool("Needle", true);
    let ticks = p.bool("Ticks", true);
    let label = p.text("Label", "CPU load");
    let unit = String::from("%");
    ui.vertical_centered(|ui| {
        ui.add(
            Gauge::new(value as f64)
                .size(size)
                .thickness(thickness)
                .needle(needle)
                .ticks(ticks)
                .label(&label)
                .unit(&unit),
        )
    });
    Code::new("glowkit::charts::Gauge", format!("Gauge::new({})", lit_f32(value)))
        .f("size", size)
        .f("thickness", thickness)
        .b("needle", needle)
        .b("ticks", ticks)
        .s("label", &label)
        .s("unit", &unit)
        .add()
}

fn sparkline(ui: &mut Ui, p: &mut Params) -> String {
    let kind = p.choice("Kind", &["Line", "Area", "Bar"], 1);
    let color = p.color("Color", hex("#22C55E"));
    let w = p.f32("Width", 360.0, 120.0..=640.0, 5.0);
    let h = p.f32("Height", 80.0, 24.0..=200.0, 1.0);
    let dot = p.bool("End dot", true);
    let c = p.choice("Curve", &CURVES, 1);
    let prices = [12.0, 14.0, 13.0, 17.0, 16.0, 19.0, 18.0, 22.0, 21.0, 25.0, 24.0, 28.0];
    ui.vertical_centered(|ui| {
        ui.add_space(80.0);
        ui.add(
            Sparkline::new(&prices)
                .kind([SparkKind::Line, SparkKind::Area, SparkKind::Bar][kind])
                .color(color)
                .size(vec2(w, h))
                .end_dot(dot)
                .curve(curve(c)),
        );
    });
    Code::new("glowkit::charts::{Curve, SparkKind, Sparkline}", "Sparkline::new(&prices)")
        .pre("let prices = [12.0, 14.0, 13.0, 17.0, 16.0, 19.0, 18.0, 22.0, 21.0, 25.0, 24.0, 28.0];")
        .call("kind", variant("SparkKind", &["Line", "Area", "Bar"], kind))
        .c("color", color)
        .call("size", format!("vec2({}, {})", lit_f32(w), lit_f32(h)))
        .b("end_dot", dot)
        .call("curve", variant("Curve", &CURVES, c))
        .add()
}
