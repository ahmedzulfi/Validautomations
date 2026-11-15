//! Hand-written guidance served by `get_setup`, `egui_037_notes`, the server instructions and
//! the `build-app` prompt.

/// Instructions sent to the client at initialize.
pub const INSTRUCTIONS: &str = "\
glowkit is a pure-Rust library of 700 animated UI components, pages and app templates for egui 0.37 \
(native Windows/macOS/Linux and WebAssembly). Workflow: call get_setup once for Cargo.toml and the \
eframe app skeleton, egui_037_notes before writing egui code (the 0.37 API differs from what most \
training data shows), search_components to find components, then get_component(id) for the exact \
`use` path, constructor, builder setters and a usage snippet. get_template(id) returns whole pages and \
app templates embedded in a runnable app. get_theme covers theming. Never invent setters: only use the \
ones get_component lists.";

pub const SETUP: &str = r#"# glowkit setup

## Cargo.toml

```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2021"

[dependencies]
glowkit = "0.1"   # all component crates are on by default
eframe = "0.37"   # the native/web app runner; must match glowkit's egui 0.37
```

Compile only what you use (faster builds):

```toml
glowkit = { version = "0.1", default-features = false, features = ["text", "effects", "components"] }
```

Features: `text`, `backgrounds`, `effects`, `components`, `charts`, `ai`, `blocks`, `motion`,
`three-d`, `canvas`, `templates`, `interactions`, `agents` (all on by default). `get_component`
tells you which feature each component needs. Do not add `egui` separately: use the re-export
`glowkit::egui` (or `use glowkit::prelude::*;`, which brings `egui`, `Theme`, `Easing`, `Spring`,
`Trigger`, `hex`, `hsv`, `mix`, `with_alpha`) so the versions can never diverge.

## App skeleton (eframe 0.37)

```rust
use glowkit::prelude::*;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([640.0, 420.0])
            .with_title("My app"),
        ..Default::default()
    };
    eframe::run_native(
        "my-app",
        options,
        Box::new(|cc| {
            // One call styles every glowkit component and egui's own widgets.
            Theme::dark().install(&cc.egui_ctx);
            Ok(Box::new(MyApp::default()))
        }),
    )
}

#[derive(Default)]
struct MyApp {
    // All UI state lives here (text buffers, toggles, selected tab, template app states...).
    name: String,
}

impl eframe::App for MyApp {
    // eframe 0.37: implement `ui`, not `update`.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let theme = Theme::of(ui);
        egui::Panel::top("top_bar").show(ui, |ui| {
            ui.heading("My app");
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme.background).inner_margin(24))
            .show(ui, |ui| {
                ui.label("Hello");
                ui.text_edit_singleline(&mut self.name);
            });
    }
}
```

Panels: `egui::Panel::left("id")`, `::right`, `::top`, `::bottom` (the old `SidePanel` /
`TopBottomPanel` are gone), then `egui::CentralPanel::default()`; all take `.show(ui, |ui| ..)`.
Declare side/top panels before the central panel. Overlays such as `glowkit::components::Toaster`
are shown once per frame at the end of `ui`.

## Running

```bash
cargo run --release   # debug builds work but animations are much smoother in release
```

Linux needs the usual windowing libraries (X11/Wayland, e.g. `libxkbcommon`, `libgl`).

## WebAssembly

The same code runs in the browser. Add a wasm `main`, build for `wasm32-unknown-unknown` and use
`wasm-bindgen` (or `trunk`):

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
eframe = { version = "0.37", default-features = false, features = ["glow", "default_fonts"] }
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = ["Window", "Document", "HtmlCanvasElement"] }
```

```rust
#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast;
    wasm_bindgen_futures::spawn_local(async {
        let canvas = web_sys::window().unwrap().document().unwrap()
            .get_element_by_id("app_canvas").unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
        eframe::WebRunner::new()
            .start(canvas, eframe::WebOptions::default(), Box::new(|cc| {
                Theme::dark().install(&cc.egui_ctx);
                Ok(Box::new(MyApp::default()))
            }))
            .await
            .expect("failed to start");
    });
}
```

glowkit itself never uses threads, `std::time::Instant` or the filesystem, so every component
works on wasm. Keep your own app code wasm-safe the same way (use `glowkit::core::time(ui)`).

## Bevy

glowkit draws into any `egui::Ui`, so it works inside `bevy_egui` as long as that version of
`bevy_egui` uses egui 0.37. Call `Theme::dark().install(ctx)` once on the egui context.
"#;

pub const EGUI_037_NOTES: &str = r#"# egui / eframe 0.37 notes (things agents get wrong)

glowkit 0.1 targets **egui 0.37 / eframe 0.37**. Much of the egui code online is for 0.2x and does
not compile here.

## eframe

* Implement `fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame)` on `eframe::App`,
  **not** `fn update(&mut self, ctx: &egui::Context, ..)`. The `ui` you get has no margin or
  background: wrap content in `egui::CentralPanel::default().show(ui, |ui| ..)`.
* Need the context? `ui.ctx()`. Non-UI per-frame work can go in the optional
  `fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)`.
* `eframe::run_native(name, options, Box::new(|cc| Ok(Box::new(app))))`: the creator returns a
  `Result`. `main` returns `eframe::Result`.

## Panels

* `egui::Panel::left("id")`, `egui::Panel::right(..)`, `egui::Panel::top(..)`,
  `egui::Panel::bottom(..)` replace `SidePanel` / `TopBottomPanel` (removed).
* Every panel shows into a **`Ui`**: `.show(ui, |ui| ..)` (`show_inside` is a deprecated alias;
  there is no `show(ctx, ..)` for panels). Animated: `.show_collapsible(..)`, `.show_switched(..)`.
* `egui::Window::new("Title").show(ui.ctx(), |ui| ..)` still takes a `&Context`.

## Ids

* `egui::Id::new(..)` is **deprecated**: use `egui::Id::unique(..)` for a globally unique id, or
  derive from the current scope with `ui.scope_id().with("salt")` (preferred for widget state).
* `ui.id()` is deprecated: renamed to `ui.scope_id()`.
* Prefer one tuple over chained salts: `parent.with((row, col))`.

## Painting

* `painter.rect_stroke(rect, corner_radius, stroke, egui::StrokeKind::Inside)`: the `StrokeKind`
  argument (`Inside`, `Middle`, `Outside`) is required.
* `Rounding` is now `egui::CornerRadius` (u8 per corner): `CornerRadius::same(8)`. `Frame` uses
  `.corner_radius(..)`, not `.rounding(..)`.
* `egui::Margin` is i8 per side: `Margin::same(12)`, `Margin::symmetric(16, 8)`. A plain integer
  works where `impl Into<Margin>` is taken: `Frame::new().inner_margin(16)`.
* `egui::Frame::new()` (not `Frame::none()`), `Frame::default()`, `Frame::central_panel(style)`.
* Shapes: `egui::Shape::rect_filled/rect_stroke/circle_filled/line_segment`, text via
  `painter.text(pos, Align2::CENTER_CENTER, text, FontId::proportional(14.0), color)`.

## glowkit-specific

* Theme: `Theme::dark().install(ctx)` once (e.g. in the eframe creator). Read tokens with
  `Theme::of(ui)` (returns a copy). Do not hard-code colors: use `t.foreground`,
  `t.muted_foreground`, `t.card`, `t.border`, `t.accent`, `t.chart[i]`...
* Widgets: `ui.add(Thing::new(..).setter(..))`. Stateful widgets take `&mut T` in `new` (like
  egui's): keep the `T` in your app struct, not in a local `let` inside `ui` (it would reset every
  frame).
* Containers: `SpotlightCard::new().show(ui, |ui| ..)`; backgrounds: `Aurora::new().paint(ui, rect)` before
  drawing content on top. Template apps (`CrmApp`, `AnalyticsApp`...) are `Default` structs you
  store in your app and call `.show(ui)` on every frame.
* Per-widget state: `glowkit::core::{state, set_state, with_state}` keyed by an `Id`. **Never call
  any `ui`/`ctx` method inside a `with_state` closure**: it holds egui's context lock and
  deadlocks. Read time/input first, mutate inside, request repaints after.
* Time: `glowkit::core::time(ui)` and `glowkit::core::dt(ui)`, never `std::time::Instant` (not
  available on wasm). Continuous animation: call `glowkit::core::animate(ui)` each frame.
* Overlays (`Toaster`, dialogs, command palette) must be shown every frame, typically at the end
  of `ui`.
* Never invent setters: call `get_component` and use only the listed ones.
"#;

/// The `build-app` prompt body.
pub fn build_app_prompt(what: &str, platform: &str) -> String {
    format!(
        r#"Build a polished native Rust desktop app with glowkit for: {what}

Target: {platform}.

Use the glowkit MCP tools as you go:
1. Call `get_setup` and `egui_037_notes` first. Use eframe 0.37 (`impl eframe::App` with `fn ui`, not `update`;
   `egui::Panel::left/top` + `egui::CentralPanel`).
2. Plan the screens. For each screen call `search_components` (try the domain words, e.g. "sidebar",
   "data table", "line chart", "chat", "command palette") and check `get_template` for full app templates and
   pages you can start from (dashboards, CRM, chat, settings, auth, landing...).
3. For every component you use, call `get_component(id)` and copy its `use` path, constructor and setters
   exactly. Enable the cargo features it lists.
4. Install a theme once (`Theme::dark()` or a customized one, see `get_theme`) and take colors from
   `Theme::of(ui)` instead of hard-coding them.
5. Keep all state in the app struct. Show overlays (toasts, dialogs) every frame. Keep the code wasm-safe
   (no `std::time::Instant`; use `glowkit::core::time(ui)`).
6. Run `cargo build`, fix every error and warning, then `cargo run --release` and check that it looks polished:
   consistent spacing (8/12/16/24), clear hierarchy, one accent color, subtle motion rather than everything
   animating at once.

Deliver: Cargo.toml, src/main.rs (plus modules per screen if it grows), and a short README on how to run it."#
    )
}
