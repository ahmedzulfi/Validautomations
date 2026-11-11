//! Feature sections: icon grids, bento grids with animated mini-visuals, alternating
//! image/text rows and tabbed features.

use super::kit::*;
use egui::{pos2, vec2, Align, Color32, CornerRadius, FontId, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};
use glowkit_components::{Tabs, TabsStyle, UiIcon};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::TAU;

/// Arrangement of a [`FeaturesSection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FeaturesLayout {
    /// Icon + title + text cards in a grid.
    #[default]
    IconGrid,
    /// Borderless icon list with hairline dividers (calmer than cards).
    IconList,
    /// Bento grid: mixed card sizes, each with an animated mini illustration.
    Bento,
    /// Alternating rows of text and illustration.
    Alternating,
    /// Tabs switch between features; the active one is shown with its illustration.
    Tabbed,
}

/// Small animated illustration used by bento cards and alternating rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FeatureVisual {
    /// Pick one automatically from the item's position.
    #[default]
    Auto,
    /// A live area chart.
    Chart,
    /// Dots orbiting a center tile.
    Orbit,
    /// Code lines being typed.
    Code,
    /// Bars growing and shrinking.
    Bars,
    /// Radar pulse rings.
    Pulse,
    /// A stack of toggles flipping.
    Toggles,
}

impl FeatureVisual {
    fn resolve(self, i: usize) -> Self {
        match self {
            FeatureVisual::Auto => [Self::Chart, Self::Orbit, Self::Code, Self::Bars, Self::Pulse, Self::Toggles][i % 6],
            v => v,
        }
    }
}

/// One feature: icon, title, description and optional bullet points.
#[derive(Clone, Debug)]
pub struct FeatureItem {
    /// Icon shown in the tinted tile.
    pub icon: UiIcon,
    /// Short title.
    pub title: String,
    /// One or two sentences.
    pub description: String,
    /// Checklist (alternating and tabbed layouts).
    pub bullets: Vec<String>,
    /// Illustration (bento, alternating and tabbed layouts).
    pub visual: FeatureVisual,
}

impl FeatureItem {
    /// A feature with an icon, title and description.
    pub fn new(icon: UiIcon, title: impl Into<String>, description: impl Into<String>) -> Self {
        Self { icon, title: title.into(), description: description.into(), bullets: Vec::new(), visual: FeatureVisual::Auto }
    }
    /// Add checklist bullets.
    pub fn bullets(mut self, b: &[&str]) -> Self {
        self.bullets = b.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Choose the illustration.
    pub fn visual(mut self, v: FeatureVisual) -> Self {
        self.visual = v;
        self
    }

    /// A ready-made set of six generic product features.
    pub fn defaults() -> Vec<FeatureItem> {
        vec![
            FeatureItem::new(UiIcon::Zap, "Blazing fast", "Immediate-mode rendering at 120 fps, with work skipped for anything off screen.").bullets(&["Sub-millisecond widgets", "No virtual DOM", "GPU tessellation"]),
            FeatureItem::new(UiIcon::Sparkles, "Delightful motion", "Springs, staggers and easings tuned so every interaction feels alive.").bullets(&["Spring physics", "Staggered reveals", "Hover and press states"]),
            FeatureItem::new(UiIcon::Code, "Pure Rust", "One language from logic to pixels. No JavaScript, no CSS, no build chain.").bullets(&["Type-safe builders", "Cargo features", "Zero unsafe"]),
            FeatureItem::new(UiIcon::Activity, "Live charts", "Animated, themeable charts that update in real time as data streams in.").bullets(&["Area, bar and pie", "Smooth transitions", "Tooltips"]),
            FeatureItem::new(UiIcon::Globe, "Runs everywhere", "Windows, macOS, Linux and the web via WebAssembly from one codebase.").bullets(&["Native windows", "WASM builds", "Mobile-friendly"]),
            FeatureItem::new(UiIcon::Lock, "Secure by default", "No network, no telemetry, no surprises. Your data never leaves the app.").bullets(&["Offline first", "Auditable", "MIT licensed"]),
        ]
    }
}

/// A features section in one of several layouts ([`FeaturesLayout`]).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{FeaturesSection, FeaturesLayout, FeatureItem};
/// use glowkit_components::UiIcon;
/// FeaturesSection::new("Everything you need")
///     .eyebrow("Features")
///     .subtitle("Batteries included.")
///     .items(vec![FeatureItem::new(UiIcon::Zap, "Fast", "Really fast.")])
///     .layout(FeaturesLayout::Bento)
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct FeaturesSection {
    title: String,
    eyebrow: Option<String>,
    subtitle: Option<String>,
    items: Vec<FeatureItem>,
    layout: FeaturesLayout,
    columns: usize,
    background: SectionBackground,
    align: SectionAlign,
    link_label: String,
    salt: Option<String>,
}

impl FeaturesSection {
    /// A features section with the default six features.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            eyebrow: None,
            subtitle: None,
            items: FeatureItem::defaults(),
            layout: FeaturesLayout::IconGrid,
            columns: 3,
            background: SectionBackground::Plain,
            align: SectionAlign::Center,
            link_label: "Learn more".into(),
            salt: None,
        }
    }
    /// Small label above the title.
    pub fn eyebrow(mut self, e: impl Into<String>) -> Self {
        self.eyebrow = Some(e.into());
        self
    }
    /// Paragraph under the title.
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = Some(s.into());
        self
    }
    /// The features to show.
    pub fn items(mut self, items: Vec<FeatureItem>) -> Self {
        self.items = items;
        self
    }
    /// Layout variant.
    pub fn layout(mut self, l: FeaturesLayout) -> Self {
        self.layout = l;
        self
    }
    /// Maximum columns for the grid layouts (default 3).
    pub fn columns(mut self, c: usize) -> Self {
        self.columns = c.max(1);
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Heading alignment.
    pub fn align(mut self, a: SectionAlign) -> Self {
        self.align = a;
        self
    }
    /// Label of the link under each alternating row (clicks report the feature title).
    pub fn link_label(mut self, l: impl Into<String>) -> Self {
        self.link_label = l.into();
        self
    }
    /// Distinguish several feature sections in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the section.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "features", &self.salt);
        section(ui, Frame::new(id, self.background), |ui, sec| {
            let align = self.align.align();
            let next = Heading::new(&self.title).eyebrow(self.eyebrow.as_deref()).subtitle(self.subtitle.as_deref()).align(align).show(ui, sec, 0);
            ui.add_space(if sec.narrow { 40.0 } else { 56.0 });
            match self.layout {
                FeaturesLayout::IconGrid => self.icon_grid(ui, sec, next, true),
                FeaturesLayout::IconList => self.icon_grid(ui, sec, next, false),
                FeaturesLayout::Bento => self.bento(ui, sec, next),
                FeaturesLayout::Alternating => self.alternating(ui, sec, next),
                FeaturesLayout::Tabbed => self.tabbed(ui, sec, next),
            }
        })
    }

    fn icon_grid(&self, ui: &mut Ui, sec: &mut Sec, first: usize, cards: bool) {
        let cols = sec.cols(240.0, self.columns);
        let t = sec.t.clone();
        let id = sec.id;
        let gap = if cards { 20.0 } else { 40.0 };
        grid(ui, id.with("g"), self.items.len(), cols, gap, |ui, i, row_h| {
            let it = &self.items[i];
            sec.fade(ui, first + i / cols + i % cols, |ui| {
                let body = |ui: &mut Ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
                    icon_tile(ui.painter(), r, it.icon, t.chart[i % 6], &t);
                    ui.add_space(12.0);
                    Tx::new(&it.title).size(17.0).semi().color(t.foreground).show(ui);
                    ui.add_space(2.0);
                    Tx::new(&it.description).size(14.5).line(1.55).color(t.muted_foreground).show(ui);
                };
                if cards {
                    card(ui, id.with(("fc", i)), CardLook::of(&t).min_h(row_h), body);
                } else {
                    body(ui);
                }
            });
        });
    }

    fn bento(&self, ui: &mut Ui, sec: &mut Sec, first: usize) {
        let t = sec.t.clone();
        let id = sec.id;
        let n = self.items.len().min(6);
        if sec.narrow {
            let cols = sec.cols(260.0, 2);
            grid(ui, id.with("bn"), n, cols, 16.0, |ui, i, row_h| {
                sec.fade(ui, first + i, |ui| bento_card(ui, id, &self.items[i], i, &t, row_h, 140.0));
            });
            return;
        }
        // 3-column bento: [wide][ ]  /  [ ][ ][ ]  / [ ][wide]
        let gap = 16.0;
        let w = ui.available_width();
        let cw = (w - gap * 2.0) / 3.0;
        let spans: [(usize, usize); 6] = [(0, 2), (2, 1), (0, 1), (1, 1), (2, 1), (0, 3)];
        let rows: [&[usize]; 3] = [&[0, 1], &[2, 3, 4], &[5]];
        let origin = ui.cursor().min;
        let mut y = origin.y;
        for (r, members) in rows.iter().enumerate() {
            let key = id.with(("bentorow", r));
            let prev: f32 = glowkit_core::state(ui, key, || 0.0);
            let mut max_h: f32 = 0.0;
            let mut any = false;
            for &i in members.iter() {
                if i >= n {
                    continue;
                }
                any = true;
                let (col, span) = spans[i];
                let x = origin.x + col as f32 * (cw + gap);
                let width = cw * span as f32 + gap * (span as f32 - 1.0);
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos2(x, y), vec2(width, f32::INFINITY))).layout(egui::Layout::top_down(Align::Min)).id_salt(("bento", i)));
                let vis_h = if span == 3 { 120.0 } else { 160.0 };
                sec.fade(&mut child, first + i, |ui| bento_card(ui, id, &self.items[i], i, &t, prev, vis_h));
                max_h = max_h.max(child.min_rect().height());
            }
            if !any {
                break;
            }
            if (max_h - prev).abs() > 0.5 {
                glowkit_core::set_state(ui, key, max_h);
                ui.ctx().request_repaint();
            }
            y += max_h.max(prev) + gap;
        }
        ui.advance_cursor_after_rect(Rect::from_min_max(origin, pos2(origin.x + w, y - gap)));
    }

    fn alternating(&self, ui: &mut Ui, sec: &mut Sec, first: usize) {
        let t = sec.t.clone();
        let id = sec.id;
        let mut clicked = None;
        for (i, it) in self.items.iter().enumerate() {
            if i > 0 {
                ui.add_space(if sec.narrow { 48.0 } else { 80.0 });
            }
            let text = |ui: &mut Ui| {
                sec.fade(ui, first + i, |ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
                    icon_tile(ui.painter(), r, it.icon, t.chart[i % 6], &t);
                    ui.add_space(16.0);
                    Tx::new(&it.title).size(if sec.narrow { 24.0 } else { 30.0 }).bold().line(1.2).color(t.foreground).show(ui);
                    ui.add_space(10.0);
                    Tx::new(&it.description).size(16.0).line(1.6).color(t.muted_foreground).show(ui);
                    if !it.bullets.is_empty() {
                        ui.add_space(16.0);
                        for b in &it.bullets {
                            check_row(ui, b, t.accent, false);
                            ui.add_space(2.0);
                        }
                    }
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        if link(ui, &self.link_label, 14.5, t.accent, t.foreground).clicked() {
                            clicked = Some(it.title.clone());
                        }
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                        UiIcon::ArrowRight.paint(ui.painter(), r, t.accent);
                    });
                });
            };
            let art = |ui: &mut Ui| {
                sec.fade(ui, first + i + 1, |ui| {
                    let w = ui.available_width();
                    let (r, _) = ui.allocate_exact_size(vec2(w, (w * 0.66).clamp(220.0, 340.0)), Sense::hover());
                    let cr = (t.radius * 1.6).round();
                    paint_art(ui.painter(), r, i + 1, &t, cr, glowkit_core::time(ui));
                    let inner = r.shrink(r.width().min(r.height()) * 0.14);
                    paint_mini(ui, ui.painter(), inner, it.visual.resolve(i), &t, i, true);
                });
            };
            if i % 2 == 1 && !sec.narrow {
                two_col(ui, id.with(("alt", i)), 0.5, 64.0, sec.narrow, true, art, text);
            } else {
                two_col(ui, id.with(("alt", i)), 0.46, 64.0, sec.narrow, true, text, art);
            }
        }
        if let Some(c) = clicked {
            sec.click(c);
        }
    }

    fn tabbed(&self, ui: &mut Ui, sec: &mut Sec, first: usize) {
        let t = sec.t.clone();
        let id = sec.id;
        let mut sel: usize = glowkit_core::state(ui, id.with("tab"), || 0);
        let labels: Vec<&str> = self.items.iter().map(|i| i.title.as_str()).collect();
        let before = sel;
        sec.fade(ui, first, |ui| {
            row(ui, id.with("tabs"), Align::Center, 0.0, |ui| {
                ui.add(Tabs::new(&mut sel, &labels).style(TabsStyle::Pill));
            });
        });
        if sel != before {
            glowkit_core::set_state(ui, id.with("tab"), sel);
            glowkit_core::anim::Clock::restart(ui, id.with("tabclock"));
            sec.click(labels[sel].to_string());
        }
        ui.add_space(32.0);
        let it = &self.items[sel.min(self.items.len().saturating_sub(1))];
        let p = glowkit_core::ease(glowkit_core::Easing::OutCubic, (glowkit_core::anim::Clock::elapsed(ui, id.with("tabclock")) / 0.45).min(1.0));
        if p < 1.0 {
            ui.ctx().request_repaint();
        }
        let panel = |ui: &mut Ui| {
            card(ui, id.with("tabcard"), CardLook::of(&t).pad(if sec.narrow { 20.0 } else { 40.0 }).hover(false), |ui| {
                two_col(
                    ui,
                    id.with("tabcols"),
                    0.44,
                    40.0,
                    sec.narrow,
                    true,
                    |ui| {
                        fade_up(ui, p, 10.0, |ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
                            icon_tile(ui.painter(), r, it.icon, t.accent, &t);
                            ui.add_space(16.0);
                            Tx::new(&it.title).size(26.0).bold().color(t.foreground).show(ui);
                            ui.add_space(8.0);
                            Tx::new(&it.description).size(15.5).line(1.6).color(t.muted_foreground).show(ui);
                            ui.add_space(16.0);
                            for b in &it.bullets {
                                check_row(ui, b, t.accent, false);
                                ui.add_space(2.0);
                            }
                        });
                    },
                    |ui| {
                        fade_up(ui, p, 16.0, |ui| {
                            let w = ui.available_width();
                            let (r, _) = ui.allocate_exact_size(vec2(w, 280.0), Sense::hover());
                            paint_art(ui.painter(), r, sel + 2, &t, (t.radius * 1.2).round(), glowkit_core::time(ui));
                            paint_mini(ui, ui.painter(), r.shrink(36.0), it.visual.resolve(sel), &t, sel, true);
                        });
                    },
                );
            });
        };
        sec.fade(ui, first + 1, panel);
    }
}

/// One bento card: illustration on top, title and text below.
fn bento_card(ui: &mut Ui, id: egui::Id, it: &FeatureItem, i: usize, t: &Theme, row_h: f32, vis_h: f32) {
    let out = card(ui, id.with(("bento", i)), CardLook::of(t).min_h(row_h).pad(20.0), |ui| {
        let w = ui.available_width();
        let (r, _) = ui.allocate_exact_size(vec2(w, vis_h), Sense::hover());
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            let (ir, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
            it.icon.paint(ui.painter(), ir, t.accent);
            Tx::new(&it.title).size(16.0).semi().color(t.foreground).show_inline(ui);
        });
        ui.add_space(4.0);
        Tx::new(&it.description).size(14.0).line(1.55).color(t.muted_foreground).show(ui);
        r
    });
    let r = out.inner;
    let p = ui.painter();
    let cr = CornerRadius::same((t.radius * 0.9) as u8);
    p.rect(r, cr, if t.dark { mix(t.card, t.muted, 0.55) } else { mix(t.card, t.muted, 0.6) }, Stroke::new(1.0, with_alpha(t.border, 0.7)), StrokeKind::Inside);
    let clip = ui.painter().with_clip_rect(r.intersect(ui.clip_rect()));
    paint_mini(ui, &clip, r.shrink(14.0), it.visual.resolve(i), t, i, true);
}

/// Paint a small looping illustration into `rect`.
pub(crate) fn paint_mini(ui: &Ui, p: &egui::Painter, rect: Rect, kind: FeatureVisual, t: &Theme, seed: usize, live: bool) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let time = if live { glowkit_core::time(ui) } else { 0.0 };
    let accent = t.chart[seed % 6];
    let faint = with_alpha(t.foreground, if t.dark { 0.09 } else { 0.08 });
    match kind {
        FeatureVisual::Chart | FeatureVisual::Auto => {
            let plot = rect;
            for k in 0..4 {
                let y = plot.min.y + k as f32 * plot.height() / 3.0;
                p.hline(plot.x_range(), y, Stroke::new(1.0, faint));
            }
            let n = 20;
            let pts: Vec<_> = (0..n)
                .map(|k| {
                    let u = k as f32 / (n - 1) as f32;
                    let v = 0.25 + 0.5 * u + 0.15 * (u * 9.0 + time * 1.2).sin() + 0.06 * (u * 23.0).sin();
                    pos2(plot.min.x + u * plot.width(), plot.max.y - v.clamp(0.05, 0.95) * plot.height())
                })
                .collect();
            let s = paint::monotone(&pts, 4);
            paint::area_fill(p, &s, plot.max.y, with_alpha(accent, 0.35), with_alpha(accent, 0.0));
            p.add(Shape::line(s.clone(), Stroke::new(2.0, accent)));
            if let Some(last) = s.last() {
                p.circle_filled(*last, 4.0, accent);
                p.circle_stroke(*last, 4.0 + (time * 2.0).fract() * 8.0, Stroke::new(1.5, with_alpha(accent, 1.0 - (time * 2.0).fract())));
            }
        }
        FeatureVisual::Orbit => {
            let c = rect.center();
            let rmax = rect.height().min(rect.width()) * 0.48;
            for (k, rr) in [0.45, 0.75, 1.0].iter().enumerate() {
                let r = rmax * rr;
                p.circle_stroke(c, r, Stroke::new(1.0, faint));
                let count = 2 + k;
                for j in 0..count {
                    let a = time * (0.5 - k as f32 * 0.12) * if k % 2 == 0 { 1.0 } else { -1.0 } + j as f32 / count as f32 * TAU;
                    let q = c + vec2(a.cos(), a.sin()) * r;
                    let col = t.chart[(k + j + seed) % 6];
                    p.circle_filled(q, 9.0, mix(t.card, col, 0.25));
                    p.circle_stroke(q, 9.0, Stroke::new(1.0, with_alpha(col, 0.7)));
                    p.circle_filled(q, 3.0, col);
                }
            }
            let tile = Rect::from_center_size(c, Vec2::splat(34.0));
            paint::soft_shadow(p, tile, 9.0, 16.0, vec2(0.0, 0.0), with_alpha(t.accent, 0.5));
            paint::gradient_rect(p, tile, 9.0, 135.0, &[t.gradient[0], t.gradient[1]]);
            UiIcon::Sparkles.paint(p, tile.shrink(9.0), Color32::WHITE);
        }
        FeatureVisual::Code => {
            let typed = (time * 1.6) % 10.0;
            let lens = [0.55, 0.8, 0.4, 0.7, 0.3, 0.62, 0.48];
            for (k, len) in lens.iter().enumerate() {
                let y = rect.min.y + 6.0 + k as f32 * 18.0;
                if y + 8.0 > rect.max.y {
                    break;
                }
                let indent = [0.0, 14.0, 28.0, 28.0, 14.0, 14.0, 0.0][k];
                let vis = ((typed - k as f32).clamp(0.0, 1.0)) * len;
                p.text(pos2(rect.min.x, y - 2.0), egui::Align2::LEFT_TOP, format!("{}", k + 1), FontId::monospace(10.0), with_alpha(t.muted_foreground, 0.5));
                let x = rect.min.x + 20.0 + indent;
                let full = rect.width() - 20.0 - indent;
                p.rect_filled(Rect::from_min_size(pos2(x, y), vec2(full * vis * 0.3, 7.0)), CornerRadius::same(3), with_alpha(t.chart[(k + seed) % 6], 0.85));
                p.rect_filled(Rect::from_min_size(pos2(x + full * vis * 0.3 + 4.0, y), vec2((full * vis * 0.7 - 4.0).max(0.0), 7.0)), CornerRadius::same(3), faint);
                if typed >= k as f32 && typed < k as f32 + 1.0 {
                    p.vline(x + full * vis + 3.0, (y - 3.0)..=(y + 10.0), Stroke::new(1.5, t.foreground));
                }
            }
        }
        FeatureVisual::Bars => {
            let n = 9;
            let gap = 8.0;
            let bw = (rect.width() - gap * (n as f32 - 1.0)) / n as f32;
            for k in 0..n {
                let v = 0.3 + 0.6 * (0.5 + 0.5 * (time * 1.3 + k as f32 * 0.8).sin()) * (0.6 + 0.4 * ((k * 7 % 5) as f32 / 4.0));
                let h = rect.height() * v;
                let r = Rect::from_min_max(pos2(rect.min.x + k as f32 * (bw + gap), rect.max.y - h), pos2(rect.min.x + k as f32 * (bw + gap) + bw, rect.max.y));
                let col = if k == n - 3 { accent } else { with_alpha(accent, 0.35) };
                p.rect_filled(r, CornerRadius { nw: 4, ne: 4, sw: 1, se: 1 }, col);
            }
            p.hline(rect.x_range(), rect.max.y + 0.5, Stroke::new(1.0, faint));
        }
        FeatureVisual::Pulse => {
            let c = rect.center();
            let rmax = rect.height().min(rect.width()) * 0.5;
            for k in 0..3 {
                let ph = ((time * 0.6) + k as f32 / 3.0).fract();
                p.circle_stroke(c, rmax * ph, Stroke::new(1.5, with_alpha(accent, (1.0 - ph) * 0.8)));
            }
            let sweep = time * 1.2;
            let pts: Vec<_> = (0..=16).map(|i| c + vec2((sweep - i as f32 * 0.04).cos(), (sweep - i as f32 * 0.04).sin()) * rmax * 0.95).collect();
            for (i, q) in pts.iter().enumerate() {
                p.line_segment([c, *q], Stroke::new(2.0, with_alpha(accent, 0.25 * (1.0 - i as f32 / 16.0))));
            }
            for (k, (u, v)) in [(0.3, 0.2), (-0.5, 0.35), (0.15, -0.55), (-0.3, -0.25)].iter().enumerate() {
                let q = c + vec2(*u, *v) * rmax;
                let blink = (0.5 + 0.5 * (time * 2.0 + k as f32).sin()).powi(2);
                p.circle_filled(q, 3.5, with_alpha(t.chart[(k + 1) % 6], 0.4 + 0.6 * blink));
            }
            p.circle_filled(c, 5.0, accent);
        }
        FeatureVisual::Toggles => {
            let n = 3;
            let rh = (rect.height() / n as f32).min(40.0);
            for k in 0..n {
                let y = rect.min.y + k as f32 * rh + rh / 2.0;
                let on_t = (time * 0.7 + k as f32 * 0.9).sin() > -0.2;
                let pv = if on_t { 1.0 } else { 0.0 };
                p.circle_filled(pos2(rect.min.x + 10.0, y), 9.0, with_alpha(t.chart[(k + seed) % 6], 0.2));
                p.rect_filled(Rect::from_min_size(pos2(rect.min.x + 28.0, y - 4.0), vec2(rect.width() * 0.4, 7.0)), CornerRadius::same(3), faint);
                let track = Rect::from_center_size(pos2(rect.max.x - 22.0, y), vec2(38.0, 22.0));
                p.rect_filled(track, CornerRadius::same(11), if on_t { accent } else { with_alpha(t.foreground, 0.15) });
                let kx = track.min.x + 11.0 + pv * 16.0;
                p.circle_filled(pos2(kx, y), 8.0, Color32::WHITE);
            }
        }
    }
    if live {
        glowkit_core::animate(ui);
    }
}
