# Agent instructions for this project

A native Rust desktop app built with **glowkit 0.1** (animated components) on **eframe/egui 0.37**.

## Layout

* `src/main.rs`: entry point, theme (`brand_theme`), app state and the `AppShell` (sidebar,
  top bar, command palette, notifications, toasts) that routes to screens.
* `src/screens/*.rs`: one struct per screen with its own state and a `show(&mut self, ui)`.
  Add a screen: create the module, add a field to `App`, a `NavItem` in `nav`, and a match arm.

## Commands

```bash
cargo run --release          # run (smooth animations)
cargo build && cargo clippy  # must stay free of errors and warnings
```

## Rules

* Use the `glowkit` MCP server if connected: `search_components` to find components,
  `get_component(id)` for the exact `use` path and setters, `get_template(id)` for whole pages,
  `egui_037_notes` before writing egui code. Never invent setters.
* eframe 0.37: `impl eframe::App` with `fn ui(&mut self, ui: &mut egui::Ui, frame)`, not `update`.
  Panels are `egui::Panel::left/right/top/bottom("id").show(ui, ..)`; `SidePanel`/`TopBottomPanel`
  are gone. `egui::Id::new`/`ui.id()` are deprecated: use `ui.scope_id().with(..)`.
* Import egui through glowkit (`use glowkit::prelude::*;`), never as a separate dependency.
* State that must persist goes in a struct field, never in a `let` inside `show`/`ui`.
* Colors from `Theme::of(ui)` tokens (`foreground`, `muted_foreground`, `card`, `border`,
  `accent`, `chart[i]`), not literals; the theme is set once in `main.rs`.
* Feedback via `toast(ui.ctx(), Toast::success(..))` (the shell shows the toaster).
* No `std::time::Instant`, threads or file IO in UI code if the app should also run on the web.
