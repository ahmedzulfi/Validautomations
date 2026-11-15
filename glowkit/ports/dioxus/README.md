# glowkit for Dioxus

A port of [glowkit](../../README.md)'s design language and flagship animated components to
[Dioxus](https://dioxuslabs.com) **0.7** (tested with 0.7.10). It uses the same tokens, the same
five theme presets, the same 31 easing curves and the same spring model as the egui original.
Rendering is HTML and CSS: CSS keyframes drive ambient motion, and frame-driven springs drive
interactive motion.

This is a separate cargo workspace. The main glowkit workspace excludes `ports/`.

| Crate | What |
|---|---|
| `crates/glowkit-tokens` | Framework-agnostic and egui-free. Holds `Theme` (Midnight, Daylight, Ocean, Ember, Matrix) with `css_vars()` / `css_declarations()`, the OKLab color math (`hex`, `mix`, `gradient`, `shade`, palettes), the 31 `Easing` curves (`ease`, plus `Easing::css()` for CSS timing functions, sampled to `linear(...)` for elastic and bounce), `Spring`, `Trigger`, noise (`noise3`, `fbm3`), and the typewriter, number-format and token timelines. |
| `crates/glowkit-dioxus` | The components, `GlowkitProvider` (the theme as `--gk-*` CSS custom properties plus the injected stylesheet) and the animation hooks. |
| `examples/gallery` | A Dioxus app that shows every component, with a detail page, a usage snippet and live theme switching. |

## Usage

```toml
[dependencies]
glowkit-dioxus = { path = "ports/dioxus/crates/glowkit-dioxus" }
dioxus = { version = "0.7", features = ["web", "mounted"] }   # "mounted" is needed for measuring
```

```rust
use dioxus::prelude::*;
use glowkit_dioxus::prelude::*;

fn main() { dioxus::launch(App); }

#[component]
fn App() -> Element {
    let tab = use_signal(|| 0);
    rsx! {
        GlowkitProvider { theme: Theme::dark(),
            ToastProvider {
                Aurora {
                    ShinyText { text: "Hello from Rust", size: 48.0 }
                    ShimmerButton { label: "Get started", onclick: move |_| { /* ... */ } }
                }
                AnimatedTabs { selected: tab, labels: vec!["Overview".into(), "API".into()] }
            }
        }
    }
}

// below a ToastProvider:
#[component]
fn SaveButton() -> Element {
    let toaster = use_toaster();
    rsx! { RainbowButton { label: "Save", onclick: move |_| { toaster.push(Toast::success("Saved")); } } }
}
```

Props mirror the egui builder options. For example, `ShinyText::new(t).size(32.0).speed(2.5)`
becomes `ShinyText { text: t, size: 32.0, speed: 2.5 }`. Colors are CSS strings, so `"#fff"` and
`"var(--gk-accent)"` both work. The canvas backgrounds take `glowkit_tokens::Color` instead.
State that egui passes as `&mut` (the tabs' `selected`, the prompt's text) is passed as a `Signal`.

### Animation hooks

- `use_spring(initial, SpringConfig)` / `use_springs(vec, cfg)` run glowkit's `Spring` once per
  frame (`requestAnimationFrame` on the web, about 60 Hz on desktop) and go idle when settled.
  The handle is `Copy`. Use `.get()`, `.set(target)`, `.snap(v)` and `.impulse(v)`.
- `use_tween(from, to, secs, Easing, delay)` animates a number from one value to another.
- `use_clock(running)` gives elapsed seconds.
- `use_replay(Trigger, cycle)` restarts CSS keyframes for `OnHover`, `OnClick` and `Loop`.
- `use_element_rect()` turns pointer events into coordinates local to an element.

## Components (22)

| Group | Components (egui original) |
|---|---|
| Text | `ShinyText`, `GradientText`, `BlurText`, `TypewriterText`, `CountUp` (`glowkit_text`) |
| Backgrounds | `Aurora`, `DotGrid` (`glowkit_backgrounds`). On the web each paints into a `<canvas>` with the egui math verbatim: the same 96x56 shader grid and the same per-dot spring field. |
| Buttons | `ShimmerButton`, `RainbowButton`, `MagneticButton` (`glowkit_effects`) |
| Cards | `BorderBeam`, `SpotlightCard`, `TiltCard` (`glowkit_effects`) |
| Motion | `Marquee`, `Dock` + `DockItem`, `AnimatedTabs` (`glowkit_effects`) |
| Overlays | `ToastProvider` + `use_toaster()` + `Toast` (`glowkit_components::Toaster`, Sonner-style deck) |
| AI | `ChatMessage` (with a small Markdown renderer), `MessageActions`, `StreamingText`, `PromptInput` + `Attachment`, `IconSvg` (`glowkit_ai`) |
| Agents | `ApprovalCard` + `Risk` / `Decision` (one row of `glowkit_agents::ApprovalQueue`) |
| Charts | `Sparkline` (line, area, bar), `StatTile` (`glowkit_charts`) |

## Running the gallery (web)

The Dioxus CLI is not needed. The build uses cargo and wasm-bindgen (pinned to `=0.2.130`):

```bash
cd ports/dioxus
./examples/gallery/build-web.sh                  # -> examples/gallery/web/pkg
python3 -m http.server 8790 -d examples/gallery/web
# http://localhost:8790/#dock   (append @midnight / @ocean / @ember / @matrix for a theme)
```

You can also use `dx serve -p glowkit-dioxus-gallery`.
Desktop: `cargo run -p glowkit-dioxus-gallery --no-default-features --features desktop`.

## Status

- **Web (wasm32): primary, verified.** `cargo clippy --workspace -- -D warnings` passes on both
  the native and wasm32 targets. The tokens crate has unit tests. Playwright screenshots were
  checked against the egui gallery in the light and dark presets.
- **Desktop: compiles in principle but was not verified.** The build machine has no GTK or
  WebKitGTK development libraries. Aurora and DotGrid fall back to static CSS renderings off
  the web, because they draw through `web-sys` canvas. Everything else is plain DOM and CSS.
- **Known differences from egui:**
  - The shimmer spark and the border beam follow the border using CSS conic gradients and
    `offset-path` instead of an exact perimeter parameterization.
  - The Markdown renderer covers only a small subset: headings, lists, code fences, bold,
    italic and inline code.
  - `StreamingText` reveals plain text only, not Markdown.
  - `Marquee` takes `render: Callback<usize, Element>` instead of children, because a seamless
    loop needs two independent copies of the content.
- The CSS relies on recent browser features: `@property`, `color-mix`, `linear-gradient(in oklab ...)`,
  `offset-path: inset()` and `field-sizing`. These need Chromium 123 or later, or Safari 17.4
  or later.
