---
name: glowkit
description: Build polished, animated native Rust desktop UIs (Windows, macOS, Linux, plus WebAssembly) with the glowkit component library on egui/eframe 0.37. Use when the user wants a Rust GUI app, dashboard, AI chat client, admin panel, dev tool, game menu or any native app that should look premium, or mentions glowkit, egui or eframe. Uses the glowkit MCP tools (search_components, get_component, get_template, get_setup, egui_037_notes, get_theme) when available.
---

# Building native Rust UIs with glowkit

glowkit gives Rust apps the polish web apps get from React component libraries: ~700 animated
components (buttons, inputs, tabs, sidebars, dialogs, toasts, data tables, charts, AI chat,
backgrounds, text effects, 3D, node editors), full pages (landing, auth, pricing, 404) and app
templates (analytics dashboard, CRM, chat, mail, settings, trading terminal, agent fleet...).
Everything is pure Rust on egui 0.37: no webview, no JavaScript.

## Workflow

1. **Ground yourself in the API before writing code.** If the `glowkit` MCP server is connected,
   call `get_setup` (Cargo.toml + eframe skeleton) and `egui_037_notes` (API changes your
   training data likely gets wrong). Otherwise follow the setup and rules below.
2. **Plan screens, then find components.** For each screen, call
   `search_components("<what it needs>")`: e.g. "sidebar", "data table", "stat card",
   "area chart", "command palette", "chat input", "toast". Web names work too
   ("shadcn Sidebar", "React Bits Aurora", "Magic UI Marquee").
3. **Start from a template when one fits.** `search_components(query, category: "Templates")`
   or `"Dashboards & Shells"`, then `get_template(id)` returns a runnable `main.rs` embedding it.
4. **Get exact usage.** `get_component(id)` gives the compiler-verified `use` path, the cargo
   feature, the constructor and every builder setter with its signature. Use only those setters.
5. **Theme it.** Install one theme at startup; customize tokens for the brand (`get_theme`).
6. **Build and look.** `cargo build` (fix all warnings), `cargo run --release`, check the result
   against the polish checklist below.

## Setup

```toml
[dependencies]
glowkit = "0.1"   # or: { version = "0.1", default-features = false, features = ["components", "charts"] }
eframe = "0.37"
```

```rust
use glowkit::prelude::*;                       // egui, Theme, Easing, Spring, Trigger, hex, ...
use glowkit::components::{Tabs, Toaster};       // paths from get_component

fn main() -> eframe::Result {
    eframe::run_native("My app", eframe::NativeOptions::default(), Box::new(|cc| {
        Theme::dark().install(&cc.egui_ctx);    // one call styles glowkit and egui widgets
        Ok(Box::new(App::default()))
    }))
}

#[derive(Default)]
struct App { tab: usize }

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {   // `ui`, not `update`
        egui::Panel::top("bar").show(ui, |ui| { ui.heading("My app"); });
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add(Tabs::new(&mut self.tab, &["Overview", "Activity"]));
        });
        Toaster::new().show(ui);                // overlays every frame
    }
}
```

A ready-made project lives in `agents/starter/` of the glowkit repo: copy it as a base.

## Rules that prevent most errors

* eframe 0.37: implement `App::ui(&mut self, ui, frame)`; panels are `egui::Panel::left/right/top/bottom(id).show(ui, ..)`;
  `SidePanel`/`TopBottomPanel` are gone; `Id::new` and `ui.id()` are deprecated (use
  `ui.scope_id().with(..)` / `Id::unique`); `rect_stroke` takes a `StrokeKind`; radii are
  `CornerRadius` (u8) and margins `Margin` (i8).
* Never add a separate `egui` dependency: use `glowkit::egui` / the prelude.
* Stateful widgets borrow state (`&mut T`) that must live in the app struct, not in a `let` inside `ui`.
* Template apps (`*App`) are `Default` structs: store them, call `.show(ui)` each frame.
* Colors come from `Theme::of(ui)` tokens, not literals, so light/dark both work.
* Inside `glowkit::core::with_state(..)` closures never touch `ui`/`ctx` (deadlock).
* Keep UI code wasm-safe: `glowkit::core::time(ui)` instead of `std::time::Instant`.

## Polish checklist

* One accent color; neutrals from the theme; verify in `Theme::light()` as well as dark.
* 4/8-point spacing (8, 12, 16, 24, 32); 16-24 px padding in cards; aligned edges.
* Clear hierarchy: one heading per screen, muted secondary text, numbers right-aligned.
* Purposeful motion: entrance animations (`Trigger::OnAppear`), hover feedback, animated
  numbers and charts. Avoid many looping effects on one screen.
* Empty, loading (skeletons/spinners) and error states for anything data-driven; toasts for
  feedback on actions.
* Window min size set (`ViewportBuilder::with_min_inner_size`) and layouts that survive resizing.

## Without the MCP server

Read the crate docs (`cargo doc -p glowkit --open`), the customization guide
(`crates/glowkit/docs/CUSTOMIZING.md`) and the gallery (`cargo run --release -p glowkit-gallery`,
or the live web gallery): every gallery entry shows its usage snippet.
