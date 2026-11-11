//! Gallery demos for the sections.

use super::*;
use glowkit_core::demo::{Category, Demo};

pub(crate) fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "hero-section",
            name: "Hero Section",
            category: Category::Blocks,
            description: "Landing hero with announcement pill, gradient or rotating headline, dual CTA, social proof and trust logos over an animated background.",
            covers: &["MVP Blocks: Hero", "Tailark: Hero Section", "shadcnblocks: Hero"],
            code: r##"let r = HeroSection::new("Build interfaces that glow")
    .accent("without leaving Rust")
    .subheadline("Animated, themeable components for egui.")
    .announcement("New", "glowkit 1.0 is here")
    .primary_cta("Get started")
    .secondary_cta("View on GitHub")
    .logos(&["Acme", "Globex", "Initech", "Umbrella", "Hooli"])
    .background(SectionBackground::Aurora)
    .show(ui);
if r.clicked("Get started") { /* ... */ }"##,
            show: |ui| {
                HeroSection::new("Build interfaces that glow")
                    .accent("without leaving Rust")
                    .subheadline("Animated, themeable components for egui. Native on every desktop, and on the web through WebAssembly.")
                    .announcement("New", "glowkit 1.0 is here")
                    .primary_cta("Get started")
                    .secondary_cta("View on GitHub")
                    .social_proof("Loved by 12,000+ developers")
                    .logos(&["Acme", "Globex", "Initech", "Umbrella", "Hooli", "Vandelay"])
                    .background(SectionBackground::Aurora)
                    .show(ui);
            },
        },
        Demo {
            id: "hero-split",
            name: "Hero Split",
            category: Category::Blocks,
            description: "Split hero: copy with a rotating word pill on the left, an animated product mockup on the right.",
            covers: &["MVP Blocks: Hero Split", "Tailark: Hero with Image", "shadcnblocks: Hero with Screenshot"],
            code: r##"HeroSection::new("The analytics platform")
    .rotating("for", &["startups", "agencies", "enterprises"])
    .layout(HeroLayout::Split)
    .visual(HeroVisual::Dashboard)
    .background(SectionBackground::Grid)
    .show(ui);"##,
            show: |ui| {
                HeroSection::new("The analytics platform")
                    .rotating("for", &["startups", "agencies", "enterprises"])
                    .subheadline("Understand every user journey in real time. Ship experiments, watch the numbers move, repeat.")
                    .primary_cta("Start free trial")
                    .primary_style(SectionCtaStyle::Shimmer)
                    .secondary_cta("Book a demo")
                    .social_proof("4.9/5 from 2,300+ reviews")
                    .layout(HeroLayout::Split)
                    .visual(HeroVisual::Dashboard)
                    .background(SectionBackground::Grid)
                    .show(ui);
            },
        },
    ]
}
