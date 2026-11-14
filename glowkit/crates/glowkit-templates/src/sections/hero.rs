//! Hero sections: the first screen of a landing page.

use super::kit::*;
use egui::{pos2, vec2, Align, Color32, CornerRadius, CursorIcon, FontId, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use glowkit_components::{AvatarGroup, UiIcon};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// Overall arrangement of a [`HeroSection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HeroLayout {
    /// Everything centered in one column, the visual (if any) below the CTAs.
    #[default]
    Centered,
    /// Copy on the left, product visual on the right (stacks on narrow screens).
    Split,
    /// Product visual on the left, copy on the right.
    SplitReverse,
}

/// The product "screenshot" a hero shows next to or below its copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HeroVisual {
    /// No visual.
    #[default]
    None,
    /// An analytics dashboard window.
    Dashboard,
    /// A code editor window.
    Code,
    /// A chat app window.
    Chat,
}

/// The opening section of a landing page: announcement pill, big headline with an optional
/// gradient or rotating accent line, subheadline, two CTAs, social proof, a product visual and a
/// row of trust logos, over an animated background.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{HeroSection, HeroLayout, HeroVisual, SectionBackground};
/// let r = HeroSection::new("Build interfaces that glow")
///     .rotating("for", &["designers", "developers", "founders"])
///     .subheadline("Animated, themeable components for egui.")
///     .announcement("New", "v1.0 is out")
///     .primary_cta("Get started")
///     .secondary_cta("View on GitHub")
///     .layout(HeroLayout::Split)
///     .visual(HeroVisual::Dashboard)
///     .background(SectionBackground::Aurora)
///     .show(ui);
/// if r.clicked("Get started") { /* ... */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct HeroSection {
    headline: String,
    accent: Option<String>,
    rotating_prefix: String,
    rotating: Vec<String>,
    subheadline: Option<String>,
    announcement: Option<(String, String)>,
    primary: Option<String>,
    primary_style: SectionCtaStyle,
    secondary: Option<String>,
    social_proof: Option<String>,
    trust_label: String,
    logos: Vec<String>,
    layout: HeroLayout,
    visual: HeroVisual,
    background: SectionBackground,
    align: SectionAlign,
    headline_size: f32,
    min_height: f32,
    salt: Option<String>,
}

impl HeroSection {
    /// A hero with the given headline (defaults: centered, glow background, no CTAs).
    pub fn new(headline: impl Into<String>) -> Self {
        Self {
            headline: headline.into(),
            accent: None,
            rotating_prefix: String::new(),
            rotating: Vec::new(),
            subheadline: None,
            announcement: None,
            primary: None,
            primary_style: SectionCtaStyle::Primary,
            secondary: None,
            social_proof: None,
            trust_label: "Trusted by fast-moving teams".into(),
            logos: Vec::new(),
            layout: HeroLayout::Centered,
            visual: HeroVisual::None,
            background: SectionBackground::Glow,
            align: SectionAlign::Center,
            headline_size: 60.0,
            min_height: 0.0,
            salt: None,
        }
    }
    /// Second headline line painted with the theme gradient.
    pub fn accent(mut self, text: impl Into<String>) -> Self {
        self.accent = Some(text.into());
        self
    }
    /// Second headline line: `prefix` followed by a pill that cycles through `words`.
    pub fn rotating(mut self, prefix: impl Into<String>, words: &[&str]) -> Self {
        self.rotating_prefix = prefix.into();
        self.rotating = words.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Supporting paragraph under the headline.
    pub fn subheadline(mut self, text: impl Into<String>) -> Self {
        self.subheadline = Some(text.into());
        self
    }
    /// Announcement pill above the headline: a small `badge` and a `text` (clicks report `text`).
    pub fn announcement(mut self, badge: impl Into<String>, text: impl Into<String>) -> Self {
        self.announcement = Some((badge.into(), text.into()));
        self
    }
    /// Primary call-to-action label.
    pub fn primary_cta(mut self, label: impl Into<String>) -> Self {
        self.primary = Some(label.into());
        self
    }
    /// Style of the primary CTA (default solid; try [`SectionCtaStyle::Shimmer`]).
    pub fn primary_style(mut self, s: SectionCtaStyle) -> Self {
        self.primary_style = s;
        self
    }
    /// Secondary (outline) call-to-action label.
    pub fn secondary_cta(mut self, label: impl Into<String>) -> Self {
        self.secondary = Some(label.into());
        self
    }
    /// Avatars + stars + this text under the CTAs (e.g. "Loved by 12,000+ developers").
    pub fn social_proof(mut self, text: impl Into<String>) -> Self {
        self.social_proof = Some(text.into());
        self
    }
    /// Company names for the trust-logo row.
    pub fn logos(mut self, names: &[&str]) -> Self {
        self.logos = names.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Caption above the trust logos.
    pub fn trust_label(mut self, text: impl Into<String>) -> Self {
        self.trust_label = text.into();
        self
    }
    /// Layout variant.
    pub fn layout(mut self, l: HeroLayout) -> Self {
        self.layout = l;
        self
    }
    /// Product visual.
    pub fn visual(mut self, v: HeroVisual) -> Self {
        self.visual = v;
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Copy alignment for the centered layout.
    pub fn align(mut self, a: SectionAlign) -> Self {
        self.align = a;
        self
    }
    /// Headline font size on wide screens (scaled down on narrow ones).
    pub fn headline_size(mut self, s: f32) -> Self {
        self.headline_size = s;
        self
    }
    /// Minimum section height.
    pub fn min_height(mut self, h: f32) -> Self {
        self.min_height = h;
        self
    }
    /// Distinguish several heroes in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the hero.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "hero", &self.salt);
        let frame = Frame::new(id, self.background)
            .pad(if self.layout == HeroLayout::Centered { 104.0 } else { 88.0 })
            .min_h(self.min_height);
        section(ui, frame, |ui, sec| {
            let split = self.layout != HeroLayout::Centered;
            let stack = sec.narrow;
            if split {
                let mut clicked = None;
                let copy = |ui: &mut Ui| clicked = self.copy(ui, sec, Align::Min, 0).1;
                let visual = |ui: &mut Ui| {
                    sec.fade(ui, 3, |ui| self.visual_box(ui, (ui.available_width() * 0.72).clamp(260.0, 420.0)));
                };
                if self.layout == HeroLayout::SplitReverse && !stack {
                    two_col(ui, id.with("split"), 0.52, 56.0, stack, true, visual, copy);
                } else {
                    two_col(ui, id.with("split"), 0.48, 56.0, stack, true, copy, visual);
                }
                if let Some(c) = clicked {
                    sec.click(c);
                }
            } else {
                let align = self.align.align();
                let (next, clicked) = self.copy(ui, sec, align, 0);
                if let Some(c) = clicked {
                    sec.click(c);
                }
                if self.visual != HeroVisual::None {
                    ui.add_space(56.0);
                    let h = (sec.w * 0.5).clamp(240.0, 460.0);
                    sec.fade(ui, next, |ui| self.visual_box(ui, h));
                }
            }
            if !self.logos.is_empty() {
                ui.add_space(if sec.narrow { 48.0 } else { 72.0 });
                let align = if split { Align::Center } else { self.align.align() };
                sec.fade(ui, 6, |ui| trust_logos(ui, id, &self.trust_label, &self.logos, align));
            }
        })
    }

    /// The copy column; returns the next stagger index.
    fn copy(&self, ui: &mut Ui, sec: &Sec, align: Align, first: usize) -> (usize, Option<String>) {
        let mut out_click = None;
        let t = sec.t.clone();
        let id = sec.id;
        let mut i = first;
        let hs = if sec.narrow {
            (self.headline_size * 0.62).round().max(34.0)
        } else if self.layout != HeroLayout::Centered {
            (self.headline_size * 0.86).round()
        } else {
            self.headline_size
        };
        if let Some((badge, text)) = &self.announcement {
            if sec
                .fade(ui, i, |ui| row(ui, id.with("ann"), align, 0.0, |ui| announcement_pill(ui, badge, text)))
                .clicked()
            {
                out_click = Some(text.clone());
            }
            ui.add_space(24.0);
            i += 1;
        }
        let max_w = if self.layout == HeroLayout::Centered { 860.0 } else { f32::INFINITY };
        sec.fade(ui, i, |ui| {
            Tx::new(&self.headline)
                .size(hs)
                .bold()
                .line(1.08)
                .align(align)
                .max_width(max_w)
                .color(t.foreground)
                .show(ui);
            if let Some(a) = &self.accent {
                Tx::new(a)
                    .size(hs)
                    .bold()
                    .line(1.12)
                    .align(align)
                    .max_width(max_w)
                    .gradient(&[t.gradient[0], t.gradient[1], t.gradient[2]])
                    .show(ui);
            }
            if !self.rotating.is_empty() {
                ui.add_space(6.0);
                row(ui, id.with("rot"), align, hs * 0.22, |ui| {
                    if !self.rotating_prefix.is_empty() {
                        Tx::new(&self.rotating_prefix).size(hs).bold().color(t.foreground).show_inline(ui);
                    }
                    ui.add(
                        glowkit_text::RotatingText::new(self.rotating.iter().cloned())
                            .size(hs * 0.9)
                            .pill(true)
                            .pill_color(t.accent)
                            .interval(2.4),
                    );
                });
            }
        });
        i += 1;
        if let Some(s) = &self.subheadline {
            ui.add_space(24.0);
            sec.fade(ui, i, |ui| {
                Tx::new(s)
                    .size(if sec.narrow { 16.0 } else { 19.0 })
                    .line(1.55)
                    .align(align)
                    .max_width(620.0)
                    .color(t.muted_foreground)
                    .show(ui);
            });
            i += 1;
        }
        if self.primary.is_some() || self.secondary.is_some() {
            ui.add_space(36.0);
            let mut clicked = None;
            sec.fade(ui, i, |ui| {
                row(ui, id.with("ctas"), align, 12.0, |ui| {
                    if let Some(p) = &self.primary {
                        if cta_button(ui, p, self.primary_style, true).clicked() {
                            clicked = Some(p.clone());
                        }
                    }
                    if let Some(s) = &self.secondary {
                        if cta_button(ui, s, SectionCtaStyle::Outline, true).clicked() {
                            clicked = Some(s.clone());
                        }
                    }
                });
            });
            if clicked.is_some() {
                out_click = clicked;
            }
            i += 1;
        }
        if let Some(sp) = &self.social_proof {
            ui.add_space(32.0);
            sec.fade(ui, i, |ui| {
                row(ui, id.with("proof"), align, 12.0, |ui| {
                    ui.add(
                        AvatarGroup::new(&["Ava Chen", "Liam Ortiz", "Mia Park", "Noah Smith", "Zoe King"])
                            .size(30.0)
                            .ring(t.background),
                    );
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        stars(ui, 5, 12.0);
                        Tx::new(sp).size(13.0).color(t.muted_foreground).show_inline(ui);
                    });
                });
            });
            i += 1;
        }
        (i, out_click)
    }

    fn visual_box(&self, ui: &mut Ui, h: f32) {
        let t = Theme::of(ui);
        let kind = match self.visual {
            HeroVisual::None | HeroVisual::Dashboard => 0,
            HeroVisual::Code => 1,
            HeroVisual::Chat => 2,
        };
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        if self.layout == HeroLayout::Centered {
            // subtle beam around the centered screenshot
            paint_mockup(ui, rect, kind, &t);
            glowkit_effects::BorderBeam::new().radius((t.radius * 1.4).round()).duration(8.0).paint(ui, rect);
        } else {
            // a floating secondary card for depth
            let back = rect.translate(vec2(24.0, -20.0)).shrink(12.0);
            ui.painter().rect(
                back,
                CornerRadius::same((t.radius * 1.4) as u8),
                with_alpha(t.card, 0.6),
                Stroke::new(1.0, with_alpha(t.border, 0.6)),
                StrokeKind::Inside,
            );
            paint_mockup(ui, rect, kind, &t);
            floating_stat(ui, rect, &t);
        }
    }
}

/// A small floating KPI chip over the corner of a split-hero mockup.
fn floating_stat(ui: &Ui, rect: Rect, t: &Theme) {
    let time = glowkit_core::time(ui);
    let bob = (time * 1.4).sin() * 4.0;
    let chip = Rect::from_min_size(pos2(rect.min.x - 18.0, rect.max.y - 54.0 + bob), vec2(170.0, 58.0));
    let painter = ui.painter();
    paint::soft_shadow(
        painter,
        chip,
        12.0,
        20.0,
        vec2(0.0, 8.0),
        with_alpha(Color32::BLACK, if t.dark { 0.5 } else { 0.14 }),
    );
    painter.rect(chip, CornerRadius::same(12), t.popover, Stroke::new(1.0, t.border), StrokeKind::Inside);
    let tile = Rect::from_min_size(chip.min + vec2(12.0, 13.0), Vec2::splat(32.0));
    icon_tile(painter, tile, UiIcon::Activity, t.success, t);
    painter.text(
        pos2(tile.max.x + 10.0, chip.min.y + 13.0),
        egui::Align2::LEFT_TOP,
        "Conversion",
        FontId::proportional(11.5),
        t.muted_foreground,
    );
    painter.text(
        pos2(tile.max.x + 10.0, chip.min.y + 29.0),
        egui::Align2::LEFT_TOP,
        "+38.2%",
        FontId::proportional(15.0),
        t.foreground,
    );
    painter.text(
        pos2(tile.max.x + 10.6, chip.min.y + 29.0),
        egui::Align2::LEFT_TOP,
        "+38.2%",
        FontId::proportional(15.0),
        t.foreground.gamma_multiply(0.6),
    );
}

/// The clickable "[New] v2 is here →" pill.
pub(crate) fn announcement_pill(ui: &mut Ui, badge: &str, text: &str) -> egui::Response {
    let t = Theme::of(ui);
    let bt = Tx::new(badge).size(11.5).semi().color(Color32::WHITE);
    let bg = bt.galley(ui, f32::INFINITY);
    let tt = Tx::new(text).size(13.0);
    let tg = tt.galley(ui, f32::INFINITY);
    let h = 30.0;
    let badge_w = if badge.is_empty() { 0.0 } else { bg.size().x + 16.0 };
    let w = 4.0 + badge_w + 10.0 + tg.size().x + 8.0 + 14.0 + 10.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    let hv = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.15);
    let painter = ui.painter();
    let fill = if t.dark { mix(t.card, t.secondary, 0.4 + 0.3 * hv) } else { mix(t.card, t.muted, 0.5 + 0.5 * hv) };
    painter.rect(
        rect,
        CornerRadius::same(99),
        fill,
        Stroke::new(1.0, mix(t.border, t.accent, 0.35 + 0.4 * hv)),
        StrokeKind::Inside,
    );
    let mut x = rect.min.x + 4.0;
    if !badge.is_empty() {
        let br = Rect::from_min_size(pos2(x, rect.min.y + 4.0), vec2(badge_w, h - 8.0));
        paint::gradient_rect(painter, br, (h - 8.0) / 2.0, 0.0, &[t.gradient[0], t.gradient[1]]);
        bt.paint(ui, &bg, pos2(br.min.x + 8.0, br.center().y - bg.size().y / 2.0), bg.size().x, Color32::WHITE);
        x = br.max.x;
    }
    x += 10.0;
    tt.paint(ui, &tg, pos2(x, rect.center().y - tg.size().y / 2.0), tg.size().x, t.foreground);
    x += tg.size().x + 6.0 + 3.0 * hv;
    UiIcon::ArrowRight.paint(
        painter,
        Rect::from_center_size(pos2(x + 7.0, rect.center().y), Vec2::splat(13.0)),
        t.muted_foreground.lerp_to_gamma(t.foreground, hv),
    );
    resp
}

/// "Trusted by" caption plus a wrapping row of muted logos that brighten on hover.
pub(crate) fn trust_logos(ui: &mut Ui, id: egui::Id, label: &str, logos: &[String], align: Align) {
    let t = Theme::of(ui);
    if !label.is_empty() {
        Tx::new(label).size(13.0).color(t.muted_foreground).align(align).show(ui);
        ui.add_space(20.0);
    }
    let _ = id;
    logo_rows(ui, logos, 22.0, 40.0, align);
}

/// One muted logo that brightens when hovered.
pub(crate) fn logo_item(ui: &mut Ui, name: &str, kind: usize, h: f32) -> egui::Response {
    let t = Theme::of(ui);
    let w = logo_width(ui, name, h);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h + 8.0), Sense::hover());
    let hv = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.2);
    let c = with_alpha(t.muted_foreground, 0.75).lerp_to_gamma(t.foreground, hv);
    paint_logo(ui.painter(), rect.left_center(), name, kind, c, h);
    resp
}

/// Logos in centered wrapping lines.
pub(crate) fn logo_rows(ui: &mut Ui, logos: &[String], h: f32, gap: f32, align: Align) {
    let widths: Vec<f32> = logos.iter().map(|n| logo_width(ui, n, h)).collect();
    wrap_rows(ui, &widths, h + 8.0, gap, 24.0, align, |ui, k| {
        logo_item(ui, &logos[k], k, h);
    });
}
