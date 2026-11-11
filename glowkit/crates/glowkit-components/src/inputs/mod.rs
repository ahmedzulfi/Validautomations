//! Inputs & forms: buttons, toggles, text fields, switches, checkboxes, sliders, selects,
//! pickers and form helpers.

pub(crate) mod kit;

mod button;
mod field;
mod input;
mod toggle;

pub use button::{Button, ButtonGroup, ButtonVariant};
pub use field::{FieldDescription, FieldError, FieldLabel};
pub use input::{Input, NumberInput, PasswordInput, SearchInput, Textarea};
pub use kit::{ControlSize, UiIcon};
pub use toggle::{SegmentedControl, Toggle, ToggleGroup, ToggleItem};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
