//! Marketing pages assembled from sections: landing, pricing, waitlist and contact.

use super::frame::{page, page_setters, PageOpts};
use crate::sections::*;
use glowkit_components::UiIcon;

/// The blocks a [`LandingPage`] can include, in page order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LandingBlock {
    /// Announcement bar above the header.
    Announcement,
    /// Logo cloud under the hero.
    Logos,
    /// Bento feature grid.
    Features,
    /// "How it works" steps.
    Steps,
    /// Key numbers.
    Stats,
    /// Alternating feature rows.
    Showcase,
    /// Testimonials marquee.
    Testimonials,
    /// Pricing tiers.
    Pricing,
    /// FAQ accordion.
    Faq,
    /// Closing CTA banner.
    Cta,
}

impl LandingBlock {
    /// Every block, in page order.
    pub const ALL: [LandingBlock; 10] = [
        Self::Announcement,
        Self::Logos,
        Self::Features,
        Self::Steps,
        Self::Stats,
        Self::Showcase,
        Self::Testimonials,
        Self::Pricing,
        Self::Faq,
        Self::Cta,
    ];
}

/// A complete landing page: announcement bar, header, hero, logos, bento features, steps,
/// stats, alternating showcase, testimonials, pricing, FAQ, CTA and footer.
///
/// Every click on the page is reported by label in [`SectionResponse::clicked`], email
/// submissions in [`SectionResponse::submitted`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{LandingPage, LandingBlock};
/// let r = LandingPage::new("Acme").headline("Ship faster").without(LandingBlock::Pricing).show(ui);
/// if r.clicked("Get started") { /* ... */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct LandingPage {
    brand: String,
    headline: String,
    accent: String,
    subheadline: String,
    primary: String,
    secondary: String,
    hero_layout: HeroLayout,
    hero_background: SectionBackground,
    blocks: Vec<LandingBlock>,
    opts: PageOpts,
    salt: Option<String>,
}

impl LandingPage {
    /// A landing page for `brand` with placeholder copy.
    pub fn new(brand: impl Into<String>) -> Self {
        Self {
            brand: brand.into(),
            headline: "Build interfaces that glow".into(),
            accent: "without leaving Rust".into(),
            subheadline: "Animated, themeable components and templates for egui. Native on every desktop, and on the web through WebAssembly.".into(),
            primary: "Get started".into(),
            secondary: "View on GitHub".into(),
            hero_layout: HeroLayout::Centered,
            hero_background: SectionBackground::Aurora,
            blocks: LandingBlock::ALL.to_vec(),
            opts: PageOpts::default(),
            salt: None,
        }
    }
    /// Hero headline.
    pub fn headline(mut self, h: impl Into<String>) -> Self {
        self.headline = h.into();
        self
    }
    /// Gradient second line of the headline (`""` hides it).
    pub fn accent(mut self, a: impl Into<String>) -> Self {
        self.accent = a.into();
        self
    }
    /// Hero paragraph.
    pub fn subheadline(mut self, s: impl Into<String>) -> Self {
        self.subheadline = s.into();
        self
    }
    /// Primary CTA label (also used in the header and the closing banner).
    pub fn primary_cta(mut self, l: impl Into<String>) -> Self {
        self.primary = l.into();
        self
    }
    /// Secondary CTA label.
    pub fn secondary_cta(mut self, l: impl Into<String>) -> Self {
        self.secondary = l.into();
        self
    }
    /// Hero layout.
    pub fn hero_layout(mut self, l: HeroLayout) -> Self {
        self.hero_layout = l;
        self
    }
    /// Hero background.
    pub fn hero_background(mut self, b: SectionBackground) -> Self {
        self.hero_background = b;
        self
    }
    /// Exactly which blocks to include (header, hero and footer are always shown).
    pub fn blocks(mut self, b: &[LandingBlock]) -> Self {
        self.blocks = b.to_vec();
        self
    }
    /// Remove one block.
    pub fn without(mut self, b: LandingBlock) -> Self {
        self.blocks.retain(|x| *x != b);
        self
    }
    page_setters!();

    /// Show the page.
    pub fn show(self, ui: &mut egui::Ui) -> SectionResponse {
        let id = section_id(ui, "landing_page", &self.salt);
        page(ui, id, self.opts, |ui, p| {
            let has = |b: LandingBlock| self.blocks.contains(&b);
            if has(LandingBlock::Announcement) {
                p.take(
                    AnnouncementBar::new(format!("{} 1.0 is out: templates, shells and 300+ components", self.brand))
                        .link("Read more")
                        .id_salt("lp")
                        .show(ui),
                );
            }
            p.take(HeaderSection::new(&self.brand).cta(&self.primary).link_badge(1, "New").id_salt("lp").show(ui));
            let mut hero = HeroSection::new(&self.headline)
                .subheadline(&self.subheadline)
                .announcement("New", "Templates are here")
                .primary_cta(&self.primary)
                .secondary_cta(&self.secondary)
                .social_proof("Loved by 12,000+ developers")
                .layout(self.hero_layout)
                .background(self.hero_background)
                .id_salt("lp");
            if !self.accent.is_empty() {
                hero = hero.accent(&self.accent);
            }
            if self.hero_layout != HeroLayout::Centered {
                hero = hero.visual(HeroVisual::Dashboard);
            }
            p.take(hero.show(ui));
            if has(LandingBlock::Logos) {
                p.take(
                    LogoCloudSection::new()
                        .title("Trusted by teams at")
                        .layout(LogoCloudLayout::Marquee)
                        .id_salt("lp")
                        .show(ui),
                );
            }
            if has(LandingBlock::Features) {
                p.take(
                    FeaturesSection::new("Everything you need to ship")
                        .eyebrow("Features")
                        .subtitle("A complete toolkit tuned for performance and delight.")
                        .layout(FeaturesLayout::Bento)
                        .id_salt("lp")
                        .show(ui),
                );
            }
            if has(LandingBlock::Steps) {
                p.take(
                    StepsSection::new("Up and running in minutes")
                        .background(SectionBackground::Muted)
                        .id_salt("lp")
                        .show(ui),
                );
            }
            if has(LandingBlock::Stats) {
                p.take(StatsSection::new().id_salt("lp").show(ui));
            }
            if has(LandingBlock::Showcase) {
                p.take(
                    FeaturesSection::new("Designed for the details")
                        .eyebrow("Why us")
                        .items(FeatureItem::defaults().into_iter().take(2).collect())
                        .layout(FeaturesLayout::Alternating)
                        .id_salt("lp-alt")
                        .show(ui),
                );
            }
            if has(LandingBlock::Testimonials) {
                p.take(
                    TestimonialsSection::new("Loved by builders")
                        .eyebrow("Testimonials")
                        .layout(TestimonialsLayout::Marquee)
                        .background(SectionBackground::Muted)
                        .id_salt("lp")
                        .show(ui),
                );
            }
            if has(LandingBlock::Pricing) {
                p.take(
                    PricingSection::new("Simple, transparent pricing")
                        .eyebrow("Pricing")
                        .subtitle("Start free, upgrade when you grow.")
                        .id_salt("lp")
                        .show(ui),
                );
            }
            if has(LandingBlock::Faq) {
                p.take(
                    FaqSection::new("Frequently asked questions")
                        .layout(FaqLayout::TwoColumn)
                        .eyebrow("FAQ")
                        .id_salt("lp")
                        .show(ui),
                );
            }
            if has(LandingBlock::Cta) {
                p.take(
                    CtaSection::new("Ready to build something beautiful?")
                        .primary_cta(&self.primary)
                        .note("No credit card required")
                        .id_salt("lp")
                        .show(ui),
                );
            }
            p.take(FooterSection::new(&self.brand).layout(FooterLayout::Newsletter).id_salt("lp").show(ui));
        })
    }
}

// ---------------------------------------------------------------------------------------------

/// A pricing page: header, pricing tiers with billing toggle, logo cloud, comparison table,
/// FAQ, CTA and footer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::{PricingPage, PricingTier};
/// let r = PricingPage::new("Acme").tiers(PricingTier::defaults()).show(ui);
/// if r.clicked("Pro") { /* checkout */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct PricingPage {
    brand: String,
    title: String,
    subtitle: String,
    tiers: Vec<PricingTier>,
    opts: PageOpts,
    salt: Option<String>,
}

impl PricingPage {
    /// A pricing page with the default tiers.
    pub fn new(brand: impl Into<String>) -> Self {
        Self {
            brand: brand.into(),
            title: "Pricing that scales with you".into(),
            subtitle: "Start for free. Upgrade when you need more. Cancel anytime.".into(),
            tiers: PricingTier::defaults(),
            opts: PageOpts::default(),
            salt: None,
        }
    }
    /// Page title.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    /// Paragraph under the title.
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = s.into();
        self
    }
    /// The plans.
    pub fn tiers(mut self, t: Vec<PricingTier>) -> Self {
        self.tiers = t;
        self
    }
    page_setters!();

    /// Show the page.
    pub fn show(self, ui: &mut egui::Ui) -> SectionResponse {
        let id = section_id(ui, "pricing_page", &self.salt);
        page(ui, id, self.opts, |ui, p| {
            p.take(HeaderSection::new(&self.brand).id_salt("pp").show(ui));
            p.take(
                PricingSection::new(&self.title)
                    .eyebrow("Pricing")
                    .subtitle(&self.subtitle)
                    .tiers(self.tiers.clone())
                    .background(SectionBackground::Glow)
                    .id_salt("pp")
                    .show(ui),
            );
            p.take(LogoCloudSection::new().title("Trusted by 2,000+ teams").id_salt("pp").show(ui));
            p.take(ComparisonSection::new("Compare plans and alternatives").id_salt("pp").show(ui));
            p.take(
                FaqSection::new("Billing questions")
                    .item(
                        "Can I change plans later?",
                        "Yes. Upgrade or downgrade anytime; we prorate the difference automatically.",
                    )
                    .item(
                        "Do you offer discounts?",
                        "Yearly plans save 20%. Students, open-source maintainers and non-profits get Pro for free.",
                    )
                    .item(
                        "What payment methods do you accept?",
                        "All major cards, and invoices with bank transfer for yearly Enterprise plans.",
                    )
                    .item("Is there a free trial?", "Pro comes with a 14-day free trial, no credit card required.")
                    .layout(FaqLayout::Centered)
                    .background(SectionBackground::Muted)
                    .id_salt("pp")
                    .show(ui),
            );
            p.take(
                CtaSection::new("Still deciding?")
                    .subtitle("Talk to our team and get a tailored demo.")
                    .primary_cta("Book a demo")
                    .secondary_cta("Start free")
                    .layout(CtaLayout::Split)
                    .id_salt("pp")
                    .show(ui),
            );
            p.take(FooterSection::new(&self.brand).id_salt("pp").show(ui));
        })
    }
}

// ---------------------------------------------------------------------------------------------

/// A waitlist page: floating header, waitlist hero with live counter over meteors, three
/// teaser features and a simple footer. The submitted email is in
/// [`SectionResponse::submitted`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::WaitlistPage;
/// if let Some(email) = WaitlistPage::new("Acme").show(ui).submitted { /* save */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct WaitlistPage {
    brand: String,
    title: String,
    subtitle: String,
    count: u64,
    opts: PageOpts,
    salt: Option<String>,
}

impl WaitlistPage {
    /// A waitlist page with placeholder copy.
    pub fn new(brand: impl Into<String>) -> Self {
        Self {
            brand: brand.into(),
            title: "The AI-native design tool for Rust apps".into(),
            subtitle: "Describe a screen, get production egui code. Join the waitlist for early access.".into(),
            count: 2_847,
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
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = s.into();
        self
    }
    /// People already waiting.
    pub fn count(mut self, n: u64) -> Self {
        self.count = n;
        self
    }
    page_setters!();

    /// Show the page.
    pub fn show(self, ui: &mut egui::Ui) -> SectionResponse {
        let id = section_id(ui, "waitlist_page", &self.salt);
        page(ui, id, self.opts, |ui, p| {
            p.take(
                HeaderSection::new(&self.brand)
                    .links(&["Features", "Roadmap", "Blog"])
                    .sign_in("")
                    .cta("Join waitlist")
                    .layout(HeaderLayout::Floating)
                    .background(SectionBackground::Transparent)
                    .id_salt("wl")
                    .show(ui),
            );
            p.take(
                NewsletterSection::new(&self.title)
                    .subtitle(&self.subtitle)
                    .button("Join waitlist")
                    .success_message("You're on the list! We'll email you when it's your turn.")
                    .count(self.count)
                    .layout(NewsletterLayout::Waitlist)
                    .id_salt("wl")
                    .show(ui),
            );
            p.take(
                FeaturesSection::new("What you'll get first")
                    .items(vec![
                        FeatureItem::new(
                            UiIcon::Sparkles,
                            "Prompt to screen",
                            "Describe a layout in plain words and watch it render live.",
                        ),
                        FeatureItem::new(
                            UiIcon::Code,
                            "Clean Rust output",
                            "Idiomatic builder code you'd be happy to commit.",
                        ),
                        FeatureItem::new(
                            UiIcon::Users,
                            "Founding member perks",
                            "Lifetime discount and a direct line to the team.",
                        ),
                    ])
                    .layout(FeaturesLayout::IconList)
                    .id_salt("wl")
                    .show(ui),
            );
            p.take(FooterSection::new(&self.brand).layout(FooterLayout::Simple).id_salt("wl").show(ui));
        })
    }
}

// ---------------------------------------------------------------------------------------------

/// A contact page: header, contact form with details, two-column FAQ and footer.
/// The submitted message is in [`SectionResponse::submitted`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::ContactPage;
/// let r = ContactPage::new("Acme").email("hi@acme.dev").show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct ContactPage {
    brand: String,
    email: String,
    phone: String,
    address: String,
    opts: PageOpts,
    salt: Option<String>,
}

impl ContactPage {
    /// A contact page with placeholder details.
    pub fn new(brand: impl Into<String>) -> Self {
        Self {
            brand: brand.into(),
            email: "hello@glowkit.dev".into(),
            phone: "+1 (555) 010-2024".into(),
            address: "100 Market St, San Francisco".into(),
            opts: PageOpts::default(),
            salt: None,
        }
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
    page_setters!();

    /// Show the page.
    pub fn show(self, ui: &mut egui::Ui) -> SectionResponse {
        let id = section_id(ui, "contact_page", &self.salt);
        page(ui, id, self.opts, |ui, p| {
            p.take(HeaderSection::new(&self.brand).id_salt("cp").show(ui));
            p.take(
                ContactSection::new("Talk to our team")
                    .email(&self.email)
                    .phone(&self.phone)
                    .address(&self.address)
                    .background(SectionBackground::Glow)
                    .id_salt("cp")
                    .show(ui),
            );
            p.take(
                FaqSection::new("Before you write")
                    .layout(FaqLayout::TwoColumn)
                    .background(SectionBackground::Muted)
                    .id_salt("cp")
                    .show(ui),
            );
            p.take(FooterSection::new(&self.brand).id_salt("cp").show(ui));
        })
    }
}
