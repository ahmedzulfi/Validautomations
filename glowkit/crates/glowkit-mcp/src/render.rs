//! Markdown rendering of catalog entries for the tools.

use std::fmt::Write as _;

use crate::data::{Catalog, Component, StateVar, TypeApi};
use crate::search::Hit;

fn type_name(c: &Component) -> Option<&str> {
    c.type_key.as_deref().and_then(|k| k.rsplit("::").next())
}

/// The `use` lines for a component (prelude first).
pub fn use_lines(c: &Component) -> String {
    let mut out = String::from("use glowkit::prelude::*;\n");
    for p in &c.use_paths {
        let _ = writeln!(out, "use {p};");
    }
    out
}

/// Cargo dependency line enabling just what the component needs.
pub fn cargo_line(c: &Component) -> String {
    format!("glowkit = {{ version = \"0.1\", default-features = false, features = [\"{}\"] }}", c.feature)
}

pub fn search_results(cat: &Catalog, query: &str, category: Option<&str>, hits: &[Hit<'_>]) -> String {
    let mut out = String::new();
    let scope = category.map(|c| format!(" in category \"{c}\"")).unwrap_or_default();
    if hits.is_empty() {
        let _ = writeln!(out, "No components match \"{query}\"{scope}.");
        let _ = writeln!(out, "\nTry broader words (e.g. \"button\", \"chart\", \"sidebar\", \"chat\"), drop the category filter, or call list_categories.");
        return out;
    }
    let _ = writeln!(out, "{} result(s) for \"{query}\"{scope} (of {} components). Call get_component(id) for usage and API.\n", hits.len(), cat.components.len());
    for h in hits {
        let c = h.component;
        let ty = type_name(c).map(|t| format!(" `{t}`")).unwrap_or_default();
        let tpl = if c.template { " [template: get_template]" } else { "" };
        let _ = writeln!(out, "- **{}** (`{}`){ty}: {} _{}; feature `{}`_{tpl}", c.name, c.id, c.description, c.category, c.feature);
        if !c.covers.is_empty() {
            let _ = writeln!(out, "  - web equivalents: {}", c.covers.join("; "));
        }
    }
    out
}

pub fn type_api(t: &TypeApi, out: &mut String) {
    let path = t.path.as_deref().unwrap_or(&t.name);
    let _ = writeln!(out, "### `{path}` ({})\n", t.kind);
    if !t.doc.is_empty() {
        let _ = writeln!(out, "{}\n", t.doc);
    }
    if t.widget {
        let _ = writeln!(out, "Implements `egui::Widget`: use it with `ui.add(..)`.\n");
    }
    if !t.constructors.is_empty() {
        let _ = writeln!(out, "**Constructors**\n");
        for m in &t.constructors {
            method(m.sig.as_str(), &m.doc, out);
        }
        out.push('\n');
    }
    if !t.setters.is_empty() {
        let _ = writeln!(out, "**Builder setters** (chain them; each returns `Self`)\n");
        for m in &t.setters {
            method(m.sig.as_str(), &m.doc, out);
        }
        out.push('\n');
    }
    if !t.methods.is_empty() {
        let _ = writeln!(out, "**Methods**\n");
        for m in &t.methods {
            method(m.sig.as_str(), &m.doc, out);
        }
        out.push('\n');
    }
    if !t.fields.is_empty() {
        let _ = writeln!(out, "**Public fields**\n");
        for f in &t.fields {
            let doc = if f.doc.is_empty() { String::new() } else { format!(": {}", one_line(&f.doc)) };
            let _ = writeln!(out, "- `{}: {}`{doc}", f.name, f.ty);
        }
        out.push('\n');
    }
    if !t.variants.is_empty() {
        let _ = writeln!(out, "**Variants**: {}\n", t.variants.iter().map(|v| format!("`{v}`")).collect::<Vec<_>>().join(", "));
    }
    if let Some(ex) = &t.example {
        let _ = writeln!(out, "**Doc example**\n\n```rust\n{ex}\n```\n");
    }
    let _ = writeln!(out, "_Source: `{}`_\n", t.source);
}

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn method(sig: &str, doc: &str, out: &mut String) {
    let sig = sig.strip_prefix("pub ").unwrap_or(sig);
    if doc.is_empty() {
        let _ = writeln!(out, "- `{sig}`");
    } else {
        let _ = writeln!(out, "- `{sig}`: {}", one_line(doc));
    }
}

pub fn component(cat: &Catalog, c: &Component) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# {} (`{}`)\n", c.name, c.id);
    let _ = writeln!(out, "{}\n", c.description);
    let _ = writeln!(out, "- **Category**: {}", c.category);
    let _ = writeln!(out, "- **Crate**: `{}` (re-exported as `{}`), cargo feature `{}` of `glowkit` (on by default)", c.crate_name, c.module, c.feature);
    if let Some(t) = type_name(c) {
        let _ = writeln!(out, "- **Main type**: `{t}`");
    }
    if !c.covers.is_empty() {
        let _ = writeln!(out, "- **Web equivalents**: {}", c.covers.join("; "));
    }
    if c.template {
        let _ = writeln!(out, "- **Template**: call `get_template(\"{}\")` for a complete app embedding it", c.id);
    }
    let _ = writeln!(out, "\n## Imports\n\n```rust\n{}```\n", use_lines(c));
    let _ = writeln!(out, "Cargo (minimal): `{}`\n", cargo_line(c));
    let _ = writeln!(out, "## Usage\n\nInside your `ui(&mut self, ui: &mut egui::Ui, ..)`:\n\n```rust\n{}\n```\n", c.code.trim_end());
    if let Some(t) = cat.type_api(c) {
        let _ = writeln!(out, "## API\n");
        type_api(t, &mut out);
    }
    let others: Vec<&TypeApi> = c
        .use_paths
        .iter()
        .filter(|p| Some(p.as_str()) != cat.type_api(c).and_then(|t| t.path.as_deref()))
        .filter_map(|p| cat.types.values().find(|t| t.path.as_deref() == Some(p.as_str())))
        .filter(|t| t.kind == "enum")
        .collect();
    if !others.is_empty() {
        let _ = writeln!(out, "## Related types\n");
        for t in others {
            let _ = writeln!(out, "- `{}`: {}{}", t.path.as_deref().unwrap_or(&t.name), one_line(&t.doc), if t.variants.is_empty() { String::new() } else { format!(" Variants: {}", t.variants.join(", ")) });
        }
        out.push('\n');
    }
    if !c.related.is_empty() {
        let _ = writeln!(out, "## Related components\n");
        for r in &c.related {
            if let Some(o) = cat.component(r) {
                let _ = writeln!(out, "- `{}`: {}: {}", o.id, o.name, o.description);
            }
        }
    }
    out
}

pub fn categories(cat: &Catalog) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# glowkit {} categories ({} components, egui {})\n",
        cat.glowkit_version,
        cat.components.len(),
        cat.egui_version
    );
    let _ = writeln!(out, "Pass a category name to search_components(query, category). Templates, Dashboards & Shells and Blocks are full pages/apps: use get_template.\n");
    for c in &cat.categories {
        let _ = writeln!(out, "- **{}** ({}; feature {}): e.g. {}", c.name, c.count, c.features.iter().map(|f| format!("`{f}`")).collect::<Vec<_>>().join(", "), c.examples.join(", "));
    }
    out
}

/// A runnable `main.rs` embedding a template/page/section demo.
pub fn template(cat: &Catalog, c: &Component) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Template: {} (`{}`)\n", c.name, c.id);
    let _ = writeln!(out, "{}\n", c.description);
    if !c.covers.is_empty() {
        let _ = writeln!(out, "Web equivalents: {}\n", c.covers.join("; "));
    }
    let _ = writeln!(out, "## Snippet\n\n```rust\n{}\n```\n", c.code.trim_end());
    let _ = writeln!(out, "## Cargo.toml\n\n```toml\n[dependencies]\n{}\neframe = \"0.37\"\n```\n", cargo_line(c));
    let _ = writeln!(out, "## Complete app (src/main.rs)\n\n```rust\n{}```\n", template_app(c));
    let _ = writeln!(
        out,
        "## How to embed\n\n\
         * Template apps (`*App` types) are plain `Default` structs: keep one in your app struct and call `.show(ui)` every frame. \
           Set public fields (e.g. `route`) to pick the starting screen, and read/replace their data fields to plug in your own data.\n\
         * Pages and sections are builders: build them every frame inside a panel or a `ScrollArea` and react to the returned response \
           (e.g. `r.clicked(\"Get started\")`, `.submitted`).\n\
         * Shells (`AppShell`) take `&mut` route and nav state: store those in your app struct.\n\
         * Put it in `egui::CentralPanel` (or any `Ui`): templates fill the space they are given.\n\
         * Names the snippet uses but does not define (e.g. `page(ui)`, `dashboard(ui)`, `series`) are placeholders for your own screens and data.\n"
    );
    if let Some(t) = cat.type_api(c) {
        let _ = writeln!(out, "## API\n");
        type_api(t, &mut out);
    }
    out
}

/// Turn a demo snippet into a full eframe app. Recognizes the `let mut app = XApp::default();
/// app.field = ..; app.show(ui);` template pattern and moves the state into the app struct;
/// any other snippet runs inside the central panel, with `let mut` locals hoisted to fields.
pub fn template_app(c: &Component) -> String {
    let mut out = String::new();
    out.push_str(&use_lines(c));
    out.push('\n');
    out.push_str(
        "fn main() -> eframe::Result {\n    let options = eframe::NativeOptions {\n        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 820.0]),\n        ..Default::default()\n    };\n",
    );
    let _ = writeln!(out, "    eframe::run_native(\n        \"{}\",\n        options,", c.name);
    out.push_str("        Box::new(|cc| {\n            Theme::dark().install(&cc.egui_ctx);\n            Ok(Box::new(MyApp::default()))\n        }),\n    )\n}\n\n");

    let lines: Vec<&str> = c.code.lines().collect();
    let first = lines.first().map(|l| l.trim()).unwrap_or("");
    let app_pattern = first
        .strip_prefix("let mut ")
        .and_then(|r| r.split_once(" = "))
        .and_then(|(var, rhs)| rhs.split_once("::default();").map(|(ty, _)| (var.trim(), ty.trim())))
        .filter(|(var, ty)| {
            let ident = |s: &str| !s.is_empty() && s.chars().all(|ch| ch.is_alphanumeric() || ch == '_');
            ident(var) && ident(ty) && c.code.contains(&format!("{var}.show(ui)"))
        });
    if let Some((var, ty)) = app_pattern {
        let setup: Vec<String> = lines[1..]
            .iter()
            .map(|l| l.trim())
            .filter(|l| l.starts_with(&format!("{var}.")) && l.contains(" = "))
            .map(|l| l.replacen(&format!("{var}."), "app.", 1))
            .collect();
        let _ = writeln!(out, "struct MyApp {{\n    {var}: {ty},\n}}\n");
        let _ = writeln!(out, "impl Default for MyApp {{\n    fn default() -> Self {{");
        if setup.is_empty() {
            let _ = writeln!(out, "        Self {{ {var}: {ty}::default() }}");
        } else {
            let _ = writeln!(out, "        let mut app = {ty}::default();");
            for s in setup {
                let _ = writeln!(out, "        {s}");
            }
            let _ = writeln!(out, "        Self {{ {var}: app }}");
        }
        out.push_str("    }\n}\n\n");
        out.push_str("impl eframe::App for MyApp {\n    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {\n");
        out.push_str("        egui::CentralPanel::default().frame(egui::Frame::new()).show(ui, |ui| {\n");
        let _ = writeln!(out, "            self.{var}.show(ui);");
        out.push_str("        });\n    }\n}\n");
        return out;
    }

    // Generic snippet: state the snippet keeps across frames becomes fields of the app struct
    // (types inferred by the generator from the receiving parameters), the rest runs every frame.
    let fields: Vec<&StateVar> = c.state.iter().filter(|s| s.ty.is_some()).collect();
    let mut body = c.code.clone();
    for f in &fields {
        if let Some(decl) = &f.decl {
            body = body.replacen(decl.as_str(), "", 1);
        }
        body = replace_mut_borrow(&body, &f.name);
    }
    if fields.is_empty() {
        out.push_str("#[derive(Default)]\nstruct MyApp;\n\n");
    } else {
        out.push_str("struct MyApp {\n");
        for f in &fields {
            let _ = writeln!(out, "    {}: {},", f.name, f.ty.as_deref().unwrap_or("_"));
        }
        out.push_str("}\n\nimpl Default for MyApp {\n    fn default() -> Self {\n        Self {\n");
        for f in &fields {
            let init = f.init.as_deref().map(|i| indent(i, "            ").trim_start().to_string());
            let _ = writeln!(out, "            {}: {},", f.name, init.unwrap_or_else(|| "Default::default()".into()));
        }
        out.push_str("        }\n    }\n}\n\n");
    }
    out.push_str("impl eframe::App for MyApp {\n    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {\n");
    if !fields.is_empty() {
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        let _ = writeln!(out, "        let Self {{ {} }} = self;", names.join(", "));
    }
    out.push_str("        egui::CentralPanel::default().show(ui, |ui| {\n");
    out.push_str("            egui::ScrollArea::vertical().show(ui, |ui| {\n");
    for l in body.trim().lines() {
        if l.trim().is_empty() {
            out.push('\n');
        } else {
            let _ = writeln!(out, "                {l}");
        }
    }
    out.push_str("            });\n        });\n    }\n}\n");
    out
}

fn indent(s: &str, prefix: &str) -> String {
    s.lines().map(|l| format!("{prefix}{l}")).collect::<Vec<_>>().join("\n")
}

/// `&mut name` -> `name` (the field is already a `&mut` after destructuring `self`).
fn replace_mut_borrow(code: &str, name: &str) -> String {
    let pat = format!("&mut {name}");
    let mut out = String::with_capacity(code.len());
    let mut rest = code;
    while let Some(i) = rest.find(&pat) {
        let after = &rest[i + pat.len()..];
        let boundary = !after.starts_with(|c: char| c.is_alphanumeric() || c == '_' || c == '.' || c == '[');
        out.push_str(&rest[..i]);
        out.push_str(if boundary { name } else { &pat });
        rest = after;
    }
    out.push_str(rest);
    out
}

pub fn theme(cat: &Catalog) -> String {
    let th = &cat.theme;
    let mut out = String::new();
    let _ = writeln!(out, "# glowkit theme\n\n{}\n", th.doc);
    out.push_str(
        "One `Theme` restyles every glowkit component *and* egui's built-in widgets. Install it once (e.g. in the eframe \
creator) and read it anywhere with `Theme::of(ui)`. Switching theme at runtime is just another `install` call.\n\n\
```rust\nuse glowkit::prelude::*;\n\n// presets: Theme::dark() (default), light(), ocean(), ember(), matrix()\nTheme::dark().install(&cc.egui_ctx);\n\n\
// your brand: start from a preset and change tokens\nlet mut t = Theme::dark();\nt.accent = hex(\"#FF4F00\");\nt.ring = t.accent;\n\
t.gradient = [hex(\"#FF4F00\"), hex(\"#FF0080\"), hex(\"#7928CA\")];\nt.radius = 14.0;\nt.install(ctx);\n\n\
// reading tokens in your own UI\nlet t = Theme::of(ui);\nui.label(egui::RichText::new(\"Muted\").color(t.muted_foreground));\n```\n\n",
    );
    let _ = writeln!(out, "## Tokens\n");
    let dark = th.presets.iter().find(|p| p.func == "dark");
    for f in &th.fields {
        let val = dark.and_then(|p| p.values.get(&f.name)).map(|v| format!(" (dark: {})", v.to_string().replace('"', ""))).unwrap_or_default();
        let doc = if f.doc.is_empty() { String::new() } else { format!(": {}", one_line(&f.doc)) };
        let _ = writeln!(out, "- `{}: {}`{doc}{val}", f.name, f.ty);
    }
    let _ = writeln!(out, "\n## Presets\n");
    for p in &th.presets {
        let name = p.values.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let key = |k: &str| p.values.get(k).and_then(|v| v.as_str()).unwrap_or("?").to_string();
        let _ = writeln!(out, "- `Theme::{}()` \"{name}\": {} background {}, foreground {}, accent {}", p.func, one_line(&p.doc), key("background"), key("foreground"), key("accent"));
    }
    let _ = writeln!(out, "\n## Theme methods\n");
    for m in &th.methods {
        method(&m.sig, &m.doc, &mut out);
    }
    out.push_str(
        "\n## Customizing components\n\n\
* Every component defaults its colors from the theme and offers setters to override them: `.color(..)`, `.colors(..)`, \
`.size(..)` (main dimension), `.speed(..)` (seconds per cycle), `.trigger(Trigger::OnAppear | Loop { pause_ms } | OnHover | OnClick)`.\n\
* Components must look right in every preset, including `Theme::light()`: test your app in light and dark.\n\
* Read `glowkit://docs/customizing` (resource) for the full guide: tokens, setters, motion (springs, easing, the motion engine) and composition.\n",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::catalog;

    #[test]
    fn app_template_pattern_is_hoisted() {
        let cat = catalog();
        let c = cat.component("app-crm-pipeline").unwrap();
        let app = template_app(c);
        assert!(app.contains("struct MyApp {\n    app: CrmApp,"), "{app}");
        assert!(app.contains("app.route = \"pipeline\".into();"), "{app}");
        assert!(app.contains("self.app.show(ui);"), "{app}");
        assert!(app.contains("use glowkit::templates::CrmApp;"), "{app}");
        assert!(app.contains("fn ui(&mut self, ui: &mut egui::Ui"));
    }

    #[test]
    fn component_page_has_everything() {
        let cat = catalog();
        let page = component(cat, cat.component("shiny-text").unwrap());
        for needle in ["use glowkit::text::ShinyText;", "feature `text`", "Builder setters", "fn speed(", "Related components", "Magic UI"] {
            assert!(page.contains(needle), "missing {needle}:\n{page}");
        }
    }

    /// Writes the generated app for every component to `$GLOWKIT_MCP_EXPORT/<id>.rs`, so they
    /// can be compiled against the workspace (e.g. as `#[path]` modules of a scratch crate).
    #[test]
    #[ignore = "export helper, run explicitly"]
    fn export_template_apps() {
        let Ok(dir) = std::env::var("GLOWKIT_MCP_EXPORT") else { return };
        std::fs::create_dir_all(&dir).unwrap();
        for c in &catalog().components {
            std::fs::write(format!("{dir}/{}.rs", c.id), template_app(c)).unwrap();
        }
    }
}
