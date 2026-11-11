//! Pricing section: tiers, billing toggle with animated prices, a highlighted plan.

use super::kit::*;
use egui::{pos2, vec2, Align, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_components::{Button, ButtonVariant, ControlSize, SegmentedControl, UiIcon};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// One plan in a [`PricingSection`].
#[derive(Clone, Debug)]
pub struct PricingTier {
    /// Plan name.
    pub name: String,
    /// One-line pitch.
    pub description: String,
    /// Price per month when billed monthly.
    pub monthly: f64,
    /// Price per month when billed yearly.
    pub yearly: f64,
    /// Replaces the number (e.g. `"Custom"`).
    pub price_label: Option<String>,
    /// Included features.
    pub features: Vec<String>,
    /// Line above the features, e.g. "Everything in Pro, plus:".
    pub features_intro: Option<String>,
    /// Button label.
    pub cta: String,
    /// Draw this tier as the recommended one (beam border, badge, solid button).
    pub highlighted: bool,
    /// Badge text for the highlighted tier.
    pub badge: String,
}

impl PricingTier {
    /// A tier with a name and monthly/yearly prices (per month).
    pub fn new(name: impl Into<String>, monthly: f64, yearly: f64) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            monthly,
            yearly,
            price_label: None,
            features: Vec::new(),
            features_intro: None,
            cta: "Get started".into(),
            highlighted: false,
            badge: "Most popular".into(),
        }
    }
    /// One-line pitch.
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }
    /// Show this text instead of a number.
    pub fn price_label(mut self, l: impl Into<String>) -> Self {
        self.price_label = Some(l.into());
        self
    }
    /// Included features.
    pub fn features(mut self, f: &[&str]) -> Self {
        self.features = f.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Line above the features.
    pub fn features_intro(mut self, s: impl Into<String>) -> Self {
        self.features_intro = Some(s.into());
        self
    }
    /// Button label.
    pub fn cta(mut self, c: impl Into<String>) -> Self {
        self.cta = c.into();
        self
    }
    /// Mark as the recommended tier.
    pub fn highlighted(mut self, h: bool) -> Self {
        self.highlighted = h;
        self
    }
    /// Badge text on the highlighted tier.
    pub fn badge(mut self, b: impl Into<String>) -> Self {
        self.badge = b.into();
        self
    }

    /// Three ready-made tiers (Hobby, Pro highlighted, Enterprise).
    pub fn defaults() -> Vec<PricingTier> {
        vec![
            PricingTier::new("Hobby", 0.0, 0.0).description("For side projects and tinkering.").features(&["Up to 3 projects", "Community support", "All core components", "Dark and light themes"]).cta("Start for free"),
            PricingTier::new("Pro", 24.0, 19.0)
                .description("For professionals shipping real products.")
                .features_intro("Everything in Hobby, plus:")
                .features(&["Unlimited projects", "Premium templates", "Priority email support", "Figma kit", "Commercial license"])
                .cta("Upgrade to Pro")
                .highlighted(true),
            PricingTier::new("Enterprise", 0.0, 0.0)
                .price_label("Custom")
                .description("For organizations with advanced needs.")
                .features_intro("Everything in Pro, plus:")
                .features(&["SSO and SAML", "Dedicated success manager", "Custom SLAs", "On-premise builds"])
                .cta("Contact sales"),
        ]
    }
}

/// Arrangement of a [`PricingSection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PricingLayout {
    /// Side-by-side cards.
    #[default]
    Cards,
    /// Stacked horizontal rows (plan | features | price + button).
    Rows,
}

/// A pricing table: 2-4 tiers, a monthly/yearly toggle whose prices count to their new value,
/// feature checklists and a highlighted "Most popular" tier wrapped in a border beam.
///
/// Clicking a tier's button reports the tier **name** in [`SectionResponse::clicked`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{PricingSection, PricingTier};
/// let r = PricingSection::new("Simple pricing").tiers(PricingTier::defaults()).show(ui);
/// if r.clicked("Pro") { /* start checkout */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct PricingSection {
    title: String,
    eyebrow: Option<String>,
    subtitle: Option<String>,
    tiers: Vec<PricingTier>,
    currency: String,
    period: String,
    toggle: bool,
    yearly_default: bool,
    save_label: String,
    layout: PricingLayout,
    background: SectionBackground,
    salt: Option<String>,
}

impl PricingSection {
    /// A pricing section with the default three tiers.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            eyebrow: None,
            subtitle: None,
            tiers: PricingTier::defaults(),
            currency: "$".into(),
            period: "/month".into(),
            toggle: true,
            yearly_default: true,
            save_label: "Save 20%".into(),
            layout: PricingLayout::Cards,
            background: SectionBackground::Plain,
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
    /// The plans (2-4 look best).
    pub fn tiers(mut self, tiers: Vec<PricingTier>) -> Self {
        self.tiers = tiers;
        self
    }
    /// Currency symbol (default `"$"`).
    pub fn currency(mut self, c: impl Into<String>) -> Self {
        self.currency = c.into();
        self
    }
    /// Period suffix (default `"/month"`).
    pub fn period(mut self, p: impl Into<String>) -> Self {
        self.period = p.into();
        self
    }
    /// Show the monthly/yearly toggle (default on).
    pub fn billing_toggle(mut self, on: bool) -> Self {
        self.toggle = on;
        self
    }
    /// Start on yearly billing (default true).
    pub fn yearly(mut self, y: bool) -> Self {
        self.yearly_default = y;
        self
    }
    /// Badge next to the yearly option (empty to hide).
    pub fn save_label(mut self, s: impl Into<String>) -> Self {
        self.save_label = s.into();
        self
    }
    /// Layout variant.
    pub fn layout(mut self, l: PricingLayout) -> Self {
        self.layout = l;
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Distinguish several pricing sections in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the section.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "pricing", &self.salt);
        section(ui, Frame::new(id, self.background), |ui, sec| {
            let t = sec.t.clone();
            let mut next = Heading::new(&self.title).eyebrow(self.eyebrow.as_deref()).subtitle(self.subtitle.as_deref()).show(ui, sec, 0);
            let mut billing: usize = glowkit_core::state(ui, id.with("billing"), || usize::from(self.yearly_default));
            if self.toggle {
                ui.add_space(32.0);
                sec.fade(ui, next, |ui| {
                    row(ui, id.with("toggle"), Align::Center, 12.0, |ui| {
                        ui.add(SegmentedControl::new(&mut billing, ["Monthly", "Yearly"]).size(ControlSize::Md).equal_widths(true));
                        if !self.save_label.is_empty() {
                            save_badge(ui, &self.save_label, billing == 1);
                        }
                    });
                });
                glowkit_core::set_state(ui, id.with("billing"), billing);
                next += 1;
            }
            let yearly = billing == 1;
            ui.add_space(if sec.narrow { 32.0 } else { 48.0 });
            let mut clicked = None;
            match self.layout {
                PricingLayout::Cards => {
                    let n = self.tiers.len();
                    let cols = sec.cols(260.0, n.min(4));
                    grid(ui, id.with("tiers"), n, cols, 20.0, |ui, i, row_h| {
                        let tier = &self.tiers[i];
                        sec.fade(ui, next + i, |ui| {
                            if self.tier_card(ui, id.with(("tier", i)), tier, yearly, row_h, &t) {
                                clicked = Some(tier.name.clone());
                            }
                        });
                    });
                }
                PricingLayout::Rows => {
                    for (i, tier) in self.tiers.iter().enumerate() {
                        sec.fade(ui, next + i, |ui| {
                            if self.tier_row(ui, id.with(("row", i)), tier, yearly, sec.narrow, &t) {
                                clicked = Some(tier.name.clone());
                            }
                        });
                        ui.add_space(12.0);
                    }
                }
            }
            if let Some(c) = clicked {
                sec.click(c);
            }
        })
    }

    fn price(&self, ui: &mut Ui, tier: &PricingTier, yearly: bool, size: f32, t: &Theme) {
        // fixed-height row so buttons line up across tiers whatever the price looks like
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), (size * 1.25).round()), Sense::hover());
        let mut row_ui = ui.new_child(egui::UiBuilder::new().max_rect(r).layout(egui::Layout::left_to_right(Align::Max)));
        row_ui.spacing_mut().item_spacing.x = 6.0;
        if let Some(l) = &tier.price_label {
            Tx::new(l).size(size * 0.92).bold().color(t.foreground).show_inline(&mut row_ui);
        } else {
            let v = if yearly { tier.yearly } else { tier.monthly };
            let dec = if v.fract().abs() > 1e-6 { 2 } else { 0 };
            row_ui.add(glowkit_text::CountUp::new(v).from(v).duration(0.6).decimals(dec).prefix(&self.currency).size(size).bold(true).color(t.foreground));
            row_ui.vertical(|ui| {
                Tx::new(&self.period).size(14.0).color(t.muted_foreground).show_inline(ui);
                ui.add_space(size * 0.12);
            });
        }
        let note = if tier.price_label.is_some() {
            "Tailored to your team"
        } else if tier.monthly == 0.0 && tier.yearly == 0.0 {
            "Free forever"
        } else if yearly {
            "Billed yearly"
        } else {
            "Billed monthly"
        };
        Tx::new(note).size(12.5).color(t.muted_foreground).show(ui);
    }

    fn tier_card(&self, ui: &mut Ui, id: egui::Id, tier: &PricingTier, yearly: bool, row_h: f32, t: &Theme) -> bool {
        let hl = tier.highlighted;
        let mut clicked = false;
        let fill = if hl {
            if t.dark {
                mix(t.card, t.accent, 0.06)
            } else {
                mix(t.card, t.accent, 0.025)
            }
        } else {
            t.card
        };
        let look = CardLook::of(t).min_h(row_h).pad(28.0).fill(fill).stroke(if hl { with_alpha(t.accent, 0.5) } else { t.border });
        // glow behind the highlighted tier
        if hl {
            let prev = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), row_h.max(200.0)));
            paint::glow_ellipse(ui.painter(), prev.center_top() + vec2(0.0, 40.0), vec2(prev.width() * 0.7, 160.0), with_alpha(t.accent, if t.dark { 0.18 } else { 0.08 }));
        }
        let out = card(ui, id, look, |ui| {
            ui.horizontal(|ui| {
                Tx::new(&tier.name).size(17.0).semi().color(if hl { t.accent } else { t.foreground }).show_inline(ui);
                if hl && !tier.badge.is_empty() {
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| popular_badge(ui, &tier.badge, t));
                }
            });
            if !tier.description.is_empty() {
                ui.add_space(4.0);
                Tx::new(&tier.description).size(14.0).line(1.5).color(t.muted_foreground).show(ui);
            }
            ui.add_space(20.0);
            self.price(ui, tier, yearly, 42.0, t);
            ui.add_space(20.0);
            let b = Button::new(&tier.cta).variant(if hl { ButtonVariant::Default } else { ButtonVariant::Outline }).size(ControlSize::Lg).width(f32::INFINITY);
            let b = if hl { b.fill(t.accent) } else { b };
            if ui.add(b).clicked() {
                clicked = true;
            }
            ui.add_space(24.0);
            divider(ui, None);
            ui.add_space(16.0);
            if let Some(intro) = &tier.features_intro {
                Tx::new(intro).size(13.5).semi().color(t.foreground).show(ui);
                ui.add_space(6.0);
            }
            for f in &tier.features {
                check_row(ui, f, if hl { t.accent } else { t.success }, false);
                ui.add_space(4.0);
            }
        });
        if hl {
            glowkit_effects::BorderBeam::new().radius(look.radius).duration(7.0).thickness(1.8).paint(ui, out.rect.translate(vec2(0.0, -2.0 * out.hover)));
        }
        clicked
    }

    fn tier_row(&self, ui: &mut Ui, id: egui::Id, tier: &PricingTier, yearly: bool, narrow: bool, t: &Theme) -> bool {
        let hl = tier.highlighted;
        let mut clicked = false;
        let look = CardLook::of(t).pad(24.0).stroke(if hl { with_alpha(t.accent, 0.5) } else { t.border });
        let out = card(ui, id, look, |ui| {
            let left = |ui: &mut Ui| {
                ui.horizontal(|ui| {
                    Tx::new(&tier.name).size(18.0).semi().color(t.foreground).show_inline(ui);
                    if hl {
                        popular_badge(ui, &tier.badge, t);
                    }
                });
                Tx::new(&tier.description).size(14.0).color(t.muted_foreground).show(ui);
                ui.add_space(10.0);
                let feats: Vec<&String> = tier.features.iter().take(4).collect();
                grid(ui, id.with("f"), feats.len(), if narrow { 1 } else { 2 }, 6.0, |ui, i, _| {
                    check_row(ui, feats[i], t.success, true);
                });
            };
            let right = |ui: &mut Ui| {
                self.price(ui, tier, yearly, 34.0, t);
                ui.add_space(10.0);
                let b = Button::new(&tier.cta).variant(if hl { ButtonVariant::Default } else { ButtonVariant::Outline }).width(f32::INFINITY);
                if ui.add(if hl { b.fill(t.accent) } else { b }).clicked() {
                    clicked = true;
                }
            };
            two_col(ui, id.with("cols"), 0.66, 32.0, narrow, true, left, right);
        });
        if hl {
            glowkit_effects::BorderBeam::new().radius(look.radius).paint(ui, out.rect.translate(vec2(0.0, -2.0 * out.hover)));
        }
        clicked
    }
}

/// Gradient "Most popular" pill.
fn popular_badge(ui: &mut Ui, text: &str, t: &Theme) {
    let tx = Tx::new(text).size(11.5).semi().color(Color32::WHITE);
    let g = tx.galley(ui, f32::INFINITY);
    let size = g.size() + vec2(18.0, 8.0);
    let (r, _) = ui.allocate_exact_size(size, Sense::hover());
    paint::gradient_rect(ui.painter(), r, size.y / 2.0, 0.0, &[t.gradient[0], t.gradient[1]]);
    tx.paint(ui, &g, r.min + vec2(9.0, 4.0), g.size().x, Color32::WHITE);
}

/// "Save 20%" pill that lights up while yearly billing is selected.
fn save_badge(ui: &mut Ui, text: &str, active: bool) {
    let t = Theme::of(ui);
    let k = ui.ctx().animate_bool_with_time(ui.scope_id().with("save"), active, 0.25);
    let tx = Tx::new(text).size(12.0).semi();
    let g = tx.galley(ui, f32::INFINITY);
    let size = g.size() + vec2(28.0, 10.0);
    let (r, _) = ui.allocate_exact_size(size, Sense::hover());
    let col = mix(t.muted_foreground, t.success, k);
    ui.painter().rect(r, CornerRadius::same(99), with_alpha(col, 0.12), Stroke::new(1.0, with_alpha(col, 0.35)), StrokeKind::Inside);
    UiIcon::Sparkles.paint(ui.painter(), Rect::from_center_size(pos2(r.min.x + 11.0, r.center().y), egui::Vec2::splat(11.0)), col);
    tx.paint(ui, &g, r.min + vec2(20.0, 5.0), g.size().x, col);
}
