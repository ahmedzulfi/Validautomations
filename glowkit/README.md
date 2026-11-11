<div align="center">

# ✦ glowkit

**A big, beautiful, animated component library for Rust.**
Build Windows, macOS and Linux apps (and web apps through WebAssembly) with the kind of
components you see on the best websites: shimmering text, aurora backgrounds, border beams,
spotlight cards, magnetic docks, animated charts, and a full AI chat kit. 100% Rust, built on [egui](https://github.com/emilk/egui).

[Live gallery](https://glowkit-rs.github.io/glowkit/gallery/) · [Website](https://glowkit-rs.github.io/glowkit/) · [Catalog](CATALOG.md) · [Docs](https://docs.rs/glowkit) · [Contributing](CONTRIBUTING.md)

</div>

---

## Why

The web has an incredible ecosystem of copy-paste animated components (React Bits, Magic UI,
Aceternity, Motion Primitives, shadcn/ui, AI Elements, Evil Charts, ...). Rust desktop apps have
had nothing like it. glowkit brings that whole world to Rust:

* **Hundreds of components** across text animations, backgrounds, buttons, cards & borders,
  cursor effects, motion & layout, inputs, navigation, overlays, data display, loaders, charts, AI and page blocks.
* **Native everywhere.** One codebase runs as a native app on Windows, macOS and Linux, and in
  the browser via WebAssembly. No webview, no JavaScript.
* **Themeable.** Semantic design tokens (`background`, `foreground`, `primary`, `accent`,
  `border`, `ring`, gradients, chart palette...). Five presets, or your own. One call restyles
  every glowkit component *and* egui's built-in widgets.
* **Animation engine included.** 31 easing curves, springs, staggering, triggers
  (on appear / loop / hover / click), noise, gradients, glows and per-glyph text layout.
* **Modular.** Each category is its own crate behind a cargo feature, so you only compile what you use.
* **Original implementations.** glowkit recreates well-known UI *patterns* in Rust from scratch.
  See [CATALOG.md](CATALOG.md) for a generated parity table against the popular web libraries.

## Quick start

```toml
[dependencies]
glowkit = "0.1"
eframe = "0.37"
```

```rust
use glowkit::prelude::*;
use glowkit::{backgrounds::Aurora, effects::ShimmerButton, text::ShinyText};

fn main() -> eframe::Result {
    eframe::run_native("My app", Default::default(), Box::new(|cc| {
        Theme::dark().install(&cc.egui_ctx);
        Ok(Box::new(App))
    }))
}

struct App;

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            let rect = ui.max_rect();
            Aurora::new().paint(ui, rect);
            ui.vertical_centered(|ui| {
                ui.add_space(120.0);
                ui.add(ShinyText::new("Hello from Rust").size(48.0));
                if ui.add(ShimmerButton::new("Get started")).clicked() {
                    // ...
                }
            });
        });
    }
}
```

There is a runnable version in [`examples/hello`](examples/hello).

## What's inside

| Crate | Feature | What |
|---|---|---|
| `glowkit-core` | always | Theme tokens, easings, springs, clocks, gradients, glows, noise, per-glyph text |
| `glowkit-text` | `text` | Shiny, gradient, blur, split, decrypt, typewriter, rotating words, count-up, glitch, circular text... |
| `glowkit-backgrounds` | `backgrounds` | Aurora, particles, grids, waves, beams, meteors, starfields, silk, plasma, warp speed... |
| `glowkit-effects` | `effects` | Animated buttons, border beams, spotlight / tilt / glare cards, cursor trails, dock, marquee, carousels... |
| `glowkit-components` | `components` | Buttons, inputs, switches, selects, date picker, tabs, dialogs, toasts, tables, spinners... |
| `glowkit-charts` | `charts` | Animated line, area, bar, pie, donut, radar, radial, gauge, heatmap, treemap, sparklines... |
| `glowkit-ai` | `ai` | Chat messages, streaming markdown, prompt input, reasoning, tool calls, sources, voice orb, waveform... |
| `glowkit-blocks` | `blocks` | Hero sections, feature grids, bento, pricing, testimonials, stats, CTA, footer |

Browse everything in the **[live gallery](https://glowkit-rs.github.io/glowkit/gallery/)**, or run it locally:

```bash
cargo run --release -p glowkit-gallery
```

## Theming

```rust
// built-in presets: dark (default), light, ocean, ember, matrix
Theme::light().install(ctx);

// or your own brand
let mut t = Theme::dark();
t.accent = hex("#FF4F00");
t.gradient = [hex("#FF4F00"), hex("#FF0080"), hex("#7928CA")];
t.radius = 14.0;
t.install(ctx);
```

Every component reads its colors from the active theme and offers setters to override them.

## Running the web gallery

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130
./scripts/build-web.sh          # builds website/gallery/pkg
./scripts/serve.sh              # http://localhost:8787
```

## Performance

Components paint with egui's GPU tessellated meshes. Shader-like backgrounds use coarse vertex
grids (e.g. 48 x 28) with per-vertex color, so they stay cheap. Continuous animations request
repaints; one-shot animations stop repainting when they finish, so idle apps stay idle.

## Status

`0.1` is the first release. The API may change before `1.0`. See [CATALOG.md](CATALOG.md) for
coverage and open items, and [CONTRIBUTING.md](CONTRIBUTING.md) to help close them.

## Credits

glowkit is inspired by the work of the authors of React Bits, Magic UI, Aceternity UI, Motion
Primitives, shadcn/ui, Origin UI, Kibo UI, Animate UI, Cult UI, Uiverse, Vercel AI Elements,
prompt-kit, ElevenLabs UI, Evil Charts, Tremor and many more. No code from those projects is
included. All names belong to their respective owners and are used only to describe equivalence.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
