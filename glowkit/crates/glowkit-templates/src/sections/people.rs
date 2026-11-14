//! Team, newsletter / waitlist and contact sections.

use super::kit::*;
use egui::{pos2, vec2, Align, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use glowkit_components::{Avatar, AvatarCircles, AvatarStyle, Button, ButtonVariant, ControlSize, Input, Select, Textarea, UiIcon};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// One person in a [`TeamSection`].
#[derive(Clone, Debug)]
pub struct SectionTeamMember {
    /// Full name.
    pub name: String,
    /// Role / title.
    pub role: String,
    /// Short bio (cards layouts).
    pub bio: String,
    /// Social links shown as icon buttons (clicks report `"<name>: <network>"`).
    pub socials: Vec<SectionSocial>,
}

impl SectionTeamMember {
    /// A team member.
    pub fn new(name: impl Into<String>, role: impl Into<String>) -> Self {
        Self { name: name.into(), role: role.into(), bio: String::new(), socials: vec![SectionSocial::X, SectionSocial::LinkedIn] }
    }
    /// Short bio.
    pub fn bio(mut self, b: impl Into<String>) -> Self {
        self.bio = b.into();
        self
    }
    /// Social networks.
    pub fn socials(mut self, s: &[SectionSocial]) -> Self {
        self.socials = s.to_vec();
        self
    }

    /// Eight placeholder people.
    pub fn defaults() -> Vec<SectionTeamMember> {
        vec![
            SectionTeamMember::new("Ava Chen", "Co-founder & CEO").bio("Previously led design systems at a large fintech."),
            SectionTeamMember::new("Liam Ortiz", "Co-founder & CTO").bio("Rust since 0.x. Loves GPUs and type systems.").socials(&[SectionSocial::GitHub, SectionSocial::X]),
            SectionTeamMember::new("Mia Park", "Head of Design").bio("Obsessed with motion, type and tiny details."),
            SectionTeamMember::new("Noah Smith", "Staff Engineer").bio("Makes things fast. Then makes them faster.").socials(&[SectionSocial::GitHub]),
            SectionTeamMember::new("Zoe King", "Developer Advocate").bio("Writes the docs you actually enjoy reading.").socials(&[SectionSocial::YouTube, SectionSocial::X]),
            SectionTeamMember::new("Ethan Wu", "Product Engineer").bio("Ships features end to end, pixel to database."),
            SectionTeamMember::new("Isla Brown", "Customer Success").bio("Turns support tickets into happy customers."),
            SectionTeamMember::new("Leo Rossi", "Designer").bio("Icons, illustrations and everything in between.").socials(&[SectionSocial::Instagram]),
        ]
    }
}

/// Arrangement of a [`TeamSection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TeamLayout {
    /// Centered avatar cards with bio and socials.
    #[default]
    Cards,
    /// Large portrait tiles (abstract art) with name and role below.
    Portraits,
    /// Compact rows: avatar, name and role.
    Compact,
}

/// The people behind the product.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{TeamSection, SectionTeamMember};
/// TeamSection::new("Meet the team").members(SectionTeamMember::defaults()).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct TeamSection {
    title: String,
    eyebrow: Option<String>,
    subtitle: Option<String>,
    members: Vec<SectionTeamMember>,
    layout: TeamLayout,
    background: SectionBackground,
    salt: Option<String>,
}

impl TeamSection {
    /// A team section with placeholder people.
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), eyebrow: None, subtitle: None, members: SectionTeamMember::defaults(), layout: TeamLayout::Cards, background: SectionBackground::Plain, salt: None }
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
    /// The people.
    pub fn members(mut self, m: Vec<SectionTeamMember>) -> Self {
        self.members = m;
        self
    }
    /// Layout variant.
    pub fn layout(mut self, l: TeamLayout) -> Self {
        self.layout = l;
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Distinguish several team sections in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the section.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "team", &self.salt);
        section(ui, Frame::new(id, self.background), |ui, sec| {
            let t = sec.t.clone();
            let next = Heading::new(&self.title).eyebrow(self.eyebrow.as_deref()).subtitle(self.subtitle.as_deref()).show(ui, sec, 0);
            ui.add_space(48.0);
            let mut clicked = None;
            let (min_w, max_c) = match self.layout {
                TeamLayout::Cards => (220.0, 4),
                TeamLayout::Portraits => (200.0, 4),
                TeamLayout::Compact => (240.0, 4),
            };
            let cols = sec.cols(min_w, max_c);
            grid(ui, id.with("g"), self.members.len(), cols, if self.layout == TeamLayout::Compact { 12.0 } else { 20.0 }, |ui, i, row_h| {
                let m = &self.members[i];
                let color = t.chart[i % 6];
                sec.fade(ui, next + i / cols + i % cols, |ui| match self.layout {
                    TeamLayout::Cards => {
                        card(ui, id.with(("m", i)), CardLook::of(&t).min_h(row_h), |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add(Avatar::new(&m.name).size(72.0).style(AvatarStyle::Solid).color(color).ring(t.card));
                                ui.add_space(10.0);
                                Tx::new(&m.name).size(16.5).semi().align(Align::Center).color(t.foreground).show(ui);
                                Tx::new(&m.role).size(13.5).align(Align::Center).color(t.accent).show(ui);
                                if !m.bio.is_empty() {
                                    ui.add_space(6.0);
                                    Tx::new(&m.bio).size(13.5).line(1.5).align(Align::Center).color(t.muted_foreground).show(ui);
                                }
                                ui.add_space(12.0);
                                row(ui, id.with(("soc", i)), Align::Center, 8.0, |ui| {
                                    for s in &m.socials {
                                        if social_button(ui, *s, 30.0).clicked() {
                                            clicked = Some(format!("{}: {}", m.name, s.label()));
                                        }
                                    }
                                });
                            });
                        });
                    }
                    TeamLayout::Portraits => {
                        let w = ui.available_width();
                        let (r, resp) = ui.allocate_exact_size(vec2(w, w * 1.1), Sense::hover());
                        let hv = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.2);
                        let cr = (t.radius * 1.4).round();
                        paint_art(ui.painter(), r, i, &t, cr, glowkit_core::time(ui) * (0.3 + hv));
                        let av = Rect::from_center_size(r.center() - vec2(0.0, 6.0 * hv), Vec2::splat(w * 0.42));
                        Avatar::new(&m.name).size(av.width()).style(AvatarStyle::Solid).color(mix(color, t.card, 0.2)).ring(with_alpha(Color32::WHITE, 0.3)).paint(ui, av);
                        if hv > 0.0 {
                            ui.painter().rect_stroke(r, CornerRadius::same(cr as u8), Stroke::new(1.0, with_alpha(t.accent, 0.6 * hv)), StrokeKind::Inside);
                        }
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                Tx::new(&m.name).size(16.0).semi().color(t.foreground).show_inline(ui);
                                Tx::new(&m.role).size(13.5).color(t.muted_foreground).show_inline(ui);
                            });
                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                for s in m.socials.iter().rev() {
                                    if social_button(ui, *s, 28.0).clicked() {
                                        clicked = Some(format!("{}: {}", m.name, s.label()));
                                    }
                                }
                            });
                        });
                    }
                    TeamLayout::Compact => {
                        card(ui, id.with(("c", i)), CardLook::of(&t).pad(14.0).min_h(row_h), |ui| {
                            ui.horizontal(|ui| {
                                ui.add(Avatar::new(&m.name).size(44.0).style(AvatarStyle::Soft).color(color));
                                ui.add_space(4.0);
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 2.0;
                                    Tx::new(&m.name).size(15.0).semi().color(t.foreground).show_inline(ui);
                                    Tx::new(&m.role).size(13.0).color(t.muted_foreground).show_inline(ui);
                                });
                            });
                        });
                    }
                });
            });
            if let Some(c) = clicked {
                sec.click(c);
            }
        })
    }
}

// ---------------------------------------------------------------------------------------------

/// Arrangement of a [`NewsletterSection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NewsletterLayout {
    /// Centered copy with an inline email field and button.
    #[default]
    Inline,
    /// A card with copy on the left and the form on the right.
    Card,
    /// A waitlist: big headline, form, avatars and a live counter of people waiting.
    Waitlist,
}

/// Email capture for a newsletter or waitlist, with validation and a success state.
///
/// A valid submission is reported in [`SectionResponse::submitted`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{NewsletterSection, NewsletterLayout};
/// let r = NewsletterSection::new("Join the waitlist").layout(NewsletterLayout::Waitlist).show(ui);
/// if let Some(email) = r.submitted { /* save it */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct NewsletterSection {
    title: String,
    eyebrow: Option<String>,
    subtitle: Option<String>,
    placeholder: String,
    button: String,
    success: String,
    note: Option<String>,
    count: u64,
    layout: NewsletterLayout,
    background: SectionBackground,
    salt: Option<String>,
}

impl NewsletterSection {
    /// A newsletter signup with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            eyebrow: None,
            subtitle: Some("Product updates, new components and the occasional deep dive. No spam, unsubscribe anytime.".into()),
            placeholder: "you@company.com".into(),
            button: "Subscribe".into(),
            success: "You're in! Check your inbox to confirm.".into(),
            note: Some("We care about your data. Read our privacy policy.".into()),
            count: 2_847,
            layout: NewsletterLayout::Inline,
            background: SectionBackground::Plain,
            salt: None,
        }
    }
    /// Small label above the title.
    pub fn eyebrow(mut self, e: impl Into<String>) -> Self {
        self.eyebrow = Some(e.into());
        self
    }
    /// Paragraph under the title (`""` hides it).
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        let s = s.into();
        self.subtitle = (!s.is_empty()).then_some(s);
        self
    }
    /// Email field placeholder.
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }
    /// Submit button label.
    pub fn button(mut self, b: impl Into<String>) -> Self {
        self.button = b.into();
        self
    }
    /// Message shown after a successful submission.
    pub fn success_message(mut self, m: impl Into<String>) -> Self {
        self.success = m.into();
        self
    }
    /// Small print under the form (`""` hides it).
    pub fn note(mut self, n: impl Into<String>) -> Self {
        let s = n.into();
        self.note = (!s.is_empty()).then_some(s);
        self
    }
    /// People already on the waitlist (waitlist layout).
    pub fn count(mut self, n: u64) -> Self {
        self.count = n;
        self
    }
    /// Layout variant.
    pub fn layout(mut self, l: NewsletterLayout) -> Self {
        self.layout = l;
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Distinguish several newsletter sections in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the section.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "newsletter", &self.salt);
        let bg = if self.layout == NewsletterLayout::Waitlist && self.background == SectionBackground::Plain { SectionBackground::Meteors } else { self.background };
        section(ui, Frame::new(id, bg), |ui, sec| {
            let t = sec.t.clone();
            let mut submitted = None;
            match self.layout {
                NewsletterLayout::Inline => {
                    let next = Heading::new(&self.title).eyebrow(self.eyebrow.as_deref()).subtitle(self.subtitle.as_deref()).size(32.0).show(ui, sec, 0);
                    ui.add_space(28.0);
                    sec.fade(ui, next, |ui| {
                        centered_column(ui, 460.0, |ui| submitted = email_form(ui, id, &self.placeholder, &self.button, &self.success, false));
                        if let Some(n) = &self.note {
                            ui.add_space(10.0);
                            Tx::new(n).size(12.5).align(Align::Center).color(t.muted_foreground).show(ui);
                        }
                    });
                }
                NewsletterLayout::Card => {
                    sec.fade(ui, 0, |ui| {
                        card(ui, id.with("card"), CardLook::of(&t).pad(if sec.narrow { 24.0 } else { 40.0 }).hover(false), |ui| {
                            let r = ui.max_rect();
                            paint::glow_ellipse(ui.painter(), pos2(r.min.x, r.min.y), vec2(260.0, 140.0), with_alpha(t.gradient[0], if t.dark { 0.16 } else { 0.07 }));
                            two_col(
                                ui,
                                id.with("cols"),
                                0.5,
                                40.0,
                                sec.narrow,
                                true,
                                |ui| {
                                    let (tile, _) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::hover());
                                    icon_tile(ui.painter(), tile, UiIcon::Mail, t.accent, &t);
                                    ui.add_space(14.0);
                                    Tx::new(&self.title).size(26.0).bold().line(1.2).color(t.foreground).show(ui);
                                    if let Some(s) = &self.subtitle {
                                        ui.add_space(6.0);
                                        Tx::new(s).size(15.0).line(1.55).color(t.muted_foreground).show(ui);
                                    }
                                },
                                |ui| {
                                    submitted = email_form(ui, id, &self.placeholder, &self.button, &self.success, true);
                                    if let Some(n) = &self.note {
                                        ui.add_space(10.0);
                                        Tx::new(n).size(12.5).color(t.muted_foreground).show(ui);
                                    }
                                },
                            );
                        });
                    });
                }
                NewsletterLayout::Waitlist => {
                    let joined: bool = glowkit_core::state(ui, id.with("done"), || false);
                    let next = Heading::new(&self.title).eyebrow(self.eyebrow.as_deref().or(Some("Early access"))).subtitle(self.subtitle.as_deref()).size(48.0).show(ui, sec, 0);
                    ui.add_space(32.0);
                    sec.fade(ui, next, |ui| centered_column(ui, 460.0, |ui| submitted = email_form(ui, id, &self.placeholder, &self.button, &self.success, false)));
                    ui.add_space(28.0);
                    sec.fade(ui, next + 1, |ui| {
                        row(ui, id.with("count"), Align::Center, 12.0, |ui| {
                            ui.add(AvatarCircles::new(&["Ava Chen", "Liam Ortiz", "Mia Park", "Noah Smith"], 0).size(32.0));
                            let n = if sec.seen { self.count + u64::from(joined) } else { 0 };
                            ui.add(glowkit_text::CountUp::new(n as f64).from(0.0).duration(1.4).size(15.0).bold(true).color(t.foreground));
                            Tx::new("people already on the waitlist").size(14.0).color(t.muted_foreground).show_inline(ui);
                        });
                    });
                    if submitted.is_some() {
                        glowkit_core::set_state(ui, id.with("done"), true);
                    }
                }
            }
            sec.submitted = submitted;
        })
    }
}

/// Run `add` in a centered column at most `max_w` wide.
pub(crate) fn centered_column<R>(ui: &mut Ui, max_w: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let avail = ui.available_width();
    let w = avail.min(max_w);
    let x = ui.cursor().min.x + (avail - w) / 2.0;
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos2(x, ui.cursor().min.y), vec2(w, f32::INFINITY))).layout(egui::Layout::top_down(Align::Min)));
    let r = add(&mut child);
    let used = child.min_rect();
    ui.advance_cursor_after_rect(Rect::from_min_max(pos2(ui.cursor().min.x, used.min.y), pos2(ui.cursor().min.x + avail, used.max.y)));
    r
}

/// Simple email check: something@something.tld
pub(crate) fn valid_email(s: &str) -> bool {
    let s = s.trim();
    match s.split_once('@') {
        Some((a, b)) => !a.is_empty() && b.contains('.') && !b.starts_with('.') && !b.ends_with('.') && !s.contains(' '),
        None => false,
    }
}

#[derive(Clone, Default)]
struct EmailState {
    email: String,
    error: Option<String>,
    done_at: Option<f64>,
}

/// Email field + submit button with validation and an animated success row.
/// Returns the email on a valid submit.
pub(crate) fn email_form(ui: &mut Ui, id: egui::Id, placeholder: &str, button: &str, success: &str, stacked: bool) -> Option<String> {
    let t = Theme::of(ui);
    let mut st: EmailState = glowkit_core::state(ui, id.with("email"), EmailState::default);
    let mut out = None;
    if let Some(at) = st.done_at {
        let k = glowkit_core::ease(glowkit_core::Easing::OutBack, (((ui.input(|i| i.time) - at) as f32) / 0.5).min(1.0));
        if k < 1.0 {
            ui.ctx().request_repaint();
        }
        let w = ui.available_width();
        let (r, _) = ui.allocate_exact_size(vec2(w, 44.0), Sense::hover());
        let painter = ui.painter();
        painter.rect(r, CornerRadius::same((t.radius * 0.8) as u8), with_alpha(t.success, 0.1), Stroke::new(1.0, with_alpha(t.success, 0.35)), StrokeKind::Inside);
        let c = pos2(r.min.x + 22.0, r.center().y);
        painter.circle_filled(c, 10.0 * k.max(0.0), t.success);
        UiIcon::Check.paint(painter, Rect::from_center_size(c, Vec2::splat(12.0 * k.max(0.0))), Color32::WHITE);
        let tx = Tx::new(success).size(14.0).color(t.foreground);
        let g = tx.galley(ui, w - 50.0);
        tx.paint(ui, &g, pos2(r.min.x + 42.0, r.center().y - g.size().y / 2.0), w - 50.0, t.foreground);
        return None;
    }
    let mut submit = false;
    let bw = ui.painter().layout_no_wrap(button.to_owned(), egui::FontId::proportional(15.0), Color32::WHITE).size().x + 52.0;
    let field = |ui: &mut Ui, st: &mut EmailState, w: f32| -> egui::Response {
        let r = ui.add(Input::new(&mut st.email).placeholder(placeholder).icon(UiIcon::Mail).size(ControlSize::Lg).width(w).id_salt(id.with("field")));
        if r.changed() {
            st.error = None;
        }
        r
    };
    if stacked || ui.available_width() < 380.0 {
        let r = field(ui, &mut st, f32::INFINITY);
        submit |= r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        ui.add_space(4.0);
        submit |= ui.add(Button::new(button).size(ControlSize::Lg).width(f32::INFINITY).fill(t.accent)).clicked();
    } else {
        ui.with_layout(egui::Layout::left_to_right(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let w = ui.available_width() - bw - 8.0;
            let r = field(ui, &mut st, w);
            submit |= r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            submit |= ui.add(Button::new(button).size(ControlSize::Lg).width(bw).fill(t.accent).variant(ButtonVariant::Default)).clicked();
        });
    }
    if let Some(e) = &st.error {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
            UiIcon::AlertCircle.paint(ui.painter(), r, t.destructive);
            Tx::new(e).size(12.5).color(t.destructive).show_inline(ui);
        });
    }
    if submit {
        if valid_email(&st.email) {
            st.done_at = Some(ui.input(|i| i.time));
            out = Some(st.email.trim().to_owned());
        } else if st.email.trim().is_empty() {
            st.error = Some("Please enter your email address.".into());
        } else {
            st.error = Some("That doesn't look like a valid email address.".into());
        }
    }
    glowkit_core::set_state(ui, id.with("email"), st);
    out
}

// ---------------------------------------------------------------------------------------------

/// Arrangement of a [`ContactSection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ContactLayout {
    /// Contact details on the left, the form in a card on the right.
    #[default]
    Split,
    /// Three info cards above a centered form.
    Stacked,
}

/// A contact form (name, email, topic, message) with validation and a success state, plus
/// contact details.
///
/// A valid submission is reported in [`SectionResponse::submitted`] as
/// `"<name> <email> [<topic>]: <message>"`.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::ContactSection;
/// let r = ContactSection::new("Get in touch").email("hello@acme.dev").show(ui);
/// if let Some(msg) = r.submitted { /* send it */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct ContactSection {
    title: String,
    eyebrow: Option<String>,
    subtitle: Option<String>,
    email: String,
    phone: String,
    address: String,
    topics: Vec<String>,
    button: String,
    layout: ContactLayout,
    background: SectionBackground,
    salt: Option<String>,
}

#[derive(Clone, Default)]
struct ContactState {
    name: String,
    email: String,
    topic: Option<usize>,
    message: String,
    tried: bool,
    sent_at: Option<f64>,
}

impl ContactSection {
    /// A contact section with placeholder details.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            eyebrow: Some("Contact".into()),
            subtitle: Some("We'd love to hear from you. Our team usually replies within one business day.".into()),
            email: "hello@glowkit.dev".into(),
            phone: "+1 (555) 010-2024".into(),
            address: "100 Market St, San Francisco".into(),
            topics: ["General question", "Sales", "Support", "Partnerships"].iter().map(|s| s.to_string()).collect(),
            button: "Send message".into(),
            layout: ContactLayout::Split,
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
    /// Contact email.
    pub fn email(mut self, e: impl Into<String>) -> Self {
        self.email = e.into();
        self
    }
    /// Phone number.
    pub fn phone(mut self, p: impl Into<String>) -> Self {
        self.phone = p.into();
        self
    }
    /// Office address.
    pub fn address(mut self, a: impl Into<String>) -> Self {
        self.address = a.into();
        self
    }
    /// Options of the topic select.
    pub fn topics(mut self, t: &[&str]) -> Self {
        self.topics = t.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Submit button label.
    pub fn button(mut self, b: impl Into<String>) -> Self {
        self.button = b.into();
        self
    }
    /// Layout variant.
    pub fn layout(mut self, l: ContactLayout) -> Self {
        self.layout = l;
        self
    }
    /// Background variant.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Distinguish several contact sections in one `Ui`.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.salt = Some(s.into());
        self
    }

    /// Show the section.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "contact", &self.salt);
        section(ui, Frame::new(id, self.background), |ui, sec| {
            let t = sec.t.clone();
            let mut submitted = None;
            let infos = [(UiIcon::Mail, "Email", self.email.as_str(), "Our friendly team is here to help."), (UiIcon::Globe, "Office", self.address.as_str(), "Come say hello at our HQ."), (UiIcon::Clock, "Phone", self.phone.as_str(), "Mon-Fri from 8am to 5pm.")];
            match self.layout {
                ContactLayout::Split => {
                    two_col(
                        ui,
                        id.with("cols"),
                        0.42,
                        56.0,
                        sec.narrow,
                        false,
                        |ui| {
                            let n = Heading::new(&self.title).eyebrow(self.eyebrow.as_deref()).subtitle(self.subtitle.as_deref()).align(Align::Min).size(36.0).show(ui, sec, 0);
                            ui.add_space(32.0);
                            for (k, (icon, label, value, hint)) in infos.iter().enumerate() {
                                sec.fade(ui, n + k, |ui| info_row(ui, *icon, label, value, hint, &t));
                                ui.add_space(20.0);
                            }
                        },
                        |ui| {
                            sec.fade(ui, 2, |ui| {
                                card(ui, id.with("form"), CardLook::of(&t).pad(28.0).hover(false), |ui| submitted = self.form(ui, id, &t));
                            });
                        },
                    );
                }
                ContactLayout::Stacked => {
                    let n = Heading::new(&self.title).eyebrow(self.eyebrow.as_deref()).subtitle(self.subtitle.as_deref()).show(ui, sec, 0);
                    ui.add_space(40.0);
                    let cols = sec.cols(220.0, 3);
                    grid(ui, id.with("info"), 3, cols, 16.0, |ui, k, row_h| {
                        let (icon, label, value, hint) = infos[k];
                        sec.fade(ui, n + k, |ui| {
                            card(ui, id.with(("ic", k)), CardLook::of(&t).min_h(row_h), |ui| {
                                ui.vertical_centered(|ui| {
                                    let (r, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
                                    icon_tile(ui.painter(), r, icon, t.accent, &t);
                                    ui.add_space(10.0);
                                    Tx::new(label).size(16.0).semi().align(Align::Center).color(t.foreground).show(ui);
                                    Tx::new(hint).size(13.5).align(Align::Center).color(t.muted_foreground).show(ui);
                                    ui.add_space(4.0);
                                    Tx::new(value).size(14.0).align(Align::Center).color(t.accent).show(ui);
                                });
                            });
                        });
                    });
                    ui.add_space(32.0);
                    sec.fade(ui, n + 3, |ui| {
                        centered_column(ui, 640.0, |ui| {
                            card(ui, id.with("form"), CardLook::of(&t).pad(28.0).hover(false), |ui| submitted = self.form(ui, id, &t));
                        });
                    });
                }
            }
            sec.submitted = submitted;
        })
    }

    fn form(&self, ui: &mut Ui, id: egui::Id, t: &Theme) -> Option<String> {
        let mut st: ContactState = glowkit_core::state(ui, id.with("cs"), ContactState::default);
        let mut out = None;
        if let Some(at) = st.sent_at {
            let k = glowkit_core::ease(glowkit_core::Easing::OutBack, (((ui.input(|i| i.time) - at) as f32) / 0.6).min(1.0));
            if k < 1.0 {
                ui.ctx().request_repaint();
            }
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(64.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 32.0 * k.max(0.0), with_alpha(t.success, 0.15));
                ui.painter().circle_filled(r.center(), 22.0 * k.max(0.0), t.success);
                UiIcon::Check.paint(ui.painter(), Rect::from_center_size(r.center(), Vec2::splat(24.0 * k.max(0.0))), Color32::WHITE);
                ui.add_space(12.0);
                Tx::new("Message sent").size(20.0).bold().align(Align::Center).color(t.foreground).show(ui);
                Tx::new(format!("Thanks {}, we'll get back to you at {}.", st.name.split_whitespace().next().unwrap_or(""), st.email)).size(14.0).align(Align::Center).color(t.muted_foreground).show(ui);
                ui.add_space(14.0);
                if ui.add(Button::new("Send another").variant(ButtonVariant::Outline)).clicked() {
                    st = ContactState::default();
                }
            });
            ui.add_space(24.0);
            glowkit_core::set_state(ui, id.with("cs"), st);
            return None;
        }
        let name_err = (st.tried && st.name.trim().is_empty()).then(|| "Please tell us your name.".to_string());
        let email_err = (st.tried && !valid_email(&st.email)).then(|| "Enter a valid email address.".to_string());
        let msg_err = (st.tried && st.message.trim().len() < 10).then(|| "A few more words, please (10+ characters).".to_string());
        let wide = ui.available_width() > 420.0;
        if wide {
            let half = (ui.available_width() - 12.0) / 2.0;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                ui.add(Input::new(&mut st.name).label("Name").placeholder("Jane Doe").width(half).error_opt(name_err.clone()).id_salt(id.with("n")));
                ui.add(Input::new(&mut st.email).label("Email").placeholder("jane@company.com").width(half).error_opt(email_err.clone()).id_salt(id.with("e")));
            });
        } else {
            ui.add(Input::new(&mut st.name).label("Name").placeholder("Jane Doe").width(f32::INFINITY).error_opt(name_err).id_salt(id.with("n")));
            ui.add_space(4.0);
            ui.add(Input::new(&mut st.email).label("Email").placeholder("jane@company.com").width(f32::INFINITY).error_opt(email_err).id_salt(id.with("e")));
        }
        ui.add_space(8.0);
        let topics: Vec<&str> = self.topics.iter().map(String::as_str).collect();
        ui.add(Select::new(&mut st.topic, topics).label("Topic").placeholder("Choose a topic").width(ui.available_width()).id_salt(id.with("t")));
        ui.add_space(8.0);
        let mut ta = Textarea::new(&mut st.message).label("Message").placeholder("Tell us a little about your project...").rows(5).width(ui.available_width()).id_salt(id.with("m"));
        if let Some(e) = msg_err {
            ta = ta.error(e);
        }
        ui.add(ta);
        ui.add_space(12.0);
        if ui.add(Button::new(&self.button).size(ControlSize::Lg).width(f32::INFINITY).fill(t.accent).trailing_icon(UiIcon::ArrowRight)).clicked() {
            st.tried = true;
            if !st.name.trim().is_empty() && valid_email(&st.email) && st.message.trim().len() >= 10 {
                st.sent_at = Some(ui.input(|i| i.time));
                let topic = st.topic.and_then(|i| self.topics.get(i)).cloned().unwrap_or_default();
                out = Some(format!("{} <{}> [{}]: {}", st.name.trim(), st.email.trim(), topic, st.message.trim()));
            }
        }
        ui.add_space(6.0);
        Tx::new("By sending, you agree to our privacy policy.").size(12.0).align(Align::Center).color(t.muted_foreground).show(ui);
        glowkit_core::set_state(ui, id.with("cs"), st);
        out
    }
}

/// Icon tile + label/value/hint row.
fn info_row(ui: &mut Ui, icon: UiIcon, label: &str, value: &str, hint: &str, t: &Theme) {
    ui.horizontal_top(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::hover());
        icon_tile(ui.painter(), r, icon, t.accent, t);
        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            Tx::new(label).size(15.5).semi().color(t.foreground).show_inline(ui);
            Tx::new(hint).size(13.5).color(t.muted_foreground).show_inline(ui);
            Tx::new(value).size(14.0).color(t.accent).show_inline(ui);
        });
    });
}
