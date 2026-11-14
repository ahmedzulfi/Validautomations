//! Loaders & progress: spinners, progress bars, rings and skeletons.

mod progress;
mod skeleton;
mod spinner;

pub use progress::{CircularProgress, Progress, ProgressRing};
pub use skeleton::Skeleton;
pub use spinner::{Spinner, SpinnerKind};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
