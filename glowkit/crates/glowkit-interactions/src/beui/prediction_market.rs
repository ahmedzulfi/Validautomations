use super::common::{card, cr, icon, idle_for, well, BeIcon};
use super::wallet_card::odometer;
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke, Ui};
use glowkit_core::anim::spring;
use glowkit_core::color::with_alpha;
use glowkit_core::{paint, Rng, Theme};

struct Outcome {
    name: String,
    color: Color32,
    prob: f32,
}

#[derive(Clone, Debug)]
struct MarketState {
    probs: Vec<f32>,
    history: Vec<f32>,
    volume: f64,
    saved: bool,
    last_auto: f64,
    seed: u64,
    flash: Option<(usize, f64)>,
}

/// A prediction-market card: a question, two outcomes with probability bars that spring to
/// their new length, odds multipliers, and green / red price buttons whose percentages roll like
/// an odometer when the market moves. A sparkline redraws on every update and the bookmark
/// toggles. Pressing a price "buys" that side, nudging the odds.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::PredictionMarket;
/// ui.add(
///     PredictionMarket::new("Harbor City vs. North Ridge", "Sunday · 7:30 PM · Basketball")
///         .outcome("Harbor", egui::Color32::LIGHT_GREEN, 0.51)
///         .outcome("Ridge", egui::Color32::GOLD, 0.49),
/// );
/// ```
pub struct PredictionMarket {
    title: String,
    meta: String,
    live: bool,
    badge: (String, Color32),
    outcomes: Vec<Outcome>,
    volume: f64,
    width: f32,
    autoplay: bool,
}

impl PredictionMarket {
    /// A market asking `title`, with a `meta` line (date · league).
    pub fn new(title: impl Into<String>, meta: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            meta: meta.into(),
            live: false,
            badge: ("M".into(), glowkit_core::hex("#10B981")),
            outcomes: Vec::new(),
            volume: 2_400_000.0,
            width: 340.0,
            autoplay: true,
        }
    }
    /// Add an outcome with its colour and starting probability (0..1).
    pub fn outcome(mut self, name: impl Into<String>, color: Color32, prob: f32) -> Self {
        self.outcomes.push(Outcome { name: name.into(), color, prob: prob.clamp(0.01, 0.99) });
        self
    }
    /// The round badge in the header: a monogram and its colour.
    pub fn badge(mut self, text: impl Into<String>, color: Color32) -> Self {
        self.badge = (text.into(), color);
        self
    }
    /// Show a pulsing red "live" dot before the meta line.
    pub fn live(mut self, on: bool) -> Self {
        self.live = on;
        self
    }
    /// Starting traded volume in dollars.
    pub fn volume(mut self, v: f64) -> Self {
        self.volume = v.max(0.0);
        self
    }
    /// Card width.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.clamp(260.0, 520.0);
        self
    }
    /// Move the market now and then while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn short_money(v: f64) -> String {
    if v >= 1e6 {
        format!("${:.1}M", v / 1e6)
    } else if v >= 1e3 {
        format!("${:.0}K", v / 1e3)
    } else {
        format!("${v:.0}")
    }
}

impl egui::Widget for PredictionMarket {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let painter = ui.painter().clone();
        let w = self.width;
        let pad = 16.0;
        let title_g = painter.layout(self.title.clone(), FontId::proportional(15.0), t.foreground, w - pad * 2.0 - 56.0);
        let head_h = (title_g.size().y + 22.0).max(46.0) + pad;
        let n = self.outcomes.len().min(3);
        let row_h = 58.0;
        let inner_h = n as f32 * row_h + 44.0;
        let h = head_h + inner_h + pad + 8.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let id = resp.id;
        let now = ui.input(|i| i.time);
        let idle = idle_for(ui, id, resp.hovered());
        let auto = self.autoplay && idle > 1.5;
        let inner = Rect::from_min_size(pos2(rect.min.x + 8.0, rect.min.y + head_h), vec2(w - 16.0, inner_h));
        let btn_rect = |i: usize| Rect::from_min_size(pos2(inner.max.x - 74.0, inner.min.y + 10.0 + i as f32 * row_h), vec2(62.0, 38.0));
        let mark = pos2(inner.max.x - 26.0, inner.max.y - 22.0);
        let mut bought = None;
        for i in 0..n {
            let r = ui.interact(btn_rect(i), id.with(("buy", i)), Sense::click());
            if r.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if r.clicked() {
                bought = Some(i);
            }
        }
        let mr = ui.interact(Rect::from_center_size(mark, vec2(24.0, 24.0)), id.with("mark"), Sense::click());
        let probs0: Vec<f32> = self.outcomes.iter().map(|o| o.prob).collect();
        let vol0 = self.volume;
        let st = glowkit_core::with_state(
            ui,
            id,
            || {
                let mut rng = Rng::new(9);
                let mut v = probs0.first().copied().unwrap_or(0.5);
                let history = (0..24)
                    .map(|_| {
                        v = (v + rng.range(-0.03, 0.035)).clamp(0.05, 0.95);
                        v
                    })
                    .collect();
                MarketState { probs: probs0.clone(), history, volume: vol0, saved: false, last_auto: now, seed: 1, flash: None }
            },
            |s| {
                let mut moved = None;
                if let Some(i) = bought {
                    moved = Some((i, 0.04));
                    s.flash = Some((i, now));
                }
                if auto && now - s.last_auto > 2.4 {
                    s.last_auto = now;
                    s.seed += 1;
                    let mut rng = Rng::new(s.seed * 131);
                    moved = Some((0, rng.range(-0.09, 0.09)));
                }
                if let Some((i, d)) = moved {
                    if s.probs.len() >= 2 {
                        let j = if i == 0 { 1 } else { 0 };
                        s.probs[i] = (s.probs[i] + d).clamp(0.03, 0.97);
                        s.probs[j] = 1.0 - s.probs[i];
                    }
                    s.volume += 12_000.0 + (s.seed % 7) as f64 * 9_000.0;
                    let p0 = s.probs.first().copied().unwrap_or(0.5);
                    s.history.push(p0);
                    if s.history.len() > 24 {
                        s.history.remove(0);
                    }
                }
                if mr.clicked() {
                    s.saved = !s.saved;
                }
                s.clone()
            },
        );
        if auto {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        card(&painter, &t, rect, 26.0, 0.5);
        // header
        let bc = pos2(rect.min.x + pad + 20.0, rect.min.y + pad + 20.0);
        painter.circle_filled(bc, 21.0, self.badge.1);
        painter.circle_stroke(bc, 17.5, Stroke::new(1.2, with_alpha(Color32::WHITE, 0.6)));
        painter.text(bc, Align2::CENTER_CENTER, &self.badge.0, FontId::proportional(15.0), Color32::WHITE);
        let tx = rect.min.x + pad + 54.0;
        painter.galley(pos2(tx, rect.min.y + pad), title_g.clone(), t.foreground);
        let my = rect.min.y + pad + title_g.size().y + 11.0;
        let mut mx = tx;
        if self.live {
            let pulse = ((now * 3.0).sin() * 0.5 + 0.5) as f32;
            painter.circle_filled(pos2(mx + 4.0, my), 3.5, t.destructive);
            painter.circle_stroke(pos2(mx + 4.0, my), 3.5 + pulse * 3.0, Stroke::new(1.0, with_alpha(t.destructive, 1.0 - pulse)));
            mx += 13.0;
            glowkit_core::animate(ui);
        }
        painter.text(pos2(mx, my), Align2::LEFT_CENTER, &self.meta, FontId::proportional(12.0), if self.live { t.destructive } else { t.muted_foreground });
        // inner well
        painter.rect_filled(inner, cr(20.0), well(&t));
        let green = t.success;
        let red = t.destructive;
        for (i, o) in self.outcomes.iter().take(n).enumerate() {
            let p = st.probs.get(i).copied().unwrap_or(o.prob);
            let y = inner.min.y + 10.0 + i as f32 * row_h + 19.0;
            let x = inner.min.x + 14.0;
            let mono = o.name.chars().next().unwrap_or('?').to_string();
            painter.circle_filled(pos2(x + 12.0, y - 2.0), 12.0, with_alpha(o.color, 0.18));
            painter.text(pos2(x + 12.0, y - 2.0), Align2::CENTER_CENTER, mono, FontId::proportional(12.0), o.color);
            painter.text(pos2(x + 32.0, y - 2.0), Align2::LEFT_CENTER, &o.name, FontId::proportional(14.0), t.foreground);
            // probability bar
            let bw = spring(ui, id.with(("bar", i)), p * 120.0, 180.0, 16.0);
            let by = y + 18.0;
            painter.line_segment([pos2(x, by), pos2(x + bw.max(4.0), by)], Stroke::new(2.0, o.color));
            // multiplier
            let mult = format!("{:.1}×", (1.0 / p.max(0.01)).min(99.0));
            let b = btn_rect(i);
            painter.text(pos2(b.min.x - 10.0, b.center().y), Align2::RIGHT_CENTER, mult, FontId::proportional(13.0), t.muted_foreground);
            // price button: green for the favourite, red for the other
            let col = if p >= 0.5 { green } else { red };
            let flash = st.flash.filter(|f| f.0 == i).map(|f| (1.0 - (now - f.1) as f32 / 0.6).clamp(0.0, 1.0)).unwrap_or(0.0);
            if flash > 0.0 {
                glowkit_core::animate(ui);
            }
            let bb = b.expand(flash * 2.0);
            painter.rect(bb, cr(10.0), with_alpha(col, 0.16 + 0.2 * flash), Stroke::new(1.0, with_alpha(col, 0.45)), egui::StrokeKind::Inside);
            let shadow = Rect::from_min_max(pos2(bb.min.x + 3.0, bb.max.y - 2.0), pos2(bb.max.x - 3.0, bb.max.y));
            painter.rect_filled(shadow, cr(2.0), with_alpha(col, 0.25));
            let pct = format!("{}%", (p * 100.0).round() as i32);
            odometer(ui, &painter, id.with(("pct", i)), &pct, bb.center(), 14.0, col);
        }
        // footer: sparkline, volume, bookmark
        let fy = inner.max.y - 22.0;
        let spark = Rect::from_min_size(pos2(inner.min.x + 14.0, fy - 9.0), vec2(46.0, 16.0));
        let hist = &st.history;
        let (lo, hi) = hist.iter().fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
        let pts: Vec<Pos2> = hist
            .iter()
            .enumerate()
            .map(|(k, v)| pos2(spark.min.x + k as f32 / (hist.len() - 1).max(1) as f32 * spark.width(), spark.max.y - (v - lo) / (hi - lo).max(1e-3) * spark.height()))
            .collect();
        let up = hist.last().copied().unwrap_or(0.0) >= hist.first().copied().unwrap_or(0.0);
        let sc = if up { green } else { red };
        paint::area_fill(&painter, &pts, spark.max.y, with_alpha(sc, 0.25), with_alpha(sc, 0.0));
        painter.add(Shape::line(pts, Stroke::new(1.5, sc)));
        painter.text(pos2(spark.max.x + 10.0, fy), Align2::LEFT_CENTER, format!("{} vol.", short_money(st.volume)), FontId::proportional(12.0), t.muted_foreground);
        let pop = spring(ui, id.with("save"), if st.saved { 1.0 } else { 0.0 }, 500.0, 14.0);
        if pop > 0.02 {
            let pts = [(-0.55, -0.8), (0.55, -0.8), (0.55, 0.85), (0.0, 0.45), (-0.55, 0.85)];
            let s = 6.5 * (0.6 + 0.4 * pop);
            let poly: Vec<Pos2> = pts.iter().map(|(x, y)| mark + vec2(*x, *y) * s).collect();
            super::common::fill_fan(&painter, mark, &poly, with_alpha(t.accent, pop.min(1.0)));
        }
        icon(&painter, BeIcon::Bookmark, mark, 13.0, if st.saved { t.accent } else { t.muted_foreground }, 1.3);
        if mr.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        resp
    }
}
