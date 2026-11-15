//! Live playgrounds for flagship components: typed parameters that drive the real builder
//! setters, plus the exact Rust code for the current values.
//!
//! Each submodule returns `Vec<Playground>` keyed by demo id. A playground reads its values from
//! [`Params`], renders the component, and returns the code (built with [`Code`]).

use glowkit::core::playground::{lit_color, lit_f32, lit_str};
use glowkit::egui::{self, Color32, Ui};

pub use glowkit::core::playground::{Params, Playground};

#[cfg(feature = "ai")]
mod ai;
#[cfg(feature = "backgrounds")]
mod backgrounds;
#[cfg(feature = "charts")]
mod charts;
#[cfg(feature = "components")]
mod components;
#[cfg(feature = "effects")]
mod effects;
#[cfg(feature = "interactions")]
mod interactions;
#[cfg(feature = "templates")]
mod templates;
#[cfg(feature = "text")]
mod text;

/// Every playground of every enabled crate.
pub fn all() -> Vec<Playground> {
    #[allow(unused_mut)]
    let mut v: Vec<Playground> = Vec::new();
    #[cfg(feature = "text")]
    v.extend(text::all());
    #[cfg(feature = "backgrounds")]
    v.extend(backgrounds::all());
    #[cfg(feature = "effects")]
    v.extend(effects::all());
    #[cfg(feature = "charts")]
    v.extend(charts::all());
    #[cfg(feature = "ai")]
    v.extend(ai::all());
    #[cfg(feature = "interactions")]
    v.extend(interactions::all());
    #[cfg(feature = "templates")]
    v.extend(templates::all());
    #[cfg(feature = "components")]
    v.extend(components::all());
    v
}

// --------------------------------------------------------------------------------------------
// Code builder
// --------------------------------------------------------------------------------------------

/// Builds a rustfmt-style builder chain: `Head::new(..)` followed by `.setter(value)` calls.
pub struct Code {
    uses: Vec<String>,
    pre: Vec<String>,
    head: String,
    calls: Vec<String>,
}

#[allow(dead_code)]
impl Code {
    /// `uses` is a `use` path list such as `"glowkit::text::ShinyText"` (comma separated for several).
    pub fn new(uses: &str, head: impl Into<String>) -> Self {
        Self {
            uses: uses.split(';').map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()).collect(),
            pre: Vec::new(),
            head: head.into(),
            calls: Vec::new(),
        }
    }
    /// A statement printed before the chain (e.g. data setup).
    pub fn pre(mut self, line: impl Into<String>) -> Self {
        let line = line.into();
        if !line.is_empty() {
            self.pre.push(line);
        }
        self
    }
    /// `.name(arg)`.
    pub fn call(mut self, name: &str, arg: impl std::fmt::Display) -> Self {
        self.calls.push(format!(".{name}({arg})"));
        self
    }
    /// `.name(arg)` only when `cond`.
    pub fn call_if(self, cond: bool, name: &str, arg: impl std::fmt::Display) -> Self {
        if cond {
            self.call(name, arg)
        } else {
            self
        }
    }
    /// `.name(1.5)`.
    pub fn f(self, name: &str, v: f32) -> Self {
        self.call(name, lit_f32(v))
    }
    /// `.name(true)`.
    pub fn b(self, name: &str, v: bool) -> Self {
        self.call(name, v)
    }
    /// `.name(hex("#RRGGBB"))`.
    pub fn c(self, name: &str, c: Color32) -> Self {
        self.call(name, lit_color(c))
    }
    /// `.name("text")`.
    pub fn s(self, name: &str, s: &str) -> Self {
        self.call(name, lit_str(s))
    }

    fn header(&self) -> String {
        let mut uses = self.uses.clone();
        let body = format!("{} {}", self.head, self.calls.join(" ")) + &self.pre.join(" ");
        if body.contains("hex(") && !uses.iter().any(|u| u.contains("hex")) {
            uses.push("glowkit::color::hex".into());
        }
        if body.contains("vec2(") && !uses.iter().any(|u| u.contains("vec2")) {
            uses.push("glowkit::egui::vec2".into());
        }
        let mut out = String::new();
        for u in &uses {
            out.push_str(&format!("use {u};\n"));
        }
        if !out.is_empty() {
            out.push('\n');
        }
        for p in &self.pre {
            out.push_str(p);
            out.push('\n');
        }
        out
    }

    fn chain(&self, indent: usize) -> String {
        let pad = " ".repeat(indent);
        let mut s = self.head.clone();
        for c in &self.calls {
            s.push('\n');
            s.push_str(&pad);
            s.push_str(c);
        }
        s
    }

    fn one_line(&self) -> String {
        format!("{}{}", self.head, self.calls.concat())
    }

    /// `ui.add(Chain);`
    pub fn add(self) -> String {
        let h = self.header();
        let line = format!("ui.add({});", self.one_line());
        if line.len() <= 96 && self.calls.len() <= 2 {
            format!("{h}{line}")
        } else {
            format!("{h}ui.add(\n    {},\n);", self.chain(8))
        }
    }

    /// `Chain.show(ui, |ui| { body });`
    pub fn show(self, body: &str) -> String {
        let h = self.header();
        let body: String = body.lines().map(|l| format!("    {l}\n")).collect();
        let head = if self.one_line().len() <= 80 && self.calls.len() <= 2 { self.one_line() } else { self.chain(4) };
        format!("{h}{head}.show(ui, |ui| {{\n{body}}});")
    }

    /// `prefix Chain suffix` e.g. `let r = ` ... `.show(ui);`.
    pub fn stmt(self, prefix: &str, suffix: &str) -> String {
        let h = self.header();
        let head = if self.one_line().len() + prefix.len() <= 90 && self.calls.len() <= 2 {
            self.one_line()
        } else {
            self.chain(4)
        };
        format!("{h}{prefix}{head}{suffix}")
    }
}

// --------------------------------------------------------------------------------------------
// Layout helpers shared by playgrounds
// --------------------------------------------------------------------------------------------

/// Run `f` horizontally centered.
pub fn center<R>(ui: &mut Ui, f: impl FnOnce(&mut Ui) -> R) -> R {
    ui.vertical_centered(f).inner
}

/// A bool remembered across frames in the playground's scope (for `&mut bool` widgets).
#[allow(dead_code)]
pub fn flag(ui: &Ui, key: &str, default: bool) -> bool {
    let id = ui.scope_id().with(("gk_pg_flag", key));
    ui.data_mut(|d| *d.get_temp_mut_or_insert_with(id, || default))
}

/// Store a bool from [`flag`].
#[allow(dead_code)]
pub fn set_flag(ui: &Ui, key: &str, v: bool) {
    let id = ui.scope_id().with(("gk_pg_flag", key));
    ui.data_mut(|d| d.insert_temp(id, v));
}

/// A muted caption.
#[allow(dead_code)]
pub fn hint(ui: &mut Ui, text: &str) {
    let t = glowkit::Theme::of(ui);
    ui.label(egui::RichText::new(text).size(12.5).color(t.muted_foreground));
}

/// `Name::Variant` for an enum choice.
pub fn variant(ty: &str, options: &[&str], i: usize) -> String {
    format!("{ty}::{}", options[i.min(options.len() - 1)])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every playground renders a few frames with default values, belongs to a real demo and
    /// returns code that mentions the component.
    #[test]
    fn playgrounds_render_and_return_code() {
        let demos = glowkit::all_demos();
        let pgs = all();
        assert!(pgs.len() >= 45, "only {} playgrounds", pgs.len());
        let mut seen = std::collections::HashSet::new();
        let mut bad = Vec::new();
        let ctx = egui::Context::default();
        glowkit::Theme::light().install(&ctx);
        for pg in &pgs {
            assert!(seen.insert(pg.id), "duplicate playground {}", pg.id);
            assert!(demos.iter().any(|d| d.id == pg.id), "playground {} has no demo", pg.id);
            let mut params = Params::new();
            let mut code = String::new();
            for _ in 0..3 {
                let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                    code = (pg.show)(ui, &mut params);
                });
                out.textures_delta.clear();
            }
            let n = params.controls().len();
            if !(3..=6).contains(&n) {
                bad.push(format!("{} ({n})", pg.id));
            }
            assert!(code.contains("::new("), "playground {} code: {code}", pg.id);
            assert!(!code.contains("\n\n\n"), "playground {} code has blank runs", pg.id);
        }
        assert!(bad.is_empty(), "playgrounds need 3-6 controls: {}", bad.join(", "));
    }
}
