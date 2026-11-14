//! Content sections: about / story timeline, integrations, changelog, blog grid, gallery,
//! comparison table and "how it works" steps.

use super::kit::*;
use egui::{
    pos2, vec2, Align, Color32, CornerRadius, CursorIcon, FontId, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};
use glowkit_components::{Avatar, AvatarStyle, Badge, UiIcon};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::TAU;

/// A labelled milestone for [`AboutSection`].
#[derive(Clone, Debug)]
pub struct Milestone {
    /// Year or date label.
    pub when: String,
    /// Title.
    pub title: String,
    /// Description.
    pub description: String,
}

impl Milestone {
    /// A milestone.
    pub fn new(when: impl Into<String>, title: impl Into<String>, description: impl Into<String>) -> Self {
        Self { when: when.into(), title: title.into(), description: description.into() }
    }
}

/// Arrangement of an [`AboutSection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AboutLayout {
    /// Vertical timeline down the middle with milestones alternating left and right.
    #[default]
    Timeline,
    /// Story paragraphs beside an illustration, milestones as a horizontal strip below.
    Story,
}

/// The company story: a mission paragraph and a timeline of milestones that draws itself in.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{AboutSection, Milestone};
/// AboutSection::new("Our story").milestone(Milestone::new("2024", "Founded", "Two engineers, one idea.")).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct AboutSection {
    title: String,
    eyebrow: Option<String>,
    story: Vec<String>,
    milestones: Vec<Milestone>,
    layout: AboutLayout,
    background: SectionBackground,
    salt: Option<String>,
}

impl AboutSection {
    /// An about section with placeholder story and milestones.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            eyebrow: Some("Our story".into()),
            story: vec![
                "We started glowkit because building beautiful native apps felt ten years behind the web. Rust gave us speed and safety; we wanted delight too.".into(),
                "Today thousands of teams ship desktop and web apps with our components, and we're just getting started.".into(),
            ],
            milestones: vec![
                Milestone::new("2023", "The first commit", "A weekend experiment: can an immediate-mode GUI feel as polished as the best websites?"),
                Milestone::new("2024", "Open sourced", "We published the first 40 components. The community doubled that in six months."),
                Milestone::new("2025", "Seed round", "Backed by developer-first investors to grow the team and the library."),
                Milestone::new("2026", "glowkit 1.0", "Stable APIs, 300+ components, templates and a WebAssembly gallery."),
            ],
            layout: AboutLayout::Timeline,
            background: SectionBackground::Plain,
            salt: None,
        }
    }
    /// Small label above the title.
    pub fn eyebrow(mut self, e: impl Into<String>) -> Self {
        self.eyebrow = Some(e.into());
        self
    }
    /// Story paragraphs.
    pub fn story(mut self, paragraphs: &[&str]) -> Self {
        self.story = paragraphs.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Add a milestone (the first call replaces the placeholders).
    pub fn milestone(mut self, m: Milestone) -> Self {
        if self.milestones.first().is_some_and(|f| f.title == "The first commit") {
            self.milestones.clear();
        }
        self.milestones.push(m);
        self
    }
    /// Layout variant.
    pub fn layout(mut self, l: AboutLayout) -> Self {
        self.layout = l;
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Distinguish several about sections in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the section.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "about", &self.salt);
        section(ui, Frame::new(id, self.background), |ui, sec| {
            let t = sec.t.clone();
            match self.layout {
                AboutLayout::Timeline => {
                    let lead = self.story.first().map(String::as_str);
                    let next =
                        Heading::new(&self.title).eyebrow(self.eyebrow.as_deref()).subtitle(lead).show(ui, sec, 0);
                    ui.add_space(56.0);
                    let origin = ui.cursor().min;
                    let w = ui.available_width();
                    let center_x = if sec.narrow { origin.x + 12.0 } else { origin.x + w / 2.0 };
                    let mut y = origin.y;
                    let mut dots = Vec::new();
                    let line_idx = ui.painter().add(Shape::Noop);
                    for (i, m) in self.milestones.iter().enumerate() {
                        let left_side = !sec.narrow && i % 2 == 0;
                        let cw = if sec.narrow { w - 44.0 } else { w / 2.0 - 40.0 };
                        let x = if sec.narrow {
                            origin.x + 44.0
                        } else if left_side {
                            origin.x
                        } else {
                            center_x + 40.0
                        };
                        let mut child = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(Rect::from_min_size(pos2(x, y), vec2(cw, f32::INFINITY)))
                                .layout(egui::Layout::top_down(if left_side { Align::Max } else { Align::Min }))
                                .id_salt(("ms", i)),
                        );
                        let align = if left_side { Align::Max } else { Align::Min };
                        sec.fade(&mut child, next + i, |ui| {
                            card(ui, id.with(("mc", i)), CardLook::of(&t), |ui| {
                                Tx::new(&m.when).size(13.0).semi().align(align).color(t.accent).show(ui);
                                ui.add_space(2.0);
                                Tx::new(&m.title).size(18.0).semi().align(align).color(t.foreground).show(ui);
                                ui.add_space(4.0);
                                Tx::new(&m.description)
                                    .size(14.5)
                                    .line(1.55)
                                    .align(align)
                                    .color(t.muted_foreground)
                                    .show(ui);
                            });
                        });
                        let r = child.min_rect();
                        dots.push(pos2(center_x, r.min.y + 32.0));
                        y = r.max.y + 28.0;
                    }
                    let bottom = y - 28.0;
                    // line draws itself in
                    let p = glowkit_core::ease(
                        glowkit_core::Easing::InOutCubic,
                        ((sec.elapsed - 0.2) / 1.4).clamp(0.0, 1.0),
                    );
                    let y1 = origin.y + (bottom - origin.y) * p;
                    let painter = ui.painter();
                    painter.set(
                        line_idx,
                        Shape::line_segment(
                            [pos2(center_x, origin.y), pos2(center_x, bottom)],
                            Stroke::new(2.0, t.border),
                        ),
                    );
                    paint::gradient_polyline(
                        painter,
                        &[pos2(center_x, origin.y), pos2(center_x, y1.max(origin.y + 1.0))],
                        false,
                        2.0,
                        |u| mix(t.gradient[0], t.gradient[1], u),
                    );
                    for d in dots {
                        let lit = d.y <= y1;
                        painter.circle_filled(d, 9.0, t.background);
                        painter.circle_stroke(d, 7.0, Stroke::new(2.0, if lit { t.accent } else { t.border }));
                        if lit {
                            painter.circle_filled(d, 3.5, t.accent);
                        }
                    }
                    ui.advance_cursor_after_rect(Rect::from_min_max(origin, pos2(origin.x + w, bottom)));
                }
                AboutLayout::Story => {
                    two_col(
                        ui,
                        id.with("story"),
                        0.5,
                        56.0,
                        sec.narrow,
                        true,
                        |ui| {
                            let n = Heading::new(&self.title)
                                .eyebrow(self.eyebrow.as_deref())
                                .align(Align::Min)
                                .size(36.0)
                                .show(ui, sec, 0);
                            ui.add_space(20.0);
                            for (k, p) in self.story.iter().enumerate() {
                                sec.fade(ui, n + k, |ui| {
                                    Tx::new(p).size(16.0).line(1.7).color(t.muted_foreground).show(ui)
                                });
                                ui.add_space(12.0);
                            }
                        },
                        |ui| {
                            sec.fade(ui, 2, |ui| {
                                let w = ui.available_width();
                                let (r, _) = ui.allocate_exact_size(vec2(w, (w * 0.75).min(360.0)), Sense::hover());
                                paint_art(ui.painter(), r, 4, &t, (t.radius * 1.6).round(), glowkit_core::time(ui));
                                let chip = Rect::from_min_size(r.left_bottom() + vec2(20.0, -84.0), vec2(200.0, 64.0));
                                paint::soft_shadow(
                                    ui.painter(),
                                    chip,
                                    12.0,
                                    18.0,
                                    vec2(0.0, 6.0),
                                    with_alpha(Color32::BLACK, if t.dark { 0.5 } else { 0.12 }),
                                );
                                ui.painter().rect(
                                    chip,
                                    CornerRadius::same(12),
                                    t.popover,
                                    Stroke::new(1.0, t.border),
                                    StrokeKind::Inside,
                                );
                                ui.painter().text(
                                    chip.min + vec2(16.0, 12.0),
                                    egui::Align2::LEFT_TOP,
                                    "Founded",
                                    FontId::proportional(12.0),
                                    t.muted_foreground,
                                );
                                ui.painter().text(
                                    chip.min + vec2(16.0, 30.0),
                                    egui::Align2::LEFT_TOP,
                                    self.milestones.first().map_or("2023", |m| m.when.as_str()),
                                    FontId::proportional(20.0),
                                    t.foreground,
                                );
                            });
                        },
                    );
                    ui.add_space(48.0);
                    let cols = sec.cols(200.0, self.milestones.len().max(1));
                    grid(ui, id.with("ms"), self.milestones.len(), cols, 24.0, |ui, i, _| {
                        let m = &self.milestones[i];
                        sec.fade(ui, 3 + i, |ui| {
                            let r = ui.max_rect();
                            ui.painter().hline(r.min.x..=r.max.x, r.min.y + 6.0, Stroke::new(1.0, t.border));
                            ui.painter().circle_filled(pos2(r.min.x + 6.0, r.min.y + 6.0), 6.0, t.accent);
                            ui.add_space(22.0);
                            Tx::new(&m.when).size(13.0).semi().color(t.accent).show(ui);
                            Tx::new(&m.title).size(16.0).semi().color(t.foreground).show(ui);
                            Tx::new(&m.description).size(14.0).line(1.5).color(t.muted_foreground).show(ui);
                        });
                    });
                }
            }
        })
    }
}

// ---------------------------------------------------------------------------------------------

/// An app shown in an [`IntegrationsSection`].
#[derive(Clone, Debug)]
pub struct Integration {
    /// App name.
    pub name: String,
    /// One-line description (grid layout).
    pub description: String,
    /// Category label (grid layout).
    pub category: String,
}

impl Integration {
    /// An integration.
    pub fn new(name: impl Into<String>, category: impl Into<String>, description: impl Into<String>) -> Self {
        Self { name: name.into(), category: category.into(), description: description.into() }
    }

    /// Ten placeholder apps.
    pub fn defaults() -> Vec<Integration> {
        vec![
            Integration::new("Slate", "Communication", "Get notified in your team channels."),
            Integration::new("Gitbase", "Developer", "Sync issues and pull requests."),
            Integration::new("Payflow", "Payments", "Accept cards and subscriptions."),
            Integration::new("Mailbird", "Email", "Send transactional emails."),
            Integration::new("Datapipe", "Analytics", "Stream events to your warehouse."),
            Integration::new("Calendo", "Productivity", "Schedule meetings automatically."),
            Integration::new("Notely", "Docs", "Embed live docs and wikis."),
            Integration::new("Figurin", "Design", "Import design tokens and frames."),
            Integration::new("Cloudly", "Storage", "Store files and assets."),
            Integration::new("Authly", "Security", "SSO, SAML and SCIM."),
        ]
    }
}

/// Arrangement of an [`IntegrationsSection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum IntegrationsLayout {
    /// App tiles orbiting your brand tile on two rings.
    #[default]
    Orbit,
    /// A grid of app cards with category and description.
    Grid,
}

/// "Works with your stack": app tiles orbiting the product, or a grid of integration cards.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{IntegrationsSection, IntegrationsLayout};
/// IntegrationsSection::new("Plays well with others").layout(IntegrationsLayout::Grid).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct IntegrationsSection {
    title: String,
    eyebrow: Option<String>,
    subtitle: Option<String>,
    brand: String,
    apps: Vec<Integration>,
    layout: IntegrationsLayout,
    cta: Option<String>,
    background: SectionBackground,
    salt: Option<String>,
}

impl IntegrationsSection {
    /// An integrations section with placeholder apps.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            eyebrow: Some("Integrations".into()),
            subtitle: Some("Connect the tools you already use. Two clicks, no code.".into()),
            brand: "G".into(),
            apps: Integration::defaults(),
            layout: IntegrationsLayout::Orbit,
            cta: Some("Browse all integrations".into()),
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
    /// Letter(s) shown in the center tile of the orbit.
    pub fn brand(mut self, b: impl Into<String>) -> Self {
        self.brand = b.into();
        self
    }
    /// The apps.
    pub fn apps(mut self, a: Vec<Integration>) -> Self {
        self.apps = a;
        self
    }
    /// Layout variant.
    pub fn layout(mut self, l: IntegrationsLayout) -> Self {
        self.layout = l;
        self
    }
    /// Button under the apps (`""` hides it).
    pub fn cta(mut self, c: impl Into<String>) -> Self {
        let s = c.into();
        self.cta = (!s.is_empty()).then_some(s);
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Distinguish several integration sections in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the section.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "integrations", &self.salt);
        section(ui, Frame::new(id, self.background), |ui, sec| {
            let t = sec.t.clone();
            let next = Heading::new(&self.title)
                .eyebrow(self.eyebrow.as_deref())
                .subtitle(self.subtitle.as_deref())
                .show(ui, sec, 0);
            ui.add_space(40.0);
            let mut clicked = None;
            match self.layout {
                IntegrationsLayout::Orbit => {
                    sec.fade(ui, next, |ui| {
                        let w = ui.available_width();
                        let h = if sec.narrow { 340.0 } else { 420.0 };
                        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
                        let c = rect.center();
                        let painter = ui.painter_at(rect);
                        let time = glowkit_core::time(ui);
                        let slow = glowkit_core::anim::follow(
                            ui,
                            id.with("slow"),
                            if resp.hovered() { 0.25 } else { 1.0 },
                            4.0,
                        );
                        let dt = glowkit_core::dt(ui);
                        let clock = glowkit_core::with_state(
                            ui,
                            id.with("clock"),
                            || 0.0f32,
                            |c| {
                                *c += dt * slow;
                                *c
                            },
                        );
                        let r1 = h * 0.26;
                        let r2 = h * 0.44;
                        paint::glow(&painter, c, r2 * 1.1, with_alpha(t.accent, if t.dark { 0.12 } else { 0.06 }));
                        for (k, r) in [r1, r2].iter().enumerate() {
                            let pts: Vec<_> = (0..=96)
                                .map(|i| c + vec2((i as f32 / 96.0 * TAU).cos(), (i as f32 / 96.0 * TAU).sin()) * *r)
                                .collect();
                            for (j, seg) in pts.windows(2).enumerate() {
                                if j % 2 == 0 {
                                    painter.line_segment(
                                        [seg[0], seg[1]],
                                        Stroke::new(1.0, with_alpha(t.foreground, if k == 0 { 0.14 } else { 0.1 })),
                                    );
                                }
                            }
                        }
                        let n = self.apps.len();
                        let inner = n.min(4);
                        let mut hovered_app = None;
                        for (i, app) in self.apps.iter().enumerate() {
                            let (ring, idx, count, dir) =
                                if i < inner { (r1, i, inner, 1.0) } else { (r2, i - inner, n - inner, -1.0) };
                            let a = clock * 0.25 * dir
                                + idx as f32 / count.max(1) as f32 * TAU
                                + if ring == r2 { 0.3 } else { 0.0 };
                            let p = c + vec2(a.cos(), a.sin()) * ring;
                            let size = if ring == r1 { 46.0 } else { 52.0 };
                            let tr = Rect::from_center_size(p, Vec2::splat(size));
                            let hit = ui
                                .interact(tr, id.with(("app", i)), Sense::click())
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .on_hover_text(&app.name);
                            let hv = ui.ctx().animate_bool_with_time(hit.id, hit.hovered(), 0.15);
                            if hit.hovered() {
                                hovered_app = Some(p);
                            }
                            if hit.clicked() {
                                clicked = Some(app.name.clone());
                            }
                            app_tile(&painter, tr.expand(3.0 * hv), &app.name, i, &t, hv);
                        }
                        if let Some(p) = hovered_app {
                            paint::gradient_polyline(&painter, &[c, p], false, 1.5, |u| {
                                with_alpha(t.accent, 0.8 * (1.0 - u * 0.6))
                            });
                        }
                        // center brand tile with a pulse
                        let ct = Rect::from_center_size(c, Vec2::splat(76.0));
                        let ph = (time * 0.6).fract();
                        painter.rect_stroke(
                            ct.expand(10.0 + 26.0 * ph),
                            CornerRadius::same((22.0 + 10.0 * ph) as u8),
                            Stroke::new(1.5, with_alpha(t.accent, 0.5 * (1.0 - ph))),
                            StrokeKind::Middle,
                        );
                        paint::soft_shadow(&painter, ct, 20.0, 30.0, vec2(0.0, 8.0), with_alpha(t.accent, 0.45));
                        paint::gradient_rect(&painter, ct, 20.0, 135.0, &[t.gradient[0], t.gradient[1]]);
                        let g = painter.layout_no_wrap(self.brand.clone(), FontId::proportional(32.0), Color32::WHITE);
                        painter.galley(ct.center() - g.size() / 2.0, g.clone(), Color32::WHITE);
                        painter.galley(ct.center() - g.size() / 2.0 + vec2(1.0, 0.0), g, Color32::WHITE);
                        paint::fade_edges_horizontal(
                            &painter,
                            rect,
                            (w * 0.12).min(120.0),
                            muted_or_plain(&t, self.background),
                        );
                        glowkit_core::animate(ui);
                    });
                }
                IntegrationsLayout::Grid => {
                    let cols = sec.cols(240.0, 3);
                    let n = self.apps.len().min(9);
                    grid(ui, id.with("g"), n, cols, 16.0, |ui, i, row_h| {
                        let app = &self.apps[i];
                        sec.fade(ui, next + i / cols + i % cols, |ui| {
                            let out = card(
                                ui,
                                id.with(("ic", i)),
                                CardLook::of(&t).min_h(row_h).pad(20.0).clickable(true),
                                |ui| {
                                    ui.horizontal(|ui| {
                                        let (r, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
                                        app_tile(ui.painter(), r, &app.name, i, &t, 0.0);
                                        ui.add_space(4.0);
                                        ui.vertical(|ui| {
                                            ui.spacing_mut().item_spacing.y = 2.0;
                                            Tx::new(&app.name).size(15.5).semi().color(t.foreground).show_inline(ui);
                                            Tx::new(&app.category).size(12.5).color(t.muted_foreground).show_inline(ui);
                                        });
                                    });
                                    ui.add_space(10.0);
                                    Tx::new(&app.description).size(14.0).line(1.5).color(t.muted_foreground).show(ui);
                                },
                            );
                            // arrow that slides in on hover
                            let a = out.hover;
                            if a > 0.0 {
                                let r = out.rect.translate(vec2(0.0, -2.0 * a));
                                let ar = Rect::from_center_size(
                                    pos2(r.max.x - 26.0 + 4.0 * (1.0 - a), r.min.y + 30.0),
                                    Vec2::splat(16.0),
                                );
                                UiIcon::ArrowUp.paint(ui.painter(), ar, with_alpha(t.accent, a));
                            }
                            if out.response.clicked() {
                                clicked = Some(app.name.clone());
                            }
                        });
                    });
                }
            }
            if let Some(c) = &self.cta {
                ui.add_space(32.0);
                let mut hit = false;
                sec.fade(ui, next + 4, |ui| {
                    row(ui, id.with("cta"), Align::Center, 0.0, |ui| {
                        hit = cta_button(ui, c, SectionCtaStyle::Outline, false).clicked();
                    });
                });
                if hit {
                    clicked = Some(c.clone());
                }
            }
            if let Some(c) = clicked {
                sec.click(c);
            }
        })
    }
}

fn muted_or_plain(t: &Theme, bg: SectionBackground) -> Color32 {
    if bg == SectionBackground::Muted {
        muted_band(t)
    } else {
        t.background
    }
}

/// A rounded app icon: tinted tile with a monogram.
pub(crate) fn app_tile(painter: &egui::Painter, r: Rect, name: &str, i: usize, t: &Theme, hover: f32) {
    let col = t.chart[i % 6];
    let cr = CornerRadius::same((r.width() * 0.28) as u8);
    paint::soft_shadow(
        painter,
        r,
        r.width() * 0.28,
        12.0 + 8.0 * hover,
        vec2(0.0, 4.0),
        with_alpha(Color32::BLACK, if t.dark { 0.45 } else { 0.1 }),
    );
    painter.rect(
        r,
        cr,
        if t.dark { mix(t.card, col, 0.14 + 0.08 * hover) } else { mix(t.card, col, 0.08 + 0.06 * hover) },
        Stroke::new(1.0, mix(t.border, col, 0.3 + 0.4 * hover)),
        StrokeKind::Inside,
    );
    let letter: String = name.chars().take(1).collect();
    let g = painter.layout_no_wrap(letter, FontId::proportional(r.width() * 0.42), col);
    let pos = r.center() - g.size() / 2.0;
    painter.galley(pos, g.clone(), col);
    painter.galley(pos + vec2(0.8, 0.0), g, col);
    // small corner accent shape, different per app
    let d = r.width() * 0.12;
    let q = r.right_top() + vec2(-d * 1.6, d * 1.6);
    match i % 3 {
        0 => {
            painter.circle_filled(q, d * 0.5, with_alpha(col, 0.7));
        }
        1 => {
            painter.rect_filled(Rect::from_center_size(q, Vec2::splat(d)), 2.0, with_alpha(col, 0.7));
        }
        _ => {
            painter.add(Shape::convex_polygon(
                vec![q + vec2(0.0, -d * 0.6), q + vec2(d * 0.6, d * 0.5), q + vec2(-d * 0.6, d * 0.5)],
                with_alpha(col, 0.7),
                Stroke::NONE,
            ));
        }
    }
}

// ---------------------------------------------------------------------------------------------

/// One release in a [`ChangelogSection`].
#[derive(Clone, Debug)]
pub struct ChangelogEntry {
    /// Date label.
    pub date: String,
    /// Version label.
    pub version: String,
    /// Headline.
    pub title: String,
    /// Body text.
    pub body: String,
    /// Change list.
    pub changes: Vec<String>,
    /// Tags (e.g. "New", "Improved", "Fixed").
    pub tags: Vec<String>,
}

impl ChangelogEntry {
    /// A release entry.
    pub fn new(date: impl Into<String>, version: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            date: date.into(),
            version: version.into(),
            title: title.into(),
            body: String::new(),
            changes: Vec::new(),
            tags: Vec::new(),
        }
    }
    /// Body text.
    pub fn body(mut self, b: impl Into<String>) -> Self {
        self.body = b.into();
        self
    }
    /// Change list.
    pub fn changes(mut self, c: &[&str]) -> Self {
        self.changes = c.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Tags.
    pub fn tags(mut self, t: &[&str]) -> Self {
        self.tags = t.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Three placeholder releases.
    pub fn defaults() -> Vec<ChangelogEntry> {
        vec![
            ChangelogEntry::new("Nov 4, 2026", "v1.0.0", "glowkit 1.0 is here")
                .body("Stable APIs across every crate, a brand new template collection and a much faster WebAssembly build.")
                .changes(&["40 page sections and 15 full page templates", "Dashboard shells with responsive sidebars", "30% smaller wasm binaries"])
                .tags(&["New", "Improved"]),
            ChangelogEntry::new("Oct 12, 2026", "v0.9.0", "Charts get a glow-up").body("Every chart now animates between datasets and supports tooltips, legends and the new theme palette.").changes(&["Animated transitions", "Radial and heatmap charts"]).tags(&["New"]),
            ChangelogEntry::new("Sep 28, 2026", "v0.8.2", "Bug fixes and polish").body("Smaller fixes reported by the community.").changes(&["Fixed text input focus ring in light mode", "Smoother marquee on high refresh-rate screens"]).tags(&["Fixed"]),
        ]
    }
}

/// Release notes: dated entries on a timeline with version badges and tags.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{ChangelogSection, ChangelogEntry};
/// ChangelogSection::new("Changelog").entries(ChangelogEntry::defaults()).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct ChangelogSection {
    title: String,
    subtitle: Option<String>,
    entries: Vec<ChangelogEntry>,
    background: SectionBackground,
    salt: Option<String>,
}

impl ChangelogSection {
    /// A changelog with placeholder entries.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: Some("New updates and improvements to glowkit.".into()),
            entries: ChangelogEntry::defaults(),
            background: SectionBackground::Plain,
            salt: None,
        }
    }
    /// Paragraph under the title.
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = Some(s.into());
        self
    }
    /// The releases, newest first.
    pub fn entries(mut self, e: Vec<ChangelogEntry>) -> Self {
        self.entries = e;
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Distinguish several changelogs in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the section.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "changelog", &self.salt);
        section(ui, Frame::new(id, self.background).max_w(900.0), |ui, sec| {
            let t = sec.t.clone();
            let next = Heading::new(&self.title)
                .subtitle(self.subtitle.as_deref())
                .align(Align::Min)
                .size(40.0)
                .show(ui, sec, 0);
            ui.add_space(48.0);
            for (i, e) in self.entries.iter().enumerate() {
                let left_w = if sec.narrow { 0.0 } else { 180.0 };
                let origin = ui.cursor().min;
                let w = ui.available_width();
                let mut body = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(Rect::from_min_size(
                            origin + vec2(left_w + 32.0, 0.0),
                            vec2(w - left_w - 32.0, f32::INFINITY),
                        ))
                        .layout(egui::Layout::top_down(Align::Min))
                        .id_salt(("cl", i)),
                );
                sec.fade(&mut body, next + i, |ui| {
                    if sec.narrow {
                        Tx::new(&e.date).size(13.0).color(t.muted_foreground).show(ui);
                        ui.add_space(4.0);
                    }
                    ui.horizontal(|ui| {
                        ui.add(Badge::new(&e.version).color(t.accent).pill(true));
                        for tag in &e.tags {
                            let c = match tag.as_str() {
                                "New" => t.success,
                                "Fixed" => t.warning,
                                _ => t.info,
                            };
                            ui.add(Badge::new(tag).color(c).dot(true).pill(true));
                        }
                    });
                    ui.add_space(10.0);
                    Tx::new(&e.title).size(22.0).bold().color(t.foreground).show(ui);
                    if !e.body.is_empty() {
                        ui.add_space(6.0);
                        Tx::new(&e.body).size(15.0).line(1.6).color(t.muted_foreground).show(ui);
                    }
                    if !e.changes.is_empty() {
                        ui.add_space(12.0);
                        for c in &e.changes {
                            ui.horizontal_top(|ui| {
                                let (r, _) = ui.allocate_exact_size(vec2(14.0, 22.0), Sense::hover());
                                ui.painter().circle_filled(pos2(r.min.x + 5.0, r.min.y + 11.0), 2.5, t.accent);
                                Tx::new(c).size(14.5).line(1.5).color(t.foreground).show(ui);
                            });
                        }
                    }
                    if i == 0 {
                        ui.add_space(16.0);
                        let w = ui.available_width();
                        let (r, _) = ui.allocate_exact_size(vec2(w, (w * 0.42).clamp(160.0, 300.0)), Sense::hover());
                        paint_mockup(ui, r, 0, &t);
                    }
                });
                let used = body.min_rect();
                let painter = ui.painter();
                let lx = origin.x + left_w + 8.0;
                if !sec.narrow {
                    let p = sec.reveal(next + i);
                    painter.text(
                        pos2(origin.x, origin.y + 2.0),
                        egui::Align2::LEFT_TOP,
                        &e.date,
                        FontId::proportional(14.0),
                        with_alpha(t.muted_foreground, p),
                    );
                }
                painter.vline(
                    lx,
                    (origin.y + 14.0)..=(used.max.y + 48.0).min(if i + 1 == self.entries.len() {
                        used.max.y
                    } else {
                        f32::INFINITY
                    }),
                    Stroke::new(1.0, t.border),
                );
                painter.circle_filled(pos2(lx, origin.y + 10.0), 6.0, t.background);
                painter.circle_stroke(
                    pos2(lx, origin.y + 10.0),
                    5.0,
                    Stroke::new(2.0, if i == 0 { t.accent } else { t.border }),
                );
                ui.advance_cursor_after_rect(Rect::from_min_max(origin, pos2(origin.x + w, used.max.y)));
                ui.add_space(48.0);
            }
        })
    }
}

// ---------------------------------------------------------------------------------------------

/// A blog post card for [`BlogSection`].
#[derive(Clone, Debug)]
pub struct BlogPost {
    /// Title.
    pub title: String,
    /// Short excerpt.
    pub excerpt: String,
    /// Category label.
    pub category: String,
    /// Author name.
    pub author: String,
    /// Date label.
    pub date: String,
    /// Read time label, e.g. "6 min read".
    pub read_time: String,
}

impl BlogPost {
    /// A post.
    pub fn new(title: impl Into<String>, excerpt: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            excerpt: excerpt.into(),
            category: category.into(),
            author: "Ava Chen".into(),
            date: "Nov 2, 2026".into(),
            read_time: "5 min read".into(),
        }
    }
    /// Author name.
    pub fn author(mut self, a: impl Into<String>) -> Self {
        self.author = a.into();
        self
    }
    /// Date label.
    pub fn date(mut self, d: impl Into<String>) -> Self {
        self.date = d.into();
        self
    }
    /// Read-time label.
    pub fn read_time(mut self, r: impl Into<String>) -> Self {
        self.read_time = r.into();
        self
    }

    /// Six placeholder posts.
    pub fn defaults() -> Vec<BlogPost> {
        vec![
            BlogPost::new(
                "Designing motion that feels native",
                "Springs, easing curves and why 200ms is usually too slow for hover states.",
                "Design",
            )
            .author("Mia Park")
            .read_time("7 min read"),
            BlogPost::new(
                "Shipping Rust to the browser",
                "How we cut our WebAssembly bundle by a third without losing a single feature.",
                "Engineering",
            )
            .author("Liam Ortiz")
            .date("Oct 28, 2026"),
            BlogPost::new(
                "Theming without CSS",
                "Semantic tokens, light and dark presets and brand colors in a few lines of Rust.",
                "Guides",
            )
            .author("Zoe King")
            .date("Oct 19, 2026")
            .read_time("4 min read"),
            BlogPost::new(
                "Inside the bento grid",
                "Mixed card sizes, equal row heights and animated mini illustrations.",
                "Design",
            )
            .author("Leo Rossi")
            .date("Oct 7, 2026"),
            BlogPost::new(
                "Immediate mode at 120 fps",
                "Profiling the hot paths of an animated GUI, and what we learned.",
                "Engineering",
            )
            .author("Noah Smith")
            .date("Sep 30, 2026")
            .read_time("9 min read"),
            BlogPost::new(
                "Announcing our Seed round",
                "We're growing the team to build the best UI toolkit for Rust.",
                "Company",
            )
            .author("Ava Chen")
            .date("Sep 12, 2026")
            .read_time("3 min read"),
        ]
    }
}

/// Arrangement of a [`BlogSection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BlogLayout {
    /// Cover-image cards in a grid.
    #[default]
    Grid,
    /// One large featured post next to a list of the others.
    Featured,
    /// Horizontal rows with a thumbnail.
    List,
}

/// Latest posts from the blog. Clicking a post reports its title.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{BlogSection, BlogLayout};
/// BlogSection::new("From the blog").layout(BlogLayout::Featured).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct BlogSection {
    title: String,
    eyebrow: Option<String>,
    subtitle: Option<String>,
    posts: Vec<BlogPost>,
    layout: BlogLayout,
    background: SectionBackground,
    salt: Option<String>,
}

impl BlogSection {
    /// A blog section with placeholder posts.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            eyebrow: Some("Blog".into()),
            subtitle: None,
            posts: BlogPost::defaults(),
            layout: BlogLayout::Grid,
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
    /// The posts.
    pub fn posts(mut self, p: Vec<BlogPost>) -> Self {
        self.posts = p;
        self
    }
    /// Layout variant.
    pub fn layout(mut self, l: BlogLayout) -> Self {
        self.layout = l;
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Distinguish several blog sections in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the section.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "blog", &self.salt);
        section(ui, Frame::new(id, self.background), |ui, sec| {
            let t = sec.t.clone();
            let next = Heading::new(&self.title)
                .eyebrow(self.eyebrow.as_deref())
                .subtitle(self.subtitle.as_deref())
                .show(ui, sec, 0);
            ui.add_space(48.0);
            let mut clicked = None;
            match self.layout {
                BlogLayout::Grid => {
                    let cols = sec.cols(280.0, 3);
                    let n = self.posts.len().min(6);
                    grid(ui, id.with("g"), n, cols, 24.0, |ui, i, row_h| {
                        sec.fade(ui, next + i / cols + i % cols, |ui| {
                            if post_card(ui, id.with(("p", i)), &self.posts[i], i, row_h, 180.0, &t) {
                                clicked = Some(self.posts[i].title.clone());
                            }
                        });
                    });
                }
                BlogLayout::Featured => {
                    let mut c1 = None;
                    let mut c2 = None;
                    two_col(
                        ui,
                        id.with("f"),
                        0.56,
                        32.0,
                        sec.narrow,
                        false,
                        |ui| {
                            sec.fade(ui, next, |ui| {
                                if post_card(ui, id.with("feat"), &self.posts[0], 0, 0.0, 300.0, &t) {
                                    c1 = Some(self.posts[0].title.clone());
                                }
                            });
                        },
                        |ui| {
                            for (i, p) in self.posts.iter().enumerate().skip(1).take(4) {
                                sec.fade(ui, next + i, |ui| {
                                    if post_row(ui, id.with(("r", i)), p, i, 92.0, &t) {
                                        c2 = Some(p.title.clone());
                                    }
                                });
                                ui.add_space(8.0);
                            }
                        },
                    );
                    clicked = c1.or(c2);
                }
                BlogLayout::List => {
                    for (i, p) in self.posts.iter().enumerate().take(5) {
                        sec.fade(ui, next + i, |ui| {
                            if post_row(ui, id.with(("l", i)), p, i, if sec.narrow { 90.0 } else { 140.0 }, &t) {
                                clicked = Some(p.title.clone());
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
}

/// Category chip + date + read time.
fn post_meta(ui: &mut Ui, p: &BlogPost, i: usize, t: &Theme) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add(Badge::new(&p.category).color(t.chart[i % 6]).pill(true));
        Tx::new(format!("{}  ·  {}", p.date, p.read_time)).size(12.5).color(t.muted_foreground).show_inline(ui);
    });
}

/// A post card with cover art; returns true when clicked.
fn post_card(ui: &mut Ui, id: egui::Id, p: &BlogPost, i: usize, row_h: f32, cover_h: f32, t: &Theme) -> bool {
    let out = card(ui, id, CardLook::of(t).pad(8.0).min_h(row_h).clickable(true), |ui| {
        let w = ui.available_width();
        let (r, _) = ui.allocate_exact_size(vec2(w, cover_h), Sense::hover());
        ui.add_space(10.0);
        let mut inner = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_size(pos2(r.min.x + 12.0, ui.cursor().min.y), vec2(w - 24.0, f32::INFINITY)))
                .layout(egui::Layout::top_down(Align::Min)),
        );
        post_meta(&mut inner, p, i, t);
        inner.add_space(8.0);
        Tx::new(&p.title).size(18.0).semi().line(1.3).color(t.foreground).show(&mut inner);
        inner.add_space(4.0);
        Tx::new(&p.excerpt).size(14.0).line(1.55).color(t.muted_foreground).show(&mut inner);
        inner.add_space(14.0);
        inner.horizontal(|ui| {
            ui.add(Avatar::new(&p.author).size(26.0).style(AvatarStyle::Soft).color(t.chart[(i + 2) % 6]));
            Tx::new(&p.author).size(13.0).color(t.foreground).show_inline(ui);
        });
        let used = inner.min_rect();
        ui.advance_cursor_after_rect(Rect::from_min_max(r.min, pos2(r.max.x, used.max.y + 12.0)));
        r
    });
    let r = out.inner.translate(vec2(0.0, -2.0 * out.hover));
    let clip = ui.painter().with_clip_rect(r.intersect(ui.clip_rect()));
    // the cover zooms in slightly on hover
    let rad = (t.radius * 1.4 - 8.0).max(4.0);
    clip.rect_filled(r, CornerRadius::same(rad as u8), t.muted);
    paint_art(&clip, r.expand(6.0 * out.hover), i + 3, t, rad + 6.0 * out.hover, glowkit_core::time(ui) * 0.2);
    out.response.clicked()
}

/// A horizontal post row with thumbnail; returns true when clicked.
fn post_row(ui: &mut Ui, id: egui::Id, p: &BlogPost, i: usize, thumb: f32, t: &Theme) -> bool {
    let out = card(ui, id, CardLook::of(t).pad(12.0).clickable(true), |ui| {
        ui.horizontal_top(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(thumb * 1.3, thumb), Sense::hover());
            paint_art(ui.painter(), r, i + 3, t, (t.radius * 0.9).round(), 0.0);
            ui.add_space(8.0);
            ui.vertical(|ui| {
                post_meta(ui, p, i, t);
                ui.add_space(4.0);
                Tx::new(&p.title).size(16.0).semi().line(1.3).color(t.foreground).show(ui);
                if thumb > 100.0 {
                    ui.add_space(2.0);
                    Tx::new(&p.excerpt).size(14.0).line(1.5).color(t.muted_foreground).show(ui);
                }
            });
        });
    });
    out.response.clicked()
}

// ---------------------------------------------------------------------------------------------

/// Arrangement of a [`GallerySection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GalleryLayout {
    /// Uniform tiles.
    #[default]
    Grid,
    /// Columns of tiles with varying heights.
    Masonry,
}

/// A grid of image tiles with hover zoom and caption overlay. Clicking a tile reports its title.
///
/// Tiles show generated abstract art; draw your own images with [`GallerySection::tile_painter`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{GallerySection, GalleryLayout};
/// GallerySection::new("Made with glowkit").layout(GalleryLayout::Masonry).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct GallerySection {
    title: String,
    eyebrow: Option<String>,
    subtitle: Option<String>,
    items: Vec<(String, String)>,
    layout: GalleryLayout,
    background: SectionBackground,
    painter: Option<fn(&egui::Painter, Rect, usize)>,
    salt: Option<String>,
}

impl GallerySection {
    /// A gallery with placeholder tiles.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            eyebrow: Some("Showcase".into()),
            subtitle: None,
            items: [
                ("Northwind", "Analytics dashboard"),
                ("Lumen", "Photo editor"),
                ("Orbit", "Mission control"),
                ("Cadence", "Music studio"),
                ("Atlas", "Map explorer"),
                ("Pulse", "Health tracker"),
                ("Forge", "CI pipeline"),
                ("Drift", "Chat client"),
            ]
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect(),
            layout: GalleryLayout::Grid,
            background: SectionBackground::Plain,
            painter: None,
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
    /// Tiles as (title, caption) pairs.
    pub fn items(mut self, items: &[(&str, &str)]) -> Self {
        self.items = items.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        self
    }
    /// Paint each tile's image yourself: `f(painter, rect, index)`.
    pub fn tile_painter(mut self, f: fn(&egui::Painter, Rect, usize)) -> Self {
        self.painter = Some(f);
        self
    }
    /// Layout variant.
    pub fn layout(mut self, l: GalleryLayout) -> Self {
        self.layout = l;
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Distinguish several galleries in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the section.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "gallery", &self.salt);
        section(ui, Frame::new(id, self.background), |ui, sec| {
            let t = sec.t.clone();
            let next = Heading::new(&self.title)
                .eyebrow(self.eyebrow.as_deref())
                .subtitle(self.subtitle.as_deref())
                .show(ui, sec, 0);
            ui.add_space(44.0);
            let cols = sec.cols(220.0, 4);
            let gap = 16.0;
            let w = ui.available_width();
            let cw = (w - gap * (cols as f32 - 1.0)) / cols as f32;
            let origin = ui.cursor().min;
            let mut col_y = vec![origin.y; cols];
            let mut clicked = None;
            for (i, (title, caption)) in self.items.iter().enumerate() {
                let (c, h) = match self.layout {
                    GalleryLayout::Grid => (i % cols, cw * 0.75),
                    GalleryLayout::Masonry => {
                        let c = (0..cols).min_by(|a, b| col_y[*a].total_cmp(&col_y[*b])).unwrap_or(0);
                        (c, cw * [1.25, 0.8, 1.0, 1.4, 0.9, 1.15][i % 6])
                    }
                };
                let r = Rect::from_min_size(pos2(origin.x + c as f32 * (cw + gap), col_y[c]), vec2(cw, h));
                col_y[c] += h + gap;
                if self.layout == GalleryLayout::Grid && c + 1 == cols {
                    let y = col_y[c];
                    col_y.iter_mut().for_each(|v| *v = y);
                }
                let p = sec.reveal(next + i / cols + c);
                let resp =
                    ui.interact(r, id.with(("tile", i)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                let hv = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.25);
                if resp.clicked() {
                    clicked = Some(title.clone());
                }
                if !ui.is_rect_visible(r) {
                    continue;
                }
                let rr = r.translate(vec2(0.0, 14.0 * (1.0 - p)));
                let mut painter = ui.painter().with_clip_rect(rr.intersect(ui.clip_rect()));
                painter.set_opacity(p);
                let rad = (t.radius * 1.2).round();
                let zoom = rr.expand(10.0 * hv);
                match self.painter {
                    Some(f) => f(&painter, zoom, i),
                    None => paint_art(&painter, zoom, i, &t, 0.0, glowkit_core::time(ui) * 0.15),
                }
                // caption overlay
                let k = 0.35 + 0.65 * hv;
                let band = Rect::from_min_max(pos2(rr.min.x, rr.max.y - 90.0), rr.max);
                paint::quad(
                    &painter,
                    [band.left_top(), band.right_top(), band.right_bottom(), band.left_bottom()],
                    [
                        Color32::TRANSPARENT,
                        Color32::TRANSPARENT,
                        with_alpha(Color32::BLACK, 0.65 * k),
                        with_alpha(Color32::BLACK, 0.65 * k),
                    ],
                );
                let ty = rr.max.y - 44.0 + 8.0 * (1.0 - hv);
                painter.text(
                    pos2(rr.min.x + 16.0, ty),
                    egui::Align2::LEFT_TOP,
                    title,
                    FontId::proportional(15.0),
                    with_alpha(Color32::WHITE, 0.75 + 0.25 * hv),
                );
                painter.text(
                    pos2(rr.min.x + 16.0, ty + 19.0),
                    egui::Align2::LEFT_TOP,
                    caption,
                    FontId::proportional(12.5),
                    with_alpha(Color32::WHITE, 0.7 * hv),
                );
                if hv > 0.0 {
                    let br = Rect::from_center_size(pos2(rr.max.x - 26.0, rr.min.y + 26.0), Vec2::splat(30.0));
                    painter.circle_filled(br.center(), 15.0, with_alpha(Color32::WHITE, 0.9 * hv));
                    UiIcon::ArrowUp.paint(&painter, br.shrink(9.0), with_alpha(Color32::BLACK, hv));
                }
                // mask the zoomed art to rounded corners
                let mut frame = ui.painter().clone();
                frame.set_opacity(p);
                frame.rect_stroke(
                    rr,
                    CornerRadius::same(rad as u8),
                    Stroke::new(8.0, muted_or_plain(&t, self.background)),
                    StrokeKind::Outside,
                );
                frame.rect_stroke(
                    rr,
                    CornerRadius::same(rad as u8),
                    Stroke::new(1.0, with_alpha(t.border, 0.8)),
                    StrokeKind::Inside,
                );
            }
            let bottom = col_y.iter().cloned().fold(origin.y, f32::max) - gap;
            ui.advance_cursor_after_rect(Rect::from_min_max(origin, pos2(origin.x + w, bottom)));
            if let Some(c) = clicked {
                sec.click(c);
            }
        })
    }
}

// ---------------------------------------------------------------------------------------------

/// A cell in a [`ComparisonSection`] row.
#[derive(Clone, Debug, PartialEq)]
pub enum CompareValue {
    /// Supported (check mark).
    Yes,
    /// Not supported (cross).
    No,
    /// Partially supported (half dot).
    Partial,
    /// A text value such as "Unlimited" or "$49".
    Text(String),
}

/// "Us vs them" comparison table with the first column highlighted.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{ComparisonSection, CompareValue::*};
/// ComparisonSection::new("Why glowkit?")
///     .columns(&["glowkit", "Others"])
///     .row("Native + web", vec![Yes, No])
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct ComparisonSection {
    title: String,
    eyebrow: Option<String>,
    subtitle: Option<String>,
    columns: Vec<String>,
    rows: Vec<(String, Vec<CompareValue>)>,
    background: SectionBackground,
    salt: Option<String>,
}

impl ComparisonSection {
    /// A comparison with placeholder columns and rows.
    pub fn new(title: impl Into<String>) -> Self {
        use CompareValue::*;
        Self {
            title: title.into(),
            eyebrow: Some("Compare".into()),
            subtitle: Some("See how we stack up against the alternatives.".into()),
            columns: vec!["glowkit".into(), "Web UI kits".into(), "Native toolkits".into()],
            rows: vec![
                ("Runs natively on desktop".into(), vec![Yes, No, Yes]),
                ("Runs in the browser".into(), vec![Yes, Yes, No]),
                ("Animated components".into(), vec![Yes, Partial, No]),
                ("Single language".into(), vec![Yes, No, Partial]),
                ("Theme tokens".into(), vec![Yes, Yes, Partial]),
                ("Bundle size".into(), vec![Text("~3 MB".into()), Text("Varies".into()), Text("Large".into())]),
                ("License".into(), vec![Text("MIT".into()), Text("Mixed".into()), Text("Mixed".into())]),
            ],
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
    /// Column headers (the first is "us" and gets highlighted). Clears the placeholder rows.
    pub fn columns(mut self, c: &[&str]) -> Self {
        self.columns = c.iter().map(|s| s.to_string()).collect();
        self.rows.clear();
        self
    }
    /// Add a feature row.
    pub fn row(mut self, feature: impl Into<String>, values: Vec<CompareValue>) -> Self {
        self.rows.push((feature.into(), values));
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Distinguish several comparison sections in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the section.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "compare", &self.salt);
        section(ui, Frame::new(id, self.background).max_w(960.0), |ui, sec| {
            let t = sec.t.clone();
            let next = Heading::new(&self.title)
                .eyebrow(self.eyebrow.as_deref())
                .subtitle(self.subtitle.as_deref())
                .show(ui, sec, 0);
            ui.add_space(44.0);
            sec.fade(ui, next, |ui| {
                let w = ui.available_width();
                let nc = self.columns.len().max(1);
                let feat_w = if sec.narrow { w * 0.38 } else { w * 0.34 };
                let cw = (w - feat_w) / nc as f32;
                let head_h = 64.0;
                let row_h = 52.0;
                let total_h = head_h + row_h * self.rows.len() as f32 + 16.0;
                let (rect, _) = ui.allocate_exact_size(vec2(w, total_h), Sense::hover());
                let painter = ui.painter();
                let rad = (t.radius * 1.4).round();
                // highlighted "us" column
                let us = Rect::from_min_size(pos2(rect.min.x + feat_w, rect.min.y), vec2(cw, total_h));
                paint::glow_ellipse(
                    painter,
                    us.center_top() + vec2(0.0, 30.0),
                    vec2(cw * 0.9, 120.0),
                    with_alpha(t.accent, if t.dark { 0.18 } else { 0.08 }),
                );
                painter.rect(
                    us,
                    CornerRadius::same(rad as u8),
                    with_alpha(t.accent, if t.dark { 0.07 } else { 0.04 }),
                    Stroke::new(1.0, with_alpha(t.accent, 0.4)),
                    StrokeKind::Inside,
                );
                glowkit_effects::BorderBeam::new().radius(rad).duration(8.0).paint(ui, us);
                // header
                for (c, name) in self.columns.iter().enumerate() {
                    let cx = rect.min.x + feat_w + cw * (c as f32 + 0.5);
                    let col = if c == 0 { t.foreground } else { t.muted_foreground };
                    if c == 0 {
                        let g = painter.layout_no_wrap(name.clone(), FontId::proportional(16.0), col);
                        let lw = g.size().x + 26.0;
                        let tile = Rect::from_center_size(
                            pos2(cx - lw / 2.0 + 9.0, rect.min.y + head_h / 2.0),
                            Vec2::splat(18.0),
                        );
                        paint::gradient_rect(painter, tile, 5.0, 135.0, &[t.gradient[0], t.gradient[1]]);
                        painter.galley(
                            pos2(tile.max.x + 8.0, rect.min.y + head_h / 2.0 - g.size().y / 2.0),
                            g.clone(),
                            col,
                        );
                        painter.galley(pos2(tile.max.x + 8.6, rect.min.y + head_h / 2.0 - g.size().y / 2.0), g, col);
                    } else {
                        painter.text(
                            pos2(cx, rect.min.y + head_h / 2.0),
                            egui::Align2::CENTER_CENTER,
                            name,
                            FontId::proportional(14.5),
                            col,
                        );
                    }
                }
                painter.text(
                    pos2(rect.min.x + 4.0, rect.min.y + head_h / 2.0),
                    egui::Align2::LEFT_CENTER,
                    "Feature",
                    FontId::proportional(13.0),
                    t.muted_foreground,
                );
                for (r, (feat, vals)) in self.rows.iter().enumerate() {
                    let y = rect.min.y + head_h + r as f32 * row_h;
                    let row_rect = Rect::from_min_size(pos2(rect.min.x, y), vec2(w, row_h));
                    let hov = ui.rect_contains_pointer(row_rect);
                    if hov {
                        painter.rect_filled(row_rect, CornerRadius::same(8), with_alpha(t.foreground, 0.03));
                    }
                    painter.hline(rect.x_range(), y, Stroke::new(1.0, with_alpha(t.border, 0.8)));
                    let p = sec.reveal(next + 1 + r / 2);
                    let tg = painter.layout(
                        feat.clone(),
                        FontId::proportional(14.5),
                        with_alpha(t.foreground, p),
                        feat_w - 12.0,
                    );
                    painter.galley(pos2(rect.min.x + 4.0, y + row_h / 2.0 - tg.size().y / 2.0), tg, t.foreground);
                    for (c, v) in vals.iter().enumerate().take(nc) {
                        let cc = pos2(rect.min.x + feat_w + cw * (c as f32 + 0.5), y + row_h / 2.0);
                        let s = 9.0 * glowkit_core::ease(glowkit_core::Easing::OutBack, p);
                        match v {
                            CompareValue::Yes => {
                                let col = if c == 0 { t.accent } else { t.success };
                                painter.circle_filled(cc, s + 2.0, with_alpha(col, if c == 0 { 1.0 } else { 0.16 }));
                                UiIcon::Check.paint(
                                    painter,
                                    Rect::from_center_size(cc, Vec2::splat(s * 1.3)),
                                    if c == 0 { glowkit_core::color::contrast_text(t.accent) } else { col },
                                );
                            }
                            CompareValue::No => {
                                UiIcon::X.paint(
                                    painter,
                                    Rect::from_center_size(cc, Vec2::splat(s * 1.4)),
                                    with_alpha(t.muted_foreground, 0.6),
                                );
                            }
                            CompareValue::Partial => {
                                painter.circle_stroke(cc, s * 0.8, Stroke::new(1.5, t.warning));
                                let half: Vec<egui::Pos2> = (0..=12)
                                    .map(|k| {
                                        let a = std::f32::consts::FRAC_PI_2 + k as f32 / 12.0 * std::f32::consts::PI;
                                        cc + vec2(a.cos(), a.sin()) * s * 0.8
                                    })
                                    .collect();
                                painter.add(Shape::convex_polygon(half, t.warning, Stroke::NONE));
                            }
                            CompareValue::Text(s) => {
                                painter.text(
                                    cc,
                                    egui::Align2::CENTER_CENTER,
                                    s,
                                    FontId::proportional(14.0),
                                    with_alpha(if c == 0 { t.foreground } else { t.muted_foreground }, p),
                                );
                            }
                        }
                    }
                }
            });
        })
    }
}

// ---------------------------------------------------------------------------------------------

/// Arrangement of a [`StepsSection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StepsLayout {
    /// Numbered circles in a row joined by a line that fills in (stacks vertically when narrow).
    #[default]
    Horizontal,
    /// A vertical list with the connecting line on the left.
    Vertical,
    /// Numbered cards with a large faded step number.
    Cards,
}

/// "How it works": numbered steps joined by a connecting line that animates in.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::StepsSection;
/// StepsSection::new("How it works").step("Install", "cargo add glowkit").show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct StepsSection {
    title: String,
    eyebrow: Option<String>,
    subtitle: Option<String>,
    steps: Vec<(String, String, UiIcon)>,
    layout: StepsLayout,
    background: SectionBackground,
    salt: Option<String>,
}

impl StepsSection {
    /// A steps section with three placeholder steps.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            eyebrow: Some("How it works".into()),
            subtitle: Some("From zero to a polished app in three steps.".into()),
            steps: vec![
                (
                    "Install the crate".into(),
                    "Add glowkit to Cargo.toml and pick the feature sets you need.".into(),
                    UiIcon::Download,
                ),
                (
                    "Pick a template".into(),
                    "Start from a landing page, dashboard or app template and make it yours.".into(),
                    UiIcon::Grid,
                ),
                (
                    "Ship everywhere".into(),
                    "Build native binaries for every desktop and a WebAssembly bundle for the web.".into(),
                    UiIcon::Zap,
                ),
            ],
            layout: StepsLayout::Horizontal,
            background: SectionBackground::Plain,
            salt: None,
        }
    }
    /// Add a step (the first call replaces the placeholders).
    pub fn step(self, title: impl Into<String>, description: impl Into<String>) -> Self {
        self.step_with_icon(title, description, UiIcon::Check)
    }
    /// Add a step with an icon (the first call replaces the placeholders).
    pub fn step_with_icon(mut self, title: impl Into<String>, description: impl Into<String>, icon: UiIcon) -> Self {
        if self.steps.first().is_some_and(|s| s.0 == "Install the crate") {
            self.steps.clear();
        }
        self.steps.push((title.into(), description.into(), icon));
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
    pub fn layout(mut self, l: StepsLayout) -> Self {
        self.layout = l;
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Distinguish several steps sections in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the section.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "steps", &self.salt);
        section(ui, Frame::new(id, self.background), |ui, sec| {
            let t = sec.t.clone();
            let next = Heading::new(&self.title)
                .eyebrow(self.eyebrow.as_deref())
                .subtitle(self.subtitle.as_deref())
                .show(ui, sec, 0);
            ui.add_space(52.0);
            let n = self.steps.len();
            let layout =
                if sec.narrow && self.layout == StepsLayout::Horizontal { StepsLayout::Vertical } else { self.layout };
            // progress of the connecting line: 0..n-1 (which segment is filled)
            let prog = ((sec.elapsed - 0.4) / 0.6).clamp(0.0, (n.max(1) - 1) as f32);
            if prog < (n.max(1) - 1) as f32 {
                ui.ctx().request_repaint();
            }
            let badge = |p: &egui::Painter, c: egui::Pos2, i: usize, lit: f32| {
                p.circle_filled(c, 26.0, t.background);
                p.circle(c, 24.0, mix(t.card, t.accent, 0.12 * lit), Stroke::new(1.5, mix(t.border, t.accent, lit)));
                if lit > 0.5 {
                    p.circle_stroke(c, 30.0, Stroke::new(1.0, with_alpha(t.accent, 0.25 * lit)));
                }
                let g = p.layout_no_wrap(format!("{}", i + 1), FontId::proportional(18.0), Color32::PLACEHOLDER);
                let col = mix(t.muted_foreground, t.accent, lit);
                p.galley(c - g.size() / 2.0, g.clone(), col);
                p.galley(c - g.size() / 2.0 + vec2(0.7, 0.0), g, col);
            };
            match layout {
                StepsLayout::Horizontal => {
                    let w = ui.available_width();
                    let cw = w / n.max(1) as f32;
                    let origin = ui.cursor().min;
                    let cy = origin.y + 30.0;
                    let line = ui.painter().add(Shape::Noop);
                    let fill = ui.painter().add(Shape::Noop);
                    let mut bottom = origin.y;
                    for (i, (title, desc, _icon)) in self.steps.iter().enumerate() {
                        let cx = origin.x + cw * (i as f32 + 0.5);
                        let lit = (prog - i as f32 + 1.0).clamp(0.0, 1.0);
                        badge(ui.painter(), pos2(cx, cy), i, lit);
                        let mut child = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(Rect::from_min_size(
                                    pos2(cx - cw / 2.0 + 16.0, cy + 46.0),
                                    vec2(cw - 32.0, f32::INFINITY),
                                ))
                                .layout(egui::Layout::top_down(Align::Center))
                                .id_salt(("st", i)),
                        );
                        sec.fade(&mut child, next + i, |ui| {
                            Tx::new(title).size(18.0).semi().align(Align::Center).color(t.foreground).show(ui);
                            ui.add_space(6.0);
                            Tx::new(desc)
                                .size(14.5)
                                .line(1.55)
                                .align(Align::Center)
                                .max_width(300.0)
                                .color(t.muted_foreground)
                                .show(ui);
                        });
                        bottom = bottom.max(child.min_rect().max.y);
                    }
                    let x0 = origin.x + cw * 0.5;
                    let x1 = origin.x + cw * (n as f32 - 0.5);
                    ui.painter()
                        .set(line, Shape::line_segment([pos2(x0, cy), pos2(x1, cy)], Stroke::new(2.0, t.border)));
                    let xf = x0 + (x1 - x0) * (prog / (n.max(2) - 1) as f32);
                    if xf > x0 {
                        ui.painter()
                            .set(fill, Shape::line_segment([pos2(x0, cy), pos2(xf, cy)], Stroke::new(2.0, t.accent)));
                    }
                    ui.advance_cursor_after_rect(Rect::from_min_max(origin, pos2(origin.x + w, bottom)));
                }
                StepsLayout::Vertical => {
                    let origin = ui.cursor().min;
                    let w = ui.available_width().min(680.0);
                    let x = origin.x + (ui.available_width() - w) / 2.0;
                    let line = ui.painter().add(Shape::Noop);
                    let fill = ui.painter().add(Shape::Noop);
                    let mut y = origin.y;
                    let mut centers = Vec::new();
                    for (i, (title, desc, icon)) in self.steps.iter().enumerate() {
                        let c = pos2(x + 30.0, y + 30.0);
                        centers.push(c);
                        let lit = (prog - i as f32 + 1.0).clamp(0.0, 1.0);
                        badge(ui.painter(), c, i, lit);
                        let mut child = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(Rect::from_min_size(pos2(x + 80.0, y + 8.0), vec2(w - 80.0, f32::INFINITY)))
                                .layout(egui::Layout::top_down(Align::Min))
                                .id_salt(("sv", i)),
                        );
                        sec.fade(&mut child, next + i, |ui| {
                            card(ui, id.with(("svc", i)), CardLook::of(&t).pad(20.0), |ui| {
                                ui.horizontal(|ui| {
                                    let (r, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                                    icon.paint(ui.painter(), r, t.accent);
                                    Tx::new(title).size(17.0).semi().color(t.foreground).show_inline(ui);
                                });
                                ui.add_space(4.0);
                                Tx::new(desc).size(14.5).line(1.55).color(t.muted_foreground).show(ui);
                            });
                        });
                        y = child.min_rect().max.y + 24.0;
                    }
                    if let (Some(a), Some(b)) = (centers.first(), centers.last()) {
                        ui.painter().set(line, Shape::line_segment([*a, *b], Stroke::new(2.0, t.border)));
                        let yf = a.y + (b.y - a.y) * (prog / (n.max(2) - 1) as f32);
                        ui.painter().set(fill, Shape::line_segment([*a, pos2(a.x, yf)], Stroke::new(2.0, t.accent)));
                    }
                    ui.advance_cursor_after_rect(Rect::from_min_max(
                        origin,
                        pos2(origin.x + ui.available_width(), y - 24.0),
                    ));
                }
                StepsLayout::Cards => {
                    let cols = sec.cols(240.0, n.min(4));
                    grid(ui, id.with("g"), n, cols, 20.0, |ui, i, row_h| {
                        let (title, desc, icon) = &self.steps[i];
                        sec.fade(ui, next + i, |ui| {
                            let out = card(ui, id.with(("sc", i)), CardLook::of(&t).min_h(row_h).pad(24.0), |ui| {
                                let (r, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
                                icon_tile(ui.painter(), r, *icon, t.chart[i % 6], &t);
                                ui.add_space(14.0);
                                Tx::new(title).size(17.0).semi().color(t.foreground).show(ui);
                                ui.add_space(4.0);
                                Tx::new(desc).size(14.5).line(1.55).color(t.muted_foreground).show(ui);
                            });
                            let r = out.rect.translate(vec2(0.0, -2.0 * out.hover));
                            let g = ui.painter().layout_no_wrap(
                                format!("{:02}", i + 1),
                                FontId::proportional(56.0),
                                Color32::WHITE,
                            );
                            let pos = pos2(r.max.x - g.size().x - 18.0, r.min.y + 8.0);
                            let top = with_alpha(t.foreground, if t.dark { 0.09 } else { 0.07 });
                            paint::galley_with(ui.painter(), pos, &g, |q| {
                                with_alpha(
                                    top,
                                    (1.0 - (q.y - pos.y) / g.size().y).clamp(0.0, 1.0)
                                        * if t.dark { 0.12 } else { 0.08 },
                                )
                            });
                        });
                    });
                }
            }
        })
    }
}
