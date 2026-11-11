//! Inputs & forms: buttons, toggles, text fields, switches, checkboxes, sliders, selects,
//! pickers and form helpers.

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

pub use button::{Button, ButtonGroup, ButtonVariant};
pub use color_picker::{ColorPicker, DEFAULT_SWATCHES};
pub use date_picker::{days_in_month, Calendar, CalendarDate, DatePicker};
pub use dropzone::FileDropzone;
pub use field::{FieldDescription, FieldError, FieldLabel};
pub use input::{Input, NumberInput, PasswordInput, SearchInput, Textarea};
pub use kit::{ControlSize, UiIcon};
pub use otp::OtpInput;
pub use select::{Combobox, Select, SelectOption};
pub use slider::{RangeSlider, Slider};
pub use switch::{Checkbox, RadioGroup, RadioOption, Switch};
pub use tag_input::{Rating, TagInput};
pub use toggle::{SegmentedControl, Toggle, ToggleGroup, ToggleItem};

pub(crate) use dropzone::dashed_closed;
pub(crate) use switch::paint_checkbox;

#[cfg(feature = "demos")]
pub(crate) mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
