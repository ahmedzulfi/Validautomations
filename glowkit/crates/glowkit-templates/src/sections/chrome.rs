//! Page chrome: header / navbar, footer and the announcement bar.

use super::kit::*;
use super::people::email_form;
use egui::{pos2, vec2, Align, Color32, CornerRadius, CursorIcon, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use glowkit_components::{Button, ButtonVariant, UiIcon};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// Look of a [`HeaderSection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HeaderLayout {
    /// Full-width bar with a hairline bottom border; links centered.
    #[default]
    Bar,
    /// A floating rounded pill with a soft shadow, centered on the page.
    Floating,
    /// Brand on the left, links right next to it, CTAs on the right.
    Left,
}

/// The site header / navbar: brand, links with a gliding hover highlight, sign-in link and a
/// CTA. Collapses into a hamburger menu on narrow screens.
///
/// Clicked links and CTAs are reported by label; the active link is remembered.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{HeaderSection, HeaderLayout};
/// let r = HeaderSection::new("Glowkit").links(&["Product", "Pricing", "Docs"]).cta("Get started").layout(HeaderLayout::Floating).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct HeaderSection {
    brand: String,
    links: Vec<String>,
    sign_in: Option<String>,
    cta: Option<String>,
    layout: HeaderLayout,
    background: SectionBackground,
    badge: Option<(usize, String)>,
    salt: Option<String>,
}

impl HeaderSection {
    /// A header with the brand name and default links.
    pub fn new(brand: impl Into<String>) -> Self {
        Self {
            brand: brand.into(),
            links: ["Product", "Features", "Pricing", "Customers", "Docs"].iter().map(|s| s.to_string()).collect(),
            sign_in: Some("Sign in".into()),
            cta: Some("Get started".into()),
            layout: HeaderLayout::Bar,
            background: SectionBackground::Plain,
            badge: None,
            salt: None,
        }
    }
    /// Navigation links.
    pub fn links(mut self, l: &[&str]) -> Self {
        self.links = l.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Sign-in link label (`""` hides it).
    pub fn sign_in(mut self, s: impl Into<String>) -> Self {
        let s = s.into();
        self.sign_in = (!s.is_empty()).then_some(s);
        self
    }
    /// CTA button label (`""` hides it).
    pub fn cta(mut self, c: impl Into<String>) -> Self {
        let s = c.into();
        self.cta = (!s.is_empty()).then_some(s);
        self
    }
    /// A small "New" style badge after link number `index`.
    pub fn link_badge(mut self, index: usize, text: impl Into<String>) -> Self {
        self.badge = Some((index, text.into()));
        self
    }
    /// Layout variant.
    pub fn layout(mut self, l: HeaderLayout) -> Self {
        self.layout = l;
        self
    }
    /// Background behind the header.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Distinguish several headers in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the header.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "header", &self.salt);
        let floating = self.layout == HeaderLayout::Floating;
        let frame = Frame::new(id, self.background).pad(if floating { 16.0 } else { 0.0 }).max_w(if floating { 860.0 } else { SECTION_MAX_WIDTH });
        section(ui, frame, |ui, sec| {
            let t = sec.t.clone();
            let narrow = sec.w < 760.0;
            let mut active: usize = glowkit_core::state(ui, id.with("active"), || 0);
            let mut open: bool = glowkit_core::state(ui, id.with("open"), || false);
            let h = if floating { 56.0 } else { 64.0 };
            let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
            let painter = ui.painter().clone();
            if floating {
                paint::soft_shadow(&painter, bar, h / 2.0, 24.0, vec2(0.0, 8.0), with_alpha(Color32::BLACK, if t.dark { 0.5 } else { 0.1 }));
                let fill = if t.dark { mix(t.card, t.popover, 0.5) } else { t.card };
                painter.rect(bar, CornerRadius::same((h / 2.0) as u8), with_alpha(fill, 0.92), Stroke::new(1.0, t.border), StrokeKind::Inside);
            } else {
                let full = Rect::from_min_max(pos2(bar.min.x - 2000.0, bar.max.y - 0.5), pos2(bar.max.x + 2000.0, bar.max.y + 0.5));
                ui.painter().with_clip_rect(ui.clip_rect()).rect_filled(full, 0.0, t.border);
            }
            let inner = bar.shrink2(vec2(if floating { 10.0 } else { 0.0 }, 0.0));
            let mut clicked: Option<String> = None;
            // brand
            let mut left = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(inner.min + vec2(if floating { 8.0 } else { 0.0 }, 0.0), vec2(220.0, h))).layout(egui::Layout::left_to_right(Align::Center)));
            if brand(&mut left, &self.brand, 16.0).clicked() {
                clicked = Some(self.brand.clone());
            }
            let brand_right = left.min_rect().max.x;
            // right side
            let mut right = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(inner.center().x, inner.min.y), inner.max)).layout(egui::Layout::right_to_left(Align::Center)));
            right.spacing_mut().item_spacing.x = 8.0;
            if narrow {
                let icon = if open { UiIcon::X } else { UiIcon::List };
                if right.add(Button::icon_only(icon).variant(ButtonVariant::Ghost)).clicked() {
                    open = !open;
                }
            } else {
                if let Some(c) = &self.cta {
                    let b = Button::new(c).trailing_icon(UiIcon::ArrowRight);
                    let b = if floating { b.corners(CornerRadius::same(99)) } else { b };
                    if right.add(b).clicked() {
                        clicked = Some(c.clone());
                    }
                }
                if let Some(s) = &self.sign_in {
                    if right.add(Button::new(s).variant(ButtonVariant::Ghost)).clicked() {
                        clicked = Some(s.clone());
                    }
                }
            }
            let right_left = right.min_rect().min.x;
            // links
            if !narrow && !self.links.is_empty() {
                let gap = 4.0;
                let widths: Vec<f32> = self.links.iter().enumerate().map(|(i, l)| link_w(ui, l, self.badge.as_ref().filter(|b| b.0 == i).map(|b| b.1.as_str()))).collect();
                let total: f32 = widths.iter().sum::<f32>() + gap * (widths.len() as f32 - 1.0);
                let x0 = match self.layout {
                    HeaderLayout::Left => brand_right + 28.0,
                    _ => (inner.center().x - total / 2.0).max(brand_right + 16.0).min(right_left - 16.0 - total),
                };
                let mut x = x0;
                let mut rects = Vec::new();
                for w in &widths {
                    rects.push(Rect::from_min_size(pos2(x, inner.center().y - 16.0), vec2(*w, 32.0)));
                    x += w + gap;
                }
                let hovered = rects.iter().enumerate().find_map(|(i, r)| ui.rect_contains_pointer(*r).then_some(i));
                // gliding hover highlight
                let target = hovered.map(|i| rects[i]);
                let vis = ui.ctx().animate_bool_with_time(id.with("hl_vis"), target.is_some(), 0.15);
                let last: Rect = glowkit_core::state(ui, id.with("hl_last"), || rects[0]);
                let goal = target.unwrap_or(last);
                glowkit_core::set_state(ui, id.with("hl_last"), goal);
                let cx = glowkit_core::anim::spring(ui, id.with("hlx"), goal.center().x, 400.0, 32.0);
                let cw = glowkit_core::anim::spring(ui, id.with("hlw"), goal.width(), 400.0, 32.0);
                if vis > 0.0 {
                    let hr = Rect::from_center_size(pos2(cx, goal.center().y), vec2(cw, 32.0));
                    painter.rect_filled(hr, CornerRadius::same(if floating { 16 } else { 8 }), with_alpha(t.foreground, (if t.dark { 0.07 } else { 0.05 }) * vis));
                }
                for (i, l) in self.links.iter().enumerate() {
                    let r = rects[i];
                    let resp = ui.interact(r, id.with(("link", i)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                    let hv = ui.ctx().animate_bool_with_time(resp.id, resp.hovered() || active == i, 0.15);
                    let col = t.muted_foreground.lerp_to_gamma(t.foreground, hv);
                    let tx = Tx::new(l).size(14.0);
                    let g = tx.galley(ui, f32::INFINITY);
                    tx.paint(ui, &g, pos2(r.min.x + 12.0, r.center().y - g.size().y / 2.0), g.size().x, col);
                    if let Some((_, b)) = self.badge.as_ref().filter(|b| b.0 == i) {
                        let bt = Tx::new(b).size(10.5).semi().color(t.accent);
                        let bg = bt.galley(ui, f32::INFINITY);
                        let br = Rect::from_min_size(pos2(r.min.x + 12.0 + g.size().x + 6.0, r.center().y - bg.size().y / 2.0 - 2.0), bg.size() + vec2(10.0, 4.0));
                        painter.rect_filled(br, CornerRadius::same(99), with_alpha(t.accent, 0.14));
                        bt.paint(ui, &bg, br.min + vec2(5.0, 2.0), bg.size().x, t.accent);
                    }
                    if active == i && !floating {
                        painter.hline(r.min.x + 12.0..=r.min.x + 12.0 + g.size().x, bar.max.y - 1.0, Stroke::new(2.0, t.accent));
                    } else if active == i {
                        painter.circle_filled(pos2(r.min.x + 12.0 + g.size().x / 2.0, r.max.y + 1.0), 2.0, t.accent);
                    }
                    if resp.clicked() {
                        active = i;
                        clicked = Some(l.clone());
                    }
                }
            }
            // mobile menu
            let k = ui.ctx().animate_bool_with_time(id.with("menu"), open && narrow, 0.2);
            if k > 0.0 {
                ui.add_space(8.0);
                fade_up(ui, k, -8.0, |ui| {
                    card(ui, id.with("mobile"), CardLook::of(&t).pad(10.0).hover(false), |ui| {
                        for (i, l) in self.links.iter().enumerate() {
                            let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
                            let hv = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
                            if hv > 0.0 || active == i {
                                ui.painter().rect_filled(r, CornerRadius::same(8), with_alpha(t.foreground, 0.05 * hv.max(if active == i { 1.0 } else { 0.0 })));
                            }
                            ui.painter().text(r.left_center() + vec2(12.0, 0.0), egui::Align2::LEFT_CENTER, l, egui::FontId::proportional(15.0), if active == i { t.foreground } else { t.muted_foreground });
                            UiIcon::ChevronRight.paint(ui.painter(), Rect::from_center_size(r.right_center() - vec2(16.0, 0.0), Vec2::splat(14.0)), with_alpha(t.muted_foreground, 0.6));
                            if resp.clicked() {
                                active = i;
                                open = false;
                                clicked = Some(l.clone());
                            }
                        }
                        ui.add_space(8.0);
                        if let Some(s) = &self.sign_in {
                            if ui.add(Button::new(s).variant(ButtonVariant::Outline).width(f32::INFINITY)).clicked() {
                                clicked = Some(s.clone());
                            }
                        }
                        if let Some(c) = &self.cta {
                            if ui.add(Button::new(c).width(f32::INFINITY)).clicked() {
                                clicked = Some(c.clone());
                            }
                        }
                    });
                });
            }
            glowkit_core::set_state(ui, id.with("active"), active);
            glowkit_core::set_state(ui, id.with("open"), open);
            if let Some(c) = clicked {
                sec.click(c);
            }
        })
    }
}

fn link_w(ui: &Ui, l: &str, badge: Option<&str>) -> f32 {
    let g = ui.painter().layout_no_wrap(l.to_owned(), egui::FontId::proportional(14.0), Color32::WHITE);
    let b = badge.map_or(0.0, |b| ui.painter().layout_no_wrap(b.to_owned(), egui::FontId::proportional(10.5), Color32::WHITE).size().x + 16.0);
    g.size().x + 24.0 + b
}

// ---------------------------------------------------------------------------------------------

/// Look of a [`FooterSection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FooterLayout {
    /// Brand + tagline + socials on the left, link columns on the right, bottom bar.
    #[default]
    Columns,
    /// Newsletter row on top, then columns, then a giant faded brand word.
    Newsletter,
    /// One centered column: brand, a row of links, socials, copyright.
    Simple,
}

/// The site footer: multi-column links, newsletter, socials and a bottom bar.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{FooterSection, FooterLayout};
/// let r = FooterSection::new("Glowkit").column("Product", &["Features", "Pricing"]).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct FooterSection {
    brand: String,
    tagline: String,
    columns: Vec<(String, Vec<String>)>,
    socials: Vec<SectionSocial>,
    copyright: String,
    legal: Vec<String>,
    layout: FooterLayout,
    background: SectionBackground,
    status: Option<String>,
    salt: Option<String>,
}

impl FooterSection {
    /// A footer with default columns.
    pub fn new(brand: impl Into<String>) -> Self {
        let brand = brand.into();
        Self {
            copyright: format!("© 2026 {brand}, Inc. All rights reserved."),
            brand,
            tagline: "Beautiful, animated interfaces for Rust. Native and on the web.".into(),
            columns: vec![
                ("Product".into(), ["Features", "Pricing", "Changelog", "Roadmap"].iter().map(|s| s.to_string()).collect()),
                ("Resources".into(), ["Documentation", "Guides", "Blog", "Templates"].iter().map(|s| s.to_string()).collect()),
                ("Company".into(), ["About", "Careers", "Press", "Contact"].iter().map(|s| s.to_string()).collect()),
            ],
            socials: vec![SectionSocial::X, SectionSocial::GitHub, SectionSocial::Discord, SectionSocial::YouTube],
            legal: ["Privacy", "Terms", "Cookies"].iter().map(|s| s.to_string()).collect(),
            layout: FooterLayout::Columns,
            background: SectionBackground::Plain,
            status: Some("All systems operational".into()),
            salt: None,
        }
    }
    /// Line under the brand.
    pub fn tagline(mut self, t: impl Into<String>) -> Self {
        self.tagline = t.into();
        self
    }
    /// Add a link column (the first call replaces the defaults).
    pub fn column(mut self, title: impl Into<String>, links: &[&str]) -> Self {
        if self.columns.first().is_some_and(|c| c.0 == "Product" && c.1.first().is_some_and(|l| l == "Features") && c.1.len() == 4 && self.columns.len() == 3) {
            self.columns.clear();
        }
        self.columns.push((title.into(), links.iter().map(|s| s.to_string()).collect()));
        self
    }
    /// Social icons.
    pub fn socials(mut self, s: &[SectionSocial]) -> Self {
        self.socials = s.to_vec();
        self
    }
    /// Copyright line.
    pub fn copyright(mut self, c: impl Into<String>) -> Self {
        self.copyright = c.into();
        self
    }
    /// Legal links in the bottom bar.
    pub fn legal(mut self, l: &[&str]) -> Self {
        self.legal = l.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Status pill text in the bottom bar (`""` hides it).
    pub fn status(mut self, s: impl Into<String>) -> Self {
        let s = s.into();
        self.status = (!s.is_empty()).then_some(s);
        self
    }
    /// Layout variant.
    pub fn layout(mut self, l: FooterLayout) -> Self {
        self.layout = l;
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Distinguish several footers in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the footer.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "footer", &self.salt);
        section(ui, Frame::new(id, self.background).pad(64.0).divider(true), |ui, sec| {
            let t = sec.t.clone();
            let mut clicked: Option<String> = None;
            let socials = |ui: &mut Ui, clicked: &mut Option<String>| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for s in &self.socials {
                        if social_button(ui, *s, 34.0).clicked() {
                            *clicked = Some(s.label().to_string());
                        }
                    }
                });
            };
            let columns = |ui: &mut Ui, clicked: &mut Option<String>| {
                let n = self.columns.len();
                let cols = cols_for(ui.available_width(), 130.0, 24.0, n.max(1));
                grid(ui, id.with("cols"), n, cols, 24.0, |ui, i, _| {
                    let (title, links) = &self.columns[i];
                    Tx::new(title).size(14.0).semi().color(t.foreground).show(ui);
                    ui.add_space(10.0);
                    for l in links {
                        if link(ui, l, 14.0, t.muted_foreground, t.foreground).clicked() {
                            *clicked = Some(l.clone());
                        }
                        ui.add_space(4.0);
                    }
                });
            };
            match self.layout {
                FooterLayout::Columns | FooterLayout::Newsletter => {
                    if self.layout == FooterLayout::Newsletter {
                        two_col(
                            ui,
                            id.with("nl"),
                            0.5,
                            40.0,
                            sec.narrow,
                            true,
                            |ui| {
                                Tx::new("Stay in the loop").size(22.0).bold().color(t.foreground).show(ui);
                                Tx::new("New components and templates, once a month.").size(14.5).color(t.muted_foreground).show(ui);
                            },
                            |ui| {
                                if let Some(e) = email_form(ui, id.with("nlform"), "you@company.com", "Subscribe", "Thanks! You're subscribed.", false) {
                                    sec.submitted = Some(e);
                                }
                            },
                        );
                        ui.add_space(40.0);
                        divider(ui, None);
                        ui.add_space(40.0);
                    }
                    let mut clicked_r = None;
                    two_col(
                        ui,
                        id.with("main"),
                        0.36,
                        48.0,
                        sec.narrow,
                        false,
                        |ui| {
                            if brand(ui, &self.brand, 16.0).clicked() {
                                clicked = Some(self.brand.clone());
                            }
                            ui.add_space(12.0);
                            Tx::new(&self.tagline).size(14.0).line(1.55).max_width(300.0).color(t.muted_foreground).show(ui);
                            ui.add_space(20.0);
                            socials(ui, &mut clicked);
                        },
                        |ui| columns(ui, &mut clicked_r),
                    );
                    if clicked_r.is_some() {
                        clicked = clicked_r;
                    }
                    ui.add_space(48.0);
                    divider(ui, None);
                    ui.add_space(20.0);
                    let bottom = |ui: &mut Ui, clicked: &mut Option<String>| {
                        Tx::new(&self.copyright).size(13.0).color(t.muted_foreground).show_inline(ui);
                        if let Some(s) = &self.status {
                            status_pill(ui, s, &t);
                        }
                        for l in &self.legal {
                            if link(ui, l, 13.0, t.muted_foreground, t.foreground).clicked() {
                                *clicked = Some(l.clone());
                            }
                        }
                    };
                    if sec.narrow {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = vec2(16.0, 10.0);
                            bottom(ui, &mut clicked);
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 16.0;
                            Tx::new(&self.copyright).size(13.0).color(t.muted_foreground).show_inline(ui);
                            if let Some(s) = &self.status {
                                status_pill(ui, s, &t);
                            }
                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                ui.spacing_mut().item_spacing.x = 20.0;
                                for l in self.legal.iter().rev() {
                                    if link(ui, l, 13.0, t.muted_foreground, t.foreground).clicked() {
                                        clicked = Some(l.clone());
                                    }
                                }
                            });
                        });
                    }
                    if self.layout == FooterLayout::Newsletter {
                        ui.add_space(24.0);
                        let size = (sec.w / (self.brand.chars().count().max(1) as f32 * 0.62)).min(220.0);
                        let tx = Tx::new(self.brand.to_lowercase()).size(size).bold().align(Align::Center);
                        let g = tx.galley(ui, sec.w);
                        let (r, _) = ui.allocate_exact_size(vec2(sec.w, g.size().y * 0.78), Sense::hover());
                        let top = with_alpha(t.foreground, if t.dark { 0.1 } else { 0.08 });
                        let tx = tx.gradient(&[top, top]);
                        let clip = ui.new_child(egui::UiBuilder::new().max_rect(r));
                        let mut clip = clip;
                        clip.set_clip_rect(r.intersect(ui.clip_rect()));
                        tx.paint(&clip, &g, r.min, r.width(), top);
                        paint::fade_edges_vertical(&ui.painter_at(r), Rect::from_min_max(pos2(r.min.x, r.max.y - r.height() * 0.9), r.max + vec2(0.0, r.height() * 0.9)), r.height() * 0.9, if self.background == SectionBackground::Muted { muted_band(&t) } else { t.background });
                    }
                }
                FooterLayout::Simple => {
                    ui.vertical_centered(|ui| {
                        if brand(ui, &self.brand, 16.0).clicked() {
                            clicked = Some(self.brand.clone());
                        }
                    });
                    ui.add_space(20.0);
                    let all: Vec<&String> = self.columns.iter().flat_map(|c| c.1.iter()).take(7).collect();
                    let widths: Vec<f32> = all.iter().map(|l| ui.painter().layout_no_wrap((*l).clone(), egui::FontId::proportional(14.0), Color32::WHITE).size().x).collect();
                    wrap_rows(ui, &widths, 20.0, 28.0, 10.0, Align::Center, |ui, i| {
                        if link(ui, all[i], 14.0, t.muted_foreground, t.foreground).clicked() {
                            clicked = Some(all[i].clone());
                        }
                    });
                    ui.add_space(24.0);
                    row(ui, id.with("soc"), Align::Center, 8.0, |ui| socials(ui, &mut clicked));
                    ui.add_space(24.0);
                    Tx::new(&self.copyright).size(13.0).align(Align::Center).color(t.muted_foreground).show(ui);
                }
            }
            if let Some(c) = clicked {
                sec.click(c);
            }
        })
    }
}

/// Green pulsing "All systems operational" pill.
fn status_pill(ui: &mut Ui, text: &str, t: &Theme) {
    let tx = Tx::new(text).size(12.0).color(t.muted_foreground);
    let g = tx.galley(ui, f32::INFINITY);
    let size = g.size() + vec2(30.0, 10.0);
    let (r, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect(r, CornerRadius::same(99), Color32::TRANSPARENT, Stroke::new(1.0, t.border), StrokeKind::Inside);
    let c = pos2(r.min.x + 12.0, r.center().y);
    let ph = (glowkit_core::time(ui) * 0.8).fract();
    ui.painter().circle_filled(c, 3.0 + 5.0 * ph, with_alpha(t.success, 0.4 * (1.0 - ph)));
    ui.painter().circle_filled(c, 3.0, t.success);
    tx.paint(ui, &g, r.min + vec2(22.0, 5.0), g.size().x, t.muted_foreground);
    glowkit_core::animate(ui);
}

// ---------------------------------------------------------------------------------------------

/// Look of an [`AnnouncementBar`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AnnouncementStyle {
    /// Theme gradient with white text.
    #[default]
    Gradient,
    /// Solid accent color.
    Accent,
    /// Muted bar with a shimmering message.
    Subtle,
}

/// A thin dismissible bar across the top of a page: message, link and close button.
///
/// Clicking the link reports the link label; closing reports `"dismiss"` (and hides the bar,
/// collapsing smoothly).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::AnnouncementBar;
/// AnnouncementBar::new("We just raised our Series A").link("Read more").show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct AnnouncementBar {
    text: String,
    link: Option<String>,
    style: AnnouncementStyle,
    dismissible: bool,
    icon: UiIcon,
    salt: Option<String>,
}

impl AnnouncementBar {
    /// A bar with a message.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), link: None, style: AnnouncementStyle::Gradient, dismissible: true, icon: UiIcon::Sparkles, salt: None }
    }
    /// Link label after the message.
    pub fn link(mut self, l: impl Into<String>) -> Self {
        self.link = Some(l.into());
        self
    }
    /// Visual style.
    pub fn style(mut self, s: AnnouncementStyle) -> Self {
        self.style = s;
        self
    }
    /// Show a close button (default on).
    pub fn dismissible(mut self, d: bool) -> Self {
        self.dismissible = d;
        self
    }
    /// Leading icon.
    pub fn icon(mut self, i: UiIcon) -> Self {
        self.icon = i;
        self
    }
    /// Distinguish several bars in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Reset a dismissed bar so it shows again.
    pub fn reset(ui: &Ui, salt: Option<&str>) {
        let id = match salt {
            Some(s) => ui.scope_id().with(("banner", s)),
            None => ui.scope_id().with("banner"),
        };
        glowkit_core::set_state(ui, id.with("closed"), false);
    }

    /// Show the bar.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "banner", &self.salt);
        let t = Theme::of(ui);
        let closed: bool = glowkit_core::state(ui, id.with("closed"), || false);
        let k = ui.ctx().animate_bool_with_time(id.with("vis"), !closed, 0.25);
        let full_h = 40.0;
        let h = (full_h * k).round();
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
        let mut clicked = None;
        if h >= 1.0 {
            let painter = ui.painter_at(rect);
            let (fg, link_col) = match self.style {
                AnnouncementStyle::Gradient => {
                    paint::gradient_rect(&painter, rect, 0.0, 0.0, &[t.gradient[0], t.gradient[1], t.gradient[2]]);
                    (Color32::WHITE, Color32::WHITE)
                }
                AnnouncementStyle::Accent => {
                    painter.rect_filled(rect, 0.0, t.accent);
                    let c = glowkit_core::color::contrast_text(t.accent);
                    (c, c)
                }
                AnnouncementStyle::Subtle => {
                    painter.rect_filled(rect, 0.0, muted_band(&t));
                    painter.hline(rect.x_range(), rect.max.y - 0.5, Stroke::new(1.0, t.border));
                    (t.foreground, t.accent)
                }
            };
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.translate(vec2(0.0, h - full_h))).layout(egui::Layout::left_to_right(Align::Center)));
            child.set_clip_rect(rect.intersect(ui.clip_rect()));
            child.multiply_opacity(k);
            let msg_w = child.painter().layout_no_wrap(self.text.clone(), egui::FontId::proportional(13.5), fg).size().x;
            let link_w = self.link.as_ref().map_or(0.0, |l| child.painter().layout_no_wrap(l.clone(), egui::FontId::proportional(13.5), fg).size().x + 30.0);
            let total = 22.0 + msg_w + link_w;
            child.add_space(((rect.width() - total) / 2.0).max(12.0));
            let (ir, _) = child.allocate_exact_size(Vec2::splat(15.0), Sense::hover());
            self.icon.paint(child.painter(), ir, fg);
            child.add_space(2.0);
            if self.style == AnnouncementStyle::Subtle {
                child.add(glowkit_text::ShinyText::new(&self.text).size(13.5).color(t.muted_foreground).shine_color(t.foreground));
            } else {
                Tx::new(&self.text).size(13.5).color(fg).show_inline(&mut child);
            }
            if let Some(l) = &self.link {
                child.add_space(8.0);
                let r = link(&mut child, l, 13.5, link_col, link_col);
                let (ar, _) = child.allocate_exact_size(Vec2::splat(13.0), Sense::hover());
                UiIcon::ArrowRight.paint(child.painter(), ar.translate(vec2(if r.hovered() { 2.0 } else { 0.0 }, 0.0)), link_col);
                if r.clicked() {
                    clicked = Some(l.clone());
                }
            }
            if self.dismissible {
                let cr = Rect::from_center_size(pos2(rect.max.x - 22.0, rect.center().y + (h - full_h) / 2.0), Vec2::splat(24.0));
                let resp = ui.interact(cr, id.with("x"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                if resp.hovered() {
                    painter.rect_filled(cr, CornerRadius::same(6), with_alpha(fg, 0.15));
                }
                UiIcon::X.paint(&painter, cr.shrink(6.0), with_alpha(fg, 0.85 * k));
                if resp.clicked() {
                    glowkit_core::set_state(ui, id.with("closed"), true);
                    clicked = Some("dismiss".into());
                }
            }
        }
        SectionResponse { response, clicked, submitted: None }
    }
}
