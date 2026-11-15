//! Live playgrounds: typed, remembered parameters for gallery demos.
//!
//! A [`Playground`] renders one component with values read from [`Params`] and returns the exact
//! Rust code for those values. The first call to a control method (e.g. [`Params::f32`]) registers
//! the control and returns its default; afterwards it returns whatever the user picked in the
//! gallery's "Customize" panel.
//!
//! ```no_run
//! use glowkit_core::playground::{lit_f32, lit_str, Params};
//! fn shiny(ui: &mut egui::Ui, p: &mut Params) -> String {
//!     let text = p.text("Text", "Just some shiny text!");
//!     let size = p.f32("Size", 40.0, 16.0..=72.0, 1.0);
//!     // ui.add(ShinyText::new(&text).size(size));
//!     let _ = ui;
//!     format!("ui.add(ShinyText::new({}).size({}));", lit_str(&text), lit_f32(size))
//! }
//! ```

use egui::{Color32, Ui};
use std::collections::HashMap;
use std::ops::RangeInclusive;

/// A playground for one demo, keyed by the demo's id.
#[derive(Clone, Copy)]
pub struct Playground {
    /// The [`Demo::id`](crate::demo::Demo::id) this playground customizes.
    pub id: &'static str,
    /// Renders the component with the current values and returns the Rust code for them.
    pub show: fn(&mut Ui, &mut Params) -> String,
}

impl std::fmt::Debug for Playground {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Playground").field("id", &self.id).finish()
    }
}

/// The value of one control.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    F32(f32),
    Int(i64),
    Bool(bool),
    Color(Color32),
    Choice(usize),
    Text(String),
}

/// What kind of control a parameter uses (and its default).
#[derive(Clone, Debug, PartialEq)]
pub enum ControlKind {
    /// Slider over `min..=max`, snapped to `step` (0 = continuous).
    F32 { min: f32, max: f32, step: f32, default: f32 },
    /// Integer slider over `min..=max`.
    Int { min: i64, max: i64, default: i64 },
    /// On/off toggle.
    Bool { default: bool },
    /// Color swatch with a hex field.
    Color { default: Color32 },
    /// One of a few named options.
    Choice { options: Vec<&'static str>, default: usize },
    /// Free text.
    Text { default: String },
}

/// One registered control.
#[derive(Clone, Debug, PartialEq)]
pub struct Control {
    pub label: &'static str,
    pub kind: ControlKind,
}

impl Control {
    /// The control's default value.
    pub fn default_value(&self) -> Value {
        match &self.kind {
            ControlKind::F32 { default, .. } => Value::F32(*default),
            ControlKind::Int { default, .. } => Value::Int(*default),
            ControlKind::Bool { default } => Value::Bool(*default),
            ControlKind::Color { default } => Value::Color(*default),
            ControlKind::Choice { default, .. } => Value::Choice(*default),
            ControlKind::Text { default } => Value::Text(default.clone()),
        }
    }
}

/// Typed parameters for a playground. Remembers the user's values; values never edited fall
/// back to the defaults passed by the playground.
#[derive(Clone, Debug, Default)]
pub struct Params {
    controls: Vec<Control>,
    values: HashMap<&'static str, Value>,
}

impl Params {
    /// Empty parameter set.
    pub fn new() -> Self {
        Self::default()
    }

    fn register(&mut self, label: &'static str, kind: ControlKind) -> Value {
        let control = Control { label, kind };
        let default = control.default_value();
        match self.controls.iter_mut().find(|c| c.label == label) {
            Some(c) => *c = control,
            None => self.controls.push(control),
        }
        match self.values.get(label) {
            Some(v) if std::mem::discriminant(v) == std::mem::discriminant(&default) => v.clone(),
            _ => default,
        }
    }

    /// A number slider over `range`, snapped to `step` (0 = continuous).
    pub fn f32(&mut self, label: &'static str, default: f32, range: RangeInclusive<f32>, step: f32) -> f32 {
        let (min, max) = (*range.start(), *range.end());
        match self.register(label, ControlKind::F32 { min, max, step, default }) {
            Value::F32(v) => v.clamp(min, max),
            _ => default,
        }
    }

    /// An integer slider over `range`.
    pub fn int(&mut self, label: &'static str, default: i64, range: RangeInclusive<i64>) -> i64 {
        let (min, max) = (*range.start(), *range.end());
        match self.register(label, ControlKind::Int { min, max, default }) {
            Value::Int(v) => v.clamp(min, max),
            _ => default,
        }
    }

    /// An integer slider, returned as `usize` (for counts).
    pub fn count(&mut self, label: &'static str, default: usize, range: RangeInclusive<usize>) -> usize {
        self.int(label, default as i64, *range.start() as i64..=*range.end() as i64).max(0) as usize
    }

    /// An on/off toggle.
    pub fn bool(&mut self, label: &'static str, default: bool) -> bool {
        match self.register(label, ControlKind::Bool { default }) {
            Value::Bool(v) => v,
            _ => default,
        }
    }

    /// A color with a hex field.
    pub fn color(&mut self, label: &'static str, default: Color32) -> Color32 {
        match self.register(label, ControlKind::Color { default }) {
            Value::Color(v) => v,
            _ => default,
        }
    }

    /// One of `options`; returns the selected index.
    pub fn choice(&mut self, label: &'static str, options: &[&'static str], default: usize) -> usize {
        let n = options.len().max(1);
        match self.register(label, ControlKind::Choice { options: options.to_vec(), default: default.min(n - 1) }) {
            Value::Choice(v) => v.min(n - 1),
            _ => default.min(n - 1),
        }
    }

    /// Free text.
    pub fn text(&mut self, label: &'static str, default: &str) -> String {
        match self.register(label, ControlKind::Text { default: default.to_owned() }) {
            Value::Text(v) => v,
            _ => default.to_owned(),
        }
    }

    /// The controls registered so far, in call order.
    pub fn controls(&self) -> &[Control] {
        &self.controls
    }

    /// Current value of a control (its default if never edited).
    pub fn value(&self, label: &str) -> Option<Value> {
        let c = self.controls.iter().find(|c| c.label == label)?;
        Some(self.values.get(c.label).cloned().unwrap_or_else(|| c.default_value()))
    }

    /// Set a control's value (as the "Customize" panel does).
    pub fn set(&mut self, label: &'static str, value: Value) {
        self.values.insert(label, value);
    }

    /// True when any value differs from its default.
    pub fn is_modified(&self) -> bool {
        self.controls.iter().any(|c| self.values.get(c.label).is_some_and(|v| *v != c.default_value()))
    }

    /// Forget every edited value.
    pub fn reset(&mut self) {
        self.values.clear();
    }
}

/// A Rust `f32` literal that always has a decimal point (`40.0`, `2.5`, `0.35`).
pub fn lit_f32(v: f32) -> String {
    let r = (v * 1000.0).round() / 1000.0;
    let s = format!("{r}");
    if s.contains('.') || s.contains('e') || s.contains("inf") || s.contains("NaN") {
        s
    } else {
        format!("{s}.0")
    }
}

/// A Rust string literal with escapes.
pub fn lit_str(s: &str) -> String {
    format!("{s:?}")
}

/// `hex("#RRGGBB")` (or `#RRGGBBAA` for translucent colors).
pub fn lit_color(c: Color32) -> String {
    format!("hex(\"{}\")", hex_string(c))
}

/// `#RRGGBB`, or `#RRGGBBAA` when the color is not opaque.
pub fn hex_string(c: Color32) -> String {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    if a == 255 {
        format!("#{r:02X}{g:02X}{b:02X}")
    } else {
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
    }
}

/// Parse `#rgb`, `#rrggbb` or `#rrggbbaa` (the `#` is optional). `None` on bad input.
pub fn parse_hex(s: &str) -> Option<Color32> {
    let s = s.trim().trim_start_matches('#');
    if !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match s.len() {
        3 | 6 | 8 => Some(crate::color::hex(s)),
        _ => None,
    }
}

/// `&[a, b, c]` as a Rust slice literal of `hex(...)` calls.
pub fn lit_colors(cs: &[Color32]) -> String {
    let parts: Vec<String> = cs.iter().map(|c| lit_color(*c)).collect();
    format!("&[{}]", parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_remember_values_and_defaults() {
        let mut p = Params::new();
        assert_eq!(p.f32("Size", 40.0, 10.0..=80.0, 1.0), 40.0);
        p.set("Size", Value::F32(60.0));
        assert_eq!(p.f32("Size", 40.0, 10.0..=80.0, 1.0), 60.0);
        assert!(p.is_modified());
        assert_eq!(p.choice("Mode", &["A", "B"], 1), 1);
        p.set("Mode", Value::Choice(7));
        assert_eq!(p.choice("Mode", &["A", "B"], 1), 1);
        p.reset();
        assert_eq!(p.f32("Size", 40.0, 10.0..=80.0, 1.0), 40.0);
        assert_eq!(p.controls().len(), 2);
    }

    #[test]
    fn literals() {
        assert_eq!(lit_f32(40.0), "40.0");
        assert_eq!(lit_f32(2.5), "2.5");
        assert_eq!(lit_str("a\"b"), "\"a\\\"b\"");
        assert_eq!(lit_color(Color32::from_rgb(255, 0, 16)), "hex(\"#FF0010\")");
        assert_eq!(parse_hex("#ff0010"), Some(Color32::from_rgb(255, 0, 16)));
        assert_eq!(parse_hex("#ff001"), None);
        assert_eq!(parse_hex("zzz"), None);
    }
}
