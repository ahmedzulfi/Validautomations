# Customizing glowkit

Everything in glowkit has a good default. Everything can also be changed. This guide shows the
four layers of customization, from global to local:

1. **Theme**: one set of tokens restyles every component (and egui's own widgets).
2. **Component setters**: every component takes chainable options.
3. **Motion**: easing, springs, triggers and the motion engine.
4. **Composition**: backgrounds behind content, cards around content, your own components.

Every code sample in this guide is compiled by `cargo test`, so it is always up to date.

## 1. Theme

```rust
use glowkit::prelude::*;

fn setup(ctx: &egui::Context) {
    // Built-in presets: dark (default), light, ocean, ember, matrix.
    Theme::light().install(ctx);
}
```

A theme is a plain struct of semantic tokens. Start from a preset and change what you need:

```rust
use glowkit::prelude::*;

fn brand_theme(ctx: &egui::Context) {
    let mut t = Theme::dark();
    t.accent = hex("#FF4F00");                                  // focus rings, highlights
    t.primary = hex("#FAFAFA");                                 // primary buttons
    t.gradient = [hex("#FF4F00"), hex("#FF0080"), hex("#7928CA")]; // "magic" effects
    t.chart = [hex("#FF4F00"), hex("#FF0080"), hex("#7928CA"), hex("#0070F3"), hex("#50E3C2"), hex("#F5A623")];
    t.radius = 14.0;                                            // every corner scales from this
    t.install(ctx);
}
```

| Token | Used for |
|---|---|
| `background`, `foreground` | page background and main text |
| `card`, `card_foreground`, `popover` | surfaces: cards, menus, dialogs |
| `muted`, `muted_foreground` | subtle fills and secondary text |
| `border`, `input`, `ring` | hairlines, input borders, focus rings |
| `primary`, `secondary`, `accent` (+ `_foreground`) | buttons and highlights |
| `destructive`, `success`, `warning`, `info` | status colors |
| `gradient` | shimmer, beams, glows and gradient text |
| `chart` | the six chart series colors |
| `radius` | base corner radius |

Read the active theme anywhere with `Theme::of(ui)`. Switching theme at runtime is just another
`install` call; every component picks it up on the next frame.

## 2. Component setters

Components are builders. Defaults come from the theme; setters override them:

```rust
use glowkit::prelude::*;
use glowkit::text::{GradientText, ShinyText};

fn headline(ui: &mut egui::Ui) {
    ui.add(ShinyText::new("Default look"));
    ui.add(
        ShinyText::new("Customized")
            .size(48.0)
            .color(hex("#71717A"))       // base color
            .shine_color(hex("#FFFFFF")) // highlight color
            .speed(2.0)                  // seconds per sweep
            .angle(30.0),                // tilt of the band
    );
    ui.add(GradientText::new("Brand gradient").colors(&[hex("#FF4F00"), hex("#FF0080")]).border(true));
}
```

Conventions shared across the library:

* `.size(..)` sets the main dimension (font size for text, height for backgrounds and charts).
* `.color(..)` / `.colors(..)` override theme colors.
* `.speed(..)` is seconds per cycle for looping effects.
* `.trigger(..)` picks when a one-shot animation plays (see below).
* Interactive effects that react to the pointer have an idle autoplay option, so they stay
  alive when nobody is hovering.

## 3. Motion

### Triggers

```rust
use glowkit::prelude::*;
use glowkit::text::{BlurText, SplitBy};

fn intro(ui: &mut egui::Ui) {
    ui.add(BlurText::new("Plays once when shown").trigger(Trigger::OnAppear));
    ui.add(BlurText::new("Replays on hover").trigger(Trigger::OnHover));
    ui.add(BlurText::new("Replays on click").trigger(Trigger::OnClick));
    ui.add(BlurText::new("Loops forever").split_by(SplitBy::Letters).trigger(Trigger::Loop { pause_ms: 1500 }));
}
```

One-shot animations stop requesting repaints when they finish, so an idle app stays idle.

### Easing and springs (core)

```rust
use glowkit::prelude::*;
use glowkit::anim::{ease, spring};

fn custom_widget(ui: &mut egui::Ui, open: bool) {
    // 31 easing curves.
    let t = ease(Easing::OutBack, 0.5);
    // A spring stored per widget id: call it every frame with the target.
    let x = spring(ui, ui.scope_id().with("panel"), if open { 1.0 } else { 0.0 }, 300.0, 22.0);
    let _ = (t, x);
}
```

### The motion engine (`glowkit::motion`)

For Framer-Motion-style work: animate any value, wrap content in a `MotionBox`, stagger groups,
animate presence and layout changes, drag with inertia, and build timelines.

```rust
use glowkit::prelude::*;
use glowkit::motion::{animate, MotionBox, Props, Transition};

fn motion(ui: &mut egui::Ui, expanded: bool) {
    // Animate a plain value toward a target with a spring.
    let width = animate(ui, ui.scope_id().with("w"), if expanded { 320.0 } else { 120.0 }, Transition::spring(260.0, 24.0));
    ui.label(format!("width: {width:.0}"));

    // Enter animation + hover state on arbitrary content.
    MotionBox::new(ui.scope_id().with("card"))
        .initial(Props::new().opacity(0.0).y(24.0))
        .animate(Props::new())
        .while_hover(Props::new().scale(1.03))
        .transition(Transition::tween(0.4).ease(Easing::OutCubic))
        .show(ui, |ui| {
            ui.label("I fade up, and grow a little on hover");
        });
}
```

## 4. Composition

Backgrounds paint into any rect, so they can sit behind your own layout:

```rust
use glowkit::backgrounds::{Aurora, Particles};

fn hero(ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    Aurora::new().paint(ui, rect);
    Particles::new().paint(ui, rect);
    ui.vertical_centered(|ui| {
        ui.add_space(120.0);
        ui.heading("Your content on top");
    });
}
```

Cards and borders wrap your content with `.show(ui, |ui| ...)`:

```rust
use glowkit::effects::{BorderBeam, SpotlightCard};

fn cards(ui: &mut egui::Ui) {
    SpotlightCard::new().show(ui, |ui| {
        ui.heading("Spotlight");
        ui.label("A light follows the pointer.");
    });
    // Border effects can also be painted around any rect you already have.
    let r = ui.min_rect();
    BorderBeam::new().paint(ui, r);
}
```

### Writing your own components

Everything glowkit uses is public in `glowkit::core`: painting helpers (`paint::gradient_rect`,
`paint::glow`, `paint::galley_with` for gradient text, `paint::shader_grid` for shader-like
backgrounds), per-glyph text layout (`core::text::Glyphs`), noise, color math, springs and clocks.
See [CONTRIBUTING.md](https://github.com/glowkit-rs/glowkit/blob/main/CONTRIBUTING.md) for the conventions every built-in component follows.

## Feature flags

Compile only what you use:

```toml
glowkit = { version = "0.1", default-features = false, features = ["text", "effects"] }
```

Features: `text`, `backgrounds`, `effects`, `components`, `charts`, `ai`, `blocks`, `motion`,
`three-d`, `canvas`, `templates` (all on by default).
