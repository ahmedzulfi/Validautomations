//! Full-page templates (landing, pricing, auth, error, onboarding, blog post, profile...),
//! composed from the [`crate::sections`]. Each page is a builder rendered with `.show(ui)`;
//! by default it fills the available height with its own scroll area.

mod app;
mod auth;
mod frame;
mod marketing;
mod status;

pub use app::{ArticleBlock, BlogPostPage, OnboardingPage, ProfilePage};
pub use auth::{AuthLayout, ForgotPasswordPage, LoginPage, OtpPage, SignupPage};
pub use marketing::{ContactPage, LandingBlock, LandingPage, PricingPage, WaitlistPage};
pub use status::{ComingSoonPage, NotFoundPage, ServerErrorPage, ThankYouPage};

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}

#[cfg(feature = "demos")]
mod demos;
