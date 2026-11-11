//! Context-window usage meter.

use crate::common;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, Order, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::{paint, Theme};
use std::f32::consts::{FRAC_PI_2, TAU};

/// Format a token count: 950, 12.4K, 1.2M.
pub fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1e6)
    } else if n >= 10_000 {
        format!("{:.0}K", n as f64 / 1e3)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1e3)
    } else {
        n.to_string()
    }
}

/// How a [`ContextMeter`] is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MeterStyle {
    /// A compact pill with a ring and a percentage; details in a hover card.
    #[default]
    Ring,
    /// A full-width segmented bar with a legend.
    Bar,
}

/// Shows how much of the model's context window is used: a ring (or segmented bar) with a
/// breakdown of input / output / reasoning / cached tokens and an optional cost.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_ai::ContextMeter::new(48_200, 128_000)
///     .segment("Input", 30_100)
///     .segment("Output", 12_900)
///     .segment("Reasoning", 5_200)
///     .cost(0.0241));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ContextMeter<'a> {
    used: u64,
    max: u64,
    segments: Vec<(&'a str, u64)>,
    cost: Option<f64>,
    model: Option<&'a str>,
    style: MeterStyle,
    colors: Option<Vec<Color32>>,
}

impl<'a> ContextMeter<'a> {
    /// `used` of `max` tokens.
    pub fn new(used: u64, max: u64) -> Self {
        Self { used, max: max.max(1), segments: Vec::new(), cost: None, model: None, style: MeterStyle::Ring, colors: None }
    }
    /// Add a breakdown entry.
    pub fn segment(mut self, label: &'a str, tokens: u64) -> Self {
        self.segments.push((label, tokens));
        self
    }
    /// Cost so far in dollars.
    pub fn cost(mut self, dollars: f64) -> Self {
        self.cost = Some(dollars);
        self
    }
    /// Model name shown in the details.
    pub fn model(mut self, name: &'a str) -> Self {
        self.model = Some(name);
        self
    }
    /// Ring (compact) or bar.
    pub fn style(mut self, style: MeterStyle) -> Self {
        self.style = style;
        self
    }
    /// Segment colors (defaults to the theme chart palette).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }

    fn color(&self, i: usize, t: &Theme) -> Color32 {
        match &self.colors {
            Some(c) if !c.is_empty() => c[i % c.len()],
            _ => t.chart[i % t.chart.len()],
        }
    }

    fn frac(&self) -> f32 {
        (self.used as f64 / self.max as f64).clamp(0.0, 1.0) as f32
    }

    fn level_color(&self, t: &Theme) -> Color32 {
        let f = self.frac();
        if f > 0.9 {
            t.destructive
        } else if f > 0.75 {
            t.warning
        } else {
            t.foreground
        }
    }

    fn details(&self, ui: &mut Ui, t: &Theme, width: f32) {
        ui.spacing_mut().item_spacing.y = 8.0;
        let pct = self.frac() * 100.0;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{pct:.1}%")).size(13.0).color(t.foreground));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(format!("{} / {}", format_tokens(self.used), format_tokens(self.max))).size(12.0).color(t.muted_foreground));
            });
        });
        let (bar, _) = ui.allocate_exact_size(vec2(width, 6.0), Sense::hover());
        self.paint_bar(ui, bar, t);
        for (i, (label, n)) in self.segments.iter().enumerate() {
            let (r, _) = ui.allocate_exact_size(vec2(width, 18.0), Sense::hover());
            let p = ui.painter();
            p.circle_filled(pos2(r.min.x + 4.0, r.center().y), 3.5, self.color(i, t));
            common::text(p, pos2(r.min.x + 14.0, r.center().y), Align2::LEFT_CENTER, label, 12.5, t.muted_foreground);
            common::text(p, pos2(r.max.x, r.center().y), Align2::RIGHT_CENTER, format_tokens(*n), 12.5, t.foreground);
        }
        if self.cost.is_some() || self.model.is_some() {
            let (r, _) = ui.allocate_exact_size(vec2(width, 1.0), Sense::hover());
            ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0, t.border));
            let (r, _) = ui.allocate_exact_size(vec2(width, 18.0), Sense::hover());
            let p = ui.painter();
            if let Some(m) = self.model {
                common::text(p, pos2(r.min.x, r.center().y), Align2::LEFT_CENTER, m, 12.5, t.muted_foreground);
            }
            if let Some(c) = self.cost {
                common::text(p, pos2(r.max.x, r.center().y), Align2::RIGHT_CENTER, format!("${c:.4}"), 12.5, t.foreground);
            }
        }
    }

    fn paint_bar(&self, ui: &Ui, bar: Rect, t: &Theme) {
        let p = ui.painter();
        let r = CornerRadius::same((bar.height() / 2.0) as u8);
        p.rect_filled(bar, r, with_alpha(t.muted_foreground, 0.16));
        let mut x = bar.min.x;
        let segs: Vec<(usize, u64)> = if self.segments.is_empty() { vec![(0, self.used)] } else { self.segments.iter().enumerate().map(|(i, s)| (i, s.1)).collect() };
        let clip = p.with_clip_rect(bar);
        let fill_w = bar.width() * self.frac();
        let mut fill = Vec::new();
        for (i, n) in segs {
            let w = bar.width() * (n as f64 / self.max as f64) as f32;
            if w <= 0.0 {
                continue;
            }
            fill.push((Rect::from_min_size(pos2(x, bar.min.y), vec2(w, bar.height())), self.color(i, t)));
            x += w;
        }
        // rounded overall fill: paint segments clipped to a rounded rect of the filled width
        let fr = Rect::from_min_size(bar.min, vec2(fill_w.max(x - bar.min.x), bar.height()));
        for (k, (sr, c)) in fill.iter().enumerate() {
            let cr = if k == 0 { CornerRadius { nw: r.nw, sw: r.sw, ne: 0, se: 0 } } else { CornerRadius::ZERO };
            let cr = if k + 1 == fill.len() { CornerRadius { ne: r.ne, se: r.se, ..cr } } else { cr };
            clip.rect_filled(sr.intersect(fr), cr, *c);
        }
    }
}

impl Widget for ContextMeter<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        match self.style {
            MeterStyle::Bar => {
                let w = ui.available_width().min(420.0);
                ui.allocate_ui(vec2(w, 0.0), |ui| {
                    ui.set_width(w);
                    self.details(ui, &t, w);
                })
                .response
            }
            MeterStyle::Ring => {
                let pct = format!("{:.0}%", self.frac() * 100.0);
                let g = common::galley(ui, pct, 12.0, t.muted_foreground);
                let (rect, resp) = ui.allocate_exact_size(vec2(g.size().x + 40.0, 30.0), Sense::hover());
                let h = common::anim_bool(ui, resp.id.with("h"), resp.hovered(), 0.12);
                let p = ui.painter();
                p.rect_filled(rect, CornerRadius::same(15), common::hover_fill(&t).gamma_multiply(0.4 + h * 0.6));
                p.rect_stroke(rect, CornerRadius::same(15), Stroke::new(1.0, t.border), StrokeKind::Inside);
                let c = pos2(rect.min.x + 16.0, rect.center().y);
                let f = glowkit_core::anim::follow(ui, resp.id.with("f"), self.frac(), 6.0);
                p.circle_stroke(c, 7.0, Stroke::new(2.4, with_alpha(t.muted_foreground, 0.22)));
                if f > 0.0 {
                    paint::arc(p, c, 7.0, -FRAC_PI_2, -FRAC_PI_2 + TAU * f, 2.4, self.level_color(&t));
                }
                p.galley(pos2(rect.min.x + 29.0, rect.center().y - g.size().y / 2.0), g, t.muted_foreground.lerp_to_gamma(t.foreground, h));
                if resp.hovered() {
                    egui::Area::new(resp.id.with("card"))
                        .order(Order::Tooltip)
                        .fixed_pos(pos2(rect.min.x, rect.max.y + 6.0))
                        .show(ui.ctx(), |ui| {
                            egui::Frame::new()
                                .fill(t.popover)
                                .stroke(Stroke::new(1.0, t.border))
                                .corner_radius(t.corner())
                                .shadow(egui::Shadow { offset: [0, 6], blur: 24, spread: 0, color: Color32::from_black_alpha(if t.dark { 120 } else { 30 }) })
                                .inner_margin(egui::Margin::same(14))
                                .show(ui, |ui| {
                                    ui.set_width(240.0);
                                    self.details(ui, &t, 240.0);
                                });
                        });
                }
                resp
            }
        }
    }
}
