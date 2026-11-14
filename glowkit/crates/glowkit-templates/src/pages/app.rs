//! In-product pages: onboarding wizard, blog post article and user profile.

use super::frame::{page, page_setters, PageOpts};
use crate::sections::*;
use egui::{pos2, vec2, Align, Color32, CornerRadius, CursorIcon, FontId, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use glowkit_components::{
    Avatar, AvatarGroup, AvatarStyle, Badge, Button, ButtonVariant, Chip, CodeBlock, ControlSize, Input, Progress,
    Stepper, Switch, Tabs, TabsStyle, UiIcon,
};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

#[derive(Clone)]
struct OnboardState {
    step: usize,
    name: String,
    role: usize,
    workspace: String,
    theme: usize,
    notify_email: bool,
    notify_digest: bool,
    invite: String,
    invited: Vec<String>,
    tried: bool,
    changed_at: f64,
}

/// A multi-step onboarding wizard: stepper, per-step forms (profile, workspace, preferences,
/// invite team), validation, back / continue buttons with an animated step transition and a
/// final "all set" screen.
///
/// Finishing reports `"<name> | <workspace>"` in [`SectionResponse::submitted`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::OnboardingPage;
/// if let Some(done) = OnboardingPage::new("Acme").show(ui).submitted { /* create workspace */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct OnboardingPage {
    brand: String,
    roles: Vec<String>,
    background: SectionBackground,
    opts: PageOpts,
    salt: Option<String>,
}

impl OnboardingPage {
    /// An onboarding wizard for `brand`.
    pub fn new(brand: impl Into<String>) -> Self {
        Self {
            brand: brand.into(),
            roles: ["Engineering", "Design", "Product", "Founder", "Other"].iter().map(|s| s.to_string()).collect(),
            background: SectionBackground::Dots,
            opts: PageOpts::default(),
            salt: None,
        }
    }
    /// Role choices on the first step.
    pub fn roles(mut self, r: &[&str]) -> Self {
        self.roles = r.iter().map(|s| s.to_string()).collect();
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
        let id = section_id(ui, "onboarding_page", &self.salt);
        page(ui, id, self.opts, |ui, p| {
            let h = p.height;
            let r =
                section(ui, Frame::new(id.with("body"), self.background).pad(40.0).min_h(h).max_w(720.0), |ui, sec| {
                    let t = sec.t.clone();
                    let now = ui.input(|i| i.time);
                    let mut st: OnboardState = glowkit_core::state(ui, id.with("st"), || OnboardState {
                        step: 0,
                        name: String::new(),
                        role: 0,
                        workspace: String::new(),
                        theme: 0,
                        notify_email: true,
                        notify_digest: false,
                        invite: String::new(),
                        invited: vec!["mia@company.com".into()],
                        tried: false,
                        changed_at: -10.0,
                    });
                    let steps = [
                        ("Profile", "About you"),
                        ("Workspace", "Name it"),
                        ("Preferences", "Make it yours"),
                        ("Team", "Invite people"),
                    ];
                    let n = steps.len();
                    row(ui, id.with("brand"), Align::Center, 0.0, |ui| {
                        brand(ui, &self.brand, 15.0);
                    });
                    ui.add_space(24.0);
                    let mut cur = st.step.min(n - 1);
                    if !sec.narrow {
                        sec.fade(ui, 0, |ui| {
                            ui.add(Stepper::new(&mut cur, &steps));
                        });
                    } else {
                        ui.add(Progress::new((st.step as f32 + 1.0) / (n as f32 + 1.0)));
                    }
                    ui.add_space(28.0);
                    let pk = glowkit_core::ease(
                        glowkit_core::Easing::OutCubic,
                        (((now - st.changed_at) as f32) / 0.4).clamp(0.0, 1.0),
                    );
                    if pk < 1.0 {
                        ui.ctx().request_repaint();
                    }
                    let mut clicked = None;
                    let mut submitted = None;
                    sec.fade(ui, 1, |ui| {
                        card(
                            ui,
                            id.with("card"),
                            CardLook::of(&t).pad(if sec.narrow { 20.0 } else { 32.0 }).hover(false),
                            |ui| {
                                fade_up(ui, pk, 12.0, |ui| {
                                    if st.step >= n {
                                        let k = glowkit_core::ease(glowkit_core::Easing::OutBack, pk);
                                        let (r, _) =
                                            ui.allocate_exact_size(vec2(ui.available_width(), 80.0), Sense::hover());
                                        ui.painter().circle_filled(r.center(), 34.0 * k, with_alpha(t.success, 0.15));
                                        ui.painter().circle_filled(r.center(), 24.0 * k, t.success);
                                        UiIcon::Check.paint(
                                            ui.painter(),
                                            Rect::from_center_size(r.center(), Vec2::splat(22.0 * k)),
                                            Color32::WHITE,
                                        );
                                        ui.add_space(8.0);
                                        Tx::new(format!(
                                            "You're all set, {}!",
                                            st.name.split_whitespace().next().unwrap_or("friend")
                                        ))
                                        .size(26.0)
                                        .bold()
                                        .align(Align::Center)
                                        .color(t.foreground)
                                        .show(ui);
                                        Tx::new(format!(
                                            "{} is ready. We've sent {} invitation(s).",
                                            if st.workspace.is_empty() {
                                                "Your workspace"
                                            } else {
                                                st.workspace.as_str()
                                            },
                                            st.invited.len()
                                        ))
                                        .size(15.0)
                                        .align(Align::Center)
                                        .color(t.muted_foreground)
                                        .show(ui);
                                        ui.add_space(20.0);
                                        row(ui, id.with("done"), Align::Center, 10.0, |ui| {
                                            if ui.add(Button::new("Start over").variant(ButtonVariant::Ghost)).clicked()
                                            {
                                                st.step = 0;
                                                st.changed_at = now;
                                                clicked = Some("Start over".to_string());
                                            }
                                            if ui
                                                .add(
                                                    Button::new("Open dashboard")
                                                        .size(ControlSize::Lg)
                                                        .fill(t.accent)
                                                        .trailing_icon(UiIcon::ArrowRight),
                                                )
                                                .clicked()
                                            {
                                                clicked = Some("Open dashboard".to_string());
                                            }
                                        });
                                        return;
                                    }
                                    let (title, sub) = match st.step {
                                        0 => ("Tell us about yourself", "We'll personalize your experience."),
                                        1 => ("Create your workspace", "This is where your team's projects live."),
                                        2 => ("Set your preferences", "You can change these anytime in settings."),
                                        _ => ("Invite your team", "Collaboration is better together."),
                                    };
                                    Tx::new(format!("Step {} of {}", st.step + 1, n))
                                        .size(12.5)
                                        .semi()
                                        .color(t.accent)
                                        .show(ui);
                                    Tx::new(title).size(24.0).bold().color(t.foreground).show(ui);
                                    Tx::new(sub).size(14.5).color(t.muted_foreground).show(ui);
                                    ui.add_space(20.0);
                                    match st.step {
                                        0 => {
                                            let err = (st.tried && st.name.trim().len() < 2)
                                                .then(|| "Please enter your name.".to_string());
                                            ui.add(
                                                Input::new(&mut st.name)
                                                    .label("Full name")
                                                    .placeholder("Jane Doe")
                                                    .icon(UiIcon::User)
                                                    .width(f32::INFINITY)
                                                    .error_opt(err)
                                                    .id_salt(id.with("name")),
                                            );
                                            ui.add_space(12.0);
                                            Tx::new("What do you do?").size(13.5).semi().color(t.foreground).show(ui);
                                            ui.add_space(4.0);
                                            let w = ui.available_width();
                                            let cols = cols_for(w, 120.0, 8.0, 3);
                                            let roles = &self.roles;
                                            let mut pick = None;
                                            grid(ui, id.with("roles"), roles.len(), cols, 8.0, |ui, i, _| {
                                                if choice_tile(
                                                    ui,
                                                    &roles[i],
                                                    [
                                                        UiIcon::Code,
                                                        UiIcon::Sparkles,
                                                        UiIcon::Activity,
                                                        UiIcon::Zap,
                                                        UiIcon::User,
                                                    ][i % 5],
                                                    st.role == i,
                                                    &t,
                                                )
                                                .clicked()
                                                {
                                                    pick = Some(i);
                                                }
                                            });
                                            if let Some(i) = pick {
                                                st.role = i;
                                            }
                                        }
                                        1 => {
                                            let err = (st.tried && st.workspace.trim().len() < 2)
                                                .then(|| "Give your workspace a name.".to_string());
                                            ui.add(
                                                Input::new(&mut st.workspace)
                                                    .label("Workspace name")
                                                    .placeholder("Acme Inc.")
                                                    .icon(UiIcon::Home)
                                                    .width(f32::INFINITY)
                                                    .error_opt(err)
                                                    .id_salt(id.with("ws")),
                                            );
                                            ui.add_space(12.0);
                                            let mut slug: String = st
                                                .workspace
                                                .to_lowercase()
                                                .split_whitespace()
                                                .collect::<Vec<_>>()
                                                .join("-")
                                                .chars()
                                                .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                                                .collect();
                                            ui.add(
                                                Input::new(&mut slug)
                                                    .label("URL")
                                                    .prefix("glowkit.app/")
                                                    .placeholder("acme")
                                                    .width(f32::INFINITY)
                                                    .disabled(true)
                                                    .description("Generated from the workspace name.")
                                                    .id_salt(id.with("slug")),
                                            );
                                        }
                                        2 => {
                                            Tx::new("Theme").size(13.5).semi().color(t.foreground).show(ui);
                                            ui.add_space(4.0);
                                            let mut pick = None;
                                            grid(ui, id.with("themes"), 3, 3, 10.0, |ui, i, _| {
                                                if theme_tile(ui, ["Dark", "Light", "System"][i], i, st.theme == i, &t)
                                                    .clicked()
                                                {
                                                    pick = Some(i);
                                                }
                                            });
                                            if let Some(i) = pick {
                                                st.theme = i;
                                            }
                                            ui.add_space(16.0);
                                            ui.add(
                                                Switch::new(&mut st.notify_email)
                                                    .label("Email notifications")
                                                    .description("Mentions, replies and assignments."),
                                            );
                                            ui.add_space(6.0);
                                            ui.add(
                                                Switch::new(&mut st.notify_digest)
                                                    .label("Weekly digest")
                                                    .description("A summary of your team's activity every Monday."),
                                            );
                                        }
                                        _ => {
                                            let w = ui.available_width();
                                            let mut add = false;
                                            ui.horizontal(|ui| {
                                                let r = ui.add(
                                                    Input::new(&mut st.invite)
                                                        .placeholder("teammate@company.com")
                                                        .icon(UiIcon::Mail)
                                                        .width(w - 96.0)
                                                        .id_salt(id.with("inv")),
                                                );
                                                add |= r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                                add |= ui
                                                    .add(
                                                        Button::new("Invite")
                                                            .variant(ButtonVariant::Secondary)
                                                            .width(88.0)
                                                            .icon(UiIcon::Plus),
                                                    )
                                                    .clicked();
                                            });
                                            let bad = add && !valid_email(&st.invite);
                                            if add
                                                && valid_email(&st.invite)
                                                && !st.invited.contains(&st.invite.trim().to_string())
                                            {
                                                st.invited.push(st.invite.trim().to_string());
                                                st.invite.clear();
                                            }
                                            if bad {
                                                Tx::new("Enter a valid email address.")
                                                    .size(12.5)
                                                    .color(t.destructive)
                                                    .show(ui);
                                            }
                                            ui.add_space(10.0);
                                            let mut remove = None;
                                            for (k, e) in st.invited.iter().enumerate() {
                                                ui.horizontal(|ui| {
                                                    ui.add(
                                                        Avatar::new(e)
                                                            .size(30.0)
                                                            .style(AvatarStyle::Soft)
                                                            .color(t.chart[k % 6]),
                                                    );
                                                    Tx::new(e).size(14.0).color(t.foreground).show_inline(ui);
                                                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                                        if ui
                                                            .add(
                                                                Button::icon_only(UiIcon::X)
                                                                    .variant(ButtonVariant::Ghost)
                                                                    .size(ControlSize::Sm),
                                                            )
                                                            .clicked()
                                                        {
                                                            remove = Some(k);
                                                        }
                                                        ui.add(Badge::new("Pending").color(t.warning).pill(true));
                                                    });
                                                });
                                            }
                                            if let Some(k) = remove {
                                                st.invited.remove(k);
                                            }
                                        }
                                    }
                                    ui.add_space(24.0);
                                    divider(ui, None);
                                    ui.add_space(16.0);
                                    ui.horizontal(|ui| {
                                        if st.step > 0
                                            && ui
                                                .add(
                                                    Button::new("Back")
                                                        .variant(ButtonVariant::Ghost)
                                                        .icon(UiIcon::ArrowLeft),
                                                )
                                                .clicked()
                                        {
                                            st.step -= 1;
                                            st.changed_at = now;
                                            st.tried = false;
                                            clicked = Some("Back".to_string());
                                        }
                                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                            let last = st.step + 1 == n;
                                            if ui
                                                .add(
                                                    Button::new(if last { "Finish setup" } else { "Continue" })
                                                        .fill(t.accent)
                                                        .trailing_icon(UiIcon::ArrowRight),
                                                )
                                                .clicked()
                                            {
                                                st.tried = true;
                                                let ok = match st.step {
                                                    0 => st.name.trim().len() >= 2,
                                                    1 => st.workspace.trim().len() >= 2,
                                                    _ => true,
                                                };
                                                if ok {
                                                    st.step += 1;
                                                    st.tried = false;
                                                    st.changed_at = now;
                                                    clicked = Some(
                                                        if last { "Finish setup" } else { "Continue" }.to_string(),
                                                    );
                                                    if last {
                                                        submitted = Some(format!(
                                                            "{} | {}",
                                                            st.name.trim(),
                                                            st.workspace.trim()
                                                        ));
                                                    }
                                                }
                                            }
                                            if st.step == 3
                                                && ui
                                                    .add(Button::new("Skip for now").variant(ButtonVariant::Ghost))
                                                    .clicked()
                                            {
                                                st.step = n;
                                                st.changed_at = now;
                                                clicked = Some("Skip for now".to_string());
                                            }
                                        });
                                    });
                                });
                            },
                        );
                    });
                    if cur != st.step.min(n - 1) && cur < st.step {
                        // clicking a completed step in the stepper goes back to it
                        st.step = cur;
                        st.changed_at = now;
                    }
                    glowkit_core::set_state(ui, id.with("st"), st);
                    if let Some(c) = clicked {
                        sec.click(c);
                    }
                    sec.submitted = submitted;
                });
            p.take(r);
        })
    }
}

/// A selectable tile with icon and label.
fn choice_tile(ui: &mut Ui, label: &str, icon: UiIcon, selected: bool, t: &Theme) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    let hv = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let sel = ui.ctx().animate_bool_with_time(resp.id.with("sel"), selected, 0.18);
    let p = ui.painter();
    p.rect(
        r,
        CornerRadius::same((t.radius * 0.9) as u8),
        mix(mix(t.card, t.secondary, 0.4 * hv), with_alpha(t.accent, 0.12), sel * 0.8),
        Stroke::new(1.0 + sel, mix(t.border, t.accent, sel)),
        StrokeKind::Inside,
    );
    icon.paint(
        p,
        Rect::from_center_size(pos2(r.min.x + 22.0, r.center().y), Vec2::splat(16.0)),
        mix(t.muted_foreground, t.accent, sel),
    );
    p.text(
        pos2(r.min.x + 40.0, r.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        t.foreground,
    );
    if sel > 0.0 {
        check_dot(p, pos2(r.max.x - 18.0, r.center().y), 8.0 * sel, t.accent, t);
    }
    resp
}

/// A theme choice with a tiny painted app preview.
fn theme_tile(ui: &mut Ui, label: &str, i: usize, selected: bool, t: &Theme) -> egui::Response {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, w * 0.62 + 30.0), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    let sel = ui.ctx().animate_bool_with_time(resp.id, selected, 0.18);
    let p = ui.painter();
    let prev = Rect::from_min_size(r.min, vec2(w, w * 0.62));
    let (bg, fg, card) = match i {
        0 => (glowkit_core::hex("#0B0B0F"), glowkit_core::hex("#3F3F46"), glowkit_core::hex("#18181B")),
        1 => (glowkit_core::hex("#FFFFFF"), glowkit_core::hex("#D4D4D8"), glowkit_core::hex("#F4F4F5")),
        _ => (glowkit_core::hex("#0B0B0F"), glowkit_core::hex("#3F3F46"), glowkit_core::hex("#18181B")),
    };
    let cr = CornerRadius::same((t.radius * 0.9) as u8);
    p.rect_filled(prev, cr, bg);
    if i == 2 {
        // half light, half dark
        let half = Rect::from_min_max(pos2(prev.center().x, prev.min.y), prev.max);
        p.rect_filled(half, CornerRadius { nw: 0, sw: 0, ne: cr.ne, se: cr.se }, glowkit_core::hex("#FFFFFF"));
    }
    let inner = prev.shrink(8.0);
    p.rect_filled(
        Rect::from_min_size(inner.min, vec2(inner.width() * 0.28, inner.height())),
        CornerRadius::same(4),
        card,
    );
    for k in 0..3 {
        p.rect_filled(
            Rect::from_min_size(
                pos2(inner.min.x + inner.width() * 0.34, inner.min.y + k as f32 * 12.0),
                vec2(inner.width() * (0.6 - k as f32 * 0.12), 6.0),
            ),
            CornerRadius::same(3),
            fg,
        );
    }
    p.rect_filled(
        Rect::from_min_size(pos2(inner.min.x + inner.width() * 0.34, inner.max.y - 12.0), vec2(28.0, 10.0)),
        CornerRadius::same(3),
        t.accent,
    );
    p.rect_stroke(prev, cr, Stroke::new(1.0 + 1.5 * sel, mix(t.border, t.accent, sel)), StrokeKind::Inside);
    p.text(
        pos2(r.min.x + 2.0, prev.max.y + 15.0),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.5),
        mix(t.muted_foreground, t.foreground, sel),
    );
    if sel > 0.0 {
        check_dot(p, pos2(r.max.x - 10.0, prev.max.y + 15.0), 8.0 * sel, t.accent, t);
    }
    resp
}

// ---------------------------------------------------------------------------------------------

/// A block of article content for [`BlogPostPage`].
#[derive(Clone, Debug)]
pub enum ArticleBlock {
    /// A paragraph of body text.
    Paragraph(String),
    /// A section heading.
    Heading(String),
    /// A pull quote with attribution.
    Quote(String, String),
    /// A code sample with its language.
    Code(String, String),
    /// A bulleted list.
    List(Vec<String>),
    /// An illustration with a caption.
    Figure(String),
    /// A tinted callout box.
    Callout(String),
}

impl ArticleBlock {
    /// Placeholder article content.
    pub fn defaults() -> Vec<ArticleBlock> {
        vec![
            ArticleBlock::Paragraph("Motion is the difference between an interface that works and one that feels alive. On the web we've had CSS transitions and a decade of animation libraries; on the desktop, immediate-mode GUIs have mostly been static. It doesn't have to be that way.".into()),
            ArticleBlock::Paragraph("In this post we walk through the three primitives behind every animation in glowkit, and show how to use them in your own widgets.".into()),
            ArticleBlock::Heading("Springs, not durations".into()),
            ArticleBlock::Paragraph("A spring has no fixed duration: it settles when it settles. That makes it interruptible by design. If the user hovers, leaves and hovers again, the value simply changes direction instead of restarting.".into()),
            ArticleBlock::Code("rust".into(), "let x = glowkit_core::anim::spring(ui, id, target, 400.0, 30.0);\npainter.circle_filled(pos2(x, y), 6.0, accent);".into()),
            ArticleBlock::Quote("The best animation is the one you only notice when it's missing.".into(), "Mia Park, Head of Design".into()),
            ArticleBlock::Heading("Stagger everything".into()),
            ArticleBlock::List(vec!["Delay each item by 60-80ms".into(), "Keep the total under 600ms".into(), "Animate opacity and a small offset, never layout".into()]),
            ArticleBlock::Figure("Staggered entrance of a bento grid: each card fades up 80ms after the previous one.".into()),
            ArticleBlock::Callout("Tip: respect reduced-motion preferences by shortening durations instead of removing feedback entirely.".into()),
            ArticleBlock::Paragraph("That's it. Three primitives, a little taste, and your app will feel twice as polished.".into()),
        ]
    }
}

/// A blog post page: header, article with category, title, author meta, cover, rich prose
/// (headings, quotes, code, lists, figures, callouts), share buttons, author card, related
/// posts and footer. Clicked related posts are reported by title.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{BlogPostPage, ArticleBlock};
/// BlogPostPage::new("Designing motion that feels native").body(ArticleBlock::defaults()).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct BlogPostPage {
    brand: String,
    title: String,
    subtitle: String,
    category: String,
    author: String,
    role: String,
    date: String,
    read_time: String,
    body: Vec<ArticleBlock>,
    opts: PageOpts,
    salt: Option<String>,
}

impl BlogPostPage {
    /// A blog post with the given title and placeholder content.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            brand: "Glowkit".into(),
            title: title.into(),
            subtitle: "Springs, staggers and the small details that make immediate-mode interfaces feel alive.".into(),
            category: "Design".into(),
            author: "Mia Park".into(),
            role: "Head of Design".into(),
            date: "Nov 2, 2026".into(),
            read_time: "7 min read".into(),
            body: ArticleBlock::defaults(),
            opts: PageOpts::default(),
            salt: None,
        }
    }
    /// Brand in the header and footer.
    pub fn brand(mut self, b: impl Into<String>) -> Self {
        self.brand = b.into();
        self
    }
    /// Standfirst under the title.
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = s.into();
        self
    }
    /// Category badge.
    pub fn category(mut self, c: impl Into<String>) -> Self {
        self.category = c.into();
        self
    }
    /// Author name and role.
    pub fn author(mut self, name: impl Into<String>, role: impl Into<String>) -> Self {
        self.author = name.into();
        self.role = role.into();
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
    /// Article content.
    pub fn body(mut self, b: Vec<ArticleBlock>) -> Self {
        self.body = b;
        self
    }
    page_setters!();

    /// Show the page.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "blog_post_page", &self.salt);
        page(ui, id, self.opts, |ui, p| {
            p.take(
                HeaderSection::new(&self.brand).links(&["Product", "Blog", "Docs", "Changelog"]).id_salt("bp").show(ui),
            );
            let r = section(
                ui,
                Frame::new(id.with("article"), SectionBackground::Glow).pad(56.0).max_w(720.0),
                |ui, sec| {
                    let t = sec.t.clone();
                    let mut clicked = None;
                    sec.fade(ui, 0, |ui| {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                            UiIcon::ArrowLeft.paint(ui.painter(), r, t.muted_foreground);
                            if link(ui, "All posts", 13.5, t.muted_foreground, t.foreground).clicked() {
                                clicked = Some("All posts".to_string());
                            }
                        });
                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            ui.add(Badge::new(&self.category).color(t.accent).pill(true));
                            Tx::new(format!("{}  ·  {}", self.date, self.read_time))
                                .size(13.0)
                                .color(t.muted_foreground)
                                .show_inline(ui);
                        });
                    });
                    ui.add_space(14.0);
                    sec.fade(ui, 1, |ui| {
                        Tx::new(&self.title)
                            .size(if sec.narrow { 32.0 } else { 44.0 })
                            .bold()
                            .line(1.15)
                            .color(t.foreground)
                            .show(ui);
                        ui.add_space(12.0);
                        Tx::new(&self.subtitle).size(19.0).line(1.5).color(t.muted_foreground).show(ui);
                    });
                    ui.add_space(24.0);
                    sec.fade(ui, 2, |ui| {
                        ui.horizontal(|ui| {
                            ui.add(Avatar::new(&self.author).size(40.0).style(AvatarStyle::Solid).color(t.chart[2]));
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 1.0;
                                Tx::new(&self.author).size(14.5).semi().color(t.foreground).show_inline(ui);
                                Tx::new(&self.role).size(13.0).color(t.muted_foreground).show_inline(ui);
                            });
                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                for s in [SectionSocial::LinkedIn, SectionSocial::X] {
                                    if social_button(ui, s, 32.0).clicked() {
                                        clicked = Some(format!("Share on {}", s.label()));
                                    }
                                }
                                if ui
                                    .add(
                                        Button::icon_only(UiIcon::Link)
                                            .variant(ButtonVariant::Outline)
                                            .size(ControlSize::Sm),
                                    )
                                    .on_hover_text("Copy link")
                                    .clicked()
                                {
                                    clicked = Some("Copy link".to_string());
                                }
                            });
                        });
                    });
                    ui.add_space(28.0);
                    sec.fade(ui, 3, |ui| {
                        let w = ui.available_width();
                        let (r, _) = ui.allocate_exact_size(vec2(w, (w * 0.5).min(360.0)), Sense::hover());
                        paint_art(ui.painter(), r, 7, &t, (t.radius * 1.6).round(), glowkit_core::time(ui) * 0.3);
                        let tile = Rect::from_center_size(r.center(), Vec2::splat(84.0));
                        paint::soft_shadow(
                            ui.painter(),
                            tile,
                            22.0,
                            30.0,
                            vec2(0.0, 10.0),
                            with_alpha(Color32::BLACK, 0.35),
                        );
                        paint::gradient_rect(ui.painter(), tile, 22.0, 135.0, &[t.gradient[0], t.gradient[1]]);
                        UiIcon::Sparkles.paint(ui.painter(), tile.shrink(24.0), Color32::WHITE);
                    });
                    ui.add_space(32.0);
                    for (k, block) in self.body.iter().enumerate() {
                        sec.fade(ui, 4 + k.min(6), |ui| article_block(ui, block, k, &t));
                    }
                    ui.add_space(32.0);
                    divider(ui, None);
                    ui.add_space(24.0);
                    sec.fade(ui, 10, |ui| {
                    card(ui, id.with("author"), CardLook::of(&t).pad(20.0).hover(false), |ui| {
                        ui.horizontal_top(|ui| {
                            ui.add(Avatar::new(&self.author).size(56.0).style(AvatarStyle::Solid).color(t.chart[2]));
                            ui.add_space(6.0);
                            ui.vertical(|ui| {
                                Tx::new("Written by").size(12.0).color(t.muted_foreground).show(ui);
                                Tx::new(&self.author).size(17.0).semi().color(t.foreground).show(ui);
                                Tx::new(format!("{}. Obsessed with motion, typography and tiny details. Writes about design systems and interaction.", self.role)).size(14.0).line(1.5).color(t.muted_foreground).show(ui);
                            });
                        });
                    });
                });
                    if let Some(c) = clicked {
                        sec.click(c);
                    }
                },
            );
            p.take(r);
            p.take(
                BlogSection::new("Keep reading")
                    .eyebrow("Related")
                    .posts(BlogPost::defaults().into_iter().skip(1).take(3).collect())
                    .background(SectionBackground::Muted)
                    .id_salt("bp")
                    .show(ui),
            );
            p.take(NewsletterSection::new("Get new posts in your inbox").id_salt("bp").show(ui));
            p.take(FooterSection::new(&self.brand).layout(FooterLayout::Simple).id_salt("bp").show(ui));
        })
    }
}

/// Render one block of article prose.
fn article_block(ui: &mut Ui, block: &ArticleBlock, k: usize, t: &Theme) {
    match block {
        ArticleBlock::Paragraph(s) => {
            Tx::new(s).size(17.0).line(1.75).color(mix(t.foreground, t.muted_foreground, 0.25)).show(ui);
            ui.add_space(18.0);
        }
        ArticleBlock::Heading(s) => {
            ui.add_space(14.0);
            Tx::new(s).size(26.0).bold().line(1.25).color(t.foreground).show(ui);
            ui.add_space(12.0);
        }
        ArticleBlock::Quote(q, who) => {
            ui.add_space(6.0);
            let top = ui.cursor().min;
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(Rect::from_min_size(
                        top + vec2(24.0, 4.0),
                        vec2(ui.available_width() - 24.0, f32::INFINITY),
                    ))
                    .layout(egui::Layout::top_down(Align::Min)),
            );
            Tx::new(q).size(22.0).line(1.5).color(t.foreground).show(&mut child);
            child.add_space(8.0);
            Tx::new(format!("— {who}")).size(14.0).color(t.muted_foreground).show(&mut child);
            let r = child.min_rect();
            paint::gradient_rect(
                ui.painter(),
                Rect::from_min_max(top, pos2(top.x + 3.0, r.max.y + 4.0)),
                1.5,
                90.0,
                &[t.gradient[0], t.gradient[1]],
            );
            ui.advance_cursor_after_rect(Rect::from_min_max(top, pos2(top.x + ui.available_width(), r.max.y + 4.0)));
            ui.add_space(24.0);
        }
        ArticleBlock::Code(lang, code) => {
            ui.add(
                CodeBlock::new(code)
                    .language(lang)
                    .title(format!("example.{}", if lang == "rust" { "rs" } else { lang.as_str() }))
                    .width(ui.available_width()),
            );
            ui.add_space(20.0);
        }
        ArticleBlock::List(items) => {
            for it in items {
                ui.horizontal_top(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(18.0, 28.0), Sense::hover());
                    ui.painter().circle_filled(pos2(r.min.x + 7.0, r.min.y + 14.0), 3.0, t.accent);
                    Tx::new(it).size(17.0).line(1.65).color(mix(t.foreground, t.muted_foreground, 0.25)).show(ui);
                });
            }
            ui.add_space(18.0);
        }
        ArticleBlock::Figure(caption) => {
            let w = ui.available_width();
            let (r, _) = ui.allocate_exact_size(vec2(w, w * 0.42), Sense::hover());
            paint_art(ui.painter(), r, k + 11, t, (t.radius * 1.2).round(), 0.0);
            paint_mini(ui, ui.painter(), r.shrink(r.height() * 0.2), FeatureVisual::Bars, t, k, true);
            ui.add_space(8.0);
            Tx::new(caption).size(13.5).line(1.5).align(Align::Center).color(t.muted_foreground).show(ui);
            ui.add_space(22.0);
        }
        ArticleBlock::Callout(s) => {
            card(
                ui,
                ui.scope_id().with(("callout", k)),
                CardLook::of(t)
                    .pad(16.0)
                    .hover(false)
                    .fill(with_alpha(t.accent, if t.dark { 0.08 } else { 0.05 }))
                    .stroke(with_alpha(t.accent, 0.3))
                    .shadow(false),
                |ui| {
                    ui.horizontal_top(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
                        UiIcon::Info.paint(ui.painter(), r, t.accent);
                        Tx::new(s).size(15.0).line(1.6).color(t.foreground).show(ui);
                    });
                },
            );
            ui.add_space(20.0);
        }
    }
}

// ---------------------------------------------------------------------------------------------

/// A user profile page: cover banner, overlapping avatar, name, handle, bio and meta, follow /
/// message buttons (follow toggles), animated stats, tabs (overview / projects / activity) with
/// pinned projects, skills and a contribution graph.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::ProfilePage;
/// let r = ProfilePage::new("Ava Chen", "@ava").bio("Design engineer.").show(ui);
/// if r.clicked("Follow") { /* ... */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct ProfilePage {
    name: String,
    handle: String,
    bio: String,
    location: String,
    website: String,
    stats: [(f64, String); 3],
    skills: Vec<String>,
    projects: Vec<(String, String, u32)>,
    opts: PageOpts,
    salt: Option<String>,
}

impl ProfilePage {
    /// A profile for `name` / `handle` with placeholder details.
    pub fn new(name: impl Into<String>, handle: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            handle: handle.into(),
            bio: "Design engineer building delightful tools for Rust. Previously design systems at a fintech. Coffee, synths and type.".into(),
            location: "San Francisco, CA".into(),
            website: "ava.dev".into(),
            stats: [(12_480.0, "Followers".into()), (318.0, "Following".into()), (42.0, "Projects".into())],
            skills: ["Rust", "egui", "WebAssembly", "Motion design", "Design systems", "Shaders"].iter().map(|s| s.to_string()).collect(),
            projects: vec![
                ("glowkit".into(), "Animated UI components for egui.".into(), 4210),
                ("aurora-rs".into(), "GPU aurora backgrounds in 200 lines.".into(), 980),
                ("spring-lab".into(), "Interactive spring physics playground.".into(), 612),
                ("type-scale".into(), "Generate modular type scales.".into(), 301),
            ],
            opts: PageOpts::default(),
            salt: None,
        }
    }
    /// Bio text.
    pub fn bio(mut self, b: impl Into<String>) -> Self {
        self.bio = b.into();
        self
    }
    /// Location line.
    pub fn location(mut self, l: impl Into<String>) -> Self {
        self.location = l.into();
        self
    }
    /// Website line.
    pub fn website(mut self, w: impl Into<String>) -> Self {
        self.website = w.into();
        self
    }
    /// Three stats as (value, label).
    pub fn stats(mut self, s: [(f64, &str); 3]) -> Self {
        self.stats = s.map(|(v, l)| (v, l.to_string()));
        self
    }
    /// Skill chips.
    pub fn skills(mut self, s: &[&str]) -> Self {
        self.skills = s.iter().map(|x| x.to_string()).collect();
        self
    }
    /// Pinned projects as (name, description, stars).
    pub fn projects(mut self, p: &[(&str, &str, u32)]) -> Self {
        self.projects = p.iter().map(|(a, b, c)| (a.to_string(), b.to_string(), *c)).collect();
        self
    }
    page_setters!();

    /// Show the page.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "profile_page", &self.salt);
        page(ui, id, self.opts, |ui, p| {
            p.take(
                HeaderSection::new("Glowkit")
                    .links(&["Explore", "Projects", "Community"])
                    .cta("New project")
                    .id_salt("pf")
                    .show(ui),
            );
            let r = section(
                ui,
                Frame::new(id.with("body"), SectionBackground::Plain).pad(24.0).max_w(1000.0),
                |ui, sec| {
                    let t = sec.t.clone();
                    let mut clicked = None;
                    let w = ui.available_width();
                    // cover
                    let (cover, _) =
                        ui.allocate_exact_size(vec2(w, if sec.narrow { 140.0 } else { 200.0 }), Sense::hover());
                    paint_art(ui.painter(), cover, 5, &t, (t.radius * 1.6).round(), glowkit_core::time(ui) * 0.3);
                    // avatar overlapping the cover
                    let av = 112.0;
                    let av_rect =
                        Rect::from_min_size(pos2(cover.min.x + 28.0, cover.max.y - av * 0.5), Vec2::splat(av));
                    ui.painter().circle_filled(av_rect.center(), av / 2.0 + 5.0, t.background);
                    Avatar::new(&self.name).size(av).style(AvatarStyle::Solid).color(t.chart[0]).paint(ui, av_rect);
                    ui.painter().circle_filled(av_rect.right_bottom() - vec2(16.0, 16.0), 9.0, t.background);
                    ui.painter().circle_filled(av_rect.right_bottom() - vec2(16.0, 16.0), 6.0, t.success);
                    // actions row to the right of the avatar
                    let mut following: bool = glowkit_core::state(ui, id.with("follow"), || false);
                    ui.horizontal(|ui| {
                        ui.set_min_height(64.0);
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add(Button::icon_only(UiIcon::MoreHorizontal).variant(ButtonVariant::Outline))
                                .clicked()
                            {
                                clicked = Some("More".to_string());
                            }
                            if ui
                                .add(Button::new("Message").variant(ButtonVariant::Outline).icon(UiIcon::Mail))
                                .clicked()
                            {
                                clicked = Some("Message".to_string());
                            }
                            let fb = if following {
                                Button::new("Following").variant(ButtonVariant::Secondary).icon(UiIcon::Check)
                            } else {
                                Button::new("Follow").fill(t.accent).icon(UiIcon::Plus)
                            };
                            if ui.add(fb).clicked() {
                                clicked = Some(if following { "Unfollow" } else { "Follow" }.to_string());
                                following = !following;
                            }
                        });
                    });
                    glowkit_core::set_state(ui, id.with("follow"), following);
                    ui.add_space(8.0);
                    sec.fade(ui, 0, |ui| {
                        ui.horizontal(|ui| {
                            Tx::new(&self.name).size(28.0).bold().color(t.foreground).show_inline(ui);
                            ui.add(Badge::new("Pro").color(t.accent).pill(true).icon(UiIcon::Sparkles));
                        });
                        Tx::new(&self.handle).size(15.0).color(t.muted_foreground).show(ui);
                        ui.add_space(10.0);
                        Tx::new(&self.bio).size(15.5).line(1.6).max_width(640.0).color(t.foreground).show(ui);
                        ui.add_space(10.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 18.0;
                            for (icon, s) in [
                                (UiIcon::Globe, self.location.as_str()),
                                (UiIcon::Link, self.website.as_str()),
                                (UiIcon::Calendar, "Joined March 2023"),
                            ] {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    let (r, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                                    icon.paint(ui.painter(), r, t.muted_foreground);
                                    if icon == UiIcon::Link {
                                        if link(ui, s, 14.0, t.accent, t.foreground).clicked() {
                                            clicked = Some(s.to_string());
                                        }
                                    } else {
                                        Tx::new(s).size(14.0).color(t.muted_foreground).show_inline(ui);
                                    }
                                });
                            }
                        });
                    });
                    ui.add_space(20.0);
                    sec.fade(ui, 1, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 28.0;
                            for (k, (v, label)) in self.stats.iter().enumerate() {
                                let extra = if k == 0 && following { 1.0 } else { 0.0 };
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    ui.add(
                                        glowkit_text::CountUp::new(if sec.seen { v + extra } else { 0.0 })
                                            .from(0.0)
                                            .duration(1.2)
                                            .size(18.0)
                                            .bold(true)
                                            .color(t.foreground),
                                    );
                                    Tx::new(label).size(14.0).color(t.muted_foreground).show_inline(ui);
                                });
                            }
                            ui.add(
                                AvatarGroup::new(&["Liam Ortiz", "Mia Park", "Noah Smith"])
                                    .size(26.0)
                                    .ring(t.background),
                            );
                        });
                    });
                    ui.add_space(24.0);
                    let mut tab: usize = glowkit_core::state(ui, id.with("tab"), || 0);
                    ui.add(Tabs::new(&mut tab, &["Overview", "Projects", "Activity"]).style(TabsStyle::Underline));
                    glowkit_core::set_state(ui, id.with("tab"), tab);
                    ui.add_space(24.0);
                    let projects = |ui: &mut Ui, clicked: &mut Option<String>, n: usize| {
                        let cols = cols_for(ui.available_width(), 260.0, 16.0, 2);
                        grid(ui, id.with(("proj", n)), self.projects.len().min(n), cols, 16.0, |ui, i, row_h| {
                            let (name, desc, stars_n) = &self.projects[i];
                            let out = card(
                                ui,
                                id.with(("pc", i)),
                                CardLook::of(&t).pad(18.0).min_h(row_h).clickable(true),
                                |ui| {
                                    ui.horizontal(|ui| {
                                        let (r, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
                                        app_tile(ui.painter(), r, name, i, &t, 0.0);
                                        ui.vertical(|ui| {
                                            ui.spacing_mut().item_spacing.y = 1.0;
                                            Tx::new(name).size(15.5).semi().color(t.foreground).show_inline(ui);
                                            Tx::new("Public").size(12.0).color(t.muted_foreground).show_inline(ui);
                                        });
                                    });
                                    ui.add_space(8.0);
                                    Tx::new(desc).size(14.0).line(1.5).color(t.muted_foreground).show(ui);
                                    ui.add_space(10.0);
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 14.0;
                                        let lang = ["Rust", "Rust", "WGSL", "Rust"][i % 4];
                                        let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                                        ui.painter().circle_filled(r.center(), 5.0, t.chart[(i + 3) % 6]);
                                        Tx::new(lang).size(13.0).color(t.muted_foreground).show_inline(ui);
                                        let (r, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                                        UiIcon::Star.paint(ui.painter(), r, t.muted_foreground);
                                        Tx::new(glowkit_text::format_number(*stars_n as f64, 0, ",", "."))
                                            .size(13.0)
                                            .color(t.muted_foreground)
                                            .show_inline(ui);
                                    });
                                },
                            );
                            if out.response.clicked() {
                                *clicked = Some(name.clone());
                            }
                        });
                    };
                    match tab {
                        0 => {
                            let mut c2 = None;
                            two_col(
                                ui,
                                id.with("ov"),
                                0.64,
                                28.0,
                                sec.narrow,
                                false,
                                |ui| {
                                    Tx::new("Pinned").size(16.0).semi().color(t.foreground).show(ui);
                                    ui.add_space(8.0);
                                    projects(ui, &mut clicked, 4);
                                    ui.add_space(24.0);
                                    Tx::new("Contributions").size(16.0).semi().color(t.foreground).show(ui);
                                    ui.add_space(8.0);
                                    contributions(ui, &t);
                                },
                                |ui| {
                                    card(ui, id.with("skills"), CardLook::of(&t).pad(18.0).hover(false), |ui| {
                                        Tx::new("Skills").size(15.0).semi().color(t.foreground).show(ui);
                                        ui.add_space(8.0);
                                        ui.horizontal_wrapped(|ui| {
                                            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                                            for s in &self.skills {
                                                ui.add(Chip::new(s));
                                            }
                                        });
                                    });
                                    ui.add_space(12.0);
                                    card(ui, id.with("ach"), CardLook::of(&t).pad(18.0).hover(false), |ui| {
                                        Tx::new("Achievements").size(15.0).semi().color(t.foreground).show(ui);
                                        ui.add_space(8.0);
                                        for (k, (icon, s)) in [
                                            (UiIcon::Star, "Top 1% contributor"),
                                            (UiIcon::Zap, "100-day streak"),
                                            (UiIcon::Heart, "Community favorite"),
                                        ]
                                        .iter()
                                        .enumerate()
                                        {
                                            ui.horizontal(|ui| {
                                                let (r, _) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::hover());
                                                icon_tile(ui.painter(), r, *icon, t.chart[(k + 1) % 6], &t);
                                                Tx::new(*s).size(14.0).color(t.foreground).show_inline(ui);
                                            });
                                            ui.add_space(4.0);
                                        }
                                    });
                                    ui.add_space(12.0);
                                    card(ui, id.with("soc"), CardLook::of(&t).pad(18.0).hover(false), |ui| {
                                        Tx::new("Elsewhere").size(15.0).semi().color(t.foreground).show(ui);
                                        ui.add_space(8.0);
                                        ui.horizontal(|ui| {
                                            for s in [
                                                SectionSocial::GitHub,
                                                SectionSocial::X,
                                                SectionSocial::LinkedIn,
                                                SectionSocial::YouTube,
                                            ] {
                                                if social_button(ui, s, 34.0).clicked() {
                                                    c2 = Some(s.label().to_string());
                                                }
                                            }
                                        });
                                    });
                                },
                            );
                            if c2.is_some() {
                                clicked = c2;
                            }
                        }
                        1 => projects(ui, &mut clicked, 99),
                        _ => {
                            let items = [
                                (UiIcon::Star, "Starred", "glowkit/templates", "2h ago"),
                                (UiIcon::Code, "Pushed 3 commits to", "aurora-rs", "5h ago"),
                                (UiIcon::Sparkles, "Released", "spring-lab v0.4.0", "Yesterday"),
                                (UiIcon::Users, "Started following", "Liam Ortiz", "2 days ago"),
                                (UiIcon::File, "Opened an issue in", "egui/egui", "4 days ago"),
                            ];
                            for (k, (icon, verb, obj, when)) in items.iter().enumerate() {
                                sec.fade(ui, 2 + k, |ui| {
                                    ui.horizontal(|ui| {
                                        let (r, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
                                        icon_tile(ui.painter(), r, *icon, t.chart[k % 6], &t);
                                        Tx::new(*verb).size(14.5).color(t.muted_foreground).show_inline(ui);
                                        Tx::new(*obj).size(14.5).semi().color(t.foreground).show_inline(ui);
                                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                            Tx::new(*when).size(13.0).color(t.muted_foreground).show_inline(ui);
                                        });
                                    });
                                });
                                ui.add_space(4.0);
                                divider(ui, None);
                                ui.add_space(4.0);
                            }
                        }
                    }
                    if let Some(c) = clicked {
                        sec.click(c);
                    }
                },
            );
            p.take(r);
            p.take(FooterSection::new("Glowkit").layout(FooterLayout::Simple).id_salt("pf").show(ui));
        })
    }
}

/// A contribution graph (weeks x days) with deterministic pseudo-random intensity.
fn contributions(ui: &mut Ui, t: &Theme) {
    let w = ui.available_width();
    let weeks = ((w - 4.0) / 13.0).floor().clamp(10.0, 52.0) as usize;
    let cell = 10.0;
    let gap = 3.0;
    let (r, _) = ui.allocate_exact_size(vec2(w, 7.0 * (cell + gap) + 22.0), Sense::hover());
    let p = ui.painter();
    let hover = ui.input(|i| i.pointer.hover_pos());
    let mut tip = None;
    for wk in 0..weeks {
        for d in 0..7 {
            let h = glowkit_core::rng::hash01((wk * 7 + d) as u64 * 2654435761);
            let trend = wk as f32 / weeks as f32;
            let v = ((h * 1.2 + trend * 0.5 - 0.35) * if d == 0 || d == 6 { 0.5 } else { 1.0 }).clamp(0.0, 1.0);
            let level = (v * 4.0).round() / 4.0;
            let c = Rect::from_min_size(
                pos2(r.min.x + wk as f32 * (cell + gap), r.min.y + d as f32 * (cell + gap)),
                Vec2::splat(cell),
            );
            let col = if level <= 0.0 {
                with_alpha(t.foreground, if t.dark { 0.06 } else { 0.07 })
            } else {
                with_alpha(t.success, 0.25 + 0.75 * level)
            };
            p.rect_filled(c, CornerRadius::same(2), col);
            if hover.is_some_and(|h| c.contains(h)) {
                p.rect_stroke(c, CornerRadius::same(2), Stroke::new(1.0, t.foreground), StrokeKind::Outside);
                tip = Some(format!("{} contributions", (level * 12.0) as u32));
            }
        }
    }
    let ly = r.max.y - 8.0;
    p.text(
        pos2(r.min.x, ly),
        egui::Align2::LEFT_CENTER,
        "1,284 contributions in the last year",
        FontId::proportional(12.0),
        t.muted_foreground,
    );
    let mut x = r.max.x - 5.0 * (cell + gap);
    p.text(pos2(x - 6.0, ly), egui::Align2::RIGHT_CENTER, "Less", FontId::proportional(11.5), t.muted_foreground);
    for k in 0..5 {
        let col =
            if k == 0 { with_alpha(t.foreground, 0.07) } else { with_alpha(t.success, 0.25 + 0.75 * k as f32 / 4.0) };
        p.rect_filled(Rect::from_min_size(pos2(x, ly - cell / 2.0), Vec2::splat(cell)), CornerRadius::same(2), col);
        x += cell + gap;
    }
    if let (Some(s), Some(h)) = (tip, hover) {
        let g = p.layout_no_wrap(s, FontId::proportional(12.0), t.foreground);
        let br = Rect::from_min_size(h + vec2(12.0, -30.0), g.size() + vec2(16.0, 10.0));
        p.rect(br, CornerRadius::same(6), t.popover, Stroke::new(1.0, t.border), StrokeKind::Inside);
        p.galley(br.min + vec2(8.0, 5.0), g, t.foreground);
    }
}
