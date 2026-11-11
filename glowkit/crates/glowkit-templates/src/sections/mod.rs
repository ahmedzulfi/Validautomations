//! Marketing page sections in the MVP Blocks style: heroes, feature grids, pricing, FAQ,
//! footers and more. Every section is a builder with sensible defaults, rendered with
//! `.show(ui)` which returns a [`SectionResponse`].

mod kit;
mod hero;
mod chrome;
mod content;
mod features;
mod logos_stats;
mod people;
mod pricing;
mod social;

pub use chrome::{AnnouncementBar, AnnouncementStyle, FooterLayout, FooterSection, HeaderLayout, HeaderSection};
pub use content::{
    AboutLayout, AboutSection, BlogLayout, BlogPost, BlogSection, ChangelogEntry, ChangelogSection, CompareValue, ComparisonSection, GalleryLayout, GallerySection, Integration, IntegrationsLayout, IntegrationsSection, Milestone, StepsLayout, StepsSection,
};
pub use features::{FeatureItem, FeatureVisual, FeaturesLayout, FeaturesSection};
pub use logos_stats::{LogoCloudLayout, LogoCloudSection, StatItem, StatsLayout, StatsSection};
pub use people::{ContactLayout, ContactSection, NewsletterLayout, NewsletterSection, TeamLayout, TeamMember, TeamSection};
pub use pricing::{PricingLayout, PricingSection, PricingTier};
pub use social::{CtaLayout, CtaSection, FaqLayout, FaqSection, Testimonial, TestimonialsLayout, TestimonialsSection};
pub use hero::{HeroLayout, HeroSection, HeroVisual};

pub use kit::{SectionAlign, SectionBackground, SectionCtaStyle, SectionResponse, SectionSocial, SECTION_MAX_WIDTH, SECTION_STACK_BELOW};
#[allow(unused_imports)]
pub(crate) use kit::*;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}

#[cfg(feature = "demos")]
mod demos;
