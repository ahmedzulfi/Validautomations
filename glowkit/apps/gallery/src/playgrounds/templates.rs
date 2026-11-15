//! Template playgrounds.

use super::{variant, Code, Params, Playground};
use glowkit::core::playground::lit_str;
use glowkit::egui::Ui;
use glowkit::templates::*;

pub fn all() -> Vec<Playground> {
    vec![Playground { id: "login-page", show: login }, Playground { id: "landing-page", show: landing }]
}

const BGS: [&str; 8] = ["Plain", "Muted", "Glow", "Aurora", "Grid", "Dots", "Particles", "Gradient"];

fn bg(i: usize) -> SectionBackground {
    [
        SectionBackground::Plain,
        SectionBackground::Muted,
        SectionBackground::Glow,
        SectionBackground::Aurora,
        SectionBackground::Grid,
        SectionBackground::Dots,
        SectionBackground::Particles,
        SectionBackground::Gradient,
    ][i]
}

fn login(ui: &mut Ui, p: &mut Params) -> String {
    let brand = p.text("Brand", "Glowkit");
    let title = p.text("Title", "Welcome back");
    let subtitle = p.text("Subtitle", "Sign in to your account to continue");
    let layout = p.choice("Layout", &["Centered", "Split"], 1);
    let b = p.choice("Background", &BGS, 2);
    let oauth = p.bool("OAuth buttons", true);
    LoginPage::new(&brand)
        .title(&title)
        .subtitle(&subtitle)
        .layout([AuthLayout::Centered, AuthLayout::Split][layout])
        .background(bg(b))
        .oauth(oauth)
        .height(640.0)
        .id_salt("pg-login")
        .show(ui);
    Code::new(
        "glowkit::templates::{AuthLayout, LoginPage, SectionBackground}",
        format!("LoginPage::new({})", lit_str(&brand)),
    )
    .s("title", &title)
    .s("subtitle", &subtitle)
    .call("layout", variant("AuthLayout", &["Centered", "Split"], layout))
    .call("background", variant("SectionBackground", &BGS, b))
    .b("oauth", oauth)
    .stmt("let r = ", ".show(ui);\nif let Some(email) = r.submitted { /* signed in */ }")
}

fn landing(ui: &mut Ui, p: &mut Params) -> String {
    let brand = p.text("Brand", "Glowkit");
    let headline = p.text("Headline", "Build interfaces that glow");
    let accent = p.text("Accent", "without leaving Rust");
    let cta = p.text("Primary CTA", "Get started");
    let layout = p.choice("Hero layout", &["Centered", "Split", "SplitReverse"], 0);
    let b = p.choice("Hero background", &BGS, 3);
    LandingPage::new(&brand)
        .headline(&headline)
        .accent(&accent)
        .primary_cta(&cta)
        .hero_layout([HeroLayout::Centered, HeroLayout::Split, HeroLayout::SplitReverse][layout])
        .hero_background(bg(b))
        .height(640.0)
        .id_salt("pg-landing")
        .show(ui);
    Code::new(
        "glowkit::templates::{HeroLayout, LandingPage, SectionBackground}",
        format!("LandingPage::new({})", lit_str(&brand)),
    )
    .s("headline", &headline)
    .s("accent", &accent)
    .s("primary_cta", &cta)
    .call("hero_layout", variant("HeroLayout", &["Centered", "Split", "SplitReverse"], layout))
    .call("hero_background", variant("SectionBackground", &BGS, b))
    .stmt("let r = ", &format!(".show(ui);\nif r.clicked({}) {{ /* ... */ }}", lit_str(&cta)))
}
