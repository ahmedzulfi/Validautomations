//! Logo clouds and stats sections.

use super::hero::logo_item;
use super::kit::*;
use egui::{pos2, vec2, Align, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use glowkit_components::UiIcon;
use glowkit_core::color::{mix, with_alpha};

/// Arrangement of a [`LogoCloudSection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LogoCloudLayout {
    /// A single wrapping row of logos.
    #[default]
    Row,
    /// Logos in a bordered grid of cells.
    Grid,
    /// Two infinite marquee rows moving in opposite directions.
    Marquee,
}

/// A "trusted by" strip of company logos.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{LogoCloudSection, LogoCloudLayout};
/// LogoCloudSection::new().title("Powering the best teams").layout(LogoCloudLayout::Marquee).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct LogoCloudSection {
    title: String,
    logos: Vec<String>,
    layout: LogoCloudLayout,
    background: SectionBackground,
    logo_height: f32,
    salt: Option<String>,
}

impl Default for LogoCloudSection {
    fn default() -> Self {
        Self::new()
    }
}

impl LogoCloudSection {
    /// A logo cloud with placeholder company names.
    pub fn new() -> Self {
        Self {
            title: "Trusted by teams at the world's most ambitious companies".into(),
            logos: ["Acme", "Globex", "Initech", "Umbrella", "Hooli", "Vandelay", "Stark", "Wayne"].iter().map(|s| s.to_string()).collect(),
            layout: LogoCloudLayout::Row,
            background: SectionBackground::Plain,
            logo_height: 22.0,
            salt: None,
        }
    }
    /// Caption above the logos (empty to hide).
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    /// Company names.
    pub fn logos(mut self, names: &[&str]) -> Self {
        self.logos = names.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Layout variant.
    pub fn layout(mut self, l: LogoCloudLayout) -> Self {
        self.layout = l;
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Height of each logo mark in points.
    pub fn logo_height(mut self, h: f32) -> Self {
        self.logo_height = h;
        self
    }
    /// Distinguish several logo clouds in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the section.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "logos", &self.salt);
        section(ui, Frame::new(id, self.background).pad(56.0), |ui, sec| {
            let t = sec.t.clone();
            if !self.title.is_empty() {
                sec.fade(ui, 0, |ui| Tx::new(&self.title).size(14.0).color(t.muted_foreground).align(Align::Center).show(ui));
                ui.add_space(28.0);
            }
            let h = self.logo_height;
            match self.layout {
                LogoCloudLayout::Row => {
                    sec.fade(ui, 1, |ui| {
                        row(ui, id.with("r"), Align::Center, 48.0, |ui| {
                            for (k, n) in self.logos.iter().enumerate() {
                                logo_item(ui, n, k, h);
                            }
                        });
                    });
                }
                LogoCloudLayout::Grid => {
                    let cols = sec.cols(160.0, 4);
                    sec.fade(ui, 1, |ui| {
                        let n = self.logos.len();
                        let w = ui.available_width();
                        let cw = w / cols as f32;
                        let rows = n.div_ceil(cols);
                        let ch = 96.0;
                        let (rect, _) = ui.allocate_exact_size(vec2(w, ch * rows as f32), Sense::hover());
                        let cr = CornerRadius::same((t.radius * 1.4) as u8);
                        ui.painter().rect(rect, cr, egui::Color32::TRANSPARENT, Stroke::new(1.0, t.border), StrokeKind::Inside);
                        for c in 1..cols {
                            ui.painter().vline(rect.min.x + c as f32 * cw, rect.y_range(), Stroke::new(1.0, t.border));
                        }
                        for r in 1..rows {
                            ui.painter().hline(rect.x_range(), rect.min.y + r as f32 * ch, Stroke::new(1.0, t.border));
                        }
                        for (k, name) in self.logos.iter().enumerate() {
                            let cell = Rect::from_min_size(pos2(rect.min.x + (k % cols) as f32 * cw, rect.min.y + (k / cols) as f32 * ch), vec2(cw, ch));
                            let resp = ui.interact(cell, id.with(("cell", k)), Sense::hover());
                            let hv = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.2);
                            if hv > 0.0 {
                                ui.painter().rect_filled(cell.shrink(1.0), 0.0, with_alpha(t.foreground, 0.025 * hv));
                            }
                            let lw = logo_width(ui, name, h);
                            let col = with_alpha(t.muted_foreground, 0.75).lerp_to_gamma(t.foreground, hv);
                            paint_logo(ui.painter(), pos2(cell.center().x - lw / 2.0, cell.center().y), name, k, col, h);
                        }
                    });
                }
                LogoCloudLayout::Marquee => {
                    for (r, rev) in [(0, false), (1, true)] {
                        sec.fade(ui, 1 + r, |ui| {
                            glowkit_effects::Marquee::new().speed(32.0).reverse(rev).gap(56.0).fade(120.0).fade_color(muted_or_bg(&t, self.background)).id_salt(("logos", r)).show(ui, |ui| {
                                ui.spacing_mut().item_spacing.x = 56.0;
                                let n = self.logos.len();
                                for k in 0..n {
                                    let kk = if rev { (k + n / 2) % n } else { k };
                                    logo_item(ui, &self.logos[kk], kk, h);
                                }
                            });
                        });
                        ui.add_space(20.0);
                    }
                }
            }
        })
    }
}

/// The color under a section with background `bg` (for marquee edge fades).
pub(crate) fn muted_or_bg(t: &glowkit_core::Theme, bg: SectionBackground) -> egui::Color32 {
    if bg == SectionBackground::Muted {
        muted_band(t)
    } else {
        t.background
    }
}

// ---------------------------------------------------------------------------------------------

/// One number in a [`StatsSection`].
#[derive(Clone, Debug)]
pub struct StatItem {
    /// Target value (counted up to when the section scrolls into view).
    pub value: f64,
    /// Text before the number, e.g. `"$"`.
    pub prefix: String,
    /// Text after the number, e.g. `"%"` or `"k+"`.
    pub suffix: String,
    /// Decimal places.
    pub decimals: usize,
    /// Short label.
    pub label: String,
    /// Optional detail line (cards layout).
    pub detail: String,
    /// Icon (cards layout).
    pub icon: UiIcon,
}

impl StatItem {
    /// A stat with a value and label.
    pub fn new(value: f64, label: impl Into<String>) -> Self {
        Self { value, prefix: String::new(), suffix: String::new(), decimals: 0, label: label.into(), detail: String::new(), icon: UiIcon::Activity }
    }
    /// Prefix such as `"$"`.
    pub fn prefix(mut self, p: impl Into<String>) -> Self {
        self.prefix = p.into();
        self
    }
    /// Suffix such as `"%"`.
    pub fn suffix(mut self, s: impl Into<String>) -> Self {
        self.suffix = s.into();
        self
    }
    /// Decimal places.
    pub fn decimals(mut self, d: usize) -> Self {
        self.decimals = d;
        self
    }
    /// Detail line.
    pub fn detail(mut self, d: impl Into<String>) -> Self {
        self.detail = d.into();
        self
    }
    /// Icon for the cards layout.
    pub fn icon(mut self, i: UiIcon) -> Self {
        self.icon = i;
        self
    }
}

/// Arrangement of a [`StatsSection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StatsLayout {
    /// Big numbers in a row separated by hairlines.
    #[default]
    Row,
    /// Each stat in a card with an icon and detail line.
    Cards,
    /// Heading on the left, a 2x2 grid of numbers on the right.
    Split,
}

/// Animated key numbers (counting up when scrolled into view).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{StatsSection, StatItem};
/// StatsSection::new()
///     .items(vec![StatItem::new(99.9, "Uptime").suffix("%").decimals(1)])
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct StatsSection {
    title: Option<String>,
    eyebrow: Option<String>,
    subtitle: Option<String>,
    items: Vec<StatItem>,
    layout: StatsLayout,
    background: SectionBackground,
    salt: Option<String>,
}

impl Default for StatsSection {
    fn default() -> Self {
        Self::new()
    }
}

impl StatsSection {
    /// A stats section with four placeholder numbers.
    pub fn new() -> Self {
        Self {
            title: None,
            eyebrow: None,
            subtitle: None,
            items: vec![
                StatItem::new(12.4, "Active developers").suffix("k+").decimals(1).icon(UiIcon::Users).detail("Across 90 countries"),
                StatItem::new(99.99, "Uptime").suffix("%").decimals(2).icon(UiIcon::Activity).detail("Over the last 12 months"),
                StatItem::new(48.0, "Components").suffix("+").icon(UiIcon::Grid).detail("And growing every week"),
                StatItem::new(2.1, "Saved per sprint").prefix("$").suffix("M").decimals(1).icon(UiIcon::Dollar).detail("Reported by customers"),
            ],
            layout: StatsLayout::Row,
            background: SectionBackground::Plain,
            salt: None,
        }
    }
    /// Optional heading.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }
    /// Small label above the heading.
    pub fn eyebrow(mut self, e: impl Into<String>) -> Self {
        self.eyebrow = Some(e.into());
        self
    }
    /// Paragraph under the heading.
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = Some(s.into());
        self
    }
    /// The numbers.
    pub fn items(mut self, items: Vec<StatItem>) -> Self {
        self.items = items;
        self
    }
    /// Layout variant.
    pub fn layout(mut self, l: StatsLayout) -> Self {
        self.layout = l;
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Distinguish several stats sections in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the section.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "stats", &self.salt);
        section(ui, Frame::new(id, self.background).pad(72.0), |ui, sec| {
            let t = sec.t.clone();
            let mut next = 0;
            let split = self.layout == StatsLayout::Split && !sec.narrow;
            if !split {
                if let Some(title) = &self.title {
                    next = Heading::new(title).eyebrow(self.eyebrow.as_deref()).subtitle(self.subtitle.as_deref()).show(ui, sec, 0);
                    ui.add_space(48.0);
                }
            }
            let seen = sec.seen;
            let number = |ui: &mut Ui, it: &StatItem, size: f32, align: Align| {
                let v = if seen { it.value } else { 0.0 };
                row(ui, ui.scope_id().with("num"), align, 0.0, |ui| {
                    ui.add(glowkit_text::CountUp::new(v).from(0.0).duration(1.6).decimals(it.decimals).prefix(&it.prefix).suffix(&it.suffix).size(size).bold(true).color(t.foreground).affix_color(t.accent));
                });
            };
            match self.layout {
                StatsLayout::Row => {
                    let cols = sec.cols(180.0, self.items.len());
                    let n = self.items.len();
                    grid(ui, id.with("g"), n, cols, 0.0, |ui, i, row_h| {
                        let it = &self.items[i];
                        let r = ui.max_rect();
                        if i % cols != 0 {
                            ui.painter().vline(r.min.x, r.min.y + 6.0..=r.min.y + row_h.max(60.0) - 6.0, Stroke::new(1.0, t.border));
                        }
                        sec.fade(ui, next + i, |ui| {
                            ui.add_space(4.0);
                            number(ui, it, if sec.narrow { 34.0 } else { 44.0 }, Align::Center);
                            ui.add_space(4.0);
                            Tx::new(&it.label).size(14.0).color(t.muted_foreground).align(Align::Center).show(ui);
                            ui.add_space(4.0);
                        });
                    });
                }
                StatsLayout::Cards => {
                    let cols = sec.cols(200.0, 4);
                    grid(ui, id.with("g"), self.items.len(), cols, 16.0, |ui, i, row_h| {
                        let it = &self.items[i];
                        sec.fade(ui, next + i, |ui| {
                            card(ui, id.with(("sc", i)), CardLook::of(&t).min_h(row_h), |ui| {
                                ui.horizontal(|ui| {
                                    Tx::new(&it.label).size(13.5).color(t.muted_foreground).show_inline(ui);
                                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                        let (r, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
                                        icon_tile(ui.painter(), r, it.icon, t.chart[i % 6], &t);
                                    });
                                });
                                ui.add_space(6.0);
                                number(ui, it, 34.0, Align::Min);
                                if !it.detail.is_empty() {
                                    ui.add_space(4.0);
                                    Tx::new(&it.detail).size(13.0).color(t.muted_foreground).show(ui);
                                }
                            });
                        });
                    });
                }
                StatsLayout::Split => {
                    let title = self.title.clone().unwrap_or_else(|| "Numbers that speak for themselves".into());
                    two_col(
                        ui,
                        id.with("split"),
                        0.42,
                        56.0,
                        sec.narrow,
                        true,
                        |ui| {
                            Heading::new(&title).eyebrow(self.eyebrow.as_deref()).subtitle(self.subtitle.as_deref()).align(Align::Min).size(34.0).show(ui, sec, 0);
                        },
                        |ui| {
                            grid(ui, id.with("g2"), self.items.len(), 2, 16.0, |ui, i, row_h| {
                                let it = &self.items[i];
                                sec.fade(ui, 2 + i, |ui| {
                                    let fill = if t.dark { mix(t.card, t.muted, 0.3) } else { t.card };
                                    card(ui, id.with(("sp", i)), CardLook::of(&t).min_h(row_h).fill(fill), |ui| {
                                        number(ui, it, 38.0, Align::Min);
                                        ui.add_space(4.0);
                                        Tx::new(&it.label).size(14.5).semi().color(t.foreground).show(ui);
                                        if !it.detail.is_empty() {
                                            Tx::new(&it.detail).size(13.0).color(t.muted_foreground).show(ui);
                                        }
                                    });
                                });
                            });
                        },
                    );
                }
            }
        })
    }
}
