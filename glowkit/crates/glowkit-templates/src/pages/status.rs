//! Status pages: 404, 500, coming soon / maintenance and thank-you.

use super::frame::{page, page_setters, PageCtx, PageOpts};
use crate::sections::*;
use egui::{pos2, vec2, Align, Color32, CornerRadius, FontId, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};
use glowkit_components::{Button, ButtonVariant, ControlSize, Input, Progress, UiIcon};
use glowkit_core::color::with_alpha;
use glowkit_core::{paint, Theme};

/// Full-height centered column on a background; content vertically centered.
fn centered_page(
    ui: &mut Ui,
    p: &mut PageCtx,
    id: egui::Id,
    bg: SectionBackground,
    max_w: f32,
    add: impl FnOnce(&mut Ui, &mut Sec),
) {
    let h = p.height;
    let r = section(ui, Frame::new(id, bg).pad(48.0).min_h(h).max_w(max_w), |ui, sec| {
        let ch: f32 = glowkit_core::state(ui, id.with("ch"), || 0.0);
        ui.add_space(((h - 96.0 - ch) / 2.0).max(0.0));
        let start = ui.cursor().min.y;
        add(ui, sec);
        let nh = ui.cursor().min.y - start;
        if (nh - ch).abs() > 0.5 {
            glowkit_core::set_state(ui, id.with("ch"), nh);
            ui.ctx().request_repaint();
        }
    });
    p.take(r);
}

/// Giant gradient status code that floats gently, with a soft reflection glow.
fn big_code(ui: &mut Ui, text: &str, sec: &Sec, glitch: bool) {
    let t = &sec.t;
    let size = if sec.narrow { 120.0 } else { 180.0 };
    if glitch {
        row(ui, sec.id.with("code"), Align::Center, 0.0, |ui| {
            ui.add(glowkit_text::GlitchText::new(text).size(size).bold(true).color(t.foreground).intensity(0.6));
        });
        return;
    }
    let tx =
        Tx::new(text).size(size).bold().align(Align::Center).gradient(&[t.gradient[0], t.gradient[1], t.gradient[2]]);
    let g = tx.galley(ui, sec.w);
    let time = glowkit_core::time(ui);
    let bob = (time * 1.3).sin() * 6.0;
    let (r, _) = ui.allocate_exact_size(vec2(sec.w, g.size().y * 0.92), Sense::hover());
    paint::glow_ellipse(
        ui.painter(),
        r.center() + vec2(0.0, g.size().y * 0.35),
        vec2(g.size().x * 0.5, 30.0),
        with_alpha(t.gradient[1], if t.dark { 0.25 } else { 0.12 }),
    );
    tx.paint(ui, &g, r.min + vec2(0.0, bob - g.size().y * 0.04), r.width(), t.foreground);
    glowkit_core::animate(ui);
}

// ---------------------------------------------------------------------------------------------

/// A 404 page: a giant floating gradient "404" over an animated background, message, search
/// field, two buttons and quick links.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::NotFoundPage;
/// let r = NotFoundPage::new().show(ui);
/// if r.clicked("Go back home") { /* route */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct NotFoundPage {
    title: String,
    message: String,
    primary: String,
    secondary: String,
    search: bool,
    links: Vec<(UiIcon, String, String)>,
    background: SectionBackground,
    opts: PageOpts,
    salt: Option<String>,
}

impl Default for NotFoundPage {
    fn default() -> Self {
        Self::new()
    }
}

impl NotFoundPage {
    /// A 404 page with default copy.
    pub fn new() -> Self {
        Self {
            title: "Lost in space".into(),
            message:
                "The page you're looking for drifted out of orbit. It may have been moved, renamed or never existed."
                    .into(),
            primary: "Go back home".into(),
            secondary: "Contact support".into(),
            search: true,
            links: vec![
                (UiIcon::File, "Documentation".into(), "Guides and API reference".into()),
                (UiIcon::Sparkles, "Blog".into(), "News and deep dives".into()),
                (UiIcon::Users, "Community".into(), "Ask the community".into()),
            ],
            background: SectionBackground::Particles,
            opts: PageOpts::default(),
            salt: None,
        }
    }
    /// Headline under the number.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    /// Explanation text.
    pub fn message(mut self, m: impl Into<String>) -> Self {
        self.message = m.into();
        self
    }
    /// Primary button label.
    pub fn primary_cta(mut self, l: impl Into<String>) -> Self {
        self.primary = l.into();
        self
    }
    /// Secondary button label.
    pub fn secondary_cta(mut self, l: impl Into<String>) -> Self {
        self.secondary = l.into();
        self
    }
    /// Show the search field (submitted queries arrive in `submitted`).
    pub fn search(mut self, on: bool) -> Self {
        self.search = on;
        self
    }
    /// Quick links as (icon, title, description); empty hides them.
    pub fn links(mut self, links: &[(UiIcon, &str, &str)]) -> Self {
        self.links = links.iter().map(|(i, a, b)| (*i, a.to_string(), b.to_string())).collect();
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    page_setters!();

    /// Show the page.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "not_found_page", &self.salt);
        page(ui, id, self.opts, |ui, p| {
            centered_page(ui, p, id.with("body"), self.background, 720.0, |ui, sec| {
                let t = sec.t.clone();
                sec.fade(ui, 0, |ui| {
                    eyebrow(ui, "Error 404", Align::Center, &t);
                    ui.add_space(8.0);
                    big_code(ui, "404", sec, false);
                });
                sec.fade(ui, 1, |ui| {
                    Tx::new(&self.title).size(30.0).bold().align(Align::Center).color(t.foreground).show(ui);
                    ui.add_space(8.0);
                    Tx::new(&self.message)
                        .size(16.0)
                        .line(1.55)
                        .align(Align::Center)
                        .max_width(520.0)
                        .color(t.muted_foreground)
                        .show(ui);
                });
                if self.search {
                    ui.add_space(24.0);
                    let mut query = None;
                    sec.fade(ui, 2, |ui| {
                        centered_column(ui, 420.0, |ui| {
                            let mut q: String = glowkit_core::state(ui, id.with("q"), String::new);
                            let r = ui.add(
                                Input::new(&mut q)
                                    .placeholder("Search the docs...")
                                    .icon(UiIcon::Search)
                                    .shortcut("Enter")
                                    .width(f32::INFINITY)
                                    .pill(true)
                                    .id_salt(id.with("search")),
                            );
                            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !q.trim().is_empty() {
                                query = Some(q.trim().to_string());
                            }
                            glowkit_core::set_state(ui, id.with("q"), q);
                        });
                    });
                    if query.is_some() {
                        sec.submitted = query;
                    }
                }
                ui.add_space(24.0);
                let mut clicked = None;
                sec.fade(ui, 3, |ui| {
                    row(ui, id.with("btns"), Align::Center, 12.0, |ui| {
                        if ui.add(Button::new(&self.primary).icon(UiIcon::ArrowLeft).size(ControlSize::Lg)).clicked() {
                            clicked = Some(self.primary.clone());
                        }
                        if ui
                            .add(Button::new(&self.secondary).variant(ButtonVariant::Outline).size(ControlSize::Lg))
                            .clicked()
                        {
                            clicked = Some(self.secondary.clone());
                        }
                    });
                });
                if !self.links.is_empty() {
                    ui.add_space(40.0);
                    let cols = sec.cols(180.0, 3);
                    let n = self.links.len();
                    grid(ui, id.with("links"), n, cols, 12.0, |ui, i, row_h| {
                        let (icon, title, desc) = &self.links[i];
                        sec.fade(ui, 4 + i, |ui| {
                            let out = card(
                                ui,
                                id.with(("l", i)),
                                CardLook::of(&t).pad(16.0).min_h(row_h).clickable(true),
                                |ui| {
                                    ui.horizontal(|ui| {
                                        let (r, _) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
                                        icon_tile(ui.painter(), r, *icon, t.accent, &t);
                                        ui.vertical(|ui| {
                                            ui.spacing_mut().item_spacing.y = 1.0;
                                            Tx::new(title).size(14.5).semi().color(t.foreground).show_inline(ui);
                                            Tx::new(desc).size(12.5).color(t.muted_foreground).show_inline(ui);
                                        });
                                    });
                                },
                            );
                            if out.response.clicked() {
                                clicked = Some(title.clone());
                            }
                        });
                    });
                }
                if let Some(c) = clicked {
                    sec.click(c);
                }
            });
        })
    }
}

// ---------------------------------------------------------------------------------------------

/// A 500 page: glitching "500", an incident card with service status rows, and a retry button
/// with a loading state.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::ServerErrorPage;
/// if ServerErrorPage::new().show(ui).clicked("Try again") { /* reload */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct ServerErrorPage {
    title: String,
    message: String,
    incident: String,
    services: Vec<(String, bool)>,
    background: SectionBackground,
    opts: PageOpts,
    salt: Option<String>,
}

impl Default for ServerErrorPage {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerErrorPage {
    /// A 500 page with default copy.
    pub fn new() -> Self {
        Self {
            title: "Something went wrong".into(),
            message: "We hit an unexpected error on our side. Our engineers have been notified and are on it.".into(),
            incident: "INC-2026-1104".into(),
            services: vec![
                ("API".into(), false),
                ("Dashboard".into(), true),
                ("Authentication".into(), true),
                ("Webhooks".into(), true),
            ],
            background: SectionBackground::Grid,
            opts: PageOpts::default(),
            salt: None,
        }
    }
    /// Headline.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    /// Explanation text.
    pub fn message(mut self, m: impl Into<String>) -> Self {
        self.message = m.into();
        self
    }
    /// Incident id shown in the status card.
    pub fn incident(mut self, i: impl Into<String>) -> Self {
        self.incident = i.into();
        self
    }
    /// Services and whether each is operational.
    pub fn services(mut self, s: &[(&str, bool)]) -> Self {
        self.services = s.iter().map(|(a, b)| (a.to_string(), *b)).collect();
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    page_setters!();

    /// Show the page.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "server_error_page", &self.salt);
        page(ui, id, self.opts, |ui, p| {
            centered_page(ui, p, id.with("body"), self.background, 640.0, |ui, sec| {
                let t = sec.t.clone();
                let mut clicked = None;
                sec.fade(ui, 0, |ui| {
                    row(ui, id.with("pill"), Align::Center, 0.0, |ui| {
                        let txt = Tx::new("Internal server error").size(12.5).semi().color(t.destructive);
                        let g = txt.galley(ui, f32::INFINITY);
                        let (r, _) = ui.allocate_exact_size(g.size() + vec2(34.0, 12.0), Sense::hover());
                        ui.painter().rect(
                            r,
                            CornerRadius::same(99),
                            with_alpha(t.destructive, 0.1),
                            Stroke::new(1.0, with_alpha(t.destructive, 0.35)),
                            StrokeKind::Inside,
                        );
                        let ph = (glowkit_core::time(ui) * 0.9).fract();
                        let c = pos2(r.min.x + 13.0, r.center().y);
                        ui.painter().circle_filled(c, 3.0 + 5.0 * ph, with_alpha(t.destructive, 0.4 * (1.0 - ph)));
                        ui.painter().circle_filled(c, 3.0, t.destructive);
                        txt.paint(ui, &g, r.min + vec2(24.0, 6.0), g.size().x, t.destructive);
                        glowkit_core::animate(ui);
                    });
                    ui.add_space(8.0);
                    big_code(ui, "500", sec, true);
                });
                ui.add_space(8.0);
                sec.fade(ui, 1, |ui| {
                    Tx::new(&self.title).size(30.0).bold().align(Align::Center).color(t.foreground).show(ui);
                    ui.add_space(8.0);
                    Tx::new(&self.message)
                        .size(16.0)
                        .line(1.55)
                        .align(Align::Center)
                        .max_width(500.0)
                        .color(t.muted_foreground)
                        .show(ui);
                });
                ui.add_space(28.0);
                let retry_at: Option<f64> = glowkit_core::state(ui, id.with("retry"), || None);
                let loading = retry_at.is_some_and(|at| ui.input(|i| i.time) - at < 1.6);
                if loading {
                    ui.ctx().request_repaint();
                }
                sec.fade(ui, 2, |ui| {
                    row(ui, id.with("btns"), Align::Center, 12.0, |ui| {
                        if ui
                            .add(
                                Button::new(if loading { "Retrying..." } else { "Try again" })
                                    .size(ControlSize::Lg)
                                    .loading(loading)
                                    .icon(UiIcon::ArrowUpDown),
                            )
                            .clicked()
                        {
                            glowkit_core::set_state(ui, id.with("retry"), Some(ui.input(|i| i.time)));
                            clicked = Some("Try again".to_string());
                        }
                        if ui
                            .add(
                                Button::new("Status page")
                                    .variant(ButtonVariant::Outline)
                                    .size(ControlSize::Lg)
                                    .trailing_icon(UiIcon::ArrowRight),
                            )
                            .clicked()
                        {
                            clicked = Some("Status page".to_string());
                        }
                    });
                });
                ui.add_space(36.0);
                sec.fade(ui, 3, |ui| {
                    card(ui, id.with("status"), CardLook::of(&t).pad(20.0).hover(false), |ui| {
                        ui.horizontal(|ui| {
                            Tx::new("Service status").size(14.5).semi().color(t.foreground).show_inline(ui);
                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                Tx::new(&self.incident).size(12.5).mono().color(t.muted_foreground).show_inline(ui);
                            });
                        });
                        ui.add_space(8.0);
                        for (k, (name, ok)) in self.services.iter().enumerate() {
                            if k > 0 {
                                divider(ui, None);
                            }
                            ui.horizontal(|ui| {
                                ui.set_min_height(28.0);
                                Tx::new(name).size(14.0).color(t.foreground).show_inline(ui);
                                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                    let (c, label) =
                                        if *ok { (t.success, "Operational") } else { (t.destructive, "Degraded") };
                                    Tx::new(label).size(13.0).color(c).show_inline(ui);
                                    let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                                    ui.painter().circle_filled(r.center(), 4.0, c);
                                    // tiny uptime bars
                                    let (br, _) = ui.allocate_exact_size(vec2(120.0, 18.0), Sense::hover());
                                    for b in 0..24 {
                                        let x = br.min.x + b as f32 * 5.0;
                                        let bad = !*ok && b >= 21 || (b * 7 + k * 3) % 23 == 0;
                                        let col = if bad {
                                            with_alpha(t.destructive, 0.8)
                                        } else {
                                            with_alpha(t.success, 0.7)
                                        };
                                        ui.painter().rect_filled(
                                            Rect::from_min_size(pos2(x, br.min.y + 2.0), vec2(3.0, 14.0)),
                                            1.0,
                                            col,
                                        );
                                    }
                                });
                            });
                        }
                    });
                });
                if let Some(c) = clicked {
                    sec.click(c);
                }
            });
        })
    }
}

// ---------------------------------------------------------------------------------------------

/// A coming-soon (or maintenance) page: countdown with sliding digits, email notify form and
/// socials over an animated background. With [`ComingSoonPage::maintenance`] it shows a
/// "we'll be back" message and progress bar instead.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::ComingSoonPage;
/// ComingSoonPage::new("Acme").launch_in(3.0 * 86400.0).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct ComingSoonPage {
    brand: String,
    title: String,
    message: String,
    seconds: f64,
    maintenance: bool,
    progress: f32,
    background: SectionBackground,
    opts: PageOpts,
    salt: Option<String>,
}

impl ComingSoonPage {
    /// A coming-soon page launching in 12 days.
    pub fn new(brand: impl Into<String>) -> Self {
        Self {
            brand: brand.into(),
            title: "Something big is coming".into(),
            message: "We're putting the finishing touches on a brand new experience. Leave your email and be the first to know.".into(),
            seconds: 12.0 * 86400.0 + 7.0 * 3600.0 + 42.0 * 60.0 + 9.0,
            maintenance: false,
            progress: 0.68,
            background: SectionBackground::Aurora,
            opts: PageOpts::default(),
            salt: None,
        }
    }
    /// Headline.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    /// Paragraph under the headline.
    pub fn message(mut self, m: impl Into<String>) -> Self {
        self.message = m.into();
        self
    }
    /// Seconds until launch, counted from when the page is first shown.
    pub fn launch_in(mut self, seconds: f64) -> Self {
        self.seconds = seconds.max(0.0);
        self
    }
    /// Maintenance mode: "We'll be back soon" with a progress bar.
    pub fn maintenance(mut self, on: bool) -> Self {
        self.maintenance = on;
        if on && self.title == "Something big is coming" {
            self.title = "We'll be back soon".into();
            self.message =
                "We're performing scheduled maintenance to make things faster. Thanks for your patience.".into();
            self.seconds = 47.0 * 60.0 + 12.0;
        }
        self
    }
    /// Maintenance progress 0..1.
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = p.clamp(0.0, 1.0);
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    page_setters!();

    /// Show the page.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "coming_soon_page", &self.salt);
        page(ui, id, self.opts, |ui, p| {
            let t_now = ui.input(|i| i.time);
            let start: f64 = glowkit_core::state(ui, id.with("start"), || t_now);
            let left = (self.seconds - (ui.input(|i| i.time) - start)).max(0.0);
            centered_page(ui, p, id.with("body"), self.background, 760.0, |ui, sec| {
                let t = sec.t.clone();
                let mut brand_hit = false;
                sec.fade(ui, 0, |ui| {
                    row(ui, id.with("brand"), Align::Center, 0.0, |ui| {
                        brand_hit = brand(ui, &self.brand, 17.0).clicked();
                    });
                });
                if brand_hit {
                    sec.click(self.brand.clone());
                }
                ui.add_space(28.0);
                let heading = Heading::new(&self.title)
                    .eyebrow(Some(if self.maintenance { "Scheduled maintenance" } else { "Launching soon" }))
                    .subtitle(Some(&self.message))
                    .size(if sec.narrow { 34.0 } else { 52.0 });
                let next = heading.show(ui, sec, 1);
                ui.add_space(36.0);
                sec.fade(ui, next, |ui| {
                    let units: Vec<(u64, &str)> = if self.maintenance {
                        vec![
                            ((left / 3600.0) as u64, "Hours"),
                            ((left / 60.0) as u64 % 60, "Minutes"),
                            (left as u64 % 60, "Seconds"),
                        ]
                    } else {
                        vec![
                            ((left / 86400.0) as u64, "Days"),
                            ((left / 3600.0) as u64 % 24, "Hours"),
                            ((left / 60.0) as u64 % 60, "Minutes"),
                            (left as u64 % 60, "Seconds"),
                        ]
                    };
                    let frac = left.fract() as f32;
                    countdown(ui, &units, frac, &t, sec.narrow);
                });
                ui.add_space(36.0);
                if self.maintenance {
                    sec.fade(ui, next + 1, |ui| {
                        centered_column(ui, 420.0, |ui| {
                            ui.horizontal(|ui| {
                                Tx::new("Upgrading databases").size(13.5).color(t.muted_foreground).show_inline(ui);
                                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                    Tx::new(format!("{}%", (self.progress * 100.0).round()))
                                        .size(13.5)
                                        .semi()
                                        .color(t.foreground)
                                        .show_inline(ui);
                                });
                            });
                            ui.add(Progress::new(self.progress));
                        });
                    });
                } else {
                    let mut sent = None;
                    sec.fade(ui, next + 1, |ui| {
                        centered_column(ui, 440.0, |ui| {
                            sent = email_form(
                                ui,
                                id.with("notify"),
                                "you@company.com",
                                "Notify me",
                                "Thanks! We'll email you on launch day.",
                                false,
                            );
                        });
                    });
                    if sent.is_some() {
                        sec.submitted = sent;
                    }
                }
                ui.add_space(28.0);
                let mut clicked = None;
                sec.fade(ui, next + 2, |ui| {
                    row(ui, id.with("soc"), Align::Center, 8.0, |ui| {
                        for s in [SectionSocial::X, SectionSocial::GitHub, SectionSocial::Discord] {
                            if social_button(ui, s, 36.0).clicked() {
                                clicked = Some(s.label().to_string());
                            }
                        }
                    });
                });
                if let Some(c) = clicked {
                    sec.click(c);
                }
            });
            glowkit_core::animate(ui);
        })
    }
}

/// Countdown boxes whose last digit slides when it changes.
fn countdown(ui: &mut Ui, units: &[(u64, &str)], frac: f32, t: &Theme, narrow: bool) {
    let bw = if narrow { 68.0 } else { 104.0 };
    let bh = bw * 0.95;
    let gap = if narrow { 10.0 } else { 18.0 };
    let total = units.len() as f32 * bw + (units.len() as f32 - 1.0) * gap;
    let (row_r, _) = ui.allocate_exact_size(vec2(ui.available_width(), bh + 26.0), Sense::hover());
    let x0 = row_r.center().x - total / 2.0;
    let painter = ui.painter();
    let font = FontId::proportional(bw * 0.42);
    for (k, (v, label)) in units.iter().enumerate() {
        let r = Rect::from_min_size(pos2(x0 + k as f32 * (bw + gap), row_r.min.y), vec2(bw, bh));
        paint::soft_shadow(
            painter,
            r,
            14.0,
            20.0,
            vec2(0.0, 8.0),
            with_alpha(Color32::BLACK, if t.dark { 0.4 } else { 0.1 }),
        );
        let fill = if t.dark { with_alpha(t.card, 0.85) } else { with_alpha(t.card, 0.92) };
        painter.rect(r, CornerRadius::same(14), fill, Stroke::new(1.0, t.border), StrokeKind::Inside);
        painter.hline(r.shrink(1.0).x_range(), r.center().y, Stroke::new(1.0, with_alpha(t.border, 0.6)));
        let clip = painter.with_clip_rect(r.shrink(2.0).intersect(ui.clip_rect()));
        // seconds slide during the first 0.35s of each second (counting down: fract goes 1 -> 0)
        let anim = k == units.len() - 1;
        let slide = if anim { ((frac - 0.65) / 0.35).clamp(0.0, 1.0) } else { 0.0 };
        let k_ease = glowkit_core::ease(glowkit_core::Easing::OutCubic, 1.0 - slide);
        let cur = format!("{v:02}");
        let prev = format!("{:02}", v + 1);
        let draw = |s: &str, dy: f32, a: f32| {
            let g = clip.layout_no_wrap(s.to_owned(), font.clone(), Color32::PLACEHOLDER);
            let pos = r.center() - g.size() / 2.0 + vec2(0.0, dy);
            clip.galley(pos, g.clone(), with_alpha(t.foreground, a));
            clip.galley(pos + vec2(1.0, 0.0), g, with_alpha(t.foreground, a));
        };
        if anim && slide > 0.0 {
            draw(&prev, -bh * 0.5 * k_ease, 1.0 - k_ease);
            draw(&cur, bh * 0.5 * (1.0 - k_ease), k_ease);
        } else {
            draw(&cur, 0.0, 1.0);
        }
        painter.text(
            pos2(r.center().x, r.max.y + 14.0),
            egui::Align2::CENTER_CENTER,
            label.to_uppercase(),
            FontId::proportional(11.0),
            t.muted_foreground,
        );
    }
}

// ---------------------------------------------------------------------------------------------

/// A thank-you / success page: a check mark that draws itself, a confetti burst, an order
/// summary card, next steps and two buttons.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::ThankYouPage;
/// ThankYouPage::new("Thanks for your order!").item("Pro plan (yearly)", "$228.00").show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct ThankYouPage {
    title: String,
    message: String,
    reference: String,
    items: Vec<(String, String)>,
    total: String,
    steps: Vec<(UiIcon, String)>,
    primary: String,
    secondary: String,
    confetti: bool,
    background: SectionBackground,
    opts: PageOpts,
    salt: Option<String>,
}

impl ThankYouPage {
    /// A thank-you page with the given title and a placeholder receipt.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: "Your payment was successful and your workspace has been upgraded. A receipt is on its way to your inbox.".into(),
            reference: "#GK-48213".into(),
            items: vec![("Pro plan, yearly".into(), "$228.00".into()), ("Extra seats x 3".into(), "$108.00".into()), ("Discount (20%)".into(), "-$67.20".into())],
            total: "$268.80".into(),
            steps: vec![(UiIcon::Mail, "Check your inbox for the receipt".into()), (UiIcon::Users, "Invite your teammates".into()), (UiIcon::Zap, "Explore the premium templates".into())],
            primary: "Go to dashboard".into(),
            secondary: "Download receipt".into(),
            confetti: true,
            background: SectionBackground::Glow,
            opts: PageOpts::default(),
            salt: None,
        }
    }
    /// Paragraph under the title.
    pub fn message(mut self, m: impl Into<String>) -> Self {
        self.message = m.into();
        self
    }
    /// Order reference shown on the summary card.
    pub fn reference(mut self, r: impl Into<String>) -> Self {
        self.reference = r.into();
        self
    }
    /// Add a summary line (the first call replaces the placeholders).
    pub fn item(mut self, label: impl Into<String>, amount: impl Into<String>) -> Self {
        if self.items.first().is_some_and(|i| i.0 == "Pro plan, yearly") {
            self.items.clear();
        }
        self.items.push((label.into(), amount.into()));
        self
    }
    /// Total line.
    pub fn total(mut self, t: impl Into<String>) -> Self {
        self.total = t.into();
        self
    }
    /// Hide the summary card.
    pub fn no_summary(mut self) -> Self {
        self.items.clear();
        self
    }
    /// Primary button label.
    pub fn primary_cta(mut self, l: impl Into<String>) -> Self {
        self.primary = l.into();
        self
    }
    /// Secondary button label.
    pub fn secondary_cta(mut self, l: impl Into<String>) -> Self {
        self.secondary = l.into();
        self
    }
    /// Burst confetti when the page appears (default on).
    pub fn confetti(mut self, on: bool) -> Self {
        self.confetti = on;
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    page_setters!();

    /// Show the page.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "thank_you_page", &self.salt);
        page(ui, id, self.opts, |ui, p| {
            let area = ui.clip_rect();
            centered_page(ui, p, id.with("body"), self.background, 560.0, |ui, sec| {
                let t = sec.t.clone();
                // self-drawing check
                let k = ((sec.elapsed - 0.1) / 0.9).clamp(0.0, 1.0);
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 104.0), Sense::hover());
                let c = r.center();
                let painter = ui.painter();
                let ring = glowkit_core::ease(glowkit_core::Easing::OutCubic, (k / 0.6).min(1.0));
                paint::glow(painter, c, 90.0, with_alpha(t.success, 0.18 * ring));
                painter.circle_filled(c, 44.0 * ring, with_alpha(t.success, 0.14));
                paint::arc(
                    painter,
                    c,
                    34.0,
                    -std::f32::consts::FRAC_PI_2,
                    -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * ring,
                    3.0,
                    t.success,
                );
                let ck = ((k - 0.5) / 0.5).clamp(0.0, 1.0);
                if ck > 0.0 {
                    let a = c + vec2(-14.0, 1.0);
                    let b = c + vec2(-4.0, 11.0);
                    let e = c + vec2(15.0, -10.0);
                    let pts = if ck < 0.4 {
                        vec![a, a + (b - a) * (ck / 0.4)]
                    } else {
                        vec![a, b, b + (e - b) * ((ck - 0.4) / 0.6)]
                    };
                    painter.add(Shape::line(pts, Stroke::new(4.0, t.success)));
                }
                if self.confetti {
                    let fired: bool = glowkit_core::state(ui, id.with("fired"), || false);
                    let confetti = glowkit_effects::ConfettiBurst::new().count(140);
                    if !fired && sec.seen && k > 0.6 {
                        confetti.fire(ui, c);
                        glowkit_core::set_state(ui, id.with("fired"), true);
                    }
                    ui.scope(|ui| confetti.show(ui, area));
                }
                ui.add_space(16.0);
                Heading::new(&self.title).subtitle(Some(&self.message)).size(34.0).show(ui, sec, 1);
                if !self.items.is_empty() {
                    ui.add_space(28.0);
                    sec.fade(ui, 3, |ui| {
                        card(ui, id.with("sum"), CardLook::of(&t).pad(22.0).hover(false), |ui| {
                            ui.horizontal(|ui| {
                                Tx::new("Order summary").size(15.0).semi().color(t.foreground).show_inline(ui);
                                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                    Tx::new(&self.reference)
                                        .size(13.0)
                                        .mono()
                                        .color(t.muted_foreground)
                                        .show_inline(ui);
                                });
                            });
                            ui.add_space(10.0);
                            for (label, amount) in &self.items {
                                ui.horizontal(|ui| {
                                    Tx::new(label).size(14.0).color(t.muted_foreground).show_inline(ui);
                                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                        Tx::new(amount)
                                            .size(14.0)
                                            .color(if amount.starts_with('-') { t.success } else { t.foreground })
                                            .show_inline(ui);
                                    });
                                });
                                ui.add_space(2.0);
                            }
                            ui.add_space(6.0);
                            divider(ui, None);
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                Tx::new("Total").size(15.0).semi().color(t.foreground).show_inline(ui);
                                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                    Tx::new(&self.total).size(18.0).bold().color(t.foreground).show_inline(ui);
                                });
                            });
                        });
                    });
                }
                if !self.steps.is_empty() {
                    ui.add_space(24.0);
                    sec.fade(ui, 4, |ui| {
                        Tx::new("What's next").size(13.0).semi().color(t.muted_foreground).show(ui);
                        ui.add_space(8.0);
                        for (k, (icon, s)) in self.steps.iter().enumerate() {
                            ui.horizontal(|ui| {
                                let (r, _) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::hover());
                                icon_tile(ui.painter(), r, *icon, t.chart[k % 6], &t);
                                Tx::new(s).size(14.5).color(t.foreground).show_inline(ui);
                            });
                            ui.add_space(4.0);
                        }
                    });
                }
                ui.add_space(28.0);
                let mut clicked = None;
                sec.fade(ui, 5, |ui| {
                    let w = (ui.available_width() - 12.0) / 2.0;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        if ui
                            .add(
                                Button::new(&self.secondary)
                                    .variant(ButtonVariant::Outline)
                                    .size(ControlSize::Lg)
                                    .icon(UiIcon::Download)
                                    .width(w),
                            )
                            .clicked()
                        {
                            clicked = Some(self.secondary.clone());
                        }
                        if ui
                            .add(
                                Button::new(&self.primary)
                                    .size(ControlSize::Lg)
                                    .fill(t.accent)
                                    .trailing_icon(UiIcon::ArrowRight)
                                    .width(w),
                            )
                            .clicked()
                        {
                            clicked = Some(self.primary.clone());
                        }
                    });
                });
                if let Some(c) = clicked {
                    sec.click(c);
                }
            });
        })
    }
}
