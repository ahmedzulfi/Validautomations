//! Gallery demos for the page templates.

use super::*;
use crate::sections::*;
use glowkit_core::demo::{Category, Demo};

/// Viewport height for page previews (the page scrolls inside it).
const H: f32 = 700.0;

macro_rules! page_demo {
    ($id:literal, $name:literal, $desc:literal, [$($cov:literal),*], $code:literal, $show:expr) => {
        Demo { id: $id, name: $name, category: Category::Templates, description: $desc, covers: &[$($cov),*], code: $code, show: $show }
    };
}

pub(crate) fn all() -> Vec<Demo> {
    vec![
        page_demo!(
            "landing-page",
            "Landing Page",
            "A complete scrollable landing page: announcement bar, header, aurora hero, logo marquee, bento features, steps, stats, showcase, testimonials, pricing, FAQ, CTA and newsletter footer.",
            ["MVP Blocks: Landing Page", "Tailark: Landing", "shadcnblocks: Landing Page"],
            r##"let r = LandingPage::new("Glowkit")
    .headline("Build interfaces that glow")
    .accent("without leaving Rust")
    .without(LandingBlock::Announcement)
    .show(ui);
if r.clicked("Get started") { /* ... */ }"##,
            |ui| {
                LandingPage::new("Glowkit").height(H).show(ui);
            }
        ),
        page_demo!(
            "pricing-page",
            "Pricing Page",
            "Pricing page with billing toggle, beam-highlighted plan, logo cloud, comparison table, billing FAQ and CTA.",
            ["MVP Blocks: Pricing Page", "shadcnblocks: Pricing Page"],
            r##"let r = PricingPage::new("Glowkit").tiers(PricingTier::defaults()).show(ui);
if r.clicked("Pro") { /* checkout */ }"##,
            |ui| {
                PricingPage::new("Glowkit").height(H).show(ui);
            }
        ),
        page_demo!(
            "login-page",
            "Login Page",
            "Sign-in with GitHub/Google buttons, divider, email + password validation, remember me, forgot link, loading and success states; centered card or split with brand panel.",
            ["MVP Blocks: Login", "shadcn/ui: Login blocks", "Tailark: Login"],
            r##"let r = LoginPage::new("Glowkit").layout(AuthLayout::Split).show(ui);
if let Some(email) = r.submitted { /* signed in */ }"##,
            |ui| {
                let v = crate::sections::demos::picker(ui, &["Centered", "Split"]);
                LoginPage::new("Glowkit").layout([AuthLayout::Centered, AuthLayout::Split][v]).height(H).id_salt(format!("{v}")).show(ui);
            }
        ),
        page_demo!(
            "signup-page",
            "Signup Page",
            "Create-account page: social buttons, divider, name/email/password with strength meter, terms checkbox, inline validation errors and a check-your-inbox state.",
            ["MVP Blocks: Signup", "shadcn/ui: Signup blocks", "Tailark: Sign Up"],
            r##"if let Some(email) = SignupPage::new("Glowkit").show(ui).submitted { /* create */ }"##,
            |ui| {
                let v = crate::sections::demos::picker(ui, &["Split", "Centered"]);
                SignupPage::new("Glowkit").layout([AuthLayout::Split, AuthLayout::Centered][v]).height(H).id_salt(format!("{v}")).show(ui);
            }
        ),
        page_demo!(
            "forgot-password-page",
            "Forgot Password",
            "Reset-password request with validation, then a check-your-email state with a resend countdown and a back link.",
            ["MVP Blocks: Forgot Password", "shadcnblocks: Forgot Password"],
            r##"ForgotPasswordPage::new("Glowkit").resend_after(30.0).show(ui);"##,
            |ui| {
                ForgotPasswordPage::new("Glowkit").height(H).show(ui);
            }
        ),
        page_demo!(
            "otp-page",
            "OTP Verification",
            "Six-digit code entry that auto-verifies, shakes on a wrong code, shows success and offers a resend countdown. Try 424242.",
            ["MVP Blocks: OTP", "shadcn/ui: Input OTP block"],
            r##"OtpPage::new("Glowkit").email("jane@company.com").expected("424242").show(ui);"##,
            |ui| {
                OtpPage::new("Glowkit").expected("424242").height(H).show(ui);
            }
        ),
        page_demo!(
            "not-found-page",
            "404 Not Found",
            "A giant floating gradient 404 over drifting particles, message, docs search, two actions and quick-link cards.",
            ["MVP Blocks: 404", "shadcnblocks: 404", "Tailark: Not Found"],
            r##"let r = NotFoundPage::new().background(SectionBackground::Particles).show(ui);
if r.clicked("Go back home") { /* route */ }"##,
            |ui| {
                NotFoundPage::new().height(H).show(ui);
            }
        ),
        page_demo!(
            "server-error-page",
            "500 Server Error",
            "Glitching 500 with a pulsing error pill, retry button with loading state and a service-status card with uptime bars.",
            ["MVP Blocks: 500", "shadcnblocks: Error page"],
            r##"if ServerErrorPage::new().incident("INC-42").show(ui).clicked("Try again") { /* retry */ }"##,
            |ui| {
                ServerErrorPage::new().height(H).show(ui);
            }
        ),
        page_demo!(
            "coming-soon-page",
            "Coming Soon / Maintenance",
            "Launch countdown with sliding seconds, notify-me form and socials over aurora; maintenance mode swaps in a progress bar.",
            ["MVP Blocks: Coming Soon", "MVP Blocks: Maintenance", "shadcnblocks: Coming Soon"],
            r##"ComingSoonPage::new("Glowkit").launch_in(12.0 * 86400.0).show(ui);
ComingSoonPage::new("Glowkit").maintenance(true).progress(0.7).show(ui);"##,
            |ui| {
                let v = crate::sections::demos::picker(ui, &["Coming soon", "Maintenance"]);
                ComingSoonPage::new("Glowkit").maintenance(v == 1).background(if v == 1 { SectionBackground::Grid } else { SectionBackground::Aurora }).height(H).id_salt(format!("{v}")).show(ui);
            }
        ),
        page_demo!(
            "onboarding-page",
            "Onboarding Wizard",
            "Four-step onboarding: stepper, profile with role tiles, workspace with generated URL, theme preview tiles and switches, team invites, validation and a finish screen.",
            ["MVP Blocks: Onboarding", "shadcnblocks: Multi-step form"],
            r##"if let Some(done) = OnboardingPage::new("Glowkit").show(ui).submitted { /* create workspace */ }"##,
            |ui| {
                OnboardingPage::new("Glowkit").height(H).show(ui);
            }
        ),
        page_demo!(
            "waitlist-page",
            "Waitlist Page",
            "Pre-launch page: floating header, waitlist form with live counter over meteors, teaser features and a simple footer.",
            ["MVP Blocks: Waitlist", "Magic UI: Waitlist template"],
            r##"if let Some(email) = WaitlistPage::new("Glowkit").count(2847).show(ui).submitted { /* save */ }"##,
            |ui| {
                WaitlistPage::new("Glowkit").height(H).show(ui);
            }
        ),
        page_demo!(
            "contact-page",
            "Contact Page",
            "Contact page: header, validated contact form with details, FAQ and footer.",
            ["MVP Blocks: Contact Page", "shadcnblocks: Contact"],
            r##"if let Some(msg) = ContactPage::new("Glowkit").email("hi@glowkit.dev").show(ui).submitted { /* send */ }"##,
            |ui| {
                ContactPage::new("Glowkit").height(H).show(ui);
            }
        ),
        page_demo!(
            "blog-post-page",
            "Blog Post",
            "Article page: category, title, author meta and share buttons, cover, prose with headings, quote, code, list, figure and callout, author card, related posts and newsletter.",
            ["MVP Blocks: Blog Post", "shadcnblocks: Blog Post", "Tailwind Typography"],
            r##"BlogPostPage::new("Designing motion that feels native")
    .author("Mia Park", "Head of Design")
    .body(ArticleBlock::defaults())
    .show(ui);"##,
            |ui| {
                BlogPostPage::new("Designing motion that feels native").height(H).show(ui);
            }
        ),
        page_demo!(
            "profile-page",
            "Profile Page",
            "User profile: cover art, overlapping avatar with status, follow toggle, counting stats, tabs with pinned projects, skills, achievements, contribution graph and activity feed.",
            ["MVP Blocks: Profile", "shadcnblocks: Profile"],
            r##"let r = ProfilePage::new("Ava Chen", "@ava").bio("Design engineer.").show(ui);
if r.clicked("Follow") { /* ... */ }"##,
            |ui| {
                ProfilePage::new("Ava Chen", "@avachen").height(H).show(ui);
            }
        ),
        page_demo!(
            "thank-you-page",
            "Thank You Page",
            "Success page: self-drawing check with a confetti burst, order summary, next steps and two actions.",
            ["MVP Blocks: Thank You", "shadcnblocks: Order confirmation"],
            r##"ThankYouPage::new("Thanks for your order!").item("Pro plan", "$228.00").total("$228.00").show(ui);"##,
            |ui| {
                ThankYouPage::new("Thanks for your order!").height(H).show(ui);
            }
        ),
    ]
}
