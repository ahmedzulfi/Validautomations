//! `glowkit init`: scaffold a new eframe app that uses glowkit from crates.io.

use std::path::Path;

pub const TEMPLATES: &[(&str, &str)] = &[
    ("blank", "Aurora background, shiny headline and a shimmer button"),
    ("dashboard", "Analytics dashboard (sidebar shell, KPI cards, charts)"),
    ("ai-chat", "Team chat app with channels, threads and a composer"),
    ("agent-sessions", "Coding-agent session manager (sessions, transcript, diffs)"),
    ("trading", "Trading terminal (candles, order book, positions)"),
    ("game", "Game main menu with live HUD resource bars"),
];

fn features(template: &str) -> &'static str {
    match template {
        "blank" => r#""backgrounds", "text", "effects""#,
        "game" => r#""interactions""#,
        _ => r#""templates""#,
    }
}

const PRELUDE: &str = r#"// `::glowkit` is the crates.io dependency. Components you copy with `glowkit add` live in
// `crate::glowkit` (src/glowkit/), so both can be used side by side.
"#;

fn main_rs(template: &str, title: &str) -> String {
    let run = format!(
        r#"fn main() -> eframe::Result {{
    let options = eframe::NativeOptions {{
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]).with_title({title:?}),
        ..Default::default()
    }};
    eframe::run_native(
        {title:?},
        options,
        Box::new(|cc| {{
            // One call styles every glowkit component and egui's own widgets.
            Theme::dark().install(&cc.egui_ctx);
            Ok(Box::new(App::default()))
        }}),
    )
}}
"#
    );
    let app_template = |ty: &str, doc: &str| {
        format!(
            r#"{PRELUDE}use ::glowkit::prelude::*;
use ::glowkit::templates::{ty};

{run}
/// {doc}
#[derive(Default)]
struct App {{
    app: {ty},
}}

impl eframe::App for App {{
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {{
        let theme = Theme::of(ui);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme.background))
            .show(ui, |ui| self.app.show(ui));
    }}
}}
"#
        )
    };
    match template {
        "dashboard" => {
            app_template("AnalyticsApp", "The analytics dashboard template: replace its sample data with yours.")
        }
        "ai-chat" => app_template("ChatApp", "A chat app: channels, messages, reactions and a composer."),
        "agent-sessions" => app_template("AgentSessionsApp", "A session manager for coding agents."),
        "trading" => app_template("TradingTerminalApp", "A trading terminal with live candles and an order book."),
        "game" => format!(
            r#"{PRELUDE}use ::glowkit::interactions::{{GameMainMenu, GameResourceBar, GameResourceKind, GameStyle}};
use ::glowkit::prelude::*;

{run}
struct App {{
    selected: usize,
    activated: Option<usize>,
    hp: f32,
    mana: f32,
}}

impl Default for App {{
    fn default() -> Self {{
        Self {{ selected: 0, activated: None, hp: 1000.0, mana: 600.0 }}
    }}
}}

const ITEMS: &[&str] = &["Continue", "New Game", "Settings", "Quit"];

impl eframe::App for App {{
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {{
        let theme = Theme::of(ui);
        // fake game state: health and mana drift over time
        let t = ::glowkit::core::time(ui);
        self.hp = 640.0 + 360.0 * (t * 0.7).sin().abs();
        self.mana = 300.0 + 300.0 * (t * 0.4).cos().abs();
        egui::CentralPanel::default().frame(egui::Frame::new().fill(theme.background)).show(ui, |ui| {{
            ui.add(
                GameMainMenu::new(&mut self.selected, ITEMS)
                    .title({title:?})
                    .subtitle("Press Enter to start")
                    .style(GameStyle::SciFi)
                    .height(520.0),
            );
            ui.add_space(16.0);
            ui.horizontal(|ui| {{
                ui.add(GameResourceBar::new(self.hp, 1000.0).kind(GameResourceKind::Health).segments(10));
                ui.add(GameResourceBar::new(self.mana, 600.0).kind(GameResourceKind::Mana));
            }});
            if let Some(i) = self.activated.take() {{
                ui.label(format!("activated: {{}}", ITEMS[i]));
            }}
        }});
    }}
}}
"#
        ),
        _ => format!(
            r#"{PRELUDE}use ::glowkit::backgrounds::Aurora;
use ::glowkit::effects::ShimmerButton;
use ::glowkit::prelude::*;
use ::glowkit::text::ShinyText;

{run}
#[derive(Default)]
struct App {{
    clicks: u32,
}}

impl eframe::App for App {{
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {{
        let theme = Theme::of(ui);
        egui::CentralPanel::default().frame(egui::Frame::new().fill(theme.background)).show(ui, |ui| {{
            // Paint the background behind everything else in this panel.
            Aurora::new().paint(ui, ui.max_rect());
            ui.vertical_centered(|ui| {{
                ui.add_space(ui.available_height() * 0.3);
                ui.add(ShinyText::new({title:?}).size(48.0));
                ui.add_space(24.0);
                if ui.add(ShimmerButton::new("Click me")).clicked() {{
                    self.clicks += 1;
                }}
                ui.add_space(8.0);
                ui.label(egui::RichText::new(format!("clicked {{}} times", self.clicks)).color(theme.muted_foreground));
            }});
        }});
    }}
}}
"#
        ),
    }
}

fn cargo_toml(name: &str, template: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
glowkit = {{ version = "0.1", default-features = false, features = [{feats}] }}
eframe = "0.37"
# Used by components copied into src/glowkit/ with `glowkit add <id>`:
glowkit-core = "0.1"
egui = "0.37"
"#,
        feats = features(template)
    )
}

fn agents_md(name: &str, template: &str) -> String {
    format!(
        r#"# {name}: notes for AI coding agents

This is a native Rust GUI app built with [eframe](https://crates.io/crates/eframe) 0.37 /
[egui](https://crates.io/crates/egui) 0.37 and [glowkit](https://crates.io/crates/glowkit)
(animated components on egui). Scaffolded with `glowkit init --template {template}`.

## Layout

- `src/main.rs`: app entry point and the `eframe::App` impl.
- `src/glowkit/`: components copied with `glowkit add <id>`. This code belongs to the project:
  edit it freely. It depends only on `glowkit-core` and `egui`.

## Commands

```bash
cargo run                      # run the app
glowkit list [query]           # find components (e.g. `glowkit list chart`)
glowkit info <id>              # crate, feature, use path and a usage snippet
glowkit add <id>               # copy a component's source into src/glowkit/
```

## Using components

- From the crate: `use ::glowkit::text::ShinyText;` then `ui.add(ShinyText::new("Hi"))`.
  Enable the category in Cargo.toml (`features = ["text", "charts", ...]`).
- From copied source: add `mod glowkit;` to `src/main.rs` once, then
  `use crate::glowkit::text::ShinyText;`. Keep the `::glowkit` / `crate::glowkit` prefixes: a bare
  `use glowkit::...` is ambiguous once both exist.
- Components are builders that implement `egui::Widget` (`ui.add(Thing::new(..).size(..))`);
  full-page templates and backgrounds expose `.show(ui)` / `.paint(ui, rect)`.

## Rules

- Theme: install once with `Theme::dark().install(ctx)`; read colors with `Theme::of(ui)`
  (`t.foreground`, `t.muted_foreground`, `t.accent`, `t.border`, `t.card`, ...). Never hard-code
  colors as the only option.
- Time and animation: use `glowkit_core::time(ui)` and `glowkit_core::dt(ui)`, not
  `std::time::Instant` (keeps the app WebAssembly-friendly). Continuous animations call
  `glowkit_core::animate(ui)`; one-shot ones stop requesting repaints when done.
- Per-widget state: `glowkit_core::{{state, set_state, with_state}}` keyed by a widget id. Never call
  `ui`/`ctx` methods inside a `with_state` closure (it holds egui's context lock).
- egui 0.37: use `ui.scope_id()`, not the deprecated `ui.id()`.
- Check work with `cargo build` and `cargo clippy`.
"#
    )
}

/// Create the project directory.
pub fn init(dir: &Path, template: &str) -> Result<(), String> {
    if !TEMPLATES.iter().any(|(t, _)| *t == template) {
        let list: Vec<String> = TEMPLATES.iter().map(|(t, d)| format!("  {t:15} {d}")).collect();
        return Err(format!("unknown template `{template}`. Available:\n{}", list.join("\n")));
    }
    let name = dir.file_name().map(|n| n.to_string_lossy().to_string()).ok_or("invalid project path")?;
    let pkg: String =
        name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).collect();
    if pkg.is_empty() || pkg.starts_with(|c: char| c.is_ascii_digit()) {
        return Err(format!("`{name}` is not a valid package name"));
    }
    if dir.exists() && std::fs::read_dir(dir).map(|mut d| d.next().is_some()).unwrap_or(true) {
        return Err(format!("{} already exists and is not empty", dir.display()));
    }
    let title: String = pkg
        .split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ");
    let write = |rel: &str, content: &str| -> Result<(), String> {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&p, content).map_err(|e| format!("{}: {e}", p.display()))
    };
    write("Cargo.toml", &cargo_toml(&pkg, template))?;
    write("src/main.rs", &main_rs(template, &title))?;
    write("AGENTS.md", &agents_md(&pkg, template))?;
    write(".gitignore", "/target\n")?;
    println!("Created {} (template: {template})", dir.display());
    println!("  Cargo.toml\n  src/main.rs\n  AGENTS.md\n  .gitignore");
    println!(
        "\nNext:\n  cd {}\n  cargo run\n  glowkit add shiny-text    # copy components into src/glowkit/",
        dir.display()
    );
    Ok(())
}
