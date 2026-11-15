# Building apps with glowkit (instructions for coding agents)

glowkit is a pure-Rust library of ~700 animated UI components, pages and full app templates on
**egui 0.37 / eframe 0.37**. Apps run natively on Windows, macOS and Linux, and in the browser
via WebAssembly. Copy this file into your project as `AGENTS.md` (or `CLAUDE.md`).

## Use the MCP server

If the `glowkit` MCP server is connected, use it instead of guessing APIs:

| Tool | Use it to |
|---|---|
| `get_setup` | get Cargo.toml deps and the eframe 0.37 app skeleton (once, at the start) |
| `egui_037_notes` | read the egui 0.37 API changes before writing egui code |
| `search_components(query, category?, limit?)` | find components ("sidebar", "line chart", "chat input", "React Bits Aurora") |
| `get_component(id)` | exact `use` path, constructor, builder setters, cargo feature, usage snippet |
| `get_template(id)` | a full page or app template embedded in a runnable `main.rs` |
| `list_categories` / `get_theme` | browse categories; theme tokens, presets and customization |

Install: `cargo install glowkit-mcp`, then `claude mcp add glowkit -- glowkit-mcp` (other clients:
see `agents/mcp/`). **Only use setters that `get_component` lists.** If the server is not
available, read the crate docs (`cargo doc -p glowkit --open`) or the source.

## Setup

```toml
[dependencies]
glowkit = "0.1"
eframe = "0.37"
```

Use `use glowkit::prelude::*;` (brings `egui`, `Theme`, `Easing`, `Spring`, `Trigger`, `hex`,
`hsv`, `mix`, `with_alpha`). Do not add a separate `egui` dependency: use the re-export
`glowkit::egui` so versions never diverge. Components live under `glowkit::<feature>::Type`
(`glowkit::text::ShinyText`, `glowkit::components::Tabs`, `glowkit::charts::AreaChart`,
`glowkit::templates::AppShell`...). All features are on by default; trim with
`default-features = false, features = [...]` for faster builds.

## eframe / egui 0.37 rules

* Implement `fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame)` on `eframe::App`,
  **not** `update(ctx, ..)`. Wrap content in `egui::CentralPanel::default().show(ui, |ui| ..)`.
* Panels: `egui::Panel::left("id")` / `right` / `top` / `bottom`, then `.show(ui, |ui| ..)`.
  `SidePanel` and `TopBottomPanel` no longer exist. Declare them before the `CentralPanel`.
* Ids: `egui::Id::new` is deprecated: use `ui.scope_id().with("salt")` or `egui::Id::unique(..)`.
  `ui.id()` is now `ui.scope_id()`.
* `painter.rect_stroke(rect, radius, stroke, egui::StrokeKind::Inside)` needs a `StrokeKind`.
  Corner radii are `egui::CornerRadius` (u8), margins `egui::Margin` (i8): `Frame::new().inner_margin(16)`.
* `egui::Window::new(..).show(ui.ctx(), ..)` still takes a context.

## glowkit conventions

* **Theme once, read everywhere.** `Theme::dark().install(&cc.egui_ctx)` in the eframe creator
  (presets: `dark`, `light`, `ocean`, `ember`, `matrix`). Read colors with `Theme::of(ui)`
  (`t.foreground`, `t.muted_foreground`, `t.card`, `t.border`, `t.accent`, `t.chart[i]`) instead
  of hard-coding them. A brand theme is a preset with a few tokens changed.
* **Builders.** `ui.add(Thing::new(..).size(..).color(..))`. Containers: `SpotlightCard::new().show(ui, |ui| ..)`.
  Backgrounds paint behind content: `Aurora::new().paint(ui, ui.max_rect())`, then draw on top.
* **State lives in your app struct.** Widgets that need state take `&mut T` in `new` (like egui's):
  `Tabs::new(&mut self.tab, &["A", "B"])`. A `let mut` inside `ui` resets every frame.
  Template apps (`CrmApp`, `AnalyticsApp`, `ChatApp`...) are `Default` structs: store one and call
  `.show(ui)` each frame.
* **Overlays every frame.** `Toaster::new().show(ui)` (and dialogs, palettes) at the end of `ui`;
  `toast(ui.ctx(), Toast::success("Saved"))` from anywhere.
* **Per-widget state** in your own widgets: `glowkit::core::{state, set_state, with_state}` keyed
  by an `Id`. Never call a `ui`/`ctx` method inside a `with_state` closure: it holds egui's
  context lock and deadlocks.
* **wasm-safe.** No `std::time::Instant`, threads or filesystem in UI code: use
  `glowkit::core::time(ui)` / `dt(ui)`; call `glowkit::core::animate(ui)` for continuous animation.

## Making it look polished

* One accent color, the theme's neutrals for everything else; test in `Theme::light()` too.
* Spacing on a 4/8 grid (8, 12, 16, 24, 32); generous padding inside cards (16-24).
* Hierarchy: one large heading per screen, muted secondary text (`t.muted_foreground`).
* Motion with restraint: an entrance (`Trigger::OnAppear`), hover feedback, animated numbers;
  not everything looping at once.
* Prefer a template or shell (`get_template`) over building a layout from scratch.

## Checklist

1. `cargo build` with zero errors and warnings (`cargo clippy` too).
2. `cargo run --release` (animations are much smoother in release).
3. Every component's `use` path and setters came from `get_component` (or the docs).
