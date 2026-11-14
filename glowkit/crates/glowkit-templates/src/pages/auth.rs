//! Authentication pages: login, signup, forgot password and OTP verification.

use super::frame::{page, page_setters, PageCtx, PageOpts};
use crate::sections::*;
use egui::{pos2, vec2, Align, Color32, CornerRadius, CursorIcon, FontId, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use glowkit_components::{Avatar, AvatarStyle, Button, ButtonVariant, Checkbox, ControlSize, Input, OtpInput, UiIcon};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// Arrangement shared by the auth pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AuthLayout {
    /// A card centered on a patterned background.
    #[default]
    Centered,
    /// Form on the left, a gradient brand panel with a testimonial on the right.
    Split,
}

/// Lay out the auth shell (background, centered card or split panel) around `form`.
fn auth_shell(ui: &mut Ui, p: &mut PageCtx, id: egui::Id, brand_name: &str, layout: AuthLayout, bg: SectionBackground, form: impl FnOnce(&mut Ui, &mut Option<String>, &mut Option<String>)) {
    let t = p.t.clone();
    let mut clicked = None;
    let mut submitted = None;
    let h = p.height;
    let split = layout == AuthLayout::Split;
    let r = section(ui, Frame::new(id, if split { SectionBackground::Plain } else { bg }).pad(if split { 0.0 } else { 40.0 }).min_h(h).max_w(if split { 100_000.0 } else { 440.0 }), |ui, sec| {
        let narrow = sec.w < 820.0;
        let content_h: f32 = glowkit_core::state(ui, id.with("ch"), || 0.0);
        let top = ui.cursor().min.y;
        let body = |ui: &mut Ui, clicked: &mut Option<String>, submitted: &mut Option<String>| {
            if brand(ui, brand_name, 15.0).clicked() {
                *clicked = Some(brand_name.to_string());
            }
            ui.add_space(28.0);
            form(ui, clicked, submitted);
        };
        if split && !narrow {
            let w = ui.available_width();
            let lw = (w * 0.5).round();
            let full = Rect::from_min_size(pos2(ui.cursor().min.x, top), vec2(w, h));
            // right brand panel
            let panel = Rect::from_min_max(pos2(full.min.x + lw, full.min.y), full.max).shrink(12.0);
            brand_panel(ui, panel, &t);
            let col_w = (lw - 96.0).min(380.0);
            let x = full.min.x + (lw - col_w) / 2.0;
            let y = top + ((h - content_h) / 2.0).max(32.0);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos2(x, y), vec2(col_w, f32::INFINITY))).layout(egui::Layout::top_down(Align::Min)));
            sec.fade(&mut child, 0, |ui| body(ui, &mut clicked, &mut submitted));
            let ch = child.min_rect().height();
            if (ch - content_h).abs() > 0.5 {
                glowkit_core::set_state(ui, id.with("ch"), ch);
                ui.ctx().request_repaint();
            }
            ui.advance_cursor_after_rect(full);
        } else {
            let avail = h - if split { 0.0 } else { 80.0 };
            ui.add_space(((avail - content_h) / 2.0).max(if split { 24.0 } else { 0.0 }));
            let start = ui.cursor().min.y;
            sec.fade(ui, 0, |ui| {
                if split {
                    centered_column(ui, 380.0, |ui| body(ui, &mut clicked, &mut submitted));
                } else {
                    card(ui, id.with("card"), CardLook::of(&t).pad(32.0).hover(false), |ui| body(ui, &mut clicked, &mut submitted));
                }
            });
            let ch = ui.cursor().min.y - start;
            if (ch - content_h).abs() > 0.5 {
                glowkit_core::set_state(ui, id.with("ch"), ch);
                ui.ctx().request_repaint();
            }
        }
    });
    p.take(r);
    if clicked.is_some() {
        p.clicked = clicked;
    }
    if submitted.is_some() {
        p.submitted = submitted;
    }
}

/// The gradient brand panel of the split auth layout.
fn brand_panel(ui: &Ui, r: Rect, t: &Theme) {
    let painter = ui.painter();
    let rad = (t.radius * 2.0).round();
    paint::gradient_rect(painter, r, rad, 135.0, &[mix(t.gradient[0], Color32::BLACK, 0.25), t.gradient[1], t.gradient[2]]);
    let clip = painter.with_clip_rect(r.intersect(ui.clip_rect()));
    let time = glowkit_core::time(ui);
    paint::glow(&clip, r.left_top() + vec2(r.width() * (0.3 + 0.1 * (time * 0.3).sin()), r.height() * 0.25), r.width() * 0.7, with_alpha(Color32::WHITE, 0.16));
    paint::glow(&clip, r.right_bottom(), r.width() * 0.8, with_alpha(Color32::BLACK, 0.3));
    let step = 36.0;
    let mut x = r.min.x + step;
    while x < r.max.x {
        clip.vline(x, r.y_range(), Stroke::new(1.0, with_alpha(Color32::WHITE, 0.05)));
        x += step;
    }
    let mut y = r.min.y + step;
    while y < r.max.y {
        clip.hline(r.x_range(), y, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.05)));
        y += step;
    }
    // glass quote card
    let q = Rect::from_min_max(pos2(r.min.x + 40.0, r.max.y - 220.0), pos2(r.max.x - 40.0, r.max.y - 40.0));
    clip.rect(q, CornerRadius::same(16), with_alpha(Color32::WHITE, 0.1), Stroke::new(1.0, with_alpha(Color32::WHITE, 0.2)), StrokeKind::Inside);
    let quote = "\u{201C}We rebuilt our whole desktop app in six weeks. Our users keep asking which framework we switched to.\u{201D}";
    let g = clip.layout(quote.to_owned(), FontId::proportional(18.0), Color32::WHITE, q.width() - 48.0);
    clip.galley(q.min + vec2(24.0, 24.0), g.clone(), Color32::WHITE);
    let ay = q.min.y + 24.0 + g.size().y + 20.0;
    Avatar::new("Ava Chen").size(36.0).style(AvatarStyle::Solid).color(with_alpha(Color32::WHITE, 0.25)).paint(ui, Rect::from_min_size(pos2(q.min.x + 24.0, ay), Vec2::splat(36.0)));
    clip.text(pos2(q.min.x + 70.0, ay + 1.0), egui::Align2::LEFT_TOP, "Ava Chen", FontId::proportional(14.0), Color32::WHITE);
    clip.text(pos2(q.min.x + 70.0, ay + 19.0), egui::Align2::LEFT_TOP, "CTO, Northwind", FontId::proportional(12.5), with_alpha(Color32::WHITE, 0.75));
    // headline on the panel
    let hg = clip.layout("Build apps people love.".to_owned(), FontId::proportional(30.0), Color32::WHITE, r.width() - 80.0);
    clip.galley(r.min + vec2(40.0, 48.0), hg.clone(), Color32::WHITE);
    clip.galley(r.min + vec2(40.8, 48.0), hg, Color32::WHITE);
    glowkit_core::animate(ui);
}

/// Title + subtitle block of an auth form.
fn auth_title(ui: &mut Ui, title: &str, subtitle: &str, t: &Theme) {
    Tx::new(title).size(26.0).bold().color(t.foreground).show(ui);
    ui.add_space(2.0);
    Tx::new(subtitle).size(14.5).line(1.5).color(t.muted_foreground).show(ui);
}

/// Outline "Continue with X" button with a painted glyph.
fn oauth_button(ui: &mut Ui, label: &str, google: bool, w: f32) -> egui::Response {
    let t = Theme::of(ui);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 40.0), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    let hv = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let painter = ui.painter();
    painter.rect(rect, CornerRadius::same((t.radius * 0.8) as u8), if t.dark { mix(with_alpha(t.input, 0.3), t.secondary, hv) } else { mix(t.background, t.secondary, hv) }, Stroke::new(1.0, t.input), StrokeKind::Inside);
    let g = painter.layout_no_wrap(label.to_owned(), FontId::proportional(14.0), t.foreground);
    let total = 18.0 + 10.0 + g.size().x;
    let x = rect.center().x - total / 2.0;
    let ir = Rect::from_min_size(pos2(x, rect.center().y - 9.0), Vec2::splat(18.0));
    if google {
        // four-color ring "G"
        let c = ir.center();
        let cols = [glowkit_core::hex("#EA4335"), glowkit_core::hex("#FBBC05"), glowkit_core::hex("#34A853"), glowkit_core::hex("#4285F4")];
        for (k, col) in cols.iter().enumerate() {
            let a0 = -2.4 + k as f32 * 1.35;
            paint::arc(painter, c, 6.5, a0, a0 + 1.35, 2.6, *col);
        }
        painter.line_segment([c, c + vec2(7.5, 0.0)], Stroke::new(2.6, cols[3]));
    } else {
        SectionSocial::GitHub.paint(painter, ir, t.foreground);
    }
    painter.galley(pos2(x + 28.0, rect.center().y - g.size().y / 2.0), g, t.foreground);
    resp
}

/// Two OAuth buttons side by side + "or" divider. Returns the clicked provider.
fn oauth_row(ui: &mut Ui, divider_label: &str) -> Option<String> {
    let w = (ui.available_width() - 10.0) / 2.0;
    let mut out = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        if oauth_button(ui, "GitHub", false, w).clicked() {
            out = Some("GitHub".to_string());
        }
        if oauth_button(ui, "Google", true, w).clicked() {
            out = Some("Google".to_string());
        }
    });
    ui.add_space(14.0);
    divider(ui, Some(divider_label));
    ui.add_space(14.0);
    out
}

/// Small centered "prompt + link" line.
fn footer_link(ui: &mut Ui, id: egui::Id, prompt: &str, label: &str, t: &Theme) -> bool {
    let mut hit = false;
    row(ui, id, Align::Center, 4.0, |ui| {
        Tx::new(prompt).size(13.5).color(t.muted_foreground).show_inline(ui);
        hit = link(ui, label, 13.5, t.accent, t.foreground).clicked();
    });
    hit
}

/// A label row with an optional link on the right ("Password ... Forgot password?").
fn label_row(ui: &mut Ui, label: &str, link_label: Option<&str>, t: &Theme) -> bool {
    let mut hit = false;
    ui.horizontal(|ui| {
        Tx::new(label).size(13.5).semi().color(t.foreground).show_inline(ui);
        if let Some(l) = link_label {
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                hit = link(ui, l, 13.0, t.accent, t.foreground).clicked();
            });
        }
    });
    hit
}

/// Big circular success badge with a check that pops in.
fn success_badge(ui: &mut Ui, since: f64, icon: UiIcon, color: Color32) {
    let k = glowkit_core::ease(glowkit_core::Easing::OutBack, (((ui.input(|i| i.time) - since) as f32) / 0.6).clamp(0.0, 1.0));
    if k < 1.0 {
        ui.ctx().request_repaint();
    }
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 72.0), Sense::hover());
    let c = pos2(r.min.x + 28.0, r.center().y);
    let p = ui.painter();
    p.circle_filled(c, 28.0 * k, with_alpha(color, 0.15));
    p.circle_filled(c, 20.0 * k, color);
    icon.paint(p, Rect::from_center_size(c, Vec2::splat(20.0 * k)), Color32::WHITE);
}

fn now(ui: &Ui) -> f64 {
    ui.input(|i| i.time)
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct LoginState {
    email: String,
    password: String,
    remember: bool,
    tried: bool,
    pending: Option<f64>,
    done: Option<f64>,
}

/// A sign-in page: OAuth buttons, email + password with validation, remember me, forgot
/// password link, a loading state on submit and a success state.
///
/// Clicks (providers, links, "Sign in") are reported by label; the signed-in email arrives in
/// [`SectionResponse::submitted`] once the (simulated) request finishes.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{LoginPage, AuthLayout};
/// let r = LoginPage::new("Acme").layout(AuthLayout::Split).show(ui);
/// if r.clicked("Forgot password?") { /* route */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct LoginPage {
    brand: String,
    title: String,
    subtitle: String,
    layout: AuthLayout,
    background: SectionBackground,
    oauth: bool,
    opts: PageOpts,
    salt: Option<String>,
}

impl LoginPage {
    /// A login page for `brand`.
    pub fn new(brand: impl Into<String>) -> Self {
        Self { brand: brand.into(), title: "Welcome back".into(), subtitle: "Sign in to your account to continue.".into(), layout: AuthLayout::Centered, background: SectionBackground::Grid, oauth: true, opts: PageOpts::default(), salt: None }
    }
    /// Form title.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    /// Line under the title.
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = s.into();
        self
    }
    /// Layout variant.
    pub fn layout(mut self, l: AuthLayout) -> Self {
        self.layout = l;
        self
    }
    /// Background of the centered layout.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    /// Show the GitHub / Google buttons (default on).
    pub fn oauth(mut self, on: bool) -> Self {
        self.oauth = on;
        self
    }
    page_setters!();

    /// Show the page.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "login_page", &self.salt);
        page(ui, id, self.opts, |ui, p| {
            let t = p.t.clone();
            auth_shell(ui, p, id.with("shell"), &self.brand, self.layout, self.background, |ui, clicked, submitted| {
                let mut st: LoginState = glowkit_core::state(ui, id.with("st"), LoginState::default);
                if let Some(at) = st.pending {
                    ui.ctx().request_repaint();
                    if now(ui) - at > 1.2 {
                        st.pending = None;
                        st.done = Some(now(ui));
                        *submitted = Some(st.email.trim().to_string());
                    }
                }
                if let Some(at) = st.done {
                    success_badge(ui, at, UiIcon::Check, t.success);
                    auth_title(ui, "You're signed in", &format!("Welcome back, {}. Redirecting to your dashboard...", st.email.trim()), &t);
                    ui.add_space(20.0);
                    if ui.add(Button::new("Sign out").variant(ButtonVariant::Outline).width(f32::INFINITY)).clicked() {
                        st = LoginState::default();
                        *clicked = Some("Sign out".into());
                    }
                    glowkit_core::set_state(ui, id.with("st"), st);
                    return;
                }
                auth_title(ui, &self.title, &self.subtitle, &t);
                ui.add_space(24.0);
                if self.oauth {
                    if let Some(c) = oauth_row(ui, "or continue with email") {
                        *clicked = Some(c);
                    }
                }
                let email_err = (st.tried && !valid_email(&st.email)).then(|| "Enter a valid email address.".to_string());
                let pass_err = (st.tried && st.password.len() < 8).then(|| "Password must be at least 8 characters.".to_string());
                label_row(ui, "Email", None, &t);
                ui.add(Input::new(&mut st.email).placeholder("you@company.com").icon(UiIcon::Mail).width(f32::INFINITY).error_opt(email_err).id_salt(id.with("email")));
                ui.add_space(8.0);
                if label_row(ui, "Password", Some("Forgot password?"), &t) {
                    *clicked = Some("Forgot password?".into());
                }
                let pr = ui.add(Input::new(&mut st.password).placeholder("••••••••").icon(UiIcon::Lock).password(true).width(f32::INFINITY).error_opt(pass_err).id_salt(id.with("pass")));
                ui.add_space(8.0);
                ui.add(Checkbox::new(&mut st.remember).label("Remember me for 30 days"));
                ui.add_space(16.0);
                let enter = pr.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let b = ui.add(Button::new(if st.pending.is_some() { "Signing in..." } else { "Sign in" }).size(ControlSize::Lg).width(f32::INFINITY).fill(t.accent).loading(st.pending.is_some()));
                if (b.clicked() || enter) && st.pending.is_none() {
                    st.tried = true;
                    *clicked = Some("Sign in".into());
                    if valid_email(&st.email) && st.password.len() >= 8 {
                        st.pending = Some(now(ui));
                    }
                }
                ui.add_space(20.0);
                if footer_link(ui, id.with("fl"), "Don't have an account?", "Sign up", &t) {
                    *clicked = Some("Sign up".into());
                }
                glowkit_core::set_state(ui, id.with("st"), st);
            });
        })
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct SignupState {
    name: String,
    email: String,
    password: String,
    terms: bool,
    tried: bool,
    done: Option<f64>,
}

/// A sign-up page: OAuth buttons, divider, name / email / password (with strength meter) and
/// terms checkbox, inline validation errors and a "check your inbox" success state.
///
/// The new account's email arrives in [`SectionResponse::submitted`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::SignupPage;
/// if let Some(email) = SignupPage::new("Acme").show(ui).submitted { /* create account */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct SignupPage {
    brand: String,
    title: String,
    subtitle: String,
    layout: AuthLayout,
    background: SectionBackground,
    opts: PageOpts,
    salt: Option<String>,
}

impl SignupPage {
    /// A sign-up page for `brand`.
    pub fn new(brand: impl Into<String>) -> Self {
        Self { brand: brand.into(), title: "Create your account".into(), subtitle: "Start your 14-day free trial. No credit card required.".into(), layout: AuthLayout::Split, background: SectionBackground::Dots, opts: PageOpts::default(), salt: None }
    }
    /// Form title.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    /// Line under the title.
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = s.into();
        self
    }
    /// Layout variant (default split).
    pub fn layout(mut self, l: AuthLayout) -> Self {
        self.layout = l;
        self
    }
    /// Background of the centered layout.
    pub fn background(mut self, b: SectionBackground) -> Self {
        self.background = b;
        self
    }
    page_setters!();

    /// Show the page.
    pub fn show(self, ui: &mut Ui) -> SectionResponse {
        let id = section_id(ui, "signup_page", &self.salt);
        page(ui, id, self.opts, |ui, p| {
            let t = p.t.clone();
            auth_shell(ui, p, id.with("shell"), &self.brand, self.layout, self.background, |ui, clicked, submitted| {
                let mut st: SignupState = glowkit_core::state(ui, id.with("st"), SignupState::default);
                if let Some(at) = st.done {
                    success_badge(ui, at, UiIcon::Mail, t.accent);
                    auth_title(ui, "Check your inbox", &format!("We sent a confirmation link to {}. Click it to activate your account.", st.email.trim()), &t);
                    ui.add_space(20.0);
                    if ui.add(Button::new("Use a different email").variant(ButtonVariant::Outline).width(f32::INFINITY)).clicked() {
                        st.done = None;
                        *clicked = Some("Use a different email".into());
                    }
                    glowkit_core::set_state(ui, id.with("st"), st);
                    return;
                }
                auth_title(ui, &self.title, &self.subtitle, &t);
                ui.add_space(24.0);
                if let Some(c) = oauth_row(ui, "or sign up with email") {
                    *clicked = Some(c);
                }
                let name_err = (st.tried && st.name.trim().len() < 2).then(|| "Please enter your full name.".to_string());
                let email_err = (st.tried && !valid_email(&st.email)).then(|| "Enter a valid email address.".to_string());
                let pass_err = (st.tried && st.password.len() < 8).then(|| "Use 8 or more characters.".to_string());
                ui.add(Input::new(&mut st.name).label("Full name").placeholder("Jane Doe").icon(UiIcon::User).width(f32::INFINITY).error_opt(name_err).id_salt(id.with("n")));
                ui.add_space(6.0);
                ui.add(Input::new(&mut st.email).label("Work email").placeholder("jane@company.com").icon(UiIcon::Mail).width(f32::INFINITY).error_opt(email_err).id_salt(id.with("e")));
                ui.add_space(6.0);
                ui.add(Input::new(&mut st.password).label("Password").placeholder("8+ characters").icon(UiIcon::Lock).password(true).strength_meter(true).width(f32::INFINITY).error_opt(pass_err).id_salt(id.with("p")));
                ui.add_space(10.0);
                let terms_err = st.tried && !st.terms;
                ui.horizontal(|ui| {
                    ui.add(Checkbox::new(&mut st.terms).label("I agree to the Terms and Privacy Policy"));
                });
                if terms_err {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(13.0), Sense::hover());
                        UiIcon::AlertCircle.paint(ui.painter(), r, t.destructive);
                        Tx::new("You need to accept the terms to continue.").size(12.5).color(t.destructive).show_inline(ui);
                    });
                }
                ui.add_space(16.0);
                if ui.add(Button::new("Create account").size(ControlSize::Lg).width(f32::INFINITY).fill(t.accent).trailing_icon(UiIcon::ArrowRight)).clicked() {
                    st.tried = true;
                    *clicked = Some("Create account".into());
                    if st.name.trim().len() >= 2 && valid_email(&st.email) && st.password.len() >= 8 && st.terms {
                        st.done = Some(now(ui));
                        *submitted = Some(st.email.trim().to_string());
                    }
                }
                ui.add_space(20.0);
                if footer_link(ui, id.with("fl"), "Already have an account?", "Sign in", &t) {
                    *clicked = Some("Sign in".into());
                }
                glowkit_core::set_state(ui, id.with("st"), st);
            });
        })
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct ForgotState {
    email: String,
    tried: bool,
    sent: Option<f64>,
}

/// A forgot-password page: email field, "send reset link", then a "check your email" state with
/// a resend countdown. The email arrives in [`SectionResponse::submitted`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::ForgotPasswordPage;
/// let r = ForgotPasswordPage::new("Acme").show(ui);
/// if r.clicked("Back to sign in") { /* route */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct ForgotPasswordPage {
    brand: String,
    resend_after: f32,
    background: SectionBackground,
    opts: PageOpts,
    salt: Option<String>,
}

impl ForgotPasswordPage {
    /// A forgot-password page.
    pub fn new(brand: impl Into<String>) -> Self {
        Self { brand: brand.into(), resend_after: 30.0, background: SectionBackground::Dots, opts: PageOpts::default(), salt: None }
    }
    /// Seconds before "resend" becomes available (default 30).
    pub fn resend_after(mut self, s: f32) -> Self {
        self.resend_after = s;
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
        let id = section_id(ui, "forgot_page", &self.salt);
        page(ui, id, self.opts, |ui, p| {
            let t = p.t.clone();
            auth_shell(ui, p, id.with("shell"), &self.brand, AuthLayout::Centered, self.background, |ui, clicked, submitted| {
                let mut st: ForgotState = glowkit_core::state(ui, id.with("st"), ForgotState::default);
                let (tile, _) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
                if let Some(at) = st.sent {
                    icon_tile(ui.painter(), tile, UiIcon::Mail, t.accent, &t);
                    ui.add_space(16.0);
                    auth_title(ui, "Check your email", &format!("We sent a password reset link to {}.", st.email.trim()), &t);
                    ui.add_space(24.0);
                    if ui.add(Button::new("Open email app").size(ControlSize::Lg).width(f32::INFINITY).fill(t.accent)).clicked() {
                        *clicked = Some("Open email app".into());
                    }
                    ui.add_space(16.0);
                    let left = self.resend_after - (now(ui) - at) as f32;
                    if left > 0.0 {
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                        Tx::new(format!("Didn't get it? You can resend in {}s", left.ceil() as i32)).size(13.5).align(Align::Center).color(t.muted_foreground).show(ui);
                    } else if footer_link(ui, id.with("rs"), "Didn't get it?", "Resend email", &t) {
                        st.sent = Some(now(ui));
                        *clicked = Some("Resend email".into());
                    }
                } else {
                    icon_tile(ui.painter(), tile, UiIcon::Lock, t.accent, &t);
                    ui.add_space(16.0);
                    auth_title(ui, "Forgot your password?", "No worries. Enter your email and we'll send you a reset link.", &t);
                    ui.add_space(24.0);
                    let err = (st.tried && !valid_email(&st.email)).then(|| "Enter the email you signed up with.".to_string());
                    let r = ui.add(Input::new(&mut st.email).label("Email").placeholder("you@company.com").icon(UiIcon::Mail).width(f32::INFINITY).error_opt(err).id_salt(id.with("e")));
                    ui.add_space(16.0);
                    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.add(Button::new("Send reset link").size(ControlSize::Lg).width(f32::INFINITY).fill(t.accent)).clicked() || enter {
                        st.tried = true;
                        *clicked = Some("Send reset link".into());
                        if valid_email(&st.email) {
                            st.sent = Some(now(ui));
                            *submitted = Some(st.email.trim().to_string());
                        }
                    }
                }
                ui.add_space(20.0);
                let mut back = false;
                row(ui, id.with("back"), Align::Center, 6.0, |ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                    UiIcon::ArrowLeft.paint(ui.painter(), r, t.muted_foreground);
                    back = link(ui, "Back to sign in", 13.5, t.muted_foreground, t.foreground).clicked();
                });
                if back {
                    *clicked = Some("Back to sign in".into());
                }
                glowkit_core::set_state(ui, id.with("st"), st);
            });
        })
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct OtpState {
    code: String,
    error_at: Option<f64>,
    ok_at: Option<f64>,
    sent_at: Option<f64>,
}

/// One-time-code verification: six digit boxes, auto-verify when complete, a shake on a wrong
/// code, a success state and a resend countdown. The accepted code is in
/// [`SectionResponse::submitted`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::OtpPage;
/// OtpPage::new("Acme").email("jane@acme.dev").expected("424242").show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct OtpPage {
    brand: String,
    email: String,
    length: usize,
    expected: Option<String>,
    background: SectionBackground,
    opts: PageOpts,
    salt: Option<String>,
}

impl OtpPage {
    /// A verification page.
    pub fn new(brand: impl Into<String>) -> Self {
        Self { brand: brand.into(), email: "jane@company.com".into(), length: 6, expected: None, background: SectionBackground::Grid, opts: PageOpts::default(), salt: None }
    }
    /// Address the code was sent to (shown masked).
    pub fn email(mut self, e: impl Into<String>) -> Self {
        self.email = e.into();
        self
    }
    /// Number of digits (default 6).
    pub fn length(mut self, n: usize) -> Self {
        self.length = n.clamp(4, 8);
        self
    }
    /// The correct code; any other complete code shows the error state. Without it every
    /// complete code is accepted (validate it yourself from `submitted`).
    pub fn expected(mut self, code: impl Into<String>) -> Self {
        self.expected = Some(code.into());
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
        let id = section_id(ui, "otp_page", &self.salt);
        page(ui, id, self.opts, |ui, p| {
            let t = p.t.clone();
            auth_shell(ui, p, id.with("shell"), &self.brand, AuthLayout::Centered, self.background, |ui, clicked, submitted| {
                let mut st: OtpState = glowkit_core::state(ui, id.with("st"), || OtpState { sent_at: Some(now(ui)), ..Default::default() });
                if let Some(at) = st.ok_at {
                    success_badge(ui, at, UiIcon::Check, t.success);
                    auth_title(ui, "Email verified", "Thanks! Your account is ready to go.", &t);
                    ui.add_space(20.0);
                    if ui.add(Button::new("Continue to dashboard").size(ControlSize::Lg).width(f32::INFINITY).fill(t.accent).trailing_icon(UiIcon::ArrowRight)).clicked() {
                        *clicked = Some("Continue to dashboard".into());
                    }
                    glowkit_core::set_state(ui, id.with("st"), st);
                    return;
                }
                let (tile, _) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
                icon_tile(ui.painter(), tile, UiIcon::Mail, t.accent, &t);
                ui.add_space(16.0);
                let masked = match self.email.split_once('@') {
                    Some((a, b)) => format!("{}***@{}", a.chars().next().unwrap_or('*'), b),
                    None => self.email.clone(),
                };
                auth_title(ui, "Verify your email", &format!("Enter the {}-digit code we sent to {}.", self.length, masked), &t);
                ui.add_space(24.0);
                // shake on error
                let shake = st.error_at.map_or(0.0, |at| {
                    let e = (now(ui) - at) as f32;
                    if e < 0.45 {
                        ui.ctx().request_repaint();
                        (e * 50.0).sin() * 8.0 * (1.0 - e / 0.45)
                    } else {
                        0.0
                    }
                });
                let before = st.code.clone();
                let err = st.error_at.is_some();
                let w = ui.available_width();
                let bw = ((w - 8.0 * (self.length as f32 - 1.0) - 28.0) / self.length as f32).min(52.0);
                let total = bw * self.length as f32 + 8.0 * (self.length as f32 - 1.0) + 28.0;
                ui.horizontal(|ui| {
                    ui.add_space(((w - total) / 2.0 + shake).max(0.0));
                    ui.add(OtpInput::new(&mut st.code).length(self.length).separator_after(self.length / 2).digits_only(true).error(err).box_size(vec2(bw, bw * 1.15)));
                });
                if st.code != before {
                    st.error_at = None;
                }
                if err {
                    ui.add_space(4.0);
                    Tx::new("That code didn't work. Check it and try again.").size(13.0).align(Align::Center).color(t.destructive).show(ui);
                }
                ui.add_space(20.0);
                let complete = st.code.chars().count() == self.length;
                let verify = ui.add(Button::new("Verify").size(ControlSize::Lg).width(f32::INFINITY).fill(t.accent).disabled(!complete)).clicked();
                if verify || (complete && before.chars().count() + 1 == self.length && st.code != before) {
                    *clicked = Some("Verify".into());
                    if self.expected.as_ref().is_none_or(|e| *e == st.code) {
                        st.ok_at = Some(now(ui));
                        *submitted = Some(st.code.clone());
                    } else {
                        st.error_at = Some(now(ui));
                    }
                }
                ui.add_space(16.0);
                let left = 30.0 - st.sent_at.map_or(99.0, |at| (now(ui) - at) as f32);
                if left > 0.0 {
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                    Tx::new(format!("Resend code in 0:{:02}", left.ceil() as i32)).size(13.5).align(Align::Center).color(t.muted_foreground).show(ui);
                } else if footer_link(ui, id.with("rs"), "Didn't receive a code?", "Resend", &t) {
                    st.sent_at = Some(now(ui));
                    st.code.clear();
                    *clicked = Some("Resend".into());
                }
                glowkit_core::set_state(ui, id.with("st"), st);
            });
        })
    }
}
