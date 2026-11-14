//! Inputs & forms: buttons, text fields, switches, checkboxes, sliders, selects, segmented
//! controls, pickers and form labels.

pub(crate) mod kit;

mod button;
mod color_picker;
mod date_picker;
mod dropzone;
mod field;
mod input;
mod otp;
mod select;
mod slider;
mod switch;
mod tag_input;
mod toggle;

pub use button::{Button, ButtonVariant};
pub use color_picker::{ColorPicker, DEFAULT_SWATCHES};
pub use date_picker::{days_in_month, Calendar, CalendarDate, DatePicker};
pub use dropzone::FileDropzone;
pub use field::FieldLabel;
pub use input::{Input, PasswordInput};
pub use kit::{ControlSize, UiIcon};
pub use otp::OtpInput;
pub use select::{Combobox, Select, SelectOption};
pub use slider::{RangeSlider, Slider};
pub use switch::{Checkbox, Switch};
pub use tag_input::{Rating, TagInput};
pub use toggle::{SegmentedControl, ToggleItem};

pub(crate) use dropzone::dashed_closed;
pub(crate) use switch::paint_checkbox;

#[cfg(feature = "demos")]
pub(crate) mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
