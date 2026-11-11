# Contributing to glowkit

Thanks for helping. This guide is also the spec every component in the repo follows, so please
read it fully before adding one.

## Layout

```
crates/
  glowkit-core         theme tokens, animation engine, painting helpers (everything builds on this)
  glowkit-text         animated text effects
  glowkit-backgrounds  animated backgrounds
  glowkit-effects      buttons, cards & borders, cursor & interaction, motion & layout
  glowkit-components   everyday components: inputs, navigation, overlays, data display, loaders
  glowkit-charts       animated charts
  glowkit-ai           AI interface components
  glowkit-blocks       full page sections composed from the others
  glowkit              umbrella crate, re-exports everything behind cargo features
apps/gallery           the gallery app (native + WebAssembly)
website/               landing page + the WebAssembly gallery
scripts/               build-web.sh, serve.sh, shot.mjs (headless screenshots)
```

## The rules for a component

1. **Pure egui.** Only depend on `egui` and `glowkit-core` (plus sibling glowkit crates where noted
   in `Cargo.toml`). No new crates without discussion. Everything must compile for
   `wasm32-unknown-unknown` too, so no threads, no `std::time::Instant` (use `glowkit_core::time(ui)`),
   no filesystem.
2. **Builder + `egui::Widget`.** A component is a struct with `new(...)` and chainable setters, and
   implements `egui::Widget` so people write `ui.add(Thing::new(..).size(..))`. Components that
   need caller-owned state (a text input, a switch) take `&mut T` in `new`, like egui's own widgets.
   Backgrounds also expose `paint(&self, ui, rect)` so they can sit behind other content.
3. **Theme-aware.** Default every color from `glowkit_core::Theme::of(ui)` (`t.foreground`,
   `t.muted_foreground`, `t.accent`, `t.border`, `t.card`, `t.gradient`, `t.chart`, ...). Never
   hard-code a color as the only option; offer a setter. Components must look right in every
   preset (`Theme::presets()`), including the light theme.
4. **Animation.**
   * Time: `glowkit_core::time(ui)`, frame delta: `glowkit_core::dt(ui)`.
   * Continuous animations call `glowkit_core::animate(ui)` (requests a repaint) every frame.
   * One-shot animations must stop requesting repaints when finished (`Clock::triggered` does this).
   * Per-widget state: `glowkit_core::{state, set_state, with_state}` keyed by the widget's
     `Response::id` (or `ui.id().with(...)`). State types must be `Clone + Send + Sync + 'static`.
   * Springs and easing: `glowkit_core::anim::{spring, spring2, follow, ease, Easing, Spring, stagger, Clock, Trigger}`.
   * Respect `Trigger` where it makes sense (`OnAppear`, `Loop`, `OnHover`, `OnClick`).
5. **Painting helpers** live in `glowkit_core::paint`: `gradient_rect`, `gradient_circle`,
   `fill_convex_fn` + `linear_fn` / `radial_fn` / `conic_fn`, `glow`, `glow_ellipse`, `soft_shadow`,
   `gradient_polyline`, `gradient_border`, `comet_fn`, `galley_with` (gradient text),
   `blurred_galley` (fake blur), `shader_grid` (per-vertex "shader" backgrounds), `fade_edges_*`,
   `vignette`, `star`, `sparkle`, `catmull_rom`, `monotone`, `area_fill`, `arc`, `arc_band`,
   `arc_gradient`, bezier helpers. Text per glyph: `glowkit_core::text::{Glyphs, paint_char, paint_rotated, scramble_char}`.
   Color: `glowkit_core::color::{hex, hsv, mix, gradient, gradient_cyclic, with_alpha, fade, shade, contrast_text, palettes}`.
   Noise: `glowkit_core::noise::{noise2, noise3, fbm3, noise01}`. RNG: `glowkit_core::Rng`.
   Read `crates/glowkit-core/src` before writing a component: the helper you need probably exists.
6. **Performance.** A component should cost well under 1 ms per frame in release builds. Keep
   meshes reasonable (e.g. `shader_grid` with ~40x24 cells, a few hundred particles).
   Skip painting when `!ui.is_rect_visible(rect)`.
7. **Docs.** Every public type gets a doc comment explaining what it looks like, with a short
   `no_run` example. Every setter gets a one-line doc.
8. **No copying.** glowkit components are original Rust implementations of well-known UI
   *patterns*. Never paste or translate source code from another library; build it from the visual
   idea. Use our own names for types if the original name is a trademark/brand.

## Demos (the gallery)

Each crate has `src/demos.rs` (compiled only with the `demos` feature) returning
`Vec<glowkit_core::demo::Demo>`:

```rust
Demo {
    id: "shiny-text",                     // unique kebab-case, used in URLs (#shiny-text)
    name: "Shiny Text",
    category: Category::TextAnimations,
    description: "One sentence on what it looks like.",
    covers: &["React Bits: Shiny Text", "Magic UI: Animated Shiny Text"], // web components this matches
    code: r##"ui.add(ShinyText::new("Hi"));"##, // ALWAYS r##"..."## (hex colors contain "#)
    show: |ui| { /* live preview, called every frame */ },
}
```

* The preview area is about 900 x 360+ points. Make previews look great: centered, generous
  spacing, show 2-3 variants when the component has interesting options.
* `covers` is how we track parity with web libraries: list every web component the demo is
  equivalent to, as `"Library: Component"`.

## Checking your work

```bash
cargo build -p glowkit-<crate> --features demos              # compile
cargo clippy -p glowkit-<crate> --features demos             # lint
cargo test -p glowkit-<crate>                                # tests
./scripts/serve.sh &                                         # serve ./website on :8787
FEATURES=<crate> PKG=pkg-<crate> PROFILE=dev ./scripts/build-web.sh
PKG=pkg-<crate> node scripts/shot.mjs /tmp/shots <demo-id> shot/<demo-id> shot/<demo-id>@1
#   "<id>" = full gallery page, "shot/<id>" = preview only, "@1" = light theme (index into Theme::presets())
```

Look at every screenshot. If it does not look polished, it is not done.
