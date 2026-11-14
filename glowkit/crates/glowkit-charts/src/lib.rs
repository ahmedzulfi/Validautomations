//! # glowkit-charts
//!
//! Animated, theme-aware charts for egui: line, area, bar, combo, pie & donut, radar, radial
//! bars, gauges, sparklines, scatter & bubble, heatmaps, calendar heatmaps, funnels, bar lists,
//! treemaps, candlesticks, KPI tiles and trackers.
//!
//! Every chart is a builder implementing [`egui::Widget`]:
//!
//! ```no_run
//! # let ui: &mut egui::Ui = todo!();
//! use glowkit_charts::{LineChart, Series};
//! let series = [Series::new("Revenue", [4.2, 5.1, 4.8, 6.3, 7.0, 7.9])];
//! ui.add(LineChart::new(&series).labels(&["Jan", "Feb", "Mar", "Apr", "May", "Jun"]).height(260.0));
//! ```
//!
//! Charts play an entry animation once (lines draw in, bars grow, pies sweep) and then stop
//! repainting; value changes glide with a spring; legends toggle series; hovering shows a
//! crosshair and a tooltip card.

#![warn(missing_docs)]

mod bar;
mod cartesian;
mod common;
mod heatmap;
mod lists;
mod pie;
mod radial;
mod spark;
mod xy;

pub use bar::{BarChart, BarFill, ComboChart, Orientation};
pub use cartesian::{AreaChart, LineChart};
pub use common::{
    format_compact, format_percent, format_thousands, format_tick, nice_number, nice_ticks, palette, squarify, stack,
    Curve, Datum, Formatter, LineStyle, Series, Stacking, Ticks,
};
pub use heatmap::{CalendarHeatmap, Heatmap};
pub use lists::{BarList, FunnelChart, ProgressList, Treemap};
pub use pie::{DonutChart, PieChart, PieLabels};
pub use radial::{DonutProgress, Gauge, RadarChart, RadarGrid, RadialBarChart};
pub use spark::{SparkKind, Sparkline, StatTile, Status, Tracker, TrackerBlock};
pub use xy::{Candle, CandlestickChart, ScatterChart, ScatterSeries};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
