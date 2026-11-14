//! A traffic card with stacked bars, a gliding tooltip and a Day / Week switch that regrows
//! the chart.

use super::common::{cr, idle_for, raised, soft_card, surface, track};
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Rect, Response, Sense, Stroke, Ui};
use glowkit_core::anim::{ease, Easing};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{Rng, Theme};

const SERIES: [&str; 4] = ["Direct", "Search", "Social", "Email"];
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const MONTH_DAYS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

fn date_label(day_of_year: i32) -> String {
    let mut d = day_of_year.rem_euclid(365) as u32;
    for (m, len) in MONTH_DAYS.iter().enumerate() {
        if d < *len {
            return format!("{} {}", MONTHS[m], d + 1);
        }
        d -= len;
    }
    "Dec 31".into()
}

fn thousands(v: u64) -> String {
    let s = v.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

#[derive(Clone)]
struct BarsState {
    weekly: bool,
    switched_at: f32,
    auto_step: i32,
}

/// A "Traffic" card of stacked bars: four positive segments per day plus a red "bounced"
/// segment hanging below the baseline. Run the pointer along the bars and one column is ringed
/// while a tooltip glides beside it with the day's breakdown; the Day / Week switch regrows
/// every bar with a stagger and rolls the headline number.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::StackedBars;
/// ui.add(StackedBars::new().bars(20).gap(6.0).monochrome(false));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct StackedBars {
    title: String,
    bars: usize,
    gap: f32,
    mono: bool,
    corner: f32,
    stroke: bool,
    width: f32,
    seed: u64,
    colors: Option<[Color32; 5]>,
    autoplay: bool,
}

impl Default for StackedBars {
    fn default() -> Self {
        Self::new()
    }
}

impl StackedBars {
    /// A 20-bar traffic card with generated data.
    pub fn new() -> Self {
        Self {
            title: "Traffic".into(),
            bars: 20,
            gap: 6.0,
            mono: false,
            corner: 20.0,
            stroke: false,
            width: 310.0,
            seed: 7,
            colors: None,
            autoplay: false,
        }
    }
    /// Card title.
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Number of bars (6..=40).
    pub fn bars(mut self, n: usize) -> Self {
        self.bars = n.clamp(6, 40);
        self
    }
    /// Gap between bars in points.
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g.clamp(0.0, 16.0);
        self
    }
    /// Draw every series in shades of the foreground instead of colors.
    pub fn monochrome(mut self, on: bool) -> Self {
        self.mono = on;
        self
    }
    /// Card corner radius in points.
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// Draw a hairline border.
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Card width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(220.0);
        self
    }
    /// Seed for the generated data.
    pub fn seed(mut self, s: u64) -> Self {
        self.seed = s;
        self
    }
    /// Colors of the four series (bottom to top) and the negative "bounced" series.
    pub fn colors(mut self, c: [Color32; 5]) -> Self {
        self.colors = Some(c);
        self
    }
    /// Sweep the tooltip and flip Day / Week by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn data(seed: u64, weekly: bool, n: usize) -> Vec<[f32; 5]> {
    let mut r = Rng::new(seed * 31 + if weekly { 999 } else { 1 });
    let scale = if weekly { 7.0 } else { 1.0 };
    (0..n)
        .map(|i| {
            let wave = 0.75 + 0.25 * ((i as f32) * 0.7).sin();
            let weekend = if !weekly && i % 7 >= 5 { 0.62 } else { 1.0 };
            let base = wave * weekend * scale;
            [
                (300.0 + r.range(0.0, 180.0)) * base,
                (180.0 + r.range(0.0, 160.0)) * base,
                (110.0 + r.range(0.0, 90.0)) * base,
                (90.0 + r.range(0.0, 110.0)) * base,
                (5.0 + r.range(0.0, 60.0)) * base,
            ]
        })
        .collect()
}

impl egui::Widget for StackedBars {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let h = 290.0;
        let (outer, outer_resp) = ui.allocate_exact_size(vec2(ui.available_width(), h + 10.0), Sense::hover());
        let rect = Rect::from_center_size(outer.center(), vec2(self.width.min(outer.width()), h));
        let id = outer_resp.id;
        let now = glowkit_core::time(ui);
        let n = self.bars;
        let pad = 18.0;
        // segmented control
        let seg = Rect::from_min_size(pos2(rect.max.x - pad - 112.0, rect.min.y + 16.0), vec2(112.0, 30.0));
        let day_r = Rect::from_min_max(seg.min, pos2(seg.center().x, seg.max.y));
        let week_r = Rect::from_min_max(pos2(seg.center().x, seg.min.y), seg.max);
        let day_resp = ui.interact(day_r, id.with("day"), Sense::click());
        let week_resp = ui.interact(week_r, id.with("week"), Sense::click());
        let chart = Rect::from_min_max(pos2(rect.min.x + pad, rect.min.y + 118.0), pos2(rect.max.x - pad, rect.max.y - 40.0));
        let chart_resp = ui.interact(chart.expand2(vec2(4.0, 10.0)), id.with("chart"), Sense::hover());
        let any = chart_resp.hovered() || day_resp.hovered() || week_resp.hovered();
        let idle = idle_for(ui, id, any);
        let mut st: BarsState = glowkit_core::state(ui, id.with("bars"), || BarsState { weekly: false, switched_at: -10.0, auto_step: -1 });
        if day_resp.clicked() && st.weekly {
            st.weekly = false;
            st.switched_at = now;
        }
        if week_resp.clicked() && !st.weekly {
            st.weekly = true;
            st.switched_at = now;
        }
        let colw = chart.width() / n as f32;
        let mut hovered = chart_resp.hover_pos().map(|p| (((p.x - chart.min.x) / colw).floor() as i32).clamp(0, n as i32 - 1) as usize);
        if self.autoplay && !any && idle > 0.8 {
            let cycle = 6.0;
            let u = (idle - 0.8) % cycle;
            let k = ((idle - 0.8) / cycle) as i32;
            if u < 4.2 {
                let f = ease(Easing::InOutSine, (u / 4.2).clamp(0.0, 1.0));
                let f = if k % 2 == 0 { f } else { 1.0 - f };
                hovered = Some(((0.1 + f * 0.8) * n as f32) as usize);
            } else if st.auto_step != k {
                st.auto_step = k;
                st.weekly = !st.weekly;
                st.switched_at = now;
            }
        }
        glowkit_core::set_state(ui, id.with("bars"), st.clone());
        let vals = data(self.seed, st.weekly, n);
        let max_pos = vals.iter().map(|v| v[0] + v[1] + v[2] + v[3]).fold(1.0, f32::max);
        let max_neg = vals.iter().map(|v| v[4]).fold(1.0, f32::max);
        let total: f32 = vals.iter().map(|v| v[0] + v[1] + v[2] + v[3]).sum();
        let shown_total = glowkit_core::anim::spring(ui, id.with("total"), total, 120.0, 22.0);
        let seg_x = glowkit_core::anim::spring(ui, id.with("seg"), if st.weekly { 1.0 } else { 0.0 }, 300.0, 24.0);
        let tip_target = hovered.map(|i| chart.min.x + (i as f32 + 0.5) * colw).unwrap_or(chart.center().x);
        let tip_x = glowkit_core::anim::spring(ui, id.with("tipx"), tip_target, 260.0, 26.0);
        let tip_a = glowkit_core::anim::follow(ui, id.with("tipa"), if hovered.is_some() { 1.0 } else { 0.0 }, 12.0);
        let focus = glowkit_core::anim::follow(ui, id.with("focus"), if hovered.is_some() { 1.0 } else { 0.0 }, 10.0);
        glowkit_core::animate(ui);
        if day_resp.hovered() || week_resp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(rect) {
            return outer_resp;
        }
        let painter = ui.painter();
        soft_card(painter, &t, rect, self.corner, 1.0, self.stroke);
        painter.text(pos2(rect.min.x + pad, rect.min.y + 24.0), Align2::LEFT_CENTER, &self.title, FontId::proportional(15.0), t.foreground);
        let sub = if st.weekly { format!("Last {n} weeks") } else { format!("Last {n} days") };
        painter.text(pos2(rect.min.x + pad, rect.min.y + 42.0), Align2::LEFT_CENTER, sub, FontId::proportional(12.0), t.muted_foreground);
        // segmented control
        painter.rect_filled(seg, cr(9.0), track(&t));
        let knob = Rect::from_min_size(pos2(seg.min.x + 3.0 + seg_x * (seg.width() / 2.0), seg.min.y + 3.0), vec2(seg.width() / 2.0 - 6.0, seg.height() - 6.0));
        painter.rect_filled(knob, cr(7.0), raised(&t));
        if !t.dark {
            painter.rect_stroke(knob, cr(7.0), Stroke::new(1.0, with_alpha(t.border, 0.8)), egui::StrokeKind::Inside);
        }
        for (k, (r, label)) in [(day_r, "Day"), (week_r, "Week")].iter().enumerate() {
            let active = if k == 0 { 1.0 - seg_x } else { seg_x };
            painter.text(r.center(), Align2::CENTER_CENTER, *label, FontId::proportional(12.5), mix(t.muted_foreground, t.foreground, active.clamp(0.0, 1.0)));
        }
        // headline number
        let num = thousands(shown_total.max(0.0) as u64);
        let g = painter.layout_no_wrap(num, FontId::proportional(30.0), t.foreground);
        let np = pos2(rect.min.x + pad, rect.min.y + 66.0);
        let nw = g.size().x;
        painter.galley(np, g, t.foreground);
        painter.text(pos2(np.x + nw + 8.0, np.y + 22.0), Align2::LEFT_CENTER, "visits", FontId::proportional(12.5), t.muted_foreground);
        // bars
        let cols: [Color32; 5] = if self.mono {
            let f = t.foreground;
            [with_alpha(f, 0.85), with_alpha(f, 0.6), with_alpha(f, 0.4), with_alpha(f, 0.25), with_alpha(f, 0.55)]
        } else {
            self.colors.unwrap_or([
                glowkit_core::hex("#A78BFA"),
                glowkit_core::hex("#5B8DEF"),
                glowkit_core::hex("#9DB7E0"),
                glowkit_core::hex("#F4C46A"),
                glowkit_core::hex("#EF4444"),
            ])
        };
        let neg_h = 18.0;
        let base_y = chart.max.y - neg_h - 4.0;
        let pos_h = base_y - chart.min.y;
        let bw = (colw - self.gap).max(2.0);
        let sgap = 2.0;
        for (i, v) in vals.iter().enumerate() {
            let grow = ease(Easing::OutBack, ((now - st.switched_at - i as f32 * 0.022) / 0.5).clamp(0.0, 1.0));
            let grow = if now - st.switched_at > 5.0 { 1.0 } else { grow };
            let x = chart.min.x + (i as f32 + 0.5) * colw;
            let dim = if hovered == Some(i) { 1.0 } else { 1.0 - 0.6 * focus };
            let mut y = base_y;
            for (s, val) in v.iter().take(4).enumerate() {
                let hh = (val / max_pos * pos_h * grow - sgap).max(0.0);
                if hh < 0.5 {
                    continue;
                }
                let r = Rect::from_min_max(pos2(x - bw / 2.0, y - hh), pos2(x + bw / 2.0, y));
                painter.rect_filled(r, cr((bw / 2.0).min(3.0)), with_alpha(cols[s], dim));
                y -= hh + sgap;
            }
            let nh = v[4] / max_neg * neg_h * grow;
            if nh > 0.5 {
                let r = Rect::from_min_max(pos2(x - bw / 2.0, base_y + 3.0), pos2(x + bw / 2.0, base_y + 3.0 + nh));
                painter.rect_filled(r, cr((bw / 2.0).min(3.0)), with_alpha(cols[4], dim));
            }
            if hovered == Some(i) {
                let top = y.min(base_y - 4.0);
                let ring = Rect::from_min_max(pos2(x - bw / 2.0 - 3.0, top - 1.0), pos2(x + bw / 2.0 + 3.0, base_y + 5.0 + nh));
                painter.rect_stroke(ring, cr(bw / 2.0 + 3.0), Stroke::new(1.3, with_alpha(t.foreground, 0.55 * focus)), egui::StrokeKind::Outside);
            }
        }
        // axis labels
        let (first, last) = if st.weekly {
            (date_label(284 - 7 * (n as i32 - 1)), date_label(284))
        } else {
            (date_label(284 - (n as i32 - 1)), date_label(284))
        };
        let ly = rect.max.y - 20.0;
        painter.text(pos2(chart.min.x, ly), Align2::LEFT_CENTER, first, FontId::proportional(11.0), with_alpha(t.muted_foreground, 0.8));
        painter.text(pos2(chart.max.x, ly), Align2::RIGHT_CENTER, last, FontId::proportional(11.0), with_alpha(t.muted_foreground, 0.8));
        // tooltip
        if tip_a > 0.02 {
            if let Some(i) = hovered.or(Some(((tip_x - chart.min.x) / colw) as usize).filter(|k| *k < n)) {
                let v = vals[i];
                let day = if st.weekly { 284 - 7 * (n as i32 - 1 - i as i32) } else { 284 - (n as i32 - 1 - i as i32) };
                let tw = 132.0;
                let th = 96.0;
                let right_side = tip_x + 14.0 + tw < rect.max.x - 4.0;
                let tx = if right_side { tip_x + 12.0 } else { tip_x - 12.0 - tw };
                let tr = Rect::from_min_size(pos2(tx, chart.min.y - 6.0), vec2(tw, th));
                let tp = painter.with_clip_rect(rect.expand(30.0));
                glowkit_core::paint::soft_shadow(&tp, tr, 10.0, 14.0, vec2(0.0, 4.0), Color32::from_black_alpha((40.0 * tip_a) as u8));
                tp.rect(tr, cr(10.0), with_alpha(surface(&t), tip_a), Stroke::new(1.0, with_alpha(t.border, tip_a)), egui::StrokeKind::Inside);
                let a = tip_a;
                let sum: f32 = v[..4].iter().sum();
                tp.text(pos2(tr.min.x + 10.0, tr.min.y + 15.0), Align2::LEFT_CENTER, date_label(day), FontId::proportional(11.5), with_alpha(t.muted_foreground, a));
                tp.text(pos2(tr.max.x - 10.0, tr.min.y + 15.0), Align2::RIGHT_CENTER, thousands(sum as u64), FontId::proportional(11.5), with_alpha(t.foreground, a));
                for (k, name) in [(0usize, SERIES[0]), (1, SERIES[1]), (3, SERIES[3]), (4, "Bounced")].iter() {
                    let row = match k {
                        0 => 0,
                        1 => 1,
                        3 => 2,
                        _ => 3,
                    };
                    let y = tr.min.y + 34.0 + row as f32 * 16.0;
                    tp.rect_filled(Rect::from_center_size(pos2(tr.min.x + 14.0, y), vec2(7.0, 7.0)), cr(2.0), with_alpha(cols[*k], a));
                    tp.text(pos2(tr.min.x + 24.0, y), Align2::LEFT_CENTER, *name, FontId::proportional(11.5), with_alpha(t.foreground, a * 0.85));
                    tp.text(pos2(tr.max.x - 10.0, y), Align2::RIGHT_CENTER, format!("{}", v[*k] as u32), FontId::proportional(11.5), with_alpha(t.foreground, a));
                }
            }
        }
        outer_resp
    }
}
