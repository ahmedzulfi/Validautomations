//! Testimonials, CTA banners and FAQ sections.

use super::kit::*;
use super::logos_stats::muted_or_bg;
use egui::{pos2, vec2, Align, Color32, CornerRadius, CursorIcon, FontId, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use glowkit_components::{Accordion, Avatar, AvatarStyle, UiIcon};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// One customer quote.
#[derive(Clone, Debug)]
pub struct Testimonial {
    /// The quote.
    pub quote: String,
    /// Person's name.
    pub name: String,
    /// Role and company.
    pub role: String,
    /// Star rating 0-5 (0 hides the stars).
    pub rating: usize,
}

impl Testimonial {
    /// A testimonial.
    pub fn new(quote: impl Into<String>, name: impl Into<String>, role: impl Into<String>) -> Self {
        Self { quote: quote.into(), name: name.into(), role: role.into(), rating: 5 }
    }
    /// Star rating (0 hides).
    pub fn rating(mut self, r: usize) -> Self {
        self.rating = r.min(5);
        self
    }

    /// Six placeholder testimonials.
    pub fn defaults() -> Vec<Testimonial> {
        vec![
            Testimonial::new("We replaced three UI libraries with glowkit and our app finally feels premium. The animations are buttery.", "Ava Chen", "CTO, Northwind"),
            Testimonial::new("Shipping a native desktop app and a web demo from the same Rust code is a superpower.", "Liam Ortiz", "Founder, Lumen Labs"),
            Testimonial::new("The theming is so well thought out. Light mode, dark mode, brand colors: everything just works.", "Mia Park", "Design Lead, Globex"),
            Testimonial::new("Our onboarding conversion went up 23% after we rebuilt the flow with these templates.", "Noah Smith", "PM, Initech"),
            Testimonial::new("I prototype in hours what used to take a sprint. The defaults are tasteful, the API is obvious.", "Zoe King", "Indie hacker"),
            Testimonial::new("Best developer experience I've had with an immediate-mode GUI. Period.", "Ethan Wu", "Staff Engineer, Hooli"),
        ]
    }
}

/// Arrangement of a [`TestimonialsSection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TestimonialsLayout {
    /// Quote cards in a grid.
    #[default]
    Grid,
    /// Two rows of cards scrolling in opposite directions.
    Marquee,
    /// One large quote at a time with avatar, cycling with dots.
    Featured,
}

/// Customer quotes: card grid, marquee rows, or a single featured quote.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{TestimonialsSection, TestimonialsLayout};
/// TestimonialsSection::new("Loved by builders").layout(TestimonialsLayout::Marquee).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct TestimonialsSection {
    title: String,
    eyebrow: Option<String>,
    subtitle: Option<String>,
    items: Vec<Testimonial>,
    layout: TestimonialsLayout,
    background: SectionBackground,
    salt: Option<String>,
}

impl TestimonialsSection {
    /// A testimonials section with placeholder quotes.
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), eyebrow: None, subtitle: None, items: Testimonial::defaults(), layout: TestimonialsLayout::Grid, background: SectionBackground::Plain, salt: None }
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
    /// The quotes.
    pub fn items(mut self, items: Vec<Testimonial>) -> Self {
        self.items = items;
        self
    }
    /// Layout variant.
    pub fn layout(mut self, l: TestimonialsLayout) -> Self {
        self.layout = l;
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Distinguish several testimonial sections in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the section.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "testimonials", &self.salt);
        section(ui, Frame::new(id, self.background), |ui, sec| {
            let t = sec.t.clone();
            let next = Heading::new(&self.title).eyebrow(self.eyebrow.as_deref()).subtitle(self.subtitle.as_deref()).show(ui, sec, 0);
            ui.add_space(if sec.narrow { 36.0 } else { 52.0 });
            match self.layout {
                TestimonialsLayout::Grid => {
                    let cols = sec.cols(280.0, 3);
                    grid(ui, id.with("g"), self.items.len(), cols, 20.0, |ui, i, row_h| {
                        sec.fade(ui, next + i / cols + i % cols, |ui| {
                            card(ui, id.with(("q", i)), CardLook::of(&t).min_h(row_h), |ui| quote_body(ui, &self.items[i], i, &t, f32::INFINITY));
                        });
                    });
                }
                TestimonialsLayout::Marquee => {
                    let half = self.items.len().div_ceil(2);
                    for (r, rev) in [(0usize, false), (1, true)] {
                        let slice: Vec<(usize, &Testimonial)> = self.items.iter().enumerate().skip(r * half).take(half).collect();
                        sec.fade(ui, next + r, |ui| {
                            glowkit_effects::Marquee::new().speed(28.0).reverse(rev).gap(20.0).fade(140.0).fade_color(muted_or_bg(&t, self.background)).pause_on_hover(true).id_salt(("tm", r)).show(ui, |ui| {
                                ui.spacing_mut().item_spacing.x = 20.0;
                                for (i, q) in &slice {
                                    ui.allocate_ui_with_layout(vec2(340.0, 400.0), egui::Layout::top_down(Align::Min), |ui| {
                                        ui.set_width(340.0);
                                        card(ui, id.with(("mq", r, *i)), CardLook::of(&t).min_h(196.0), |ui| quote_body(ui, q, *i, &t, f32::INFINITY));
                                    });
                                }
                            });
                        });
                        ui.add_space(20.0);
                    }
                }
                TestimonialsLayout::Featured => {
                    let n = self.items.len().max(1);
                    let period = 6.0;
                    let manual: Option<(usize, f64)> = glowkit_core::state(ui, id.with("pick"), || None);
                    let now = ui.input(|i| i.time);
                    let cur = match manual {
                        Some((k, at)) => (k + ((now - at) as f32 / period) as usize) % n,
                        None => (sec.elapsed / period) as usize % n,
                    };
                    let phase = match manual {
                        Some((_, at)) => ((now - at) as f32 % period) / period,
                        None => (sec.elapsed % period) / period,
                    };
                    ui.ctx().request_repaint();
                    let p = glowkit_core::ease(glowkit_core::Easing::OutCubic, (phase * period / 0.6).min(1.0));
                    let q = &self.items[cur.min(self.items.len() - 1)];
                    sec.fade(ui, next, |ui| {
                        fade_up(ui, p, 10.0, |ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::hover());
                            paint_quote_mark(ui.painter(), r.center_top() + vec2(0.0, 6.0), 40.0, t.accent);
                            ui.add_space(8.0);
                            Tx::new(format!("\u{201C}{}\u{201D}", q.quote)).size(if sec.narrow { 20.0 } else { 28.0 }).line(1.45).align(Align::Center).max_width(820.0).color(t.foreground).show(ui);
                            ui.add_space(32.0);
                            row(ui, id.with("who"), Align::Center, 14.0, |ui| {
                                ui.add(Avatar::new(&q.name).size(48.0).style(AvatarStyle::Solid).color(t.chart[cur % 6]));
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 2.0;
                                    Tx::new(&q.name).size(15.5).semi().color(t.foreground).show_inline(ui);
                                    Tx::new(&q.role).size(13.5).color(t.muted_foreground).show_inline(ui);
                                });
                            });
                        });
                        ui.add_space(28.0);
                        // progress dots
                        let dot_w = 8.0;
                        let active_w = 28.0;
                        let total = (n as f32 - 1.0) * (dot_w + 8.0) + active_w;
                        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 10.0), Sense::hover());
                        let mut x = r.center().x - total / 2.0;
                        for k in 0..n {
                            let w = if k == cur { active_w } else { dot_w };
                            let dr = Rect::from_min_size(pos2(x, r.min.y + 1.0), vec2(w, 8.0));
                            let resp = ui.interact(dr.expand(4.0), id.with(("dot", k)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                            ui.painter().rect_filled(dr, CornerRadius::same(4), with_alpha(t.muted_foreground, if resp.hovered() { 0.6 } else { 0.3 }));
                            if k == cur {
                                ui.painter().rect_filled(Rect::from_min_size(dr.min, vec2(w * phase, 8.0)), CornerRadius::same(4), t.accent);
                            }
                            if resp.clicked() {
                                glowkit_core::set_state(ui, id.with("pick"), Some((k, now)));
                            }
                            x += w + 8.0;
                        }
                    });
                }
            }
        })
    }
}

/// Paint a big typographic quote mark.
fn paint_quote_mark(p: &egui::Painter, center: egui::Pos2, size: f32, color: Color32) {
    p.text(center + vec2(0.0, size * 0.25), egui::Align2::CENTER_CENTER, "\u{201C}", FontId::proportional(size * 1.8), with_alpha(color, 0.8));
}

/// Stars, quote and author row (used by cards).
fn quote_body(ui: &mut Ui, q: &Testimonial, i: usize, t: &Theme, _max_w: f32) {
    if q.rating > 0 {
        stars(ui, q.rating, 13.0);
        ui.add_space(12.0);
    }
    Tx::new(&q.quote).size(15.0).line(1.6).color(t.foreground).show(ui);
    ui.add_space(18.0);
    ui.horizontal(|ui| {
        ui.add(Avatar::new(&q.name).size(38.0).style(AvatarStyle::Soft).color(t.chart[i % 6]));
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            Tx::new(&q.name).size(14.0).semi().color(t.foreground).show_inline(ui);
            Tx::new(&q.role).size(12.5).color(t.muted_foreground).show_inline(ui);
        });
    });
}

// ---------------------------------------------------------------------------------------------

/// Look of a [`CtaSection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CtaLayout {
    /// A rounded panel filled with the theme gradient and soft glows.
    #[default]
    Gradient,
    /// A dark panel on a grid with a light beam running around its border.
    Beam,
    /// Text on the left, buttons on the right, on a subtle card.
    Split,
    /// Just centered copy and buttons (put it on a background of your choice).
    Minimal,
}

/// A closing call-to-action banner.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{CtaSection, CtaLayout};
/// let r = CtaSection::new("Ready to ship?").primary_cta("Start building").layout(CtaLayout::Beam).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct CtaSection {
    title: String,
    subtitle: Option<String>,
    primary: Option<String>,
    secondary: Option<String>,
    note: Option<String>,
    layout: CtaLayout,
    background: SectionBackground,
    salt: Option<String>,
}

impl CtaSection {
    /// A CTA banner with the given title and default buttons.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: Some("Join thousands of teams building faster, more beautiful apps today.".into()),
            primary: Some("Get started free".into()),
            secondary: Some("Talk to sales".into()),
            note: None,
            layout: CtaLayout::Gradient,
            background: SectionBackground::Plain,
            salt: None,
        }
    }
    /// Paragraph under the title (`""` hides it).
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        let s = s.into();
        self.subtitle = (!s.is_empty()).then_some(s);
        self
    }
    /// Primary button label (`""` hides it).
    pub fn primary_cta(mut self, l: impl Into<String>) -> Self {
        let s = l.into();
        self.primary = (!s.is_empty()).then_some(s);
        self
    }
    /// Secondary button label (`""` hides it).
    pub fn secondary_cta(mut self, l: impl Into<String>) -> Self {
        let s = l.into();
        self.secondary = (!s.is_empty()).then_some(s);
        self
    }
    /// Small print under the buttons (e.g. "No credit card required").
    pub fn note(mut self, n: impl Into<String>) -> Self {
        self.note = Some(n.into());
        self
    }
    /// Layout variant.
    pub fn layout(mut self, l: CtaLayout) -> Self {
        self.layout = l;
        self
    }
    /// Section background (behind the panel).
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Distinguish several CTA sections in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the section.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "cta", &self.salt);
        let bg = if self.layout == CtaLayout::Minimal && self.background == SectionBackground::Plain { SectionBackground::Glow } else { self.background };
        section(ui, Frame::new(id, bg).pad(72.0), |ui, sec| {
            let t = sec.t.clone();
            let mut clicked = None;
            let on_color = self.layout == CtaLayout::Gradient;
            let fg = if on_color { Color32::WHITE } else { t.foreground };
            let fg2 = if on_color { with_alpha(Color32::WHITE, 0.82) } else { t.muted_foreground };
            let buttons = |ui: &mut Ui, align: Align, clicked: &mut Option<String>| {
                row(ui, id.with(("btns", align as u8)), align, 12.0, |ui| {
                    if let Some(p) = &self.primary {
                        let r = if on_color {
                            ui.add(glowkit_components::Button::new(p).size(glowkit_components::ControlSize::Lg).fill(Color32::WHITE).trailing_icon(UiIcon::ArrowRight))
                        } else {
                            cta_button(ui, p, if self.layout == CtaLayout::Beam { SectionCtaStyle::Shimmer } else { SectionCtaStyle::Primary }, true)
                        };
                        if r.clicked() {
                            *clicked = Some(p.clone());
                        }
                    }
                    if let Some(s) = &self.secondary {
                        let r = if on_color {
                            ui.add(glowkit_components::Button::new(s).size(glowkit_components::ControlSize::Lg).fill(mix(t.gradient[0], Color32::BLACK, 0.3)))
                        } else {
                            cta_button(ui, s, SectionCtaStyle::Outline, true)
                        };
                        if r.clicked() {
                            *clicked = Some(s.clone());
                        }
                    }
                });
            };
            let copy = |ui: &mut Ui, align: Align, size: f32| {
                Tx::new(&self.title).size(size).bold().line(1.15).align(align).max_width(720.0).color(fg).show(ui);
                if let Some(s) = &self.subtitle {
                    ui.add_space(12.0);
                    Tx::new(s).size(16.5).line(1.55).align(align).max_width(600.0).color(fg2).show(ui);
                }
            };
            let title_size = if sec.narrow { 28.0 } else { 40.0 };
            match self.layout {
                CtaLayout::Gradient | CtaLayout::Beam => {
                    sec.fade(ui, 0, |ui| {
                        let origin = ui.cursor().min;
                        let w = ui.available_width();
                        let prev_h: f32 = glowkit_core::state(ui, id.with("panel"), || 300.0);
                        let panel = Rect::from_min_size(origin, vec2(w, prev_h));
                        let r = (t.radius * 2.0).round();
                        let painter = ui.painter().clone();
                        if self.layout == CtaLayout::Gradient {
                            paint::soft_shadow(&painter, panel, r, 40.0, vec2(0.0, 16.0), with_alpha(t.gradient[0], if t.dark { 0.35 } else { 0.25 }));
                            paint::gradient_rect(&painter, panel, r, 120.0, &[t.gradient[0], t.gradient[1], t.gradient[2]]);
                            let clip = painter.with_clip_rect(panel.intersect(ui.clip_rect()));
                            let time = glowkit_core::time(ui);
                            paint::glow(&clip, panel.left_top() + vec2(panel.width() * (0.2 + 0.05 * (time * 0.4).sin()), 20.0), panel.height() * 0.9, with_alpha(Color32::WHITE, 0.18));
                            paint::glow(&clip, panel.right_bottom() - vec2(80.0, 10.0), panel.height(), with_alpha(Color32::BLACK, 0.18));
                            // faint grid on the gradient
                            let step = 32.0;
                            let mut x = panel.min.x + step;
                            while x < panel.max.x {
                                clip.vline(x, panel.y_range(), Stroke::new(1.0, with_alpha(Color32::WHITE, 0.06)));
                                x += step;
                            }
                            let mut y = panel.min.y + step;
                            while y < panel.max.y {
                                clip.hline(panel.x_range(), y, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.06)));
                                y += step;
                            }
                            glowkit_core::animate(ui);
                        } else {
                            let fill = if t.dark { mix(t.background, t.card, 0.8) } else { t.card };
                            painter.rect(panel, CornerRadius::same(r as u8), fill, Stroke::new(1.0, t.border), StrokeKind::Inside);
                            glowkit_backgrounds::DotGrid::new().id_salt(id.with("dots")).transparent().gap(20.0).dot_size(1.4).auto_hover(false).paint(ui, panel.shrink(2.0));
                            paint::glow_ellipse(&painter, panel.center_bottom(), vec2(panel.width() * 0.45, panel.height() * 0.7), with_alpha(t.accent, if t.dark { 0.22 } else { 0.1 }));
                            glowkit_effects::BorderBeam::new().radius(r).duration(6.0).thickness(2.0).paint(ui, panel);
                            glowkit_effects::BorderBeam::new().radius(r).duration(6.0).delay(3.0).thickness(2.0).paint(ui, panel);
                        }
                        let pad = if sec.narrow { 28.0 } else { 64.0 };
                        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(origin + vec2(pad, pad), vec2(w - pad * 2.0, f32::INFINITY))).layout(egui::Layout::top_down(Align::Min)));
                        copy(&mut child, Align::Center, title_size);
                        child.add_space(28.0);
                        buttons(&mut child, Align::Center, &mut clicked);
                        if let Some(n) = &self.note {
                            child.add_space(14.0);
                            Tx::new(n).size(12.5).align(Align::Center).color(fg2).show(&mut child);
                        }
                        let h = child.min_rect().max.y - origin.y + pad;
                        if (h - prev_h).abs() > 0.5 {
                            glowkit_core::set_state(ui, id.with("panel"), h);
                            ui.ctx().request_repaint();
                        }
                        ui.advance_cursor_after_rect(Rect::from_min_size(origin, vec2(w, h)));
                    });
                }
                CtaLayout::Split => {
                    sec.fade(ui, 0, |ui| {
                        card(ui, id.with("split"), CardLook::of(&t).pad(if sec.narrow { 24.0 } else { 40.0 }).hover(false), |ui| {
                            let r = ui.max_rect();
                            paint::glow_ellipse(ui.painter(), pos2(r.max.x, r.min.y), vec2(300.0, 160.0), with_alpha(t.accent, if t.dark { 0.14 } else { 0.07 }));
                            two_col(ui, id.with("cols"), 0.62, 32.0, sec.narrow, true, |ui| copy(ui, Align::Min, title_size * 0.8), |ui| buttons(ui, if sec.narrow { Align::Min } else { Align::Max }, &mut clicked));
                        });
                    });
                }
                CtaLayout::Minimal => {
                    sec.fade(ui, 0, |ui| copy(ui, Align::Center, title_size));
                    ui.add_space(28.0);
                    sec.fade(ui, 1, |ui| buttons(ui, Align::Center, &mut clicked));
                    if let Some(n) = &self.note {
                        ui.add_space(14.0);
                        Tx::new(n).size(12.5).align(Align::Center).color(fg2).show(ui);
                    }
                }
            }
            if let Some(c) = clicked {
                sec.click(c);
            }
        })
    }
}

// ---------------------------------------------------------------------------------------------

/// Arrangement of a [`FaqSection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FaqLayout {
    /// Centered heading over an accordion.
    #[default]
    Centered,
    /// Heading and contact prompt on the left, accordion on the right.
    TwoColumn,
    /// All questions and answers visible in a two-column grid.
    Grid,
}

/// Frequently asked questions.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::FaqSection;
/// FaqSection::new("Questions?").item("Is it free?", "Yes, MIT licensed.").show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct FaqSection {
    title: String,
    eyebrow: Option<String>,
    subtitle: Option<String>,
    items: Vec<(String, String)>,
    layout: FaqLayout,
    contact: Option<(String, String)>,
    background: SectionBackground,
    salt: Option<String>,
}

impl FaqSection {
    /// A FAQ section with placeholder questions (calling [`FaqSection::item`] replaces them).
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            eyebrow: None,
            subtitle: None,
            items: vec![
                ("Is glowkit free to use?".into(), "Yes. glowkit is open source under the MIT license, free for personal and commercial projects.".into()),
                ("Does it work on the web?".into(), "Every component compiles to WebAssembly and runs in any modern browser, as well as natively on Windows, macOS and Linux.".into()),
                ("Can I customize the theme?".into(), "Every color, radius and gradient comes from a theme token. Install one of the presets or build your own in a few lines.".into()),
                ("How big is the binary?".into(), "Components are pure Rust on top of egui with no extra dependencies. Unused components are removed by the compiler.".into()),
                ("Do you offer support?".into(), "Community support is available on GitHub. Pro and Enterprise plans include priority support with guaranteed response times.".into()),
            ],
            layout: FaqLayout::Centered,
            contact: Some(("Still have questions?".into(), "Contact support".into())),
            background: SectionBackground::Plain,
            salt: None,
        }
    }
    /// Add a question and answer (the first call replaces the placeholders).
    pub fn item(mut self, q: impl Into<String>, a: impl Into<String>) -> Self {
        if !self.items.is_empty() && self.items[0].0 == "Is glowkit free to use?" {
            self.items.clear();
        }
        self.items.push((q.into(), a.into()));
        self
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
    /// Layout variant.
    pub fn layout(mut self, l: FaqLayout) -> Self {
        self.layout = l;
        self
    }
    /// The "still have questions" prompt and its link label (clicks report the link label).
    pub fn contact(mut self, prompt: impl Into<String>, link: impl Into<String>) -> Self {
        self.contact = Some((prompt.into(), link.into()));
        self
    }
    /// Hide the contact prompt.
    pub fn no_contact(mut self) -> Self {
        self.contact = None;
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Distinguish several FAQ sections in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the section.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "faq", &self.salt);
        section(ui, Frame::new(id, self.background), |ui, sec| {
            let t = sec.t.clone();
            let mut clicked = None;
            let contact = |ui: &mut Ui, align: Align, clicked: &mut Option<String>| {
                if let Some((prompt, link_label)) = &self.contact {
                    row(ui, id.with(("contact", align as u8)), align, 6.0, |ui| {
                        Tx::new(prompt).size(14.5).color(t.muted_foreground).show_inline(ui);
                        if link(ui, link_label, 14.5, t.accent, t.foreground).clicked() {
                            *clicked = Some(link_label.clone());
                        }
                    });
                }
            };
            let accordion = |ui: &mut Ui| {
                let mut acc = Accordion::new(id.with("acc")).card(true).default_open(Some(0)).width(ui.available_width());
                for (q, a) in &self.items {
                    acc = acc.item_text(q, a);
                }
                ui.add(acc);
            };
            match self.layout {
                FaqLayout::Centered => {
                    let next = Heading::new(&self.title).eyebrow(self.eyebrow.as_deref()).subtitle(self.subtitle.as_deref()).show(ui, sec, 0);
                    ui.add_space(40.0);
                    sec.fade(ui, next, |ui| {
                        let w = ui.available_width().min(760.0);
                        let x = ui.cursor().min.x + (ui.available_width() - w) / 2.0;
                        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos2(x, ui.cursor().min.y), vec2(w, f32::INFINITY))).layout(egui::Layout::top_down(Align::Min)));
                        accordion(&mut child);
                        let used = child.min_rect();
                        ui.advance_cursor_after_rect(Rect::from_min_max(pos2(ui.cursor().min.x, used.min.y), used.max));
                    });
                    ui.add_space(32.0);
                    sec.fade(ui, next + 1, |ui| contact(ui, Align::Center, &mut clicked));
                }
                FaqLayout::TwoColumn => {
                    two_col(
                        ui,
                        id.with("2c"),
                        0.36,
                        56.0,
                        sec.narrow,
                        false,
                        |ui| {
                            Heading::new(&self.title).eyebrow(self.eyebrow.as_deref()).subtitle(self.subtitle.as_deref()).align(Align::Min).size(34.0).show(ui, sec, 0);
                            ui.add_space(24.0);
                            sec.fade(ui, 3, |ui| contact(ui, Align::Min, &mut clicked));
                        },
                        |ui| sec.fade(ui, 2, accordion),
                    );
                }
                FaqLayout::Grid => {
                    let next = Heading::new(&self.title).eyebrow(self.eyebrow.as_deref()).subtitle(self.subtitle.as_deref()).show(ui, sec, 0);
                    ui.add_space(48.0);
                    let cols = sec.cols(320.0, 2);
                    grid(ui, id.with("g"), self.items.len(), cols, 36.0, |ui, i, _| {
                        let (q, a) = &self.items[i];
                        sec.fade(ui, next + i / cols, |ui| {
                            ui.horizontal_top(|ui| {
                                let (r, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                                icon_tile(ui.painter(), r, UiIcon::Info, t.accent, &t);
                                ui.vertical(|ui| {
                                    Tx::new(q).size(16.0).semi().color(t.foreground).show(ui);
                                    ui.add_space(2.0);
                                    Tx::new(a).size(14.5).line(1.6).color(t.muted_foreground).show(ui);
                                });
                            });
                        });
                    });
                    ui.add_space(40.0);
                    sec.fade(ui, next + 3, |ui| contact(ui, Align::Center, &mut clicked));
                }
            }
            if let Some(c) = clicked {
                sec.click(c);
            }
        })
    }
}
