//! Funnel, bar list, progress list and treemap.

use crate::common::*;
use egui::epaint::Mesh;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::anim::{ease, stagger, Easing};
use glowkit_core::color::{contrast_text, mix, shade, with_alpha};
use glowkit_core::Theme;

// ---------------------------------------------------------------------------------------------
// FunnelChart
// ---------------------------------------------------------------------------------------------

/// A conversion funnel: centered stage bars connected by smooth tapered bands, with the stage
/// name on the left and the value plus conversion rate on the right.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::{FunnelChart, Datum};
/// let d = [Datum::new("Visited", 12_400.0), Datum::new("Signed up", 4_210.0), Datum::new("Paid", 980.0)];
/// ui.add(FunnelChart::new(&d));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct FunnelChart<'a> {
    data: &'a [Datum],
    base: Base,
    color: Option<Color32>,
    gap: f32,
    multicolor: bool,
}

impl<'a> FunnelChart<'a> {
    /// A funnel of `data` stages, widest first.
    pub fn new(data: &'a [Datum]) -> Self {
        Self { data, base: Base::new(300.0), color: None, gap: 0.42, multicolor: false }
    }
    base_setters!();
    /// Base color (stages get progressively lighter). Defaults to the accent.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Height of the connector between stages as a fraction of a stage row (default 0.42).
    pub fn connector(mut self, f: f32) -> Self {
        self.gap = f.clamp(0.05, 0.8);
        self
    }
    /// Use a palette color per stage instead of shades of one color.
    pub fn multicolor(mut self, on: bool) -> Self {
        self.multicolor = on;
        self
    }
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

impl Widget for FunnelChart<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp, id) = self.base.allocate(ui, Sense::hover());
        let n = self.data.len();
        if n == 0 || !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter_at(rect);
        let font = FontId::proportional(12.0);
        let left_w = self.data.iter().map(|d| text_width(&painter, &d.label, &font)).fold(0.0, f32::max) + 16.0;
        let right_w = 96.0;
        let area = Rect::from_min_max(pos2(rect.left() + left_w, rect.top()), pos2(rect.right() - right_w, rect.bottom()));
        let row = area.height() / (n as f32 + (n as f32 - 1.0) * self.gap);
        let conn = row * self.gap;
        let max = self.data.iter().fold(0.0f64, |m, d| m.max(d.value)).max(1e-9);
        let targets: Vec<f32> = self.data.iter().map(|d| (d.value / max) as f32).collect();
        let cur = tween(ui, id, &targets);
        let e = entry_time(ui, id, self.base.animate, 0.6 + n as f32 * 0.12 + 0.4);
        let base_col = self.color.unwrap_or(t.accent);
        let col = |i: usize| -> Color32 {
            if let Some(c) = self.data[i].color {
                return c;
            }
            if self.multicolor {
                palette(&t, i)
            } else {
                let k = i as f32 / (n.max(2) - 1) as f32;
                if t.dark { mix(base_col, shade(base_col, 0.18), k).gamma_multiply(1.0 - 0.35 * k) } else { mix(base_col, shade(base_col, 0.2), k) }
            }
        };
        let hover = resp.hover_pos().filter(|p| rect.contains(*p)).map(|p| (((p.y - area.top()) / (row + conn)).floor().max(0.0) as usize).min(n - 1));
        let halpha = glowkit_core::anim::follow(ui, id.with("hover_alpha"), if hover.is_some() { 1.0 } else { 0.0 }, 16.0);
        let cx = area.center().x;
        let widths: Vec<f32> = (0..n).map(|i| area.width() * cur[i].max(0.02) * ease(Easing::OutCubic, stagger(e, i, 0.12, 0.6))).collect();
        for i in 0..n {
            let y = area.top() + i as f32 * (row + conn);
            let w = widths[i];
            let dim = if hover.is_some() && hover != Some(i) { 1.0 - 0.45 * halpha } else { 1.0 };
            let c = col(i).gamma_multiply(dim);
            let r = Rect::from_center_size(pos2(cx, y + row / 2.0), vec2(w.max(2.0), row));
            painter.rect_filled(r, CornerRadius::same((row * 0.18).min(8.0) as u8), c);
            // connector to the next stage
            if i + 1 < n {
                let w2 = widths[i + 1];
                let (y0, y1) = (y + row, y + row + conn);
                let steps = 16;
                let mut m = Mesh::default();
                let c0 = with_alpha(col(i), 0.32 * dim);
                let c1 = with_alpha(col(i + 1), 0.18 * dim);
                for s in 0..=steps {
                    let k = s as f32 / steps as f32;
                    let hw = (w + (w2 - w) * smooth(k)) / 2.0;
                    let yy = y0 + (y1 - y0) * k;
                    let cc = c0.lerp_to_gamma(c1, k);
                    m.colored_vertex(pos2(cx - hw, yy), cc);
                    m.colored_vertex(pos2(cx + hw, yy), cc);
                }
                for s in 0..steps as u32 {
                    let a = s * 2;
                    m.add_triangle(a, a + 1, a + 3);
                    m.add_triangle(a, a + 3, a + 2);
                }
                painter.add(Shape::mesh(m));
                // drop-off label in the connector
                let conv = if self.data[i].value > 0.0 { self.data[i + 1].value / self.data[i].value } else { 0.0 };
                if conn > 14.0 && e > 0.8 {
                    painter.text(pos2(cx, (y0 + y1) / 2.0), Align2::CENTER_CENTER, format!("↓ {}", format_percent(conv)), FontId::proportional(10.5), t.muted_foreground);
                }
            }
            let hl = hover == Some(i);
            painter.text(pos2(rect.left(), y + row / 2.0), Align2::LEFT_CENTER, &self.data[i].label, font.clone(), if hl { t.foreground } else { t.muted_foreground.lerp_to_gamma(t.foreground, 0.4) });
            if w > 60.0 {
                painter.text(r.center(), Align2::CENTER_CENTER, (self.base.format)(self.data[i].value), FontId::proportional(12.5), contrast_text(col(i)));
            }
            let of_top = self.data[i].value / self.data[0].value.max(1e-9);
            painter.text(pos2(rect.right(), y + row / 2.0 - 7.0), Align2::RIGHT_CENTER, format_percent(of_top), FontId::proportional(13.0), t.foreground);
            painter.text(pos2(rect.right(), y + row / 2.0 + 8.0), Align2::RIGHT_CENTER, "of total", FontId::proportional(10.5), t.muted_foreground);
        }
        if let (Some(i), true) = (hover, self.base.tooltip) {
            let mut rows = vec![TipRow::new(Some(col(i)), "Count", (self.base.format)(self.data[i].value))];
            if i > 0 {
                rows.push(TipRow::new(None, "From previous", format_percent(self.data[i].value / self.data[i - 1].value.max(1e-9))));
            }
            rows.push(TipRow::new(None, "Of total", format_percent(self.data[i].value / self.data[0].value.max(1e-9))));
            tooltip(ui, id, resp.hover_pos().unwrap_or(rect.center()), &self.data[i].label, &rows, halpha);
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// BarList
// ---------------------------------------------------------------------------------------------

/// A ranked list of horizontal bars with the label inside each bar and the value on the right
/// (Tremor-style bar list): top pages, referrers, countries...
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::{BarList, Datum};
/// let d = [Datum::new("/home", 1230.0), Datum::new("/pricing", 751.0), Datum::new("/blog", 471.0)];
/// ui.add(BarList::new(&d).sorted(true));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BarList<'a> {
    data: &'a [Datum],
    base: Base,
    color: Option<Color32>,
    row_h: f32,
    sorted: bool,
    share: bool,
}

impl<'a> BarList<'a> {
    /// A bar list of `data`.
    pub fn new(data: &'a [Datum]) -> Self {
        let mut base = Base::new(0.0);
        base.legend = false;
        Self { data, base, color: None, row_h: 32.0, sorted: false, share: false }
    }
    /// Width in points (defaults to all available width).
    pub fn width(mut self, w: f32) -> Self {
        self.base.width = Some(w);
        self
    }
    /// Value formatter.
    pub fn format(mut self, f: Formatter) -> Self {
        self.base.format = f;
        self
    }
    /// Entry animation (default `true`).
    pub fn animate(mut self, on: bool) -> Self {
        self.base.animate = on;
        self
    }
    /// Bar color (default: the theme accent, tinted).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Row height (default 32).
    pub fn row_height(mut self, h: f32) -> Self {
        self.row_h = h;
        self
    }
    /// Sort descending by value.
    pub fn sorted(mut self, on: bool) -> Self {
        self.sorted = on;
        self
    }
    /// Show each row's share of the total next to its value.
    pub fn share(mut self, on: bool) -> Self {
        self.share = on;
        self
    }
}

impl Widget for BarList<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.data.len();
        let gap = 6.0;
        let mut base = self.base.clone();
        base.height = n as f32 * (self.row_h + gap) - gap;
        let (rect, resp, id) = base.allocate(ui, Sense::hover());
        if n == 0 || !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter_at(rect.expand(2.0));
        let mut order: Vec<usize> = (0..n).collect();
        if self.sorted {
            order.sort_by(|a, b| self.data[*b].value.partial_cmp(&self.data[*a].value).unwrap_or(std::cmp::Ordering::Equal));
        }
        let font = FontId::proportional(12.5);
        let total: f64 = self.data.iter().map(|d| d.value.max(0.0)).sum();
        let vals: Vec<String> = order.iter().map(|&i| {
            let v = (self.base.format)(self.data[i].value);
            if self.share { format!("{v}  {}", format_percent(self.data[i].value / total.max(1e-9))) } else { v }
        }).collect();
        let val_w = vals.iter().map(|v| text_width(&painter, v, &font)).fold(0.0, f32::max) + 16.0;
        let bar_area = rect.width() - val_w;
        let max = self.data.iter().fold(0.0f64, |m, d| m.max(d.value)).max(1e-9);
        let targets: Vec<f32> = order.iter().map(|&i| (self.data[i].value / max) as f32).collect();
        let cur = tween(ui, id, &targets);
        let e = entry_time(ui, id, self.base.animate, 0.7 + 0.05 * n as f32);
        let hover = resp.hover_pos().map(|p| (((p.y - rect.top()) / (self.row_h + gap)).floor().max(0.0) as usize).min(n - 1));
        for (row, &i) in order.iter().enumerate() {
            let d = &self.data[i];
            let y = rect.top() + row as f32 * (self.row_h + gap);
            let g = ease(Easing::OutCubic, stagger(e, row, 0.05, 0.6));
            let w = (bar_area * cur[row] * g).max(4.0);
            let base_c = d.color.or(self.color).unwrap_or(t.accent);
            let hl = hover == Some(row);
            let hov = glowkit_core::anim::follow(ui, id.with(("hov", row)), if hl { 1.0 } else { 0.0 }, 16.0);
            if hov > 0.01 {
                painter.rect_filled(Rect::from_min_size(pos2(rect.left() - 4.0, y - 2.0), vec2(rect.width() + 8.0, self.row_h + 4.0)), CornerRadius::same(8), t.muted.gamma_multiply(hov * 0.8));
            }
            let fill = with_alpha(base_c, if t.dark { 0.24 + 0.1 * hov } else { 0.18 + 0.08 * hov });
            let r = Rect::from_min_size(pos2(rect.left(), y), vec2(w, self.row_h));
            painter.rect_filled(r, CornerRadius::same(6), fill);
            painter.rect_filled(Rect::from_min_size(r.min, vec2(3.0, self.row_h)), CornerRadius { nw: 6, sw: 6, ne: 0, se: 0 }, base_c.gamma_multiply(0.9));
            painter.text(pos2(rect.left() + 12.0, y + self.row_h / 2.0), Align2::LEFT_CENTER, &d.label, font.clone(), t.foreground);
            painter.text(pos2(rect.right(), y + self.row_h / 2.0), Align2::RIGHT_CENTER, &vals[row], font.clone(), if hl { t.foreground } else { t.foreground.gamma_multiply(0.85) });
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// ProgressList
// ---------------------------------------------------------------------------------------------

/// Labelled progress bars stacked in a list: name and "value / target" on top, a rounded
/// gradient bar below (quotas, budgets, goals).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::{ProgressList, Datum};
/// let d = [Datum::new("Marketing", 42_000.0), Datum::new("Engineering", 118_000.0)];
/// ui.add(ProgressList::new(&d, &[60_000.0, 150_000.0]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ProgressList<'a> {
    data: &'a [Datum],
    targets: &'a [f64],
    width: Option<f32>,
    format: Formatter,
    animate: bool,
    thickness: f32,
}

impl<'a> ProgressList<'a> {
    /// Progress rows: each datum's value against the matching target.
    pub fn new(data: &'a [Datum], targets: &'a [f64]) -> Self {
        Self { data, targets, width: None, format: format_compact, animate: true, thickness: 8.0 }
    }
    /// Width (defaults to all available width).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Value formatter.
    pub fn format(mut self, f: Formatter) -> Self {
        self.format = f;
        self
    }
    /// Entry animation (default `true`).
    pub fn animate(mut self, on: bool) -> Self {
        self.animate = on;
        self
    }
    /// Bar thickness (default 8).
    pub fn thickness(mut self, th: f32) -> Self {
        self.thickness = th;
        self
    }
}

impl Widget for ProgressList<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.data.len();
        let row = 26.0 + self.thickness + 14.0;
        let w = self.width.unwrap_or_else(|| ui.available_width());
        let (rect, resp) = ui.allocate_exact_size(vec2(w, (n as f32 * row - 14.0).max(0.0)), Sense::hover());
        if n == 0 || !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        let painter = ui.painter_at(rect.expand(4.0));
        let e = entry_time(ui, id, self.animate, 0.8 + 0.08 * n as f32);
        let fracs: Vec<f32> = self.data.iter().enumerate().map(|(i, d)| (d.value / self.targets.get(i).copied().unwrap_or(d.value).max(1e-9)) as f32).collect();
        let cur = tween(ui, id, &fracs);
        let font = FontId::proportional(12.5);
        for (i, d) in self.data.iter().enumerate() {
            let y = rect.top() + i as f32 * row;
            let c = d.color.unwrap_or_else(|| palette(&t, i));
            let target = self.targets.get(i).copied().unwrap_or(d.value);
            painter.text(pos2(rect.left(), y + 9.0), Align2::LEFT_CENTER, &d.label, font.clone(), t.foreground);
            let over = cur[i] > 1.0;
            let pct = format_percent(cur[i] as f64);
            let pr = painter.text(pos2(rect.right(), y + 9.0), Align2::RIGHT_CENTER, &pct, font.clone(), if over { t.destructive } else { t.foreground });
            painter.text(pos2(pr.left() - 10.0, y + 9.0), Align2::RIGHT_CENTER, format!("{} / {}", (self.format)(d.value), (self.format)(target)), FontId::proportional(11.5), t.muted_foreground);
            let track = Rect::from_min_size(pos2(rect.left(), y + 24.0), vec2(rect.width(), self.thickness));
            let cr = CornerRadius::same((self.thickness / 2.0) as u8);
            painter.rect_filled(track, cr, with_alpha(t.muted_foreground, if t.dark { 0.12 } else { 0.14 }));
            let g = ease(Easing::OutCubic, stagger(e, i, 0.08, 0.8));
            let fw = track.width() * cur[i].min(1.0) * g;
            if fw > 0.5 {
                let fr = Rect::from_min_size(track.min, vec2(fw.max(self.thickness), self.thickness));
                let pts = rounded_points(fr, [self.thickness / 2.0; 4], 6);
                let stops = [shade(c, if t.dark { -0.08 } else { 0.06 }), c];
                glowkit_core::paint::fill_convex_fn(&painter, &pts, 2, glowkit_core::paint::linear_fn(fr, 0.0, &stops));
            }
        }
        let _ = Pos2::ZERO;
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Treemap
// ---------------------------------------------------------------------------------------------

/// A squarified treemap: tile area proportional to value, labels when they fit, tiles scale in
/// on appear, hover brightens a tile and shows its value and share.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_charts::{Treemap, Datum};
/// let d = [Datum::new("Compute", 42.0), Datum::new("Storage", 21.0), Datum::new("Network", 12.0)];
/// ui.add(Treemap::new(&d).height(300.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Treemap<'a> {
    data: &'a [Datum],
    base: Base,
    gap: f32,
    radius: f32,
    color: Option<Color32>,
}

impl<'a> Treemap<'a> {
    /// A treemap of `data`.
    pub fn new(data: &'a [Datum]) -> Self {
        Self { data, base: Base::new(300.0), gap: 3.0, radius: 6.0, color: None }
    }
    base_setters!();
    /// Gap between tiles (default 3).
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g;
        self
    }
    /// Tile corner radius (default 6).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r;
        self
    }
    /// Use one color with intensity by value instead of the palette.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

impl Widget for Treemap<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp, id) = self.base.allocate(ui, Sense::hover());
        let n = self.data.len();
        if n == 0 || !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter_at(rect);
        let targets: Vec<f32> = self.data.iter().map(|d| d.value.max(0.0) as f32).collect();
        let cur = tween(ui, id, &targets);
        let vals: Vec<f64> = cur.iter().map(|v| *v as f64).collect();
        let tiles = squarify(&vals, rect.expand(self.gap / 2.0));
        let total: f64 = self.data.iter().map(|d| d.value.max(0.0)).sum::<f64>().max(1e-9);
        let max = self.data.iter().fold(0.0f64, |m, d| m.max(d.value)).max(1e-9);
        let e = entry_time(ui, id, self.base.animate, 1.4);
        // rank for stagger (largest first)
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|a, b| self.data[*b].value.partial_cmp(&self.data[*a].value).unwrap_or(std::cmp::Ordering::Equal));
        let mut rank = vec![0; n];
        for (r, &i) in order.iter().enumerate() {
            rank[i] = r;
        }
        let hover = resp.hover_pos().and_then(|p| tiles.iter().position(|r| r.contains(p)));
        let halpha = glowkit_core::anim::follow(ui, id.with("hover_alpha"), if hover.is_some() { 1.0 } else { 0.0 }, 16.0);
        for (i, d) in self.data.iter().enumerate() {
            let full = tiles[i].shrink(self.gap / 2.0);
            if full.width() < 1.0 || full.height() < 1.0 {
                continue;
            }
            let g = ease(Easing::OutBack, stagger(e, rank[i], 0.06, 0.6));
            let r = Rect::from_center_size(full.center(), full.size() * (0.85 + 0.15 * g));
            let base_c = match (d.color, self.color) {
                (Some(c), _) => c,
                (None, Some(c)) => mix(mix(t.card, c, 0.35), c, (d.value / max) as f32),
                (None, None) => palette(&t, i),
            };
            let hl = hover == Some(i);
            let hov = glowkit_core::anim::follow(ui, id.with(("hov", i)), if hl { 1.0 } else { 0.0 }, 18.0);
            let dim = if hover.is_some() && !hl { 1.0 - 0.3 * halpha } else { 1.0 };
            let c = shade(base_c, 0.06 * hov).gamma_multiply(g.clamp(0.0, 1.0) * dim);
            let cr = CornerRadius::same(self.radius.min(r.width().min(r.height()) / 2.0) as u8);
            glowkit_core::paint::gradient_rect(&painter, r, cr.nw as f32, 120.0, &[c, shade(c, if t.dark { -0.05 } else { -0.04 })]);
            if hov > 0.01 {
                painter.rect_stroke(r, cr, Stroke::new(1.5, with_alpha(Color32::WHITE, 0.6 * hov)), StrokeKind::Inside);
            }
            let tc = contrast_text(base_c).gamma_multiply(g.clamp(0.0, 1.0));
            let name_w = text_width(&painter, &d.label, &FontId::proportional(12.5));
            if r.width() > name_w + 16.0 && r.height() > 40.0 {
                painter.text(r.left_top() + vec2(10.0, 9.0), Align2::LEFT_TOP, &d.label, FontId::proportional(12.5), tc);
                painter.text(r.left_top() + vec2(10.0, 26.0), Align2::LEFT_TOP, (self.base.format)(d.value), FontId::proportional(11.5), tc.gamma_multiply(0.75));
            } else if r.width() > name_w + 8.0 && r.height() > 20.0 {
                painter.text(r.center(), Align2::CENTER_CENTER, &d.label, FontId::proportional(11.0), tc);
            }
        }
        if let (Some(i), true) = (hover, self.base.tooltip) {
            let d = &self.data[i];
            let rows = [TipRow::new(Some(d.color.unwrap_or_else(|| palette(&t, i))), "Value", (self.base.format)(d.value)), TipRow::new(None, "Share", format_percent(d.value / total))];
            tooltip(ui, id, resp.hover_pos().unwrap_or(rect.center()), &d.label, &rows, halpha);
        }
        resp
    }
}
