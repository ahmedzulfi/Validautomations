//! Sparklines and the KPI stat tile (SVG).

use crate::components::text::CountUp;
use dioxus::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Sparkline style.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SparkKind {
    /// A line with an end dot.
    #[default]
    Line,
    /// A line with a gradient fill.
    Area,
    /// Tiny columns (negative values in the destructive color).
    Bar,
}

/// A tiny chart without axes, for tables and cards. The line draws itself in, bars grow up.
///
/// Mirrors `glowkit_charts::Sparkline`: `data`, `kind`, `color` (default accent), `width`,
/// `height`, `animate`.
#[component]
pub fn Sparkline(
    data: Vec<f64>,
    #[props(default)] kind: SparkKind,
    #[props(into)] color: Option<String>,
    #[props(default = 120.0)] width: f32,
    #[props(default = 32.0)] height: f32,
    #[props(default = true)] animate: bool,
    /// Stretch to the parent's width (the `width` then only sets the aspect of the drawing).
    #[props(default)]
    fluid: bool,
) -> Element {
    let uid = use_hook(|| NEXT_ID.fetch_add(1, Ordering::Relaxed));
    let color = color.unwrap_or_else(|| "var(--gk-accent)".into());
    let (w, h) = (width as f64, height as f64);
    let n = data.len();
    let anim = if animate { "gk-spark gk-spark-anim" } else { "gk-spark" };
    let size_style = if fluid { "width:100%; height:auto;".to_string() } else { format!("width:{w}px; height:{h}px;") };
    if n == 0 {
        return rsx! { svg { class: anim, style: size_style, view_box: "0 0 {w} {h}" } };
    }
    let (mut lo, mut hi) = data.iter().fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
    if kind == SparkKind::Bar {
        lo = lo.min(0.0);
        hi = hi.max(0.0);
    }
    if (hi - lo).abs() < 1e-9 {
        hi = lo + 1.0;
    }
    let pad = 3.0;
    let y_of = |v: f64| pad + (1.0 - (v - lo) / (hi - lo)) * (h - pad * 2.0);
    match kind {
        SparkKind::Bar => {
            let slot = w / n as f64;
            let bw = (slot * 0.7).max(1.0);
            let zero = y_of(0.0);
            rsx! {
                svg { class: anim, style: size_style, view_box: "0 0 {w} {h}", preserve_aspect_ratio: "none",
                    for (i, v) in data.iter().enumerate() {
                        {
                            let y = y_of(*v);
                            let (top, bh) = if y < zero { (y, zero - y) } else { (zero, y - zero) };
                            let fill = if *v < 0.0 { "var(--gk-destructive)".to_string() } else { color.clone() };
                            rsx! {
                                rect { key: "{i}", class: "gk-spark-bar", x: "{i as f64 * slot + (slot - bw) / 2.0}", y: "{top}", width: "{bw}", height: "{bh.max(1.0)}",
                                    rx: "{(bw * 0.2).min(2.0)}", fill: "{fill}", style: "animation-delay:{i as f64 * 0.04}s; transform-origin: 0 {zero}px;" }
                            }
                        }
                    }
                }
            }
        }
        _ => {
            let pts: Vec<(f64, f64)> = data
                .iter()
                .enumerate()
                .map(|(i, v)| (pad + i as f64 / (n - 1).max(1) as f64 * (w - pad * 2.0), y_of(*v)))
                .collect();
            // smooth path (Catmull-Rom to cubic Bézier)
            let mut d = format!("M{:.2},{:.2}", pts[0].0, pts[0].1);
            for i in 0..pts.len() - 1 {
                let p0 = pts[i.saturating_sub(1)];
                let p1 = pts[i];
                let p2 = pts[i + 1];
                let p3 = pts[(i + 2).min(pts.len() - 1)];
                let c1 = (p1.0 + (p2.0 - p0.0) / 6.0, p1.1 + (p2.1 - p0.1) / 6.0);
                let c2 = (p2.0 - (p3.0 - p1.0) / 6.0, p2.1 - (p3.1 - p1.1) / 6.0);
                d.push_str(&format!(" C{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}", c1.0, c1.1, c2.0, c2.1, p2.0, p2.1));
            }
            let last = *pts.last().unwrap_or(&(0.0, 0.0));
            let area = format!("{d} L{:.2},{h} L{:.2},{h} Z", last.0, pts[0].0);
            let gid = format!("gk-spark-grad-{uid}");
            rsx! {
                svg { class: anim, style: size_style, view_box: "0 0 {w} {h}", overflow: "visible",
                    defs {
                        linearGradient { id: "{gid}", x1: "0", y1: "0", x2: "0", y2: "1",
                            stop { offset: "0%", stop_color: "{color}", stop_opacity: "0.32" }
                            stop { offset: "100%", stop_color: "{color}", stop_opacity: "0" }
                        }
                    }
                    if kind == SparkKind::Area {
                        path { class: "gk-spark-area", d: "{area}", fill: "url(#{gid})" }
                    }
                    path { class: "gk-spark-line", d: "{d}", fill: "none", stroke: "{color}", stroke_width: "2",
                        stroke_linecap: "round", stroke_linejoin: "round", "pathLength": "1" }
                    circle { class: "gk-spark-dot", cx: "{last.0}", cy: "{last.1}", r: "3", fill: "{color}" }
                }
            }
        }
    }
}

/// A KPI card: title, big number that counts up, a colored delta badge and a sparkline.
///
/// Mirrors `glowkit_charts::StatTile`: `title`, `value`, `prefix`, `suffix`, `decimals`,
/// `delta` (percent; ▲ good / ▼ bad), `invert_delta` (a decrease is good), `caption`, `spark`,
/// `kind`, `color` (default: success/destructive by trend), `width`/`height`, `animate`.
#[component]
pub fn StatTile(
    #[props(into)] title: String,
    value: f64,
    #[props(into, default)] prefix: String,
    #[props(into, default)] suffix: String,
    #[props(default)] decimals: usize,
    delta: Option<f64>,
    #[props(default)] invert_delta: bool,
    #[props(into, default)] caption: String,
    #[props(default)] spark: Vec<f64>,
    #[props(default = SparkKind::Area)] kind: SparkKind,
    #[props(into)] color: Option<String>,
    #[props(default = 260.0)] width: f32,
    #[props(default = 170.0)] height: f32,
    #[props(default = true)] animate: bool,
) -> Element {
    let good = delta.map(|d| (d >= 0.0) != invert_delta);
    let trend = match good {
        Some(true) => "var(--gk-success)",
        Some(false) => "var(--gk-destructive)",
        None => "var(--gk-accent)",
    };
    let color = color.unwrap_or_else(|| trend.to_string());
    let badge = delta.map(|d| {
        let arrow = if d >= 0.0 { "▲" } else { "▼" };
        (format!("{arrow} {:.1}%", d.abs()), if good == Some(true) { "gk-delta gk-delta-good" } else { "gk-delta gk-delta-bad" })
    });
    let duration = if animate { 1.6 } else { 0.001 };
    rsx! {
        div { class: "gk-stat", style: "width:{width}px; min-height:{height}px;",
            div { class: "gk-stat-title", "{title}" }
            div { class: "gk-stat-value",
                CountUp { to: value, decimals, prefix: prefix.clone(), suffix: suffix.clone(), size: 26.0, duration, bold: false }
            }
            div { class: "gk-stat-row",
                if let Some((t, c)) = badge { span { class: c, "{t}" } }
                if !caption.is_empty() { span { class: "gk-stat-caption", "{caption}" } }
            }
            if !spark.is_empty() {
                div { class: "gk-stat-spark",
                    Sparkline { data: spark.clone(), kind, color, width: width - 32.0, height: 46.0, animate, fluid: true }
                }
            }
        }
    }
}
