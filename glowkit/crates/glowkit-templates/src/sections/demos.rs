//! Gallery demos for the sections.

use super::*;
use egui::Ui;
use glowkit_core::demo::{Category, Demo};

/// A centered segmented control to flip between a section's variants.
pub(crate) fn picker(ui: &mut Ui, labels: &[&str]) -> usize {
    let id = ui.scope_id().with("variant_picker");
    let mut sel: usize = glowkit_core::state(ui, id, || 0);
    ui.vertical_centered(|ui| {
        ui.add(glowkit_components::SegmentedControl::new(&mut sel, labels.iter().copied()).size(glowkit_components::ControlSize::Sm));
    });
    glowkit_core::set_state(ui, id, sel);
    ui.add_space(12.0);
    sel
}

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
        Demo {
            id: "features-section",
            name: "Features Section",
            category: Category::Blocks,
            description: "Feature grid in four layouts: icon cards, icon list, bento with animated mini-visuals, alternating rows and tabs.",
            covers: &["MVP Blocks: Features", "Tailark: Features", "shadcnblocks: Feature", "Magic UI: Bento Grid"],
            code: r##"FeaturesSection::new("Everything you need to ship")
    .eyebrow("Features")
    .subtitle("A complete toolkit, tuned for delight.")
    .items(FeatureItem::defaults())
    .layout(FeaturesLayout::Bento)
    .show(ui);"##,
            show: |ui| {
                let v = picker(ui, &["Icon grid", "Icon list", "Bento", "Alternating", "Tabbed"]);
                let layout = [FeaturesLayout::IconGrid, FeaturesLayout::IconList, FeaturesLayout::Bento, FeaturesLayout::Alternating, FeaturesLayout::Tabbed][v];
                let items = if v == 3 { FeatureItem::defaults().into_iter().take(3).collect() } else { FeatureItem::defaults() };
                FeaturesSection::new("Everything you need to ship")
                    .eyebrow("Features")
                    .subtitle("A complete toolkit for building interfaces people love, tuned for performance and delight.")
                    .items(items)
                    .layout(layout)
                    .id_salt(format!("{v}"))
                    .show(ui);
            },
        },
        Demo {
            id: "features-bento",
            name: "Bento Features",
            category: Category::Blocks,
            description: "Bento grid of mixed-size feature cards, each with a looping mini illustration (chart, orbit, code, bars, radar, toggles).",
            covers: &["MVP Blocks: Bento Grid", "Magic UI: Bento Grid", "Aceternity: Bento Grid"],
            code: r##"FeaturesSection::new("Built for modern teams").layout(FeaturesLayout::Bento).show(ui);"##,
            show: |ui| {
                FeaturesSection::new("Built for modern teams")
                    .eyebrow("Platform")
                    .subtitle("Everything is animated, themeable and responsive out of the box.")
                    .layout(FeaturesLayout::Bento)
                    .show(ui);
            },
        },
        Demo {
            id: "logo-cloud",
            name: "Logo Cloud",
            category: Category::Blocks,
            description: "Trust logos as a row, a bordered grid with hover, or two marquee rows moving in opposite directions.",
            covers: &["MVP Blocks: Logo Cloud", "Tailark: Logo Cloud", "Magic UI: Marquee logos"],
            code: r##"LogoCloudSection::new().layout(LogoCloudLayout::Marquee).show(ui);"##,
            show: |ui| {
                let v = picker(ui, &["Row", "Grid", "Marquee"]);
                LogoCloudSection::new().layout([LogoCloudLayout::Row, LogoCloudLayout::Grid, LogoCloudLayout::Marquee][v]).id_salt(format!("{v}")).show(ui);
            },
        },
        Demo {
            id: "stats-section",
            name: "Stats Section",
            category: Category::Blocks,
            description: "Key numbers that count up when scrolled into view: a divided row, icon cards, or a split heading + grid.",
            covers: &["MVP Blocks: Stats", "Tailark: Stats", "shadcnblocks: Stats"],
            code: r##"StatsSection::new()
    .title("Trusted at scale")
    .items(vec![StatItem::new(99.99, "Uptime").suffix("%").decimals(2)])
    .layout(StatsLayout::Cards)
    .show(ui);"##,
            show: |ui| {
                let v = picker(ui, &["Row", "Cards", "Split"]);
                StatsSection::new()
                    .title("Trusted at scale")
                    .subtitle("Numbers from teams who build with glowkit every day.")
                    .layout([StatsLayout::Row, StatsLayout::Cards, StatsLayout::Split][v])
                    .id_salt(format!("{v}"))
                    .show(ui);
            },
        },
        Demo {
            id: "pricing-section",
            name: "Pricing Section",
            category: Category::Blocks,
            description: "Pricing tiers with a monthly/yearly toggle (prices count to their new value), checklists and a beam-bordered Most popular plan.",
            covers: &["MVP Blocks: Pricing", "Tailark: Pricing", "shadcnblocks: Pricing", "Magic UI: Pricing"],
            code: r##"let r = PricingSection::new("Simple, transparent pricing")
    .tiers(PricingTier::defaults())
    .show(ui);
if r.clicked("Pro") { /* checkout */ }"##,
            show: |ui| {
                let v = picker(ui, &["Cards", "Rows"]);
                PricingSection::new("Simple, transparent pricing")
                    .eyebrow("Pricing")
                    .subtitle("Start free, upgrade when you grow. No hidden fees.")
                    .layout([PricingLayout::Cards, PricingLayout::Rows][v])
                    .id_salt(format!("{v}"))
                    .show(ui);
            },
        },
        Demo {
            id: "testimonials-section",
            name: "Testimonials",
            category: Category::Blocks,
            description: "Customer quotes as a card grid, two marquee rows, or one featured quote cycling with progress dots.",
            covers: &["MVP Blocks: Testimonials", "Tailark: Testimonials", "Magic UI: Marquee testimonials", "Aceternity: Animated Testimonials"],
            code: r##"TestimonialsSection::new("Loved by builders").layout(TestimonialsLayout::Marquee).show(ui);"##,
            show: |ui| {
                let v = picker(ui, &["Grid", "Marquee", "Featured"]);
                TestimonialsSection::new("Loved by builders everywhere")
                    .eyebrow("Testimonials")
                    .layout([TestimonialsLayout::Grid, TestimonialsLayout::Marquee, TestimonialsLayout::Featured][v])
                    .id_salt(format!("{v}"))
                    .show(ui);
            },
        },
        Demo {
            id: "cta-section",
            name: "CTA Banner",
            category: Category::Blocks,
            description: "Closing call-to-action: gradient panel, beam-bordered dark panel, split card or minimal centered copy.",
            covers: &["MVP Blocks: CTA", "Tailark: Call to Action", "shadcnblocks: CTA"],
            code: r##"CtaSection::new("Ready to build something beautiful?")
    .primary_cta("Get started free")
    .layout(CtaLayout::Beam)
    .show(ui);"##,
            show: |ui| {
                let v = picker(ui, &["Gradient", "Beam", "Split", "Minimal"]);
                CtaSection::new("Ready to build something beautiful?")
                    .note("No credit card required")
                    .layout([CtaLayout::Gradient, CtaLayout::Beam, CtaLayout::Split, CtaLayout::Minimal][v])
                    .id_salt(format!("{v}"))
                    .show(ui);
            },
        },
        Demo {
            id: "faq-section",
            name: "FAQ Section",
            category: Category::Blocks,
            description: "Frequently asked questions as a centered accordion, a two-column layout or an open grid.",
            covers: &["MVP Blocks: FAQ", "Tailark: FAQs", "shadcnblocks: FAQ"],
            code: r##"FaqSection::new("Frequently asked questions")
    .item("Is it free?", "Yes, MIT licensed.")
    .layout(FaqLayout::TwoColumn)
    .show(ui);"##,
            show: |ui| {
                let v = picker(ui, &["Centered", "Two column", "Grid"]);
                FaqSection::new("Frequently asked questions")
                    .eyebrow("FAQ")
                    .subtitle("Everything you need to know about the product and billing.")
                    .layout([FaqLayout::Centered, FaqLayout::TwoColumn, FaqLayout::Grid][v])
                    .id_salt(format!("{v}"))
                    .show(ui);
            },
        },
        Demo {
            id: "team-section",
            name: "Team Section",
            category: Category::Blocks,
            description: "The people behind the product: avatar cards with bios and socials, portrait tiles, or compact rows.",
            covers: &["MVP Blocks: Team", "Tailark: Team", "shadcnblocks: Team"],
            code: r##"TeamSection::new("Meet the team").members(TeamMember::defaults()).layout(TeamLayout::Cards).show(ui);"##,
            show: |ui| {
                let v = picker(ui, &["Cards", "Portraits", "Compact"]);
                TeamSection::new("Meet the team")
                    .eyebrow("Team")
                    .subtitle("A small, senior team obsessed with craft.")
                    .layout([TeamLayout::Cards, TeamLayout::Portraits, TeamLayout::Compact][v])
                    .id_salt(format!("{v}"))
                    .show(ui);
            },
        },
        Demo {
            id: "newsletter-section",
            name: "Newsletter / Waitlist",
            category: Category::Blocks,
            description: "Email capture with validation and an animated success state: inline, split card, or a waitlist with a live counter.",
            covers: &["MVP Blocks: Newsletter", "MVP Blocks: Waitlist", "Tailark: Newsletter"],
            code: r##"let r = NewsletterSection::new("Join the waitlist").layout(NewsletterLayout::Waitlist).show(ui);
if let Some(email) = r.submitted { /* save */ }"##,
            show: |ui| {
                let v = picker(ui, &["Inline", "Card", "Waitlist"]);
                let title = ["Subscribe to our newsletter", "Stay in the loop", "Be the first to try glowkit Cloud"][v];
                NewsletterSection::new(title).layout([NewsletterLayout::Inline, NewsletterLayout::Card, NewsletterLayout::Waitlist][v]).id_salt(format!("{v}")).show(ui);
            },
        },
        Demo {
            id: "contact-section",
            name: "Contact Section",
            category: Category::Blocks,
            description: "Contact form (name, email, topic, message) with validation states and a success screen, plus contact details.",
            covers: &["MVP Blocks: Contact", "Tailark: Contact", "shadcnblocks: Contact"],
            code: r##"let r = ContactSection::new("Get in touch").email("hello@acme.dev").show(ui);
if let Some(msg) = r.submitted { /* send */ }"##,
            show: |ui| {
                let v = picker(ui, &["Split", "Stacked"]);
                ContactSection::new("Get in touch").layout([ContactLayout::Split, ContactLayout::Stacked][v]).id_salt(format!("{v}")).show(ui);
            },
        },
        Demo {
            id: "header-section",
            name: "Header / Navbar",
            category: Category::Blocks,
            description: "Site header with brand, links with a gliding hover highlight, sign-in and CTA; bar, floating pill or left-aligned; hamburger menu when narrow.",
            covers: &["MVP Blocks: Header", "Tailark: Header", "Aceternity: Floating Navbar", "Magic UI: Navbar"],
            code: r##"let r = HeaderSection::new("Glowkit")
    .links(&["Product", "Pricing", "Docs"])
    .cta("Get started")
    .layout(HeaderLayout::Floating)
    .show(ui);"##,
            show: |ui| {
                for (k, layout) in [HeaderLayout::Bar, HeaderLayout::Floating, HeaderLayout::Left].into_iter().enumerate() {
                    HeaderSection::new("Glowkit").layout(layout).link_badge(1, "New").background(SectionBackground::Dots).id_salt(format!("{k}")).show(ui);
                    ui.add_space(24.0);
                }
                ui.label(egui::RichText::new("Narrow (mobile) header, menu open:").size(12.0));
                ui.allocate_ui(egui::vec2(380.0, 320.0), |ui| {
                    HeaderSection::new("Glowkit").layout(HeaderLayout::Floating).menu_open(true).id_salt("narrow").show(ui);
                });
            },
        },
        Demo {
            id: "footer-section",
            name: "Footer Section",
            category: Category::Blocks,
            description: "Site footer: brand, socials and link columns with a status pill and legal bar; newsletter variant with a giant faded wordmark; simple centered variant.",
            covers: &["MVP Blocks: Footer", "Tailark: Footer", "shadcnblocks: Footer"],
            code: r##"FooterSection::new("Glowkit").column("Product", &["Features", "Pricing"]).layout(FooterLayout::Newsletter).show(ui);"##,
            show: |ui| {
                let v = picker(ui, &["Columns", "Newsletter", "Simple"]);
                FooterSection::new("Glowkit").layout([FooterLayout::Columns, FooterLayout::Newsletter, FooterLayout::Simple][v]).id_salt(format!("{v}")).show(ui);
            },
        },
        Demo {
            id: "announcement-bar",
            name: "Announcement Bar",
            category: Category::Blocks,
            description: "Thin dismissible bar across the top of a page with icon, message and link; gradient, accent or subtle shimmering style.",
            covers: &["MVP Blocks: Banner", "Tailark: Announcement", "shadcnblocks: Banner"],
            code: r##"AnnouncementBar::new("glowkit 1.0 is out").link("Read the announcement").show(ui);"##,
            show: |ui| {
                AnnouncementBar::new("glowkit 1.0 is out: 300+ components and templates").link("Read the announcement").id_salt("a").show(ui);
                ui.add_space(16.0);
                AnnouncementBar::new("Join us at RustConf, booth 42").link("Get tickets").style(AnnouncementStyle::Accent).icon(glowkit_components::UiIcon::Calendar).id_salt("b").show(ui);
                ui.add_space(16.0);
                AnnouncementBar::new("We're hiring designers and Rust engineers").link("See open roles").style(AnnouncementStyle::Subtle).icon(glowkit_components::UiIcon::Users).id_salt("c").show(ui);
                ui.add_space(16.0);
                if ui.button("Reset dismissed bars").clicked() {
                    for s in ["a", "b", "c"] {
                        AnnouncementBar::reset(ui, Some(s));
                    }
                }
            },
        },
        Demo {
            id: "about-section",
            name: "About / Story",
            category: Category::Blocks,
            description: "Company story with a timeline that draws itself in, or story paragraphs beside an illustration with a milestone strip.",
            covers: &["MVP Blocks: About", "Tailark: About", "shadcnblocks: Timeline"],
            code: r##"AboutSection::new("From side project to 1.0").milestone(Milestone::new("2024", "Founded", "Two engineers, one idea.")).show(ui);"##,
            show: |ui| {
                let v = picker(ui, &["Timeline", "Story"]);
                AboutSection::new("From side project to 1.0").layout([AboutLayout::Timeline, AboutLayout::Story][v]).id_salt(format!("{v}")).show(ui);
            },
        },
        Demo {
            id: "integrations-section",
            name: "Integrations",
            category: Category::Blocks,
            description: "App tiles orbiting your brand (hover to slow down and draw a beam), or a grid of integration cards.",
            covers: &["MVP Blocks: Integrations", "Tailark: Integrations", "Magic UI: Orbiting Circles"],
            code: r##"IntegrationsSection::new("Works with your stack").layout(IntegrationsLayout::Orbit).show(ui);"##,
            show: |ui| {
                let v = picker(ui, &["Orbit", "Grid"]);
                IntegrationsSection::new("Works with your whole stack").layout([IntegrationsLayout::Orbit, IntegrationsLayout::Grid][v]).id_salt(format!("{v}")).show(ui);
            },
        },
        Demo {
            id: "changelog-section",
            name: "Changelog",
            category: Category::Blocks,
            description: "Release notes on a dated timeline with version badges, tags, change lists and a screenshot for the latest release.",
            covers: &["MVP Blocks: Changelog", "shadcnblocks: Changelog"],
            code: r##"ChangelogSection::new("Changelog").entries(ChangelogEntry::defaults()).show(ui);"##,
            show: |ui| {
                ChangelogSection::new("Changelog").show(ui);
            },
        },
        Demo {
            id: "blog-section",
            name: "Blog Grid",
            category: Category::Blocks,
            description: "Latest posts as cover cards, a featured post with a side list, or thumbnail rows; covers zoom on hover.",
            covers: &["MVP Blocks: Blog", "Tailark: Blog", "shadcnblocks: Blog"],
            code: r##"BlogSection::new("From the blog").posts(BlogPost::defaults()).layout(BlogLayout::Featured).show(ui);"##,
            show: |ui| {
                let v = picker(ui, &["Grid", "Featured", "List"]);
                BlogSection::new("From the blog").subtitle("Guides, deep dives and product news.").layout([BlogLayout::Grid, BlogLayout::Featured, BlogLayout::List][v]).id_salt(format!("{v}")).show(ui);
            },
        },
        Demo {
            id: "gallery-section",
            name: "Gallery Grid",
            category: Category::Blocks,
            description: "Image tiles in a uniform grid or masonry columns with hover zoom, caption overlay and staggered entrance.",
            covers: &["MVP Blocks: Gallery", "shadcnblocks: Gallery", "Aceternity: Layout Grid"],
            code: r##"GallerySection::new("Made with glowkit").layout(GalleryLayout::Masonry).show(ui);"##,
            show: |ui| {
                let v = picker(ui, &["Grid", "Masonry"]);
                GallerySection::new("Made with glowkit").layout([GalleryLayout::Grid, GalleryLayout::Masonry][v]).id_salt(format!("{v}")).show(ui);
            },
        },
        Demo {
            id: "comparison-section",
            name: "Comparison Table",
            category: Category::Blocks,
            description: "Us-vs-them table with check, cross, partial and text cells; the first column is highlighted with a border beam.",
            covers: &["MVP Blocks: Comparison", "shadcnblocks: Compare"],
            code: r##"use CompareValue::*;
ComparisonSection::new("Why glowkit?")
    .columns(&["glowkit", "Others"])
    .row("Native + web", vec![Yes, No])
    .row("License", vec![Text("MIT".into()), Text("Mixed".into())])
    .show(ui);"##,
            show: |ui| {
                ComparisonSection::new("Why teams switch to glowkit").show(ui);
            },
        },
        Demo {
            id: "steps-section",
            name: "Steps / How It Works",
            category: Category::Blocks,
            description: "Numbered steps joined by a line that fills in: horizontal, vertical with cards, or cards with large faded numbers.",
            covers: &["MVP Blocks: How it works", "Tailark: Steps", "shadcnblocks: Process"],
            code: r##"StepsSection::new("How it works").step("Install", "cargo add glowkit").step("Build", "Pick a template").show(ui);"##,
            show: |ui| {
                let v = picker(ui, &["Horizontal", "Vertical", "Cards"]);
                StepsSection::new("Up and running in minutes").layout([StepsLayout::Horizontal, StepsLayout::Vertical, StepsLayout::Cards][v]).id_salt(format!("{v}")).show(ui);
            },
        },
    ]
}
