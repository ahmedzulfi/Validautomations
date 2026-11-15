use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Rect, Response, Sense, Stroke, Ui, Widget};
use glowkit_ai::Icon;
use glowkit_charts::{AreaChart, BarChart, Curve, LineChart, Series};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// Column alignment / formatting in a [`DataTableResult`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColumnKind {
    /// Left-aligned text.
    #[default]
    Text,
    /// Right-aligned number (sorted numerically).
    Number,
    /// Right-aligned signed change: `+` values are green, `-` values red.
    Delta,
}

#[derive(Clone, Copy, Default)]
struct SortState {
    col: Option<usize>,
    desc: bool,
    changed_at: f64,
}

/// A tool result rendered as a compact data table: a calm card with the tool header, sortable
/// columns (click a header: ascending, descending, off), rows that slide in one after another,
/// hover highlighting, right-aligned numbers and colored deltas, and a footer with a row count.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{ColumnKind, DataTableResult};
/// let cols = [("Ticker", ColumnKind::Text), ("Price", ColumnKind::Number), ("Day", ColumnKind::Delta)];
/// let rows = vec![vec!["ACME".into(), "182.40".into(), "+1.2%".into()]];
/// ui.add(DataTableResult::new("query_stocks", &cols, &rows));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DataTableResult<'a> {
    tool: &'a str,
    subtitle: Option<&'a str>,
    cols: &'a [(&'a str, ColumnKind)],
    rows: &'a [Vec<String>],
    width: Option<f32>,
    max_rows: usize,
    salt: Option<egui::IdSalt>,
}

impl<'a> DataTableResult<'a> {
    /// A table produced by `tool` with columns `cols` and string `rows`.
    pub fn new(tool: &'a str, cols: &'a [(&'a str, ColumnKind)], rows: &'a [Vec<String>]) -> Self {
        Self { tool, subtitle: None, cols, rows, width: None, max_rows: 8, salt: None }
    }
    /// Second line under the tool name (e.g. the query).
    pub fn subtitle(mut self, s: &'a str) -> Self {
        self.subtitle = Some(s);
        self
    }
    /// Card width (default: available width, at most 560).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Rows shown before "+ n more" (default 8).
    pub fn max_rows(mut self, n: usize) -> Self {
        self.max_rows = n.max(1);
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }
}

fn num(s: &str) -> f64 {
    let c: String = s.chars().filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-').collect();
    c.parse().unwrap_or(0.0)
}

impl Widget for DataTableResult<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_table");
        let width = self.width.unwrap_or_else(|| ui.available_width().min(560.0));
        let key = self.rows.len() as u64 ^ self.rows.first().map(|r| hash_str(&r.concat())).unwrap_or(0);
        let e = since_change(ui, id, key);
        keep_alive(ui, e, 1.6);
        let now = ui.input(|i| i.time);
        let sort: SortState = glowkit_core::state(ui, id.with("sort"), SortState::default);
        let mut order: Vec<usize> = (0..self.rows.len()).collect();
        if let Some(c) = sort.col {
            let kind = self.cols.get(c).map(|c| c.1).unwrap_or_default();
            order.sort_by(|&a, &b| {
                let (x, y) = (&self.rows[a][c], &self.rows[b][c]);
                let o = if kind == ColumnKind::Text {
                    x.cmp(y)
                } else {
                    num(x).partial_cmp(&num(y)).unwrap_or(std::cmp::Ordering::Equal)
                };
                if sort.desc {
                    o.reverse()
                } else {
                    o
                }
            });
        }
        let sort_e = (now - sort.changed_at) as f32;
        if sort_e < 0.6 {
            ui.ctx().request_repaint();
        }
        let status = format!("{} rows", self.rows.len());
        let head = ToolHead { icon: Icon::Search, title: self.tool, subtitle: self.subtitle, status: Some((&status, t.success)) };
        let cols = self.cols;
        let rows = self.rows;
        let max_rows = self.max_rows;
        let mut new_sort = None;
        let rect = tool_card(ui, appear_curve(e), Some(head), width, |ui| {
            let w = ui.available_width();
            // column widths: text columns share the remaining space
            let fixed: f32 = cols.iter().filter(|c| c.1 != ColumnKind::Text).count() as f32 * 84.0;
            let ntext = cols.iter().filter(|c| c.1 == ColumnKind::Text).count().max(1) as f32;
            let text_w = ((w - fixed) / ntext).max(60.0);
            let cw: Vec<f32> = cols.iter().map(|c| if c.1 == ColumnKind::Text { text_w } else { 84.0 }).collect();
            let row_h = 32.0;
            let (hr, _) = ui.allocate_exact_size(vec2(w, 30.0), Sense::hover());
            let p = ui.painter().clone();
            p.rect_filled(hr, cr(8.0), well(&t));
            let mut x = hr.min.x;
            for (ci, (name, kind)) in cols.iter().enumerate() {
                let cell = Rect::from_min_size(pos2(x, hr.min.y), vec2(cw[ci], hr.height()));
                let r = ui.interact(cell, id.with(("h", ci)), Sense::click());
                let hk = toggle(ui, id.with(("hh", ci)), r.hovered());
                let active = sort.col == Some(ci);
                let col = mix(t.muted_foreground, t.foreground, if active { 1.0 } else { hk });
                let right = *kind != ColumnKind::Text;
                let tx = if right { cell.max.x - 26.0 } else { cell.min.x + 10.0 };
                let lr = text(&p, pos2(tx, cell.center().y), if right { Align2::RIGHT_CENTER } else { Align2::LEFT_CENTER }, name, 11.5, col);
                if active || hk > 0.05 {
                    let ic = if active && sort.desc { Icon::ChevronDown } else { Icon::ChevronUp };
                    let a = if active { 1.0 } else { hk * 0.6 };
                    icon(&p, ic, pos2(lr.max.x + 9.0, cell.center().y), 11.0, with_alpha(col, a));
                }
                if r.clicked() {
                    new_sort = Some(ci);
                }
                x += cw[ci];
            }
            for (k, &ri) in order.iter().take(max_rows).enumerate() {
                let (rr, resp) = ui.allocate_exact_size(vec2(w, row_h), Sense::hover());
                let k_in = ((e - 0.18 - k as f32 * 0.055) * 5.0).clamp(0.0, 1.0);
                let k_sort = if sort_e < 0.6 { ((sort_e - k as f32 * 0.025) * 6.0).clamp(0.0, 1.0) } else { 1.0 };
                let a = k_in * (0.4 + 0.6 * k_sort);
                let dy = (1.0 - k_in) * 8.0 + (1.0 - k_sort) * 4.0;
                let hk = toggle(ui, id.with(("rh", ri)), resp.hovered());
                if hk > 0.01 {
                    p.rect_filled(rr.shrink2(vec2(0.0, 1.0)), cr(8.0), with_alpha(well(&t), hk * 0.8));
                }
                if k > 0 {
                    p.line_segment(
                        [pos2(rr.min.x + 8.0, rr.min.y), pos2(rr.max.x - 8.0, rr.min.y)],
                        Stroke::new(1.0, with_alpha(t.border, 0.6 * a)),
                    );
                }
                let mut x = rr.min.x;
                for (ci, (_, kind)) in cols.iter().enumerate() {
                    let s = rows[ri].get(ci).map(|s| s.as_str()).unwrap_or("");
                    let cell = Rect::from_min_size(pos2(x, rr.min.y + dy), vec2(cw[ci], row_h));
                    match kind {
                        ColumnKind::Text => {
                            let c = if ci == 0 { t.foreground } else { t.muted_foreground };
                            let tx = pos2(cell.min.x + 10.0, cell.center().y);
                            if ci == 0 {
                                text_bold(&p, tx, Align2::LEFT_CENTER, s, 12.5, with_alpha(c, a));
                            } else {
                                text(&p, tx, Align2::LEFT_CENTER, s, 12.5, with_alpha(c, a));
                            }
                        }
                        ColumnKind::Number => {
                            mono(&p, pos2(cell.max.x - 26.0, cell.center().y), Align2::RIGHT_CENTER, s, 12.0, with_alpha(t.foreground, a));
                        }
                        ColumnKind::Delta => {
                            let up = !s.starts_with('-');
                            let c = if up { t.success } else { t.destructive };
                            let g = galley(ui, s, 11.5, c);
                            let pill = Rect::from_min_size(
                                pos2(cell.max.x - 22.0 - g.size().x - 12.0, cell.center().y - 10.0),
                                vec2(g.size().x + 12.0, 20.0),
                            );
                            p.rect_filled(pill, cr(10.0), with_alpha(c, (if t.dark { 0.16 } else { 0.09 }) * a));
                            mono(&p, pill.center(), Align2::CENTER_CENTER, s, 11.0, with_alpha(c, a));
                        }
                    }
                    x += cw[ci];
                }
            }
            if rows.len() > max_rows {
                let (fr, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
                text(
                    &p,
                    pos2(fr.min.x + 10.0, fr.center().y),
                    Align2::LEFT_CENTER,
                    &format!("+ {} more rows", rows.len() - max_rows),
                    11.5,
                    t.muted_foreground,
                );
            }
        });
        if let Some(ci) = new_sort {
            let s = match (sort.col, sort.desc) {
                (Some(c), false) if c == ci => SortState { col: Some(ci), desc: true, changed_at: now },
                (Some(c), true) if c == ci => SortState { col: None, desc: false, changed_at: now },
                _ => SortState { col: Some(ci), desc: false, changed_at: now },
            };
            glowkit_core::set_state(ui, id.with("sort"), s);
        }
        ui.interact(rect, id.with("card"), Sense::hover())
    }
}

/// Chart flavour of a [`ChartResult`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ChartKind {
    /// Filled area.
    #[default]
    Area,
    /// Plain lines.
    Line,
    /// Grouped bars.
    Bar,
}

/// A tool result that answers with a chart: the tool header, a headline value with a colored
/// change pill, and an animated glowkit chart (area, line or bars) that draws itself in when the
/// card appears.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{ChartKind, ChartResult};
/// use glowkit_charts::Series;
/// let s = [Series::new("Signups", vec![12.0, 18.0, 16.0, 24.0, 31.0])];
/// ui.add(ChartResult::new("analytics.signups", &s).headline("31", "+29%").kind(ChartKind::Area));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ChartResult<'a> {
    tool: &'a str,
    series: &'a [Series],
    labels: Vec<String>,
    headline: Option<(&'a str, &'a str)>,
    caption: Option<&'a str>,
    kind: ChartKind,
    width: Option<f32>,
    height: f32,
    salt: Option<egui::IdSalt>,
}

impl<'a> ChartResult<'a> {
    /// A chart of `series` produced by `tool`.
    pub fn new(tool: &'a str, series: &'a [Series]) -> Self {
        Self {
            tool,
            series,
            labels: Vec::new(),
            headline: None,
            caption: None,
            kind: ChartKind::Area,
            width: None,
            height: 170.0,
            salt: None,
        }
    }
    /// X-axis labels.
    pub fn labels<S: AsRef<str>>(mut self, labels: &[S]) -> Self {
        self.labels = labels.iter().map(|s| s.as_ref().to_owned()).collect();
        self
    }
    /// Big headline value and its change (a leading `-` paints it red).
    pub fn headline(mut self, value: &'a str, change: &'a str) -> Self {
        self.headline = Some((value, change));
        self
    }
    /// Muted caption under the headline.
    pub fn caption(mut self, c: &'a str) -> Self {
        self.caption = Some(c);
        self
    }
    /// Chart flavour.
    pub fn kind(mut self, k: ChartKind) -> Self {
        self.kind = k;
        self
    }
    /// Card width (default: available, at most 560).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Chart height (default 170).
    pub fn chart_height(mut self, h: f32) -> Self {
        self.height = h.max(80.0);
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }
}

impl Widget for ChartResult<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_chart");
        let width = self.width.unwrap_or_else(|| ui.available_width().min(560.0));
        let key = self.series.iter().map(|s| s.values.len() as u64).sum::<u64>() ^ ((self.kind as u64) << 32);
        let e = since_change(ui, id, key);
        keep_alive(ui, e, 1.0);
        let head = ToolHead { icon: Icon::Bolt, title: self.tool, subtitle: None, status: Some(("done", t.success)) };
        let rect = tool_card(ui, appear_curve(e), Some(head), width, |ui| {
            if let Some((v, ch)) = self.headline {
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::hover());
                let p = ui.painter();
                let vr = text_bold(p, pos2(r.min.x, r.min.y + 18.0), Align2::LEFT_CENTER, v, 26.0, t.foreground);
                let up = !ch.starts_with('-');
                let c = if up { t.success } else { t.destructive };
                let tw = text_w(ui, ch, 11.5);
                let pill = Rect::from_min_size(pos2(vr.max.x + 10.0, vr.center().y - 10.0), vec2(tw + 26.0, 20.0));
                p.rect_filled(pill, cr(10.0), with_alpha(c, if t.dark { 0.16 } else { 0.09 }));
                icon(p, if up { Icon::ArrowUp } else { Icon::ArrowDown }, pos2(pill.min.x + 10.0, pill.center().y), 10.0, c);
                text(p, pos2(pill.min.x + 18.0, pill.center().y), Align2::LEFT_CENTER, ch, 11.5, c);
                if let Some(cap) = self.caption {
                    text(p, pos2(r.min.x, r.max.y - 4.0), Align2::LEFT_CENTER, cap, 11.5, t.muted_foreground);
                }
            }
            if e > 0.15 {
                let w = ui.available_width();
                match self.kind {
                    ChartKind::Area => ui.add(
                        AreaChart::new(self.series)
                            .labels(&self.labels)
                            .curve(Curve::Monotone)
                            .height(self.height)
                            .width(w)
                            .legend(self.series.len() > 1)
                            .id_salt(id.with("c")),
                    ),
                    ChartKind::Line => ui.add(
                        LineChart::new(self.series)
                            .labels(&self.labels)
                            .dots(true)
                            .height(self.height)
                            .width(w)
                            .legend(self.series.len() > 1)
                            .id_salt(id.with("c")),
                    ),
                    ChartKind::Bar => ui.add(
                        BarChart::new(self.series)
                            .labels(&self.labels)
                            .radius(4.0)
                            .height(self.height)
                            .width(w)
                            .legend(self.series.len() > 1)
                            .id_salt(id.with("c")),
                    ),
                };
            } else {
                ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
            }
        });
        let _ = Color32::WHITE;
        ui.interact(rect, id.with("card"), Sense::hover())
    }
}
