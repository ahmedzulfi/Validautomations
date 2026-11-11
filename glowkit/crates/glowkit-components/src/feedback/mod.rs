//! Loaders & progress: spinners, progress bars and rings, skeletons and steppers.

mod progress;
mod skeleton;
mod spinner;

pub use progress::{CircularProgress, Progress, ProgressRing, StepProgress};
pub use skeleton::Skeleton;
pub use spinner::{LoadingDots, Spinner, SpinnerKind};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
